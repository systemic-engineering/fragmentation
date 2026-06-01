//! Integration test — the binary spawns, accepts a `tools/list`
//! over stdin, returns the fifteen tool names over stdout.
//!
//! Per `docs/specs/fragmentation-mcp.md` §9 T1 acceptance criteria,
//! refined by T2 to track the four shard sub-tools (net 15) + the
//! binary rename to `frgmnt` (Alex's directive). T3 extends with
//! the load-bearing round-trip: open → commit → read → status.

use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

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

#[tokio::test]
async fn binary_lists_fifteen_tools_over_stdio() {
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
    let mut reader = BufReader::new(stdout).lines();

    let request = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;
    stdin
        .write_all(request.as_bytes())
        .await
        .expect("write request");
    stdin.write_all(b"\n").await.expect("write newline");
    stdin.flush().await.expect("flush stdin");
    // Drop stdin so the server's loop exits after replying. Without
    // this, the test would hang on `wait`.
    drop(stdin);

    let line = tokio::time::timeout(Duration::from_secs(5), reader.next_line())
        .await
        .expect("read response within timeout")
        .expect("read line io")
        .expect("response line");

    let parsed: serde_json::Value = serde_json::from_str(&line).expect("parse response JSON");
    assert_eq!(parsed.get("jsonrpc").and_then(|v| v.as_str()), Some("2.0"));
    assert_eq!(parsed.get("id").and_then(|v| v.as_u64()), Some(1));
    let tools = parsed
        .get("result")
        .and_then(|r| r.get("tools"))
        .and_then(|t| t.as_array())
        .expect("tools array");
    assert_eq!(tools.len(), 15, "expected fifteen tool callables");

    let names: Vec<&str> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
        .collect();
    for expected in [
        "fragmentation.commit",
        "fragmentation.snapshot",
        "fragmentation.read",
        "fragmentation.diff",
        "fragmentation.merge",
        "fragmentation.branch",
        "fragmentation.refs.list",
        "fragmentation.refs.update",
        "fragmentation.history",
        "fragmentation.search",
        "fragmentation.shard.open",
        "fragmentation.shard.status",
        "fragmentation.shard.flush",
        "fragmentation.shard.close",
        "fragmentation.observe",
    ] {
        assert!(
            names.contains(&expected),
            "missing tool: {expected} (got {names:?})"
        );
    }

    // Server should exit cleanly once stdin closes.
    let status = tokio::time::timeout(Duration::from_secs(5), child.wait())
        .await
        .expect("wait within timeout")
        .expect("wait");
    assert!(status.success(), "binary exited non-zero: {status:?}");
}

#[tokio::test]
async fn binary_opens_a_shard_over_stdio() {
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
    let mut reader = BufReader::new(stdout).lines();

    let request = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"fragmentation.shard.open","arguments":{"budget_mb":64}}}"#;
    stdin.write_all(request.as_bytes()).await.expect("write");
    stdin.write_all(b"\n").await.expect("write newline");
    stdin.flush().await.expect("flush");
    drop(stdin);

    let line = tokio::time::timeout(Duration::from_secs(5), reader.next_line())
        .await
        .expect("read response within timeout")
        .expect("read line io")
        .expect("response line");

    let parsed: serde_json::Value = serde_json::from_str(&line).expect("parse JSON");
    assert_eq!(parsed.get("id").and_then(|v| v.as_u64()), Some(2));
    // T7: tools/call result is wrapped per MCP §tools/call —
    // result.content[0].text is the JSON-serialized payload.
    let payload = unwrap_call_content(&parsed);
    let shard_id = payload
        .get("shard_id")
        .and_then(|s| s.as_str())
        .expect("shard_id in result");
    assert_eq!(shard_id.len(), 36, "expected hyphenated UUID");

    let status = tokio::time::timeout(Duration::from_secs(5), child.wait())
        .await
        .expect("wait")
        .expect("wait result");
    assert!(status.success());
}

// ---------------------------------------------------------------------------
// T3 — the load-bearing round-trip: open → commit → read → status.
// ---------------------------------------------------------------------------

/// The full content round-trip per the T3 brief acceptance:
///
/// 1. `shard.open(budget_mb=64)` → returns a `ShardId`.
/// 2. `fragmentation.commit(shard_id, path, content, message)` → returns OID.
/// 3. `fragmentation.read(shard_id, oid)` → returns the original content.
/// 4. `shard.status(shard_id)` → `hot_bytes > 0` (the commit landed in the store).
///
/// Drives the live `frgmnt` binary with all four requests pipelined
/// on stdin, then closes stdin and reads four response lines in order.
#[tokio::test]
async fn binary_round_trip_open_commit_read_status() {
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
    let mut reader = BufReader::new(stdout).lines();

    // Request 1: open a shard.
    let open_req = r#"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"fragmentation.shard.open","arguments":{"budget_mb":64}}}"#;
    stdin
        .write_all(open_req.as_bytes())
        .await
        .expect("write open");
    stdin.write_all(b"\n").await.expect("newline");

    // Read the open response BEFORE writing the commit (which depends
    // on the returned shard_id).
    let open_line = tokio::time::timeout(Duration::from_secs(5), reader.next_line())
        .await
        .expect("open within timeout")
        .expect("open line io")
        .expect("open line");
    let open_parsed: serde_json::Value =
        serde_json::from_str(&open_line).expect("parse open JSON");
    assert_eq!(open_parsed.get("id").and_then(|v| v.as_u64()), Some(10));
    // T7: tools/call results are wrapped per MCP §tools/call.
    let open_payload = unwrap_call_content(&open_parsed);
    let shard_id = open_payload
        .get("shard_id")
        .and_then(|s| s.as_str())
        .expect("shard_id in open result")
        .to_string();
    assert_eq!(shard_id.len(), 36, "expected hyphenated UUID");

    // Request 2: commit "hello world" at path "hello.txt".
    let commit_req = format!(
        r#"{{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{{"name":"fragmentation.commit","arguments":{{"shard_id":"{shard_id}","path":"hello.txt","content":"hello world","message":"init"}}}}}}"#
    );
    stdin
        .write_all(commit_req.as_bytes())
        .await
        .expect("write commit");
    stdin.write_all(b"\n").await.expect("newline");

    let commit_line = tokio::time::timeout(Duration::from_secs(5), reader.next_line())
        .await
        .expect("commit within timeout")
        .expect("commit line io")
        .expect("commit line");
    let commit_parsed: serde_json::Value =
        serde_json::from_str(&commit_line).expect("parse commit JSON");
    assert_eq!(commit_parsed.get("id").and_then(|v| v.as_u64()), Some(11));
    assert!(
        commit_parsed.get("error").is_none(),
        "commit returned error: {commit_parsed}"
    );
    let commit_payload = unwrap_call_content(&commit_parsed);
    let oid = commit_payload
        .get("oid")
        .and_then(|s| s.as_str())
        .expect("oid in commit result")
        .to_string();
    assert_eq!(
        oid.len(),
        40,
        "expected 40-char hex OID (git blob SHA-1), got {oid}"
    );

    // Request 3: read the content back by OID.
    let read_req = format!(
        r#"{{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{{"name":"fragmentation.read","arguments":{{"shard_id":"{shard_id}","oid":"{oid}"}}}}}}"#
    );
    stdin
        .write_all(read_req.as_bytes())
        .await
        .expect("write read");
    stdin.write_all(b"\n").await.expect("newline");

    let read_line = tokio::time::timeout(Duration::from_secs(5), reader.next_line())
        .await
        .expect("read within timeout")
        .expect("read line io")
        .expect("read line");
    let read_parsed: serde_json::Value =
        serde_json::from_str(&read_line).expect("parse read JSON");
    assert_eq!(read_parsed.get("id").and_then(|v| v.as_u64()), Some(12));
    assert!(
        read_parsed.get("error").is_none(),
        "read returned error: {read_parsed}"
    );
    let read_payload = unwrap_call_content(&read_parsed);
    let content = read_payload
        .get("content")
        .and_then(|s| s.as_str())
        .expect("content in read result");
    assert_eq!(
        content, "hello world",
        "round-trip content mismatch: got {content:?}"
    );

    // Request 4: status — hot_bytes > 0 confirms the commit landed.
    let status_req = format!(
        r#"{{"jsonrpc":"2.0","id":13,"method":"tools/call","params":{{"name":"fragmentation.shard.status","arguments":{{"shard_id":"{shard_id}"}}}}}}"#
    );
    stdin
        .write_all(status_req.as_bytes())
        .await
        .expect("write status");
    stdin.write_all(b"\n").await.expect("newline");
    stdin.flush().await.expect("flush");
    drop(stdin);

    let status_line = tokio::time::timeout(Duration::from_secs(5), reader.next_line())
        .await
        .expect("status within timeout")
        .expect("status line io")
        .expect("status line");
    let status_parsed: serde_json::Value =
        serde_json::from_str(&status_line).expect("parse status JSON");
    assert_eq!(status_parsed.get("id").and_then(|v| v.as_u64()), Some(13));
    let status_payload = unwrap_call_content(&status_parsed);
    let hot_bytes = status_payload
        .get("hot_bytes")
        .and_then(|v| v.as_u64())
        .expect("hot_bytes in status result");
    assert!(
        hot_bytes > 0,
        "expected hot_bytes > 0 after commit, got {hot_bytes}"
    );
    let total_bytes = status_payload
        .get("total_bytes")
        .and_then(|v| v.as_u64())
        .expect("total_bytes in status result");
    assert!(
        total_bytes >= hot_bytes,
        "expected total_bytes >= hot_bytes, got {total_bytes} < {hot_bytes}"
    );

    let status = tokio::time::timeout(Duration::from_secs(5), child.wait())
        .await
        .expect("wait within timeout")
        .expect("wait");
    assert!(status.success(), "binary exited non-zero: {status:?}");
}

/// Locate the freshly-built `frgmnt` binary. cargo sets
/// `CARGO_BIN_EXE_<name>` for integration tests of bin crates; we
/// use that to avoid PATH dependencies.
fn locate_binary() -> std::path::PathBuf {
    let env_key = format!("CARGO_BIN_EXE_{BINARY_NAME}");
    if let Ok(path) = std::env::var(&env_key) {
        return std::path::PathBuf::from(path);
    }
    // Fallback for unusual configurations.
    let mut path = std::env::current_exe().expect("current exe");
    path.pop(); // tests dir
    if path.ends_with("deps") {
        path.pop();
    }
    path.push(BINARY_NAME);
    path
}
