{
  description = "your personal rust based cephalon";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";

    oranda = {
      url = "github:axodotdev/oranda";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils, fenix, oranda, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        inherit (pkgs) lib;

        toolchain = with fenix.packages.${system};
          combine [
            minimal.rustc
            minimal.cargo
            targets.x86_64-unknown-linux-gnu.latest.rust-std
            targets.x86_64-pc-windows-gnu.latest.rust-std
          ];

        craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;

        sqlFilter = path: _type: null != builtins.match ".*sql$" path;
        sqlOrCargo = path: type: (sqlFilter path type) || (craneLib.filterCargoSources path type);

        src = lib.cleanSourceWith {
          src = ./.;
          filter = sqlOrCargo;
          name = "source";
        };

        commonArgs = {
          strictDeps = true;

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs = with pkgs; [
            openssl
          ];

          # fixes issues related to libring
          TARGET_CC = "${pkgs.pkgsCross.mingwW64.stdenv.cc}/bin/${pkgs.pkgsCross.mingwW64.stdenv.cc.targetPrefix}cc";

          #fixes issues related to openssl
          OPENSSL_DIR = "${pkgs.openssl.dev}";
          OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
          OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include/";

          depsBuildBuild = with pkgs; [
            pkgsCross.mingwW64.stdenv.cc
            pkgsCross.mingwW64.windows.pthreads
          ];

        };


        commonArgsDaemon = commonArgs // {
          pname = "cephalon_rust_daemon";
          src = src;
          cargoExtraArgs = "-p cephalon_rust_daemon";
          version = "0.1.0";
        };

        cargoArtifactsDaemon = craneLib.buildDepsOnly commonArgsDaemon;

        # Build the actual crate itself, reusing the dependency
        # artifacts from above.
        cephalon_rust_daemon = craneLib.buildPackage (commonArgsDaemon // {
          inherit cargoArtifactsDaemon;

          nativeBuildInputs = (commonArgsDaemon.nativeBuildInputs or [ ]) ++ [
            pkgs.sqlx-cli
          ];

          preBuild = ''
            export DATABASE_URL=sqlite:./db.sqlite3
            sqlx database create
            sqlx migrate run
          '';
        });

        commonArgsOverlay = commonArgs // {
          pname = "cephalon_rust_overlay";
          src = craneLib.cleanCargoSource ./.;
          cargoExtraArgs = "-p cephalon_rust_overlay";
          version = "0.1.0";
        };

        cargoArtifactsOverlay = craneLib.buildDepsOnly commonArgsOverlay;

        cephalon_rust_overlay = craneLib.buildPackage (commonArgsOverlay // {
          inherit cargoArtifactsOverlay;

          strictDeps = true;
          doCheck = false;
        });
      in
      {
        packages = {
          overlay = cephalon_rust_overlay;
          daemon = cephalon_rust_daemon;
        };
        devShells = {
          default = craneLib.devShell (commonArgsDaemon // {
            packages = with pkgs; [
              bacon
              sqlx-cli
              pkg-config
              rust-analyzer
              rustfmt
              wineWowPackages.staging
            ];
            shellHook = ''
              export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath commonArgsDaemon.buildInputs}:$LD_LIBRARY_PATH
            '';
          });
          overlay = craneLib.devShell (commonArgsOverlay // {
            packages = with pkgs; [
              bacon
              wineWowPackages.staging
              rust-analyzer
              rustfmt
            ];
            shellHook = ''
            '';
          });
          daemon = craneLib.devShell (commonArgsDaemon // {
            packages = with pkgs; [
              bacon
              sqlx-cli
              pkg-config
              rust-analyzer
              rustfmt
            ];
            shellHook = ''
              export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath commonArgsDaemon.buildInputs}:$LD_LIBRARY_PATH
            '';
          });
        };
      });
}
