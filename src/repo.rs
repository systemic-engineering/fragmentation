//! Repository trait for content-addressed storage.
//!
//! `Repo` defines the interface. `Store` (in store.rs) is the in-memory implementation.
//! A future git2 backend would implement the same trait.

use crate::commit::Commit;
use crate::fragment::Fragmentable;
use crate::sha::Sha;

/// Content-addressed repository.
///
/// Owned returns — Store clones from HashMaps, a git2 backend would construct fresh.
pub trait Repo {
    type Node: Fragmentable + Clone;

    /// Store all nodes of a tree recursively. Returns the root content OID.
    fn write_tree(&mut self, node: &Self::Node) -> String;

    /// Look up a tree/blob by its content OID.
    fn read_tree(&self, oid: &str) -> Option<Self::Node>;

    /// Store a commit.
    fn write_commit(&mut self, commit: Commit<Self::Node>);

    /// Look up a commit by its SHA.
    fn read_commit(&self, sha: &Sha) -> Option<Commit<Self::Node>>;

    /// Point a ref at a commit SHA.
    fn update_ref(&mut self, name: &str, sha: Sha);

    /// Resolve a ref to a commit SHA.
    fn resolve_ref(&self, name: &str) -> Option<Sha>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::Encode;
    use crate::fragment::{content_oid, Fragmentable};
    use crate::ref_::Ref;
    use crate::sha;
    use crate::store::Store;

    /// A non-Fractal type that implements Fragmentable.
    /// Proves the generalization: any Fragmentable can be stored.
    #[derive(Clone, Debug, PartialEq, Eq)]
    enum TestNode {
        Leaf {
            ref_: Ref,
            data: String,
        },
        Branch {
            ref_: Ref,
            data: String,
            children: Vec<TestNode>,
        },
    }

    impl Fragmentable for TestNode {
        type Data = String;

        fn self_ref(&self) -> &Ref {
            match self {
                TestNode::Leaf { ref_, .. } => ref_,
                TestNode::Branch { ref_, .. } => ref_,
            }
        }

        fn data(&self) -> &String {
            match self {
                TestNode::Leaf { data, .. } => data,
                TestNode::Branch { data, .. } => data,
            }
        }

        fn children(&self) -> &[TestNode] {
            match self {
                TestNode::Leaf { .. } => &[],
                TestNode::Branch { children, .. } => children,
            }
        }
    }

    fn test_ref(label: &str) -> Ref {
        Ref::new(sha::hash(label), label)
    }

    #[test]
    fn non_fractal_fragmentable_works_with_store() {
        let mut store = Store::<TestNode>::new();
        let node = TestNode::Branch {
            ref_: test_ref("root"),
            data: "hello".into(),
            children: vec![TestNode::Leaf {
                ref_: test_ref("child"),
                data: "world".into(),
            }],
        };
        let oid = store.write_tree(&node);
        assert_eq!(store.read_tree(&oid), Some(node));
    }

    #[test]
    fn non_fractal_content_oid() {
        let node = TestNode::Leaf {
            ref_: test_ref("a"),
            data: "test".into(),
        };
        let oid = content_oid(&node);
        assert!(!oid.is_empty());
    }
}
