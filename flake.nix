{
  description = "fragmentation — content-addressed, arbitrary-depth fragment trees";
  inputs = {
    nixpkgs.url     = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        fragmentation = pkgs.rustPlatform.buildRustPackage {
          pname = "fragmentation";
          version = "0.1.0";
          src = pkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;
          buildFeatures = [ "cli" ];
          nativeBuildInputs = [ pkgs.pkg-config ];
          # Integration tests require git/ssh/gpg features not enabled in this build
          doCheck = false;
          buildInputs = [
            pkgs.openssl pkgs.zlib
          ] ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
            pkgs.libiconv
            pkgs.apple-sdk_15
          ];
        };
      in {
        packages.default = fragmentation;
        packages.fragmentation = fragmentation;

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
            pkgs.apple-sdk_15
          ];
          shellHook = ''
            export LANG=en_US.UTF-8
            export CARGO_HOME=$PWD/.nix-cargo
            export PATH=$CARGO_HOME/bin:$PATH
            export LLVM_COV=${pkgs.llvmPackages.llvm}/bin/llvm-cov
            export LLVM_PROFDATA=${pkgs.llvmPackages.llvm}/bin/llvm-profdata
          '';
        };
      }
    ) // {
      # Cross-system library: nix functions that use the fragmentation binary
      lib.project =
        { pkgs
        , fragmentation ? self.packages.${pkgs.system}.default
        , src
        , lenses ? {}
        , name ? "projection"
        }:
        let
          # Convert attrset { "target" = "source"; } to manifest JSON
          lensEntries = builtins.map
            (target: { source = lenses.${target}; inherit target; })
            (builtins.attrNames lenses);
          manifestJson = builtins.toJSON { lenses = lensEntries; };
          manifestFile = pkgs.writeText "lenses.json" manifestJson;
        in
        pkgs.runCommand name {
          nativeBuildInputs = [ fragmentation ];
        } ''
          mkdir -p $out
          fragmentation project \
            --manifest ${manifestFile} \
            --source ${src} \
            --output $out
        '';
    };
}
