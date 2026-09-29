//! Lowering of checked definitions to a small SSA IR with checked arithmetic.
//!
//! Building block: Lowering. Owning unit: U1 walking-skeleton (thin first
//! version; completed by U6 execution-core).
//!
//! Every skeleton value is a 64-bit integer: `i64` as itself, `bool` as 0 or
//! 1, `unit` and `Caps` as 0, and `main`'s Result as its tag (0 for `Ok`, 1
//! for `Err`). Arithmetic is checked: `+ - *` branch to an overflow trap, and
//! `/ %` branch to a division-by-zero trap and to an overflow trap for the
//! minimum `i64` divided by -1 (BR5.4). A module with `main` also gets the
//! exported `rail_entry` wrapper the runtime calls (E7).

#![forbid(unsafe_code)]
#![deny(clippy::print_stdout, clippy::print_stderr)]

use std::fmt::Write as _;

use rail_check::TypedModule;
use rail_syntax::{Expr, ExprKind, Pattern};

/// The generated entry point the runtime's C `main` calls.
pub const ENTRY_SYMBOL: &str = "rail_entry";
/// The runtime's `skel.print_i64`.
pub const RT_PRINT_I64: &str = "rail_rt_print_i64";
/// The runtime's trap entry.
pub const RT_TRAP: &str = "rail_rt_trap";

/// An SSA value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Value(pub u32);

/// A block index within its function.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlockId(pub u32);

/// Two-operand operations that cannot fail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    /// Wrapping addition (after an overflow check).
    Add,
    /// Wrapping subtraction (after an overflow check).
    Sub,
    /// Wrapping multiplication (after an overflow check).
    Mul,
    /// Signed division (after the zero and overflow checks).
    Div,
    /// Signed remainder (after the zero and overflow checks).
    Rem,
    /// `==`, giving 0 or 1.
    Eq,
    /// `!=`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
}

impl BinOp {
    /// The operation's name in the IR text.
    pub fn name(self) -> &'static str {
        match self {
            BinOp::Add => "add",
            BinOp::Sub => "sub",
            BinOp::Mul => "mul",
            BinOp::Div => "div",
            BinOp::Rem => "rem",
            BinOp::Eq => "eq",
            BinOp::Ne => "ne",
            BinOp::Lt => "lt",
            BinOp::Le => "le",
            BinOp::Gt => "gt",
            BinOp::Ge => "ge",
        }
    }
}

/// Operations whose overflow is checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckedOp {
    /// `a + b` overflows.
    Add,
    /// `a - b` overflows.
    Sub,
    /// `a * b` overflows.
    Mul,
    /// `a / b` (and `a % b`) overflows: minimum `i64` divided by -1.
    Div,
}

impl CheckedOp {
    /// The operation's name in the IR text.
    pub fn name(self) -> &'static str {
        match self {
            CheckedOp::Add => "add",
            CheckedOp::Sub => "sub",
            CheckedOp::Mul => "mul",
            CheckedOp::Div => "div",
        }
    }
}

/// What a call calls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Callee {
    /// A function of this module, by symbol.
    Function(String),
    /// The runtime's `rail_rt_print_i64`; the call's value is unit (0).
    PrintI64,
}

/// A trap kind, passed to the runtime's trap entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrapKind {
    /// Integer overflow.
    Overflow,
    /// Division or remainder by zero.
    DivByZero,
}

impl TrapKind {
    /// The code passed to `rail_rt_trap`.
    pub fn code(self) -> i64 {
        match self {
            TrapKind::Overflow => 1,
            TrapKind::DivByZero => 2,
        }
    }

    fn name(self) -> &'static str {
        match self {
            TrapKind::Overflow => "overflow",
            TrapKind::DivByZero => "div_by_zero",
        }
    }
}

/// An instruction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Inst {
    /// `dst = value`
    Iconst {
        /// Result.
        dst: Value,
        /// Constant.
        value: i64,
    },
    /// `dst = op lhs, rhs`
    Binary {
        /// Result.
        dst: Value,
        /// Operation.
        op: BinOp,
        /// Left operand.
        lhs: Value,
        /// Right operand.
        rhs: Value,
    },
    /// `dst = 1 if op lhs, rhs overflows, else 0`
    Overflows {
        /// Result.
        dst: Value,
        /// Operation.
        op: CheckedOp,
        /// Left operand.
        lhs: Value,
        /// Right operand.
        rhs: Value,
    },
    /// `dst = callee(args)`
    Call {
        /// Result.
        dst: Value,
        /// Called function.
        callee: Callee,
        /// Arguments.
        args: Vec<Value>,
    },
}

/// A block terminator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Term {
    /// Return a value.
    Return(Value),
    /// Jump, passing block arguments.
    Jump(BlockId, Vec<Value>),
    /// Branch on a nonzero condition.
    Brif {
        /// Condition.
        cond: Value,
        /// Target when nonzero.
        then_block: BlockId,
        /// Target when zero.
        else_block: BlockId,
    },
    /// Call the runtime trap entry; never returns.
    Trap(TrapKind),
}

/// A basic block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    /// Block parameters.
    pub params: Vec<Value>,
    /// Instructions.
    pub insts: Vec<Inst>,
    /// Terminator.
    pub term: Term,
}

/// A lowered function (a thin `IrFunction`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrFunction {
    /// Symbol name.
    pub symbol: String,
    /// Whether the symbol is exported from the object file.
    pub export: bool,
    /// Blocks; block 0 is the entry and its parameters are the arguments.
    pub blocks: Vec<Block>,
}

impl IrFunction {
    /// The function's parameters (the entry block's parameters).
    pub fn params(&self) -> &[Value] {
        self.blocks.first().map_or(&[], |b| &b.params)
    }

    /// Deterministic text form, used by tests and for debugging.
    pub fn render(&self) -> String {
        let list = |values: &[Value]| {
            values
                .iter()
                .map(|v| format!("v{}", v.0))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let mut out = format!(
            "function {}({}) {}\n",
            self.symbol,
            list(self.params()),
            if self.export { "export" } else { "local" }
        );
        for (i, block) in self.blocks.iter().enumerate() {
            let _ = writeln!(out, "b{i}({}):", list(&block.params));
            for inst in &block.insts {
                let _ = match inst {
                    Inst::Iconst { dst, value } => writeln!(out, "  v{} = iconst {value}", dst.0),
                    Inst::Binary { dst, op, lhs, rhs } => {
                        writeln!(out, "  v{} = {} v{}, v{}", dst.0, op.name(), lhs.0, rhs.0)
                    }
                    Inst::Overflows { dst, op, lhs, rhs } => {
                        writeln!(
                            out,
                            "  v{} = overflows {} v{}, v{}",
                            dst.0,
                            op.name(),
                            lhs.0,
                            rhs.0
                        )
                    }
                    Inst::Call { dst, callee, args } => {
                        let name = match callee {
                            Callee::Function(symbol) => symbol.as_str(),
                            Callee::PrintI64 => RT_PRINT_I64,
                        };
                        writeln!(out, "  v{} = call {name}({})", dst.0, list(args))
                    }
                };
            }
            let _ = match &block.term {
                Term::Return(v) => writeln!(out, "  return v{}", v.0),
                Term::Jump(b, args) => writeln!(out, "  jump b{}({})", b.0, list(args)),
                Term::Brif {
                    cond,
                    then_block,
                    else_block,
                } => writeln!(
                    out,
                    "  brif v{}, b{}(), b{}()",
                    cond.0, then_block.0, else_block.0
                ),
                Term::Trap(kind) => writeln!(out, "  trap {}", kind.name()),
            };
        }
        out
    }
}

/// A lowered module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrModule {
    /// Functions in module order, then `rail_entry` when there is a `main`.
    pub functions: Vec<IrFunction>,
}

impl IrModule {
    /// Deterministic text form of every function.
    pub fn render(&self) -> String {
        self.functions.iter().map(IrFunction::render).collect()
    }
}

/// The symbol of the function with definition ID `defid`.
pub fn symbol(defid: &str) -> String {
    format!("rail_f_{}", defid.trim_start_matches('#'))
}

/// Lowers a checked module.
pub fn lower(typed: &TypedModule) -> IrModule {
    let mut functions = Vec::new();
    let mut main_symbol = None;
    for f in &typed.module.fns {
        let sym = symbol(&f.id.text);
        if f.name.text == "main" {
            main_symbol = Some(sym.clone());
        }
        let mut builder = Builder::new(sym, false);
        let mut scope = Vec::new();
        for p in &f.params {
            let v = builder.value();
            builder.blocks[0].params.push(v);
            if let Pattern::Name(name) = p {
                scope.push((name.text.clone(), v));
            }
        }
        let result = builder.expr(&f.body, &mut scope, typed);
        functions.push(builder.finish(Term::Return(result)));
    }
    if let Some(main) = main_symbol {
        let mut builder = Builder::new(ENTRY_SYMBOL.to_string(), true);
        let caps = builder.iconst(0);
        let tag = builder.value();
        builder.emit(Inst::Call {
            dst: tag,
            callee: Callee::Function(main),
            args: vec![caps],
        });
        functions.push(builder.finish(Term::Return(tag)));
    }
    IrModule { functions }
}

struct PartialBlock {
    params: Vec<Value>,
    insts: Vec<Inst>,
    term: Option<Term>,
}

struct Builder {
    symbol: String,
    export: bool,
    blocks: Vec<PartialBlock>,
    current: usize,
    next_value: u32,
}

impl Builder {
    fn new(symbol: String, export: bool) -> Builder {
        let mut builder = Builder {
            symbol,
            export,
            blocks: Vec::new(),
            current: 0,
            next_value: 0,
        };
        builder.block();
        builder
    }

    fn value(&mut self) -> Value {
        let v = Value(self.next_value);
        self.next_value += 1;
        v
    }

    fn block(&mut self) -> BlockId {
        self.blocks.push(PartialBlock {
            params: Vec::new(),
            insts: Vec::new(),
            term: None,
        });
        BlockId(u32::try_from(self.blocks.len() - 1).unwrap_or(u32::MAX))
    }

    fn emit(&mut self, inst: Inst) {
        if let Some(block) = self.blocks.get_mut(self.current) {
            block.insts.push(inst);
        }
    }

    fn terminate(&mut self, term: Term) {
        if let Some(block) = self.blocks.get_mut(self.current) {
            block.term = Some(term);
        }
    }

    fn switch_to(&mut self, block: BlockId) {
        self.current = block.0 as usize;
    }

    fn iconst(&mut self, value: i64) -> Value {
        let dst = self.value();
        self.emit(Inst::Iconst { dst, value });
        dst
    }

    fn binary(&mut self, op: BinOp, lhs: Value, rhs: Value) -> Value {
        let dst = self.value();
        self.emit(Inst::Binary { dst, op, lhs, rhs });
        dst
    }

    /// Branches to a trap when `cond` is nonzero; continues in a new block.
    fn trap_if(&mut self, cond: Value, kind: TrapKind) {
        let trap = self.block();
        let next = self.block();
        self.terminate(Term::Brif {
            cond,
            then_block: trap,
            else_block: next,
        });
        self.switch_to(trap);
        self.terminate(Term::Trap(kind));
        self.switch_to(next);
    }

    fn checked(&mut self, op: CheckedOp, lhs: Value, rhs: Value) {
        let dst = self.value();
        self.emit(Inst::Overflows { dst, op, lhs, rhs });
        self.trap_if(dst, TrapKind::Overflow);
    }

    fn finish(mut self, term: Term) -> IrFunction {
        self.terminate(term);
        IrFunction {
            symbol: self.symbol,
            export: self.export,
            blocks: self
                .blocks
                .into_iter()
                .map(|b| Block {
                    params: b.params,
                    insts: b.insts,
                    term: b.term.unwrap_or(Term::Trap(TrapKind::Overflow)),
                })
                .collect(),
        }
    }

    fn expr(&mut self, e: &Expr, scope: &mut Vec<(String, Value)>, typed: &TypedModule) -> Value {
        match &e.kind {
            ExprKind::Int(n) => self.iconst(*n),
            ExprKind::Bool(b) => self.iconst(i64::from(*b)),
            ExprKind::Unit => self.iconst(0),
            ExprKind::Ref(name) => match scope.iter().rev().find(|(n, _)| n == name) {
                Some((_, v)) => *v,
                // Unreachable after a clean check: references are locals.
                None => self.iconst(0),
            },
            ExprKind::Let { bindings, body } => {
                let depth = scope.len();
                for b in bindings {
                    let v = self.expr(&b.value, scope, typed);
                    if let Pattern::Name(name) = &b.pattern {
                        scope.push((name.text.clone(), v));
                    }
                }
                let v = self.expr(body, scope, typed);
                scope.truncate(depth);
                v
            }
            ExprKind::Match {
                scrutinee,
                on_true,
                on_false,
            } => {
                let cond = self.expr(scrutinee, scope, typed);
                let then_block = self.block();
                let else_block = self.block();
                let join = self.block();
                self.terminate(Term::Brif {
                    cond,
                    then_block,
                    else_block,
                });
                self.switch_to(then_block);
                let t = self.expr(on_true, scope, typed);
                self.terminate(Term::Jump(join, vec![t]));
                self.switch_to(else_block);
                let f = self.expr(on_false, scope, typed);
                self.terminate(Term::Jump(join, vec![f]));
                let result = self.value();
                self.blocks[join.0 as usize].params.push(result);
                self.switch_to(join);
                result
            }
            ExprKind::Apply { head, args } => {
                let name = match &head.kind {
                    ExprKind::Ref(name) => name.as_str(),
                    _ => "",
                };
                let values: Vec<Value> = args.iter().map(|a| self.expr(a, scope, typed)).collect();
                self.apply(name, &values, typed)
            }
        }
    }

    fn apply(&mut self, name: &str, args: &[Value], typed: &TypedModule) -> Value {
        let (a, b) = (args.first().copied(), args.get(1).copied());
        let pair = a.zip(b);
        match (name, pair) {
            ("+", Some((a, b))) => self.arith(CheckedOp::Add, BinOp::Add, a, b),
            ("-", Some((a, b))) => self.arith(CheckedOp::Sub, BinOp::Sub, a, b),
            ("*", Some((a, b))) => self.arith(CheckedOp::Mul, BinOp::Mul, a, b),
            ("/", Some((a, b))) => self.division(BinOp::Div, a, b),
            ("%", Some((a, b))) => self.division(BinOp::Rem, a, b),
            ("==", Some((a, b))) => self.binary(BinOp::Eq, a, b),
            ("!=", Some((a, b))) => self.binary(BinOp::Ne, a, b),
            ("<", Some((a, b))) => self.binary(BinOp::Lt, a, b),
            ("<=", Some((a, b))) => self.binary(BinOp::Le, a, b),
            (">", Some((a, b))) => self.binary(BinOp::Gt, a, b),
            (">=", Some((a, b))) => self.binary(BinOp::Ge, a, b),
            ("Ok", _) => self.iconst(0),
            ("Err", _) => self.iconst(1),
            ("skel.print_i64", _) => {
                let dst = self.value();
                self.emit(Inst::Call {
                    dst,
                    callee: Callee::PrintI64,
                    args: args.to_vec(),
                });
                dst
            }
            (callee, _) => {
                let defid = typed
                    .module
                    .fns
                    .iter()
                    .find(|f| f.name.text == callee)
                    .map_or_else(String::new, |f| f.id.text.clone());
                let dst = self.value();
                self.emit(Inst::Call {
                    dst,
                    callee: Callee::Function(symbol(&defid)),
                    args: args.to_vec(),
                });
                dst
            }
        }
    }

    fn arith(&mut self, check: CheckedOp, op: BinOp, a: Value, b: Value) -> Value {
        self.checked(check, a, b);
        self.binary(op, a, b)
    }

    fn division(&mut self, op: BinOp, a: Value, b: Value) -> Value {
        let zero = self.iconst(0);
        let is_zero = self.binary(BinOp::Eq, b, zero);
        self.trap_if(is_zero, TrapKind::DivByZero);
        self.checked(CheckedOp::Div, a, b);
        self.binary(op, a, b)
    }
}
