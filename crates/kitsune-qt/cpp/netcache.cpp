#include "netcache.h"

#include <QDir>
#include <QNetworkAccessManager>
#include <QNetworkDiskCache>
#include <QQmlApplicationEngine>
#include <QQmlEngine>
#include <QQmlNetworkAccessManagerFactory>
#include <QStandardPaths>
#include <QString>

namespace {

// Every manager the engine hands out carries the same disk cache. The cache
// object is parented to the manager, so Qt owns it.
class DiskCachingFactory final : public QQmlNetworkAccessManagerFactory {
public:
    explicit DiskCachingFactory(QString cacheDir, int maxBytes)
        : m_cacheDir(std::move(cacheDir)), m_maxBytes(maxBytes) {}

    QNetworkAccessManager* create(QObject* parent) override
    {
        auto* manager = new QNetworkAccessManager(parent);

        auto* cache = new QNetworkDiskCache(manager);
        cache->setCacheDirectory(m_cacheDir);
        cache->setMaximumCacheSize(m_maxBytes);
        manager->setCache(cache);

        return manager;
    }

private:
    QString m_cacheDir;
    int m_maxBytes;
};

} // namespace

void kitsuneInstallNetworkDiskCache(QQmlApplicationEngine& engine, int maxBytes)
{
    // `CacheLocation` is the per-app cache directory Qt already derives from
    // the org/app name, so the app does not need its own XDG lookup. Images get
    // a subdirectory of their own so a future cache purge can target them.
    const QString base = QStandardPaths::writableLocation(
        QStandardPaths::CacheLocation);
    if (base.isEmpty()) {
        // No writable cache location: run without a disk cache rather than
        // guessing a path. Images still work, they just re-download.
        return;
    }

    QDir dir(base);
    dir.mkpath("images");

    // QQmlEngine takes ownership of the factory and deletes it in its
    // destructor, so this is not a leak.
    engine.setNetworkAccessManagerFactory(
        new DiskCachingFactory(dir.filePath("images"), maxBytes));
}