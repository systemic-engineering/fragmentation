use std::collections::HashMap;

use crate::fragment::Fragment;
use crate::sha::Sha;

#[derive(Clone, Debug)]
pub struct Store {
    fragments: HashMap<String, Fragment>,
}

impl Store {
    pub fn new() -> Self {
        Store {
            fragments: HashMap::new(),
        }
    }

    pub fn put(&mut self, _frag: Fragment) {
        todo!("implement put")
    }

    pub fn get(&self, _sha: &Sha) -> Option<&Fragment> {
        todo!("implement get")
    }

    pub fn has(&self, _sha: &Sha) -> bool {
        todo!("implement has")
    }

    pub fn size(&self) -> usize {
        self.fragments.len()
    }

    pub fn merge(&mut self, _other: Store) {
        todo!("implement merge")
    }

    pub fn keys(&self) -> Vec<Sha> {
        todo!("implement keys")
    }
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}
