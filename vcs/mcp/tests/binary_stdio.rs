//! Integration test — the binary spawns, accepts a `tools/list`
//! over stdin, returns the fifteen tool names over stdout.
//!
//! Per `docs/specs/fragmentation-mcp.md` §9 T1 acceptance criteria,
//! refined by T2 to track the four shard sub-tools (net 15) + the
//! binary rename to `frgmnt` (Alex's directive).

use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

const BINARY_NAME: &str = "frgmnt";

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
    let shard_id = parsed
        .get("result")
        .and_then(|r| r.get("shard_id"))
        .and_then(|s| s.as_str())
        .expect("shard_id in result");
    assert_eq!(shard_id.len(), 36, "expected hyphenated UUID");

    let status = tokio::time::timeout(Duration::from_secs(5), child.wait())
        .await
        .expect("wait")
        .expect("wait result");
    assert!(status.success());
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
