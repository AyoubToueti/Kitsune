#include "platcompat.h"

#include <clocale>

#include <QByteArray>
#include <QCoreApplication>
#include <QProcess>
#include <QString>

void
kitsuneSetEnv(rust::Str name, rust::Str value)
{
  // `qputenv` writes through the Qt/C runtime in a platform-correct way, which
  // is exactly what `setenv` was doing on Linux (and failing to do on Windows).
  const QByteArray n(reinterpret_cast<const char*>(name.data()),
                     static_cast<int>(name.size()));
  const QByteArray v(reinterpret_cast<const char*>(value.data()),
                     static_cast<int>(value.size()));
  qputenv(n.constData(), v);
}

void
kitsuneResetNumericLocale()
{
  // `LC_NUMERIC` resolves to the right constant for whichever CRT is in use,
  // so no hand-copied integer (the old code hardcoded glibc's 1) is needed.
  std::setlocale(LC_NUMERIC, "C");
}

bool
kitsuneRestartApp()
{
  const QString exe = QCoreApplication::applicationFilePath();
  if (exe.isEmpty())
    return false;

  // Arguments are forwarded so a relaunch keeps any switches the reader
  // started the app with (the auth deep link is handled before Qt exists, so
  // it is not among them).
  const QStringList args = QCoreApplication::arguments().mid(1);

  // Detached so the child does not die with this process: the whole point is
  // that THIS instance exits while the new one keeps running. The parent's
  // pipes are not inherited, which is what "detached" buys here.
  const bool started = QProcess::startDetached(exe, args);
  if (!started)
    return false;

  // Only quit once the replacement is confirmed running. Doing it the other
  // way round turns a failed spawn into a closed application.
  QCoreApplication::quit();
  return true;
}