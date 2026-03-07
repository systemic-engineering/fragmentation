# What Fragmentation Is

Fragmentation is a content-addressed tree library. That sentence is true but insufficient.

Git stores blobs, trees, commits, and tags. Four object types. Every object is addressed by the SHA of its content. Same content, same address. This is what makes git work -- not branches, not the staging area, not the porcelain. The content-addressing.

Fragmentation takes that principle and makes it the only principle. Two types of node: `Shard` (terminal) and `Fragment` (contains other fragments). Trees all the way down, arbitrary depth, every node carrying its own content address.

## The Types

```rust
pub struct Sha(pub String);
pub struct Ref { pub sha: Sha, pub label: String }

pub enum Fragment {
    Shard { ref_: Ref, data: String },
    Fragment { ref_: Ref, data: String, fragments: Vec<Fragment> },
}
```

Three types. `Sha` is a content-addressed hash. `Ref` is an address with a label -- a named pointer. `Fragment` is a node that is either terminal (Shard) or recursive (Fragment).

Witness metadata -- who observed, when, what they said -- lives on git commits. Not on fragment nodes. Content is content. Witnessing is a different act. See [Witnessed](WITNESSED.md).

## What This Is Not

This is not a database. There is no persistence layer built in. The `Store` module gives you an in-memory content-addressed map, which is useful for working with trees, but it's not the point.

This is not a version control system. It's the data structure that version control systems are built from. Git is one realization. Fragmentation is the abstract structure.

This is not a document format. But documents are trees. Decisions are trees. Identity is a tree. Anything that has parts which have parts which have parts can be a fragment tree.

## The Self-Similar Property

A `Fragment` contains a `Vec<Fragment>`. Not a list of IDs. Not references to be resolved later. The children are embedded directly. This means:

- Walking a tree requires no lookups. The tree carries itself.
- Hashing a tree hashes everything. The hash covers the entire structure, including all children, recursively.
- Same content, same hash. Always.

This is intentional. A fragment tree is a self-contained unit of structure. If you have the root, you have everything.

## Content Addressing

Same content, same hash. Enforced by `content_oid`, which computes git-compatible SHA-1 hashes. A shard produces a blob OID. A fragment produces a tree OID. The computation is byte-identical to what git itself would produce for the same structure.

```rust
// Shard -> blob OID: SHA-1("blob {len}\0{data}")
let oid = fragment::blob_oid("hello");
// "b6fc4c620b67d95f953a5c1c1230aaab5db5a1b0"
// Identical to: printf "hello" | git hash-object --stdin

// Fragment -> tree OID: SHA-1 of the binary tree object
let oid = fragment::content_oid(&my_fragment);
```

The content OID includes:
- The data (as a `.data` blob entry)
- All children, recursively (as numbered tree/blob entries)
- The structure (shard vs. fragment, child ordering)

The content OID does not include:
- The ref (the address the fragment claims)
- The label
- Who observed it, when, or what they said about it

Two fragments with the same data and the same children have the same `content_oid`, regardless of who created them. This is the foundation. Witnessing happens at commit time, not at construction time.
