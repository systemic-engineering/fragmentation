use crate::fragment::Fragment;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    Added(Fragment),
    Removed(Fragment),
    Modified { old: Fragment, new: Fragment },
    Unchanged(Fragment),
}

pub fn diff(_old: &Fragment, _new: &Fragment) -> Vec<Change> {
    todo!("implement diff")
}

pub fn summary(_changes: &[Change]) -> (usize, usize, usize, usize) {
    todo!("implement summary")
}
