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

          # GStreamer: WebKitGTK loads these at runtime for media playback.
          # Without them `tauri dev` aborts with
          # "GStreamer element appsink not found. Please install it."
          gst_all_1.gstreamer
          gst_all_1.gst-plugins-base
          gst_all_1.gst-plugins-good
          gst_all_1.gst-plugins-bad
          gst_all_1.gst-plugins-ugly
          gst_all_1.gst-libav

          # Media player used by the app
          mpv

          # Misc utilities
          git
          curl
          wget
        ];

        shellHook = ''
          # Point WebKitGTK at the GStreamer plugins from this shell, since
          # they are not installed system-wide on NixOS.
          export GST_PLUGIN_SYSTEM_PATH_1_0="${pkgs.gst_all_1.gstreamer}/lib/gstreamer-1.0:${pkgs.gst_all_1.gst-plugins-base}/lib/gstreamer-1.0:${pkgs.gst_all_1.gst-plugins-good}/lib/gstreamer-1.0:${pkgs.gst_all_1.gst-plugins-bad}/lib/gstreamer-1.0:${pkgs.gst_all_1.gst-plugins-ugly}/lib/gstreamer-1.0:${pkgs.gst_all_1.gst-libav}/lib/gstreamer-1.0"
          export GST_PLUGIN_PATH_1_0="$GST_PLUGIN_SYSTEM_PATH_1_0"

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