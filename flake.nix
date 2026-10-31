{
  description = "Kitsune — Qt6/QML anime streaming app";

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

          # Locates libmpv's include/link flags (build.rs probes it).
          pkg-config

          # Runs scripts/sync-version.mjs. No package manager and no frontend
          # toolchain: the QML is compiled into the binary by build.rs.
          nodejs_22

          # Qt6 build tooling. CMake + Ninja drive the cxx-qt build;
          # wrapQtAppsHook wires the runtime Qt plugin paths into any binary it
          # wraps (without it, a Qt app cannot find its platform plugin and
          # aborts with "could not find or load the Qt platform plugin").
          cmake
          ninja
          qt6.wrapQtAppsHook
        ];

        buildInputs = with pkgs; [
          # TLS for reqwest's default-tls feature (AniList, Jikan, Nyaa).
          openssl

          # The media player the app embeds. `mpv-unwrapped` ships the libmpv
          # shared library and the mpv/render.h header the video surface links
          # against; the plain `mpv` wrapper does not expose them.
          mpv
          mpv-unwrapped

          # Qt6. `qtdeclarative` is QML/Quick; `qtbase` gives QtCore/Gui plus the
          # platform plugins; `qtsvg` supplies the SVG image-format PLUGIN, which
          # is what lets a QML `Image` decode an SVG at runtime (the linked
          # Qt6Svg library alone is not enough -- Qt loads the decoder as a
          # plugin from `plugins/imageformats`).
          #
          # No QtMultimedia: playback is embedded libmpv through the render API,
          # so Qt only provides the window and the QML scene graph.
          qt6.qtbase
          qt6.qtdeclarative
          qt6.qtsvg

          # Misc utilities
          git
          curl
        ];

        shellHook = ''
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
          echo "  mpv   : $(mpv --version | head -1)"
        '';
      };
    };
}