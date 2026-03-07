# Modules

Fragmentation has eight modules. They compose in one direction: core types flow outward, operations build on each other.

```
fragment       core type, construction, content addressing
ref_           self-address: SHA + label
sha            content hash wrapper
witnessed      git commit metadata (author, committer, timestamp, message)
store          content-addressed storage (Sha -> Fragment)
walk           recursive tree traversal
diff           structural comparison between trees
encoding       text as content-addressed trees
git            native git objects (behind "git" feature flag)
```

## fragment

Source: `src/fragment.rs`

The core type and content addressing.

**Type**: `Fragment` with variants `Shard` (terminal) and `Fractal` (recursive). Both carry a `Ref` (self-address) and `data` (string). `Fragment` additionally carries `fragments: Vec<Fragment>`.

**Construction**: `Fragment::shard(ref_, data)` and `Fragment::fractal(ref_, data, children)`. No witness parameter. No validation. What you give is what you get.

**Content addressing**: `content_oid` computes a git-compatible SHA-1 for any fragment. Shards produce blob OIDs, fragments produce tree OIDs. The computation is byte-identical to git's own object hashing.

```rust
// Blob OID -- matches `printf "hello" | git hash-object --stdin`
let oid = fragment::blob_oid("hello");

// Tree OID -- matches what git would produce for the same tree structure
let oid = fragment::tree_oid("data", &children);

// Dispatches to blob_oid or tree_oid based on variant
let oid = fragment::content_oid(&my_fragment);
```

**Queries**: `self_ref`, `data`, `children`, `is_shard`, `is_fractal`. Accessors that work on both variants.

## ref_

Source: `src/ref_.rs`

A self-address: a `Sha` paired with a label string. The `Sha` is typically the `content_oid` of the fragment. The label provides semantic context (`"utf8/a"`, `"token/hello"`, `"sentence"`, `"document"`).

## sha

Source: `src/sha.rs`

`Sha` wraps a string. `sha::hash` computes SHA-256. Used for general-purpose hashing. Content addressing uses SHA-1 via `fragment::content_oid` for git compatibility.

## witnessed

Source: `src/witnessed.rs`

Git commit metadata. `Author`, `Committer`, `Timestamp`, `Message` -- four typed wrappers. `Witnessed` composes them.

Used exclusively by `git::write_commit` to populate git author/committer fields. Not part of `Fragment`. Not part of the content hash.

## store

Source: `src/store.rs`

An in-memory content-addressed map. Keyed by self-ref SHA.

```rust
let mut s = Store::new();
s.put(my_fragment);               // keyed by self_ref SHA
let result = s.get(&some_sha);    // Option<&Fragment>
```

`put` is idempotent -- same content, same key. `merge` combines two stores; deduplication is free. `keys`, `has`, `size` for introspection.

The store is a building block for deduplication and lookup. Not a persistence layer.

## walk

Source: `src/walk.rs`

Recursive depth-first traversal. Fragments embed their children directly, so walking requires no store lookups.

**`collect`**: all fragments as a flat `Vec`, depth-first, root first.

**`fold`**: depth-first fold with early termination. The visitor returns `Continue(acc)` or `Stop(acc)`.

```rust
// Count all nodes
let count = walk::fold(&root, 0, &|acc, _frag| walk::Visitor::Continue(acc + 1));

// Find first match
let found = walk::find(&root, &|frag| frag.data() == "target");
```

**`depth`**: maximum depth. Shard = 0. Fragment with only shard children = 1.

**`find`**: first fragment matching a predicate. Returns `Option<&Fragment>`.

## diff

Source: `src/diff.rs`

Structural comparison between two fragment trees.

```rust
pub enum Change {
    Added(Fragment),
    Removed(Fragment),
    Modified { old: Fragment, new: Fragment },
    Unchanged(Fragment),
}
```

**`diff`** compares two trees from their roots. If `content_oid` matches, the entire subtree is `Unchanged` -- one comparison, not a recursive check. If they differ, it reports `Modified` for the root and compares children positionally.

Positional: first child of old vs. first child of new. Extras are `Added` or `Removed`.

**`summary`** reduces changes to four counts: `(added, removed, modified, unchanged)`.

## encoding

Source: `src/encoding.rs`

Text as content-addressed trees. Five levels: document, paragraph, sentence, word, character. Characters are terminal shards.

```rust
let root = encoding::encode("Hello world.");
// document Fragment
//   paragraph Fragment
//     sentence Fragment
//       word Fragment ("Hello")
//         char Shard ("H"), Shard ("e"), Shard ("l"), Shard ("l"), Shard ("o")
//       word Fragment ("world.")
//         ...
```

**`encode`**: text to document fragment. Splits on `\n\n` into paragraphs, sentences on `. ` / `! ` / `? `, words on spaces, words into character shards.

**`encode_paragraph`**, **`encode_sentence`**, **`encode_word`**, **`encode_char`**: enter the hierarchy at any level.

**`ingest`**: encodes text and collects every node into a `Store`. Shared subtrees deduplicate automatically -- same word in two sentences exists once in the store.

**`decode`**: extracts the data string. Lossless round-trip: `encode` then `decode` returns the original text.

Labels prevent cross-level collisions. Character `"a"` has label `utf8/a`, one-letter word `"a"` has label `token/a`. Different labels, different refs, even for identical data.

## git

Source: `src/git.rs` (behind `features = ["git"]`, requires `git2`)

Native git object creation. Fragments become real git objects.

**`write_tree`**: writes a fragment as git objects. Shard becomes a blob. Fragment becomes a tree with a `.data` blob entry and numbered child entries (`0000`, `0001`, ...). Returns the root OID. The OID is byte-identical to `content_oid`.

**`write_commit`**: writes a fragment tree and commits it. Takes a `Witnessed` for author/committer fields and a message. This is where witnessing happens -- the act of committing content to a repository with your name on it.

```rust
let tree = encoding::encode("hello world");
let w = Witnessed::new(
    Author("alex".into()),
    Committer("reed".into()),
    Timestamp("2026-03-07".into()),
    Message("observation".into()),
);
let commit_oid = git::write_commit(&repo, &tree, &w, "observation", None)?;
```

**`read_tree`**: reconstructs a `Fragment` from git objects. Blob becomes Shard, tree becomes Fragment. Round-trips cleanly with `write_tree`.

## How They Compose

1. **Construct** fragments with `fragment::shard` and `fragment::fractal`.
2. **Address** them with `fragment::content_oid`.
3. **Store** them for deduplication and lookup.
4. **Walk** trees to traverse, search, or aggregate.
5. **Diff** trees to see what changed.
6. **Encode** text into fragment trees.
7. **Commit** trees to git with witness metadata via `git::write_commit`.

These modules are independent operations on the same data structure. You can use walk without store, diff without walk, encoding without git. The only shared dependency is the core types.
