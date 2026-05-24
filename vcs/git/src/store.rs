//! GitStore — persistent content-addressed store backed by git.

use fragmentation::encoding::Decode;
use fragmentation::fragment::{Fragmentable, Reconstructable};
use fragmentation::repo::Repo;
use fragmentation::sha::HashAlg;

/// Content-addressed store backed by the git object database.
///
/// Two tiers:
///   Memory: in-process HashMap (hot path, no I/O per operation).
///   Git:    git2 object database (persistent, cloneable, shareable).
///
/// Write: memory only (fast). Call `flush()` to persist to git.
/// Read:  memory first, then git, then miss.
///
/// After `flush()`, everything is in the git repo. `git clone` includes it.
pub struct GitStore<N: Fragmentable + Clone> {
    memory: fragmentation::store::Store<N, N::Hash>,
    repo: git2::Repository,
}

impl<N: Reconstructable + Clone> GitStore<N>
where
    N::Data: Decode,
{
    /// Open a GitStore backed by the git repo at (or above) the given path.
    pub fn open(path: &std::path::Path) -> Result<Self, git2::Error> {
        let repo = git2::Repository::discover(path)?;
        Ok(GitStore {
            memory: fragmentation::store::Store::new(),
            repo,
        })
    }

    /// Open a GitStore from an existing git2::Repository.
    pub fn from_repo(repo: git2::Repository) -> Self {
        GitStore {
            memory: fragmentation::store::Store::new(),
            repo,
        }
    }

    /// Flush: write objects to git ODB + update git ref.
    /// Returns the number of objects written.
    pub fn flush(&self) -> usize {
        let mut count = 0;

        // Write objects to git ODB.
        for oid in self.memory.keys() {
            if let Some(node) = Repo::read_tree(&self.memory, &oid) {
                if crate::git::write_node(&self.repo, &node).is_ok() {
                    count += 1;
                }
            }
        }

        // Collapse the index to a single tree, update the ref.
        if let Ok(tree_oid) = self.collapse_index() {
            let _ = self
                .repo
                .reference("refs/store/index", tree_oid, true, "collapse");
        }

        count
    }

    /// Hydrate: read git ref + refract.
    pub fn hydrate(&mut self) {
        let oid = match self
            .repo
            .find_reference("refs/store/index")
            .ok()
            .and_then(|r| r.target())
        {
            Some(oid) => oid,
            None => return,
        };

        let _ = self.refract_from(oid);
    }

    /// Number of in-memory objects.
    pub fn memory_count(&self) -> usize {
        self.memory.object_count()
    }

    /// Access the underlying git2::Repository.
    pub fn repo(&self) -> &git2::Repository {
        &self.repo
    }

    /// Refract (hydrate) from a specific tree OID.
    fn refract_from(&mut self, oid: git2::Oid) -> Result<(), git2::Error> {
        let tree = self.repo.find_tree(oid)?;
        for entry in tree.iter() {
            if let Some(name) = entry.name() {
                let target_oid = entry.id().to_string();
                Repo::update_ref(&mut self.memory, name, N::Hash::from_hex(target_oid));
            }
        }
        Ok(())
    }
}

/// Collapse the in-memory ref index to a single git tree OID.
impl<N: Reconstructable + Clone> GitStore<N>
where
    N::Data: Decode,
{
    /// Collapse: serialize all in-memory refs as entries in one git tree.
    /// Returns the tree OID. Merges with any existing index on disk.
    pub fn collapse_index(&self) -> Result<git2::Oid, git2::Error> {
        let existing_tree = self
            .repo
            .find_reference("refs/store/index")
            .ok()
            .and_then(|r| r.peel_to_tree().ok());
        let mut builder = self.repo.treebuilder(existing_tree.as_ref())?;

        for ref_name in self.memory.ref_names() {
            if let Some(sha) = Repo::resolve_ref(&self.memory, ref_name) {
                let oid = git2::Oid::from_str(sha.as_str())?;
                builder.insert(ref_name, oid, 0o100644)?;
            }
        }

        builder.write()
    }
}

impl<N: Reconstructable + Clone> Repo for GitStore<N>
where
    N::Data: Decode,
{
    type Node = N;
    type Hash = N::Hash;

    fn write_tree(&mut self, node: &N) -> String {
        self.memory.write_tree(node)
    }

    fn read_tree(&self, oid: &str) -> Option<N> {
        // Tier 1: memory.
        if let Some(node) = self.memory.read_tree(oid) {
            return Some(node);
        }
        // Tier 2: git.
        let git_oid = git2::Oid::from_str(oid).ok()?;
        crate::git::read_node::<N>(&self.repo, git_oid).ok()
    }

    fn write_commit(&mut self, commit: fragmentation::commit::Commit<N, N::Hash>) {
        self.memory.write_commit(commit);
    }

    fn read_commit(&self, sha: &N::Hash) -> Option<fragmentation::commit::Commit<N, N::Hash>> {
        self.memory.read_commit(sha)
    }

    fn update_ref(&mut self, name: &str, sha: N::Hash) {
        self.memory.update_ref(name, sha);
    }

    fn resolve_ref(&self, name: &str) -> Option<N::Hash> {
        // Memory only — call hydrate() on startup to populate from git.
        self.memory.resolve_ref(name)
    }
}
