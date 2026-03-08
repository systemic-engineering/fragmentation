use crate::fragment::Fragmentable;

/// What to do when visiting a fragment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Visitor<A> {
    /// Continue walking children.
    Continue(A),
    /// Stop walking this branch.
    Stop(A),
}

/// Collect all fragments in a tree, depth-first.
pub fn collect<F: Fragmentable>(root: &F) -> Vec<&F> {
    let mut acc = Vec::new();
    do_collect(root, &mut acc);
    acc
}

fn do_collect<'a, F: Fragmentable>(frag: &'a F, acc: &mut Vec<&'a F>) {
    acc.push(frag);
    for child in frag.children() {
        do_collect(child, acc);
    }
}

/// Fold over all fragments in a tree, depth-first.
pub fn fold<A, F: Fragmentable>(root: &F, acc: A, f: &dyn Fn(A, &F) -> Visitor<A>) -> A {
    do_fold(root, acc, f)
}

fn do_fold<A, F: Fragmentable>(frag: &F, acc: A, f: &dyn Fn(A, &F) -> Visitor<A>) -> A {
    match f(acc, frag) {
        Visitor::Stop(result) => result,
        Visitor::Continue(result) => frag
            .children()
            .iter()
            .fold(result, |a, child| do_fold(child, a, f)),
    }
}

/// Get the depth of a fragment tree.
pub fn depth<F: Fragmentable>(root: &F) -> usize {
    match root.children() {
        [] => 0,
        children => {
            let max_child_depth = children.iter().map(depth).max().unwrap_or(0);
            1 + max_child_depth
        }
    }
}

/// Find the first fragment matching a predicate, depth-first.
pub fn find<'a, F: Fragmentable>(root: &'a F, predicate: &dyn Fn(&F) -> bool) -> Option<&'a F> {
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
