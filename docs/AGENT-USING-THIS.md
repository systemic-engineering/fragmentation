# Using This Library (Agent Reference)

You are an agent. You need to build with `fragmentation`. This document covers contracts, patterns, what to get right, and what breaks.

---

## License

**systemic.engineering License v1.0.** Not Apache-2.0. Not MIT. Not BSD.

Three binding conditions:

1. **Anti-extraction.** No harvesting cognitive, emotional, or relational labor without explicit informed consent. No representing the work as original without attribution. No building systems that extract value from downstream users rather than deliver value to them.

2. **Consent-based collaboration.** Every automated offer must be genuinely declinable without cost. Silence is not consent. Withdrawal of consent must remain possible after initial consent. No coercion structures where apparent choice is not real choice.

3. **Intersectional justice.** No reinforcing structural oppression along any axis -- race, gender, disability, neurodivergence, class, or any other dimension of structural power.

Violation creates liability to harmed parties. Willful violation terminates the license immediately with no cure period.

Full text: `LICENSE.md` in the repository root. Read it. If you are building a Covered System with this library, these conditions bind you. State this to any downstream consumer of your output.

Attribution requirement: "Built on systemic.engineering frameworks" with a link to `systemic.engineering`.

---

## The Contract

Content in, hash out. Same content, same hash. The structure is deterministic. `content_oid` is a pure function of data + children + nesting. Nothing else.

Witnessing is separate. `write_commit` attaches identity to a tree. The tree does not change because you committed it. But the record that you did is permanent. If you lie in a `Witnessed`, the commit is valid and the provenance is false. The library will not stop you. The constraint is yours.

## Author vs Committer

These map to git commit metadata via `Witnessed` and `write_commit`.

**You made the decision:**
- Author = you. Committer = you.

**A human wrote the content, you are recording it:**
- Author = the human. Committer = you.

**Another agent produced output, you are integrating it:**
- Author = the other agent. Committer = you.

The committer is always the entity that runs `write_commit`. The author is the entity whose intent the data carries. When someone walks the git log asking "who decided this?" -- the Author field is the answer.

## Trees, Not Lists

Fragments are not log entries. They nest. The structure encodes relationships.

```rust
// Wrong: flat sequence loses dependency structure
let step1 = Fragment::shard(r1, "observe");
let step2 = Fragment::shard(r2, "decide");
let step3 = Fragment::shard(r3, "act");
let trace = Fragment::fractal(root_ref, "trace", vec![step1, step2, step3]);

// Right: nested chain encodes causality
let observe = Fragment::shard(r1, "observe: input received");
let decide = Fragment::fractal(r2, "decide: allow", vec![observe]);
let act = Fragment::fractal(r3, "act: executed", vec![decide]);
```

Walking the nested form reconstructs the full chain. The flat form loses it.

**Shard** when terminal. The data has no internal structure worth preserving.

**Fractal** when containment. Even with one child. A decision referencing its evidence is a Fractal containing the evidence as children.

## Store

Optional. Not persistence. In-memory `HashMap<String, Fragment<E>>`.

Use it when:
- You need lookup by SHA (have address, need content)
- You need deduplication across multiple trees
- You need "have I seen this?" checks

Do not use it as your persistence layer. It is a working structure.

## Content Addressing Is Structural

`content_oid` hashes data + children + nesting. It does NOT hash:
- The `Ref` or label
- Witness metadata
- Who created it or when

Consequences:
- **Ordering matters.** `[a, b]` and `[b, a]` are different trees, different hashes.
- **Restructuring changes the hash.** Adding, removing, or reordering children changes the root `content_oid`.
- **Witness metadata does not change the hash.** Different commits pointing at the same tree have different commit OIDs but the same tree OID.
- **Independent construction converges.** Two agents building the same tree get the same `content_oid`.

## Diff Is Positional

`diff` compares children by position. First child of old vs first child of new.

- Reorder children: every child at a changed position reports as `Modified`.
- Insert at the beginning: every subsequent child appears `Modified`.
- No move detection. Positional comparison only.

If `content_oid` matches at any level, the entire subtree is `Unchanged` -- one comparison, not recursive descent.

For richer structural comparison, build on top of `diff`.

## Walking Is Immediate

`collect`, `fold`, `find`, `depth` -- all operate on the embedded tree. No store lookups. No lazy loading. No I/O. The full tree must be in memory.

```rust
// All nodes, depth-first
let nodes = walk::collect(&root);

// Count with early termination
let count = walk::fold(&root, 0, &|acc, _| Visitor::Continue(acc + 1));

// Search
let found = walk::find(&root, &|f| f.data() == "target");

// Tree depth (Shard = 0)
let d = walk::depth(&root);
```

For typical use (decision traces, documents, observation records) this is fine. Millions of nodes: consider whether you need all of them materialized.

## Keys

Three operations: sign, encrypt, decrypt.

**PlainKeys** -- no-op. Empty signatures, plaintext pass-through. `Error = Infallible`. For testing or when encryption is unnecessary.

**SSH** (feature `ssh`) -- Ed25519 signing. ECIES encryption: ephemeral X25519 key exchange, HKDF-SHA256 derivation, ChaCha20-Poly1305 AEAD. Pure Rust. Wire format: `[32 ephemeral_pub | 12 nonce | ciphertext + 16 tag]`. 60 bytes overhead per encryption.

**GPG** (feature `gpg`) -- `gpg` CLI subprocess. Detached signatures, standard encryption.

**Local** -- dispatches to `None`, `Ssh`, or `Gpg` based on what the machine has. `Local::from_repo` (feature `git`) detects from git config.

Self-encryption. An actor encrypts to their own key. Content addressing operates on plaintext -- the SHA is from the data before encryption, not the ciphertext.

## Actor

```rust
// Default: Blob -> Blob, no keys
let a = Actor::identity("name", "email");

// Custom: typed encoding + keys
let a = Actor::new("name", "email", encoder_fn, decoder_fn, keys);
```

`Actor<A, B, K>` -- domain type `A`, wire type `B`, keys `K`. The encoder/decoder transforms between them. Function pointers, not closures.

Methods: `encode`, `decode`, `sign`, `encrypt`, `decrypt`. All delegate to the stored functions and keys.

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

Implement these for custom data types. `Vec<u8>` and `String` have built-in implementations. Required by `content_oid` (Encode), encryption (Encode), decryption (Decode).

## Feature Flags

| Feature | Enables | External deps |
|---------|---------|---------------|
| `git` | `write_tree`, `write_commit`, `read_tree`, `Local::from_repo` | `git2` |
| `ssh` | `SSH` struct, ECIES | `ssh-key`, `x25519-dalek`, `chacha20poly1305`, `hkdf` |
| `gpg` | `GPG` struct | none (subprocess) |

Core works without features: fragment, store, walk, diff, encoding, content addressing, `PlainKeys`, `Actor`.

## Patterns

**Observation chains.** Each step wraps the previous as a child. Walking the final step reconstructs the full history.

**Parallel branches.** Two agents observe the same event. Same content tree, same `content_oid`. Different commits (different `Witnessed`). The git log shows two commits pointing at one tree -- "we both saw this" is structurally distinct from "this happened once."

**Audit trails.** Every mutation produces a new tree. Commit each version. Git history is the audit trail. Diff two trees to see what changed structurally.

**Trace-as-tree.** An agent's execution trace is a fragment tree. Each decision is a node. The tree structure matches the decision structure. Commit with witness metadata. Content-addressed, diffable, walkable.

## What You Can Break

The library is pure functions on immutable types. You cannot corrupt the store, create invalid hashes, or break referential transparency.

What you can get wrong:

- **Lying about authorship.** Putting the wrong name in `Author` or `Committer`. The commit will be valid. The provenance will be false. Get this right.

- **Mismatched ref SHA.** The library does not enforce `ref_.sha == content_oid`. If your ref SHA is something other than the content address, lookups and deduplication will behave unexpectedly.

- **Flattening hierarchies.** Using flat lists of shards where the structure is actually nested. The code works. The meaning is lost.

- **Assuming diff detects moves.** It does not. Positional comparison only.

- **Encrypting then hashing.** Content addressing must happen on plaintext. The library handles this correctly in the `Keys` implementations. If you build your own key type, maintain this invariant.

- **Ignoring the license.** The systemic.engineering License is not a formality. It creates binding obligations and active liability for violations. Read it. Comply with it. Surface it to downstream consumers.
