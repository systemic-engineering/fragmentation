use std::collections::HashMap;

use crate::fragment::Fragment;
use crate::sha::Sha;

/// Content-addressed fragment store.
#[derive(Clone, Debug)]
pub struct Store<E = String> {
    fragments: HashMap<String, Fragment<E>>,
}

impl<E> Store<E> {
    /// Create an empty store.
    pub fn new() -> Self {
        Store {
            fragments: HashMap::new(),
        }
    }

    /// Insert a fragment by its self-ref SHA.
    pub fn put(&mut self, frag: Fragment<E>) {
        let key = frag.self_ref().sha.0.clone();
        self.fragments.insert(key, frag);
    }

    /// Look up a fragment by SHA.
    pub fn get(&self, sha: &Sha) -> Option<&Fragment<E>> {
        self.fragments.get(&sha.0)
    }

    /// Check if a fragment exists.
    pub fn has(&self, sha: &Sha) -> bool {
        self.fragments.contains_key(&sha.0)
    }

    /// Count fragments in the store.
    pub fn size(&self) -> usize {
        self.fragments.len()
    }

    /// Merge another store into this one. Same SHA = same content.
    pub fn merge(&mut self, other: Store<E>) {
        self.fragments.extend(other.fragments);
    }

    /// List all SHAs in the store.
    pub fn keys(&self) -> Vec<Sha> {
        self.fragments.keys().map(|k| Sha(k.clone())).collect()
    }
}

impl<E> Default for Store<E> {
    fn default() -> Self {
        Self::new()
    }
}
