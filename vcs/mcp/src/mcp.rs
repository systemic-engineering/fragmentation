//! The `Mcp` server — the dispatch surface + stdio loop.
//!
//! T1 ships:
//! - [`Mcp::new`] — construct with the default twelve-tool registry.
//! - [`Mcp::dispatch_line`] — synchronous parse + dispatch for one
//!   wire-format line. The unit-test seam.
//! - [`Mcp::run_stdio`] — async stdio read/write loop.
//!
//! Per the brief: "the smallest possible thing that boots, lists
//! tools, and tells external callers 'I'm here'."

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::registry::ToolRegistry;
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
            registry: ToolRegistry::with_twelve_tools(),
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
    /// convention). All other paths return a structured response.
    pub fn dispatch_line(&self, line: &str) -> Response {
        match Request::parse(line) {
            Ok(req) => self.registry.dispatch(&req),
            Err(err) => Response::err(
                crate::types::RequestId::from(0),
                ResponseError::new(ERROR_PARSE, format!("parse error: {err}")),
            ),
        }
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
