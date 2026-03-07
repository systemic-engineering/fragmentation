use fragmentation::fragment;
use fragmentation::git;
use fragmentation::ref_::Ref;
use fragmentation::sha;
use fragmentation::witnessed::{Author, Committer, Message, Timestamp, Witnessed};
use std::path::Path;

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

fn make_shard(data: &str) -> fragment::Fragment {
    let r = Ref::new(sha::hash(data), "self");
    fragment::Fragment::shard(r, test_witnessed(), data)
}

// ---------------------------------------------------------------------------
// write_fragment_creates_file
// ---------------------------------------------------------------------------

#[test]
fn write_fragment_creates_file() {
    let dir = tempfile::tempdir().unwrap();
    let frag = make_shard("hello-world");
    let sha = fragment::hash_fragment(&frag);

    let result = git::write(&frag, dir.path().to_str().unwrap());
    assert!(result.is_ok());

    let path = dir.path().join(&sha);
    assert!(path.exists());
}

// ---------------------------------------------------------------------------
// write_fragment_idempotent
// ---------------------------------------------------------------------------

#[test]
fn write_fragment_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let frag = make_shard("idempotent-shard");
    let sha = fragment::hash_fragment(&frag);

    let r1 = git::write(&frag, dir.path().to_str().unwrap());
    let r2 = git::write(&frag, dir.path().to_str().unwrap());

    assert!(r1.is_ok());
    assert!(r2.is_ok());

    let path = dir.path().join(&sha);
    assert!(path.exists());
}

// ---------------------------------------------------------------------------
// write_two_fragments
// ---------------------------------------------------------------------------

#[test]
fn write_two_fragments() {
    let dir = tempfile::tempdir().unwrap();
    let frag_a = make_shard("fragment-alpha");
    let frag_b = make_shard("fragment-beta");
    let sha_a = fragment::hash_fragment(&frag_a);
    let sha_b = fragment::hash_fragment(&frag_b);

    git::write(&frag_a, dir.path().to_str().unwrap()).unwrap();
    git::write(&frag_b, dir.path().to_str().unwrap()).unwrap();

    assert_ne!(sha_a, sha_b);
    assert!(Path::new(&dir.path().join(&sha_a)).exists());
    assert!(Path::new(&dir.path().join(&sha_b)).exists());
}
