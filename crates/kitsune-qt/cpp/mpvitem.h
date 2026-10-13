// An embedded libmpv video surface for Qt Quick.
//
// Ported from mpv-examples/libmpv/qml (main.h / main.cpp). mpv renders into an
// FBO that Qt Quick composites like any other item, so QML overlays (controls,
// OSD) sit on top. libass draws ASS/SSA subtitles natively, which the webview
// could never do -- the reason playback moved out of the webview in the first
// place.
#pragma once

#include <QtQuick/QQuickFramebufferObject>

#include <mpv/client.h>
#include <mpv/render_gl.h>

class MpvRenderer;

/// Register `MpvObject` with the QML engine under the `com.kitsune.app` module.
/// Must be called before the QML is loaded.
void kitsuneRegisterMpv();

class MpvObject : public QQuickFramebufferObject
{
  Q_OBJECT

  mpv_handle* mpv;
  mpv_render_context* mpv_gl;

  friend class MpvRenderer;

public:
  static void on_update(void* ctx);

  explicit MpvObject(QQuickItem* parent = nullptr);
  ~MpvObject() override;
  Renderer* createRenderer() const override;

public slots:
  /// Run an mpv command, e.g. `command(["loadfile", url])`.
  void command(const QVariant& params);
  void setProperty(const QString& name, const QVariant& value);

signals:
  void onUpdate();

private slots:
  void doUpdate();
};