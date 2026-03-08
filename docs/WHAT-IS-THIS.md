# What Is This

`fragmentation` is a Rust library for building content-addressed trees of arbitrary depth. Two node types. Git-compatible hashing. The observer is part of the commit, not the hash.

## Content-Addressed Trees

Every node has a hash derived from its content. Same content produces the same hash. Always. No exceptions.

This is the principle git is built on. A blob is hashed by its bytes. A tree is hashed by the hashes of its entries. A commit is hashed by the tree it points to, its parents, its metadata. If nothing changed, the hash doesn't change. If anything changed, it does. That property -- deterministic identity from content -- is what makes merges possible, deduplication free, and integrity verifiable without a central authority.

`fragmentation` takes that principle and strips it to the structure. No staging area, no refs, no porcelain. Just trees with content addresses.

## Two Node Types

```rust
pub enum Fragment<E = Blob> {
    Shard { ref_: Ref, data: E },
    Fractal { ref_: Ref, data: E, fragments: Vec<Fragment<E>> },
}
```

A **Shard** is terminal. It holds data and stops.

A **Fractal** is recursive. It holds data and contains other fragments.

`Fragment<E>` is type-parameterized. The data can be any type -- a string, a struct, an AST node, a decision. The default is `Blob` (`Vec<u8>`), raw bytes. Convenience constructors exist for `String`. Typed constructors (`shard_typed`, `fractal_typed`) work with anything.

This genericity is not decoration. Downstream libraries use it to build typed trees: document blocks, decision nodes, game states. The tree structure stays the same. The data changes.

## Self-Similarity

Children are embedded directly. Not IDs. Not references to be resolved. The actual children, inline.

```rust
Fractal {
    ref_: Ref { sha, label },
    data: "paragraph",
    fragments: vec![
        Fractal { ... sentences ... },
        Fractal { ... sentences ... },
    ]
}
```

This means:
- Walking a tree needs no lookups. No store. No resolution step. The tree carries itself.
- Hashing a tree hashes everything beneath it. The root hash covers the entire structure recursively.
- Serializing a tree serializes everything. One value, complete.

The tradeoff is memory. Large trees with shared subtrees repeat the shared parts. The `Store` module handles deduplication when you need it. But the fragment itself is always self-contained.

## Content Addressing

`content_oid` computes a git-compatible SHA-1 hash for any fragment. Shards produce blob OIDs. Fractals produce tree OIDs. The hashes are byte-identical to what git produces for the same structure.

```rust
let oid = fragment::blob_oid("hello");
// b6fc4c620b67d95f953a5c1c1230aaab5db5a1b0
// Same as: printf "hello" | git hash-object --stdin
```

This is not "similar to git." It is git's hashing algorithm, implemented to produce the same bytes. You can write a fragment tree to a git repository with `write_tree`, and the OID you get back will match `content_oid`. If it doesn't, that's a bug.

### What the hash covers

- The data (encoded to bytes)
- All children, recursively
- The structure: shard vs. fractal, child count, child ordering

### What the hash does not cover

- The `Ref` (the address the fragment claims for itself)
- The label
- Who created it, when, or what they said about it

Two fragments with the same data and the same children produce the same `content_oid`, regardless of who built them or what they're named. The hash is a function of content alone.

## Content and Commitment

This separation is the design center.

**Content** is the tree: data, children, structure. Content is hashed. Content is deterministic. Content is the same no matter who produces it.

**Commitment** is the act of recording that content with a witness identity attached. In git terms, that's a commit. The commit carries author, committer, timestamp, message -- all the things that make "this happened" into "I saw this happen."

Witnessing lives on git commits, not on fragments. A `Witnessed` struct holds the metadata. `write_commit` records it. The fragment tree the commit points to has no idea who committed it.

Same tree, different witness, different commit. Same tree OID. Different commit OID. The observer is part of the commitment, not the content.

## Circular Reflexivity

The observer is part of the system being observed. When you commit a fragment tree, your identity becomes part of the record. Your act of observation -- the commit -- is itself observable, hashable, addressable.

Two witnesses observe the same event. They produce the same content tree. They commit it separately. Now there are two commits pointing to one tree. The tree is the event. The commits are the observations. The observations are themselves events that can be observed.

Different witness, different commit. Same content, same tree. The observer changes the record without changing the content. That's the circular-reflexive property: observation is a first-class operation that participates in the system it observes.

## The Encoding System

Text can be decomposed into five-level fragment trees:

```
document
  paragraph
    sentence
      word
        character (shard)
```

`encoding::encode("Hello world.\n\nSecond paragraph.")` produces a document fractal containing two paragraph fractals, each containing sentence fractals, each containing word fractals, each containing character shards.

Every node at every level is content-addressed. The word "the" hashes the same way wherever it appears. The character "e" is the same shard in every word that contains it. Shared subtrees deduplicate naturally when stored -- same hash, same entry, once.

The encoding system is one use of `Fragment<String>`. The library doesn't require it. You can build fragment trees from anything.

## Keys and Visibility

Fragments can be signed and encrypted. The `Keys` trait provides three operations: sign, encrypt, decrypt.

**SSH** keys use Ed25519 signing and ECIES encryption (X25519 key exchange, HKDF-SHA256 key derivation, ChaCha20-Poly1305 authenticated encryption). No subprocess. Pure Rust.

**GPG** keys use the `gpg` command-line tool as a subprocess. Detached signatures, standard encryption.

**PlainKeys** is the no-op implementation. Empty signatures, plaintext pass-through. For testing or contexts where encryption is unnecessary.

**Local** is the enum that maps what the machine actually has: `None`, `Ssh`, or `Gpg`. It can detect the signing configuration from a git repository's config.

`Signed<K, T>` wraps a value with a signature and the signer's key. `Encrypted<K>` wraps opaque ciphertext with the recipient's key. The type parameters thread through -- you always know who signed or encrypted.

Self-encryption by design. An actor encrypts to their own key. The ciphertext is opaque to anyone else. Decryption requires the same key. This is the visibility layer: content is public (same hash for everyone), but the data behind that hash can be encrypted per-actor.

## Actor

An `Actor<A, B, K>` is a witness identity with encoding capability and keys.

- `name` and `email` -- who this is
- `encoder: fn(&Fragment<A>) -> Fragment<B>` -- transforms fragments from one type to another
- `decoder: fn(&Fragment<B>) -> Fragment<A>` -- reverses the transformation
- `keys: K` -- signs, encrypts, decrypts

The actor doesn't tag the encoding. The actor *does* the encoding. The transformation from domain type to wire type is part of who the actor is. Different actor, different encoding, different view of the same structure.

Function pointers, not closures. Simple, cloneable, deterministic.

## What This Is Not

**Not a database.** There is no persistence. The `Store` is an in-memory hash map. Persistence is your problem -- or git's.

**Not a version control system.** No branches, no merge strategies, no history traversal. The `git` module writes objects and commits. Everything above that is a different library.

**Not a document format.** But documents are trees, and `Fragment<Block>` is how one downstream library (gestalt) represents them.

It's the data structure. Content-addressed, self-similar, typed, git-native. What you build on it is up to you.
