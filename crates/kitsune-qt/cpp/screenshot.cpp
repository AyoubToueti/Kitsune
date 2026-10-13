#include "screenshot.h"

#include <QCoreApplication>
#include <QGuiApplication>
#include <QImage>
#include <QQuickWindow>
#include <QString>
#include <QTimer>
#include <QWindow>

#include <cstdio>

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
      std::fprintf(stderr, "kitsune-qt: no QQuickWindow to screenshot\n");
    } else {
      const QImage image = window->grabWindow();
      if (image.isNull()) {
        std::fprintf(stderr, "kitsune-qt: grabWindow returned a null image\n");
      } else if (!image.save(target)) {
        std::fprintf(stderr,
                     "kitsune-qt: could not write %s\n",
                     target.toUtf8().constData());
      } else {
        std::fprintf(stderr,
                     "kitsune-qt: wrote %s (%dx%d)\n",
                     target.toUtf8().constData(),
                     image.width(),
                     image.height());
      }
    }

    QCoreApplication::quit();
  });
}