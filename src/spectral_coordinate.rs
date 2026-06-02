//! `SpectralCoordinate<N>` — the substrate's content address as a position in
//! N-dimensional information geometry.
//!
//! The OID IS a coordinate in the spectrum of the content's graph Laplacian.
//! `SpectralCoordinate<5>` is mirror's substrate hash: five projections of
//! one spectrum (Fiedler value, eigengap, three heat-trace samples) per
//! `docs/specs/mirror-native-vcs.md` §4.6. The five is the *substrate optic
//! count*, not a matrix dimension.
//!
//! The eigenvalue-based body lives in `coincidence` (the math primitives —
//! Lanczos, heat kernel, Hodge decomposition — stay there). This module
//! owns the *type* and the `HashAlg` impl that fragmentation defaults to;
//! richer constructors that consume the coincidence math attach via
//! extension in the `coincidence` crate.
//!
//! The fallback path inside `HashAlg::hash` here uses SHA-256 prefixed with
//! the const generic `N`, so the trait stays usable without pulling in the
//! eigen-decomposition stack. Callers who want the Lanczos-derived 5-tuple
//! reach for `coincidence::spectral_coordinate::detect` (or equivalent
//! extension) explicitly.
//!
//! # Why this lives in fragmentation
//!
//! Pre-rename home was `coincidence/src/hash.rs` and the trait default in
//! `commit.rs` had to fall back to `Sha` because fragmentation could not
//! depend on coincidence (cycle). With the type *here*, the trait can
//! default to `SpectralCoordinate<5>` directly; the git adapter overrides
//! to `Sha` at its boundary, as is structurally honest. See
//! `docs/specs/mirror-native-vcs.md` §4.7.
//!
//! # Name
//!
//! The prior name `CoincidenceHash<N>` framed the value as a hash function
//! output. This name reframes: the value IS a coordinate. Identity and
//! locality collapse — every coordinate IS navigable, because it locates
//! content AND directs navigation toward it via gradient descent in
//! coordinate space. λ₀ = 0 (the void axis, per
//! `~/dev/systemic.engineering/practice/insights/coincidence/void-dual-geometry.md`)
//! is the origin of the manifold.

use std::fmt;
use std::hash;

use crate::sha::HashAlg;

/// Content address as a position in N-dimensional spectral coordinate space.
///
/// `SpectralCoordinate<2>` and `SpectralCoordinate<5>` are different types —
/// different projections of the same underlying spectrum, with different
/// discriminatory power. `SpectralCoordinate<5>` is mirror's default per
/// `mirror-native-vcs.md` §4.6.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpectralCoordinate<const N: usize> {
    eigenvalue: String,
}

impl<const N: usize> SpectralCoordinate<N> {
    /// Construct from a pre-computed eigenvalue hex string.
    ///
    /// Used by `coincidence`'s richer constructors that have already
    /// computed the spectral projection. The hex string is the canonical
    /// 80-character (5 f64 × 16 hex) byte representation per §4.6 for
    /// `N = 5`; smaller N is shorter proportionally.
    pub fn from_eigenvalue(eigenvalue: impl Into<String>) -> Self {
        SpectralCoordinate {
            eigenvalue: eigenvalue.into(),
        }
    }

    /// Access the eigenvalue string (the coordinate's canonical hex form).
    pub fn eigenvalue(&self) -> &str {
        &self.eigenvalue
    }
}

impl<const N: usize> hash::Hash for SpectralCoordinate<N> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.eigenvalue.hash(state);
    }
}

impl<const N: usize> HashAlg for SpectralCoordinate<N> {
    /// Hash bytes by computing the SHA-256 of `b"spectral-coord:" || N || bytes`.
    ///
    /// This is the fallback path — the bytes-only entry that fragmentation
    /// itself can compute without pulling in the eigendecomposition stack.
    /// The richer path (Lanczos on the content's incidence Laplacian)
    /// lives in `coincidence` and produces the canonical 5-tuple per §4.6.
    /// Callers that want the substrate hash use the coincidence path
    /// explicitly; callers that want `HashAlg::hash` over raw bytes get the
    /// SHA-prefixed form here.
    ///
    /// Type-distinguishing: different `N` produce different outputs over
    /// the same input by mixing `N` into the prefix.
    fn hash(data: &[u8]) -> Self {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(b"spectral-coord:");
        hasher.update((N as u64).to_le_bytes());
        hasher.update(data);
        SpectralCoordinate {
            eigenvalue: hex::encode(hasher.finalize()),
        }
    }

    fn from_hex(hex: impl Into<String>) -> Self {
        SpectralCoordinate {
            eigenvalue: hex.into(),
        }
    }

    fn as_str(&self) -> &str {
        &self.eigenvalue
    }
}

impl<const N: usize> fmt::Display for SpectralCoordinate<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.eigenvalue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- HashAlg contract --

    #[test]
    fn hash_deterministic() {
        let a = SpectralCoordinate::<2>::hash(b"hello");
        let b = SpectralCoordinate::<2>::hash(b"hello");
        assert_eq!(a, b);
    }

    #[test]
    fn hash_different_input_different_output() {
        let a = SpectralCoordinate::<2>::hash(b"hello");
        let b = SpectralCoordinate::<2>::hash(b"world");
        assert_ne!(a, b);
    }

    #[test]
    fn hash_produces_hex() {
        let h = SpectralCoordinate::<2>::hash(b"test");
        assert!(!h.as_str().is_empty());
        assert!(h.as_str().chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn from_hex_round_trips() {
        let h = SpectralCoordinate::<2>::hash(b"test");
        let h2 = SpectralCoordinate::<2>::from_hex(h.as_str());
        assert_eq!(h, h2);
    }

    #[test]
    fn different_n_different_coordinate() {
        let h2 = SpectralCoordinate::<2>::hash(b"hello");
        let h3 = SpectralCoordinate::<3>::hash(b"hello");
        assert_ne!(h2.as_str(), h3.as_str());
    }

    #[test]
    fn from_eigenvalue_round_trip() {
        let h = SpectralCoordinate::<5>::from_eigenvalue(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        );
        assert_eq!(h.eigenvalue().len(), 80);
    }

    // -- Display --

    #[test]
    fn display_matches_as_str() {
        let h = SpectralCoordinate::<2>::hash(b"test");
        assert_eq!(format!("{h}"), h.as_str());
    }
}
