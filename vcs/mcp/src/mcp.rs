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
//! T5 splits the dispatch by JSON-RPC §4 / §4.1: Requests get a
//! Response on the wire; Notifications get NO response but still
//! advance internal state (notably [`SessionInitialized`] after
//! `notifications/initialized`). The stdio loop only writes when
//! [`Mcp::dispatch_line`] returns `Some(Response)`.
//!
//! T2 ships:
//! - [`Mcp::new`] — construct with the default fifteen-tool
//!   registry (per `ToolRegistry::with_default_tools`).
//! - [`Mcp::dispatch_line`] — synchronous parse + tick + dispatch
//!   for one wire-format line. T5: returns `Option<Response>`.
//! - [`Mcp::run_stdio`] — async stdio read/write loop.
//!
//! Substrate-pull: `[substrate-pull:realize]` — the entry-tick is
//! boundary binding (when the call lands, the substrate's
//! scheduler advances); the capability lives in
//! `fragmentation::hamilton_scheduler`. The Request/Notification
//! seam is also boundary-only: it shapes the wire, not the
//! capability.

use std::sync::atomic::{AtomicBool, Ordering};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::registry::ToolRegistry;
use crate::shard::ShardId;
use crate::types::{SessionInitialized, ToolName};
use crate::wire::{Envelope, Notification, Request, Response, ResponseError, ERROR_PARSE};

/// The MCP server.
///
/// `session_initialized` tracks whether the client has sent
/// `notifications/initialized` — a boolean wire-side state flag
/// per the MCP 2024-11-05+ lifecycle. T5 wires the flag flip; the
/// gate (refuse certain methods before initialized) is a refinement
/// for a later tick.
pub struct Mcp {
    registry: ToolRegistry,
    session_initialized: AtomicBool,
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
            session_initialized: AtomicBool::new(false),
        }
    }

    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }

    pub fn tool_names(&self) -> Vec<ToolName> {
        self.registry.tool_names()
    }

    /// Session initialization state, per the MCP lifecycle.
    pub fn session_initialized(&self) -> SessionInitialized {
        SessionInitialized(self.session_initialized.load(Ordering::Acquire))
    }

    /// Process one wire-format line.
    ///
    /// Returns `Some(Response)` for JSON-RPC requests and
    /// `None` for notifications (per JSON-RPC §4.1: a server MUST
    /// NOT respond to a notification).
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
    pub fn dispatch_line(&self, line: &str) -> Option<Response> {
        match Envelope::parse(line) {
            Ok(Envelope::Request(req)) => {
                self.pre_tick_request(&req);
                Some(self.registry.dispatch(&req))
            }
            Ok(Envelope::Notification(notif)) => {
                self.handle_notification(&notif);
                None
            }
            Err(err) => Some(Response::err(
                crate::types::RequestId::from(0),
                ResponseError::new(ERROR_PARSE, format!("parse error: {err}")),
            )),
        }
    }

    /// Handle a JSON-RPC notification (no response on the wire).
    ///
    /// T5 wires only `notifications/initialized` (the load-bearing
    /// MCP lifecycle event). Other notifications are accepted
    /// silently — the spec mandates no response, even for unknown
    /// notification methods.
    fn handle_notification(&self, notif: &Notification) {
        if notif.method.as_str() == "notifications/initialized" {
            self.session_initialized.store(true, Ordering::Release);
        }
        // Unknown notifications: per JSON-RPC §4.1, silently accept.
        // A future tick may log to stderr; we don't write to stdout.
    }

    /// Pre-route shard tick. Peeks at `params.arguments.shard_id`;
    /// when it parses to a known shard, ticks once. Otherwise
    /// no-op (the body will surface the right error to the wire).
    fn pre_tick_request(&self, request: &Request) {
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

    /// Stdio loop. One request per line; one response per line —
    /// EXCEPT notifications, which read a line but write nothing.
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
            let Some(response) = self.dispatch_line(&line) else {
                // Notification: per JSON-RPC §4.1, write nothing.
                continue;
            };
            let serialized = serde_json::to_string(&response)
                .unwrap_or_else(|e| format!(r#"{{"error":"serialize: {e}"}}"#));
            stdout.write_all(serialized.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }
        Ok(())
    }
}
