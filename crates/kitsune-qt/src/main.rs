//! Kitsune's Qt6/QML entry point.
//!
//! Loads `qml/main.qml` (embedded by `build.rs` under the
//! `qrc:/qt/qml/com/kitsune/app/` prefix) into a `QQmlApplicationEngine`.

mod backend;
mod mpv;
mod screenshot;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QUrl};

fn main() {
    // Do not let Qt register with the XDG desktop portal.
    //
    // Setting a desktop file name (below, for the Wayland taskbar icon) makes
    // Qt try to register the app with `org.freedesktop.host.portal.Registry`.
    // That lookup fails here with "App info not found for 'com.kitsune.app'",
    // because the entry it finds has an `Exec=` pointing at a build artifact
    // rather than this binary -- and the failure is only a noisy warning. This
    // switch turns the portal integration off entirely, so the message cannot
    // appear. It costs nothing: the app uses no portal features (no file
    // chooser, screenshot, or notification portal), and the taskbar icon is
    // supplied directly via `setWindowIcon`.
    //
    // MUST be set before `QGuiApplication` is constructed -- Qt reads it while
    // building the application object -- so it goes first, ahead of everything.
    std::env::set_var("QT_NO_XDG_DESKTOP_PORTAL", "1");

    // Force Qt's scene graph to OpenGL BEFORE the app is built.
    //
    // The embedded mpv surface is a `QQuickFramebufferObject` that renders
    // through libmpv's *OpenGL* render API. Qt 6 may otherwise pick the Vulkan
    // RHI (it does by default on some Wayland sessions), and a
    // QQuickFramebufferObject simply does not render there -- the video stays
    // black while mpv's audio, which runs on its own thread, keeps playing.
    // Pinning OpenGL removes that intermittent black-video-with-sound.
    force_opengl_rhi();

    let mut app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();

    // Tell the desktop environment which entry this window belongs to. On
    // Wayland the window's app_id comes from this, and the compositor matches
    // it against an installed `.desktop` file to choose the taskbar icon.
    // Without it the executable name ("kitsune-qt") is used, which matches no
    // entry and shows a generic icon. The Tauri build shipped
    // `com.kitsune.app.desktop` plus hicolor icons, so reuse that identity.
    screenshot::set_desktop_file_name("com.kitsune.app");

    // Give the window a real icon. On Wayland Qt otherwise sends
    // `xdg_toplevel_icon.set_icon(nil)`, which tells the compositor "no icon"
    // and overrides the app_id -> `.desktop` match above, so the taskbar falls
    // back to the generic placeholder. The asset is embedded by build.rs.
    screenshot::set_window_icon(":/qt/qml/com/kitsune/app/assets/logo.png");

    // Reset LC_NUMERIC to "C" AFTER constructing the app: Qt's QGuiApplication
    // constructor sets the locale from the environment, which overrides an
    // earlier setlocale. libmpv parses numbers with strtod and requires a dot
    // decimal separator, so a non-C locale makes it abort with
    // "Non-C locale detected".
    libc_setlocale();

    // Drop known-harmless Qt startup warnings before anything can emit them.
    screenshot::install_log_filter();

    // Register the embedded mpv QML item before any QML loads.
    mpv::register();

    // Dev aid: KITSUNE_OPEN_ID=<id> makes the app push the detail page for that
    // AniList id on startup, so a screenshot can capture it without a click.
    let open_id: i32 = std::env::var("KITSUNE_OPEN_ID")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    // KITSUNE_OPEN_BROWSE=1 pushes the browse page on startup, for screenshots.
    let open_browse: bool = std::env::var("KITSUNE_OPEN_BROWSE").is_ok();
    // KITSUNE_OPEN_SCHEDULE=1 pushes the schedule page on startup.
    let open_schedule: bool = std::env::var("KITSUNE_OPEN_SCHEDULE").is_ok();
    // KITSUNE_OPEN_TOP=1 pushes the top-rated page on startup.
    let open_top: bool = std::env::var("KITSUNE_OPEN_TOP").is_ok();
    // KITSUNE_OPEN_MYLIST=1 pushes the reader's list page on startup.
    let open_mylist: bool = std::env::var("KITSUNE_OPEN_MYLIST").is_ok();
    // KITSUNE_OPEN_SETTINGS=1 pushes the settings page on startup.
    let open_settings: bool = std::env::var("KITSUNE_OPEN_SETTINGS").is_ok();
    // KITSUNE_OPEN_WATCH=<url> pushes the watch page and plays the URL.
    let watch_url: String = std::env::var("KITSUNE_OPEN_WATCH").unwrap_or_default();
    // KITSUNE_OPEN_EPISODE="<animeId>:<episode>" opens the watch modal for that
    // episode, for screenshots and the smoke test. `absoluteEpisode` defaults
    // to `episode`.
    let (open_episode_anime, open_episode_number): (i32, i32) =
        std::env::var("KITSUNE_OPEN_EPISODE")
            .ok()
            .and_then(|v| {
                let (a, e) = v.split_once(':')?;
                Some((a.parse().ok()?, e.parse().ok()?))
            })
            .unwrap_or((0, 0));
    // KITSUNE_AUTOPLAY=1 makes the watch modal pick the first release and its
    // first file on its own, so the buffering/player stage can be screenshotted
    // without a click. Development only.
    let auto_play: bool = std::env::var("KITSUNE_AUTOPLAY").is_ok();

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
        props.insert(
            cxx_qt_lib::QString::from("initialSchedule"),
            cxx_qt_lib::QVariant::from(&open_schedule),
        );
        props.insert(
            cxx_qt_lib::QString::from("initialTop"),
            cxx_qt_lib::QVariant::from(&open_top),
        );
        props.insert(
            cxx_qt_lib::QString::from("initialMyList"),
            cxx_qt_lib::QVariant::from(&open_mylist),
        );
        props.insert(
            cxx_qt_lib::QString::from("initialSettings"),
            cxx_qt_lib::QVariant::from(&open_settings),
        );
        let watch_qstring = cxx_qt_lib::QString::from(&watch_url);
        props.insert(
            cxx_qt_lib::QString::from("initialWatchUrl"),
            cxx_qt_lib::QVariant::from(&watch_qstring),
        );
        props.insert(
            cxx_qt_lib::QString::from("initialEpisodeAnime"),
            cxx_qt_lib::QVariant::from(&open_episode_anime),
        );
        props.insert(
            cxx_qt_lib::QString::from("initialEpisodeNumber"),
            cxx_qt_lib::QVariant::from(&open_episode_number),
        );
        props.insert(
            cxx_qt_lib::QString::from("autoPlay"),
            cxx_qt_lib::QVariant::from(&auto_play),
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

/// Force the Qt Quick scene graph onto the OpenGL RHI, which the mpv
/// `QQuickFramebufferObject` requires. Honours an explicit user override so a
/// developer can still experiment with another backend.
fn force_opengl_rhi() {
    // `setenv` lives in libc; declared here rather than pulling in a crate.
    extern "C" {
        fn setenv(name: *const i8, value: *const i8, overwrite: i32) -> i32;
    }
    // Only set it when the caller has not chosen a backend themselves.
    if std::env::var_os("QSG_RHI_BACKEND").is_none() {
        let name = b"QSG_RHI_BACKEND\0";
        let value = b"opengl\0";
        unsafe {
            setenv(name.as_ptr() as *const i8, value.as_ptr() as *const i8, 1);
        }
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