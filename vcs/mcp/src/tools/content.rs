//! Content tools — `fragmentation.commit` + `fragmentation.read`.
//!
//! T3 of `docs/specs/fragmentation-mcp.md` §3.1. Wires the
//! load-bearing pair: agents commit text content into the shard's
//! `FrgmntStore<Fractal<String>>` and read it back by OID. The
//! round-trip is the acceptance test in `tests/binary_stdio.rs`.
//!
//! # Substrate-pull
//!
//! `[substrate-pull:realize]` — the tools here are boundary Rust at
//! the `tools/call` dispatch altitude. The capability (content-
//! addressed storage via `encoding::encode` → `content_oid` →
//! `FrgmntStore::insert_persistent`) lives in `fragmentation`; the
//! wire-binding work is below.
//!
//! # The simplified `commit` shape
//!
//! `docs/specs/fragmentation-mcp.md` §3.1 defines `commit` as taking
//! `paths: [string]` (multi-path) + a `realtime` admission flag. T3
//! ships the simpler single-path `path + content` variant — the
//! shape the round-trip test exercises. The multi-path / `realtime`
//! axes land in a follow-up tick once the `Pure<G>` admission
//! discipline (per `[[hamilton-scheduler]]` §3.8) is realised in
//! Rust.

use serde_json::{json, Value};

use crate::shard::{ShardContentError, ShardRegistry};
use crate::types::{CommitContent, CommitMessage, ContentPath, OidString, RequestId};
use crate::wire::{Response, ResponseError, ERROR_INTERNAL, ERROR_INVALID_PARAMS};

/// JSON-RPC code for "the requested content (OID) is not in the
/// shard's store, neither in the cache nor on disk". Outside the
/// reserved JSON-RPC range; mirrors `ERROR_NOT_IMPLEMENTED_YET`'s
/// substrate-defined-code discipline.
pub const ERROR_OID_NOT_FOUND: i64 = -32002;

// ---------------------------------------------------------------------------
// fragmentation.commit
// ---------------------------------------------------------------------------

/// Wire-altitude parameters for `fragmentation.commit`.
///
/// Newtypes carry the boundary discipline: `OidString`,
/// `CommitMessage`, `ContentPath`, `CommitContent` — no bare
/// strings cross the tool surface.
#[derive(Debug, Clone)]
pub struct CommitParams {
    pub shard_id: crate::shard::ShardId,
    pub path: ContentPath,
    pub content: CommitContent,
    pub message: CommitMessage,
}

impl CommitParams {
    /// Parse from the `tools/call` arguments object.
    pub fn parse(args: &Value) -> Result<Self, CommitParseError> {
        let shard_str = args
            .get("shard_id")
            .and_then(|v| v.as_str())
            .ok_or(CommitParseError::MissingField("shard_id"))?;
        let shard_id = crate::shard::ShardId::parse(shard_str)
            .map_err(|e| CommitParseError::InvalidShardId(e.to_string()))?;

        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or(CommitParseError::MissingField("path"))?;
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or(CommitParseError::MissingField("content"))?;
        let message = args
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or(CommitParseError::MissingField("message"))?;

        Ok(CommitParams {
            shard_id,
            path: ContentPath::from(path),
            content: CommitContent::from(content),
            message: CommitMessage::from(message),
        })
    }
}

/// Parse-time error for `fragmentation.commit` arguments. Distinct
/// from a substrate-level error: this is "the wire payload is
/// malformed," not "the substrate refused the work."
#[derive(Debug, Clone)]
pub enum CommitParseError {
    MissingField(&'static str),
    InvalidShardId(String),
}

impl std::fmt::Display for CommitParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommitParseError::MissingField(name) => {
                write!(f, "fragmentation.commit missing required field `{name}`")
            }
            CommitParseError::InvalidShardId(msg) => write!(f, "invalid shard_id: {msg}"),
        }
    }
}

/// Dispatch the `fragmentation.commit` tool.
pub fn dispatch_commit(
    request_id: RequestId,
    args: &Value,
    shards: &ShardRegistry,
) -> Response {
    let params = match CommitParams::parse(args) {
        Ok(p) => p,
        Err(e) => {
            return Response::err(
                request_id,
                ResponseError::new(ERROR_INVALID_PARAMS, e.to_string()),
            );
        }
    };

    let outcome = shards.with(&params.shard_id, |shard| {
        shard.commit_content(
            params.path.as_str(),
            params.content.as_str(),
            params.message.as_str(),
        )
    });

    match outcome {
        None => shard_not_found(request_id, &params.shard_id),
        Some(Err(e)) => commit_failed(request_id, e),
        Some(Ok(oid_str)) => {
            let oid = OidString::from(oid_str);
            Response::ok(
                request_id,
                json!({
                    "oid": oid.as_str(),
                    "shard_id": params.shard_id.to_string(),
                    "path": params.path.as_str(),
                    "message": params.message.as_str(),
                }),
            )
        }
    }
}

// ---------------------------------------------------------------------------
// fragmentation.read
// ---------------------------------------------------------------------------

/// Wire-altitude parameters for `fragmentation.read`.
#[derive(Debug, Clone)]
pub struct ReadParams {
    pub shard_id: crate::shard::ShardId,
    pub oid: OidString,
}

impl ReadParams {
    pub fn parse(args: &Value) -> Result<Self, ReadParseError> {
        let shard_str = args
            .get("shard_id")
            .and_then(|v| v.as_str())
            .ok_or(ReadParseError::MissingField("shard_id"))?;
        let shard_id = crate::shard::ShardId::parse(shard_str)
            .map_err(|e| ReadParseError::InvalidShardId(e.to_string()))?;

        let oid = args
            .get("oid")
            .and_then(|v| v.as_str())
            .ok_or(ReadParseError::MissingField("oid"))?;

        Ok(ReadParams {
            shard_id,
            oid: OidString::from(oid),
        })
    }
}

#[derive(Debug, Clone)]
pub enum ReadParseError {
    MissingField(&'static str),
    InvalidShardId(String),
}

impl std::fmt::Display for ReadParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadParseError::MissingField(name) => {
                write!(f, "fragmentation.read missing required field `{name}`")
            }
            ReadParseError::InvalidShardId(msg) => write!(f, "invalid shard_id: {msg}"),
        }
    }
}

/// Dispatch the `fragmentation.read` tool.
pub fn dispatch_read(
    request_id: RequestId,
    args: &Value,
    shards: &ShardRegistry,
) -> Response {
    let params = match ReadParams::parse(args) {
        Ok(p) => p,
        Err(e) => {
            return Response::err(
                request_id,
                ResponseError::new(ERROR_INVALID_PARAMS, e.to_string()),
            );
        }
    };

    let outcome = shards.with(&params.shard_id, |shard| {
        shard.read_content(params.oid.as_str())
    });

    match outcome {
        None => shard_not_found(request_id, &params.shard_id),
        Some(Err(ShardContentError::NotFound { oid })) => Response::err(
            request_id,
            ResponseError::with_data(
                ERROR_OID_NOT_FOUND,
                format!("oid not present in shard: {oid}"),
                json!({
                    "shard_id": params.shard_id.to_string(),
                    "oid": params.oid.as_str(),
                }),
            ),
        ),
        Some(Err(e)) => Response::err(
            request_id,
            ResponseError::new(ERROR_INTERNAL, e.to_string()),
        ),
        Some(Ok(content)) => Response::ok(
            request_id,
            json!({
                "oid": params.oid.as_str(),
                "shard_id": params.shard_id.to_string(),
                "content": content,
            }),
        ),
    }
}

// ---------------------------------------------------------------------------
// Shared helpers.
// ---------------------------------------------------------------------------

fn shard_not_found(id: RequestId, shard: &crate::shard::ShardId) -> Response {
    Response::err(
        id,
        ResponseError::with_data(
            ERROR_INVALID_PARAMS,
            format!("shard not found: {shard}"),
            json!({ "shard_id": shard.to_string() }),
        ),
    )
}

fn commit_failed(id: RequestId, e: ShardContentError) -> Response {
    Response::err(
        id,
        ResponseError::new(ERROR_INTERNAL, format!("commit failed: {e}")),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_shard(reg: &ShardRegistry) -> crate::shard::ShardId {
        reg.open(crate::shard::BudgetMb(8)).expect("open shard")
    }

    #[test]
    fn commit_parse_rejects_missing_fields() {
        let args = json!({"shard_id": uuid::Uuid::new_v4().to_string()});
        assert!(matches!(
            CommitParams::parse(&args),
            Err(CommitParseError::MissingField("path"))
        ));
    }

    #[test]
    fn commit_parse_rejects_invalid_shard_id() {
        let args = json!({
            "shard_id": "not-a-uuid",
            "path": "x",
            "content": "y",
            "message": "z",
        });
        assert!(matches!(
            CommitParams::parse(&args),
            Err(CommitParseError::InvalidShardId(_))
        ));
    }

    #[test]
    fn dispatch_commit_round_trips() {
        let reg = ShardRegistry::new();
        let id = open_shard(&reg);
        let args = json!({
            "shard_id": id.to_string(),
            "path": "hello.txt",
            "content": "hello world",
            "message": "init",
        });
        let response = dispatch_commit(RequestId::from(1), &args, &reg);
        assert!(
            response.error.is_none(),
            "unexpected error: {:?}",
            response.error
        );
        let oid = response
            .result
            .as_ref()
            .and_then(|r| r.get("oid"))
            .and_then(|s| s.as_str())
            .expect("oid in response");
        assert_eq!(oid.len(), 40);

        let read_args = json!({
            "shard_id": id.to_string(),
            "oid": oid,
        });
        let read_response = dispatch_read(RequestId::from(2), &read_args, &reg);
        assert!(read_response.error.is_none());
        let content = read_response
            .result
            .as_ref()
            .and_then(|r| r.get("content"))
            .and_then(|s| s.as_str())
            .expect("content");
        assert_eq!(content, "hello world");
    }

    #[test]
    fn dispatch_read_returns_oid_not_found_for_missing() {
        let reg = ShardRegistry::new();
        let id = open_shard(&reg);
        let args = json!({
            "shard_id": id.to_string(),
            "oid": "0000000000000000000000000000000000000000",
        });
        let response = dispatch_read(RequestId::from(1), &args, &reg);
        let err = response.error.as_ref().expect("expected error");
        assert_eq!(err.code, ERROR_OID_NOT_FOUND);
    }

    #[test]
    fn dispatch_commit_unknown_shard_returns_invalid_params() {
        let reg = ShardRegistry::new();
        let args = json!({
            "shard_id": uuid::Uuid::new_v4().to_string(),
            "path": "x",
            "content": "y",
            "message": "z",
        });
        let response = dispatch_commit(RequestId::from(1), &args, &reg);
        let err = response.error.as_ref().expect("expected error");
        assert_eq!(err.code, ERROR_INVALID_PARAMS);
    }
}
