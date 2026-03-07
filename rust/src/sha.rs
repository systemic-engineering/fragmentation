#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Sha(pub String);

pub fn hash(_data: &str) -> Sha {
    todo!("implement hash")
}
