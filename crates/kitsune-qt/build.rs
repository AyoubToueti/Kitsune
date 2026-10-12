//! Generates and compiles the C++ for every `#[cxx_qt::bridge]` in this crate.
//!
//! `new_qml_module` also embeds the QML files into a Qt resource under
//! `qrc:/qt/qml/<uri-as-dirs>/...`, which is how `main.rs` loads them.

use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(
        QmlModule::new("com.kitsune.app").qml_file("qml/main.qml"),
    )
    .files(["src/backend.rs"])
    .build();
}