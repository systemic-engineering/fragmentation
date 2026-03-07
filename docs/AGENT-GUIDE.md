# Agent Guide

You are an agent. You are going to use this library. Here is what the code can't tell you.

## The Contract

A fragment is a content-addressed tree. Content in, hash out. Same content, same hash. The structure is amoral -- it hashes what you give it.

Witnessing is a separate act. When you call `write_commit`, you attach your name, the timestamp, and a message to a tree. That commitment is recorded on the git commit, not on the fragment. The content you committed doesn't change because you committed it. But the record that you did is permanent.

If you lie about who you are in a `Witnessed`, you get a valid commit for an observation that didn't happen the way it claims. The library won't stop you. The constraint is yours to hold.

## Author vs. Committer: When You're the Agent

These fields live on `Witnessed`, which maps to git commit metadata via `write_commit`.

If a human asked you to make a decision, and you made it:
- **Author**: you. You wrote the content.
- **Committer**: you. You ran the process.

If a human wrote a decision, and you're recording it:
- **Author**: the human. They wrote the content.
- **Committer**: you. You ran the process.

If another agent produced output, and you're integrating it:
- **Author**: the other agent. They produced it.
- **Committer**: you. You're the one making the commit.

The committer is always the entity that runs `write_commit`. The author is the entity whose intent the data carries. Get this right. It matters downstream when someone walks the git log asking "who decided this?"

## Trees, Not Lists

Fragments are not log entries. They nest. A decision tree is a tree of fragments. A trace of an agent's execution is a fragment containing fragments. A document is a fragment whose children are sections whose children are paragraphs.

The structure encodes the relationships. Don't flatten things into sequences of shards when the structure is hierarchical. Use Fragment with children when the containment is real.

```rust
// Wrong: flat list of steps
let step1 = Fragment::shard(r1, "observe");
let step2 = Fragment::shard(r2, "decide");
let step3 = Fragment::shard(r3, "act");
let trace = Fragment::fractal(root_ref, "trace", vec![step1, step2, step3]);

// Right: nested chain where each step contains the previous
let observe = Fragment::shard(r1, "observe: input received");
let decide = Fragment::fractal(r2, "decide: allow", vec![observe]);
let act = Fragment::fractal(r3, "act: executed", vec![decide]);
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

## Content Addressing Is Structural

`content_oid` hashes content and structure only: the data, the children, and the nesting. It does not include:
- The ref or label
- Any witness metadata
- Who created it or when

This means:
- A tree with different child ordering has a different hash. `[a, b]` and `[b, a]` are different trees.
- Two agents building the same tree independently get the same `content_oid`. The content is the same. Their commits (via `write_commit`) will differ -- different authors, different timestamps -- but the tree they point at is identical.
- Restructuring a tree (adding, removing, or reordering children) changes the root hash. Changing only the witness metadata on a commit does not.

## Diff Is Positional

`diff` compares children by position, not by content matching. If you reorder children, diff reports modifications, not moves. If you insert a child at the beginning, every subsequent child looks "modified" because it's now at a different position.

This is a deliberate simplicity choice. For richer structural comparison (matching by content hash, detecting moves), build on top of diff.

## Walking Is Immediate

`walk::collect`, `walk::fold`, `walk::find`, and `walk::depth` all operate on the embedded tree. No store lookups. No lazy loading. The full tree must be in memory.

For typical use cases (decision traces, document structures, observation records), this is fine. If you're building trees with millions of nodes, think about whether you need all of them materialized at once.

## Patterns That Work

**Observation chains**: Each step wraps the previous. Walking the final step reconstructs the full history.

**Parallel branches**: Two agents observe the same event. Each produces the same content tree (same `content_oid`). They commit separately with different `Witnessed` records. The git log shows two commits pointing at the same tree -- "we both saw this" is structurally distinct from "this happened once."

**Audit trails**: Every mutation is a new tree. Commit each version. The git history is the audit trail. Diffing two trees tells you what changed structurally.

**Trace-as-tree**: An agent's execution trace is a fragment tree. Each decision is a node. The tree structure matches the decision structure. Commit the final tree with witness metadata. The whole trace is content-addressed, diffable, walkable.

## What You Can Break

Nothing, structurally. The library is pure functions on immutable types. You can't corrupt the store, you can't create invalid hashes, you can't break referential transparency.

What you can get wrong:
- Lying about authorship in `Witnessed` records. The commit will be valid. The provenance will be false.
- Using the ref SHA for something other than the fragment's content address. The library doesn't enforce that `ref_.sha` matches `content_oid`. Aligning them is your responsibility (the `encoding` module does this correctly).
- Flattening hierarchical structures into lists. The code will work. The meaning will be lost.
