/// Diff: structural comparison between two fragment trees.
///
/// Walks two trees and reports what changed.
/// Since fragments carry their own refs, comparison uses
/// self-addressed identity.

import fragmentation.{type Fragment, type Sha}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A change between two fragment trees.
pub type Change {
  /// Fragment exists only in the new tree.
  Added(fragment: Fragment)
  /// Fragment exists only in the old tree.
  Removed(fragment: Fragment)
  /// Same ref, different content.
  Modified(old: Fragment, new: Fragment)
  /// Same ref, same content.
  Unchanged(fragment: Fragment)
}

// ---------------------------------------------------------------------------
// Diffing
// ---------------------------------------------------------------------------

/// Diff two fragment trees by their roots.
pub fn diff(old: Fragment, new: Fragment) -> List(Change) {
  todo
}

/// Summarize a list of changes: #(added, removed, modified, unchanged).
pub fn summary(changes: List(Change)) -> #(Int, Int, Int, Int) {
  todo
}
