// The watch page: an embedded libmpv surface that plays a stream URL.
//
// The player is a real QQuickFramebufferObject (MpvObject, C++), so libass draws
// ASS/SSA subtitles natively -- the reason playback moved out of the webview.
// A later step feeds it the torrent http_bridge URL; for now it takes any URL,
// which is enough to prove the render path.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.kitsune.app

Item {
    id: page

    // The stream URL to play. Setting it (re)loads it into mpv.
    property string streamUrl: ""

    signal back()

    function load(url) {
        page.streamUrl = url
        if (url.length > 0) {
            player.command(["loadfile", url, "replace"])
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // Header
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 56
            color: Theme.surface

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Theme.pad
                anchors.rightMargin: Theme.pad

                NavButton {
                    label: "Back"
                    onClicked: page.back()
                }
                Text {
                    Layout.fillWidth: true
                    text: page.streamUrl.length > 0 ? page.streamUrl : "(no stream)"
                    color: Theme.inkMuted
                    font.pixelSize: 13
                    elide: Text.ElideMiddle
                }
            }
        }

        // The embedded mpv surface.
        MpvObject {
            id: player
            Layout.fillWidth: true
            Layout.fillHeight: true
        }
    }

    Component.onCompleted: {
        if (streamUrl.length > 0) {
            player.command(["loadfile", streamUrl, "replace"])
        }
    }
}