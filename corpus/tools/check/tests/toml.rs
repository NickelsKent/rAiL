//! The std-only TOML reader (the corpus adds no crates to the checker).

use corpus_check::toml::{Value, parse};

#[test]
fn reads_scalars_tables_arrays_and_comments() {
    let doc = parse(
        "# a comment\nname = \"rail\" # trailing\ncount = 7\nneg = -3\non = true\n\
         list = [\"a\", \"b\"]\nnums = [1, 2]\n\n[sandbox]\nmin = \"0.6.0\"\n\n[a.b]\nx = 1\n",
    )
    .expect("parses");
    assert_eq!(doc.get("name").and_then(Value::as_str), Some("rail"));
    assert_eq!(doc.get("count").and_then(Value::as_int), Some(7));
    assert_eq!(doc.get("neg").and_then(Value::as_int), Some(-3));
    assert_eq!(doc.get("on").and_then(Value::as_bool), Some(true));
    let list = doc.get("list").and_then(Value::as_array).unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[1].as_str(), Some("b"));
    assert_eq!(
        doc.get("sandbox")
            .and_then(|t| t.get("min"))
            .and_then(Value::as_str),
        Some("0.6.0")
    );
    let x = doc
        .get("a")
        .and_then(|a| a.get("b"))
        .and_then(|b| b.get("x"));
    assert_eq!(x.and_then(Value::as_int), Some(1));
}

#[test]
fn reads_arrays_of_tables_and_multiline_arrays() {
    let doc = parse(
        "[[entries]]\nid = \"one\"\nitems = [\n  \"x\", # first\n  \"y\",\n]\n\n[[entries]]\nid = \"two\"\n",
    )
    .expect("parses");
    let entries = doc.get("entries").and_then(Value::as_array).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].get("id").and_then(Value::as_str), Some("one"));
    let items = entries[0].get("items").and_then(Value::as_array).unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(entries[1].get("id").and_then(Value::as_str), Some("two"));
}

#[test]
fn reads_inline_tables_and_string_escapes() {
    let doc = parse("dep = { version = \"1.0\", optional = true }\ns = \"a\\\"b\\\\c\\n\"\n")
        .expect("parses");
    let dep = doc.get("dep").unwrap();
    assert_eq!(dep.get("version").and_then(Value::as_str), Some("1.0"));
    assert_eq!(dep.get("optional").and_then(Value::as_bool), Some(true));
    assert_eq!(doc.get("s").and_then(Value::as_str), Some("a\"b\\c\n"));
}

#[test]
fn errors_name_the_line() {
    for (text, line) in [
        ("ok = 1\nbroken line\n", 2),
        ("a = 1\na = 2\n", 2),
        ("s = \"unterminated\n", 1),
        ("[tbl\nx = 1\n", 1),
    ] {
        let err = parse(text).expect_err(text);
        assert_eq!(err.line, line, "{text:?}: {err}");
    }
}
