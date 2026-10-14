//! Generates and compiles the C++ for every `#[cxx_qt::bridge]` in this crate.
//!
//! `new_qml_module` also embeds the QML files into a Qt resource under
//! `qrc:/qt/qml/<uri-as-dirs>/...`, which is how `main.rs` loads them.

use cxx_qt_build::{CppFile, CxxQtBuilder, QmlFile, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(
        QmlModule::new("com.kitsune.app")
            .qml_file("qml/main.qml")
            .qml_file("qml/HomePage.qml")
            .qml_file("qml/DetailPage.qml")
            .qml_file("qml/BrowsePage.qml")
            .qml_file("qml/SchedulePage.qml")
            .qml_file("qml/TopPage.qml")
            .qml_file("qml/MyListPage.qml")
            .qml_file("qml/SettingsPopup.qml")
            .qml_file("qml/EpisodeWatchModal.qml")
            .qml_file("qml/PlayerControls.qml")
            .qml_file("qml/PlayerSettings.qml")
            .qml_file("qml/WatchPage.qml")
            .qml_file("qml/NavBar.qml")
            .qml_file("qml/ThemeSelect.qml")
            .qml_file("qml/Chip.qml")
            .qml_file("qml/NavButton.qml")
            .qml_file("qml/PosterCard.qml")
            .qml_file("qml/HeroCarousel.qml")
            .qml_file("qml/TrendingCard.qml")
            .qml_file("qml/AnimeListRow.qml")
            .qml_file("qml/ListBlock.qml")
            .qml_file("qml/ScheduleWidget.qml")
            .qml_file("qml/TopList.qml")
            // Theme is a `pragma Singleton`; the qmldir must mark it as one or
            // QML refuses to import it.
            .qml_file(QmlFile::from("qml/Theme.qml").singleton(true)),
    )
    .files(["src/backend.rs", "src/screenshot.rs", "src/mpv.rs"])
    // Link QtQuick so its headers (QQuickWindow, QQuickFramebufferObject) are on
    // the include path, and QtOpenGL for QOpenGLFramebufferObject (the mpv
    // render target).
    .qt_module("Quick")
    .qt_module("OpenGL")
    // The screenshot helper is plain C++ driven from Rust.
    .cpp_file("cpp/screenshot.cpp")
    // `MpvObject` has a Q_OBJECT (virtuals + signals). CppFile's defaults run
    // moc over the HEADER and compile the .cpp, so both files are listed.
    .cpp_file(CppFile::from("cpp/mpvitem.h"))
    .cpp_file(CppFile::from("cpp/mpvitem.cpp"))
    .include_dir("cpp")
    .build();

    // Link libmpv, which the mpv item calls directly. `pkg-config` supplies the
    // correct -L/-l flags on NixOS; a bare `-lmpv` is the fallback.
    match pkg_config::Config::new().probe("mpv") {
        Ok(lib) => {
            for path in &lib.link_paths {
                println!("cargo:rustc-link-search=native={}", path.display());
            }
            for name in &lib.libs {
                println!("cargo:rustc-link-lib=dylib={}", name);
            }
        }
        Err(_) => println!("cargo:rustc-link-lib=dylib=mpv"),
    }
}