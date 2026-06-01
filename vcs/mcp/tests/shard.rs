//! T2 RED — the shard sub-tools + scheduler tick.
//!
//! T4 NOTE: `ShardId(uuid::Uuid)` became `ShardId(SpectralUuid)` per the
//! CRDT spec (`reality-shard-as-crdt.md`). Two opens-without-content
//! share the canonical `ShardId::EMPTY` (the deduplication property);
//! tests that relied on `ShardId::new() != ShardId::new()` have been
//! migrated to honor the new semantics.
//!
//! Per `docs/specs/fragmentation-mcp.md` §3.4 (shard sub-tools), §4
//! (HamiltonScheduler at the agent altitude), and §9 T2 (scope +
//! acceptance).
//!
//! What the tests pin:
//!
//! 1. `ShardId(SpectralUuid)` newtype — no bare UUIDs cross the wire.
//! 2. `BudgetMb(u64)` newtype — no bare megabytes cross the wire.
//! 3. The four `fragmentation.shard.*` sub-tools (open / status /
//!    flush / close) exist and dispatch as separate callables.
//! 4. `TWELVE_TOOL_NAMES` has been replaced by
//!    `FIFTEEN_TOOL_NAMES` — the §3.6 row count is still twelve
//!    *categories*, but the wire callable count is fifteen (the
//!    shard category has four sub-tools).
//! 5. `Mcp::dispatch_line` ticks the named shard's
//!    `HamiltonScheduler` before routing — observable via a per-
//!    shard tick counter.
//! 6. `shard.close` removes the shard; a subsequent call with the
//!    same `ShardId` returns `ERROR_INVALID_PARAMS` (not a panic).

/// Extract the structured payload from a `tools/call` response.
///
/// Per T7 (MCP 2025-06-18 §tools/call), every `tools/call` result is
/// wrapped as `{content: [{type: "text", text: "<json>"}], isError:
/// false}`. This helper unwraps it back to the payload object that
/// the tool body actually produced. NOT for tools/list (which is
/// unwrapped per spec).
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

use fragmentation_mcp::{
    BudgetMb, Mcp, MethodName, Request, RequestId, ShardId, ToolName, ToolRegistry,
    FIFTEEN_TOOL_NAMES, JSON_RPC_VERSION,
};

// ---------------------------------------------------------------------------
// Newtype discipline — ShardId + BudgetMb.
// ---------------------------------------------------------------------------

#[test]
fn shard_id_wraps_spectral_uuid() {
    // Post-T4: ShardId wraps SpectralUuid. Display is still the
    // 36-char hyphenated form (wire-stable with the prior
    // uuid::Uuid v4 output).
    let id = ShardId::EMPTY;
    let s = id.to_string();
    assert_eq!(
        s.len(),
        36,
        "ShardId should serialize as a hyphenated 36-char string: {s}"
    );
    // The CRDT semilattice's bottom element: two reads of EMPTY are
    // byte-identical. The dedup property is a feature.
    assert_eq!(ShardId::EMPTY, ShardId::EMPTY);
    // Content-derived ids DIFFER for different content_hash values.
    let a = ShardId::from_content(0, &[0x11u8; 32]);
    let b = ShardId::from_content(0, &[0x22u8; 32]);
    assert_ne!(a, b);
}

#[test]
fn shard_id_round_trips_through_string() {
    // Round-trip a content-derived ShardId through Display → parse.
    let id = ShardId::from_content(0x0001_2345_6789_ABCD, &[0x77u8; 32]);
    let s = id.to_string();
    let parsed = ShardId::parse(&s).expect("parse ShardId from its own Display");
    assert_eq!(id, parsed);
}

#[test]
fn shard_id_parse_rejects_garbage() {
    assert!(ShardId::parse("not-a-uuid").is_err());
    assert!(ShardId::parse("").is_err());
}

#[test]
fn budget_mb_to_bytes() {
    let budget = BudgetMb(64);
    assert_eq!(budget.as_bytes(), 64 * 1024 * 1024);
}

// ---------------------------------------------------------------------------
// Tool surface — fifteen callables; the shard category has four.
// ---------------------------------------------------------------------------

#[test]
fn fifteen_tool_names_per_spec_section_3_6_refined() {
    // The spec §3.6 lists twelve CATEGORIES; the shard category
    // expands into four sub-tools (`open`/`status`/`flush`/`close`)
    // per §3.4. Net wire callables: 11 + 4 = 15.
    let expected = [
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
    ];
    assert_eq!(FIFTEEN_TOOL_NAMES.len(), 15);
    for name in expected {
        assert!(
            FIFTEEN_TOOL_NAMES.iter().any(|t| *t == name),
            "missing tool name: {name}"
        );
    }
    // The aggregate `fragmentation.shard` slot is GONE — it has
    // been split into four sub-tools per the T1 refinement.
    assert!(
        !FIFTEEN_TOOL_NAMES.iter().any(|t| *t == "fragmentation.shard"),
        "aggregate slot `fragmentation.shard` should be split into four sub-tools"
    );
}

#[test]
fn tools_list_returns_fifteen() {
    let registry = ToolRegistry::with_default_tools();
    let request = Request {
        jsonrpc: JSON_RPC_VERSION,
        id: RequestId::from(1u64),
        method: MethodName::from("tools/list"),
        params: None,
    };
    let response = registry.dispatch(&request);
    let value = serde_json::to_value(&response).expect("serialize");
    let tools = value
        .get("result")
        .and_then(|r| r.get("tools"))
        .and_then(|t| t.as_array())
        .expect("tools array");
    assert_eq!(tools.len(), 15, "expected fifteen tool callables");
}

// ---------------------------------------------------------------------------
// shard.open — allocates a shard, returns its id.
// ---------------------------------------------------------------------------

#[test]
fn shard_open_returns_a_shard_id() {
    let mcp = Mcp::new();
    let line = r#"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"fragmentation.shard.open","arguments":{"budget_mb":64}}}"#;
    let response = mcp.dispatch_line(line);
    let value = serde_json::to_value(&response).expect("serialize");
    assert!(
        value.get("error").is_none(),
        "shard.open should not error: {value:#?}"
    );
    let payload = unwrap_call_content(&value);
    let shard_id = payload
        .get("shard_id")
        .and_then(|s| s.as_str())
        .expect("shard_id in result");
    let parsed = ShardId::parse(shard_id).expect("parse shard_id");
    // Round-trip through Display.
    assert_eq!(parsed.to_string(), shard_id);
}

#[test]
fn shard_open_rejects_missing_budget() {
    let mcp = Mcp::new();
    let line = r#"{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"fragmentation.shard.open","arguments":{}}}"#;
    let response = mcp.dispatch_line(line);
    let value = serde_json::to_value(&response).expect("serialize");
    let code = value
        .get("error")
        .and_then(|e| e.get("code"))
        .and_then(|c| c.as_i64())
        .expect("error code");
    // Missing required argument is INVALID_PARAMS per JSON-RPC.
    assert_eq!(code, fragmentation_mcp::ERROR_INVALID_PARAMS);
}

// ---------------------------------------------------------------------------
// shard.status — returns sensible numbers for an open shard.
// ---------------------------------------------------------------------------

#[test]
fn shard_status_returns_budget_and_scheduler_stats() {
    let mcp = Mcp::new();
    let shard_id = open_shard(&mcp, 32);

    let status_line = format!(
        r#"{{"jsonrpc":"2.0","id":20,"method":"tools/call","params":{{"name":"fragmentation.shard.status","arguments":{{"shard_id":"{shard_id}"}}}}}}"#
    );
    let response = mcp.dispatch_line(&status_line);
    let value = serde_json::to_value(&response).expect("serialize");
    assert!(
        value.get("error").is_none(),
        "shard.status should not error: {value:#?}"
    );
    let result = unwrap_call_content(&value);
    let budget_bytes = result
        .get("budget_bytes")
        .and_then(|b| b.as_u64())
        .expect("budget_bytes");
    assert_eq!(budget_bytes, 32 * 1024 * 1024);
    // Stub scheduler returns zero hot/cold/total in T2.
    assert_eq!(result.get("hot_bytes").and_then(|b| b.as_u64()), Some(0));
    assert_eq!(result.get("cold_bytes").and_then(|b| b.as_u64()), Some(0));
    assert_eq!(result.get("total_bytes").and_then(|b| b.as_u64()), Some(0));
    // Tick count: every dispatch ticks the named shard once. By the
    // time `shard.status` returns, the scheduler has ticked at least
    // once (this call). The exact count depends on dispatch ordering;
    // we assert lower-bound monotonicity.
    let tick_count = result
        .get("tick_count")
        .and_then(|t| t.as_u64())
        .expect("tick_count");
    assert!(tick_count >= 1, "tick_count should be >= 1 after dispatch");
}

#[test]
fn shard_status_unknown_id_yields_invalid_params() {
    let mcp = Mcp::new();
    // Synthesize an unknown shard_id by deriving from a content hash
    // that no shard in the registry could have produced. Different from
    // ShardId::EMPTY (which is what `shard.open` would have created).
    let bogus = ShardId::from_content(0xDEAD_BEEF_CAFE, &[0xAAu8; 32]);
    let line = format!(
        r#"{{"jsonrpc":"2.0","id":21,"method":"tools/call","params":{{"name":"fragmentation.shard.status","arguments":{{"shard_id":"{bogus}"}}}}}}"#
    );
    let response = mcp.dispatch_line(&line);
    let value = serde_json::to_value(&response).expect("serialize");
    let code = value
        .get("error")
        .and_then(|e| e.get("code"))
        .and_then(|c| c.as_i64())
        .expect("error code");
    assert_eq!(code, fragmentation_mcp::ERROR_INVALID_PARAMS);
}

// ---------------------------------------------------------------------------
// shard.flush — callable; no-op against stub scheduler; does not error.
// ---------------------------------------------------------------------------

#[test]
fn shard_flush_is_callable_on_open_shard() {
    let mcp = Mcp::new();
    let shard_id = open_shard(&mcp, 8);
    let line = format!(
        r#"{{"jsonrpc":"2.0","id":30,"method":"tools/call","params":{{"name":"fragmentation.shard.flush","arguments":{{"shard_id":"{shard_id}"}}}}}}"#
    );
    let response = mcp.dispatch_line(&line);
    let value = serde_json::to_value(&response).expect("serialize");
    assert!(
        value.get("error").is_none(),
        "shard.flush should not error on open shard: {value:#?}"
    );
    let result = unwrap_call_content(&value);
    // Stub: no entries to evict; bytes_released = 0.
    assert_eq!(
        result.get("bytes_released").and_then(|b| b.as_u64()),
        Some(0)
    );
}

// ---------------------------------------------------------------------------
// shard.close — removes the shard; subsequent ops fail INVALID_PARAMS.
// ---------------------------------------------------------------------------

#[test]
fn shard_close_removes_the_shard() {
    let mcp = Mcp::new();
    let shard_id = open_shard(&mcp, 16);

    let close_line = format!(
        r#"{{"jsonrpc":"2.0","id":40,"method":"tools/call","params":{{"name":"fragmentation.shard.close","arguments":{{"shard_id":"{shard_id}"}}}}}}"#
    );
    let response = mcp.dispatch_line(&close_line);
    let value = serde_json::to_value(&response).expect("serialize");
    assert!(
        value.get("error").is_none(),
        "shard.close should succeed: {value:#?}"
    );

    // Subsequent shard.status with the SAME id returns INVALID_PARAMS.
    let status_line = format!(
        r#"{{"jsonrpc":"2.0","id":41,"method":"tools/call","params":{{"name":"fragmentation.shard.status","arguments":{{"shard_id":"{shard_id}"}}}}}}"#
    );
    let response = mcp.dispatch_line(&status_line);
    let value = serde_json::to_value(&response).expect("serialize");
    let code = value
        .get("error")
        .and_then(|e| e.get("code"))
        .and_then(|c| c.as_i64())
        .expect("error code");
    assert_eq!(code, fragmentation_mcp::ERROR_INVALID_PARAMS);
}

// ---------------------------------------------------------------------------
// dispatch_line ticks the named shard before routing.
// ---------------------------------------------------------------------------

#[test]
fn dispatch_ticks_the_named_shard_before_routing() {
    let mcp = Mcp::new();
    let shard_id = open_shard(&mcp, 4);

    // Two status calls; the second should observe a tick_count
    // strictly greater than the first.
    let status_line = format!(
        r#"{{"jsonrpc":"2.0","id":50,"method":"tools/call","params":{{"name":"fragmentation.shard.status","arguments":{{"shard_id":"{shard_id}"}}}}}}"#
    );
    let r1 = mcp.dispatch_line(&status_line);
    let v1 = serde_json::to_value(&r1).expect("serialize");
    let tick_1 = unwrap_call_content(&v1)
        .get("tick_count")
        .and_then(|t| t.as_u64())
        .expect("tick_count 1");

    let r2 = mcp.dispatch_line(&status_line);
    let v2 = serde_json::to_value(&r2).expect("serialize");
    let tick_2 = unwrap_call_content(&v2)
        .get("tick_count")
        .and_then(|t| t.as_u64())
        .expect("tick_count 2");

    assert!(
        tick_2 > tick_1,
        "second dispatch should observe higher tick_count: {tick_1} -> {tick_2}"
    );
}

// ---------------------------------------------------------------------------
// Helpers.
// ---------------------------------------------------------------------------

fn open_shard(mcp: &Mcp, budget_mb: u64) -> ShardId {
    let line = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"fragmentation.shard.open","arguments":{{"budget_mb":{budget_mb}}}}}}}"#
    );
    let response = mcp.dispatch_line(&line);
    let value = serde_json::to_value(&response).expect("serialize");
    let payload = unwrap_call_content(&value);
    let shard_id = payload
        .get("shard_id")
        .and_then(|s| s.as_str())
        .expect("shard_id in result");
    ShardId::parse(shard_id).expect("parse shard_id")
}

// ---------------------------------------------------------------------------
// Re-exports for the wire-error constants used in the asserts above.
// ---------------------------------------------------------------------------

/// Sanity: the wire-error constants stay re-exported at the crate root.
#[test]
fn wire_error_constants_stay_re_exported() {
    let _ = fragmentation_mcp::ERROR_INVALID_PARAMS;
    let _ = fragmentation_mcp::ERROR_METHOD_NOT_FOUND;
    let _ = fragmentation_mcp::ERROR_NOT_IMPLEMENTED_YET;
}

// ---------------------------------------------------------------------------
// ToolName carrying the new sub-tool strings.
// ---------------------------------------------------------------------------

#[test]
fn tool_name_carries_shard_sub_tool_strings() {
    let n = ToolName::from("fragmentation.shard.open");
    assert_eq!(n.as_str(), "fragmentation.shard.open");
}
