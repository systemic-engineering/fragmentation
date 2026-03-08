use fragmentation::commit::Commit;
use fragmentation::fragment::{self, Fractal, Fragment};
use fragmentation::ref_::Ref;
use fragmentation::sha;
use fragmentation::witnessed::Author;

fn make_shard(data: &str) -> Fractal<String> {
    let r = Ref::new(sha::Sha(fragment::blob_oid(data)), "self");
    Fractal::shard(r, data)
}

// ===========================================================================
// Construction
// ===========================================================================

#[test]
fn commit_root_construction() {
    let shard = make_shard("hello");
    let commit = Commit::root("initial", shard);
    assert_eq!(commit.message().0, "initial");
    assert!(commit.parent().is_none());
    assert!(commit.sha().is_none());
}

#[test]
fn commit_with_parent() {
    let shard = make_shard("data");
    let parent = sha::Sha("abc123".into());
    let commit = Commit::new("child commit", shard, parent.clone());
    assert_eq!(commit.parent(), Some(&parent));
}

#[test]
fn commit_fractal_accessor() {
    let shard = make_shard("payload");
    let commit = Commit::root("test", shard);
    assert_eq!(commit.fractal().data(), "payload");
}

#[test]
fn commit_message_accessor() {
    let shard = make_shard("x");
    let commit = Commit::root("the message", shard);
    assert_eq!(commit.message().0, "the message");
}

#[test]
fn commit_witnessed_empty_by_default() {
    let shard = make_shard("x");
    let commit = Commit::root("test", shard);
    assert_eq!(commit.witnessed().author.name, "");
    assert_eq!(commit.witnessed().committer.name, "");
}

#[test]
fn commit_message_from_string() {
    let msg = String::from("owned message");
    let commit = Commit::root(msg, make_shard("x"));
    assert_eq!(commit.message().0, "owned message");
}

// ===========================================================================
// authored()
// ===========================================================================

#[test]
fn authored_stamps_name() {
    let commit = Commit::root("test", make_shard("x"))
        .authored(Author::new("mara", "mara@systemic.engineer"));
    assert_eq!(commit.witnessed().author.name, "mara");
}

#[test]
fn authored_stamps_email() {
    let commit = Commit::root("test", make_shard("x"))
        .authored(Author::new("mara", "mara@systemic.engineer"));
    assert_eq!(commit.witnessed().author.email, "mara@systemic.engineer");
}

#[test]
fn authored_preserves_message() {
    let commit = Commit::root("preserved", make_shard("x"))
        .authored(Author::new("mara", "mara@systemic.engineer"));
    assert_eq!(commit.message().0, "preserved");
}

#[test]
fn authored_preserves_fractal() {
    let commit = Commit::root("test", make_shard("payload"))
        .authored(Author::new("mara", "mara@systemic.engineer"));
    assert_eq!(commit.fractal().data(), "payload");
}

#[test]
fn authored_preserves_parent() {
    let parent = sha::Sha("parent123".into());
    let commit = Commit::new("test", make_shard("x"), parent.clone())
        .authored(Author::new("mara", "mara@systemic.engineer"));
    assert_eq!(commit.parent(), Some(&parent));
}
