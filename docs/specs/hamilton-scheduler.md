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
- `fragmentation/src/lib.rs` — the module surface this spec extends.
  One new module (`scheduler`) plus the FrgmntStore mode-flag delta
  (§5.6). No `pure` module — Pure is an AST verdict, not a Rust
  marker (§4).
- `mirror/bootstrap/src/body.rs` (new) — the Body = prism + glass +
  AST restructure (§5.1). Replaces today's `Arc<dyn Fn(...)>`
  closure shape; the AST is what the analyses walk.
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
  `build.rs` leak. The Pure AST-analysis section (§4) names what
  each of those leaks looks like in mirror terms; the structural
  defense is `requires deterministic(...)` plus the `Pure` AST
  verdict.
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
  `Transparency`, `PropertyVerdict`, `Imperfect`. The `Pure` and
  `WcetBounded` named properties join this family as
  `PropertyVerdict`-shaped verdicts, NOT as new traits. Section
  §4 names the home and the analysis shape.
- `prism/imperfect/src/transparency.rs` — the `PropertyVerdict` /
  `Transparency<P>` algebra the AST-analysis verdicts integrate
  with. A Pure verdict is `PropertyVerdict::Pass` when the AST
  contains only pure-by-construction nodes, `Fail(Diagnostic)` when
  an impurity is located at a substrate path,
  `Partial { confidence, diagnostics }` when the analysis cannot
  classify a site. This is the existing seam; no new framework.
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
  from ⚠️ to ✅ via the `check_pure` AST analysis (§4.2). A
  `Body<H>` whose AST discharges Pure as `Pass` is by-construction
  deterministic; the verdict is content-addressable; the property
  check `requires deterministic(body)` becomes a substrate
  invariant rather than an audit pass.
- C9 of [[kintsugi-thesis]] ("@io boundary discipline") gets the
  AST-analysis half: `check_pure` mechanically detects `@io`-namespaced
  calls in a body's AST and emits a Fail verdict located at the call
  site. The substrate refuses to admit a Pure-required body whose
  AST contains `@io`.
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

3. **Property altitude.** Fragmentation owns the discipline that
   makes Rust-side determinism *verifiable from content*, not audited
   from convention. That's the AST-analysis verdict family (`Pure`,
   `WcetBounded`) carried by the existing `prism_core::PropertyVerdict`
   / `Transparency<Ref>` algebra. The `Body<H>` of a crystallization
   is structured (prism + glass + AST, §5.1) precisely so the AST
   IS what the analyses walk; the verdicts are content-addressable
   because the AST is content-addressable. Sub-Turing by composition
   with the substrate's own content discipline; no new Rust
   type-system features required. §4 names the home and the analysis
   shape.

4. **Realtime altitude.** Fragmentation owns the hard-realtime /
   soft-realtime contract over the content store — the discipline
   inherited line-for-line from Margaret Hamilton's Apollo executive
   (§1.2). The store carries a per-entry `RealtimeClass`; the
   dispatcher offers `crystallize` (soft) and `crystallize_bounded`
   (hard); the scheduler honours priority drops under overload. This
   is what makes the substrate viable for critical-industry consumers
   without forcing them to invent a bypass.

The consequences of the claim:

- The HamiltonScheduler lives in `fragmentation/src/scheduler.rs`,
  **not** in `spectral-db/src/scheduler.rs`. Spectral-db consumes the
  scheduler; it does not host it.
- Mirror consumes the scheduler via `prism_core` (which re-exports the
  fragmentation surface where it makes sense) and via direct
  fragmentation dependency where it doesn't. The `Crystallizations<H>`
  table at `bootstrap/src/crystallize.rs:434` migrates to a
  fragmentation-backed store with a scheduler hook; the `Body<H>`
  type at the same site restructures to a (prism, glass, AST) triple
  (§5.1).
- Property verdicts — `Pure`, `WcetBounded`, the family that grows
  next — are named in `prism_core` and computed by AST analyses
  (§4). No new Rust marker trait. The dependency direction is
  `fragmentation -> prism_core`, not the reverse. See §4.4.
- Future Rust-substrate primitives (the body system's interpreter;
  the cross-language FFI shape; the BEAM-side equivalents) belong
  in fragmentation unless they're property-shaped, in which case
  they belong in prism_core. The recurring decision lens:
  *substrate management* (memory, content, lifetime, ordering,
  realtime class) → fragmentation; *substrate property* (purity,
  transparency, loss, verdict, WCET bound) → prism_core.

The claim refuses three re-conflations the substrate has tried
before:

- **"The scheduler is engine-specific."** No. The scheduler is a
  management discipline over content-addressed entries with access
  patterns and realtime classes. Spectral-db is one consumer;
  mirror's crystallizations table is another; any future consumer
  with a hot/cold distinction is a third. The discipline is general;
  the strategies are general; the four-strategy taxonomy maps onto
  any content-addressed cache with eviction.
- **"Pure is a Rust marker on the body type."** No. Pure is a
  *property the body's AST satisfies* (or doesn't), produced by an
  AST analysis, carried as `PropertyVerdict::Pass | Partial | Fail`.
  The body's AST is content; the analysis is mechanical; the verdict
  is content-addressable. See §4.
- **"Realtime is the consumer's problem."** No. The substrate makes
  the contract first-class: hard-realtime entries pinned-resident,
  hard-realtime dispatch with deadline, drops surfaced as
  `Transparency<Ref>::Opaque({path → Fail(NotResident)})`. Critical-
  industry consumers depend on the substrate honouring this, not on
  themselves bypassing it (§1.5).

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

### 1.2 Margaret Hamilton — the woman who knew what to drop

**The HamiltonScheduler is named for Margaret Hamilton.** Not for
Hamiltonian mechanics — that is coincidence-of-name, addressed at the
end of this section. The real lineage is the woman who wrote a
system that knew what to drop under load, and flew it to the moon
on 74 kilobytes.

Margaret Hamilton (b. 1936) led the Software Engineering Division at
MIT's Charles Stark Draper Laboratory, where she ran the team that
built the Apollo onboard flight software (1961-). She coined
**"software engineering"** as a discipline — the term itself is
hers, asserted against an industry that did not yet consider software
an engineering practice. She was awarded the **Presidential Medal of
Freedom (2016)** by Barack Obama for that work. After Apollo she
founded Hamilton Technologies and developed the Universal Systems
Language (USL) — the less-cited follow-on that tried to formalise
what Apollo had taught her.

The load-bearing inheritance is the **1202 alarm**, Apollo 11, lunar
descent, 1969. The Lunar Module's executive overloaded — a
misconfigured rendezvous radar was flooding the computer with
unscheduled interrupts. The Apollo Guidance Computer had 64 KB of
rope ROM and 2 KB of erasable RAM. There was no room to fail
silently and no room to fail safely.

Hamilton's **priority-driven asynchronous executive** did the thing
that made the landing possible: it **dropped low-priority work**
(the radar updates) and **kept the landing-priority tasks running**
(the navigation, the throttle, the displays). It **announced the
drop** — that's what the 1202 alarm WAS, a structured diagnostic
surfaced to the crew, not a silent corruption. The astronauts saw
the alarm, looked at Mission Control, were told to proceed, and
landed. They trusted her code because her code had told them the
truth.

The shape Hamilton invented — and the shape the HamiltonScheduler
inherits, directly — is four-property:

1. **Bounded resources.** Memory, time, dispatch slots are finite
   and known. The scheduler operates within a stated bound; the
   bound is part of the contract, not an aspirational note.
2. **Priority discipline.** Work is ordered. High-priority work is
   not preempted by low-priority work, regardless of order of
   arrival or queue pressure.
3. **Graceful drop under overload.** When the bound is approached,
   low-priority work is dropped first. The scheduler chooses what
   to release; nothing is silently corrupted to make room.
4. **No silent failure.** Drops are surfaced as structured
   diagnostics — Hamilton's 1202; mirror's
   `PropertyVerdict::Partial { confidence, diagnostics }` or
   `Fail(Diagnostic)`. The substrate above the scheduler must be
   able to *see* what was dropped and decide what to do next.

These are not metaphor either. They map line-for-line to the
realtime-discipline §1.5 below, to the four `Strategy` variants in
§3.5 (Abyss is the algedonic-low pole; Explorer is the
algedonic-high pole; Pathfinder and Cartographer are the priority
gradient between), and to `Transparency<P>`'s
`Pass/Partial/Fail-with-diagnostics` shape carrying the discipline
through the type system.

Margaret Hamilton must be **cited by name in the eventual Rust
doc-comments** for `scheduler::hamilton`. The module-level rustdoc
block opens with her — name, work, the 1202 alarm, the four-property
shape — before it opens with type signatures. The lineage is
load-bearing; the substrate honours it by reading her name on the
way in.

#### 1.2.1 Coincidence-of-name — Hamiltonian mechanics

William Rowan Hamilton (1805–1865) gave the world
`H(p, q) = T(p) + V(q)` — kinetic plus potential, total energy
conserved under time evolution. There is a clean reading of the
scheduler's hot↔cold conservation in that frame: in-RAM
representation is the "kinetic" coordinate; on-disk
content-addressed bytes are the "potential" coordinate;
total content is conserved across the boundary. That reading is
mnemonically useful and intellectually fine.

It is **not** the source of the name. The HamiltonScheduler is
Margaret Hamilton's, not William Rowan Hamilton's. The mechanics
reading is a coincidence the substrate is happy to carry, but the
lineage section opens with the woman, not with the equation, because
that is the truth of where the discipline comes from.

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

### 1.5 Hard + soft realtime — the discipline made explicit

The content store gives **hard-realtime guarantees where possible**
and **soft-realtime guarantees everywhere else**. This is engineering
substance, not aesthetics: Alex's background includes realtime
computation; the substrate inherits the discipline from there as well
as from Apollo. Hard-realtime is non-optional for critical industries
— safety verdicts on the control loop of a medical device, a
fly-by-wire trim authority, an inverter's current-limit response
are not allowed to miss their deadline. The substrate must be able
to serve those consumers without making them invent their own
bypass.

**Hard-realtime contract** (the load-bearing subset):

- **WCET is bounded and known.** Worst-case execution time is a
  computable property of the body; the scheduler refuses to admit
  a hard-realtime body whose WCET is unbounded or unknown.
- **No disk I/O on the critical path.** A hard-realtime
  `crystallize_bounded` call MUST be served from memory-resident
  state. Cache miss returns a structured
  `Transparency::single(path, PropertyVerdict::Fail(
      Diagnostic::new("NotResident")))` verdict; it does NOT block
  on a disk read.
- **No unbounded allocations.** Allocation, if any, comes from a
  bounded pool sized at admission time.
- **No GC-like pauses.** Reference-counting decrements that could
  cascade into deallocation chains are deferred (epoch-style or
  explicit-queue) past the critical-path window.
- **Locks bounded or lock-free.** No mutex acquisition with
  unbounded waiters; lock-free queues where contention is plausible.
- **Dispatch latency measurable to a constant.** The scheduler's
  `decide` step on a hard-realtime tick has a bounded number of
  branches; the bound is part of the documented WCET.

**Soft-realtime contract** (the typical case):

- **Statistical guarantees.** "95% of ticks complete within X ms";
  exceedances are graceful, not catastrophic.
- **Graceful degradation.** Slow disk, full cache, contended lock
  — the substrate adapts; throughput drops; the call still
  eventually completes.
- **Typical-case fast, occasional disk OK.** Cache misses fall back
  to `.frgmnt/objects/` reads; the call returns the value, just
  slower.

The four substrate consequences this discipline introduces — each
elaborated in later sections:

1. **`FrgmntStore` carries a mode flag.** Hard-realtime entries are
   *pinned* memory-resident; soft-realtime entries get the existing
   disk-spillover behaviour. §5.6 names the type-level shape.
2. **Dispatcher offers two shapes.**
   `crystallize_bounded(ref, deadline)` for hard-realtime;
   `crystallize(ref)` for soft-realtime. The caller declares which
   contract applies; the substrate cannot guess. §5.7 names the
   signatures.
3. **WCET is a PropertyVerdict.** "This body's worst-case
   execution time is bounded by D" is a property the AST satisfies
   (or doesn't). It joins `Pure` in the `prism_core::PropertyVerdict`
   family. Hard-realtime admission requires
   `Pure ∧ WcetBounded(D)` to discharge as `Pass`. §4.7 names the
   verdict.
4. **HamiltonScheduler honours priorities under hard-realtime
   overload.** This is Margaret Hamilton's 1202 made literal: when
   the bound is approached, soft-realtime work is dropped first.
   Hard-realtime work either meets its budget OR returns
   `NotResident`-as-Fail with the budget exhausted; it never
   silently misses. §3.8 names the dispatch rule.

The HamiltonScheduler's lineage section is load-bearing precisely
because this is what the lineage is *for*. It is not
"scheduler with priorities" — it is bounded WCET, drop-under-load,
structured diagnostic on the drop, no silent failure. The four
properties from §1.2 map exactly:

| Hamilton's discipline (1969)   | Substrate realisation (today) |
|---|---|
| Bounded resources              | `BoundedStore<N>` byte budget + admission bounds |
| Priority discipline            | `Hard` work admitted ahead of `Soft` under pressure |
| Graceful drop under overload   | `Soft` work dropped first; `Hard` returns `NotResident` rather than block |
| No silent failure              | `Transparency<Ref>` with `Fail(Diagnostic)` payload — the 1202 made type-level |

#### 1.5.1 Hardware floors — the speed-of-light analogue

Hard-realtime is bounded *below* by what the local hardware allows.
The substrate cannot observe sub-nanosecond on a CPU whose TSC has
nanosecond granularity; cannot measure FP loss below machine epsilon;
cannot account for cache effects below cache-line granularity. The
hardware floor is the upper bound on observable precision — the
speed-of-light analogue. The HamiltonScheduler honours it by stating
its WCET claims *to* the hardware floor; cross-hardware claims are
weaker by the ratio of the two floors.

The measurement primitive that carries these claims into the
substrate — wall-clock time, FP precision loss, cache pressure,
budget-exhaustion-as-Fail — is `@mirror/lens/transit`. See
[[lens-transit]] for the full design; §8.1 below names the
integration point at the scheduler altitude.

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

### 3.8 Realtime class — the 1202 discipline made dispatch-level

The HamiltonScheduler honours hard-realtime priority at admission and
at overload. Two pieces of state make this dispatch-level:

**Per-entry realtime class.** Every entry in the
`CrystallizationStore<H>` (§5) carries a `RealtimeClass`:

```rust
/// Realtime contract a registered body declares. Pure newtype —
/// no bare booleans on the dispatcher surface (no-bare-types).
pub enum RealtimeClass {
    /// Hard-realtime. WCET bounded; admission requires
    /// Pure ∧ WcetBounded(D) (§4.7). Pinned memory-resident; no
    /// disk fallback on the critical path. Dropped LAST under
    /// pressure.
    Hard { deadline: TickInterval },
    /// Soft-realtime. Best-effort; disk fallback OK; dropped
    /// FIRST under pressure.
    Soft,
}
```

**Per-strategy drop policy.** Each `Strategy` declares which class
it drops under pressure:

| Strategy     | Soft drops? | Hard drops?                                             |
|--------------|-------------|----------------------------------------------------------|
| Abyss        | No (read)   | No (read)                                                |
| Pathfinder   | Eviction OK | Never — hard entries pinned                              |
| Cartographer | Eviction OK | Never — hard entries pinned                              |
| Explorer     | Eviction OK | Never — hard entries are the *last* thing to leave RAM   |

Under hard-realtime overload — the scheduler observes that admitting
the next hard-realtime call would exceed `BoundedStore::capacity()`
— the scheduler does what Margaret Hamilton's executive did at the
1202 alarm: it drops the lowest-priority work (soft-realtime
entries, in evict-LRU order) until the hard call fits, OR it
refuses admission with a structured
`Transparency::single(path, PropertyVerdict::Fail(
    Diagnostic::new("NotResident: hard-realtime capacity exhausted")))`
verdict. It never silently misses; it never blocks indefinitely;
it never falls back to disk.

This is what the Hamilton name names. The four strategies are the
shape of the dispatch; the realtime class is the shape of the
priority discipline; the verdict is the shape of the 1202.

---

## 4. `Pure` — the AST verdict, not the Rust marker

### 4.1 What Pure is (and what it stopped being)

Pure is a **`PropertyVerdict` that the body's AST satisfies, or
doesn't, under an analysis pass**. It is not a Rust marker trait.

The earlier version of this spec placed `Pure` as a `prism_core`
marker trait — `Body<H>: Pure + Fn(...)`. That homing was wrong
for an architectural reason that became clearer with the Body =
prism + glass + AST restructure (§5.1 below). When the body's
inner representation is an opaque Rust closure, a Rust-level marker
is the best the substrate can ask for. Once the body's inner
representation is the **AST**, the property check is an **AST
analysis**, and the verdict shape `prism_core` already exports —
`PropertyVerdict::Pass | Partial | Fail` — IS the right home for
the answer.

A body's AST satisfies Pure when:

1. **Same input → same output.** No AST node reads the clock, the
   environment, thread-local state, or any external mutable state.
   The analysis enumerates AST node kinds and verifies each is in
   the pure-by-construction set (arithmetic, content-addressed
   lookup, structural recursion over bounded data, calls to other
   `Pure`-verdicted bodies).
2. **No side effects observable from outside.** No file writes, no
   network calls, no `@io`-namespaced calls in the body's AST.
3. **No randomness.** No `@rand/*`, no platform-RNG-flavoured
   substrate refs.
4. **Floating-point determinism by discipline.** The AST does not
   contain parallel-reduction kernels whose FP order is
   non-deterministic; `requires deterministic(...)` declarations on
   any external substrate refs the AST calls are honoured.

### 4.2 The analysis pass

The Pure analysis is a function the substrate exposes:

```rust
//! Pure analysis — inspects a Body<H>'s AST and produces a
//! PropertyVerdict locating any impurity at its substrate path.

/// Substrate path the verdict is located at. Reuses the existing
/// `prism_core::Ref`-keyed Transparency<Ref> machinery.
pub type PurityVerdict = Transparency<Ref>;

/// Walk the AST; for each node, classify against the pure-by-
/// construction set; merge verdicts via PropertyVerdict::merge_with
/// at the substrate paths the impurities (if any) sit at.
pub fn check_pure<H: MerkleHash>(body: &Body<H>) -> PurityVerdict {
    body.ast.walk(|node, path| match classify(node) {
        PureKind::PureByConstruction => PropertyVerdict::Pass,
        PureKind::CallsExternal(reference) => verdict_for_reference(reference, path),
        PureKind::ReadsClock => PropertyVerdict::Fail(
            Diagnostic::new("Pure: AST reads clock at this site"),
        ),
        PureKind::UnknownAtThisAltitude => PropertyVerdict::Partial {
            confidence: confidence_bound(),
            diagnostics: vec![Diagnostic::new("Pure: site not classifiable")],
        },
        /* ... */
    })
}
```

The analysis is itself a `@mirror/lens/*` consumer — a lens over
the AST. The verdict is `Transparency<Ref>`-shaped: `Clear` when
the whole AST is pure-by-construction; `Opaque({path → verdict})`
when impurities exist, with each impurity located at its substrate
path. The merge law is the existing `PropertyVerdict::merge_with`
from `prism/imperfect/src/transparency.rs`; `Fail` dominates;
`Partial`s combine diagnostics; `Pass`es are neutral.

### 4.3 Why this is structurally cleaner than the marker trait

The marker-trait approach made three concessions the AST-verdict
approach does not have to make:

1. **The marker had no payload.** `impl Pure for FooBody {}` carries
   no diagnostic about *why* the body is pure or *where* it is
   impure. The verdict approach carries
   `Transparency<Ref>::Opaque({path → PropertyVerdict})` — located,
   structured, audit-channel-ready (per Beer's algedonic discipline,
   §1.1).
2. **The marker propagated through Rust generics; the AST is its
   own propagation.** Marker propagation — "impl Pure for X iff
   impl Pure for Y" — collides with `dyn Trait` and with closure
   types whose impl-blocks have to be hand-written. The AST walks
   itself; recursion through called bodies is a substrate-ref
   resolution, not a trait-coherence problem.
3. **The marker required audit discipline.** `impl Pure for FooBody {}`
   could be added to a body whose AST contains a clock read; the
   Rust compiler cannot tell. The AST-verdict approach **mechanically
   inspects** — the verdict is generated, not declared.

The verdict approach also collapses cleanly with the realtime work
(§1.5, §3.8): the WCET property check (§4.7) is the same shape,
the same algebra, the same verdict carrier. `Pure` and
`WcetBounded(D)` are two property analyses over the same AST;
admission to the hard-realtime class requires both to discharge as
`Pass` (or `Partial` with acceptable confidence).

### 4.4 The verdict's home — `prism_core::PropertyVerdict`, no new trait

The earlier draft homed `Pure` in `prism_core::pure::Pure` as a
marker trait. With the AST-verdict framing, Pure does not need a
new trait at all: it is a **named property** that the existing
`PropertyVerdict` algebra carries. The home is
`prism_core::PropertyVerdict` (already present), keyed at a
substrate path the analysis selects, e.g.
`@property/pure/<body-ref>`.

The earlier draft's argument for `prism_core` over fragmentation
(algebraic locality, dependency direction, reusability) all still
apply — they just apply to the *verdict carrier* (already in
`prism_core`) rather than to a new marker trait. The structural win
is larger: no new surface, no propagation through Rust generics, no
audit discipline for hand-written `impl Pure for Foo {}` blocks.
The verdict is computed from the AST; the AST is content; content
is content-addressable; the verdict is reproducible from the body's
OID alone.

Where the `Pure` *name* surfaces:

- `prism_core::properties::Pure` — a `pub const PURE: PropertyName`
  (or moral equivalent newtype) naming the property in the
  `Transparency<Ref>`-keyed map. Not a trait; a stable name the
  substrate and consumers agree on.
- `fragmentation::scheduler::admit` — admission for hard-realtime
  work consults `check_pure(body)` and `check_wcet(body, deadline)`,
  refuses admission unless both discharge as `Pass` (or `Partial`
  within a documented confidence bound).

Future refactor still worth recording: if `prism_core` becomes too
opinionated about substrate-pull, there's an argument for splitting
out a `prism_props` crate (the property algebra including `Pure`,
`WcetBounded`, and the analysis-pass framework). Bigger than this
spec; flagged.

### 4.5 Why an AST analysis, not an effect system?

Mirror is sub-Turing. An effect system — Eff, Frank, Koka — is in
scope but the cost is the wrong shape:

- The effect-system approach: `Body: Fn(...) -> Imperfect<...>`
  becomes `Body: Eff<Pure>` (the body's effects are bounded by a
  permission set; Pure is the empty permission set). Cost: effect
  rows, effect handlers, effect inference, effect inheritance
  through generics, integration with `dyn Trait`. Its own research
  area.
- The AST-analysis approach: `Body<H>` carries its AST; the analysis
  walks the AST. Cost: one analysis pass per property; reuses the
  existing `PropertyVerdict` + `Transparency<Ref>` algebra. No new
  type-system features.

The AST-analysis approach also has the property that effect systems
strain to deliver: **the verdict is content-addressable**. Same
AST → same verdict, by construction — because the AST has bytes,
the bytes have an OID, and the analysis is a pure function of the
OID. Effect-system verdicts attach to Rust types whose identity
shifts across compiler versions; AST-analysis verdicts attach to
content the substrate already content-addresses.

### 4.6 Pure and `@io`

A body whose AST contains `@io`-namespaced calls cannot discharge
Pure as `Pass`. The analysis detects the `@io` call site, locates
the impurity at its substrate path, and returns
`PropertyVerdict::Fail(Diagnostic::new("Pure: @io call at this
site"))`. The substrate's `glass_wall` property check
(per [[kintsugi-thesis]] §C2) consults this verdict.

For `@io` bodies that *are* deterministic under named flags (e.g. a
`rustc` invocation with `codegen_units = 1`, `incremental = false`,
`source_date_epoch = 0`), the discipline is different:

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
the `@io` analogue of the Pure verdict: it doesn't claim the body
is pure (it can't; the body's literally an external tool), it claims
the body is deterministic *under the named flag set*. The AST
analysis can read the declaration and downgrade `Fail(@io)` to
`Partial(@io, confidence = flag-coverage)` accordingly. Closing
§C9 fully remains owed work; the AST-analysis framing makes the
closure path cleaner than the marker-trait framing did.

### 4.7 `WcetBounded(D)` — the realtime sibling verdict

Hard-realtime admission (§1.5, §3.8) requires a second property
verdict alongside Pure:

```rust
/// Substrate path name for the WCET-bounded property.
pub const WCET_BOUNDED: PropertyName = /* ... */;

/// Verdict shape:
///   Pass        — AST has bounded WCET ≤ D under analysis.
///   Partial    — AST has bounded WCET probabilistically, with
///                 a confidence bound the analysis surfaces.
///   Fail       — AST has an unbounded loop, unbounded recursion,
///                 or a call to a body whose own WCET is unbounded.
pub fn check_wcet<H: MerkleHash>(body: &Body<H>, deadline: TickInterval)
    -> Transparency<Ref> { /* ... */ }
```

The analysis walks the AST and checks: bounded recursion, bounded
data, no unbounded loops, all called bodies themselves have
`WcetBounded` verdicts whose deadlines sum to within `D`. This is
a standard WCET analysis, lifted into the substrate's verdict
algebra rather than reinvented.

The hard-realtime admission rule:

```
admit_hard(body, deadline) := combine(
    check_pure(body),
    check_wcet(body, deadline),
) discharges as Pass-or-Partial-above-threshold
```

This is the literal substrate realisation of Margaret Hamilton's
admission discipline (§1.2): a hard-realtime task may run iff its
worst-case behaviour is bounded and its determinism is provable.
If either verdict fails, admission is refused; the caller sees a
structured `Transparency<Ref>` payload telling them which property
failed and where in the AST it failed.

The measurement primitive that *observes* whether actual execution
stays within the declared `WcetBounded(D)` claim is
[[lens-transit]]. The verdict declares the bound; transit measures
the reality; the substrate compares.

---

## 5. `Crystallizations<H>.table` migration

The `HashMap<Ref, Body<H>>` at `mirror/bootstrap/src/crystallize.rs:434`
migrates to a fragmentation-backed content-addressed store with
HamiltonScheduler governance. The shape of the change is larger than
the earlier draft acknowledged: `Body<H>` itself becomes structured.

### 5.1 Body becomes prism + glass + AST

Today: `Body<H> = Arc<dyn Fn(Optic<(), Splinter<H>>) -> Imperfect<...>>`
— an opaque, non-content-addressable Rust closure. The closure's
bytes do not exist in a stable sense; the closure's identity does
not survive recompilation. §9.2 of the earlier draft named this as
an open question. It is no longer an open question; it is solved
by restructuring `Body<H>`.

After the restructure, `Body<H>` carries the three pieces it always
should have: a **prism** (the five-operation lens), a **glass** (the
structural-edge construct — the 9-keyword floor that names what is
runnable through the substrate boundary), and an **AST** (the
body's content, read *through* the prism *through* the glass):

```rust
/// A realised substrate action body. Structured — NOT an opaque Rust
/// closure.
///
/// The body is the prism AND the glass: the prism through which the
/// AST ought to be read. The dispatcher's `crystallize()` step is
/// the act of interpreting the AST through the prism through the
/// glass.
///
/// Content-addressable by construction: the AST has bytes; the bytes
/// have an OID via BLAKE3 Merkle (§5.2); the body's identity IS its
/// content OID, recoverable across Rust recompilations, across
/// processes, across machines.
pub struct Body<H: MerkleHash> {
    /// The five-operation lens — focus / project / split / zoom /
    /// refract — selected for this body. Tells the interpreter HOW
    /// to traverse the AST.
    pub prism: Prism,
    /// The structural-edge construct: the 9-keyword floor that fixes
    /// which boundary-side operations the AST is allowed to invoke.
    /// The glass wall made transparent: the substrate can SEE through
    /// it because what's on the other side IS structured AST content.
    pub glass: Glass,
    /// The AST itself. Content-addressed; reproducible; analysable;
    /// the property-verdict carrier (§4) reads THIS to discharge
    /// Pure, WcetBounded, etc.
    pub ast: Ast<H>,
}

/// Newtype around the structured five-operation selector. No bare
/// enum on the body surface (no-bare-types).
pub struct Prism(/* ... */);

/// Newtype around the structural-edge construct — the 9-keyword
/// floor's boundary-side surface.
pub struct Glass(/* ... */);

/// AST with `H`-world content addressing.
pub struct Ast<H: MerkleHash> { /* ... */ }
```

What this restructure simultaneously collapses:

1. **Pure body OID (§9.2) — solved.** The AST has bytes; the bytes
   have an OID; `Body<H>` is content-addressable by construction.
   No "function-pointer-as-OID" hack; no per-machine identity; no
   cross-recompile drift. Same AST + same prism + same glass →
   same OID, on any machine, under any rustc.

2. **Glass wall — transparent.** The Rust-substrate boundary stops
   being opaque. We can SEE through it because what's on the other
   side IS structured AST content. Rust becomes the AST interpreter;
   no bodies live in Rust. The "glass wall" name finally names what
   it is: a wall you can see through, not a black box at the
   altitude transition.

3. **Self-hosting direction.** Bodies live in mirror's own substrate
   — in grammars, with substrate refs, with the substrate's existing
   declaration discipline. `@kintsugi/fracture/rename.mirror` IS a
   Body; its AST is the mirror substrate; the dispatcher loads the
   AST through the prism through the glass and runs it. The
   [[kintsugi-minimum-runnable]] spec already implied this; the
   restructure makes it the only possible shape.

4. **Reproducibility chain.** `BodyEntry<H>` is `Reconstructable`
   trivially because Body's bytes ARE the AST bytes plus the prism
   selector plus the glass selector. The persistent-mode disk
   spillover from §5.4 (in the earlier draft, an open problem about
   serialising a closure) is no longer an open problem.

5. **Property verdicts.** §4's restructure (Pure as an AST-analysis
   verdict) presupposes this restructure. Without an AST inside the
   body, the only available property check is a Rust-marker audit.
   With an AST, the check is mechanical and reproducible.

The field type changes accordingly:

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
/// content-addressed keying maps directly to Ref byte order; the
/// stored value's content OID is the (prism, glass, AST) triple's
/// Merkle hash under `H`.
pub struct CrystallizationStore<H: MerkleHash> {
    inner: FrgmntStore<BodyEntry<H>>,
}

/// One (Ref, Body<H>) pair as a Fragmentable entry. The Ref's bytes
/// are the content key; the Body — prism + glass + AST — is the
/// carried data. BodyEntry is what the scheduler observes, what the
/// store hot/cold-manages, and what the property analyses (§4) walk.
pub struct BodyEntry<H: MerkleHash> {
    pub path: Ref,
    pub body: Body<H>,
    pub realtime: RealtimeClass,  // §3.8
}
```

The migration is structurally larger than the earlier draft — the
closure goes away, the AST arrives. But the FrgmntStore wrapper
shape and the scheduler hook are exactly as the earlier draft
named them; the inside of `Body<H>` changed, not the outside of
`Crystallizations<H>`.

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

With the Body=prism+glass+AST restructure (§5.1), `BodyEntry<H>`
impls `Reconstructable` trivially: the (prism, glass, AST) triple
IS a content-addressable byte payload; serialising the triple
serialises the body. The earlier draft's open question — "the body
itself (`Arc<dyn Fn(...)>`) is not directly serialisable" — is
dissolved. Persistent mode stores the AST bytes plus the prism and
glass selectors plus the substrate ref; rehydration reads them back
and reconstructs the `Body<H>` value. Same bytes, same body, every
process, every machine.

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

### 5.6 `FrgmntStore` mode flag — hard vs soft realtime

The store gains a per-entry mode flag carried by the `RealtimeClass`
field on `BodyEntry<H>` (§5.1). The store's behaviour bifurcates:

```rust
/// Mode-aware insert. Hard entries are pinned: they cannot be
/// evicted to disk, cannot be admitted if pinning would exceed
/// the hard-resident byte budget. Soft entries get the existing
/// LIFO-evict-to-disk behaviour.
impl<H: MerkleHash> CrystallizationStore<H> {
    pub fn insert(&mut self, entry: BodyEntry<H>) -> Imperfect<
        Inserted,
        AdmissionError,
        Transparency<Ref>,
    > {
        match entry.realtime {
            RealtimeClass::Hard { deadline } => self.admit_hard(entry, deadline),
            RealtimeClass::Soft => self.admit_soft(entry),
        }
    }
}
```

The hard-resident byte budget is a substrate-level configuration:
`FrgmntStore::new_with_realtime_budget(hard_bytes, soft_bytes)`.
Hard-resident bytes are bounded; soft entries get whatever's left of
the total store capacity. Admission of a hard entry that would
exceed `hard_bytes` returns `Transparency::single(path,
PropertyVerdict::Fail(Diagnostic::new(
    "NotResident: hard-realtime budget exhausted")))`. Cache miss on
a hard entry returns the same Fail verdict; never blocks; never
falls back to disk.

Soft entries get the existing semantics unchanged: bounded cache,
LIFO eviction, disk spillover, fall-back-to-disk on miss.

### 5.7 Dispatcher offers two shapes

The `Crystallizations<H>` dispatcher exposes two methods. The
caller declares which contract applies; the substrate cannot guess.

```rust
impl<H: MerkleHash> Crystallizations<H> {
    /// Soft-realtime dispatch. Cache miss may fall back to disk;
    /// disk read may block; verdict carries whatever loss the
    /// interpretation incurred. The existing crystallize() shape.
    pub fn crystallize(
        &self,
        path: &Ref,
        input: Optic<(), Splinter<H>>,
    ) -> Imperfect<Splinter<H>, CrystallizeError, Transparency<Ref>>;

    /// Hard-realtime dispatch. Cache miss returns NotResident-as-
    /// Fail immediately; never blocks; never falls back to disk;
    /// admission required `check_pure ∧ check_wcet ≤ deadline` at
    /// registration time (§4.7). The dispatcher checks the
    /// deadline against the body's declared WCET and refuses if
    /// the declared bound exceeds the requested deadline.
    pub fn crystallize_bounded(
        &self,
        path: &Ref,
        input: Optic<(), Splinter<H>>,
        deadline: TickInterval,
    ) -> Imperfect<Splinter<H>, CrystallizeError, Transparency<Ref>>;
}
```

The split is mirrored by the scheduler's drop discipline (§3.8):
under pressure, soft-realtime work is dropped first; hard-realtime
work either meets its budget or returns a structured Fail verdict.
The substrate above the dispatcher — the kintsugi engine, the
build-graph altitude, the CLI surface — declares which contract
applies, the dispatcher honours it.

This is the Apollo 1202 made API: the caller of a hard-realtime
dispatch is the astronaut; the substrate is Hamilton's executive;
the Fail verdict is the alarm. Nothing silently misses.

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
2. **AST analyses in prism_core / mirror.** Land `check_pure` and
   `check_wcet` (§4.2, §4.7) as substrate-side AST analyses with
   `PropertyVerdict`-shaped output. No new traits; existing
   verdict carriers reused.
3. **Migrate `Body<H>` to prism+glass+AST.** Replace the
   `Arc<dyn Fn(...)>` shape with the structured triple (§5.1).
   This is the largest single piece of work; it touches every
   call site that constructs a `Body<H>` value and every site that
   invokes one. The interpreter (Rust-side AST evaluator) becomes
   the new dispatcher core.
4. **Migrate `Crystallizations<H>`.** Replace the `HashMap` field;
   add the scheduler field; add `crystallize_bounded` alongside the
   existing `crystallize` (§5.7); route `crystallize`/`register`
   through the new store; preserve the public API of `crystallize`,
   `register`, `knows`, `new` (`new` now constructs a
   `MetronomeScheduler` by default).
5. **Wire admission through Pure + WcetBounded verdicts.**
   `register_hard(c, deadline)` consults `check_pure` and
   `check_wcet` on the body's AST; refuses registration if either
   verdict fails. `register_soft(c)` (existing `register` renamed)
   skips the analysis. Pure verdict moves from `\` (parked) to
   `Pass` for the audited refs.
6. **Wire scheduler through bootstrap.** Bootstrap's main loop calls
   `crystallizations.tick()` periodically; the scheduler observes
   and decides; hard-realtime entries are pinned-resident.
7. **Audit + tests.** All existing tests that exercise iteration
   order across the registry get updated to expect deterministic
   Ref-byte-order. New tests cover the scheduler's tick semantics,
   the hard/soft admission split, the NotResident-as-Fail discipline.

No shim. No deprecation. The pre-cascade `HashMap<Ref, Body<H>>`
is gone after step 4; the `Arc<dyn Fn(...)>` Body shape is gone after
step 3.

---

## 7. What this assembly looks like — the substrate-pull markers

Each piece carries `[substrate-pull:realize]` per AGENTS.md
§"Boundary Rust is not frozen capability":

| Piece | Lives in | Marker |
|---|---|---|
| `Scheduler` trait | `fragmentation/src/scheduler.rs` | `[substrate-pull:realize]` — trait surface only, no capability |
| `GraphObservation` | `fragmentation/src/scheduler.rs` | `[substrate-pull:realize]` — pure observation, no capability |
| `Strategy` + `ScheduleAction` | `fragmentation/src/scheduler/strategy.rs` | `[substrate-pull:realize]` — closed sum type, no capability |
| `RealtimeClass` (Hard / Soft) | `fragmentation/src/scheduler/realtime.rs` | `[substrate-pull:realize]` — closed sum type, no capability |
| `MetronomeScheduler` impl | `fragmentation/src/scheduler/metronome.rs` | `[substrate-pull:realize]` — deterministic dispatch, no I/O |
| `HamiltonScheduler` impl | `fragmentation/src/scheduler/hamilton.rs` | `[substrate-pull:realize]` — Fate dispatch (substrate-side), Rust binding (boundary-side), 1202 drop discipline (§3.8) |
| `Body<H> = (Prism, Glass, Ast<H>)` | `mirror/bootstrap/src/body.rs` | `[substrate-pull:realize]` — structured, content-addressable; the glass wall made transparent (§5.1) |
| `check_pure`, `check_wcet` | `prism_core::properties` + analysis crate | `[substrate-pull:realize]` — AST analyses, `PropertyVerdict`-shaped output, no new traits |
| `CrystallizationStore<H>` | `mirror/bootstrap/src/crystallize.rs` | `[substrate-pull:realize]` — thin wrapper over `FrgmntStore<BodyEntry<H>>`, mode-aware (§5.6) |
| `BodyEntry<H>` | `mirror/bootstrap/src/crystallize.rs` | `[substrate-pull:realize]` — `(Ref, Body<H>, RealtimeClass)` triple, Fragmentable impl |

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
| Property check determinism | ⚠️ audit | ✅ by `check_pure` AST analysis (§4.2) |
| **Body OID is content-addressable** | ❌ (`Arc<dyn Fn>`) | ✅ (Body = prism + glass + AST, §5.1) |
| @fate model weights (store OID) | ✅ | ✅ |
| @fate cache key includes model OID | ⚠️ | ⚠️ (Fate-side work, not closed here) |
| @fate inference seed-pinned | ❌ | ❌ (Fate-side work, not closed here) |
| Au value cache key composition | ⚠️ | ⚠️ (depends on Fate work) |
| **Crystallizations iteration order** | ❌ HashMap | ✅ FrgmntStore + Ref-byte-order |
| @io tool wrapper determinism flags | ❌ | ⚠️ (AST analysis names `@io` sites; flag declarations owed) |
| Hard-realtime admission discipline | ❌ | ✅ (`check_pure ∧ check_wcet` at registration, §4.7) |
| Cross-machine toolchain reproducibility | ❌ | ❌ (v1.x) |

Three claims close (Pure, body-OID, Crystallizations iteration
order). The hard-realtime admission row is genuinely new — not on
the earlier chain table because the realtime discipline only got
named in this spec; named here so the chain accounts for it.

### 8.0 Use case — agent memory management via `fragmentation-mcp`

The HamiltonScheduler's first production consumer outside the build-
graph altitude is **agent memory management**, via the
`fragmentation-mcp` server (per
[[fragmentation-mcp]]). The mapping is direct and load-bearing:

- An MCP session is a shard. The shard owns one `FrgmntStore<
  BodyEntry<H>>` + one `HamiltonScheduler` instance + one
  [[lens-transit]] accumulator.
- The shard's budget (`fragmentation.shard.open(budget_bytes, ...)`)
  is the `BoundedStore::capacity()` the scheduler observes via the
  `pressure_load` and `entry_occupancy` features (§3.2 features
  #1, #2).
- The scheduler ticks on every incoming MCP tool call (per the
  reload-contract discipline from
  [[../../../mirror/docs/specs/lsp-and-mcp]]'s `@mirror/reload`).
  Each tick observes → selects a strategy → executes; the
  resulting `TickResult` carries the strategy choice + the transit
  report; both ride back to the agent on the wire.
- The four strategies acquire agent-altitude readings (per
  [[fragmentation-mcp]] §4.3): **Abyss** = session idle / all-read;
  **Pathfinder** = focused change crystallization; **Cartographer**
  = full tick; **Explorer** = boundary recovery (disk corruption,
  upstream-git fetch failure, ref-update race).
- The 1202 discipline reaches the wire: under hard-realtime
  pressure, soft-realtime MCP tool calls (`diff`, `history`,
  `search`, large `merge`) are dropped FIRST, with a structured
  `PropertyVerdict::Partial { confidence: 0.0, diagnostics: [...] }`
  surfaced to the agent on the same tool's response. Hard-realtime
  calls (`commit`, `read`, `refs.update` invoked with
  `realtime: "hard"` + a deadline) either meet their budget or
  return `PropertyVerdict::Fail(NotResident)` immediately; never
  block on disk. The agent sees the 1202.

This is what makes fragmentation-mcp the first deployment target
of the wider stack: the HamiltonScheduler's substrate-management
discipline transfers directly to agent-runtime substrate, and the
result is a structurally different shape than existing CLI-wrapped
git-MCPs (which are stateless and unbounded). Bounded RAM by
construction; structured drops; hard-realtime contracts at the
wire. See [[fragmentation-mcp]] for the full design.

### 8.1 Transit — the measurement carrier

The chain table above states *claims*. Observing whether the substrate
actually honours those claims requires measurement. [[lens-transit]]
is the substrate-side benchmark facility that measures computation
loss to hardware precision and carries the verdict as a
`Transparency<P>` payload.

The HamiltonScheduler integrates with transit at four points:

1. **Hard-realtime WCET observation.** When a hard-realtime body
   crystallizes, transit measures actual execution time. If actual
   exceeds the declared `WcetBounded(D)` deadline, transit emits a
   Fail verdict at the body's substrate path; the scheduler can
   demote the body's class or refuse future hard-realtime
   admission. The `WcetBounded` verdict declares the bound; transit
   observes the reality; the scheduler compares.
2. **Soft-realtime statistical envelope.** Transit accumulates
   per-body timing histograms; the soft-realtime contract
   ("95% under X ms") is a Partial verdict whose confidence is the
   empirical quantile.
3. **Drop accounting.** When the scheduler drops soft-realtime work
   under hard-realtime pressure (§3.8), transit records the drop
   as a structured Diagnostic at the dropped body's substrate path.
   The 1202 alarm shape, kept type-level: drops are visible, located,
   ordered.
4. **Hardware-floor declaration.** Transit names the local
   hardware's precision floor (machine epsilon for FP; nanosecond
   cycle granularity for time). The scheduler's WCET claims are
   stated *to* this floor; cross-hardware comparison goes through
   transit's documented hardware-translation rules.

See [[lens-transit]] for the full design; this section names the
integration points only.

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

### 9.2 Pure body OID computation — RESOLVED (§5.1)

This was an open question in the earlier draft. With the
Body = prism + glass + AST restructure (§5.1), it is solved. The
(prism, glass, AST) triple has a stable byte serialisation; the
bytes have an OID via BLAKE3 Merkle; the body's identity IS its
content OID, reproducible across recompilations, across processes,
across machines, across rustc versions.

The substrate-pull discipline ("every body should have a substrate
ref") is preserved — the substrate ref is the `BodyEntry<H>.path`;
the body's content OID is the `BodyEntry<H>.body`'s Merkle hash;
they are independent and both load-bearing. Pure-Rust bodies
without substrate refs are still an anti-pattern; the AST is
mirror's, not Rust's.

Kept in the open-questions list as a resolved item, with a pointer
to §5.1, so the historical record shows the path.

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

### 9.4 Counter to the `prism_core` home for verdicts

The AST-analysis approach (§4) homes `Pure` and `WcetBounded` as
*named properties* on the existing `prism_core::PropertyVerdict` /
`Transparency<Ref>` algebra. The counter-argument is that
`prism_core` is the optics-and-beams crate and has been gathering
property-shaped definitions (`Loss`, `Transparency`, now the named
property constants) without an articulated theory of why those
belong with optics. A future refactor could split out a dedicated
`prism_props` crate; the placement assumes that refactor does not
happen, or that when it does, the properties move with their family.
The restructure to AST-verdicts actually *strengthens* this counter
— there are now more named properties, not fewer.

The AST-walk analyses themselves (`check_pure`, `check_wcet`) may
belong in a `mirror/analysis` crate rather than in `prism_core`,
since they require an AST type that's mirror's. Named here as a
future layering decision.

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
lives (fragmentation), where the property verdicts live
(`prism_core::PropertyVerdict` carrying `Pure` and `WcetBounded`),
what they close in the reproducibility chain (C7, C8, body-OID,
hard-realtime admission, partial C9), and which open questions
remain. It carries lineage explicitly — Margaret Hamilton (the
loadbearing one), Beer, the metronome, the Prism-Scheduler plan. It
lands two architectural restructures Alex surfaced after the earlier
draft: Body = prism + glass + AST (§5.1), and hard/soft realtime
grade as a first-class substrate property (§1.5, §3.8, §5.6).

**Is not:** an implementation spec. The Rust does not land with this
commit. The next tick (forthcoming) lands the trait, the four
strategies, the AST-analysis verdicts, the `Crystallizations<H>`
migration, the Body restructure.

**Specifically refuses:**

- A rename of `HamiltonScheduler` to anything else. The Hamilton
  name honours **Margaret Hamilton** — her priority-driven
  asynchronous executive, the 1202 alarm, the four-property
  discipline (§1.2). Hamiltonian mechanics is coincidence-of-name;
  the woman is the source.
- A home for the scheduler that is not fragmentation. Spectral-db
  *consumes* the scheduler; it does not host it. The substrate
  management layer is fragmentation.
- A `Pure` marker trait. Pure is an **AST analysis** producing a
  `PropertyVerdict`, not a Rust marker. The verdict is
  content-addressable; the analysis is mechanical (§4).
- An opaque `Arc<dyn Fn(...)>` body. Body = prism + glass + AST.
  The glass wall is transparent; the AST is what's on the other
  side (§5.1).
- A scheduler that silently misses hard-realtime deadlines. The
  1202 discipline is mandatory: drops are structured verdicts;
  hard-realtime work either meets its budget or returns
  `NotResident`-as-Fail. No silent failure (§3.8).
- A compat shim for the `HashMap` → `FrgmntStore` migration. Per
  [[feedback-no-compat-shim]], hard cutover.

---

## 11. Cross-references

- [[mirror-native-vcs]] §1, §2 — the layering claim this spec extends.
- [[kintsugi-thesis]] §C7, §C8, §C9 — the reproducibility-chain
  claims this spec closes or partially closes.
- [[lens-transit]] — the measurement carrier that observes hard-
  realtime WCET, soft-realtime statistical envelopes, and drop
  accounting (§8.1). Sibling spec; cross-referenced both ways.
- [[prior-art]] §1.7 (Nix), §1.8 (Cargo), §1.5 (Bazel) — the leaks
  the AST-analysis verdicts + scheduler determinism address
  structurally.
- [[kintsugi-minimum-runnable]] — Tick A landed `Crystallizations<H>`;
  this spec migrates its table and restructures `Body<H>`. Tick B
  doesn't need the scheduler; Tick C does.
- [[store-vs-db-and-the-cascade]] §1 — the open-foundation / closed-
  engine boundary the scheduler straddles.
- [[2026-04-05-prism-scheduler]] (spectral-db) — the design migrated
  here. Read in full for the per-task implementation detail; this
  spec lifts the altitude.
- [[cartographer-design]] — the `SpectralBudget` framing; the
  Cartographer strategy's lineage.
- [[2026-04-03-spectral-swap]] — "Hamilton priority" context.
- Margaret Hamilton, *Universal Systems Language* (Hamilton
  Technologies; the follow-on to Apollo); the Apollo 11 1202 alarm
  contemporaneous flight-software documentation; the
  Presidential Medal of Freedom 2016 citation. The lineage source
  for the four-property discipline (§1.2): bounded resources +
  priority + drop-under-load + structured diagnostic.
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
what to drop. Margaret Hamilton's executive dropped the radar updates
and kept the landing-priority tasks; her system told the astronauts
the truth about what it was doing; they trusted it and landed. The
HamiltonScheduler inherits the shape: bounded resources, priority
discipline, graceful drop under overload, no silent failure. The
four Strategies are the dispatch surface; the realtime classes are
the priority discipline; the verdicts are the 1202 made type-level.
One name per altitude; one property per analysis; one assembly that
closes three claims of the reproducibility chain, adds a fourth, and
tells the truth about the rest.*
