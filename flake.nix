{
  description = "Minimal flake environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = { url = "github:oxalica/rust-overlay"; inputs.nixpkgs.follows = "nixpkgs"; inputs.flake-utils.follows = "flake-utils"; };
    pre-commit-hooks.url = "github:cachix/pre-commit-hooks.nix";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, pre-commit-hooks }:
    with flake-utils.lib;

    let overlays = [ rust-overlay.overlays.default (_self: super: { rustc = super.rust-bin.stable.latest.default; }) ]; in
    eachDefaultSystem (system:
      let pkgs = import nixpkgs { inherit system overlays; }; in
      {
        checks = {
          pre-commit-check = pre-commit-hooks.lib.${system}.run {
            src = ./.;
            hooks = {
              nixpkgs-fmt.enable = true;
              nix-linter.enable = true;
              clippy =
                let
                  wrapper = pkgs.symlinkJoin {
                    name = "clippy-wrapped";
                    paths = [ pkgs.rustc ];
                    nativeBuildInputs = [ pkgs.makeWrapper ];
                    postBuild = ''
                      wrapProgram $out/bin/cargo-clippy \
                        --prefix PATH : ${lib.makeBinPath [ pkgs.rustc ]}
                    '';
                  };
                in
                {
                  name = "clippy";
                  description = "Lint Rust code.";
                  entry = "${wrapper}/bin/cargo-clippy clippy";
                  files = "\\.(rs|toml)$";
                  pass_filenames = false;
                };
              rustfmt =
                let
                  wrapper = pkgs.symlinkJoin {
                    name = "rustfmt-wrapped";
                    paths = [ pkgs.rustc ];
                    nativeBuildInputs = [ pkgs.makeWrapper ];
                    postBuild = ''
                      wrapProgram $out/bin/cargo-fmt \
                        --prefix PATH : ${lib.makeBinPath [ pkgs.rustc ]}
                    '';
                  };
                in
                {
                  name = "rustfmt";
                  description = "Format Rust code.";
                  entry = "${wrapper}/bin/cargo-fmt fmt -- --check --color always";
                  files = "\\.(rs|toml)$";
                  pass_filenames = false;
                };
            };
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            bacon
            cargo-audit
            cargo-outdated
            cargo-watch
            crate2nix
            openssl.dev
            pkg-config
            rustc
          ];

          shellHook = ''
            ${self.checks.${system}.pre-commit-check.shellHook}
          '';
        };
      });
}
