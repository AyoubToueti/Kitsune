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

          # Qt6 build tooling (the migration target). CMake + Ninja drive the
          # cxx-qt build; wrapQtAppsHook wires the runtime Qt plugin paths into
          # any binary it wraps (without it, a Qt app cannot find its platform
          # plugin and aborts with "could not find or load the Qt platform
          # plugin").
          cmake
          ninja
          qt6.wrapQtAppsHook
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

          # Media player used by the app. `mpv-unwrapped` ships the libmpv
          # shared library and the mpv/render.h header the embedded player
          # links against; the plain `mpv` wrapper does not expose them.
          mpv
          mpv-unwrapped

          # Qt6: the migration target. `qtdeclarative` is QML/Quick.
          # `qtbase` gives QtCore/Gui/Widgets and the platform plugins.
          # `qtsvg` supplies the SVG image-format PLUGIN, which is what lets a
          # QML `Image` decode an SVG at runtime (the linked Qt6Svg library
          # alone is not enough -- Qt loads the decoder as a plugin from
          # `plugins/imageformats`).
          # No QtMultimedia -- playback is embedded libmpv (render API), so
          # Qt only needs to provide the window and the QML scene graph.
          qt6.qtbase
          qt6.qtdeclarative
          qt6.qtsvg

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

          # --- Qt6 merged prefix (REQUIRED for the CXX-Qt build) ---------------
          # CXX-Qt's Cargo-only build (qt-build-utils) asks `qmake -query
          # QT_INSTALL_LIBS` for a single lib directory and then looks for each
          # Qt module's .prl file there. On NixOS, Qt modules are split across
          # SEPARATE store paths (qtbase, qtdeclarative, ...), so the qtbase
          # qmake reports only qtbase/lib -- and libQt6Qml.prl (which lives in
          # qtdeclarative/lib) is never found, aborting with
          # "Could not find a prl path for Qt module: Qml".
          #
          # Fix: build ONE prefix that symlinks every Qt module's dirs together,
          # drop a qt.conf in it, and wrap qmake to always pass that qt.conf.
          # Then `qmake -query QT_INSTALL_LIBS` returns the merged lib dir, where
          # every module's .prl is present. Symlinks (not hardlinks) so the tree
          # stays removable.
          export QT6_MERGED_PREFIX="''${XDG_CACHE_HOME:-$HOME/.cache}/kitsune-qt6-merged"
          if [ ! -f "$QT6_MERGED_PREFIX/.ready" ]; then
            rm -rf "$QT6_MERGED_PREFIX"
            # `libexec` matters: qmake -query QT_HOST_LIBEXECS points here and
            # that is where `moc` lives (not `bin`), so the merge must include it
            # or cxx-qt-build aborts with "Could not find moc".
            for sub in lib include plugins qml mkspecs bin libexec; do
              mkdir -p "$QT6_MERGED_PREFIX/$sub"
            done
            # QT_ADDITIONAL_PACKAGES_PREFIX_PATH lists every Qt module prefix
            # nixpkgs injected; symlink each module's dirs into one tree.
            for p in ''${QT_ADDITIONAL_PACKAGES_PREFIX_PATH//:/ }; do
              case "$p" in
                *qtbase*|*qtdeclarative*|*qtshadertools*|*qtsvg*|*qtlanguageserver*)
                  for sub in lib include plugins qml mkspecs bin libexec; do
                    [ -d "$p/$sub" ] || continue
                    for f in "$p/$sub"/*; do
                      [ -e "$f" ] || continue
                      ln -sfn "$f" "$QT6_MERGED_PREFIX/$sub/$(basename "$f")"
                    done
                  done
                  ;;
              esac
            done
            cat > "$QT6_MERGED_PREFIX/qt.conf" <<QTCONF
[Paths]
Prefix = $QT6_MERGED_PREFIX
Libraries = $QT6_MERGED_PREFIX/lib
Headers = $QT6_MERGED_PREFIX/include
Plugins = $QT6_MERGED_PREFIX/plugins
Qml2Imports = $QT6_MERGED_PREFIX/qml
QTCONF
            touch "$QT6_MERGED_PREFIX/.ready"
          fi
          # A qmake that always injects the merged qt.conf.
          cat > "$QT6_MERGED_PREFIX/qmake" <<QMAKEWRAP
#!/bin/sh
exec "${pkgs.qt6.qtbase}/bin/qmake" -qtconf "$QT6_MERGED_PREFIX/qt.conf" "\$@"
QMAKEWRAP
          chmod +x "$QT6_MERGED_PREFIX/qmake"
          # Prepend so `qmake` on PATH is the wrapper, and pin QMAKE explicitly
          # since qt-build-utils reads the env var first.
          export QMAKE="$QT6_MERGED_PREFIX/qmake"
          export PATH="$QT6_MERGED_PREFIX:$PATH"

          # Let Qt find its IMAGE-FORMAT plugins. On NixOS each Qt module keeps
          # its plugins under `<prefix>/lib/qt-6/plugins`, and the merge above
          # only links top-level `<prefix>/plugins` -- so the SVG decoder
          # (`libqsvg.so`, shipped by qtsvg, not qtbase) never reaches the
          # merged tree and a QML `Image` cannot render an SVG at runtime.
          # Point QT_PLUGIN_PATH straight at the module's plugin dir.
          export QT_PLUGIN_PATH="${pkgs.qt6.qtsvg}/lib/qt-6/plugins''${QT_PLUGIN_PATH:+:$QT_PLUGIN_PATH}"

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