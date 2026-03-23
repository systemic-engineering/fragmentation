# FUSE + Nix Binary Pipeline Research

How to get from conversation compiler output (content-addressed trees in git objects) to shippable binaries via Nix.

## Executive Summary

**What works:**
- `builtins.fetchGit` reading from a local git repository where the compiler wrote its output. This is the path of least resistance. No FUSE required at build time. No sandbox violations. Content-addressed at the git level. Nix copies the content into the store and builds from there.
- Standard BEAM release packaging via `mixRelease` / `rebar3Relx` in nixpkgs. The tooling exists and is maintained.
- The fragmentation FUSE module as a development-time interface (editing, browsing compiler output), not a build-time dependency.

**What does not work:**
- FUSE inside Nix build sandboxes. The sandbox does not expose `/dev/fuse`. The kernel module is not loadable from within the sandbox namespace. This is a hard constraint, not a configuration problem.
- Nix CA derivations as a direct bridge to fragmentation's content addressing. The two systems address different things (Nix addresses derivation outputs, fragmentation addresses content trees). The concepts are adjacent but not mappable.

**Recommended path:**
`.conv` source -> conversation compiler -> ETF/EAF blobs -> git objects (via fragmentation) -> `builtins.fetchGit` -> Nix derivation that compiles EAF to .beam files -> BEAM release

The FUSE mount is valuable for development and inspection. The Nix pipeline reads directly from git.

---

## 1. FUSE + Nix Integration

### Can a Nix build read from a FUSE-mounted filesystem?

**No.** Not in any standard or recommended configuration.

Nix build sandboxes (enabled by default since Nix 2.0+) isolate the build environment from the host filesystem. The build process sees:
- Its declared dependencies in `/nix/store`
- A temporary build directory
- Private `/proc`, `/dev`, `/dev/shm`, `/dev/pts`

FUSE requires `/dev/fuse`, which is a kernel-level device node. The Nix sandbox's mount namespace does not include it. Even if you could somehow mount a FUSE filesystem on the host, the sandbox would not see it unless explicitly configured via `sandbox-paths`.

**`sandbox-paths` option:** You can bind-mount host paths into the sandbox using `sandbox-paths = /some/host/path`. This could theoretically expose a FUSE mountpoint. However:
- The FUSE mount must exist before the build starts.
- The content must be stable for the duration of the build.
- This breaks reproducibility guarantees since the build now depends on external mutable state.
- It requires every machine building this derivation to have the same FUSE mount at the same path.

**`__noChroot = true`:** Disables the sandbox for a specific derivation. Requires `sandbox = relaxed` in the builder's `nix.conf`. This:
- Breaks Nix's core reproducibility guarantee.
- Requires explicit opt-in from every user building the derivation.
- Means the build cannot be cached or substituted by Hydra/binary caches (since purity cannot be verified).
- Is never appropriate for a shippable pipeline.

**`builtins.path`:** Imports a local filesystem path into the Nix store. If a FUSE mount is at a known path, `builtins.path { path = /mnt/fragmentation; }` would copy the mounted content into the store at evaluation time (not build time). This works but:
- Evaluation is not sandboxed, so the FUSE mount is visible.
- The content is copied eagerly into the store as a snapshot.
- The hash of the store path depends on the content, so changes produce new store paths.
- This is essentially pre-materialization with extra steps.

**Import From Derivation (IFD):** IFD allows a build step to produce Nix expressions that are then evaluated. This is conceptually interesting but does not solve the FUSE problem; it just moves it. The derivation that generates the Nix expressions still runs in the sandbox.

**Conclusion:** FUSE inside Nix builds is not viable. Pre-materialization (getting the content into a form Nix can natively ingest) is required.

---

## 2. Content-Addressed Nix Stores (CA Derivations)

### Status as of 2025-2026

CA derivations remain experimental. They require `experimental-features = ca-derivations` in `nix.conf` and `__contentAddressed = true` on individual derivations.

### What they do

In standard Nix, a derivation's output path is determined by its inputs (input-addressed). Change any input and the output path changes, even if the output content is identical. CA derivations compute the output path from the output content itself. Benefits:
- **Early cutoff:** If a rebuild produces the same output, downstream rebuilds are skipped.
- **Trust model change:** Multiple users can share a store without trusting each other, since output paths are verified by content.

### Relationship to fragmentation

Fragmentation's content addressing and Nix's CA derivations operate at different levels:

| Aspect | fragmentation | Nix CA derivations |
|--------|--------------|-------------------|
| What is addressed | Fragment tree content (data + children) | Derivation output (entire directory) |
| Hash algorithm | SHA-1 (git-compatible) | SHA-256 (Nix default) |
| Granularity | Per-node (shard, fractal, lens) | Per-derivation-output (whole directory) |
| Purpose | Identify identical content across witnesses | Skip rebuilds when output unchanged |

They cannot directly map to each other. Fragmentation's SHA-1 tree hashes cannot serve as Nix CA derivation output hashes. The addressing is at fundamentally different granularities.

However, CA derivations do enable an optimization: if the compiler produces the same output for different source versions, the downstream BEAM release build would be skipped. This is "free" early cutoff that fragmentation's content addressing makes especially likely (since identical subtrees produce identical hashes).

### Dynamic Derivations

More relevant than CA derivations is the `dynamic-derivations` experimental feature (March 2025). Dynamic derivations allow a build step to produce another derivation as output, which Nix then builds. This eliminates IFD for pipeline patterns like:

1. Derivation A: run the conversation compiler, produce ETF/EAF files
2. Derivation B (generated by A): take those files and build a BEAM release

This requires `experimental-features = ["dynamic-derivations" "ca-derivations"]`. The feature is actively developed but not production-stable.

---

## 3. Git Object Store as Nix Input

### This is the recommended approach.

Since fragmentation writes git-native objects (blobs, trees, commits), the compiler output already lives in a git repository. Nix can read this directly.

### `builtins.fetchGit` with local repos

```nix
src = builtins.fetchGit {
  url = "/path/to/compiler-output-repo";
  ref = "refs/fragmentation/output";
  # or a specific rev:
  rev = "abc123...";
};
```

**Behavior with local repos:**
- If no `ref` or `rev` is given: uses current checked-out content (including uncommitted changes tracked by `git ls-files`). This is useful for development but not reproducible.
- If `ref` is given: fetches the tip of that ref. The content is copied into the Nix store.
- If `rev` is given: fetches that specific commit. Fully reproducible.
- `allRefs = true`: allows fetching revs from any ref, not just the specified one.
- Bare repos are treated as remote repositories (fetched, not read directly).

**What `fetchGit` copies into the store:** The working tree at the specified revision, exported as a directory. Not the `.git` directory itself. Just the files. This means the tree that fragmentation committed becomes a regular directory of files in `/nix/store`.

### The pipeline

The compiler writes output as fragmentation trees to a git repository. Each compilation produces a commit on a ref like `refs/fragmentation/conversation/<module>`. The flake.nix fetches from that ref:

```nix
{
  inputs.compiler-output = {
    url = "git+file:///path/to/repo?ref=refs/fragmentation/output";
    flake = false;
  };

  outputs = { self, nixpkgs, compiler-output }: {
    packages.default = nixpkgs.legacyPackages.x86_64-linux.callPackage ./release.nix {
      etfSources = compiler-output;
    };
  };
}
```

Or using `builtins.fetchGit` directly in the derivation for tighter control.

### Key constraints

- The ref must exist before `nix build` runs. The compiler must have already written its output.
- For reproducible builds, pin to a specific `rev` (commit SHA), not just a `ref`.
- The content is always copied into the Nix store. Nix does not reference git objects in-place.
- `fetchGit` with a local path and no rev/ref does not guarantee reproducibility, since it reads the current working tree state.

### Why this beats FUSE for the build pipeline

- No sandbox violations.
- Git objects are immutable and content-addressed. The same fragmentation tree always produces the same commit.
- Standard Nix evaluation. No experimental features required.
- Binary caches work. The store path is deterministic from the commit hash.

---

## 4. BEAM Releases from Nix

### Current state of the art

**`beamPackages.mixRelease`** (nixpkgs): The standard way to build Elixir/Mix releases in Nix.

```nix
{ beamPackages, fetchMixDeps }:
beamPackages.mixRelease {
  pname = "my-app";
  version = "0.1.0";
  src = ./.;
  mixFodDeps = fetchMixDeps {
    pname = "my-app-deps";
    src = ./.;
    hash = "sha256-...";
  };
}
```

This runs `mix release` inside a Nix derivation, producing a self-contained release with ERTS (Erlang Runtime System) bundled.

**`beamPackages.rebar3Relx`** (nixpkgs): For Rebar3-based Erlang projects.

**`mix2nix`**: Generates Nix expressions from `mix.lock` for precise dependency tracking. Alternative to the fixed-output-derivation approach of `fetchMixDeps`.

**Gleam in Nix**: Several options exist:
- `arnarg/nix-gleam`: Generic builder that reads `manifest.toml` for dependency hashing.
- `vic/gleam-nix`: Development shells and build packages.
- Gleam itself is packaged in nixpkgs.

### The conversation compiler's output

The compiler produces ETF (External Term Format) binaries containing EAF (Erlang Abstract Format). At runtime, `loader_ffi:load_etf_module/1` does:

```erlang
Forms = binary_to_term(EtfBinary),
{ok, Module, Binary} = compile:forms(Forms),
{module, Module} = code:load_binary(Module, "conversation", Binary).
```

This is a three-step process: deserialize ETF -> compile EAF to BEAM bytecode -> load into the running VM.

### Minimal viable release

For shipping, we need to decouple "compile" from "load." The Nix derivation should:

1. Read ETF files from the compiler output (fetched via `fetchGit`).
2. Compile each to `.beam` files using `compile:forms/2` with `[binary]`.
3. Write the `.beam` files to an output directory.
4. Package as a standard OTP release.

This can be done with an escript or a small Erlang script that runs during the Nix build:

```erlang
%% compile_etf.escript
main([InDir, OutDir]) ->
    {ok, Files} = file:list_dir(InDir),
    [compile_one(InDir, OutDir, F) || F <- Files, filename:extension(F) =:= ".etf"],
    ok.

compile_one(InDir, OutDir, File) ->
    {ok, Bin} = file:read_file(filename:join(InDir, File)),
    Forms = binary_to_term(Bin),
    {ok, Module, BeamBin} = compile:forms(Forms),
    OutFile = filename:join(OutDir, atom_to_list(Module) ++ ".beam"),
    file:write_file(OutFile, BeamBin).
```

The resulting `.beam` files then go into a release:

```nix
stdenv.mkDerivation {
  name = "conversation-release";
  src = compiler-output;  # from fetchGit

  nativeBuildInputs = [ erlang ];

  buildPhase = ''
    escript compile_etf.escript ./etf ./ebin
  '';

  installPhase = ''
    mkdir -p $out/lib/conversation/ebin
    cp ./ebin/*.beam $out/lib/conversation/ebin/
    # Add boot script, sys.config, vm.args...
  '';
}
```

For a full OTP release, you would additionally need:
- A `.app` file (application resource file)
- A `.rel` file (release resource file)
- A boot script (generated by `systools:make_script/1`)
- `sys.config` and `vm.args`
- Optionally, bundled ERTS

The `relx` tool or `mix release` can generate all of this from a properly structured OTP application.

---

## 5. FUSE in Nix Build Sandboxes

### Has anyone done this?

**No.** Searching NixOS Discourse, GitHub issues, and the broader Nix community reveals no successful use of FUSE within Nix build sandboxes.

The attempt that comes closest: a user tried to mount a `redoxfs` FUSE filesystem inside a derivation and got `fuse: device not found`. The fundamental issue is:

1. **`/dev/fuse` is not available.** The sandbox creates a minimal `/dev` with only essential devices. FUSE is not among them.
2. **Kernel modules cannot be loaded.** The sandbox runs in a restricted namespace. `modprobe fuse` fails.
3. **Privilege escalation is blocked.** FUSE mounting requires CAP_SYS_ADMIN or a setuid helper. Neither is available in the sandbox.

### Workarounds

**`__noChroot = true`:** Disables the sandbox entirely for that derivation. Not suitable for distributable builds. Requires `sandbox = relaxed` globally.

**`sandbox-paths`:** Could expose a pre-existing FUSE mount, but this is fragile, non-portable, and defeats Nix's reproducibility.

**Pre-materialization:** The only sound approach. Materialize the content-addressed store into files that Nix can ingest normally:
- Write files to a git repo (which fragmentation already does).
- Write files to a regular directory and use `builtins.path`.
- Write files to a tarball and use `fetchurl` with a content hash.

### Verdict

FUSE inside Nix builds is architecturally incompatible. The alternative is not a workaround; it is the correct design. Content-addressed storage should be materialized into a form Nix understands (git repo, store path, tarball) before the build begins.

---

## 6. The Pipeline Shape

### Recommended architecture

```
.conv source
    |
    v
conversation compiler (Rust binary)
    |
    | Produces ETF blobs containing EAF
    v
fragmentation::git::write_tree + write_commit
    |
    | Writes to refs/fragmentation/conversation/<module>
    v
git repository (local, can be bare)
    |
    | builtins.fetchGit or flake input
    v
Nix store (/nix/store/<hash>-compiler-output/)
    |
    | Nix derivation: escript compiles ETF -> .beam
    v
.beam files in $out/lib/conversation/ebin/
    |
    | Nix derivation: relx or mix release
    v
OTP release (shippable binary)
```

### The ??? step is: `builtins.fetchGit`

No FUSE mount needed. No experimental features needed. The compiler writes git objects. Nix reads git objects. The content-addressing is handled by git's object model, which fragmentation already produces natively.

### Two-derivation approach

**Derivation 1: ETF -> BEAM compilation**

Input: compiler output from git (ETF files).
Output: compiled `.beam` files.
Tool: Erlang compiler (escript or Erlang shell).

**Derivation 2: BEAM release packaging**

Input: `.beam` files from derivation 1 + runtime dependencies.
Output: OTP release tarball or directory.
Tool: `relx`, `mix release`, or manual `systools`.

Splitting this enables Nix's caching: if the ETF files don't change, derivation 1 is cached, and derivation 2 only reruns if runtime configuration changes.

### Development-time FUSE

The fragmentation FUSE mount remains valuable for development:
- Browse compiler output as a filesystem.
- Edit files and have writes create git commits automatically.
- Use standard filesystem tools (ls, cat, tree) to inspect content-addressed trees.
- Feed into editors, language servers, and other tools that expect filesystem paths.

The FUSE mount is the development interface. Git is the build interface. Same underlying content-addressed store, two access patterns.

### Flake structure

```nix
{
  description = "Conversation compiler release";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    # Pin to a specific compiler output commit for reproducibility
    compiler-output = {
      url = "git+file:///path/to/output/repo?ref=refs/fragmentation/latest&rev=<sha>";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, compiler-output }:
  let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};
    erlang = pkgs.beam.interpreters.erlang_27;
  in {
    packages.${system} = {
      beam-modules = pkgs.stdenv.mkDerivation {
        name = "conversation-beam-modules";
        src = compiler-output;
        nativeBuildInputs = [ erlang ];
        buildPhase = ''
          mkdir -p $out/ebin
          for etf in $(find . -name '*.etf'); do
            escript ${./compile_etf.escript} "$etf" "$out/ebin"
          done
        '';
        installPhase = "true";  # buildPhase writes to $out directly
      };

      release = pkgs.stdenv.mkDerivation {
        name = "conversation-release";
        src = ./release;  # release config: sys.config, vm.args, .app, .rel
        buildInputs = [ self.packages.${system}.beam-modules erlang ];
        buildPhase = ''
          # Build OTP release using the compiled .beam files
        '';
      };
    };
  };
}
```

---

## 7. Precedent and Related Work

### Content-addressed storage as Nix input

**IPFS + Nix (Obsidian Systems):** Active work on using IPFS as a content-addressed cache for Nix store paths. The Nix store path can be derived from IPFS CIDs. However, this is about distributing/caching store paths, not about using IPFS as a build input. The content-addressed layer is below Nix's evaluation, not above it.

**Tvix (tvlfyi):** Rust reimplementation of Nix with a content-addressed store (`tvix-castore`). Key differences from C++ Nix:
- Store paths are content-addressed at the file level (chunked, deduplicated).
- Replit used tvix-store to reduce their Nix cache from 6TB to 1.2TB (90% reduction).
- The store model separates path metadata from content, allowing deduplication across packages.
- Not production-ready. APIs are unstable.

Tvix's content-addressed store is architecturally closer to fragmentation than Nix's CA derivations. Both operate at the content level rather than the derivation level. However, tvix-castore uses its own chunking and hashing scheme, not git's object model. A tvix store backend that maps to git objects would be the most natural bridge -- but this does not exist and would be a significant engineering effort.

**nix-ninja (2025):** Uses dynamic derivations for incremental compilation. A Nix build step produces derivations for individual compilation units, which are then built in parallel. This is the closest precedent to the "compiler produces build inputs" pattern.

### The git-native advantage

Fragmentation's git compatibility is the strongest architectural card here. Git repositories are already first-class Nix inputs. No adapter layer is needed. The conversation compiler writing to a git repo via fragmentation means the output is immediately ingestible by Nix without any intermediate format conversion.

This is not an accident. It is a consequence of fragmentation's design decision to produce git-native SHA-1 hashes and write standard git objects (blobs, trees, commits).

### envfs (Mic92)

A FUSE filesystem in NixOS that returns symlinks to executables based on the requesting process's PATH. Demonstrates FUSE usage in the NixOS ecosystem for runtime filesystem tricks -- but explicitly not during builds.

---

## Open Questions and Risks

### Open questions

1. **ETF file layout in the git tree.** What directory structure should the compiler write? One ETF file per module? Nested by namespace? The Nix derivation needs to find and iterate over all ETF files. A flat `etf/` directory with `<module_name>.etf` files is simplest.

2. **Ref naming convention.** What ref does the compiler write to? Options:
   - `refs/fragmentation/conversation/latest` (mutable, tracks latest compilation)
   - `refs/fragmentation/conversation/<source-hash>` (immutable, per-compilation)
   - Both: mutable for development, immutable for production builds

3. **OTP application metadata.** The `.app` file needs module lists, application dependencies, and startup configuration. Who generates this? Options:
   - The conversation compiler generates it alongside ETF files.
   - A separate Nix derivation generates it from the ETF file listing.
   - It is hand-written and versioned separately.

4. **Runtime code loading vs. ahead-of-time compilation.** The current `loader_ffi.erl` loads modules at runtime. For a release, we want ahead-of-time compilation. Do both paths need to coexist? Probably yes: runtime loading for development, AOT for releases.

5. **ERTS bundling.** Should the release include ERTS? For shipping, yes (`include_erts: true`). For deployment to a system with Erlang already installed, no. The Nix derivation can support both.

6. **Gleam interop.** The conversation project has a Gleam runtime (`beam/`). How do the Gleam-compiled modules interact with the conversation-compiler-produced modules? Do they need to be in the same release? The Gleam modules would be built separately via `nix-gleam` or `mixRelease` and combined in the release.

### Risks

1. **`fetchGit` with local paths and flake evaluation caching.** Nix aggressively caches flake inputs. If the compiler updates the ref but Nix has cached the previous evaluation, `nix build` may use stale content. Mitigation: use `--refresh` flag or pin to specific `rev`.

2. **ETF compatibility across Erlang versions.** ETF (External Term Format) is Erlang-version-dependent for certain term types. If the compiler runs on one Erlang version and the Nix build uses another, `binary_to_term/1` may fail. Mitigation: pin Erlang version in the flake and the compiler's build environment.

3. **Large compilation outputs.** If the compiler produces many modules, the git repository could grow. Fragmentation's content deduplication helps (identical subtrees are stored once), but git packfiles are the real compression mechanism. Periodic `git gc` on the output repo.

4. **macOS vs Linux.** The FUSE module uses `fuser` which depends on macFUSE on macOS and libfuse on Linux. The Nix build pipeline runs on Linux (typically). The FUSE development interface may be macOS-only in practice. This is fine since FUSE is not in the build path.

5. **Dynamic derivations maturity.** If you want the pipeline to be fully Nix-native (compiler runs as a Nix derivation, output feeds into the release derivation), dynamic derivations are the right tool but are experimental. The safe path is to run the compiler outside Nix and let Nix consume its output.
