// A tiny helper that saves the app's window to a PNG and quits.
//
// Qt's own `QQuickWindow::grabWindow()` is the reliable capture path: external
// tools (spectacle, grim) are flaky here and grab the wrong window. Only used
// when KITSUNE_SCREENSHOT is set, so it costs nothing in normal runs.
//
// Takes `rust::Str`, not QString: this is a plain CXX bridge (not CXX-Qt), so
// cxx-qt-lib's QString type is not available here. The QString is built inside
// the .cpp.
#pragma once

#include "rust/cxx.h"

// Wait `delayMs` for the scene to settle, grab the first top-level window, save
// it to `path`, then quit the application. A failed grab is logged and still
// quits, so a CI run never hangs.
void kitsuneSaveScreenshot(rust::Str path, int delayMs);

// Install a Qt message handler that drops a handful of known-harmless startup
// warnings, so the console is not buried under them on every run. Everything
// else is forwarded to Qt's own formatting. Call once, before QML loads.
void kitsuneInstallLogFilter();

// Declare which desktop entry this window belongs to, e.g. "com.kitsune.app".
//
// On Wayland the compositor derives the window's `app_id` from this and matches
// it against an installed `.desktop` file's `Icon=`, which is how the taskbar
// picks the window's icon. Qt defaults it to the executable name
// ("kitsune-qt"), which matches nothing and leaves the generic icon.
void kitsuneSetDesktopFileName(rust::Str name);

// Set the application-wide window icon from an image inside the Qt resource
// system, e.g. ":/qt/qml/com/kitsune/app/assets/logo.png".
//
// This is what populates the compositor's `xdg_toplevel_icon` for every
// window. Without it Qt sends `set_icon(nil)` on Wayland, which tells the
// compositor "no icon" and overrides the app_id -> `.desktop` lookup, leaving
// the generic placeholder. Also sets the X11/XWayland window icon.
void kitsuneSetWindowIcon(rust::Str resourcePath);