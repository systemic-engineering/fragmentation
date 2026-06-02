//! RED: T10 session bootstrap.
//!
//! Tests that `shard_open` returns a ShardRef with session context
//! committed (git branch, cwd, timestamp), `shard_open_empty` returns
//! a bare ShardRef with no context, and the session shard UUID is
//! derived from session metadata (not the EMPTY canonical).

use std::process::Stdio;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

const BINARY_NAME: &str = "frgmnt";
const EMPTY_CANONICAL_UUID: &str = "00000000-0000-af13-49b9-f5f9a1a6a040";

const INITIALIZE_REQUEST: &str = r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"session-test","version":"0.1"}}}"#;
const NOTIFICATIONS_INITIALIZED: &str =
    r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;

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

fn spawn_frgmnt() -> (Child, ChildStdin, BufReader<ChildStdout>) {
    let binary = locate_binary();
    let mut child = Command::new(&binary)
        .arg("--stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn frgmnt");
    let stdin = child.stdin.take().expect("stdin");
    let stdout = child.stdout.take().expect("stdout");
    (child, stdin, BufReader::new(stdout))
}

async fn write_line(stdin: &mut ChildStdin, payload: &str) {
    stdin.write_all(payload.as_bytes()).await.unwrap();
    stdin.write_all(b"\n").await.unwrap();
    stdin.flush().await.unwrap();
}

async fn read_line(reader: &mut BufReader<ChildStdout>) -> String {
    let fut = async {
        let mut buf = String::new();
        reader.read_line(&mut buf).await?;
        Ok::<_, std::io::Error>(buf.trim_end_matches('\n').trim_end_matches('\r').to_string())
    };
    tokio::time::timeout(Duration::from_secs(5), fut)
        .await
        .expect("read_line timed out")
        .expect("read_line io error")
}

/// Extract `result.content[0].text` and parse as JSON.
fn unwrap_call_content(resp: &Value) -> Value {
    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("content[0].text missing in: {resp}"));
    serde_json::from_str(text).unwrap_or_else(|e| panic!("content[0].text not JSON: {e} — raw: {text}"))
}

async fn do_handshake(stdin: &mut ChildStdin, reader: &mut BufReader<ChildStdout>) {
    write_line(stdin, INITIALIZE_REQUEST).await;
    read_line(reader).await; // consume initialize response
    write_line(stdin, NOTIFICATIONS_INITIALIZED).await;
}

async fn call_tool(stdin: &mut ChildStdin, reader: &mut BufReader<ChildStdout>, name: &str, args: Value) -> Value {
    let req = json!({
        "jsonrpc": "2.0",
        "id": format!("call-{name}"),
        "method": "tools/call",
        "params": { "name": name, "arguments": args }
    });
    write_line(stdin, &req.to_string()).await;
    let raw = read_line(reader).await;
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("response not JSON: {e} — raw: {raw}"))
}

// ---------------------------------------------------------------------------
// T10-1: shard_open response includes context_oid
// ---------------------------------------------------------------------------

#[tokio::test]
async fn shard_open_returns_context_oid() {
    let (mut child, mut stdin, mut reader) = spawn_frgmnt();
    do_handshake(&mut stdin, &mut reader).await;

    let resp = call_tool(&mut stdin, &mut reader, "fragmentation_shard_open", json!({"budget_mb": 64})).await;
    assert!(resp.get("error").is_none(), "shard_open errored: {resp}");
    let result = unwrap_call_content(&resp);
    assert!(
        result.get("context_oid").is_some(),
        "shard_open must include context_oid; got: {result}"
    );
    let _ = tokio::time::timeout(Duration::from_secs(2), child.wait()).await;
}

// ---------------------------------------------------------------------------
// T10-2: context_oid is readable and contains cwd
// ---------------------------------------------------------------------------

#[tokio::test]
async fn shard_open_context_oid_is_readable_with_cwd() {
    let (mut child, mut stdin, mut reader) = spawn_frgmnt();
    do_handshake(&mut stdin, &mut reader).await;

    let open = unwrap_call_content(
        &call_tool(&mut stdin, &mut reader, "fragmentation_shard_open", json!({"budget_mb": 64})).await,
    );
    let shard_id = open["shard_id"].as_str().unwrap().to_string();
    let context_oid = open["context_oid"].as_str().unwrap().to_string();

    let read = unwrap_call_content(
        &call_tool(&mut stdin, &mut reader, "fragmentation.read", json!({
            "shard_id": shard_id,
            "oid": context_oid,
        })).await,
    );
    let meta: Value = serde_json::from_str(read["content"].as_str().unwrap())
        .expect("context content must be JSON");
    assert!(
        meta.get("cwd").is_some(),
        "session context must include cwd; got: {meta}"
    );
    let _ = tokio::time::timeout(Duration::from_secs(2), child.wait()).await;
}

// ---------------------------------------------------------------------------
// T10-3: shard_open_empty tool exists and returns no context_oid
// ---------------------------------------------------------------------------

#[tokio::test]
async fn shard_open_empty_has_no_context_oid() {
    let (mut child, mut stdin, mut reader) = spawn_frgmnt();
    do_handshake(&mut stdin, &mut reader).await;

    let resp = call_tool(&mut stdin, &mut reader, "fragmentation_shard_open_empty", json!({"budget_mb": 64})).await;
    assert!(resp.get("error").is_none(), "shard_open_empty errored: {resp}");
    let result = unwrap_call_content(&resp);
    assert!(
        result.get("context_oid").is_none(),
        "shard_open_empty must not include context_oid; got: {result}"
    );
    assert!(result.get("shard_id").is_some(), "shard_open_empty must return shard_id");
    let _ = tokio::time::timeout(Duration::from_secs(2), child.wait()).await;
}

// ---------------------------------------------------------------------------
// T10-4: session shard id is not the EMPTY canonical UUID
// ---------------------------------------------------------------------------

#[tokio::test]
async fn shard_open_session_id_differs_from_empty_canonical() {
    let (mut child, mut stdin, mut reader) = spawn_frgmnt();
    do_handshake(&mut stdin, &mut reader).await;

    let result = unwrap_call_content(
        &call_tool(&mut stdin, &mut reader, "fragmentation_shard_open", json!({"budget_mb": 64})).await,
    );
    let shard_id = result["shard_id"].as_str().unwrap();
    assert_ne!(
        shard_id,
        EMPTY_CANONICAL_UUID,
        "shard_open session UUID must be derived from session context, not the EMPTY canonical"
    );
    let _ = tokio::time::timeout(Duration::from_secs(2), child.wait()).await;
}

// ---------------------------------------------------------------------------
// T10-5: shard_open_empty appears in tools/list
// ---------------------------------------------------------------------------

#[tokio::test]
async fn shard_open_empty_in_tools_list() {
    let (mut child, mut stdin, mut reader) = spawn_frgmnt();
    do_handshake(&mut stdin, &mut reader).await;

    write_line(&mut stdin, r#"{"jsonrpc":"2.0","id":"list","method":"tools/list"}"#).await;
    let raw = read_line(&mut reader).await;
    assert!(
        raw.contains("fragmentation_shard_open_empty"),
        "tools/list must include fragmentation_shard_open_empty; got: {raw}"
    );
    let _ = tokio::time::timeout(Duration::from_secs(2), child.wait()).await;
}
