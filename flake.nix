{
  description = "fragmentation — content-addressed, arbitrary-depth fragment trees";
  inputs = {
    nixpkgs.url     = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let pkgs = nixpkgs.legacyPackages.${system};
      in {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            pkgs.rustc pkgs.cargo pkgs.clippy pkgs.rustfmt
            pkgs.rust-analyzer pkgs.pkg-config
            pkgs.git pkgs.just
          ];
          shellHook = ''
            export LANG=en_US.UTF-8
            export CARGO_HOME=$PWD/.nix-cargo
            export PATH=$CARGO_HOME/bin:$PATH
          '';
        };
      });
}
