//! Name resolution, typing and effects (BR1.6, BR1.7, BR2.1–BR2.7,
//! BR3.1–BR3.3, BR4.3).

use rail_check::{Ty, check};

/// A module `t.m` with these fn items (already sorted by name) and exports.
fn module(exports: &str, fns: &[&str]) -> String {
    let mut src = String::from("(mod t.m)\n");
    if !exports.is_empty() {
        src.push_str(&format!("(pub {exports})\n"));
    }
    for f in fns {
        src.push_str(f);
        src.push('\n');
    }
    src
}

/// `f : (-> (i64) i64)` with the given body.
fn f_body(body: &str) -> String {
    module(
        "f",
        &[&format!("(fn #aaaaaa f (x) (: (-> (i64) i64)) {body})")],
    )
}

/// `f : (-> (i64) i64)` whose parameter is `_`, with the given body.
fn f_const(body: &str) -> String {
    module(
        "f",
        &[&format!("(fn #aaaaaa f (_) (: (-> (i64) i64)) {body})")],
    )
}

/// (rule, def, path, message) of every diagnostic, in output order.
fn diags(src: &str) -> Vec<(String, String, String, String)> {
    check("t.m", src.as_bytes())
        .result
        .diagnostics
        .iter()
        .map(|d| {
            (
                d.rule.as_str().to_string(),
                d.loc.def.clone(),
                d.loc.path.clone(),
                d.message.clone(),
            )
        })
        .collect()
}

/// Exactly one diagnostic; returns (rule, path, message).
fn one(src: &str) -> (String, String, String) {
    let all = diags(src);
    assert_eq!(all.len(), 1, "{src}\n{all:?}");
    let (rule, _, path, message) = all.into_iter().next().unwrap();
    (rule, path, message)
}

fn clean(src: &str) {
    let all = diags(src);
    assert!(all.is_empty(), "{src}\n{all:?}");
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/skeleton")
            .join(name),
    )
    .unwrap()
}

#[test]
fn the_fixtures_check_as_designed() {
    let answer = check("skeleton.answer", &fixture("answer.rlc"));
    assert!(!answer.result.blocking);
    let typed = answer.typed.expect("typed module");
    assert_eq!(typed.fns.len(), 2);
    assert_eq!(typed.fns[1].ty.params, [Ty::Caps]);
    assert_eq!(
        typed.fns[1].ty.result,
        Ty::Result(Box::new(Ty::Unit), Box::new(Ty::Unit))
    );
    assert!(typed.fns[1].ty.log);

    let broken = check("skeleton.broken", &fixture("broken.rlc"));
    assert!(broken.typed.is_none());
    assert!(broken.result.blocking);
    let d = &broken.result.diagnostics;
    assert_eq!(d.len(), 1);
    assert_eq!(
        d[0].render(),
        "error TY001 skeleton.broken #sk0a1b/4 [82,90]: expected i64, found bool"
    );
}

#[test]
fn ty001_for_a_mismatch_at_the_smallest_node() {
    assert_eq!(
        one(&f_const("true")),
        (
            "TY001".into(),
            "4".into(),
            "expected i64, found bool".into()
        )
    );
    assert_eq!(
        one(&f_body("(+ x true)")),
        (
            "TY001".into(),
            "4.2".into(),
            "expected i64, found bool".into()
        )
    );
    assert_eq!(
        one(&f_body("(< x 1)")),
        (
            "TY001".into(),
            "4".into(),
            "expected i64, found bool".into()
        )
    );
    assert_eq!(
        one(&f_const("()")),
        (
            "TY001".into(),
            "4".into(),
            "expected i64, found unit".into()
        )
    );
    let (rule, path, message) = one(&f_body("(f x 1)"));
    assert_eq!((rule.as_str(), path.as_str()), ("TY001", "4"));
    assert!(message.contains("1 argument"), "{message}");
}

#[test]
fn ty001_for_operator_arguments() {
    assert_eq!(
        one(&f_body("(* (== x 1) 2)")),
        (
            "TY001".into(),
            "4.1".into(),
            "expected i64, found bool".into()
        )
    );
    assert_eq!(
        one(&f_body("(let (b (== x true)) (? b (true 1) (false 0)))")).1,
        "4.0.1.2"
    );
    let (rule, path, message) = one(&f_body("(let (u (== () ())) (? u (true 1) (false x)))"));
    assert_eq!((rule.as_str(), path.as_str()), ("TY001", "4.0.1.1"));
    assert!(message.contains("unit"), "{message}");
    clean(&f_body(
        "(let (b (!= (== x 1) false)) (? b (true (% x 2)) (false (/ (- x 1) 3))))",
    ));
}

#[test]
fn ty001_for_a_non_bool_scrutinee_and_differing_arms() {
    assert_eq!(
        one(&f_body("(? x (true 1) (false 2))")),
        (
            "TY001".into(),
            "4.0".into(),
            "expected bool, found i64".into()
        )
    );
    assert_eq!(
        one(&f_body("(let (y (? (== x 1) (true 1) (false false))) y)")),
        (
            "TY001".into(),
            "4.0.1.2.1".into(),
            "expected i64, found bool".into()
        )
    );
}

#[test]
fn ty001_for_main_shape_and_misplaced_caps_or_result() {
    let main = |sig: &str| {
        module(
            "main",
            &[&format!("(fn #aaaaaa main (_) (: {sig}) (Ok ()))")],
        )
    };
    clean(&main("(-> (Caps) (Result unit bool))"));
    clean(&main("(-> (Caps) (Result unit i64) log)"));
    assert_eq!(one(&main("(-> (i64) (Result unit unit))")).1, "3.0.0.0");
    let (rule, path, _) = one(&module(
        "main",
        &["(fn #aaaaaa main (_) (: (-> (Caps) i64)) 1)"],
    ));
    assert_eq!((rule.as_str(), path.as_str()), ("TY001", "3.0.1"));
    let (rule, path, _) = one(&module(
        "main",
        &["(fn #aaaaaa main (_ _) (: (-> (Caps Caps) (Result unit unit))) (Ok ()))"],
    ));
    assert_eq!((rule.as_str(), path.as_str()), ("TY001", "3.0.0"));
    let caps = module("f", &["(fn #aaaaaa f (_) (: (-> (Caps) i64)) 1)"]);
    assert_eq!(
        one(&caps),
        (
            "TY001".into(),
            "3.0.0.0".into(),
            "Caps is only allowed as the parameter of main".into()
        )
    );
    let result = module(
        "f",
        &["(fn #aaaaaa f () (: (-> () (Result unit unit))) (Ok ()))"],
    );
    assert_eq!(one(&result).0, "TY001");
    assert_eq!(one(&result).1, "3.0.1");
}

#[test]
fn ok_and_err_take_the_result_payload_types() {
    let main = |body: &str| {
        module(
            "main",
            &[&format!(
                "(fn #aaaaaa main (_) (: (-> (Caps) (Result unit bool))) {body})"
            )],
        )
    };
    clean(&main("(Err true)"));
    assert_eq!(
        one(&main("(Ok 1)")),
        (
            "TY001".into(),
            "4.1".into(),
            "expected unit, found i64".into()
        )
    );
    assert_eq!(
        one(&main("(Err 1)")),
        (
            "TY001".into(),
            "4.1".into(),
            "expected bool, found i64".into()
        )
    );
    let (rule, path, _) = one(&main("(let (r (Ok ())) r)"));
    assert_eq!((rule.as_str(), path.as_str()), ("TY001", "4.0.1"));
}

#[test]
fn skel_print_i64_is_typed_and_carries_log() {
    let g = |sig: &str, body: &str| module("g", &[&format!("(fn #aaaaaa g (x) (: {sig}) {body})")]);
    clean(&g("(-> (i64) unit log)", "(skel.print_i64 x)"));
    assert_eq!(
        one(&g("(-> (i64) unit)", "(skel.print_i64 x)")),
        (
            "FX001".into(),
            "4".into(),
            "this call needs the log effect, but the signature does not declare it".into()
        )
    );
    assert_eq!(
        one(&g("(-> (i64) unit log)", "(skel.print_i64 (== x 1))")).1,
        "4.1"
    );
    assert_eq!(
        one(&g("(-> (i64) i64 log)", "(skel.print_i64 x)")).2,
        "expected i64, found unit"
    );
}

#[test]
fn fx001_for_a_call_to_a_logging_function() {
    let src = module(
        "a b",
        &[
            "(fn #aaaaaa a (x) (: (-> (i64) unit)) (b x))",
            "(fn #aaaaab b (x) (: (-> (i64) unit log)) (skel.print_i64 x))",
        ],
    );
    let all = diags(&src);
    assert_eq!(
        all,
        [(
            "FX001".into(),
            "#aaaaaa".into(),
            "4".into(),
            "this call needs the log effect, but the signature does not declare it".into()
        )]
    );
}

#[test]
fn skl001_for_unsupported_types_and_effects() {
    let sig = |s: &str| module("f", &[&format!("(fn #aaaaaa f (_) (: {s}) 1)")]);
    assert_eq!(one(&sig("(-> (str) i64)")).1, "3.0.0.0");
    assert_eq!(one(&sig("(-> (i32) i64)")).0, "SKL001");
    assert_eq!(one(&sig("(-> ((Vec i64)) i64)")).1, "3.0.0.0");
    assert_eq!(one(&sig("(-> ((-> (i64) i64)) i64)")).1, "3.0.0.0");
    assert_eq!(
        one(&sig("(-> (i64) i64 fs)")),
        (
            "SKL001".into(),
            "3.0.2".into(),
            "effect fs is not yet supported; log is the only effect".into()
        )
    );
    assert_eq!(one(&sig("(-> (i64) i64 e)")).1, "3.0.2");
    assert_eq!(one(&sig("(-> (i64) i64 log log)")).1, "3.0.3");
    assert_eq!(
        one(&module("f", &["(fn #aaaaaa f (_) (: i64) 1)"])).1,
        "3.0"
    );
    let result = module(
        "main",
        &["(fn #aaaaaa main (_) (: (-> (Caps) (Result i64 unit))) (Ok 1))"],
    );
    assert_eq!(
        one(&result),
        (
            "SKL001".into(),
            "3.0.1.1".into(),
            "Result must have ok type unit in this toolchain build".into()
        )
    );
}

#[test]
fn skl001_for_shadowing_unused_and_unknown_names() {
    assert_eq!(
        one(&f_body("(let (x 1) x)")),
        (
            "SKL001".into(),
            "4.0.0".into(),
            "x is already bound in this definition".into()
        )
    );
    assert_eq!(
        one(&f_body("(let (y 1) x)")),
        (
            "SKL001".into(),
            "4.0.0".into(),
            "y is never used; name it _".into()
        )
    );
    assert_eq!(
        one(&module("f", &["(fn #aaaaaa f (y) (: (-> (i64) i64)) 1)"])).1,
        "2.0"
    );
    assert_eq!(
        one(&f_body("(+ x z)")),
        ("SKL001".into(), "4.2".into(), "unknown name z".into())
    );
    assert_eq!(one(&f_body("(skel.other x)")).1, "4.0");
    assert_eq!(one(&f_body("(other.g x)")).1, "4.0");
    assert_eq!(one(&f_body("(let (g f) (_ g) x)")).1, "4.0.1");
    assert_eq!(
        one(&f_body(
            "(? (== x 1) (true (let (y 1) y)) (false (let (y 2) y)))"
        ))
        .1,
        "4.2.1.0.0"
    );
}

#[test]
fn skl001_for_missing_signatures_and_exports() {
    let no_sig = module("f", &["(fn #aaaaaa f (x) x)"]);
    assert_eq!(
        one(&no_sig),
        (
            "SKL001".into(),
            "1".into(),
            "f has no signature; every function needs one".into()
        )
    );
    let not_exported = module("", &["(fn #aaaaaa f (x) (: (-> (i64) i64)) x)"]);
    assert_eq!(
        one(&not_exported),
        (
            "SKL001".into(),
            "1".into(),
            "f has a signature but is not exported".into()
        )
    );
    let unknown_export = module("f g", &["(fn #aaaaaa f (x) (: (-> (i64) i64)) x)"]);
    let (rule, path, message) = one(&unknown_export);
    assert_eq!((rule.as_str(), path.as_str()), ("SKL001", ""));
    assert!(message.contains('g'), "{message}");
}

#[test]
fn diagnostics_are_sorted_and_a_module_without_main_is_valid() {
    let src = module(
        "a b",
        &[
            "(fn #aaaaab a (x) (: (-> (i64) i64)) (+ true (+ x z)))",
            "(fn #aaaaaa b (_) (: (-> (i64) i64)) true)",
        ],
    );
    let all = diags(&src);
    let order: Vec<(&str, &str, &str)> = all
        .iter()
        .map(|(r, d, p, _)| (d.as_str(), p.as_str(), r.as_str()))
        .collect();
    assert_eq!(
        order,
        [
            ("#aaaaaa", "4", "TY001"),
            ("#aaaaab", "4.1", "TY001"),
            ("#aaaaab", "4.2.2", "SKL001")
        ]
    );
    assert_eq!(diags(&src), all, "deterministic");
    let typed = check("t.m", f_body("(* x 7)").as_bytes())
        .typed
        .expect("valid without main");
    assert!(!typed.has_main());
}

#[test]
fn parse_diagnostics_end_the_check() {
    let result = check(
        "t.m",
        b"(mod t.m)\n(pub f)\n(fn #aaaaaa f (x) (: (-> (i64) i64)) (\\ (y) y))\n",
    );
    assert_eq!(result.result.diagnostics.len(), 1);
    assert_eq!(result.result.diagnostics[0].rule.as_str(), "SKL001");
    assert!(result.typed.is_none());
}
