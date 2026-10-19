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
            .qml_file("qml/PlayerIcon.qml")
            .qml_file("qml/GlassPanel.qml")
            .qml_file("qml/WatchPage.qml")
            .qml_file("qml/NavBar.qml")
            .qml_file("qml/ThemeSelect.qml")
            .qml_file("qml/ThemedSwitch.qml")
            .qml_file("qml/ThemedSlider.qml")
            .qml_file("qml/ThemedSpinBox.qml")
            .qml_file("qml/Chip.qml")
            .qml_file("qml/NavButton.qml")
            .qml_file("qml/PosterCard.qml")
            .qml_file("qml/QuickAddButton.qml")
            .qml_file("qml/RecommendationCard.qml")
            .qml_file("qml/RecommendationsPage.qml")
            .qml_file("qml/ContinueWatchingCard.qml")
            .qml_file("qml/ResumeDisc.qml")
            .qml_file("qml/HeroCarousel.qml")
            .qml_file("qml/TrendingCard.qml")
            .qml_file("qml/AnimeListRow.qml")
            .qml_file("qml/ListBlock.qml")
            .qml_file("qml/ScheduleWidget.qml")
            .qml_file("qml/TopList.qml")
            .qml_file("qml/Skeleton.qml")
            .qml_file("qml/HeroSkeleton.qml")
            .qml_file("qml/TrendingRailSkeleton.qml")
            .qml_file("qml/ListBlockSkeleton.qml")
            .qml_file("qml/ScheduleWidgetSkeleton.qml")
            .qml_file("qml/RankedListSkeleton.qml")
            .qml_file("qml/DetailSkeleton.qml")
            .qml_file("qml/PosterGridSkeleton.qml")
            .qml_file("qml/RetryOverlay.qml")
            .qml_file("qml/InlineRetry.qml")
            .qml_file("qml/LoadingMore.qml")
            .qml_file("qml/DragToStatus.qml")
            // These are `pragma Singleton`; the qmldir must mark them or QML
            // refuses to import them.
            .qml_file(QmlFile::from("qml/Theme.qml").singleton(true))
            .qml_file(QmlFile::from("qml/ListStatusStore.qml").singleton(true))
            .qml_file(QmlFile::from("qml/RetryState.qml").singleton(true))
            .qml_file(QmlFile::from("qml/DragState.qml").singleton(true))
            .qml_file(QmlFile::from("qml/TitlePreference.qml").singleton(true))
            .qml_file(QmlFile::from("qml/SpeedTestStore.qml").singleton(true)),
    )
    .files(["src/backend.rs", "src/screenshot.rs", "src/mpv.rs"])
    // The brand mark, embedded so the UI can use it without shipping a loose
    // file next to the binary. `qrc_resources` inherits the QML module's
    // prefix, so these land at
    // qrc:/qt/qml/com/kitsune/app/assets/logo.{png,svg}.
    //
    // Both are shipped: the SVG scales to any size without blurring, but it
    // needs the qtsvg image-format plugin at runtime (see flake.nix's
    // QT_PLUGIN_PATH), so the PNG stays as a fallback for the window icon.
    .qrc_resources(["assets/logo.png", "assets/logo.svg"])
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