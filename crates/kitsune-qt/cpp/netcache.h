// Install a disk-backed HTTP cache on the QML engine.
//
// Why: `goForward()` in main.qml pushes a NEW page instance, so every `Image`
// in it is constructed afresh and re-requests its source. Qt's in-memory
// QQuickPixmapCache covers some of that, but it evicts under pressure -- a long
// running show is 100 episode thumbnails plus related covers plus a hero banner
// -- so the network gets hit again on every forward.
//
// Nothing configured an HTTP cache before this: QML's default
// QNetworkAccessManager has no QNetworkDiskCache attached, so every image was a
// fresh request. Attaching one makes covers, banners and episode stills survive
// both page rebuilds and app restarts.
//
// Takes the engine by reference (cxx-qt-lib exposes no raw pointer) and a
// `rust::Str` directory, because this is a plain CXX bridge like screenshot.h,
// not a CXX-Qt one.
#pragma once

#include "rust/cxx.h"

class QQmlApplicationEngine;

// Attach a QNetworkDiskCache to every QNetworkAccessManager this engine creates,
// under `QStandardPaths::CacheLocation` + "/images". Must be called BEFORE
// `engine.load(...)`: the factory is only consulted when the engine builds its
// manager, which happens on the first network request.
//
// The directory is derived in C++ rather than passed from Rust so the app does
// not need a Rust-side XDG lookup: Qt already knows the right per-app cache
// path, and `QCoreApplication::setApplicationName` decides the last segment.
void kitsuneInstallNetworkDiskCache(QQmlApplicationEngine& engine, int maxBytes);