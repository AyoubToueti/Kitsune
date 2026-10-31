# kitsune-qt

The Qt6/QML frontend for Kitsune: a CXX-Qt binary that drives the shared
`kitsune-core` backend and plays video through embedded libmpv.

## Prerequisites

| | Linux | Windows |
| --- | --- | --- |
| Toolchain | Rust stable | Rust stable (MSVC) |
| Qt | 6.8+ (`qtbase`, `qtdeclarative`, `qtsvg`) | Qt 6.8+ for MSVC 2022 (`qtsvg`) |
| libmpv | `libmpv-dev` / `mpv-unwrapped` | a `libmpv` dev tree, see below |
| Other | `pkg-config`, `cmake`, `ninja`, `libclang` | `libclang` (`LIBCLANG_PATH`), `dumpbin`/`lib` |

Qt is discovered by `qt-build-utils` (inside `cxx-qt-build`) by asking `qmake`.
Whatever installs a working `qmake` on `PATH` is enough — there is no
`CMakeLists.txt` and `CMAKE_PREFIX_PATH` is not used.

## Linux

With Nix, `nix develop` provides everything, including the merged Qt prefix the
build needs:

    nix develop
    cargo run -p kitsune-qt

The merged prefix exists because NixOS splits Qt across store paths, so
`qmake -query QT_INSTALL_LIBS` reports only `qtbase`'s lib directory and
`libQt6Qml.prl` is never found. The shell hook symlinks every module into one
tree and wraps `qmake` to point at it. See `flake.nix`.

Without Nix, install the packages listed in the table above (on Debian/Ubuntu:
`qt6-base-dev qt6-declarative-dev qt6-svg-dev libmpv-dev libclang-dev`) and
build normally.

## Windows

Three things need setting up by hand.

### 1. Qt for MSVC

Install a Qt 6.8+ **MSVC 2022 64-bit** kit (the online installer, or
`aqtinstall`), including the **Qt SVG** additional library. Put the kit's `bin`
directory on `PATH` so `qmake` resolves.

### 2. libmpv

mpv has no vcpkg port, so use a prebuilt [shinchiro
build](https://github.com/shinchiro/mpv-winbuild-cmake/releases):
`mpv-dev-x86_64-*.7z` (not the `-v3` variant, which requires AVX2).

That archive ships a **MinGW** import library (`libmpv.dll.a`), which MSVC
cannot link. Generate an MSVC one from the DLL's exports:

    :: in a "x64 Native Tools Command Prompt for VS 2022"
    dumpbin /nologo /exports libmpv-2.dll > exports.txt
    :: turn the export table into a .def (one symbol per line under EXPORTS)
    lib /nologo /def:mpv.def /machine:x64 /out:mpv.lib

Lay the result out as:

    libmpv\
      include\mpv\client.h, render_gl.h, ...
      lib\mpv.lib
      libmpv-2.dll

Then point the build at it:

    set MPV_DIR=C:\path\to\libmpv
    cargo run -p kitsune-qt

`MPV_DIR` is required on Windows — there is no pkg-config fallback, and the
build fails with an explicit message if it is missing or malformed.

### 3. At runtime

`libmpv-2.dll` must sit beside `kitsune.exe`, together with the ffmpeg DLLs
from the matching `mpv-x86_64-*.7z` runtime archive. Running from
`target\debug` without them fails to start.

## Packaging

The release workflow builds and packages both platforms; the definitions live
in `packaging/`.

- **Windows** — `packaging/windows/kitsune-qt.nsi`. `windeployqt` produces the
  payload (Qt DLLs, `platforms/`, `imageformats/`, `qml/`), and the NSIS script
  installs it per user, registers the `kitsune://` scheme under
  `HKCU\Software\Classes\kitsune`, and writes an uninstaller.
- **Linux** — `packaging/linux/build-appimage.sh` assembles an AppDir and runs
  `linuxdeploy --plugin qt`; `packaging/linux/nfpm.yaml` turns the same AppDir
  into `.deb` and `.rpm`. The launcher script exports `QT_PLUGIN_PATH` and
  `QML2_IMPORT_PATH` because the Qt tree is installed under
  `/usr/lib/kitsune-qt`, not beside the binary.

## Notes

- Playback is libmpv's **OpenGL** render API inside a
  `QQuickFramebufferObject`, so `QSG_RHI_BACKEND=opengl` is forced at startup.
  On Windows `hwdec=auto-safe` selects D3D11VA; if video is black while audio
  plays, that interop needs `hwdec=auto-safe-copy` instead.
- Single instance and `kitsune://` deep-link forwarding use D-Bus on Linux and
  a loopback TCP listener on Windows (see `src/single_instance.rs`). Both
  degrade quietly: if the transport is unavailable the app still starts, it
  just does not forward.