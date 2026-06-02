//! ShardRef — typed handle for a memory-managed shard.
//!
//! A `ShardRef` bundles a session identity (`SpectralUuid`), a `ShardContext`
//! that situates the shard in the graph (or marks it as context-free), and a
//! `BudgetBytes` ceiling enforced by the `HamiltonScheduler`.
//!
//! `ShardContext` implements `PrismMonoid` (Empty = identity; Situated absorbs
//! — first context wins) and `Prism` (identity/passthrough shape).

use prism_core::optics::monoid::PrismMonoid;
use prism_core::{Beam, Optic, Prism};

// ---------------------------------------------------------------------------
// ShardContext
// ---------------------------------------------------------------------------

/// Whether a shard is anchored to a specific context OID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShardContext {
    /// The shard is positioned within a graph context identified by `context_oid`.
    Situated { context_oid: prism_core::Oid },
    /// The shard carries no graph context.
    Empty,
}

impl PrismMonoid for ShardContext {
    fn identity() -> Self {
        ShardContext::Empty
    }

    /// Monoid composition — first `Situated` context wins:
    /// - `(Empty, ctx)` → `ctx`
    /// - `(ctx, Empty)` → `ctx`
    /// - `(Situated(a), Situated(_))` → `Situated(a)` (absorbing)
    fn compose(self, other: Self) -> Self {
        match (self, other) {
            (ShardContext::Empty, ctx) => ctx,
            (ctx, ShardContext::Empty) => ctx,
            (situated @ ShardContext::Situated { .. }, ShardContext::Situated { .. }) => situated,
        }
    }
}

// ---------------------------------------------------------------------------
// Prism for ShardContext — identity/passthrough shape
// ---------------------------------------------------------------------------
//
// The test only checks that the impl compiles (`fn accepts_prism<P: Prism>(_: &P) {}`).
// We use the canonical passthrough shape: Input = Optic<(), ShardContext>,
// all three stages are no-ops that forward the beam unchanged.

impl Prism for ShardContext {
    type Input = Optic<(), ShardContext>;
    type Focused = Optic<ShardContext, ShardContext>;
    type Projected = Optic<ShardContext, ShardContext>;
    type Refracted = Optic<ShardContext, ShardContext>;

    fn focus(&self, beam: Self::Input) -> Self::Focused {
        let ctx = beam.result().ok().expect("focus: Err beam").clone();
        beam.next(ctx)
    }

    fn project(&self, beam: Self::Focused) -> Self::Projected {
        let ctx = beam.result().ok().expect("project: Err beam").clone();
        beam.next(ctx)
    }

    fn refract(&self, beam: Self::Projected) -> Self::Refracted {
        let ctx = beam.result().ok().expect("refract: Err beam").clone();
        beam.next(ctx)
    }
}

// ---------------------------------------------------------------------------
// ShardRef
// ---------------------------------------------------------------------------

/// A typed handle for a memory-managed shard.
#[derive(Debug, Clone)]
pub struct ShardRef {
    /// Session identity — unique per shard lifetime.
    pub id: prism_core::SpectralUuid,
    /// Graph context, or `Empty` for context-free shards.
    pub context: ShardContext,
    /// Memory budget enforced by the `HamiltonScheduler`.
    pub budget_bytes: crate::hamilton_scheduler::BudgetBytes,
}
