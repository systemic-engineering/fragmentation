//! `HamiltonScheduler` — the substrate-management scheduler.
//!
//! T2 ships the MINIMAL STUB. The full spec lives at
//! `docs/specs/hamilton-scheduler.md` — Fate-driven strategy
//! selection across `{Abyss, Pathfinder, Cartographer, Explorer}`,
//! the 16-feature `GraphObservation`, the AdaptiveInterval, the
//! `RealtimeClass` hard/soft discipline, the Apollo-1202
//! drop-under-pressure contract. None of THAT lives here yet.
//!
//! What lives here, today:
//!
//! - `HamiltonScheduler<H>` — a budget-bearing handle with a tick
//!   counter. Generic over a hash algorithm `H: HashAlg` (the
//!   spec writes `<H: MerkleHash>`; `MerkleHash` doesn't exist as
//!   a trait yet — `HashAlg` is the substrate's nearest extant
//!   seam, so the stub uses it. The names converge when the
//!   `MerkleHash` trait lands).
//! - `tick(&mut self) -> TickReport` — increments the counter and
//!   returns a zero-filled `TickReport`. No hot/cold accounting;
//!   no Fate; no strategy selection.
//! - `budget()`, `tick_count()` — pure accessors.
//!
//! The stub is the right CALL SHAPE for fragmentation-mcp's T2
//! shard surface. The bodies are TODO; the surface is wire-ready.
//!
//! Substrate-pull marker: `[substrate-pull:realize]` — this is the
//! scheduler shim that lets the boundary (fragmentation-mcp) bind
//! to a real capability that hasn't been built in Rust yet. The
//! capability is in the substrate (the .mirror spec at
//! `docs/specs/hamilton-scheduler.md`); the Rust here is the
//! boundary realization at minimal shape.

use std::marker::PhantomData;

use crate::sha::HashAlg;

/// Newtype for the scheduler's byte budget. No bare `u64` crosses
/// the scheduler surface; the budget is named at its altitude.
///
/// Per the no-bare-types discipline (`[[feedback-no-bare-types]]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BudgetBytes(pub u64);

impl BudgetBytes {
    pub const fn new(bytes: u64) -> Self {
        BudgetBytes(bytes)
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// Newtype for a tick count. Monotonically non-decreasing across
/// the lifetime of a `HamiltonScheduler`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct TickCount(pub u64);

impl TickCount {
    pub const fn new(value: u64) -> Self {
        TickCount(value)
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// The result of one tick.
///
/// T2 returns zeros for `hot_bytes`/`cold_bytes`/`total_bytes` —
/// the stub doesn't observe a real store. T3+ wires the body and
/// fills these in from the underlying `FrgmntStore`.
///
/// TODO(T-scheduler-impl): full implementation per
/// `docs/specs/hamilton-scheduler.md` §3.1 — return a real
/// `TickResult<Strategy>` carrying the chosen strategy + its
/// `StrategyMetrics`. The T2 stub returns the minimum a budget-
/// aware caller needs to observe; the rest lands when the spec's
/// Rust translation lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TickReport {
    pub tick: TickCount,
    pub hot_bytes: u64,
    pub cold_bytes: u64,
    pub total_bytes: u64,
}

/// `HamiltonScheduler` — budget + tick counter (T2 stub).
///
/// Generic over `H: HashAlg`. The spec's `<H: MerkleHash>` is the
/// eventual surface; until `MerkleHash` lands as a public trait,
/// `HashAlg` is the substrate's nearest extant seam.
///
/// # Stub contract
///
/// - `new(budget_bytes)` constructs with the budget; tick count 0.
/// - `tick()` increments the counter; returns a zero-filled
///   `TickReport` carrying the new tick number.
/// - `budget()` returns the configured budget.
/// - `tick_count()` returns the current tick number.
///
/// TODO(T-scheduler-impl): full implementation per
/// `docs/specs/hamilton-scheduler.md`:
///   §3.1 — the `Scheduler` trait with `observe`/`decide`/`execute`.
///   §3.2 — the 16-feature `GraphObservation`.
///   §3.3 — Fate-driven strategy selection (Abyss/Pathfinder/
///          Cartographer/Explorer).
///   §3.7 — `AdaptiveInterval` (settled grows, mutation shrinks).
///   §3.8 — `RealtimeClass` admission + per-strategy drop policy.
pub struct HamiltonScheduler<H: HashAlg> {
    budget: BudgetBytes,
    tick_count: TickCount,
    _phantom: PhantomData<fn(H) -> H>,
}

impl<H: HashAlg> HamiltonScheduler<H> {
    /// Construct a scheduler with the given byte budget.
    ///
    /// TODO(T-scheduler-impl): also accept the `RealtimeClass`
    /// admission policy + the `AdaptiveInterval` initial state per
    /// `docs/specs/hamilton-scheduler.md` §3.7/§3.8.
    pub fn new(budget: BudgetBytes) -> Self {
        HamiltonScheduler {
            budget,
            tick_count: TickCount::default(),
            _phantom: PhantomData,
        }
    }

    /// One tick — observe, decide, act.
    ///
    /// TODO(T-scheduler-impl): the real tick walks the store,
    /// extracts the 16-feature `GraphObservation`, selects a
    /// strategy via Fate, executes the strategy's actions, returns
    /// the `TickResult<Strategy>` per §3.1 of the spec.
    pub fn tick(&mut self) -> TickReport {
        self.tick_count = TickCount(self.tick_count.0 + 1);
        TickReport {
            tick: self.tick_count,
            hot_bytes: 0,
            cold_bytes: 0,
            total_bytes: 0,
        }
    }

    /// Configured byte budget.
    pub fn budget(&self) -> BudgetBytes {
        self.budget
    }

    /// Current tick count.
    pub fn tick_count(&self) -> TickCount {
        self.tick_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sha::Sha;

    #[test]
    fn new_starts_at_tick_zero() {
        let s = HamiltonScheduler::<Sha>::new(BudgetBytes::new(1024));
        assert_eq!(s.tick_count(), TickCount(0));
        assert_eq!(s.budget(), BudgetBytes(1024));
    }

    #[test]
    fn tick_increments_count() {
        let mut s = HamiltonScheduler::<Sha>::new(BudgetBytes::new(1024));
        let r1 = s.tick();
        assert_eq!(r1.tick, TickCount(1));
        let r2 = s.tick();
        assert_eq!(r2.tick, TickCount(2));
        assert_eq!(s.tick_count(), TickCount(2));
    }

    #[test]
    fn stub_report_is_zero_filled() {
        let mut s = HamiltonScheduler::<Sha>::new(BudgetBytes::new(64));
        let r = s.tick();
        assert_eq!(r.hot_bytes, 0);
        assert_eq!(r.cold_bytes, 0);
        assert_eq!(r.total_bytes, 0);
    }
}
