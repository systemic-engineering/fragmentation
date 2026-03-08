use fragmentation::fragment::{self, Fractal, Fragment};
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
    use fragmentation::git;
    use fragmentation::witnessed::{Author, Committer, Message, Timestamp, Witnessed};

    fn init_repo() -> (tempfile::TempDir, git2::Repository) {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        (dir, repo)
    }

    fn test_witnessed() -> Witnessed {
        Witnessed::new(
            Author("alex".into()),
            Committer("reed".into()),
            Timestamp("2026-03-01T00:00:00Z".into()),
            Message("test".into()),
        )
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

    #[test]
    fn write_commit_carries_witness_metadata() {
        let (_dir, repo) = init_repo();
        let shard = make_shard("committed");
        let w = test_witnessed();
        let oid = git::write_commit(&repo, &shard, &w, "test commit", None).unwrap();
        let commit = repo.find_commit(oid).unwrap();
        assert_eq!(commit.author().name(), Some("alex"));
        assert!(commit.message().unwrap().contains("test commit"));
    }

    #[test]
    fn write_commit_parent_chain() {
        let (_dir, repo) = init_repo();
        let s1 = make_shard("first");
        let w = test_witnessed();
        let oid1 = git::write_commit(&repo, &s1, &w, "first commit", None).unwrap();
        let commit1 = repo.find_commit(oid1).unwrap();

        let s2 = make_shard("second");
        let oid2 = git::write_commit(&repo, &s2, &w, "second commit", Some(&commit1)).unwrap();
        let commit2 = repo.find_commit(oid2).unwrap();
        assert_eq!(commit2.parent_count(), 1);
        assert_eq!(commit2.parent_id(0).unwrap(), oid1);
    }

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

    // =====================================================================
    // read_commit — extract Witnessed from git commits
    // =====================================================================

    #[test]
    fn read_commit_from_real_repo() {
        // Open THIS repo and read HEAD of main
        let repo = git2::Repository::open("/Users/alexwolf/dev/projects/fragmentation").unwrap();
        let main_ref = repo.find_branch("main", git2::BranchType::Local).unwrap();
        let commit_oid = main_ref.get().target().unwrap();

        let (witnessed, tree_oid) = git::read_commit(&repo, commit_oid).unwrap();

        // Main has real commits with real authors
        assert!(!witnessed.author.0.is_empty());
        assert!(!witnessed.committer.0.is_empty());
        assert!(!witnessed.message.0.is_empty());
        assert!(!witnessed.timestamp.0.is_empty());

        // Tree OID should be valid
        let _tree = repo.find_tree(tree_oid).unwrap();
    }

    #[test]
    fn read_commit_witnessed_matches_git2() {
        // Verify our Witnessed extraction matches what git2 reports
        let repo = git2::Repository::open("/Users/alexwolf/dev/projects/fragmentation").unwrap();
        let main_ref = repo.find_branch("main", git2::BranchType::Local).unwrap();
        let commit_oid = main_ref.get().target().unwrap();

        let commit = repo.find_commit(commit_oid).unwrap();
        let (witnessed, _) = git::read_commit(&repo, commit_oid).unwrap();

        assert_eq!(witnessed.author.0, commit.author().name().unwrap());
        assert_eq!(witnessed.committer.0, commit.committer().name().unwrap());
    }

    #[test]
    fn read_commit_roundtrip() {
        // Write a commit with write_commit, read it back with read_commit
        let (_dir, repo) = init_repo();
        let shard = make_shard("roundtrip-commit");
        let w = test_witnessed();
        let oid = git::write_commit(&repo, &shard, &w, "roundtrip test", None).unwrap();

        let (recovered, tree_oid) = git::read_commit(&repo, oid).unwrap();
        assert_eq!(recovered.author.0, "alex");
        assert_eq!(recovered.committer.0, "reed");
        assert!(recovered.message.0.contains("roundtrip test"));

        // Tree OID should be valid
        let _tree = repo.find_tree(tree_oid).unwrap();
    }

    #[test]
    fn commit_signature_unsigned() {
        // Commits in test repos are unsigned
        let (_dir, repo) = init_repo();
        let shard = make_shard("unsigned");
        let w = test_witnessed();
        let oid = git::write_commit(&repo, &shard, &w, "unsigned commit", None).unwrap();

        let sig = git::commit_signature(&repo, oid).unwrap();
        assert!(sig.is_none());
    }
}
