//! RED: T12.2a — `impl Prism for LAPACKPrism`.
//!
//! Per `prism/docs/specs/pq.md` §6.5, `LAPACKPrism` is the canonical
//! numerical Prism impl that backs pq's wire surface. T12.2a ships
//! the type-level wiring + pure-Rust matrix substrate (no flang FFI
//! yet — the FFI integration is T12.2a.5).
//!
//! Per `fragmentation/docs/specs/fragmentation-mcp.md` §0.5.5:
//!
//! ```text
//!    PrismQuery (FrgmntMcp)  ─ T12.2b ─ the wire trait + DSLs
//!         │ dispatches into
//!         ▼
//!    LAPACKPrism             ─ T12.2a ─ the canonical numerical Prism impl
//!         │ is-a
//!         ▼
//!    prism_core::Prism                  the operator algebra
//! ```

use fragmentation::lapack_prism::{LAPACKPrism, ShardMatrix};
use prism_core::{Beam, Optic, Prism};

// ---------------------------------------------------------------------------
// 1. Type-level wiring: LAPACKPrism implements `prism_core::Prism`.
// ---------------------------------------------------------------------------

#[test]
fn lapack_prism_implements_prism() {
    fn accepts_prism<P: Prism>(_: &P) {}
    let p = LAPACKPrism::empty();
    accepts_prism(&p);
}

// ---------------------------------------------------------------------------
// 2. Construction: `empty()` builds a Prism with a zero-shape matrix.
// ---------------------------------------------------------------------------

#[test]
fn empty_lapack_prism_has_no_rows_or_columns() {
    let p = LAPACKPrism::empty();
    let m = p.matrix();
    assert_eq!(m.rows(), 0);
    assert_eq!(m.cols(), 0);
}

// ---------------------------------------------------------------------------
// 3. focus dispatches; identity-on-matrix per T12.2a stub contract.
// ---------------------------------------------------------------------------

#[test]
fn focus_passes_matrix_through() {
    let p = LAPACKPrism::empty();
    let seed = ShardMatrix::identity(3);
    let beam: Optic<(), ShardMatrix> = Optic::ok((), seed.clone());
    let focused = p.focus(beam);
    let out = focused.result().ok().expect("focus produced a value");
    assert_eq!(out, &seed);
}

// ---------------------------------------------------------------------------
// 4. project dispatches; identity-on-matrix per T12.2a stub contract.
// ---------------------------------------------------------------------------

#[test]
fn project_passes_matrix_through() {
    let p = LAPACKPrism::empty();
    let seed = ShardMatrix::identity(2);
    let beam: Optic<(), ShardMatrix> = Optic::ok((), seed.clone());
    let focused = p.focus(beam);
    let projected = p.project(focused);
    let out = projected.result().ok().expect("project produced a value");
    assert_eq!(out, &seed);
}

// ---------------------------------------------------------------------------
// 5. refract dispatches; identity-on-matrix per T12.2a stub contract.
// ---------------------------------------------------------------------------

#[test]
fn refract_passes_matrix_through() {
    let p = LAPACKPrism::empty();
    let seed = ShardMatrix::identity(1);
    let beam: Optic<(), ShardMatrix> = Optic::ok((), seed.clone());
    let focused = p.focus(beam);
    let projected = p.project(focused);
    let refracted = p.refract(projected);
    let out = refracted.result().ok().expect("refract produced a value");
    assert_eq!(out, &seed);
}

// ---------------------------------------------------------------------------
// 6. End-to-end pipeline via `prism_core::apply`.
// ---------------------------------------------------------------------------

#[test]
fn end_to_end_pipeline_yields_input_matrix() {
    let p = LAPACKPrism::empty();
    let seed = ShardMatrix::identity(4);
    let result = prism_core::apply(&p, Optic::ok((), seed.clone()));
    let out = result.result().ok().expect("pipeline produced a value");
    assert_eq!(out, &seed);
}

// ---------------------------------------------------------------------------
// 7. ShardMatrix structural laws — rows × cols, identity is square.
// ---------------------------------------------------------------------------

#[test]
fn shard_matrix_identity_is_square() {
    let m = ShardMatrix::identity(5);
    assert_eq!(m.rows(), 5);
    assert_eq!(m.cols(), 5);
}

#[test]
fn shard_matrix_zeros_has_requested_shape() {
    let m = ShardMatrix::zeros(3, 7);
    assert_eq!(m.rows(), 3);
    assert_eq!(m.cols(), 7);
}
