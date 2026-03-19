use std::collections::HashMap;

use crate::commit::Commit;
use crate::fragment::{content_oid, Blob, Fractal, Fragmentable};
use crate::repo::Repo;
use crate::sha::{HashAlg, Sha};

/// In-memory content-addressed store.
#[derive(Clone, Debug)]
pub struct Store<N: Fragmentable + Clone = Fractal<Blob>, H: HashAlg = Sha> {
    objects: HashMap<String, N>,
    commits: HashMap<String, Commit<N, H>>,
    refs: HashMap<String, H>,
}

impl<N: Fragmentable + Clone, H: HashAlg> Store<N, H> {
    /// Create an empty store.
    pub fn new() -> Self {
        Store {
            objects: HashMap::new(),
            commits: HashMap::new(),
            refs: HashMap::new(),
        }
    }

    /// Number of stored objects (trees + blobs).
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    /// Merge another store into this one. Same OID = same content.
    pub fn merge(&mut self, other: Store<N, H>) {
        self.objects.extend(other.objects);
        self.commits.extend(other.commits);
        self.refs.extend(other.refs);
    }

    /// List all object OIDs.
    pub fn keys(&self) -> Vec<String> {
        self.objects.keys().cloned().collect()
    }

    /// List all ref names.
    pub fn ref_names(&self) -> Vec<&str> {
        self.refs.keys().map(|s| s.as_str()).collect()
    }
}

impl<N: Fragmentable + Clone, H: HashAlg> Default for Store<N, H> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N: Fragmentable + Clone, H: HashAlg> Repo for Store<N, H> {
    type Node = N;
    type Hash = H;

    fn write_tree(&mut self, node: &N) -> String {
        for child in node.children() {
            self.write_tree(child);
        }
        let oid = content_oid(node);
        self.objects
            .entry(oid.clone())
            .or_insert_with(|| node.clone());
        oid
    }

    fn read_tree(&self, oid: &str) -> Option<N> {
        self.objects.get(oid).cloned()
    }

    fn write_commit(&mut self, commit: Commit<N, H>) {
        self.commits
            .insert(commit.sha().as_str().to_string(), commit);
    }

    fn read_commit(&self, sha: &H) -> Option<Commit<N, H>> {
        self.commits.get(sha.as_str()).cloned()
    }

    fn update_ref(&mut self, name: &str, sha: H) {
        self.refs.insert(name.to_string(), sha);
    }

    fn resolve_ref(&self, name: &str) -> Option<H> {
        self.refs.get(name).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commit::Draft;
    use crate::encoding;
    use crate::witnessed::Committer;

    #[cfg(feature = "git")]
    use crate::witnessed::Author;

    fn test_fractal() -> Fractal<String> {
        encoding::encode("hello world")
    }

    fn test_committer() -> Committer {
        Committer::new("Test", "test@test.com")
    }

    const TEST_TIMESTAMP: &str = "1234567890 +0000";

    // -- Repo trait conformance --

    fn uses_repo_trait(r: &mut impl Repo<Node = Fractal<String>>) {
        let fractal = test_fractal();
        let oid = r.write_tree(&fractal);
        assert!(r.read_tree(&oid).is_some());
    }

    #[test]
    fn store_implements_repo() {
        let mut store = Store::<Fractal<String>>::new();
        uses_repo_trait(&mut store);
    }

    // -- Basic operations --

    #[test]
    fn store_empty() {
        let store = Store::<Fractal<String>>::new();
        assert!(store.read_commit(&Sha("anything".into())).is_none());
        assert!(store.resolve_ref("HEAD").is_none());
    }

    #[test]
    fn store_write_read_tree() {
        let mut store = Store::<Fractal<String>>::new();
        let fractal = test_fractal();
        let oid = store.write_tree(&fractal);
        let read_back = store.read_tree(&oid).expect("should find tree");
        assert_eq!(read_back, fractal);
    }

    #[test]
    fn store_commit_root() {
        let mut store = Store::<Fractal<String>>::new();
        let fractal = test_fractal();
        let draft = Draft::root("initial", fractal);
        let commit = draft.commit(&mut store, test_committer(), TEST_TIMESTAMP);
        assert!(matches!(commit, Commit::Root { .. }));
        assert!(!commit.sha().0.is_empty());
    }

    #[test]
    fn store_commit_shard() {
        let mut store = Store::<Fractal<String>>::new();
        let shard = Fractal::shard(
            crate::ref_::Ref::new(crate::sha::Sha(crate::fragment::blob_oid("leaf")), "self"),
            "leaf",
        );
        let draft = Draft::root("shard commit", shard);
        let commit = draft.commit(&mut store, test_committer(), TEST_TIMESTAMP);
        assert!(matches!(commit, Commit::Root { .. }));
        assert!(!commit.sha().0.is_empty());
    }

    #[test]
    fn store_commit_child() {
        let mut store = Store::<Fractal<String>>::new();
        let fractal = test_fractal();
        let root_draft = Draft::root("root", fractal.clone());
        let root = root_draft.commit(&mut store, test_committer(), TEST_TIMESTAMP);
        let child_draft = root.child("child", fractal);
        let child = child_draft.commit(&mut store, test_committer(), "1234567891 +0000");
        assert!(matches!(child, Commit::Child { .. }));
        assert_ne!(child.sha(), root.sha());
    }

    #[cfg(feature = "git")]
    #[test]
    fn store_commit_sha_matches_git() {
        let fractal = test_fractal();
        let timestamp = "1234567890 +0000";
        let author = Author::new("Test", "test@test.com");
        let committer = Committer::new("Test", "test@test.com");

        // In-memory
        let mut store = Store::<Fractal<String>>::new();
        let draft = Draft::root("test commit", fractal.clone()).authored(author.clone());
        let mem_commit = draft.commit(&mut store, committer.clone(), timestamp);

        // git2
        let tmp = tempfile::tempdir().unwrap();
        let git_repo = git2::Repository::init(tmp.path()).unwrap();
        let tree_oid = crate::git::write_tree(&git_repo, &fractal).unwrap();
        let tree = git_repo.find_tree(tree_oid).unwrap();
        let epoch: i64 = 1234567890;
        let git_sig =
            git2::Signature::new("Test", "test@test.com", &git2::Time::new(epoch, 0)).unwrap();
        let git_oid = git_repo
            .commit(None, &git_sig, &git_sig, "test commit", &tree, &[])
            .unwrap();

        assert_eq!(mem_commit.sha().0, git_oid.to_string());
    }

    #[test]
    fn store_refs() {
        let mut store = Store::<Fractal<String>>::new();
        let sha = Sha("abc123".into());
        store.update_ref("refs/heads/main", sha.clone());
        assert_eq!(store.resolve_ref("refs/heads/main"), Some(sha));
        assert_eq!(store.resolve_ref("refs/heads/other"), None);
    }

    #[test]
    fn store_commit_chain() {
        let mut store = Store::<Fractal<String>>::new();
        let fractal = test_fractal();

        let c1 = Draft::root("first", fractal.clone()).commit(
            &mut store,
            test_committer(),
            "1000000000 +0000",
        );
        let c2 = c1.child("second", fractal.clone()).commit(
            &mut store,
            test_committer(),
            "1000000001 +0000",
        );
        let c3 =
            c2.child("third", fractal)
                .commit(&mut store, test_committer(), "1000000002 +0000");

        assert!(matches!(
            store.read_commit(c3.sha()),
            Some(Commit::Child { .. })
        ));
        assert!(matches!(
            store.read_commit(c2.sha()),
            Some(Commit::Child { .. })
        ));
        assert!(matches!(
            store.read_commit(c1.sha()),
            Some(Commit::Root { .. })
        ));
    }

    #[test]
    fn store_ref_names() {
        let mut store = Store::<Fractal<String>>::new();
        assert!(store.ref_names().is_empty());
        store.update_ref("refs/heads/main", Sha("abc".into()));
        store.update_ref("grammar/test", Sha("def".into()));
        let mut names = store.ref_names();
        names.sort();
        assert_eq!(names, vec!["grammar/test", "refs/heads/main"]);
    }

    #[test]
    fn store_deduplication() {
        let mut store = Store::<Fractal<String>>::new();
        let fractal = test_fractal();
        let oid1 = store.write_tree(&fractal);
        let oid2 = store.write_tree(&fractal);
        assert_eq!(oid1, oid2);
    }
}
