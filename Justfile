# fragmentation

check: check-gleam check-rust

check-gleam:
    cd gleam && nix develop ../ -c gleam test
    cd gleam && nix develop ../ -c gleam format --check src test

check-rust:
    nix develop -c cargo clippy --manifest-path rust/Cargo.toml -- -D warnings
    nix develop -c cargo test --manifest-path rust/Cargo.toml
    nix develop -c cargo fmt --manifest-path rust/Cargo.toml -- --check

pre-commit: check

pre-push: check

format: format-gleam format-rust

format-gleam:
    cd gleam && nix develop ../ -c gleam format src test

format-rust:
    nix develop -c cargo fmt --manifest-path rust/Cargo.toml
