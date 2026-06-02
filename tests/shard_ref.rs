//! RED: ShardContext + ShardRef don't exist yet.
//! T9: ShardContext as Prism + monoid, ShardRef as typed session handle.

use fragmentation::shard_ref::{ShardContext, ShardRef};
use fragmentation::hamilton_scheduler::BudgetBytes;
use prism_core::optics::monoid::PrismMonoid;
use prism_core::{Oid, Prism, SpectralUuid};

// 1. Both variants construct
#[test]
fn shard_context_variants_construct() {
    let _situated = ShardContext::Situated { context_oid: Oid::new("abc123") };
    let _empty = ShardContext::Empty;
}

// 2. ShardRef has the three fields
#[test]
fn shard_ref_constructs() {
    let _r = ShardRef {
        id: SpectralUuid::EMPTY,
        context: ShardContext::Empty,
        budget_bytes: BudgetBytes::new(64 * 1024 * 1024),
    };
}

// 3. Empty is PrismMonoid identity (left)
#[test]
fn empty_is_left_identity() {
    let ctx = ShardContext::Situated { context_oid: Oid::new("abc") };
    assert_eq!(ShardContext::identity().compose(ctx.clone()), ctx);
}

// 4. Empty is PrismMonoid identity (right)
#[test]
fn empty_is_right_identity() {
    let ctx = ShardContext::Situated { context_oid: Oid::new("abc") };
    assert_eq!(ctx.clone().compose(ShardContext::identity()), ctx);
}

// 5. Empty ∘ Empty = Empty
#[test]
fn empty_compose_empty_is_empty() {
    assert_eq!(
        ShardContext::identity().compose(ShardContext::identity()),
        ShardContext::Empty,
    );
}

// 6. Situated absorbs: Situated(a) ∘ Situated(b) = Situated(a)
//    (Situated is the non-empty element; first wins)
#[test]
fn situated_absorbs_second_situated() {
    let a = ShardContext::Situated { context_oid: Oid::new("aaa") };
    let b = ShardContext::Situated { context_oid: Oid::new("bbb") };
    assert_eq!(a.clone().compose(b), a);
}

// 7. Prism impl exists — compilation gate
#[test]
fn shard_context_is_prism() {
    fn accepts_prism<P: Prism>(_: &P) {}
    accepts_prism(&ShardContext::Empty);
}
