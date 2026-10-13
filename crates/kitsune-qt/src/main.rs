//! Kitsune's Qt6/QML entry point.
//!
//! Loads `qml/main.qml` (embedded by `build.rs` under the
//! `qrc:/qt/qml/com/kitsune/app/` prefix) into a `QQmlApplicationEngine`.

mod backend;
mod screenshot;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QUrl};

fn main() {
    // Qt sets LC_NUMERIC; libmpv (added later) requires "C". Setting it here
    // keeps the same discipline the old Tauri app used.
    libc_setlocale();

    let mut app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();

    // Dev aid: KITSUNE_OPEN_ID=<id> makes the app push the detail page for that
    // AniList id on startup, so a screenshot can capture it without a click.
    let open_id: i32 = std::env::var("KITSUNE_OPEN_ID")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    // KITSUNE_OPEN_BROWSE=1 pushes the browse page on startup, for screenshots.
    let open_browse: bool = std::env::var("KITSUNE_OPEN_BROWSE").is_ok();

    // `initialProperties` seeds the root object's properties before the QML
    // runs, so `initialOpenId` is already set when Component.onCompleted fires.
    // Each cxx-qt-lib method consumes the `Pin`, so a fresh `as_mut()` is taken
    // for each call.
    {
        let mut props = cxx_qt_lib::QMap::<cxx_qt_lib::QMapPair_QString_QVariant>::default();
        props.insert(
            cxx_qt_lib::QString::from("initialOpenId"),
            cxx_qt_lib::QVariant::from(&open_id),
        );
        props.insert(
            cxx_qt_lib::QString::from("initialBrowse"),
            cxx_qt_lib::QVariant::from(&open_browse),
        );
        if let Some(engine) = engine.as_mut() {
            engine.set_initial_properties(&props);
        }
    }

    if let Some(engine) = engine.as_mut() {
        // On failure Qt logs "QQmlApplicationEngine failed to load component"
        // to stderr, which is the signal the smoke run greps for.
        engine.load(&QUrl::from("qrc:/qt/qml/com/kitsune/app/qml/main.qml"));
        eprintln!("kitsune-qt: load() returned");
    }

    // Dev/CI aid: with KITSUNE_SCREENSHOT=<path> set, grab the window after the
    // scene settles and quit. External capture tools are unreliable here (see
    // repo memory), so the app photographs itself.
    if let Ok(path) = std::env::var("KITSUNE_SCREENSHOT") {
        let delay = std::env::var("KITSUNE_SCREENSHOT_DELAY_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2500);
        screenshot::save_after(&path, delay);
    }

    if let Some(app) = app.as_mut() {
        app.exec();
    }
}

/// Reset the numeric locale to "C" for C libraries (libmpv) that parse numbers
/// with `strtod` and assume a dot decimal separator.
fn libc_setlocale() {
    // `setlocale` lives in libc; we only need the one call, so declare it here
    // rather than pull in a whole crate for it.
    extern "C" {
        fn setlocale(category: i32, locale: *const i8) -> *mut i8;
    }
    // LC_NUMERIC == 1 on glibc; "C" is the portable locale name.
    const LC_NUMERIC: i32 = 1;
    let c = b"C\0";
    unsafe {
        setlocale(LC_NUMERIC, c.as_ptr() as *const i8);
    }
}