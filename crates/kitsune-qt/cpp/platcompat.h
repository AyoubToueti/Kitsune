// Process-wide environment/locale tweaks that must be correct on Windows too.
//
// These used to be two `extern "C"` declarations inside main.rs. Both were
// glibc-shaped and broke the MSVC build:
//
//   * `setenv` does not exist in the Universal CRT (MSVC only exports
//     `_putenv_s`). Linking failed outright.
//   * `LC_NUMERIC` is 1 in glibc but 2 in the MSVC CRT, so `setlocale(1, "C")`
//     silently reset the wrong category on Windows.
//
// Doing it in C++ sidesteps both: `qputenv` and `std::setlocale` are the same
// call on every platform and Qt already pulls in the CRT.
#pragma once

#include "rust/cxx.h"

// Set `name` to `value` in the process environment, overwriting any existing
// value. Used for `QSG_RHI_BACKEND=opengl`, which must be visible before
// QGuiApplication is constructed.
void kitsuneSetEnv(rust::Str name, rust::Str value);

// Reset LC_NUMERIC to the "C" locale so libmpv's `strtod` sees a dot decimal
// separator. Must be called AFTER QGuiApplication is constructed: Qt's
// constructor applies the environment locale and would undo an earlier reset.
void kitsuneResetNumericLocale();

// Restart the process: start a detached copy of this executable, then quit the
// current instance. Returns false when the new process could not be started, in
// which case the caller should stay alive rather than leave the reader with no
// window. Never returns when the relaunch succeeded.
//
// Done in C++ because QProcess is the only portable way to spawn and detach:
// Rust's `Command` has no "detach from this process group" notion, and a child
// that dies with its parent would close the window the reader asked to reopen.
bool kitsuneRestartApp();