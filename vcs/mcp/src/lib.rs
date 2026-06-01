//! fragmentation-mcp — the MCP server for content-addressed agent
//! workflows.
//!
//! Per `docs/specs/fragmentation-mcp.md`. T1 of §9 lands the
//! workspace member + the JSON-RPC stdio wire + the twelve-tool
//! registry stub. Every tool registered here returns
//! [`ERROR_NOT_IMPLEMENTED_YET`] from `tools/call`; T2+ wires the
//! bodies against fragmentation primitives.
//!
//! # Public surface (T1)
//!
//! - [`Mcp`] — the server (default registry + stdio loop).
//! - [`ToolRegistry`] — JSON-RPC method dispatch.
//! - [`Request`], [`Response`], [`ResponseError`] — the wire.
//! - [`RequestId`], [`MethodName`], [`ToolName`] — newtypes per
//!   `[[feedback-no-bare-types]]`. No bare `u64`/`String` crosses
//!   the wire.
//! - [`JSON_RPC_VERSION`] — the protocol-version sentinel value.
//! - [`TWELVE_TOOL_NAMES`] — the §3.6 twelve.
//! - [`ERROR_NOT_IMPLEMENTED_YET`] — the substrate code returned for
//!   every `tools/call` in T1.
//!
//! # Substrate-pull discipline
//!
//! This crate is boundary Rust at the `@io` altitude. The capability
//! lives in `fragmentation` (the substrate). The wire is binding,
//! not capability. The `fragmentation` dependency is direct; the
//! `fragmentation-git` dependency is feature-gated to `git-interop`
//! (T4). `prism_core` stays dependency-free.

pub mod mcp;
pub mod registry;
pub mod types;
pub mod wire;

// Re-exports — flat public surface for callers.
pub use mcp::Mcp;
pub use registry::{
    is_known_method, Tool, ToolRegistry, ERROR_NOT_IMPLEMENTED_YET, TWELVE_TOOL_NAMES,
};
pub use types::{JsonRpcVersion, MethodName, RequestId, ToolName};
pub use wire::{
    ParseError, Request, Response, ResponseError, ERROR_INTERNAL, ERROR_INVALID_PARAMS,
    ERROR_INVALID_REQUEST, ERROR_METHOD_NOT_FOUND, ERROR_PARSE,
};

/// The JSON-RPC 2.0 version sentinel as a value. Use this when
/// constructing [`Request`]/[`Response`] literals; the type system
/// keeps the protocol version single-valued.
pub const JSON_RPC_VERSION: JsonRpcVersion = JsonRpcVersion;
