import fragmentation
import fragmentation/commit
import gleam/option.{None, Some}
import gleam/string

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_node() -> fragmentation.Fragment(String) {
  let r = fragmentation.ref_(fragmentation.hash("data"), "self")
  fragmentation.shard(r, "hello")
}

fn test_committer() -> commit.Committer {
  commit.Committer("reed", "reed@systemic.engineer")
}

fn test_timestamp() -> commit.Timestamp {
  commit.Timestamp("2026-03-01T00:00:00Z")
}

fn hash_node(frag: fragmentation.Fragment(String)) -> String {
  fragmentation.hash_string_fragment(frag)
}

// ===========================================================================
// Author
// ===========================================================================

pub fn author_construction_test() {
  let a = commit.Author("alex", "alex@example.com")
  assert a == commit.Author("alex", "alex@example.com")
}

pub fn author_fields_test() {
  let a = commit.Author("alex", "alex@example.com")
  assert a.name == "alex"
  assert a.email == "alex@example.com"
}

// ===========================================================================
// Committer
// ===========================================================================

pub fn committer_construction_test() {
  let c = commit.Committer("reed", "reed@systemic.engineer")
  assert c == commit.Committer("reed", "reed@systemic.engineer")
}

pub fn committer_fields_test() {
  let c = commit.Committer("reed", "reed@systemic.engineer")
  assert c.name == "reed"
  assert c.email == "reed@systemic.engineer"
}

// ===========================================================================
// Timestamp
// ===========================================================================

pub fn timestamp_construction_test() {
  let t = commit.Timestamp("2026-03-01T00:00:00Z")
  assert t == commit.Timestamp("2026-03-01T00:00:00Z")
}

// ===========================================================================
// Message
// ===========================================================================

pub fn message_construction_test() {
  let m = commit.Message("initial commit")
  assert m == commit.Message("initial commit")
}

// ===========================================================================
// Witnessed
// ===========================================================================

pub fn witnessed_construction_test() {
  let w =
    commit.Witnessed(
      commit.Author("alex", "alex@example.com"),
      commit.Committer("reed", "reed@systemic.engineer"),
      commit.Timestamp("2026-03-01T00:00:00Z"),
    )
  assert w.author == commit.Author("alex", "alex@example.com")
  assert w.committer == commit.Committer("reed", "reed@systemic.engineer")
  assert w.timestamp == commit.Timestamp("2026-03-01T00:00:00Z")
}

// ===========================================================================
// Draft
// ===========================================================================

pub fn root_draft_test() {
  let node = make_node()
  let d = commit.root("initial", node)
  assert d.message == commit.Message("initial")
  assert d.node == node
  assert d.parent == None
}

pub fn child_draft_test() {
  let node = make_node()
  let parent = commit.Parent(fragmentation.sha("abc"))
  let d = commit.child("second", node, parent)
  assert d.message == commit.Message("second")
  assert d.parent == Some(parent)
}

pub fn authored_sets_author_test() {
  let node = make_node()
  let d = commit.root("initial", node)
  assert d.author == None
  let a = commit.Author("alex", "alex@example.com")
  let d2 = commit.authored(d, a)
  assert d2.author == Some(a)
}

pub fn authored_preserves_other_fields_test() {
  let node = make_node()
  let d = commit.root("initial", node)
  let a = commit.Author("alex", "alex@example.com")
  let d2 = commit.authored(d, a)
  assert d2.node == node
  assert d2.message == commit.Message("initial")
  assert d2.parent == None
}

// ===========================================================================
// Commit construction
// ===========================================================================

pub fn commit_root_creates_root_test() {
  let node = make_node()
  let d = commit.root("initial", node)
  let c = commit.commit(d, test_committer(), test_timestamp(), hash_node)
  let assert commit.Root(_, _, _, _) = c
}

pub fn commit_child_creates_child_test() {
  let node = make_node()
  let parent = commit.Parent(fragmentation.sha("abc"))
  let d = commit.child("second", node, parent)
  let c = commit.commit(d, test_committer(), test_timestamp(), hash_node)
  let assert commit.Child(_, _, _, _, _) = c
}

// ===========================================================================
// Commit SHA
// ===========================================================================

pub fn commit_sha_length_test() {
  let node = make_node()
  let d = commit.root("initial", node)
  let c = commit.commit(d, test_committer(), test_timestamp(), hash_node)
  let fragmentation.Sha(sha_str) = commit.sha(c)
  assert string.length(sha_str) == 128
}

pub fn commit_sha_deterministic_test() {
  let node = make_node()
  let d = commit.root("initial", node)
  let c1 = commit.commit(d, test_committer(), test_timestamp(), hash_node)
  let c2 = commit.commit(d, test_committer(), test_timestamp(), hash_node)
  assert commit.sha(c1) == commit.sha(c2)
}

pub fn commit_different_message_different_sha_test() {
  let node = make_node()
  let c1 =
    commit.commit(
      commit.root("first", node),
      test_committer(),
      test_timestamp(),
      hash_node,
    )
  let c2 =
    commit.commit(
      commit.root("second", node),
      test_committer(),
      test_timestamp(),
      hash_node,
    )
  assert commit.sha(c1) != commit.sha(c2)
}

pub fn commit_different_committer_different_sha_test() {
  // The observer is part of the hash. Different witness → different commit SHA.
  let node = make_node()
  let c1 =
    commit.commit(
      commit.root("same", node),
      commit.Committer("alice", "alice@example.com"),
      test_timestamp(),
      hash_node,
    )
  let c2 =
    commit.commit(
      commit.root("same", node),
      commit.Committer("bob", "bob@example.com"),
      test_timestamp(),
      hash_node,
    )
  assert commit.sha(c1) != commit.sha(c2)
}

pub fn commit_different_timestamp_different_sha_test() {
  let node = make_node()
  let c1 =
    commit.commit(
      commit.root("same", node),
      test_committer(),
      commit.Timestamp("2026-03-01T00:00:00Z"),
      hash_node,
    )
  let c2 =
    commit.commit(
      commit.root("same", node),
      test_committer(),
      commit.Timestamp("2026-03-02T00:00:00Z"),
      hash_node,
    )
  assert commit.sha(c1) != commit.sha(c2)
}

// ===========================================================================
// Commit accessors
// ===========================================================================

pub fn commit_witnessed_test() {
  let node = make_node()
  let committer = commit.Committer("reed", "reed@systemic.engineer")
  let timestamp = commit.Timestamp("2026-03-01T00:00:00Z")
  let d = commit.root("initial", node)
  let c = commit.commit(d, committer, timestamp, hash_node)
  let w = commit.witnessed(c)
  assert w.committer == committer
  assert w.timestamp == timestamp
}

pub fn commit_node_test() {
  let node = make_node()
  let d = commit.root("initial", node)
  let c = commit.commit(d, test_committer(), test_timestamp(), hash_node)
  assert commit.node(c) == node
}

pub fn commit_message_test() {
  let node = make_node()
  let d = commit.root("initial", node)
  let c = commit.commit(d, test_committer(), test_timestamp(), hash_node)
  assert commit.message(c) == commit.Message("initial")
}

pub fn commit_root_parent_is_none_test() {
  let node = make_node()
  let d = commit.root("initial", node)
  let c = commit.commit(d, test_committer(), test_timestamp(), hash_node)
  assert commit.parent(c) == None
}

pub fn commit_child_parent_is_some_test() {
  let node = make_node()
  let parent = commit.Parent(fragmentation.sha("abc"))
  let d = commit.child("second", node, parent)
  let c = commit.commit(d, test_committer(), test_timestamp(), hash_node)
  assert commit.parent(c) == Some(parent)
}

pub fn authored_commit_witness_uses_author_test() {
  let node = make_node()
  let author = commit.Author("alex", "alex@example.com")
  let d = commit.authored(commit.root("initial", node), author)
  let c = commit.commit(d, test_committer(), test_timestamp(), hash_node)
  let w = commit.witnessed(c)
  assert w.author == author
}

pub fn unset_author_defaults_to_committer_test() {
  // When no author is set, committer acts as author
  let node = make_node()
  let committer = commit.Committer("reed", "reed@systemic.engineer")
  let d = commit.root("initial", node)
  let c = commit.commit(d, committer, test_timestamp(), hash_node)
  let w = commit.witnessed(c)
  assert w.author == commit.Author("reed", "reed@systemic.engineer")
}
