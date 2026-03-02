# Modules

Fragmentation is six modules. They compose in one direction: core types flow outward, operations build on each other.

```
fragmentation              core types, construction, hashing, queries
  fragmentation/store      content-addressed storage (Sha -> Fragment)
  fragmentation/walk       recursive tree traversal
  fragmentation/diff       structural comparison between trees
  fragmentation/encoding   text as content-addressed trees
  fragmentation/git        content-addressed fragment persistence
```

## fragmentation (core)

Source: `src/fragmentation.gleam`

Everything starts here. Types, construction functions, serialization, hashing, and queries.

**Types**: `Sha`, `Ref`, `Author`, `Committer`, `Timestamp`, `Message`, `Witnessed`, `Fragment` (with variants `Shard` and `Fragment`).

**Construction**: `sha`, `ref`, `author`, `committer`, `timestamp`, `message`, `witnessed`, `shard`, `fragment`. Each constructs the corresponding type. No validation. No magic. What you give is what you get.

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

## fragmentation/encoding

Source: `src/fragmentation/encoding.gleam`

Text as content-addressed trees. Five levels of structure: document, paragraph, sentence, word, character. Each level is a fragment containing the next level down. Characters are terminal shards.

```gleam
let root = encoding.encode("Hello world.", witness)
// document Fragment
//   paragraph Fragment
//     sentence Fragment
//       word Fragment ("Hello")
//         char Shard ("H")
//         char Shard ("e")
//         char Shard ("l")
//         char Shard ("l")
//         char Shard ("o")
//       word Fragment ("world.")
//         ...
```

**`encode`**: takes text and a witness, returns a document fragment. Splits on double newlines into paragraphs, paragraphs into sentences (on `. `, `! `, `? ` boundaries), sentences into words (on spaces), words into characters (grapheme clusters).

**`encode_paragraph`**, **`encode_sentence`**, **`encode_word`**, **`encode_char`**: individual constructors for each level. You can enter the hierarchy wherever you want.

**`ingest`**: encodes text and collects every node into a `Store`, returning the root fragment and the populated store. Shared subtrees deduplicate automatically -- if two paragraphs contain the same word, that word's character shards exist once in the store.

**`decode`**: extracts the data string from a fragment. Lossless round-trip: `encode` then `decode` returns the original text.

**`DecodeError`**: error type for decode failures. Variant `UnknownLabel(String)`.

Labels prevent cross-level collisions. A character "a" and a one-letter word "a" have different SHAs because their labels differ (`utf8/a` vs `token/a`). The label is hashed alongside the data via `labeled_hash`, which prefixes the data with its level before hashing.

## fragmentation/git

Source: `src/fragmentation/git.gleam`

Content-addressed fragment persistence. Writes a fragment to disk named by its SHA.

**`write`**: takes a fragment and a directory path. Computes the SHA via `hash_fragment`, serializes via `serialize`, writes to `<dir>/<sha>`. Returns `Ok(Nil)` on success, `Error(simplifile.FileError)` on failure.

Idempotent. Writing the same fragment twice produces the same file at the same path with the same content. The file name is the content address. The file content is the canonical serialization.

The store is a directory. Each fragment becomes a file. This is the simplest possible persistence layer for content-addressed data -- the same principle as git's object store, without the pack files.

## How They Compose

A typical workflow:

1. **Construct** fragments with the core module.
2. **Store** them for deduplication and lookup.
3. **Walk** trees to traverse, search, or aggregate.
4. **Diff** trees to understand what changed between two versions.
5. **Encode** text into fragment trees for content-addressed document storage.
6. **Persist** fragments to disk with git for durable, content-addressed storage.

These modules don't depend on each other (except that all depend on core types). You can use walk without store. You can use diff without walk. Encoding uses walk and store internally but doesn't require you to. They're independent operations on the same data structure.
