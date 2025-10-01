{
  description = "Development shell with rust and additional tooling";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    polkadot-overlay = {
      # 2506-1
      url = "github:andresilva/polkadot.nix/282edc2ce11c2934237e379a778ad1a6978715f0";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, polkadot-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays =
            [ rust-overlay.overlays.default polkadot-overlay.overlays.default ];
        };

        nativeBuildInputs = with pkgs; [
          (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
          clang
          rocksdb
          llvmPackages.libclang.lib
          pkg-config
          openssl
          cargo-nextest
          git-cliff
          toml-cli
          cargo-deb
          polkadot
          zombienet
          zepter
          subwasm
        ];
      in {
        devShell = pkgs.mkShell {
          inherit nativeBuildInputs;

          PROTOC = "${pkgs.protobuf}/bin/protoc";
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath nativeBuildInputs;

          shellHook = "cargo install cargo-deny@0.18.4";
        };
      });
}
