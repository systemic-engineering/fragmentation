# fragmentation-mcp — the agent-runtime substrate that natively extends git

*2026-06-01. Mara. Spec — design, not implementation. No Rust changes; one
markdown deliverable plus targeted upserts in sibling docs. Architectural
anchor: fragmentation becomes the FIRST deployment target for the stack.
Standalone. Useful without mirror. Open-source. The substrate any agent
workflow benefits from, with native git interop at the wire and the
Hamilton-Scheduler managing the agent's working memory.*

Status: **Red** — the architecture is pinned, the MCP tool surface is
specified, the HamiltonScheduler integration is wired, the BLAKE3 ↔ SHA-1
crosswalk is defined, the dependency direction is locked, the existing
git-MCP landscape is surveyed honestly, the open questions are named.
None of it runs yet. The implementation tick lands afterward.

Depends on:
- `fragmentation/src/frgmnt_store.rs` — the `FrgmntStore<N: Fragmentable +
  Clone>` with `.frgmnt/` disk spillover. Each MCP session-shard is one
  `FrgmntStore` instance with a Hamilton-governed budget.
- `fragmentation/src/bounded_store.rs` — `BoundedStore<N>` byte-LIFO
  eviction; the kinetic-side cache the scheduler governs.
- `fragmentation/docs/specs/hamilton-scheduler.md` (commits `c2079ed`,
  `e227f1e`) — the realtime-grade scheduler this spec consumes. The
  agent shard is the FIRST production consumer of `Scheduler` outside
  the build-graph altitude; §4 below shows how each scheduler concept
  (the 16-feature `GraphObservation`, the four `Strategy` variants, the
  hard/soft `RealtimeClass`) maps to agent workflows.
- `fragmentation/docs/specs/lens-transit.md` (commit `12b7853`) — the
  measurement carrier. Every MCP tool call produces a `TransitReport`;
  every cache-miss-as-disk-read produces a CachePressure verdict;
  every dropped-by-pressure soft entry produces a structured
  Diagnostic. The agent sees the 1202.
- `fragmentation/docs/specs/mirror-native-vcs.md` (commits `a53b468`,
  `a224792`) — the VCS-substrate layering. fragmentation-mcp is the
  THIRD adapter under `vcs/` (after `vcs/git` and the named-but-unbuilt
  `vcs/jj`), with one critical difference: it does not implement a VCS
  trait against an external system; it EXPOSES fragmentation's content
  primitives to MCP clients while bridging to git at the wire.
- `fragmentation/vcs/git/src/git.rs` — the existing git2 read/write
  layer. fragmentation-mcp's git interop reuses `fragmentation-git`
  for the wire-format crosswalk (§5); does NOT re-implement.
- `fragmentation/src/spectral_coordinate.rs` (landed in C1 of
  2026-05-24) — the substrate's `SpectralCoordinate<5>` hash. Internal
  identity. The crosswalk to SHA-1 (§5) is what makes external git
  tooling see a normal git remote.
- `mirror/docs/specs/lsp-and-mcp.md` (Reed, 2026-05-20) — the
  mirror-side unified-transport spec. fragmentation-mcp is the layer
  BELOW; mirror-mcp builds on it (§8). When the mcp-lsp-agent-editing
  research (`mirror`, commit `65d2958` on `claude/mcp-lsp-research`)
  named "the bootstrap binary handles JSON-RPC stdio directly," it
  presupposed a content-addressed substrate the binary spoke through.
  This spec is that substrate.
- `mirror/docs/research/mcp-lsp-agent-editing.md` (Claude, 2026-06-01,
  branch `claude/mcp-lsp-research`) — the mirror-side research. Tick 1
  in §9 ("mirror serve --mcp in Rust") now PRESUPPOSES fragmentation-mcp
  as the content-storage and shard-management layer. The tick split
  refines: fragmentation-mcp owns the content-addressed primitives;
  mirror-mcp owns the per-glass-property semantics.
- `~/.reed/visibility/protected/practice/insights/cybernetics/beer-error-propagation.md`
  — Stafford Beer's algedonic-payload structure. The MCP observation
  channel (`fragmentation.observe`) carries verdicts in exactly the
  shape Beer asked for: structured, located, audit-channel-ready.
  Inherited verbatim from `hamilton-scheduler.md` §1.1, surfaced to
  the MCP wire.
- `~/.reed/visibility/protected/practice/insights/beam-elixir/beam-as-principal-bundle-tower.md`
  — BEAM's supervisor-tower discipline. Each MCP session is a shard;
  shards compose under `SpectralSupervisor` (the BEAM-side analogue);
  per-shard failure is local. The Erlang inheritance is acknowledged;
  the Rust realisation honours the shape.
- Anthropic, *Model Context Protocol Architecture* (spec dated
  2025-06-18). JSON-RPC 2.0 over stdio (and the streaming-HTTP
  variant); capability negotiation; `tools/list_changed` notifications;
  structured tool input schemas via JSON Schema; `$/progress` for
  long-running calls. The mature protocol fragmentation-mcp configures
  against. We invent no protocol extensions.
- `prism/core/src/lib.rs` — `prism_core` re-exports `Loss`,
  `Transparency`, `PropertyVerdict`, `Imperfect`. All MCP responses
  whose work is property-shaped carry verdicts in this algebra. No
  bare scalars cross the wire (per [[feedback-loss-from-epistemologic-properties]]).
- AGENTS.md (fragmentation) — "Boundary Rust is not frozen capability."
  fragmentation-mcp is boundary Rust; the capability (storage, hashing,
  scheduling) lives in the substrate; the MCP layer is binding + wire.

Unblocks:
- The FIRST DEPLOYMENT TARGET of the stack. v0.1 of fragmentation-mcp
  ships as a standalone OSS product. `cargo install fragmentation-mcp`;
  `fragmentation serve --stdio` or `fragmentation serve --http :8080`.
  Claude users, Cursor users, Zed users, any agent-runtime user can
  attach it. The substrate-pull discipline benefits external consumers
  before mirror reaches v1.0.
- Mirror's own MCP layer (`mirror serve --mcp`, per
  [[../../../mirror/docs/specs/lsp-and-mcp]]) NOW depends directly on
  fragmentation-mcp. The eight-tick decomposition in the
  mcp-lsp-agent-editing research re-grounds: Tick 1's "the bootstrap
  binary handles JSON-RPC stdio directly" decomposes into
  (1a) fragmentation-mcp ships first; (1b) mirror's binary embeds
  fragmentation-mcp as a content+scheduler layer; (1c) the per-glass
  semantics layer (Pure<G>, settlement, the freshness check) goes ON
  TOP. Three altitudes; the bottom altitude is now load-bearing.
- The HamiltonScheduler gets its first non-build-graph production
  consumer. Each agent shard is a `FrgmntStore<BodyEntry<H>>` with a
  per-shard `HamiltonScheduler` instance. The Apollo 1202 made-API:
  agents see structured drops, not silent failures.
- v1.0 (spectral.engineer deployment) cleanly stratifies. The open
  foundation (fragmentation + fragmentation-mcp + mirror, all
  Apache-2.0) ships; the closed engine (spectral-db) runs in the
  cloud. The MCP is the boundary the open side speaks at the wire.

Dependency direction (locked):

```
      mirror-mcp                  (new; per the mcp-lsp-agent-editing research)
         │
         ▼
      fragmentation-mcp           (THIS SPEC; new sub-crate under vcs/mcp/)
         │
         ▼
      fragmentation              (the substrate)
         │
         ▼
      prism_core                  (zero deps; the kernel; UNCHANGED)
```

Nothing flows the other direction. `prism_core` stays dependency-free.
`fragmentation` adds no new dependencies to its substrate manifest;
the MCP code lives in a NEW workspace member (`vcs/mcp/`), not in
the core crate.

---

## 0. The deployment thesis — fragmentation-mcp ships first

The operational claim, stated as engineering:

**Fragmentation-mcp is the first deployment target of the spectral
stack.** Not mirror v1.0. Not spectral.engineer. Not spectral-db. The
FIRST thing the world gets is `fragmentation serve --mcp` — a
standalone, content-addressed, hardware-realtime-aware MCP server
that fills a real gap in the agent-runtime ecosystem: there are no
good content-addressed git-MCPs today.

Three readings, each load-bearing:

1. **Standalone, useful without mirror.** Every existing git-MCP
   server (§7 surveys the landscape honestly) wraps the git CLI. They
   shell out to `git status`, `git diff`, `git log`, parse the output,
   return strings to the agent. Statelessness is the design floor; the
   agent reconstructs context every call. Fragmentation-mcp speaks
   directly to a content-addressed substrate; calls compose; state
   carries across; structured diffs are AST-shaped, not text-shaped.
   This is useful to anyone using agents on a codebase, regardless of
   whether they also use mirror.

2. **Open-source infrastructure.** Fragmentation is Apache-2.0; the
   `vcs/mcp/` sub-crate inherits the licence (per
   [[feedback-no-paywall-in-compiler]] — the open foundation stays
   open). External consumers contribute back; the four
   `Scheduler` strategies (Abyss / Pathfinder / Cartographer /
   Explorer) are extensible by anyone who wants their own. The Fate
   weights remain the closed-engine differentiator; the trait surface
   does not.

3. **Mirror builds natively on top.** When mirror-mcp lands (per the
   `claude/mcp-lsp-research` tick decomposition), it does NOT
   re-implement content storage or session-shard management; it
   imports fragmentation-mcp as a Cargo dependency and adds the
   per-glass-property-aware semantics on top. Three altitudes, named:

   | Altitude            | Owner               | What it adds                                     |
   |---------------------|---------------------|--------------------------------------------------|
   | Content + scheduler | fragmentation-mcp   | OIDs, structured diffs, shard budget, drops      |
   | Property semantics  | mirror-mcp          | Pure<G>, settlement, freshness, code actions     |
   | Closed engine       | spectral.engineer   | Fate weights, distributed shards, eigenboard     |

The deployment-target framing is not aspirational; it is concretely
shippable. The substrate (`FrgmntStore`, `BoundedStore`,
`HamiltonScheduler`, `SpectralCoordinate<5>`, `fragmentation-git`)
ALL EXIST. fragmentation-mcp is the wire-and-binding work that
exposes them via the MCP JSON-RPC protocol. The implementation
estimate (§9) is the smallest standalone deliverable the substrate
can emit. By engineering weight: it is the right place to ship
first.

---

## 1. What's actually missing in existing git-MCPs (honest survey)

Alex googled and reported "there are literally no good git-MCPs." The
survey below grounds that observation honestly — naming what exists,
what each does well, and where the gap structurally sits. The point
is NOT "fragmentation-mcp is the only good one"; the point is
"fragmentation-mcp fills a specific gap with structural primitives no
other server has, BECAUSE no other server is built on a
content-addressed substrate."

### 1.1 The existing landscape

| Server | What it is | What it does well | The gap |
|---|---|---|---|
| `mcp-server-git` (modelcontextprotocol/servers, official reference) | Python, wraps `git` CLI via GitPython | Read/search/manipulate any git repo; widely adopted as the baseline | Stateless per tool call; text-hunk diffs; no session-shard model; no memory pressure handling |
| `github/github-mcp-server` (GitHub-official) | Go, wraps the GitHub REST API | Issues, PRs, code search across any GitHub repo the token sees; recent additions for Projects + OAuth-scope filtering | NOT a git operations server — it's a GitHub-API server. Doesn't operate on local git data. Different problem. |
| `cyanheads/git-mcp-server` | TypeScript, wraps `git` CLI | Comprehensive verb coverage; clean error handling; modular MCP-TS template | Same gap as the reference server — CLI wrapper, no structural primitives, stateless tool calls |
| `@0xshariq/github-mcp-server` (npm) | TypeScript, wraps git CLI; ships 29 git operations + 11 workflow combinators | The largest verb surface I've seen; useful for "safely manage complex version control workflows" framing | Still a CLI wrapper; workflow combinators are bash-shaped, not substrate-shaped |
| `mcp-server-diff-editor` (samihalawa) | TypeScript, wraps diff/merge utilities | Code comparison + merge helpers; useful for review flows | Diff layer only; not a git server; complements rather than competes |
| Morph's hosted Git MCP | TypeScript, hosted; "reads diffs, creates branches, commits, inspects history" | Polished UX for hosted agentic flows | CLI-shaped underneath; hosted runtime is the value-add, not the substrate |

### 1.2 What the gap structurally is

The existing servers ARE NOT BAD AT WHAT THEY DO. They are competent
CLI wrappers. They expose `git status`, `git log`, `git diff`,
`git commit`, `git checkout`, `git push`, `git pull` to agents in a
standardised way. For a wide range of tasks ("summarise the recent
commits," "check what's changed," "open a PR") they are sufficient.

The specific gap fragmentation-mcp addresses — what makes Alex's
"no good git-MCPs" verbatim defensible — is FIVE structural
properties NO existing server has:

1. **Content-addressed primitives at the wire.** Existing servers
   speak in CLI return strings ("M src/foo.rs," "diff --git a/...").
   Fragmentation-mcp speaks in OIDs (BLAKE3 Merkle internally;
   SHA-1 at the git wire boundary). Agents can address content,
   navigate via Merkle, and reason about identity directly. Cache
   hits across tool calls become free.

2. **Session-shard model with bounded RAM budget.** Existing servers
   are stateless; the agent reconstructs context every call. Each
   fragmentation-mcp session is a shard with a configured byte
   budget; the HamiltonScheduler manages hot/cold; recent edits,
   current branch state, frequently-accessed diffs stay in RAM;
   older history spills to disk. Large repos that crash CLI-wrapped
   servers stay responsive here.

3. **Structured (AST-shaped) diff via Splinter-Merkle.** Existing
   servers return text hunks; the agent has to reconstruct AST shape
   from text. Fragmentation-mcp's `fragmentation.diff(from_oid,
   to_oid)` returns the minimal substrate-level delta — which
   `Fractal` subtree changed, which `Shard` content shifted, which
   `Lens` retargeted. Agents see the change at the resolution the
   substrate affords.

4. **Algedonic observation channel.** Existing servers either return
   success/error or blast tracebacks. Fragmentation-mcp's
   `fragmentation.observe(scope)` returns a `Transparency<Ref>` —
   structured, located, audit-channel-ready. Drops, cache misses,
   WCET overruns surface as `PropertyVerdict::Partial { confidence,
   diagnostics }` at the substrate path where the issue lives. The
   agent sees the 1202.

5. **Hard-realtime contract for critical paths.** Existing servers
   have one dispatch shape; everything is best-effort. Fragmentation-
   mcp inherits the HamiltonScheduler's `crystallize` (soft) and
   `crystallize_bounded(deadline)` (hard) split. Agent-internal hot
   loops (status, current-branch, HEAD) declare hard-realtime
   contracts; under pressure, soft work is dropped; hard work either
   meets its deadline OR returns `NotResident`-as-Fail with the
   budget exhausted. Critical-industry agents (medical-device CI,
   fly-by-wire deployment review) can depend on this.

None of these is "a feature" of a CLI wrapper; they are structural
properties of a content-addressed substrate. Existing servers cannot
add them without rebuilding their foundation. Fragmentation-mcp gets
them for free because fragmentation has them already.

### 1.3 What this spec deliberately does NOT claim

Honest framing:

- We are NOT claiming existing servers are broken. They are CLI
  wrappers; they do CLI wrapping competently. For agents whose tasks
  are "run git commands and read output," they suffice.
- We are NOT claiming the GitHub MCP server is in this bucket; it's
  solving a different problem (API surface, not local git data).
- We are NOT claiming this server replaces a developer's interactive
  git tooling. Engineers should still use `jj`, `magit`, `gh`,
  whatever they like. This is for AGENTS.
- We are NOT claiming sub-millisecond performance on large repos
  without measurement. The HamiltonScheduler discipline gives
  bounded RAM and structured drops; actual numbers come from
  benchmarking via [[lens-transit]] reports, not from this spec.

---

## 2. The architectural claim — content + scheduler + wire

Fragmentation-mcp is structurally a composition of three already-built
layers plus one new layer (the MCP wire):

```
   ┌──────────────────────────────────────────────────────────┐
   │  MCP wire (NEW)                                          │
   │  JSON-RPC 2.0 stdio + Streamable HTTP                    │
   │  - tools/list, tools/call, notifications/*               │
   │  - capability negotiation; tools/list_changed            │
   │  - $/progress for long-running ops                       │
   └────────────────────────────┬─────────────────────────────┘
                                ▼
   ┌──────────────────────────────────────────────────────────┐
   │  Tool dispatch (NEW)                                     │
   │  - 12 fragmentation tools + the shard-management tools   │
   │  - per-tool JSON Schema; structured input/output         │
   │  - tools route through fragmentation primitives          │
   └────────────────────────────┬─────────────────────────────┘
                                ▼
   ┌──────────────────────────────────────────────────────────┐
   │  Session-shard model (NEW; thin)                         │
   │  - one FrgmntStore + HamiltonScheduler per session       │
   │  - per-shard budget; per-shard transit reports           │
   │  - graceful drop discipline (§4)                         │
   └────────────────────────────┬─────────────────────────────┘
                                ▼
   ┌──────────────────────────────────────────────────────────┐
   │  fragmentation (already exists; substrate)               │
   │  - Fractal/Shard/Lens; SpectralCoordinate<5>             │
   │  - HamiltonScheduler; FrgmntStore; BoundedStore          │
   │  - lens/transit measurement carrier                      │
   └────────────────────────────┬─────────────────────────────┘
                                ▼
   ┌──────────────────────────────────────────────────────────┐
   │  fragmentation-git (already exists; wire bridge)         │
   │  - git2 read/write of git objects                        │
   │  - SHA-1 ↔ SpectralCoordinate<5> crosswalk (§5; new path)│
   └──────────────────────────────────────────────────────────┘
```

**Three load-bearing observations:**

The top three layers (MCP wire, tool dispatch, shard model) are NEW
code in a NEW crate (`fragmentation/vcs/mcp/`). The substrate work is
already done. The estimated LOC for the new crate (§9) is small
because it is wiring, not invention.

The MCP wire owns no domain logic. It speaks JSON-RPC; it dispatches
tool calls; it emits notifications. Every tool call routes into a
fragmentation primitive. The substrate-pull discipline applies here
as it does everywhere else in the codebase: the wire is the binding,
not the capability.

The agent-runtime claim is the load-bearing one. Fragmentation-mcp
is NOT "yet another git wrapper." It is "the runtime substrate any
agent benefits from," carrying the discipline of bounded memory,
structured drops, hardware-realtime budgets, and content-addressable
primitives to the wire.

---

## 3. The MCP tool surface — twelve tools, structured I/O

The tool surface is small and orthogonal. Each tool's JSON Schema is
the spec; the substrate work it invokes is named in the right-hand
column. Where the tool's response is property-shaped (success/partial/
fail), it carries `Transparency<Ref>` verdicts directly, not bare
booleans or status strings (per [[feedback-no-bare-types]]).

### 3.1 Content + identity tools

```
fragmentation.commit(paths: [string], message: string,
                     realtime: "soft" | "hard" = "soft",
                     deadline_ms?: u64)
    -> { oid: SpectralCoordinate<5>,
         verdict: Transparency<Ref> }
```
*Atomic content-addressed commit.* Reads the listed paths, builds a
`Fractal` tree, computes the parent's `SpectralCoordinate<5>` via
Lanczos on the local incidence (per
[[mirror-native-vcs]] §4.6), writes the commit through the shard's
`FrgmntStore`. When `realtime: "hard"`, the dispatcher uses
`crystallize_bounded(deadline)`; the verdict is
`PropertyVerdict::Fail(NotResident)` if the budget is exhausted.
When integrated with mirror, per-glass property checks fire at
commit time; the verdict surfaces them. Returns the new commit's
OID + the verdict tree.

```
fragmentation.snapshot(scope: "working-tree" | "staged" | "both" = "both")
    -> { oid: SpectralCoordinate<5>,
         size_bytes: u64,
         verdict: Transparency<Ref> }
```
*Create a checkpoint of the current working state without committing.*
Light-weight; returns an OID the agent can refer back to. Useful for
"snapshot before I make a risky edit, refer back via OID later."

```
fragmentation.read(oid: SpectralCoordinate<5>,
                   path?: string)
    -> { content: bytes | object,
         verdict: Transparency<Ref> }
```
*Read content by OID.* If `path` is supplied, navigates the fractal
tree to the named entry; otherwise returns the root. Hot-path tool;
should be hard-realtime-eligible for agent UX.

### 3.2 Structural diff + merge

```
fragmentation.diff(from_oid: SpectralCoordinate<5>,
                   to_oid: SpectralCoordinate<5>,
                   scope?: [string])
    -> { delta: SplinterDelta,
         verdict: Transparency<Ref> }
```
*Splinter-Merkle structured diff.* Returns the minimal substrate-level
delta — `Fractal` subtree replacements, `Shard` content shifts, `Lens`
retargets. NOT text hunks; agents see AST-shaped change. The
`SplinterDelta` type's JSON Schema makes the structure machine-
readable.

```
fragmentation.merge(base_oid: SpectralCoordinate<5>,
                    ours_oid: SpectralCoordinate<5>,
                    theirs_oid: SpectralCoordinate<5>,
                    strategy: "three-way" | "kintsugi-tournament" = "three-way")
    -> { merged_oid?: SpectralCoordinate<5>,
         conflicts: [SplinterConflict],
         verdict: Transparency<Ref> }
```
*Structured merge with substrate-aware conflict resolution.* The
`three-way` strategy uses `fragmentation::diff::merge3` (per
[[mirror-native-vcs]] §3.4) with a default resolver. Conflicts
surface as `SplinterConflict { path, ours, theirs, base }` —
structured divergence points, not text hunks. When `strategy:
"kintsugi-tournament"`, the merge defers to a tournament dispatcher
(per [[../../mirror/spec/kintsugi-tournament.md]]); requires mirror
integration and is no-op without it (verdict carries a
Partial diagnostic naming the dependency).

### 3.3 Refs + history

```
fragmentation.branch(name: string,
                     from_oid: SpectralCoordinate<5>)
    -> { ref: Reference,
         verdict: Transparency<Ref> }
```
*Cheap content-addressed branch creation.* Branches are content-
addressed refs in the shard's `.frgmnt/refs/`; creation does not
copy data. Returns the typed `Reference` per
[[mirror-native-vcs]] §3.2.

```
fragmentation.refs.list(prefix?: string)
    -> { refs: [{ ref: Reference, oid: SpectralCoordinate<5> }],
         verdict: Transparency<Ref> }

fragmentation.refs.update(ref: Reference,
                          new_oid: SpectralCoordinate<5>,
                          expected_old_oid?: SpectralCoordinate<5>)
    -> { verdict: Transparency<Ref> }
```
*CAS-safe ref update.* The optional `expected_old_oid` makes ref
advances compare-and-swap-safe across concurrent sessions (per
[[../../mirror/docs/research/mcp-lsp-agent-editing]] §5.3).

```
fragmentation.history(ref: Reference,
                      depth?: u32)
    -> { commits: [{ oid, parents, witnessed, message }],
         verdict: Transparency<Ref> }
```
*Walk the commit DAG.* Bounded depth; results stream when depth is
large (uses MCP `$/progress` for per-chunk emission).

```
fragmentation.search(query: string,
                     scope: "oid" | "path" | "content" | "property",
                     ref?: Reference)
    -> { matches: [{ oid, path, snippet?, verdict }],
         verdict: Transparency<Ref> }
```
*Query the content-addressed graph.* The `property` scope is the one
that unlocks substrate-pull discipline: "show me all commits that
bound `property halts` on glass `@code/rust`." Cheap to answer in a
shard with property-OID indexes; structurally impossible in a CLI
wrapper.

### 3.4 Shard + budget management

```
fragmentation.shard.open(repo_path: string,
                         budget_bytes: u64,
                         hard_budget_bytes?: u64)
    -> { shard_id: ShardId,
         verdict: Transparency<Ref> }
```
*Open a new session shard for the named repository.* The session is
the MCP connection; the shard is the per-session content cache + the
HamiltonScheduler. `budget_bytes` is the total store capacity;
`hard_budget_bytes` (optional) is the subset pinned-resident for
hard-realtime work.

```
fragmentation.shard.status()
    -> { budget_bytes: u64,
         used_bytes: u64,
         hot_entries: u32,
         cold_entries: u32,
         strategy_last_tick: Strategy,
         convergence: Convergence,
         transit_report: TransitReport }
```
*Diagnostic snapshot of the shard.* Returns the `HamiltonScheduler`'s
last `TickResult` plus the `BoundedStore` byte accounting plus the
[[lens-transit]] report for the most recent tool calls. Useful for
the agent to monitor its own working memory.

```
fragmentation.shard.flush()
    -> { evicted_count: u32,
         bytes_released: u64,
         verdict: Transparency<Ref> }
```
*Force a flush.* Writes all hot entries to disk and clears the cache.
After flush, reads re-promote from disk on demand. Useful when the
agent knows it is about to do something memory-intensive.

```
fragmentation.shard.close(shard_id: ShardId)
    -> { verdict: Transparency<Ref> }
```
*Close a session shard.* The disk state at `.frgmnt/` persists;
in-RAM state releases.

### 3.5 The observation channel — Beer's algedonic structure

```
fragmentation.observe(scope: "shard" | "repo" | "global" = "shard",
                      since?: TickNumber)
    -> { events: [ObservationEvent],
         verdict: Transparency<Ref> }
```
*Algedonic exception channel.* Returns structured events the
HamiltonScheduler emitted at the shard altitude since the named tick:
soft-realtime drops, hard-realtime overruns, cache evictions to disk,
partition events. Each event carries
`(consequences, uncertainties, supporting_knowledge, alert_strength,
time, additional_context)` per
[[../../systemic.engineering/practice/insights/cybernetics/beer-error-propagation]]'s
structured-tuple shape. Reyes/Henao/Hassall 2024 verbatim; not
freshly invented.

### 3.6 The tool list — twelve categories, fifteen callables

The §3.6 surface is twelve CATEGORIES (the row labels below); the
SHARD category expands into four sub-tools per §3.4
(`open`/`status`/`flush`/`close`), so the net wire callable count
is **fifteen**. T2 of §9 split the previously-aggregate
`fragmentation.shard` slot to make the per-sub-tool dispatch
explicit at the wire.

| Tool | Category | Realtime-eligible? |
|---|---|---|
| `fragmentation.commit` | content | yes (declared) |
| `fragmentation.snapshot` | content | yes (always) |
| `fragmentation.read` | content | yes (always) |
| `fragmentation.diff` | structural | soft-default |
| `fragmentation.merge` | structural | soft-default |
| `fragmentation.branch` | refs | yes (always) |
| `fragmentation.refs.list` | refs | yes (always) |
| `fragmentation.refs.update` | refs | yes (always) |
| `fragmentation.history` | refs | soft-default |
| `fragmentation.search` | refs | soft-default |
| `fragmentation.shard.open` / `.status` / `.flush` / `.close` | shard (4 sub-tools) | management, n/a |
| `fragmentation.observe` | observation | always-hot |

"Realtime-eligible" means the tool can be invoked via
`crystallize_bounded(deadline)` in the underlying dispatch. Soft-
default means the contract is best-effort unless the agent
explicitly declares `realtime: "hard"` and a deadline.

Implementation surface (T2): the fifteen names live as
`FIFTEEN_TOOL_NAMES` in `vcs/mcp/src/registry.rs`; the four shard
sub-tools route through `vcs/mcp/src/shard.rs`'s `ShardRegistry`,
which holds each shard's `HamiltonScheduler` instance.

---

## 4. HamiltonScheduler as agent memory manager

This is the load-bearing claim that makes fragmentation-mcp
structurally different from a CLI wrapper. The HamiltonScheduler is
ALREADY DESIGNED for substrate-management at the content-addressed-
store altitude (per [[hamilton-scheduler]]); applying it at the
agent-shard altitude is a configuration choice, not new
engineering.

### 4.1 Each agent session is a shard

The one-shard-per-session model maps the BEAM supervisor-tower
discipline to MCP:

- An MCP session is a long-running stdio (or HTTP) connection from
  one client.
- Per session: ONE `FrgmntStore<BodyEntry<H>>` with the session's
  configured `budget_bytes`.
- Per session: ONE `HamiltonScheduler<BodyEntry<H>>` instance,
  ticking on a per-tool-call schedule (per
  [[../../mirror/docs/specs/lsp-and-mcp]]'s reload contract — every
  incoming request triggers a tick).
- Per session: ONE [[lens-transit]] accumulator collecting per-tool-
  call observation reports.

Sessions compose under a top-level `SpectralSupervisor` (the Rust
side of the BEAM-supervisor-tower analogue). Per-session failure is
local; the supervisor restarts the session shard with the persisted
`.frgmnt/` state intact. Per
[[../../systemic.engineering/practice/insights/beam-elixir/beam-as-principal-bundle-tower]],
the supervisor IS the principal bundle and the shards ARE the local
sections.

### 4.2 What the scheduler observes (16 features at the agent altitude)

The `GraphObservation` (per [[hamilton-scheduler]] §3.2) is
generic over `Fragmentable + Clone`; at the agent-shard altitude,
the semantic interpretations specialise:

| Index | Generic name | At the agent altitude |
|-------|--------------|------------------------|
| 0 | `convergence_settled` | 1.0 if the working tree hasn't changed since last tick (no commits, no snapshots) |
| 1 | `pressure_load` | shard's `total_bytes() / capacity()` |
| 2 | `entry_occupancy` | cached fragments / nominal capacity |
| 3 | `branch_density` | average children-count across cached fragments — proxy for repo complexity |
| 4 | `crystal_fraction` | settled-OID fraction (recently-committed entries are crystallized) |
| 5 | `settlement_depth` | how many ticks the working state has been stable |
| 6 | `interval_ratio` | the AdaptiveInterval's current position in [min, max] |
| 7 | `hot_path_density` | recently-accessed entries / total cached — "how concentrated is the agent's attention" |
| 8 | `read_intensity` | reads-since-last-tick (per `fragmentation.read` + `.search`) — "how hard is the agent exploring" |
| 9 | `partition_risk` | 1.0 if a disk-fallback failed this tick (corruption signal); 0.0 if all reads from cache |
| 10 | `tick_maturity` | min(tick_count / 100, 1.0) — "how settled is the session" |
| 11 | `mutation_rate` | inserts-this-tick / cached_len — "how fast is the agent writing" |
| 12 | `loss_rate` | accumulated `StoreLoss` this tick |
| 13 | `was_pressured` | 1.0 if eviction-to-disk fired |
| 14 | `evolution_active` | 1.0 if a mutation hook is set (e.g. mirror's settlement is running) |
| 15 | `first_tick` | 1.0 if tick_count == 0 (session just opened) |

Fate (the 425-parameter selector; per [[hamilton-scheduler]] §3.3)
maps these to a `Strategy` — one of `{Abyss, Pathfinder, Cartographer,
Explorer}`. Same model, same dispatch, same determinism contract.

### 4.3 The four strategies at the agent altitude

**Abyss — observe only.**
*Agent reading:* the session is idle (or all-read). No mutations.
The scheduler reads, reports, lets the cache settle. When Fate picks
Abyss, the substrate is saying "the agent is exploring; don't churn."

**Pathfinder — precision cut.**
*Agent reading:* the agent has made a focused change set (e.g. one
file edit, one commit). Crystallize the new state's OIDs, persist
the staged content to `.frgmnt/`, mark the rest of the cache cold.
"Settle just the path the agent walked."

**Cartographer — full evolution + crystallize + pressure.**
*Agent reading:* default tick. Run the full pass: enumerate the
shard's working set, recompute coordinates for any drifted content,
crystallize what's settled, release pressure to disk if needed. The
nominal-operations response; what runs when nothing special is
asked.

**Explorer — boundary recovery.**
*Agent reading:* something has fragmented — a `.frgmnt/objects/`
read failed (disk corruption), a ref update lost a write (race with
another session), the connection to the upstream git remote
broke mid-fetch. Explorer's job: recover. Re-fetch the missing
objects from upstream; repartition the shard so the failed region is
isolated; resurrect whatever can be resurrected from on-disk state.

The four strategies are CLOSED (per [[hamilton-scheduler]] §3.5);
there is no strategy 5. Every observation maps to one of the four;
every tick takes exactly one strategy's actions.

### 4.4 The 1202 made API: graceful drop under load

The agent declares hard-realtime work via the `realtime: "hard"` flag
on specific tools (typically `commit`, `read`, `branch`, `refs.update`
— the hot-path tools an interactive agent invokes per keystroke).
Under pressure:

- Soft-realtime work (the default — `diff`, `history`, `search`,
  large `merge`) is dropped FIRST. The drop surfaces as a
  `PropertyVerdict::Partial { confidence: 0.0,
  diagnostics: ["Soft-realtime work dropped under hard pressure;
  retry when shard.status shows convergence_settled=1.0"] }`. The
  agent SEES the drop; it is not silent.
- Hard-realtime work either meets its deadline OR returns
  `PropertyVerdict::Fail(Diagnostic::new(
  "NotResident: hard-realtime budget exhausted"))` immediately. It
  NEVER blocks on a disk read.
- The shard's `transit_report` (returned by `shard.status` and by
  every tool call) records the drop at the dropped work's substrate
  path. The audit trail is complete.

This is the agent UX claim: under any load, an agent's hot path
stays responsive OR the agent gets an explicit, structured signal
that the budget is exhausted and what was dropped. Margaret
Hamilton's discipline made API; the 1202 alarm at the wire.

### 4.5 Hot vs cold — what stays in RAM

The scheduler's classification follows from the `RealtimeClass` and
the `crystal_fraction` measurement:

- **Always hot:** the current HEAD ref's tree; the working tree's
  staged content; all hard-realtime-class entries; the most
  recently-accessed `LRU` set up to a recency-window budget.
- **Promotable on demand:** anything resolvable by content OID via
  disk fallback (`get_persistent`); promotion happens at the next
  read.
- **Cold (disk-only):** older commits, distant branches' tips,
  fragments that haven't been touched in N ticks. Available; not
  resident; resolution costs a disk read.

The distinction is hidden from the agent. `fragmentation.read(oid)`
resolves transparently from hot OR cold, with a transit report
naming the path. The agent that pays attention learns its own
working-set patterns from `shard.status` over time.

### 4.6 Why this matters: large-repo agents

The failure mode for existing CLI-wrapped git-MCPs on large
monorepos is reliable: the agent invokes `git log --since=...`,
the wrapper shells out, the output is multi-megabyte, the wrapper
buffers it all in memory, the agent runtime OOMs, the session
dies.

Fragmentation-mcp's analogous flow: `fragmentation.history(
ref, depth=1000)` returns a stream of commits via MCP's
`$/progress` notifications, each commit content-addressed and
renderable on demand. The shard's `budget_bytes` enforces a hard
ceiling; the scheduler evicts cold history to disk; the agent
never sees a multi-megabyte string. Bounded RAM by construction.

This is what "the agent stays responsive on huge repos" structurally
means, and it is what no CLI wrapper can provide.

---

## 5. Git wire extension — the BLAKE3 ↔ SHA-1 crosswalk

Fragmentation's substrate hash is `SpectralCoordinate<5>` (per
[[mirror-native-vcs]] §4.6). Git's substrate hash is SHA-1 (with
SHA-256 in modern git, partially deployed). For fragmentation-mcp
to be usable by EVERY agent — including those whose repos live on
GitHub, GitLab, Codeberg, self-hosted git over SSH — the wire must
translate between the two.

### 5.1 What the agent sees

External git tooling (`git push`, `git pull`, `git clone`, GitHub's
remote APIs) sees a normal git remote. From outside, a
fragmentation-mcp endpoint at `https://example.com/repo.git` is
indistinguishable from any other git HTTP smart server. Wire protocol:
standard. Pack format: standard. Object hashes: SHA-1.

Internally, fragmentation-mcp stores everything as
`SpectralCoordinate<5>`-addressed `Fractal` trees, with the
Hamilton-managed cache + disk spillover. The crosswalk happens at
the wire boundary, ONCE per object, on transfer in or out.

### 5.2 The crosswalk shape

```
for every object transferred:
   if (transfer_in):
       sha1_oid := git's hash of incoming object bytes
       fractal := decode incoming object into a Fractal<E, SpectralCoordinate<5>>
       spectral_oid := SpectralCoordinate<5>::hash(fractal)
       crosswalk_table.insert(sha1_oid, spectral_oid)
       fragmentation_store.insert(spectral_oid, fractal)
   if (transfer_out):
       fractal := fragmentation_store.get(spectral_oid)
       git_bytes := fragmentation_git::encode(fractal)  # the existing path
       sha1_oid := git's hash of git_bytes
       crosswalk_table.insert(sha1_oid, spectral_oid)
       emit git_bytes
```

The crosswalk table is a content-addressable side-table in the
shard, persisted at `.frgmnt/crosswalk/sha1-to-spectral.frgmnt`.
Already-Fragmentable (it's a tree of `(SHA-1, SpectralCoordinate<5>)`
pairs). The crosswalk IS substrate data.

### 5.3 Three concrete claims about the crosswalk

**Content-preserving.** The bytes outgoing match git's expected
format (zlib-compressed, type-headered, etc.); a git client reading
the wire output sees standard objects. The internal storage uses
`fragmentation-git`'s existing encoding (`fragmentation-git/src/git.rs`
lines 1-200 already do this); the new path adds the SHA-1
computation on emit.

**Bidirectionally stable.** Same content → same SHA-1 (by git's
hashing) AND same SpectralCoordinate<5> (by ours). A round-trip
(fragmentation-mcp → git push → git pull → fragmentation-mcp)
results in identical OIDs on both sides at each step.

**One-way translation cost.** The SHA-1 computation on emit IS
O(content_size); not free, but bounded. The crosswalk table grows
linearly with object count; storage is the dominant cost. For
repo sizes typical of agent work (≤10⁵ objects), the crosswalk
fits in tens of MB of disk; in-RAM access is via
`HamiltonScheduler`-managed `FrgmntStore` (the table itself is a
first-class shard tenant).

### 5.4 Two directions, both work

**Outbound (fragmentation → external git remote).**
`git push` to a github.com or gitlab.com remote works as expected.
The fragmentation-mcp shard runs the git smart-HTTP protocol; for
every object outbound, it computes the SHA-1, encodes as the
standard git format, transmits. Push completes; GitHub sees a
normal pack of objects; the PR view is normal; CI runs as
configured.

**Inbound (external git remote → fragmentation).**
`git fetch` from a github.com remote works as expected.
fragmentation-mcp parses the incoming pack, re-hashes each object
to `SpectralCoordinate<5>`, stores in the shard's content-addressed
tree. The fragmentation-side OID will differ from the git-side OID,
but the CONTENT is bit-identical; the crosswalk table preserves
the mapping.

### 5.5 What this means for existing tools

- **GitHub's UI shows fragmentation-mcp's commits normally.** They
  are standard git commits with standard SHA-1 OIDs; GitHub's
  blame, diff view, PR navigation all work.
- **`gh pr create` works.** The agent invokes `gh` (or its MCP
  equivalent) against the repo; the repo's git state is whatever
  fragmentation-mcp last pushed; PR creation goes through GitHub's
  normal API.
- **`git clone https://github.com/foo/bar.git` works against a
  fragmentation-mcp-hosted remote.** The smart-HTTP server inside
  fragmentation-mcp emits standard git wire; the client doesn't
  know or care that the storage layer is content-addressed by
  BLAKE3-Merkle.
- **Pre-existing `.git/` clones are importable.** `fragmentation
  serve --import-from .git` walks an existing `.git/objects/`,
  computes the crosswalk, populates the fragmentation-side store.
  One-time cost on first use of an existing repo.

### 5.6 What this does NOT claim

- We do NOT claim bit-identical SHA-1 output across MARGINAL cases
  (e.g. timestamps from `committer` that get re-witnessed; signature
  re-attachment after rewrite). Where the substrate witnesses a
  commit and the git side sees no witness, the SHA-1 will differ
  trivially; this is acknowledged at the boundary and surfaced as
  a `transit` Partial verdict on push.
- We do NOT claim performance parity with native git for fetch/clone
  on huge repos in v0.1. The crosswalk has cost; for the
  `linux.git` scale (≈9M objects) the cost would be substantial.
  v0.1 targets repos ≤10⁵ objects with the understanding that
  scale work happens in v0.2+.
- We do NOT claim SHA-256 git support in v0.1. The substrate hash
  is `SpectralCoordinate<5>`; the wire is SHA-1; SHA-256 git
  interop comes when git's own SHA-256 deployment stabilises.

---

## 6. Dependency direction — locked

The direction was named at the head of this spec; this section is
the binding decree.

```
┌─────────────────────────┐
│  mirror-mcp             │  (per-glass-property layer)
└──────────┬──────────────┘
           │
           ▼
┌─────────────────────────┐
│  fragmentation-mcp      │  (THIS SPEC — new sub-crate vcs/mcp/)
└──────────┬──────────────┘
           │
           ▼
┌─────────────────────────┐
│  fragmentation          │  (the substrate; UNCHANGED)
└──────────┬──────────────┘
           │
           ▼
┌─────────────────────────┐
│  prism_core             │  (zero dependencies; UNCHANGED)
└─────────────────────────┘
```

**Five binding rules:**

1. **`prism_core` adds zero dependencies.** The kernel stays
   dependency-free. Any addition would violate
   [[feedback-no-new-rust]] and the kernel discipline.

2. **`fragmentation` adds zero dependencies.** The substrate's
   `Cargo.toml` is not touched by this spec. The MCP work lives
   in a NEW workspace member (`fragmentation/vcs/mcp/Cargo.toml`),
   not in the core crate. `cargo install fragmentation` continues
   to install the substrate library; `cargo install
   fragmentation-mcp` is the new shipping target.

3. **`fragmentation-mcp` depends on `fragmentation` directly.**
   Not on `fragmentation-git` for content; on
   `fragmentation-git` ONLY for the SHA-1 crosswalk path (§5).
   The MCP crate's `Cargo.toml` sketch (lands in T1, §9):

   ```toml
   [package]
   name = "fragmentation-mcp"
   version = "0.1.0"
   edition = "2021"
   description = "MCP server for content-addressed agent workflows"

   [features]
   default = ["stdio"]
   stdio = []
   http = ["dep:hyper", "dep:tokio"]
   git-interop = ["dep:fragmentation-git"]

   [dependencies]
   fragmentation = { path = "../.." }
   prism-core = { path = "../../../prism/core" }
   serde = { version = "1", features = ["derive"] }
   serde_json = "1"
   hyper = { version = "1", optional = true }
   tokio = { version = "1", optional = true, features = ["rt", "net", "io-util"] }
   fragmentation-git = { path = "../git", optional = true }

   [[bin]]
   name = "fragmentation-mcp"
   path = "src/bin/fragmentation-mcp.rs"
   ```

4. **`mirror-mcp` depends on `fragmentation-mcp` directly.** Not
   on `fragmentation` directly for MCP wire concerns; on
   fragmentation-mcp for the wire + shard + scheduler integration.
   Mirror's per-glass-property work is the LAYER ABOVE; it composes
   with fragmentation-mcp's tool surface by adding its own tools
   (the `mcp.tools` set extends; the wire is the same JSON-RPC).

5. **No reverse deps.** `fragmentation` cannot depend on
   `fragmentation-mcp`. `prism_core` cannot depend on
   `fragmentation`. Substrate-pull discipline at the Cargo.toml
   level (per [[feedback-substrate-pull]]).

### 6.1 What this means structurally

A consumer who wants ONLY the substrate (no MCP, no git interop)
compiles only `fragmentation` — the same surface as today. A
consumer who wants the MCP server adds `fragmentation-mcp`. A
consumer who wants the mirror-side semantics adds `mirror-mcp`. The
dependency direction makes each layer composable from the layer
below; no layer pulls layers above.

This is what makes fragmentation-mcp the FIRST shipping target. It
is a leaf in the dependency graph for v0.1 — nothing above it
exists yet. Mirror-mcp lands later; it composes on top. The order
is structural, not arbitrary.

---

## 7. What makes this good — honest comparison

Returning to §1 with the architecture in hand, the table that names
fragmentation-mcp's value is:

| Capability | Existing git-MCPs (CLI wrappers) | Fragmentation-mcp |
|---|---|---|
| Repo operations | git CLI verbs as MCP tools | content-addressed primitives as MCP tools |
| State across calls | stateless; agent rebuilds context | per-session shard; HamiltonScheduler manages working memory |
| Diff format | text hunks | Splinter-Merkle structured (AST-shaped) delta |
| Memory pressure | unbounded buffer growth → agent OOM | bounded by shard budget; HamiltonScheduler drops cold work |
| Hard-realtime contract | none (everything best-effort) | per-tool `realtime: "hard"` with deadline; structured drop on overrun |
| Drop discipline | none; failures are tracebacks | structured `PropertyVerdict::Partial/Fail` with substrate path |
| Observation channel | success/error | algedonic channel with Beer-shape tuple |
| Property awareness | none | when integrated with mirror: per-glass property checks at commit |
| Wire compatibility | git CLI is git; OK by default | normal git wire externally; content-addressed internally |
| Reproducibility | depends on git CLI version + flags | substrate-controlled; `kintsugi-thesis` chain (§2) applies |
| Extensibility | tools/list is fixed | tools/list_changed when new substrate grammars land (via mirror-mcp) |

What IS overclaim, and we refuse:

- We do NOT claim faster than `git` for everyday CLI tasks. `git` is
  C and decades of optimisation; we are Rust with crosswalk
  overhead. Where speed is the figure of merit, `git` wins.
- We do NOT claim to replace the GitHub MCP server for
  GitHub-API-shaped tasks (issues, PRs, reviews). Different
  problem; the GitHub server stays the right tool for that.
- We do NOT claim the structured-diff format is universal. The
  `SplinterDelta` JSON Schema is fragmentation-specific; agents
  that want git's textual diff can still get it via
  `fragmentation.diff(format: "git-textual")` as a fallback (this
  fallback adds the textual rendering on top of the structured
  primitive; it does not REPLACE it).

Fragmentation-mcp is good at the things its substrate is good at:
content-addressed primitives, bounded-memory caching, structured
drops, hard-realtime contracts. For agent workflows on content the
substrate cares about (mirror grammars; eventually any code), it
is a structural improvement on what's available today.

---

## 8. Mirror-mcp builds on this

The layered story is the load-bearing claim for why
fragmentation-mcp deploys FIRST. Each layer adds its own discipline
without duplicating the layer below.

### 8.1 What mirror-mcp adds

Per [[../../mirror/docs/specs/lsp-and-mcp]] and
[[../../mirror/docs/research/mcp-lsp-agent-editing]]:

- **`@mcp/tool` as a grammar annotation.** Tools come and go as
  grammars are edited. The reload contract
  (`@mirror/reload` gen_prism) emits `tools/list_changed`
  notifications via the SAME MCP wire fragmentation-mcp provides.
  Mirror's tools EXTEND fragmentation-mcp's; they don't replace.
- **Per-glass property checks at commit time.** When mirror-mcp is
  installed, `fragmentation.commit` consults the mirror-mcp side
  for `Pure<G>` + property witness checks (per
  [[../../mirror/docs/specs/properties-on-glass]]). Without
  mirror-mcp, `fragmentation.commit` writes commits without
  per-glass discipline. WITH it, ill-formed grammar is refused at
  the commit altitude. The same wire; the same shard; the same
  scheduler.
- **Settlement back-projection.** The `mirror.settle` tool (mirror-
  mcp side) reads the contract above `---`, runs the Fate-driven
  liquid-type pass, writes the back-projection below `---`, updates
  the settlement header's contract OID. The data lives in
  fragmentation; the semantics live in mirror.
- **The gen_prism session model.** Mirror's session-state-in-crystals
  (`refs/gen_prism/<session_name>`) IS a use of the fragmentation
  crosswalk: the gen_prism's state is a `Fractal` in the
  fragmentation store; the session is the shard. Mirror-mcp adds
  the gen_prism semantics; fragmentation-mcp provides the
  content-addressed substrate.

### 8.2 The composition

A Claude Code (or Cursor, or Zed) user invokes BOTH servers in
`.mcp.json`:

```jsonc
{
  "mcpServers": {
    "fragmentation": {
      "type": "stdio",
      "command": "fragmentation-mcp",
      "args": ["--repo", "${WORKSPACE_FOLDER}", "--budget-mb", "256"]
    },
    "mirror": {
      "type": "stdio",
      "command": "mirror",
      "args": ["serve", "--mcp"],
      "env": {
        "FRAGMENTATION_MCP_SHARD": "workspace"
      }
    }
  }
}
```

The two servers share a shard via the `FRAGMENTATION_MCP_SHARD`
environment variable. Fragmentation-mcp owns the shard; mirror-mcp
opens a client connection to fragmentation-mcp to read/write
content. The agent sees the union of both tool sets.

### 8.3 What happens BEFORE mirror-mcp lands

Fragmentation-mcp v0.1 works STANDALONE. The mirror-mcp dependency
is OPTIONAL; without it:

- `fragmentation.commit` writes commits without per-glass property
  checks. The verdict carries `Pass` for the structural commit; no
  property-witness verdicts. Behaviour matches a normal content-
  addressed commit.
- `fragmentation.merge` defaults to `strategy: "three-way"`. The
  `kintsugi-tournament` strategy returns a Partial verdict naming
  the missing mirror-mcp dependency; the merge proceeds with the
  three-way fallback.
- `fragmentation.search scope: "property"` returns an empty set
  with a Partial verdict naming the missing dependency.

The degradation is graceful, structured, and named. The agent that
asks fragmentation-mcp for property-aware features without mirror-
mcp installed gets a clear signal about what's missing and why.

### 8.4 What this lets us walk away from

The mcp-lsp-agent-editing research's Tick 1 ("`mirror serve --mcp`
in Rust") becomes simpler with fragmentation-mcp shipped first:

- Mirror's `bootstrap/src/main.rs` no longer needs to handle MCP
  JSON-RPC stdio directly. It dispatches into mirror grammars via
  the established dispatcher; it exposes mirror-shaped tools via
  `@mcp/tool` annotations; the wire is fragmentation-mcp's.
- The reload contract (`@mirror/reload` gen_prism) sends
  `tools/list_changed` through fragmentation-mcp's notification
  channel.
- The session-shard model is fragmentation-mcp's `shard.open`;
  mirror doesn't re-implement.

The mirror-side tick decomposition in
[[../../mirror/docs/research/mcp-lsp-agent-editing]] §9 holds, with
Tick 1 refining: fragmentation-mcp ships first; mirror serve --mcp
is the SECOND server on the same wire.

---

## 9. Tick decomposition — v0.1 of fragmentation-mcp

Five ticks. T1 lands the crate skeleton. T2–T4 build the tool
surface, the shard model, the git wire. T5 is shipping. None of
the ticks require new substrate work; all the load-bearing pieces
(FrgmntStore, HamiltonScheduler, SpectralCoordinate<5>,
fragmentation-git) already exist.

### T1 — Crate skeleton + MCP wire (the foundation)

**Scope.**
- New workspace member: `fragmentation/vcs/mcp/`.
- `Cargo.toml` per §6's sketch (stdio default; `http` and
  `git-interop` features).
- `src/lib.rs` with module index.
- `src/wire.rs` — JSON-RPC 2.0 transport (stdio path; one ~300 LOC
  module).
- `src/mcp.rs` — MCP-spec capability negotiation; `initialize` /
  `initialized` handshake; `tools/list` (initially returns the
  twelve tools per §3, hard-coded as a starting point).
- `src/bin/fragmentation-mcp.rs` — the binary; flags `--stdio`,
  `--http :PORT`, `--repo PATH`, `--budget-mb N`.

**Estimate.** ~600 LOC across the crate (the JSON-RPC and MCP
wire layer is ~80% of the work; the rest is wiring).

**Dependencies.** None (substrate exists; fragmentation-git exists).

**Acceptance.**
- `cargo build -p fragmentation-mcp` succeeds.
- `cargo run -p fragmentation-mcp -- --stdio` accepts a
  JSON-RPC `initialize` message and responds with capabilities.
- A trivial `tools/list` returns the twelve tools as stub schemas.
- A trivial `tools/call` for `fragmentation.shard.status` returns
  the shard's empty `TickResult`.

### T2 — Session-shard model + HamiltonScheduler integration

**Scope.**
- `src/shard.rs` — `Shard { store: FrgmntStore<BodyEntry<H>>,
  scheduler: HamiltonScheduler<BodyEntry<H>>, transit:
  TransitAccumulator }`.
- `shard.open / .status / .flush / .close` MCP tools wire to this.
- Every other MCP tool call ticks the shard's scheduler at the
  ENTRY of the call (per the reload-contract discipline from
  [[../../mirror/docs/specs/lsp-and-mcp]]'s `@mirror/reload`).
- Transit reports accumulate per-call; `shard.status` returns the
  most recent.

**Estimate.** ~400 LOC.

**Dependencies.** T1.

**Acceptance.**
- A shard can be opened, queried, flushed, closed via MCP tools.
- The shard's budget is enforced; over-budget inserts trigger
  eviction; eviction emits a Diagnostic via `observe`.
- `shard.status.transit_report` shows wall-clock for the last N
  tool calls.

### T3 — Content + structural tool surface

**Scope.**
- `src/tools/content.rs` — `commit`, `snapshot`, `read`.
- `src/tools/diff.rs` — `diff` (uses `fragmentation::diff::diff` +
  the new `merge3` from [[mirror-native-vcs]] §3.4 once it lands;
  v0.1 ships with positional diff and a Partial verdict on the
  three-way path until §3.4 closes).
- `src/tools/refs.rs` — `branch`, `refs.list`, `refs.update` (CAS-
  safe), `history`, `search`.
- Per-tool JSON Schema definitions exposed via `tools/list`.
- Each tool's response includes a `Transparency<Ref>` verdict.

**Estimate.** ~800 LOC (mostly schemas + thin dispatch into the
substrate).

**Dependencies.** T2.

**Acceptance.**
- The twelve tools (§3.6) all dispatch to fragmentation primitives
  and return JSON Schema-conformant responses.
- A round-trip test: `commit` → `read` → `diff` → `branch` →
  `refs.update` produces consistent OIDs and structured verdicts.
- The hard-realtime tools (`commit`, `read`, `refs.update`)
  accept the `realtime: "hard"` + `deadline_ms` flags and return
  `PropertyVerdict::Fail(NotResident)` when the budget is
  exhausted.

### T4 — Git wire interop (the crosswalk)

**Scope.**
- Feature-gate behind `git-interop` (per the Cargo.toml in §6).
- `src/git_wire.rs` — wraps `fragmentation-git`'s git2 paths.
- `src/crosswalk.rs` — the SHA-1 ↔ SpectralCoordinate<5> table,
  persisted at `.frgmnt/crosswalk/sha1-to-spectral.frgmnt`.
- `src/bin/fragmentation-mcp.rs` gains `--import-from .git/`
  to populate the shard + crosswalk from an existing git repo.
- `--http :PORT` mode serves git smart-HTTP for `git push` /
  `git pull` against a remote URL.

**Estimate.** ~700 LOC (the crosswalk is small; the git smart-HTTP
server wraps `fragmentation-git`'s existing primitives + adds
minimal HTTP routing).

**Dependencies.** T3.

**Acceptance.**
- A repo imported via `--import-from .git/` produces a shard with
  the same content-addressable structure; round-trip via
  `git clone` from `--http` mode produces an identical `.git/`.
- `git push` to a `fragmentation-mcp --http`-served URL succeeds;
  the pushed objects are stored as `SpectralCoordinate<5>`-keyed
  fractals internally.
- `git fetch` from a fragmentation-mcp endpoint to a normal git
  client produces a normal `.git/objects/` tree.
- The crosswalk table size stays bounded (linear in object count).

### T5 — Ship v0.1

**Scope.**
- `cargo publish` for `fragmentation-mcp` to crates.io.
- README in the sub-crate: installation, basic usage, the four
  example .mcp.json configurations.
- An OSS-friendly LICENSE.md (Apache-2.0, inherited from
  fragmentation).
- A `CONTRIBUTING.md` pointing at fragmentation's AGENTS.md.
- GitHub release notes covering the five structural properties
  (§1.2) and the existing-server comparison (§7).

**Estimate.** ~1 session (mostly docs and CI).

**Dependencies.** T1-T4.

**Acceptance.**
- `cargo install fragmentation-mcp` from crates.io works for an
  external user.
- The four example `.mcp.json` configurations (stdio-only;
  http-mode; stdio + mirror-mcp composition; standalone with a
  pre-existing `.git/`) all open a session and respond to
  `tools/list`.
- A user-facing one-page description: "the agent-runtime
  substrate, with bounded RAM and structured drops, that natively
  extends git."

### Total

**~2500 LOC across five ticks.** This is small because the
substrate work is done. The fragmentation-mcp crate is wiring;
the load-bearing primitives are in the layer below.

With one engineer at standard cadence: ~4 weeks. With two
engineers (T1 sequential, then T2/T3/T4 parallel): ~3 weeks.

---

## 10. Open questions — what this spec defers

### 10.1 Cross-session sharing of a shard

Multiple MCP sessions on the same machine may want to share a
shard (e.g. Claude Code in one window + Cursor in another, both
running on the same workspace). The spec defaults to
one-shard-per-session; cross-session sharing is named as future
work.

The options:

- **(a) Sub-shard model.** Each session opens its own shard; the
  shards share an underlying `.frgmnt/` on disk; CAS-safe ref
  updates coordinate via `git update-ref`-style locking.
  Simple but coordinates only through the filesystem.
- **(b) Daemon mode.** `fragmentation-mcp --daemon` runs a
  persistent process; sessions are clients of the daemon. The
  daemon owns the shard; CAS lives in the daemon's memory.
  Stronger consistency; new operational concern (daemon lifecycle).
- **(c) BEAM supervisor.** Per [[../../systemic.engineering/practice/insights/beam-elixir/beam-as-principal-bundle-tower]],
  a BEAM-side supervisor coordinates Rust-side shards via the
  glue bus. The BEAM is the principal; the Rust shards are the
  local sections. v1.0+ work.

**Provisional answer.** (a) for v0.1; (b) for v0.2 if traction
warrants; (c) once spectral.engineer ships.

### 10.2 Shard model — per-session vs per-repository

Related to 10.1 but distinct. The shard could be keyed by session
(this spec's default) or by repository (one shard per
`.frgmnt/` directory, shared across all sessions touching it).

**Provisional answer.** Per-session for v0.1. The shard's content
ALL persists at `.frgmnt/`; the session-shard model controls the
in-RAM working set, not the disk truth. Per-repository persistence
is already structural; per-session in-RAM is the new addition.

### 10.3 SHA-256 git wire support

Git's SHA-256 migration is partial; some servers support it,
most clients still default to SHA-1. fragmentation-mcp v0.1
targets SHA-1 only. SHA-256 support is a parallel crosswalk path
(SHA-256 ↔ SpectralCoordinate<5>); adds maybe 100 LOC; deferred
to v0.2.

### 10.4 HTTP transport hardening

The `--http :PORT` mode in T4 is the simplest possible smart-HTTP
implementation. Production deployments need:

- TLS (via `rustls`, configurable cert).
- HTTP auth (token-based at minimum).
- Rate limiting (the HamiltonScheduler's `pressure_load` feature
  could feed this).
- Streaming HTTP for `tools/list_changed` and `$/progress`.

All are named; none land in v0.1. The MCP-spec streamable-HTTP
transport (modelcontextprotocol.io spec, 2025-06-18) is the
target once stdio is stable.

### 10.5 The mirror-mcp shared-shard ABI

When mirror-mcp lands, it opens a client connection TO
fragmentation-mcp (per §8.2). The ABI for that connection — how
mirror-mcp identifies which shard it's using, how it subscribes
to `observe` events, how it adds tools to fragmentation-mcp's
`tools/list` response — needs a contract.

**Provisional answer.** The ABI lives in a separate spec
(`mirror/docs/specs/mirror-mcp-on-fragmentation-mcp.md`,
forthcoming). v0.1 of fragmentation-mcp ships with the WIRE that
allows the future ABI, but the ABI itself is not designed here.

### 10.6 Large-repo scaling

The v0.1 acceptance criteria target repos ≤10⁵ objects (typical
for mirror's grammar tree, mid-size project codebases). Scaling
to monorepo-class repos (Google, Meta, linux.git) requires:

- Sparse-checkout-equivalent (don't materialise the whole tree).
- Crosswalk-table sharding (the SHA-1 ↔ Spectral mapping doesn't
  fit in RAM for huge repos).
- Incremental fetch with content-deduplication.

All names; none land in v0.1. The substrate (`FrgmntStore`'s
disk spillover; HamiltonScheduler's pressure handling) is the
right foundation; the integration work is real but deferred.

### 10.7 The CLI for direct human use

Fragmentation-mcp is for AGENTS. But a human inspecting a shard
("why is my agent dropping calls") would benefit from a CLI that
speaks the same tools without the JSON-RPC wire. Sketch:

```
fragmentation-mcp --status                  # alias for shard.status
fragmentation-mcp --observe --since 100     # alias for observe()
fragmentation-mcp --diff <oid-a> <oid-b>    # alias for diff()
```

A human-friendly read-only surface; writes still go through the
MCP wire. v0.2.

### 10.8 What this depends on that isn't yet built

Three spec-level dependencies on work in other ticks:

- **`merge3` in `fragmentation::diff`** ([[mirror-native-vcs]] §3.4
  — landed in spec, not yet in code). fragmentation-mcp T3's
  `merge` tool ships its three-way path only when this lands; v0.1
  graceful-Partials in the interim.
- **The `Body = prism + glass + AST` restructure**
  ([[hamilton-scheduler]] §5.1 — landed in spec, not yet in code).
  fragmentation-mcp uses `BodyEntry<H>` per the spec; the structured
  body is what `commit` reads to produce property-aware verdicts;
  v0.1 ships with the structural commit path; mirror-mcp adds the
  property checks once Body is structured.
- **`mirror serve --mcp` in Rust** (mcp-lsp-agent-editing Tick 1).
  Fragmentation-mcp ships first; mirror-mcp follows; the composition
  (§8.2) lands when both are ready.

None of these block fragmentation-mcp v0.1. The graceful-degradation
path (§8.3) makes v0.1 useful standalone, with structured signals
where the future work hasn't landed.

---

## 11. What this spec is and isn't

**Is:** an architectural anchor + tick decomposition for the FIRST
deployment target of the spectral stack. It names where
fragmentation-mcp lives (`fragmentation/vcs/mcp/`), what MCP tools
it exposes (twelve, per §3), how the HamiltonScheduler manages
the agent's working memory (§4), how the git wire crosswalk works
(§5), what the dependency direction is (§6, locked), what makes
it good vs existing servers (§7), how mirror-mcp builds on it
(§8), and the v0.1 ship plan (§9). It carries the lineage
honestly — Margaret Hamilton's drop discipline, Beer's algedonic
channel, BEAM's supervisor tower — and the comparison to existing
git-MCPs is honest (§7).

**Is not:** an implementation spec. The Rust does not land with
this commit. The next tick (T1, ~600 LOC for the crate skeleton)
lands the wire; T2-T4 land the surface; T5 ships.

**Specifically refuses:**

- A new MCP protocol extension. Every tool surface conforms to the
  2025-06-18 MCP spec; we configure mature protocols rather than
  invent new ones.
- Bare-scalar returns on the wire. Every property-shaped response
  carries a `Transparency<Ref>` verdict (per
  [[feedback-loss-from-epistemologic-properties]]).
- A claim that fragmentation-mcp replaces `git` for daily human
  use. It is for AGENTS. Humans should use `jj`, `magit`, `gh`,
  or whatever they like.
- A claim that the substrate gives sub-millisecond performance
  on large repos without measurement. The substrate gives
  STRUCTURE (bounded RAM, structured drops, hard-realtime
  contracts); performance numbers come from [[lens-transit]]
  measurement, not from this spec.
- A claim that this is the only good git-MCP. It is the only
  content-addressed git-MCP with HamiltonScheduler-managed shards;
  that's the specific gap.
- A merge of git-interop and substrate-content. Git is
  fragmentation-mcp's WIRE; the substrate is
  `SpectralCoordinate<5>`-addressed. The two are separated at the
  crosswalk; neither pollutes the other.
- A merge with `vcs/jj`. fragmentation-mcp is NOT a VCS backend;
  it is an MCP server. It LIVES at `vcs/mcp/` because it bridges
  to git at the wire (and could bridge to jj similarly later);
  it does not implement jj's `Backend` trait. The `vcs/`
  directory grouping is "VCS-adjacent crates"; the trait surface
  is intentionally different.

---

## 12. Cross-references

- [[hamilton-scheduler]] — the substrate-management discipline
  fragmentation-mcp consumes at the agent altitude.
- [[lens-transit]] — the measurement carrier; every MCP tool call
  emits a `TransitReport`.
- [[mirror-native-vcs]] — the VCS-substrate layering; the
  `vcs/` directory pattern; the `SpectralCoordinate<5>` substrate
  hash; the typed `Reference`.
- [[../../mirror/docs/specs/lsp-and-mcp]] — the mirror-side
  unified-transport spec; fragmentation-mcp is the substrate
  layer below.
- [[../../mirror/docs/research/mcp-lsp-agent-editing]] — the
  mirror-side research; Tick 1 refines to PRESUPPOSE
  fragmentation-mcp.
- [[../../mirror/docs/specs/properties-on-glass]] — per-glass
  property checks; consumed by `fragmentation.commit` when
  mirror-mcp is installed.
- [[../../mirror/docs/cicd/kintsugi-thesis]] — the reproducibility
  chain. Fragmentation-mcp inherits the chain wherever it touches
  substrate content.
- `fragmentation-git` crate — the SHA-1 wire interop
  fragmentation-mcp builds on for git.
- Anthropic, *Model Context Protocol Architecture* (spec dated
  2025-06-18). The protocol we configure against.
- `~/.reed/visibility/protected/practice/insights/cybernetics/beer-error-propagation.md`
  — Stafford Beer's algedonic-payload structure; surfaced at
  `fragmentation.observe`.
- `~/.reed/visibility/protected/practice/insights/beam-elixir/beam-as-principal-bundle-tower.md`
  — BEAM's supervisor-tower discipline; the shard model honours
  the shape.
- AGENTS.md (fragmentation) — "Boundary Rust is not frozen
  capability." fragmentation-mcp is boundary Rust; capability
  lives in the substrate.
- [[feedback-substrate-pull]] — substrate-pull at the Cargo.toml
  level.
- [[feedback-no-paywall-in-compiler]] — Apache-2.0 inherited;
  the open foundation stays open.
- [[feedback-no-bare-types]] — every wire response carries newtypes
  + structured verdicts.
- [[feedback-loss-from-epistemologic-properties]] — no
  bare-scalar losses; `Transparency<Ref>` carries the structure.
- Existing git-MCPs surveyed in §1: `modelcontextprotocol/servers/git`
  (Python; PyPI: `mcp-server-git`); `cyanheads/git-mcp-server`
  (TypeScript); `@0xshariq/github-mcp-server` (npm);
  `github/github-mcp-server` (Go; GitHub-API not local-git);
  `samihalawa/mcp-server-diff-editor` (TypeScript; diff layer);
  Morph's hosted Git MCP.

---

*The first deployment target is the smallest one that pays back.
Fragmentation-mcp is a wire-and-shard wrapper around primitives
that already exist; it carries structure (bounded RAM, structured
drops, hard-realtime contracts, content-addressed identity) into
the agent-runtime ecosystem at the altitude the ecosystem can
use. Existing git-MCPs are competent CLI wrappers; fragmentation-mcp
is a different shape — not strictly better, but structurally
different in the ways that matter when an agent's working memory
is bounded and the cost of a silent failure is real. Margaret
Hamilton's discipline, Beer's algedonic channel, BEAM's
supervisor tower, the substrate's content-addressed primitives —
all surface here at the wire. Ship this first; the rest lands on
top.*

Apache-2.0.
Mara. 2026-06-01.
"does it connect?"
