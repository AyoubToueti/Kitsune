// Throwaway spike entry point. DELETE once Phase 1 lands for real.
//
// Goal: prove a Qt6 QML window opens on this NixOS box. The Slint frontend
// (see repo memory) hung forever with NO window because femtovg/glutin could
// not dlopen libEGL on NixOS. Qt may or may not have the same problem, which
// is exactly what this binary answers.

#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQuickWindow>
#include <QTimer>

#ifdef HAVE_LIBMPV
#include <mpv/client.h>
#include <mpv/render_gl.h>
#endif

#include <clocale>
#include <cstdio>

int main(int argc, char **argv) {
    // Qt sets LC_NUMERIC in the QGuiApplication ctor; libmpv requires "C".
    QGuiApplication app(argc, argv);

    std::printf("[spike] Qt %s\n", qVersion());

#ifdef HAVE_LIBMPV
    std::printf("[spike] libmpv client API version: %lu\n",
                mpv_client_api_version());
    // Proving the render symbols exist is enough; we do not create a context.
    std::printf("[spike] mpv_render_context_create symbol available: %s\n",
                (&mpv_render_context_create ? "yes" : "no"));
#else
    std::printf("[spike] built WITHOUT libmpv\n");
#endif

    QQmlApplicationEngine engine;
    QObject::connect(
        &engine, &QQmlApplicationEngine::objectCreationFailed, &app,
        []() { std::fprintf(stderr, "[spike] QML failed to load\n"); },
        Qt::QueuedConnection);
    engine.load(QUrl::fromLocalFile(QStringLiteral(SPIKE_QML_PATH)));

    if (engine.rootObjects().isEmpty()) {
        std::fprintf(stderr, "[spike] FAIL: no root object\n");
        return 1;
    }

    auto *window = qobject_cast<QQuickWindow *>(engine.rootObjects().first());
    if (window) {
        std::printf("[spike] window visible=%d size=%dx%d\n",
                    window->isVisible(), window->width(), window->height());
    }

    // This box has no reliable screenshot capture, so prove the window was
    // actually mapped by quitting after a beat with a success code. A hang
    // here (no window) is the exact failure mode the Slint spike hit.
    QTimer::singleShot(1500, &app, []() {
        std::printf("[spike] OK: ran for 1.5s without hanging\n");
        QGuiApplication::quit();
    });

    return app.exec();
}