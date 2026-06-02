//! fragmentation-mcp — the MCP server for content-addressed agent
//! workflows.
//!
//! Per `docs/specs/fragmentation-mcp.md`. T1 of §9 landed the
//! workspace member + the JSON-RPC stdio wire + the twelve-tool
//! registry stub. T2 of §9 splits the shard category into four
//! sub-tools (net fifteen wire callables), wires their bodies
//! against [`shard::ShardRegistry`], and ticks the named shard's
//! `HamiltonScheduler` at the dispatch entry. T3+ wires the
//! content surface bodies.
//!
//! # Public surface (T2)
//!
//! - [`Mcp`] — the server (default registry + stdio loop + entry
//!   tick).
//! - [`ToolRegistry`] — JSON-RPC method dispatch + the shard
//!   sub-tool wiring.
//! - [`Request`], [`Response`], [`ResponseError`] — the wire.
//! - [`RequestId`], [`MethodName`], [`ToolName`] — newtypes per
//!   `[[feedback-no-bare-types]]`. No bare `u64`/`String` crosses
//!   the wire.
//! - [`ShardId`], [`BudgetMb`], [`Shard`], [`ShardRegistry`] — the
//!   T2 shard surface.
//! - [`JSON_RPC_VERSION`] — the protocol-version sentinel value.
//! - [`FIFTEEN_TOOL_NAMES`] — the §3.4 + §3.6 fifteen.
//! - [`ERROR_NOT_IMPLEMENTED_YET`] — the substrate code returned
//!   for every `tools/call` whose body hasn't been wired yet
//!   (T3+).
//!
//! # Substrate-pull discipline
//!
//! This crate is boundary Rust at the `@io` altitude. The capability
//! lives in `fragmentation` (the substrate). The wire is binding,
//! not capability. The `fragmentation` dependency is direct; the
//! `fragmentation-git` dependency is feature-gated to `git-interop`
//! (T4). `prism_core` stays dependency-free.
//!
//! # Binary
//!
//! Built as `frgmnt` per Alex's directive (renamed from
//! `fragmentation-mcp` in T2). Single-word; PATH-friendly; matches
//! the `frgmt-git` neighbour-binary's pattern.

pub mod mcp;
pub mod registry;
pub mod shard;
pub mod tools;
pub mod types;
pub mod wire;

// Re-exports — flat public surface for callers.
pub use mcp::Mcp;
pub use registry::{
    is_known_method, Tool, ToolRegistry, ERROR_NOT_IMPLEMENTED_YET, FIFTEEN_TOOL_NAMES,
};
pub use shard::{BudgetMb, Shard, ShardContentError, ShardId, ShardIdParseError, ShardRegistry};
pub use tools::content::ERROR_OID_NOT_FOUND;
pub use types::{
    CommitContent, CommitMessage, ContentPath, JsonRpcVersion, MethodName, OidString, RequestId,
    SessionInitialized, ToolName,
};
pub use wire::{
    Envelope, Notification, ParseError, Request, Response, ResponseError, ERROR_INTERNAL,
    ERROR_INVALID_PARAMS, ERROR_INVALID_REQUEST, ERROR_METHOD_NOT_FOUND, ERROR_PARSE,
};

/// The JSON-RPC 2.0 version sentinel as a value. Use this when
/// constructing [`Request`]/[`Response`] literals; the type system
/// keeps the protocol version single-valued.
pub const JSON_RPC_VERSION: JsonRpcVersion = JsonRpcVersion;
