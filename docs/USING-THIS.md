# Using This

## License

This library is released under the **systemic.engineering License v1.0**. Three conditions:

1. **Anti-extraction.** Labor has value. Using work without attribution, compensation, or consent is extraction.
2. **Consent-based collaboration.** Offers are not commands. Silence is a legitimate response. Systems must not punish refusal.
3. **Intersectional justice.** Systems must not reinforce structural oppression.

Full text: [`LICENSE.md`](../LICENSE.md) in the repository root.

Using this library means you accept those conditions as binding.

---

## Adding the Dependency

```toml
[dependencies]
fragmentation = { git = "https://github.com/systemic-engineering/fragmentation" }
```

### Feature Flags

| Flag  | What it enables |
|-------|-----------------|
| `git` | Native git objects via `git2`. `write_tree`, `write_commit`, `read_tree`, `Local::from_repo`. |
| `ssh` | Ed25519 signing + ECIES encryption (X25519 + ChaCha20-Poly1305). |
| `gpg` | GPG subprocess signing + encryption. |

```toml
fragmentation = { git = "...", features = ["git", "ssh"] }
```

All features are off by default. The core library has no optional dependencies.

---

## Core Types

### `Sha(String)`

Content-addressed hash. Wraps a hex string. The `sha` module provides `sha::hash(data)` for raw SHA-256.

### `Ref { sha, label }`

Named pointer. Every fragment carries one as its self-address.

```rust
use fragmentation::ref_::Ref;
use fragmentation::sha::Sha;

let r = Ref::new(Sha("abc123...".into()), "my-label");
```

### `Fragment<E>`

The tree node. Two variants:

```rust
enum Fragment<E = Blob> {
    Shard { ref_: Ref, data: E },               // terminal
    Fractal { ref_: Ref, data: E, fragments: Vec<Fragment<E>> }, // recursive
}
```

`Blob` is `Vec<u8>`. That is the default. `String` is the other common choice.

**Constructors for `Fragment<String>`:**

```rust
Fragment::shard(ref_, "leaf data")
Fragment::fractal(ref_, "branch data", children)
```

**Constructors for any `E`:**

```rust
Fragment::shard_typed(ref_, my_data)
Fragment::fractal_typed(ref_, my_data, children)
```

**Accessors:** `self_ref()`, `data()`, `children()`, `is_shard()`, `is_fractal()`.

---

## Content Addressing

Content OIDs are git-compatible SHA-1 hashes. They exclude ref, label, and witness metadata. Same content, same hash. Always.

```rust
use fragmentation::fragment;

let oid = fragment::content_oid(&frag);     // Shard -> blob OID, Fractal -> tree OID
let oid = fragment::blob_oid("some text");  // blob OID for a string
let oid = fragment::tree_oid("data", &children); // tree OID for data + children
```

Child ordering matters. Reorder children, get a different hash.

---

## Modules

### `fragment` -- core type + content addressing

`Fragment<E>`, `Blob`, `content_oid`, `blob_oid`, `tree_oid`. Everything above.

### `ref_` -- Sha + label

`Ref::new(sha, label)`.

### `sha` -- SHA-256 general hashing

`sha::hash("data")` returns `Sha`. Not used for content OIDs (those are SHA-1, git-compatible). This is for general-purpose content addressing.

### `store` -- in-memory content-addressed map

```rust
use fragmentation::store::Store;

let mut store = Store::new();
store.put(frag);
store.get(&sha);     // Option<&Fragment<E>>
store.has(&sha);     // bool
store.size();        // usize
store.keys();        // Vec<Sha>
store.merge(other);  // absorb another store
```

Not persistence. A `HashMap<String, Fragment<E>>` keyed by self-ref SHA.

### `walk` -- depth-first traversal

```rust
use fragmentation::walk;

let all: Vec<&Fragment<_>> = walk::collect(&root);
let d: usize = walk::depth(&root);
let found = walk::find(&root, &|f| f.data() == "target");

let count = walk::fold(&root, 0, &|acc, _frag| {
    walk::Visitor::Continue(acc + 1)
});
```

`fold` takes a `Visitor<A>`: return `Continue(acc)` to keep walking children, `Stop(acc)` to prune.

### `diff` -- structural comparison

```rust
use fragmentation::diff;

let changes: Vec<diff::Change<E>> = diff::diff(&old, &new);
let (added, removed, modified, unchanged) = diff::summary(&changes);
```

Four variants: `Added`, `Removed`, `Modified { old, new }`, `Unchanged`. Comparison is positional -- child at index 0 compared against child at index 0.

### `encoding` -- text to fragment trees

Five structural levels: document, paragraph, sentence, word, character.

```rust
use fragmentation::encoding;

let doc = encoding::encode("Hello world.\n\nSecond paragraph.");
let text = encoding::decode(&doc).unwrap();

// Encode + store (deduplicates):
let (root, store) = encoding::ingest("text", Store::new());
```

Individual levels: `encode_char`, `encode_word`, `encode_sentence`, `encode_paragraph`.

Traits `Encode` and `Decode` for custom types:

```rust
impl Encode for MyType {
    fn encode(&self) -> Vec<u8> { /* ... */ }
}

impl Decode for MyType {
    type Error = MyError;
    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> { /* ... */ }
}
```

### `witnessed` -- git commit metadata

```rust
use fragmentation::witnessed::*;

let w = Witnessed::new(
    Author("Mara".into()),
    Committer("Mara".into()),
    Timestamp("2026-03-08T12:00:00Z".into()),
    Message("initial".into()),
);
```

Who was here when this happened. Author wrote the content. Committer ran the process. Witness metadata is excluded from content OIDs -- the observation is separate from the observed.

### `git` -- native git objects (feature: `git`)

```rust
use fragmentation::git;

let tree_oid = git::write_tree(&repo, &fragment)?;
let commit_oid = git::write_commit(&repo, &fragment, &witnessed, "message", parent.as_ref())?;
let fragment = git::read_tree(&repo, oid)?;
```

Shards become blobs. Fractals become trees with `.data` + numbered child entries (`0000`, `0001`, ...).

### `keys` -- signing + encryption

The `Keys` trait: `sign`, `encrypt`, `decrypt`.

```rust
use fragmentation::keys::*;

// No-op (testing, unencrypted contexts):
let plain = PlainKeys;
let signed = plain.sign(fragment)?;
let encrypted = plain.encrypt(fragment)?;
let decrypted: Fragment<String> = plain.decrypt(&encrypted)?;

// Local -- what the machine has:
let local = Local::None;                           // no signing
let local = Local::Ssh(Box::new(SSH::from_path("~/.ssh/id_ed25519")?));  // feature: ssh
let local = Local::Gpg(GPG::new("KEYID"));         // feature: gpg
let local = Local::from_repo(&repo)?;              // feature: git -- reads git config
```

`Signed<K, T>` wraps a value with signature bytes and signer identity. `Encrypted<K>` wraps opaque ciphertext with a key reference.

SSH encryption uses ECIES: ephemeral X25519 key agreement, HKDF-SHA256 key derivation, ChaCha20-Poly1305 AEAD. GPG encryption shells out to the `gpg` subprocess.

### `actor` -- identity + encoding + keys

```rust
use fragmentation::actor::Actor;

// Default: Blob -> Blob identity transform, Local::None keys
let a = Actor::identity("Mara", "mara@systemic.engineer");

// Custom encoder/decoder + keys:
let a = Actor::new("Mara", "mara@systemic.engineer", my_encoder, my_decoder, my_keys);

a.encode(&fragment);    // A -> B
a.decode(&fragment);    // B -> A
a.sign(fragment)?;      // sign encoded fragment
a.encrypt(fragment)?;   // encrypt encoded fragment
a.decrypt(&encrypted)?; // decrypt to encoded fragment
```

An actor does the encoding, not tags it. The encoder/decoder are `fn` pointers, not closures. Simple, cloneable, deterministic.

---

## Examples

### Build a tree and walk it

```rust
use fragmentation::fragment::Fragment;
use fragmentation::ref_::Ref;
use fragmentation::sha;
use fragmentation::walk;

let leaf_a = Fragment::shard(
    Ref::new(sha::hash("a"), "leaf-a"),
    "alpha",
);
let leaf_b = Fragment::shard(
    Ref::new(sha::hash("b"), "leaf-b"),
    "beta",
);
let root = Fragment::fractal(
    Ref::new(sha::hash("root"), "root"),
    "top",
    vec![leaf_a, leaf_b],
);

assert_eq!(walk::depth(&root), 1);
assert_eq!(walk::collect(&root).len(), 3);
```

### Diff two trees

```rust
use fragmentation::diff;
use fragmentation::fragment::Fragment;
use fragmentation::ref_::Ref;
use fragmentation::sha;

let old = Fragment::fractal(
    Ref::new(sha::hash("v1"), "doc"),
    "version one",
    vec![Fragment::shard(Ref::new(sha::hash("a"), "p"), "unchanged")],
);
let new = Fragment::fractal(
    Ref::new(sha::hash("v2"), "doc"),
    "version two",
    vec![
        Fragment::shard(Ref::new(sha::hash("a"), "p"), "unchanged"),
        Fragment::shard(Ref::new(sha::hash("b"), "p"), "added"),
    ],
);

let changes = diff::diff(&old, &new);
let (added, removed, modified, unchanged) = diff::summary(&changes);
assert_eq!(added, 1);
assert_eq!(unchanged, 1);
assert_eq!(modified, 1); // root itself changed
```

### Encode text and ingest

```rust
use fragmentation::encoding;
use fragmentation::store::Store;

let (root, store) = encoding::ingest("Hello world.", Store::new());
assert!(root.is_fractal());
assert!(store.size() > 0);

// Round-trip:
let text = encoding::decode(&root).unwrap();
assert_eq!(text, "Hello world.");
```

---

## Invariants

- `content_oid` excludes ref, label, and witness metadata. Same content = same hash.
- Child ordering matters. `[a, b]` and `[b, a]` produce different tree OIDs.
- Diff is positional. Children are compared by index, not by content matching.
- `Sha` (SHA-256) and content OIDs (SHA-1, git-compatible) serve different purposes. Don't mix them.
