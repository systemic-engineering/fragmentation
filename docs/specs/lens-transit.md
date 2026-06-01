# lens-transit — the benchmark facility carried as Transparency

*2026-06-01. Taut. Spec — design, not implementation. No Rust
changes; one markdown deliverable. Sibling to
[[hamilton-scheduler]]; the measurement carrier that observes
what the realtime contract DECLARES.*

Status: **Red** — the shape is pinned, the property family is named,
the composition law is borrowed verbatim from `Transparency<P>`. The
implementation tick lands afterward.

Depends on:
- `prism/imperfect/src/transparency.rs` — the `Transparency<P>`
  monoid + `PropertyVerdict::Pass | Partial | Fail` + the
  `merge_with` per-location combine. Transit is a *consumer* of
  this algebra, not a parallel one. The composition law is the
  monoid's; transit adds property kinds.
- `prism/core/src/lib.rs` — `prism_core` re-exports `Loss`,
  `Transparency`, `PropertyVerdict`, `Imperfect`. The transit
  verdicts are `Transparency<Ref>`-shaped and join this family.
- `fragmentation/docs/specs/hamilton-scheduler.md` (commit
  `e227f1e`) — the sibling spec. The realtime contract that this
  facility OBSERVES. Cross-reference is symmetric; §8.1 of
  [[hamilton-scheduler]] names the four integration points, this
  spec names how each integration is carried.
- `mirror/docs/cicd/kintsugi-thesis.md` — the reproducibility chain.
  Transit observations must themselves be reproducible (§6 below);
  cross-hardware claims are bounded by hardware-floor translation.
- `~/.reed/visibility/protected/practice/insights/cybernetics/beer-error-propagation.md`
  — Beer's algedonic-payload structure. Transit's per-property
  verdicts are the realisation of that structure at the
  measurement altitude: located, structured, audit-channel-ready.
- `prism/core/src/lib.rs` Prism trait — focus / project / split /
  zoom / refract. The metaphor that names this facility: a body
  is a prism; transit observes the spectrum the body produces.

Unblocks:
- [[hamilton-scheduler]] §8.1 — the four scheduler↔transit
  integration points. Without transit, the scheduler can DECLARE
  WCET bounds but cannot OBSERVE whether they hold; with transit,
  the chain closes.
- The kintsugi build-graph altitude — fracture loss measurement
  uses transit verdicts as the structured carrier, replacing any
  remaining bare-scalar loss callers (per
  [[feedback-loss-from-epistemologic-properties]]).
- Cross-process reproducibility audits — transit's hardware-floor
  declaration is the substrate's honest statement of what it can
  observe; cross-process comparisons go through the documented
  translation rules rather than implicit assumptions.
- A general spectral-decomposition primitive for the substrate.
  Transit IS the spectrum of a body's execution dispersed by
  property (time, FP loss, cache pressure, allocation). Once it
  lands, other substrate consumers — the Crystallizer's settled-
  budget audit; the kintsugi engine's fracture-by-fracture cost
  accounting — use the same shape.

---

## 0. The metaphor (load-bearing, not decorative)

**Transit measures what's lost in passage.**

Light enters a prism. The prism slows it — refractive index above
unity. Information passes through bodies. Bodies cost time, FP
precision, cache, allocation. **The body is the prism; the transit
is the spectrum; the loss is what dispersion shows.**

The metaphor cashes in three structural commitments:

1. **Spectral decomposition.** Transit measurements are multi-axis
   (time, FP precision, cache, allocation, branch predictions),
   not single-axis (wall-clock only). Each axis is a property; each
   property's loss is a `PropertyVerdict`; the report is the
   `Transparency<P>` over the property space. Flame-graph-shaped,
   not stopwatch-shaped.
2. **Located, not summed.** Loss happens at substrate paths —
   inside a specific AST node, at a specific body's invocation,
   under a specific property. The substrate carries the location,
   not just the magnitude. "Where did the time go?" is answerable.
3. **Hardware floor.** Transit's precision is bounded by the local
   hardware. Machine epsilon for FP; nanosecond cycle granularity
   for time; cache-line granularity for memory. Below the floor,
   the substrate cannot observe; above the floor, it must. The
   floor is documented per property per platform.

The Hamilton-spec analogue: the scheduler DECLARES a `WcetBounded(D)`
verdict at admission time. Transit OBSERVES the actual execution
time. The substrate compares the declaration to the observation
and produces a verdict. Both are `Transparency<Ref>`-shaped; the
declaration and the observation live in the same algebra.

**Lost in transit** — every computation loses information:

| Loss kind             | What's lost                  | Hardware floor               |
|-----------------------|------------------------------|------------------------------|
| Floating-point        | Bits per operation           | Machine epsilon (IEEE 754)   |
| Lossy compression     | Bits per encode/decode       | Codec-dependent              |
| Probabilistic step    | Certainty per sample         | Sampling-policy-bounded      |
| Cache eviction        | Warmth per evicted line      | Cache-line size              |
| Budget exhaustion     | The dropped computation      | TickInterval granularity     |
| Branch misprediction  | Speculative work             | Pipeline depth               |
| Allocation churn      | Locality + amortised cost    | Page granularity             |

Transit measures each — to the local hardware's floor. Anything
below the floor is structurally unobservable; transit names this
explicitly rather than pretending it doesn't exist.

---

## 1. The measurement primitive

There is no `Transit<P>` newtype. The measurement primitive is
**`Transparency<Ref>` over a property family the transit module
names**:

```rust
//! lens/transit — the benchmark facility. Measures computation
//! loss to hardware precision, carries the verdict as
//! Transparency<Ref> per the existing algebra.
//!
//! No new monoid; no new combine law. Transit reuses
//! Transparency<Ref> verbatim — verdicts compose with the rest of
//! the substrate's loss carriers under the same merge_with
//! discipline.

/// Property family transit names. Each variant is a stable
/// substrate path the verdict is located at.
pub enum TransitProperty {
    /// Wall-clock time elapsed. Floor: nanosecond.
    WallClock,
    /// Cumulative FP precision loss. Floor: machine epsilon.
    FpPrecision,
    /// Cache-line touches. Floor: cache-line granularity.
    CachePressure,
    /// Bytes allocated on the critical path. Floor: page size.
    Allocation,
    /// Branch mispredictions. Floor: pipeline depth.
    BranchMisses,
    /// Hard-realtime budget consumed. Floor: TickInterval granularity.
    BudgetConsumption,
}

/// Stable substrate path for a transit verdict.
/// Example: `@transit/wall_clock/@kintsugi/fracture/rename`
///   means "wall-clock observation, located at the rename body".
pub fn transit_path(property: TransitProperty, body: &Ref) -> Ref { /* ... */ }
```

The **dispatch shape** is the existing
[[hamilton-scheduler]] §5.7 pair, with a `with_transit` variant that
composes:

```rust
impl<H: MerkleHash> Crystallizations<H> {
    /// Crystallize and observe. Returns the body's normal verdict
    /// PLUS a transit report. The body sees no behaviour change;
    /// transit is a transparent wrapper.
    pub fn crystallize_with_transit(
        &self,
        path: &Ref,
        input: Optic<(), Splinter<H>>,
    ) -> (
        Imperfect<Splinter<H>, CrystallizeError, Transparency<Ref>>,
        TransitReport,
    );

    /// Hard-realtime crystallize with deadline + transit. Transit
    /// emits BudgetConsumption verdict against the deadline
    /// automatically; FP/cache/etc. depend on enabled properties.
    pub fn crystallize_bounded_with_transit(
        &self,
        path: &Ref,
        input: Optic<(), Splinter<H>>,
        deadline: TickInterval,
    ) -> (
        Imperfect<Splinter<H>, CrystallizeError, Transparency<Ref>>,
        TransitReport,
    );
}
```

The `TransitReport` newtype is the
`Transparency<Ref>` over the enabled property family:

```rust
/// One execution's transit report. Newtype around the
/// Transparency<Ref> carrying per-property verdicts located at
/// the body's substrate paths. No bare scalars on the surface;
/// no bare maps; the existing monoid carries the structure.
pub struct TransitReport {
    /// Per-property verdicts, located at substrate paths. Pass when
    /// the property's budget was met; Partial when partially met
    /// (e.g. soft-realtime statistical envelope); Fail when
    /// exceeded (e.g. WCET overrun, FP loss above tolerance).
    pub verdicts: Transparency<Ref>,
    /// Hardware floor declarations active for this report.
    /// Documents what the substrate could and could not observe.
    pub floors: HardwareFloors,
}

/// Per-hardware precision floors. Each property has a floor; reads
/// below the floor are structurally impossible.
pub struct HardwareFloors {
    pub wall_clock: NanosecondFloor,
    pub fp_precision: MachineEpsilon,
    pub cache_pressure: CacheLineSize,
    pub allocation: PageSize,
    pub branch_misses: PipelineDepth,
    pub budget: TickInterval,
}

/// Newtypes (no bare numbers on the surface, per
/// [[feedback-no-bare-types]]).
pub struct NanosecondFloor(u64);
pub struct MachineEpsilon(f64);
pub struct CacheLineSize(usize);
pub struct PageSize(usize);
pub struct PipelineDepth(u32);
```

---

## 2. What transit measures

Six axes, each a property:

### 2.1 WallClock

Time elapsed from dispatch entry to body return. Measured via
`Instant::now()` deltas (or platform equivalent for `@io` boundaries
where `Instant` is not available). Floor: nanosecond on most
platforms; coarser on virtualised hosts; transit reports the
actual floor at measurement time, not the nominal one.

Verdict shape:

- `Pass` — wall-clock ≤ declared deadline (or no deadline was
  declared).
- `Partial { confidence, diagnostics }` — wall-clock exceeded
  soft-realtime statistical envelope but within Hamilton's
  documented allowance; confidence is the empirical quantile rank
  of this measurement.
- `Fail(Diagnostic::new("WallClock: deadline D exceeded by E"))` —
  hard-realtime deadline exceeded. Triggers the
  [[hamilton-scheduler]] §3.8 drop discipline.

### 2.2 FpPrecision

Cumulative floating-point precision loss across the body's
execution. Measured via interval arithmetic or shadow-execution
in a higher-precision mode (the choice belongs to a future tick).
Floor: IEEE 754 machine epsilon (≈2.22e-16 for f64).

Verdict shape: bounded loss `Pass`; loss between bound and
tolerance `Partial`; loss above tolerance `Fail`. The body's
substrate declaration may include a `requires fp_tolerance(...)`
clause; transit honours it.

### 2.3 CachePressure

Number of cache lines touched during the body's execution.
Measured via performance-counter access where available; on
platforms without PMC access, transit reports `Partial { confidence:
0.0, ... }` and names the platform limitation. Floor: cache-line
granularity (typically 64 bytes).

### 2.4 Allocation

Bytes allocated on the critical path. For hard-realtime bodies,
allocation should be zero or bounded; transit verdicts the
allocation as `Pass` (zero), `Partial` (within bounded pool), or
`Fail` (unbounded growth). Floor: page size (typically 4 KB or 16
KB).

### 2.5 BranchMisses

Branch-misprediction count over the body's execution. Useful for
tight-loop bodies; less useful for I/O-dominated bodies. Floor:
pipeline depth (platform-dependent; ≈20 on modern x86).

### 2.6 BudgetConsumption

The load-bearing one for hard-realtime. Measures budget-consumed
ratio against the declared `TickInterval`. Pass when consumption
ratio ≤ 1.0; Fail when > 1.0; Partial when between 1.0 and a
substrate-configurable jitter allowance.

---

## 3. Composition — the monoid is borrowed

When a body invokes sub-bodies, transit composes parent and
children via the existing
[`Transparency::combine`](file:///Users/alexwolf/dev/projects/prism/imperfect/src/transparency.rs)
law. **No new combine; no new merge; no new identity.**

The parent body's `TransitReport.verdicts` is the union of the
children's verdicts (via `verdict_union`) merged with the parent's
own per-property observations. `Fail` dominates; `Partial`s
combine diagnostics; `Pass`es are neutral. The composition is
associative and commutative (per `Transparency<P>`'s monoid laws).

What this gives the substrate: when an outer body declares
`WcetBounded(D)`, transit's BudgetConsumption verdict for THE WHOLE
SUBTREE is computed by composition. The outer body's deadline
applies to outer+inner combined; inner-body bookkeeping does not
need special handling.

Where it stops: transit does NOT measure recursion outside the
substrate's body dispatch. If a body calls a Rust `std::*`
function directly (which it shouldn't — see
[[feedback-substrate-pull]]), transit measures it as an opaque
leaf. The body's AST should make all sub-calls visible (per
[[hamilton-scheduler]] §5.1's Body=prism+glass+AST restructure);
transit reads through the AST, not around it.

---

## 4. Hard-realtime integration — the WCET admission check

[[hamilton-scheduler]] §4.7 declares the hard-realtime admission
rule:

```
admit_hard(body, deadline) := combine(
    check_pure(body),
    check_wcet(body, deadline),
) discharges as Pass-or-Partial-above-threshold
```

`check_wcet` is a STATIC analysis over the body's AST — it proves a
bound by reading the AST's structure (bounded recursion, bounded
loops, called-bodies' bounds). It declares
`WcetBounded(D) → Pass | Partial | Fail`.

Transit is the DYNAMIC observation against the declaration:

```
admit_check     :  check_wcet(body, deadline)   -> Transparency<Ref>
actual_observe  :  transit BudgetConsumption    -> Transparency<Ref>
verdict         :  admit_check.combine(actual_observe)
```

When `admit_check.Pass` and `actual_observe.Pass`: the body is
hard-realtime-compliant at this invocation.

When `admit_check.Pass` and `actual_observe.Fail`: the static
analysis was wrong, OR the platform deviated from documented WCET
assumptions. The substrate **demotes the body's class** to
Soft and emits a structured diagnostic at the body's substrate
path. The 1202 made post-hoc: the substrate noticed; the substrate
spoke; the substrate adjusted.

When `admit_check.Fail`: the body was never admissible for
hard-realtime; the scheduler refused admission upstream; transit
never runs.

### 4.1 Why both — static AND dynamic

Static WCET analysis is necessary but not sufficient. The static
bound may be optimistic (the analysis trusts platform documentation
that the platform may not honour under load, contention, or thermal
throttling). Dynamic transit observation catches deviations the
static analysis cannot predict. Both verdicts together discharge
the hard-realtime claim; either alone is partial.

The substrate-pull discipline says: both verdicts live in the same
algebra, compose under the same law, surface to the same audit
channel. Hamilton's executive at 1202 did this implicitly — the
software declared what it could promise AND watched what actually
happened AND reconciled them in real time. The HamiltonScheduler
+ transit makes the dual explicit.

---

## 5. The report shape — spectral, not scalar

Multi-axis. The prism metaphor cashed in: a transit report
disperses the body's execution into property-located verdicts. The
shape resembles a flame graph rotated through property-space:

```
@kintsugi/fracture/rename:
  @transit/wall_clock        : Pass(82μs / 100μs budget)
  @transit/fp_precision      : Pass(loss 2e-15, tolerance 1e-12)
  @transit/cache_pressure    : Partial(247 lines, soft bound 500, hard bound 1000)
  @transit/allocation        : Pass(0 bytes critical-path)
  @transit/branch_misses     : Pass(11 / 100)
  @transit/budget_consumption: Pass(0.82)

  child @ast/walk:
    @transit/wall_clock      : Pass(34μs)
    @transit/cache_pressure  : Partial(180 lines, soft bound 200)
    ...
```

The substrate reads this as `Transparency<Ref>::Opaque({path →
verdict})`. Visualisation is a downstream concern; the report
shape is structured payload, not pre-formatted text.

A scalar collapse (sum of wall-clocks, total allocation) is
recoverable by walking the report — the report carries strictly
MORE information than a scalar; the scalar is a derived statistic
over the spectrum.

---

## 6. Reproducibility — within the hardware floor

Transit measurements MUST themselves be reproducible to within the
hardware floor:

- Same input + same body + same hardware + same kernel + same
  power state → same transit verdicts (within machine epsilon for
  FP; within nanosecond for time; within cache-line for cache).
- Same input + same body + DIFFERENT hardware → the verdicts may
  differ; the hardware-floor declaration documents the translation.
  Cross-hardware verdicts compose under a documented rule (the
  rule belongs to a future tick; named here as deferred).

The substrate's discipline:

1. Transit's measurement code is itself `Pure` (AST-verdict
   discharge per [[hamilton-scheduler]] §4.2). The measurement
   primitive cannot introduce its own non-determinism.
2. Per-tick hardware-floor declarations are stable across the
   tick. The substrate snapshots floors at tick entry; mid-tick
   floor changes (CPU frequency scaling) are documented in the
   report.
3. Cross-process verdicts on the SAME hardware should match
   bit-for-bit (within floors). Cross-process verdicts on
   DIFFERENT hardware are weakly comparable; the substrate names
   this weakness in the chain table.

### 6.1 The chain table delta

[[hamilton-scheduler]] §8 gained a row for hard-realtime admission.
This spec adds (or refines) one row in the chain:

| Layer | Before | After this spec |
|---|---|---|
| Transit observation reproducibility (same hardware) | n/a | ✅ (within hardware floor) |
| Transit observation reproducibility (cross hardware) | n/a | ⚠️ (translation rules owed) |

Neither row was on Mara's chain because transit didn't exist.
They're named here so the chain accounts for the new shape.

---

## 7. Hardware floors — the speed-of-light analogue

The substrate cannot measure below the local hardware's precision.
This is structural, not a limitation to apologise for. The
speed-of-light analogue: there is an upper bound on what can be
observed; the bound is part of the physics; the substrate carries
it explicitly.

The floors are per-platform, per-property:

| Property               | macOS arm64 floor | Linux x86_64 floor | Notes |
|------------------------|-------------------|--------------------|-------|
| WallClock              | ≈1 ns (mach_absolute_time) | ≈1 ns (CLOCK_MONOTONIC) | Coarser under virtualisation |
| FpPrecision (f64)      | 2.220446e-16      | 2.220446e-16       | IEEE 754 |
| FpPrecision (f32)      | 1.192093e-7       | 1.192093e-7        | IEEE 754 |
| CachePressure          | 128 B (L1)        | 64 B (L1)          | M-series vs x86 |
| Allocation             | 16 KB (page)      | 4 KB (page)        | Platform pagesize |
| BranchMisses           | ≈25 (pipeline)   | ≈20 (pipeline)    | Microarchitecture-dependent |
| BudgetConsumption      | TickInterval granularity — set by scheduler |

Transit reports MUST include the floors that applied at measurement
time. A report from macOS arm64 carries different floors than one
from Linux x86_64; downstream consumers compare like-with-like or
go through the documented translation.

Where a platform CANNOT measure a property (e.g. branch-miss PMC
denied by the OS), transit reports `Partial { confidence: 0.0,
diagnostics: vec![Diagnostic::new("Platform: BranchMisses
unavailable")] }` rather than omitting the property. Absence is
structural; the substrate states it explicitly.

---

## 8. Open questions — what this spec defers

### 8.1 Shadow execution for FpPrecision

Measuring FP precision loss requires either interval arithmetic
or shadow execution in a higher-precision mode (f128, mpfr). The
shadow-execution choice belongs to a future tick; this spec names
the verdict shape but defers the measurement implementation.
Until shadow execution lands, FpPrecision reports
`Partial { confidence: 0.0, ... }` with a platform diagnostic.

### 8.2 PMC access on locked-down platforms

CachePressure and BranchMisses require performance-counter access.
On macOS without elevated privileges, PMC access is denied; on
Linux without `perf_event_paranoid` adjustment, similarly. The
substrate is honest about this: verdicts report `Partial { confidence:
0.0, ... }` where access is denied; consumers know what they're not
getting.

### 8.3 Cross-hardware translation rules

Cross-hardware verdict comparison is weak. The substrate needs
a documented translation — "this Apple M-series report compares
to this Intel x86_64 report via THIS adjustment" — to make claims
composable across machines. The rules are not designed here;
named as future work.

### 8.4 The Transit's own overhead

Transit measurement HAS cost. Measuring wall-clock costs nanoseconds;
measuring cache pressure costs PMC reads. The substrate must:

- Default to a low-overhead property set on production paths.
- Allow opt-in to the full property set for explicit benchmark runs.
- Report the OBSERVATION OVERHEAD itself as a transit verdict (a
  separate `@transit/self` property). Heisenberg's principle made
  type-level: the substrate measures the cost of measuring.

The property-set selection and overhead-accounting are deferred to
the implementation tick.

### 8.5 Statistical-envelope soft-realtime contracts

The soft-realtime contract is "95% of ticks under X ms". This
requires accumulating per-body timing histograms across ticks. The
accumulator state is itself a `FrgmntStore` consumer; the
histograms are content-addressable; the verdict at any given tick
is a `Partial { confidence: <quantile>, ... }`. The histogram
storage shape is named but not designed here.

---

## 9. What this spec is and isn't

**Is:** an architectural anchor for the substrate's benchmark and
realtime-observation facility. It names the property family, the
verdict shape (`Transparency<Ref>` reused, not parallel), the
dispatch surface (`crystallize_with_transit` /
`crystallize_bounded_with_transit`), the hardware-floor discipline,
the composition law (the existing monoid; no new combine), and the
integration with [[hamilton-scheduler]]'s realtime contract.

**Is not:** an implementation spec. The Rust does not land with
this commit. The next tick lands the property enum, the measurement
implementations per property, the platform-floor detection, the
integration with the dispatcher.

**Specifically refuses:**

- A bare scalar loss. The substrate retired `ScalarLoss` already
  (per [[feedback-loss-from-epistemologic-properties]]); transit
  honours the retirement. A wall-clock measurement is not an
  `f64`; it is a `PropertyVerdict::Pass(WallClockReading)` located
  at a substrate path with a documented hardware floor.
- A single-axis report. Transit is multi-axis by construction.
  Wall-clock-only is the lowest-overhead property set; it is not
  the only one.
- A parallel composition monoid. The existing
  `Transparency<P>::combine` does exactly what transit needs; the
  spec refuses to invent a sibling. One algebra, many consumers.
- A pretender to hardware-floor-free precision. The substrate is
  honest about what it cannot measure. The floor declarations
  ride on every report; downstream consumers see them.
- A measurement primitive that introduces non-determinism. Transit
  itself is `Pure` per [[hamilton-scheduler]] §4.2's AST analysis;
  the substrate's reproducibility chain extends through the
  measurement primitive.

---

## 10. Cross-references

- [[hamilton-scheduler]] — sibling spec. §8.1 names the four
  scheduler↔transit integration points; this spec names how each
  integration is carried. Cross-reference is symmetric.
- [[../../mirror/docs/cicd/kintsugi-thesis]] — the reproducibility
  chain transit extends. The hard-realtime admission row is
  observed by transit; the cross-hardware translation is owed.
- [[mirror-native-vcs]] — the layering claim. Transit lives at the
  measurement altitude; lens layer; consumer of fragmentation's
  store and scheduler.
- `prism/imperfect/src/transparency.rs` — the monoid this spec
  reuses. `Transparency<P>::combine`, `PropertyVerdict::merge_with`,
  `verdict_union`. No new combine law.
- `prism/core/src/lib.rs` — the Prism trait the metaphor is named
  for. focus / project / split / zoom / refract. A body is a
  prism; transit is the spectrum.
- [[feedback-loss-from-epistemologic-properties]] — the retirement
  of bare-scalar loss; transit honours it.
- [[feedback-no-bare-types]] — the newtype discipline followed in
  the type sketches.
- `~/.reed/visibility/protected/practice/insights/cybernetics/beer-error-propagation.md`
  — Beer's algedonic-payload structure; transit verdicts realise
  the structure at the measurement altitude.
- AGENTS.md (fragmentation) — "Boundary Rust is not frozen
  capability"; transit's measurement code is boundary Rust;
  it carries observation, not capability.

---

*Light passes through a prism and is dispersed. Information passes
through a body and loses some of itself — to floating-point, to
cache eviction, to budget exhaustion, to whatever the local hardware
allows. Transit names the loss, locates it at its substrate path,
composes it under the same algebra the rest of the substrate uses,
and tells the truth about what the hardware floor permits the
substrate to observe. The body is the prism; transit is the
spectrum; Transparency is the carrier.*
