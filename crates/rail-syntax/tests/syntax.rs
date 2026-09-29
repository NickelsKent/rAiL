//! Canonical-text parser and printer for the skeleton form set
//! (BR1.1–BR1.6, BR2.4, BR2.6, NFR4.4).

use rail_syntax::{ExprKind, MAX_NESTING, Module, Pattern, parse, print};

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/skeleton")
        .join(name);
    std::fs::read(path).expect("fixture")
}

/// A one-function module `t.m` whose `f` has the given body.
fn with_body(body: &str) -> String {
    format!("(mod t.m)\n(pub f)\n(fn #aaaaaa f (x) (: (-> (i64) i64)) {body})\n")
}

fn ok(src: &str) -> Module {
    parse("t.m", src.as_bytes()).unwrap_or_else(|d| panic!("rejected {src:?}: {}", d.render()))
}

/// Parses `src` as module `t.m`, expecting SKL001; returns (span start, message).
fn skl(src: &str) -> (usize, String) {
    skl_as("t.m", src)
}

fn skl_as(qname: &str, src: &str) -> (usize, String) {
    match parse(qname, src.as_bytes()) {
        Ok(_) => panic!("accepted {src:?}"),
        Err(d) => {
            assert_eq!(d.rule.as_str(), "SKL001", "{}", d.render());
            assert_eq!(d.loc.module, qname);
            (d.loc.span.start, d.message)
        }
    }
}

#[test]
fn both_fixtures_parse_and_print_back_byte_for_byte() {
    for (qname, file) in [
        ("skeleton.answer", "answer.rlc"),
        ("skeleton.broken", "broken.rlc"),
    ] {
        let bytes = fixture(file);
        let module = parse(qname, &bytes).expect("fixture parses");
        assert_eq!(module.qname, qname);
        assert_eq!(print(&module).as_bytes(), bytes.as_slice(), "{file}");
        let names: Vec<&str> = module.fns.iter().map(|f| f.name.text.as_str()).collect();
        assert_eq!(names, ["answer", "main"]);
        let ids: Vec<&str> = module.fns.iter().map(|f| f.id.text.as_str()).collect();
        assert_eq!(ids, ["#sk0a1b", "#sk0m4n"]);
        let exports: Vec<&str> = module.exports.iter().map(|n| n.text.as_str()).collect();
        assert_eq!(exports, ["answer", "main"]);
    }
}

#[test]
fn nodes_carry_anchor_paths_and_spans() {
    let bytes = fixture("broken.rlc");
    let module = parse("skeleton.broken", &bytes).unwrap();
    let answer = &module.fns[0];
    assert_eq!(answer.body.path, "4");
    assert_eq!((answer.body.span.start, answer.body.span.end), (82, 90));
    assert!(matches!(&answer.params[0], Pattern::Name(n) if n.text == "x"));
    let ExprKind::Apply { head, args } = &answer.body.kind else {
        panic!("application expected");
    };
    assert_eq!(head.path, "4.0");
    assert!(matches!(&head.kind, ExprKind::Ref(name) if name == "=="));
    assert_eq!(args[1].path, "4.2");
    assert!(matches!(args[1].kind, ExprKind::Int(7)));

    let main = &module.fns[1];
    let ExprKind::Let { bindings, body } = &main.body.kind else {
        panic!("let expected");
    };
    assert_eq!(bindings[0].value.path, "4.0.1");
    assert!(matches!(&bindings[1].pattern, Pattern::Wildcard(_)));
    let ExprKind::Match { on_false, .. } = &body.kind else {
        panic!("match expected");
    };
    assert_eq!(on_false.path, "4.2.2.1");
    assert_eq!(&bytes[on_false.span.start..on_false.span.end], b"(Err ())");
}

#[test]
fn unsupported_forms_are_skl001_at_the_form() {
    for body in [
        "(\\ (y) y)",
        "(. x f)",
        "(! x)",
        "(, x x)",
        "(@ x)",
        "(: x i64)",
        "(upd x (a 1))",
        "\"text\"",
        "1.5",
        "((f x) 1)",
    ] {
        let src = with_body(body);
        let (at, message) = skl(&src);
        assert_eq!(at, src.find(body).unwrap(), "{body}: {message}");
        assert!(message.contains("not yet supported"), "{body}: {message}");
    }
    for item in [
        "(use a.b)",
        "(val #aaaaab v 1)",
        "(typ #aaaaab T () (rec (a i64)))",
        "(tst #aaaaab t (Ok ()))",
        "(meta #aaaaaa doc \"d\")",
    ] {
        let src = format!("(mod t.m)\n{item}\n");
        let (at, message) = skl(&src);
        assert_eq!(at, 10, "{item}");
        assert!(message.contains("not yet supported"), "{item}: {message}");
    }
}

#[test]
fn layout_violations_are_skl001_at_the_first_offending_byte() {
    let good = with_body("(* x 7)");
    assert_eq!(print(&ok(&good)), good);
    let cases: Vec<(String, usize)> = vec![
        (good.replacen('\n', "\r\n", 1), 9),
        (good.trim_end_matches('\n').to_string(), good.len() - 1),
        (format!("\u{feff}{good}"), 0),
        (good.replacen("(pub f)", "(pub  f)", 1), 14),
        (
            good.replacen("(* x 7)", "( * x 7)", 1),
            good.find("(* x").unwrap() + 1,
        ),
        (
            good.replacen("(* x 7)", "(* x 7 )", 1),
            good.find("(* x").unwrap() + 6,
        ),
        (good.replacen("(pub f)", "(pub\tf)", 1), 14),
        (good.replacen("(pub f)\n", "(pub f)\n\n", 1), 18),
        (good.replacen("(pub f)\n", "(pub f) ", 1), 17),
        (good.replacen("(* x 7)", "(* x 7) ", 1), good.len() - 2),
        (good.replacen("7", "\u{e9}", 1), good.find('7').unwrap()),
    ];
    for (src, at) in cases {
        let (found, message) = skl(&src);
        assert_eq!(found, at, "{src:?}: {message}");
    }
}

#[test]
fn items_must_be_in_canonical_order() {
    let two = "(mod t.m)\n(pub a b)\n(fn #aaaaaa b () (: (-> () i64)) 1)\n(fn #aaaaab a () (: (-> () i64)) 2)\n";
    let (at, message) = skl(two);
    assert_eq!(at, two.find("(fn #aaaaab").unwrap(), "{message}");
    let unsorted_pub = "(mod t.m)\n(pub b a)\n";
    assert_eq!(skl(unsorted_pub).0, 17);
    let pub_after_fn = "(mod t.m)\n(fn #aaaaaa f () (: (-> () i64)) 1)\n(pub f)\n";
    assert_eq!(skl(pub_after_fn).0, pub_after_fn.find("(pub").unwrap());
    let no_mod = "(pub f)\n";
    assert_eq!(skl(no_mod).0, 0);
}

#[test]
fn malformed_input_fails_at_a_precise_location() {
    let good = with_body("(* x 7)");
    let missing_close = good.replacen("(* x 7))", "(* x 7)", 1);
    assert_eq!(skl(&missing_close).0, missing_close.len() - 1);
    let extra_close = good.replacen("(* x 7))", "(* x 7)))", 1);
    assert_eq!(skl(&extra_close).0, extra_close.len() - 2);
    let truncated = &good[..good.len() - 5];
    assert!(parse("t.m", truncated.as_bytes()).is_err());
    let bad_name = with_body("(* X-1 7)");
    assert_eq!(skl(&bad_name).0, bad_name.find("X-1").unwrap());
    assert_eq!(skl("").0, 0);
}

#[test]
fn definition_ids_are_well_formed_and_unique() {
    let good = with_body("x");
    for (bad, at_id) in [
        (good.replacen("#aaaaaa ", "", 1), false),
        (good.replacen("#aaaaaa", "#ABCDEF", 1), true),
        (good.replacen("#aaaaaa", "#aaaaa", 1), true),
        (good.replacen("#aaaaaa", "#aaaail", 1), true),
    ] {
        let (at, message) = skl(&bad);
        let expected = if at_id {
            bad.find('#').unwrap()
        } else {
            bad.find("f (x)").unwrap()
        };
        assert_eq!(at, expected, "{bad:?}: {message}");
    }
    let dup = "(mod t.m)\n(pub a b)\n(fn #aaaaaa a () (: (-> () i64)) 1)\n(fn #aaaaaa b () (: (-> () i64)) 2)\n";
    let (at, message) = skl(dup);
    assert_eq!(at, dup.rfind("#aaaaaa").unwrap());
    assert!(message.contains("duplicate"), "{message}");
}

#[test]
fn the_skel_module_name_is_reserved_and_must_match_the_path() {
    assert!(parse("skeleton.answer", &fixture("answer.rlc")).is_ok());
    assert_eq!(skl_as("skel", "(mod skel)\n").0, 5);
    assert_eq!(skl_as("skel.x", "(mod skel.x)\n").0, 5);
    let (at, message) = skl_as("t.m", "(mod t.other)\n");
    assert_eq!(at, 5);
    assert!(message.contains("t.m"), "{message}");
}

#[test]
fn nesting_deeper_than_256_is_rejected() {
    assert_eq!(MAX_NESTING, 256);
    // The fn item itself is level 1, so k nested subtractions reach 1 + k.
    let nested = |k: usize| with_body(&format!("{}x{}", "(- ".repeat(k), " 1)".repeat(k)));
    assert!(parse("t.m", nested(255).as_bytes()).is_ok());
    let deep = nested(256);
    let (at, message) = skl(&deep);
    assert_eq!(at, deep.find("(- ").unwrap() + 3 * 255);
    assert!(message.contains("256"), "{message}");
    // Far deeper input is rejected without exhausting the stack.
    assert!(parse("t.m", nested(100_000).as_bytes()).is_err());
}

#[test]
fn integer_literals_are_unsuffixed_canonical_i64() {
    assert!(matches!(
        ok(&with_body("-9223372036854775808")).fns[0].body.kind,
        ExprKind::Int(i64::MIN)
    ));
    for literal in [
        "7i32",
        "7u8",
        "9223372036854775808",
        "-9223372036854775809",
        "007",
        "-0",
        "1x",
    ] {
        let src = with_body(literal);
        let (at, message) = skl(&src);
        assert_eq!(at, src.find(literal).unwrap(), "{literal}: {message}");
    }
}

#[test]
fn let_and_match_have_their_canonical_shapes() {
    ok(&with_body(
        "(let (y x) (_ y) (? (== y 1) (true y) (false 0)))",
    ));
    for body in [
        "(? (== x 1) (false 1) (true 2))",
        "(? (== x 1) (true 1))",
        "(? (== x 1) (true 1) (false 2) (true 3))",
        "(let (y x) (let (z y) z))",
        "(let x)",
        "(let ((Ok y) x) y)",
        "(let (1 x) x)",
    ] {
        let src = with_body(body);
        let (_, message) = skl(&src);
        assert!(!message.is_empty(), "{body}");
    }
}
