# fragmentation

check:
    nix develop -c gleam test
    nix develop -c gleam format --check src test

pre-push: check

format:
    nix develop -c gleam format src test
