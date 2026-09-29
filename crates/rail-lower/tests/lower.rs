//! Lowering to SSA IR with checked arithmetic (BR5.3, BR5.4, BR5.7).

use rail_lower::{ENTRY_SYMBOL, IrModule, lower};

fn ir(fns: &[&str], exports: &str) -> IrModule {
    let mut src = format!("(mod t.m)\n(pub {exports})\n");
    for f in fns {
        src.push_str(f);
        src.push('\n');
    }
    let checked = rail_check::check("t.m", src.as_bytes());
    let typed = checked
        .typed
        .unwrap_or_else(|| panic!("{src}\n{:?}", checked.result.diagnostics));
    lower(&typed)
}

fn one_fn(sig: &str, params: &str, body: &str) -> String {
    let module = ir(
        &[&format!("(fn #aaaaaa f ({params}) (: {sig}) {body})")],
        "f",
    );
    assert_eq!(module.functions.len(), 1);
    module.functions[0].render()
}

#[test]
fn literals_lower_to_constants() {
    assert_eq!(
        one_fn("(-> (i64) i64)", "_", "-7"),
        "function rail_f_aaaaaa(v0) local\nb0(v0):\n  v1 = iconst -7\n  return v1\n"
    );
    assert!(one_fn("(-> (i64) bool)", "_", "true").contains("v1 = iconst 1\n"));
    assert!(one_fn("(-> (i64) bool)", "_", "false").contains("v1 = iconst 0\n"));
    assert!(one_fn("(-> (i64) unit)", "_", "()").contains("v1 = iconst 0\n"));
}

#[test]
fn multiplication_is_checked_and_branches_to_the_trap() {
    assert_eq!(
        one_fn("(-> (i64) i64)", "x", "(* x 7)"),
        "function rail_f_aaaaaa(v0) local\n\
         b0(v0):\n  v1 = iconst 7\n  v2 = overflows mul v0, v1\n  brif v2, b1(), b2()\n\
         b1():\n  trap overflow\n\
         b2():\n  v3 = mul v0, v1\n  return v3\n"
    );
    for (op, name) in [("+", "add"), ("-", "sub")] {
        let text = one_fn("(-> (i64) i64)", "x", &format!("({op} x 1)"));
        assert!(text.contains(&format!("overflows {name} v0, v1")), "{text}");
        assert!(text.contains("trap overflow"), "{text}");
    }
}

#[test]
fn division_and_remainder_check_zero_and_overflow() {
    for (op, name) in [("/", "div"), ("%", "rem")] {
        let text = one_fn("(-> (i64) i64)", "x", &format!("({op} 100 x)"));
        assert!(text.contains("v2 = iconst 0\n  v3 = eq v0, v2\n"), "{text}");
        assert!(text.contains("trap div_by_zero"), "{text}");
        assert!(text.contains("overflows div v1, v0"), "{text}");
        assert!(text.contains(&format!("= {name} v1, v0")), "{text}");
    }
}

#[test]
fn comparisons_are_not_checked() {
    let text = one_fn("(-> (i64) bool)", "x", "(<= x 3)");
    assert!(text.contains("v2 = le v0, v1\n  return v2\n"), "{text}");
    assert!(!text.contains("trap"), "{text}");
}

#[test]
fn calls_and_let_bindings() {
    let module = ir(
        &[
            "(fn #aaaaab a (x) (: (-> (i64) i64)) (let (y (b x)) (_ (b y)) y))",
            "(fn #aaaaaa b (x) (: (-> (i64) i64)) x)",
        ],
        "a b",
    );
    let a = module.functions[0].render();
    assert_eq!(
        a,
        "function rail_f_aaaaab(v0) local\nb0(v0):\n  v1 = call rail_f_aaaaaa(v0)\n  v2 = call rail_f_aaaaaa(v1)\n  return v1\n"
    );
}

#[test]
fn the_boolean_match_branches_to_a_join_block() {
    let text = one_fn("(-> (i64) i64)", "x", "(? (== x 0) (true 1) (false 2))");
    assert_eq!(
        text,
        "function rail_f_aaaaaa(v0) local\n\
         b0(v0):\n  v1 = iconst 0\n  v2 = eq v0, v1\n  brif v2, b1(), b2()\n\
         b1():\n  v3 = iconst 1\n  jump b3(v3)\n\
         b2():\n  v4 = iconst 2\n  jump b3(v4)\n\
         b3(v5):\n  return v5\n"
    );
}

#[test]
fn main_gets_an_entry_wrapper_and_print_calls_the_runtime() {
    let module = ir(
        &[
            "(fn #aaaaaa main (_) (: (-> (Caps) (Result unit unit) log)) (let (_ (skel.print_i64 5)) (Err ())))",
        ],
        "main",
    );
    assert_eq!(module.functions.len(), 2);
    let main = module.functions[0].render();
    assert!(main.contains("call rail_rt_print_i64(v1)"), "{main}");
    assert!(main.contains("iconst 1\n  return"), "Err is tag 1: {main}");
    let entry = module.functions[1].render();
    assert_eq!(
        entry,
        format!(
            "function {ENTRY_SYMBOL}() export\nb0():\n  v0 = iconst 0\n  v1 = call rail_f_aaaaaa(v0)\n  return v1\n"
        )
    );
}

#[test]
fn lowering_is_deterministic() {
    let src = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/skeleton/answer.rlc"),
    )
    .unwrap();
    let render = || {
        let typed = rail_check::check("skeleton.answer", &src).typed.unwrap();
        lower(&typed).render()
    };
    let first = render();
    assert_eq!(first, render());
    assert!(first.contains("function rail_entry() export"));
}
