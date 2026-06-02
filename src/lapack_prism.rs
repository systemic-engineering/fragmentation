//! LAPACKPrism — the canonical numerical Prism impl.
//!
//! Per `prism/docs/specs/pq.md` §6.5:
//!
//! > **`LAPACKPrism` is the canonical `Prism` impl that backs pq's wire
//! > surface.** The numerical engine already ships: mirror links flang
//! > (the LAPACK provider in mirror's substrate). What's missing is the
//! > `LAPACKPrism: Prism` wrapper that exposes the linear-algebra
//! > substrate as a Prism.
//!
//! Per `fragmentation/docs/specs/fragmentation-mcp.md` §0.5.5, T12.2a
//! ships **this Prism impl** — the numerical engine. T12.2b will ship
//! `impl PrismQuery for FrgmntMcp` and dispatch into this Prism. The
//! altitude triple:
//!
//! ```text
//!    PrismQuery (FrgmntMcp)  — T12.2b — the wire trait + DSLs
//!         │ dispatches into
//!         ▼
//!    LAPACKPrism             — T12.2a — the canonical numerical Prism impl
//!         │ is-a
//!         ▼
//!    prism_core::Prism                  the operator algebra
//! ```
//!
//! ## Scope of T12.2a
//!
//! T12.2a ships the pure-Rust matrix substrate + the Prism dispatch
//! shape. The flang FFI integration that exchanges these `ShardMatrix`
//! values for LAPACK-backed routines lives in T12.2a.5. Until then the
//! three trait methods are **structural identity on the matrix** — the
//! wire dispatches correctly, the type chain composes, and the matrix
//! flows through unchanged. The Cramér-Rao residual norm is `zero` per
//! step (no information is lost in identity).
//!
//! ## ShardMatrix shape
//!
//! Per pq §6.5.1 the shard is "functionally a sparse matrix `M` indexed
//! by `OID × path`." T12.2a ships the **dense** `Vec<Vec<f64>>` form;
//! sparsity is a T12.2a.5 / T12.2b concern (the flang FFI consumes
//! whatever LAPACK convention the kernel asks for).
//!
//! Row index → OID, column index → path. The two indexes are carried as
//! sibling `Vec`s so the matrix retains its provenance; the actual
//! `Oid` ↔ row and `String` ↔ column maps land when the
//! `PrismQuery` dispatcher needs them (T12.2b).

use crate::sha::Sha;
use prism_core::{Beam, Optic, Prism};

// ---------------------------------------------------------------------------
// ShardMatrix
// ---------------------------------------------------------------------------

/// The shard's numerical state — a dense `OID × path → f64` matrix.
///
/// Per pq §6.5.2, every pq operation is a linear operator on this matrix.
/// T12.2a ships the dense form; the sparse/LAPACK-FFI form lands in
/// T12.2a.5.
///
/// The two index vectors carry the matrix's provenance:
/// - `row_oids[i]` is the content address of the row's source.
/// - `col_paths[j]` is the working-tree path of the column.
///
/// `data[i][j]` is the cell. Rows-of-cols layout matches LAPACK's
/// row-major convention; the FFI seam in T12.2a.5 will pick a
/// LAPACK-friendly column-major mirror when it lands.
#[derive(Debug, Clone, PartialEq)]
pub struct ShardMatrix {
    data: Vec<Vec<f64>>,
    row_oids: Vec<Sha>,
    col_paths: Vec<String>,
}

impl ShardMatrix {
    /// Construct an empty matrix (0 × 0). The neutral element under the
    /// `LAPACKPrism::empty()` constructor's algebra.
    pub fn empty() -> Self {
        Self {
            data: Vec::new(),
            row_oids: Vec::new(),
            col_paths: Vec::new(),
        }
    }

    /// Construct an `n × n` identity matrix with synthetic row/col
    /// labels. Useful for the T12.2a stub tests and as the algebra's
    /// multiplicative identity once the FFI lands.
    pub fn identity(n: usize) -> Self {
        let mut data = vec![vec![0.0_f64; n]; n];
        for (i, row) in data.iter_mut().enumerate() {
            row[i] = 1.0;
        }
        let row_oids = (0..n).map(|i| Sha(format!("row{}", i))).collect();
        let col_paths = (0..n).map(|j| format!("col{}", j)).collect();
        Self {
            data,
            row_oids,
            col_paths,
        }
    }

    /// Construct a `rows × cols` zero matrix.
    pub fn zeros(rows: usize, cols: usize) -> Self {
        let data = vec![vec![0.0_f64; cols]; rows];
        let row_oids = (0..rows).map(|i| Sha(format!("row{}", i))).collect();
        let col_paths = (0..cols).map(|j| format!("col{}", j)).collect();
        Self {
            data,
            row_oids,
            col_paths,
        }
    }

    /// Row count.
    pub fn rows(&self) -> usize {
        self.data.len()
    }

    /// Column count. Zero if the matrix has no rows.
    pub fn cols(&self) -> usize {
        self.data.first().map(|r| r.len()).unwrap_or(0)
    }

    /// Borrow the row-OID index.
    pub fn row_oids(&self) -> &[Sha] {
        &self.row_oids
    }

    /// Borrow the column-path index.
    pub fn col_paths(&self) -> &[String] {
        &self.col_paths
    }

    /// Borrow the dense cell array.
    pub fn data(&self) -> &[Vec<f64>] {
        &self.data
    }
}

// ---------------------------------------------------------------------------
// LAPACKPrism
// ---------------------------------------------------------------------------

/// The canonical numerical Prism impl per pq spec §6.5.
///
/// T12.2a ships the pure-Rust matrix substrate; T12.2a.5 wires the
/// flang LAPACK FFI under the same Prism dispatch shape. T12.2b lifts
/// this Prism to the wire altitude via `impl PrismQuery for FrgmntMcp`.
#[derive(Debug, Clone, PartialEq)]
pub struct LAPACKPrism {
    matrix: ShardMatrix,
}

impl LAPACKPrism {
    /// Construct a LAPACKPrism with an empty shard matrix. The T12.2a
    /// surface — the dispatcher is in place; the matrix gets populated
    /// at the wire altitude (T12.2b).
    pub fn empty() -> Self {
        Self {
            matrix: ShardMatrix::empty(),
        }
    }

    /// Construct a LAPACKPrism wrapping the given shard matrix.
    pub fn new(matrix: ShardMatrix) -> Self {
        Self { matrix }
    }

    /// Borrow the underlying shard matrix.
    pub fn matrix(&self) -> &ShardMatrix {
        &self.matrix
    }
}

// ---------------------------------------------------------------------------
// impl Prism for LAPACKPrism
// ---------------------------------------------------------------------------
//
// Per the T12.2a stub contract: focus/project/refract dispatch correctly
// and carry the matrix through unchanged. Real linear algebra lands in
// T12.2a.5 (focus = row/column select via flang LAPACK; project =
// projector / Banach iteration / adjacency power; refract = rank-1
// update + persist).

impl Prism for LAPACKPrism {
    type Input = Optic<(), ShardMatrix>;
    type Focused = Optic<ShardMatrix, ShardMatrix>;
    type Projected = Optic<ShardMatrix, ShardMatrix>;
    type Refracted = Optic<ShardMatrix, ShardMatrix>;

    /// Per pq §6.5.2: `focus({oid})` → row select, `focus({path})` →
    /// column select, `focus({})` → identity. T12.2a ships the
    /// identity-on-matrix dispatcher; the typed `Target` DSL routes to
    /// the right LAPACK kernel in T12.2a.5 / T12.2b.
    fn focus(&self, beam: Self::Input) -> Self::Focused {
        let m = beam.result().ok().expect("focus: Err beam").clone();
        beam.next(m)
    }

    /// Per pq §6.5.2: `project({prefix})` → projector, `project({walk})`
    /// → adjacency power, `project({kintsugi})` → Banach iteration.
    /// T12.2a ships identity-on-matrix; the typed `Filter` DSL routes
    /// in T12.2a.5 / T12.2b.
    fn project(&self, beam: Self::Focused) -> Self::Projected {
        let m = beam.result().ok().expect("project: Err beam").clone();
        beam.next(m)
    }

    /// Per pq §6.5.2: `refract({to_path})` → rank-1 update,
    /// `refract({snapshot})` → persist. T12.2a ships identity-on-matrix;
    /// the typed `Output` DSL routes in T12.2a.5 / T12.2b.
    fn refract(&self, beam: Self::Projected) -> Self::Refracted {
        let m = beam.result().ok().expect("refract: Err beam").clone();
        beam.next(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_shard_matrix_is_zero_by_zero() {
        let m = ShardMatrix::empty();
        assert_eq!(m.rows(), 0);
        assert_eq!(m.cols(), 0);
    }

    #[test]
    fn identity_matrix_diagonal_is_one() {
        let m = ShardMatrix::identity(3);
        assert_eq!(m.data()[0][0], 1.0);
        assert_eq!(m.data()[1][1], 1.0);
        assert_eq!(m.data()[2][2], 1.0);
        assert_eq!(m.data()[0][1], 0.0);
        assert_eq!(m.data()[1][0], 0.0);
    }

    #[test]
    fn zeros_matrix_is_all_zero() {
        let m = ShardMatrix::zeros(2, 3);
        assert!(m.data().iter().all(|row| row.iter().all(|&x| x == 0.0)));
    }

    #[test]
    fn shard_matrix_provenance_indexes_match_shape() {
        let m = ShardMatrix::identity(4);
        assert_eq!(m.row_oids().len(), 4);
        assert_eq!(m.col_paths().len(), 4);
    }

    #[test]
    fn lapack_prism_new_wraps_supplied_matrix() {
        let m = ShardMatrix::identity(2);
        let p = LAPACKPrism::new(m.clone());
        assert_eq!(p.matrix(), &m);
    }
}
