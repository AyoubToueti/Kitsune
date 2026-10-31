#include "screenshot.h"

#include <QCoreApplication>
#include <QGuiApplication>
#include <QIcon>
#include <QImage>
#include <QQuickWindow>
#include <QString>
#include <QTimer>
#include <QWindow>
#include <QtGlobal>

#include <cstdio>

namespace {
/// The handler Qt was using before ours, or null for Qt's built-in default.
QtMessageHandler g_previousHandler = nullptr;

/// Drop a few fixed, known-harmless startup warnings; forward the rest.
///
/// The only one today is "qmlRegisterType requires absolute URLs", which Qt
/// emits once per compiled QML file from CXX-Qt's generated module
/// registration. The registration still succeeds -- it is console noise, not a
/// fault -- and it carries no category, so it cannot be turned off with
/// QT_LOGGING_RULES without silencing every warning. Filtering the exact line
/// keeps real warnings visible.
void
kitsuneLogFilter(QtMsgType type,
                 const QMessageLogContext& context,
                 const QString& message)
{
  if (message.contains(
        QLatin1String("qmlRegisterType requires absolute URLs"))) {
    return;
  }
  if (g_previousHandler != nullptr) {
    g_previousHandler(type, context, message);
    return;
  }
  // Qt's default formatting, reproduced so nothing is lost when no handler
  // was installed before this one.
  const char* prefix = "info";
  switch (type) {
    case QtDebugMsg: prefix = "debug"; break;
    case QtInfoMsg: prefix = "info"; break;
    case QtWarningMsg: prefix = "warning"; break;
    case QtCriticalMsg: prefix = "critical"; break;
    case QtFatalMsg: prefix = "fatal"; break;
  }
  std::fprintf(stderr, "%s: %s\n", prefix, message.toUtf8().constData());
}
} // namespace

void
kitsuneInstallLogFilter()
{
  g_previousHandler = qInstallMessageHandler(kitsuneLogFilter);
}

void
kitsuneSetDesktopFileName(rust::Str name)
{
  const QString id =
    QString::fromUtf8(name.data(), static_cast<int>(name.size()));
  // Static, so it can be set before any window exists. On Wayland Qt uses this
  // as the window's `app_id`; the compositor then finds the matching `.desktop`
  // entry (and thus its `Icon=`), so the taskbar shows the app's own icon
  // rather than the generic Wayland placeholder.
  QGuiApplication::setDesktopFileName(id);
}

void
kitsuneSetWindowIcon(rust::Str resourcePath)
{
  const QString path =
    QString::fromUtf8(resourcePath.data(),
                      static_cast<int>(resourcePath.size()));
  // Every new window inherits this. On Wayland Qt turns it into the
  // `xdg_toplevel_icon`; without it Qt sends `set_icon(nil)`, which overrides
  // the app_id -> `.desktop` match and leaves the generic placeholder.
  QGuiApplication::setWindowIcon(QIcon(path));
}

void
kitsuneSaveScreenshot(rust::Str path, int delayMs)
{
  const QString target =
    QString::fromUtf8(path.data(), static_cast<int>(path.size()));

  QTimer::singleShot(delayMs, qApp, [target]() {
    QQuickWindow* window = nullptr;
    // Find the first top-level window that is actually a QQuickWindow.
    for (QWindow* w : QGuiApplication::topLevelWindows()) {
      if (auto* quick = qobject_cast<QQuickWindow*>(w)) {
        window = quick;
        break;
      }
    }

    if (window == nullptr) {
      std::fprintf(stderr, "kitsune: no QQuickWindow to screenshot\n");
    } else {
      const QImage image = window->grabWindow();
      if (image.isNull()) {
        std::fprintf(stderr, "kitsune: grabWindow returned a null image\n");
      } else if (!image.save(target)) {
        std::fprintf(stderr,
                     "kitsune: could not write %s\n",
                     target.toUtf8().constData());
      } else {
        std::fprintf(stderr,
                     "kitsune: wrote %s (%dx%d)\n",
                     target.toUtf8().constData(),
                     image.width(),
                     image.height());
      }
    }

    QCoreApplication::quit();
  });
}