{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    flake-utils,
    rust-overlay,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [(import rust-overlay)];
      };

      INPUT_EVENT_CODES_PATH = "${pkgs.linuxHeaders}/include/linux/input-event-codes.h";

      selectToolchain = p:
        p.rust-bin.stable.latest.default.override {
          extensions = ["rust-analyzer" "rust-src"];
        };

      craneLib = (crane.mkLib pkgs).overrideToolchain selectToolchain;

      sourceFilter = path: type: (craneLib.filterCargoSources path type);

      commonArgs = {
        src = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter = sourceFilter;
          name = "hands-hub-source";
        };
        strictDeps = true;

        buildInputs = [
          # Add additional build inputs here
        ];
      };

      cargoArtifacts = craneLib.buildDepsOnly (commonArgs
        // {
          pname = "hands-hub-deps";
        });

      hands-hub = craneLib.buildPackage (commonArgs
        // {
          inherit cargoArtifacts;

          inherit INPUT_EVENT_CODES_PATH;
        });

      watch = pkgs.writeScriptBin "watch" ''
        cargo watch --clear --delay .1 -x 'clippy --workspace' -x 'nextest run --workspace' -x 'doc --workspace'
      '';
    in {
      checks = {
        inherit hands-hub;
      };

      packages.default = hands-hub;

      devShells.default = craneLib.devShell {
        checks = self.checks.${system};

        inherit INPUT_EVENT_CODES_PATH;

        packages = [
          pkgs.cargo-nextest
          pkgs.cargo-watch
          watch
        ];
      };
    });
}
