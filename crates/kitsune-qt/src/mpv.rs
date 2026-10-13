//! Rust binding for the C++ mpv QML item.
//!
//! `MpvObject` is a plain C++ `QQuickFramebufferObject` (CXX-Qt cannot subclass
//! it), registered as a QML type. This bridge only exposes the registration
//! call so `main.rs` can run it before loading QML.

#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("mpvitem.h");

        /// Register `MpvObject` with the QML engine. Must run before load().
        #[rust_name = "register_mpv"]
        fn kitsuneRegisterMpv();
    }
}

/// Register the embedded mpv QML type with the Qt QML engine.
pub fn register() {
    ffi::register_mpv();
}