{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        defaultPackage = pkgs.rustPlatform.buildRustPackage {
          pname = "pageturtle";
          version = "0.1.1";
          src = pkgs.lib.cleanSource ./.;
          cargoHash = "sha256-aml6dyRSsG/Dq5dck0WRUS6EZb24qIqFbxDHxpPFijo=";
        };
        devShell = with pkgs; mkShell {
          buildInputs = [ 
            cargo
            clippy
            rustfmt
            rustc
            rust-analyzer

            nodejs
          ];
        };
      });
}
