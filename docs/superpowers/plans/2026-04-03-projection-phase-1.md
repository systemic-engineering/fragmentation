# Projection Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the `projection` Rust crate — the grammar-projected interface to spectral-db. Every consumer of spectral-db goes through projection.

**Architecture:** A thin layer over spectral-db that adds grammar-based type filtering, `Beam<T>` trace wrapping on all operations, working memory management, consolidation between memory layers with measured loss, and file import/export. No conversation compiler dependency in Phase 1 — grammar filters are constructed manually via a builder API.

**Tech Stack:** Rust, spectral-db (local), prism (local), tempfile (dev)

---

## Prerequisites

### Build & test

```bash
nix develop -c cargo test           # all tests
nix develop -c cargo clippy -- -D warnings  # lint
nix develop -c cargo fmt -- --check  # format
```

### Commit conventions

- Identity: `Mara <mara@systemic.engineer>`
- Branch: `projection/phase-1` (agents never commit to main directly)
- Arc: `🔴` (tests compile, tests fail) → `🟢` (all tests pass)
- Scaffold commits: `🔧`
- The pre-commit hook enforces the TDD arc

### spectral-db API (key methods)

All methods take `&self`. Location: `/Users/alexwolf/dev/projects/spectral-db/src/lib.rs`

```rust
// Construction
SpectralDb::open(repo_path: impl AsRef<Path>, schema_source: &str, precision: f64, memory_bytes: usize) -> Result<Self, Error>

// Nodes
fn insert(&self, node_type: &str, data: &[u8]) -> Result<String, Error>  // returns OID
fn get(&self, oid: &str) -> Option<store::Node>  // Node { oid, node_type, data }

// Edges
fn connect(&self, from: &str, to: &str) -> Result<(), Error>
fn connect_weighted(&self, from: &str, to: &str, weight: f64) -> Result<(), Error>
fn disconnect(&self, from: &str, to: &str) -> Result<(), Error>
fn neighbors(&self, oid: &str) -> Vec<String>
fn edges_weighted(&self) -> Vec<(String, String, f64)>

// Queries
fn find(&self, node_type: &str) -> query::ResultSet
fn walk(&self, from: &query::ResultSet, depth: usize) -> query::ResultSet
fn near(&self, target_oid: &str, distance: f64) -> query::ResultSet

// Spectral
fn compute_spectral_coordinates(&self)
fn spectral_distance_eigen(&self, oid_a: &str, oid_b: &str) -> Option<f64>

// Pressure & crystallization
fn crystallize(&self) -> Vec<crystallize::Crystal>
fn crystals(&self) -> Vec<crystallize::Crystal>
fn pressure_check(&self) -> Option<pressure::PressureEvent>

// Convergence
fn graph_hash(&self) -> convergence::GraphHash
fn graph_stats(&self) -> (usize, usize)  // (node_count, edge_count)

// Persistence
fn flush(&self)
fn precision(&self) -> f64
```

**Important:** Read `spectral-db/src/query.rs` to understand how `ResultSet` works (iteration, OID extraction). The plan assumes ResultSet has a way to extract OID strings — verify the exact API.

### prism API (key types)

Location: `/Users/alexwolf/dev/projects/prism/src/`

```rust
Beam::new(result: T) -> Beam<T>
beam.with_step(oid: Oid) -> Beam<T>
beam.with_loss(loss: ShannonLoss) -> Beam<T>
beam.with_precision(p: Precision) -> Beam<T>
beam.with_recovery(r: Recovery) -> Beam<T>
beam.is_lossless() -> bool
beam.has_loss() -> bool
beam.map(f: FnOnce(T) -> U) -> Beam<U>

Oid::new(s: &str) -> Oid          // AsRef<str>, Display, Hash, Ord
ShannonLoss::new(bits: f64)       // Display: "X.XXXXXX bits"
ShannonLoss::zero()
Precision::new(margin: f64)       // Display: "±X.XXXXXX"
Pressure::new(load: f64)          // [0.0, 1.0], is_critical() at >= 0.9

Recovery::Coarsened { from: Precision, to: Precision }
Recovery::Replayed { from_step: usize }
Recovery::Failed { reason: String }
```

---

## File Structure

```
/Users/alexwolf/dev/projects/projection/
├── Cargo.toml
├── flake.nix              — model after spectral-db/flake.nix
├── Justfile
├── src/
│   ├── lib.rs             — Projection struct, public API, re-exports
│   ├── types.rs           — NodeType, Distance, NodeData, Depth, ProjectionDelta, SubgraphSnapshot
│   ├── filter.rs          — GrammarFilter: builder, validation, to_schema(), intersect()
│   └── export.rs          — ExportFormat, export to markdown, ingest from markdown
└── tests/
    ├── types_test.rs      — newtype validation and edge cases
    ├── filter_test.rs     — grammar filter logic
    ├── store_read_test.rs — store, read, connect operations
    ├── query_test.rs      — recall, walk, find
    ├── pressure_test.rs   — forget, activate, evict, curate, crystallize
    ├── project_test.rs    — consolidation, preview, measure
    ├── export_test.rs     — file export/ingest round-trip
    └── compose_test.rs    — projection composition, witness scoping
```

---

## Task 1: Crate Scaffold

**Files:**
- Create: `Cargo.toml`, `flake.nix`, `Justfile`, `src/lib.rs`, `src/types.rs`, `src/filter.rs`, `src/export.rs`

- [ ] **Step 1: Create project directory and init git**

```bash
mkdir -p /Users/alexwolf/dev/projects/projection/src
mkdir -p /Users/alexwolf/dev/projects/projection/tests
cd /Users/alexwolf/dev/projects/projection
git init
git checkout -b projection/phase-1
```

- [ ] **Step 2: Write Cargo.toml**

Create `/Users/alexwolf/dev/projects/projection/Cargo.toml`:

```toml
[package]
name = "projection"
version = "0.1.0"
edition = "2021"
description = "Grammar-projected spectral memory. The interface to spectral-db."

[dependencies]
spectral-db = { path = "../spectral-db" }
prism = { path = "../prism" }

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 3: Write flake.nix**

Copy `spectral-db/flake.nix` and change the package name reference. The dev shell needs the same toolchain (Rust, gfortran for coincidence transitively, openssl, zlib, git, just). Verify with:

```bash
diff /Users/alexwolf/dev/projects/spectral-db/flake.nix flake.nix
```

Only the description/name should differ.

- [ ] **Step 4: Write Justfile**

Create `/Users/alexwolf/dev/projects/projection/Justfile`:

```just
test:
    nix develop -c cargo test

check: test lint fmt-check

lint:
    nix develop -c cargo clippy -- -D warnings

fmt:
    nix develop -c cargo fmt

fmt-check:
    nix develop -c cargo fmt -- --check
```

- [ ] **Step 5: Write empty module files**

Create `/Users/alexwolf/dev/projects/projection/src/lib.rs`:

```rust
pub mod types;
pub mod filter;
pub mod export;
```

Create `/Users/alexwolf/dev/projects/projection/src/types.rs`:

```rust
// Newtypes for projection operations.
```

Create `/Users/alexwolf/dev/projects/projection/src/filter.rs`:

```rust
// Grammar-based graph filter.
```

Create `/Users/alexwolf/dev/projects/projection/src/export.rs`:

```rust
// File export/ingest for memory serialization.
```

- [ ] **Step 6: Verify build**

```bash
nix develop -c cargo check
```

Expected: compiles with no errors.

- [ ] **Step 7: Create .gitignore**

Create `/Users/alexwolf/dev/projects/projection/.gitignore`:

```
/target
/result
.direnv/
```

- [ ] **Step 8: Commit scaffold**

```bash
git add Cargo.toml Cargo.lock flake.nix flake.lock Justfile .gitignore src/
git commit --author="Mara <mara@systemic.engineer>" -m "🔧 scaffold projection crate"
```

---

## Task 2: Newtype Definitions

**Files:**
- Modify: `src/types.rs`
- Create: `tests/types_test.rs`

- [ ] **Step 1: Write failing tests**

Create `/Users/alexwolf/dev/projects/projection/tests/types_test.rs`:

```rust
use projection::types::{NodeType, Distance, NodeData, Depth, ProjectionDelta, SubgraphSnapshot};
use prism::Oid;

#[test]
fn node_type_stores_and_displays() {
    let nt = NodeType::new_unchecked("event");
    assert_eq!(nt.as_str(), "event");
    assert_eq!(format!("{}", nt), "event");
}

#[test]
#[should_panic(expected = "NodeType cannot be empty")]
fn node_type_rejects_empty() {
    NodeType::new_unchecked("");
}

#[test]
fn node_type_equality() {
    let a = NodeType::new_unchecked("event");
    let b = NodeType::new_unchecked("event");
    let c = NodeType::new_unchecked("entity");
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn distance_stores_value() {
    let d = Distance::new(1.5);
    assert_eq!(d.as_f64(), 1.5);
}

#[test]
#[should_panic(expected = "Distance must be non-negative")]
fn distance_rejects_negative() {
    Distance::new(-1.0);
}

#[test]
fn distance_zero_allowed() {
    let d = Distance::new(0.0);
    assert_eq!(d.as_f64(), 0.0);
}

#[test]
fn distance_display() {
    let d = Distance::new(1.5);
    assert_eq!(format!("{}", d), "1.500000");
}

#[test]
fn node_data_from_bytes() {
    let nd = NodeData::new(vec![1, 2, 3]);
    assert_eq!(nd.as_bytes(), &[1, 2, 3]);
    assert_eq!(nd.len(), 3);
    assert!(!nd.is_empty());
}

#[test]
fn node_data_from_str() {
    let nd = NodeData::from_str("hello");
    assert_eq!(nd.as_bytes(), b"hello");
}

#[test]
fn node_data_empty() {
    let nd = NodeData::new(vec![]);
    assert!(nd.is_empty());
    assert_eq!(nd.len(), 0);
}

#[test]
fn depth_stores_value() {
    let d = Depth::new(3);
    assert_eq!(d.as_u32(), 3);
}

#[test]
fn projection_delta_tracks_kept_and_lost() {
    let delta = ProjectionDelta {
        kept: vec![Oid::new("abc")],
        lost: vec![Oid::new("def"), Oid::new("ghi")],
    };
    assert_eq!(delta.kept.len(), 1);
    assert_eq!(delta.lost.len(), 2);
}

#[test]
fn subgraph_snapshot_counts() {
    let snap = SubgraphSnapshot {
        nodes: vec![Oid::new("a"), Oid::new("b")],
        edges: vec![(Oid::new("a"), Oid::new("b"))],
    };
    assert_eq!(snap.node_count(), 2);
    assert_eq!(snap.edge_count(), 1);
}
```

- [ ] **Step 2: Write types with `todo!()` stubs**

Write `/Users/alexwolf/dev/projects/projection/src/types.rs`:

```rust
use std::fmt;
use prism::Oid;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeType(String);

impl NodeType {
    pub(crate) fn new_unchecked(_s: &str) -> Self {
        todo!()
    }

    pub fn as_str(&self) -> &str {
        todo!()
    }
}

impl fmt::Display for NodeType {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Distance(f64);

impl Distance {
    pub fn new(_d: f64) -> Self {
        todo!()
    }

    pub fn as_f64(self) -> f64 {
        todo!()
    }
}

impl fmt::Display for Distance {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeData(Vec<u8>);

impl NodeData {
    pub fn new(_data: Vec<u8>) -> Self {
        todo!()
    }

    pub fn from_str(_s: &str) -> Self {
        todo!()
    }

    pub fn as_bytes(&self) -> &[u8] {
        todo!()
    }

    pub fn len(&self) -> usize {
        todo!()
    }

    pub fn is_empty(&self) -> bool {
        todo!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Depth(u32);

impl Depth {
    pub fn new(_d: u32) -> Self {
        todo!()
    }

    pub fn as_u32(self) -> u32 {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub struct ProjectionDelta {
    pub kept: Vec<Oid>,
    pub lost: Vec<Oid>,
}

#[derive(Debug, Clone)]
pub struct SubgraphSnapshot {
    pub nodes: Vec<Oid>,
    pub edges: Vec<(Oid, Oid)>,
}

impl SubgraphSnapshot {
    pub fn node_count(&self) -> usize {
        todo!()
    }

    pub fn edge_count(&self) -> usize {
        todo!()
    }
}
```

- [ ] **Step 3: Verify tests compile but fail**

```bash
nix develop -c cargo test --test types_test
```

Expected: compiles, all tests panic with `not yet implemented`.

- [ ] **Step 4: Commit red**

```bash
git add src/types.rs tests/types_test.rs
git commit --author="Mara <mara@systemic.engineer>" -m "🔴 types: newtypes for projection operations"
```

- [ ] **Step 5: Implement all types**

Replace todo!() stubs in `src/types.rs`:

```rust
use std::fmt;
use prism::Oid;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeType(String);

impl NodeType {
    pub(crate) fn new_unchecked(s: &str) -> Self {
        assert!(!s.is_empty(), "NodeType cannot be empty");
        Self(s.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Distance(f64);

impl Distance {
    pub fn new(d: f64) -> Self {
        assert!(d >= 0.0, "Distance must be non-negative");
        Self(d)
    }

    pub fn as_f64(self) -> f64 {
        self.0
    }
}

impl fmt::Display for Distance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.6}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeData(Vec<u8>);

impl NodeData {
    pub fn new(data: Vec<u8>) -> Self {
        Self(data)
    }

    pub fn from_str(s: &str) -> Self {
        Self(s.as_bytes().to_vec())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Depth(u32);

impl Depth {
    pub fn new(d: u32) -> Self {
        Self(d)
    }

    pub fn as_u32(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct ProjectionDelta {
    pub kept: Vec<Oid>,
    pub lost: Vec<Oid>,
}

#[derive(Debug, Clone)]
pub struct SubgraphSnapshot {
    pub nodes: Vec<Oid>,
    pub edges: Vec<(Oid, Oid)>,
}

impl SubgraphSnapshot {
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}
```

- [ ] **Step 6: Run tests**

```bash
nix develop -c cargo test --test types_test
```

Expected: all tests pass.

- [ ] **Step 7: Commit green**

```bash
git add src/types.rs
git commit --author="Mara <mara@systemic.engineer>" -m "🟢 types: implement newtypes for projection operations"
```

---

## Task 3: GrammarFilter

**Files:**
- Modify: `src/filter.rs`
- Create: `tests/filter_test.rs`

- [ ] **Step 1: Write failing tests**

Create `/Users/alexwolf/dev/projects/projection/tests/filter_test.rs`:

```rust
use projection::filter::GrammarFilter;
use projection::types::NodeType;

#[test]
fn filter_allows_declared_type() {
    let filter = GrammarFilter::new("test")
        .allow_type("event")
        .allow_type("entity");
    let nt = NodeType::new_unchecked("event");
    assert!(filter.allows(&nt));
}

#[test]
fn filter_rejects_undeclared_type() {
    let filter = GrammarFilter::new("test")
        .allow_type("event");
    let nt = NodeType::new_unchecked("entity");
    assert!(!filter.allows(&nt));
}

#[test]
fn validate_type_returns_node_type_for_known() {
    let filter = GrammarFilter::new("test")
        .allow_type("event");
    let result = filter.validate_type("event");
    assert!(result.is_some());
    assert_eq!(result.unwrap().as_str(), "event");
}

#[test]
fn validate_type_returns_none_for_unknown() {
    let filter = GrammarFilter::new("test")
        .allow_type("event");
    assert!(filter.validate_type("entity").is_none());
}

#[test]
fn to_schema_generates_valid_conv() {
    let filter = GrammarFilter::new("episodic")
        .allow_type("event")
        .allow_type("observation");
    let schema = filter.to_schema();
    assert!(schema.contains("grammar @episodic"));
    assert!(schema.contains("type ="));
    assert!(schema.contains("event"));
    assert!(schema.contains("observation"));
}

#[test]
fn to_schema_empty_types_uses_any() {
    let filter = GrammarFilter::new("empty");
    let schema = filter.to_schema();
    assert!(schema.contains("type = any"));
}

#[test]
fn to_schema_types_sorted_deterministically() {
    let a = GrammarFilter::new("test")
        .allow_type("zebra")
        .allow_type("alpha");
    let b = GrammarFilter::new("test")
        .allow_type("alpha")
        .allow_type("zebra");
    assert_eq!(a.to_schema(), b.to_schema());
}

#[test]
fn filter_name_preserved() {
    let filter = GrammarFilter::new("episodic");
    assert_eq!(filter.name(), "episodic");
}

#[test]
fn compose_adds_domain() {
    let filter = GrammarFilter::new("episodic")
        .compose("mcp")
        .compose("memory");
    assert_eq!(filter.composed_domains(), &["mcp", "memory"]);
}

#[test]
fn intersect_narrows_types() {
    let a = GrammarFilter::new("wide")
        .allow_type("event")
        .allow_type("entity")
        .allow_type("fact");
    let b = GrammarFilter::new("narrow")
        .allow_type("entity")
        .allow_type("fact");
    let combined = a.intersect(&b);
    assert!(combined.allows(&NodeType::new_unchecked("entity")));
    assert!(combined.allows(&NodeType::new_unchecked("fact")));
    assert!(!combined.allows(&NodeType::new_unchecked("event")));
}

#[test]
fn intersect_combines_composed_domains() {
    let a = GrammarFilter::new("a").compose("mcp");
    let b = GrammarFilter::new("b").compose("ca");
    let combined = a.intersect(&b);
    assert!(combined.composed_domains().contains(&"mcp".to_string()));
    assert!(combined.composed_domains().contains(&"ca".to_string()));
}

#[test]
fn intersect_name_combined() {
    let a = GrammarFilter::new("episodic");
    let b = GrammarFilter::new("mara");
    let combined = a.intersect(&b);
    assert_eq!(combined.name(), "episodic+mara");
}
```

- [ ] **Step 2: Write filter with `todo!()` stubs**

Write `/Users/alexwolf/dev/projects/projection/src/filter.rs`:

```rust
use std::collections::HashSet;
use crate::types::NodeType;

#[derive(Debug, Clone)]
pub struct GrammarFilter {
    name: String,
    allowed_types: HashSet<String>,
    composed_domains: Vec<String>,
}

impl GrammarFilter {
    pub fn new(_name: &str) -> Self { todo!() }
    pub fn allow_type(self, _t: &str) -> Self { todo!() }
    pub fn compose(self, _domain: &str) -> Self { todo!() }
    pub fn allows(&self, _node_type: &NodeType) -> bool { todo!() }
    pub fn validate_type(&self, _type_name: &str) -> Option<NodeType> { todo!() }
    pub fn name(&self) -> &str { todo!() }
    pub fn allowed_types(&self) -> &HashSet<String> { todo!() }
    pub fn composed_domains(&self) -> &[String] { todo!() }
    pub fn to_schema(&self) -> String { todo!() }
    pub fn intersect(&self, _other: &GrammarFilter) -> GrammarFilter { todo!() }
}
```

- [ ] **Step 3: Verify tests compile but fail**

```bash
nix develop -c cargo test --test filter_test
```

Expected: compiles, all tests panic with `not yet implemented`.

- [ ] **Step 4: Commit red**

```bash
git add src/filter.rs tests/filter_test.rs
git commit --author="Mara <mara@systemic.engineer>" -m "🔴 filter: grammar-based graph filtering"
```

- [ ] **Step 5: Implement GrammarFilter**

Replace `src/filter.rs`:

```rust
use std::collections::HashSet;
use crate::types::NodeType;

#[derive(Debug, Clone)]
pub struct GrammarFilter {
    name: String,
    allowed_types: HashSet<String>,
    composed_domains: Vec<String>,
}

impl GrammarFilter {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            allowed_types: HashSet::new(),
            composed_domains: Vec::new(),
        }
    }

    pub fn allow_type(mut self, t: &str) -> Self {
        self.allowed_types.insert(t.to_string());
        self
    }

    pub fn compose(mut self, domain: &str) -> Self {
        self.composed_domains.push(domain.to_string());
        self
    }

    pub fn allows(&self, node_type: &NodeType) -> bool {
        self.allowed_types.contains(node_type.as_str())
    }

    pub fn validate_type(&self, type_name: &str) -> Option<NodeType> {
        if self.allowed_types.contains(type_name) {
            Some(NodeType::new_unchecked(type_name))
        } else {
            None
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn allowed_types(&self) -> &HashSet<String> {
        &self.allowed_types
    }

    pub fn composed_domains(&self) -> &[String] {
        &self.composed_domains
    }

    pub fn to_schema(&self) -> String {
        if self.allowed_types.is_empty() {
            return format!("grammar @{} {{\n  type = any\n}}", self.name);
        }
        let mut types: Vec<&str> = self.allowed_types.iter().map(|s| s.as_str()).collect();
        types.sort();
        format!(
            "grammar @{} {{\n  type = {}\n}}",
            self.name,
            types.join(" | ")
        )
    }

    pub fn intersect(&self, other: &GrammarFilter) -> GrammarFilter {
        let intersection: HashSet<String> = self
            .allowed_types
            .intersection(&other.allowed_types)
            .cloned()
            .collect();
        let mut combined_domains = self.composed_domains.clone();
        for d in &other.composed_domains {
            if !combined_domains.contains(d) {
                combined_domains.push(d.clone());
            }
        }
        GrammarFilter {
            name: format!("{}+{}", self.name, other.name),
            allowed_types: intersection,
            composed_domains: combined_domains,
        }
    }
}
```

- [ ] **Step 6: Run tests**

```bash
nix develop -c cargo test --test filter_test
```

Expected: all tests pass.

- [ ] **Step 7: Commit green**

```bash
git add src/filter.rs
git commit --author="Mara <mara@systemic.engineer>" -m "🟢 filter: implement grammar-based graph filtering"
```

---

## Task 4: Projection + Store + Read

**Files:**
- Modify: `src/lib.rs`
- Create: `tests/store_read_test.rs`

**Before starting:** Read `spectral-db/src/query.rs` to understand how `ResultSet` iterates (you need to extract OID strings from it). Also read `spectral-db/src/store.rs` to confirm the `Node` struct fields.

- [ ] **Step 1: Write failing tests**

Create `/Users/alexwolf/dev/projects/projection/tests/store_read_test.rs`:

```rust
use projection::Projection;
use projection::filter::GrammarFilter;
use projection::types::{NodeType, NodeData};
use tempfile::tempdir;

fn setup(types: &[&str]) -> (tempfile::TempDir, Projection) {
    let dir = tempdir().unwrap();
    let mut filter = GrammarFilter::new("test");
    for t in types {
        filter = filter.allow_type(t);
    }
    let proj = Projection::open(dir.path(), filter, "test-witness", 1e-6, 50_000_000)
        .expect("failed to open projection");
    (dir, proj)
}

#[test]
fn open_creates_projection() {
    let (_dir, proj) = setup(&["event"]);
    assert_eq!(proj.witness(), "test-witness");
}

#[test]
fn store_valid_type_returns_lossless_beam() {
    let (_dir, proj) = setup(&["event"]);
    let beam = proj.store(
        NodeType::new_unchecked("event"),
        NodeData::from_str("hello"),
    );
    assert!(beam.is_lossless());
    assert!(!beam.result.as_ref().is_empty());
}

#[test]
fn store_invalid_type_returns_beam_with_loss() {
    let (_dir, proj) = setup(&["event"]);
    let beam = proj.store(
        NodeType::new_unchecked("entity"),
        NodeData::from_str("should fail"),
    );
    assert!(beam.has_loss());
    assert!(beam.recovered.is_some());
}

#[test]
fn read_existing_node_returns_data() {
    let (_dir, proj) = setup(&["event"]);
    let stored = proj.store(
        NodeType::new_unchecked("event"),
        NodeData::from_str("read me back"),
    );
    let beam = proj.read(stored.result.clone());
    assert!(beam.is_lossless());
    assert!(beam.result.is_some());
    assert_eq!(beam.result.unwrap().as_bytes(), b"read me back");
}

#[test]
fn read_nonexistent_returns_none() {
    let (_dir, proj) = setup(&["event"]);
    let beam = proj.read(prism::Oid::new("nonexistent"));
    assert!(beam.result.is_none());
}

#[test]
fn read_filters_by_grammar() {
    let (_dir, proj) = setup(&["event"]);
    // Store directly through spectral-db would bypass the filter,
    // but reading through projection should filter.
    // For this test, store a valid type, then create a second projection
    // with a narrower filter and verify it can't read the node.
    let stored = proj.store(
        NodeType::new_unchecked("event"),
        NodeData::from_str("visible"),
    );

    // Create narrow projection on same db (via compose with empty filter)
    let narrow = proj.with_filter(
        GrammarFilter::new("narrow").allow_type("entity"),
    );
    let beam = narrow.read(stored.result.clone());
    // Node exists but type "event" not in narrow filter
    assert!(beam.result.is_none());
    assert!(beam.has_loss());
}

#[test]
fn connect_links_two_nodes() {
    let (_dir, proj) = setup(&["event"]);
    let a = proj.store(NodeType::new_unchecked("event"), NodeData::from_str("a"));
    let b = proj.store(NodeType::new_unchecked("event"), NodeData::from_str("b"));
    let beam = proj.connect(a.result.clone(), b.result.clone());
    assert!(beam.is_lossless());
}

#[test]
fn store_two_different_values_different_oids() {
    let (_dir, proj) = setup(&["event"]);
    let a = proj.store(NodeType::new_unchecked("event"), NodeData::from_str("alpha"));
    let b = proj.store(NodeType::new_unchecked("event"), NodeData::from_str("beta"));
    assert_ne!(a.result, b.result);
}
```

- [ ] **Step 2: Write Projection with `todo!()` stubs**

Write `/Users/alexwolf/dev/projects/projection/src/lib.rs`:

```rust
pub mod types;
pub mod filter;
pub mod export;

use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, Mutex};

use prism::{Beam, Oid, Precision, Pressure, Recovery, ShannonLoss};
use spectral_db::SpectralDb;

use crate::filter::GrammarFilter;
use crate::types::{Depth, Distance, NodeData, NodeType, ProjectionDelta, SubgraphSnapshot};

pub struct Projection {
    filter: GrammarFilter,
    db: Arc<SpectralDb>,
    witness: String,
    working_set: Mutex<HashSet<String>>,
}

impl Projection {
    pub fn open(
        _repo_path: &Path,
        _filter: GrammarFilter,
        _witness: &str,
        _precision: f64,
        _memory_bytes: usize,
    ) -> Result<Self, spectral_db::Error> {
        todo!()
    }

    pub fn witness(&self) -> &str {
        todo!()
    }

    pub fn with_filter(&self, _filter: GrammarFilter) -> Projection {
        todo!()
    }

    pub fn store(&self, _node_type: NodeType, _data: NodeData) -> Beam<Oid> {
        todo!()
    }

    pub fn read(&self, _oid: Oid) -> Beam<Option<NodeData>> {
        todo!()
    }

    pub fn connect(&self, _from: Oid, _to: Oid) -> Beam<()> {
        todo!()
    }

    pub fn reindex(&self) {
        todo!()
    }

    pub fn recall(&self, _query: Oid, _distance: Distance) -> Beam<Vec<Oid>> {
        todo!()
    }

    pub fn walk(&self, _from: Oid, _depth: Depth) -> Beam<Vec<Oid>> {
        todo!()
    }

    pub fn find(&self, _node_type: NodeType) -> Beam<Vec<Oid>> {
        todo!()
    }

    pub fn forget(&self, _oid: Oid) -> Beam<()> {
        todo!()
    }

    pub fn activate(&self, _oid: Oid) -> Beam<()> {
        todo!()
    }

    pub fn evict_under_pressure(&self) -> Beam<Vec<Oid>> {
        todo!()
    }

    pub fn curate(&self, _budget: Pressure) -> Beam<Vec<Oid>> {
        todo!()
    }

    pub fn project(&self, _target: &Projection) -> Beam<ProjectionDelta> {
        todo!()
    }

    pub fn preview(&self) -> Beam<SubgraphSnapshot> {
        todo!()
    }

    pub fn measure(&self, _actual: &Projection) -> Beam<ProjectionDelta> {
        todo!()
    }

    pub fn crystallize(&self, _oid: Oid) -> Beam<()> {
        todo!()
    }

    pub fn recall_procedural(&self, _active_oids: &[Oid]) -> Beam<Vec<Oid>> {
        todo!()
    }

    pub fn graph_stats(&self) -> (usize, usize) {
        todo!()
    }
}
```

- [ ] **Step 3: Verify tests compile but fail**

```bash
nix develop -c cargo test --test store_read_test
```

Expected: compiles, all tests panic with `not yet implemented`.

- [ ] **Step 4: Commit red**

```bash
git add src/lib.rs tests/store_read_test.rs
git commit --author="Mara <mara@systemic.engineer>" -m "🔴 projection: store and read through grammar filter"
```

- [ ] **Step 5: Implement open, witness, with_filter, store, read, connect**

In `src/lib.rs`, replace the todo!() stubs for these methods:

```rust
impl Projection {
    pub fn open(
        repo_path: &Path,
        filter: GrammarFilter,
        witness: &str,
        precision: f64,
        memory_bytes: usize,
    ) -> Result<Self, spectral_db::Error> {
        let schema = filter.to_schema();
        let db = SpectralDb::open(repo_path, &schema, precision, memory_bytes)?;
        Ok(Self {
            filter,
            db: Arc::new(db),
            witness: witness.to_string(),
            working_set: Mutex::new(HashSet::new()),
        })
    }

    pub fn witness(&self) -> &str {
        &self.witness
    }

    pub fn with_filter(&self, filter: GrammarFilter) -> Projection {
        Projection {
            filter,
            db: Arc::clone(&self.db),
            witness: self.witness.clone(),
            working_set: Mutex::new(HashSet::new()),
        }
    }

    pub fn store(&self, node_type: NodeType, data: NodeData) -> Beam<Oid> {
        if !self.filter.allows(&node_type) {
            return Beam::new(Oid::new(""))
                .with_loss(ShannonLoss::new(data.len() as f64 * 8.0))
                .with_recovery(Recovery::Failed {
                    reason: format!(
                        "type '{}' not in grammar '{}'",
                        node_type,
                        self.filter.name()
                    ),
                });
        }
        match self.db.insert(node_type.as_str(), data.as_bytes()) {
            Ok(oid) => Beam::new(Oid::new(&oid)),
            Err(e) => Beam::new(Oid::new(""))
                .with_loss(ShannonLoss::new(data.len() as f64 * 8.0))
                .with_recovery(Recovery::Failed {
                    reason: e.to_string(),
                }),
        }
    }

    pub fn read(&self, oid: Oid) -> Beam<Option<NodeData>> {
        match self.db.get(oid.as_ref()) {
            Some(node) => {
                if !self.filter.allowed_types().contains(&node.node_type) {
                    Beam::new(None)
                        .with_loss(ShannonLoss::new(node.data.len() as f64 * 8.0))
                } else {
                    Beam::new(Some(NodeData::new(node.data)))
                }
            }
            None => Beam::new(None),
        }
    }

    pub fn connect(&self, from: Oid, to: Oid) -> Beam<()> {
        match self.db.connect(from.as_ref(), to.as_ref()) {
            Ok(()) => Beam::new(()),
            Err(e) => Beam::new(())
                .with_loss(ShannonLoss::new(1.0))
                .with_recovery(Recovery::Failed {
                    reason: e.to_string(),
                }),
        }
    }

    pub fn reindex(&self) {
        self.db.compute_spectral_coordinates();
    }

    pub fn graph_stats(&self) -> (usize, usize) {
        self.db.graph_stats()
    }
}
```

Keep all other methods as `todo!()`.

- [ ] **Step 6: Run tests**

```bash
nix develop -c cargo test --test store_read_test
```

Expected: all tests pass.

- [ ] **Step 7: Commit green**

```bash
git add src/lib.rs
git commit --author="Mara <mara@systemic.engineer>" -m "🟢 projection: store and read through grammar filter"
```

---

## Task 5: Recall (Spectral Query)

**Files:**
- Modify: `src/lib.rs`
- Create: `tests/query_test.rs`

**Note:** `recall` uses `spectral_distance_eigen` and `near`. These require spectral coordinates to be computed first — call `reindex()` after storing and connecting nodes.

- [ ] **Step 1: Write failing tests**

Create `/Users/alexwolf/dev/projects/projection/tests/query_test.rs`:

```rust
use projection::Projection;
use projection::filter::GrammarFilter;
use projection::types::{NodeType, NodeData, Distance, Depth};
use prism::Oid;
use tempfile::tempdir;

fn setup(types: &[&str]) -> (tempfile::TempDir, Projection) {
    let dir = tempdir().unwrap();
    let mut filter = GrammarFilter::new("test");
    for t in types {
        filter = filter.allow_type(t);
    }
    let proj = Projection::open(dir.path(), filter, "test-witness", 1e-6, 50_000_000)
        .expect("failed to open projection");
    (dir, proj)
}

#[test]
fn recall_returns_beam() {
    let (_dir, proj) = setup(&["event"]);
    let a = proj.store(NodeType::new_unchecked("event"), NodeData::from_str("alpha"));
    let beam = proj.recall(a.result.clone(), Distance::new(10.0));
    // Even with no connections, should return a valid beam
    assert!(beam.is_lossless());
}

#[test]
fn recall_finds_connected_nodes() {
    let (_dir, proj) = setup(&["event"]);
    let a = proj.store(NodeType::new_unchecked("event"), NodeData::from_str("alpha"));
    let b = proj.store(NodeType::new_unchecked("event"), NodeData::from_str("beta"));
    let c = proj.store(NodeType::new_unchecked("event"), NodeData::from_str("gamma"));
    proj.connect(a.result.clone(), b.result.clone());
    // c is isolated
    proj.reindex();

    let beam = proj.recall(a.result.clone(), Distance::new(100.0));
    // b should be near a (connected), c may or may not be (isolated)
    let oids: Vec<String> = beam.result.iter().map(|o| o.as_ref().to_string()).collect();
    assert!(oids.contains(&b.result.as_ref().to_string()));
}

#[test]
fn recall_respects_distance_threshold() {
    let (_dir, proj) = setup(&["event"]);
    let a = proj.store(NodeType::new_unchecked("event"), NodeData::from_str("alpha"));
    let b = proj.store(NodeType::new_unchecked("event"), NodeData::from_str("beta"));
    proj.connect(a.result.clone(), b.result.clone());
    proj.reindex();

    // Very tight distance should return fewer results
    let tight = proj.recall(a.result.clone(), Distance::new(0.001));
    let wide = proj.recall(a.result.clone(), Distance::new(1000.0));
    assert!(tight.result.len() <= wide.result.len());
}

#[test]
fn walk_traverses_edges() {
    let (_dir, proj) = setup(&["event"]);
    let a = proj.store(NodeType::new_unchecked("event"), NodeData::from_str("a"));
    let b = proj.store(NodeType::new_unchecked("event"), NodeData::from_str("b"));
    let c = proj.store(NodeType::new_unchecked("event"), NodeData::from_str("c"));
    proj.connect(a.result.clone(), b.result.clone());
    proj.connect(b.result.clone(), c.result.clone());

    let beam = proj.walk(a.result.clone(), Depth::new(1));
    let oids: Vec<String> = beam.result.iter().map(|o| o.as_ref().to_string()).collect();
    // Depth 1 from a: should reach b
    assert!(oids.contains(&b.result.as_ref().to_string()));
}

#[test]
fn walk_depth_zero_returns_self() {
    let (_dir, proj) = setup(&["event"]);
    let a = proj.store(NodeType::new_unchecked("event"), NodeData::from_str("a"));
    let beam = proj.walk(a.result.clone(), Depth::new(0));
    // Depth 0 should return at least the starting node
    assert!(!beam.result.is_empty());
}

#[test]
fn find_returns_nodes_of_type() {
    let (_dir, proj) = setup(&["event", "entity"]);
    proj.store(NodeType::new_unchecked("event"), NodeData::from_str("e1"));
    proj.store(NodeType::new_unchecked("event"), NodeData::from_str("e2"));
    proj.store(NodeType::new_unchecked("entity"), NodeData::from_str("n1"));

    let beam = proj.find(NodeType::new_unchecked("event"));
    assert_eq!(beam.result.len(), 2);
    assert!(beam.is_lossless());
}

#[test]
fn find_invalid_type_returns_empty() {
    let (_dir, proj) = setup(&["event"]);
    proj.store(NodeType::new_unchecked("event"), NodeData::from_str("e1"));

    let beam = proj.find(NodeType::new_unchecked("entity"));
    assert!(beam.result.is_empty());
}
```

- [ ] **Step 2: Verify tests compile but fail**

```bash
nix develop -c cargo test --test query_test
```

Expected: compiles, all tests panic with `not yet implemented`.

- [ ] **Step 3: Commit red**

```bash
git add tests/query_test.rs
git commit --author="Mara <mara@systemic.engineer>" -m "🔴 query: recall, walk, find through projection"
```

- [ ] **Step 4: Implement recall, walk, find**

In `src/lib.rs`, replace the todo!() stubs for these methods.

**Important:** You need to understand how `query::ResultSet` works. Read `spectral-db/src/query.rs` first. The implementation below assumes ResultSet provides a way to collect OID strings — adjust to match the actual API.

```rust
    pub fn recall(&self, query: Oid, distance: Distance) -> Beam<Vec<Oid>> {
        let result_set = self.db.near(query.as_ref(), distance.as_f64());
        // Convert ResultSet to Vec<Oid>, filtering by grammar
        let oids: Vec<Oid> = result_set_to_oids(&result_set)
            .into_iter()
            .filter(|oid| {
                self.db
                    .get(oid.as_ref())
                    .map(|n| self.filter.allowed_types().contains(&n.node_type))
                    .unwrap_or(false)
            })
            .collect();
        Beam::new(oids)
    }

    pub fn walk(&self, from: Oid, depth: Depth) -> Beam<Vec<Oid>> {
        // Build a single-element ResultSet from the starting OID.
        // Check spectral-db's query API for how to construct a ResultSet
        // from OIDs. You may need: ResultSet::from_oids or find + filter.
        //
        // If ResultSet can't be constructed directly, use neighbors()
        // in a manual BFS loop:
        let mut visited = HashSet::new();
        let mut frontier = vec![from.as_ref().to_string()];
        visited.insert(from.as_ref().to_string());

        for _ in 0..depth.as_u32() {
            let mut next_frontier = vec![];
            for oid in &frontier {
                for neighbor in self.db.neighbors(oid) {
                    if visited.insert(neighbor.clone()) {
                        // Filter by grammar
                        if let Some(node) = self.db.get(&neighbor) {
                            if self.filter.allowed_types().contains(&node.node_type) {
                                next_frontier.push(neighbor);
                            }
                        }
                    }
                }
            }
            frontier = next_frontier;
        }

        let oids: Vec<Oid> = visited.into_iter().map(|s| Oid::new(&s)).collect();
        Beam::new(oids)
    }

    pub fn find(&self, node_type: NodeType) -> Beam<Vec<Oid>> {
        if !self.filter.allows(&node_type) {
            return Beam::new(vec![]);
        }
        let result_set = self.db.find(node_type.as_str());
        let oids: Vec<Oid> = result_set_to_oids(&result_set);
        Beam::new(oids)
    }
```

Add a helper function (either free-standing or on Projection) to convert ResultSet to Vec<Oid>. The exact implementation depends on ResultSet's API:

```rust
/// Convert a spectral-db ResultSet to Vec<Oid>.
/// Adjust based on the actual ResultSet API.
fn result_set_to_oids(rs: &spectral_db::query::ResultSet) -> Vec<Oid> {
    // If ResultSet implements IntoIterator or has .oids():
    // rs.oids().map(|s| Oid::new(&s)).collect()
    //
    // If ResultSet has a .nodes() or .entries():
    // rs.nodes().iter().map(|n| Oid::new(&n.oid)).collect()
    //
    // Read query.rs to determine the correct extraction method.
    todo!("Implement based on ResultSet API")
}
```

- [ ] **Step 5: Run tests**

```bash
nix develop -c cargo test --test query_test
```

Expected: all tests pass.

- [ ] **Step 6: Commit green**

```bash
git add src/lib.rs
git commit --author="Mara <mara@systemic.engineer>" -m "🟢 query: recall, walk, find through projection"
```

---

## Task 6: Forget + Working Memory

**Files:**
- Modify: `src/lib.rs`
- Create: `tests/pressure_test.rs`

- [ ] **Step 1: Write failing tests**

Create `/Users/alexwolf/dev/projects/projection/tests/pressure_test.rs`:

```rust
use projection::Projection;
use projection::filter::GrammarFilter;
use projection::types::{NodeType, NodeData};
use prism::{Oid, Pressure};
use tempfile::tempdir;

fn setup(types: &[&str]) -> (tempfile::TempDir, Projection) {
    let dir = tempdir().unwrap();
    let mut filter = GrammarFilter::new("test");
    for t in types {
        filter = filter.allow_type(t);
    }
    let proj = Projection::open(dir.path(), filter, "test-witness", 1e-6, 50_000_000)
        .expect("failed to open projection");
    (dir, proj)
}

#[test]
fn forget_removes_from_working_set() {
    let (_dir, proj) = setup(&["event"]);
    let a = proj.store(NodeType::new_unchecked("event"), NodeData::from_str("a"));
    proj.activate(a.result.clone());
    assert!(proj.is_active(&a.result));

    proj.forget(a.result.clone());
    assert!(!proj.is_active(&a.result));
}

#[test]
fn activate_adds_to_working_set() {
    let (_dir, proj) = setup(&["event"]);
    let a = proj.store(NodeType::new_unchecked("event"), NodeData::from_str("a"));

    assert!(!proj.is_active(&a.result));
    let beam = proj.activate(a.result.clone());
    assert!(beam.is_lossless());
    assert!(proj.is_active(&a.result));
}

#[test]
fn activate_nonexistent_returns_loss() {
    let (_dir, proj) = setup(&["event"]);
    let beam = proj.activate(Oid::new("nonexistent"));
    assert!(beam.has_loss());
}

#[test]
fn evict_under_pressure_removes_non_active() {
    let (_dir, proj) = setup(&["event"]);
    let a = proj.store(NodeType::new_unchecked("event"), NodeData::from_str("a"));
    let b = proj.store(NodeType::new_unchecked("event"), NodeData::from_str("b"));
    proj.activate(a.result.clone());
    // b is not activated

    let beam = proj.evict_under_pressure();
    // Should suggest evicting b (not activated)
    let evicted_strs: Vec<String> = beam.result.iter().map(|o| o.as_ref().to_string()).collect();
    assert!(evicted_strs.contains(&b.result.as_ref().to_string()));
    assert!(!evicted_strs.contains(&a.result.as_ref().to_string()));
}

#[test]
fn curate_returns_most_relevant_within_budget() {
    let (_dir, proj) = setup(&["event"]);
    let a = proj.store(NodeType::new_unchecked("event"), NodeData::from_str("a"));
    let b = proj.store(NodeType::new_unchecked("event"), NodeData::from_str("b"));
    let c = proj.store(NodeType::new_unchecked("event"), NodeData::from_str("c"));
    proj.activate(a.result.clone());
    proj.activate(b.result.clone());
    // c not activated

    // Budget at 0.5 — should return active items, maybe not all
    let beam = proj.curate(Pressure::new(0.5));
    assert!(beam.is_lossless());
    // Active items should be preferred
    let curated_strs: Vec<String> = beam.result.iter().map(|o| o.as_ref().to_string()).collect();
    assert!(curated_strs.contains(&a.result.as_ref().to_string()));
}

#[test]
fn curate_at_critical_pressure_returns_only_essentials() {
    let (_dir, proj) = setup(&["event"]);
    for i in 0..10 {
        proj.store(
            NodeType::new_unchecked("event"),
            NodeData::from_str(&format!("node-{}", i)),
        );
    }
    // Critical pressure (0.95) should aggressively limit
    let beam = proj.curate(Pressure::new(0.95));
    assert!(beam.result.len() < 10);
}
```

- [ ] **Step 2: Verify tests compile but fail**

You need to add `is_active()` to Projection's public API for the tests to compile:

Add to the stub in `src/lib.rs`:

```rust
    pub fn is_active(&self, _oid: &Oid) -> bool {
        todo!()
    }
```

```bash
nix develop -c cargo test --test pressure_test
```

Expected: compiles, all tests panic with `not yet implemented`.

- [ ] **Step 3: Commit red**

```bash
git add src/lib.rs tests/pressure_test.rs
git commit --author="Mara <mara@systemic.engineer>" -m "🔴 pressure: forget, activate, evict, curate"
```

- [ ] **Step 4: Implement forget, activate, evict, curate, is_active**

In `src/lib.rs`:

```rust
    pub fn is_active(&self, oid: &Oid) -> bool {
        self.working_set
            .lock()
            .unwrap()
            .contains(oid.as_ref())
    }

    pub fn forget(&self, oid: Oid) -> Beam<()> {
        self.working_set
            .lock()
            .unwrap()
            .remove(oid.as_ref());
        Beam::new(())
    }

    pub fn activate(&self, oid: Oid) -> Beam<()> {
        // Verify node exists
        match self.db.get(oid.as_ref()) {
            Some(_) => {
                self.working_set
                    .lock()
                    .unwrap()
                    .insert(oid.as_ref().to_string());
                Beam::new(())
            }
            None => Beam::new(())
                .with_loss(ShannonLoss::new(1.0))
                .with_recovery(Recovery::Failed {
                    reason: format!("node '{}' not found", oid),
                }),
        }
    }

    pub fn evict_under_pressure(&self) -> Beam<Vec<Oid>> {
        // Find all nodes visible through this projection's grammar
        let mut all_oids = vec![];
        for type_name in self.filter.allowed_types() {
            let result_set = self.db.find(type_name);
            all_oids.extend(result_set_to_oids(&result_set));
        }

        let working = self.working_set.lock().unwrap();
        let evictable: Vec<Oid> = all_oids
            .into_iter()
            .filter(|oid| !working.contains(oid.as_ref()))
            .collect();

        let loss_bits = evictable.len() as f64; // 1 bit per evicted node as rough measure
        Beam::new(evictable)
            .with_loss(ShannonLoss::new(loss_bits))
    }

    pub fn curate(&self, budget: Pressure) -> Beam<Vec<Oid>> {
        // Collect all visible nodes
        let mut all_oids = vec![];
        for type_name in self.filter.allowed_types() {
            let result_set = self.db.find(type_name);
            all_oids.extend(result_set_to_oids(&result_set));
        }

        let working = self.working_set.lock().unwrap();

        // Active items first (always included if budget allows)
        let mut curated: Vec<Oid> = all_oids
            .iter()
            .filter(|oid| working.contains(oid.as_ref()))
            .cloned()
            .collect();

        // Fill remaining budget with non-active items
        // Budget determines how many total items to include:
        // low pressure (0.1) = include most, high pressure (0.9) = include few
        let max_items = if budget.is_critical() {
            curated.len() // Only active items at critical pressure
        } else {
            let ratio = 1.0 - budget.as_f64();
            let total = (all_oids.len() as f64 * ratio).ceil() as usize;
            total.max(curated.len())
        };

        let remaining: Vec<Oid> = all_oids
            .into_iter()
            .filter(|oid| !working.contains(oid.as_ref()))
            .collect();

        let slots = max_items.saturating_sub(curated.len());
        curated.extend(remaining.into_iter().take(slots));

        Beam::new(curated)
    }
```

**Note:** `Pressure::as_f64()` and `Pressure::is_critical()` — verify these exist in prism. If `as_f64()` doesn't exist, check for an equivalent accessor.

- [ ] **Step 5: Run tests**

```bash
nix develop -c cargo test --test pressure_test
```

Expected: all tests pass.

- [ ] **Step 6: Commit green**

```bash
git add src/lib.rs
git commit --author="Mara <mara@systemic.engineer>" -m "🟢 pressure: forget, activate, evict, curate"
```

---

## Task 7: Consolidation (project + preview + measure)

**Files:**
- Modify: `src/lib.rs`
- Create: `tests/project_test.rs`

- [ ] **Step 1: Write failing tests**

Create `/Users/alexwolf/dev/projects/projection/tests/project_test.rs`:

```rust
use projection::Projection;
use projection::filter::GrammarFilter;
use projection::types::{NodeType, NodeData};
use tempfile::tempdir;

fn setup_two_projections() -> (tempfile::TempDir, Projection, Projection) {
    let dir = tempdir().unwrap();

    // Wide projection (episodic-like): events + observations + entities
    let wide_filter = GrammarFilter::new("episodic")
        .allow_type("event")
        .allow_type("observation")
        .allow_type("entity");
    let wide = Projection::open(dir.path(), wide_filter, "witness", 1e-6, 50_000_000)
        .expect("failed to open wide projection");

    // Narrow projection (semantic-like): entities only
    let narrow_filter = GrammarFilter::new("semantic")
        .allow_type("entity");
    let narrow = wide.with_filter(narrow_filter);

    (dir, wide, narrow)
}

#[test]
fn project_keeps_matching_types() {
    let (_dir, wide, narrow) = setup_two_projections();
    wide.store(NodeType::new_unchecked("event"), NodeData::from_str("happened"));
    wide.store(NodeType::new_unchecked("entity"), NodeData::from_str("person"));

    let beam = wide.project(&narrow);
    assert_eq!(beam.result.kept.len(), 1); // entity passes
    assert_eq!(beam.result.lost.len(), 1); // event dropped
}

#[test]
fn project_measures_loss() {
    let (_dir, wide, narrow) = setup_two_projections();
    wide.store(NodeType::new_unchecked("event"), NodeData::from_str("big event data here"));
    wide.store(NodeType::new_unchecked("entity"), NodeData::from_str("x"));

    let beam = wide.project(&narrow);
    assert!(beam.has_loss());
    // Loss should reflect the size of the dropped event data
    assert!(beam.loss.as_f64() > 0.0);
}

#[test]
fn project_identical_filters_zero_loss() {
    let (_dir, wide, _narrow) = setup_two_projections();
    wide.store(NodeType::new_unchecked("event"), NodeData::from_str("e"));

    // Project onto self — everything passes
    let same = wide.with_filter(
        GrammarFilter::new("same")
            .allow_type("event")
            .allow_type("observation")
            .allow_type("entity"),
    );
    let beam = wide.project(&same);
    assert!(beam.is_lossless());
    assert!(beam.result.lost.is_empty());
}

#[test]
fn preview_snapshots_current_state() {
    let (_dir, wide, _narrow) = setup_two_projections();
    wide.store(NodeType::new_unchecked("event"), NodeData::from_str("a"));
    wide.store(NodeType::new_unchecked("entity"), NodeData::from_str("b"));

    let beam = wide.preview();
    assert_eq!(beam.result.node_count(), 2);
    assert!(beam.is_lossless());
}

#[test]
fn preview_respects_filter() {
    let (_dir, wide, narrow) = setup_two_projections();
    wide.store(NodeType::new_unchecked("event"), NodeData::from_str("a"));
    wide.store(NodeType::new_unchecked("entity"), NodeData::from_str("b"));

    let beam = narrow.preview();
    // Only entity visible through narrow filter
    assert_eq!(beam.result.node_count(), 1);
}

#[test]
fn measure_compares_projections() {
    let (_dir, wide, narrow) = setup_two_projections();
    wide.store(NodeType::new_unchecked("event"), NodeData::from_str("a"));
    wide.store(NodeType::new_unchecked("entity"), NodeData::from_str("b"));

    let beam = wide.measure(&narrow);
    // Delta should show what narrow sees vs what wide sees
    assert!(!beam.result.kept.is_empty());
    assert!(!beam.result.lost.is_empty());
}
```

- [ ] **Step 2: Verify tests compile but fail**

```bash
nix develop -c cargo test --test project_test
```

Expected: compiles, all tests panic with `not yet implemented`.

- [ ] **Step 3: Commit red**

```bash
git add tests/project_test.rs
git commit --author="Mara <mara@systemic.engineer>" -m "🔴 consolidation: project, preview, measure"
```

- [ ] **Step 4: Implement project, preview, measure**

In `src/lib.rs`:

```rust
    pub fn project(&self, target: &Projection) -> Beam<ProjectionDelta> {
        let mut kept = vec![];
        let mut lost = vec![];
        let mut total_loss_bits = 0.0;

        for type_name in self.filter.allowed_types() {
            let result_set = self.db.find(type_name);
            for oid in result_set_to_oids(&result_set) {
                if let Some(node) = self.db.get(oid.as_ref()) {
                    if target.filter.allowed_types().contains(&node.node_type) {
                        kept.push(oid);
                    } else {
                        total_loss_bits += node.data.len() as f64 * 8.0;
                        lost.push(oid);
                    }
                }
            }
        }

        let beam = Beam::new(ProjectionDelta { kept, lost });
        if total_loss_bits > 0.0 {
            beam.with_loss(ShannonLoss::new(total_loss_bits))
        } else {
            beam
        }
    }

    pub fn preview(&self) -> Beam<SubgraphSnapshot> {
        let mut nodes = vec![];
        let mut edges = vec![];

        for type_name in self.filter.allowed_types() {
            let result_set = self.db.find(type_name);
            nodes.extend(result_set_to_oids(&result_set));
        }

        // Collect edges between visible nodes
        let node_set: HashSet<String> = nodes.iter().map(|o| o.as_ref().to_string()).collect();
        for (from, to, _weight) in self.db.edges_weighted() {
            if node_set.contains(&from) && node_set.contains(&to) {
                edges.push((Oid::new(&from), Oid::new(&to)));
            }
        }

        Beam::new(SubgraphSnapshot { nodes, edges })
    }

    pub fn measure(&self, actual: &Projection) -> Beam<ProjectionDelta> {
        // measure: what does `self` have that `actual` doesn't, and vice versa
        // Same as project — what survives the target's filter
        self.project(actual)
    }
```

- [ ] **Step 5: Run tests**

```bash
nix develop -c cargo test --test project_test
```

Expected: all tests pass.

- [ ] **Step 6: Commit green**

```bash
git add src/lib.rs
git commit --author="Mara <mara@systemic.engineer>" -m "🟢 consolidation: project, preview, measure"
```

---

## Task 8: Crystallize

**Files:**
- Modify: `src/lib.rs`
- Modify: `tests/pressure_test.rs`

- [ ] **Step 1: Add failing tests to pressure_test.rs**

Append to `/Users/alexwolf/dev/projects/projection/tests/pressure_test.rs`:

```rust
#[test]
fn crystallize_marks_node() {
    let (_dir, proj) = setup(&["event"]);
    let a = proj.store(NodeType::new_unchecked("event"), NodeData::from_str("important"));
    let beam = proj.crystallize(a.result.clone());
    assert!(beam.is_lossless());
}

#[test]
fn crystallize_nonexistent_returns_loss() {
    let (_dir, proj) = setup(&["event"]);
    let beam = proj.crystallize(Oid::new("nonexistent"));
    assert!(beam.has_loss());
}

#[test]
fn recall_procedural_returns_crystallized() {
    let (_dir, proj) = setup(&["event", "pattern"]);
    let a = proj.store(NodeType::new_unchecked("event"), NodeData::from_str("trigger"));
    let b = proj.store(NodeType::new_unchecked("pattern"), NodeData::from_str("response"));
    proj.connect(a.result.clone(), b.result.clone());
    proj.crystallize(b.result.clone());

    let beam = proj.recall_procedural(&[a.result.clone()]);
    // Should find crystallized nodes connected to the active context
    assert!(!beam.result.is_empty());
}
```

- [ ] **Step 2: Verify tests compile but fail**

```bash
nix develop -c cargo test --test pressure_test
```

Expected: new tests panic with `not yet implemented`.

- [ ] **Step 3: Commit red**

```bash
git add tests/pressure_test.rs
git commit --author="Mara <mara@systemic.engineer>" -m "🔴 crystallize: procedural memory promotion"
```

- [ ] **Step 4: Implement crystallize and recall_procedural**

In `src/lib.rs`:

```rust
    pub fn crystallize(&self, oid: Oid) -> Beam<()> {
        match self.db.get(oid.as_ref()) {
            Some(_node) => {
                // Mark as active (crystallized nodes stay in working set)
                self.working_set
                    .lock()
                    .unwrap()
                    .insert(oid.as_ref().to_string());
                // Trigger spectral-db's crystallization for hot paths
                self.db.crystallize();
                Beam::new(())
            }
            None => Beam::new(())
                .with_loss(ShannonLoss::new(1.0))
                .with_recovery(Recovery::Failed {
                    reason: format!("node '{}' not found", oid),
                }),
        }
    }

    pub fn recall_procedural(&self, active_oids: &[Oid]) -> Beam<Vec<Oid>> {
        // Find crystallized nodes connected to the active context
        let crystals = self.db.crystals();
        let crystal_oids: HashSet<String> = crystals
            .iter()
            .flat_map(|c| {
                // Extract OIDs from Crystal — check crystallize.rs for fields
                // This likely has a path or oids field
                vec![c.path.clone()] // Adjust based on Crystal struct
            })
            .collect();

        // Walk from active OIDs and find crystallized neighbors
        let mut found = vec![];
        for oid in active_oids {
            for neighbor in self.db.neighbors(oid.as_ref()) {
                if crystal_oids.contains(&neighbor) {
                    found.push(Oid::new(&neighbor));
                }
            }
        }

        found.dedup();
        Beam::new(found)
    }
```

**Note:** Read `spectral-db/src/crystallize.rs` to understand the `Crystal` struct fields. The implementation above assumes Crystal has a `path` field containing an OID string — adjust to match the actual struct.

- [ ] **Step 5: Run tests**

```bash
nix develop -c cargo test --test pressure_test
```

Expected: all tests pass.

- [ ] **Step 6: Commit green**

```bash
git add src/lib.rs tests/pressure_test.rs
git commit --author="Mara <mara@systemic.engineer>" -m "🟢 crystallize: procedural memory promotion"
```

---

## Task 9: Export + Ingest

**Files:**
- Modify: `src/export.rs`
- Modify: `src/lib.rs` (add export/ingest methods)
- Create: `tests/export_test.rs`

- [ ] **Step 1: Write failing tests**

Create `/Users/alexwolf/dev/projects/projection/tests/export_test.rs`:

```rust
use projection::Projection;
use projection::export::ExportFormat;
use projection::filter::GrammarFilter;
use projection::types::{NodeType, NodeData};
use std::fs;
use tempfile::tempdir;

fn setup(types: &[&str]) -> (tempfile::TempDir, Projection) {
    let dir = tempdir().unwrap();
    let mut filter = GrammarFilter::new("test");
    for t in types {
        filter = filter.allow_type(t);
    }
    let proj = Projection::open(dir.path(), filter, "test-witness", 1e-6, 50_000_000)
        .expect("failed to open projection");
    (dir, proj)
}

#[test]
fn export_markdown_creates_files() {
    let (dir, proj) = setup(&["event"]);
    proj.store(NodeType::new_unchecked("event"), NodeData::from_str("test memory"));

    let export_dir = dir.path().join("export");
    fs::create_dir_all(&export_dir).unwrap();

    let beam = proj.export_to(&export_dir, ExportFormat::Markdown);
    assert!(beam.is_lossless());

    // Should create an index file
    let index = export_dir.join("MEMORY.md");
    assert!(index.exists());
}

#[test]
fn export_markdown_contains_frontmatter() {
    let (dir, proj) = setup(&["event"]);
    proj.store(NodeType::new_unchecked("event"), NodeData::from_str("important thing"));

    let export_dir = dir.path().join("export");
    fs::create_dir_all(&export_dir).unwrap();

    proj.export_to(&export_dir, ExportFormat::Markdown);

    // Read one of the exported files (not the index)
    let entries: Vec<_> = fs::read_dir(&export_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name() != "MEMORY.md")
        .collect();
    assert!(!entries.is_empty());

    let content = fs::read_to_string(entries[0].path()).unwrap();
    assert!(content.contains("---"));
    assert!(content.contains("type: event"));
}

#[test]
fn ingest_reads_markdown_files() {
    let (dir, proj) = setup(&["event"]);

    // Create a markdown memory file
    let import_dir = dir.path().join("import");
    fs::create_dir_all(&import_dir).unwrap();
    fs::write(
        import_dir.join("test_memory.md"),
        "---\nname: test memory\ntype: event\n---\n\nSome content here.\n",
    )
    .unwrap();

    let beam = proj.ingest_from(&import_dir);
    assert!(beam.is_lossless());
    assert_eq!(beam.result.len(), 1);

    // Verify it was stored
    let read_beam = proj.read(beam.result[0].clone());
    assert!(read_beam.result.is_some());
}

#[test]
fn ingest_skips_invalid_types() {
    let (dir, proj) = setup(&["event"]); // only "event" allowed

    let import_dir = dir.path().join("import");
    fs::create_dir_all(&import_dir).unwrap();
    fs::write(
        import_dir.join("wrong_type.md"),
        "---\nname: wrong\ntype: entity\n---\n\nWon't be ingested.\n",
    )
    .unwrap();

    let beam = proj.ingest_from(&import_dir);
    assert!(beam.has_loss());
    assert!(beam.result.is_empty());
}

#[test]
fn export_then_ingest_roundtrip() {
    let (dir, proj) = setup(&["event"]);
    proj.store(NodeType::new_unchecked("event"), NodeData::from_str("roundtrip"));

    let export_dir = dir.path().join("roundtrip");
    fs::create_dir_all(&export_dir).unwrap();

    proj.export_to(&export_dir, ExportFormat::Markdown);

    // Create a fresh projection and ingest
    let dir2 = tempdir().unwrap();
    let proj2 = Projection::open(
        dir2.path(),
        GrammarFilter::new("test").allow_type("event"),
        "other-witness",
        1e-6,
        50_000_000,
    )
    .unwrap();

    let beam = proj2.ingest_from(&export_dir);
    assert!(!beam.result.is_empty());
}
```

- [ ] **Step 2: Write export module stubs**

Write `/Users/alexwolf/dev/projects/projection/src/export.rs`:

```rust
use std::path::Path;
use prism::{Beam, Oid};

#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    Markdown,
    Json,
}

pub struct MemoryFile {
    pub name: String,
    pub node_type: String,
    pub content: String,
}

pub fn export_nodes(
    _nodes: &[(String, String, Vec<u8>)], // (oid, type, data)
    _dir: &Path,
    _format: ExportFormat,
) -> Beam<()> {
    todo!()
}

pub fn ingest_markdown(_dir: &Path) -> Vec<MemoryFile> {
    todo!()
}
```

Add methods to Projection in `src/lib.rs`:

```rust
    pub fn export_to(&self, _dir: &Path, _format: ExportFormat) -> Beam<()> {
        todo!()
    }

    pub fn ingest_from(&self, _dir: &Path) -> Beam<Vec<Oid>> {
        todo!()
    }
```

Add to the use statements at the top of lib.rs:

```rust
use std::path::Path;
use crate::export::ExportFormat;
```

- [ ] **Step 3: Verify tests compile but fail**

```bash
nix develop -c cargo test --test export_test
```

Expected: compiles, all tests panic.

- [ ] **Step 4: Commit red**

```bash
git add src/export.rs src/lib.rs tests/export_test.rs
git commit --author="Mara <mara@systemic.engineer>" -m "🔴 export: markdown export and ingest"
```

- [ ] **Step 5: Implement export module**

Write `/Users/alexwolf/dev/projects/projection/src/export.rs`:

```rust
use std::fs;
use std::path::Path;
use prism::{Beam, Oid, ShannonLoss};

#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    Markdown,
    Json,
}

pub struct MemoryFile {
    pub name: String,
    pub node_type: String,
    pub content: String,
}

pub fn export_markdown(
    nodes: &[(String, String, Vec<u8>)], // (oid, type, data)
    dir: &Path,
) -> Beam<()> {
    let mut index_lines = vec!["# Memory".to_string(), String::new()];

    for (oid, node_type, data) in nodes {
        let content = String::from_utf8_lossy(data);
        let short_oid = if oid.len() > 8 { &oid[..8] } else { oid };
        let filename = format!("{}_{}.md", node_type, short_oid);

        let file_content = format!(
            "---\nname: {}\ntype: {}\noid: {}\n---\n\n{}\n",
            short_oid, node_type, oid, content
        );

        fs::write(dir.join(&filename), file_content)
            .unwrap_or_else(|e| eprintln!("failed to write {}: {}", filename, e));

        index_lines.push(format!("- [{}]({}) — {}", short_oid, filename, node_type));
    }

    fs::write(dir.join("MEMORY.md"), index_lines.join("\n"))
        .unwrap_or_else(|e| eprintln!("failed to write MEMORY.md: {}", e));

    Beam::new(())
}

pub fn ingest_markdown(dir: &Path) -> Vec<MemoryFile> {
    let mut files = vec![];

    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return files,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().map(|e| e == "md").unwrap_or(false)
            && path.file_name().map(|n| n != "MEMORY.md").unwrap_or(false)
        {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Some(mf) = parse_memory_file(&content) {
                    files.push(mf);
                }
            }
        }
    }

    files
}

fn parse_memory_file(content: &str) -> Option<MemoryFile> {
    // Parse YAML-ish frontmatter between --- delimiters
    let parts: Vec<&str> = content.splitn(3, "---").collect();
    if parts.len() < 3 {
        return None;
    }

    let frontmatter = parts[1].trim();
    let body = parts[2].trim();

    let mut name = String::new();
    let mut node_type = String::new();

    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("name:") {
            name = val.trim().to_string();
        } else if let Some(val) = line.strip_prefix("type:") {
            node_type = val.trim().to_string();
        }
    }

    if node_type.is_empty() {
        return None;
    }

    Some(MemoryFile {
        name,
        node_type,
        content: body.to_string(),
    })
}
```

- [ ] **Step 6: Implement export_to and ingest_from on Projection**

In `src/lib.rs`:

```rust
    pub fn export_to(&self, dir: &Path, format: ExportFormat) -> Beam<()> {
        let mut nodes = vec![];
        for type_name in self.filter.allowed_types() {
            let result_set = self.db.find(type_name);
            for oid in result_set_to_oids(&result_set) {
                if let Some(node) = self.db.get(oid.as_ref()) {
                    nodes.push((node.oid, node.node_type, node.data));
                }
            }
        }

        match format {
            ExportFormat::Markdown => crate::export::export_markdown(&nodes, dir),
            ExportFormat::Json => {
                // Phase 2: JSON export via @mcp's out @json
                Beam::new(())
            }
        }
    }

    pub fn ingest_from(&self, dir: &Path) -> Beam<Vec<Oid>> {
        let files = crate::export::ingest_markdown(dir);
        let mut oids = vec![];
        let mut total_loss_bits = 0.0;

        for file in files {
            match self.filter.validate_type(&file.node_type) {
                Some(node_type) => {
                    let data = NodeData::from_str(&file.content);
                    let beam = self.store(node_type, data);
                    if !beam.result.as_ref().is_empty() {
                        oids.push(beam.result);
                    }
                }
                None => {
                    total_loss_bits += file.content.len() as f64 * 8.0;
                }
            }
        }

        let beam = Beam::new(oids);
        if total_loss_bits > 0.0 {
            beam.with_loss(ShannonLoss::new(total_loss_bits))
        } else {
            beam
        }
    }
```

- [ ] **Step 7: Run tests**

```bash
nix develop -c cargo test --test export_test
```

Expected: all tests pass.

- [ ] **Step 8: Commit green**

```bash
git add src/lib.rs src/export.rs
git commit --author="Mara <mara@systemic.engineer>" -m "🟢 export: markdown export and ingest"
```

---

## Task 10: Projection Composition

**Files:**
- Modify: `src/lib.rs`
- Create: `tests/compose_test.rs`

- [ ] **Step 1: Write failing tests**

Create `/Users/alexwolf/dev/projects/projection/tests/compose_test.rs`:

```rust
use projection::Projection;
use projection::filter::GrammarFilter;
use projection::types::{NodeType, NodeData};
use tempfile::tempdir;

#[test]
fn compose_narrows_filter() {
    let dir = tempdir().unwrap();
    let base_filter = GrammarFilter::new("memory")
        .allow_type("event")
        .allow_type("entity")
        .allow_type("fact");
    let base = Projection::open(dir.path(), base_filter, "base", 1e-6, 50_000_000).unwrap();

    // Store nodes of all types
    base.store(NodeType::new_unchecked("event"), NodeData::from_str("e"));
    base.store(NodeType::new_unchecked("entity"), NodeData::from_str("n"));
    base.store(NodeType::new_unchecked("fact"), NodeData::from_str("f"));

    // Compose with agent filter that only sees entities
    let agent_filter = GrammarFilter::new("mara")
        .allow_type("entity")
        .allow_type("fact");
    let composed = base.compose(agent_filter, "mara-witness");

    // Composed should only see entity and fact (intersection)
    let events = composed.find(NodeType::new_unchecked("event"));
    assert!(events.result.is_empty());

    let entities = composed.find(NodeType::new_unchecked("entity"));
    assert_eq!(entities.result.len(), 1);

    let facts = composed.find(NodeType::new_unchecked("fact"));
    assert_eq!(facts.result.len(), 1);
}

#[test]
fn compose_changes_witness() {
    let dir = tempdir().unwrap();
    let base_filter = GrammarFilter::new("memory").allow_type("event");
    let base = Projection::open(dir.path(), base_filter, "base", 1e-6, 50_000_000).unwrap();

    let composed = base.compose(
        GrammarFilter::new("mara").allow_type("event"),
        "mara-witness",
    );
    assert_eq!(composed.witness(), "mara-witness");
}

#[test]
fn compose_shares_underlying_db() {
    let dir = tempdir().unwrap();
    let filter = GrammarFilter::new("test").allow_type("event");
    let base = Projection::open(dir.path(), filter, "base", 1e-6, 50_000_000).unwrap();

    base.store(NodeType::new_unchecked("event"), NodeData::from_str("shared"));

    let composed = base.compose(
        GrammarFilter::new("agent").allow_type("event"),
        "agent",
    );

    // Composed sees what base stored (same db)
    let found = composed.find(NodeType::new_unchecked("event"));
    assert_eq!(found.result.len(), 1);
}

#[test]
fn compose_independent_working_sets() {
    let dir = tempdir().unwrap();
    let filter = GrammarFilter::new("test").allow_type("event");
    let base = Projection::open(dir.path(), filter, "base", 1e-6, 50_000_000).unwrap();

    let a = base.store(NodeType::new_unchecked("event"), NodeData::from_str("shared"));
    base.activate(a.result.clone());

    let composed = base.compose(
        GrammarFilter::new("agent").allow_type("event"),
        "agent",
    );

    // base has it active, composed does not
    assert!(base.is_active(&a.result));
    assert!(!composed.is_active(&a.result));
}
```

- [ ] **Step 2: Add compose method stub to Projection**

In `src/lib.rs`, add:

```rust
    pub fn compose(&self, _identity_filter: GrammarFilter, _witness: &str) -> Projection {
        todo!()
    }
```

- [ ] **Step 3: Verify tests compile but fail**

```bash
nix develop -c cargo test --test compose_test
```

Expected: compiles, all tests panic.

- [ ] **Step 4: Commit red**

```bash
git add src/lib.rs tests/compose_test.rs
git commit --author="Mara <mara@systemic.engineer>" -m "🔴 compose: projection composition with witness scoping"
```

- [ ] **Step 5: Implement compose**

In `src/lib.rs`:

```rust
    pub fn compose(&self, identity_filter: GrammarFilter, witness: &str) -> Projection {
        let narrowed = self.filter.intersect(&identity_filter);
        Projection {
            filter: narrowed,
            db: Arc::clone(&self.db),
            witness: witness.to_string(),
            working_set: Mutex::new(HashSet::new()),
        }
    }
```

- [ ] **Step 6: Run tests**

```bash
nix develop -c cargo test --test compose_test
```

Expected: all tests pass.

- [ ] **Step 7: Commit green**

```bash
git add src/lib.rs
git commit --author="Mara <mara@systemic.engineer>" -m "🟢 compose: projection composition with witness scoping"
```

---

## Task 11: Full Test Suite + Clippy + Format

**Files:**
- All files

- [ ] **Step 1: Run complete test suite**

```bash
nix develop -c cargo test
```

Expected: all tests across all test files pass.

- [ ] **Step 2: Run clippy**

```bash
nix develop -c cargo clippy -- -D warnings
```

Fix any warnings. Common issues:
- `needless_range_loop` — use `.iter().enumerate()` instead of `for i in 0..n`
- `redundant_clone` — remove unnecessary `.clone()` calls
- Missing `#[must_use]` on constructors

- [ ] **Step 3: Run format check**

```bash
nix develop -c cargo fmt -- --check
```

If it fails, run `nix develop -c cargo fmt` and verify changes look correct.

- [ ] **Step 4: Commit any fixes**

```bash
git add -A
git commit --author="Mara <mara@systemic.engineer>" -m "♻️ clippy and fmt cleanup"
```

- [ ] **Step 5: Verify final state**

```bash
nix develop -c cargo test && nix develop -c cargo clippy -- -D warnings && nix develop -c cargo fmt -- --check
```

Expected: all three pass clean.

---

## Implementer Notes

### ResultSet extraction

The plan uses a `result_set_to_oids()` helper throughout. You **must** read `spectral-db/src/query.rs` before Task 5 to understand how `ResultSet` provides OID access. The helper's implementation depends entirely on ResultSet's API.

### Crystal struct

Task 8 accesses fields on `crystallize::Crystal`. Read `spectral-db/src/crystallize.rs` to understand the struct layout before implementing `recall_procedural`.

### Pressure accessors

Task 6 uses `Pressure::as_f64()` and `Pressure::is_critical()`. Verify these exist in `prism/src/precision.rs`. If the accessor is named differently, adjust.

### spectral-db schema validation

`GrammarFilter::to_schema()` generates a `.conv` string that spectral-db parses via conversation. If the generated schema doesn't parse (e.g., `type = any` isn't valid .conv), you'll need to adjust the format. Test this in Task 4 when `Projection::open` first calls `SpectralDb::open`.

### Error handling

Phase 1 converts spectral-db errors to `Beam<T>` with `Recovery::Failed`. This is pragmatic but coarse. Phase 3 (@ca integration) will add structured observation of failures.
