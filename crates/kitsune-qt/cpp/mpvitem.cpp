// Embedded libmpv video surface. Ported from mpv-examples/libmpv/qml, adapted
// to Qt 6 (which removed QQuickWindow::resetOpenGLState and
// setPersistentOpenGLContext), then extended with a small playback API so QML
// can build a VLC/PotPlayer-style control bar on top.
#include "mpvitem.h"

#include <clocale>
#include <stdexcept>

// The wakeup-pipe drain below reads a file descriptor. On POSIX that is
// `read` from <unistd.h>; on Windows mpv never creates a wakeup pipe (it
// returns a negative fd and the QTimer fallback is used instead), but the
// translation unit still has to compile, and MSVC has no <unistd.h>.
// Deliberately not `Q_OS_WIN`: that macro only exists once a Qt header has
// been included, which happens further down this file.
#ifdef _WIN32
#include <io.h>
#else
#include <unistd.h>
#endif

#include <QDebug>
#include <QGuiApplication>
#include <QList>
#include <QMetaType>
#include <QOpenGLContext>
#include <QOpenGLFramebufferObject>
#include <QQmlEngine>
#include <QQuickWindow>
#include <QSocketNotifier>
#include <QTimer>
#include <QVariant>
#include <QVariantList>
#include <QtGlobal>

namespace {
/// `reply_userdata` tagged onto the `chapter-list` observation, so
/// `drainMpvEvents` can tell which observed property an event belongs to.
constexpr uint64_t kChapterListObserveId = 1;

/// How long mpv must be starved before the buffering overlay is shown.
///
/// mpv pauses for cache for a few hundred milliseconds quite often -- a
/// demuxer hiccup, the seek index arriving late -- and showing a panel for
/// each one makes a healthy episode look broken. Anything shorter than this
/// is invisible to the viewer anyway. Long enough to swallow those, short
/// enough that a genuine stall still reports promptly.
constexpr qint64 kBufferingGraceMs = 700;
}

// The QML type name under the com.kitsune.app module.
void
kitsuneRegisterMpv()
{
  qmlRegisterType<MpvObject>("com.kitsune.app", 1, 0, "MpvObject");
}

namespace {

void
on_mpv_events(void* ctx)
{
  Q_UNUSED(ctx)
}

void
on_mpv_redraw(void* ctx)
{
  MpvObject::on_update(ctx);
}

void*
get_proc_address_mpv(void* ctx, const char* name)
{
  Q_UNUSED(ctx)
  QOpenGLContext* glctx = QOpenGLContext::currentContext();
  if (glctx == nullptr) {
    return nullptr;
  }
  return reinterpret_cast<void*>(glctx->getProcAddress(QByteArray(name)));
}

/// Read a double mpv property, leaving `out` untouched on failure.
bool
mpv_double(mpv_handle* mpv, const char* name, double* out)
{
  double value = 0.0;
  if (mpv_get_property(mpv, name, MPV_FORMAT_DOUBLE, &value) < 0) {
    return false;
  }
  *out = value;
  return true;
}

/// Read a boolean mpv property, returning `fallback` when mpv has no answer
/// (the property does not exist yet, or the core is between files).
bool
mpv_flag(mpv_handle* mpv, const char* name, bool fallback)
{
  int value = 0;
  if (mpv_get_property(mpv, name, MPV_FORMAT_FLAG, &value) < 0) {
    return fallback;
  }
  return value != 0;
}

/// Convert an `mpv_node` to a `QVariant` for the QML side.
///
/// Scalars (string/flag/int64/double) map to their Qt equivalents. Arrays
/// (`MPV_FORMAT_NODE_ARRAY`) map to `QVariantList`; maps
/// (`MPV_FORMAT_NODE_MAP`) map to `QVariantMap` keyed by the node's key
/// strings. Both recurse, so `chapter-list` (an array of maps with `title`
/// and `time` keys) arrives in QML as a JS array of plain objects.
///
/// The caller is responsible for calling `mpv_free_node_contents` on the
/// TOP-LEVEL node after this returns; nested nodes are owned by the parent
/// and freed with it.
QVariant
node_to_variant(const mpv_node& node);

QVariant
node_to_variant(const mpv_node& node)
{
  switch (node.format) {
    case MPV_FORMAT_STRING:
      return QString::fromUtf8(node.u.string);
    case MPV_FORMAT_FLAG:
      return static_cast<bool>(node.u.flag);
    case MPV_FORMAT_INT64:
      return static_cast<qlonglong>(node.u.int64);
    case MPV_FORMAT_DOUBLE:
      return node.u.double_;
    case MPV_FORMAT_NODE_ARRAY: {
      QVariantList list;
      if (node.u.list != nullptr) {
        list.reserve(static_cast<int>(node.u.list->num));
        for (int i = 0; i < node.u.list->num; ++i) {
          list.append(node_to_variant(node.u.list->values[i]));
        }
      }
      return list;
    }
    case MPV_FORMAT_NODE_MAP: {
      QVariantMap map;
      if (node.u.list != nullptr) {
        for (int i = 0; i < node.u.list->num; ++i) {
          const QString key = QString::fromUtf8(node.u.list->keys[i]);
          map.insert(key, node_to_variant(node.u.list->values[i]));
        }
      }
      return map;
    }
    default:
      return {};
  }
}

} // namespace

class MpvRenderer : public QQuickFramebufferObject::Renderer
{
  MpvObject* obj;

public:
  explicit MpvRenderer(MpvObject* new_obj)
    : obj{ new_obj }
  {
    // The wakeup callback is installed by MpvObject itself (it owns the event
    // loop); the renderer must not race it by overwriting the callback.
  }

  ~MpvRenderer() override = default;

  QOpenGLFramebufferObject* createFramebufferObject(const QSize& size) override
  {
    if (obj->mpv_gl == nullptr) {
      mpv_opengl_init_params gl_init_params[1] = { get_proc_address_mpv,
                                                   nullptr };
      mpv_render_param params[] = {
        { MPV_RENDER_PARAM_API_TYPE,
          const_cast<char*>(MPV_RENDER_API_TYPE_OPENGL) },
        { MPV_RENDER_PARAM_OPENGL_INIT_PARAMS, &gl_init_params },
        { MPV_RENDER_PARAM_INVALID, nullptr }
      };

      if (mpv_render_context_create(&obj->mpv_gl, obj->mpv, params) < 0) {
        throw std::runtime_error("failed to initialize mpv GL context");
      }
      mpv_render_context_set_update_callback(obj->mpv_gl, on_mpv_redraw, obj);
      // The context exists NOW, which is what makes `loadfile` safe. This runs
      // on the render thread; hop to the GUI thread to touch QML-visible state
      // (a bool + a signal) rather than racing the QML thread from here.
      QMetaObject::invokeMethod(obj, "markRenderReady", Qt::QueuedConnection);
    }
    return QQuickFramebufferObject::Renderer::createFramebufferObject(size);
  }

  void render() override
  {
    QOpenGLFramebufferObject* fbo = framebufferObject();
    // mpv's render API resizes its internal targets to match this FBO on every
    // frame (ra_gl_ctx_resize -> wrap_fbo). A zero-sized FBO -- which happens
    // while a preview/offscreen item is laid out at 0x0 or just toggled
    // invisible -- crashes that resize. Render nothing until it has real size.
    if (fbo == nullptr || fbo->width() <= 0 || fbo->height() <= 0) {
      return;
    }
    // The context is created lazily in createFramebufferObject; never render
    // through a null handle.
    if (obj->mpv_gl == nullptr) {
      return;
    }
    mpv_opengl_fbo mpfbo{ static_cast<int>(fbo->handle()),
                          fbo->width(),
                          fbo->height(),
                          0 };
    int flip_y{ 0 };

    mpv_render_param params[] = {
      { MPV_RENDER_PARAM_OPENGL_FBO, &mpfbo },
      { MPV_RENDER_PARAM_FLIP_Y, &flip_y },
      { MPV_RENDER_PARAM_INVALID, nullptr }
    };
    mpv_render_context_render(obj->mpv_gl, params);
  }
};

MpvObject::MpvObject(QQuickItem* parent)
  : QQuickFramebufferObject(parent)
  , mpv{ mpv_create() }
  , mpv_gl(nullptr)
{
  // One switch for every high-volume diagnostic (per-event lines, mpv's own
  // log messages, the 1 Hz sampler). Read once here so the event loop does not
  // hit the environment on every tick.
  verbose_ = qEnvironmentVariableIsSet("KITSUNE_MPV_LOG");
  if (mpv == nullptr) {
    throw std::runtime_error("could not create mpv context");
  }

  // libmpv renders into our FBO, not its own window.
  mpv_set_option_string(mpv, "vo", "libmpv");
  // Quiet by default; KITSUNE_MPV_LOG=1 turns on mpv's own logging.
  mpv_set_option_string(mpv, "terminal",
                        qEnvironmentVariableIsSet("KITSUNE_MPV_LOG") ? "yes"
                                                                     : "no");
  // Default to "fatal" only: `hwdec=auto` probes every backend, and a probe
  // for an absent one (e.g. CUDA outside a CUDA environment) logs an
  // error-level line that is harmless but noisy on stderr. A real playback
  // failure is reported through the buffering/health UI, so nothing is lost.
  // KITSUNE_MPV_LOG=1 restores full verbose logging for debugging.
  mpv_request_log_messages(mpv,
                           qEnvironmentVariableIsSet("KITSUNE_MPV_LOG") ? "v"
                                                                        : "fatal");

  if (mpv_initialize(mpv) < 0) {
    throw std::runtime_error("could not initialize mpv context");
  }

  // Hardware decoding when available; falls back to software otherwise.
  mpv_set_option_string(mpv, "hwdec", "auto-safe");
  // Do not probe GPU hwdec interop backends. `auto` probes every one,
  // including CUDA, and on a machine with no CUDA driver that probe writes
  // "Cannot load libcuda.so.1" straight to stderr with a null log context --
  // it bypasses mpv's log-level filtering entirely, so it cannot be silenced
  // any other way. `nvdec-copy` (a copy-mode decoder) needs no interop, so
  // hardware decoding still works without this probe.
  mpv_set_option_string(mpv, "gpu-hwdec-interop", "no");

  // --- streaming source -------------------------------------------------
  //
  // The source is a loopback HTTP server that blocks on missing pieces, so it
  // is always "still growing". These are the flags that make mpv behave well
  // against one:
  //
  // - `cache=yes` keeps a demuxer cache instead of reading the socket once.
  //   Without it a seek back re-requests bytes and a brief stall ends the
  //   stream, which is what made the player take so long to start and then
  //   fight every seek.
  // - `cache-pause-initial=no` is the START-LATENCY fix: mpv's default waits
  //   for a full cache-fill pass before showing the first frame, which is
  //   pointless when the file is arriving as it plays. With this off, playback
  //   begins as soon as the demuxer has the header.
  // - `cache-pause-wait=2` still pauses for a rebuffer mid-play, so a slow
  //   swarm stalls cleanly rather than playing choppy audio.
  // - The byte caps are BOUNDED on purpose. The webview player once ate all
  //   RAM with an unbounded cache; 64 MiB forward and 16 MiB back cover a
  //   normal seek window without growing with the file.
  // - `demuxer-readahead-secs=30` matches the forward cap so the two limits
  //   agree; whichever is hit first applies.
  // mpv clamps `volume` to `volume-max`, which defaults to 130. Raise it so
  // the UI's 0..200 range is actually reachable -- without this the slider
  // and the wheel both stop dead at 130 no matter what they clamp to.
  mpv_set_option_string(mpv, "volume-max", "200");

  mpv_set_option_string(mpv, "cache", "yes");
  mpv_set_option_string(mpv, "cache-pause", "yes");
  mpv_set_option_string(mpv, "cache-pause-initial", "no");
  mpv_set_option_string(mpv, "cache-pause-wait", "2");
  mpv_set_option_string(mpv, "demuxer-max-bytes", "64MiB");
  mpv_set_option_string(mpv, "demuxer-max-back-bytes", "16MiB");
  mpv_set_option_string(mpv, "demuxer-readahead-secs", "30");

  connect(
    this, &MpvObject::onUpdate, this, &MpvObject::doUpdate, Qt::QueuedConnection);

  // One timer for every polled property: a control bar needs them together, and
  // a shared signal is simpler than six near-identical ones. 200ms is smooth
  // for a position readout without being busy.
  poll_ = new QTimer(this);
  poll_->setInterval(200);
  connect(poll_, &QTimer::timeout, this, &MpvObject::pollState);
  poll_->start();

  // Ask mpv to OBSERVE chapter-list and deliver a change event the moment the
  // demuxer has it. This is the fix for chapters never appearing: they arrive
  // after `duration` is already non-zero, so any duration-keyed refetch misses
  // them. The event path below catches the real change.
  mpv_observe_property(
    mpv, kChapterListObserveId, "chapter-list", MPV_FORMAT_NODE);

  // Wakeup delivery. On Unix/macOS mpv writes to a wakeup pipe; a
  // QSocketNotifier on the read end is the canonical Qt integration. On
  // Windows there is no pipe (mpv_get_wakeup_pipe returns a negative fd), so
  // fall back to a short poll -- otherwise chapter events would silently never
  // fire there.
  const int wakeup_fd = mpv_get_wakeup_pipe(mpv);
  if (wakeup_fd >= 0) {
    mpv_wakeup_fd_ = wakeup_fd;
    mpv_events_ = new QSocketNotifier(wakeup_fd, QSocketNotifier::Read, this);
    connect(mpv_events_,
            &QSocketNotifier::activated,
            this,
            &MpvObject::drainMpvEvents);
  } else {
    mpv_event_timer_ = new QTimer(this);
    mpv_event_timer_->setInterval(50);
    connect(mpv_event_timer_,
            &QTimer::timeout,
            this,
            &MpvObject::drainMpvEvents);
    mpv_event_timer_->start();
  }

  // The callback only needs to exist; the pipe write it triggers is what wakes
  // the notifier. (mpv calls it from its own thread, so it must stay trivial.)
  mpv_set_wakeup_callback(mpv, on_mpv_events, this);
}

MpvObject::~MpvObject()
{
  // Stop event delivery BEFORE tearing mpv down, so no callback (socket
  // activation or timer tick) can run `drainMpvEvents` against a dead handle.
  if (mpv_events_ != nullptr) {
    mpv_events_->setEnabled(false);
  }
  if (mpv_event_timer_ != nullptr) {
    mpv_event_timer_->stop();
  }
  if (mpv_gl != nullptr) {
    mpv_render_context_free(mpv_gl);
  }
  mpv_terminate_destroy(mpv);
}

void
MpvObject::on_update(void* ctx)
{
  auto* self = static_cast<MpvObject*>(ctx);
  emit self->onUpdate();
}

void
MpvObject::doUpdate()
{
  update();
}

void
MpvObject::markRenderReady()
{
  if (renderReady_) {
    return;
  }
  renderReady_ = true;
  emit renderReadyChanged();
  // Replay a load that was queued before the context existed. Clearing the
  // member first means `command` sees the real (ready) state and runs it.
  if (pendingLoad_.isValid()) {
    const QVariant params = pendingLoad_;
    pendingLoad_ = QVariant();
    command(params);
  }
}

void
MpvObject::pollState()
{
  bool changed = false;

  double position = position_;
  if (mpv_double(mpv, "time-pos", &position) && position != position_) {
    position_ = position;
    changed = true;
  }

  double duration = duration_;
  if (mpv_double(mpv, "duration", &duration) && duration != duration_) {
    duration_ = duration;
    changed = true;
  }

  const bool paused = mpv_flag(mpv, "pause", paused_);
  if (paused != paused_) {
    paused_ = paused;
    changed = true;
  }

  double speed = speed_;
  if (mpv_double(mpv, "speed", &speed) && speed != speed_) {
    speed_ = speed;
    changed = true;
  }

  double volume = volume_;
  if (mpv_double(mpv, "volume", &volume) && volume != volume_) {
    volume_ = volume;
    changed = true;
  }

  const bool muted = mpv_flag(mpv, "mute", muted_);
  if (muted != muted_) {
    muted_ = muted;
    changed = true;
  }

  // `paused-for-cache` is the ONLY reliable "waiting for data" signal.
  //
  // `core-idle` is NOT a buffering signal, despite reading like one. mpv sets
  // it whenever the core is not actively rendering -- including "nothing is
  // playing yet" AND while a frame is on screen and the core is merely waiting
  // for the next vsync. Measured: `idle=1` in the sampler while `pos` advanced
  // from 25.27s to 30.28s, i.e. the video was playing normally. Combining it
  // into `buffering` therefore pinned the overlay over healthy playback, which
  // is the "it says buffering although there is enough to play" report.
  //
  // It IS still needed for the one case it legitimately covers: the window
  // between `loadfile` and the file being opened, when there is no duration yet
  // and no cache to report. `duration_ <= 0` distinguishes that from normal
  // playback, because the demuxer fills the duration as soon as it has the
  // header.
  const bool cache_wait = mpv_flag(mpv, "paused-for-cache", false);
  const bool idle = mpv_flag(mpv, "core-idle", false);
  const bool still_opening = idle && duration_ <= 0.0;
  // A user pause is never buffering, even while the cache is empty.
  const bool buffering = !paused_ && (cache_wait || still_opening);
  if (buffering != buffering_) {
    // The overlay's on/off is logged because it is the thing the reader
    // actually complains about: "it says buffering although it is playing".
    // A transition here that does not line up with `pos` moving is the bug.
    if (verbose_) {
      qDebug("[mpv] buffering=%d (cache-wait=%d idle=%d dur=%.2fs pos=%.2fs)",
             buffering ? 1 : 0,
             cache_wait ? 1 : 0,
             idle ? 1 : 0,
             duration_,
             position_);
    }
    buffering_ = buffering;
    buffering_since_ =
      buffering ? std::chrono::steady_clock::now()
                : std::chrono::steady_clock::time_point{};
    if (!buffering && buffering_delayed_) {
      buffering_delayed_ = false;
    }
    changed = true;
  }

  // Hold the overlay until the stall proves it is real. A sub-grace pause is
  // a demuxer hiccup, not something to tell the reader about; showing it makes
  // the panel flash over healthy video.
  if (buffering_ && !buffering_delayed_) {
    const auto held = std::chrono::steady_clock::now() - buffering_since_;
    if (held >= std::chrono::milliseconds(kBufferingGraceMs)) {
      buffering_delayed_ = true;
      changed = true;
      if (verbose_) {
        qDebug("[mpv] buffering overlay shown after %lldms of stall",
               static_cast<long long>(
                 std::chrono::duration_cast<std::chrono::milliseconds>(held)
                   .count()));
      }
    }
  }

  // --- streaming diagnostics ---------------------------------------------
  //
  // `demuxer-cache-state` is a node map. It is only present while a file is
  // open, so a failed read means "nothing playing" and the values reset --
  // otherwise a finished episode's buffer would be reported for the next one.
  qint64 forward = 0;
  bool eof = false;
  int percent = -1;
  QVariantList ranges;
  mpv_node cache_node{};
  if (mpv_get_property(mpv,
                       "demuxer-cache-state",
                       MPV_FORMAT_NODE,
                       &cache_node) >= 0) {
    const QVariantMap cache = node_to_variant(cache_node).toMap();
    mpv_free_node_contents(&cache_node);

    forward = cache.value("fw-bytes").toLongLong();
    eof = cache.value("eof").toBool();
    if (cache.contains("cache-buffering-state")) {
      percent = cache.value("cache-buffering-state").toInt();
    }

    // `seekable-ranges` is the demuxer's own answer to "what can be played
    // without waiting for the network", which is what the buffered bar should
    // draw. Each entry is `{ start, end }` in seconds; `end` is exclusive.
    const QVariantList raw = cache.value("seekable-ranges").toList();
    for (const QVariant& entry : raw) {
      const QVariantMap range = entry.toMap();
      if (!range.contains("start") || !range.contains("end")) {
        continue;
      }
      const double start = range.value("start").toDouble();
      const double end = range.value("end").toDouble();
      // mpv reports a 1e99 sentinel for "to the end of the file"; clamp it so
      // the bar can divide by the duration without an infinity.
      if (end > start) {
        ranges.append(QVariantMap{ { "start", start }, { "end", end } });
      }
    }
  }
  if (forward != cache_forward_bytes_ || eof != cache_eof_ ||
      percent != cache_buffering_percent_) {
    cache_forward_bytes_ = forward;
    cache_eof_ = eof;
    cache_buffering_percent_ = percent;
    changed = true;
  }
  if (ranges != seekable_ranges_) {
    seekable_ranges_ = ranges;
    // A separate signal because the bar is the only consumer and it redraws
    // more often than the transport controls need to.
    emit seekableRangesChanged();
  }

  // Time to first frame: a positive `time-pos` plus a configured video output
  // means a frame is being shown, not merely decoded into a cache. Measured
  // once per load so the overlay can report the real start latency.
  if (!first_frame_seen_ && load_started_.time_since_epoch().count() != 0 &&
      position_ > 0.0 && mpv_flag(mpv, "vo-configured", false)) {
    first_frame_seen_ = true;
    const auto elapsed = std::chrono::steady_clock::now() - load_started_;
    time_to_first_frame_ms_ =
      std::chrono::duration_cast<std::chrono::milliseconds>(elapsed).count();
    changed = true;
    if (qEnvironmentVariableIsSet("KITSUNE_MPV_LOG")) {
      qDebug("first frame after %lld ms (fw-bytes=%lld)",
             static_cast<long long>(time_to_first_frame_ms_),
             static_cast<long long>(cache_forward_bytes_));
    }
  }

  // --- 1 Hz sampler -----------------------------------------------------
  //
  // The one line that answers "the file is 40% downloaded, why is it not
  // playing?". It pairs the player's own view (position, what the demuxer can
  // serve) with the reader's view (how many bytes are on disk, reported by the
  // Rust side when it polls stats). Reading both lines for the same second is
  // the comparison that turns a guess into a diagnosis:
  //
  //   cache-end=3.2s seekable=1 last-end=3.0s fw-bytes=0.4MiB idle=yes
  //   file_progress[2]=950MB / 1400MB
  //
  // A small cache-end next to a large file_progress means the bytes on disk
  // are NOT the bytes mpv is reading -- the file selection, not the swarm.
  //
  // Bucketed on whole seconds so it fires exactly once per second even though
  // `pollState` runs at 200ms.
  if (verbose_ && load_started_.time_since_epoch().count() != 0) {
    const qint64 bucket = msSinceLoad() / 1000;
    if (bucket != last_sample_bucket_) {
      last_sample_bucket_ = bucket;
      double cache_end = -1.0;
      mpv_double(mpv, "demuxer-cache-time", &cache_end);
      double last_seekable = -1.0;
      for (const QVariant& entry : seekable_ranges_) {
        const double end = entry.toMap().value("end").toDouble();
        // Ignore mpv's "to EOF" sentinel when reporting the edge.
        if (end < 1e90 && end > last_seekable) {
          last_seekable = end;
        }
      }
      qDebug(
        "[mpv] t=+%llds pos=%.2fs dur=%.2fs cache-end=%.2fs seekable=%d "
        "last-end=%.2fs fw-bytes=%.1fMiB cache-pct=%d idle=%d",
        static_cast<long long>(bucket),
        position_,
        duration_,
        cache_end,
        static_cast<int>(seekable_ranges_.size()),
        last_seekable,
        static_cast<double>(cache_forward_bytes_) / (1024.0 * 1024.0),
        cache_buffering_percent_,
        mpv_flag(mpv, "core-idle", false) ? 1 : 0);
    }
  }

  if (changed) {
    emit stateChanged();
  }
}

qint64
MpvObject::msSinceLoad() const
{
  if (load_started_.time_since_epoch().count() == 0) {
    return -1;
  }
  const auto elapsed = std::chrono::steady_clock::now() - load_started_;
  return std::chrono::duration_cast<std::chrono::milliseconds>(elapsed).count();
}

void
MpvObject::drainMpvEvents()
{
  if (mpv == nullptr) {
    return;
  }

  // Drain the wakeup pipe, or the notifier stays level-triggered and we spin.
  // This MUST run on every wakeup, including when mpv is alive: leaving the
  // pipe unread floods the GUI thread with socket activations and starves the
  // render loop (audio keeps playing, video goes black).
  if (mpv_wakeup_fd_ >= 0) {
    char buf[64];
    // MSVC names the POSIX read `_read` (and takes an unsigned count).
#ifdef _WIN32
    while (_read(mpv_wakeup_fd_, buf,
                 static_cast<unsigned int>(sizeof(buf))) > 0) {
    }
#else
    while (read(mpv_wakeup_fd_, buf, sizeof(buf)) > 0) {
    }
#endif
  }

  // mpv_wait_event with a 0 timeout returns MPV_EVENT_NONE once the queue is
  // empty, so this drains everything pending and returns. Must run on ONE
  // thread only; the GUI thread is that thread.
  for (;;) {
    mpv_event* event = mpv_wait_event(mpv, 0);
    if (event == nullptr || event->event_id == MPV_EVENT_NONE) {
      break;
    }

    if (event->event_id == MPV_EVENT_PROPERTY_CHANGE &&
        event->reply_userdata == kChapterListObserveId) {
      // For a property-change event, `event->data` is an `mpv_event_property*`
      // whose own `data` field points at the value (an `mpv_node*` here, since
      // the property was observed with MPV_FORMAT_NODE). Reading the event
      // pointer AS a node is the classic mistake -- it silently yields an
      // empty list. Copy the value out now; mpv owns it.
      const auto* prop = static_cast<mpv_event_property*>(event->data);
      QVariantList next;
      if (prop != nullptr && prop->format == MPV_FORMAT_NODE &&
          prop->data != nullptr) {
        const auto* node = static_cast<const mpv_node*>(prop->data);
        if (node->format == MPV_FORMAT_NODE_ARRAY) {
          next = node_to_variant(*node).toList();
        }
      }
      if (qEnvironmentVariableIsSet("KITSUNE_MPV_LOG")) {
        // `%lld` with an explicit cast: `QVariantList::size()` returns
        // `qsizetype` (a 64-bit long on this platform), not `int`. Passing it
        // to `%d` is undefined behaviour, not just a compiler grumble -- it
        // reads the wrong half of the argument.
        qDebug("chapter-list change: entries=%lld",
               static_cast<long long>(next.size()));
      }
      if (next != chapterList_) {
        chapterList_ = next;
        emit chapterListChanged();
      }
      continue;
    }

    // Everything below is diagnostic. It is what turns "it just buffers" into
    // a reason: mpv reports WHY a load failed in the END_FILE event, and until
    // this existed that reason was dropped on the floor.
    switch (event->event_id) {
      case MPV_EVENT_START_FILE:
        if (verbose_) {
          qDebug("[mpv] start-file (t=+%lldms)",
                 static_cast<long long>(msSinceLoad()));
        }
        break;

      case MPV_EVENT_FILE_LOADED:
        if (verbose_) {
          double duration = 0.0;
          mpv_double(mpv, "duration", &duration);
          qDebug("[mpv] file-loaded after %lldms (duration=%.3fs)",
                 static_cast<long long>(msSinceLoad()), duration);
        }
        break;

      case MPV_EVENT_PLAYBACK_RESTART:
        if (verbose_) {
          qDebug("[mpv] playback-restart after %lldms",
                 static_cast<long long>(msSinceLoad()));
        }
        break;

      case MPV_EVENT_VIDEO_RECONFIG:
        if (verbose_) {
          qDebug("[mpv] video-reconfig after %lldms",
                 static_cast<long long>(msSinceLoad()));
        }
        break;

      case MPV_EVENT_END_FILE: {
        // THE important one. `reason` distinguishes a normal EOF from an
        // error, and `error` carries mpv's own text ("Failed to open", HTTP
        // 500, "no video", ...). Always logged, not just under
        // KITSUNE_MPV_LOG: a failed load is never noise.
        const auto* end = static_cast<mpv_event_end_file*>(event->data);
        const char* reason = "unknown";
        const char* error = "";
        if (end != nullptr) {
          switch (end->reason) {
            case MPV_END_FILE_REASON_EOF:
              reason = "eof";
              break;
            case MPV_END_FILE_REASON_STOP:
              reason = "stop";
              break;
            case MPV_END_FILE_REASON_QUIT:
              reason = "quit";
              break;
            case MPV_END_FILE_REASON_ERROR:
              reason = "error";
              break;
            case MPV_END_FILE_REASON_REDIRECT:
              reason = "redirect";
              break;
            default:
              break;
          }
          if (end->error != MPV_ERROR_SUCCESS) {
            error = mpv_error_string(end->error);
          }
        }
        if (error[0] != '\0') {
          qWarning("[mpv] END_FILE reason=%s error=%s (after %lldms)", reason,
                   error, static_cast<long long>(msSinceLoad()));
        } else if (verbose_) {
          qDebug("[mpv] END_FILE reason=%s (after %lldms)", reason,
                 static_cast<long long>(msSinceLoad()));
        }
        break;
      }

      case MPV_EVENT_LOG_MESSAGE:
        if (verbose_) {
          const auto* msg = static_cast<mpv_event_log_message*>(event->data);
          if (msg != nullptr) {
            // One line per message: mpv's text already ends in a newline.
            qDebug("[mpv/%s] %s", msg->prefix,
                   QString::fromUtf8(msg->text).trimmed().toUtf8().constData());
          }
        }
        break;

      default:
        break;
    }
  }
}

void
MpvObject::command(const QVariant& params)
{
  // Build an argv-style command from the QVariantList (mpv_command is the
  // array API; mpv_command_string would need shell escaping).
  const QVariantList list = params.toList();

  // Defer a `loadfile` until the GL render context exists. libmpv opens its
  // video output at load time; before the context exists it reports "No render
  // context set", permanently fails the video track, and plays only audio. The
  // context is created on the first rendered frame -- after the QML item is
  // built -- so a load from `Component.onCompleted` races it and sometimes
  // loses. Queuing here fixes every caller at once.
  const bool isLoad =
    list.size() > 0 && list.first().toString() == QLatin1String("loadfile");
  if (!renderReady_ && isLoad) {
    pendingLoad_ = params;
    return;
  }
  // Start the time-to-first-frame clock at the moment of the REQUEST, and
  // forget the previous load's measurement, so a second episode's number is
  // its own rather than the first one's.
  if (isLoad) {
    load_started_ = std::chrono::steady_clock::now();
    first_frame_seen_ = false;
    time_to_first_frame_ms_ = 0;
    last_sample_bucket_ = -1;
    // The URL is the one thing that cannot be inferred from any later line:
    // it says WHICH file index and which bridge port this load used, which is
    // how a wrong-file selection or a stale port shows up immediately. Gated
    // on verbose because the hover-preview player loads files too, and those
    // would otherwise drown the main timeline.
    if (verbose_) {
      qDebug("[mpv] loadfile t=+0ms %s",
             list.size() > 1 ? list.at(1).toString().toUtf8().constData()
                             : "(no url)");
    }
  }

  QList<QByteArray> args;
  args.reserve(list.size());
  for (const QVariant& v : list) {
    args.append(v.toString().toUtf8());
  }

  QList<const char*> argv;
  argv.reserve(args.size() + 1);
  for (const QByteArray& a : args) {
    argv.append(a.constData());
  }
  argv.append(nullptr);

  // `const_cast` because mpv's C API takes `const char **` but Qt's list holds
  // pointers-to-const; the strings themselves are not modified.
  mpv_command(mpv, const_cast<const char**>(argv.constData()));
}

void
MpvObject::setProperty(const QString& name, const QVariant& value)
{
  // mpv parses a string into the property's own type, so one string path
  // covers every case the UI has: `speed` (double), `aid`/`sid` (integer),
  // `pause` (flag), `hwdec` and `video-aspect-override` (enums). The one
  // special case is a bool, which must spell as yes/no rather than true/false.
  const QByteArray n = name.toUtf8();
  const QByteArray v =
    value.typeId() == QMetaType::Bool
      ? (value.toBool() ? QByteArray("yes") : QByteArray("no"))
      : value.toString().toUtf8();

  mpv_set_property_string(mpv, n.constData(), v.constData());
}

QVariant
MpvObject::getProperty(const QString& name) const
{
  const QByteArray n = name.toUtf8();
  mpv_node node{};
  if (mpv_get_property(mpv, n.constData(), MPV_FORMAT_NODE, &node) < 0) {
    return {};
  }

  const QVariant out = node_to_variant(node);
  // `mpv_get_property` with NODE format allocates for a string/list/map node,
  // so the node's contents must be released even though we copied what we need.
  mpv_free_node_contents(&node);
  return out;
}

QQuickFramebufferObject::Renderer*
MpvObject::createRenderer() const
{
  return new MpvRenderer(const_cast<MpvObject*>(this));
}