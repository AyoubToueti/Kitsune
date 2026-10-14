// Embedded libmpv video surface. Ported from mpv-examples/libmpv/qml, adapted
// to Qt 6 (which removed QQuickWindow::resetOpenGLState and
// setPersistentOpenGLContext).
#include "mpvitem.h"

#include <clocale>
#include <stdexcept>

#include <QGuiApplication>
#include <QList>
#include <QOpenGLContext>
#include <QOpenGLFramebufferObject>
#include <QQmlEngine>
#include <QQuickWindow>
#include <QVariant>
#include <QtGlobal>

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

} // namespace

class MpvRenderer : public QQuickFramebufferObject::Renderer
{
  MpvObject* obj;

public:
  explicit MpvRenderer(MpvObject* new_obj)
    : obj{ new_obj }
  {
    mpv_set_wakeup_callback(obj->mpv, on_mpv_events, nullptr);
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
    }
    return QQuickFramebufferObject::Renderer::createFramebufferObject(size);
  }

  void render() override
  {
    QOpenGLFramebufferObject* fbo = framebufferObject();
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

  // mpv reports starved playback through its own properties, which QML cannot
  // read directly. Poll them on a slow timer: the state changes on the order of
  // seconds, so 250ms is responsive without being busy.
  buffering_poll_ = new QTimer(this);
  buffering_poll_->setInterval(250);
  connect(buffering_poll_, &QTimer::timeout, this, &MpvObject::pollBuffering);
  buffering_poll_->start();
}

MpvObject::~MpvObject()
{
  if (mpv_gl != nullptr) {
    mpv_render_context_free(mpv_gl);
  }
  mpv_terminate_destroy(mpv);
}

namespace {

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

} // namespace

void
MpvObject::pollBuffering()
{
  // A stall is either an explicit cache wait or an idle core. `core-idle` alone
  // would also read true while the reader has PAUSED, which is not buffering --
  // so a user pause is excluded.
  const bool paused = mpv_flag(mpv, "pause", false);
  const bool cache_wait = mpv_flag(mpv, "paused-for-cache", false);
  const bool idle = mpv_flag(mpv, "core-idle", false);

  const bool now_buffering = !paused && (cache_wait || idle);
  if (now_buffering != buffering_) {
    buffering_ = now_buffering;
    emit bufferingChanged();
  }
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
MpvObject::command(const QVariant& params)
{
  // Build an argv-style command from the QVariantList (mpv_command is the
  // array API; mpv_command_string would need shell escaping).
  const QVariantList list = params.toList();
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
  const QByteArray n = name.toUtf8();
  if (value.canConvert<double>()) {
    double d = value.toDouble();
    mpv_set_property(mpv, n.constData(), MPV_FORMAT_DOUBLE, &d);
  } else {
    const QByteArray s = value.toString().toUtf8();
    mpv_set_property(mpv,
                     n.constData(),
                     MPV_FORMAT_STRING,
                     const_cast<char*>(s.constData()));
  }
}

QQuickFramebufferObject::Renderer*
MpvObject::createRenderer() const
{
  return new MpvRenderer(const_cast<MpvObject*>(this));
}