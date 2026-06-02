# fragmentation
#
# T2 of fragmentation-mcp added the `install` recipe — installs the
# `frgmnt` binary (the MCP server) to ${INSTALL_DIR:-~/.local/bin}.
# Pattern-matched on mirror's Justfile install recipe; matching shape
# keeps the binary's discoverability consistent across the substrate.

# Install destination for `just install`. Override on the CLI:
#   just install INSTALL_DIR=/usr/local/bin
INSTALL_DIR := env_var_or_default("INSTALL_DIR", env_var("HOME") + "/.local/bin")

# Cargo target dir — honour the flake's CARGO_TARGET_DIR if present;
# fall back to the in-tree default.
CARGO_TARGET := env_var_or_default("CARGO_TARGET_DIR", justfile_directory() + "/target")

# The `frgmnt` binary's release path (per vcs/mcp/Cargo.toml [[bin]]).
FRGMNT_BIN_RELEASE := CARGO_TARGET + "/release/frgmnt"

check: lint test test-gleam format-check

lint:
    nix develop -c cargo clippy --all-features -- -D warnings

test:
    nix develop -c cargo test --features ssh,cli,fuse

test-gleam:
    nix develop -c sh -c 'cd "$(git rev-parse --show-toplevel)/gleam" && gleam test'

test-mount:
    nix develop -c cargo test --all-features

format-check:
    nix develop -c cargo fmt -- --check

pre-commit: check
pre-push: check

format:
    nix develop -c cargo fmt

# Build the `frgmnt` MCP server binary in release mode.
build-frgmnt:
    cargo build -p fragmentation-mcp --release --bin frgmnt

# Install the `frgmnt` release binary to {{INSTALL_DIR}}/frgmnt.
# Override with `just install INSTALL_DIR=/usr/local/bin`.
#
# Match the pattern of mirror's Justfile install recipe — the goal
# is `just install` puts `frgmnt` on PATH so MCP clients can find
# it via the bare binary name in their `.mcp.json` configs.
install: build-frgmnt
    @mkdir -p {{INSTALL_DIR}}
    install -m 0755 {{FRGMNT_BIN_RELEASE}} {{INSTALL_DIR}}/frgmnt
    @echo "installed: {{INSTALL_DIR}}/frgmnt"
    @echo "ensure PATH contains {{INSTALL_DIR}}"

# Merge the current branch into main.
#
# - Refuses if on main, or if working tree is dirty.
# - Fast-forwards if possible; falls back to --no-ff merge commit.
# - Runs the test suite after the merge.
# - Rebuilds + installs the frgmnt binary so the live MCP picks up
#   the new substrate.
# - Push stays explicit — run `git push origin main` when ready.
merge:
    #!/usr/bin/env bash
    set -euo pipefail
    branch=$(git rev-parse --abbrev-ref HEAD)
    if [ "$branch" = "main" ]; then
        echo "✖ error: already on main" >&2
        exit 1
    fi
    # Allow only submodule pointer drift (the `m` status line) and untracked
    # files that are gitignored already (no `??` should appear under normal
    # operation). Anything else — reject.
    dirty=$(git status --porcelain | grep -vE '^(\?\? |m  )' || true)
    if [ -n "$dirty" ]; then
        echo "✖ error: working tree dirty. Commit or stash first." >&2
        git status --short >&2
        exit 1
    fi
    echo "→ merging $branch into main"
    git checkout main
    git pull --ff-only origin main
    if ! git merge --ff-only "$branch" 2>/dev/null; then
        echo "→ ff-only failed; creating merge commit"
        git merge --no-ff --no-gpg-sign "$branch" -m "🔀 merge $branch into main"
    fi
    echo "→ running tests"
    just test
    echo "→ rebuilding and installing frgmnt"
    just install
    echo "✔ merged $branch into main; frgmnt reinstalled at {{INSTALL_DIR}}/frgmnt"
    echo "  next: \`git push origin main\` when ready"
