/// Walk: recursive traversal of fragment trees.
///
/// Depth-first traversal of the fragment structure.
/// Fragments embed their children directly — the walk follows
/// the tree, not SHA indirection.
import fragmentation.{type Fragment}
import gleam/list

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
  do_collect(root, [])
  |> list.reverse
}

fn do_collect(frag: Fragment, acc: List(Fragment)) -> List(Fragment) {
  let acc = [frag, ..acc]
  case fragmentation.children(frag) {
    [] -> acc
    children -> list.fold(children, acc, fn(a, child) { do_collect(child, a) })
  }
}

/// Fold over all fragments in a tree, depth-first.
pub fn fold(root: Fragment, acc: a, f: fn(a, Fragment) -> Visitor(a)) -> a {
  do_fold(root, acc, f)
}

fn do_fold(frag: Fragment, acc: a, f: fn(a, Fragment) -> Visitor(a)) -> a {
  case f(acc, frag) {
    Stop(result) -> result
    Continue(result) ->
      case fragmentation.children(frag) {
        [] -> result
        children ->
          list.fold(children, result, fn(a, child) { do_fold(child, a, f) })
      }
  }
}

/// Get the depth of a fragment tree.
pub fn depth(root: Fragment) -> Int {
  case fragmentation.children(root) {
    [] -> 0
    children -> {
      let max_child_depth =
        children
        |> list.map(fn(child) { depth(child) })
        |> list.fold(0, fn(best, d) {
          case d > best {
            True -> d
            False -> best
          }
        })
      1 + max_child_depth
    }
  }
}

/// Find the first fragment matching a predicate, depth-first.
pub fn find(
  root: Fragment,
  predicate: fn(Fragment) -> Bool,
) -> Result(Fragment, Nil) {
  case predicate(root) {
    True -> Ok(root)
    False ->
      fragmentation.children(root)
      |> do_find(predicate)
  }
}

fn do_find(
  fragments: List(Fragment),
  predicate: fn(Fragment) -> Bool,
) -> Result(Fragment, Nil) {
  case fragments {
    [] -> Error(Nil)
    [first, ..rest] ->
      case find(first, predicate) {
        Ok(found) -> Ok(found)
        Error(Nil) -> do_find(rest, predicate)
      }
  }
}
