/// Walk: recursive traversal of fragment trees.
///
/// Depth-first traversal of the fragment structure.
/// Fragments embed their children directly — the walk follows
/// the tree, not SHA indirection.

import fragmentation.{type Fragment}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// What to do when visiting a fragment.
pub type Visitor(a) {
  /// Continue walking children.
  Continue(acc: a)
  /// Stop walking this branch.
  Stop(acc: a)
}

// ---------------------------------------------------------------------------
// Traversal
// ---------------------------------------------------------------------------

/// Collect all fragments in a tree, depth-first.
pub fn collect(root: Fragment) -> List(Fragment) {
  todo
}

/// Fold over all fragments in a tree, depth-first.
pub fn fold(
  root: Fragment,
  acc: a,
  f: fn(a, Fragment) -> Visitor(a),
) -> a {
  todo
}

/// Get the depth of a fragment tree.
pub fn depth(root: Fragment) -> Int {
  todo
}

/// Find the first fragment matching a predicate, depth-first.
pub fn find(
  root: Fragment,
  predicate: fn(Fragment) -> Bool,
) -> Result(Fragment, Nil) {
  todo
}
