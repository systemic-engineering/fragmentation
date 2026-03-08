use fragmentation::actor::Actor;
use fragmentation::commit::Commit;
use fragmentation::fragment::{self, Fractal, Fragment};
use fragmentation::ref_::Ref;
use fragmentation::sha;

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
    let commit = Commit::root(shard, "initial");
    assert_eq!(commit.message().0, "initial");
    assert!(commit.parent().is_none());
}

#[test]
fn commit_with_parent() {
    let shard = make_shard("data");
    let parent = sha::Sha("abc123".into());
    let commit = Commit::new(shard, "child commit", parent.clone());
    assert_eq!(commit.parent(), Some(&parent));
}

#[test]
fn commit_fractal_accessor() {
    let shard = make_shard("payload");
    let commit = Commit::root(shard, "test");
    assert_eq!(commit.fractal().data(), "payload");
}

#[test]
fn commit_message_accessor() {
    let shard = make_shard("x");
    let commit = Commit::root(shard, "the message");
    assert_eq!(commit.message().0, "the message");
}

#[test]
fn commit_witnessed_empty_by_default() {
    let shard = make_shard("x");
    let commit = Commit::root(shard, "test");
    assert_eq!(commit.witnessed().author.name, "");
    assert_eq!(commit.witnessed().committer.name, "");
}

// ===========================================================================
// Actor stamps
// ===========================================================================

#[test]
fn actor_author_stamps_name() {
    let actor = Actor::identity("mara", "mara@systemic.engineer");
    let commit = Commit::root(make_shard("x"), "test");
    let commit = actor.author(commit);
    assert_eq!(commit.witnessed().author.name, "mara");
}

#[test]
fn actor_author_stamps_email() {
    let actor = Actor::identity("mara", "mara@systemic.engineer");
    let commit = Commit::root(make_shard("x"), "test");
    let commit = actor.author(commit);
    assert_eq!(commit.witnessed().author.email, "mara@systemic.engineer");
}

#[test]
fn actor_author_stamps_timestamp() {
    let actor = Actor::identity("mara", "mara@systemic.engineer");
    let commit = Commit::root(make_shard("x"), "test");
    let commit = actor.author(commit);
    let ts: Result<i64, _> = commit.witnessed().timestamp.0.parse();
    assert!(ts.is_ok(), "timestamp should be epoch seconds");
    assert!(ts.unwrap() > 1577836800, "timestamp should be recent");
}

#[test]
fn actor_commit_stamps_committer_name() {
    let actor = Actor::identity("reed", "reed@systemic.engineer");
    let commit = Commit::root(make_shard("x"), "test");
    let commit = actor.commit(commit);
    assert_eq!(commit.witnessed().committer.name, "reed");
}

#[test]
fn actor_commit_stamps_committer_email() {
    let actor = Actor::identity("reed", "reed@systemic.engineer");
    let commit = Commit::root(make_shard("x"), "test");
    let commit = actor.commit(commit);
    assert_eq!(commit.witnessed().committer.email, "reed@systemic.engineer");
}

#[test]
fn two_actors_author_and_committer() {
    let alice = Actor::identity("alice", "alice@example.com");
    let bob = Actor::identity("bob", "bob@example.com");
    let commit = Commit::root(make_shard("patch"), "alice's patch applied by bob");
    let commit = alice.author(commit);
    let commit = bob.commit(commit);
    assert_eq!(commit.witnessed().author.name, "alice");
    assert_eq!(commit.witnessed().committer.name, "bob");
    assert_eq!(commit.witnessed().author.email, "alice@example.com");
    assert_eq!(commit.witnessed().committer.email, "bob@example.com");
}

#[test]
fn commit_preserves_message_through_stamps() {
    let actor = Actor::identity("mara", "mara@systemic.engineer");
    let commit = Commit::root(make_shard("x"), "preserved");
    let commit = actor.author(commit);
    let commit = actor.commit(commit);
    assert_eq!(commit.message().0, "preserved");
}

#[test]
fn commit_preserves_fractal_through_stamps() {
    let actor = Actor::identity("mara", "mara@systemic.engineer");
    let shard = make_shard("payload");
    let commit = Commit::root(shard, "test");
    let commit = actor.author(commit);
    let commit = actor.commit(commit);
    assert_eq!(commit.fractal().data(), "payload");
}

#[test]
fn commit_preserves_parent_through_stamps() {
    let actor = Actor::identity("mara", "mara@systemic.engineer");
    let parent = sha::Sha("parent123".into());
    let commit = Commit::new(make_shard("x"), "test", parent.clone());
    let commit = actor.author(commit);
    let commit = actor.commit(commit);
    assert_eq!(commit.parent(), Some(&parent));
}

#[test]
fn commit_message_from_string() {
    let msg = String::from("owned message");
    let commit = Commit::root(make_shard("x"), msg);
    assert_eq!(commit.message().0, "owned message");
}
