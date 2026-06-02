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
