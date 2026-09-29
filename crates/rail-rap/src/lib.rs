//! The rAiL Agent Protocol server: JSON-RPC 2.0 over stdio with framing.
//!
//! Building block: RapServer (contract E1). Owning unit: U1 walking-skeleton
//! (thin first version; completed by U5 agent-loop).
//!
//! One thread reads a frame, handles it and writes the response before
//! reading the next, so responses come back in request order (NFR3.4). Each
//! message passes the layered checks of the security design (header, size,
//! JSON, request shape, method, session, parameter shape, parameter support);
//! the first failure answers that message with a structured error and the
//! session carries on (BR6.4, NFR9.6). Only protocol frames are written to
//! the output (BR6.5).

#![forbid(unsafe_code)]
#![deny(clippy::print_stdout, clippy::print_stderr)]

use std::io::{self, BufRead, BufReader, ErrorKind, Read, Write};

use rail_json::Value;
use rail_tools::{ErrorCode, LogLevel, Operation, Record, ToolError, Workspace, invoke_logged};

/// The protocol version this server speaks.
pub const RAP_VERSION: &str = "0.1";

/// Largest header block, in bytes.
pub const MAX_HEADER_BYTES: usize = 1024;

/// Largest message body, in bytes.
pub const MAX_BODY_BYTES: usize = 16 * 1024 * 1024;

/// Chunk size for discarding an oversized body.
const DISCARD_CHUNK: usize = 64 * 1024;

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const TOOL_ERROR: i64 = -32000;

/// Serves one session: reads framed requests from `input` until it ends and
/// writes framed responses to `output`. Log records go to `log`.
pub fn serve<R: Read, W: Write>(
    input: R,
    mut output: W,
    log: &mut dyn FnMut(Record),
) -> io::Result<()> {
    let mut reader = BufReader::new(input);
    let mut session = Session { workspace: None };
    loop {
        let response = match read_frame(&mut reader) {
            Frame::End => return Ok(()),
            Frame::Invalid(message) => {
                log_protocol(log, &message);
                Some(error(Value::Null, INVALID_REQUEST, message, None))
            }
            Frame::Body(bytes) => session.handle(&bytes, log),
        };
        if let Some(response) = response {
            let body = rail_json::to_string(&response);
            write!(output, "Content-Length: {}\r\n\r\n", body.len())?;
            output.write_all(body.as_bytes())?;
            output.flush()?;
        }
    }
}

fn log_protocol(log: &mut dyn FnMut(Record), message: &str) {
    log(Record {
        level: LogLevel::Error,
        operation: "rap".to_string(),
        module: "-".to_string(),
        code: None,
        message: message.to_string(),
    });
}

fn error(id: Value, code: i64, message: impl Into<String>, data: Option<Value>) -> Value {
    let mut fields = vec![
        ("code", Value::int(code)),
        ("message", Value::String(message.into())),
    ];
    if let Some(data) = data {
        fields.push(("data", data));
    }
    Value::object([
        ("jsonrpc", Value::str("2.0")),
        ("id", id),
        ("error", Value::object(fields)),
    ])
}

fn tool_error(id: Value, e: &ToolError) -> Value {
    error(id, TOOL_ERROR, e.message.clone(), Some(e.to_json()))
}

fn result(id: Value, result: Value) -> Value {
    Value::object([
        ("jsonrpc", Value::str("2.0")),
        ("id", id),
        ("result", result),
    ])
}

enum Frame {
    /// The input ended (cleanly or inside a message).
    End,
    /// A framing error; the reader has resynchronised.
    Invalid(String),
    /// A message body.
    Body(Vec<u8>),
}

fn read_byte<R: BufRead>(reader: &mut R) -> Option<u8> {
    loop {
        match reader.fill_buf() {
            Ok([]) => return None,
            Ok(buffer) => {
                let byte = buffer[0];
                reader.consume(1);
                return Some(byte);
            }
            Err(e) if e.kind() == ErrorKind::Interrupted => {}
            // A broken input is the end of the session.
            Err(_) => return None,
        }
    }
}

/// Reads one frame (layers 1 and 2: header and size).
fn read_frame<R: BufRead>(reader: &mut R) -> Frame {
    let mut header = Vec::new();
    let mut last = [0u8; 4];
    let mut too_long = false;
    while last != *b"\r\n\r\n" {
        let Some(byte) = read_byte(reader) else {
            return Frame::End;
        };
        last = [last[1], last[2], last[3], byte];
        if header.len() < MAX_HEADER_BYTES {
            header.push(byte);
        } else {
            // Keep discarding up to the next blank line to resynchronise.
            too_long = true;
        }
    }
    if too_long {
        return Frame::Invalid("invalid request: header block larger than 1 KiB".to_string());
    }
    let length = match content_length(&header) {
        Ok(length) => length,
        Err(message) => return Frame::Invalid(format!("invalid request: {message}")),
    };
    if length > MAX_BODY_BYTES as u64 {
        let mut remaining = length;
        let mut chunk = vec![0u8; DISCARD_CHUNK];
        while remaining > 0 {
            let want =
                usize::try_from(remaining.min(DISCARD_CHUNK as u64)).unwrap_or(DISCARD_CHUNK);
            if reader.read_exact(&mut chunk[..want]).is_err() {
                return Frame::End;
            }
            remaining -= want as u64;
        }
        return Frame::Invalid("invalid request: message body larger than 16 MiB".to_string());
    }
    let mut body = vec![0u8; usize::try_from(length).unwrap_or(0)];
    match reader.read_exact(&mut body) {
        Ok(()) => Frame::Body(body),
        Err(_) => Frame::End,
    }
}

/// Validates the header block: exactly one `Content-Length` of decimal
/// digits; `Content-Type` is accepted and ignored; nothing else.
fn content_length(header: &[u8]) -> Result<u64, String> {
    let text = std::str::from_utf8(header).map_err(|_| "header is not UTF-8".to_string())?;
    let text = text.strip_suffix("\r\n\r\n").unwrap_or(text);
    let mut length = None;
    for line in text.split("\r\n") {
        let Some((name, value)) = line.split_once(": ") else {
            return Err("malformed header line".to_string());
        };
        if name.eq_ignore_ascii_case("content-length") {
            if length.is_some() {
                return Err("duplicate Content-Length".to_string());
            }
            if value.is_empty() || value.len() > 19 || !value.bytes().all(|b| b.is_ascii_digit()) {
                return Err("Content-Length must be decimal digits".to_string());
            }
            length = value.parse::<u64>().ok();
        } else if !name.eq_ignore_ascii_case("content-type") {
            return Err(format!("unsupported header {}", name.escape_default()));
        }
    }
    length.ok_or_else(|| "missing Content-Length".to_string())
}

/// What a request asks for.
enum Method {
    Initialize,
    Op(Operation),
}

struct Session {
    /// Set by a successful `initialize`; the session is `ready` when set.
    workspace: Option<Workspace>,
}

impl Session {
    /// Handles one message body; `None` for a notification.
    fn handle(&mut self, body: &[u8], log: &mut dyn FnMut(Record)) -> Option<Value> {
        let fail = |log: &mut dyn FnMut(Record), id: Value, code: i64, message: String| {
            log_protocol(log, &message);
            Some(error(id, code, message, None))
        };
        // Layer 3: JSON.
        let request = match rail_json::parse(body) {
            Ok(value) => value,
            Err(e) => return fail(log, Value::Null, PARSE_ERROR, format!("parse error: {e}")),
        };
        // Layer 4: request shape.
        if request.as_object().is_none() {
            return fail(
                log,
                Value::Null,
                INVALID_REQUEST,
                "invalid request: expected an object".into(),
            );
        }
        let id = match request.get("id") {
            None => None,
            Some(id @ Value::String(_)) => Some(id.clone()),
            Some(id) if id.as_i64().is_some() => Some(id.clone()),
            Some(_) => {
                return fail(
                    log,
                    Value::Null,
                    INVALID_REQUEST,
                    "invalid request: id must be a string or an integer".into(),
                );
            }
        };
        let reply_id = id.clone().unwrap_or(Value::Null);
        if request.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
            return fail(
                log,
                reply_id,
                INVALID_REQUEST,
                "invalid request: jsonrpc must be \"2.0\"".into(),
            );
        }
        let Some(method) = request.get("method").and_then(Value::as_str) else {
            return fail(
                log,
                reply_id,
                INVALID_REQUEST,
                "invalid request: method must be a string".into(),
            );
        };
        // Notifications (no id) get no response; the skeleton handles none.
        let id = id?;
        // Layer 5: method.
        let method = match method {
            "initialize" => Method::Initialize,
            other => match Operation::from_method(other) {
                Some(op) => Method::Op(op),
                None => {
                    return fail(
                        log,
                        id,
                        METHOD_NOT_FOUND,
                        format!("method not found: {}", other.escape_default()),
                    );
                }
            },
        };
        // Layer 6: session state.
        let params = request.get("params");
        match (&method, &self.workspace) {
            (Method::Op(op), None) => {
                let e = ToolError::new(
                    ErrorCode::RapNotInitialized,
                    "the session is not initialized; send initialize first",
                );
                log_tool(log, op.method(), "-", &e);
                Some(tool_error(id, &e))
            }
            (Method::Initialize, Some(_)) => fail(
                log,
                id,
                INVALID_REQUEST,
                "invalid request: the session is already initialized".into(),
            ),
            (Method::Initialize, None) => Some(self.initialize(id, params, log)),
            (Method::Op(op), Some(ws)) => Some(operation(ws, *op, id, params, log)),
        }
    }

    fn initialize(
        &mut self,
        id: Value,
        params: Option<&Value>,
        log: &mut dyn FnMut(Record),
    ) -> Value {
        // Layer 7: parameter shape.
        let fields = match params.and_then(Value::as_object) {
            Some(fields) => fields,
            None => return invalid_params(log, id, "params must be an object"),
        };
        let versions = params
            .and_then(|p| p.get("client_versions"))
            .and_then(Value::as_array)
            .filter(|items| items.iter().all(|v| v.as_str().is_some()));
        let Some(versions) = versions else {
            return invalid_params(log, id, "client_versions must be an array of strings");
        };
        let Some(root) = params
            .and_then(|p| p.get("workspace_root"))
            .and_then(Value::as_str)
        else {
            return invalid_params(log, id, "workspace_root must be a string");
        };
        // Layer 8: parameter support.
        if let Some(e) = unsupported(fields, &["client_versions", "workspace_root"]) {
            log_tool(log, "initialize", "-", &e);
            return tool_error(id, &e);
        }
        if !versions.iter().any(|v| v.as_str() == Some(RAP_VERSION)) {
            let e = ToolError::new(
                ErrorCode::RapUnsupportedParam,
                "client_versions must include 0.1",
            );
            log_tool(log, "initialize", "-", &e);
            return tool_error(id, &e);
        }
        let Ok(workspace) = Workspace::open(std::path::Path::new(root)) else {
            return invalid_params(log, id, "workspace_root is not a usable directory");
        };
        self.workspace = Some(workspace);
        let capabilities = Operation::ALL
            .iter()
            .map(|op| Value::str(op.method()))
            .collect();
        result(
            id,
            Value::object([
                ("rap_version", Value::str(RAP_VERSION)),
                ("capabilities", Value::Array(capabilities)),
            ]),
        )
    }
}

fn operation(
    ws: &Workspace,
    op: Operation,
    id: Value,
    params: Option<&Value>,
    log: &mut dyn FnMut(Record),
) -> Value {
    // Layer 7: parameter shape.
    let Some(fields) = params.and_then(Value::as_object) else {
        return invalid_params(log, id, "params must be an object");
    };
    let Some(module) = params.and_then(|p| p.get("module")).and_then(Value::as_str) else {
        return invalid_params(log, id, "module must be a string");
    };
    // Layer 8: parameter support.
    if let Some(e) = unsupported(fields, &["module"]) {
        log_tool(log, op.method(), module, &e);
        return tool_error(id, &e);
    }
    match invoke_logged(ws, op, module, log) {
        Ok(value) => result(id, value),
        Err(e) => tool_error(id, &e),
    }
}

fn invalid_params(log: &mut dyn FnMut(Record), id: Value, message: &str) -> Value {
    let message = format!("invalid params: {message}");
    log_protocol(log, &message);
    error(id, INVALID_PARAMS, message, None)
}

/// The first parameter the skeleton does not support, as a ToolError.
fn unsupported(fields: &[(String, Value)], supported: &[&str]) -> Option<ToolError> {
    fields
        .iter()
        .find(|(key, _)| !supported.contains(&key.as_str()))
        .map(|(key, _)| {
            ToolError::new(
                ErrorCode::RapUnsupportedParam,
                format!("unsupported parameter {}", key.escape_default()),
            )
        })
}

fn log_tool(log: &mut dyn FnMut(Record), operation: &str, module: &str, e: &ToolError) {
    log(Record {
        level: LogLevel::Error,
        operation: operation.to_string(),
        module: module.to_string(),
        code: Some(e.code.as_str()),
        message: e.message.clone(),
    });
}
