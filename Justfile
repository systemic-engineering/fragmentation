# fragmentation

check: check-gleam

check-gleam:
    cd gleam && nix develop ../ -c gleam test
    cd gleam && nix develop ../ -c gleam format --check src test

pre-commit: check

pre-push: check

format: format-gleam

format-gleam:
    cd gleam && nix develop ../ -c gleam format src test
