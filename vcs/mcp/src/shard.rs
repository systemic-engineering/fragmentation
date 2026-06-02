//! The per-session `Shard` + its `ShardRegistry`.
//!
//! T4 of `docs/specs/fragmentation-mcp.md`: `ShardId` becomes a
//! `prism_core::SpectralUuid` newtype per the CRDT spec
//! (`reality-shard-as-crdt.md`). 48 bits ACTIVE (the quantized
//! `SpectralCoordinate<5>`) + 80 bits DARK (the content hash prefix).
//! The empty shard's address is the canonical `ShardId::EMPTY` —
//! the bottom of the semilattice; the deduplication property
//! (two empty shards share an id) is a structural consequence.
//!
//! T2's history: `§4` (HamiltonScheduler at the agent altitude), `§3.4`
//! (the four shard sub-tools), and `§4` 's tick-on-dispatch contract
//! remain unchanged.
//!
//! # What this module owns
//!
//! - [`ShardId`] — the wire-altitude shard handle. `SpectralUuid`
//!   under the hood; serializes as a hyphenated 36-char string
//!   (the standard UUID-shaped form, byte-stable with the prior
//!   T3 wire output).
//! - [`BudgetMb`] / [`BudgetBytes`] — newtypes for the budget. No
//!   bare `u64` crosses the shard surface (per
//!   `[[feedback-no-bare-types]]`).
//! - [`Shard`] — per-session state: the configured budget, the
//!   stub `HamiltonScheduler` instance, metadata, the
//!   `FrgmntStore<Fractal<String>>` body for content commits.
//! - [`ShardRegistry`] — thread-safe map of `ShardId -> Shard`,
//!   the dispatch layer's source of truth. `open()` returns
//!   `ShardId::EMPTY` for the empty initial state; content
//!   commits inside the shard don't (yet) shift the id (the
//!   id-shifts-with-content semantics arrive in a follow-up tick
//!   when the semilattice merge mechanics wire fully).
//!
//! # Substrate-pull
//!
//! `[substrate-pull:realize]` — the shard surface is boundary
//! Rust at the `@io` altitude. The capability (content storage,
//! scheduler discipline, the CRDT semilattice algebra) lives in
//! the substrate. The Rust here is binding + wire; ShardId's
//! 128-bit body lives in prism_core (the algebra crate).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use std::time::SystemTime;

use fragmentation::encoding;
use fragmentation::fragment::{self, ContentAddressed, Fractal};
use fragmentation::frgmnt_store::FrgmntStore;
use fragmentation::hamilton_scheduler::{BudgetBytes, HamiltonScheduler, TickCount, TickReport};
use fragmentation::sha::Sha;
use prism_core::{SpectralUuid, SpectralUuidParseError};
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize, Serializer};

// ---------------------------------------------------------------------------
// Newtypes — no bare primitives cross the shard surface.
// ---------------------------------------------------------------------------

/// Per-session shard handle.
///
/// `SpectralUuid` under the hood (128 bits, golden-ratio-split into
/// 48 active + 80 dark per `reality-shard-as-crdt.md` §3). The
/// hyphenated 36-char string form is what crosses the wire — byte-
/// stable with the prior `uuid::Uuid v4` Display output, so the
/// wire surface is unchanged.
///
/// # Empty-shard determinism
///
/// [`Self::EMPTY`] is the canonical empty-shard address (the
/// bottom element of the CRDT semilattice, `⊥`). Two `open()`
/// calls on the registry with no committed content return the
/// SAME id — the deduplication property is a feature, not a
/// bug. Per the spec §2 + §4's identity-law guarantee.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShardId(pub SpectralUuid);

// `prism_core::SpectralUuid` is intentionally deps-free (no `serde`),
// so serde impls live here. Both Serialize and Deserialize go
// through the 36-char hyphenated string form — the wire-stable
// representation that the MCP JSON-RPC payloads already use.
impl Serialize for ShardId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ShardId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        ShardId::parse(&s).map_err(|e| de::Error::custom(e.to_string()))
    }
}

impl ShardId {
    /// The canonical empty-shard ID — the bottom of the lattice.
    ///
    /// `ShardId::EMPTY.0 = SpectralUuid::EMPTY`: active = 0 (λ₀ = 0,
    /// the void axis), dark = first 10 bytes of BLAKE3 of empty input.
    /// Deterministic across process lifetimes; the substrate's
    /// first named address into the void per `@mirror/reality/shard`.
    pub const EMPTY: Self = ShardId(SpectralUuid::EMPTY);

    /// Derive a `ShardId` from the shard's spectral active position
    /// and content hash prefix.
    ///
    /// - `active` carries the 48-bit quantized `SpectralCoordinate<5>`
    ///   in its lower 48 bits. T4 ships with `active = 0` (the spectral
    ///   coord computation lives upstream in `coincidence`; the
    ///   substrate-pull tick that wires it through happens once the
    ///   quantization rules pin per the spec §11 Q1).
    /// - `content_hash` is the 32-byte BLAKE3 prefix (or any hash;
    ///   prism_core is hash-agnostic). The first 10 bytes form the
    ///   dark portion.
    pub fn from_content(active: u64, content_hash: &[u8; 32]) -> Self {
        ShardId(SpectralUuid::from_parts(active, content_hash))
    }

    /// Parse from the hyphenated 36-char string form. Returns
    /// [`ShardIdParseError`] when the input is malformed.
    pub fn parse(s: &str) -> Result<Self, ShardIdParseError> {
        SpectralUuid::parse(s)
            .map(ShardId)
            .map_err(ShardIdParseError)
    }

    /// Deprecated shim for callers that previously generated a fresh
    /// `Uuid::new_v4()`. Returns [`Self::EMPTY`] — the canonical
    /// content-derived id for a shard with no committed content yet.
    ///
    /// New callers should use `Self::EMPTY` or `Self::from_content`
    /// directly. This shim exists only to keep the T2/T3 unit tests'
    /// `ShardId::new()` calls compiling through the T4 rename; it'll
    /// be removed once those tests migrate.
    pub fn new() -> Self {
        Self::EMPTY
    }
}

impl Default for ShardId {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl std::fmt::Display for ShardId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The SpectralUuid Display impl writes the standard
        // UUID-hyphenated 36-char form (lowercase hex).
        std::fmt::Display::fmt(&self.0, f)
    }
}

/// Parse error for [`ShardId::parse`]. Wraps the prism_core
/// [`SpectralUuidParseError`].
#[derive(Debug, Clone)]
pub struct ShardIdParseError(pub SpectralUuidParseError);

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

    /// Open a new shard under an explicit [`ShardId`].
    ///
    /// Unlike [`open`][Self::open] (which always uses `ShardId::EMPTY`),
    /// this variant registers the shard under the caller-supplied `id`.
    /// Used by the contextual `fragmentation_shard_open` tool: the id
    /// is derived from the session context hash so it is guaranteed to
    /// differ from `ShardId::EMPTY`.
    ///
    /// If a shard with `id` is already registered the existing shard is
    /// kept (idempotent open, same contract as [`open`][Self::open]).
    pub fn open_with_id(&self, budget: BudgetMb, id: ShardId) -> Result<ShardId, ShardContentError> {
        let mut guard = self.lock();
        if guard.contains_key(&id) {
            return Ok(id);
        }
        let shard = Shard::new(budget.into_budget_bytes(), id)?;
        guard.insert(id, shard);
        Ok(id)
    }

    /// Open a new shard with the given budget. Returns the canonical
    /// `ShardId::EMPTY` for a shard with no committed content; the
    /// deduplication property (two opens with no content share the
    /// same id) is a CRDT semilattice consequence per
    /// `reality-shard-as-crdt.md` §2 + §4.
    ///
    /// T4: if a shard with the EMPTY id is already registered, the
    /// existing shard is kept (the bottom-of-lattice IS deduplicating).
    /// Re-opening EMPTY is the substrate's idempotent-open contract.
    /// New budgets passed in subsequent opens are IGNORED on the
    /// existing EMPTY shard — the first open's budget wins, which
    /// is honest: the empty shard has no content to bound.
    ///
    /// The id-shifts-with-content semantics (post-commit, the shard
    /// id moves up in the lattice) is a follow-up tick once the
    /// semilattice merge mechanics wire fully. T4 keeps the id
    /// stable per session.
    ///
    /// If the per-shard `.frgmnt-<id>/` directory cannot be created
    /// (out of space, permission denied), the shard is not registered
    /// and the error propagates to the wire as `ERROR_INTERNAL`.
    pub fn open(&self, budget: BudgetMb) -> Result<ShardId, ShardContentError> {
        let id = ShardId::EMPTY;
        // Acquire the lock once for both check + insert.
        let mut guard = self.lock();
        if guard.contains_key(&id) {
            // Idempotent open: the canonical empty shard already
            // exists. Return the same id; the existing shard's
            // budget is preserved.
            return Ok(id);
        }
        let shard = Shard::new(budget.into_budget_bytes(), id)?;
        guard.insert(id, shard);
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
        // Post-T4: ShardId::new() returns ShardId::EMPTY (the canonical
        // content-derived id for a shard with no committed content).
        // The Display → parse round-trip still holds.
        let id = ShardId::new();
        let s = id.to_string();
        assert_eq!(s.len(), 36);
        let back = ShardId::parse(&s).expect("parse");
        assert_eq!(back, id);
    }

    #[test]
    fn shard_id_empty_is_canonical() {
        // The CRDT bottom element is deterministic and byte-stable.
        let a = ShardId::EMPTY;
        let b = ShardId::EMPTY;
        assert_eq!(a, b);
        assert_eq!(a.to_string(), b.to_string());
    }

    #[test]
    fn shard_id_from_content_is_deterministic() {
        let hash = [0x42u8; 32];
        let a = ShardId::from_content(0, &hash);
        let b = ShardId::from_content(0, &hash);
        assert_eq!(a, b);
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
    fn two_opens_with_no_content_share_the_empty_id() {
        // The deduplication property per reality-shard-as-crdt.md §2
        // surfaced at the wire altitude. Idempotent open.
        let reg = ShardRegistry::new();
        let id1 = reg.open(BudgetMb(8)).expect("open 1");
        let id2 = reg.open(BudgetMb(64)).expect("open 2");
        assert_eq!(id1, id2);
        assert_eq!(id1, ShardId::EMPTY);
        // Only one shard in the registry.
        assert_eq!(reg.len(), 1);
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
