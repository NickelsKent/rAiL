//! Cranelift object generation for the host (BR5.1, BR5.4, NFR3.2).

use rail_codegen::{compile, host_target};

fn object(src: &str) -> Vec<u8> {
    let checked = rail_check::check("t.m", src.as_bytes());
    let typed = checked
        .typed
        .unwrap_or_else(|| panic!("{:?}", checked.result.diagnostics));
    compile(&rail_lower::lower(&typed), "t.m").expect("object file")
}

fn answer() -> Vec<u8> {
    let src = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/skeleton/answer.rlc"),
    )
    .unwrap();
    let typed = rail_check::check("skeleton.answer", &src).typed.unwrap();
    compile(&rail_lower::lower(&typed), "skeleton.answer").expect("object file")
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn the_host_target_is_one_of_the_two_skeleton_platforms() {
    let target = host_target().expect("supported host");
    let expected = if cfg!(target_os = "macos") {
        "aarch64-macos"
    } else {
        "x86_64-linux"
    };
    assert_eq!(target.name, expected);
}

#[test]
fn an_object_file_is_produced_for_the_answer_fixture() {
    let bytes = answer();
    if cfg!(target_os = "macos") {
        assert_eq!(&bytes[..4], &[0xcf, 0xfa, 0xed, 0xfe], "Mach-O 64-bit");
    } else {
        assert_eq!(&bytes[..4], b"\x7fELF");
    }
    for symbol in [&b"rail_entry"[..], b"rail_rt_print_i64", b"rail_rt_trap"] {
        assert!(
            contains(&bytes, symbol),
            "{}",
            String::from_utf8_lossy(symbol)
        );
    }
}

#[test]
fn the_same_input_gives_the_same_bytes() {
    assert_eq!(answer(), answer());
}

#[test]
fn trap_calls_appear_only_with_checked_arithmetic() {
    let checked = object("(mod t.m)\n(pub f)\n(fn #aaaaaa f (x) (: (-> (i64) i64)) (- x 1))\n");
    assert!(contains(&checked, b"rail_rt_trap"));
    let plain = object("(mod t.m)\n(pub f)\n(fn #aaaaaa f (x) (: (-> (i64) bool)) (< x 1))\n");
    assert!(!contains(&plain, b"rail_rt_trap"));
    assert!(!contains(&plain, b"rail_entry"), "no main, no entry point");
}

#[test]
fn every_form_compiles() {
    let src = "(mod t.m)\n(pub a b main)\n\
        (fn #aaaaaa a (x y) (: (-> (i64 i64) i64)) (let (q (/ x y)) (r (% x y)) (? (!= q r) (true (* q r)) (false (+ q (- r 1))))))\n\
        (fn #aaaaab b (x) (: (-> (bool) bool)) (? x (true (== 1 2)) (false (>= 3 (a 4 5)))))\n\
        (fn #aaaaac main (_) (: (-> (Caps) (Result unit i64) log)) (let (_ (skel.print_i64 (a 1 2))) (? (b false) (true (Ok ())) (false (Err 3)))))\n";
    let bytes = object(src);
    assert!(contains(&bytes, b"rail_entry"));
}
