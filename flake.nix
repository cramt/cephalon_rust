{
  description = "your personal rust based cephalon";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, crane, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        inherit (pkgs) lib;

        craneLib = crane.mkLib pkgs;

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

          nativeBuildInputs = [
            pkgs.pkg-config
          ];

          buildInputs = [
            pkgs.openssl
          ] ++ lib.optionals pkgs.stdenv.isDarwin [
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
          ];

          shellHook = ''
            touch config.env
            export $(grep -v '^#' config.env | xargs -d '\n')
          '';
        };
      });
}
