//! T5 integration tests — the full MCP client handshake against the
//! live `frgmnt` binary.
//!
//! These tests spawn `frgmnt --stdio` as a subprocess and drive the
//! full protocol per the MCP 2024-11-05+ lifecycle spec:
//!
//!   client → `initialize` (request)         → server → capabilities
//!   client → `notifications/initialized`    → server → (no response)
//!   client → `tools/list` (request)         → server → tool list
//!   client → `tools/call` (request)         → server → result
//!
//! Per JSON-RPC 2.0 §4.1, NOTIFICATIONS (no `id`) MUST NOT receive a
//! response and MUST NOT crash the server. T1's `Request::parse`
//! required `id` to be present, so `notifications/initialized` would
//! fail parse and crash the dispatch path before the second `tools/*`
//! request could land. T5 introduces an `Envelope::Notification` arm
//! alongside the existing `Envelope::Request`, plumbed through the
//! stdio loop in `Mcp::run_stdio`.
//!
//! Substrate-pull: `[substrate-pull:realize]` — protocol semantics
//! (request vs. notification routing) are boundary discipline; the
//! capability (shard state, scheduler ticks) lives in
//! `fragmentation`.

use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

const BINARY_NAME: &str = "frgmnt";

/// Extract the structured payload from a `tools/call` response.
///
/// Per T7 (MCP 2025-06-18 §tools/call), every `tools/call` result is
/// wrapped as `{content: [{type: "text", text: "<json>"}], isError:
/// false}`. This helper unwraps it back to the payload object that
/// the tool body actually produced.
fn unwrap_call_content(parsed: &serde_json::Value) -> serde_json::Value {
    let text = parsed
        .get("result")
        .and_then(|r| r.get("content"))
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|b| b.get("text"))
        .and_then(|t| t.as_str())
        .expect("tools/call result must carry content[0].text per MCP §tools/call");
    serde_json::from_str(text).expect("content[0].text must parse as JSON payload")
}

/// Spawn `frgmnt --stdio`. Returns the child + an owned stdin handle
/// + a buffered stdout reader. Caller drops stdin to signal EOF.
fn spawn_frgmnt() -> (Child, ChildStdin, BufReader<ChildStdout>) {
    let binary = locate_binary();
    let mut child = Command::new(&binary)
        .arg("--stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn frgmnt");
    let stdin = child.stdin.take().expect("stdin");
    let stdout = child.stdout.take().expect("stdout");
    let reader = BufReader::new(stdout);
    (child, stdin, reader)
}

/// Write one newline-terminated JSON-RPC line and flush.
async fn write_line(stdin: &mut ChildStdin, payload: &str) {
    stdin
        .write_all(payload.as_bytes())
        .await
        .expect("write payload");
    stdin.write_all(b"\n").await.expect("write newline");
    stdin.flush().await.expect("flush stdin");
}

/// Read one response line with a 5-second timeout. Strips trailing
/// CRLF for parse convenience.
async fn read_line(reader: &mut BufReader<ChildStdout>) -> String {
    let fut = async {
        let mut buf = String::new();
        let n = reader.read_line(&mut buf).await?;
        if n == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "stdout closed before response",
            ));
        }
        if buf.ends_with('\n') {
            buf.pop();
            if buf.ends_with('\r') {
                buf.pop();
            }
        }
        std::io::Result::Ok(buf)
    };
    tokio::time::timeout(Duration::from_secs(5), fut)
        .await
        .expect("read response within timeout")
        .expect("read response")
}

/// The standard MCP `initialize` request payload (id=0).
const INITIALIZE_REQUEST: &str = r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"frgmnt-test","version":"0.1"}}}"#;

/// The standard MCP `notifications/initialized` payload (NO id).
const NOTIFICATIONS_INITIALIZED: &str =
    r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;

// ---------------------------------------------------------------------------
// 1. initialize_handshake_completes — baseline; T1/T2 implicitly wires this.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn initialize_handshake_completes() {
    let (mut child, mut stdin, mut reader) = spawn_frgmnt();
    write_line(&mut stdin, INITIALIZE_REQUEST).await;
    let line = read_line(&mut reader).await;
    drop(stdin);

    let parsed: serde_json::Value = serde_json::from_str(&line).expect("parse initialize JSON");
    assert_eq!(parsed.get("jsonrpc").and_then(|v| v.as_str()), Some("2.0"));
    assert_eq!(parsed.get("id").and_then(|v| v.as_u64()), Some(0));
    let result = parsed.get("result").expect("initialize result");
    assert!(
        result.get("protocolVersion").is_some(),
        "initialize result missing protocolVersion: {result}"
    );
    let capabilities = result
        .get("capabilities")
        .expect("initialize capabilities");
    assert!(
        capabilities.get("tools").is_some(),
        "capabilities missing tools: {capabilities}"
    );
    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
}

// ---------------------------------------------------------------------------
// 2. notifications_initialized_does_not_crash — THE T5 BUG ORACLE.
// ---------------------------------------------------------------------------

/// After `initialize`, send `notifications/initialized` (no `id`).
/// Assert: (a) no response on stdout, (b) the process is still alive
/// and a subsequent `tools/list` returns a fifteen-tool list.
///
/// If `Request::parse` rejects the notification (because `id` is
/// non-optional), the parse-error response will appear on stdout
/// before the `tools/list` response, and the test will see a malformed
/// transcript. THIS IS THE TEST THAT FAILS BEFORE THE FIX.
#[tokio::test]
async fn notifications_initialized_does_not_crash() {
    let (mut child, mut stdin, mut reader) = spawn_frgmnt();

    // Full handshake: initialize → notifications/initialized → tools/list.
    write_line(&mut stdin, INITIALIZE_REQUEST).await;
    let init_line = read_line(&mut reader).await;
    let init_parsed: serde_json::Value =
        serde_json::from_str(&init_line).expect("parse initialize");
    assert_eq!(init_parsed.get("id").and_then(|v| v.as_u64()), Some(0));

    // Send the notification. Server must NOT respond on stdout.
    write_line(&mut stdin, NOTIFICATIONS_INITIALIZED).await;

    // Drive a follow-up request. Whatever comes back on stdout next
    // MUST be the response to THIS request — not a parse-error from
    // the notification.
    write_line(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
    )
    .await;
    drop(stdin);

    let next_line = read_line(&mut reader).await;
    let parsed: serde_json::Value = serde_json::from_str(&next_line).expect("parse next line");
    assert_eq!(
        parsed.get("id").and_then(|v| v.as_u64()),
        Some(1),
        "expected tools/list response (id=1), got: {parsed}"
    );
    assert!(
        parsed.get("error").is_none(),
        "notification triggered an error response: {parsed}"
    );
    let tools = parsed
        .get("result")
        .and_then(|r| r.get("tools"))
        .and_then(|t| t.as_array())
        .expect("tools array");
    assert_eq!(tools.len(), 18);

    let status = tokio::time::timeout(Duration::from_secs(5), child.wait())
        .await
        .expect("wait")
        .expect("wait result");
    assert!(status.success(), "binary exited non-zero: {status:?}");
}

// ---------------------------------------------------------------------------
// 3. tools_list_after_full_handshake — the realistic boot path.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tools_list_after_full_handshake() {
    let (mut child, mut stdin, mut reader) = spawn_frgmnt();

    write_line(&mut stdin, INITIALIZE_REQUEST).await;
    let _ = read_line(&mut reader).await;
    write_line(&mut stdin, NOTIFICATIONS_INITIALIZED).await;
    write_line(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
    )
    .await;
    drop(stdin);

    let list_line = read_line(&mut reader).await;
    let parsed: serde_json::Value = serde_json::from_str(&list_line).expect("parse tools/list");
    let tools = parsed
        .get("result")
        .and_then(|r| r.get("tools"))
        .and_then(|t| t.as_array())
        .expect("tools array");
    assert_eq!(tools.len(), 18);

    let names: Vec<&str> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
        .collect();
    for expected in [
        "fragmentation.commit",
        "fragmentation.read",
        "fragmentation.shard.open",
        "fragmentation.shard.status",
        "fragmentation.shard.close",
        "fragmentation.observe",
    ] {
        assert!(names.contains(&expected), "missing tool: {expected}");
    }

    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
}

// ---------------------------------------------------------------------------
// 4. tools_call_after_full_handshake — shard.open through the handshake.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tools_call_after_full_handshake() {
    let (mut child, mut stdin, mut reader) = spawn_frgmnt();

    write_line(&mut stdin, INITIALIZE_REQUEST).await;
    let _ = read_line(&mut reader).await;
    write_line(&mut stdin, NOTIFICATIONS_INITIALIZED).await;
    write_line(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"fragmentation.shard.open","arguments":{"budget_mb":64}}}"#,
    )
    .await;
    drop(stdin);

    let call_line = read_line(&mut reader).await;
    let parsed: serde_json::Value = serde_json::from_str(&call_line).expect("parse tools/call");
    assert_eq!(parsed.get("id").and_then(|v| v.as_u64()), Some(1));
    assert!(parsed.get("error").is_none(), "tools/call errored: {parsed}");
    let payload = unwrap_call_content(&parsed);
    let shard_id = payload
        .get("shard_id")
        .and_then(|s| s.as_str())
        .expect("shard_id");
    assert_eq!(shard_id.len(), 36, "expected hyphenated UUID");

    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
}

// ---------------------------------------------------------------------------
// 5. agent_workflow_round_trip — the full T3 round-trip behind a handshake.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn agent_workflow_round_trip() {
    let (mut child, mut stdin, mut reader) = spawn_frgmnt();

    // Step 0–1: handshake.
    write_line(&mut stdin, INITIALIZE_REQUEST).await;
    let _ = read_line(&mut reader).await;
    write_line(&mut stdin, NOTIFICATIONS_INITIALIZED).await;

    // Step 2: shard.open
    write_line(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"fragmentation.shard.open","arguments":{"budget_mb":64}}}"#,
    )
    .await;
    let open_line = read_line(&mut reader).await;
    let open_parsed: serde_json::Value = serde_json::from_str(&open_line).expect("parse open");
    let shard_id = unwrap_call_content(&open_parsed)
        .get("shard_id")
        .and_then(|s| s.as_str())
        .expect("shard_id")
        .to_string();

    // Step 3: commit
    let commit_req = format!(
        r#"{{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{{"name":"fragmentation.commit","arguments":{{"shard_id":"{shard_id}","path":"agent.txt","content":"agent payload","message":"t5"}}}}}}"#
    );
    write_line(&mut stdin, &commit_req).await;
    let commit_line = read_line(&mut reader).await;
    let commit_parsed: serde_json::Value =
        serde_json::from_str(&commit_line).expect("parse commit");
    assert!(
        commit_parsed.get("error").is_none(),
        "commit errored: {commit_parsed}"
    );
    let oid = unwrap_call_content(&commit_parsed)
        .get("oid")
        .and_then(|s| s.as_str())
        .expect("oid")
        .to_string();

    // Step 4: read
    let read_req = format!(
        r#"{{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{{"name":"fragmentation.read","arguments":{{"shard_id":"{shard_id}","oid":"{oid}"}}}}}}"#
    );
    write_line(&mut stdin, &read_req).await;
    let read_line_str = read_line(&mut reader).await;
    let read_parsed: serde_json::Value =
        serde_json::from_str(&read_line_str).expect("parse read");
    let content = unwrap_call_content(&read_parsed)
        .get("content")
        .and_then(|s| s.as_str())
        .expect("content")
        .to_string();
    assert_eq!(content, "agent payload");

    // Step 5: status
    let status_req = format!(
        r#"{{"jsonrpc":"2.0","id":13,"method":"tools/call","params":{{"name":"fragmentation.shard.status","arguments":{{"shard_id":"{shard_id}"}}}}}}"#
    );
    write_line(&mut stdin, &status_req).await;
    let status_line = read_line(&mut reader).await;
    let status_parsed: serde_json::Value =
        serde_json::from_str(&status_line).expect("parse status");
    let hot_bytes = unwrap_call_content(&status_parsed)
        .get("hot_bytes")
        .and_then(|v| v.as_u64())
        .expect("hot_bytes");
    assert!(hot_bytes > 0, "expected hot_bytes > 0");

    // Step 6: close
    let close_req = format!(
        r#"{{"jsonrpc":"2.0","id":14,"method":"tools/call","params":{{"name":"fragmentation.shard.close","arguments":{{"shard_id":"{shard_id}"}}}}}}"#
    );
    write_line(&mut stdin, &close_req).await;
    drop(stdin);
    let close_line = read_line(&mut reader).await;
    let close_parsed: serde_json::Value =
        serde_json::from_str(&close_line).expect("parse close");
    assert_eq!(
        unwrap_call_content(&close_parsed)
            .get("closed")
            .and_then(|b| b.as_bool()),
        Some(true)
    );

    let status = tokio::time::timeout(Duration::from_secs(5), child.wait())
        .await
        .expect("wait")
        .expect("wait result");
    assert!(status.success());
}

// ---------------------------------------------------------------------------
// 6. empty_shard_determinism_wire_visible — two opens → same id.
// ---------------------------------------------------------------------------

/// The CRDT-layer recognition: empty shards have structurally identical
/// state, so opening two of them returns the same `ShardId`. T4 wired
/// this at the type level (`ShardId(SpectralUuid)`); T5 verifies the
/// determinism survives the full MCP wire round-trip.
#[tokio::test]
async fn empty_shard_determinism_wire_visible() {
    let (mut child, mut stdin, mut reader) = spawn_frgmnt();

    write_line(&mut stdin, INITIALIZE_REQUEST).await;
    let _ = read_line(&mut reader).await;
    write_line(&mut stdin, NOTIFICATIONS_INITIALIZED).await;
    write_line(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"fragmentation.shard.open","arguments":{"budget_mb":64}}}"#,
    )
    .await;
    let line_a = read_line(&mut reader).await;
    write_line(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"fragmentation.shard.open","arguments":{"budget_mb":64}}}"#,
    )
    .await;
    drop(stdin);
    let line_b = read_line(&mut reader).await;

    let a: serde_json::Value = serde_json::from_str(&line_a).expect("parse a");
    let b: serde_json::Value = serde_json::from_str(&line_b).expect("parse b");
    let payload_a = unwrap_call_content(&a);
    let payload_b = unwrap_call_content(&b);
    let id_a = payload_a
        .get("shard_id")
        .and_then(|s| s.as_str())
        .expect("a shard_id");
    let id_b = payload_b
        .get("shard_id")
        .and_then(|s| s.as_str())
        .expect("b shard_id");
    assert_eq!(
        id_a, id_b,
        "two empty-shard opens should produce the same ShardId (CRDT-layer recognition)"
    );

    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
}

// ---------------------------------------------------------------------------
// 7. malformed_json_returns_parse_error — JSON-RPC §5.1, code -32700.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn malformed_json_returns_parse_error() {
    let (mut child, mut stdin, mut reader) = spawn_frgmnt();
    write_line(&mut stdin, "{not json}").await;
    drop(stdin);
    let line = read_line(&mut reader).await;

    let parsed: serde_json::Value = serde_json::from_str(&line).expect("parse error response");
    let code = parsed
        .get("error")
        .and_then(|e| e.get("code"))
        .and_then(|c| c.as_i64())
        .expect("error code");
    assert_eq!(
        code, -32700,
        "expected JSON-RPC parse-error code, got {code}"
    );

    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
}

// ---------------------------------------------------------------------------
// 8. unknown_method_returns_method_not_found — code -32601.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn unknown_method_returns_method_not_found() {
    let (mut child, mut stdin, mut reader) = spawn_frgmnt();
    write_line(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":99,"method":"fragmentation.does_not_exist"}"#,
    )
    .await;
    drop(stdin);
    let line = read_line(&mut reader).await;

    let parsed: serde_json::Value = serde_json::from_str(&line).expect("parse response");
    assert_eq!(parsed.get("id").and_then(|v| v.as_u64()), Some(99));
    let code = parsed
        .get("error")
        .and_then(|e| e.get("code"))
        .and_then(|c| c.as_i64())
        .expect("error code");
    assert_eq!(code, -32601);

    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
}

// ---------------------------------------------------------------------------
// 9. unknown_shard_id_returns_invalid_params — code -32602 (T2 convention).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn unknown_shard_id_returns_invalid_params() {
    let (mut child, mut stdin, mut reader) = spawn_frgmnt();
    // A well-formed UUID that no live shard maps to.
    let bogus = "00000000-0000-0000-0000-000000000000";
    let req = format!(
        r#"{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{"name":"fragmentation.shard.status","arguments":{{"shard_id":"{bogus}"}}}}}}"#
    );
    write_line(&mut stdin, &req).await;
    drop(stdin);
    let line = read_line(&mut reader).await;

    let parsed: serde_json::Value = serde_json::from_str(&line).expect("parse response");
    let code = parsed
        .get("error")
        .and_then(|e| e.get("code"))
        .and_then(|c| c.as_i64())
        .expect("error code");
    assert_eq!(code, -32602);

    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
}

// ---------------------------------------------------------------------------
// 10. multi_session_concurrent_shards — two subprocesses don't interfere.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn multi_session_concurrent_shards() {
    let (mut child_a, mut stdin_a, mut reader_a) = spawn_frgmnt();
    let (mut child_b, mut stdin_b, mut reader_b) = spawn_frgmnt();

    // Each session does its own handshake + opens + commits.
    write_line(&mut stdin_a, INITIALIZE_REQUEST).await;
    write_line(&mut stdin_b, INITIALIZE_REQUEST).await;
    let _ = read_line(&mut reader_a).await;
    let _ = read_line(&mut reader_b).await;
    write_line(&mut stdin_a, NOTIFICATIONS_INITIALIZED).await;
    write_line(&mut stdin_b, NOTIFICATIONS_INITIALIZED).await;

    // Each opens a shard and commits DIFFERENT content. The OIDs
    // differ (content-addressed); the shards live in isolated
    // process memory.
    write_line(
        &mut stdin_a,
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"fragmentation.shard.open","arguments":{"budget_mb":64}}}"#,
    )
    .await;
    write_line(
        &mut stdin_b,
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"fragmentation.shard.open","arguments":{"budget_mb":64}}}"#,
    )
    .await;
    let open_a = read_line(&mut reader_a).await;
    let open_b = read_line(&mut reader_b).await;
    let shard_a = serde_json::from_str::<serde_json::Value>(&open_a)
        .map(|v: serde_json::Value| unwrap_call_content(&v))
        .unwrap()
        .get("shard_id")
        .and_then(|s| s.as_str())
        .unwrap()
        .to_string();
    let shard_b = serde_json::from_str::<serde_json::Value>(&open_b)
        .map(|v: serde_json::Value| unwrap_call_content(&v))
        .unwrap()
        .get("shard_id")
        .and_then(|s| s.as_str())
        .unwrap()
        .to_string();

    // Commit DIFFERENT content to each. Different content => different
    // OIDs. (Identical content would yield identical OIDs by
    // content-addressing — not interference, but indistinguishability.)
    let commit_a = format!(
        r#"{{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{{"name":"fragmentation.commit","arguments":{{"shard_id":"{shard_a}","path":"a.txt","content":"alpha","message":"A"}}}}}}"#
    );
    let commit_b = format!(
        r#"{{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{{"name":"fragmentation.commit","arguments":{{"shard_id":"{shard_b}","path":"b.txt","content":"beta","message":"B"}}}}}}"#
    );
    write_line(&mut stdin_a, &commit_a).await;
    write_line(&mut stdin_b, &commit_b).await;
    let ca_line = read_line(&mut reader_a).await;
    let cb_line = read_line(&mut reader_b).await;
    let oid_a = serde_json::from_str::<serde_json::Value>(&ca_line)
        .map(|v: serde_json::Value| unwrap_call_content(&v))
        .unwrap()
        .get("oid")
        .and_then(|s| s.as_str())
        .unwrap()
        .to_string();
    let oid_b = serde_json::from_str::<serde_json::Value>(&cb_line)
        .map(|v: serde_json::Value| unwrap_call_content(&v))
        .unwrap()
        .get("oid")
        .and_then(|s| s.as_str())
        .unwrap()
        .to_string();
    assert_ne!(oid_a, oid_b, "different content must yield different OIDs");

    // Cross-process read MUST fail (each shard lives in its own
    // subprocess; B's OID is unknown to A's store).
    let cross_read = format!(
        r#"{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{"name":"fragmentation.read","arguments":{{"shard_id":"{shard_a}","oid":"{oid_b}"}}}}}}"#
    );
    write_line(&mut stdin_a, &cross_read).await;
    drop(stdin_a);
    drop(stdin_b);
    let cross_line = read_line(&mut reader_a).await;
    let cross_parsed: serde_json::Value = serde_json::from_str(&cross_line).unwrap();
    assert!(
        cross_parsed.get("error").is_some(),
        "cross-process read should error: {cross_parsed}"
    );

    let _ = tokio::time::timeout(Duration::from_secs(5), child_a.wait()).await;
    let _ = tokio::time::timeout(Duration::from_secs(5), child_b.wait()).await;
}

// ---------------------------------------------------------------------------
// 11. process_exits_cleanly_on_close — EOF on stdin => exit 0.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn process_exits_cleanly_on_close() {
    let (mut child, stdin, mut reader) = spawn_frgmnt();
    drop(stdin); // immediate EOF

    // Drain any output (there should be none — no request was sent).
    let mut sink = Vec::new();
    let _ = tokio::time::timeout(
        Duration::from_millis(500),
        reader.get_mut().read_to_end(&mut sink),
    )
    .await;

    let status = tokio::time::timeout(Duration::from_secs(2), child.wait())
        .await
        .expect("exit within 2s")
        .expect("wait status");
    assert!(status.success(), "expected exit 0, got {status:?}");
}

// ---------------------------------------------------------------------------
// 12. graceful_response_to_id_with_null — JSON-RPC permits `id: null`.
// ---------------------------------------------------------------------------

/// JSON-RPC 2.0 §4: an `id` of `null` is permitted (typically used
/// when the client encountered an error detecting an id). The server
/// should NOT crash. T5 surfaces this — the current `RequestId(u64)`
/// type doesn't accept `null`, so the parser will reject it. The fix
/// keeps the request-vs-notification distinction but allows `null`
/// in `id` for graceful-degradation error reporting.
#[tokio::test]
async fn graceful_response_to_id_with_null() {
    let (mut child, mut stdin, mut reader) = spawn_frgmnt();
    // Send an `id: null` request. The server should not crash; either
    // a response with `id: null` OR a parse-error (with id:0) is
    // acceptable as long as the subsequent request lands.
    write_line(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":null,"method":"tools/list"}"#,
    )
    .await;
    let _first = read_line(&mut reader).await;
    // Follow up to prove the process is alive.
    write_line(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":7,"method":"tools/list"}"#,
    )
    .await;
    drop(stdin);
    let second = read_line(&mut reader).await;
    let parsed: serde_json::Value = serde_json::from_str(&second).expect("parse second");
    assert_eq!(parsed.get("id").and_then(|v| v.as_u64()), Some(7));

    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
}

// ---------------------------------------------------------------------------
// 13. T6 — inputSchema published for every tool (MCP-spec conformance).
//
// Per MCP 2025-06-18 §tools, every entry in `tools/list` MUST carry
// an `inputSchema` field. Clients (Claude Code, Cursor, Claude
// Desktop) validate the response and refuse to load tool surfaces
// that omit it.
//
// T1–T5 emitted only `name` + `description`. Alex hit the
// validation failure on first attempt to drive `frgmnt` from
// Claude Code:
//
//   Reconnected to frgmnt, but fetching tools failed: [
//     {"path":["tools",0,"inputSchema"],
//      "message":"Invalid input: expected object, received undefined"},
//     ... (15 errors, one per tool)
//   ]
//
// This test reproduces that validation and lands as RED before the
// schemas exist.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tools_list_emits_input_schema_for_every_tool() {
    let (mut child, mut stdin, mut reader) = spawn_frgmnt();

    write_line(&mut stdin, INITIALIZE_REQUEST).await;
    let _ = read_line(&mut reader).await;
    write_line(&mut stdin, NOTIFICATIONS_INITIALIZED).await;
    write_line(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
    )
    .await;
    drop(stdin);

    let list_line = read_line(&mut reader).await;
    let parsed: serde_json::Value =
        serde_json::from_str(&list_line).expect("parse tools/list");
    let tools = parsed
        .get("result")
        .and_then(|r| r.get("tools"))
        .and_then(|t| t.as_array())
        .expect("tools array");
    assert_eq!(tools.len(), 18);

    for (i, tool) in tools.iter().enumerate() {
        let name = tool
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("<missing>");

        // The load-bearing assertion: inputSchema must be present.
        let schema = tool.get("inputSchema").unwrap_or_else(|| {
            panic!("tool {i} (`{name}`) missing required field `inputSchema`");
        });

        // The substrate's minimum claim: every schema is a JSON
        // object with `type` = `"object"`. Per JSON Schema
        // 2020-12, this is the minimum well-formed input schema.
        assert!(
            schema.is_object(),
            "tool `{name}` has non-object inputSchema: {schema}"
        );
        assert_eq!(
            schema.get("type").and_then(|v| v.as_str()),
            Some("object"),
            "tool `{name}` inputSchema.type is not \"object\": {schema}"
        );
    }

    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
}

// ---------------------------------------------------------------------------
// 14. T6 — wired-tool schemas carry their required-args.
//
// Beyond "schema present", the spec calls for the schema to describe
// the actual argument shape. For the four tools T2/T3 wired against
// real bodies (shard.open, shard.status, commit, read), we know the
// required arguments exactly — lock them in so future spec drift
// surfaces immediately.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn wired_tool_schemas_publish_required_arguments() {
    let (mut child, mut stdin, mut reader) = spawn_frgmnt();

    write_line(&mut stdin, INITIALIZE_REQUEST).await;
    let _ = read_line(&mut reader).await;
    write_line(&mut stdin, NOTIFICATIONS_INITIALIZED).await;
    write_line(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
    )
    .await;
    drop(stdin);

    let list_line = read_line(&mut reader).await;
    let parsed: serde_json::Value =
        serde_json::from_str(&list_line).expect("parse tools/list");
    let tools = parsed
        .get("result")
        .and_then(|r| r.get("tools"))
        .and_then(|t| t.as_array())
        .expect("tools array");

    let expectations: &[(&str, &[&str])] = &[
        ("fragmentation.shard.open", &["budget_mb"]),
        ("fragmentation.shard.status", &["shard_id"]),
        ("fragmentation.shard.flush", &["shard_id"]),
        ("fragmentation.shard.close", &["shard_id"]),
        (
            "fragmentation.commit",
            &["shard_id", "path", "content", "message"],
        ),
        ("fragmentation.read", &["shard_id", "oid"]),
    ];

    for (tool_name, required) in expectations {
        let tool = tools
            .iter()
            .find(|t| t.get("name").and_then(|n| n.as_str()) == Some(*tool_name))
            .unwrap_or_else(|| panic!("tool `{tool_name}` not in tools/list"));
        let schema = tool
            .get("inputSchema")
            .unwrap_or_else(|| panic!("tool `{tool_name}` missing inputSchema"));
        let req = schema
            .get("required")
            .and_then(|r| r.as_array())
            .unwrap_or_else(|| panic!("tool `{tool_name}` schema missing `required` array"));
        let actual: Vec<&str> = req.iter().filter_map(|v| v.as_str()).collect();
        for field in *required {
            assert!(
                actual.iter().any(|a| a == field),
                "tool `{tool_name}` schema.required missing `{field}` (got {actual:?})"
            );
        }
    }

    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
}

// ---------------------------------------------------------------------------
// 15. T7 — tools/call result wrapped in MCP content envelope.
//
// Per MCP 2025-06-18 §tools/call, the result MUST be a `CallToolResult`
// with shape `{content: [<ContentBlock>...], isError?: bool}` where each
// ContentBlock has a `type` discriminator (`text` / `image` / `audio` /
// `resource_link` / `resource`).
//
// T2/T3/T6 returned the raw payload directly as `result`:
//   {"jsonrpc":"2.0","id":1,"result":{"budget_bytes":...,"shard_id":"..."}}
//
// Required shape:
//   {"jsonrpc":"2.0","id":1,
//    "result":{"content":[{"type":"text",
//                          "text":"{\"budget_bytes\":...,\"shard_id\":\"...\"}"}],
//              "isError":false}}
//
// Alex hit this on first live drive 2026-06-01: tool calls succeeded
// but Claude Code rendered "no output" because it looks for
// `result.content[0].text` and finds nothing.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tools_call_result_wrapped_in_content_envelope() {
    let (mut child, mut stdin, mut reader) = spawn_frgmnt();

    write_line(&mut stdin, INITIALIZE_REQUEST).await;
    let _ = read_line(&mut reader).await;
    write_line(&mut stdin, NOTIFICATIONS_INITIALIZED).await;
    write_line(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"fragmentation.shard.open","arguments":{"budget_mb":64}}}"#,
    )
    .await;
    drop(stdin);

    let line = read_line(&mut reader).await;
    let parsed: serde_json::Value =
        serde_json::from_str(&line).expect("parse tools/call result");

    let result = parsed
        .get("result")
        .expect("tools/call must return result, not error");

    // Required envelope: `content` array of ContentBlock objects.
    let content = result
        .get("content")
        .and_then(|c| c.as_array())
        .expect("tools/call result.content must be an array (MCP 2025-06-18 §tools/call)");
    assert!(
        !content.is_empty(),
        "tools/call result.content must contain at least one ContentBlock"
    );

    let first = &content[0];
    let ty = first.get("type").and_then(|t| t.as_str());
    assert_eq!(
        ty,
        Some("text"),
        "ContentBlock.type must be 'text' (or other discriminated variant); got {ty:?}"
    );
    assert!(
        first.get("text").and_then(|t| t.as_str()).is_some(),
        "text ContentBlock must carry a `text` field (string)"
    );

    // isError is optional but MUST be a bool if present.
    if let Some(is_err) = result.get("isError") {
        assert!(
            is_err.is_boolean(),
            "isError must be a boolean if present; got {is_err}"
        );
        assert_eq!(is_err.as_bool(), Some(false), "successful call: isError=false");
    }

    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
}

// ---------------------------------------------------------------------------
// 16. T7 — the structured payload is recoverable from the text content.
//
// The text ContentBlock is a JSON-serialized payload. Round-trip:
// parsing `text` should yield the structured fields the original raw
// result carried (`shard_id`, `budget_bytes` for shard.open). This
// locks the convention: text MUST be JSON-serialized, not arbitrary
// prose, so agents can extract structured data.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tools_call_text_content_is_json_with_structured_payload() {
    let (mut child, mut stdin, mut reader) = spawn_frgmnt();

    write_line(&mut stdin, INITIALIZE_REQUEST).await;
    let _ = read_line(&mut reader).await;
    write_line(&mut stdin, NOTIFICATIONS_INITIALIZED).await;
    write_line(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"fragmentation.shard.open","arguments":{"budget_mb":64}}}"#,
    )
    .await;
    drop(stdin);

    let line = read_line(&mut reader).await;
    let parsed: serde_json::Value = serde_json::from_str(&line).expect("parse result");
    let text = parsed
        .get("result")
        .and_then(|r| r.get("content"))
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|b| b.get("text"))
        .and_then(|t| t.as_str())
        .expect("text field present");

    let payload: serde_json::Value =
        serde_json::from_str(text).expect("text content must parse as JSON");
    assert!(
        payload.get("shard_id").and_then(|s| s.as_str()).is_some(),
        "payload missing shard_id: {payload}"
    );
    assert!(
        payload.get("budget_bytes").and_then(|b| b.as_u64()).is_some(),
        "payload missing budget_bytes: {payload}"
    );

    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
}

// ---------------------------------------------------------------------------
// Binary location — see binary_stdio.rs for the standard cargo idiom.
// ---------------------------------------------------------------------------

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
