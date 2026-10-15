// Embedded libmpv video surface. Ported from mpv-examples/libmpv/qml, adapted
// to Qt 6 (which removed QQuickWindow::resetOpenGLState and
// setPersistentOpenGLContext), then extended with a small playback API so QML
// can build a VLC/PotPlayer-style control bar on top.
#include "mpvitem.h"

#include <clocale>
#include <stdexcept>
#include <unistd.h>

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
  if (mpv == nullptr) {
    throw std::runtime_error("could not create mpv context");
  }

  // libmpv renders into our FBO, not its own window.
  mpv_set_option_string(mpv, "vo", "libmpv");
  // Quiet by default; KITSUNE_MPV_LOG=1 turns on mpv's own logging.
  mpv_set_option_string(mpv, "terminal",
                        qEnvironmentVariableIsSet("KITSUNE_MPV_LOG") ? "yes"
                                                                     : "no");
  mpv_request_log_messages(mpv,
                           qEnvironmentVariableIsSet("KITSUNE_MPV_LOG") ? "v"
                                                                        : "warn");

  if (mpv_initialize(mpv) < 0) {
    throw std::runtime_error("could not initialize mpv context");
  }

  // Hardware decoding when available; falls back to software otherwise.
  mpv_set_option_string(mpv, "hwdec", "auto");

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

  // A stall is either an explicit cache wait or an idle core. `core-idle` alone
  // would also read true while the reader has PAUSED, which is not buffering --
  // so a user pause is excluded.
  const bool cache_wait = mpv_flag(mpv, "paused-for-cache", false);
  const bool idle = mpv_flag(mpv, "core-idle", false);
  const bool buffering = !paused_ && (cache_wait || idle);
  if (buffering != buffering_) {
    buffering_ = buffering;
    changed = true;
  }

  if (changed) {
    emit stateChanged();
  }
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
    while (read(mpv_wakeup_fd_, buf, sizeof(buf)) > 0) {
    }
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
        qDebug("chapter-list change: entries=%d", next.size());
      }
      if (next != chapterList_) {
        chapterList_ = next;
        emit chapterListChanged();
      }
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
  if (!renderReady_ && list.size() > 0
      && list.first().toString() == QLatin1String("loadfile")) {
    pendingLoad_ = params;
    return;
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