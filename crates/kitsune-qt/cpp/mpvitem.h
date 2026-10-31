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

#include <chrono>

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
  /// Volume, 0..200 (above 100 boosts). The ceiling is mpv's `volume-max`,
  /// raised from its 130 default in mpvitem.cpp.
  Q_PROPERTY(double volume READ volume NOTIFY stateChanged)
  Q_PROPERTY(bool muted READ isMuted NOTIFY stateChanged)
  /// True while mpv is starved for data or has not produced a frame yet.
  Q_PROPERTY(bool buffering READ isBuffering NOTIFY stateChanged)
  /// `buffering`, but only after it has PERSISTED for [`kBufferingGraceMs`].
  ///
  /// The overlay binds to this, not to `buffering`. mpv routinely pauses for
  /// cache for a fraction of a second -- a demuxer hiccup, a seek index being
  /// fetched -- and `buffering` snaps true and false around it, which flashes
  /// the "Buffering..." panel over perfectly good video. Measured example:
  ///   buffering=1 (cache-wait=1 ... pos=15.22s)
  ///   buffering=0 (cache-wait=0 ... pos=15.47s)   <- 250ms
  /// while 7s of video was already buffered ahead.
  ///
  /// A stall shorter than the grace period is not worth telling the reader
  /// about; a real one is, and this still reports it.
  Q_PROPERTY(bool bufferingDelayed READ isBufferingDelayed NOTIFY stateChanged)
  /// Bytes buffered AHEAD of the playback position in mpv's demuxer.
  ///
  /// This is the number that explains a player stuck on a growing torrent: mpv
  /// keeps its read-ahead window filled from the bridge, and the window only
  /// fills as fast as librqbit can serve the NEXT range. Zero here while the
  /// download percentage climbs means mpv is blocked on a specific range, not
  /// merely waiting for more of the file.
  Q_PROPERTY(qint64 cacheForwardBytes READ cacheForwardBytes NOTIFY stateChanged)
  /// True once the demuxer has read to the end of the stream.
  Q_PROPERTY(bool cacheEof READ cacheEof NOTIFY stateChanged)
  /// mpv's own cache-fill percentage, 0..100; -1 when not reported.
  Q_PROPERTY(int cacheBufferingPercent READ cacheBufferingPercent NOTIFY stateChanged)
  /// Milliseconds from `loadfile` to the first frame actually being shown.
  ///
  /// Zero until the first frame arrives. Turns "it takes ages to start" into a
  /// number, so a fix can be proven rather than assumed.
  Q_PROPERTY(qint64 timeToFirstFrameMs READ timeToFirstFrameMs NOTIFY stateChanged)
  /// The time ranges mpv can seek to WITHOUT touching the network, as an
  /// array of `{ start, end }` objects in seconds.
  ///
  /// This is the honest "grey bar": mpv reports the demuxer cache's own
  /// seekable ranges, so a seek inside one is served from memory. Drawing the
  /// buffer from the torrent's byte progress instead is a lie -- bytes can be
  /// downloaded out of order (or be the wrong file entirely) and say nothing
  /// about what the demuxer holds.
  Q_PROPERTY(QVariantList seekableRanges READ seekableRanges NOTIFY seekableRangesChanged)

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

  /// When the current uninterrupted buffering stretch began, in
  /// `steady_clock` terms. Cleared when buffering ends.
  std::chrono::steady_clock::time_point buffering_since_{};
  /// `buffering_` held long enough to be worth showing.
  bool buffering_delayed_ = false;

  // --- streaming diagnostics ---------------------------------------------
  //
  // Read from mpv's `demuxer-cache-state` on the same poll as everything else.
  // Cheap to read and the only way to tell "the swarm is slow" apart from
  // "mpv is blocked on a range the torrent has not fetched". They are also
  // what a buffering overlay should show instead of a silent spinner.
  qint64 cache_forward_bytes_ = 0;
  bool cache_eof_ = false;
  int cache_buffering_percent_ = -1;

  /// Latest `seekable-ranges` from `demuxer-cache-state`, normalised to
  /// `[{ start, end }]`. Cached so QML can redraw the buffered bar without a
  /// property read per frame.
  QVariantList seekable_ranges_;

  /// Monotonic clock reading taken when the current `loadfile` was issued, so
  /// time-to-first-frame is measured from the request, not from construction.
  std::chrono::steady_clock::time_point load_started_{};
  /// Set once the first frame after a load has been shown; reset per load.
  qint64 time_to_first_frame_ms_ = 0;
  /// False until the first frame of the current load. Cleared in `command`
  /// when a `loadfile` is issued, set once `time-pos`/`vo-configured` shows
  /// real playback.
  bool first_frame_seen_ = false;

  /// `KITSUNE_MPV_LOG=1` was set at construction.
  ///
  /// Gates the high-volume diagnostics (per-event lines, every mpv log
  /// message, the 1 Hz sampler). Load FAILURES are always logged regardless,
  /// because a failed load is never noise.
  bool verbose_ = false;

  /// Which whole second of the current load has been sampled last.
  ///
  /// The sampler fires when this changes, which is exactly once per second,
  /// without a second timer: `pollState` already ticks at 200ms.
  qint64 last_sample_bucket_ = -1;

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
  QVariantList seekableRanges() const { return seekable_ranges_; }
  bool isBuffering() const { return buffering_; }
  bool isBufferingDelayed() const { return buffering_delayed_; }
  qint64 cacheForwardBytes() const { return cache_forward_bytes_; }
  bool cacheEof() const { return cache_eof_; }
  int cacheBufferingPercent() const { return cache_buffering_percent_; }
  qint64 timeToFirstFrameMs() const { return time_to_first_frame_ms_; }
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
  /// The demuxer's seekable ranges changed, so the buffered bar should redraw.
  void seekableRangesChanged();
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

  /// Milliseconds since the current `loadfile` was issued.
  ///
  /// Every diagnostic line is stamped with this so the log reads as a
  /// timeline from the user's click: a bare wall-clock timestamp makes it
  /// impossible to say whether a stall happened before or after the load.
  /// Returns -1 when no load has been requested yet.
  qint64 msSinceLoad() const;
};