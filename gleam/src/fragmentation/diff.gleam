/// Diff: structural comparison between two fragment trees.
///
/// Walks two trees and reports what changed.
/// Comparison uses content hash (encoder-aware).
import fragmentation.{type Fragment}
import gleam/list

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A change between two fragment trees.
pub type Change(data) {
  /// Fragment exists only in the new tree.
  Added(fragment: Fragment(data))
  /// Fragment exists only in the old tree.
  Removed(fragment: Fragment(data))
  /// Same position, different content.
  Modified(old: Fragment(data), new: Fragment(data))
  /// Same hash, same content.
  Unchanged(fragment: Fragment(data))
}

// ---------------------------------------------------------------------------
// Diffing
// ---------------------------------------------------------------------------

/// Diff two fragment trees by their roots.
/// Compares structurally: same hash = unchanged, different hash = modified.
/// Children compared positionally.
pub fn diff(
  old: Fragment(data),
  new: Fragment(data),
  encode: fn(data) -> String,
) -> List(Change(data)) {
  case
    fragmentation.hash_fragment(old, encode)
    == fragmentation.hash_fragment(new, encode)
  {
    True -> [Unchanged(old)]
    False -> diff_fragments(old, new, encode)
  }
}

fn diff_fragments(
  old: Fragment(data),
  new: Fragment(data),
  encode: fn(data) -> String,
) -> List(Change(data)) {
  let old_children = fragmentation.children(old)
  let new_children = fragmentation.children(new)

  let root_change = [Modified(old, new)]
  let child_changes = diff_children(old_children, new_children, encode)

  list.append(root_change, child_changes)
}

fn diff_children(
  old: List(Fragment(data)),
  new: List(Fragment(data)),
  encode: fn(data) -> String,
) -> List(Change(data)) {
  case old, new {
    [], [] -> []
    [], [n, ..rest] -> [Added(n), ..diff_children([], rest, encode)]
    [o, ..rest], [] -> [Removed(o), ..diff_children(rest, [], encode)]
    [o, ..orest], [n, ..nrest] ->
      list.append(diff(o, n, encode), diff_children(orest, nrest, encode))
  }
}

/// Summarize a list of changes: #(added, removed, modified, unchanged).
pub fn summary(changes: List(Change(data))) -> #(Int, Int, Int, Int) {
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
