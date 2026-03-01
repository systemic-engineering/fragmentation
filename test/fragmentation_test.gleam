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
// Fragment construction
// ===========================================================================

pub fn shard_construction_test() {
  let r = fragmentation.ref(fragmentation.hash("data"), "self")
  let s = fragmentation.shard(r, "hello")
  assert s == fragmentation.Shard(r, "hello")
}

pub fn fractal_construction_test() {
  let leaf_ref = fragmentation.ref(fragmentation.hash("leaf"), "self")
  let leaf = fragmentation.shard(leaf_ref, "leaf-data")

  let root_ref = fragmentation.ref(fragmentation.hash("root"), "self")
  let f = fragmentation.fractal(root_ref, "root-data", [leaf])
  assert f == fragmentation.Fractal(root_ref, "root-data", [leaf])
}

pub fn fractal_empty_children_test() {
  let r = fragmentation.ref(fragmentation.hash("empty"), "self")
  let f = fragmentation.fractal(r, "no-children", [])
  assert fragmentation.children(f) == []
}

pub fn fractal_multiple_children_test() {
  let r1 = fragmentation.ref(fragmentation.hash("a"), "self")
  let r2 = fragmentation.ref(fragmentation.hash("b"), "self")
  let r3 = fragmentation.ref(fragmentation.hash("root"), "self")
  let a = fragmentation.shard(r1, "alpha")
  let b = fragmentation.shard(r2, "beta")
  let f = fragmentation.fractal(r3, "parent", [a, b])
  assert fragmentation.children(f) == [a, b]
}

// ===========================================================================
// Queries
// ===========================================================================

pub fn self_ref_shard_test() {
  let r = fragmentation.ref(fragmentation.hash("x"), "self")
  let s = fragmentation.shard(r, "data")
  assert fragmentation.self_ref(s) == r
}

pub fn self_ref_fractal_test() {
  let r = fragmentation.ref(fragmentation.hash("x"), "self")
  let f = fragmentation.fractal(r, "data", [])
  assert fragmentation.self_ref(f) == r
}

pub fn data_shard_test() {
  let r = fragmentation.ref(fragmentation.hash("x"), "self")
  let s = fragmentation.shard(r, "payload")
  assert fragmentation.data(s) == "payload"
}

pub fn data_fractal_test() {
  let r = fragmentation.ref(fragmentation.hash("x"), "self")
  let f = fragmentation.fractal(r, "payload", [])
  assert fragmentation.data(f) == "payload"
}

pub fn is_shard_test() {
  let r = fragmentation.ref(fragmentation.hash("x"), "self")
  assert fragmentation.is_shard(fragmentation.shard(r, "x")) == True
  assert fragmentation.is_shard(fragmentation.fractal(r, "x", [])) == False
}

pub fn is_fractal_test() {
  let r = fragmentation.ref(fragmentation.hash("x"), "self")
  assert fragmentation.is_fractal(fragmentation.fractal(r, "x", [])) == True
  assert fragmentation.is_fractal(fragmentation.shard(r, "x")) == False
}

pub fn children_shard_test() {
  let r = fragmentation.ref(fragmentation.hash("x"), "self")
  assert fragmentation.children(fragmentation.shard(r, "x")) == []
}

// ===========================================================================
// Content addressing
// ===========================================================================

pub fn hash_fragment_deterministic_test() {
  let r = fragmentation.ref(fragmentation.hash("x"), "self")
  let s = fragmentation.shard(r, "hello")
  let h1 = fragmentation.hash_fragment(s)
  let h2 = fragmentation.hash_fragment(s)
  assert h1 == h2
}

pub fn hash_fragment_different_data_test() {
  let r1 = fragmentation.ref(fragmentation.hash("a"), "self")
  let r2 = fragmentation.ref(fragmentation.hash("b"), "self")
  let s1 = fragmentation.shard(r1, "hello")
  let s2 = fragmentation.shard(r2, "world")
  assert fragmentation.hash_fragment(s1) != fragmentation.hash_fragment(s2)
}

pub fn serialize_roundtrip_hash_test() {
  let r = fragmentation.ref(fragmentation.hash("x"), "self")
  let f = fragmentation.shard(r, "test")
  let hash_direct = fragmentation.hash_fragment(f)
  let fragmentation.Sha(hash_via_serial) =
    fragmentation.hash(fragmentation.serialize(f))
  assert hash_direct == hash_via_serial
}

// ===========================================================================
// Store
// ===========================================================================

pub fn store_new_is_empty_test() {
  let s = store.new()
  assert store.size(s) == 0
}

pub fn store_put_and_get_test() {
  let r = fragmentation.ref(fragmentation.hash("hello"), "self")
  let frag = fragmentation.shard(r, "hello")
  let s = store.put(store.new(), frag)
  let fragmentation.Ref(sha, _) = r
  assert store.get(s, sha) == Ok(frag)
}

pub fn store_has_test() {
  let r = fragmentation.ref(fragmentation.hash("exists"), "self")
  let frag = fragmentation.shard(r, "exists")
  let s = store.put(store.new(), frag)
  let fragmentation.Ref(sha, _) = r
  assert store.has(s, sha) == True
  assert store.has(s, fragmentation.sha("nonexistent")) == False
}

pub fn store_size_test() {
  let s = store.new()
  assert store.size(s) == 0
  let r1 = fragmentation.ref(fragmentation.hash("a"), "self")
  let s = store.put(s, fragmentation.shard(r1, "a"))
  assert store.size(s) == 1
  let r2 = fragmentation.ref(fragmentation.hash("b"), "self")
  let s = store.put(s, fragmentation.shard(r2, "b"))
  assert store.size(s) == 2
}

pub fn store_put_idempotent_test() {
  let r = fragmentation.ref(fragmentation.hash("same"), "self")
  let frag = fragmentation.shard(r, "same")
  let s = store.put(store.new(), frag)
  let s = store.put(s, frag)
  assert store.size(s) == 1
}

pub fn store_get_missing_test() {
  let s = store.new()
  assert store.get(s, fragmentation.sha("nope")) == Error(Nil)
}

pub fn store_merge_test() {
  let r1 = fragmentation.ref(fragmentation.hash("alpha"), "self")
  let r2 = fragmentation.ref(fragmentation.hash("beta"), "self")
  let a = store.put(store.new(), fragmentation.shard(r1, "alpha"))
  let b = store.put(store.new(), fragmentation.shard(r2, "beta"))
  let merged = store.merge(a, b)
  assert store.size(merged) == 2
}

pub fn store_merge_dedup_test() {
  let r = fragmentation.ref(fragmentation.hash("shared"), "self")
  let frag = fragmentation.shard(r, "shared")
  let a = store.put(store.new(), frag)
  let b = store.put(store.new(), frag)
  let merged = store.merge(a, b)
  assert store.size(merged) == 1
}

// ===========================================================================
// Walk
// ===========================================================================

pub fn walk_single_shard_test() {
  let r = fragmentation.ref(fragmentation.hash("leaf"), "self")
  let s = fragmentation.shard(r, "leaf")
  let result = walk.collect(s)
  assert result == [s]
}

pub fn walk_fractal_depth_first_test() {
  let lr = fragmentation.ref(fragmentation.hash("leaf"), "self")
  let leaf = fragmentation.shard(lr, "leaf")
  let pr = fragmentation.ref(fragmentation.hash("parent"), "self")
  let parent = fragmentation.fractal(pr, "parent", [leaf])

  let collected = walk.collect(parent)
  assert list.length(collected) == 2
  // Parent first
  let assert Ok(first) = list.first(collected)
  assert first == parent
}

pub fn walk_nested_three_levels_test() {
  let r1 = fragmentation.ref(fragmentation.hash("l"), "self")
  let leaf = fragmentation.shard(r1, "leaf")
  let r2 = fragmentation.ref(fragmentation.hash("m"), "self")
  let mid = fragmentation.fractal(r2, "mid", [leaf])
  let r3 = fragmentation.ref(fragmentation.hash("r"), "self")
  let root = fragmentation.fractal(r3, "root", [mid])

  let collected = walk.collect(root)
  assert list.length(collected) == 3
}

pub fn walk_fold_count_test() {
  let r1 = fragmentation.ref(fragmentation.hash("a"), "self")
  let r2 = fragmentation.ref(fragmentation.hash("b"), "self")
  let r3 = fragmentation.ref(fragmentation.hash("root"), "self")
  let a = fragmentation.shard(r1, "aaa")
  let b = fragmentation.shard(r2, "bbb")
  let root = fragmentation.fractal(r3, "root", [a, b])

  let count = walk.fold(root, 0, fn(acc, _frag) {
    walk.Continue(acc + 1)
  })
  assert count == 3
}

pub fn walk_fold_stop_test() {
  let r1 = fragmentation.ref(fragmentation.hash("a"), "self")
  let r2 = fragmentation.ref(fragmentation.hash("b"), "self")
  let r3 = fragmentation.ref(fragmentation.hash("root"), "self")
  let a = fragmentation.shard(r1, "aaa")
  let b = fragmentation.shard(r2, "bbb")
  let root = fragmentation.fractal(r3, "root", [a, b])

  // Stop after first fragment
  let count = walk.fold(root, 0, fn(acc, _frag) {
    walk.Stop(acc + 1)
  })
  assert count == 1
}

pub fn walk_depth_shard_test() {
  let r = fragmentation.ref(fragmentation.hash("x"), "self")
  assert walk.depth(fragmentation.shard(r, "x")) == 0
}

pub fn walk_depth_one_level_test() {
  let lr = fragmentation.ref(fragmentation.hash("leaf"), "self")
  let leaf = fragmentation.shard(lr, "leaf")
  let pr = fragmentation.ref(fragmentation.hash("parent"), "self")
  let parent = fragmentation.fractal(pr, "parent", [leaf])
  assert walk.depth(parent) == 1
}

pub fn walk_depth_two_levels_test() {
  let r1 = fragmentation.ref(fragmentation.hash("l"), "self")
  let leaf = fragmentation.shard(r1, "leaf")
  let r2 = fragmentation.ref(fragmentation.hash("m"), "self")
  let mid = fragmentation.fractal(r2, "mid", [leaf])
  let r3 = fragmentation.ref(fragmentation.hash("r"), "self")
  let root = fragmentation.fractal(r3, "root", [mid])
  assert walk.depth(root) == 2
}

pub fn walk_find_test() {
  let r1 = fragmentation.ref(fragmentation.hash("target"), "self")
  let target = fragmentation.shard(r1, "needle")
  let r2 = fragmentation.ref(fragmentation.hash("other"), "self")
  let other = fragmentation.shard(r2, "hay")
  let r3 = fragmentation.ref(fragmentation.hash("root"), "self")
  let root = fragmentation.fractal(r3, "root", [other, target])

  let result = walk.find(root, fn(f) { fragmentation.data(f) == "needle" })
  assert result == Ok(target)
}

pub fn walk_find_not_found_test() {
  let r = fragmentation.ref(fragmentation.hash("x"), "self")
  let s = fragmentation.shard(r, "x")
  let result = walk.find(s, fn(f) { fragmentation.data(f) == "missing" })
  assert result == Error(Nil)
}

// ===========================================================================
// Diff
// ===========================================================================

pub fn diff_identical_test() {
  let r = fragmentation.ref(fragmentation.hash("x"), "self")
  let s = fragmentation.shard(r, "same")
  let changes = diff.diff(s, s)
  assert changes == [diff.Unchanged(s)]
}

pub fn diff_different_roots_test() {
  let r1 = fragmentation.ref(fragmentation.hash("old"), "self")
  let r2 = fragmentation.ref(fragmentation.hash("new"), "self")
  let old = fragmentation.shard(r1, "old")
  let new = fragmentation.shard(r2, "new")
  let changes = diff.diff(old, new)
  let has_added = list.any(changes, fn(c) {
    case c {
      diff.Added(_) -> True
      _ -> False
    }
  })
  assert has_added == True
}

pub fn diff_summary_test() {
  let r1 = fragmentation.ref(fragmentation.hash("a"), "self")
  let r2 = fragmentation.ref(fragmentation.hash("b"), "self")
  let changes = [
    diff.Added(fragmentation.Shard(r1, "x")),
    diff.Removed(fragmentation.Shard(r2, "y")),
    diff.Modified(
      fragmentation.Shard(r1, "old"),
      fragmentation.Shard(r2, "new"),
    ),
    diff.Unchanged(fragmentation.Shard(r1, "z")),
    diff.Unchanged(fragmentation.Shard(r2, "w")),
  ]
  assert diff.summary(changes) == #(1, 1, 1, 2)
}

pub fn diff_summary_empty_test() {
  assert diff.summary([]) == #(0, 0, 0, 0)
}

// ===========================================================================
// Parallel branch pattern (trace-as-branch)
// ===========================================================================

pub fn parallel_branch_pattern_test() {
  // Bias branch: a fractal containing a decision shard
  let decision_ref = fragmentation.ref(fragmentation.hash("decision:allow"), "self")
  let decision = fragmentation.shard(decision_ref, "decision:allow")
  let bias_ref = fragmentation.ref(fragmentation.hash("bias"), "self")
  let bias_root = fragmentation.fractal(bias_ref, "bias", [decision])

  // Trace branch: a fractal that embeds the bias tree (or references it)
  let trace_ref = fragmentation.ref(fragmentation.hash("trace"), "self")
  let trace = fragmentation.fractal(trace_ref, "trace", [bias_root])

  // Walk from trace reaches everything
  let collected = walk.collect(trace)
  assert list.length(collected) == 3
  // trace -> bias_root -> decision
}

pub fn trace_chain_test() {
  // Chain of traces, each carrying data and nesting the previous
  let bias_ref = fragmentation.ref(fragmentation.hash("bias:v1"), "self")
  let bias = fragmentation.shard(bias_ref, "bias:v1")

  let t1_ref = fragmentation.ref(fragmentation.hash("t1"), "self")
  let t1 = fragmentation.fractal(t1_ref, "step:observe", [bias])

  let t2_ref = fragmentation.ref(fragmentation.hash("t2"), "self")
  let t2 = fragmentation.fractal(t2_ref, "step:decide", [t1])

  let t3_ref = fragmentation.ref(fragmentation.hash("t3"), "self")
  let t3 = fragmentation.fractal(t3_ref, "step:act", [t2])

  assert walk.depth(t3) == 3
  let collected = walk.collect(t3)
  assert list.length(collected) == 4
}
