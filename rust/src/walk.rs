use crate::fragment::Fragment;

/// What to do when visiting a fragment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Visitor<A> {
    /// Continue walking children.
    Continue(A),
    /// Stop walking this branch.
    Stop(A),
}

/// Collect all fragments in a tree, depth-first.
pub fn collect(root: &Fragment) -> Vec<&Fragment> {
    let mut acc = Vec::new();
    do_collect(root, &mut acc);
    acc
}

fn do_collect<'a>(frag: &'a Fragment, acc: &mut Vec<&'a Fragment>) {
    acc.push(frag);
    for child in frag.children() {
        do_collect(child, acc);
    }
}

/// Fold over all fragments in a tree, depth-first.
pub fn fold<A>(root: &Fragment, acc: A, f: &dyn Fn(A, &Fragment) -> Visitor<A>) -> A {
    do_fold(root, acc, f)
}

fn do_fold<A>(frag: &Fragment, acc: A, f: &dyn Fn(A, &Fragment) -> Visitor<A>) -> A {
    match f(acc, frag) {
        Visitor::Stop(result) => result,
        Visitor::Continue(result) => frag
            .children()
            .iter()
            .fold(result, |a, child| do_fold(child, a, f)),
    }
}

/// Get the depth of a fragment tree.
pub fn depth(root: &Fragment) -> usize {
    match root.children() {
        [] => 0,
        children => {
            let max_child_depth = children.iter().map(depth).max().unwrap_or(0);
            1 + max_child_depth
        }
    }
}

/// Find the first fragment matching a predicate, depth-first.
pub fn find<'a>(root: &'a Fragment, predicate: &dyn Fn(&Fragment) -> bool) -> Option<&'a Fragment> {
    if predicate(root) {
        return Some(root);
    }
    for child in root.children() {
        if let Some(found) = find(child, predicate) {
            return Some(found);
        }
    }
    None
}
