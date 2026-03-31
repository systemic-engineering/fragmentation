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
            pkgs.cargo-llvm-cov pkgs.llvmPackages.llvm
            pkgs.git pkgs.just
            pkgs.openssl pkgs.zlib
            pkgs.gleam pkgs.erlang
          ] ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
            pkgs.libiconv
            pkgs.macfuse-stubs
            pkgs.apple-sdk_15           # Security, SystemConfiguration, etc.
          ];
          shellHook = ''
            export LANG=en_US.UTF-8
            export CARGO_HOME=$PWD/.nix-cargo
            export PATH=$CARGO_HOME/bin:$PATH
            export LLVM_COV=${pkgs.llvmPackages.llvm}/bin/llvm-cov
            export LLVM_PROFDATA=${pkgs.llvmPackages.llvm}/bin/llvm-profdata
          '';
        };
      });
}
