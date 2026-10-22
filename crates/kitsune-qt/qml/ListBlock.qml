// A home-page list block: a titled card of five compact rows with a
// "View more" footer pinned to the bottom. Mirrors `ListBlock.svelte`.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: block

    property string title: ""
    // The home-page section title, e.g. "Top airing".
    property var items: []

    // `provider` is the id's namespace, forwarded so the detail page fetches
    // from the source that issued the id.
    signal openAnime(int id, string provider)
    signal viewMore()

    Layout.fillWidth: true
    radius: 12
    color: "transparent"
    border.width: 1
    border.color: Theme.borderSubtle
    implicitHeight: content.implicitHeight + 24

    ColumnLayout {
        id: content
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: 12
        spacing: 6

        Text {
            text: block.title
            color: Theme.accent
            font.pixelSize: 14
            font.bold: true
        }

        Repeater {
            model: block.items
            delegate: AnimeListRow {
                required property var modelData
                Layout.fillWidth: true
                anime: modelData
                onClicked: block.openAnime(modelData.id, modelData.provider || "anilist")
            }
        }

        Item { Layout.fillHeight: true; Layout.preferredHeight: 4 }

        Text {
            text: "View more ›"
            color: viewMouse.containsMouse ? Theme.accent : Theme.inkMuted
            font.pixelSize: 13
            MouseArea {
                id: viewMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: block.viewMore()
            }
        }
    }
}