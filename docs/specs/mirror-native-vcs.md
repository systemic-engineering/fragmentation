# mirror-native VCS — fragmentation as the VCS-agnostic content store, jj as native, git as interop

*2026-05-24. Mara. Spec.*

Status: **Red.** The destination is named. The current state is audited file-by-file.
The minimal viable surface is fixed. The jj backend alignment is grounded in jj's
actual `Backend` trait. The decomposition is tick-shaped. No code lands.

Depends on:
- `fragmentation/Cargo.toml`, `fragmentation/src/*.rs` — the substrate being
  audited; the manifest carrying the `git`/`ssh`/`gpg`/`fuse`/`cli`/`fuse-mount`
  feature flags this spec rebalances.
- `fragmentation/docs/specs/fragmentation-vcs-spec.md` — the April 14 draft that
  named the git-isomorphism. This spec subsumes it and reframes: fragmentation is
  NOT git-compatible-by-default; fragmentation IS the substrate, git is an
  outlet, jj is native.
- `fragmentation/ROADMAP.md` — `frgmt collapse`/`refract`/`portal`; the
  `NakedSingularity` trajectory; the "Beyond Git" section (Pijul/jj/Irmin/Darcs
  cross-references).
- `fragmentation-git/Cargo.toml`, `fragmentation-git/src/*.rs` — the extraction
  that already happened in commit `f1e1135` (`extract git code to
  fragmentation-git`). This spec finishes the extraction and names what's left
  in the wrong place.
- `mirror/docs/specs/mirror-store.md` — the May 22 audit that landed three of
  this spec's findings (Cut 1, Cut 2, Cut 3 of the fragmentation cleanup). This
  spec carries those forward and adds the jj-as-native finding.
- `/tmp/jj_backend.rs` (`https://raw.githubusercontent.com/jj-vcs/jj/main/lib/src/backend.rs`)
  — jj's `Backend` trait; the exact contract a `fragmentation-jj` adapter must
  satisfy.
- `/tmp/jj_git_backend.rs` (`https://raw.githubusercontent.com/jj-vcs/jj/main/lib/src/git_backend.rs`)
  — jj's reference `Backend` impl over git objects; how it stores `change_id`
  (as an extra git commit header), how it bridges its richer model to git's
  poorer one.

Unblocks:
- The mirror F-2 tick (per `mirror/docs/specs/mirror-store.md` §6.4): the
  fragmentation cleanup this spec orders is the load-bearing pre-requisite.
- The `spectral-db` task (#43): spectral-db consumes the same
  `fragmentation`-as-substrate surface. The cuts here remove the git-flavored
  cruft that would otherwise contaminate spectral-db's adoption.
- The mirror daily-driver workflow: once `fragmentation-jj` lands, mirror has a
  real VCS that records grammar/source history at the resolution the substrate
  affords (per-grammar OIDs, per-witness commits), not the resolution git
  permits (whole-file blobs).

---

## 1. The recognition

Fragmentation today is presented as a "content-addressed, arbitrary-depth,
circular-reflexive tree" library that happens to ship git compatibility, a
FUSE portal, witnessed commits, signing, encryption, projections, supervision
trees, and a CLI binary. Read the source: it is already a VCS, and most of
the non-tree machinery is the implicit substrate of one. `Shard`/`Fractal`/`Lens`
are blob/tree/submodule. `Witnessed` is signature metadata. `Commit<N,H>` is a
git commit. `FrgmntStore` is `.git/objects` with eviction. `Repo` is
`update_ref`/`resolve_ref`. There is no "versioning" feature to add; the
versioning IS what's already there. What's missing is the recognition that
this IS what's there, and the cleanup that follows from naming it.

The destination — in two paragraphs:

**Fragmentation is the VCS-agnostic content-backed store for mirror.** It owns
the content primitives: content-addressed `Shard`/`Fractal`/`Lens` nodes
with pluggable hash (`HashAlg`) and pluggable encoding (`Encode`/`Decode`),
plus the storage backends to put them somewhere
(`ConcurrentStore`/`BoundedStore`/`FrgmntStore`/in-memory `Store`). It does
NOT own git, jj, or any other VCS opinion; those are translation layers
that consume fragmentation. A mirror-side `MirrorStore` (per
`mirror/docs/specs/mirror-store.md` §3) wraps fragmentation's primitives into
the mirror Lift dispatch's six-function surface. A spectral-db crate wraps
the same primitives into the garden's distribution/delta/conflict layer. Both
are peers above fragmentation, not extensions of it.

**jj is the native daily-driver VCS for mirror; git is the interop bridge.** jj
is a content-addressed VCS with a `Backend` trait built for substitution — its
own `GitBackend` is an implementation, not the canonical store. fragmentation's
`Shard`/`Fractal`/`Lens` primitives map directly onto jj's `FileId`/`TreeId`/
`CommitId` shape with one substantive substrate gap (the change-id layer, §4).
git's blob/tree/commit model maps onto fragmentation's primitives too, but the
mapping is lossier — git has no change-id, no operation log, no native conflict
storage, no per-file witness. fragmentation-git is a translation layer for
codeforge interop (push/pull/clone with GitHub/GitLab), not the daily workflow.

This move is structural, not aspirational. The library has primitives that
are too rich for git's wire format and too poor for arbitrary key-value
storage. They sit at exactly the level where a substitutable VCS substrate
lives. The recognition is naming the level it already sits at.

Why this destination and not "key-value store" or "object database":

- A key-value store has no native notion of `Fragmentable` recursion — it
  hands the caller back a value and an opaque key. fragmentation hashes
  recursively; the tree shape is intrinsic. K/V is what's underneath, not
  what fragmentation IS.
- An object database typically owns its schema — Mnesia, RocksDB, FoundationDB
  expose tables and indexes. fragmentation has no schema; the schema is the
  recursive type that implements `Fragmentable`. The hash IS the index. The
  tree IS the table. An object DB is again what's underneath.
- A VCS-agnostic content-backed store sits at the level where a `Backend`
  trait makes sense: it answers "given an OID, return its content" and
  "given content, return its OID" with type and structure information about
  what the content IS (shard? tree? lens? commit?). That's exactly what jj's
  `Backend` trait answers. fragmentation already speaks that vocabulary.

The thing the types don't say: **fragmentation has been waiting to become a
VCS substrate.** Every Rust import (`git2`, `dashmap`, `fuser`,
`ssh-key`/`x25519-dalek`/`chacha20poly1305`) is a feature it grew to *avoid
being* the substrate — extensions piled on a tree library because nobody
named the level it was already living at. Name the level. The extensions sort
themselves into their right crates by gravity.

---

## 2. Current state audit — the simplification round

File-by-file walk of `/Users/alexwolf/dev/projects/fragmentation/src/` (the
tree at HEAD of `mara/prism-bridge`). For each module: what it does today,
where it belongs in the destination, what retires. Modules are grouped by the
decision (STAYS / MOVES / RETIRES) at the end. Per-file rationale up top.

### 2.1 The per-file walk

**`lib.rs`** — module index. Will need `cfg(feature = ...)` gating to remove
the optional modules' compile-time presence (`git`, `fuse`, etc. are already
feature-gated; `prism_bridge` and `keys` aren't but should be). **STAYS,
rewritten** to expose the new module layout (§3).

**`fragment.rs`** (18 KB) — `Fragmentable` trait, `Fractal<E, H>` enum,
`Reconstructable` trait, `merge()` with caller-provided conflict resolver,
`content_oid`/`blob_oid`/`tree_oid`/`lens_oid` computing git-compatible
SHA-1 hashes. **The core primitive. STAYS.** Two adjustments needed (per
`mirror-store.md` Cut 2 + Cut 3): split `Fragmentable` into `ContentAddressed`
(small required surface: `self_ref`, `data`, `encode`) plus optional
`TreeShaped` (the `is_shard`/`is_fractal`/`children`/`targets` accessors).
Rename `Fractal::Fractal` to `Fractal::Branch` for clarity. The OID functions
stay git-compatible by *default* but the hash is pluggable; renaming them
from `blob_oid`/`tree_oid` to `shard_oid_git`/`branch_oid_git` honors the
fact that they're one serialization choice, not the substrate's only choice.

**`sha.rs`** (2 KB) — `HashAlg` trait, `Sha` (SHA-1) implementation, hash()
helper. **STAYS.** The trait is the pluggability the spec relies on. Add
clear documentation that `Sha` is one impl among many (per
`coincidence::CoincidenceHash<5,5>`, jj's `Blake2b<32>` in the default
backend, future SHA-256 work).

**`ref_.rs`** (0.8 KB) — `Ref<H>` wraps `(hash, label)`. **STAYS.** Tiny.
Label is the human-readable name; hash is the address. Used everywhere.

**`cid.rs`** (3.7 KB) — `Cid<H>` self-describing identifier wrapping `Ref<H>`
with `Codec` and `HashId` discriminators. Per `ROADMAP.md` item 10
("Self-describing identifiers — done"). **STAYS.** This is the
forward-compatibility envelope for codec variation. Not yet load-bearing for
v1 but cheap to keep.

**`encoding.rs`** (4.3 KB) — `Encode`/`Decode` traits + the five-level
document/paragraph/sentence/word/char text encoding. **STAYS.** The trait is
core; the text encoder is one impl. Document the text encoder as a reference
implementation, not the substrate's only encoder. (mirror has `Combinator`
encoding; spectral-db will have its own; future spectral-rendering grammars
will have more.)

**`walk.rs`** (1.7 KB) — `Visitor<A>`, `collect`/`fold`/`depth`/`find`.
**STAYS.** The walk surface a content-addressed tree library has to expose.
Generic over `Fragmentable`. Zero git in it.

**`diff.rs`** (2 KB) — positional structural diff between two `Fractal` trees,
O(log n) by content-OID early cutoff. **STAYS.** Generic over `Fragmentable`,
zero git. The three-way merge (per `fragmentation-vcs-spec.md` §3.5) is the
next extension and belongs here.

**`store.rs`** (6.9 KB) — in-memory `Store<N, H>` with objects + commits +
refs `HashMap`s. Implements the `Repo` trait. **STAYS, REFRAMED.** This is
the reference in-memory backend. Rename `Store` → `MemoryStore` so the type's
name reflects its role; reserve `Store` for the trait-object form that the
backends implement (or drop `Store` as a name entirely — `Repo` already covers
the abstraction).

**`repo.rs`** (3.7 KB) — `Repo` trait: `write_tree`/`read_tree`/`write_commit`/
`read_commit`/`update_ref`/`resolve_ref`. **STAYS.** This is the trait jj's
`Backend` aligns to (with adjustments — see §4). One adjustment needed:
`Repo` today returns `String` for OIDs and uses string ref names; align
to typed `Oid` and `Reference` types (matches mirror's `MirrorStore` per
`mirror-store.md` §3.1).

**`commit.rs`** (8.8 KB) — `Draft<N, H>` + `Commit<N, H>` (Root/Child variants)
+ `Draftable` trait + `compute_commit_sha` (mirrors git's commit object format
exactly, including `tree`/`parent`/`author`/`committer`/blank-line/message).
**STAYS, REFRAMED.** The `compute_commit_sha` function is git-format-specific;
moves to `fragmentation-git`. The `Draft`/`Commit`/`Draftable` types stay
generic — they hold structure, not format. Add a `compute_commit_oid<H:
HashAlg>` that's format-agnostic (just hashes the canonical serialized form
of the typed parts); git-format computation becomes `fragmentation-git`'s
impl of a `CommitFormat` trait.

**`bounded_store.rs`** (6.2 KB) — byte-bounded LIFO-evicting cache,
`DashMap`-backed. **STAYS.** Generic in-memory cache primitive. Per
`mirror-store.md` Cut 1, `dashmap` becomes feature-gated behind `concurrent`
(default-on). Single-threaded `RefCell<HashMap>` fallback for no_std + alloc.

**`concurrent_store.rs`** (8.1 KB) — `ConcurrentStore<N, H>`, lock-free
reads, shard-locked writes, `Send + Sync`. **STAYS.** Same Cut-1 treatment.
Generic primitive.

**`frgmnt_store.rs`** (11 KB) — `.frgmnt/` on-disk store + `BoundedStore`
cache, persistence via `write_to_disk`/`read_from_disk`. **STAYS.** The
default on-disk backend for fragmentation. Zero git in it. The `.frgmnt/`
directory shape (`objects/` with fan-out by first 2 hex chars, `refs/` plain
text) is the on-disk substrate format. Document explicitly: this format is
fragmentation's, not git's; git happens to have the same layout but the
contents differ (raw `Encode::encode()` bytes here vs zlib-compressed
zobject-headered bytes in `.git/`).

**`witnessed.rs`** (1.4 KB) — `Author`, `Committer`, `Timestamp`, `Message`,
`Witnessed`. **STAYS.** Generic witness vocabulary. Identical to git's
signature shape but trivially also fits jj's `Signature` struct.

**`prism_bridge.rs`** (9.2 KB) — `impl HashAlg for prism_core::Oid`,
`impl Addressable for Fractal<E,H>`, `impl MerkleTree for Fractal<E,H>`,
`StoreLoss` (Loss impl for prism-core), `impl Store for FrgmntStore<...>`.
**STAYS.** This is the bridge to prism — the pipeline DSL fragmentation
emits Imperfect into. Feature-gate behind `prism-bridge` (which it isn't
today — it's unconditional via the workspace dependency on `prism-core`).
Makes the no_std stretch cleaner.

**`naked.rs`** (15.8 KB) — `NakedSingularity<E, H>`, `NakedArtifact<H>`, dual
OID (content + naked), implements `Singularity` trait, has its own serialized
format with `|` separator + escape rules + a 280-line byte walker to skip past
serialized fractals. **STAYS, ISOLATED.** This is a substantive design statement
(per ROADMAP §"The Singularity Gradient") and `Singularity` lives in
`singularity.rs`. The artifact format is its own — has nothing to do with git
or jj. Keep, but make it clear in module docs that this is one of several
collapse modes (Tree/WitnessedCommit/Naked), with its own serialization.

**`singularity.rs`** (24 KB) — `Singularity` trait, identity `Fractal`
implementation, `WitnessedSingularity<R: Repo>` that creates a commit whose
node is a Lens. **STAYS, NARROWED.** The trait is core. The
`WitnessedSingularity` implementation wraps a `Repo` — but the implementation
is Lens-construction logic, NOT git logic. Stays generic. The 24KB is mostly
tests and docs; the surface is small.

**`manifest.rs`** (1.6 KB) — `LensEntry` + `Manifest` describing
name→target-OID mappings for projecting files. **STAYS.** Tiny, generic.

**`project.rs`** (5.1 KB) — file-projection: reads files from a source dir,
computes their blob OIDs, maps to target paths via a Manifest. **STAYS,
NARROWED.** "Compute blob OID" is fragmentation-side; the `Encode` is
generic. The current implementation calls `blob_oid` (SHA-1 git-formatted);
that's an implementation choice. Generic up: `compute_oid<H: HashAlg, E: Encode>`.

**`visibility.rs`** (8.3 KB) — `Public<K, T>` / `Protected<K, H>` / `Private<K, H>`
visibility tiers with sign/encrypt boundaries. Uses `Keys` from `keys.rs`.
**STAYS, FEATURE-GATED.** The visibility model is generic; the cryptographic
implementation is `keys.rs`. Move behind a `visibility` feature; the
fragmentation core doesn't need to compile this for mirror's store consumer.

**`keys.rs`** (18 KB) — `Keys` trait, `PlainKeys` no-op, `Local` enum,
`SSH` (Ed25519 + ChaCha20Poly1305), `GPG` (subprocess shell-out). Already
feature-gated via `Cargo.toml` (`ssh`, `gpg`). **STAYS, AS IS.** Already at
the right level. The `Local::detect_in_repo` git interaction (reading SSH
signing key from `~/.gitconfig`) is one method; could be moved to
fragmentation-git eventually, but the rest of `keys.rs` is generic crypto.

**`supervision.rs`** (10.3 KB) — `Witness` trait (continuous coincidence
in [0,1]), `SupervisionTree<L, B>` enum. **STAYS, RECONSIDERED.** This is a
substrate concept from coincidence; lives here because of the witness-as-observer
thread. Not load-bearing for VCS; not VCS-flavored either. Keep for now, but
flag for migration to a `fragmentation-observers` (or coincidence-side)
crate when the substrate layering is settled.

**`git.rs`** (19.7 KB) — `read_witnessed`/`read_commit`/`commit_signature`/
`write_tree`/`write_node`/`read_node`/`write_commit`/`write_tree_named`/
`read_tree_named`/`read_tree`. The git2 interop layer. Already feature-gated
behind `git`. **MOVES to `fragmentation-git`.** The functions in this file
are already duplicated in `fragmentation-git/src/git.rs` (per the
`f1e1135 ♻️ extract git code to fragmentation-git` commit). This module is
the pre-extraction copy. Delete from fragmentation; keep in fragmentation-git.

**`fuse.rs`** (33.3 KB) — `FsInner`/`FragmentFs`, FUSE filesystem implementing
`fuser::Filesystem`, git-backed (every flush creates a git commit at
`refs/<namespace>/<ref_name>`, every read creates a `ReadAnnotation` shard).
**MOVES to `fragmentation-git`** (or, more accurately, to `fragmentation-fuse`
or `fragmentation-git-fuse` — see §2.2). The portal is a meaningful piece of
the `ROADMAP.md` story (`frgmt portal`) but its current implementation is
entirely git-backed. The portal *concept* — a filesystem view of a
content-addressed tree — is fragmentation-substrate. The portal *binding* —
write-through to git commits, read-with-witnessed-annotation to a git ref — is
an integration. Move to fragmentation-git as `fuse.rs` there.

**`main.rs`** (17.3 KB) — `fragmentation`/`frgmnt` CLI binary. Subcommands
including `shard`, `fractal`, `commit`, `link`, `mount`, `sign`, `encrypt`,
`decrypt`, `filter`. Heavily git2-flavored — `resolve_namespace(repo)`,
`detect_keys(repo)` both take `git2::Repository`. **MOVES to
`fragmentation-git`.** The CLI is the user-facing surface that ships an
opinion about how fragmentation is used; today the opinion is "git-backed."
The right shape is: `fragmentation-git`'s CLI handles git-backed flows;
`fragmentation-jj`'s CLI handles jj-backed flows; `fragmentation` itself
ships *no* CLI binary, only the library. This kills the `cli`/`fuse-mount`
binary-feature-flags from `fragmentation/Cargo.toml`.

### 2.2 The three lists

**STAYS — 15 modules, the substrate**

| Module | Role | Notes |
|---|---|---|
| `lib.rs` | module index | rewrite to reflect new layout |
| `fragment.rs` | core primitive | apply Cuts 2 + 3 (rename `Fractal::Fractal` → `Branch`; split trait into `ContentAddressed` + `TreeShaped`) |
| `sha.rs` | hash trait + Sha impl | document Sha as one impl |
| `ref_.rs` | `Ref<H>` type | |
| `cid.rs` | self-describing identifier | |
| `encoding.rs` | Encode/Decode + reference text encoder | |
| `walk.rs` | tree walk surface | |
| `diff.rs` | structural diff | extend with three-way merge |
| `store.rs` | in-memory backend | rename `Store` → `MemoryStore` |
| `repo.rs` | `Repo` trait | typed OIDs instead of `String` |
| `commit.rs` | Draft/Commit/Draftable | extract git-format SHA logic to fragmentation-git |
| `bounded_store.rs` | byte-bounded cache | Cut 1 (dashmap feature-gated) |
| `concurrent_store.rs` | concurrent backend | Cut 1 |
| `frgmnt_store.rs` | on-disk backend | document `.frgmnt/` as substrate format |
| `witnessed.rs` | witness vocab | |

**STAYS, FEATURE-GATED — 5 modules**

| Module | Feature | Notes |
|---|---|---|
| `prism_bridge.rs` | `prism-bridge` | currently unconditional; gate |
| `naked.rs` | `naked` (new) | substantive design but optional consumer surface |
| `singularity.rs` | `naked` | the trait `naked.rs` implements |
| `visibility.rs` | `visibility` | wraps `keys.rs`; only meaningful if keys are present |
| `keys.rs` | `ssh` / `gpg` (existing) + `keys` (new umbrella) | currently always-compile, sub-features behind real backends |
| `manifest.rs` | `project` (new) | only needed by `project.rs`; tiny |
| `project.rs` | `project` | filesystem projection; not substrate-essential |
| `supervision.rs` | `supervision` (new) | flagged for future migration |

**MOVES — 4 modules**

| Module | Destination | Notes |
|---|---|---|
| `git.rs` | `fragmentation/vcs/git/src/git.rs` | already duplicated in the standalone `../fragmentation-git/` repo from `f1e1135`; that repo retires (§4.5), contents fold into the workspace member |
| `fuse.rs` | `fragmentation/vcs/git/src/fuse.rs` | git-flavored; jj equivalent (if any) lives in `fragmentation/vcs/jj/` |
| `main.rs` | `fragmentation/vcs/git/src/bin/frgmt-git.rs` | rename binary; fragmentation crate ships no binary |
| (top-level `[[bin]]` entries in `Cargo.toml`) | (delete) | with `main.rs` gone |

**RETIRES — 0 modules, 4 features**

No modules retire outright. The retirement is in `Cargo.toml`'s `[features]` section:

| Feature | Status | Reason |
|---|---|---|
| `git` | DELETE from fragmentation | fragmentation-git owns this |
| `fuse` | DELETE from fragmentation | fragmentation-git owns this |
| `fuse-mount` | DELETE from fragmentation | fragmentation-git owns this |
| `cli` | DELETE from fragmentation | fragmentation-git ships CLI |

After the move, fragmentation's `Cargo.toml` `[features]` becomes:

```toml
[features]
default = ["concurrent"]
concurrent = ["dep:dashmap"]
prism-bridge = ["dep:prism-core"]
naked = []
visibility = []
keys = []
ssh = ["keys", "dep:ssh-key", "dep:x25519-dalek", "dep:chacha20poly1305", "dep:hkdf"]
gpg = ["keys"]
project = []
supervision = []
```

The `fragmentation` crate is the substrate. The CLIs, FUSE bindings, transport
layers live elsewhere. The `[[bin]]` entries vanish.

### 2.3 The new fragmentation-git surface

What fragmentation-git ends up with after this:

```
fragmentation-git/src/
├── lib.rs             (re-exports + module index)
├── git.rs             (write_tree, read_tree, write_commit, read_commit — moved from fragmentation/src/git.rs)
├── commit.rs          (git-format compute_commit_sha — moved from fragmentation/src/commit.rs's compute_commit_sha fn)
├── fuse.rs            (FUSE portal, moved)
├── store.rs           (GitStore — exists)
├── bounded_store.rs   (GitBoundedStore — exists)
├── concurrent_store.rs (ConcurrentStoreGitExt — exists)
├── namespaced.rs      (exists)
├── notes.rs           (exists — git-notes for fragmentation metadata)
├── walk.rs            (exists — git-walk extension)
├── atomic.rs          (exists — atomic ref writes)
└── bin/
    └── frgmt-git.rs   (the CLI, moved)
```

The `fragmentation-git` `Cargo.toml` already exists; it gets the binary entry
and the new modules.

### 2.4 The new fragmentation-jj surface (preview)

Lives at `fragmentation/vcs/jj/` (workspace member, per §4.5).

```
fragmentation/vcs/jj/src/
├── lib.rs             (module index + Backend wiring)
├── backend.rs         (impl jj_lib::backend::Backend for FragmentationBackend)
├── change_id.rs       (the change-id store — see §4 for the design)
├── tree.rs            (Fractal ↔ jj::Tree translation)
├── commit.rs          (Commit ↔ jj::Commit translation)
├── conflict.rs        (Merge<TreeId> ↔ Fractal storage)
├── op_store.rs        (the operation log — see §4)
└── bin/
    └── frgmt-jj.rs    (optional CLI; jj itself is the primary CLI)
```

Sized to the jj `Backend` trait's 14 required methods (per §4.3). Conservative
estimate: 800–1200 LOC. Substantially smaller than fragmentation-git's git2
integration because jj's `Backend` trait already matches fragmentation's
shape; the work is wiring, not translation.

### 2.5 What this audit confirms vs. earlier specs

This audit aligns with `mirror/docs/specs/mirror-store.md` §4.5's three cuts
(feature-gate `dashmap`, split `Fragmentable`, rename `Fractal::Fractal`) and
adds three structural moves the May 22 spec didn't name:

1. The git interop (`git.rs`, `fuse.rs`, `main.rs`) leaves fragmentation
   entirely. Mirror's spec said "fragmentation's git/fuse/cli features are not
   needed for Layer 1" — true, but they're not needed for *anything*
   substrate-shaped. They're a separate crate's concern.
2. The `Repo` trait gets typed OIDs (matches mirror-store's `MirrorStore`).
3. The `[[bin]]` entries vanish from fragmentation. The crate is a library
   only.

---

## 3. The minimal viable surface

The API surface fragmentation MUST expose to be a VCS-agnostic
content-backed store. Twelve functions, grouped by concern. Each function's
type signature IS the spec.

### 3.1 Content — read, write, hash typed entries

```rust
// fragment.rs — core primitive (per Cut 2)
pub trait ContentAddressed {
    type Hash: HashAlg;
    fn self_ref(&self) -> &Ref<Self::Hash>;
    fn encode(&self) -> Vec<u8>;
}

pub trait TreeShaped: ContentAddressed where Self: Sized {
    fn children(&self) -> &[Self];
    fn is_shard(&self) -> bool { self.children().is_empty() }
    fn targets(&self) -> &[Self::Hash] { &[] }
}

// fragment.rs — hash computation, hash-pluggable
pub fn compute_oid<F: TreeShaped>(frag: &F) -> Oid<F::Hash>;
```

Three definitions. The `ContentAddressed` trait is the minimal contract — any
type whose identity is its content. `TreeShaped` is the extension trait for
recursive structure (carries the Cut-2 split). `compute_oid` is the hash
entrypoint; in the substrate it's hash-format-agnostic. The git-format hash
(`tree_oid_bytes`'s `tree {len}\0...` framing) becomes `fragmentation-git::git_oid`.

### 3.2 References — name → content mapping

```rust
// ref_.rs
pub struct Reference {
    pub path: Vec<Cow<'static, str>>,   // ["mirror", "glass"] for @mirror/glass
    pub tags: Vec<Cow<'static, str>>,   // optional discriminators
}

impl Reference {
    pub fn parse(s: &str) -> Result<Self, RefParseError>;  // "@mirror/glass" → typed
    pub fn as_str(&self) -> String;                         // typed → "@mirror/glass"
}
```

The typed reference replaces today's `Combinator::Lift { grammar: String, ... }`
representation. Matches `mirror/docs/specs/mirror-store.md` §3.1's typed form.
Keeps `@<path>` as the canonical user-facing shape; the substrate stores it
structured.

### 3.3 Versioning — commit-equivalent typed content with provenance

```rust
// commit.rs (today's Commit<N,H>, slightly typed)
pub enum Commit<N: ContentAddressed, H: HashAlg = CoincidenceHash<5, 5>> {
    Root { node: N, witnessed: Witnessed, message: Message, oid: H },
    Child { node: N, witnessed: Witnessed, message: Message, parents: Vec<H>, oid: H },
}

// commit.rs — substrate-side hash; format-agnostic
pub fn commit_oid<N: ContentAddressed, H: HashAlg>(commit: &Commit<N, H>) -> H;
```

Three changes from today:

1. `Commit::Child` now carries `parents: Vec<H>` instead of `parent: Parent<H>`.
   Matches jj's `Commit::parents: Vec<CommitId>` and supports kintsugi
   octopus-merges (the `fragmentation-git/src/commit.rs::write_commit_with_parents`
   function already supports `&[git2::Oid]`).
2. The git-format `compute_commit_sha` leaves to fragmentation-git as the impl
   of a `CommitFormat<H>` trait the backend chooses. fragmentation-side
   `commit_oid` is the format-agnostic canonical hash.
3. The format-agnostic part is what jj's `Backend::write_commit` returns —
   `(CommitId, Commit)` — except fragmentation parameterizes over the hash.

### 3.4 History/queries — walk parents, find ancestors, diff

```rust
// walk.rs — already exists, extend for commit-chain walking
pub fn walk_commits<N, H, R>(repo: &R, head: &H) -> impl Iterator<Item = Commit<N, H>>
where
    N: ContentAddressed,
    H: HashAlg,
    R: Repo<Node = N, Hash = H>;

// diff.rs — already exists, add three-way merge (per fragmentation-vcs-spec.md §3.5)
pub fn merge3<F>(base: &F, ours: &F, theirs: &F, resolve: &impl Fn(&F, &F, &F) -> F) -> F
where
    F: TreeShaped + Clone;

// diff.rs — find common ancestor in the commit DAG
pub fn common_ancestor<N, H, R>(repo: &R, a: &H, b: &H) -> Option<H>
where
    N: ContentAddressed,
    H: HashAlg,
    R: Repo<Node = N, Hash = H>;
```

Three functions. `walk_commits` is the log primitive. `merge3` is the
three-way merge over content-addressed trees. `common_ancestor` is the
DAG-walk for merge-base resolution.

### 3.5 Storage backend — pluggable persistence

```rust
// repo.rs — already exists, retyped to use Oid<H> instead of String
pub trait Repo {
    type Node: ContentAddressed;
    type Hash: HashAlg;

    fn write_tree(&mut self, node: &Self::Node) -> Oid<Self::Hash>;
    fn read_tree(&self, oid: &Oid<Self::Hash>) -> Option<Self::Node>;

    fn write_commit(&mut self, commit: Commit<Self::Node, Self::Hash>);
    fn read_commit(&self, oid: &Self::Hash) -> Option<Commit<Self::Node, Self::Hash>>;

    fn update_ref(&mut self, r: &Reference, oid: Self::Hash);
    fn resolve_ref(&self, r: &Reference) -> Option<Self::Hash>;
}
```

Six methods. This IS the substrate. Every backend (MemoryStore, FrgmntStore,
GitStore in fragmentation-git, FragmentationBackend in fragmentation-jj)
implements this trait.

### 3.6 The full surface — 12 functions and 4 types

```
Traits:    ContentAddressed, TreeShaped, HashAlg, Repo
Types:     Oid<H>, Reference, Commit<N,H>, Witnessed
Functions: compute_oid, commit_oid, walk_commits, merge3, common_ancestor
Repo methods (6): write_tree, read_tree, write_commit, read_commit,
                  update_ref, resolve_ref
```

That's it. The substrate. Everything else (cache strategies, hash format
choice, on-disk layout, transport, conflict semantics, change-id, op-log) is
a consumer's concern.

### 3.7 What's deliberately not in the surface

- **No removal.** Content-addressed stores don't remove. GC is a backend
  concern (jj's `Backend::gc(&self, index: &dyn Index, keep_newer: SystemTime)`
  is in jj's trait; fragmentation's substrate puts it in a separate
  `GarbageCollect` trait on backends that support it).
- **No iteration.** No `iter()`/`keys()`. The substrate is for lookup, not
  enumeration. Backends can expose this if needed (`FrgmntStore::keys`
  exists today).
- **No transport.** Push/pull/clone are codeforge interop, owned by
  fragmentation-git (smart HTTP/SSH) and the jj transport (which jj-native
  already implements via its operation log).
- **No conflict storage.** Conflicts are a typed value, stored by the
  consumer. jj stores them as `Merge<TreeId>`; mirror's store doesn't store
  them at all. The substrate doesn't have an opinion.
- **No working-tree.** The working tree is a consumer concern (jj treats it
  as the current commit; git stages it via index; mirror's grammar boot has
  no working tree at all).

The omissions are deliberate. The substrate has one job: typed, recursive,
content-addressed storage. Every consumer adds the opinion that makes it a
VCS *of a particular kind*.

---

## 4. The git vs jj evaluation

The central question. Read jj's `Backend` trait. Compare to fragmentation.
Name the gaps. Recommend.

### 4.1 The jj Backend trait — what it actually wants

From `/tmp/jj_backend.rs` (jj `main`, commit-of-day). Annotated for the
fragmentation comparison:

```rust
#[async_trait]
pub trait Backend: Any + Send + Sync + Debug {
    fn name(&self) -> &str;
    fn commit_id_length(&self) -> usize;
    fn change_id_length(&self) -> usize;
    fn root_commit_id(&self) -> &CommitId;
    fn root_change_id(&self) -> &ChangeId;
    fn empty_tree_id(&self) -> &TreeId;
    fn concurrency(&self) -> usize;

    async fn read_file(&self, path: &RepoPath, id: &FileId) -> BackendResult<Pin<Box<dyn AsyncRead + Send>>>;
    async fn write_file(&self, path: &RepoPath, contents: &mut (dyn AsyncRead + Send + Unpin)) -> BackendResult<FileId>;

    async fn read_symlink(&self, path: &RepoPath, id: &SymlinkId) -> BackendResult<String>;
    async fn write_symlink(&self, path: &RepoPath, target: &str) -> BackendResult<SymlinkId>;

    async fn read_copy(&self, id: &CopyId) -> BackendResult<CopyHistory>;
    async fn write_copy(&self, copy: &CopyHistory) -> BackendResult<CopyId>;
    async fn get_related_copies(&self, copy_id: &CopyId) -> BackendResult<Vec<RelatedCopy>>;

    async fn read_tree(&self, path: &RepoPath, id: &TreeId) -> BackendResult<Tree>;
    async fn write_tree(&self, path: &RepoPath, contents: &Tree) -> BackendResult<TreeId>;

    async fn read_commit(&self, id: &CommitId) -> BackendResult<Commit>;
    async fn write_commit(&self, contents: Commit, sign_with: Option<&mut SigningFn>) -> BackendResult<(CommitId, Commit)>;

    fn get_copy_records(&self, paths: Option<&[RepoPathBuf]>, root: &CommitId, head: &CommitId) -> BackendResult<BoxStream<'_, BackendResult<CopyRecord>>>;
    fn gc(&self, index: &dyn Index, keep_newer: SystemTime) -> BackendResult<()>;
}
```

Key observations:

- **Async.** Every read/write is `async fn`. fragmentation's `Repo` is
  synchronous today. An adapter has to wrap; doable, but adds a runtime
  surface (tokio or smol).
- **Path-keyed reads.** `read_file(path, id)` takes both the file's path AND
  its `FileId`. The path is for caching/locality; jj's reference
  `GitBackend` ignores the path and just reads by OID. fragmentation's
  `read_tree(oid)` doesn't take a path — same as git's. Trivial to add a
  ignored-path parameter on the adapter.
- **Five id types.** `CommitId`, `ChangeId`, `TreeId`, `FileId`, `SymlinkId`,
  `CopyId`. Each is a typed `[u8; N]` newtype. fragmentation has one
  pluggable `HashAlg` that produces a single `H` type per backend. The
  adapter needs to map fragmentation's `Oid<H>` to five typed newtypes —
  trivial, since they're all just byte arrays.
- **Tree is one level of entries.** `jj::Tree` is `Vec<(RepoPathComponentBuf,
  TreeValue)>`, NOT recursive. Subdirectories are `TreeValue::Tree(TreeId)`
  references, resolved by a separate `read_tree` call. fragmentation's
  `Fractal` is recursive — same content embedded inline. The adapter has to
  flatten: each `Fractal::Branch` becomes a `Tree` whose entries reference
  child OIDs, not embed them. fragmentation supports this; `content_oid`
  computes the OID without materializing.
- **TreeValue is a closed sum.** `File{id, executable, copy_id} | Symlink |
  Tree | GitSubmodule(CommitId)`. fragmentation's `Fractal` is also a closed
  sum (`Shard | Branch | Lens`). The mapping: `Shard` → `File`, `Branch` →
  `Tree`, `Lens` → likely `GitSubmodule` or a custom "reference" variant.
  Lens references multiple targets (`Vec<H>`); the jj variant references one
  commit. Spec-relevant gap — see §4.4.
- **ChangeId is independent of CommitId.** jj's defining feature. When a
  commit is rewritten (rebased, amended), its `CommitId` changes (content
  changed) but its `ChangeId` stays the same. fragmentation has no
  change-id concept today. **This is the substrate gap.** See §4.4.
- **The Commit struct.** Carries `root_tree: Merge<TreeId>`. The `Merge` is
  jj's structured-conflict type — when the working state has unresolved
  conflicts, `root_tree` is a non-resolved `Merge` carrying both sides.
  fragmentation has `merge()` with caller-provided resolution but no
  structured-conflict storage. Adapter gap, but minor — see §4.4.
- **Commit has `predecessors: Vec<CommitId>`.** The rewrite history. jj tracks
  every rewrite. fragmentation's `Commit::Child` only tracks one parent
  (today); the new surface (§3.3) tracks `Vec<H>` parents. Predecessors are
  a separate dimension — a commit knows its *content* parents AND its
  *rewrite* predecessors. fragmentation's substrate doesn't track predecessors
  today; adapter would need to store this metadata. Workable as a
  fragmentation-jj-side extension to `Commit`.
- **`gc(&self, index: &dyn Index, keep_newer: SystemTime)`.** Garbage
  collection takes a reference to jj's `Index` — a separate primitive jj's
  library provides. fragmentation has no GC today; for native jj-as-backend
  this becomes a fragmentation-side implementation that walks the index's
  retained commits and prunes non-reachable content. Substantial work, but
  isolated to the adapter; the substrate doesn't need it.
- **`get_copy_records`.** jj tracks copy/move provenance. fragmentation's
  `Lens` is the closest analog (cross-tree reference). jj's reference
  `GitBackend` can return `BackendError::Unsupported` here — copy tracking
  is optional. fragmentation-jj can do the same and add later.
- **The CopyHistory.** A separate object with its own ID, holding
  `current_path`, `parents: Vec<CopyId>`, `salt: Vec<u8>`. Optional support;
  defer.
- **Operation log is NOT in `Backend`.** jj's `op_store` is a separate trait
  (`OpStore`, in `jj_lib::op_store`). It records every operation (commit
  creation, branch update, working-copy snapshot) as a content-addressed
  entry. The `Backend` trait above is the *commit* backend; the op log is
  parallel. For fragmentation-jj to be a *complete* jj backend, BOTH have to
  be implemented. Good news: jj already provides `SimpleOpStore`, a
  filesystem-backed implementation; fragmentation-jj can use it unchanged
  for v1.

### 4.2 The git object model — what fragmentation already does

For contrast: git's object model is `blob | tree | commit | tag`. Each is a
zlib-compressed bytestream with a type header. The commit object is plain
text with `tree`/`parent`/`author`/`committer`/blank-line/message. Refs are
filesystem entries. fragmentation already mirrors this:

- `Fractal::Shard` ↔ blob (fragmentation's `blob_oid` produces git-compatible
  SHA-1 with `blob {len}\0...` framing)
- `Fractal::Branch` ↔ tree (fragmentation's `tree_oid_bytes` produces git
  trees with `.data` blob + numbered children)
- `Fractal::Lens` ↔ no native git equivalent (closest is submodule, but
  `Lens` is multi-target and embeds data)
- `Commit<N, H>` ↔ commit (fragmentation's `compute_commit_sha` produces
  git-compatible commit SHAs)
- `Repo::update_ref`/`resolve_ref` ↔ git refs (plain text files at
  `.git/refs/heads/<name>`)

The git side requires **translation, not adapter**:

- The `.data` blob convention plus numbered children is fragmentation's
  shape, not git's natural shape. Git tools see a tree with a `.data` file
  and numbered subentries — readable, but not idiomatic git.
- `Lens` has no git representation. fragmentation-git's current
  implementation invents `.lens` as a sibling blob to `.data` — also
  fragmentation-shaped, not git-shaped.
- A consumer who only speaks git (GitHub UI, GitLab MR view, `git log`,
  `git diff`) sees the fragmentation structure as a peculiar nested layout.
  Works, but not how anyone would design git data.

### 4.3 The alignment matrix

| jj Backend concept | fragmentation today | Gap | Where it goes |
|---|---|---|---|
| `name() -> &str` | `"fragmentation"` constant | none | impl |
| `commit_id_length() -> usize` | depends on hash (20 for Sha-1, 64 for Sha-512) | none | impl |
| `change_id_length() -> usize` | **no change-id today** | substantive | §4.4 |
| `root_commit_id() / root_change_id() / empty_tree_id()` | constants | none | impl |
| `concurrency() -> usize` | 1 for `MemoryStore`, configurable for `ConcurrentStore` | none | impl |
| `read_file / write_file (async)` | `Repo::read_tree / write_tree` (sync, no path) | wrap in async; ignore path | impl |
| `read_symlink / write_symlink` | `Lens` variants could represent | translation | fragmentation-jj |
| `read_copy / write_copy / get_related_copies / get_copy_records` | none | optional; return Unsupported | impl |
| `read_tree / write_tree` (one-level) | `Fractal::Branch` (recursive) | flatten on write, embed on read | fragmentation-jj |
| `read_commit / write_commit` | `Commit<N, H>` Root/Child | adapt to multi-parent + change-id | impl |
| `Tree = Vec<(name, TreeValue)>` | `Fractal<E, H>` recursive | flatten | impl |
| `TreeValue = File \| Symlink \| Tree \| GitSubmodule` | `Fractal = Shard \| Branch \| Lens` | direct map; Lens → GitSubmodule | impl |
| `Commit.parents: Vec<CommitId>` | `Commit::Child.parent: Parent<H>` | extend to Vec (§3.3) | substrate change |
| `Commit.predecessors: Vec<CommitId>` | none | adapter stores | fragmentation-jj |
| `Commit.change_id: ChangeId` | none | §4.4 | substrate or adapter |
| `Commit.root_tree: Merge<TreeId>` | `Commit::Child.node: N` | add conflict layer | adapter for conflicts |
| `Signature {name, email, timestamp}` | `Witnessed {author, committer, timestamp}` | trivial | impl |
| `SecureSig (signature)` | `keys::Signature<K>` (when feature on) | trivial | impl |
| `BackendError` | adapter-side error enum | trivial | impl |
| `gc(&self, index: &dyn Index, ...)` | none | new fragmentation-side impl | substrate change |
| `OpStore` trait (separate) | none in fragmentation | use jj's `SimpleOpStore` for v1 | reuse |

### 4.4 The substantive gaps — what's NOT just impl

Three gaps require structural decisions, not just trait method implementation.

#### Gap 1: change-id

jj's `ChangeId` is a stable identifier that follows a commit when it's
rewritten. Same change rebased to new parents → new `CommitId`, same
`ChangeId`. This is jj's defining UX feature (`jj log` shows change ids;
`jj describe` updates the description without rewriting the change).

fragmentation has no change-id today.

**Three options:**

- **(a) Add change-id to fragmentation's substrate.** Extend
  `Commit<N, H>` to carry `change_id: Option<Vec<u8>>`. The substrate is
  agnostic about what produces it (jj generates random 16-byte ids;
  fragmentation-jj generates similarly; fragmentation-git uses
  jj's `synthetic_change_id_from_git_commit_id` algorithm). Pros: clean
  layering; the surface is what jj needs. Cons: the substrate now carries a
  jj-flavored concept; other consumers (mirror's grammar store) don't need
  it but pay the field.

- **(b) Adapter-side storage.** fragmentation-jj keeps a parallel
  `ChangeIdStore` (mapping CommitId → ChangeId) alongside the fragmentation
  Repo. Pros: substrate stays pure. Cons: read-after-write consistency
  becomes a fragmentation-jj concern; the OpStore would have to coordinate.

- **(c) Mimic jj's GitBackend approach.** jj's GitBackend stores ChangeId as
  an *extra git commit header* (`change-id` trailer in the commit object).
  Fragmentation-jj could store change-id as an extra field in fragmentation's
  `Commit::Child::extras: HashMap<String, Vec<u8>>` — extensible side-band
  for adapter-specific metadata. Pros: substrate stays minimal but extensible;
  the storage is co-located with the commit. Cons: opens an extensibility
  surface that may grow.

**Recommendation: (c).** Add a single `extras: HashMap<String, Vec<u8>>`
field to `Commit::Child` (and `Commit::Root`). Document one well-known key:
`"jj/change-id"`. fragmentation-jj reads/writes it via the existing trait
surface. spectral-db can add `"spectral-db/eigenvalue-hash"` later without
changing the substrate. mirror's store ignores `extras` entirely.

#### Gap 2: structured conflict storage

jj stores unresolved conflicts as `Merge<TreeId>` — a typed structure that's
recursive in jj-land (a `Merge` of `Merge`s is a Merge). fragmentation's
`merge()` resolves conflicts on read with a caller-provided resolver; there's
no storage of unresolved conflicts.

The option here is structural: fragmentation can stay merge-on-read, in which
case fragmentation-jj needs a side-table mapping `CommitId → Merge<TreeId>`
for unresolved commits. Alternative: add a `Fractal::Conflict { ours, theirs,
base }` variant to the closed sum — every consumer pays the variant cost, but
the storage is co-located.

**Recommendation: side-table in fragmentation-jj.** A `Fractal::Conflict`
variant is too jj-flavored for the substrate. Mirror's grammar trees aren't
conflicted in the jj sense (grammars merge by tournament, per
`spec/kintsugi-tournament.md`); spectral-db's conflicts are a fate-resolved
morphism on the eigenboard sheaf (per Reed's memory `project-eigenboard-is-sheaf`).
Each consumer has its own conflict shape; substrate doesn't pick a winner.

Side-table cost: fragmentation-jj keeps a `ConflictStore<H>` keyed by commit
OID, holding `Merge<H>`. Implements jj's read/write contract by consulting
both tables (regular `Commit` for resolved, `ConflictStore` for unresolved).

#### Gap 3: working-copy semantics

jj treats the working copy as a commit (the "@" commit, a.k.a.
`@working_copy`). Every change is auto-committed; rebase is a normal
operation; the working tree is just another tree in the store. This is jj's
UX. fragmentation has no working-copy concept.

Three resolutions:

- **(a) Substrate-side working copy.** Add a `WorkingCopy` trait fragmentation
  exposes; backends implement it. Too presumptuous — mirror has no working
  copy.

- **(b) Adapter-side working copy.** fragmentation-jj implements jj's
  `LocalWorkingCopy` trait (jj already provides this as part of
  `jj_lib::working_copy`). Backend is just `Backend`; working-copy is
  separate. **This is what jj's own architecture does.**

- **(c) Defer to v1.5.** fragmentation-jj v1 doesn't support working-copy;
  use jj's existing `GitBackend`-backed working copy and synchronize.

**Recommendation: (b).** Implement `LocalWorkingCopy` adapter in
fragmentation-jj that snapshots the working tree to fragmentation primitives.
jj's existing `LocalWorkingCopy` impl is over filesystem state and writes
trees through a `Backend`; fragmentation-jj's working-copy is the same logic
with fragmentation as the backend. Reuse jj's code where possible.

### 4.5 The verdict — git as interop, jj as native

The alignment matrix in §4.3 is overwhelmingly favorable for jj. Of the 22
rows, **14 are direct impl** (no structural gap), **5 are adapter-only**
(fragmentation-jj-side handling), and **3 are substantive** (change-id,
conflict storage, working copy). The three substantive gaps have clean
resolutions: change-id via an `extras` field on `Commit`, conflict storage
via a side-table in fragmentation-jj, working-copy via reusing jj's
`LocalWorkingCopy`.

For git, the situation is inverted. fragmentation's primitives MAP to git
(the existing `fragmentation/src/git.rs` proves it), but the mapping is
fragmentation-shaped, not git-shaped — `.data` + numbered children, `.lens`
as sibling blob. A git consumer sees fragmentation through a peculiar lens.
The value of git compatibility is the *transport*: smart HTTP, SSH, pack
negotiation, every codeforge supporting it. NOT the daily-driver workflow.

**The three options from the brief:**

1. ~~Native: fragmentation IS jj's backend. git is interop-only.~~ — close,
   but not quite right. fragmentation's substrate is a tier under jj's
   backend; fragmentation-jj IS jj's backend, AS a wrapper.
2. **Native: fragmentation exposes a jj-shaped backend via fragmentation-jj.
   git is interop-only.** ✓ — this is the recommendation.
3. ~~Native: both are full backends; fragmentation-jj and fragmentation-git
   are peer adapters.~~ — git can't be a full daily-driver backend because
   the lossy mapping degrades the substrate's resolution.

**Recommendation: option 2.**

**Defense:**

- The substantive jj gaps are small (3 of 22) and have clean resolutions
  that don't pollute the substrate (extras field; side-table; jj reuse).
- jj's `Backend` trait already does what fragmentation wants to expose. The
  trait *is* the contract a VCS-agnostic content store satisfies. Aligning
  to it is alignment to the right level.
- git's value is the transport. fragmentation-git becomes a translation
  layer that exports fragmentation state to git for codeforge interop
  (push, fetch, clone). It is NOT the workflow.
- jj itself uses git as interop already (`GitBackend`). Mirror users running
  jj-native fragmentation can still `jj git push` to GitHub. The interop
  story is already solved by jj; fragmentation-git becomes
  fragmentation-side support for the same flow.
- The thing this lets us walk away from: fragmentation's git-feature
  contamination of the substrate. The `git2` import, the `.frgmnt/`-imitates-`.git/`
  layout choices, the `commit_oid` being git-formatted by default. All of
  that moves to fragmentation-git, which owns being-git-compatible. The
  substrate gets to be substrate.

This is the architecture the audit produces. The library has been waiting for
it. Name it.

---

## 4.5 Repo layout — workspace with `vcs/` adapters

Structural decision before the audit's moves land: **fragmentation becomes a
workspace; adapters live inside it under `vcs/`.** Not separate sibling repos.
This matches the prism repo's pattern (`prism/core/`, `prism/imperfect/`,
`prism/derive/`, etc.) and self-contains the substrate + every adapter the
substrate ships with.

Target layout:

```
fragmentation/
├── Cargo.toml          (workspace manifest + fragmentation crate manifest)
├── src/                (fragmentation core code)
├── vcs/
│   ├── git/            (fragmentation-git crate)
│   └── jj/             (fragmentation-jj crate)
├── docs/
└── …
```

The `vcs/` grouping signals intent (these are VCS implementations) and leaves
room for future adapters (`vcs/mercurial/`, `vcs/pijul/`, `vcs/sapling/`)
without sprawling the workspace root.

**Crate names stay short and stable:** `fragmentation`, `fragmentation-git`,
`fragmentation-jj`. The published names don't carry the `vcs/` directory.

**Feature-flag posture:**

- **Within `fragmentation/`** (the substrate crate): keep in-crate capability
  gates (`concurrent` per Cut 1, `prism_bridge`, `keys`, etc.). These select
  capability *inside* one crate; they don't pull siblings.
- **Cross-crate adapter selection:** consumers depend on `fragmentation-git`
  or `fragmentation-jj` directly (or via their own features that gate the
  dependency). fragmentation itself has no `git` or `jj` feature — that would
  muddle the workspace model. Mirror's `Cargo.toml` gets `git = ["dep:fragmentation-git"]`
  and `jj = ["dep:fragmentation-jj"]`; mirror picks; fragmentation doesn't.

**Retirement of the standalone `../fragmentation-git/` stub.** The repo at
`/Users/alexwolf/dev/projects/fragmentation-git/` (the `f1e1135` extraction
target) gets folded into `fragmentation/vcs/git/`. Its existing contents
(the duplicate `git.rs`, basic `Cargo.toml`) merge in; the standalone repo
archives. Use `git subtree` to preserve history if needed; otherwise a clean
import into the new workspace path.

---

## 4.6 Hash function — `CoincidenceHash<5,5>` as default

Structural decision before T2's `Repo` retyping lands: **`CoincidenceHash<5,5>`
is fragmentation's default `HashAlg` for mirror's consumption path.** SHA-1
stays available as an explicit choice in `fragmentation/vcs/git/` where git
interop requires it. SHA-256, blake2b, and any other format-specific hash
are pluggable per `HashAlg` impl, used by their respective VCS adapters.
Mirror itself never computes a SHA; the substrate it consumes computes
CoincidenceHash<5,5>.

### Why this isn't a stylistic choice

`CoincidenceHash<5,5>` IS the Dirac operator D restricted to mirror's
five-operation algebra. Per Reed & Alex's research synthesis
[`~/dev/systemic.engineering/practice/insights/spectral-db/dirac-operator-on-graphs.md`](file:///Users/reed/dev/systemic.engineering/practice/insights/spectral-db/dirac-operator-on-graphs.md):

- **A** (algebra) = mirror grammars (the five-operation surface).
- **H** (Hilbert space) = l²(V) + l²(E) over the content-addressed graph.
- **D** (Dirac operator) = d + d* where d is the signed incidence matrix B.
  D² = the Hodge Laplacian = block-diagonal of (L₀, L₁) where L₀ is the
  vertex Laplacian fragmentation already computes implicitly.

The 5×5 in `CoincidenceHash<5,5>` is this restriction: 5 operations
(focus/project/split/zoom/refract) along one axis, 5 projections along the
orthogonal axis, producing the matrix form of D on the spectral triple
(A, H, D). The hash IS D's spectrum on the content tree. The Merkle
combine step IS D acting on child eigenvalues, with parent D constructed
from the child incidence structure (the operator's recursive form). The
hash function is not arbitrary; it's the operator the rest of the
architecture has been computing pieces of without naming it.

### What this gives the rest of the architecture for free

Per the same insight document:

- **Tournament C4 tiebreaker = Connes distance.** §4 of the insight doc:
  Connes distance on graphs reduces to Dijkstra with edge lengths
  1/√w_e. Polynomial-time computable; triangle inequality holds; real
  geometric meaning. `kintsugi-tournament.md`'s C4 "minimize OID churn"
  becomes "minimize Connes distance between proposed and current states"
  — a real metric, not byte noise on opaque hashes.
- **Kintsugi's loss function = spectral action difference.** §5: replace
  ShannonLoss with `Tr(f(D_before/Λ)) − Tr(f(D_after/Λ))`. Scale-aware
  (the Λ parameter), structural (derived from the spectrum), and
  contraction-map-shaped. Gives `kintsugi-formatter.md`'s Banach
  contraction argument an actual foundation.
- **`--strict` gains a structural narcissus check.** §6: anomalous
  spectral action relative to degree-class expectation flags
  geometrically pathological grammars. New strict check beyond Dark spans
  and depth bounds.
- **spectral-db's four separate computations collapse to one operator.**
  Per §1 of the insight doc: today spectral-db computes ego Laplacian
  eigenvalues, Fiedler, BGS entropy, ad hoc spectral distance — four
  separate things. Under D as the unifying operator, they all derive from
  the same spectrum. spectral-db's substrate work and mirror's hash work
  become the same architectural project.

### What this asks of the implementation

- **`HashAlg` impl for `CoincidenceHash<5,5>` lives in `prism-core`** (the
  Dirac operator machinery already lives there per the eigenboard work).
  Fragmentation imports it; doesn't redefine it.
- **The Merkle combine step is short to spec** (≈40 lines), not
  long-form. The combine isn't arbitrary; it's D's recursive form
  applied to the child incidence structure. Spec lands as
  `prism/docs/specs/coincidence-hash-merkle.md` (or in fragmentation's
  docs if cleaner; either way, mechanical from the insight doc's §3).
- **The boot corpus rebakes once.** Every smoke OID (`a8312da6…`,
  `3ba4c79d…`), every test assertion, every spec reference that pins an
  OID gets new bytes. Not lossy; one-way migration; totally fine pre-v1.
  Happens at T5 (mirror's F-2) when mirror starts using fragmentation's
  typed `Oid<H>`.
- **No backward-compatibility shim.** Pre-v1, the bytes change once and
  that's the end of it. Spec language going forward: *CoincidenceHash<5,5>
  is mirror's hash. SHA is what git adapters speak. There is no
  mirror-with-SHA.*

### The two-axis policy, named

| Consumer | Default `HashAlg` | Reason |
|---|---|---|
| `fragmentation` (substrate) | `CoincidenceHash<5,5>` | The Dirac operator on the content graph. The substrate's own invariant. |
| `fragmentation/vcs/git` | `Sha` (SHA-1) | Git interop requirement. Adapter boundary. |
| `fragmentation/vcs/jj` | `CoincidenceHash<5,5>` (substrate) + `Sha` for git-export paths | Native consumes substrate; export reaches into the adapter. |
| `mirror` | `CoincidenceHash<5,5>` (via fragmentation) | Spectral-triple coherence. |
| `spectral-db` | `CoincidenceHash<5,5>` (via fragmentation) + its own typed entries | Same substrate; broader content surface. |

T2's `Repo` retyping consumes this directly: the default `H` is
`CoincidenceHash<5,5>`; the git adapter's `Repo` impl overrides to `Sha`.
The `Commit<N, H>` default in §3.4 changes from `H: HashAlg = Sha` to
`H: HashAlg = CoincidenceHash<5,5>`. SHA stays as the git-adapter's
single-impl override, exposed via the same generic.

---

## 5. Tick decomposition

Six ticks. T1 lands the audit's structural moves. T2–T4 build the jj path.
T5/T6 hand off to consumers. The load-bearing tick is T1 — without it, the
rest is built on the wrong layer.

### T1 — Simplification audit + commits (the cleanup)

**Scope.**
- **Convert fragmentation to a workspace** (per §4.5). Root `Cargo.toml`
  gains a `[workspace]` section listing members `.`, `vcs/git`, `vcs/jj`
  (jj added in T4 but the slot is declared up front).
- **Retire the standalone `../fragmentation-git/` stub.** Move its contents
  (the duplicate `git.rs`, the basic `Cargo.toml`) into the new
  `fragmentation/vcs/git/` workspace member. Use `git subtree` if preserving
  history matters; otherwise clean import. Archive or delete the standalone
  repo once the move is verified.
- Move `git.rs` from `fragmentation/src/` to `fragmentation/vcs/git/src/`
  (deduplicate against the merged-in standalone-repo copy; keep one).
- Move `fuse.rs` from `fragmentation/src/` to `fragmentation/vcs/git/src/`.
- Move `main.rs` from `fragmentation/src/` to
  `fragmentation/vcs/git/src/bin/frgmt-git.rs`.
- Delete the `git`, `fuse`, `fuse-mount`, `cli` features and the `[[bin]]`
  entries from `fragmentation/Cargo.toml` (the root crate manifest).
- Apply mirror-store.md Cut 1: feature-gate `dashmap` behind `concurrent`
  (default-on).
- Apply mirror-store.md Cut 2: split `Fragmentable` into `ContentAddressed`
  + `TreeShaped`.
- Apply mirror-store.md Cut 3: rename `Fractal::Fractal` → `Fractal::Branch`.
- Feature-gate `prism_bridge.rs` behind `prism-bridge`.
- Feature-gate `naked.rs`, `singularity.rs`, `visibility.rs`, `project.rs`,
  `manifest.rs`, `supervision.rs` behind their respective features (per §2.2).
- Add `[[bin]]` entry to `fragmentation/vcs/git/Cargo.toml` for `frgmt-git`.

**Estimate.** Large. ~3.5 sessions (the workspace conversion + standalone
repo retirement adds ~half a session vs. the prior estimate). Touches every
consumer (`mirror/`, `spectral-db/`, `coincidence/`, internal tests). Most
of the work is updating call sites; the moves themselves are straightforward
`git mv`.

**Dependencies.** None. Can start immediately.

**Acceptance.**
- `cargo test -p fragmentation` passes with default features (no git, no fuse,
  no cli).
- `cargo test -p fragmentation --all-features` passes (gates work in both
  directions).
- `cargo test -p fragmentation-git` passes.
- `cargo test -p fragmentation --no-default-features` passes for the no_std
  stretch (post-`concurrent` gating).
- Mirror compiles against the new surface (this drags in mirror-store.md F-2
  work; do these together).
- `Fragmentable` trait is gone or deprecated-aliased to `TreeShaped`;
  `ContentAddressed` is the new minimum.
- `Fractal::Branch` replaces `Fractal::Fractal` everywhere; no double-named
  variant.
- `fragmentation/Cargo.toml`'s `[features]` matches §2.2's target list.

**This is the load-bearing tick.** Until it lands, the substrate carries
git/fuse/cli weight that contradicts "VCS-agnostic." Until the trait split
lands, mirror's `Entry` enum has to implement no-op trait methods (per
mirror-store.md §4.5 Cut 2 rationale). Until `Fractal::Branch` lands, every
grep + doc page suffers the doubly-named variant.

### T2 — Decouple git into fragmentation-git completion

**Scope.**
- Move `compute_commit_sha` from `fragmentation/src/commit.rs` to
  `fragmentation-git/src/commit.rs` as `impl CommitFormat<Sha> for GitCommitFormat`.
- Define `CommitFormat<H: HashAlg>` trait in fragmentation
  (`fragmentation/src/commit.rs`): `fn commit_oid(commit: &Commit<N, H>) -> H`.
- `fragmentation-git` implements `CommitFormat<Sha>` with git's exact
  `tree {oid}\n parent {oid}\n ...` format.
- Rename `Store` → `MemoryStore` in `fragmentation/src/store.rs`.
- Retype `Repo` trait per §3.5: `Oid<H>` and `Reference` instead of `String`.
  **Default `H` is `CoincidenceHash<5,5>`** per §4.6. SHA is the
  `fragmentation/vcs/git` override, not the substrate's choice.
- **Wire `prism-core`'s `CoincidenceHash<5,5>` as a `HashAlg` impl** in
  `fragmentation/src/sha.rs` (or move the trait to a hash-agnostic module).
  fragmentation imports the operator from prism-core; doesn't redefine it.
- Multi-parent `Commit::Child::parents: Vec<H>`.
- Add `extras: HashMap<String, Vec<u8>>` field to `Commit::Root` and
  `Commit::Child` (per §4.4 Gap 1).

**Estimate.** Medium. ~1.5 sessions.

**Dependencies.** T1.

**Acceptance.**
- `fragmentation/Cargo.toml` has no `git2` dependency, no `git`/`fuse` features.
- `fragmentation-git` builds and tests pass.
- `Commit::Child` supports `Vec<H>` parents; existing one-parent call sites
  updated.
- `extras` field is empty for existing call sites; readable for new ones.
- The `Repo` trait uses typed `Oid<H>` and `Reference`; backends updated.

### T3 — Define the VCS-agnostic surface in fragmentation

**Scope.**
- Land the 12-function surface from §3 in fragmentation.
- `walk_commits<N, H, R>` in `walk.rs`.
- `merge3` in `diff.rs` (extends `merge` to three-way).
- `common_ancestor<N, H, R>` in `diff.rs`.
- Documentation: each function's contract; the layering picture; what's NOT
  in the surface (per §3.7).

**Estimate.** Medium. ~1 session.

**Dependencies.** T2 (the retyped `Repo` trait).

**Acceptance.**
- All 12 functions land with tests.
- `merge3` matches `fragmentation-vcs-spec.md` §3.5's three-way merge
  semantics (same-base→take-theirs; same-theirs→take-ours; conflict→recurse).
- `common_ancestor` returns the LCA in the DAG.
- A doc page (`docs/specs/surface.md`) lists the 12 functions, their
  signatures, and a one-line description.

### T4 — Build fragmentation-jj

**Scope.**
- Initialize the `fragmentation-jj` crate at
  `fragmentation/vcs/jj/` (workspace member, per §4.5).
  Add to the workspace's `[workspace]` members list (or activate the slot
  T1 declared up front).
- Add `jj_lib` as a dependency.
- Implement `jj_lib::backend::Backend` for `FragmentationBackend<R: Repo>`.
- The 14 required methods, mapping per §4.3.
- Conflict storage as a side-table (per §4.4 Gap 2).
- Change-id via the `Commit::extras` field (per §4.4 Gap 1).
- Working-copy: reuse `jj_lib::working_copy::LocalWorkingCopy`.
- OpStore: use `jj_lib::simple_op_store::SimpleOpStore` (filesystem-backed,
  v1; native fragmentation-backed OpStore is v1.5).
- A `frgmt-jj` CLI binary that does `jj init --backend=fragmentation`,
  `jj log`, `jj describe`, `jj rebase`, etc. by dispatching to
  fragmentation-jj backend.

**Estimate.** Large. ~4–6 sessions. Heavy on jj API familiarization. The
actual code volume is ~800–1200 LOC (per §2.4 estimate), but the
familiarization with jj's API surface, error handling, and async runtime is
load-bearing.

**Dependencies.** T3 (the surface fragmentation-jj wraps).

**Acceptance.**
- `fragmentation-jj` builds.
- `jj init --backend=fragmentation .` in a test directory creates a working
  jj repo backed by fragmentation primitives.
- Round-trip: create commit via jj → read back content matches; create commit
  via fragmentation → readable via `jj log`.
- Change-id stable across rebases (the jj UX test).
- `jj git push` to a remote works (interop via fragmentation-git layer).
- Test suite covers the 14 Backend methods + the OpStore path.

### T5 — Mirror consumes fragmentation

**Scope.**
- Per `mirror/docs/specs/mirror-store.md` §6.4: implement F-2's MirrorStore
  trait, Entry enum, boot(), Lift dispatch.
- Mirror's Cargo.toml: `fragmentation = { path = "../fragmentation",
  default-features = false }`.
- The store wraps fragmentation's `ConcurrentStore<Entry>` or
  `MemoryStore<Entry>`.
- The Layer-2 FP1 assertion lands.

**Estimate.** Per mirror-store.md, F-2 was already specified. Medium-to-large.

**Dependencies.** T1 (the cleanup unblocks mirror's dep), T3 (the surface
mirror's store wraps).

**Acceptance.** Per mirror-store.md §6.4. The grammar boot succeeds; OIDs
are stable across runs; `Lift` dispatches via `store.fetch(@<ref>)`.

### T6 — Spectral-db consumes fragmentation (out of scope; flag only)

spectral-db (task #43) consumes the same substrate. Once T3 lands, the
spectral side has fragmentation's surface to wrap. The integration is the
spectral team's spec; this tick just flags the dependency.

spectral-db likely wants the same Cuts (1/2/3) and the same surface (§3) but
adds its own typed `Entry` variants (`Project`, `Crystal`, `Gestalt`,
`Session`, `Eigenboard`) plus distribution / delta / conflict-resolution /
MNESIA-backed persistence. None of that is fragmentation's concern.

**Out of scope for this spec.** Flagged for the spectral side per
mirror-store.md §7.

### Tick ordering — the critical path

```
T1 (cleanup) ─┬─ T2 (git extraction) ─ T3 (surface) ─┬─ T4 (fragmentation-jj)
              │                                       └─ T5 (mirror F-2)
              │                                       └─ T6 (spectral-db, separate team)
              └─ (also unblocks T5's mirror integration directly via the trait split)
```

T1 is the bottleneck. Land it. Everything else is downstream.

---

## 6. Open questions

Eight. Ranked by load-bearing weight.

### 6.1 Hash function — does CoincidenceHash<5,5> cover commit-id AND change-id

jj's backend exposes `change_id_length()` and `commit_id_length()` as
separate values. The reference `GitBackend` uses 20-byte commit IDs (git
SHA-1) and 16-byte change IDs (random / synthetic). Mirror uses
`CoincidenceHash<5,5>` for grammar OIDs — a 5×5 matrix Dirac hash.

Questions:

- Does `CoincidenceHash<5,5>` produce a stable byte representation that fits
  jj's `id_type!` macro (which expects `Vec<u8>`)?
- If yes, can the same hash function produce two distinct id types (a fresh
  random `ChangeId` plus the content `CommitId`), or do we need a separate
  change-id generator?
- jj's GitBackend uses *reversed hex* for change ids (`reverse_hex()` in the
  `id_type!` macro). Why? (Per source comments: to make it visually distinct
  from commit ids.) Does CoincidenceHash have an analog?

**Provisional answer.** `CoincidenceHash<5,5>` produces deterministic
`[u8; N]` (per `bootstrap/src/hash.rs`); fits jj's id_type! shape. Change-id
is independent: fragmentation-jj generates 16 random bytes per new change
(matching jj's default). Reversed-hex display is jj's UX choice; fragmentation
doesn't have to mimic. **MOST LOAD-BEARING.** Without confirming this,
fragmentation-jj is blocked.

### 6.2 Operation log — fragmentation-native OpStore or reuse jj's SimpleOpStore

jj's `OpStore` is a separate trait recording every operation
(commit-creation, ref-update, working-copy-snapshot). jj provides
`SimpleOpStore` (filesystem-backed). fragmentation-jj v1 reuses it (per T4
scope). But fragmentation's content-addressed substrate IS a suitable
operation log: every op is a typed entry with provenance.

**Provisional answer.** v1: reuse `SimpleOpStore`. v1.5: implement
`OpStore` with fragmentation as the backing store. The native impl unlocks
operation-log-based features (jj op log, op restore) over fragmentation's
stronger consistency guarantees. **SECOND MOST LOAD-BEARING.** Decision
affects v1.5 scope.

### 6.3 Conflict representation — Fractal::Conflict variant vs side-table

Per §4.4 Gap 2. The spec recommends side-table to keep substrate clean.
Alternative: add a `Fractal::Conflict` variant.

Open sub-question: does spectral-db's conflict shape (kintsugi-tournament
fate-resolved morphism) want a side-table too, or a different variant? If
side-tables proliferate, the substrate isn't doing its job.

**Provisional answer.** Side-table for v1; revisit at v1.5 if multiple
consumers want similar variants. Likely consolidates into a single
`Fractal::Unresolved { resolution_strategy, payload }` variant where
strategy is an enum (`Merge | KintsugiTournament | ...`).

### 6.4 Working-copy semantics — reuse jj's LocalWorkingCopy vs roll our own

Per §4.4 Gap 3. The spec recommends reuse. Open: how does the jj reuse
handle fragmentation's `Lens` variants (jj doesn't have native multi-target
refs)?

**Provisional answer.** Lens unmaterialized in the working copy; show as a
symlink-like entry. The jj LocalWorkingCopy already handles symlinks; the
Lens-as-symlink convention works for v1.

### 6.5 Anonymous branches — does fragmentation's @<ref> layer support them

jj defaults to anonymous branches keyed by change-id (no human-given name).
fragmentation's `Reference` today is a named path (`@mirror/glass`). For
fragmentation-jj, anonymous branches map to change-id-keyed refs:
`refs/jj/change/<change-id-hex>`.

**Provisional answer.** fragmentation's `Reference` allows arbitrary paths;
fragmentation-jj uses `refs/jj/change/<change-id-hex>` for anonymous. No
substrate change needed.

### 6.6 Async vs sync — fragmentation's Repo is sync, jj's Backend is async

Adapter has to wrap sync calls in `async move { ... }`. Trivial via tokio
or smol, but adds a runtime dependency to fragmentation-jj.

**Provisional answer.** smol for fragmentation-jj (smaller than tokio,
fine for backend usage). jj itself uses pollster (sync executor on async
futures) — fragmentation-jj can use pollster too if it wants to stay
synchronous internally.

### 6.7 GC — fragmentation's substrate has none

jj's `Backend::gc(&self, index, keep_newer)` requires the backend to walk
the index, find reachable commits, and prune unreachable content. fragmentation
has no GC primitive.

**Provisional answer.** fragmentation-jj v1: gc returns `Ok(())` (no-op);
the `FrgmntStore` cache eviction is the closest thing. v1.5: implement
proper GC over the on-disk store. Acceptable to ship v1 without GC.

### 6.8 The spectral-db dependency — does it also want jj

If spectral-db uses fragmentation-jj as its backend, does spectral-db inherit
jj's whole UX (jj log, jj describe, etc.)? Or does spectral-db wrap
fragmentation directly, with its own surface, and fragmentation-jj is only
for mirror?

**Provisional answer.** spectral-db wraps fragmentation directly; jj is the
daily-driver VCS for *mirror*. spectral-db's distribution/delta/conflict layer
is its own thing. They share substrate, not workflow.

---

## 7. The thing the types don't say

Fragmentation today is presented as a tree library that grew git, fuse,
encryption, signing, projections, and a CLI. Read the source: it's a
VCS-substrate that grew the wrong skin. Every git2 import, every
fuser::Filesystem impl, every `[[bin]]` entry is the library trying to BE
the consumer instead of the substrate consumers use.

The destination is to name this and let the wrong skin slough off. Three
crates after the cleanup:

```
fragmentation        ← the substrate. Library only. 15 modules + 8 feature gates.
fragmentation-git    ← git interop. Push/pull/clone. CLI. FUSE.
fragmentation-jj     ← jj-native backend. Daily-driver VCS for mirror.
```

Mirror consumes fragmentation. spectral-db consumes fragmentation. Users
running mirror with `jj init --backend=fragmentation` get a native VCS
whose change-ids follow rebase, whose conflicts are typed, whose history
is content-addressed at the resolution the substrate affords (per-grammar,
not per-file). Users who need to push to GitHub get fragmentation-git's
translation layer — lossy but functional.

The substrate becomes substrate. The opinions become outlets. The library
does one thing and the rest of the world plugs into it.

That's the whole spec. The thing the types don't say is: fragmentation has
been waiting to become a VCS. It already IS one. The cleanup is the
recognition.

---

*Apache-2.0.*
*Mara. 2026-05-24.*
*"does it connect?"*
