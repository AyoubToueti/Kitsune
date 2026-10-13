// The reader's own list: Continue Watching on top, then the full list grouped
// by status. Mirrors the Svelte /list route.
//
// Signed out, it shows a "sign in" prompt rather than an error: the commands
// answer an empty list when there is no token, and "nothing to show" is the
// correct state for a reader with no account.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: page

    property var backend: null
    property bool signedIn: false

    signal openAnime(int id)

    ListModel { id: continueModel }
    ListModel { id: listModel }

    Connections {
        target: page.backend

        function onAuthStatusReady(signedInNow) {
            page.signedIn = signedInNow
            if (signedInNow) {
                page.backend.continueWatching(20)
                page.backend.userList()
            } else {
                continueModel.clear()
                listModel.clear()
                status.text = "Sign in to see your list"
            }
        }

        function onContinueWatchingReady(json) {
            const items = JSON.parse(json)
            continueModel.clear()
            for (const entry of items) {
                const work = entry.anime || entry
                continueModel.append({
                    "id": work.id,
                    "title": displayTitle(work),
                    "cover": work.coverImage || "",
                    "progress": entry.progress || 0,
                    "total": work.episodeCount || 0,
                })
            }
        }

        function onUserListReady(json) {
            const entries = JSON.parse(json)
            listModel.clear()
            for (const entry of entries) {
                const work = entry.anime || entry.media || entry
                listModel.append({
                    "id": work.id,
                    "title": displayTitle(work),
                    "cover": work.coverImage || "",
                    "status": entry.status || "",
                    "progress": entry.progress || 0,
                    "total": work.episodeCount || 0,
                })
            }
            status.text = listModel.count + " in your list"
        }

        function onListChanged() {
            // Any write re-reads, so the view is never stale.
            page.backend.continueWatching(20)
            page.backend.userList()
        }

        function onAuthError(message) {
            status.text = "Failed: " + message
        }
    }

    function displayTitle(item) {
        const t = item.title || {}
        return t.userPreferred || t.english || t.romaji || t.native || "(untitled)"
    }

    Component.onCompleted: {
        if (backend) backend.authStatus()
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // Header
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 64
            color: Theme.surface

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Theme.pad
                anchors.rightMargin: Theme.pad

                Text {
                    text: "My List"
                    color: Theme.accent
                    font.pixelSize: 24
                    font.bold: true
                }
                Text {
                    id: status
                    Layout.fillWidth: true
                    color: Theme.inkFaint
                    font.pixelSize: 13
                }
            }
        }

        // Continue Watching rail (only when signed in and non-empty).
        Column {
            Layout.fillWidth: true
            Layout.preferredHeight: continueModel.count > 0 ? 210 : 0
            visible: continueModel.count > 0
            spacing: 8
            topPadding: Theme.gap
            leftPadding: Theme.pad

            Text {
                text: "Continue Watching"
                color: Theme.ink
                font.pixelSize: 18
                font.bold: true
            }

            ListView {
                width: parent.width - Theme.pad
                height: 168
                orientation: ListView.Horizontal
                spacing: Theme.gap
                clip: true
                model: continueModel

                delegate: Item {
                    width: 120
                    height: 168

                    Rectangle {
                        width: 120
                        height: 120
                        radius: Theme.radius
                        color: Theme.surfaceRaised
                        clip: true
                        Image {
                            anchors.fill: parent
                            source: model.cover
                            fillMode: Image.PreserveAspectCrop
                            asynchronous: true
                        }
                    }
                    Text {
                        anchors.top: parent.top
                        anchors.topMargin: 124
                        width: parent.width
                        text: model.title
                        color: Theme.inkMuted
                        font.pixelSize: 12
                        elide: Text.ElideRight
                        maximumLineCount: 2
                        wrapMode: Text.WordWrap
                    }
                    MouseArea {
                        anchors.fill: parent
                        onClicked: page.openAnime(model.id)
                    }
                }
            }
        }

        // Full list
        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: listModel
            leftMargin: Theme.pad
            rightMargin: Theme.pad
            topMargin: Theme.gap
            bottomMargin: Theme.pad
            spacing: 8

            delegate: Rectangle {
                width: ListView.view.width - Theme.pad * 2
                height: 84
                radius: Theme.radius
                color: rowHover.hovered ? Theme.surfaceHover : Theme.surfaceRaised

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 8
                    spacing: 12

                    Rectangle {
                        Layout.preferredWidth: 48
                        Layout.preferredHeight: 68
                        radius: 8
                        color: Theme.surface
                        clip: true
                        Image {
                            anchors.fill: parent
                            source: model.cover
                            fillMode: Image.PreserveAspectCrop
                            asynchronous: true
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 3
                        Text {
                            Layout.fillWidth: true
                            text: model.title
                            color: Theme.ink
                            font.pixelSize: 15
                            font.bold: true
                            elide: Text.ElideRight
                        }
                        Text {
                            color: Theme.inkMuted
                            font.pixelSize: 13
                            text: {
                                const parts = []
                                if (model.status) parts.push(model.status)
                                if (model.total > 0) parts.push(model.progress + "/" + model.total)
                                return parts.join("  ·  ")
                            }
                        }
                    }
                }

                HoverHandler { id: rowHover }
                MouseArea {
                    anchors.fill: parent
                    onClicked: page.openAnime(model.id)
                }
            }
        }
    }
}