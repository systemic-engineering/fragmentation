# What Is This (Agent Reference)

`fragmentation` -- content-addressed, arbitrary-depth, circular-reflexive trees. Rust crate.

You are reading this because you need to understand the data model. This document is the type-level truth. No narrative. No motivation. Types, contracts, structure.

---

## Core Type

```rust
pub type Blob = Vec<u8>;

pub enum Fragment<E = Blob> {
    Shard { ref_: Ref, data: E },
    Fractal { ref_: Ref, data: E, fragments: Vec<Fragment<E>> },
}
```

Two variants. `Shard` is terminal. `Fractal` is recursive -- it contains `Vec<Fragment<E>>`. The type parameter `E` is the data type. Default is `Blob` (`Vec<u8>`). String constructors exist. Typed constructors (`shard_typed`, `fractal_typed`) work with any `E`.

Children are embedded. Not IDs, not lazy references. The actual values, inline. Walking a tree requires zero lookups.

The tradeoff is memory. Shared subtrees are duplicated in the tree. Use `Store` when deduplication matters.

## Self-Address

```rust
pub struct Sha(pub String);

pub struct Ref {
    pub sha: Sha,
    pub label: String,
}
```

Every fragment carries a `Ref`. The `sha` is typically the `content_oid` of the fragment. The `label` is semantic context -- `"utf8/a"`, `"token/hello"`, `"sentence"`, `"document"`, `"self"`. Labels prevent cross-level hash collisions: the character `"a"` (label `utf8/a`) and the one-letter word `"a"` (label `token/a`) have different refs.

The library does not enforce that `ref_.sha` equals `content_oid`. Keeping them aligned is your responsibility.

## Content Addressing

```rust
pub fn content_oid<E: Encode>(frag: &Fragment<E>) -> String;
pub fn blob_oid(data: &str) -> String;
pub fn blob_oid_bytes(data: &[u8]) -> String;
pub fn tree_oid<E: Encode>(data: &str, children: &[Fragment<E>]) -> String;
pub fn tree_oid_bytes<E: Encode>(data: &[u8], children: &[Fragment<E>]) -> String;
```

`content_oid` computes a git-compatible SHA-1. Shards produce blob OIDs. Fractals produce tree OIDs. The output is byte-identical to what git produces for the same structure.

### What the hash covers

- The data (encoded to bytes via `Encode`)
- All children, recursively
- The structure: shard vs fractal, child count, child ordering

### What the hash does NOT cover

- The `Ref` (sha + label)
- Witness metadata (author, committer, timestamp, message)
- Who created it or when

**Same content, same hash. Always.** Two agents building the same tree independently get the same `content_oid`. No exceptions.

## Encode / Decode

```rust
pub trait Encode {
    fn encode(&self) -> Vec<u8>;
}

pub trait Decode: Sized {
    type Error: Display + Debug;
    fn decode(bytes: &[u8]) -> Result<Self, Self::Error>;
}
```

Implement these for custom data types. Built-in implementations exist for `Vec<u8>` (identity) and `String` (UTF-8). `content_oid` requires `E: Encode`. Encryption requires `E: Encode`. Decryption requires `E: Decode`.

## Witnessing

```rust
pub struct Author(pub String);
pub struct Committer(pub String);
pub struct Timestamp(pub String);
pub struct Message(pub String);

pub struct Witnessed {
    pub author: Author,
    pub committer: Committer,
    pub timestamp: Timestamp,
    pub message: Message,
}
```

Witness metadata lives on git commits, not on fragments. `Witnessed` is consumed by `git::write_commit`. The fragment tree the commit points to has no knowledge of who committed it.

Content is deterministic. Commitment is the act of recording content with an identity attached. Same tree, different witness, different commit. Same tree OID. Different commit OID.

The observer changes the record without changing the content. That is the circular-reflexive property.

## Keys

```rust
pub trait Keys: Sized + Clone {
    type Error: Display + Debug;
    fn sign<E>(&self, fragment: Fragment<E>) -> Result<Signed<Self, Fragment<E>>, Self::Error>;
    fn encrypt<E: Encode>(&self, fragment: Fragment<E>) -> Result<Encrypted<Self>, Self::Error>;
    fn decrypt<E: Decode>(&self, encrypted: &Encrypted<Self>) -> Result<Fragment<E>, Self::Error>;
}

pub struct Signed<K, T> {
    inner: T,
    signature: Vec<u8>,
    signer: K,
}

pub struct Encrypted<K> {
    ciphertext: Vec<u8>,
    key: K,
}
```

Three operations: sign, encrypt, decrypt. `Self` threads through as real data -- the signer in `Signed`, the recipient in `Encrypted`. You always know who signed or encrypted.

Self-encryption by design. An actor encrypts to their own key. The ciphertext is opaque to everyone else.

### Implementations

```rust
pub struct PlainKeys;     // No-op. Empty signatures, plaintext pass-through.
                          // Error = Infallible.

pub enum Local {
    None,                 // No signing.
    Ssh(Box<SSH>),        // Ed25519 signing + ECIES encryption. Feature: "ssh".
    Gpg(GPG),             // gpg CLI subprocess. Feature: "gpg".
}
```

**PlainKeys**: testing, unencrypted contexts. Operations cannot fail.

**SSH** (feature `ssh`): Ed25519 signing. ECIES encryption (X25519 key exchange, HKDF-SHA256 derivation, ChaCha20-Poly1305 AEAD). Pure Rust. No subprocess. Wire format: `[32 ephemeral_pub | 12 nonce | ciphertext + 16 tag]` -- 60 bytes overhead.

**GPG** (feature `gpg`): `gpg` command-line subprocess. Detached signatures, standard encryption/decryption.

**Local** (feature `git`): `from_repo(&git2::Repository)` detects signing configuration from git config (`gpg.format` + `user.signingkey`).

Content addressing operates on plaintext, not ciphertext. The SHA is computed before encryption.

## Actor

```rust
pub struct Actor<A = Blob, B = Blob, K: Keys = Local> {
    name: String,
    email: String,
    encoder: fn(&Fragment<A>) -> Fragment<B>,
    decoder: fn(&Fragment<B>) -> Fragment<A>,
    keys: K,
}
```

Identity + encoding boundary + keys. The actor doesn't tag the encoding -- the actor *does* the encoding. Different actor, different transformation, different view of the same structure.

Function pointers, not closures. Simple, cloneable, deterministic.

```rust
// Default: Blob -> Blob identity, Local::None keys
Actor::identity("name", "email")

// Custom: typed encoding + real keys
Actor::new("name", "email", my_encoder, my_decoder, my_keys)
```

`encode`, `decode`, `sign`, `encrypt`, `decrypt` methods delegate to the encoder/decoder/keys.

## Store

```rust
pub struct Store<E = Blob> {
    fragments: HashMap<String, Fragment<E>>,
}
```

In-memory content-addressed map. Keyed by self-ref SHA. `put` is idempotent -- same content, same key. `merge` combines two stores. Not a persistence layer.

Operations: `new`, `put`, `get`, `has`, `size`, `merge`, `keys`.

## Walking

```rust
pub fn collect<E>(root: &Fragment<E>) -> Vec<&Fragment<E>>;
pub fn fold<A, E>(root: &Fragment<E>, acc: A, f: &dyn Fn(A, &Fragment<E>) -> Visitor<A>) -> A;
pub fn find<'a, E>(root: &'a Fragment<E>, predicate: &dyn Fn(&Fragment<E>) -> bool) -> Option<&'a Fragment<E>>;
pub fn depth<E>(root: &Fragment<E>) -> usize;

pub enum Visitor<A> {
    Continue(A),
    Stop(A),
}
```

Depth-first. No store lookups. The full tree must be in memory. `fold` supports early termination via `Visitor::Stop`.

## Diff

```rust
pub enum Change<E = Blob> {
    Added(Fragment<E>),
    Removed(Fragment<E>),
    Modified { old: Fragment<E>, new: Fragment<E> },
    Unchanged(Fragment<E>),
}

pub fn diff<E: Encode + Clone>(old: &Fragment<E>, new: &Fragment<E>) -> Vec<Change<E>>;
pub fn summary<E>(changes: &[Change<E>]) -> (usize, usize, usize, usize);
```

Structural comparison. If `content_oid` matches, the entire subtree is `Unchanged` -- one comparison, not recursive. Children compared positionally. Reorder = modifications, not moves. `summary` returns `(added, removed, modified, unchanged)`.

## Git (feature `git`)

```rust
pub fn write_tree<E: Encode>(repo: &Repository, fragment: &Fragment<E>) -> Result<Oid, git2::Error>;
pub fn write_commit<E: Encode>(repo: &Repository, fragment: &Fragment<E>, witnessed: &Witnessed, message: &str, parent: Option<&Commit>) -> Result<Oid, git2::Error>;
pub fn read_tree(repo: &Repository, oid: Oid) -> Result<Fragment<String>, Box<dyn Error>>;
```

Native git object creation. `write_tree` maps shards to blobs, fractals to trees with `.data` blob entry + numbered child entries (`0000`, `0001`, ...). The OID from `write_tree` is byte-identical to `content_oid`. `write_commit` is where witnessing happens. `read_tree` reconstructs fragments from git objects.

## Text Encoding

Five-level decomposition: document > paragraph > sentence > word > character (shard).

```rust
pub fn encode(text: &str) -> Fragment<String>;         // Full document
pub fn encode_paragraph(text: &str) -> Fragment<String>;
pub fn encode_sentence(text: &str) -> Fragment<String>;
pub fn encode_word(word: &str) -> Fragment<String>;
pub fn encode_char(ch: &str) -> Fragment<String>;
pub fn ingest(text: &str, store: Store<String>) -> (Fragment<String>, Store<String>);
pub fn decode(fragment: &Fragment<String>) -> Result<String, DecodeError>;
```

Splits: `\n\n` for paragraphs, `. `/`! `/`? ` for sentences, spaces for words, characters for shards. `encode` then `decode` is lossless. `ingest` populates a store with all nodes -- shared subtrees deduplicate.

This encoding system is one use of `Fragment<String>`. The library does not require it. Build fragment trees from anything.

## Feature Flags

| Feature | What it enables | Dependencies |
|---------|----------------|--------------|
| `git` | `write_tree`, `write_commit`, `read_tree`, `Local::from_repo` | `git2` |
| `ssh` | `SSH` key type, ECIES encryption | `ssh-key`, `x25519-dalek`, `chacha20poly1305`, `hkdf` |
| `gpg` | `GPG` key type | (none -- subprocess) |

Core functionality (fragment, store, walk, diff, encoding, content addressing, PlainKeys, Actor) works without any feature flags.
