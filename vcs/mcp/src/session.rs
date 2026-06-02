//! Session context — gather environment metadata and commit it into a
//! shard at `session/context`.
//!
//! T10 of `docs/specs/fragmentation-mcp.md`. When `fragmentation_shard_open`
//! is called (the underscore-named contextual variant), the MCP layer
//! gathers git branch, git HEAD short hash, cwd, and a UTC timestamp;
//! serialises the bundle to JSON; commits it into the shard; and returns
//! the resulting OID alongside the shard ID.
//!
//! The companion `fragmentation_shard_open_empty` tool skips this step
//! entirely and returns the bare EMPTY shard with no committed content.
//!
//! # Substrate-pull
//!
//! `[substrate-pull:realize]` — the context-gathering is boundary Rust
//! at the `@io` altitude. No new crate deps: git is probed via
//! `std::process::Command`; the timestamp is hand-rolled from
//! `std::time::SystemTime`; the UUID derivation uses
//! `fragmentation::sha::Sha` (SHA-256, already in the dep tree).

use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use fragmentation::sha::{HashAlg, Sha};
use serde::{Deserialize, Serialize};

use crate::shard::{BudgetMb, ShardContentError, ShardId, ShardRegistry};

/// Session metadata gathered from the environment.
///
/// All fields are `String` — the wire-altitude representation. Unknown
/// or error values fall back to `"unknown"` rather than propagating
/// failures upward; the context commit is best-effort, not load-bearing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    pub cwd: String,
    pub git_branch: String,
    pub git_head: String,
    pub timestamp: String,
}

impl SessionContext {
    /// Gather session context from the environment.
    ///
    /// Git calls are best-effort via `std::process::Command`. If the
    /// process is not in a git repo, or git is not on PATH, the field
    /// is set to `"unknown"`.
    pub fn gather() -> Self {
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "unknown".to_string());

        let git_branch = run_git(&["rev-parse", "--abbrev-ref", "HEAD"])
            .unwrap_or_else(|| "unknown".to_string());

        let git_head = run_git(&["rev-parse", "--short", "HEAD"])
            .unwrap_or_else(|| "unknown".to_string());

        let timestamp = format_timestamp_utc(SystemTime::now());

        SessionContext {
            cwd,
            git_branch,
            git_head,
            timestamp,
        }
    }

    /// Serialize to a JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| r#"{"error":"serialize failed"}"#.to_string())
    }

    /// Commit this context into the shard at path `session/context`.
    ///
    /// Returns the OID of the committed content.
    pub fn commit_into(&self, shard: &crate::shard::Shard) -> Result<String, ShardContentError> {
        shard.commit_content("session/context", &self.to_json(), "session bootstrap")
    }

    /// Derive a [`ShardId`] from this context's JSON representation.
    ///
    /// Hashes the JSON with SHA-256 (already available via
    /// `fragmentation::sha::Sha`), decodes the first 32 bytes of the
    /// hex digest, and passes them to `ShardId::from_content(0, &bytes)`.
    /// The result is deterministic for a given context and is guaranteed
    /// to differ from `ShardId::EMPTY` (the empty-input SHA-256 hash
    /// is structurally different from the EMPTY canonical).
    pub fn derive_shard_id(&self) -> ShardId {
        let json = self.to_json();
        let sha = Sha::hash(json.as_bytes());
        // SHA-256 hex is 64 chars = 32 bytes.
        let hex_str = sha.as_str();
        let mut hash_bytes = [0u8; 32];
        // Decode hex pairs into bytes; safe because Sha always produces
        // exactly 64 lowercase hex chars.
        for (i, chunk) in hex_str.as_bytes().chunks(2).enumerate() {
            let hi = hex_nibble(chunk[0]);
            let lo = hex_nibble(chunk[1]);
            hash_bytes[i] = (hi << 4) | lo;
        }
        ShardId::from_content(0, &hash_bytes)
    }
}

/// Open a contextual shard: derive a UUID from session metadata, create
/// the shard under that UUID, commit the session context, and return
/// `(shard_id, context_oid, budget_bytes)`.
///
/// This is the backing implementation for `fragmentation_shard_open`.
pub fn open_contextual(
    registry: &ShardRegistry,
    budget: BudgetMb,
) -> Result<(ShardId, String, u64), ShardContentError> {
    let ctx = SessionContext::gather();
    let derived_id = ctx.derive_shard_id();
    let budget_bytes = budget.as_bytes();

    // Create the shard under the derived id (not EMPTY).
    registry.open_with_id(budget, derived_id)?;

    // Commit the context into the shard.
    let context_oid = registry
        .with(&derived_id, |shard| ctx.commit_into(shard))
        .ok_or_else(|| ShardContentError::Store("shard vanished after open".to_string()))?
        .map_err(|e| ShardContentError::Store(format!("context commit failed: {e}")))?;

    Ok((derived_id, context_oid, budget_bytes))
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Run a git command with the given arguments, capturing trimmed stdout.
/// Returns `None` on any failure (non-zero exit, not in a repo, git
/// not found, etc.).
fn run_git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let s = std::str::from_utf8(&output.stdout).ok()?;
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Format a `SystemTime` as `YYYY-MM-DDTHH:MM:SSZ` (RFC 3339 / ISO 8601
/// UTC, second precision). Hand-rolled to avoid adding `chrono` or
/// `time` as a dependency.
fn format_timestamp_utc(t: SystemTime) -> String {
    let secs = t
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Convert Unix epoch seconds to calendar components.
    // Algorithm: days since epoch, then Gregorian.
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hour = time_of_day / 3600;
    let min = (time_of_day % 3600) / 60;
    let sec = time_of_day % 60;

    // Gregorian calendar from day count (since 1970-01-01).
    let (year, month, day) = days_to_ymd(days);

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hour, min, sec
    )
}

/// Convert days-since-epoch (1970-01-01 = day 0) to (year, month, day).
///
/// Uses the Gregorian algorithm from the public-domain Fliegel-Van Flandern
/// formula adapted for 0-based epoch days.
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Shift epoch from 1970-01-01 to the Gregorian reference point
    // 2000-03-01 (day 11017 in Unix epoch). This simplifies the
    // 400-year Gregorian cycle handling.
    //
    // We work entirely in u64 arithmetic; dates past ~580 million AD
    // overflow, which is fine.
    let days = days + 719468; // shift to 0001-03-01 as day 0 reference
    let era = days / 146097;
    let doe = days % 146097; // day of era [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // year of era [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year [0, 365]
    let mp = (5 * doy + 2) / 153; // month prime [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // day [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // month [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Decode a single ASCII hex nibble to its 0–15 value.
#[inline]
fn hex_nibble(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_context_gather_has_cwd() {
        let ctx = SessionContext::gather();
        assert!(!ctx.cwd.is_empty());
        assert_ne!(ctx.cwd, "unknown");
    }

    #[test]
    fn session_context_to_json_has_cwd_key() {
        let ctx = SessionContext::gather();
        let json = ctx.to_json();
        assert!(json.contains("\"cwd\""), "JSON must have cwd: {json}");
    }

    #[test]
    fn derive_shard_id_differs_from_empty() {
        let ctx = SessionContext::gather();
        let id = ctx.derive_shard_id();
        assert_ne!(id, ShardId::EMPTY, "derived id must not be EMPTY");
    }

    #[test]
    fn derive_shard_id_is_deterministic() {
        let ctx = SessionContext {
            cwd: "/test".to_string(),
            git_branch: "main".to_string(),
            git_head: "abc1234".to_string(),
            timestamp: "2026-06-02T00:00:00Z".to_string(),
        };
        let id1 = ctx.derive_shard_id();
        let id2 = ctx.derive_shard_id();
        assert_eq!(id1, id2);
    }

    #[test]
    fn format_timestamp_utc_epoch_is_1970() {
        let epoch = UNIX_EPOCH;
        let ts = format_timestamp_utc(epoch);
        assert_eq!(ts, "1970-01-01T00:00:00Z");
    }

    #[test]
    fn format_timestamp_utc_known_date() {
        // 2024-01-15T12:30:45Z → Unix: 1705321845
        let t = UNIX_EPOCH + std::time::Duration::from_secs(1_705_321_845);
        let ts = format_timestamp_utc(t);
        assert_eq!(ts, "2024-01-15T12:30:45Z");
    }
}
