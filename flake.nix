{
  description = "Development shell with rust and additional tooling";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    polkadot-overlay = {
      # 2412-3
      url = "github:andresilva/polkadot.nix/9f6aa99b30c21469d58ba226b9bfc85e594c499e";
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
          cargo-deny
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
        };
      });
}
