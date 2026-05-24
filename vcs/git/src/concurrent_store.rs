//! Git integration for ConcurrentStore.
//!
//! Extension trait that adds flush/hydrate/collapse_index to ConcurrentStore
//! without pulling git2 into the core fragmentation crate.

use fragmentation::concurrent_store::ConcurrentStore;
use fragmentation::encoding::Decode;
use fragmentation::fragment::{Fragmentable, Reconstructable};
use fragmentation::sha::HashAlg;

/// Git persistence extension for ConcurrentStore.
pub trait ConcurrentStoreGitExt<N: Fragmentable + Clone, H: HashAlg>
where
    N: Reconstructable,
    N::Data: Decode,
{
    /// Write all in-memory objects to the git ODB and update refs/store/index.
    /// Returns the number of objects written.
    fn flush(&self, repo: &git2::Repository) -> usize;

    /// Load refs from git (refs/store/index) into memory.
    fn hydrate(&self, repo: &git2::Repository);

    /// Collapse: serialize all in-memory refs as entries in one git tree.
    fn collapse_index(&self, repo: &git2::Repository) -> Result<git2::Oid, git2::Error>;
}

impl<N, H> ConcurrentStoreGitExt<N, H> for ConcurrentStore<N, H>
where
    N: Fragmentable + Clone + Reconstructable,
    N::Data: Decode,
    H: HashAlg,
{
    fn flush(&self, repo: &git2::Repository) -> usize {
        let mut count = 0;

        for oid in self.keys() {
            if let Some(node) = self.read_tree(&oid) {
                if crate::git::write_node(repo, &node).is_ok() {
                    count += 1;
                }
            }
        }

        match self.collapse_index(repo) {
            Ok(tree_oid) => {
                if let Err(e) = repo.reference("refs/store/index", tree_oid, true, "collapse") {
                    eprintln!("warning: failed to update refs/store/index: {e}");
                }
            }
            Err(e) => {
                eprintln!("warning: collapse_index failed: {e}");
            }
        }

        count
    }

    fn hydrate(&self, repo: &git2::Repository) {
        let oid = match repo
            .find_reference("refs/store/index")
            .ok()
            .and_then(|r| r.target())
        {
            Some(oid) => oid,
            None => return,
        };

        if let Ok(tree) = repo.find_tree(oid) {
            for entry in tree.iter() {
                if let Some(name) = entry.name() {
                    if let Ok(blob) = repo.find_blob(entry.id()) {
                        if let Ok(content) = std::str::from_utf8(blob.content()) {
                            self.update_ref(name, H::from_hex(content));
                        }
                    }
                }
            }
        }
    }

    fn collapse_index(&self, repo: &git2::Repository) -> Result<git2::Oid, git2::Error> {
        let existing_tree = repo
            .find_reference("refs/store/index")
            .ok()
            .and_then(|r| r.peel_to_tree().ok());
        let mut builder = repo.treebuilder(existing_tree.as_ref())?;

        for ref_name in self.ref_names() {
            if let Some(sha) = self.resolve_ref(&ref_name) {
                let blob_oid = repo.blob(sha.as_str().as_bytes())?;
                builder.insert(ref_name.as_str(), blob_oid, 0o100644)?;
            }
        }

        builder.write()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fragmentation::concurrent_store::ConcurrentStore;
    use fragmentation::encoding;
    use fragmentation::fragment::Fractal;
    use fragmentation::sha::Sha;

    fn test_fractal() -> Fractal<String> {
        encoding::encode("hello world")
    }

    #[test]
    fn flush_writes_objects_to_git() {
        let store = ConcurrentStore::<Fractal<String>>::new();
        let fractal = test_fractal();
        store.write_tree(&fractal);

        let tmp = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(tmp.path()).unwrap();
        let count = store.flush(&repo);
        assert!(count > 0);
    }

    #[test]
    fn flush_updates_index_ref() {
        let store = ConcurrentStore::<Fractal<String>>::new();
        let fractal = test_fractal();
        let oid = store.write_tree(&fractal);
        store.update_ref("test_ref", Sha(oid));

        let tmp = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(tmp.path()).unwrap();
        store.flush(&repo);

        let reference = repo.find_reference("refs/store/index");
        assert!(reference.is_ok());
    }

    #[test]
    fn hydrate_loads_refs_from_git() {
        let store1 = ConcurrentStore::<Fractal<String>>::new();
        let fractal = test_fractal();
        let oid = store1.write_tree(&fractal);
        store1.update_ref("eigen_test", Sha(oid.clone()));

        let tmp = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(tmp.path()).unwrap();
        store1.flush(&repo);

        let store2 = ConcurrentStore::<Fractal<String>>::new();
        store2.hydrate(&repo);
        let resolved = store2.resolve_ref("eigen_test");
        assert_eq!(resolved, Some(Sha(oid)));
    }

    #[test]
    fn flush_then_hydrate_roundtrip() {
        let store1 = ConcurrentStore::<Fractal<String>>::new();
        let fractal = test_fractal();
        let oid = store1.write_tree(&fractal);
        store1.update_ref("ref_a", Sha(oid.clone()));
        store1.update_ref("ref_b", Sha("deadbeef".into()));

        let tmp = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(tmp.path()).unwrap();
        store1.flush(&repo);

        let store2 = ConcurrentStore::<Fractal<String>>::new();
        store2.hydrate(&repo);

        assert_eq!(store2.resolve_ref("ref_a"), Some(Sha(oid)));
        assert_eq!(store2.resolve_ref("ref_b"), Some(Sha("deadbeef".into())));
    }
}
