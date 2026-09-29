//! rail-json reader and writer behaviour (NFR4.3, NFR4.4, NFR3.1).

use rail_json::{MAX_DEPTH, Number, Value, parse, to_string};

fn obj(fields: Vec<(&str, Value)>) -> Value {
    Value::Object(
        fields
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    )
}

#[test]
fn round_trip_preserves_values_and_field_order() {
    let text = r#"{"b":1,"a":[true,false,null,"x\n\"y\"",-0.5e3,{}],"c":{"z":[],"y":""}}"#;
    let value = parse(text.as_bytes()).expect("valid JSON");
    assert_eq!(to_string(&value), text);
    let keys: Vec<&str> = value
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, _)| k.as_str())
        .collect();
    assert_eq!(keys, ["b", "a", "c"]);
}

#[test]
fn accessors_navigate_a_parsed_value() {
    let value =
        parse(br#"{"id":7,"method":"check.run","ok":true,"none":null,"list":[1]}"#).unwrap();
    assert_eq!(value.get("id").and_then(Value::as_i64), Some(7));
    assert_eq!(
        value.get("method").and_then(Value::as_str),
        Some("check.run")
    );
    assert_eq!(value.get("ok").and_then(Value::as_bool), Some(true));
    assert!(value.get("none").is_some_and(Value::is_null));
    assert_eq!(
        value.get("list").and_then(Value::as_array).map(Vec::len),
        Some(1)
    );
    assert!(value.get("missing").is_none());
    assert_eq!(parse(b"1.5").unwrap().as_i64(), None);
    assert_eq!(parse(b"9223372036854775808").unwrap().as_i64(), None);
    assert_eq!(
        parse(b"-9223372036854775808").unwrap().as_i64(),
        Some(i64::MIN)
    );
}

#[test]
fn duplicate_keys_are_rejected() {
    let err = parse(br#"{"a":1,"b":2,"a":3}"#).unwrap_err();
    assert_eq!(err.offset, 13);
    assert!(parse(br#"{"a":{"a":1},"b":{"a":2}}"#).is_ok());
}

#[test]
fn trailing_data_is_rejected() {
    assert!(parse(b"{} {}").is_err());
    assert!(parse(b"1 x").is_err());
    assert!(parse(b" [1] \n").is_ok());
}

#[test]
fn invalid_utf8_is_rejected() {
    assert!(parse(b"\"\xff\"").is_err());
    assert!(parse(b"[\"\xc3\x28\"]").is_err());
    assert!(parse("\"\u{e9}\"".as_bytes()).is_ok());
    assert!(parse(br#""\ud800""#).is_err(), "lone surrogate escape");
    assert_eq!(
        parse(br#""\ud83d\ude00""#).unwrap().as_str(),
        Some("\u{1f600}")
    );
}

#[test]
fn nesting_depth_64_is_accepted_and_65_rejected() {
    assert_eq!(MAX_DEPTH, 64);
    let nested = |depth: usize| format!("{}{}", "[".repeat(depth), "]".repeat(depth));
    assert!(parse(nested(64).as_bytes()).is_ok());
    assert!(parse(nested(65).as_bytes()).is_err());
    let objects = format!("{}1{}", "{\"a\":".repeat(65), "}".repeat(65));
    assert!(parse(objects.as_bytes()).is_err());
    // Far deeper input fails cleanly instead of exhausting the stack.
    assert!(parse(nested(1_000_000).as_bytes()).is_err());
}

#[test]
fn writer_escapes_strings_and_keeps_declared_field_order() {
    let value = obj(vec![
        (
            "z",
            Value::String("tab\tquote\"back\\slash/\u{1}\u{7f}é".to_string()),
        ),
        ("a", Value::Number(Number::from_i64(-42))),
        (
            "m",
            Value::Number(Number::from_lexeme("1.0").expect("valid lexeme")),
        ),
        ("n", Value::Array(vec![Value::Null, Value::Bool(false)])),
    ]);
    assert_eq!(
        to_string(&value),
        "{\"z\":\"tab\\tquote\\\"back\\\\slash/\\u0001\u{7f}é\",\"a\":-42,\"m\":1.0,\"n\":[null,false]}"
    );
    assert!(Number::from_lexeme("01").is_none());
    assert!(Number::from_lexeme("1.").is_none());
    assert!(Number::from_lexeme("+1").is_none());
}

#[test]
fn errors_carry_a_byte_offset_and_a_message() {
    let err = parse(b"[1,,2]").unwrap_err();
    assert_eq!(err.offset, 3);
    assert!(!err.to_string().is_empty());
    assert!(parse(b"").is_err());
}
