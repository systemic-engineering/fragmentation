import fragmentation
import fragmentation/diff
import fragmentation/store
import fragmentation/walk
import gleam/list
import gleam/string
import gleeunit

pub fn main() -> Nil {
  gleeunit.main()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_witnessed() -> fragmentation.Witnessed {
  fragmentation.witnessed("alex", "reed", "2026-03-01T00:00:00Z", "test")
}

fn make_shard(data: String) -> fragmentation.Fragment {
  let r = fragmentation.ref(fragmentation.hash(data), "self")
  fragmentation.shard(r, test_witnessed(), data)
}

fn make_fragment(
  label: String,
  children: List(fragmentation.Fragment),
) -> fragmentation.Fragment {
  let r = fragmentation.ref(fragmentation.hash(label), "self")
  fragmentation.fragment(r, test_witnessed(), label, children)
}

// ===========================================================================
// SHA
// ===========================================================================

pub fn sha_construction_test() {
  let s = fragmentation.sha("abc123")
  assert s == fragmentation.Sha("abc123")
}

pub fn hash_returns_sha_test() {
  let s = fragmentation.hash("test")
  let fragmentation.Sha(value) = s
  assert string.length(value) == 64
}

pub fn hash_deterministic_test() {
  let h1 = fragmentation.hash("same")
  let h2 = fragmentation.hash("same")
  assert h1 == h2
}

pub fn hash_different_input_different_sha_test() {
  let h1 = fragmentation.hash("hello")
  let h2 = fragmentation.hash("world")
  assert h1 != h2
}

// ===========================================================================
// Ref
// ===========================================================================

pub fn ref_construction_test() {
  let s = fragmentation.sha("abc")
  let r = fragmentation.ref(s, "parent")
  assert r == fragmentation.Ref(fragmentation.Sha("abc"), "parent")
}

// ===========================================================================
// Witnessed value types
// ===========================================================================

pub fn author_construction_test() {
  let a = fragmentation.author("alex")
  assert a == fragmentation.Author("alex")
}

pub fn committer_construction_test() {
  let c = fragmentation.committer("reed")
  assert c == fragmentation.Committer("reed")
}

pub fn timestamp_construction_test() {
  let t = fragmentation.timestamp("2026-03-01T00:00:00Z")
  assert t == fragmentation.Timestamp("2026-03-01T00:00:00Z")
}

pub fn message_construction_test() {
  let m = fragmentation.message("commit msg")
  assert m == fragmentation.Message("commit msg")
}

// ===========================================================================
// Meta
// ===========================================================================

pub fn witnessed_construction_test() {
  let w = fragmentation.witnessed(
    fragmentation.author("alex"),
    fragmentation.committer("reed"),
    fragmentation.timestamp("2026-03-01T00:00:00Z"),
    fragmentation.message("initial"),
  )
  assert w
    == fragmentation.Witnessed(
      fragmentation.Author("alex"),
      fragmentation.Committer("reed"),
      fragmentation.Timestamp("2026-03-01T00:00:00Z"),
      fragmentation.Message("initial"),
    )
}

pub fn witnessed_serialize_deterministic_test() {
  let w = test_witnessed()
  let s1 = fragmentation.serialize_witnessed(w)
  let s2 = fragmentation.serialize_witnessed(w)
  assert s1 == s2
}

pub fn witnessed_fields_in_serialization_test() {
  let w = fragmentation.witnessed("alex", "reed", "2026-03-01", "commit msg")
  let s = fragmentation.serialize_witnessed(w)
  assert string.contains(s, "author:alex")
  assert string.contains(s, "committer:reed")
  assert string.contains(s, "timestamp:2026-03-01")
  assert string.contains(s, "message:commit msg")
}

// ===========================================================================
// Fragment construction
// ===========================================================================

pub fn shard_construction_test() {
  let r = fragmentation.ref(fragmentation.hash("data"), "self")
  let w = test_witnessed()
  let s = fragmentation.shard(r, w, "hello")
  assert s == fragmentation.Shard(r, w, "hello")
}

pub fn fragment_construction_test() {
  let leaf = make_shard("leaf-data")
  let r = fragmentation.ref(fragmentation.hash("root"), "self")
  let w = test_witnessed()
  let f = fragmentation.fragment(r, w, "root-data", [leaf])
  assert f == fragmentation.Fragment(r, w, "root-data", [leaf])
}

pub fn fragment_empty_children_test() {
  let f = make_fragment("empty", [])
  assert fragmentation.children(f) == []
}

pub fn fragment_multiple_children_test() {
  let a = make_shard("alpha")
  let b = make_shard("beta")
  let f = make_fragment("parent", [a, b])
  assert fragmentation.children(f) == [a, b]
}

// ===========================================================================
// Queries
// ===========================================================================

pub fn self_ref_shard_test() {
  let s = make_shard("data")
  let r = fragmentation.self_ref(s)
  let fragmentation.Ref(sha, _) = r
  assert sha == fragmentation.hash("data")
}

pub fn self_ref_fragment_test() {
  let f = make_fragment("node", [])
  let r = fragmentation.self_ref(f)
  let fragmentation.Ref(sha, _) = r
  assert sha == fragmentation.hash("node")
}

pub fn self_witnessed_test() {
  let s = make_shard("x")
  assert fragmentation.self_witnessed(s) == test_witnessed()
}

pub fn data_shard_test() {
  let s = make_shard("payload")
  assert fragmentation.data(s) == "payload"
}

pub fn data_fragment_test() {
  let f = make_fragment("payload", [])
  assert fragmentation.data(f) == "payload"
}

pub fn is_shard_test() {
  assert fragmentation.is_shard(make_shard("x")) == True
  assert fragmentation.is_shard(make_fragment("x", [])) == False
}

pub fn is_fragment_test() {
  assert fragmentation.is_fragment(make_fragment("x", [])) == True
  assert fragmentation.is_fragment(make_shard("x")) == False
}

pub fn children_shard_test() {
  assert fragmentation.children(make_shard("x")) == []
}

// ===========================================================================
// Content addressing
// ===========================================================================

pub fn hash_fragment_deterministic_test() {
  let s = make_shard("hello")
  let h1 = fragmentation.hash_fragment(s)
  let h2 = fragmentation.hash_fragment(s)
  assert h1 == h2
}

pub fn hash_fragment_different_data_test() {
  let s1 = make_shard("hello")
  let s2 = make_shard("world")
  assert fragmentation.hash_fragment(s1) != fragmentation.hash_fragment(s2)
}

pub fn hash_fragment_witnessed_matters_test() {
  let r = fragmentation.ref(fragmentation.hash("x"), "self")
  let w1 = fragmentation.witnessed("alex", "reed", "2026-03-01", "first")
  let w2 = fragmentation.witnessed("alex", "reed", "2026-03-01", "second")
  let s1 = fragmentation.shard(r, w1, "same-data")
  let s2 = fragmentation.shard(r, w2, "same-data")
  // Different witness = different hash (different witness = different reality)
  assert fragmentation.hash_fragment(s1) != fragmentation.hash_fragment(s2)
}

pub fn serialize_roundtrip_hash_test() {
  let s = make_shard("test")
  let hash_direct = fragmentation.hash_fragment(s)
  let fragmentation.Sha(hash_via_serial) =
    fragmentation.hash(fragmentation.serialize(s))
  assert hash_direct == hash_via_serial
}

pub fn serialize_shard_not_empty_test() {
  let s = make_shard("data")
  assert fragmentation.serialize(s) != ""
}

pub fn serialize_fragment_not_empty_test() {
  let f = make_fragment("root", [make_shard("leaf")])
  assert fragmentation.serialize(f) != ""
}

// ===========================================================================
// Store
// ===========================================================================

pub fn store_new_is_empty_test() {
  let s = store.new()
  assert store.size(s) == 0
}

pub fn store_put_and_get_test() {
  let frag = make_shard("hello")
  let s = store.put(store.new(), frag)
  let fragmentation.Ref(sha, _) = fragmentation.self_ref(frag)
  assert store.get(s, sha) == Ok(frag)
}

pub fn store_has_test() {
  let frag = make_shard("exists")
  let s = store.put(store.new(), frag)
  let fragmentation.Ref(sha, _) = fragmentation.self_ref(frag)
  assert store.has(s, sha) == True
  assert store.has(s, fragmentation.sha("nonexistent")) == False
}

pub fn store_size_test() {
  let s = store.new()
  assert store.size(s) == 0
  let s = store.put(s, make_shard("a"))
  assert store.size(s) == 1
  let s = store.put(s, make_shard("b"))
  assert store.size(s) == 2
}

pub fn store_put_idempotent_test() {
  let frag = make_shard("same")
  let s = store.put(store.new(), frag)
  let s = store.put(s, frag)
  assert store.size(s) == 1
}

pub fn store_get_missing_test() {
  let s = store.new()
  assert store.get(s, fragmentation.sha("nope")) == Error(Nil)
}

pub fn store_merge_test() {
  let a = store.put(store.new(), make_shard("alpha"))
  let b = store.put(store.new(), make_shard("beta"))
  let merged = store.merge(a, b)
  assert store.size(merged) == 2
}

pub fn store_merge_dedup_test() {
  let frag = make_shard("shared")
  let a = store.put(store.new(), frag)
  let b = store.put(store.new(), frag)
  let merged = store.merge(a, b)
  assert store.size(merged) == 1
}

// ===========================================================================
// Walk
// ===========================================================================

pub fn walk_single_shard_test() {
  let s = make_shard("leaf")
  let result = walk.collect(s)
  assert result == [s]
}

pub fn walk_depth_first_test() {
  let leaf = make_shard("leaf")
  let parent = make_fragment("parent", [leaf])
  let collected = walk.collect(parent)
  assert list.length(collected) == 2
  let assert Ok(first) = list.first(collected)
  assert first == parent
}

pub fn walk_nested_three_levels_test() {
  let leaf = make_shard("leaf")
  let mid = make_fragment("mid", [leaf])
  let root = make_fragment("root", [mid])
  let collected = walk.collect(root)
  assert list.length(collected) == 3
}

pub fn walk_wide_tree_test() {
  let a = make_shard("a")
  let b = make_shard("b")
  let c = make_shard("c")
  let root = make_fragment("root", [a, b, c])
  let collected = walk.collect(root)
  assert list.length(collected) == 4
}

pub fn walk_fold_count_test() {
  let root = make_fragment("root", [make_shard("a"), make_shard("b")])
  let count = walk.fold(root, 0, fn(acc, _frag) { walk.Continue(acc + 1) })
  assert count == 3
}

pub fn walk_fold_stop_test() {
  let root = make_fragment("root", [make_shard("a"), make_shard("b")])
  let count = walk.fold(root, 0, fn(acc, _frag) { walk.Stop(acc + 1) })
  assert count == 1
}

pub fn walk_fold_collect_data_test() {
  let root = make_fragment("root", [make_shard("a"), make_shard("b")])
  let data =
    walk.fold(root, [], fn(acc, frag) {
      walk.Continue([fragmentation.data(frag), ..acc])
    })
  assert list.length(data) == 3
  assert list.contains(data, "a")
  assert list.contains(data, "b")
  assert list.contains(data, "root")
}

pub fn walk_depth_shard_test() {
  assert walk.depth(make_shard("x")) == 0
}

pub fn walk_depth_one_level_test() {
  let parent = make_fragment("parent", [make_shard("leaf")])
  assert walk.depth(parent) == 1
}

pub fn walk_depth_two_levels_test() {
  let leaf = make_shard("leaf")
  let mid = make_fragment("mid", [leaf])
  let root = make_fragment("root", [mid])
  assert walk.depth(root) == 2
}

pub fn walk_depth_asymmetric_test() {
  // One branch deeper than the other
  let deep = make_fragment("deep", [make_shard("leaf")])
  let shallow = make_shard("shallow")
  let root = make_fragment("root", [deep, shallow])
  assert walk.depth(root) == 2
}

pub fn walk_find_test() {
  let target = make_shard("needle")
  let other = make_shard("hay")
  let root = make_fragment("root", [other, target])
  let result = walk.find(root, fn(f) { fragmentation.data(f) == "needle" })
  assert result == Ok(target)
}

pub fn walk_find_not_found_test() {
  let s = make_shard("x")
  let result = walk.find(s, fn(f) { fragmentation.data(f) == "missing" })
  assert result == Error(Nil)
}

pub fn walk_find_nested_test() {
  let target = make_shard("deep-needle")
  let mid = make_fragment("mid", [target])
  let root = make_fragment("root", [make_shard("hay"), mid])
  let result =
    walk.find(root, fn(f) { fragmentation.data(f) == "deep-needle" })
  assert result == Ok(target)
}

// ===========================================================================
// Diff
// ===========================================================================

pub fn diff_identical_test() {
  let s = make_shard("same")
  let changes = diff.diff(s, s)
  assert changes == [diff.Unchanged(s)]
}

pub fn diff_different_roots_test() {
  let old = make_shard("old")
  let new = make_shard("new")
  let changes = diff.diff(old, new)
  let has_modified = list.any(changes, fn(c) {
    case c {
      diff.Modified(_, _) -> True
      _ -> False
    }
  })
  assert has_modified == True
}

pub fn diff_added_child_test() {
  let child = make_shard("child")
  let old = make_fragment("root", [])
  let new = make_fragment("root", [child])
  let changes = diff.diff(old, new)
  let has_added = list.any(changes, fn(c) {
    case c {
      diff.Added(_) -> True
      _ -> False
    }
  })
  assert has_added == True
}

pub fn diff_removed_child_test() {
  let child = make_shard("child")
  let old = make_fragment("root", [child])
  let new = make_fragment("root", [])
  let changes = diff.diff(old, new)
  let has_removed = list.any(changes, fn(c) {
    case c {
      diff.Removed(_) -> True
      _ -> False
    }
  })
  assert has_removed == True
}

pub fn diff_summary_test() {
  let changes = [
    diff.Added(make_shard("x")),
    diff.Removed(make_shard("y")),
    diff.Modified(make_shard("old"), make_shard("new")),
    diff.Unchanged(make_shard("z")),
    diff.Unchanged(make_shard("w")),
  ]
  assert diff.summary(changes) == #(1, 1, 1, 2)
}

pub fn diff_summary_empty_test() {
  assert diff.summary([]) == #(0, 0, 0, 0)
}

// ===========================================================================
// Witnessed reality (trace-as-branch)
// ===========================================================================

pub fn different_witness_different_hash_test() {
  // A fragment witnessed by different people produces different hashes
  let r = fragmentation.ref(fragmentation.hash("x"), "self")
  let w_alex = fragmentation.witnessed("alex", "alex", "2026-03-01", "observed")
  let w_reed = fragmentation.witnessed("reed", "reed", "2026-03-01", "traced")
  let s_alex = fragmentation.shard(r, w_alex, "same-data")
  let s_reed = fragmentation.shard(r, w_reed, "same-data")
  assert fragmentation.hash_fragment(s_alex)
    != fragmentation.hash_fragment(s_reed)
}

pub fn parallel_branch_pattern_test() {
  // Bias branch: a fragment containing a decision shard
  let decision = make_shard("decision:allow")
  let bias_root = make_fragment("bias", [decision])

  // Trace branch: a fragment that embeds the bias tree
  let trace = make_fragment("trace", [bias_root])

  // Walk from trace reaches everything
  let collected = walk.collect(trace)
  assert list.length(collected) == 3
}

pub fn trace_chain_test() {
  // Chain of traces, each carrying data and nesting the previous
  let bias = make_shard("bias:v1")
  let t1 = make_fragment("step:observe", [bias])
  let t2 = make_fragment("step:decide", [t1])
  let t3 = make_fragment("step:act", [t2])

  assert walk.depth(t3) == 3
  let collected = walk.collect(t3)
  assert list.length(collected) == 4
}

pub fn author_committer_split_test() {
  // The committer is who ran the bias. The author is who wrote it.
  let r = fragmentation.ref(fragmentation.hash("decision"), "self")
  let w = fragmentation.witnessed(
    "alex",     // author: wrote the bias
    "reed",     // committer: ran the bias
    "2026-03-01T19:30:00Z",
    "bias execution trace",
  )
  let traced = fragmentation.shard(r, w, "decision:allow")
  let witness = fragmentation.self_witnessed(traced)
  assert witness.author == "alex"
  assert witness.committer == "reed"
  assert witness.message == "bias execution trace"
}
