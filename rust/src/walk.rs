use crate::fragment::Fragment;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Visitor<A> {
    Continue(A),
    Stop(A),
}

pub fn collect(_root: &Fragment) -> Vec<&Fragment> {
    todo!("implement collect")
}

pub fn fold<A>(_root: &Fragment, _acc: A, _f: &dyn Fn(A, &Fragment) -> Visitor<A>) -> A {
    todo!("implement fold")
}

pub fn depth(_root: &Fragment) -> usize {
    todo!("implement depth")
}

pub fn find<'a>(
    _root: &'a Fragment,
    _predicate: &dyn Fn(&Fragment) -> bool,
) -> Option<&'a Fragment> {
    todo!("implement find")
}
