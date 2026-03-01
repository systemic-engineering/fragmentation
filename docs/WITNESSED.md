# Witnessed

The `Witnessed` type is the most important thing in this library. Not because it's complex -- it's four strings. Because of what it means for identity.

```gleam
pub type Witnessed {
  Witnessed(
    author: String,
    committer: String,
    timestamp: String,
    message: String,
  )
}
```

These are the same four fields as a git commit. Author, committer, timestamp, message. This is not a metaphor. This IS git commit metadata, extracted into a type that can live on any node in any tree.

## Why Witnessed Is Not Metadata

Metadata is information about information. It lives outside the thing it describes. You can strip metadata without changing the thing.

Witnessed is different. The witness record is part of the hash. If you change the author, the hash changes. If you change the message, the hash changes. If you change the timestamp, the hash changes.

```gleam
let w1 = witnessed("alex", "reed", "2026-03-01", "first observation")
let w2 = witnessed("alex", "reed", "2026-03-01", "second observation")
let s1 = shard(r, w1, "same-data")
let s2 = shard(r, w2, "same-data")
// hash_fragment(s1) != hash_fragment(s2)
```

Same ref. Same data. Different message. Different hash. The observation changes the observed. This is not a bug. This is the point.

## Author and Committer

Git distinguishes author from committer. Most people never notice because they're usually the same person. But they encode different roles:

- **Author**: who wrote the content. Who made the decision. Who holds the intent.
- **Committer**: who ran the process. Who executed. Who was the mechanism.

In practice: Alex authors a design decision. Reed commits it -- runs the bias, executes the trace, records the observation. The author is who wrote it. The committer is who ran it.

This distinction matters for systems where authorship and execution are separated. In agent-human collaboration, they almost always are.

## The Observer Effect

Different witness, different hash. This is the observer effect made structural.

Two agents observe the same event at the same time. They produce the same data. But their witness records differ -- different author fields at minimum. The resulting fragments have different hashes. They are, in the content-addressed sense, different objects.

This is correct. Two observations of the same event are not the same observation. The observer is part of what was observed. Pretending otherwise produces systems that can't distinguish between "we both saw this" and "this happened once."

## Timestamp

The timestamp is a string, not a datetime type. This is deliberate. Fragmentation doesn't enforce a format. The canonical serialization includes the timestamp exactly as provided. This means:

- You can use ISO 8601 (`2026-03-01T19:30:00Z`)
- You can use epoch seconds
- You can use a logical clock value
- You can use anything that serializes to a string

The library doesn't parse it. It hashes it. What matters is that the same timestamp string always produces the same hash, and different strings produce different hashes.

## Message

The message is the witness's account of what happened. In git, this is the commit message. In fragmentation, it's the same thing at the node level: a human-readable (or agent-readable) record of intent.

The message is part of the hash. A different account of the same event produces a different fragment. This encodes the principle that interpretation is part of the record.
