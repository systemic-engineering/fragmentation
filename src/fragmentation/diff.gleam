/// Diff: structural comparison between two fragment trees.
///
/// Walks two trees and reports what changed.
/// Since fragments carry their own refs, comparison uses
/// self-addressed identity.

import fragmentation.{type Fragment}
import gleam/list

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A change between two fragment trees.
pub type Change {
  /// Fragment exists only in the new tree.
  Added(fragment: Fragment)
  /// Fragment exists only in the old tree.
  Removed(fragment: Fragment)
  /// Same position, different content.
  Modified(old: Fragment, new: Fragment)
  /// Same ref, same content.
  Unchanged(fragment: Fragment)
}

// ---------------------------------------------------------------------------
// Diffing
// ---------------------------------------------------------------------------

/// Diff two fragment trees by their roots.
/// Compares structurally: same hash = unchanged, different hash = modified.
/// Children compared positionally.
pub fn diff(old: Fragment, new: Fragment) -> List(Change) {
  case fragmentation.hash_fragment(old) == fragmentation.hash_fragment(new) {
    True -> [Unchanged(old)]
    False -> diff_fragments(old, new)
  }
}

fn diff_fragments(old: Fragment, new: Fragment) -> List(Change) {
  let old_children = fragmentation.children(old)
  let new_children = fragmentation.children(new)

  // Root changed
  let root_change = [Modified(old, new)]

  // Compare children positionally
  let child_changes = diff_children(old_children, new_children)

  list.append(root_change, child_changes)
}

fn diff_children(
  old: List(Fragment),
  new: List(Fragment),
) -> List(Change) {
  case old, new {
    [], [] -> []
    [], [n, ..rest] -> [Added(n), ..diff_children([], rest)]
    [o, ..rest], [] -> [Removed(o), ..diff_children(rest, [])]
    [o, ..orest], [n, ..nrest] ->
      list.append(diff(o, n), diff_children(orest, nrest))
  }
}

/// Summarize a list of changes: #(added, removed, modified, unchanged).
pub fn summary(changes: List(Change)) -> #(Int, Int, Int, Int) {
  list.fold(changes, #(0, 0, 0, 0), fn(acc, change) {
    let #(a, r, m, u) = acc
    case change {
      Added(_) -> #(a + 1, r, m, u)
      Removed(_) -> #(a, r + 1, m, u)
      Modified(_, _) -> #(a, r, m + 1, u)
      Unchanged(_) -> #(a, r, m, u + 1)
    }
  })
}
