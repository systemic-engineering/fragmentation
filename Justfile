# fragmentation

check: lint test format-check

lint:
    nix develop -c cargo clippy -- -D warnings

test:
    nix develop -c cargo test

format-check:
    nix develop -c cargo fmt -- --check

pre-commit: check
pre-push: check

format:
    nix develop -c cargo fmt
