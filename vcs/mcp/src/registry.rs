//! Tool registry — the fifteen fragmentation-mcp wire callables.
//!
//! Per `docs/specs/fragmentation-mcp.md` §3.6, the tool surface is
//! twelve CATEGORIES; §3.4 names four sub-tools for the SHARD
//! category, so the net wire callable count is fifteen. T2 splits
//! `fragmentation.shard` into `shard.open` / `.status` / `.flush` /
//! `.close` and wires their bodies against [`crate::shard::ShardRegistry`].
//!
//! Every other tool still returns [`crate::ERROR_NOT_IMPLEMENTED_YET`]
//! from `tools/call`; T3 wires the content surface bodies.
//!
//! Substrate-pull: `[substrate-pull:realize]` — the registry is
//! boundary Rust at the dispatch altitude. The capability (shard
//! state + scheduler ticks) lives in `shard::ShardRegistry`; the
//! registry here is the wire binding.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::shard::{BudgetMb, ShardId, ShardRegistry};
use crate::types::{MethodName, ToolName};
use crate::wire::{Request, Response, ResponseError, ERROR_INVALID_PARAMS, ERROR_METHOD_NOT_FOUND};

/// Substrate-defined error code for "the tool exists but T3+ has not
/// implemented its body yet". OUTSIDE JSON-RPC's reserved
/// `-32768..-32000` range; uses the application-error space the
/// JSON-RPC spec permits.
pub const ERROR_NOT_IMPLEMENTED_YET: i64 = -32001;

/// The fifteen wire callables per §3.4 + §3.6 of fragmentation-mcp.md.
///
/// §3.6 lists twelve CATEGORIES; the SHARD category expands into
/// four sub-tools (`open`/`status`/`flush`/`close`) per §3.4, so
/// the net wire callable count is fifteen.
///
/// Declaration order matches the §3.6 table with the shard category
/// expanded inline.
pub const FIFTEEN_TOOL_NAMES: [&str; 15] = [
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

/// A registered MCP tool.
///
/// T1 ships name + description only. T2 keeps the same shape; the
/// per-tool JSON Schema (`inputSchema`) per the MCP 2025-06-18 spec
/// lands in T3.
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
/// Owns the [`ShardRegistry`] — the shard sub-tools route through
/// it, and the dispatch entry (`Mcp::dispatch_line`) ticks the
/// named shard's scheduler before invoking the tool body.
pub struct ToolRegistry {
    tools: Vec<Tool>,
    by_name: HashMap<ToolName, usize>,
    shards: ShardRegistry,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::with_default_tools()
    }
}

impl ToolRegistry {
    /// Build the default registry with the fifteen wire callables.
    ///
    /// The constructor was named `with_twelve_tools` in T1 (one
    /// aggregate `fragmentation.shard` slot); T2 renames it to
    /// `with_default_tools` once the shard category split into
    /// four sub-tools.
    pub fn with_default_tools() -> Self {
        let descriptions: &[(&str, &str)] = &[
            (
                "fragmentation.commit",
                "Atomic content-addressed commit. T3 wires the body.",
            ),
            (
                "fragmentation.snapshot",
                "Working-state checkpoint without commit. T3 wires the body.",
            ),
            (
                "fragmentation.read",
                "Read content by SpectralCoordinate<5>. T3 wires the body.",
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
                "fragmentation.shard.open",
                "Allocate a new session shard with a budget. Returns ShardId.",
            ),
            (
                "fragmentation.shard.status",
                "Diagnostic snapshot of a shard (budget, hot/cold/total, tick count).",
            ),
            (
                "fragmentation.shard.flush",
                "Force a flush of a shard's hot cache to disk.",
            ),
            (
                "fragmentation.shard.close",
                "Close a session shard; release in-RAM state.",
            ),
            (
                "fragmentation.observe",
                "Algedonic observation channel (Beer-shape). T3 wires the body.",
            ),
        ];
        let mut tools = Vec::with_capacity(15);
        let mut by_name = HashMap::with_capacity(15);
        for (i, (name, desc)) in descriptions.iter().enumerate() {
            let tool = Tool::new(*name, *desc);
            by_name.insert(tool.name.clone(), i);
            tools.push(tool);
        }
        debug_assert_eq!(tools.len(), FIFTEEN_TOOL_NAMES.len());
        ToolRegistry {
            tools,
            by_name,
            shards: ShardRegistry::new(),
        }
    }

    pub fn tools(&self) -> &[Tool] {
        &self.tools
    }

    pub fn tool_names(&self) -> Vec<ToolName> {
        self.tools.iter().map(|t| t.name.clone()).collect()
    }

    /// Borrowing accessor for the shard registry — needed by the
    /// `Mcp` wrapper to inspect shard state.
    pub fn shards(&self) -> &ShardRegistry {
        &self.shards
    }

    /// Dispatch a JSON-RPC request through MCP routing.
    ///
    /// Recognised methods in T2:
    /// - `tools/list` — return the fifteen tool stubs.
    /// - `tools/call` — route shard sub-tools to the
    ///   [`ShardRegistry`]; every other tool returns
    ///   `ERROR_NOT_IMPLEMENTED_YET` (T3 wires the bodies).
    /// - `initialize` — capability negotiation.
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

        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));

        // Shard sub-tools route to the live ShardRegistry; everything
        // else is still T1's not-implemented-yet stub.
        match tool_str {
            "fragmentation.shard.open" => self.tool_shard_open(request, &arguments),
            "fragmentation.shard.status" => self.tool_shard_status(request, &arguments),
            "fragmentation.shard.flush" => self.tool_shard_flush(request, &arguments),
            "fragmentation.shard.close" => self.tool_shard_close(request, &arguments),
            _ => Response::err(
                request.id,
                ResponseError::with_data(
                    ERROR_NOT_IMPLEMENTED_YET,
                    format!("tool `{tool_str}` registered in T1; body lands in T3+"),
                    json!({ "tool": tool_str, "tick": "T2" }),
                ),
            ),
        }
    }

    // -----------------------------------------------------------------
    // Shard sub-tool bodies — the load-bearing T2 wiring.
    // -----------------------------------------------------------------

    fn tool_shard_open(&self, request: &Request, args: &Value) -> Response {
        let Some(budget_mb_raw) = args.get("budget_mb").and_then(|v| v.as_u64()) else {
            return Response::err(
                request.id,
                ResponseError::new(
                    ERROR_INVALID_PARAMS,
                    "fragmentation.shard.open requires `budget_mb` (u64)",
                ),
            );
        };
        let budget = BudgetMb(budget_mb_raw);
        let id = self.shards.open(budget);
        Response::ok(
            request.id,
            json!({
                "shard_id": id.to_string(),
                "budget_bytes": budget.as_bytes(),
            }),
        )
    }

    fn tool_shard_status(&self, request: &Request, args: &Value) -> Response {
        let id = match parse_shard_id_arg(args) {
            Ok(id) => id,
            Err(err) => return err.into_response(request.id),
        };
        // `Mcp::dispatch_line` already pre-ticked the shard at the
        // dispatch entry; the body reads post-tick state via `with`.
        let snapshot = self
            .shards
            .with(&id, |shard| (shard.budget(), shard.tick_count()));
        let Some((budget, tick_count)) = snapshot else {
            return shard_not_found(request.id, &id);
        };
        Response::ok(
            request.id,
            json!({
                "shard_id": id.to_string(),
                "budget_bytes": budget.as_u64(),
                "hot_bytes": 0,
                "cold_bytes": 0,
                "total_bytes": 0,
                "tick_count": tick_count.as_u64(),
                "scheduler": "stub",
            }),
        )
    }

    fn tool_shard_flush(&self, request: &Request, args: &Value) -> Response {
        let id = match parse_shard_id_arg(args) {
            Ok(id) => id,
            Err(err) => return err.into_response(request.id),
        };
        // Stub: there's no content-bearing FrgmntStore in T2; flush
        // is a zero-eviction report. The dispatch entry already
        // ticked the scheduler; the body reads post-tick state.
        // T3+ wires the body once content tools land.
        let ticked = self.shards.with(&id, |shard| shard.tick_count());
        let Some(tick) = ticked else {
            return shard_not_found(request.id, &id);
        };
        Response::ok(
            request.id,
            json!({
                "shard_id": id.to_string(),
                "evicted_count": 0,
                "bytes_released": 0,
                "tick_count": tick.as_u64(),
            }),
        )
    }

    fn tool_shard_close(&self, request: &Request, args: &Value) -> Response {
        let id = match parse_shard_id_arg(args) {
            Ok(id) => id,
            Err(err) => return err.into_response(request.id),
        };
        // No tick on close: the shard is about to be removed.
        let removed = self.shards.close(&id);
        if !removed {
            return shard_not_found(request.id, &id);
        }
        Response::ok(
            request.id,
            json!({
                "shard_id": id.to_string(),
                "closed": true,
            }),
        )
    }

    fn handle_initialize(&self, request: &Request) -> Response {
        // MCP 2025-06-18 §initialize: server returns its capabilities.
        // T2 advertises only the tools capability; resources/prompts
        // land later. The version string follows the MCP draft.
        Response::ok(
            request.id,
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "tools": { "listChanged": false }
                },
                "serverInfo": {
                    "name": "frgmnt",
                    "version": env!("CARGO_PKG_VERSION"),
                }
            }),
        )
    }
}

// ---------------------------------------------------------------------------
// Shared shard-id parsing — every shard sub-tool needs this.
// ---------------------------------------------------------------------------

/// Error from `parse_shard_id_arg`. Carries the message it would
/// surface; the caller stamps the request id on conversion.
struct ShardIdArgError {
    message: String,
}

impl ShardIdArgError {
    fn into_response(self, id: crate::types::RequestId) -> Response {
        Response::err(id, ResponseError::new(ERROR_INVALID_PARAMS, self.message))
    }
}

fn parse_shard_id_arg(args: &Value) -> Result<ShardId, ShardIdArgError> {
    let Some(shard_str) = args.get("shard_id").and_then(|v| v.as_str()) else {
        return Err(ShardIdArgError {
            message: "missing required `shard_id` (string) argument".to_string(),
        });
    };
    ShardId::parse(shard_str).map_err(|e| ShardIdArgError {
        message: format!("invalid shard_id: {e}"),
    })
}

fn shard_not_found(id: crate::types::RequestId, shard: &ShardId) -> Response {
    // INVALID_PARAMS, not METHOD_NOT_FOUND: the method exists, the
    // PARAMETERS reference a shard that isn't open. Per JSON-RPC's
    // -32602 contract.
    Response::err(
        id,
        ResponseError::with_data(
            ERROR_INVALID_PARAMS,
            format!("shard not found: {shard}"),
            json!({ "shard_id": shard.to_string() }),
        ),
    )
}

/// Helper for tests / consumers that want a method-name routing
/// check without constructing a full Request.
pub fn is_known_method(method: &MethodName) -> bool {
    matches!(method.as_str(), "tools/list" | "tools/call" | "initialize")
}
