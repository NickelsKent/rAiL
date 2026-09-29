//! The protocol server, driven in process over byte buffers (E1, BR6.3–BR6.5,
//! NFR3.4, NFR4.4, NFR9.6).

use rail_json::Value;
use rail_rap::serve;
use rail_testkit::{TempDir, copy_dir, frame, repo_root};

/// Runs a whole session over `input`; returns the parsed responses and the
/// log records, and checks that the output holds nothing but frames.
fn session(input: &[u8]) -> (Vec<Value>, Vec<String>) {
    let mut output = Vec::new();
    let mut records = Vec::new();
    serve(input, &mut output, &mut |record| {
        records.push(record.line())
    })
    .expect("server ends cleanly");
    (frames(&output), records)
}

fn frames(mut output: &[u8]) -> Vec<Value> {
    let mut out = Vec::new();
    while !output.is_empty() {
        let text = std::str::from_utf8(output).expect("UTF-8 output");
        let rest = text
            .strip_prefix("Content-Length: ")
            .unwrap_or_else(|| panic!("not a frame: {text:?}"));
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        let length: usize = digits.parse().unwrap();
        let header = "Content-Length: ".len() + digits.len() + 4;
        assert_eq!(&output[header - 4..header], b"\r\n\r\n");
        out.push(rail_json::parse(&output[header..header + length]).expect("JSON body"));
        output = &output[header + length..];
    }
    out
}

fn workspace() -> TempDir {
    let dir = TempDir::new("rap");
    copy_dir(&repo_root().join("fixtures"), dir.path()).unwrap();
    dir
}

fn init(ws: &TempDir) -> Vec<u8> {
    let root = rail_json::to_string(&Value::str(&ws.path().display().to_string()));
    frame(format!(r#"{{"jsonrpc":"2.0","id":0,"method":"initialize","params":{{"client_versions":["0.1"],"workspace_root":{root}}}}}"#).as_bytes())
}

fn req(id: i64, method: &str, params: &str) -> Vec<u8> {
    frame(
        format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"{method}","params":{params}}}"#)
            .as_bytes(),
    )
}

const ANSWER: &str = r#"{"module":"skeleton/answer.rlc"}"#;

fn code(response: &Value) -> Option<i64> {
    response.get("error")?.get("code")?.as_i64()
}

fn tool_code(response: &Value) -> Option<&str> {
    response.get("error")?.get("data")?.get("code")?.as_str()
}

fn check_ok(response: &Value, id: i64) {
    assert_eq!(
        response.get("id").and_then(Value::as_i64),
        Some(id),
        "{response:?}"
    );
    assert_eq!(
        response.get("result").map(rail_json::to_string).as_deref(),
        Some(r#"{"module":"skeleton.answer","diagnostics":[],"blocking":false}"#)
    );
}

#[test]
fn a_framed_session_answers_initialize_and_check() {
    let ws = workspace();
    let mut input = init(&ws);
    input.extend(req(1, "check.run", ANSWER));
    let (responses, _) = session(&input);
    assert_eq!(responses.len(), 2);
    assert_eq!(
        rail_json::to_string(&responses[0]),
        r#"{"jsonrpc":"2.0","id":0,"result":{"rap_version":"0.1","capabilities":["tree.get","check.run","build.run","run.run"]}}"#
    );
    check_ok(&responses[1], 1);
}

#[test]
fn bad_framing_is_invalid_request_and_the_session_recovers() {
    let ws = workspace();
    let body = br#"{"jsonrpc":"2.0","id":9,"method":"check.run","params":{"module":"skeleton/answer.rlc"}}"#;
    let mut input = init(&ws);
    input.extend(b"Content-Type: application/json\r\n\r\n");
    // After a rejected header the reader resynchronises at the next blank
    // line, so these headers are sent without bodies.
    input.extend(b"Content-Length: 5\r\nContent-Length: 5\r\n\r\n");
    input.extend(b"Content-Length: 12x\r\n\r\n");
    input.extend(b"X-Other: 1\r\nContent-Length: 5\r\n\r\n");
    input.extend(format!("Content-Length: 1\r\n{}\r\n\r\n", "x".repeat(2000)).as_bytes());
    input.extend(
        format!(
            "Content-Type: application/json\r\nContent-Length: {}\r\n\r\n",
            body.len()
        )
        .as_bytes(),
    );
    input.extend(body);
    let (responses, _) = session(&input);
    let codes: Vec<Option<i64>> = responses.iter().map(code).collect();
    assert_eq!(
        codes,
        [
            None,
            Some(-32600),
            Some(-32600),
            Some(-32600),
            Some(-32600),
            Some(-32600),
            None
        ]
    );
    assert!(
        responses[1..6]
            .iter()
            .all(|r| r.get("id").is_some_and(Value::is_null))
    );
    check_ok(&responses[6], 9);
}

#[test]
fn an_oversized_body_is_discarded_and_framing_recovers() {
    let ws = workspace();
    let mut input = init(&ws);
    let size = 16 * 1024 * 1024 + 1;
    input.extend(format!("Content-Length: {size}\r\n\r\n").as_bytes());
    input.extend(std::iter::repeat_n(b' ', size));
    input.extend(req(1, "check.run", ANSWER));
    let (responses, _) = session(&input);
    assert_eq!(responses.len(), 3);
    assert_eq!(code(&responses[1]), Some(-32600));
    check_ok(&responses[2], 1);
}

#[test]
fn json_rpc_errors_use_the_standard_codes() {
    let ws = workspace();
    let mut input = init(&ws);
    for body in [
        "{\"jsonrpc\":\"2.0\",\"id\":1,",
        "[1,2]",
        r#"{"jsonrpc":"1.0","id":2,"method":"check.run"}"#,
        r#"{"jsonrpc":"2.0","id":3,"method":7}"#,
        r#"{"jsonrpc":"2.0","id":1.5,"method":"check.run"}"#,
        r#"{"jsonrpc":"2.0","id":4,"method":"fmt.apply","params":{}}"#,
        r#"{"jsonrpc":"2.0","id":5,"method":"check.run"}"#,
        r#"{"jsonrpc":"2.0","id":6,"method":"check.run","params":{}}"#,
        r#"{"jsonrpc":"2.0","id":7,"method":"check.run","params":{"module":3}}"#,
        r#"{"jsonrpc":"2.0","id":"s","method":"check.run","params":[]}"#,
    ] {
        input.extend(frame(body.as_bytes()));
    }
    let (responses, _) = session(&input);
    let got: Vec<(Option<i64>, String)> = responses[1..]
        .iter()
        .map(|r| (code(r), rail_json::to_string(r.get("id").unwrap())))
        .collect();
    let expected: Vec<(Option<i64>, String)> = [
        (-32700, "null"),
        (-32600, "null"),
        (-32600, "2"),
        (-32600, "3"),
        (-32600, "null"),
        (-32601, "4"),
        (-32602, "5"),
        (-32602, "6"),
        (-32602, "7"),
        (-32602, "\"s\""),
    ]
    .into_iter()
    .map(|(c, id)| (Some(c), id.to_string()))
    .collect();
    assert_eq!(got, expected);
}

#[test]
fn unsupported_params_are_refused_not_ignored() {
    let ws = workspace();
    let mut input = init(&ws);
    input.extend(req(
        1,
        "check.run",
        r#"{"module":"skeleton/answer.rlc","depth":2}"#,
    ));
    input.extend(req(
        2,
        "run.run",
        r#"{"module":"skeleton/answer.rlc","_deadline_ms":100}"#,
    ));
    input.extend(req(
        3,
        "build.run",
        r#"{"module":"skeleton/answer.rlc","target":"wasm"}"#,
    ));
    input.extend(req(
        4,
        "tree.get",
        r#"{"module":"skeleton/answer.rlc","surprise":true}"#,
    ));
    let (responses, _) = session(&input);
    for r in &responses[1..] {
        assert_eq!(code(r), Some(-32000), "{r:?}");
        assert_eq!(tool_code(r), Some("rap.unsupported_param"));
    }
    let message = responses[1]
        .get("error")
        .and_then(|e| e.get("message"))
        .and_then(Value::as_str)
        .unwrap();
    assert!(message.contains("depth"), "{message}");
}

#[test]
fn initialize_comes_first_exactly_once() {
    let ws = workspace();
    let mut input = req(1, "check.run", ANSWER);
    input.extend(frame(br#"{"jsonrpc":"2.0","id":2,"method":"initialize","params":{"client_versions":["9.9"],"workspace_root":"/"}}"#));
    input.extend(frame(
        br#"{"jsonrpc":"2.0","id":3,"method":"initialize","params":{"client_versions":["0.1"]}}"#,
    ));
    input.extend(init(&ws));
    input.extend(init(&ws));
    input.extend(req(4, "check.run", ANSWER));
    let (responses, _) = session(&input);
    assert_eq!(code(&responses[0]), Some(-32000));
    assert_eq!(tool_code(&responses[0]), Some("rap.not_initialized"));
    assert_eq!(tool_code(&responses[1]), Some("rap.unsupported_param"));
    assert_eq!(code(&responses[2]), Some(-32602));
    assert!(responses[3].get("result").is_some());
    assert_eq!(
        code(&responses[4]),
        Some(-32600),
        "a second initialize is invalid"
    );
    check_ok(&responses[5], 4);
}

#[test]
fn notifications_get_no_response() {
    let ws = workspace();
    let mut input = init(&ws);
    input.extend(frame(
        br#"{"jsonrpc":"2.0","method":"$/cancelRequest","params":{"id":1}}"#,
    ));
    input.extend(frame(
        br#"{"jsonrpc":"2.0","method":"check.run","params":{"module":"skeleton/answer.rlc"}}"#,
    ));
    input.extend(req(1, "check.run", ANSWER));
    let (responses, _) = session(&input);
    assert_eq!(responses.len(), 2);
    check_ok(&responses[1], 1);
}

#[test]
fn tool_failures_are_minus_32000_and_the_next_request_succeeds() {
    let ws = workspace();
    let mut input = init(&ws);
    input.extend(req(1, "check.run", r#"{"module":"skeleton/missing.rlc"}"#));
    input.extend(req(2, "build.run", r#"{"module":"skeleton/broken.rlc"}"#));
    input.extend(req(3, "check.run", ANSWER));
    let (responses, records) = session(&input);
    assert_eq!(tool_code(&responses[1]), Some("module.not_found"));
    assert_eq!(tool_code(&responses[2]), Some("build.blocked"));
    let data = responses[2]
        .get("error")
        .and_then(|e| e.get("data"))
        .and_then(|d| d.get("data"))
        .unwrap();
    assert_eq!(data.get("blocking").and_then(Value::as_bool), Some(true));
    check_ok(&responses[3], 3);
    assert!(records.iter().any(|r| r == "rail error check.run skeleton/missing.rlc code=module.not_found module skeleton/missing.rlc not found in the workspace"), "{records:?}");
    let root = ws.path().display().to_string();
    assert!(records.iter().all(|r| !r.contains(&root)), "{records:?}");
}

#[test]
fn responses_come_back_in_request_order() {
    let ws = workspace();
    let mut input = init(&ws);
    for id in 1..=5 {
        let method = if id % 2 == 0 { "tree.get" } else { "check.run" };
        input.extend(req(id, method, ANSWER));
    }
    let (responses, _) = session(&input);
    let ids: Vec<Option<i64>> = responses
        .iter()
        .map(|r| r.get("id").and_then(Value::as_i64))
        .collect();
    assert_eq!(ids, [Some(0), Some(1), Some(2), Some(3), Some(4), Some(5)]);
}

#[test]
fn run_output_stays_inside_the_result() {
    let ws = workspace();
    let mut input = init(&ws);
    input.extend(req(1, "run.run", ANSWER));
    let (responses, _) = session(&input);
    assert_eq!(
        responses[1]
            .get("result")
            .map(rail_json::to_string)
            .as_deref(),
        Some(r#"{"exit_code":0,"stdout":"42\n","stderr":""}"#)
    );
}

#[test]
fn an_empty_or_truncated_input_ends_the_session() {
    assert_eq!(session(b"").0.len(), 0);
    assert_eq!(session(b"Content-Length: 10\r\n\r\n{").0.len(), 0);
    assert_eq!(session(b"Content-Len").0.len(), 0);
}
