use fragmentation::fragment::{self, Fragment};
use fragmentation::ref_::Ref;
use fragmentation::sha;
use fragmentation::witnessed::{Author, Committer, Message, Timestamp, Witnessed};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_witnessed() -> Witnessed {
    Witnessed::new(
        Author("alex".into()),
        Committer("reed".into()),
        Timestamp("2026-03-01T00:00:00Z".into()),
        Message("test".into()),
    )
}

fn alt_witnessed() -> Witnessed {
    Witnessed::new(
        Author("mara".into()),
        Committer("cairn".into()),
        Timestamp("2026-03-07T00:00:00Z".into()),
        Message("different observer".into()),
    )
}

fn make_shard(data: &str) -> Fragment {
    let oid = fragment::blob_oid(data);
    Fragment::shard(Ref::new(sha::Sha(oid), "self"), test_witnessed(), data)
}

fn make_fragment(label: &str, data: &str, children: Vec<Fragment>) -> Fragment {
    let oid = fragment::tree_oid(data, &children);
    Fragment::new_fragment(
        Ref::new(sha::Sha(oid), label),
        test_witnessed(),
        data,
        children,
    )
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
fn content_oid_same_data_different_witness_same_oid() {
    let r = Ref::new(sha::Sha("placeholder".into()), "self");
    let s1 = Fragment::shard(r.clone(), test_witnessed(), "same-data");
    let s2 = Fragment::shard(r, alt_witnessed(), "same-data");
    // Core semantic change: witness does NOT affect content OID
    assert_eq!(fragment::content_oid(&s1), fragment::content_oid(&s2));
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
    // A shard with data "a" (blob) must differ from a fragment with data "a" (tree)
    // This is the structural collision prevention that replaces labeled_hash
    let shard = make_shard("a");
    let frag = make_fragment("test", "a", vec![]);
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
    let oid = fragment::tree_oid("data", &[]);
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
        let parent = make_fragment("root", "root-data", vec![child]);
        let oid = git::write_tree(&repo, &parent).unwrap();
        let obj = repo.find_object(oid, None).unwrap();
        assert_eq!(obj.kind(), Some(git2::ObjectType::Tree));
    }

    #[test]
    fn write_tree_fragment_has_data_and_children() {
        let (_dir, repo) = init_repo();
        let child = make_shard("leaf");
        let parent = make_fragment("root", "root-data", vec![child]);
        let oid = git::write_tree(&repo, &parent).unwrap();
        let tree = repo.find_tree(oid).unwrap();
        // .data entry + one numbered child
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
        let parent = make_fragment("root", "parent-data", vec![child]);
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
        let parent = make_fragment("root", "parent-data", vec![child]);
        let oid = git::write_tree(&repo, &parent).unwrap();
        let recovered = git::read_tree(&repo, oid).unwrap();
        assert!(recovered.is_fragment());
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
        let parent = make_fragment("root", "data", vec![a, b, c]);
        let oid = git::write_tree(&repo, &parent).unwrap();
        let recovered = git::read_tree(&repo, oid).unwrap();
        let data: Vec<&str> = recovered.children().iter().map(|f| f.data()).collect();
        assert_eq!(data, vec!["alpha", "beta", "gamma"]);
    }
}
