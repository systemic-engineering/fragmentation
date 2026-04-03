# Projection: Grammar-Projected Spectral Memory for Agentic Workers

**Date:** 2026-04-03
**Status:** Design approved
**Scope:** New `projection` crate + `@mcp` grammar + `@memory` grammars + compiler changes to conversation

---

## Summary

A memory layer for LLM agents (starting with Claude Code) built on spectral-db,
where each memory type is a `.conv` grammar that projects the underlying spectral
graph. The projection crate is the general-purpose interface to spectral-db —
no consumer talks to spectral-db directly.

The file system is an import/export projection. MCP is a serialization projection.
Memory layers are cognitive projections. The LSP is an editor projection.
All use the same mechanism: grammar defines visibility, projection filters
the graph, spectral coordinates are computed within the projection, `Beam<T>`
carries loss through every operation, `@ca` observes everything.

---

## Crate Topology

```
conversation (parser/compiler)
  |
projection (grammar->graph projection, spectral versioning, @ca observation)
  |
spectral-db (graph storage, spectral coordinates, pressure, crystallization)
  |
fragmentation + coincidence + prism (foundations)
```

### New artifacts

| Artifact | Type | Location |
|----------|------|----------|
| `projection` | Rust crate | `/Users/alexwolf/dev/projects/projection` |
| `@mcp` | `.conv` grammar | garden or inline initially |
| `@memory` | `.conv` grammar | garden or inline initially |
| `@episodic` | `.conv` grammar | composes `in @mcp`, `in @memory`, `in @ca` |
| `@semantic` | `.conv` grammar | composes `in @mcp`, `in @memory`, `in @ca` |
| `@procedural` | `.conv` grammar | composes `in @mcp`, `in @memory`, `in @ca` |
| `@working` | `.conv` grammar | composes `in @mcp`, `in @memory`, `in @ca` |
| `@file_memory` | `.conv` grammar | composes `in @memory` |

### Dependency changes

- `conversation-lsp` drops direct `spectral-db` dependency, depends on `projection` instead.
  Becomes `@projection(@lsp)`.
- `projection` depends on: `spectral-db`, `conversation` (compiled grammar types), `prism` (`Beam<T>`, `ShannonLoss`).
- `projection` does not depend on `coincidence` directly — spectral math stays in spectral-db.

### Consumers

```
@projection(@memory)  -- memory layer for agents
@projection(@lsp)     -- editor tooling (conversation-lsp)
@projection(@ca)      -- continuous alignment observations
```

---

## Terminology

`@lens` in conversation becomes `@projection`. The grammar-level concept of
filtering and transforming a graph with measured loss is a projection, not a lens.

Fragmentation's `Lens` type in the `Prism` enum keeps its name. It is a
structural edge to an external tree, not a projection operation.

---

## The projection Crate

### Core types

```rust
/// A compiled grammar projected onto a spectral graph.
struct Projection {
    grammar: CompiledGrammar,
    db: SpectralDb,
    witness: Author,
    projections: Vec<ProjectionRef>,  // composed-in domains
}

/// Validated node type from a compiled grammar.
struct NodeType(String);

/// Spectral distance threshold.
struct Distance(f64);

/// Content bytes validated against grammar type constraints.
struct NodeData(Vec<u8>);

/// Graph traversal depth.
struct Depth(u32);

/// A structural query pattern (node type + optional edge constraints).
/// Exact structure determined during implementation.
struct Pattern { node_type: Option<NodeType>, edges: Vec<EdgeConstraint> }

/// Active task context for procedural recall.
/// Determined by the working projection's current state.
struct Context { active_oids: Vec<Oid>, task_description: NodeData }

/// How the graph renders to a specific output format.
struct ExportProjection {
    grammar: CompiledGrammar,
    format: ExportFormat,  // file tree, JSON, MCP resource, markdown
}
```

### Core operations

```rust
impl Projection {
    fn store(&self, node_type: NodeType, data: NodeData) -> Beam<Oid>;
    fn recall(&self, query: Oid, distance: Distance) -> Beam<Vec<Oid>>;
    fn read(&self, oid: Oid) -> Beam<NodeData>;
    fn project(&self, target: &Projection) -> Beam<ProjectionDelta>;
    fn preview(&self) -> Beam<SubgraphSnapshot>;
    fn measure(&self, actual: &Projection) -> Beam<ProjectionDelta>;
    fn export(&self, projection: &ExportProjection) -> Beam<ExportResult>;
    fn ingest(&self, projection: &ExportProjection, source: &Path) -> Beam<Vec<Oid>>;
    fn forget(&self, oid: Oid) -> Beam<()>;
    fn walk(&self, from: Oid, depth: Depth) -> Beam<Vec<Oid>>;
    fn find(&self, pattern: Pattern) -> Beam<Vec<Oid>>;
    fn activate(&self, oid: Oid) -> Beam<()>;
    fn evict_under_pressure(&self) -> Beam<Vec<Oid>>;
    fn curate(&self, budget: Pressure) -> Beam<Vec<Oid>>;
    fn crystallize(&self, path: Oid) -> Beam<()>;
    fn recall_procedural(&self, context: Context) -> Beam<Vec<Oid>>;
}
```

All operations return `Beam<T>`. All beams are observed through `@ca`.

### Projection composition

Agent memory is constructed by composing base grammars with agent identity:

```rust
let mara_episodic = base_episodic
    .compose(mara_identity);  // witness-scoped
```

Two agents observing the same fact produce different graph nodes
(fragmentation's witness principle). The structure between nodes — the
spectral shape — can be compared across agents via eigenvalue distance.

---

## The @mcp Grammar

Models the MCP protocol as a composable `.conv` domain.

```conv
grammar @mcp {
  type = tool | resource | prompt
  type parameter = string | number | boolean | array | object
  type schema = parameter | required | optional
  type response = content | error

  action call(tool: tool) in @rust { projection::route(tool) }
  action read(resource: resource) in @rust { projection::read(resource) }
  action list(type: type) in @rust { projection::list(type) }

  out @json {
    tool -> { name, inputSchema, description }
    resource -> { uri, name, mimeType }
    response -> { content, _meta: { loss, precision, path } }
  }
}
```

Any grammar that declares `in @mcp` gets its actions exposed as MCP tools
and its nodes as MCP resources. The `out @json` arm defines serialization,
including `Beam<T>` metadata as `_meta`.

---

## Memory Grammars

### @memory (shared base)

```conv
grammar @memory {
  type witness = author | timestamp | context
  type oid = content_address
  type distance = spectral_distance

  action store(type: type, data: data) in @rust { projection.store(type, data) }
  action recall(query: oid, distance: distance) in @rust { projection.recall(query, distance) }
  action forget(oid: oid) in @rust { projection.forget(oid) }

  out @json {
    witness -> { author, timestamp, context }
    beam -> { result, loss, precision, path }
  }
}
```

### @episodic

```conv
grammar @episodic {
  in @mcp
  in @memory
  in @ca

  type = event | observation | interaction | session
  type event = { data, witness, predecessors }

  action replay(from: oid, depth: depth) in @rust { projection.walk(from, depth) }

  out @json {
    event -> { type, data, witness, predecessors }
    session -> { events, author, timespan }
  }
}
```

### @semantic

```conv
grammar @semantic {
  in @mcp
  in @memory
  in @ca

  type = entity | relation | fact
  type relation = { source, target, kind, weight }

  action consolidate(source: grammar) in @rust { projection.project(source) }
  action query(pattern: pattern) in @rust { projection.find(pattern) }

  out @json {
    entity -> { oid, type, data, relations }
    fact -> { entity, property, value, confidence }
  }
}
```

### @procedural

```conv
grammar @procedural {
  in @mcp
  in @memory
  in @ca

  type = strategy | pattern | skill
  type pattern = { trigger, actions, outcome, frequency }

  action crystallize(path: oid) in @rust { projection.crystallize(path) }
  action match(context: context) in @rust { projection.recall_procedural(context) }

  out @json {
    strategy -> { trigger, actions, outcome }
    pattern -> { frequency, last_used, success_rate }
  }
}
```

### @working

```conv
grammar @working {
  in @mcp
  in @memory
  in @ca

  type = active | evicted | promoted
  type pressure = load_factor

  action activate(oid: oid) in @rust { projection.activate(oid) }
  action evict() in @rust { projection.evict_under_pressure() }
  action curate(budget: pressure) in @rust { projection.curate(budget) }

  out @json {
    active -> { oid, relevance, last_accessed }
    pressure -> { current, capacity, critical }
  }
}
```

### @file_memory (import/export projection)

```conv
grammar @file_memory {
  in @memory

  type = memory_file | index | field_log
  type memory_file = { frontmatter, content }
  type index = { entries }

  action ingest(path: path) in @rust { projection.ingest(path) }
  action export(path: path) in @rust { projection.export(path) }

  out @markdown {
    memory_file -> { frontmatter: yaml, content: body }
    index -> { entries: bullet_list }
  }
}
```

`out @markdown` — not JSON. The serialization grammar determines the output
format. MEMORY.md is a markdown projection. MCP is a JSON projection.
Same graph, different `out`.

### Agent composition

```conv
grammar @mara_memory {
  in @episodic
  in @semantic
  in @procedural
  in @working
  in @file_memory

  use @mara as @witness
}
```

Each agent composes the shared base grammars with their identity.
Shared structure, witness-scoped content.

---

## @ca as the Observability Primitive

`@ca` (continuous alignment) is the observation layer. `Beam<T>` is its
native primitive via `in @prism`.

```conv
grammar @ca {
  in @prism
  in @ci

  type observation = shift | settlement | drift
  type decision = spawn | notify | wait
  type action = agent | crystal | converge

  action observe(beam: beam) in @rust { ca.observe(beam) }
  action measure(beam: beam) in @rust { ca.measure(beam) }
  action align(declared: beam, actual: beam) in @rust { ca.align(declared, actual) }
}
```

Every memory grammar declares `in @ca`. Every `Beam<T>` emitted by projection
is observed through `@ca`. This makes projection the first runtime consumer
of continuous alignment as load-bearing infrastructure, not just monitoring.

### What @ca observes

| Signal | @ca type | Source |
|--------|----------|--------|
| Memory stored | observation: shift | projection.store() |
| Eigenvalues stabilized | observation: settlement | spectral evolution |
| Knowledge drifting | observation: drift | spectral distance over time |
| Consolidation loss | beam.loss | projection.project() |
| Pressure threshold | observation: shift | working memory eviction |
| Agent convergence | observation: settlement | tick/tock sync |
| Agent divergence | observation: drift | spectral distance between agent projections |

---

## Consolidation and Spectral Evolution

### Consolidation = projection with measured loss

Episodic to semantic consolidation is `@semantic`'s `consolidate` action:

1. `projection.project(episodic)` takes the episodic subgraph
2. Grammar types filter what passes (entities, relations, facts survive; raw events don't)
3. Spectral coordinates computed on resulting semantic subgraph
4. Returns `Beam<ProjectionDelta>` with `ShannonLoss`

The loss is intentional. A debugging conversation becomes a fact. The
back-and-forth is the loss. Measurable. Retrievable from episodic if needed.

### Crystallization = procedural memory

spectral-db's existing crystallization: hot paths become immutable, surviving
pressure. Maps directly:

1. Optimizer tracks access patterns (exists)
2. Hot semantic paths flagged for crystallization (exists)
3. `@procedural`'s `crystallize` promotes to immutable pattern
4. Crystallized patterns survive working memory pressure

### Working memory = pressure management

spectral-db's existing pressure management: memory budget exceeded,
non-crystallized nodes evicted. Working memory is this mechanism as
a first-class projection:

1. `curate` takes a pressure budget (context window size)
2. Retrieves from all layers by spectral proximity to current task
3. Fills budget, evicts least-relevant under pressure
4. Returns curated set for context window

### Forgetting

Active eviction, not passive decay:

1. Working memory: evicts under context window pressure (immediate)
2. Episodic memory: evicts under storage pressure, oldest/least-connected first (medium-term)
3. Semantic memory: evicts on spectral redundancy — spectrally close facts merge (long-term)
4. Procedural memory: does not forget — crystallized patterns are immutable

Each eviction is a `Beam<T>` with loss. Total information forgotten is
a number in bits.

### Spectral evolution as memory health

- **Spectral drift**: eigenvalues shifting = knowledge evolving
- **Convergence**: eigenvalues stabilizing = consolidation settling
- **Partition detection**: Fiedler value dropping = disconnected knowledge clusters
- **Multi-agent coherence**: spectral distance between agent projections = alignment

---

## Compiler Integration and Spectral Versioning

### Grammar init blocks (new conversation feature)

```conv
grammar @episodic {
  in @rust {
    use projection::Projection;
    use ca::Observer;

    let projection = Projection::open(config);
    let observer = Observer::new(projection.witness());
  }

  action store(event: event) in @rust {
    let beam = projection.store(event);
    observer.observe(beam);
    beam
  }
}
```

Top-level `in @rust { ... }` runs once on grammar load. Sets up bindings
that action bodies reference.

### Deprecated syntax flag

All grammars without `in @rust { ... }` init blocks and action bodies
emit deprecation warnings. The warning is a `Beam<T>` observed through
`@ca` — a shift observation.

### Spectral versioning

The parser commits each grammar's Prism eigenvalues to the store.
On load:

1. Load stored eigenvalues from spectral-db
2. Parse current `.conv` source, compute eigenvalues
3. Compare:
   - **Equal**: hot path. Incremental recompilation — only changed action bodies.
   - **Diverged**: commit new eigenvalues. Full recompile. Project through
     `@projection(@diverged)` to find all downstream code that doesn't
     compile against the new grammar target.

The `@projection(@diverged)` subgraph is the breakage set. Typed,
observable via `@ca`, queryable by spectral proximity (most-connected
breakages first).

---

## Migration Path

### Phase 1: projection crate (Rust, approach B)

- Core `Projection` type with store/recall/project/preview/measure
- Newtyped arguments throughout
- `Beam<T>` on all operations
- spectral-db integration
- Basic @memory grammar support
- File import/export (@file_memory)
- Tests via TDD (red/green arc)

### Phase 2: @mcp grammar + Claude Code integration

- `@mcp` grammar with `in @rust` action bodies and `out @json`
- MCP server exposing memory tools
- Migration from file-based memory (MEMORY.md, field logs) via @file_memory ingest
- conversation-lsp migrated to depend on projection (@projection(@lsp))

### Phase 3: @ca integration

- `@ca` updated with `in @prism` for native `Beam<T>`
- All memory operations observed through @ca
- Spectral evolution metrics
- Deprecation warnings as @ca observations

### Phase 4: Compiler changes (conversation)

- `in @rust { ... }` init blocks
- `in @rust { ... }` action bodies
- Deprecated syntax flag
- Spectral versioning (eigenvalue comparison, incremental recompilation)
- `@projection(@diverged)` for breakage projection

### Phase 5: BEAM migration (approach C)

- Projection logic migrates to BEAM actors
- Rust crate becomes reference implementation and test harness
- BEAM actors become runtime
- gen_mcp pattern for MCP surface

---

## What This Does Not Cover

- External embedding bridge (no non-toolchain consumers for now)
- Specific Claude Code UX (will emerge from MCP tool surface)
- @ci integration details (exists, @ca composes through it)
- Replication topology for multi-agent sync (tick/tock exists in spectral-db)
- Specific `.conv` syntax changes needed in conversation parser (design-level only)
