use crate::fragment::{self, ContentAddressed, Fragmentable, TreeShaped};

/// A change between two fragment trees.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change<F> {
    /// Node exists only in the new tree.
    Added(F),
    /// Node exists only in the old tree.
    Removed(F),
    /// Same position, different content.
    Modified { old: F, new: F },
    /// Same ref, same content.
    Unchanged(F),
}

/// Diff two fragment trees by their roots.
/// Compares structurally: same hash = unchanged, different hash = modified.
/// Children compared positionally.
pub fn diff<F: Fragmentable + Clone>(old: &F, new: &F) -> Vec<Change<F>> {
    if fragment::content_oid(old) == fragment::content_oid(new) {
        vec![Change::Unchanged(old.clone())]
    } else {
        diff_nodes(old, new)
    }
}

fn diff_nodes<F: Fragmentable + Clone>(old: &F, new: &F) -> Vec<Change<F>> {
    let mut changes = vec![Change::Modified {
        old: old.clone(),
        new: new.clone(),
    }];

    let child_changes = diff_children(old.children(), new.children());
    changes.extend(child_changes);
    changes
}

fn diff_children<F: Fragmentable + Clone>(old: &[F], new: &[F]) -> Vec<Change<F>> {
    let mut changes = Vec::new();
    let max_len = old.len().max(new.len());

    for i in 0..max_len {
        match (old.get(i), new.get(i)) {
            (Some(o), Some(n)) => changes.extend(diff(o, n)),
            (None, Some(n)) => changes.push(Change::Added(n.clone())),
            (Some(o), None) => changes.push(Change::Removed(o.clone())),
            (None, None) => unreachable!(),
        }
    }

    changes
}

/// Summarize a list of changes: (added, removed, modified, unchanged).
pub fn summary<F>(changes: &[Change<F>]) -> (usize, usize, usize, usize) {
    changes
        .iter()
        .fold((0, 0, 0, 0), |(a, r, m, u), change| match change {
            Change::Added(_) => (a + 1, r, m, u),
            Change::Removed(_) => (a, r + 1, m, u),
            Change::Modified { .. } => (a, r, m + 1, u),
            Change::Unchanged(_) => (a, r, m, u + 1),
        })
}
