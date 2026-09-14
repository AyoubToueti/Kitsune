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

          # Networking for WebKitGTK: it fetches remote <img> sources in its own
          # network process via GIO, which resolves https:// through
          # glib-networking's TLS module. Without it remote images fail
          # silently, even though reqwest in Rust still works.
          glib-networking
          gsettings-desktop-schemas

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
          # NOTE: `${pkgs.gst_all_1.gstreamer}` resolves to the `-bin` output,
          # which holds no plugins. The CORE elements WebKitGTK needs --
          # filesrc, typefind, fakesink, queue -- live in the `out` output, so
          # `.out` must be named explicitly. Without it decodebin cannot build
          # a pipeline and EVERY <video> playback fails, whatever the codec.
          export GST_PLUGIN_SYSTEM_PATH_1_0="${pkgs.gst_all_1.gstreamer.out}/lib/gstreamer-1.0:${pkgs.gst_all_1.gst-plugins-base}/lib/gstreamer-1.0:${pkgs.gst_all_1.gst-plugins-good}/lib/gstreamer-1.0:${pkgs.gst_all_1.gst-plugins-bad}/lib/gstreamer-1.0:${pkgs.gst_all_1.gst-plugins-ugly}/lib/gstreamer-1.0:${pkgs.gst_all_1.gst-libav}/lib/gstreamer-1.0"
          export GST_PLUGIN_PATH_1_0="$GST_PLUGIN_SYSTEM_PATH_1_0"

          # Same story for HTTPS: the GIO TLS module lives in the store, but
          # NixOS puts no GIO module directory on the default search path, so
          # point GIO at it explicitly.
          export GIO_EXTRA_MODULES="${pkgs.glib-networking}/lib/gio/modules"
          # WebKitGTK reads GSettings; without the schemas it warns and some
          # proxy/locale resolution paths misbehave.
          export XDG_DATA_DIRS="${pkgs.gsettings-desktop-schemas}/share''${XDG_DATA_DIRS:+:$XDG_DATA_DIRS}"

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