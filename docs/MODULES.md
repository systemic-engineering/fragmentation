# Modules

Fragmentation is four modules. They compose in one direction: core types flow outward, operations build on each other.

```
fragmentation          core types, construction, hashing, queries
  fragmentation/store  content-addressed storage (Sha -> Fragment)
  fragmentation/walk   recursive tree traversal
  fragmentation/diff   structural comparison between trees
```

## fragmentation (core)

Source: `src/fragmentation.gleam`

Everything starts here. Types, construction functions, serialization, hashing, and queries.

**Types**: `Sha`, `Ref`, `Witnessed`, `Fragment` (with variants `Shard` and `Fragment`).

**Construction**: `sha`, `ref`, `witnessed`, `shard`, `fragment`. Each constructs the corresponding type. No validation. No magic. What you give is what you get.

**Hashing**: `hash` takes a string, returns its SHA-256 as a `Sha`. `hash_fragment` serializes a fragment canonically and SHA-256 hashes the result. `serialize`, `serialize_witnessed`, and `serialize_ref` produce deterministic string representations.

**Queries**: `self_ref`, `self_witnessed`, `data`, `children`, `is_shard`, `is_fragment`. Accessors that work on both variants of Fragment.

The Erlang FFI (`src/fragmentation_ffi.erl`) provides the SHA-256 implementation via OTP's `crypto` module.

## fragmentation/store

Source: `src/fragmentation/store.gleam`

An in-memory content-addressed map. `Store` is opaque -- you interact through the module's functions.

```gleam
let s = store.new()
let s = store.put(s, my_fragment)    // keyed by self_ref SHA
let result = store.get(s, some_sha)  // Ok(fragment) or Error(Nil)
```

`put` extracts the fragment's self-ref SHA and uses it as the key. This means: same content, same key, put is idempotent. Inserting the same fragment twice doesn't duplicate it.

`merge` combines two stores. If both contain a fragment with the same SHA, it's the same content (by the content-addressing guarantee), so deduplication is free.

`keys` returns all SHAs in the store. `has` checks existence. `size` counts entries.

The store is a building block. It gives you deduplication and lookup by address. What you build on top of it -- persistence, networking, replication -- is up to you.

## fragmentation/walk

Source: `src/fragmentation/walk.gleam`

Recursive depth-first traversal. Since fragments embed their children directly (not by reference), walking requires no store lookups. You give it a root, it gives you the tree.

**`collect`**: returns all fragments in the tree as a flat list, depth-first order, root first.

**`fold`**: depth-first fold with early termination. The visitor function returns `Continue(acc)` to keep walking children or `Stop(acc)` to prune the branch.

```gleam
// Count all nodes
let count = walk.fold(root, 0, fn(acc, _frag) { walk.Continue(acc + 1) })

// Stop at first match
let found = walk.fold(root, False, fn(_acc, frag) {
  case fragmentation.data(frag) == "target" {
    True -> walk.Stop(True)
    False -> walk.Continue(False)
  }
})
```

**`depth`**: returns the maximum depth of the tree. A shard has depth 0. A fragment with only shard children has depth 1.

**`find`**: returns the first fragment matching a predicate, depth-first. Returns `Ok(fragment)` or `Error(Nil)`.

## fragmentation/diff

Source: `src/fragmentation/diff.gleam`

Structural comparison between two fragment trees. Produces a list of changes.

```gleam
pub type Change {
  Added(fragment: Fragment)
  Removed(fragment: Fragment)
  Modified(old: Fragment, new: Fragment)
  Unchanged(fragment: Fragment)
}
```

**`diff`** compares two trees starting at their roots. If the root hashes match, the entire tree is `Unchanged` (content addressing makes this a single comparison, not a recursive check). If they differ, it reports `Modified` for the root and then compares children positionally.

Positional comparison means: the first child of old is compared with the first child of new, and so on. If one tree has more children, the extras are `Added` or `Removed`.

**`summary`** reduces a list of changes to four counts: `#(added, removed, modified, unchanged)`.

## How They Compose

A typical workflow:

1. **Construct** fragments with the core module.
2. **Store** them for deduplication and lookup.
3. **Walk** trees to traverse, search, or aggregate.
4. **Diff** trees to understand what changed between two versions.

These modules don't depend on each other (except that all depend on core types). You can use walk without store. You can use diff without walk. They're independent operations on the same data structure.
