// An embedded libmpv video surface for Qt Quick.
//
// Ported from mpv-examples/libmpv/qml (main.h / main.cpp). mpv renders into an
// FBO that Qt Quick composites like any other item, so QML overlays (controls,
// OSD) sit on top. libass draws ASS/SSA subtitles natively, which the webview
// could never do -- the reason playback moved out of the webview in the first
// place.
#pragma once

#include <QtQuick/QQuickFramebufferObject>
#include <QSocketNotifier>
#include <QTimer>
#include <QVariant>
#include <QVariantList>

#include <mpv/client.h>
#include <mpv/render_gl.h>

class MpvRenderer;

/// Register `MpvObject` with the QML engine under the `com.kitsune.app` module.
/// Must be called before the QML is loaded.
void kitsuneRegisterMpv();

class MpvObject : public QQuickFramebufferObject
{
  Q_OBJECT

  // --- polled playback state ---------------------------------------------
  //
  // mpv's properties are not Qt properties, so QML cannot bind to them. These
  // mirror the handful a control bar needs, refreshed by one timer. Everything
  // else is reached on demand through `getProperty`/`setProperty`.

  /// Playback position in seconds. Writable (seek) via `setProperty`.
  Q_PROPERTY(double position READ position NOTIFY stateChanged)
  /// Total duration in seconds; 0 until the file's header is parsed.
  Q_PROPERTY(double duration READ duration NOTIFY stateChanged)
  Q_PROPERTY(bool paused READ isPaused NOTIFY stateChanged)
  /// Playback rate, 1.0 = normal.
  Q_PROPERTY(double speed READ speed NOTIFY stateChanged)
  /// Volume, 0..130 (above 100 boosts).
  Q_PROPERTY(double volume READ volume NOTIFY stateChanged)
  Q_PROPERTY(bool muted READ isMuted NOTIFY stateChanged)
  /// True while mpv is starved for data or has not produced a frame yet.
  Q_PROPERTY(bool buffering READ isBuffering NOTIFY stateChanged)

  // --- event-driven state ------------------------------------------------
  //
  // `chapter-list` is not polled: the demuxer fills it a moment AFTER
  // `duration` becomes non-zero, so a duration-keyed refetch misses it. mpv
  // is asked to OBSERVE the property, and the wakeup pipe (or, on Windows, a
  // fallback timer) drains the change event the instant chapters arrive.
  Q_PROPERTY(QVariantList chapterList READ chapterList NOTIFY chapterListChanged)

  // --- render-context readiness -----------------------------------------
  //
  // The mpv GL render context is created lazily in the renderer's
  // `createFramebufferObject`, which runs on the render thread at the first
  // frame -- AFTER the QML item is created. Issuing `loadfile` before that
  // makes mpv's `libmpv` video output open with no render context and fail
  // permanently ("No render context set" -> "Video: no video"), while audio
  // (its own thread) keeps playing. QML must wait for this flag before
  // loading, or playback is black-with-sound.
  Q_PROPERTY(bool renderReady READ renderReady NOTIFY renderReadyChanged)

  mpv_handle* mpv;
  mpv_render_context* mpv_gl;

  /// Wakes `drainMpvEvents` when mpv has events waiting (pipe on Unix).
  QSocketNotifier* mpv_events_ = nullptr;
  /// The wakeup pipe fd, or -1 when mpv has none (Windows). `drainMpvEvents`
  /// reads from it to clear the readable state.
  int mpv_wakeup_fd_ = -1;
  /// Windows fallback: mpv has no wakeup pipe there, so poll events instead.
  QTimer* mpv_event_timer_ = nullptr;

  /// Latest `chapter-list` as an array of `{ title, time }` maps. Cached so a
  /// QML read is a field access; refreshed only on an mpv change event.
  QVariantList chapterList_;

  /// Set once the render context exists. Guards against a `loadfile` that
  /// would otherwise race the first frame.
  bool renderReady_ = false;

  /// A `loadfile` issued before the render context existed, replayed the
  /// moment `renderReady_` flips. Empty when nothing is pending. Deferring in
  /// the item (not in each caller) means every QML user is protected.
  QVariant pendingLoad_;

  // Cached mirror of the polled properties, so a `Q_PROPERTY` read is a field
  // access rather than an mpv call.
  double position_ = 0.0;
  double duration_ = 0.0;
  bool paused_ = false;
  double speed_ = 1.0;
  double volume_ = 100.0;
  bool muted_ = false;
  bool buffering_ = false;

  QTimer* poll_ = nullptr;

  friend class MpvRenderer;

public:
  static void on_update(void* ctx);

  explicit MpvObject(QQuickItem* parent = nullptr);
  ~MpvObject() override;
  Renderer* createRenderer() const override;

  double position() const { return position_; }
  double duration() const { return duration_; }
  bool isPaused() const { return paused_; }
  double speed() const { return speed_; }
  double volume() const { return volume_; }
  bool isMuted() const { return muted_; }
  bool isBuffering() const { return buffering_; }
  QVariantList chapterList() const { return chapterList_; }
  bool renderReady() const { return renderReady_; }

public slots:
  /// Run an mpv command, e.g. `command(["loadfile", url])`.
  void command(const QVariant& params);
  /// Set an mpv property. A bool/int is sent as INT64, a real as DOUBLE, a
  /// string as STRING -- so `aid`/`sid` (integer ids) work, not just doubles.
  void setProperty(const QString& name, const QVariant& value);
  /// Read an mpv property as a `QVariant`. Returns an invalid `QVariant` when
  /// mpv has no answer, which QML reads as `undefined`.
  QVariant getProperty(const QString& name) const;

signals:
  void onUpdate();
  /// Any polled property may have changed. One signal for all of them: a
  /// control bar re-reads the few it shows, and per-property signals would be
  /// six near-identical declarations.
  void stateChanged();
  /// The `chapter-list` observed from mpv changed (file loaded, or the
  /// demuxer finished parsing chapters). QML re-reads `chapterList`.
  /// The mpv GL render context now exists; `loadfile` is safe.
  void renderReadyChanged();
  void chapterListChanged();

private slots:
  void doUpdate();
  /// Flip `renderReady_` and emit, on the GUI thread. The renderer calls this
  /// via a queued invocation, since it runs on the render thread.
  void markRenderReady();
  void pollState();
  /// Drain every pending mpv event, emitting Qt signals for the ones QML
  /// cares about. Runs on the GUI thread only (the mpv event API is not
  /// thread-safe for concurrent `mpv_wait_event`).
  void drainMpvEvents();
};