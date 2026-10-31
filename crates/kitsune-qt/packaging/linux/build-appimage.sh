#!/usr/bin/env bash
# Assemble the AppDir and produce a portable AppImage for the Qt build.
#
# Run from the repo root, after `cargo build -p kitsune-qt --release`, with
# QT_ROOT_DIR pointing at the Qt kit (install-qt-action sets it) and
# LINUXDEPLOY / LINUXDEPLOY_PLUGIN_QT pointing at the downloaded tools.
#
# linuxdeploy is used rather than a hand-rolled copy step because it resolves
# the transitive shared-library closure (libmpv, xcb, fontconfig, ...) that a
# Qt app needs on a distro that may not have them installed.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
cd "$ROOT"

: "${QT_ROOT_DIR:?QT_ROOT_DIR must point at the Qt kit}"
: "${LINUXDEPLOY:?LINUXDEPLOY must point at the linuxdeploy AppImage}"
: "${LINUXDEPLOY_PLUGIN_QT:?LINUXDEPLOY_PLUGIN_QT must point at linuxdeploy-plugin-qt}"

VERSION="${VERSION:-0.1.0}"
OUT="dist"
APPDIR="$OUT/AppDir"

rm -rf "$OUT"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" \
         "$APPDIR/usr/share/icons/hicolor/256x256/apps" \
         "$APPDIR/usr/share/metainfo"

install -m 0755 target/release/kitsune-qt "$APPDIR/usr/bin/kitsune-qt"

# The icon the window/taskbar uses. The same PNG is embedded in the binary via
# the Qt resource system, but the desktop entry needs a file on disk.
install -m 0644 crates/kitsune-qt/assets/logo.png \
  "$APPDIR/usr/share/icons/hicolor/256x256/apps/com.kitsune.app.png"

install -m 0644 crates/kitsune-qt/packaging/linux/com.kitsune.app.qt.desktop \
  "$APPDIR/usr/share/applications/com.kitsune.app.desktop"

cat > "$APPDIR/usr/share/metainfo/com.kitsune.app.appdata.xml" <<XML
<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>com.kitsune.app</id>
  <name>Kitsune</name>
  <summary>Anime streaming from torrents</summary>
  <metadata_license>MIT</metadata_license>
  <project_license>MIT</project_license>
  <launchable type="desktop-id">com.kitsune.app.desktop</launchable>
  <releases>
    <release version="$VERSION" date="$(date -u +%Y-%m-%d)"/>
  </releases>
</component>
XML

# Tell linuxdeploy where the Qt kit is and that this is a QML app. Without the
# qml imports being scanned, the app starts to a blank window on a machine with
# no system Qt.
export QML_SOURCES_PATHS="$ROOT/crates/kitsune-qt/qml"
export EXTRA_QT_MODULES="svg"
export EXTRA_PLATFORM_PLUGINS="libqsvg.so"
export OUTPUT="$ROOT/$OUT/Kitsune-Qt-${VERSION}-x86_64.AppImage"

"$LINUXDEPLOY" --appdir "$APPDIR" \
  --plugin qt \
  --desktop-file "$APPDIR/usr/share/applications/com.kitsune.app.desktop" \
  --icon-file "$APPDIR/usr/share/icons/hicolor/256x256/apps/com.kitsune.app.png" \
  --output appimage

echo "wrote $OUTPUT"