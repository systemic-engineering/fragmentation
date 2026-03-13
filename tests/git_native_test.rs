use fragmentation::fragment::{self, Fractal, Fragmentable};
use fragmentation::ref_::Ref;
use fragmentation::sha;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_shard(data: &str) -> Fractal<String> {
    let oid = fragment::blob_oid(data);
    Fractal::shard(Ref::new(sha::Sha(oid), "self"), data)
}

fn make_fractal(label: &str, data: &str, children: Vec<Fractal<String>>) -> Fractal<String> {
    let oid = fragment::tree_oid(data, &children);
    Fractal::new(Ref::new(sha::Sha(oid), label), data, children)
}

// ===========================================================================
// content_oid — in-memory git-compatible OID computation
// ===========================================================================

#[test]
fn content_oid_shard_matches_git_blob() {
    let shard = make_shard("hello");
    let oid = fragment::content_oid(&shard);
    // Known: printf "hello" | git hash-object --stdin
    assert_eq!(oid, "b6fc4c620b67d95f953a5c1c1230aaab5db5a1b0");
}

#[test]
fn content_oid_deterministic() {
    let shard = make_shard("deterministic");
    let oid1 = fragment::content_oid(&shard);
    let oid2 = fragment::content_oid(&shard);
    assert_eq!(oid1, oid2);
}

#[test]
fn content_oid_different_data_different_oid() {
    let s1 = make_shard("hello");
    let s2 = make_shard("world");
    assert_ne!(fragment::content_oid(&s1), fragment::content_oid(&s2));
}

#[test]
fn content_oid_shard_is_40_hex_chars() {
    let shard = make_shard("test");
    let oid = fragment::content_oid(&shard);
    assert_eq!(oid.len(), 40);
    assert!(oid.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn content_oid_fragment_differs_from_shard_same_data() {
    let shard = make_shard("a");
    let frag = make_fractal("test", "a", vec![]);
    assert_ne!(fragment::content_oid(&shard), fragment::content_oid(&frag));
}

#[test]
fn blob_oid_matches_git_hash_object() {
    // printf "a" | git hash-object --stdin
    assert_eq!(
        fragment::blob_oid("a"),
        "2e65efe2a145dda7ee51d1741299f848e5bf752e"
    );
}

#[test]
fn blob_oid_hello() {
    // printf "hello" | git hash-object --stdin
    assert_eq!(
        fragment::blob_oid("hello"),
        "b6fc4c620b67d95f953a5c1c1230aaab5db5a1b0"
    );
}

#[test]
fn tree_oid_is_40_hex_chars() {
    let oid = fragment::tree_oid::<String>("data", &[]);
    assert_eq!(oid.len(), 40);
    assert!(oid.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn tree_oid_different_children_different_oid() {
    let child_a = make_shard("alpha");
    let child_b = make_shard("beta");
    let oid1 = fragment::tree_oid("root", &[child_a.clone()]);
    let oid2 = fragment::tree_oid("root", &[child_b]);
    assert_ne!(oid1, oid2);
}

#[test]
fn tree_oid_children_order_matters() {
    let a = make_shard("a");
    let b = make_shard("b");
    let oid_ab = fragment::tree_oid("root", &[a.clone(), b.clone()]);
    let oid_ba = fragment::tree_oid("root", &[b, a]);
    assert_ne!(oid_ab, oid_ba);
}

// ===========================================================================
// write_tree — git2 blob/tree creation (requires "git" feature)
// ===========================================================================

#[cfg(feature = "git")]
mod git_native {
    use super::*;
    use fragmentation::commit::{Commit, Draft, Draftable};
    use fragmentation::git;
    use fragmentation::witnessed::{Author, Committer};

    fn init_repo() -> (tempfile::TempDir, git2::Repository) {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        (dir, repo)
    }

    #[test]
    fn write_tree_shard_creates_blob() {
        let (_dir, repo) = init_repo();
        let shard = make_shard("hello");
        let oid = git::write_tree(&repo, &shard).unwrap();
        let obj = repo.find_object(oid, None).unwrap();
        assert_eq!(obj.kind(), Some(git2::ObjectType::Blob));
    }

    #[test]
    fn write_tree_fragment_creates_tree() {
        let (_dir, repo) = init_repo();
        let child = make_shard("leaf");
        let parent = make_fractal("root", "root-data", vec![child]);
        let oid = git::write_tree(&repo, &parent).unwrap();
        let obj = repo.find_object(oid, None).unwrap();
        assert_eq!(obj.kind(), Some(git2::ObjectType::Tree));
    }

    #[test]
    fn write_tree_fragment_has_data_and_children() {
        let (_dir, repo) = init_repo();
        let child = make_shard("leaf");
        let parent = make_fractal("root", "root-data", vec![child]);
        let oid = git::write_tree(&repo, &parent).unwrap();
        let tree = repo.find_tree(oid).unwrap();
        assert_eq!(tree.len(), 2);
        assert!(tree.get_name(".data").is_some());
        assert!(tree.get_name("0000").is_some());
    }

    #[test]
    fn write_tree_oid_matches_content_oid() {
        let (_dir, repo) = init_repo();
        let shard = make_shard("verify");
        let git_oid = git::write_tree(&repo, &shard).unwrap();
        let mem_oid = fragment::content_oid(&shard);
        assert_eq!(git_oid.to_string(), mem_oid);
    }

    #[test]
    fn write_tree_fragment_oid_matches_content_oid() {
        let (_dir, repo) = init_repo();
        let child = make_shard("leaf");
        let parent = make_fractal("root", "parent-data", vec![child]);
        let git_oid = git::write_tree(&repo, &parent).unwrap();
        let mem_oid = fragment::content_oid(&parent);
        assert_eq!(git_oid.to_string(), mem_oid);
    }

    #[test]
    fn write_tree_dedup() {
        let (_dir, repo) = init_repo();
        let s1 = make_shard("same");
        let s2 = make_shard("same");
        let oid1 = git::write_tree(&repo, &s1).unwrap();
        let oid2 = git::write_tree(&repo, &s2).unwrap();
        assert_eq!(oid1, oid2);
    }

    // =================================================================
    // Draft::write — low-level API
    // =================================================================

    #[test]
    fn write_carries_metadata() {
        let (_dir, repo) = init_repo();
        let c = Draft::root("test commit", make_shard("committed"))
            .authored(Author::new("alex", "alex@systemic.engineer"))
            .write(&repo, Committer::new("reed", "reed@systemic.engineer"))
            .unwrap();
        let git_oid = git2::Oid::from_str(&c.sha().0).unwrap();
        let git_commit = repo.find_commit(git_oid).unwrap();
        assert_eq!(git_commit.author().name(), Some("alex"));
        assert_eq!(git_commit.committer().name(), Some("reed"));
        assert!(git_commit.message().unwrap().contains("test commit"));
    }

    #[test]
    fn write_sets_sha() {
        let (_dir, repo) = init_repo();
        let c = Draft::root("test", make_shard("x"))
            .write(&repo, Committer::new("test", "test@test"))
            .unwrap();
        assert_eq!(c.sha().0.len(), 40);
    }

    #[test]
    fn write_sets_timestamp() {
        let (_dir, repo) = init_repo();
        let c = Draft::root("test", make_shard("x"))
            .write(&repo, Committer::new("test", "test@test"))
            .unwrap();
        let ts: Result<i64, _> = c.witnessed().timestamp.0.parse();
        assert!(ts.is_ok(), "timestamp should be epoch seconds");
        assert!(ts.unwrap() > 1577836800, "timestamp should be recent");
    }

    #[test]
    fn write_default_author_from_committer() {
        let (_dir, repo) = init_repo();
        let c = Draft::root("test", make_shard("x"))
            .write(&repo, Committer::new("mara", "mara@systemic.engineer"))
            .unwrap();
        assert_eq!(c.witnessed().author.name, "mara");
        assert_eq!(c.witnessed().author.email, "mara@systemic.engineer");
        assert_eq!(c.witnessed().committer.name, "mara");
    }

    #[test]
    fn write_uses_email() {
        let (_dir, repo) = init_repo();
        let c = Draft::root("email commit", make_shard("email-test"))
            .authored(Author::new("mara", "mara@systemic.engineer"))
            .write(&repo, Committer::new("mara", "mara@systemic.engineer"))
            .unwrap();
        let git_oid = git2::Oid::from_str(&c.sha().0).unwrap();
        let git_commit = repo.find_commit(git_oid).unwrap();
        assert_eq!(git_commit.author().email(), Some("mara@systemic.engineer"));
        assert_eq!(
            git_commit.committer().email(),
            Some("mara@systemic.engineer")
        );
    }

    #[test]
    fn write_different_author_committer() {
        let (_dir, repo) = init_repo();
        let c = Draft::root("split commit", make_shard("split-identity"))
            .authored(Author::new("alex", "alex@example.com"))
            .write(&repo, Committer::new("reed", "reed@example.com"))
            .unwrap();
        let git_oid = git2::Oid::from_str(&c.sha().0).unwrap();
        let git_commit = repo.find_commit(git_oid).unwrap();
        assert_eq!(git_commit.author().name(), Some("alex"));
        assert_eq!(git_commit.author().email(), Some("alex@example.com"));
        assert_eq!(git_commit.committer().name(), Some("reed"));
        assert_eq!(git_commit.committer().email(), Some("reed@example.com"));
    }

    // =================================================================
    // Commit::Root vs Commit::Child
    // =================================================================

    #[test]
    fn write_root_has_no_parent() {
        let (_dir, repo) = init_repo();
        let c = Draft::root("root", make_shard("x"))
            .write(&repo, Committer::new("test", "test@test"))
            .unwrap();
        assert!(matches!(c, Commit::Root { .. }));
        assert!(c.parent().is_none());
    }

    #[test]
    fn write_child_has_parent() {
        let (_dir, repo) = init_repo();
        let committer = Committer::new("test", "test@test");
        let c1 = Draft::root("first", make_shard("first"))
            .write(&repo, committer.clone())
            .unwrap();
        let c2 = c1
            .child("second", make_shard("second"))
            .write(&repo, committer)
            .unwrap();
        assert!(matches!(c2, Commit::Child { .. }));
        assert!(c2.parent().is_some());
    }

    // =================================================================
    // Commit::child — parent chain
    // =================================================================

    #[test]
    fn write_parent_chain() {
        let (_dir, repo) = init_repo();
        let committer = Committer::new("test", "test@test");

        let c1 = Draft::root("first commit", make_shard("first"))
            .write(&repo, committer.clone())
            .unwrap();

        let c2 = c1
            .child("second commit", make_shard("second"))
            .write(&repo, committer)
            .unwrap();

        let oid1 = git2::Oid::from_str(&c1.sha().0).unwrap();
        let oid2 = git2::Oid::from_str(&c2.sha().0).unwrap();
        let git_commit2 = repo.find_commit(oid2).unwrap();
        assert_eq!(git_commit2.parent_count(), 1);
        assert_eq!(git_commit2.parent_id(0).unwrap(), oid1);
    }

    #[test]
    fn child_preserves_authored() {
        let (_dir, repo) = init_repo();
        let c1 = Draft::root("first", make_shard("first"))
            .authored(Author::new("alex", "alex@example.com"))
            .write(&repo, Committer::new("reed", "reed@example.com"))
            .unwrap();

        let c2 = c1
            .child("second", make_shard("second"))
            .authored(Author::new("mara", "mara@systemic.engineer"));
        assert_eq!(c2.author().unwrap().name, "mara");
        assert_eq!(c2.parent().unwrap().0, *c1.sha());
    }

    // =================================================================
    // Draft::write — direct commit at boundary
    // =================================================================

    #[test]
    fn commit_single_identity() {
        let (_dir, repo) = init_repo();
        let c = Draft::root("test", make_shard("x"))
            .authored(Author::new("mara", "mara@systemic.engineer"))
            .write(&repo, Committer::new("mara", "mara@systemic.engineer"))
            .unwrap();
        assert_eq!(c.witnessed().author.name, "mara");
        assert_eq!(c.witnessed().committer.name, "mara");
        assert_eq!(c.witnessed().author.email, "mara@systemic.engineer");
        assert_eq!(c.witnessed().committer.email, "mara@systemic.engineer");
    }

    #[test]
    fn commit_preserves_authored_vs_committer() {
        let (_dir, repo) = init_repo();
        let c = Draft::root("test", make_shard("x"))
            .authored(Author::new("alex", "alex@systemic.engineer"))
            .write(&repo, Committer::new("reed", "reed@systemic.engineer"))
            .unwrap();
        assert_eq!(c.witnessed().author.name, "alex");
        assert_eq!(c.witnessed().committer.name, "reed");
    }

    // =================================================================
    // Draftable for Commit
    // =================================================================

    #[test]
    fn commit_implements_draftable() {
        let (_dir, repo) = init_repo();
        let c = Draft::root("test", make_shard("x"))
            .write(&repo, Committer::new("test", "test@test"))
            .unwrap();
        fn accepts_draftable<T: Draftable>(_d: &T) {}
        accepts_draftable(&c);
    }

    #[test]
    fn commit_draftable_fractal() {
        let (_dir, repo) = init_repo();
        let c = Draft::root("test", make_shard("payload"))
            .write(&repo, Committer::new("test", "test@test"))
            .unwrap();
        let d: &dyn Draftable<Element = String> = &c;
        assert_eq!(d.fractal().data(), "payload");
    }

    #[test]
    fn commit_draftable_message() {
        let (_dir, repo) = init_repo();
        let c = Draft::root("the msg", make_shard("x"))
            .write(&repo, Committer::new("test", "test@test"))
            .unwrap();
        let d: &dyn Draftable<Element = String> = &c;
        assert_eq!(d.message().0, "the msg");
    }

    #[test]
    fn commit_root_draftable_parent_none() {
        let (_dir, repo) = init_repo();
        let c = Draft::root("test", make_shard("x"))
            .write(&repo, Committer::new("test", "test@test"))
            .unwrap();
        let d: &dyn Draftable<Element = String> = &c;
        assert!(d.parent().is_none());
    }

    #[test]
    fn commit_child_draftable_parent_some() {
        let (_dir, repo) = init_repo();
        let committer = Committer::new("test", "test@test");
        let c1 = Draft::root("first", make_shard("first"))
            .write(&repo, committer.clone())
            .unwrap();
        let c2 = c1
            .child("second", make_shard("second"))
            .write(&repo, committer)
            .unwrap();
        let d: &dyn Draftable<Element = String> = &c2;
        assert!(d.parent().is_some());
        assert_eq!(d.parent().unwrap().0, *c1.sha());
    }

    // =================================================================
    // read_tree roundtrip
    // =================================================================

    #[test]
    fn read_tree_roundtrip_shard() {
        let (_dir, repo) = init_repo();
        let shard = make_shard("roundtrip");
        let oid = git::write_tree(&repo, &shard).unwrap();
        let recovered = git::read_tree(&repo, oid).unwrap();
        assert!(recovered.is_shard());
        assert_eq!(recovered.data(), "roundtrip");
    }

    #[test]
    fn read_tree_roundtrip_fragment() {
        let (_dir, repo) = init_repo();
        let child = make_shard("leaf");
        let parent = make_fractal("root", "parent-data", vec![child]);
        let oid = git::write_tree(&repo, &parent).unwrap();
        let recovered = git::read_tree(&repo, oid).unwrap();
        assert!(recovered.is_fractal());
        assert_eq!(recovered.data(), "parent-data");
        assert_eq!(recovered.children().len(), 1);
        assert_eq!(recovered.children()[0].data(), "leaf");
    }

    #[test]
    fn read_tree_children_order_preserved() {
        let (_dir, repo) = init_repo();
        let a = make_shard("alpha");
        let b = make_shard("beta");
        let c = make_shard("gamma");
        let parent = make_fractal("root", "data", vec![a, b, c]);
        let oid = git::write_tree(&repo, &parent).unwrap();
        let recovered = git::read_tree(&repo, oid).unwrap();
        let data: Vec<&str> = recovered
            .children()
            .iter()
            .map(|f| f.data().as_str())
            .collect();
        assert_eq!(data, vec!["alpha", "beta", "gamma"]);
    }

    // =================================================================
    // read_witnessed — extract metadata from any git commit
    // =================================================================

    #[test]
    fn read_witnessed_from_real_repo() {
        let repo = git2::Repository::open("/Users/alexwolf/dev/projects/fragmentation").unwrap();
        let main_ref = repo.find_branch("main", git2::BranchType::Local).unwrap();
        let commit_oid = main_ref.get().target().unwrap();

        let (witnessed, message, tree_oid) = git::read_witnessed(&repo, commit_oid).unwrap();

        assert!(!witnessed.author.name.is_empty());
        assert!(!witnessed.committer.name.is_empty());
        assert!(!message.0.is_empty());
        assert!(!witnessed.timestamp.0.is_empty());

        let _tree = repo.find_tree(tree_oid).unwrap();
    }

    #[test]
    fn read_witnessed_matches_git2() {
        let repo = git2::Repository::open("/Users/alexwolf/dev/projects/fragmentation").unwrap();
        let main_ref = repo.find_branch("main", git2::BranchType::Local).unwrap();
        let commit_oid = main_ref.get().target().unwrap();

        let commit = repo.find_commit(commit_oid).unwrap();
        let (witnessed, _, _) = git::read_witnessed(&repo, commit_oid).unwrap();

        assert_eq!(witnessed.author.name, commit.author().name().unwrap());
        assert_eq!(witnessed.committer.name, commit.committer().name().unwrap());
        assert_eq!(witnessed.author.email, commit.author().email().unwrap());
        assert_eq!(
            witnessed.committer.email,
            commit.committer().email().unwrap()
        );
    }

    // =================================================================
    // read_commit — full roundtrip for fragmentation commits
    // =================================================================

    #[test]
    fn read_commit_roundtrip() {
        let (_dir, repo) = init_repo();
        let c = Draft::root("roundtrip test", make_shard("roundtrip-commit"))
            .authored(Author::new("mara", "mara@systemic.engineer"))
            .write(&repo, Committer::new("mara", "mara@systemic.engineer"))
            .unwrap();

        let recovered = git::read_commit(&repo, git2::Oid::from_str(&c.sha().0).unwrap()).unwrap();
        assert_eq!(recovered.witnessed().author.name, "mara");
        assert_eq!(recovered.witnessed().author.email, "mara@systemic.engineer");
        assert_eq!(recovered.witnessed().committer.name, "mara");
        assert_eq!(
            recovered.witnessed().committer.email,
            "mara@systemic.engineer"
        );
        assert!(recovered.message().0.contains("roundtrip test"));
        assert_eq!(recovered.fractal().data(), "roundtrip-commit");
        assert!(recovered.parent().is_none());
        assert!(matches!(recovered, Commit::Root { .. }));
    }

    #[test]
    fn read_commit_parent_chain_roundtrip() {
        let (_dir, repo) = init_repo();
        let committer = Committer::new("test", "test@test");

        let c1 = Draft::root("first", make_shard("first"))
            .write(&repo, committer.clone())
            .unwrap();

        let c2 = c1
            .child("second", make_shard("second"))
            .write(&repo, committer)
            .unwrap();

        let recovered = git::read_commit(&repo, git2::Oid::from_str(&c2.sha().0).unwrap()).unwrap();
        assert_eq!(recovered.parent().unwrap().0, *c1.sha());
        assert_eq!(recovered.fractal().data(), "second");
        assert!(matches!(recovered, Commit::Child { .. }));
    }

    #[test]
    fn read_commit_captures_email() {
        let (_dir, repo) = init_repo();
        let c = Draft::root("email roundtrip test", make_shard("email-roundtrip"))
            .authored(Author::new("mara", "mara@systemic.engineer"))
            .write(&repo, Committer::new("cairn", "cairn@systemic.engineer"))
            .unwrap();

        let recovered = git::read_commit(&repo, git2::Oid::from_str(&c.sha().0).unwrap()).unwrap();
        assert_eq!(recovered.witnessed().author.name, "mara");
        assert_eq!(recovered.witnessed().author.email, "mara@systemic.engineer");
        assert_eq!(recovered.witnessed().committer.name, "cairn");
        assert_eq!(
            recovered.witnessed().committer.email,
            "cairn@systemic.engineer"
        );
    }

    // =================================================================
    // Signature
    // =================================================================

    #[test]
    fn commit_signature_unsigned() {
        let (_dir, repo) = init_repo();
        let c = Draft::root("unsigned commit", make_shard("unsigned"))
            .write(&repo, Committer::new("test", "test@test"))
            .unwrap();
        let git_oid = git2::Oid::from_str(&c.sha().0).unwrap();
        let sig = git::commit_signature(&repo, git_oid).unwrap();
        assert!(sig.is_none());
    }
}
