# What Fragmentation Is

Fragmentation is a content-addressed tree library. That sentence is true but insufficient.

Git stores blobs, trees, commits, and tags. Four object types. Every object is addressed by the SHA of its content. Same content, same address. This is what makes git work -- not branches, not the staging area, not the porcelain. The content-addressing.

Fragmentation takes that principle and makes it the only principle. Two types of node: `Shard` (terminal) and `Fragment` (contains other fragments). Trees all the way down, arbitrary depth, every node carrying its own address and its own witness record.

## The Types

```gleam
pub type Sha { Sha(self: String) }
pub type Ref { Ref(sha: Sha, label: String) }
pub type Witnessed {
  Witnessed(author: String, committer: String, timestamp: String, message: String)
}
pub type Fragment {
  Shard(ref: Ref, witnessed: Witnessed, data: String)
  Fragment(ref: Ref, witnessed: Witnessed, data: String, fragments: List(Fragment))
}
```

Four types. That's it. Everything else is operations on these.

`Sha` is a content-addressed hash. `Ref` is an address with a label -- a named pointer. `Witnessed` is who was here when this happened. `Fragment` is a node that is either terminal (Shard) or recursive (Fragment).

## What This Is Not

This is not a database. There is no persistence layer built in. The `Store` module gives you an in-memory content-addressed map, which is useful for working with trees, but it's not the point.

This is not a version control system. It's the data structure that version control systems are built from. Git is one realization. Fragmentation is the abstract structure.

This is not a document format. But documents are trees. Decisions are trees. Identity is a tree. Anything that has parts which have parts which have parts can be a fragment tree.

## The Self-Similar Property

A `Fragment` contains a list of `Fragment`. Not a list of IDs. Not references to be resolved later. The children are embedded directly. This means:

- Walking a tree requires no lookups. The tree carries itself.
- Serializing a tree serializes everything. The canonical form is complete.
- Hashing a tree hashes everything. The hash covers the entire structure, including all children, recursively.

This is intentional. A fragment tree is a self-contained unit of witnessed reality. If you have the root, you have everything.

## Content Addressing

Same content, same hash. This is enforced by the `hash_fragment` function, which serializes a fragment into a canonical string form and SHA-256 hashes it. The serialization is deterministic: same fragment always produces the same string, which always produces the same hash.

The canonical serialization includes:
- The type tag (`shard` or `fragment`)
- The ref (address and label)
- The witness record (author, committer, timestamp, message)
- The data
- For fragments: the serialized children, recursively

All of these contribute to the hash. Change any of them, the hash changes. This is the foundation everything else rests on.
