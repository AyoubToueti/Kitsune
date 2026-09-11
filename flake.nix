{
  description = "Kitsune — Tauri + Svelte anime streaming app";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in {
      devShells.${system}.default = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          # Rust toolchain
          rustc
          cargo
          rustfmt
          clippy
          rust-analyzer

          # Node / frontend tooling
          nodejs_22
          pnpm

          # Native build glue for Tauri
          pkg-config
          gobject-introspection
        ];

        buildInputs = with pkgs; [
          # Tauri (WebKitGTK) system dependencies
          webkitgtk_4_1
          librsvg
          gtk3
          libsoup_3
          openssl
          glib
          cairo
          pango
          gdk-pixbuf
          atk
          libayatana-appindicator

          # Media player used by the app
          mpv

          # Misc utilities
          git
          curl
          wget
        ];

        shellHook = ''
          echo "Kitsune dev shell"
          echo "  rustc : $(rustc --version)"
          echo "  cargo : $(cargo --version)"
          echo "  node  : $(node --version)"
          echo "  pnpm  : $(pnpm --version)"
          echo "  mpv   : $(mpv --version | head -1)"
        '';
      };
    };
}