//! T1 RED — the scaffold tests.
//!
//! These tests reference types and functions that DO NOT EXIST in
//! `src/lib.rs` yet. The RED commit's job is to assert the contract;
//! the GREEN commit's job is to make them compile and pass.
//!
//! Per `docs/specs/fragmentation-mcp.md` §9 T1:
//! - JSON-RPC 2.0 envelope parses round-trip.
//! - Tool registry names the twelve §3.6 tools.
//! - `tools/list` dispatch returns the twelve names.
//! - A non-existent method yields JSON-RPC error `-32601` (Method not found).

use fragmentation_mcp::{
    Mcp, MethodName, Request, RequestId, Response, ToolName, ToolRegistry,
    JSON_RPC_VERSION, TWELVE_TOOL_NAMES,
};

#[test]
fn twelve_tools_named_per_spec_section_3_6() {
    // §3.6 says "twelve in total" — these are the row labels in the
    // table at line 545 of fragmentation-mcp.md.
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
        "fragmentation.shard",
        "fragmentation.observe",
    ];
    assert_eq!(TWELVE_TOOL_NAMES.len(), 12);
    for name in expected {
        assert!(
            TWELVE_TOOL_NAMES.iter().any(|t| t.as_str() == name),
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
    let registry = ToolRegistry::with_twelve_tools();
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
    assert_eq!(tools.len(), 12);
}

#[test]
fn tool_call_for_stub_returns_not_implemented_yet() {
    let registry = ToolRegistry::with_twelve_tools();
    let params = serde_json::json!({
        "name": "fragmentation.commit",
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
    let registry = ToolRegistry::with_twelve_tools();
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
    let names: Vec<&str> = mcp.tool_names().iter().map(ToolName::as_str).collect();
    assert!(names.contains(&"fragmentation.commit"));
    assert_eq!(names.len(), 12);
}

#[test]
fn newtype_request_id_does_not_accept_raw_u64_at_construction_sites() {
    // Compile-time discipline: RequestId is a newtype. The substrate
    // forbids bare primitives crossing the wire.
    let id: RequestId = RequestId::from(7u64);
    assert_eq!(id.as_u64(), 7);
}

#[test]
fn newtype_tool_name_carries_its_string() {
    let name = ToolName::from("fragmentation.commit");
    assert_eq!(name.as_str(), "fragmentation.commit");
}
