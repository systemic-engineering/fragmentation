# fragmentation — Roadmap

## Root Definition

Two node types. One hash function.

```
shard | fractal
```

Everything else is composition. `Lens` is a fractal with cross-tree references.
Content addressing uses git-compatible SHA. The observer is part of the commit,
not the hash — same content, different witness, different commit, same tree OID.

---

## The Binary: `frgmt`

Three verbs. Three directions. Same underlying structure.

```
frgmt collapse <ref>    # tree → artifact (build)
frgmt refract <ref>     # artifact → tree (trace)
frgmt mount <ref>       # tree → filesystem (navigate)
```

**`collapse`** resolves a content-addressed tree into a shippable artifact.
The tree of possibilities collapses to a single output. Deterministic: same
tree, same artifact, same hash. The collapse is witnessed — the output carries
a trace back to the tree that produced it.

**`refract`** is the inverse. Given an artifact (or any OID), expand it back
into the tree structure that produced it. Follow the trace lineage. See the
intermediate representations. The collapsed artifact opens back up.

**`mount`** exposes a content-addressed tree as a FUSE filesystem. Navigate
compiler output, inspect intermediate representations, diff between versions —
all as files. The development surface for everything `collapse` produces.

### Existing Surface

The CLI already has primitives that map to this:

- `frgmnt shard` → becomes internal to `collapse` (terminal node computation)
- `frgmnt fractal` → becomes internal to `collapse` (tree computation)
- `frgmnt commit` → becomes the write step of `collapse`
- `frgmnt link` → becomes `frgmt lens` (cross-tree reference)
- `frgmnt mount` → becomes `frgmt mount`
- `frgmnt sign` / `encrypt` / `decrypt` → utility subcommands, stay as-is
- `frgmnt filter` → git smudge/clean, stays as-is

The rename from `frgmnt` to `frgmt` happens when `collapse` lands.

---

## The Pipeline: `.conv` → Binary

The conversation compiler writes compilation output to git. fragmentation
provides the content-addressed tree structure. The pipeline:

```
.conv source
  → conversation compiler (parse → resolve → compile)
  → EAF + BEAM bytecode as content-addressed tree (fragmentation)
  → git objects (fragmentation writes native git objects)
  → frgmt collapse (tree → Nix derivation → release)
```

### How Collapse Produces a Binary

`frgmt collapse <ref>` reads the content-addressed tree at `<ref>`, materializes
it into a structure Nix can consume, and invokes a flake to build the release.

The bridge between fragmentation and Nix is git. fragmentation already writes
native git objects. Nix already reads git repos via `builtins.fetchGit`. The
FUSE mount is not in the build path — it's the development/inspection surface.

```
fragmentation tree (git objects)
  → builtins.fetchGit (Nix reads the repo)
  → Derivation 1: escript compiles ETF → .beam files
  → Derivation 2: OTP release packaging
  → /nix/store/... (shippable artifact)
```

The flake lives in the repo. `collapse` orchestrates: ensure the tree is written,
invoke `nix build`, return the store path. The collapse output is itself
content-addressed — same source tree, same binary, same hash.

### Two-Derivation Build

The conversation compiler produces ETF (External Term Format) containing EAF
(Erlang Abstract Format). Shipping requires compiling EAF to `.beam` bytecode
ahead of time, then packaging as an OTP release. Two derivations:

**Derivation 1: ETF → BEAM compilation.** An escript runs `binary_to_term →
compile:forms → write .beam`. This is the same three-step process
`loader_ffi.erl` does at runtime, but ahead of time:

```erlang
%% compile_etf.escript
main([InDir, OutDir]) ->
    {ok, Files} = file:list_dir(InDir),
    [compile_one(InDir, OutDir, F)
     || F <- Files, filename:extension(F) =:= ".etf"],
    ok.

compile_one(InDir, OutDir, File) ->
    {ok, Bin} = file:read_file(filename:join(InDir, File)),
    Forms = binary_to_term(Bin),
    {ok, Module, BeamBin} = compile:forms(Forms),
    OutFile = filename:join(OutDir, atom_to_list(Module) ++ ".beam"),
    file:write_file(OutFile, BeamBin).
```

**Derivation 2: OTP release packaging.** Takes the `.beam` files from
derivation 1, adds `.app` resource file, boot script, `sys.config`, `vm.args`,
and optionally bundles ERTS. Uses `relx`, `mix release`, or `systools`.

Splitting the derivations enables Nix's caching: if the ETF files don't change,
derivation 1 is cached and derivation 2 only reruns if runtime configuration
changes.

### Why Not FUSE in the Build Path

Nix builds run in a sandbox. The sandbox does not expose `/dev/fuse`. The
kernel module is not loadable from within the sandbox namespace. This is a
hard constraint, not a configuration problem.

`__noChroot = true` disables the sandbox but breaks reproducibility and
prevents binary caching. `sandbox-paths` could expose a pre-existing FUSE
mount but requires the mount to exist on every machine that builds. Neither
is viable for a shippable pipeline.

FUSE serves a different role: the development loop. Mount a tree, navigate it,
inspect compiler output as files, diff between versions. The developer sees
the content-addressed store as a filesystem. But when it's time to ship,
`collapse` reads the same git objects directly.

Two consumers of the same store. Not a chain.

### BEAM Release Shape

A conversation release is a BEAM release containing:

- Compiled `.beam` modules (from EAF, produced by the conversation compiler)
- The supervision tree (`conversation/supervisor.gleam`, `conversation/garden.gleam`)
- The `@compiler` actor and its bootstrap grammar
- Domain grammars loaded at boot

The flake uses `beam.packages` from nixpkgs. `builtins.fetchGit` reads the
compiler output repo at a pinned ref. The ETF → BEAM compilation happens in
derivation 1. The release packaging happens in derivation 2. The `.conv` → EAF
step happens before `collapse`. Collapse takes the already-compiled tree and
packages it.

---

## Content Addressing Meets Nix

Nix has its own content-addressing story. Experimental CA derivations
(`ca-derivations`) let Nix identify build outputs by their content hash
rather than their input hash. This means: if two different derivations produce
the same output bytes, they share a store path.

fragmentation's content addressing and Nix's CA derivations operate at
different levels:

| Aspect | fragmentation | Nix CA derivations |
|--------|--------------|-------------------|
| What is addressed | Fragment tree content (data + children) | Derivation output (entire directory) |
| Hash algorithm | SHA-1 (git-compatible) | SHA-256 (Nix default) |
| Granularity | Per-node (shard, fractal, lens) | Per-derivation-output (whole directory) |

They cannot directly map to each other. But CA derivations enable an
optimization: if the compiler produces identical output for different source
versions, the downstream BEAM release build is skipped. Early cutoff that
fragmentation's content addressing makes especially likely — identical
subtrees produce identical hashes.

`collapse` bridges the two systems. The fragmentation OID identifies what
went in. The Nix store path identifies what came out. The trace links both.

### Dynamic Derivations

More relevant than CA derivations is `dynamic-derivations` (experimental,
March 2025). Dynamic derivations let a build step produce another derivation
as output, which Nix then builds. This eliminates IFD for the pipeline:

1. Derivation A: run the conversation compiler, produce ETF files
2. Derivation B (generated by A): compile ETF to `.beam`, package release

Requires `experimental-features = ["dynamic-derivations" "ca-derivations"]`.
Not production-stable yet. The safe path: run the compiler outside Nix, let
Nix consume its output via `fetchGit`.

---

## What's Built

- Content-addressed tree (`Fractal<E, H>` generic over element and hash)
- Three node variants: `Shard` (terminal), `Fractal` (recursive), `Lens` (cross-tree reference)
- Git-native read/write (shards → blobs, fractals → trees)
- FUSE filesystem mount (read-write, ref-backed)
- Ed25519 signing + ECIES encryption
- Witnessed commits (author/committer as observation metadata)
- `HashAlg` trait with `Sha` implementation
- CLI with shard, fractal, commit, link, mount, sign, encrypt, decrypt, filter
- Diff between trees
- Walk/traversal

---

## What Needs to Land

**`collapse` subcommand.** Read a content-addressed tree at a git ref,
materialize it into a form Nix can consume, invoke the flake, return the store
path. The core of the shippable binary story.

**`refract` subcommand.** Given an OID, reconstruct the tree. Follow trace
lineage through parent references. Display the intermediate representations
that produced this artifact.

**Flake template.** A `flake.nix` that builds a BEAM release from
fragmentation's git objects. Uses `builtins.fetchGit` for the local repo,
`beam.packages` for the BEAM toolchain, and outputs a runnable release.

**Binary rename.** `frgmnt` → `frgmt`. Cargo.toml already has both binary
entries. When `collapse` lands, the old name becomes an alias.

**Lens in FUSE.** The FUSE mount doesn't yet resolve `Lens` nodes. A lens
should appear as a symlink (or a transparently-followed directory) pointing
to its target tree. This makes cross-tree references navigable in the
mounted filesystem.

**Trace materialization.** `collapse` needs to write a trace that links the
source tree OID to the Nix store path. This is the receipt: proof that this
source became this artifact. The trace is itself content-addressed.

---

## Open Questions

**ETF file layout in the git tree.** What directory structure should the
compiler write? One ETF file per module? Nested by namespace? A flat `etf/`
directory with `<module_name>.etf` is simplest for the escript to iterate.

**Ref naming convention.** What ref does the compiler write to?
- `refs/fragmentation/conversation/latest` — mutable, tracks latest compilation
- `refs/fragmentation/conversation/<source-hash>` — immutable, per-compilation
- Both: mutable for development, immutable for production builds

**Runtime loading vs. AOT.** The current `loader_ffi.erl` loads modules at
runtime via `code:load_binary`. For releases, we want ahead-of-time
compilation via the escript. Both paths coexist: runtime for development, AOT
for `collapse`.

**ETF compatibility across Erlang versions.** ETF is version-dependent for
certain term types. The compiler and the Nix build must use the same Erlang
version. Pin in both the flake and the compiler's build environment.

**`fetchGit` caching.** Nix aggressively caches flake inputs. If the compiler
updates the ref but Nix has cached the previous evaluation, `nix build` uses
stale content. Mitigation: `--refresh` flag or pin to specific `rev`.

---

## Sequencing

1. `frgmt collapse` — the build verb (depends on flake template)
2. Flake template for BEAM releases
3. `frgmt refract` — the trace verb
4. Lens in FUSE mount
5. Binary rename (`frgmnt` → `frgmt`)
6. Trace materialization (collapse receipt)

---

*Session 2026-03-23. Alex + Mara.*
