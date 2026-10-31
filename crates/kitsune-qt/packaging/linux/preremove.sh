#!/bin/sh
# Reverse of postinstall: drop the cached desktop entry so a `kitsune://` link
# stops pointing at a binary that is about to disappear.
set -e

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database -q /usr/share/applications || true
fi

exit 0