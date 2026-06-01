//! Tool registry — the twelve fragmentation-mcp tool slots.
//!
//! Per `docs/specs/fragmentation-mcp.md` §3.6, the tool surface is
//! twelve entries. T1 registers the names + stub descriptions;
//! every dispatch returns [`crate::ERROR_NOT_IMPLEMENTED_YET`]. T2+
//! fills in the bodies against fragmentation primitives.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::types::{MethodName, ToolName};
use crate::wire::{Request, Response, ResponseError, ERROR_INVALID_PARAMS, ERROR_METHOD_NOT_FOUND};

/// Substrate-defined error code for "the tool exists but T2+ has not
/// implemented its body yet". OUTSIDE JSON-RPC's reserved
/// `-32768..-32000` range; uses the application-error space the
/// JSON-RPC spec permits.
pub const ERROR_NOT_IMPLEMENTED_YET: i64 = -32001;

/// The twelve tool-name slots per §3.6 of fragmentation-mcp.md.
///
/// The literal twelve in declaration order. `fragmentation.shard` is
/// surfaced as ONE aggregate slot here per the §3.6 row count; the
/// spec's §3.4 names four sub-tools (`shard.open`, `shard.status`,
/// `shard.flush`, `shard.close`) which T2 will split.
pub const TWELVE_TOOL_NAMES: [&str; 12] = [
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

/// A registered MCP tool.
///
/// T1 ships name + description only. T2 adds the per-tool JSON
/// Schema (`inputSchema`) per the MCP 2025-06-18 spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: ToolName,
    pub description: String,
}

impl Tool {
    pub fn new(name: impl Into<ToolName>, description: impl Into<String>) -> Self {
        Tool {
            name: name.into(),
            description: description.into(),
        }
    }
}

/// The tool registry — names + dispatch.
///
/// T1's registry holds only the names; every `tools/call` invocation
/// returns `ERROR_NOT_IMPLEMENTED_YET`. T2 wires bodies; T3 adds the
/// per-tool input schemas; T4 adds git-interop tools.
pub struct ToolRegistry {
    tools: Vec<Tool>,
    by_name: HashMap<ToolName, usize>,
}

impl ToolRegistry {
    /// Build the default registry with the twelve §3.6 tool stubs.
    pub fn with_twelve_tools() -> Self {
        let descriptions: &[(&str, &str)] = &[
            (
                "fragmentation.commit",
                "Atomic content-addressed commit. T2 wires the body.",
            ),
            (
                "fragmentation.snapshot",
                "Working-state checkpoint without commit. T2 wires the body.",
            ),
            (
                "fragmentation.read",
                "Read content by SpectralCoordinate<5>. T2 wires the body.",
            ),
            (
                "fragmentation.diff",
                "Splinter-Merkle structured diff. T3 wires the body.",
            ),
            (
                "fragmentation.merge",
                "Substrate-aware merge (three-way + kintsugi). T3 wires the body.",
            ),
            (
                "fragmentation.branch",
                "Cheap content-addressed branch creation. T3 wires the body.",
            ),
            (
                "fragmentation.refs.list",
                "List refs with their OIDs. T3 wires the body.",
            ),
            (
                "fragmentation.refs.update",
                "CAS-safe ref update. T3 wires the body.",
            ),
            (
                "fragmentation.history",
                "Walk the commit DAG. T3 wires the body.",
            ),
            (
                "fragmentation.search",
                "Query the content-addressed graph. T3 wires the body.",
            ),
            (
                "fragmentation.shard",
                "Session-shard management (open/status/flush/close). T2 splits + wires.",
            ),
            (
                "fragmentation.observe",
                "Algedonic observation channel (Beer-shape). T2 wires the body.",
            ),
        ];
        let mut tools = Vec::with_capacity(12);
        let mut by_name = HashMap::with_capacity(12);
        for (i, (name, desc)) in descriptions.iter().enumerate() {
            let tool = Tool::new(*name, *desc);
            by_name.insert(tool.name.clone(), i);
            tools.push(tool);
        }
        debug_assert_eq!(tools.len(), TWELVE_TOOL_NAMES.len());
        ToolRegistry { tools, by_name }
    }

    pub fn tools(&self) -> &[Tool] {
        &self.tools
    }

    pub fn tool_names(&self) -> Vec<ToolName> {
        self.tools.iter().map(|t| t.name.clone()).collect()
    }

    /// Dispatch a JSON-RPC request through MCP routing.
    ///
    /// Recognised methods in T1:
    /// - `tools/list` — return the twelve tool stubs.
    /// - `tools/call` — return `ERROR_NOT_IMPLEMENTED_YET` for any
    ///   registered tool; `ERROR_METHOD_NOT_FOUND` for unknown ones.
    /// - everything else — `ERROR_METHOD_NOT_FOUND`.
    pub fn dispatch(&self, request: &Request) -> Response {
        match request.method.as_str() {
            "tools/list" => self.handle_tools_list(request),
            "tools/call" => self.handle_tools_call(request),
            "initialize" => self.handle_initialize(request),
            other => Response::err(
                request.id,
                ResponseError::new(ERROR_METHOD_NOT_FOUND, format!("method not found: {other}")),
            ),
        }
    }

    fn handle_tools_list(&self, request: &Request) -> Response {
        let tools_value: Vec<Value> = self
            .tools
            .iter()
            .map(|t| {
                json!({
                    "name": t.name.as_str(),
                    "description": t.description,
                })
            })
            .collect();
        Response::ok(request.id, json!({ "tools": tools_value }))
    }

    fn handle_tools_call(&self, request: &Request) -> Response {
        let Some(params) = request.params.as_ref() else {
            return Response::err(
                request.id,
                ResponseError::new(ERROR_INVALID_PARAMS, "tools/call requires params"),
            );
        };
        let name = params.get("name").and_then(|n| n.as_str());
        let Some(tool_str) = name else {
            return Response::err(
                request.id,
                ResponseError::new(
                    ERROR_INVALID_PARAMS,
                    "tools/call params missing `name` field",
                ),
            );
        };
        let tool_name = ToolName::from(tool_str);
        if !self.by_name.contains_key(&tool_name) {
            return Response::err(
                request.id,
                ResponseError::new(ERROR_METHOD_NOT_FOUND, format!("unknown tool: {tool_str}")),
            );
        }
        // Tool is registered; body is not yet implemented.
        Response::err(
            request.id,
            ResponseError::with_data(
                ERROR_NOT_IMPLEMENTED_YET,
                format!("tool `{tool_str}` registered in T1; body lands in T2+"),
                json!({ "tool": tool_str, "tick": "T1" }),
            ),
        )
    }

    fn handle_initialize(&self, request: &Request) -> Response {
        // MCP 2025-06-18 §initialize: server returns its capabilities.
        // T1 advertises only the tools capability; resources/prompts
        // land later. The version string follows the MCP draft.
        Response::ok(
            request.id,
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "tools": { "listChanged": false }
                },
                "serverInfo": {
                    "name": "fragmentation-mcp",
                    "version": env!("CARGO_PKG_VERSION"),
                }
            }),
        )
    }
}

/// Helper for tests / consumers that want a method-name routing
/// check without constructing a full Request.
pub fn is_known_method(method: &MethodName) -> bool {
    matches!(method.as_str(), "tools/list" | "tools/call" | "initialize")
}
