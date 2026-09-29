//! Fuzz target: any input the JSON parser accepts must survive a write and
//! a re-parse unchanged, and no input may panic or hang the parser.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(value) = rail_json::parse(data) {
        let written = rail_json::to_string(&value);
        let again = rail_json::parse(written.as_bytes()).expect("written JSON parses");
        assert_eq!(value, again, "round trip changed the value");
    }
});
