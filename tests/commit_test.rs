use fragmentation::commit::{Draft, Draftable, Parent};
use fragmentation::fragment::{self, Fractal, Fragmentable};
use fragmentation::ref_::Ref;
use fragmentation::sha::{self, Sha};
use fragmentation::witnessed::Author;

fn make_shard(data: &str) -> Fractal<String> {
    let r = Ref::new(sha::Sha(fragment::blob_oid(data)), "self");
    Fractal::shard(r, data)
}

// ===========================================================================
// Parent
// ===========================================================================

#[test]
fn parent_construction() {
    let parent = Parent(sha::Sha("abc123".into()));
    assert_eq!(parent.0, sha::Sha("abc123".into()));
}

#[test]
fn parent_clone() {
    let parent = Parent(sha::Sha("abc".into()));
    let cloned = parent.clone();
    assert_eq!(parent, cloned);
}

// ===========================================================================
// Draft construction
// ===========================================================================

#[test]
fn draft_root_construction() {
    let shard = make_shard("hello");
    let draft: Draft<Fractal<String>> = Draft::root("initial", shard);
    assert_eq!(draft.message().0, "initial");
    assert!(draft.parent().is_none());
    assert!(draft.author().is_none());
}

#[test]
fn draft_with_parent() {
    let shard = make_shard("data");
    let parent = Parent(sha::Sha("abc123".into()));
    let draft: Draft<Fractal<String>> = Draft::new("child commit", shard, parent.clone());
    assert_eq!(draft.parent(), Some(&parent));
}

#[test]
fn draft_node_accessor() {
    let shard = make_shard("payload");
    let draft: Draft<Fractal<String>> = Draft::root("test", shard);
    assert_eq!(draft.node().data(), "payload");
}

#[test]
fn draft_message_accessor() {
    let shard = make_shard("x");
    let draft: Draft<Fractal<String>> = Draft::root("the message", shard);
    assert_eq!(draft.message().0, "the message");
}

#[test]
fn draft_message_from_string() {
    let msg = String::from("owned message");
    let draft: Draft<Fractal<String>> = Draft::root(msg, make_shard("x"));
    assert_eq!(draft.message().0, "owned message");
}

// ===========================================================================
// Draft::authored()
// ===========================================================================

#[test]
fn authored_stamps_name() {
    let draft: Draft<Fractal<String>> = Draft::root("test", make_shard("x"));
    let draft = draft.authored(Author::new("mara", "mara@systemic.engineer"));
    assert_eq!(draft.author().unwrap().name, "mara");
}

#[test]
fn authored_stamps_email() {
    let draft: Draft<Fractal<String>> = Draft::root("test", make_shard("x"));
    let draft = draft.authored(Author::new("mara", "mara@systemic.engineer"));
    assert_eq!(draft.author().unwrap().email, "mara@systemic.engineer");
}

#[test]
fn authored_preserves_message() {
    let draft: Draft<Fractal<String>> = Draft::root("preserved", make_shard("x"));
    let draft = draft.authored(Author::new("mara", "mara@systemic.engineer"));
    assert_eq!(draft.message().0, "preserved");
}

#[test]
fn authored_preserves_node() {
    let draft: Draft<Fractal<String>> = Draft::root("test", make_shard("payload"));
    let draft = draft.authored(Author::new("mara", "mara@systemic.engineer"));
    assert_eq!(draft.node().data(), "payload");
}

#[test]
fn authored_preserves_parent() {
    let parent = Parent(sha::Sha("parent123".into()));
    let draft: Draft<Fractal<String>> = Draft::new("test", make_shard("x"), parent.clone());
    let draft = draft.authored(Author::new("mara", "mara@systemic.engineer"));
    assert_eq!(draft.parent(), Some(&parent));
}

// ===========================================================================
// Draftable trait
// ===========================================================================

#[test]
fn draft_implements_draftable() {
    let draft: Draft<Fractal<String>> = Draft::root("test", make_shard("x"));
    fn accepts_draftable<T: Draftable>(_d: &T) {}
    accepts_draftable(&draft);
}

#[test]
fn draftable_node() {
    let draft: Draft<Fractal<String>> = Draft::root("test", make_shard("payload"));
    let d: &dyn Draftable<Node = Fractal<String>, Hash = Sha> = &draft;
    assert_eq!(d.node().data(), "payload");
}

#[test]
fn draftable_message() {
    let draft: Draft<Fractal<String>> = Draft::root("the msg", make_shard("x"));
    let d: &dyn Draftable<Node = Fractal<String>, Hash = Sha> = &draft;
    assert_eq!(d.message().0, "the msg");
}

#[test]
fn draftable_parent_none() {
    let draft: Draft<Fractal<String>> = Draft::root("test", make_shard("x"));
    let d: &dyn Draftable<Node = Fractal<String>, Hash = Sha> = &draft;
    assert!(d.parent().is_none());
}

#[test]
fn draftable_parent_some() {
    let parent = Parent(sha::Sha("abc".into()));
    let draft: Draft<Fractal<String>> = Draft::new("test", make_shard("x"), parent.clone());
    let d: &dyn Draftable<Node = Fractal<String>, Hash = Sha> = &draft;
    assert_eq!(d.parent(), Some(&parent));
}
