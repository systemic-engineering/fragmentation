# Witnessed

This document used to argue that the observer must be part of the content hash. That every fragment should carry its witness record, and that changing the observer should change the hash. We built that. Then we removed it.

Live and learn.

## What Changed

The first version of fragmentation put `Witnessed` on every fragment node. Author, committer, timestamp, message -- baked into `Shard` and `Fragment` alike. The hash included all of it. Same data, different observer, different hash. We called this "the observer effect made structural."

It was wrong. Not philosophically wrong -- the observer does matter. Structurally wrong. It conflated two distinct acts: constructing content and witnessing content.

Git already knew the difference. Blobs and trees are content-addressed by structure alone. Commits carry the witness metadata -- author, committer, timestamp, message. The content doesn't change when a different person commits it. The act of commitment is recorded separately, pointing at the content it witnessed.

We rebuilt fragmentation to match. `Witnessed` is no longer on `Fragment`. Content addressing (`content_oid`) covers structure only. Witnessing happens at commit time via `write_commit`.

## The Types

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

Four typed wrappers and a struct that composes them. The same four fields as a git commit. Each field is a distinct type -- you cannot pass a `Committer` where an `Author` is expected.

These types exist for `write_commit`. They map directly to git's `author` and `committer` signature fields.

## Content vs. Commitment

A fragment is structure. A commit is a witnessed act.

```rust
let tree = encoding::encode("same data");
let oid = fragment::content_oid(&tree);
// This OID is the same no matter who runs this code.
// "hello" is "hello" regardless of observer.

// Witnessing happens here:
let w = Witnessed::new(
    Author("alex".into()),
    Committer("reed".into()),
    Timestamp("2026-03-07T00:00:00Z".into()),
    Message("first observation".into()),
);
git::write_commit(&repo, &tree, &w, "first observation", None);
// Now the witness is recorded -- on the commit, not the content.
```

Same tree, different witness, different commit. But the same tree OID. The content is invariant. The commitment is the variable.

## Author and Committer

Git distinguishes author from committer. Most people never notice because they're usually the same person. But they encode different roles:

- **Author**: who wrote the content. Who made the decision. Who holds the intent.
- **Committer**: who ran the process. Who executed. Who was the mechanism.

In practice: Alex authors a design decision. Reed commits it -- runs the process, records the observation. The author is who wrote it. The committer is who ran it.

This distinction matters for systems where authorship and execution are separated. In agent-human collaboration, they almost always are.

## Why This Separation Is Right

The old design said: the observation changes the observed. Poetic. Also expensive, and it lied in practice. `read_tree` had to fill `Witnessed` with empty strings because git trees don't carry witness metadata. Every fragment reconstructed from git had a hollow witness record pretending to be something it wasn't.

The new design says: content is content. Witnessing is commitment. The act of committing a tree to a repository, with your name and timestamp, is the observation. The content you're observing doesn't change because you looked at it.

Two agents observe the same event. They produce the same content tree. They commit it separately -- different authors, different timestamps, different commits. But the tree they both point at is the same tree. Because it is. "We both saw this" and "this happened once" are now distinguishable: same tree OID, different commit OIDs.

The observer effect is real. It just lives on commits, not content.
