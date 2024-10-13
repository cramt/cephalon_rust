{
  description = "your personal rust based cephalon";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane = {
      url = "github:ipetkov/crane";
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

        craneLib = crane.mkLib pkgs;

        commonArgs = {
          strictDeps = true;

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs = with pkgs; [
            openssl
            freetype

            libxkbcommon
            libGL

            # WINIT_UNIX_BACKEND=wayland
            wayland

            # WINIT_UNIX_BACKEND=x11
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi
            xorg.libxcb
            xorg.libX11
          ];
        };


        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the actual crate itself, reusing the dependency
        # artifacts from above.
        cephalon_rust = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
        });
      in
      {
        packages = {
          default = cephalon_rust;
        };
        devShells = {
          default = craneLib.devShell (commonArgs // {
            packages = with pkgs; [
              bacon
              pkg-config
              rust-analyzer
              rustfmt
              cargo-dist
              cargo-edit
            ];
            shellHook = ''
              export $(cat config.env | xargs)
              export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath commonArgs.buildInputs}:$LD_LIBRARY_PATH
            '';
          });
        };
      });
}
