//! Taut's debug round — reproduce the one-hour hang on
//! `fragmentation_read` after a commit with an em-dash.
//!
//! Hypothesis under test: the em-dash in the commit content
//! breaks a length-counted / partial-UTF-8 read in Claude Code's
//! MCP client. We CANNOT test the client here — we test the
//! frgmnt side: does the server return a well-formed, complete,
//! newline-terminated JSON response in milliseconds, with the
//! same shape it returns for pure-ASCII content?

use std::process::Stdio;
use std::time::{Duration, Instant};

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

const BINARY_NAME: &str = "frgmnt";

fn locate_binary() -> std::path::PathBuf {
    let env_key = format!("CARGO_BIN_EXE_{BINARY_NAME}");
    if let Ok(path) = std::env::var(&env_key) {
        return std::path::PathBuf::from(path);
    }
    let mut path = std::env::current_exe().expect("current exe");
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.push(BINARY_NAME);
    path
}

const INITIALIZE_REQUEST: &str = r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"taut","version":"0.1"}}}"#;
const NOTIFICATIONS_INITIALIZED: &str =
    r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;

/// Exact sequence Claude Code drove, with the em-dash content.
/// Returns Vec<(label, raw_bytes)> for each response line read.
async fn drive_full_sequence(content: &str) -> Vec<(String, Vec<u8>)> {
    let binary = locate_binary();
    let mut child = Command::new(&binary)
        .arg("--stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn frgmnt");

    let mut stdin = child.stdin.take().expect("stdin");
    let stdout = child.stdout.take().expect("stdout");
    let mut reader = BufReader::new(stdout);

    let mut responses: Vec<(String, Vec<u8>)> = Vec::new();

    // ---- initialize ----
    stdin.write_all(INITIALIZE_REQUEST.as_bytes()).await.unwrap();
    stdin.write_all(b"\n").await.unwrap();
    stdin.flush().await.unwrap();
    let mut buf = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), reader.read_until(b'\n', &mut buf))
        .await
        .expect("initialize timeout")
        .expect("initialize io");
    responses.push(("initialize".to_string(), buf));

    // ---- notifications/initialized (no response expected) ----
    stdin
        .write_all(NOTIFICATIONS_INITIALIZED.as_bytes())
        .await
        .unwrap();
    stdin.write_all(b"\n").await.unwrap();
    stdin.flush().await.unwrap();

    // ---- shard.open ----
    let open_req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"fragmentation.shard.open","arguments":{"budget_mb":64}}}"#;
    stdin.write_all(open_req.as_bytes()).await.unwrap();
    stdin.write_all(b"\n").await.unwrap();
    stdin.flush().await.unwrap();
    let mut buf = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), reader.read_until(b'\n', &mut buf))
        .await
        .expect("open timeout")
        .expect("open io");
    let open_parsed: serde_json::Value =
        serde_json::from_slice(&buf).expect("open parse");
    responses.push(("open".to_string(), buf));

    let result = open_parsed.get("result").expect("open result");
    let text = result
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|b| b.get("text"))
        .and_then(|t| t.as_str())
        .expect("open text");
    let inner: serde_json::Value = serde_json::from_str(text).expect("open inner");
    let shard_id = inner
        .get("shard_id")
        .and_then(|s| s.as_str())
        .expect("shard_id")
        .to_string();

    // ---- commit (with the given content) ----
    let commit_args = serde_json::json!({
        "shard_id": shard_id,
        "path": "hello.txt",
        "content": content,
        "message": "init",
    });
    let commit_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": { "name": "fragmentation.commit", "arguments": commit_args },
    });
    let commit_line = commit_req.to_string();
    stdin.write_all(commit_line.as_bytes()).await.unwrap();
    stdin.write_all(b"\n").await.unwrap();
    stdin.flush().await.unwrap();
    let mut buf = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), reader.read_until(b'\n', &mut buf))
        .await
        .expect("commit timeout")
        .expect("commit io");
    let commit_parsed: serde_json::Value =
        serde_json::from_slice(&buf).expect("commit parse");
    responses.push(("commit".to_string(), buf));

    let result = commit_parsed.get("result").expect("commit result");
    let text = result
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|b| b.get("text"))
        .and_then(|t| t.as_str())
        .expect("commit text");
    let inner: serde_json::Value = serde_json::from_str(text).expect("commit inner");
    let oid = inner
        .get("oid")
        .and_then(|s| s.as_str())
        .expect("oid")
        .to_string();

    // ---- read ----
    let read_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "fragmentation.read",
            "arguments": { "shard_id": shard_id, "oid": oid },
        },
    });
    let read_line = read_req.to_string();
    let read_start = Instant::now();
    stdin.write_all(read_line.as_bytes()).await.unwrap();
    stdin.write_all(b"\n").await.unwrap();
    stdin.flush().await.unwrap();
    let mut buf = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), reader.read_until(b'\n', &mut buf))
        .await
        .expect("read timeout — THIS IS THE HANG WE WANT TO REPRODUCE")
        .expect("read io");
    let elapsed = read_start.elapsed();
    eprintln!(
        "[taut] read response received in {:?} ({} bytes)",
        elapsed,
        buf.len()
    );
    responses.push(("read".to_string(), buf));

    drop(stdin);
    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
    responses
}

#[tokio::test]
async fn em_dash_round_trip_completes_in_under_a_second() {
    let content = "hello from inside claude code — first real drive of frgmnt";
    let responses = drive_full_sequence(content).await;
    assert_eq!(responses.len(), 4);

    let (label, read_bytes) = &responses[3];
    assert_eq!(label, "read");

    // Print every byte of the read response so we can eyeball framing.
    eprintln!("[taut] em-dash read response ({} bytes):", read_bytes.len());
    eprintln!("[taut] raw bytes (hex): {}", hex_dump(read_bytes));
    eprintln!(
        "[taut] as utf8 (lossy): {}",
        String::from_utf8_lossy(read_bytes)
    );

    let parsed: serde_json::Value =
        serde_json::from_slice(read_bytes).expect("read response parses as JSON");
    let text = parsed
        .get("result")
        .and_then(|r| r.get("content"))
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|b| b.get("text"))
        .and_then(|t| t.as_str())
        .expect("read text");
    let inner: serde_json::Value = serde_json::from_str(text).expect("read inner");
    let echoed = inner
        .get("content")
        .and_then(|s| s.as_str())
        .expect("echoed content");
    assert_eq!(echoed, content, "round-trip content mismatch");
}

#[tokio::test]
async fn ascii_only_round_trip_for_byte_comparison() {
    let content = "hello world";
    let responses = drive_full_sequence(content).await;
    assert_eq!(responses.len(), 4);
    let (_, read_bytes) = &responses[3];
    eprintln!("[taut] ascii read response ({} bytes):", read_bytes.len());
    eprintln!("[taut] as utf8: {}", String::from_utf8_lossy(read_bytes));
}

fn hex_dump(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && i % 16 == 0 {
            out.push('\n');
        }
        out.push_str(&format!("{:02x} ", b));
    }
    out
}

/// THE ACTUAL HANG: Claude Code's MCP client uses STRING ids
/// (per the JSON-RPC §4 / MCP 2025-06-18 §JSON-RPC contract, which
/// permits "String, Number, or NULL"). Our `RequestId(pub u64)`
/// newtype rejects string ids at parse time. Server emits
/// `id: 0` parse-error response. Claude Code is waiting for a
/// response with the ORIGINAL string id and never sees it → hangs.
#[tokio::test]
async fn string_id_read_request_should_echo_string_id() {
    let binary = locate_binary();
    let mut child = Command::new(&binary)
        .arg("--stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn frgmnt");

    let mut stdin = child.stdin.take().expect("stdin");
    let stdout = child.stdout.take().expect("stdout");
    let mut reader = BufReader::new(stdout);

    // Drive initialize with a STRING id (mimicking Claude Code).
    let init = r#"{"jsonrpc":"2.0","id":"req-abc-123","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"taut","version":"0.1"}}}"#;
    stdin.write_all(init.as_bytes()).await.unwrap();
    stdin.write_all(b"\n").await.unwrap();
    stdin.flush().await.unwrap();

    let mut buf = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(2), reader.read_until(b'\n', &mut buf)).await;
    eprintln!(
        "[taut] init w/ string id response: {}",
        String::from_utf8_lossy(&buf)
    );
    let parsed: serde_json::Value = serde_json::from_slice(&buf).expect("parse");
    let id_val = parsed.get("id").cloned();
    eprintln!("[taut] echoed id: {:?}", id_val);

    drop(stdin);
    let _ = tokio::time::timeout(Duration::from_secs(2), child.wait()).await;

    // ASSERTION: per JSON-RPC 2.0 §4 + MCP spec, the server MUST
    // echo the request id at the same JSON type it came in. A
    // string in, a string out. Today this asserts the BUG: id
    // round-trips as integer 0 because RequestId rejects strings.
    assert_eq!(
        id_val,
        Some(serde_json::json!("req-abc-123")),
        "BUG: server failed to echo string id; this is why Claude Code hangs"
    );
}

/// Floating-point id (JavaScript JSON.stringify default for large
/// integers): does the server accept `id: 3.0` / `id: 1e2`?
#[tokio::test]
async fn float_id_request_behavior() {
    let binary = locate_binary();
    let mut child = Command::new(&binary)
        .arg("--stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn frgmnt");

    let mut stdin = child.stdin.take().expect("stdin");
    let stdout = child.stdout.take().expect("stdout");
    let mut reader = BufReader::new(stdout);

    let init = r#"{"jsonrpc":"2.0","id":3.0,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"taut","version":"0.1"}}}"#;
    stdin.write_all(init.as_bytes()).await.unwrap();
    stdin.write_all(b"\n").await.unwrap();
    stdin.flush().await.unwrap();
    let mut buf = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(2), reader.read_until(b'\n', &mut buf)).await;
    eprintln!(
        "[taut] float id response: {}",
        String::from_utf8_lossy(&buf)
    );
    drop(stdin);
    let _ = tokio::time::timeout(Duration::from_secs(2), child.wait()).await;
}

/// Negative-id: JavaScript's first id is often 0 or 1 then
/// incremented per request. But some clients use negative ids for
/// notifications-with-tracking. Does the server accept negative?
#[tokio::test]
async fn negative_id_request_behavior() {
    let binary = locate_binary();
    let mut child = Command::new(&binary)
        .arg("--stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn frgmnt");

    let mut stdin = child.stdin.take().expect("stdin");
    let stdout = child.stdout.take().expect("stdout");
    let mut reader = BufReader::new(stdout);

    let init = r#"{"jsonrpc":"2.0","id":-1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"taut","version":"0.1"}}}"#;
    stdin.write_all(init.as_bytes()).await.unwrap();
    stdin.write_all(b"\n").await.unwrap();
    stdin.flush().await.unwrap();
    let mut buf = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(2), reader.read_until(b'\n', &mut buf)).await;
    eprintln!(
        "[taut] negative id response: {}",
        String::from_utf8_lossy(&buf)
    );
    drop(stdin);
    let _ = tokio::time::timeout(Duration::from_secs(2), child.wait()).await;
}

/// Smaller stand-alone test: does the inner-text escaping of the
/// em-dash collide with anything? This drives wrap_tool_response's
/// `payload.to_string()` shape.
#[test]
fn payload_to_string_em_dash_shape() {
    let payload = serde_json::json!({
        "oid": "deadbeef",
        "shard_id": "00000000-0000-af13-49b9-f5f9a1a6a040",
        "content": "hello — em",
    });
    let text = payload.to_string();
    eprintln!("[taut] payload.to_string() = {}", text);
    eprintln!("[taut] bytes: {:?}", text.as_bytes());

    // Wrap into the outer response shape and serialize.
    let outer = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "result": {
            "content": [{ "type": "text", "text": text }],
            "isError": false,
        },
    });
    let wire = serde_json::to_string(&outer).unwrap();
    eprintln!("[taut] wire = {}", wire);
    eprintln!("[taut] wire bytes len = {}", wire.len());
    // Sanity: wire is valid UTF-8 and parses round-trip.
    let reparsed: serde_json::Value = serde_json::from_str(&wire).unwrap();
    let inner_text = reparsed
        .get("result")
        .and_then(|r| r.get("content"))
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|b| b.get("text"))
        .and_then(|t| t.as_str())
        .unwrap();
    let reinner: serde_json::Value = serde_json::from_str(inner_text).unwrap();
    assert_eq!(
        reinner.get("content").and_then(|s| s.as_str()),
        Some("hello — em")
    );
}
