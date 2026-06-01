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
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use std::time::SystemTime;

use fragmentation::encoding;
use fragmentation::fragment::{self, ContentAddressed, Fractal};
use fragmentation::frgmnt_store::FrgmntStore;
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

/// Error returned from shard-content operations (commit / read).
#[derive(Debug, Clone)]
pub enum ShardContentError {
    /// The requested OID was not present in this shard's store.
    NotFound { oid: String },
    /// Underlying store I/O failure (e.g. tempdir creation).
    Store(String),
}

impl std::fmt::Display for ShardContentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShardContentError::NotFound { oid } => write!(f, "content not found for oid {oid}"),
            ShardContentError::Store(msg) => write!(f, "store error: {msg}"),
        }
    }
}

impl std::error::Error for ShardContentError {}

/// One session shard.
///
/// T3 wires the real content-bearing surface:
/// - the configured budget (in bytes; the wire takes MB);
/// - the stub [`HamiltonScheduler`] instance — `tick()` increments;
///   real hot/cold accounting is `TODO(T-scheduler-impl)` in
///   `fragmentation::hamilton_scheduler`;
/// - the [`FrgmntStore<Fractal<String>>`] body. Backed by a per-shard
///   `.frgmnt-<shard_id>` directory under `std::env::temp_dir()`.
///   T4 wires a `--repo PATH` flag for caller-supplied roots.
/// - the creation timestamp (best-effort `SystemTime`;
///   monotonic-clock altitude is a T3+ refinement).
///
/// `Body = prism + glass + AST` (per `docs/specs/fragmentation-mcp.md`)
/// hasn't landed yet; `Fractal<String>` is the simplest extant content
/// shape and round-trips git-compatibly through `content_oid`.
///
/// [substrate-pull:realize] — the shard surface is boundary Rust at
/// the `@io` altitude. The capability (content-addressed storage,
/// scheduler discipline) lives in the substrate; the Rust here is
/// binding + wire.
pub struct Shard {
    budget: BudgetBytes,
    scheduler: HamiltonScheduler<Sha>,
    store: FrgmntStore<Fractal<String>>,
    store_root: PathBuf,
    created_at: SystemTime,
}

impl Shard {
    /// Construct a shard with the given budget.
    ///
    /// Allocates a per-shard `.frgmnt-<shard_id>` directory under
    /// `std::env::temp_dir()` for disk spillover. Returns an error
    /// if the directory cannot be created.
    pub fn new(budget: BudgetBytes, shard_id: ShardId) -> Result<Self, ShardContentError> {
        let store_root = std::env::temp_dir().join(format!(".frgmnt-{shard_id}"));
        let root_str = store_root
            .to_str()
            .ok_or_else(|| ShardContentError::Store("shard root path is not UTF-8".to_string()))?;
        // Cap usize from the (potentially-huge) u64 budget. On 64-bit
        // targets this is a no-op; on 32-bit it saturates at
        // `usize::MAX`, which is fine — a 32-bit host can't allocate
        // more than that anyway.
        let cap_usize = usize::try_from(budget.as_u64()).unwrap_or(usize::MAX);
        let store = FrgmntStore::<Fractal<String>>::open(root_str, cap_usize)
            .map_err(|e| ShardContentError::Store(e.to_string()))?;
        Ok(Shard {
            scheduler: HamiltonScheduler::new(budget),
            budget,
            store,
            store_root,
            created_at: SystemTime::now(),
        })
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

    /// Borrow the shard's content store.
    pub fn store(&self) -> &FrgmntStore<Fractal<String>> {
        &self.store
    }

    /// On-disk root for the shard's `.frgmnt/` directory.
    pub fn store_root(&self) -> &PathBuf {
        &self.store_root
    }

    /// Cached entry count (the hot side of the store).
    pub fn hot_entries(&self) -> usize {
        self.store.cached_len()
    }

    /// Bytes resident in the cache (the hot side of the store).
    pub fn hot_bytes(&self) -> usize {
        self.store.total_bytes()
    }

    /// Commit content into the shard's store, returning the content OID.
    ///
    /// T3 ships the single-path variant: text content at one logical
    /// `path` becomes one `Fractal<String>` tree via
    /// `encoding::encode`; the tree's `content_oid` is the git-
    /// compatible blob/tree SHA-1 hex string. `insert_persistent`
    /// lands the fragment in the cache with disk spillover when the
    /// budget would be exceeded.
    ///
    /// `path` and `message` are accepted at the wire altitude per
    /// `docs/specs/fragmentation-mcp.md` §3.1; T3 lands the OID
    /// computation and storage — the `path` is preserved on the
    /// `Fractal`'s root `Ref::label` for future `read(path=...)`
    /// navigation, and `message` is reserved for the commit-graph
    /// layer that T-future wires on top of `commit::Draft`.
    pub fn commit_content(
        &self,
        path: &str,
        content: &str,
        _message: &str,
    ) -> Result<String, ShardContentError> {
        // Build a Fractal<String> from the content. `encoding::encode`
        // produces a paragraph/sentence/word/char tree; the ROOT data
        // is the full content string. `content_oid` walks the tree
        // and returns the git-compatible OID of the ROOT data blob
        // (when it's a single shard) or the tree (when there's
        // structure).
        let _ = path; // reserved — see doc-comment.
        let fragment = encoding::encode(content);
        let oid = fragment::content_oid(&fragment);
        // `std::mem::size_of_val` undercounts heap allocations — the
        // String + child Vec are not in the stack-size summary. T3's
        // accounting matches the substrate's existing test discipline
        // (e.g. `frgmnt_store.rs::insert_persistent_evicts_to_disk`
        // uses a manual `50` literal). The 1-shard content fits in
        // the default 64 MB budget by an enormous margin; T-future
        // wires a real `Encode`-aware byte-size estimator.
        let approx_size = content.len() + path.len() + 64;
        self.store
            .insert_persistent(oid.clone(), fragment, approx_size);
        Ok(oid)
    }

    /// Read content by OID. Returns the root data string.
    ///
    /// Hits the cache first (per `FrgmntStore::get_persistent`);
    /// falls back to the on-disk `.frgmnt/objects/` tree on miss.
    pub fn read_content(&self, oid: &str) -> Result<String, ShardContentError> {
        let fragment = self
            .store
            .get_persistent(oid)
            .ok_or_else(|| ShardContentError::NotFound {
                oid: oid.to_string(),
            })?;
        Ok(fragment.data().clone())
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
    /// minted `ShardId`, or an error if the per-shard `.frgmnt/`
    /// directory cannot be created.
    ///
    /// T3 wires the disk-backed [`FrgmntStore`] into the shard
    /// constructor; if the temp-dir allocation fails (out of space,
    /// permission denied), the shard is not registered and the
    /// error propagates to the wire as `ERROR_INTERNAL`.
    pub fn open(&self, budget: BudgetMb) -> Result<ShardId, ShardContentError> {
        let id = ShardId::new();
        let shard = Shard::new(budget.into_budget_bytes(), id)?;
        self.lock().insert(id, shard);
        Ok(id)
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
        let id = reg.open(BudgetMb(16)).expect("open");
        assert!(reg.contains(&id));
        assert_eq!(reg.len(), 1);
        assert!(reg.close(&id));
        assert!(!reg.contains(&id));
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn tick_then_with_increments_tick() {
        let reg = ShardRegistry::new();
        let id = reg.open(BudgetMb(8)).expect("open");
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

    #[test]
    fn shard_commit_then_read_round_trips() {
        // T3 GREEN — the load-bearing unit test. Open a shard, commit
        // content, read it back by OID.
        let reg = ShardRegistry::new();
        let id = reg.open(BudgetMb(8)).expect("open");
        let oid = reg
            .with(&id, |shard| {
                shard
                    .commit_content("hello.txt", "hello world", "init")
                    .expect("commit_content")
            })
            .expect("shard present");
        assert_eq!(oid.len(), 40, "git OID is 40 hex chars");
        let content = reg
            .with(&id, |shard| {
                shard.read_content(&oid).expect("read_content")
            })
            .expect("shard present");
        assert_eq!(content, "hello world");
    }

    #[test]
    fn shard_status_reflects_committed_content() {
        // T3 GREEN — `hot_bytes > 0` after a commit.
        let reg = ShardRegistry::new();
        let id = reg.open(BudgetMb(8)).expect("open");
        let before = reg
            .with(&id, |s| s.hot_bytes())
            .expect("shard present");
        assert_eq!(before, 0);
        reg.with(&id, |s| s.commit_content("x", "data", "msg").unwrap())
            .expect("shard present");
        let after = reg
            .with(&id, |s| s.hot_bytes())
            .expect("shard present");
        assert!(after > 0, "hot_bytes should grow after commit");
    }

    #[test]
    fn read_unknown_oid_returns_not_found() {
        let reg = ShardRegistry::new();
        let id = reg.open(BudgetMb(8)).expect("open");
        let err = reg
            .with(&id, |s| s.read_content("0000000000000000000000000000000000000000"))
            .expect("shard present")
            .expect_err("unknown OID must error");
        assert!(matches!(err, ShardContentError::NotFound { .. }));
    }
}
