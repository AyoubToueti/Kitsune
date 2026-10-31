#include "platcompat.h"

#include <clocale>

#include <QByteArray>
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