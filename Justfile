# fragmentation

check: lint test format-check

lint:
    nix develop -c cargo clippy --all-features -- -D warnings

test:
    nix develop -c cargo test --all-features

format-check:
    nix develop -c cargo fmt -- --check

pre-commit: check
pre-push: check

format:
    nix develop -c cargo fmt
