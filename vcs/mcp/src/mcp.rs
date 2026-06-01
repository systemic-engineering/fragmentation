//! The `Mcp` server — the dispatch surface + stdio loop.
//!
//! T2 wires the per-call shard tick at the dispatch entry per
//! `docs/specs/fragmentation-mcp.md` §9 T2: "every other MCP tool
//! call ticks the shard's scheduler at the ENTRY of the call."
//! [`Mcp::dispatch_line`] peeks at `params.arguments.shard_id`;
//! when present and parseable, the named shard's scheduler is
//! ticked before the registry dispatches the call. The body
//! reads state (post-tick) and returns.
//!
//! T2 ships:
//! - [`Mcp::new`] — construct with the default fifteen-tool
//!   registry (per `ToolRegistry::with_default_tools`).
//! - [`Mcp::dispatch_line`] — synchronous parse + tick + dispatch
//!   for one wire-format line.
//! - [`Mcp::run_stdio`] — async stdio read/write loop.
//!
//! Substrate-pull: `[substrate-pull:realize]` — the entry-tick is
//! boundary binding (when the call lands, the substrate's
//! scheduler advances); the capability lives in
//! `fragmentation::hamilton_scheduler`.

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::registry::ToolRegistry;
use crate::shard::ShardId;
use crate::types::ToolName;
use crate::wire::{Request, Response, ResponseError, ERROR_PARSE};

/// The MCP server.
pub struct Mcp {
    registry: ToolRegistry,
}

impl Default for Mcp {
    fn default() -> Self {
        Self::new()
    }
}

impl Mcp {
    pub fn new() -> Self {
        Mcp {
            registry: ToolRegistry::with_default_tools(),
        }
    }

    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }

    pub fn tool_names(&self) -> Vec<ToolName> {
        self.registry.tool_names()
    }

    /// Process one wire-format line.
    ///
    /// Parse errors yield a JSON-RPC parse-error response with
    /// `id = 0` (per JSON-RPC §5: the server cannot recover the id
    /// from a malformed request, so it returns `null`/0 by
    /// convention).
    ///
    /// On well-formed input: if `params.arguments.shard_id` is
    /// present and parseable to a known [`ShardId`], the named
    /// shard's `HamiltonScheduler` is ticked BEFORE the registry
    /// dispatches the call (per the §9 T2 reload-contract
    /// discipline). Unknown or unparseable shard ids are not
    /// ticked here; the body emits the right INVALID_PARAMS.
    pub fn dispatch_line(&self, line: &str) -> Response {
        match Request::parse(line) {
            Ok(req) => {
                self.pre_tick(&req);
                self.registry.dispatch(&req)
            }
            Err(err) => Response::err(
                crate::types::RequestId::from(0),
                ResponseError::new(ERROR_PARSE, format!("parse error: {err}")),
            ),
        }
    }

    /// Pre-route shard tick. Peeks at `params.arguments.shard_id`;
    /// when it parses to a known shard, ticks once. Otherwise
    /// no-op (the body will surface the right error to the wire).
    fn pre_tick(&self, request: &Request) {
        // Only `tools/call` carries arguments; other methods skip
        // the pre-tick.
        if request.method.as_str() != "tools/call" {
            return;
        }
        let Some(params) = request.params.as_ref() else {
            return;
        };
        let Some(shard_str) = params
            .get("arguments")
            .and_then(|a| a.get("shard_id"))
            .and_then(|v| v.as_str())
        else {
            return;
        };
        let Ok(id) = ShardId::parse(shard_str) else {
            return;
        };
        // tick_then_with returns None if the shard is unknown — the
        // body will emit INVALID_PARAMS with the right message; we
        // simply skip the tick here.
        let _ = self.registry.shards().tick_then_with(&id, |_| ());
    }

    /// Stdio loop. One request per line; one response per line.
    ///
    /// MCP 2025-06-18 stdio: newline-delimited JSON. The line-based
    /// framing is what mcp-server-git and the official reference
    /// implementations use; matches the Anthropic spec's stdio
    /// transport.
    pub async fn run_stdio(&self) -> std::io::Result<()> {
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut lines = BufReader::new(stdin).lines();
        while let Some(line) = lines.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }
            let response = self.dispatch_line(&line);
            let serialized = serde_json::to_string(&response)
                .unwrap_or_else(|e| format!(r#"{{"error":"serialize: {e}"}}"#));
            stdout.write_all(serialized.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }
        Ok(())
    }
}
