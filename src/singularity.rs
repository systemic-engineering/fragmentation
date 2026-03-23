use std::convert::Infallible;

use crate::commit::{Commit, Draftable, Draft};
use crate::fragment::{Fractal, Fragmentable};
use crate::ref_::Ref;
use crate::repo::Repo;
use crate::sha::HashAlg;
use crate::witnessed::Committer;

/// The point where a tree of possibilities collapses into a single artifact.
/// `collapse` resolves. `refract` reconstructs.
pub trait Singularity: Sized {
    type Artifact;
    type Error;

    fn collapse(&self) -> Result<Self::Artifact, Self::Error>;
    fn refract(artifact: &Self::Artifact) -> Result<Self, Self::Error>;
}

/// Identity singularity: collapse = clone, refract = clone. No information loss.
/// This is the Iso in the optics hierarchy. Full recovery. No dimensional reduction.
impl<E: Clone, H: HashAlg> Singularity for Fractal<E, H> {
    type Artifact = Self;
    type Error = Infallible;

    fn collapse(&self) -> Result<Self, Infallible> {
        Ok(self.clone())
    }

    fn refract(artifact: &Self) -> Result<Self, Infallible> {
        Ok(artifact.clone())
    }
}

// ============================================================================
// Witnessed Singularity: collapse writes a commit with a Lens back to the
// original tree. refract follows the Lens. The inverse is written by the
// forward operation itself.
// ============================================================================

/// Error type for witnessed singularity operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SingularityError {
    /// The commit's node is not a Lens.
    NotALens,
    /// The Lens has no targets.
    EmptyLens,
    /// The target OID was not found in the repo.
    TargetNotFound(String),
    /// The commit was not found in the repo.
    CommitNotFound(String),
}

impl std::fmt::Display for SingularityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SingularityError::NotALens => write!(f, "commit node is not a Lens"),
            SingularityError::EmptyLens => write!(f, "Lens has no targets"),
            SingularityError::TargetNotFound(oid) => {
                write!(f, "Lens target not found in repo: {}", oid)
            }
            SingularityError::CommitNotFound(sha) => {
                write!(f, "commit not found: {}", sha)
            }
        }
    }
}

/// A witnessed singularity: collapse creates a commit whose node is a Lens
/// pointing back to the original tree. The observer (&self) is part of the
/// commit. Different observer, different commit, same Lens target.
///
/// This is the Lens in the optics hierarchy: focused, total, partial information.
/// The commit carries the witness metadata. The Lens carries the way back.
pub struct WitnessedSingularity<'a, H: HashAlg, R: Repo<Node = Fractal<String, H>, Hash = H>> {
    pub repo: &'a mut R,
    pub committer: Committer,
    pub timestamp: String,
    _hash: std::marker::PhantomData<H>,
}

impl<'a, H: HashAlg, R: Repo<Node = Fractal<String, H>, Hash = H>> WitnessedSingularity<'a, H, R> {
    /// Create a new witnessed singularity with the given observer.
    pub fn new(repo: &'a mut R, committer: Committer, timestamp: impl Into<String>) -> Self {
        WitnessedSingularity {
            repo,
            committer,
            timestamp: timestamp.into(),
            _hash: std::marker::PhantomData,
        }
    }

    /// Collapse a Fractal into a commit with a Lens back to the original tree.
    ///
    /// The collapse writes:
    /// 1. The original tree to the repo (preserving its content OID)
    /// 2. A Lens node targeting the original tree's content OID
    /// 3. A commit containing the Lens node
    ///
    /// The commit SHA depends on the observer (committer). The Lens target
    /// does not. Same tree, different witness, different commit, same target.
    pub fn collapse(
        &mut self,
        tree: &Fractal<String, H>,
        message: impl Into<String>,
    ) -> Result<Commit<Fractal<String, H>, H>, SingularityError> {
        let message: String = message.into();

        // 1. Write the original tree to the repo, preserving its content OID
        let tree_oid = self.repo.write_tree(tree);

        // 2. Create a Lens node targeting the original tree's content OID.
        //    The Lens data carries the collapse message.
        //    The Lens ref uses the tree OID as its SHA — it IS about that tree.
        let lens_ref = Ref::new(H::from_hex(&tree_oid), "collapse");
        let target = vec![H::from_hex(&tree_oid)];
        let lens = Fractal::lens(lens_ref, &message, target);

        // 3. Write the Lens to the repo (so its OID is stored too)
        self.repo.write_tree(&lens);

        // 4. Create a draft commit containing the Lens, and commit it.
        //    The commit SHA depends on the observer (committer).
        let draft = Draft::root(&message, lens);
        let commit = draft.commit(
            self.repo,
            self.committer.clone(),
            &self.timestamp,
        );

        Ok(commit)
    }

    /// Refract a commit back into the original tree by following the Lens.
    ///
    /// Reads the commit, extracts the Lens node, follows the target OID
    /// back to the original tree in the repo.
    pub fn refract(
        repo: &R,
        commit: &Commit<Fractal<String, H>, H>,
    ) -> Result<Fractal<String, H>, SingularityError> {
        let node = commit.node();

        // The commit's node must be a Lens
        if !node.is_lens() {
            return Err(SingularityError::NotALens);
        }

        // The Lens must have at least one target
        let targets = node.targets();
        if targets.is_empty() {
            return Err(SingularityError::EmptyLens);
        }

        // Follow the first target back to the original tree
        let target_oid = targets[0].as_str();
        repo.read_tree(target_oid)
            .ok_or_else(|| SingularityError::TargetNotFound(target_oid.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding;
    use crate::fragment::{content_oid, Fractal};
    use crate::sha::Sha;
    use crate::store::Store;
    use crate::witnessed::Committer;

    fn test_tree() -> Fractal<String> {
        encoding::encode("the interior of a black hole")
    }

    fn mara() -> Committer {
        Committer::new("Mara", "mara@systemic.engineer")
    }

    fn reed() -> Committer {
        Committer::new("Reed", "reed@systemic.engineer")
    }

    const TIMESTAMP: &str = "1234567890 +0000";

    // ====================================================================
    // Core: collapse produces a commit whose node is a Lens
    // ====================================================================

    #[test]
    fn collapse_produces_lens_commit() {
        let mut store = Store::<Fractal<String>>::new();
        let tree = test_tree();

        let mut singularity = WitnessedSingularity::<Sha, _>::new(&mut store, mara(), TIMESTAMP);
        let commit = singularity.collapse(&tree, "collapse").unwrap();

        // The commit's node must be a Lens
        assert!(
            commit.node().is_lens(),
            "collapse commit node must be a Lens"
        );
    }

    #[test]
    fn collapse_lens_targets_original_tree() {
        let mut store = Store::<Fractal<String>>::new();
        let tree = test_tree();
        let original_oid = content_oid(&tree);

        let mut singularity = WitnessedSingularity::<Sha, _>::new(&mut store, mara(), TIMESTAMP);
        let commit = singularity.collapse(&tree, "collapse").unwrap();

        // The Lens must target the original tree's content OID
        let targets = commit.node().targets();
        assert_eq!(targets.len(), 1);
        assert_eq!(
            targets[0].as_str(),
            original_oid,
            "Lens target must be the original tree's content OID"
        );
    }

    // ====================================================================
    // Core: refract follows the Lens and recovers the original tree
    // ====================================================================

    #[test]
    fn refract_recovers_original_tree() {
        let mut store = Store::<Fractal<String>>::new();
        let tree = test_tree();
        let original_oid = content_oid(&tree);

        let mut singularity = WitnessedSingularity::<Sha, _>::new(&mut store, mara(), TIMESTAMP);
        let commit = singularity.collapse(&tree, "collapse").unwrap();

        // refract must recover the original tree
        let recovered =
            WitnessedSingularity::<Sha, Store<Fractal<String>>>::refract(&store, &commit).unwrap();
        assert_eq!(
            content_oid(&recovered),
            original_oid,
            "refracted tree must have the same content OID as the original"
        );
    }

    // ====================================================================
    // Round-trip: collapse then refract preserves content OIDs
    // ====================================================================

    #[test]
    fn round_trip_preserves_content_oids() {
        let mut store = Store::<Fractal<String>>::new();
        let tree = test_tree();
        let original_oid = content_oid(&tree);

        let mut singularity = WitnessedSingularity::<Sha, _>::new(&mut store, mara(), TIMESTAMP);
        let commit = singularity
            .collapse(&tree, "collapse through the horizon")
            .unwrap();
        let recovered =
            WitnessedSingularity::<Sha, Store<Fractal<String>>>::refract(&store, &commit).unwrap();

        assert_eq!(content_oid(&recovered), original_oid);
    }

    #[test]
    fn round_trip_preserves_tree_structure() {
        let mut store = Store::<Fractal<String>>::new();
        let tree = test_tree();

        let mut singularity = WitnessedSingularity::<Sha, _>::new(&mut store, mara(), TIMESTAMP);
        let commit = singularity.collapse(&tree, "collapse").unwrap();
        let recovered =
            WitnessedSingularity::<Sha, Store<Fractal<String>>>::refract(&store, &commit).unwrap();

        // The recovered tree should have the same structure
        assert_eq!(recovered.data(), tree.data());
        assert_eq!(recovered.children().len(), tree.children().len());
    }

    // ====================================================================
    // Observer variance: different witnesses, different commits, same target
    // ====================================================================

    #[test]
    fn different_observers_produce_different_commits() {
        let mut store = Store::<Fractal<String>>::new();
        let tree = test_tree();

        let mut singularity_mara =
            WitnessedSingularity::<Sha, _>::new(&mut store, mara(), TIMESTAMP);
        let commit_mara = singularity_mara.collapse(&tree, "collapse").unwrap();

        let mut singularity_reed =
            WitnessedSingularity::<Sha, _>::new(&mut store, reed(), TIMESTAMP);
        let commit_reed = singularity_reed.collapse(&tree, "collapse").unwrap();

        // Different observers produce different commit SHAs
        assert_ne!(
            commit_mara.sha(),
            commit_reed.sha(),
            "different observers must produce different commit SHAs"
        );
    }

    #[test]
    fn different_observers_same_lens_target() {
        let mut store = Store::<Fractal<String>>::new();
        let tree = test_tree();

        let mut singularity_mara =
            WitnessedSingularity::<Sha, _>::new(&mut store, mara(), TIMESTAMP);
        let commit_mara = singularity_mara.collapse(&tree, "collapse").unwrap();

        let mut singularity_reed =
            WitnessedSingularity::<Sha, _>::new(&mut store, reed(), TIMESTAMP);
        let commit_reed = singularity_reed.collapse(&tree, "collapse").unwrap();

        // Same tree, same Lens target regardless of observer
        assert_eq!(
            commit_mara.node().targets(),
            commit_reed.node().targets(),
            "same tree must produce same Lens target regardless of observer"
        );
    }

    // ====================================================================
    // Optics hierarchy: the identity impl is Iso (no information loss)
    // ====================================================================

    #[test]
    fn identity_singularity_is_iso() {
        // The default Singularity impl on Fractal is the identity — Iso in optics.
        // collapse = clone, refract = clone. No dimensional reduction.
        let tree = test_tree();
        let collapsed = tree.collapse().unwrap();
        let refracted = Fractal::<String>::refract(&collapsed).unwrap();
        assert_eq!(content_oid(&tree), content_oid(&refracted));
        assert_eq!(content_oid(&tree), content_oid(&collapsed));
    }

    // ====================================================================
    // The Lens node carries data: the collapse message as metadata
    // ====================================================================

    #[test]
    fn collapse_lens_carries_data() {
        let mut store = Store::<Fractal<String>>::new();
        let tree = test_tree();

        let mut singularity = WitnessedSingularity::<Sha, _>::new(&mut store, mara(), TIMESTAMP);
        let commit = singularity.collapse(&tree, "event horizon").unwrap();

        // The Lens data is the collapse message
        assert!(
            !commit.node().data().is_empty(),
            "Lens node must carry data"
        );
    }

    // ====================================================================
    // Error cases
    // ====================================================================

    #[test]
    fn refract_non_lens_commit_errors() {
        // A commit whose node is a Fractal (not a Lens) should fail refract
        let mut store = Store::<Fractal<String>>::new();
        let tree = test_tree();
        let commit = Draft::root("not a collapse", tree).commit(&mut store, mara(), TIMESTAMP);

        let result = WitnessedSingularity::<Sha, Store<Fractal<String>>>::refract(&store, &commit);
        assert_eq!(result, Err(SingularityError::NotALens));
    }

    // ====================================================================
    // Superposition: Vec<H> targets = multiple views of the same collapse
    // ====================================================================

    #[test]
    fn lens_targets_is_superposition() {
        // A Lens with multiple targets represents superposition:
        // multiple views of the same tree from different collapse points.
        let ref_ = Ref::new(Sha::from_hex("abc"), "superposition");
        let targets = vec![
            Sha::from_hex("target_a"),
            Sha::from_hex("target_b"),
            Sha::from_hex("target_c"),
        ];
        let lens: Fractal<String> = Fractal::lens(ref_, "superposed", targets);

        assert!(lens.is_lens());
        assert_eq!(lens.targets().len(), 3);
        // Each target is a different view — a different possible collapse outcome
    }

    // ====================================================================
    // The commit is witnessed: observer metadata preserved
    // ====================================================================

    #[test]
    fn collapse_commit_is_witnessed() {
        let mut store = Store::<Fractal<String>>::new();
        let tree = test_tree();

        let mut singularity = WitnessedSingularity::<Sha, _>::new(&mut store, mara(), TIMESTAMP);
        let commit = singularity.collapse(&tree, "witnessed collapse").unwrap();

        // The commit carries witness metadata
        let witnessed = commit.witnessed();
        assert_eq!(witnessed.committer.name, "Mara");
        assert_eq!(witnessed.committer.email, "mara@systemic.engineer");
    }

    // ====================================================================
    // The Lens chain: sequential collapses create a trace
    // ====================================================================

    #[test]
    fn sequential_collapses_create_lens_chain() {
        // Each collapse produces a commit. Chain them: each new collapse
        // can reference the previous collapse commit as its parent.
        // The chain of Lenses IS the event horizon trace.
        let mut store = Store::<Fractal<String>>::new();
        let tree = test_tree();

        let mut s1 = WitnessedSingularity::<Sha, _>::new(&mut store, mara(), "1000000000 +0000");
        let c1 = s1.collapse(&tree, "first collapse").unwrap();

        // Second tree, collapsed as child of first
        let tree2 = encoding::encode("second observation");
        let mut s2 = WitnessedSingularity::<Sha, _>::new(&mut store, mara(), "1000000001 +0000");
        let c2 = s2.collapse(&tree2, "second collapse").unwrap();

        // Both commits exist, both are Lens nodes
        assert!(c1.node().is_lens());
        assert!(c2.node().is_lens());

        // They target different original trees
        assert_ne!(c1.node().targets(), c2.node().targets());

        // Both can be refracted independently
        let r1 = WitnessedSingularity::<Sha, Store<Fractal<String>>>::refract(&store, &c1).unwrap();
        let r2 = WitnessedSingularity::<Sha, Store<Fractal<String>>>::refract(&store, &c2).unwrap();
        assert_eq!(content_oid(&r1), content_oid(&tree));
        assert_eq!(content_oid(&r2), content_oid(&tree2));
    }
}
