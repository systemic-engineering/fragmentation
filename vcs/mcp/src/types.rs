//! Wire-altitude newtypes.
//!
//! Per `[[feedback-no-bare-types]]`: no bare primitives cross the
//! MCP wire. `RequestId`, `MethodName`, `ToolName` carry the
//! substrate's discipline into JSON-RPC.

use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 request `id` field.
///
/// Per JSON-RPC 2.0 §4 (inherited by MCP 2025-06-18): `id` MAY be a
/// String, a Number, or NULL. We preserve the on-wire shape across
/// the round trip — a string id in MUST round-trip as a string id
/// out; an integer in as an integer out; null in as null out. T1
/// shipped `u64`-only and that's what made
/// `mcp__frgmnt__fragmentation_read` hang in Claude Code: the
/// client (TypeScript MCP SDK) sends string ids, our parser
/// rejected them, we emitted `id: 0`, the client waited forever
/// for its real id.
///
/// Per `[[feedback-no-bare-types]]`: the enum IS the newtype. No
/// raw `i64`/`String` crosses the wire — the variant tag carries
/// the JSON-type discipline.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RequestId {
    /// JSON number id (integer). JSON-RPC §4 names "Number"; we
    /// hold `i64` (not `u64`) because negative ids are legal on the
    /// wire — some clients use them as a tracking-disambiguation
    /// convention. Whole-valued floats are normalized into this
    /// variant by the deserializer; non-whole floats are rejected.
    Number(i64),
    /// JSON string id. The common shape from JavaScript clients
    /// (Claude Code, the TypeScript MCP SDK) which often use UUIDs.
    Str(String),
    /// JSON null. Per JSON-RPC §5: when the server cannot detect
    /// the client's id (parse-time failure), the response's `id`
    /// MUST be Null. Also accepted on the request side — some
    /// clients explicitly send null when they don't need a paired
    /// response.
    Null,
}

impl From<u64> for RequestId {
    fn from(value: u64) -> Self {
        // `u64` callers in tests/internal code never exceed i64::MAX
        // — the wire range is well below that. The `as` cast is
        // intentional and lossless within the protocol's actual
        // range.
        RequestId::Number(value as i64)
    }
}

impl From<i64> for RequestId {
    fn from(value: i64) -> Self {
        RequestId::Number(value)
    }
}

impl From<i32> for RequestId {
    fn from(value: i32) -> Self {
        // Convenience for integer literals (`RequestId::from(1)`)
        // which default to `i32` in Rust. `i32` always fits in `i64`.
        RequestId::Number(value as i64)
    }
}

impl From<&str> for RequestId {
    fn from(value: &str) -> Self {
        RequestId::Str(value.to_owned())
    }
}

impl From<String> for RequestId {
    fn from(value: String) -> Self {
        RequestId::Str(value)
    }
}

impl Serialize for RequestId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            RequestId::Number(n) => serializer.serialize_i64(*n),
            RequestId::Str(s) => serializer.serialize_str(s),
            RequestId::Null => serializer.serialize_none(),
        }
    }
}

impl<'de> Deserialize<'de> for RequestId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Visitor};
        use std::fmt;

        struct RequestIdVisitor;

        impl<'de> Visitor<'de> for RequestIdVisitor {
            type Value = RequestId;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a JSON-RPC id: string, integer, whole-valued float, or null")
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(RequestId::Number(v))
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                // Above i64::MAX is exotic; clamp by erroring so the
                // wire-shape contract stays predictable.
                i64::try_from(v)
                    .map(RequestId::Number)
                    .map_err(|_| E::custom(format!("request id {v} exceeds i64::MAX")))
            }

            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
                // JavaScript JSON.stringify emits whole-valued floats
                // for integer ids (`3` → `3.0` in some encoders).
                // Accept the whole-valued case losslessly; reject
                // fractional ids — they have no integer meaning at
                // the JSON-RPC altitude.
                if v.is_finite() && v.fract() == 0.0 && v >= i64::MIN as f64 && v <= i64::MAX as f64
                {
                    Ok(RequestId::Number(v as i64))
                } else {
                    Err(E::custom(format!(
                        "request id {v} is not a whole-valued finite number"
                    )))
                }
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(RequestId::Str(v.to_owned()))
            }

            fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(RequestId::Str(v))
            }

            fn visit_borrowed_str<E: de::Error>(self, v: &'de str) -> Result<Self::Value, E> {
                Ok(RequestId::Str(v.to_owned()))
            }

            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(RequestId::Null)
            }

            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(RequestId::Null)
            }
        }

        deserializer.deserialize_any(RequestIdVisitor)
    }
}

/// JSON-RPC `method` field — e.g. `"tools/list"`, `"tools/call"`,
/// `"initialize"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MethodName(String);

impl MethodName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for MethodName {
    fn from(s: &str) -> Self {
        MethodName(s.to_string())
    }
}

impl From<String> for MethodName {
    fn from(s: String) -> Self {
        MethodName(s)
    }
}

/// MCP tool name — e.g. `"fragmentation.commit"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ToolName(String);

impl ToolName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ToolName {
    fn from(s: &str) -> Self {
        ToolName(s.to_string())
    }
}

impl From<String> for ToolName {
    fn from(s: String) -> Self {
        ToolName(s)
    }
}

// ---------------------------------------------------------------------------
// T3 content-tool newtypes.
//
// Per `[[feedback-no-bare-types]]`: no bare `String` / `Vec<u8>` /
// `PathBuf` crosses the content-tool surface. Each carries its own
// altitude on the wire.
// ---------------------------------------------------------------------------

/// Git-compatible content OID — hex-encoded SHA-1, 40 chars.
///
/// Returned by `fragmentation.commit`, consumed by `fragmentation.read`.
/// Internally `fragmentation::fragment::content_oid` produces these as
/// `String`s; the newtype carries the discipline at the wire boundary.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OidString(String);

impl OidString {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl From<String> for OidString {
    fn from(s: String) -> Self {
        OidString(s)
    }
}

impl From<&str> for OidString {
    fn from(s: &str) -> Self {
        OidString(s.to_string())
    }
}

/// Commit message — the human-readable text describing a `fragmentation.commit`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommitMessage(String);

impl CommitMessage {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl From<String> for CommitMessage {
    fn from(s: String) -> Self {
        CommitMessage(s)
    }
}

impl From<&str> for CommitMessage {
    fn from(s: &str) -> Self {
        CommitMessage(s.to_string())
    }
}

/// In-repo path — the logical path a commit's content lives at
/// (e.g. `"src/lib.rs"`).
///
/// `String` rather than `PathBuf` at the wire altitude: the MCP
/// boundary is always UTF-8, never an OS-specific byte sequence,
/// and the substrate paths are virtual (the shard's content
/// store, not the host filesystem). The newtype is the discipline.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContentPath(String);

impl ContentPath {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl From<String> for ContentPath {
    fn from(s: String) -> Self {
        ContentPath(s)
    }
}

impl From<&str> for ContentPath {
    fn from(s: &str) -> Self {
        ContentPath(s.to_string())
    }
}

/// Content payload — the bytes of a commit.
///
/// T3 wires UTF-8 string content only (the spec's `content` field is
/// a string at the JSON-RPC surface; binary content lands when the
/// MCP spec's `"contents":[{ "type":"blob", ... }]` shape is wired).
/// The newtype here lets the GREEN code be unambiguous about what
/// crosses the boundary.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommitContent(String);

impl CommitContent {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl From<String> for CommitContent {
    fn from(s: String) -> Self {
        CommitContent(s)
    }
}

impl From<&str> for CommitContent {
    fn from(s: &str) -> Self {
        CommitContent(s.to_string())
    }
}

/// MCP session initialization state, per the 2024-11-05+ lifecycle.
///
/// `false` until the client sends `notifications/initialized` after
/// the `initialize` response. T5 wires the boolean flip; future
/// ticks may use this to gate methods that the spec restricts to
/// post-initialization.
///
/// Newtype rather than bare `bool` per `[[feedback-no-bare-types]]`:
/// the state is wire-altitude and load-bearing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionInitialized(pub bool);

impl SessionInitialized {
    pub fn as_bool(self) -> bool {
        self.0
    }
}

/// JSON-RPC 2.0 protocol version sentinel.
///
/// The literal `"2.0"`. A unit-like newtype: type-system discipline at
/// the protocol-version altitude. Any `Request` or `Response` carries
/// this; a malformed payload that omits or mis-types `jsonrpc` is
/// rejected at parse-time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JsonRpcVersion;

impl JsonRpcVersion {
    pub const LITERAL: &'static str = "2.0";

    pub fn as_str(&self) -> &'static str {
        Self::LITERAL
    }
}

impl Serialize for JsonRpcVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(Self::LITERAL)
    }
}

impl<'de> Deserialize<'de> for JsonRpcVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s == Self::LITERAL {
            Ok(JsonRpcVersion)
        } else {
            Err(serde::de::Error::custom(format!(
                "expected JSON-RPC version {}, got {s}",
                Self::LITERAL
            )))
        }
    }
}
