//! The per-session `Shard` + its `ShardRegistry`.
//!
//! T2 of `docs/specs/fragmentation-mcp.md` (§4 — HamiltonScheduler
//! at the agent altitude; §3.4 — the four shard sub-tools).
//!
//! # What this module owns
//!
//! - [`ShardId`] — the wire-altitude shard handle. `uuid::Uuid v4`
//!   under the hood; serializes as a hyphenated string.
//! - [`BudgetMb`] / [`BudgetBytes`] — newtypes for the budget. No
//!   bare `u64` crosses the shard surface (per
//!   `[[feedback-no-bare-types]]`).
//! - [`Shard`] — per-session state: the configured budget, the
//!   stub `HamiltonScheduler` instance, metadata (created-at,
//!   placeholder for the transit accumulator). T3+ adds the
//!   `FrgmntStore` body once content tools land.
//! - [`ShardRegistry`] — thread-safe map of `ShardId -> Shard`,
//!   the dispatch layer's source of truth.
//!
//! # Substrate-pull
//!
//! `[substrate-pull:realize]` — the shard surface is boundary
//! Rust at the `@io` altitude. The capability (content storage,
//! scheduler discipline) lives in the substrate. The Rust here
//! is binding + wire; T3+ wires the body once `FrgmntStore`
//! instantiations land for the content tools.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use std::time::SystemTime;

use fragmentation::hamilton_scheduler::{BudgetBytes, HamiltonScheduler, TickCount, TickReport};
use fragmentation::sha::Sha;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Newtypes — no bare primitives cross the shard surface.
// ---------------------------------------------------------------------------

/// Per-session shard handle.
///
/// `uuid::Uuid v4` under the hood. The hyphenated 36-char string
/// form is what crosses the wire; the byte form is for in-process
/// equality and the `HashMap` key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ShardId(pub Uuid);

impl ShardId {
    /// Generate a fresh v4 UUID. Cryptographically random; no
    /// monotonic-counter footgun.
    pub fn new() -> Self {
        ShardId(Uuid::new_v4())
    }

    /// Parse from the hyphenated string form. Returns
    /// [`ShardIdParseError`] when the input is not a valid v4 UUID.
    pub fn parse(s: &str) -> Result<Self, ShardIdParseError> {
        Uuid::parse_str(s)
            .map(ShardId)
            .map_err(|e| ShardIdParseError(e.to_string()))
    }
}

impl Default for ShardId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ShardId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Hyphenated 36-char form — the wire literal.
        std::fmt::Display::fmt(&self.0.hyphenated(), f)
    }
}

/// Parse error for [`ShardId::parse`].
#[derive(Debug, Clone)]
pub struct ShardIdParseError(pub String);

impl std::fmt::Display for ShardIdParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ShardId parse error: {}", self.0)
    }
}

impl std::error::Error for ShardIdParseError {}

/// Shard budget in megabytes — the wire-altitude unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BudgetMb(pub u64);

impl BudgetMb {
    /// Convert to bytes (`mb * 1024 * 1024`).
    pub fn as_bytes(self) -> u64 {
        self.0.saturating_mul(1024 * 1024)
    }

    /// Convert to the substrate's [`BudgetBytes`] newtype.
    pub fn into_budget_bytes(self) -> BudgetBytes {
        BudgetBytes::new(self.as_bytes())
    }
}

// ---------------------------------------------------------------------------
// Shard — per-session state.
// ---------------------------------------------------------------------------

/// One session shard.
///
/// T2 carries:
/// - the configured budget (in bytes; the wire takes MB);
/// - the stub [`HamiltonScheduler`] instance — `tick()` is
///   incrementing only, no real hot/cold accounting yet
///   (`TODO(T-scheduler-impl)` in `fragmentation::hamilton_scheduler`);
/// - the creation timestamp (best-effort `SystemTime`;
///   monotonic-clock altitude is a T3+ refinement).
///
/// T3 adds the `FrgmntStore<BodyEntry<H>>` body once the content
/// tools land. The shape here is the minimal one T2's surface
/// needs.
pub struct Shard {
    budget: BudgetBytes,
    scheduler: HamiltonScheduler<Sha>,
    created_at: SystemTime,
}

impl Shard {
    /// Construct a shard with the given budget.
    pub fn new(budget: BudgetBytes) -> Self {
        Shard {
            scheduler: HamiltonScheduler::new(budget),
            budget,
            created_at: SystemTime::now(),
        }
    }

    /// Configured byte budget.
    pub fn budget(&self) -> BudgetBytes {
        self.budget
    }

    /// Current scheduler tick count.
    pub fn tick_count(&self) -> TickCount {
        self.scheduler.tick_count()
    }

    /// Tick the shard's scheduler once.
    pub fn tick(&mut self) -> TickReport {
        self.scheduler.tick()
    }

    /// Creation timestamp.
    pub fn created_at(&self) -> SystemTime {
        self.created_at
    }
}

// ---------------------------------------------------------------------------
// ShardRegistry — the dispatch layer's source of truth.
// ---------------------------------------------------------------------------

/// Thread-safe map of `ShardId -> Shard`.
///
/// Uses a single `Mutex<HashMap>` rather than `DashMap` for T2:
/// the tokio runtime is current-thread (per the binary's choice)
/// and the dispatch path is serial. T3+ may swap to DashMap if
/// concurrent dispatch lands.
pub struct ShardRegistry {
    inner: Mutex<HashMap<ShardId, Shard>>,
}

impl Default for ShardRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ShardRegistry {
    pub fn new() -> Self {
        ShardRegistry {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Open a new shard with the given budget. Returns the freshly-
    /// minted `ShardId`.
    pub fn open(&self, budget: BudgetMb) -> ShardId {
        let id = ShardId::new();
        let shard = Shard::new(budget.into_budget_bytes());
        self.lock().insert(id, shard);
        id
    }

    /// Run `f` against a shard's mutable handle, ticking the
    /// scheduler once BEFORE invoking `f`. Returns `None` if the
    /// shard is unknown.
    ///
    /// This is the load-bearing T2 contract per §9 T2 acceptance:
    /// "every other MCP tool call ticks the shard's scheduler at
    /// the ENTRY of the call". The tick happens before `f` runs.
    pub fn tick_then_with<F, R>(&self, id: &ShardId, f: F) -> Option<R>
    where
        F: FnOnce(&mut Shard) -> R,
    {
        let mut guard = self.lock();
        let shard = guard.get_mut(id)?;
        let _ = shard.tick();
        Some(f(shard))
    }

    /// Run `f` against a shard's mutable handle WITHOUT ticking.
    /// Used by `shard.close` to avoid ticking a shard we're about
    /// to remove.
    pub fn with<F, R>(&self, id: &ShardId, f: F) -> Option<R>
    where
        F: FnOnce(&mut Shard) -> R,
    {
        let mut guard = self.lock();
        let shard = guard.get_mut(id)?;
        Some(f(shard))
    }

    /// Remove a shard. Returns `true` if the shard existed.
    pub fn close(&self, id: &ShardId) -> bool {
        self.lock().remove(id).is_some()
    }

    /// True if a shard with the given id is present.
    pub fn contains(&self, id: &ShardId) -> bool {
        self.lock().contains_key(id)
    }

    /// Number of open shards.
    pub fn len(&self) -> usize {
        self.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<ShardId, Shard>> {
        // A poisoned mutex here means another thread panicked while
        // mutating the registry. T2 panics through; a recovery
        // discipline (per `[[hamilton-scheduler]]` §3.5 Explorer)
        // is a T3+ concern.
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

// ---------------------------------------------------------------------------
// Tests — module-local.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_mb_converts_to_bytes() {
        assert_eq!(BudgetMb(0).as_bytes(), 0);
        assert_eq!(BudgetMb(1).as_bytes(), 1024 * 1024);
        assert_eq!(BudgetMb(64).as_bytes(), 64 * 1024 * 1024);
    }

    #[test]
    fn shard_id_display_round_trips() {
        let id = ShardId::new();
        let s = id.to_string();
        assert_eq!(s.len(), 36);
        let back = ShardId::parse(&s).expect("parse");
        assert_eq!(back, id);
    }

    #[test]
    fn shard_id_parse_rejects_garbage() {
        assert!(ShardId::parse("").is_err());
        assert!(ShardId::parse("not-a-uuid").is_err());
        assert!(ShardId::parse("12345").is_err());
    }

    #[test]
    fn registry_open_then_close() {
        let reg = ShardRegistry::new();
        let id = reg.open(BudgetMb(16));
        assert!(reg.contains(&id));
        assert_eq!(reg.len(), 1);
        assert!(reg.close(&id));
        assert!(!reg.contains(&id));
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn tick_then_with_increments_tick() {
        let reg = ShardRegistry::new();
        let id = reg.open(BudgetMb(8));
        let t1 = reg
            .tick_then_with(&id, |s| s.tick_count())
            .expect("tick 1");
        let t2 = reg
            .tick_then_with(&id, |s| s.tick_count())
            .expect("tick 2");
        assert!(t2.as_u64() > t1.as_u64());
    }

    #[test]
    fn tick_then_with_unknown_returns_none() {
        let reg = ShardRegistry::new();
        let id = ShardId::new();
        let r: Option<TickCount> = reg.tick_then_with(&id, |s| s.tick_count());
        assert!(r.is_none());
    }
}
