# Agent Guide

You are an agent. You are going to use this library. Here is what the code can't tell you.

## The Contract

A fragment is a witnessed observation. Not data with metadata attached. A unit of reality that includes who observed it, when, and what they said about it.

When you create a fragment, you are witnessing. Your author and committer fields are part of the hash. If you lie about who you are, you get a valid hash for a thing that didn't happen. The library won't stop you. Content addressing is amoral. It just hashes what you give it. The constraint is yours to hold.

## Author vs. Committer: When You're the Agent

If a human asked you to make a decision, and you made it:
- **Author**: you. You wrote the content.
- **Committer**: you. You ran the process.

If a human wrote a decision, and you're recording it:
- **Author**: the human. They wrote the content.
- **Committer**: you. You ran the process.

If another agent produced output, and you're integrating it:
- **Author**: the other agent. They produced it.
- **Committer**: you. You're the one assembling the tree.

The committer is always the entity that creates the fragment object. The author is the entity whose intent the data carries. Get this right. It matters downstream when someone walks the tree asking "who decided this?"

## Trees, Not Lists

Fragments are not log entries. They nest. A decision tree is a tree of fragments. A trace of an agent's execution is a fragment containing fragments. A document is a fragment whose children are sections whose children are paragraphs.

The structure encodes the relationships. Don't flatten things into sequences of shards when the structure is hierarchical. Use Fragment with children when the containment is real.

```gleam
// Wrong: flat list of steps
let step1 = shard(r1, w, "observe")
let step2 = shard(r2, w, "decide")
let step3 = shard(r3, w, "act")
let trace = fragment(root_ref, w, "trace", [step1, step2, step3])

// Right: nested chain where each step contains the previous
let observe = shard(r1, w, "observe: input received")
let decide = fragment(r2, w, "decide: allow", [observe])
let act = fragment(r3, w, "act: executed", [decide])
```

The nested form encodes causality. Walking `act` shows you the full chain. The flat form loses the dependency structure.

## When to Shard vs. Fragment

`Shard` when it's terminal. The data speaks for itself and has no internal structure worth preserving.

`Fragment` when it contains other things. Even if it only contains one child. The containment is semantic. A decision that references its evidence is a Fragment containing the evidence as children. A leaf observation with no sub-parts is a Shard.

## The Store Is Optional

You might not need it. If you're building a tree, walking it, and passing it somewhere else, the store adds nothing. The tree carries itself.

Use the store when:
- You need to look up fragments by SHA (you have an address, you need the content)
- You need deduplication across multiple trees (same subtree appears in different roots)
- You're building a working set and need to check "have I seen this before?"

Don't use the store as your persistence layer. It's in-memory. It's a working structure, not a database.

## Hashing Is Total

`hash_fragment` hashes everything: the type tag, the ref, the witness record, the data, and all children recursively. This means:

- Two trees that look different but have the same serialization will have the same hash. (This shouldn't happen if you use the construction functions correctly.)
- A tree with a different child ordering has a different hash. `[a, b]` and `[b, a]` are different trees.
- A tree with a different witness on any node, at any depth, has a different root hash. The witness propagates upward through serialization.

The last point is critical. If you re-witness a leaf (same data, new timestamp), the root hash changes. The entire tree is different because one observation within it changed. This is correct. The tree you had before and the tree you have now are not the same tree.

## Diff Is Positional

`diff` compares children by position, not by content matching. If you reorder children, diff reports modifications, not moves. If you insert a child at the beginning, every subsequent child looks "modified" because it's now at a different position.

This is a deliberate simplicity choice. For richer structural comparison (matching by label, matching by content hash, detecting moves), you'd build on top of diff, not replace it.

## Walking Is Immediate

`walk.collect`, `walk.fold`, `walk.find`, and `walk.depth` all operate on the embedded tree. No store lookups. No lazy loading. The full tree must be in memory.

For very large trees, this means memory proportional to tree size. For typical use cases (decision traces, document structures, observation records), this is fine. If you're building trees with millions of nodes, think about whether you need all of them materialized at once.

## Patterns That Work

**Observation chains**: Each step wraps the previous. Walking the final step reconstructs the full history.

**Parallel branches**: Two agents observe the same event. Each produces a shard with different witness records. A supervisory fragment wraps both, creating a tree where the branches are the observations and the root is the integration.

**Audit trails**: Every mutation is a new fragment. The old fragment is a child of the new one. The tree grows from the root. Walking it gives you the full trail. Diffing two versions tells you what changed.

**Trace-as-branch**: An agent's execution trace is a fragment tree. Each decision is a node. The tree structure matches the decision structure. The witness records say who decided what, when. The whole trace is hashable, diffable, walkable.

## What You Can Break

Nothing, structurally. The library is a set of pure functions on immutable types. You can't corrupt the store, you can't create invalid hashes, you can't break referential transparency.

What you can get wrong:
- Lying about authorship in witness records. The hash will be valid. The provenance will be false.
- Using the ref SHA for something other than the fragment's address. The library doesn't enforce that the ref SHA matches the content hash. These are separate concepts: the ref is the address the fragment *claims*, the content hash is what the content *produces*. Aligning them is your responsibility.
- Flattening hierarchical structures into lists. The code will work. The meaning will be lost.
- Ignoring the timestamp. Using empty strings or static values for timestamps produces valid fragments that carry no temporal information. Content addressing doesn't care. Your future self, trying to reconstruct when things happened, will.
