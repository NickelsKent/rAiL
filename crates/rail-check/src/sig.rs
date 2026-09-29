//! Signature checks: presence and export (BR2.1), skeleton types (BR2.2),
//! main's shape and where Caps and Result may appear (BR2.7), effect labels
//! (BR3.3).

use rail_diag::{SKL001, TY001};
use rail_syntax::{FnDef, Module, TypeExpr};

use crate::{FnType, Sink, Ty};

/// Effect labels of the full language that this build does not support.
const OTHER_EFFECTS: [&str; 11] = [
    "fs", "net", "proc", "env", "clk", "rnd", "st", "par", "nd", "div", "ffi",
];

/// Primitive types of the full language that this build does not support.
const OTHER_PRIMS: [&str; 12] = [
    "i8", "i16", "i32", "u8", "u16", "u32", "u64", "f32", "f64", "char", "str", "bytes",
];

/// Checks `f`'s signature. Returns its type when the signature can be used to
/// type the body (placement errors are reported but do not stop that).
pub(crate) fn check_signature(f: &FnDef, module: &Module, sink: &mut Sink) -> Option<FnType> {
    let def = f.id.text.as_str();
    let name = f.name.text.as_str();
    let Some(sig) = &f.sig else {
        sink.report(
            SKL001,
            def,
            "1",
            f.name.span,
            format!("{name} has no signature; every function needs one"),
        );
        return None;
    };
    if !module.exports.iter().any(|e| e.text == name) {
        sink.report(
            SKL001,
            def,
            "1",
            f.name.span,
            format!("{name} has a signature but is not exported"),
        );
    }
    let TypeExpr::Func {
        params,
        result,
        effects,
        path,
        ..
    } = sig
    else {
        sink.report(
            SKL001,
            def,
            sig.path(),
            sig.span(),
            "a signature must be a function type".to_string(),
        );
        return None;
    };
    let mut valid = true;
    let mut log = false;
    for (j, effect) in effects.iter().enumerate() {
        let effect_path = format!("{path}.{}", j + 2);
        let label = effect.text.as_str();
        let problem = match label {
            "log" if log => format!("duplicate effect {label}"),
            "log" => {
                log = true;
                continue;
            }
            l if OTHER_EFFECTS.contains(&l) => {
                format!("effect {l} is not yet supported; log is the only effect")
            }
            l if l.len() == 1 => {
                "effect row variables are not yet supported in this toolchain build".to_string()
            }
            l => format!("unknown effect {l}"),
        };
        sink.report(SKL001, def, &effect_path, effect.span, problem);
        valid = false;
    }
    let param_tys: Vec<Option<Ty>> = params.iter().map(|p| skeleton_type(p, def, sink)).collect();
    let result_ty = skeleton_type(result, def, sink);
    let (Some(param_tys), Some(result_ty)) = (
        param_tys.into_iter().collect::<Option<Vec<Ty>>>(),
        result_ty,
    ) else {
        return None;
    };
    if !valid {
        return None;
    }
    placement(
        name,
        def,
        &format!("{path}.0"),
        params,
        &param_tys,
        result,
        &result_ty,
        sink,
    );
    if param_tys.len() != f.params.len() {
        sink.report(
            TY001,
            def,
            "2",
            f.name.span,
            format!(
                "{name} has {} parameters but its signature has {}",
                f.params.len(),
                param_tys.len()
            ),
        );
        return None;
    }
    Some(FnType {
        params: param_tys,
        result: result_ty,
        log,
    })
}

/// BR2.7: main is `(-> (Caps) (Result unit E) [log])`; Caps and Result
/// appear nowhere else.
#[allow(clippy::too_many_arguments)]
fn placement(
    name: &str,
    def: &str,
    params_path: &str,
    params: &[TypeExpr],
    param_tys: &[Ty],
    result: &TypeExpr,
    result_ty: &Ty,
    sink: &mut Sink,
) {
    let is_result = |t: &Ty| matches!(t, Ty::Result(..));
    if name == "main" {
        if param_tys.len() != 1 {
            let span = params.first().map_or(result.span(), TypeExpr::span);
            sink.report(
                TY001,
                def,
                params_path,
                span,
                "main takes exactly one parameter, of type Caps".to_string(),
            );
        } else if param_tys[0] != Ty::Caps {
            sink.report(
                TY001,
                def,
                params[0].path(),
                params[0].span(),
                "main's parameter must be Caps".to_string(),
            );
        }
        if !is_result(result_ty) {
            sink.report(
                TY001,
                def,
                result.path(),
                result.span(),
                "main must return (Result unit E) with E one of i64, bool, unit".to_string(),
            );
        }
        return;
    }
    for (p, t) in params.iter().zip(param_tys) {
        misplaced(t, p, def, sink);
    }
    misplaced(result_ty, result, def, sink);
}

fn misplaced(t: &Ty, at: &TypeExpr, def: &str, sink: &mut Sink) {
    let message = match t {
        Ty::Caps => "Caps is only allowed as the parameter of main",
        Ty::Result(..) => "Result is only allowed as the result of main",
        _ => return,
    };
    sink.report(TY001, def, at.path(), at.span(), message.to_string());
}

/// Converts a signature type to a skeleton type, reporting SKL001 for
/// anything outside the skeleton's types (BR2.2).
fn skeleton_type(t: &TypeExpr, def: &str, sink: &mut Sink) -> Option<Ty> {
    let unsupported = |sink: &mut Sink, at: &TypeExpr, message: String| {
        sink.report(SKL001, def, at.path(), at.span(), message);
        None
    };
    match t {
        TypeExpr::Name { name, .. } => match name.text.as_str() {
            "i64" => Some(Ty::I64),
            "bool" => Some(Ty::Bool),
            "unit" => Some(Ty::Unit),
            "Caps" => Some(Ty::Caps),
            n if OTHER_PRIMS.contains(&n) || rail_syntax::is_uname(n) => unsupported(
                sink,
                t,
                format!("type {n} is not yet supported in this toolchain build"),
            ),
            n if n.len() == 1 => unsupported(
                sink,
                t,
                "type variables are not yet supported in this toolchain build".to_string(),
            ),
            n => unsupported(sink, t, format!("unknown type {n}")),
        },
        TypeExpr::Apply { head, args, .. } if head.text == "Result" && args.len() == 2 => {
            if !matches!(&args[0], TypeExpr::Name { name, .. } if name.text == "unit") {
                return unsupported(
                    sink,
                    &args[0],
                    "Result must have ok type unit in this toolchain build".to_string(),
                );
            }
            let err = match &args[1] {
                TypeExpr::Name { name, .. } => match name.text.as_str() {
                    "i64" => Some(Ty::I64),
                    "bool" => Some(Ty::Bool),
                    "unit" => Some(Ty::Unit),
                    _ => None,
                },
                _ => None,
            };
            match err {
                Some(err) => Some(Ty::Result(Box::new(Ty::Unit), Box::new(err))),
                None => unsupported(
                    sink,
                    &args[1],
                    "Result's error type must be i64, bool or unit in this toolchain build"
                        .to_string(),
                ),
            }
        }
        TypeExpr::Apply { head, .. } => unsupported(
            sink,
            t,
            format!(
                "type {} is not yet supported in this toolchain build",
                head.text
            ),
        ),
        TypeExpr::Func { .. } => unsupported(
            sink,
            t,
            "function types as values are not yet supported in this toolchain build".to_string(),
        ),
    }
}
