#!/bin/sh
# Launcher installed to /usr/bin/kitsune by the .deb/.rpm.
#
# The real binary lives in /usr/lib/kitsune because the Qt plugins and QML
# modules have to sit next to each other, and dumping a whole Qt tree into
# /usr/bin is not acceptable. Qt only looks for plugins relative to the
# executable and in the compiled-in prefix, so the two paths are exported here.
set -e

PREFIX=/usr/lib/kitsune

QT_PLUGIN_PATH="$PREFIX/plugins${QT_PLUGIN_PATH:+:$QT_PLUGIN_PATH}"
QML2_IMPORT_PATH="$PREFIX/qml${QML2_IMPORT_PATH:+:$QML2_IMPORT_PATH}"
export QT_PLUGIN_PATH QML2_IMPORT_PATH

exec "$PREFIX/kitsune" "$@"