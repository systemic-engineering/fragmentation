# Witnessed

`Witnessed` is git commit metadata. Four typed fields that map directly to git's author and committer signatures.

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

Each field is a distinct type -- you cannot pass a `Committer` where an `Author` is expected.

## Content vs. Commitment

Content addressing (`content_oid`) covers structure only: data and children. It does not include who created the fragment, when, or what they said about it. Same content, same hash, always.

Witnessing happens at commit time via `write_commit`. The witness record goes on the git commit, not the content.

```rust
let tree = encoding::encode("same data");
let oid = fragment::content_oid(&tree);
// This OID is the same no matter who runs this code.

let w = Witnessed::new(
    Author("alex".into()),
    Committer("reed".into()),
    Timestamp("2026-03-07T00:00:00Z".into()),
    Message("first observation".into()),
);
git::write_commit(&repo, &tree, &w, "first observation", None);
// The witness is recorded on the commit, not the content.
```

Same tree, different witness, different commit. But the same tree OID. The content is invariant. The commitment is the variable.

## Author and Committer

Git distinguishes author from committer. Most people never notice because they're usually the same person. But they encode different roles:

- **Author**: who wrote the content. Who made the decision. Who holds the intent.
- **Committer**: who ran the process. Who executed. Who was the mechanism.

This distinction matters for systems where authorship and execution are separated. In agent-human collaboration, they almost always are.

## Two Observers, One Tree

Two agents observe the same event. They produce the same content tree -- same `content_oid`. They commit it separately: different authors, different timestamps, different commits. The tree they both point at is the same tree. Because it is.

"We both saw this" and "this happened once" are structurally distinct: same tree OID, different commit OIDs. The observer effect lives on commits, not content.
