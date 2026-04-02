{
  description = "fragmentation — content-addressed, arbitrary-depth fragment trees";

  inputs = {
    nixpkgs.url     = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    # The agent that maintains this repo.
    # cairn witnesses the work. mara does the work.
    cairn.url = "git+ssh://git@github.com/systemic-engineering/cairn";
    cairn.inputs.nixpkgs.follows = "nixpkgs";

    mara.url = "git+ssh://git@github.com/systemic-engineering/mara";
    mara.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, flake-utils, cairn, mara }:
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

        # Agent shell — mara maintains this repo, cairn witnesses the work.
        #
        # `nix develop .#agent` drops into a BEAM environment where mara
        # can observe, lint, and maintain the codebase. cairn records
        # every session as a content-addressed witness chain.
        #
        # This is the repo's declaration of who owns it. Fork the repo,
        # run the agent, get the same maintenance infrastructure.
        devShells.agent = pkgs.mkShell {
          buildInputs = [
            # The agent
            pkgs.gleam pkgs.erlang_27
            pkgs.beam.packages.erlang_27.rebar3

            # The repo's own tools
            fragmentation
            pkgs.git pkgs.just

            # Witnessing
            pkgs.openssh
          ];

          # Agent identity
          CAIRN_AGENT = "mara";
          CAIRN_REPO  = "fragmentation";

          shellHook = ''
            export LANG=en_US.UTF-8

            echo "fragmentation — agent: mara"
            echo "cairn witness chain active"
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
