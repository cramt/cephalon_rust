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
            targets.x86_64-pc-windows-msvc.latest.rust-std
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
          inherit src;
          strictDeps = true;

          nativeBuildInputs = with pkgs; [
            pkg-config
            python3
            cmake
          ];

          buildInputs = with pkgs; [
            openssl
            glib
            libGL
            libGLU
          ] ++ (with pkgs.xorg; [
            libxcb
            libXcursor
            libXrandr
            libXi
            libX11
            wayland
            libxkbcommon
            libXinerama
          ]) ++ lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
            pkgs.darwin.apple_sdk.frameworks.Security
          ];
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        cephalon_rust = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;

          nativeBuildInputs = (commonArgs.nativeBuildInputs or [ ]) ++ [
            pkgs.sqlx-cli
          ];

          preBuild = ''
            export DATABASE_URL=sqlite:./db.sqlite3
            sqlx database create
            sqlx migrate run
          '';
        });
      in
      {
        checks = {
          inherit cephalon_rust;
        };

        packages = {
          default = cephalon_rust;
          inherit cephalon_rust;
        };

        devShells.default = craneLib.devShell {
          checks = self.checks.${system};

          packages = with pkgs; [
            sqlx-cli
            bacon
            sqlite
            cargo-dist
            oranda.packages.${system}.oranda
          ];

          shellHook = ''
            export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath commonArgs.buildInputs}:$LD_LIBRARY_PATH
            touch config.env
            export $(grep -v '^#' config.env | xargs -d '\n')
          '';
        };
      });
}
