# fragmentation

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
