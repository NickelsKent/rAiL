//! Cranelift backend: SSA IR to a host object file.
//!
//! Building block: CraneliftBackend (contract C5, dev mode). Owning unit: U1
//! walking-skeleton (thin first version; completed by U6 execution-core).
//!
//! The target is looked up by its fixed triple with Cranelift's baseline
//! features (no host CPU detection), so the same input gives the same object
//! bytes on every machine of a platform (NFR3.2). No debug information is
//! emitted. This crate contains no `unsafe` code; it is exempt from
//! `forbid(unsafe_code)` only because Cranelift's API may one day need it.

#![deny(clippy::undocumented_unsafe_blocks)]
#![deny(clippy::print_stdout, clippy::print_stderr)]

use std::collections::BTreeMap;

use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::types::I64;
use cranelift_codegen::ir::{AbiParam, BlockArg, InstBuilder, Signature, TrapCode, UserFuncName};
use cranelift_codegen::isa::{self, OwnedTargetIsa};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::{Context, ir};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use rail_lower::{
    BinOp, Callee, CheckedOp, Inst, IrFunction, IrModule, RT_PRINT_I64, RT_TRAP, Term, Value,
};

/// A supported host platform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target {
    /// The name used in build results: `aarch64-macos` or `x86_64-linux`.
    pub name: &'static str,
    /// The Cranelift target triple.
    pub triple: &'static str,
}

/// A code generation failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodegenError {
    /// What went wrong.
    pub message: String,
}

fn error(what: &str, detail: impl std::fmt::Display) -> CodegenError {
    CodegenError {
        message: format!("code generation failed: {what}: {detail}"),
    }
}

/// The host platform, when it is one the skeleton supports (BR5.1).
pub fn host_target() -> Result<Target, CodegenError> {
    if cfg!(all(target_arch = "aarch64", target_os = "macos")) {
        Ok(Target {
            name: "aarch64-macos",
            triple: "aarch64-apple-darwin",
        })
    } else if cfg!(all(target_arch = "x86_64", target_os = "linux")) {
        Ok(Target {
            name: "x86_64-linux",
            triple: "x86_64-unknown-linux-gnu",
        })
    } else {
        Err(error(
            "unsupported host",
            "the skeleton builds for AArch64 macOS and x86-64 Linux only",
        ))
    }
}

fn isa() -> Result<OwnedTargetIsa, CodegenError> {
    let target = host_target()?;
    let mut flags = settings::builder();
    for (name, value) in [
        ("opt_level", "none"),
        ("is_pic", "true"),
        ("unwind_info", "false"),
    ] {
        flags.set(name, value).map_err(|e| error("setting", e))?;
    }
    isa::lookup_by_name(target.triple)
        .map_err(|e| error("target", e))?
        .finish(settings::Flags::new(flags))
        .map_err(|e| error("target", e))
}

/// Compiles a lowered module to object-file bytes for the host.
pub fn compile(module: &IrModule, name: &str) -> Result<Vec<u8>, CodegenError> {
    let builder = ObjectBuilder::new(isa()?, name, cranelift_module::default_libcall_names())
        .map_err(|e| error("object", e))?;
    let mut object = ObjectModule::new(builder);

    let signature = |object: &ObjectModule, params: usize, returns: bool| {
        let mut sig = object.make_signature();
        sig.params.extend((0..params).map(|_| AbiParam::new(I64)));
        if returns {
            sig.returns.push(AbiParam::new(I64));
        }
        sig
    };
    let mut ids: BTreeMap<String, (FuncId, Signature)> = BTreeMap::new();
    for f in &module.functions {
        let sig = signature(&object, f.params().len(), true);
        let linkage = if f.export {
            Linkage::Export
        } else {
            Linkage::Local
        };
        let id = object
            .declare_function(&f.symbol, linkage, &sig)
            .map_err(|e| error("declare", e))?;
        ids.insert(f.symbol.clone(), (id, sig));
    }
    // Runtime entries are imported only when used, so an object names
    // exactly the runtime functions it calls.
    let blocks = || module.functions.iter().flat_map(|f| &f.blocks);
    let uses_print = blocks().any(|b| {
        b.insts.iter().any(|i| {
            matches!(
                i,
                Inst::Call {
                    callee: Callee::PrintI64,
                    ..
                }
            )
        })
    });
    let uses_trap = blocks().any(|b| matches!(b.term, Term::Trap(_)));
    let mut import = |symbol: &str, used: bool| -> Result<Option<FuncId>, CodegenError> {
        if !used {
            return Ok(None);
        }
        let sig = signature(&object, 1, false);
        object
            .declare_function(symbol, Linkage::Import, &sig)
            .map(Some)
            .map_err(|e| error("declare", e))
    };
    let runtime = Runtime {
        print: import(RT_PRINT_I64, uses_print)?,
        trap: import(RT_TRAP, uses_trap)?,
    };

    let mut context = Context::new();
    let mut builder_context = FunctionBuilderContext::new();
    for (index, f) in module.functions.iter().enumerate() {
        let Some((id, sig)) = ids.get(&f.symbol) else {
            return Err(error("define", &f.symbol));
        };
        context.func.signature = sig.clone();
        context.func.name = UserFuncName::user(0, u32::try_from(index).unwrap_or(u32::MAX));
        translate(
            f,
            &mut context.func,
            &mut builder_context,
            &mut object,
            &ids,
            runtime,
        )?;
        object
            .define_function(*id, &mut context)
            .map_err(|e| error(&f.symbol, e))?;
        object.clear_context(&mut context);
    }
    object.finish().emit().map_err(|e| error("emit", e))
}

#[derive(Clone, Copy)]
struct Runtime {
    print: Option<FuncId>,
    trap: Option<FuncId>,
}

fn translate(
    f: &IrFunction,
    func: &mut ir::Function,
    builder_context: &mut FunctionBuilderContext,
    object: &mut ObjectModule,
    ids: &BTreeMap<String, (FuncId, Signature)>,
    runtime: Runtime,
) -> Result<(), CodegenError> {
    let frontend = object.isa().frontend_config();
    let mut b = FunctionBuilder::new(func, builder_context);
    let blocks: Vec<ir::Block> = f.blocks.iter().map(|_| b.create_block()).collect();
    let mut values: BTreeMap<Value, ir::Value> = BTreeMap::new();
    for (block, ir_block) in f.blocks.iter().zip(&blocks) {
        for param in &block.params {
            let v = b.append_block_param(*ir_block, I64);
            values.insert(*param, v);
        }
    }
    let get = |values: &BTreeMap<Value, ir::Value>, v: &Value| {
        values
            .get(v)
            .copied()
            .ok_or_else(|| error(&f.symbol, format!("undefined value v{}", v.0)))
    };
    for (block, ir_block) in f.blocks.iter().zip(&blocks) {
        b.switch_to_block(*ir_block);
        for inst in &block.insts {
            match inst {
                Inst::Iconst { dst, value } => {
                    let v = b.ins().iconst(I64, *value);
                    values.insert(*dst, v);
                }
                Inst::Binary { dst, op, lhs, rhs } => {
                    let (l, r) = (get(&values, lhs)?, get(&values, rhs)?);
                    let v = binary(&mut b, *op, l, r);
                    values.insert(*dst, v);
                }
                Inst::Overflows { dst, op, lhs, rhs } => {
                    let (l, r) = (get(&values, lhs)?, get(&values, rhs)?);
                    let flag = overflows(&mut b, *op, l, r);
                    let v = b.ins().uextend(I64, flag);
                    values.insert(*dst, v);
                }
                Inst::Call { dst, callee, args } => {
                    let args = args
                        .iter()
                        .map(|a| get(&values, a))
                        .collect::<Result<Vec<_>, _>>()?;
                    let v = match callee {
                        Callee::Function(symbol) => {
                            let (id, _) = ids.get(symbol).ok_or_else(|| error("call", symbol))?;
                            let fref = object.declare_func_in_func(*id, b.func);
                            let call = b.ins().call(fref, &args);
                            b.inst_results(call)
                                .first()
                                .copied()
                                .ok_or_else(|| error("call", symbol))?
                        }
                        Callee::PrintI64 => {
                            let id = runtime.print.ok_or_else(|| error("call", RT_PRINT_I64))?;
                            let fref = object.declare_func_in_func(id, b.func);
                            b.ins().call(fref, &args);
                            b.ins().iconst(I64, 0)
                        }
                    };
                    values.insert(*dst, v);
                }
            }
        }
        match &block.term {
            Term::Return(v) => {
                let v = get(&values, v)?;
                b.ins().return_(&[v]);
            }
            Term::Jump(target, args) => {
                let args = args
                    .iter()
                    .map(|a| get(&values, a).map(BlockArg::Value))
                    .collect::<Result<Vec<_>, _>>()?;
                b.ins().jump(blocks[target.0 as usize], &args);
            }
            Term::Brif {
                cond,
                then_block,
                else_block,
            } => {
                let c = get(&values, cond)?;
                b.ins().brif(
                    c,
                    blocks[then_block.0 as usize],
                    &[],
                    blocks[else_block.0 as usize],
                    &[],
                );
            }
            Term::Trap(kind) => {
                let id = runtime.trap.ok_or_else(|| error("trap", RT_TRAP))?;
                let fref = object.declare_func_in_func(id, b.func);
                let code = b.ins().iconst(I64, kind.code());
                b.ins().call(fref, &[code]);
                // rail_rt_trap never returns; this marks the block's end.
                b.ins().trap(TrapCode::unwrap_user(1));
            }
        }
    }
    b.seal_all_blocks();
    b.finalize(frontend);
    Ok(())
}

fn binary(b: &mut FunctionBuilder<'_>, op: BinOp, l: ir::Value, r: ir::Value) -> ir::Value {
    let compare = |b: &mut FunctionBuilder<'_>, cc: IntCC| {
        let flag = b.ins().icmp(cc, l, r);
        b.ins().uextend(I64, flag)
    };
    match op {
        BinOp::Add => b.ins().iadd(l, r),
        BinOp::Sub => b.ins().isub(l, r),
        BinOp::Mul => b.ins().imul(l, r),
        BinOp::Div => b.ins().sdiv(l, r),
        BinOp::Rem => b.ins().srem(l, r),
        BinOp::Eq => compare(b, IntCC::Equal),
        BinOp::Ne => compare(b, IntCC::NotEqual),
        BinOp::Lt => compare(b, IntCC::SignedLessThan),
        BinOp::Le => compare(b, IntCC::SignedLessThanOrEqual),
        BinOp::Gt => compare(b, IntCC::SignedGreaterThan),
        BinOp::Ge => compare(b, IntCC::SignedGreaterThanOrEqual),
    }
}

/// An `i8` flag that is 1 when `l op r` overflows `i64`.
fn overflows(b: &mut FunctionBuilder<'_>, op: CheckedOp, l: ir::Value, r: ir::Value) -> ir::Value {
    match op {
        CheckedOp::Add => {
            // Overflow iff both operands have the sign opposite to the sum's.
            let sum = b.ins().iadd(l, r);
            let a = b.ins().bxor(l, sum);
            let c = b.ins().bxor(r, sum);
            let both = b.ins().band(a, c);
            b.ins().icmp_imm_s(IntCC::SignedLessThan, both, 0)
        }
        CheckedOp::Sub => {
            // Overflow iff the operands differ in sign and the difference's
            // sign differs from the left operand's.
            let diff = b.ins().isub(l, r);
            let a = b.ins().bxor(l, r);
            let c = b.ins().bxor(l, diff);
            let both = b.ins().band(a, c);
            b.ins().icmp_imm_s(IntCC::SignedLessThan, both, 0)
        }
        CheckedOp::Mul => {
            // Overflow iff the high half is not the sign extension of the low.
            let low = b.ins().imul(l, r);
            let high = b.ins().smulhi(l, r);
            let sign = b.ins().sshr_imm_s(low, 63);
            b.ins().icmp(IntCC::NotEqual, high, sign)
        }
        CheckedOp::Div => {
            let min = b.ins().icmp_imm_s(IntCC::Equal, l, i64::MIN);
            let neg_one = b.ins().icmp_imm_s(IntCC::Equal, r, -1);
            b.ins().band(min, neg_one)
        }
    }
}
