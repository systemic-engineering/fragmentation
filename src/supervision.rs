//! Supervision trees — recursive, observer-dependent, continuous.
//!
//! A supervision tree partitions a computation into leaves. Each leaf
//! evolves independently. Boundary state between siblings is resolved
//! by coincidence — not boolean hash equality, but a continuous degree
//! of agreement in [0, 1].
//!
//! The tree grows by local splitting. The consumer decides when a leaf
//! should split. No global reassembly. The system runs in full fidelity
//! and collapses at the end, not at each boundary check.
//!
//! The observer is part of the hash. Different witness, different
//! measurement, different degree of coincidence. That's not error.
//! That's data.

/// Degree of coincidence between two observations of the same thing.
///
/// Returns a value in [0.0, 1.0]:
/// - 1.0 = perfect agreement (degenerate case: identical content)
/// - 0.0 = maximum disagreement
///
/// Boolean hash comparison is the degenerate case where the degree
/// is either 0.0 or 1.0. The continuous version preserves the gradient.
/// Premature collapse to boolean destroys information.
pub trait Witness {
    fn coincidence(&self, other: &Self) -> f64;
}

/// A supervision tree. Recursive. The structure emerges from the content.
///
/// `L` — leaf type (the computation at each partition)
/// `B` — boundary type (shared state between siblings)
///
/// Each Branch holds boundary items between its two children and a
/// tension value: the mean coincidence degree across those boundaries.
/// Tension of 1.0 means perfect agreement. Approaching 0.0 means
/// the children are diverging.
pub enum SupervisionTree<L, B> {
    /// A leaf: one partition of the computation.
    Leaf(L),
    /// A branch: two children + their boundary state.
    Branch {
        left: Box<SupervisionTree<L, B>>,
        right: Box<SupervisionTree<L, B>>,
        /// Boundary items between left and right.
        boundary: Vec<B>,
        /// Mean coincidence degree across boundary items. Updated by the consumer.
        tension: f64,
    },
}

impl<L, B> SupervisionTree<L, B> {
    /// Construct a branch from two children and their boundary items.
    pub fn branch(left: Self, right: Self, boundary: Vec<B>) -> Self {
        SupervisionTree::Branch {
            left: Box::new(left),
            right: Box::new(right),
            boundary,
            tension: 1.0,
        }
    }

    /// Number of leaves.
    pub fn n_leaves(&self) -> usize {
        match self {
            SupervisionTree::Leaf(_) => 1,
            SupervisionTree::Branch { left, right, .. } => left.n_leaves() + right.n_leaves(),
        }
    }

    /// Depth of the tree.
    pub fn depth(&self) -> usize {
        match self {
            SupervisionTree::Leaf(_) => 0,
            SupervisionTree::Branch { left, right, .. } => 1 + left.depth().max(right.depth()),
        }
    }

    /// All leaves (immutable).
    pub fn leaves(&self) -> Vec<&L> {
        match self {
            SupervisionTree::Leaf(l) => vec![l],
            SupervisionTree::Branch { left, right, .. } => {
                let mut result = left.leaves();
                result.extend(right.leaves());
                result
            }
        }
    }

    /// All leaves (mutable).
    pub fn leaves_mut(&mut self) -> Vec<&mut L> {
        match self {
            SupervisionTree::Leaf(l) => vec![l],
            SupervisionTree::Branch { left, right, .. } => {
                let mut result = left.leaves_mut();
                result.extend(right.leaves_mut());
                result
            }
        }
    }

    /// Total boundary items across all branches.
    pub fn n_boundary_items(&self) -> usize {
        match self {
            SupervisionTree::Leaf(_) => 0,
            SupervisionTree::Branch {
                left,
                right,
                boundary,
                ..
            } => boundary.len() + left.n_boundary_items() + right.n_boundary_items(),
        }
    }

    /// All boundary items (immutable, collected from every branch).
    pub fn all_boundary_items(&self) -> Vec<&B> {
        match self {
            SupervisionTree::Leaf(_) => vec![],
            SupervisionTree::Branch {
                left,
                right,
                boundary,
                ..
            } => {
                let mut result: Vec<&B> = boundary.iter().collect();
                result.extend(left.all_boundary_items());
                result.extend(right.all_boundary_items());
                result
            }
        }
    }

    /// Mean coincidence tension across all branches, weighted by boundary count.
    /// 1.0 = perfect agreement everywhere. 0.0 = maximum disagreement.
    pub fn boundary_tension(&self) -> f64 {
        let (total, count) = self.tension_sum();
        if count > 0 {
            total / count as f64
        } else {
            1.0
        }
    }

    fn tension_sum(&self) -> (f64, usize) {
        match self {
            SupervisionTree::Leaf(_) => (0.0, 0),
            SupervisionTree::Branch {
                left,
                right,
                boundary,
                tension,
                ..
            } => {
                let n = boundary.len();
                let (lt, lc) = left.tension_sum();
                let (rt, rc) = right.tension_sum();
                (*tension * n as f64 + lt + rt, n + lc + rc)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A simple witness: closeness of two f64 values.
    #[derive(Clone, Debug)]
    struct Measurement(f64);

    impl Witness for Measurement {
        fn coincidence(&self, other: &Self) -> f64 {
            let ratio = if other.0 > 0.0 {
                (self.0 / other.0).max(other.0 / self.0)
            } else if self.0 == 0.0 {
                return 1.0;
            } else {
                return 0.0;
            };
            1.0 / (1.0 + ratio.ln().abs())
        }
    }

    #[test]
    fn leaf_has_one_leaf() {
        let tree: SupervisionTree<i32, ()> = SupervisionTree::Leaf(42);
        assert_eq!(tree.n_leaves(), 1);
    }

    #[test]
    fn leaf_has_depth_zero() {
        let tree: SupervisionTree<i32, ()> = SupervisionTree::Leaf(42);
        assert_eq!(tree.depth(), 0);
    }

    #[test]
    fn branch_has_two_leaves() {
        let tree =
            SupervisionTree::branch(SupervisionTree::Leaf(1), SupervisionTree::Leaf(2), vec![()]);
        assert_eq!(tree.n_leaves(), 2);
    }

    #[test]
    fn branch_has_depth_one() {
        let tree =
            SupervisionTree::branch(SupervisionTree::Leaf(1), SupervisionTree::Leaf(2), vec![()]);
        assert_eq!(tree.depth(), 1);
    }

    #[test]
    fn nested_branch_depth() {
        let inner =
            SupervisionTree::branch(SupervisionTree::Leaf(1), SupervisionTree::Leaf(2), vec![()]);
        let tree = SupervisionTree::branch(inner, SupervisionTree::Leaf(3), vec![()]);
        assert_eq!(tree.depth(), 2);
        assert_eq!(tree.n_leaves(), 3);
    }

    #[test]
    fn leaves_returns_all_leaves() {
        let tree = SupervisionTree::branch(
            SupervisionTree::Leaf(10),
            SupervisionTree::branch(
                SupervisionTree::Leaf(20),
                SupervisionTree::Leaf(30),
                vec![()],
            ),
            vec![()],
        );
        let leaves: Vec<&i32> = tree.leaves();
        assert_eq!(leaves, vec![&10, &20, &30]);
    }

    #[test]
    fn leaves_mut_allows_modification() {
        let mut tree =
            SupervisionTree::branch(SupervisionTree::Leaf(1), SupervisionTree::Leaf(2), vec![()]);
        for leaf in tree.leaves_mut() {
            *leaf *= 10;
        }
        assert_eq!(tree.leaves(), vec![&10, &20]);
    }

    #[test]
    fn n_boundary_items_counts_all_levels() {
        let inner = SupervisionTree::branch(
            SupervisionTree::Leaf(1),
            SupervisionTree::Leaf(2),
            vec!["a", "b"], // 2 boundary items
        );
        let tree = SupervisionTree::branch(
            inner,
            SupervisionTree::Leaf(3),
            vec!["c"], // 1 boundary item
        );
        assert_eq!(tree.n_boundary_items(), 3);
    }

    #[test]
    fn all_boundary_items_collects_recursively() {
        let inner = SupervisionTree::branch(
            SupervisionTree::Leaf(1),
            SupervisionTree::Leaf(2),
            vec!["inner"],
        );
        let tree = SupervisionTree::branch(inner, SupervisionTree::Leaf(3), vec!["outer"]);
        let items: Vec<&&str> = tree.all_boundary_items();
        assert_eq!(items.len(), 2);
        assert!(items.contains(&&"inner"));
        assert!(items.contains(&&"outer"));
    }

    #[test]
    fn tension_defaults_to_one() {
        let tree =
            SupervisionTree::branch(SupervisionTree::Leaf(1), SupervisionTree::Leaf(2), vec![()]);
        assert!((tree.boundary_tension() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn leaf_tension_is_one() {
        let tree: SupervisionTree<i32, ()> = SupervisionTree::Leaf(42);
        assert!((tree.boundary_tension() - 1.0).abs() < 1e-10);
    }

    // -- Witness tests --

    #[test]
    fn witness_identical_returns_one() {
        let a = Measurement(1.5);
        let b = Measurement(1.5);
        assert!((a.coincidence(&b) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn witness_different_returns_less_than_one() {
        let a = Measurement(1.0);
        let b = Measurement(4.0);
        let d = a.coincidence(&b);
        assert!(d > 0.0 && d < 1.0, "degree={d} should be in (0, 1)");
    }

    #[test]
    fn witness_is_symmetric() {
        let a = Measurement(1.0);
        let b = Measurement(3.0);
        let ab = a.coincidence(&b);
        let ba = b.coincidence(&a);
        assert!(
            (ab - ba).abs() < 1e-10,
            "coincidence should be symmetric: {ab} vs {ba}"
        );
    }

    #[test]
    fn witness_close_values_near_one() {
        let a = Measurement(1.000);
        let b = Measurement(1.001);
        let d = a.coincidence(&b);
        assert!(d > 0.99, "close values should have high coincidence: {d}");
    }

    #[test]
    fn witness_far_values_near_zero() {
        let a = Measurement(1.0);
        let b = Measurement(1000.0);
        let d = a.coincidence(&b);
        assert!(d < 0.2, "far values should have low coincidence: {d}");
    }

    #[test]
    fn witness_zero_self_returns_one() {
        let a = Measurement(0.0);
        let b = Measurement(0.0);
        assert!((a.coincidence(&b) - 1.0).abs() < 1e-10);
    }
}
