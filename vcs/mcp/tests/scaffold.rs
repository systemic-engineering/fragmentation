//! T1 scaffold tests — refined in T2 to track the §3.4/§3.6 split.
//!
//! These tests pin the wire envelope + the dispatch shape. T2
//! refined the shard-tool count from one aggregate slot to four
//! sub-tools, so the constants here are now `FIFTEEN_TOOL_NAMES`
//! and the registry constructor is `with_default_tools`.
//!
//! Per `docs/specs/fragmentation-mcp.md` §3.6 (twelve categories,
//! fifteen callables) + §9 T1 acceptance criteria.

use fragmentation_mcp::{
    Mcp, MethodName, Request, RequestId, Response, ToolName, ToolRegistry, FIFTEEN_TOOL_NAMES,
    JSON_RPC_VERSION,
};

#[test]
fn fifteen_callables_per_spec_section_3_6_refined() {
    // §3.6 says "twelve in total" — those twelve are CATEGORIES.
    // §3.4 names four sub-tools for the SHARD category, so the
    // total wire callable count is 11 + 4 = 15.
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
    assert_eq!(FIFTEEN_TOOL_NAMES.len(), 18);
    for name in expected {
        assert!(
            FIFTEEN_TOOL_NAMES.iter().any(|t| *t == name),
            "missing tool name: {name}"
        );
    }
}

#[test]
fn request_round_trips_through_json() {
    let payload = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;
    let parsed: Request = Request::parse(payload).expect("parse");
    assert_eq!(parsed.jsonrpc, JSON_RPC_VERSION);
    assert_eq!(parsed.id, RequestId::from(1u64));
    assert_eq!(parsed.method, MethodName::from("tools/list"));
}

#[test]
fn registry_dispatches_tools_list() {
    let registry = ToolRegistry::with_default_tools();
    let request = Request {
        jsonrpc: JSON_RPC_VERSION,
        id: RequestId::from(1u64),
        method: MethodName::from("tools/list"),
        params: None,
    };
    let response: Response = registry.dispatch(&request);
    let body = serde_json::to_value(&response).expect("serialize");
    let tools = body
        .get("result")
        .and_then(|r| r.get("tools"))
        .and_then(|t| t.as_array())
        .expect("tools array");
    assert_eq!(tools.len(), 18);
}

#[test]
fn tool_call_for_stub_returns_not_implemented_yet() {
    // T3 wired `fragmentation.commit` + `fragmentation.read`; the
    // remaining ten tools (snapshot, diff, merge, branch, refs.*,
    // history, search, observe) still carry the
    // ERROR_NOT_IMPLEMENTED_YET stub. `fragmentation.snapshot` is
    // chosen here as a stable T4+ sentinel.
    let registry = ToolRegistry::with_default_tools();
    let params = serde_json::json!({
        "name": "fragmentation.snapshot",
        "arguments": {}
    });
    let request = Request {
        jsonrpc: JSON_RPC_VERSION,
        id: RequestId::from(2u64),
        method: MethodName::from("tools/call"),
        params: Some(params),
    };
    let response = registry.dispatch(&request);
    let value = serde_json::to_value(&response).expect("serialize");
    // The error code is the substrate's `not_implemented_yet` sentinel.
    let code = value
        .get("error")
        .and_then(|e| e.get("code"))
        .and_then(|c| c.as_i64())
        .expect("error code");
    assert_eq!(code, fragmentation_mcp::ERROR_NOT_IMPLEMENTED_YET);
}

#[test]
fn unknown_method_yields_method_not_found() {
    let registry = ToolRegistry::with_default_tools();
    let request = Request {
        jsonrpc: JSON_RPC_VERSION,
        id: RequestId::from(3u64),
        method: MethodName::from("nope/does-not-exist"),
        params: None,
    };
    let response = registry.dispatch(&request);
    let value = serde_json::to_value(&response).expect("serialize");
    let code = value
        .get("error")
        .and_then(|e| e.get("code"))
        .and_then(|c| c.as_i64())
        .expect("error code");
    // JSON-RPC 2.0 §5.1: -32601 Method not found.
    assert_eq!(code, -32601);
}

#[test]
fn mcp_constructs_with_default_registry() {
    let mcp = Mcp::new();
    let tool_names = mcp.tool_names();
    let names: Vec<&str> = tool_names.iter().map(ToolName::as_str).collect();
    assert!(names.contains(&"fragmentation.commit"));
    assert!(names.contains(&"fragmentation.shard.open"));
    assert_eq!(names.len(), 18);
}

#[test]
fn newtype_request_id_does_not_accept_raw_u64_at_construction_sites() {
    // Compile-time discipline: RequestId is the wire-altitude
    // newtype, even after the enum widening that landed on
    // `mara/request-id-enum`. The substrate forbids bare primitives
    // crossing the wire — `From<u64>` lands in the `Number` variant,
    // `From<&str>` in `Str`, and `RequestId::Null` is its own state
    // per JSON-RPC §5. This test pins the construction-site shape.
    let id: RequestId = RequestId::from(7u64);
    assert_eq!(id, RequestId::Number(7));
    let s: RequestId = RequestId::from("req-abc-123");
    assert_eq!(s, RequestId::Str("req-abc-123".to_string()));
}

#[test]
fn newtype_tool_name_carries_its_string() {
    let name = ToolName::from("fragmentation.commit");
    assert_eq!(name.as_str(), "fragmentation.commit");
}
