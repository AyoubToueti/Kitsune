//! Generates and compiles the C++ for every `#[cxx_qt::bridge]` in this crate.
//!
//! `new_qml_module` also embeds the QML files into a Qt resource under
//! `qrc:/qt/qml/<uri-as-dirs>/...`, which is how `main.rs` loads them.

use cxx_qt_build::{CxxQtBuilder, QmlFile, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(
        QmlModule::new("com.kitsune.app")
            .qml_file("qml/main.qml")
            .qml_file("qml/HomePage.qml")
            .qml_file("qml/DetailPage.qml")
            .qml_file("qml/BrowsePage.qml")
            .qml_file("qml/SchedulePage.qml")
            .qml_file("qml/NavButton.qml")
            .qml_file("qml/PosterCard.qml")
            // Theme is a `pragma Singleton`; the qmldir must mark it as one or
            // QML refuses to import it.
            .qml_file(QmlFile::from("qml/Theme.qml").singleton(true)),
    )
    .files(["src/backend.rs", "src/screenshot.rs"])
    // Link QtQuick so its headers (QQuickWindow) are on the include path for
    // the raw C++ screenshot helper below.
    .qt_module("Quick")
    // The screenshot helper is plain C++ driven from Rust, so it is added as a
    // raw source file rather than a bridge.
    .cpp_files(["cpp/screenshot.cpp"])
    .include_dir("cpp")
    .build();
}