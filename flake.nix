{
  description = "your personal rust based cephalon";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane = {
      url = "github:ipetkov/crane";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };

    flake-utils.url = "github:numtide/flake-utils";

    detection_model = {
      url = "https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.rten";
      flake = false;
    };

    recognition_model = {
      url = "https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.rten";
      flake = false;
    };
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    flake-utils,
    rust-overlay,
    detection_model,
    recognition_model,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      overlays = [(import rust-overlay)];
      pkgs = import nixpkgs {
        inherit system overlays;
      };

      rustToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

      craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

      commonArgs = {
        strictDeps = true;

        nativeBuildInputs = with pkgs; [
          pkg-config
        ];

        buildInputs = with pkgs; [
          openssl
          libGL
          wayland
          xorg.libxcb
          libgbm
          pipewire
        ];

        # TODO: these 2 should only be during build, not sure if it is currently
        DETECTION_MODEL = detection_model;
        RECOGNITION_MODEL = recognition_model;
      };

      cargoArtifacts = craneLib.buildDepsOnly commonArgs;

      # Build the actual crate itself, reusing the dependency
      # artifacts from above.
      cephalon_rust = craneLib.buildPackage (commonArgs
        // {
          inherit cargoArtifacts;
        });
    in {
      packages = {
        default = cephalon_rust;
      };
      devShells = {
        default = craneLib.devShell (commonArgs
          // {
            packages = with pkgs; [
              bacon
              pkg-config
              rust-analyzer
              rustfmt
              cargo-dist
              cargo-edit
              cargo-nextest
              cargo-llvm-cov
            ];
            shellHook = ''
              export $(cat config.env | xargs)
              export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath commonArgs.buildInputs}:$LD_LIBRARY_PATH
            '';
          });
      };
    });
}
