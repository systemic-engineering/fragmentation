//! Bridge between fragmentation and prism-core.
//!
//! Implements prism-core traits (Addressable, MerkleTree, Store) for
//! fragmentation types (Fractal, FrgmntStore). This makes fragmentation's
//! content-addressed trees visible to the prism pipeline.

use crate::encoding::Encode;
use crate::fragment::{content_oid, ContentAddressed, Fractal, Fragmentable, TreeShaped};
use crate::sha::HashAlg;
use prism_core::Loss;

// ---------------------------------------------------------------------------
// HashAlg for prism_core::Oid
// ---------------------------------------------------------------------------

impl HashAlg for prism_core::Oid {
    fn hash(data: &[u8]) -> Self {
        prism_core::Oid::hash(data)
    }

    fn from_hex(hex: impl Into<String>) -> Self {
        prism_core::Oid::new(hex)
    }

    fn as_str(&self) -> &str {
        prism_core::Oid::as_str(self)
    }
}

// ---------------------------------------------------------------------------
// Addressable for Fractal
// ---------------------------------------------------------------------------

impl<E: Encode, H: HashAlg> prism_core::Addressable for Fractal<E, H> {
    fn oid(&self) -> prism_core::Oid {
        let hash = content_oid(self);
        prism_core::Oid::new(hash)
    }
}

// ---------------------------------------------------------------------------
// MerkleTree for Fractal
// ---------------------------------------------------------------------------

impl<E: Encode + PartialEq + Clone, H: HashAlg> prism_core::MerkleTree for Fractal<E, H> {
    type Data = E;

    fn data(&self) -> &E {
        ContentAddressed::data(self)
    }

    fn children(&self) -> &[Self] {
        TreeShaped::children(self)
    }
}

// ---------------------------------------------------------------------------
// StoreLoss
// ---------------------------------------------------------------------------

/// Loss type for FrgmntStore operations.
#[derive(Clone, Debug, PartialEq)]
pub struct StoreLoss {
    /// How much content was deduplicated. 1.0 = no dedup, 0.0 = total.
    pub dedup_ratio: f64,
}

impl Default for StoreLoss {
    fn default() -> Self {
        StoreLoss { dedup_ratio: 1.0 }
    }
}

impl prism_core::Loss for StoreLoss {
    fn zero() -> Self {
        StoreLoss { dedup_ratio: 1.0 }
    }
    fn total() -> Self {
        StoreLoss { dedup_ratio: 0.0 }
    }
    fn is_zero(&self) -> bool {
        (self.dedup_ratio - 1.0).abs() < f64::EPSILON
    }
    fn combine(self, other: Self) -> Self {
        StoreLoss {
            dedup_ratio: self.dedup_ratio.min(other.dedup_ratio),
        }
    }
}

// ---------------------------------------------------------------------------
// Store for FrgmntStore
// ---------------------------------------------------------------------------

impl<E, H> prism_core::Store for crate::frgmnt_store::FrgmntStore<Fractal<E, H>>
where
    E: Encode + PartialEq + Clone,
    H: HashAlg,
{
    type Tree = Fractal<E, H>;
    type Error = String;
    type L = StoreLoss;

    fn get(
        &self,
        oid: &prism_core::Oid,
    ) -> prism_core::Imperfect<Self::Tree, Self::Error, Self::L> {
        // FrgmntStore::get uses the content OID string as key
        match self.get(oid.as_str()) {
            Some(node) => prism_core::Imperfect::Success(node),
            None => {
                prism_core::Imperfect::Failure(format!("not found: {}", oid), StoreLoss::zero())
            }
        }
    }

    fn put(
        &mut self,
        tree: Self::Tree,
    ) -> prism_core::Imperfect<prism_core::Oid, Self::Error, Self::L> {
        let oid_str = content_oid(&tree);
        let size = std::mem::size_of_val(&tree);
        self.insert(oid_str.clone(), tree, size);
        prism_core::Imperfect::Success(prism_core::Oid::new(oid_str))
    }

    fn has(
        &self,
        oid: &prism_core::Oid,
    ) -> prism_core::Imperfect<prism_core::Luminosity, Self::Error, Self::L> {
        match self.get(oid.as_str()) {
            Some(_) => prism_core::Imperfect::Success(prism_core::Luminosity::Light),
            None => prism_core::Imperfect::Success(prism_core::Luminosity::Dark),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::fragment::{self, ContentAddressed, Fractal, Fragmentable};
    use crate::ref_::Ref;
    use crate::sha::{HashAlg, Sha};
    use prism_core::merkle::MerkleTree;
    use prism_core::oid::Addressable;

    fn make_shard(label: &str) -> Fractal<String> {
        let r = Ref::new(Sha(fragment::blob_oid(label)), label);
        Fractal::shard(r, label)
    }

    fn make_branch(label: &str, children: Vec<Fractal<String>>) -> Fractal<String> {
        let r = Ref::new(Sha(fragment::tree_oid(label, &children)), label);
        Fractal::new(r, label.to_string(), children)
    }

    // --- Addressable ---

    #[test]
    fn fractal_shard_is_addressable() {
        let shard = make_shard("hello");
        let oid = shard.oid();
        assert!(!oid.is_dark());
    }

    #[test]
    fn fractal_same_content_same_oid() {
        let a = make_shard("hello");
        let b = make_shard("hello");
        assert_eq!(a.oid(), b.oid());
    }

    #[test]
    fn fractal_different_content_different_oid() {
        let a = make_shard("hello");
        let b = make_shard("world");
        assert_ne!(a.oid(), b.oid());
    }

    #[test]
    fn fractal_branch_is_addressable() {
        let child = make_shard("leaf");
        let parent = make_branch("root", vec![child]);
        let oid = parent.oid();
        assert!(!oid.is_dark());
    }

    // --- MerkleTree ---

    #[test]
    fn fractal_shard_is_leaf() {
        let shard = make_shard("leaf");
        assert!(shard.is_leaf());
        assert_eq!(shard.degree(), 0);
    }

    #[test]
    fn fractal_branch_is_not_leaf() {
        let child = make_shard("child");
        let parent = make_branch("parent", vec![child]);
        assert!(!parent.is_leaf());
        assert_eq!(parent.degree(), 1);
    }

    #[test]
    fn fractal_merkle_data() {
        let shard = make_shard("payload");
        assert_eq!(MerkleTree::data(&shard), "payload");
    }

    #[test]
    fn fractal_merkle_children() {
        let c1 = make_shard("a");
        let c2 = make_shard("b");
        let parent = make_branch("root", vec![c1, c2]);
        let children = MerkleTree::children(&parent);
        assert_eq!(children.len(), 2);
    }

    // --- MerkleTree diff ---

    #[test]
    fn merkle_diff_identical_fractals_empty() {
        let a = make_branch("root", vec![make_shard("x")]);
        let b = make_branch("root", vec![make_shard("x")]);
        let deltas = prism_core::diff(&a, &b);
        assert!(deltas.is_empty());
    }

    #[test]
    fn merkle_diff_changed_child() {
        let a = make_branch("root", vec![make_shard("x"), make_shard("y")]);
        let b = make_branch("root", vec![make_shard("x"), make_shard("z")]);
        let deltas = prism_core::diff(&a, &b);
        assert!(!deltas.is_empty());
    }

    // --- HashAlg for Oid ---

    #[test]
    fn oid_implements_hash_alg() {
        use prism_core::Oid;
        let h = <Oid as HashAlg>::hash(b"hello");
        assert!(!h.as_str().is_empty());
        assert_eq!(h.as_str().len(), 64);
    }

    #[test]
    fn oid_hash_alg_deterministic() {
        use prism_core::Oid;
        let a = <Oid as HashAlg>::hash(b"test");
        let b = <Oid as HashAlg>::hash(b"test");
        assert_eq!(a, b);
    }

    #[test]
    fn oid_hash_alg_from_hex() {
        use prism_core::Oid;
        let oid = <Oid as HashAlg>::from_hex("abcdef");
        assert_eq!(oid.as_str(), "abcdef");
    }

    // --- Store for FrgmntStore ---

    #[test]
    fn frgmnt_store_implements_prism_store() {
        use prism_core::oid::Addressable;
        use prism_core::{Luminosity, Store};

        let dir = tempfile::tempdir().unwrap();
        let frgmnt = dir.path().join(".frgmnt");
        let mut store = crate::frgmnt_store::FrgmntStore::<Fractal<String>>::open(
            frgmnt.to_str().unwrap(),
            10_000,
        )
        .unwrap();

        let frag = make_shard("test");
        let oid = frag.oid();

        // has: Dark before put
        let before = Store::has(&store, &oid);
        assert_eq!(before.ok(), Some(Luminosity::Dark));

        // put
        let put_result = Store::put(&mut store, frag.clone());
        assert!(put_result.is_ok());

        // has: Light after put
        let after = Store::has(&store, &oid);
        assert_eq!(after.ok(), Some(Luminosity::Light));

        // get
        let got = Store::get(&store, &oid);
        assert!(got.is_ok());
        // Verify data roundtrips
        let got_frag = got.ok().unwrap();
        assert_eq!(ContentAddressed::data(&got_frag), "test");
    }

    // --- StoreLoss ---

    #[test]
    fn store_loss_zero_is_identity() {
        use super::StoreLoss;
        use prism_core::Loss;
        let z = StoreLoss::zero();
        assert!(z.is_zero());
    }

    #[test]
    fn store_loss_total_is_not_zero() {
        use super::StoreLoss;
        use prism_core::Loss;
        let t = StoreLoss::total();
        assert!(!t.is_zero());
    }

    #[test]
    fn store_loss_combine() {
        use super::StoreLoss;
        use prism_core::Loss;
        let a = StoreLoss { dedup_ratio: 0.8 };
        let b = StoreLoss { dedup_ratio: 0.5 };
        let c = a.combine(b);
        assert!((c.dedup_ratio - 0.5).abs() < f64::EPSILON);
    }
}
