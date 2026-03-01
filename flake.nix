{
  description = "fragmentation — content-addressed, arbitrary-depth fragment trees";

  inputs = {
    nixpkgs.url     = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs     = nixpkgs.legacyPackages.${system};
        beamPkgs = pkgs.beam.packages.erlang_27;
        erlang   = pkgs.erlang_27;
        gleam    = pkgs.gleam;
        rebar3   = beamPkgs.rebar3;
      in {
        devShells.default = pkgs.mkShell {
          buildInputs = [ gleam erlang rebar3 pkgs.git pkgs.just ];
          shellHook = ''
            export LANG=en_US.UTF-8
          '';
        };
      });
}
