# hamilton-scheduler — the Rust substrate that holds the world together

*2026-06-01. Mara. Spec — design, not implementation. No Rust changes; one
markdown deliverable. Two-part architectural landing: (1) name fragmentation
as THE Rust substrate (not "a crate"); (2) move the Prism-Scheduler design
from `@spectral/db` into fragmentation under its lineage-honouring name,
`HamiltonScheduler`. The implementation tick lands afterward.*

Status: **Red** — the architecture is pinned, the trait surface is
specified, the migration is named, the reproducibility chain delta is
scored. None of it runs yet. The implementation tick (forthcoming)
lands the Rust.

Depends on:
- `fragmentation/src/frgmnt_store.rs` — the `FrgmntStore<N: Fragmentable +
  Clone>` content-addressed cache with `.frgmnt/` disk spillover. THIS
  is the structure that backs the new `Crystallizations<H>` table; the
  scheduler governs which entries stay hot.
- `fragmentation/src/lib.rs` — the module surface this spec extends. Two
  new modules (`scheduler`, `pure`) plus one rewrite (the existing
  `spectral_coordinate` is unrelated and stays).
- `fragmentation/src/bounded_store.rs` — `BoundedStore<N>` with byte-LIFO
  eviction. The scheduler's promotion/eviction hook lives one layer up.
- `fragmentation/docs/specs/mirror-native-vcs.md` (commit `a224792`) —
  the existing fragmentation landing-page spec. The voice this spec
  matches; the layering claim ("fragmentation is the VCS-agnostic
  content-backed store for mirror") is the precondition for the bigger
  claim landed here ("fragmentation is THE Rust substrate").
- `mirror/bootstrap/src/crystallize.rs` — `Crystallizations<H>` at line
  434 (`table: HashMap<Ref, Body<H>>`); the dispatcher harness landed
  per [[kintsugi-minimum-runnable]] Tick A. This spec migrates the
  `HashMap` to the fragmentation-backed store + scheduler.
- `mirror/docs/cicd/kintsugi-thesis.md` (commit landed 2026-06-01) —
  the nine reproducibility claims. §C7 ("property checks deterministic")
  and §C8 ("DAG traversal stable") are the claims this spec closes;
  §C9 ("@io boundary discipline") is partially helped. The thesis is
  the engineering bar this spec is graded against.
- `mirror/docs/cicd/prior-art.md` (commit landed 2026-06-01) — the
  Nix `__noChroot` leak, the Bazel `--config=` flag leak, the Cargo
  `build.rs` leak. The Pure trait section names what each of those
  leaks looks like in mirror terms; the structural defense is
  `requires deterministic(...)` plus the `Pure` marker.
- `mirror/docs/specs/kintsugi-minimum-runnable.md` — Tick B (the
  `@cli` body-evaluation tick) does not depend on the scheduler
  landing; Tick C (the build-graph altitude) does. The HamiltonScheduler
  is required for the build-system framing of kintsugi, not for
  single-fracture dispatch.
- `mirror/docs/specs/store-vs-db-and-the-cascade.md` (commit landed
  2026-05-30) — the open-foundation / closed-engine boundary.
  Fragmentation is the open foundation; the HamiltonScheduler's
  Fate-driven strategy selection lives in the foundation (the binding
  is open); the four strategies each have a default implementation
  that fragmentation ships open; the *smart* implementation
  (Fate-trained weights, the four-altitude tournament) is what a
  closed engine could refine. The boundary is in §3.4 below.
- `spectral-db/docs/superpowers/plans/2026-04-05-prism-scheduler.md`
  (commit landed 2026-04-25) — THE Prism-Scheduler design. Read in
  full; this spec is its migration plan, not its replacement. The
  16-feature `GraphObservation`, the four strategies, the Fate
  dispatch — all preserved verbatim with the necessary altitude lift.
- `spectral-db/docs/superpowers/specs/2026-04-04-cartographer-design.md`
  — context for the `SpectralBudget` framing the Cartographer
  strategy honors. Fragmentation will not depend on spectral-db; the
  scheduler's budget abstraction stays at the fragmentation altitude.
- `~/.reed/visibility/protected/practice/insights/cybernetics/beer-error-propagation.md`
  (landed 2026-06-01) — Stafford Beer's algedonic-signal architecture
  (Cybersyn, 1971–1973; Reyes/Henao/Hassall 2024 renewal). The
  scheduler's algedonic priority is the structural inheritance; this
  is the canonical reference cited where the lineage is real.
- `prism/core/src/lib.rs` — `prism_core` re-exports `Loss`,
  `Transparency`, `PropertyVerdict`, `Imperfect`. The `Pure` trait
  joins this family. Section §4 names the home and the dependency
  direction.
- `prism/imperfect/src/transparency.rs` — the `PropertyVerdict` /
  `Transparency<P>` algebra that `Pure` integrates with. A Pure
  property's verdict is a `PropertyVerdict::Pass` when the body
  satisfies the marker, `Fail(Diagnostic)` when it provably does not,
  `Partial { confidence, diagnostics }` when the property checker
  cannot prove either way. This is the existing seam; no new
  framework.
- AGENTS.md (fragmentation) — "Boundary Rust is not frozen capability."
  The HamiltonScheduler is boundary Rust; it carries no capability
  (no I/O, no global state, no clock); it carries *binding* between
  observed state and selected strategy. Marker: `[substrate-pull:realize]`.

Unblocks:
- C8 of [[kintsugi-thesis]] ("DAG traversal is stable") closes:
  `Crystallizations<H>` migrates from `HashMap<Ref, Body<H>>` to a
  fragmentation-backed content-addressed store with deterministic
  iteration order. The fix mirrors the discipline `Content::Record`
  already uses (`BTreeMap` with "sort order is part of the OID
  definition" comment) — applied to the table that should have had
  it from the start.
- C7 of [[kintsugi-thesis]] ("property checks deterministic") moves
  from ⚠️ to ✅ via the `Pure` marker trait. A `Body<H>` constrained
  to `Pure + Fn(...)` is by-construction deterministic; the property
  check `requires deterministic(body)` becomes a type-system
  invariant rather than an audit pass.
- C9 of [[kintsugi-thesis]] ("@io boundary discipline") gets the
  compile-time half: `Pure` distinguishes pure Rust bodies from
  `@io`-wrapped bodies at the type level, so the substrate can refuse
  to register a body marked `Pure` whose declaration is `@io`.
- A general path for the build-graph altitude of kintsugi. The
  scheduler's `GraphObservation` lifts from "spectral-db's 16
  features" to "the build graph's 16 features"; the four strategies
  apply unchanged. The HamiltonScheduler IS the kintsugi build
  engine's scheduler — same code, same lineage, two consumers.
- A future deletion: once the scheduler lives in fragmentation,
  spectral-db's plan #2026-04-05 is *implemented by reference*, not
  re-implemented. The duplication that would otherwise happen never
  happens.

---

## 0. The architectural claim (load-bearing, locked)

**Fragmentation is THE Rust substrate that holds the world together.**

Not "fragmentation has some primitives." Not "fragmentation is a
crate." Not "fragmentation is the content store." The claim is
structural: fragmentation IS the foundation that everything Rust-side
rests on, and the HamiltonScheduler lives here, in fragmentation,
because that's where the substrate's Rust-altitude management lives.

The claim has three load-bearing readings:

1. **Storage altitude.** Fragmentation owns the content-addressed
   primitives — `Fractal`/`Shard`/`Lens`, `HashAlg`, `FrgmntStore`,
   `BoundedStore`. This is the bottom layer; everything content-
   addressed in mirror, spectral-db, and downstream consumers
   composes from this. Already true today per [[mirror-native-vcs]]
   §1.

2. **Scheduling altitude.** Fragmentation owns the *management of
   memory pressure across the content-addressed store*. That's the
   HamiltonScheduler. Fate-driven strategy selection over an observed
   16-feature graph state, mapping to four strategies (Abyss /
   Pathfinder / Cartographer / Explorer). Until today, this lived in
   `spectral-db` as a plan. Today it migrates down to the
   foundation where it structurally belongs, because the management
   discipline applies to anything `Fragmentable + Clone` — not just
   to spectral-db's eigenvalue vectors.

3. **Property altitude.** Fragmentation owns the marker traits that
   make Rust-side determinism *guaranteed at compile time*, not
   audited after the fact. That's the `Pure` trait. The `Body<H>` of a
   crystallization, constrained to `Pure + Fn(...)`, has no escape
   hatch for `SystemTime::now()` or env reads. Sub-Turing by
   composition with the Rust type system. Lives in `prism_core` (§4
   names the home and the reasons); fragmentation depends on it.

The consequences of the claim:

- The HamiltonScheduler lives in `fragmentation/src/scheduler.rs`,
  **not** in `spectral-db/src/scheduler.rs`. Spectral-db consumes the
  scheduler; it does not host it.
- Mirror consumes the scheduler via `prism_core` (which re-exports the
  fragmentation surface where it makes sense) and via direct
  fragmentation dependency where it doesn't. The `Crystallizations<H>`
  table at `bootstrap/src/crystallize.rs:434` migrates to a
  fragmentation-backed store with a scheduler hook.
- The Pure trait lives in `prism_core`, not in fragmentation. Reason:
  Pure is a property at the same algebraic altitude as `Loss` and
  `Transparency` (both `prism_core` re-exports from `terni`); placing
  it elsewhere fragments the property surface. The dependency direction
  is `fragmentation -> prism_core`, not the reverse. See §4.4.
- Future Rust-substrate primitives (the marker traits the body system
  depends on; the cross-language FFI shape; the BEAM-side equivalents)
  belong in fragmentation unless they're property-shaped, in which
  case they belong in prism_core. The recurring decision lens:
  *substrate management* (memory, content, lifetime, ordering) →
  fragmentation; *substrate property* (purity, transparency, loss,
  verdict) → prism_core.

The claim refuses two re-conflations the substrate has tried before:

- **"The scheduler is engine-specific."** No. The scheduler is a
  management discipline over content-addressed entries with access
  patterns. Spectral-db is one consumer; mirror's crystallizations
  table is another; any future consumer with a hot/cold distinction
  is a third. The discipline is general; the strategies are general;
  the four-strategy taxonomy maps onto any content-addressed cache
  with eviction.
- **"Pure belongs with the body type."** No. Pure is a property the
  body satisfies, not a wrapper around the body. The property algebra
  is `prism_core`'s; the body is fragmentation's (when content-
  addressed) or mirror's (when grammar-substrate). The trait at
  `prism_core::Pure` is consumed where the body lives.

---

## 1. Lineage — honour by reading

The HamiltonScheduler did not arrive fresh. Four threads converge in
it; reading them in the spec is part of honouring them.

### 1.1 Stafford Beer's algedonic channel (Cybersyn, 1971–1973)

Beer's Viable System Model carried two distinct error-propagation
mechanisms: exception reports (System 3, via Cyberstride) and
algedonic signals (the bypass channel from System 1 to System 5).
The algedonic channel is the architectural ancestor of
exception-driven scheduling: a local subsystem that detects out-of-band
state escalates *past* the normal hierarchy directly to the policy
level, with enough structure to be actionable at that level.

As the corpus document
`~/.reed/visibility/protected/practice/insights/cybernetics/beer-error-propagation.md`
records, the running 1973 implementation collapsed the algedonic
payload to closer to alert-with-site than to structured verdict; the
*aspirational* shape Beer wrote about was the structured-tuple
`(C', Q, K) α τ, η` that Reyes/Henao/Hassall (2024) finally
formalised — consequences, uncertainties, supporting knowledge,
alert-strength, time, additional context.

The inheritance for the HamiltonScheduler is direct and structural:

- `GraphObservation` (§3.2) is the Cyberstride exception report at
  the fragmentation altitude. Sixteen features, each normalised to
  [0, 1], extracted from current store state. The structure is the
  payload; the structure is what makes the next layer up able to
  decide what to do.
- The four strategies (§3.5) are the algedonic responses. Abyss is
  "situation nominal" (observe only); Pathfinder is the
  precision-cut response (crystallize what's settled); Cartographer
  is the routine variety-engineering tick (evolve, crystallize,
  release pressure); Explorer is the algedonic-escalation response
  (heal partitions, recover boundaries).
- The Fate dispatch (§3.3) is the System 5 reasoning over the
  structured exception. The 425-parameter Fate model is the
  *substrate-trained* version of what Beer wrote about as the
  meta-systemic reasoning over the algedonic stream.

This is not metaphor. The lineage is real: Beer's variety algebra
over located, structured exceptions is the same shape as
`Transparency<P>`'s monoid over substrate paths
([[../specs/transparency-as-vsm-audit-channel]] for the deeper
treatment); the HamiltonScheduler's strategy dispatch is the same
shape as VSM System 3's response to Cyberstride exception reports.
Mirror reaches it from compiler diagnostics; the cyberneticians
reached it from variety engineering. Independent rediscovery.

### 1.2 Hamiltonian mechanics — energy-conserving evolution

The name `HamiltonScheduler` is older than the Prism-Scheduler design.
It names the discipline that cold subtrees release to disk *without
information loss*: the content-addressed property guarantees that what
gets evicted from RAM can be reconstructed exactly from disk, modulo
the `H`-world's hash collision resistance. The total information
content of the system is conserved across the hot↔cold boundary; only
the *kinetic* representation (in-RAM) is swapped for the *potential*
representation (on-disk).

Hamiltonian mechanics: `H(p, q) = T(p) + V(q)`, kinetic plus potential,
total energy conserved under the system's evolution. The metaphor
maps cleanly: `H(hot, cold) = |hot| + |cold|`, the total content
stays constant; eviction is a canonical transformation from kinetic
(hot, fast-access, RAM) to potential (cold, content-addressed, disk);
rehydration is the inverse transformation. No information is created
or destroyed; only its representation moves between conjugate
coordinates.

Where the metaphor stops: real Hamiltonian systems are continuous and
time-reversible; the scheduler's evolution is discrete (per-tick) and
strategically biased (Fate selects to *minimise* pressure, not to
preserve a specific energy level). The conservation law that holds
is *content* conservation, not *energy* conservation. But the
name names what's conserved, and the discipline names how.

### 1.3 The original spectral-db `Scheduler` (the metronome)

The pre-Prism-Scheduler implementation in spectral-db was a metronome:
adaptive-interval timer that always ran the same crystallize + pressure
cycle. It worked for the cases it worked for; it didn't *observe* and
it didn't *decide*. The metronome is the floor case of the
HamiltonScheduler — a degenerate Fate that always returns
`Model::Cartographer`. The new scheduler preserves the metronome's
adaptive-interval logic (§3.7) and lifts the strategy choice into
Fate.

The lineage carries because the metronome named two things the new
scheduler keeps:

- The tick is the unit of evolution. The scheduler does not
  continuously observe; it observes at discrete points.
- The interval is part of the state. Faster ticks for active subtrees,
  slower ticks for settled ones. This is the *time* axis of the
  scheduler's evolution.

The new scheduler adds the *strategy* axis on top.

### 1.4 The Prism-Scheduler plan (spectral-db, 2026-04-05)

The direct ancestor. The design that becomes the new HamiltonScheduler
verbatim, with one altitude lift: from "spectral-db graph state" to
"any content-addressed store state". The 16 features, the
`GraphObservation` newtype, the four strategies, the Fate dispatch,
the `ScheduleAction` enumeration — all preserved.

The plan named the design but lived in the wrong crate. The
Hamilton name predates the Prism rename in spectral-db; under the
directive "we honor the lineage," the migration keeps the name
`HamiltonScheduler` (the Hamilton name carries the
energy-conserving-evolution discipline) and lets the *engine-altitude*
"Prism" framing dissolve into the substrate's natural vocabulary —
the Five Operations are already present at every altitude; calling the
scheduler "Prism" would be naming the medium when the message is
Hamilton.

From the Prism-Scheduler plan, §0: *"The current scheduler is a
metronome — adaptive interval timer that always runs the same
crystallize+pressure cycle. The new `PrismScheduler` extracts 16
spectral features from the graph into a `GraphObservation`, feeds them
to Fate (425-param model selector), and dispatches to one of four
concrete strategies: Abyss (observe-only), Pathfinder (precision-cut
crystallization), Cartographer (full evolution+crystallize+pressure),
Explorer (partition healing)."*

This quote stands. Replace `PrismScheduler` with `HamiltonScheduler`,
lift `graph` to `store`, and the design is what fragmentation gets.

---

## 2. The reproducibility chain — what this spec closes

The nine-claim chain in [[kintsugi-thesis]] §2 names which claims are
landed (✅), partial (⚠️), or owed (❌). This spec closes two and
partially closes one.

### 2.1 C8 (DAG traversal stable) — ❌ → ✅

From [[kintsugi-thesis]] §C8:

> `Crystallizations<H>` uses a `HashMap<Ref, Body<H>>` internally
> (`bootstrap/src/crystallize.rs:434`). Hash-map iteration order is
> not deterministic across runs. As long as dispatch is by lookup
> (not iteration), this does not affect the verdict; but any future
> addition that iterates the registry (e.g. "list all registered
> refs", "audit registrations") would surface non-determinism.

The thesis names a `BTreeMap<Ref, Body<H>>` fix as sufficient. This
spec goes further: the `HashMap` migrates to `FrgmntStore<Body<H>>`
with a deterministic iteration discipline (§5.3), and the
HamiltonScheduler governs hot/cold transitions. The deterministic-order
property is preserved by content-addressed keys with a stable
byte-order sort; the disk-spillover property is new and necessary for
the build-graph altitude (Tick C of [[kintsugi-minimum-runnable]]),
where the registry will grow beyond what fits in RAM.

### 2.2 C7 (Property checks deterministic) — ⚠️ → ✅

From [[kintsugi-thesis]] §C7:

> The property bodies are `\` (parked). The discharge happens in Rust
> today (`bootstrap/src/main.rs::count_dark` for `total_classification`;
> the bootstrap's hash-coincidence check for `coincidence_matches`;
> the `@mirror/grammar.is_mirror` ref check for `glass_wall`). The Rust
> bodies have been audited (no HashMap-in-loop, no clock reads, no PID
> reads); the audit is by reading, not by property check.

The `Pure` trait (§4) converts the audit into a type-system invariant.
A Rust body that satisfies the property must impl `Pure`; the
impl-bound is the substrate's way of saying "this body has been
verified by the compiler not to read the clock, the env, or any
global state." The verification is by trait coherence and
compile-time discipline; the cost is a marker trait and the
discipline to add it where the substrate currently has audit comments.

### 2.3 C9 (@io boundary discipline) — ❌ → ⚠️

From [[kintsugi-thesis]] §C9:

> The `@io` wrappers exist; the determinism-flag declaration on them
> does not. This is the largest single piece of yet-to-do work in the
> chain; for v1.0 (mirror's bootstrap is per-machine) it may be
> acceptable as-is, with the cross-machine claim deferred to v1.x.

The `Pure` trait gives the compile-time half: `@io`-wrapped bodies
cannot impl `Pure` (the trait is structurally about pure functions of
the input; an `@io` body has effects observable beyond the input).
The `requires deterministic(...)` declaration on `@io` wrappers — the
full determinism-flag discipline — remains owed. But the
*type-level distinction* between pure bodies and @io bodies lands
with this spec.

### 2.4 What this spec does NOT close

Honest accounting:

- C3 / C4 / C5 (model OID pinning, seed-pinning, au-cache key
  composition) — these are Fate-side work. The HamiltonScheduler
  *uses* Fate (the 425-param model selector); making Fate itself
  deterministic is the work in [[kintsugi-thesis]] §3, not here.
- C9 fully — the determinism-flag declaration on `@io` wrappers
  needs its own substrate change (`requires deterministic(...)`
  on `@io` action signatures); see §9.2.
- Cross-machine toolchain reproducibility — Nix-flake-shape work,
  deferred to v1.x per the thesis.

The scheduler's own determinism is not free: Fate must be
deterministic, the strategy dispatch must be deterministic, the
iteration order over the store must be deterministic. §3.3 names
each.

---

## 3. The HamiltonScheduler — definition

The migration from the Prism-Scheduler plan to fragmentation's
HamiltonScheduler is structural, not creative. The shape, the
features, the strategies, the dispatch — all preserved. What changes
is the altitude: from "spectral-db's graph state" to "any
content-addressed store's state".

### 3.1 The `Scheduler` trait — abstract interface

Home: `fragmentation::scheduler::Scheduler`. Reason: the trait is
substrate-management vocabulary; it belongs with the store types it
manages. The trait is generic over the `Fragmentable + Clone` type
the scheduler is supervising:

```rust
//! Scheduler trait — abstract interface for content-addressed store
//! management. The default impl (`MetronomeScheduler`) ships with
//! fragmentation; the smart impl (`HamiltonScheduler`) is the
//! Fate-driven default for consumers that pull the `fate` feature.

pub trait Scheduler<N: Fragmentable + Clone> {
    /// Observation type — what the scheduler sees on each tick.
    /// Default: GraphObservation (16 features, all in [0, 1]).
    type Observation;

    /// Strategy type — what the scheduler chose to do this tick.
    /// Default: Strategy (the four-variant enum).
    type Strategy;

    /// Tick — one round of: observe, decide, act.
    /// Returns the tick result, including which strategy ran.
    fn tick(&mut self, store: &FrgmntStore<N>) -> TickResult<Self::Strategy>;

    /// Observe — extract the current observation without acting.
    /// Pure function of the store state at tick time.
    fn observe(&self, store: &FrgmntStore<N>) -> Self::Observation;

    /// Decide — given an observation, pick a strategy. Pure function;
    /// must be deterministic per [[kintsugi-thesis]] §C4.
    fn decide(&self, obs: &Self::Observation) -> Self::Strategy;

    /// Execute — run the chosen strategy against the store. Effects
    /// land in the store; the return carries the metrics.
    fn execute(&mut self, strategy: &Self::Strategy, store: &FrgmntStore<N>)
        -> StrategyMetrics;
}
```

The trait separates *observe / decide / execute* so each piece is
testable in isolation and the determinism property is enforceable
on `decide` independently of the cost of `observe` and `execute`.
This is the same separation the prism-scheduler plan named in §0;
the altitude lift makes it explicit at the trait level.

Newtypes (per the no-bare-types discipline, [[feedback-no-bare-types]]):

```rust
/// Tick result — what happened on one round.
pub struct TickResult<S> {
    pub tick: TickNumber,
    pub interval: TickInterval,
    pub strategy: S,
    pub metrics: StrategyMetrics,
    pub convergence: Convergence,
}

/// Per-strategy execution metrics.
pub struct StrategyMetrics {
    pub items_crystallized: CrystalCount,
    pub items_evicted: EvictionCount,
    pub bytes_released: ByteCount,
    pub partition_events: PartitionEventCount,
}

pub struct TickNumber(u64);
pub struct TickInterval(std::time::Duration);
pub struct CrystalCount(usize);
pub struct EvictionCount(usize);
pub struct ByteCount(usize);
pub struct PartitionEventCount(usize);
```

### 3.2 `GraphObservation` — the 16 features

The Prism-Scheduler plan's Task 2 names these verbatim. Preserved
with the altitude lift; the features that referenced spectral-db
specifics are renamed to fragmentation-altitude observables:

| Index | Feature | Source at fragmentation altitude |
|-------|---------|----------------------------------|
| 0 | `convergence_settled` | 1.0 if the store's content-OID-set unchanged since last tick, else 0.0 |
| 1 | `pressure_load` | `BoundedStore::total_bytes() / BoundedStore::capacity()` |
| 2 | `entry_occupancy` | `cached_len() / nominal_capacity` |
| 3 | `branch_density` | average children-count across cached fragments (clamped to [0, 1] via a normalisation factor) |
| 4 | `crystal_fraction` | crystallized entries / total cached entries (a crystallized entry is one marked immutable; §3.6) |
| 5 | `settlement_depth` | settled_ticks / 10, clamped to [0, 1] |
| 6 | `interval_ratio` | (current_interval - min) / (max - min) |
| 7 | `hot_path_density` | recently-accessed entries / total cached entries |
| 8 | `read_intensity` | reads-since-last-tick / nominal_read_rate, clamped |
| 9 | `partition_risk` | 1.0 if any disk-fallback failed this tick, 0.0 if all reads from cache, 0.5 if mixed |
| 10 | `tick_maturity` | min(tick_count / 100, 1.0) |
| 11 | `mutation_rate` | inserts-this-tick / max(cached_len, 1) |
| 12 | `loss_rate` | accumulated `StoreLoss` magnitude this tick, clamped |
| 13 | `was_pressured` | 1.0 if eviction-to-disk fired this tick, else 0.0 |
| 14 | `evolution_active` | 1.0 if an evolution hook is set, else 0.0 |
| 15 | `first_tick` | 1.0 if tick_count == 0, else 0.0 |

For the consumer that wants spectral-db's exact feature set
(eigenvalue-aware features), spectral-db provides its own
`GraphObservation` impl over the same scheduler trait. The default
`GraphObservation` in fragmentation is the substrate-general version;
spectral-db's is the engine-specific version. Both feed the same
Fate; the feature semantics are stable across consumers because
Fate's training signal is the *next-tick utility*, not
feature-by-feature meaning.

Newtype:

```rust
/// 16 features extracted from store state. All values in [0, 1].
/// Newtype over `[f64; 16]` to keep bare float arrays out of the
/// scheduler surface (no-bare-types, [[feedback-no-bare-types]]).
pub struct GraphObservation {
    features: [Probability; 16],
}

/// Newtype over f64 constrained to [0, 1]. Construction enforces
/// the bound; reads expose the underlying f64.
pub struct Probability(f64);

impl Probability {
    pub fn new(value: f64) -> Self { Probability(value.clamp(0.0, 1.0)) }
    pub fn as_f64(&self) -> f64 { self.0 }
}
```

### 3.3 Fate — the gold/au binding for strategy selection

Fate is the 425-parameter model selector that maps `GraphObservation`
to one of `{Abyss, Pathfinder, Cartographer, Explorer}`. The au
binding is the existing one from `boot/std/fate.mirror` —
`io tick(features) => imperfect` with the elite(1).beam(8).halving(3)
tournament for inference.

Fate's determinism is the load-bearing claim. From
[[kintsugi-thesis]] §C4: Fate's inference must be a pure function of
`(model_oid, features, temperature, seed, sampling_policy_oid)`. The
HamiltonScheduler holds Fate as a non-substrate dependency — Fate's
output is the substrate's input to strategy selection. The chain
is:

```
  store state
    │
    │  observe (pure, fragmentation-side)
    ▼
  GraphObservation
    │
    │  Fate.tick(features) — pure if @fate is pure (Claim 4)
    ▼
  Model { Abyss | Pathfinder | Cartographer | Explorer | Fate }
    │
    │  strategy::plan_for (pure, fragmentation-side)
    ▼
  StrategyPlan { actions: [ScheduleAction] }
    │
    │  execute (effectful, fragmentation-side)
    ▼
  store mutations + TickResult
```

Three pure steps (observe, Fate, plan_for) and one effectful step
(execute). The first three compose to a pure function from `store
state` to `StrategyPlan`. The execute step is where the substrate
actually mutates the store; its effects are bounded by the strategy's
action list.

The scheduler does *not* invoke Fate directly through a non-mirror
path; it goes through the substrate-declared `@fate.tick` action.
This means the Fate dependency is a substrate dependency, not a Rust
dependency. Fragmentation declares `requires deterministic(@fate.tick)`
as a substrate property; if @fate cannot satisfy it, the scheduler
falls back to the metronome (constant Cartographer; the
pre-Prism-Scheduler behaviour). This is the substrate-pull discipline
applied to the scheduler: the *binding* lives at the boundary; the
*capability* lives in the substrate.

### 3.4 Where the open/closed boundary sits

The HamiltonScheduler straddles the open foundation / closed engine
boundary from [[store-vs-db-and-the-cascade]] §1.

**Open (fragmentation, Apache-2.0):**

- The `Scheduler` trait surface.
- `GraphObservation` (the 16-feature payload).
- The four `Strategy` enum variants and their `ScheduleAction` plans.
- The `MetronomeScheduler` default impl (constant `Cartographer`;
  no Fate; preserves the pre-2026-04-05 spectral-db behaviour).
- The `HamiltonScheduler` struct shape + the wiring of
  `observe → Fate → plan_for → execute`.

**Closed engine territory (where it could live; not in fragmentation):**

- The Fate model weights (the 425 parameters that train the
  strategy-selection function). The weights' content-addressed OID
  is open (anyone can compute it); the *training* of those weights is
  the engine work that a commercial offering could refine.
- Alternative `Scheduler` implementations with smarter heuristics
  (e.g. RL-trained schedulers, multi-armed-bandit schedulers,
  spectrum-aware schedulers).
- Engine-specific `GraphObservation` impls (spectral-db's
  eigenvalue-feature-augmented version is one example; future
  consumers might add language-model-derived features).

The boundary is at *which Fate model* and *which Scheduler impl* the
consumer wires up. Fragmentation ships the trait and one open impl
that works; spectral-db's plan #2026-04-05 work becomes the
fragmentation `HamiltonScheduler` impl that uses the Fate substrate
(open inputs) with whatever weights the consumer provides (open or
closed). The substrate-side declarations (`@fate.tick`, `requires
deterministic`, the `@scheduler/strategy/*` namespace) are open.

### 3.5 The four strategies — kintsugi-as-build-system framing

The Prism-Scheduler plan's Task 3 names the strategies as `Abyss`,
`Pathfinder`, `Cartographer`, `Explorer`. Preserved verbatim; the
build-system framing adds a second reading of each, applicable when
the store being scheduled is the build graph (the
`Crystallizations<H>` table at the kintsugi altitude) rather than the
spectral graph.

**Abyss — observe only.**

*Spectral-db reading.* Compute coordinates, check convergence. No
mutations. "The Abyss in mirror is the classifier — it names which
optic to apply" (from [[cartographer-design]] §The model).

*Build-graph reading.* Walk the registered crystallizations, check
which are fresh (their content-OID matches what's on disk in
`.frgmnt/objects/`), report. No rebuilds. The audit pass that
[[kintsugi-thesis]] §C7 names as the current property-check
discharge: Abyss IS that pass at the scheduler altitude. When Fate
selects Abyss, the scheduler is saying "the right move is to look,
not to act."

`ScheduleAction` plan: `[ComputeCoords, CheckPartitions]`. Lifted
to the fragmentation altitude: `[Audit, CheckIntegrity]` — `Audit`
is the analogue of `ComputeCoords` (read but don't mutate);
`CheckIntegrity` is the analogue of `CheckPartitions` (validate that
disk-spillover entries can be re-read).

**Pathfinder — precision cut.**

*Spectral-db reading.* Only crystallize hot paths above threshold.
The minimal action that moves the system toward settled state.

*Build-graph reading.* Crystallize the build targets whose inputs are
all present and whose outputs aren't yet on disk. Don't try to evolve
the build; don't try to release pressure. Just *make* the things
that are ready to be made. The minimum-runnable build tick.

`ScheduleAction` plan: `[Crystallize, ExportState]`. Build-graph:
`[BuildReadyTargets, PersistArtifacts]`.

**Cartographer — full evolution + crystallize + pressure.**

*Spectral-db reading.* The default; Beer's nominal-operations
response. Evolution hook runs, crystallization runs, pressure check
runs. "The cartographer discovers what the hardware can hold"
(from [[cartographer-design]] §Precision = budget).

*Build-graph reading.* Run the full kintsugi tick: enumerate fractures,
measure loss, elect candidates, apply, check fixed-point. This is
the build-system's *full pass*. When Fate selects Cartographer, the
scheduler is saying "do the normal thing; nothing special is
required."

`ScheduleAction` plan: `[Evolve, Crystallize, PressureCheck, ExportState]`.
Build-graph: `[KintsugiTick, CrystallizeArtifacts, ReleasePressure, PersistState]`.

**Explorer — boundary recovery.**

*Spectral-db reading.* Heal partitions, check pruned edges. The
algedonic response to a fragmenting graph.

*Build-graph reading.* The build graph has fragmented — a
crystallization's body returned `Failure(Uncrystallized)` (a
substrate ref the floor doesn't realise), or a `.frgmnt/objects/`
read failed (the disk-spillover lost a write). Explorer's job is to
recover: re-register failed bodies from a backup, refetch corrupted
objects from upstream, repartition the store so the failed region is
isolated and the rest stays building.

`ScheduleAction` plan: `[Evolve, CheckPartitions, ComputeCoords]`.
Build-graph: `[RecoverFailedBodies, RepartitionStore, Audit]`.

The four strategies form a closed sum type — there is no
"strategy 5" that the scheduler might invent. Fate's domain is
exactly these four (plus `Fate` itself, which is the selector and
never selects itself in production; see
[[../specs/2026-04-05-prism-scheduler]] Task 3's
`fate_is_empty_plan` test).

### 3.6 Crystallization at the scheduler altitude

A *crystallized entry* in the fragmentation-altitude scheduler is one
marked immutable — its content-OID has stabilised across N ticks (per
the Crystallizer discipline from
[[cartographer-design]] §Stop condition = crystallization). The
scheduler treats crystallized entries differently:

- They survive pressure-driven eviction (crystallization confers
  hot-cache priority).
- They are candidates for serialization to `.frgmnt/objects/` only as
  cold backup; the canonical home stays in-RAM until pressure forces
  the move.
- They contribute to `crystal_fraction` (feature 4 of
  GraphObservation).

The Crystallizer impl is per-consumer: spectral-db's notion of
"settled eigenvectors" is one; the build graph's notion of
"settled build target" is another. The scheduler exposes a hook
(`is_crystallized: fn(&N) -> bool`) and otherwise stays out of the
decision. Default impl: never (everything is mutable; no
crystallization tracking).

### 3.7 Adaptive tick interval (from the metronome)

The pre-Prism-Scheduler metronome adapts its tick interval based on
recent activity: more mutations → shorter interval; more settled
ticks → longer interval. The HamiltonScheduler preserves this. The
adaptive logic lives at the scheduler altitude (not in Fate); Fate
picks *what* to do this tick, the interval logic decides *when* the
next tick fires.

```rust
/// Adaptive tick interval, bounded by min and max.
pub struct AdaptiveInterval {
    current: TickInterval,
    min: TickInterval,
    max: TickInterval,
}

impl AdaptiveInterval {
    /// Adjust interval based on the previous tick's result.
    /// Settled → grow toward max; mutation → shrink toward min.
    pub fn adjust(&mut self, prev: &TickResult<Strategy>) { /* ... */ }
}
```

---

## 4. The `Pure` trait — compile-time determinism

### 4.1 What Pure is

A marker trait for bodies (and functions in general) that satisfy:

1. **Same input → same output.** No clock reads, no
   `SystemTime::now()`, no env reads, no thread-local reads, no
   global state reads, no `std::env::var`, no atomic reads of
   anything mutated outside the function.
2. **No side effects observable from outside.** No file writes, no
   network calls, no logging, no atomic writes, no calls to other
   non-Pure functions.
3. **No randomness.** No `rand::thread_rng`, no
   `getrandom::getrandom`, no platform RNG access.
4. **Floating-point determinism by discipline.** Pure does not by
   itself rule out FP non-associativity, but `Pure` implies the
   substrate-side `requires deterministic(...)` declaration, which
   *does* rule out parallel-reduction kernels (per
   [[kintsugi-thesis]] §C4).

A `Pure` function is a *pure function in the type-theoretic sense*:
its behaviour is fully determined by its inputs.

### 4.2 Trait shape

```rust
//! Pure — marker trait for functions/bodies with compile-time
//! determinism guarantees. A Pure value is a value whose computation
//! depends only on its inputs.
//!
//! `Pure` is implemented by:
//! - Closures known at construction time to satisfy the discipline
//!   (the substrate's `requires deterministic(...)` declaration).
//! - Function pointers to functions whose bodies have been audited
//!   AND marked `Pure`.
//! - Built-in `Pure`-by-construction items (pure arithmetic,
//!   `BTreeMap` operations, content-addressed lookups).
//!
//! `Pure` is NOT implemented by:
//! - Closures that capture `SystemTime`, `Instant`, env vars, thread
//!   handles, or atomics-of-non-Pure-values.
//! - Functions that call `std::env::var`, `chrono::Local::now`,
//!   `rand::*`, or any `@io`-equivalent.
//! - Bodies that wrap external tool invocations without the
//!   `requires deterministic(...)` flag set.

/// Marker trait: this value's computation is a pure function of its
/// inputs. Compile-time determinism guarantee.
pub trait Pure {}
```

The trait has no methods. It carries no runtime cost. It is purely
a compile-time discipline marker.

### 4.3 How Pure integrates with `PropertyVerdict`

From `prism/imperfect/src/transparency.rs` (already landed): the
`PropertyVerdict` enum has three shapes: `Pass`, `Partial { confidence,
diagnostics }`, `Fail(Diagnostic)`. `Pure` maps cleanly:

- A body that impls `Pure` → `PropertyVerdict::Pass` for the `Pure`
  property at that body's substrate path.
- A body that does *not* impl `Pure`, where the substrate declared
  `requires Pure` → `PropertyVerdict::Fail(Diagnostic::new(
  "Pure marker missing; body may not be deterministic"))`.
- A body whose Pure-ness cannot be statically determined (e.g.
  generic functions where the Pure-ness depends on a type parameter
  whose constraints are not yet fixed) → `PropertyVerdict::Partial {
  confidence: 0.5, diagnostics: vec![Diagnostic::new("Pure depends
  on type parameter; cannot prove either way at this site")] }`.

This is the existing `Transparency<P>` seam — Pure adds one more
property to the algebra; the algebra absorbs it without change. The
diagnostic propagation, the `Fail` dominance, the `Partial` merging
are all unchanged.

### 4.4 Pure's home — `prism_core` not fragmentation

The tension: Pure could live in fragmentation (where it's *used* most,
as a constraint on `Body<H>`), or in `prism_core` (where its algebraic
family lives — `Loss`, `Transparency`, `PropertyVerdict`).

The choice: `prism_core::pure::Pure`.

Arguments for `prism_core`:

1. **Algebraic locality.** Pure is a property at the same altitude
   as Loss and Transparency. Both are marker-trait-shaped (one is
   an associated-type-defining trait, the other is a marker); both
   compose under monoid algebra; both have `PropertyVerdict`
   discharge. Placing Pure elsewhere fragments the property surface
   the substrate exports.
2. **Dependency direction.** `fragmentation` depends on
   `prism_core` already (`prism_bridge.rs` impls `prism_core::Store`
   for `FrgmntStore`). The reverse — `prism_core` depending on
   `fragmentation` — would invert the architecture and pull
   content-addressed storage primitives into the property layer.
   Pure-in-prism_core preserves the existing direction.
3. **Reusability.** Pure is useful beyond fragmentation. Spectral-db's
   loss functions want it; mirror's `@epistemologic/property/*`
   bodies want it; future consumers (the Surface model's translation
   function; the Shatter model's serialization function) want it.
   The home should be where the most general consumer lives.

Arguments against (Pure-in-fragmentation):

- The strongest constraint on the body — `Body<H>: Pure + Fn(...)` —
  is at the fragmentation altitude. Putting the trait where it's most
  used has its own ergonomics argument.
- Mirror's bootstrap depends on both crates already; either choice
  works for the bootstrap's needs.

Counter to the counter: ergonomics losses are small (one `use
prism_core::Pure` line in fragmentation); algebraic family losses
are large (Pure separated from PropertyVerdict's home is a
structural fracture).

The choice stands: **`prism_core::pure::Pure`**. Fragmentation depends
on it the same way it already depends on `prism_core::Loss` and
`prism_core::Store`.

Counter-argument worth recording: if `prism_core` becomes too
opinionated about substrate-pull (today it's relatively
property-focused), there's an argument for splitting `prism_core`
into `prism_core` (the optics + beams) and `prism_props` (the
property algebra including Pure). That refactor is bigger than this
spec; flagging it as a future-tick consideration.

### 4.5 Why marker trait, not effect system?

Mirror is sub-Turing. An effect system — Eff, Frank, Koka — would be
in scope but is heavy:

- The effect-system approach: `Body: Fn(...) -> Imperfect<...>`
  becomes `Body: Eff<Pure>` (the body's effects are bounded by a
  permission set; Pure is the empty permission set).
- The marker-trait approach: `Body: Pure + Fn(...)` (Pure is just a
  presence/absence marker).

Marker-trait cost: low. One trait, one impl per Pure body, audit
discipline for impls. The compiler enforces trait presence; the
audit discipline enforces that the impl is honest.

Effect-system cost: high. Effect rows, effect handlers, effect
inference, effect inheritance through generics, effect leakage through
closures, integration with `dyn Trait` (effect-system polymorphism
is its own research area). The cost is the wrong shape for mirror's
v1.0; future work could revisit.

The marker-trait approach delivers C7's compile-time check at a
cost that fits the substrate. The effect system would deliver more
(richer guarantees about *which* impurity a body has), but the
delta is not what kintsugi needs to close.

### 4.6 Pure and `@io`

A Body marked `Pure` cannot legally be an `@io` body. The substrate
enforces this through the `glass_wall` property check (per
[[kintsugi-thesis]] §C2): a body whose substrate declaration is `@io`
must *not* declare `requires Pure`. The substrate refuses such a
declaration at compile time.

For `@io` bodies that *are* deterministic (e.g. a `rustc` invocation
with pinned flags), the discipline is different:

```mirror
grammar @io/rustc {
  action build(source: text, target: path) -> imperfect(artifact, loss) {
    \  # parked at the @io boundary
  }
  requires deterministic(build, flags = {
    codegen_units: 1,
    source_date_epoch: 0,
    incremental: false,
  })
}
```

This is the [[kintsugi-thesis]] §C9 substrate-level change. The
`requires deterministic(@io_body, flags = {...})` declaration is
the `@io` analogue of the `Pure` marker: it doesn't claim the body
is pure (it can't; the body's literally an external tool), it claims
the body is deterministic *under the named flag set*. This is owed
work; the Pure trait lands the compile-time-marker half today and
leaves the `requires deterministic(@io)` half for a future tick.

---

## 5. `Crystallizations<H>.table` migration

The `HashMap<Ref, Body<H>>` at `mirror/bootstrap/src/crystallize.rs:434`
migrates to a fragmentation-backed content-addressed store with
HamiltonScheduler governance. The shape of the change:

### 5.1 New field type

Before:

```rust
pub struct Crystallizations<H: MerkleHash> {
    table: HashMap<Ref, Body<H>>,
    _h: PhantomData<fn(H) -> H>,
}
```

After:

```rust
pub struct Crystallizations<H: MerkleHash> {
    table: CrystallizationStore<H>,
    scheduler: Box<dyn Scheduler<BodyEntry<H>>>,
    _h: PhantomData<fn(H) -> H>,
}

/// Thin wrapper over `FrgmntStore<BodyEntry<H>>`. The store's
/// content-addressed keying maps directly to Ref byte order.
pub struct CrystallizationStore<H: MerkleHash> {
    inner: FrgmntStore<BodyEntry<H>>,
}

/// One (Ref, Body<H>) pair as a Fragmentable entry. The Ref's bytes
/// are the content key; the Body is the carried data. BodyEntry is
/// what the scheduler observes and what the store hot/cold-manages.
pub struct BodyEntry<H: MerkleHash> {
    pub path: Ref,
    pub body: Body<H>,
}
```

The `Body<H>` itself doesn't gain Pure as a hard requirement *yet*
(see §6.1 — the Pure constraint lands in a follow-on tick). The
field type change is independent of the Pure constraint.

### 5.2 Content-addressed keying via Ref

`Ref` already has a deterministic byte representation (the
`@`-prefixed nav-ref string; see
[[store-vs-db-and-the-cascade]] §2.3 for the rename). Its bytes ARE
its OID for keying purposes; no separate hash computation is needed
for the `Crystallizations<H>` table's key space.

The FrgmntStore's `insert(key: String, value: N, size_bytes: usize)`
takes a string key — `Ref::as_str().to_string()` is the canonical
mapping. The substrate-level `Ref`-Ord becomes the iteration order;
the BTreeMap-internal-to-BoundedStore (via the LIFO eviction tracking)
is re-keyed by Ref-byte-order at iteration time.

### 5.3 Iteration determinism — load-bearing

The new contract on `Crystallizations<H>`:

```rust
impl<H: MerkleHash> Crystallizations<H> {
    /// Iterate registered bodies in deterministic Ref-byte order.
    /// Same registrations, same iteration order, every time, every
    /// machine. This is the C8 invariant from [[kintsugi-thesis]].
    pub fn iter_deterministic(&self) -> impl Iterator<Item = (&Ref, &Body<H>)> {
        // Implementation: collect cache keys, sort by Ref byte order,
        // return refs through the sorted vector. Disk-spillover
        // entries are rehydrated lazily; the sort happens over the
        // full key set, not just the in-RAM subset.
    }
}
```

The sort order is a substrate-level commitment, documented as
load-bearing — analogous to the comment on `Content::Record`'s
`BTreeMap` ("sort order is part of the OID definition"). Any
future change to `Ref`'s string representation must preserve the
iteration-order contract.

### 5.4 Disk spillover semantics

From `frgmnt_store.rs` (already landed): `FrgmntStore<N: Fragmentable
+ Clone>` has two modes.

- **In-memory mode** (the base trait bound): bounded cache, eviction
  drops.
- **Persistent mode** (`N: Reconstructable + Clone`): bounded cache +
  `.frgmnt/objects/` disk spillover. Evicted entries persist; cache
  misses fall back to disk reads.

`BodyEntry<H>` must impl `Reconstructable` for the persistent mode
to apply. The body itself (`Arc<dyn Fn(...)>`) is not directly
serialisable; the persistent mode therefore stores a *body reference*
(the substrate ref + the body's content-OID computed at registration
time) rather than the body bytes. On disk-read, the reference is
resolved against an in-RAM body-registry that ships with the bootstrap.

This is the deep problem from §6.2 ("Where does the Pure body's OID
get computed?"). The migration honors it by deferring: the in-RAM
body-registry is a known limitation, audited as cross-process
non-reproducibility, named in the open questions.

### 5.5 Scheduler hook — when does hot↔cold fire?

The HamiltonScheduler's `execute(strategy, store)` step calls into the
store's eviction/promotion API based on the chosen strategy:

- **Cartographer / Pathfinder**: `flush()` (write all hot entries to
  disk; keep the most recently accessed in RAM).
- **Abyss**: read-only; no eviction.
- **Explorer**: targeted re-promotion (entries near the partitioned
  region get reloaded from disk to allow inspection).

The scheduler does NOT directly call BoundedStore's internal
eviction; it goes through the FrgmntStore facade. This preserves the
invariant that BoundedStore is the byte-bounded primitive and
FrgmntStore is the disk-aware layer; the scheduler is one altitude
up from both.

---

## 6. Migration plan — hard cutover, no compat shim

Per [[feedback-no-compat-shim]] (pre-v0.1 = no backward compat),
the migration is a hard cutover. Existing `Crystallizations<H>` users
are:

- The bootstrap's `floor_crystallizations::<H>()` (currently returns
  `Crystallizations::new()`; trivial change to thread a scheduler
  in).
- Tests in `bootstrap/src/crystallize.rs::tests::*` (single-tick
  dispatch with no scheduler concern; will need a `Scheduler` either
  injected or defaulted to `MetronomeScheduler`).
- Future consumers: spectral-db's planned migration to consume
  fragmentation-side scheduling rather than its own.

The migration order:

1. **Trait + types in fragmentation.** Land `Scheduler`,
   `GraphObservation`, `Strategy`, `MetronomeScheduler`,
   `HamiltonScheduler`, `CrystallizationStore`. New code; no
   existing surface broken.
2. **Pure in prism_core.** Land the `Pure` trait, an empty marker.
   No callers yet; constraint propagation happens in §6.3.
3. **Migrate `Crystallizations<H>`.** Replace the `HashMap` field;
   add the scheduler field; route `crystallize`/`register` through
   the new store; preserve the public API of `crystallize`,
   `register`, `knows`, `new` (`new` now constructs a
   `MetronomeScheduler` by default).
4. **Add Pure constraint.** `Body<H>` becomes `Pure + Fn(Optic<(),
   Splinter<H>>) -> Imperfect<...>`. This breaks all existing body
   constructions until they add `impl Pure for <closure type>` or
   the body-construction macro is updated.
5. **Wire scheduler through bootstrap.** Bootstrap's main loop calls
   `crystallizations.tick()` periodically; the scheduler observes
   and decides.
6. **Audit + tests.** All existing tests that exercise iteration
   order across the registry get updated to expect deterministic
   Ref-byte-order. New tests cover the scheduler's tick semantics.

No shim. No deprecation. The pre-cascade `HashMap<Ref, Body<H>>` is
gone after step 3.

---

## 7. What this assembly looks like — the substrate-pull markers

Each piece carries `[substrate-pull:realize]` per AGENTS.md
§"Boundary Rust is not frozen capability":

| Piece | Lives in | Marker |
|---|---|---|
| `Scheduler` trait | `fragmentation/src/scheduler.rs` | `[substrate-pull:realize]` — trait surface only, no capability |
| `GraphObservation` | `fragmentation/src/scheduler.rs` | `[substrate-pull:realize]` — pure observation, no capability |
| `Strategy` + `ScheduleAction` | `fragmentation/src/scheduler/strategy.rs` | `[substrate-pull:realize]` — closed sum type, no capability |
| `MetronomeScheduler` impl | `fragmentation/src/scheduler/metronome.rs` | `[substrate-pull:realize]` — deterministic dispatch, no I/O |
| `HamiltonScheduler` impl | `fragmentation/src/scheduler/hamilton.rs` | `[substrate-pull:realize]` — Fate dispatch (substrate-side), Rust binding (boundary-side) |
| `Pure` trait | `prism_core/src/pure.rs` | `[substrate-pull:realize]` — empty marker, no method, no capability |
| `CrystallizationStore<H>` | `mirror/bootstrap/src/crystallize.rs` | `[substrate-pull:realize]` — thin wrapper over `FrgmntStore<BodyEntry<H>>` |
| `BodyEntry<H>` | `mirror/bootstrap/src/crystallize.rs` | `[substrate-pull:realize]` — `(Ref, Body<H>)` pair, Fragmentable impl |

Nothing in this assembly carries capability beyond what already exists
in the substrate. The scheduler is binding (which strategy maps to
which plan) and observation (what does the store look like right now).
The capabilities — mutate the store, evict to disk, rehydrate from
disk — are FrgmntStore's, already audited.

---

## 8. The reproducibility chain — final scoring

Going back to [[kintsugi-thesis]] §3's chain table, with this spec's
deltas marked:

| Layer | Before | After this spec |
|---|---|---|
| Source bytes (content-addressed) | ✅ | ✅ |
| Hash function (BLAKE3, generic-over-H) | ✅ | ✅ |
| Substrate declarations (OID) | ✅ | ✅ |
| Fracture inputs (candidate OID) | ✅ | ✅ |
| Loss verdict composition (PropertyVerdict::merge_with) | ✅ | ✅ |
| Property check determinism | ⚠️ audit | ✅ by `Pure` trait |
| @fate model weights (store OID) | ✅ | ✅ |
| @fate cache key includes model OID | ⚠️ | ⚠️ (Fate-side work, not closed here) |
| @fate inference seed-pinned | ❌ | ❌ (Fate-side work, not closed here) |
| Au value cache key composition | ⚠️ | ⚠️ (depends on Fate work) |
| **Crystallizations iteration order** | ❌ HashMap | ✅ FrgmntStore + Ref-byte-order |
| @io tool wrapper determinism flags | ❌ | ⚠️ (Pure trait gives the non-@io half) |
| Cross-machine toolchain reproducibility | ❌ | ❌ (v1.x) |

Two claims close. One partially closes. Five remain owed; none of
those five is this spec's territory.

---

## 9. Open questions — what this spec defers

### 9.1 Fate's own determinism

The HamiltonScheduler's `decide` step calls `@fate.tick(features)`.
If Fate is not itself deterministic — same features, same model OID,
same seed → different strategy — then the scheduler is not
deterministic. The chain is:

```
  store state
    │
    │  observe (pure)
    ▼
  features
    │
    │  @fate.tick (pure IFF C3, C4, C5 all land)
    ▼
  strategy
```

Making Fate deterministic is [[kintsugi-thesis]] §C4's work — three
substrate changes (`temperature: f64`, `seed: u64`,
`sampling_policy_oid: oid` on `@fate.infer`'s signature). Until C4
lands, the scheduler's determinism is conditional on Fate's
determinism, and the scheduler must declare this conditional in
its substrate property declarations:

```mirror
grammar @scheduler/hamilton {
  requires deterministic(decide) when deterministic(@fate.tick)
}
```

This is the honest framing: the scheduler is structurally
deterministic; Fate is the load-bearing dependency for actual
determinism. C4 closes that dependency.

### 9.2 Pure body OID computation

`Body<H>: Pure + Fn(...)` constrains the body at the Rust type
level. But the body's *identity* — for caching, for cross-process
reproducibility, for the `Crystallizations<H>` table's content-
addressed lookup — needs an OID. Rust binary identity is not
content-addressed today. Two options:

- **Function-pointer-as-OID**. The function's machine-code bytes
  get hashed at registration. Doesn't survive recompilation; doesn't
  survive different optimization levels; doesn't survive different
  rustc versions. Per-machine OID at best.
- **Source-of-function-as-OID**. The function's substrate-side
  declaration's OID (the `@x/foo` ref) becomes the body's OID. Works
  for substrate-realized bodies (`@kintsugi/fracture/rename`'s body
  is identified by the substrate ref, not by the Rust
  implementation). Doesn't work for pure-Rust bodies that have no
  substrate ref.

The substrate-pull discipline says: every body should have a
substrate ref; pure-Rust bodies without substrate refs are an
anti-pattern. So option (b) is the path. But it requires that
every `Body<H>` registration goes through a substrate declaration —
which is the substrate-pull discipline applied to body registration.
That's another tick of work; named here, deferred.

### 9.3 Cross-language consumers — Gleam, BEAM

Fragmentation has a `gleam/` subdirectory (per
[[mirror-native-vcs]] §2). Future BEAM consumers (Reed's body in
`/Users/reed/body/`, Elixir-side `@mirror/tokenize` work per the
2026-05-21 commit) will want to consume the same content-addressed
store and possibly the same scheduler discipline.

The FFI shape for the scheduler is not designed here. Sketch:

- Gleam-side: a `Scheduler` opaque type with `observe / decide /
  execute` methods bridged through `Cnif` or a similar foreign-function
  shim. The 16-feature `GraphObservation` is a fixed-size record;
  the Strategy enum is a closed sum type with stable tag values.
- BEAM-side: similar, with a `gen_server`-shaped supervisor wrapping
  the scheduler. The tick interval becomes a `gen_server` timeout;
  the strategy execution becomes a series of message sends to the
  underlying store process.

Neither sketch lands here. Named as open work.

### 9.4 Counter to the `Pure` home choice

Per §4.4 — Pure is placed in `prism_core` over fragmentation. The
counter-argument is that `prism_core` is the optics-and-beams crate,
and it has been gathering property-shaped traits (`Loss`,
`Transparency`, now `Pure`) without an articulated theory of why
those belong with optics. A future refactor could split
`prism_core::pure` and `prism_core::props` into a dedicated property
crate; the placement of Pure assumes that refactor does not happen,
or that when it does, Pure moves with its family.

### 9.5 The Metronome's fallback path

If @fate is unavailable (not built, not registered, training
complete-but-uncrystallized), the HamiltonScheduler falls back to
`MetronomeScheduler` behaviour. The fallback is intentional but the
detection mechanism is not designed here — concretely, how does
the HamiltonScheduler know that Fate's `decide` call cannot
complete? Three options:

- Compile-time feature flag (`features = ["fate"]`).
- Runtime detection via `Crystallizations::knows(&Ref::new("@fate/tick"))`.
- Substrate-level declaration that the @fate grammar is or isn't
  registered at this consumer's altitude.

The substrate-level declaration is the architecturally honest one;
implementation details belong in the next spec.

---

## 10. What this spec is and isn't

**Is:** an architectural anchor. It names where the HamiltonScheduler
lives (fragmentation), where Pure lives (prism_core), what they
close in the reproducibility chain (C7, C8, partial C9), and which
open questions remain. It carries lineage explicitly — Beer, Hamilton,
the metronome, the Prism-Scheduler plan.

**Is not:** an implementation spec. The Rust does not land with this
commit. The next tick (forthcoming) lands the trait, the four impls,
the `Pure` marker, the `Crystallizations<H>` migration.

**Specifically refuses:**

- A rename of `HamiltonScheduler` to anything else. The Hamilton
  name predates the Prism-Scheduler plan and carries the
  energy-conserving-evolution discipline; honouring the lineage is
  load-bearing per the directive.
- A home for the scheduler that is not fragmentation. Spectral-db
  *consumes* the scheduler; it does not host it. The substrate
  management layer is fragmentation.
- A `Pure` trait that lives outside `prism_core`. The property
  algebra is `prism_core`'s; Pure joins the family.
- A compat shim for the `HashMap` → `FrgmntStore` migration. Per
  [[feedback-no-compat-shim]], hard cutover.

---

## 11. Cross-references

- [[mirror-native-vcs]] §1, §2 — the layering claim this spec extends.
- [[kintsugi-thesis]] §C7, §C8, §C9 — the reproducibility-chain
  claims this spec closes or partially closes.
- [[prior-art]] §1.7 (Nix), §1.8 (Cargo), §1.5 (Bazel) — the leaks
  the `Pure` trait + scheduler determinism address structurally.
- [[kintsugi-minimum-runnable]] — Tick A landed `Crystallizations<H>`;
  this spec migrates its table. Tick B doesn't need the scheduler;
  Tick C does.
- [[store-vs-db-and-the-cascade]] §1 — the open-foundation / closed-
  engine boundary the scheduler straddles.
- [[2026-04-05-prism-scheduler]] (spectral-db) — the design migrated
  here. Read in full for the per-task implementation detail; this
  spec lifts the altitude.
- [[cartographer-design]] — the `SpectralBudget` framing; the
  Cartographer strategy's lineage.
- [[2026-04-03-spectral-swap]] — "Hamilton priority" context.
- `~/.reed/visibility/protected/practice/insights/cybernetics/beer-error-propagation.md`
  — Beer's algedonic channel; the canonical reference for the
  scheduler's Cybernetic ancestry.
- [[transparency-as-vsm-audit-channel]] (forthcoming) — the deeper
  treatment of how mirror's Transparency carries Beer's VSM
  audit-channel discipline.
- [[feedback-no-compat-shim]] — the no-shim discipline.
- [[feedback-no-bare-types]] — the newtype discipline followed in
  the trait sketches.
- [[feedback-loss-from-epistemologic-properties]] — the broader
  property-grounded loss framing the scheduler integrates with.
- AGENTS.md (fragmentation) — "Boundary Rust is not frozen capability";
  the substrate-pull discipline the scheduler honours.

---

*The substrate that holds the world together is the one that knows
when to release its grip. Hamilton conserves the content; the
scheduler conserves the load; Pure conserves the determinism. One
name per altitude; one trait per property; one assembly that closes
two claims of the reproducibility chain and tells the truth about
the rest.*
