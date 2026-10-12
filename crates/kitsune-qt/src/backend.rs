//! The Rust <-> QML bridge.
//!
//! `Backend` is the single QObject QML talks to. Every command the old Svelte
//! frontend made with `invoke(...)` becomes either a `#[qinvokable]` method or
//! a signal on this object.
//!
//! This first version is deliberately minimal: it proves the bridge compiles,
//! links `kitsune-core`, and is reachable from QML. Real commands are added on
//! top of it one at a time.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        type Backend = super::BackendRust;

        /// A trivial invokable, to prove the QML -> Rust call path works.
        #[qinvokable]
        fn greet(&self) -> QString;

        /// Returns a string derived from a real `kitsune-core` type, so a
        /// broken link between this crate and the core would fail to build.
        #[qinvokable]
        #[cxx_name = "coreSummary"]
        fn core_summary(&self) -> QString;
    }
}

use cxx_qt_lib::QString;

/// The Rust half of `Backend`. No state yet -- QML-facing state is added with
/// the first real command.
#[derive(Default)]
pub struct BackendRust;

impl qobject::Backend {
    fn greet(&self) -> QString {
        QString::from("Kitsune Qt6 backend is live")
    }

    fn core_summary(&self) -> QString {
        // Referencing a core type at runtime forces the linker to prove the
        // dependency is real; a stub or a missing crate fails here, not later.
        let provider = std::any::type_name::<kitsune_core::providers::AniListProvider>();
        QString::from(format!("linked: {provider}"))
    }
}