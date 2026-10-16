// A small add-to-list control for a poster card's hover overlay.
//
// Shows a "+" when the work is not on the reader's list, and a pencil when it
// is. Clicking either opens a tiny menu of the main lists (Watching /
// Completed / Plan to watch); when the work is already listed the menu also
// offers Remove. The status comes from the shared `ListStatusStore`, so many
// cards do not each issue their own lookup.
import QtQuick
import QtQuick.Controls
import com.kitsune.app

Item {
    id: control

    // The work this control adds.
    property int animeId: 0
    // The Backend, so a click can sign the reader in when needed.
    property var backend: null
    // The work's airing status ("RELEASING", "FINISHED", ...), or "" when
    // unknown. A still-airing work cannot be marked Completed, so that menu
    // row is dropped for it.
    property string animeStatus: ""

    // The menu rows, minus Completed when the work is still airing.
    readonly property var offeredStatuses: {
        if (animeStatus === "RELEASING") {
            return ListStatusStore.menuStatuses.filter(function (s) {
                return s.value !== "completed"
            })
        }
        return ListStatusStore.menuStatuses
    }

    // The current status wire value ("current" / "completed" / ...) or "".
    readonly property string status: ListStatusStore.statusFor(animeId)
    readonly property bool listed: status !== ""

    implicitWidth: 30
    implicitHeight: 30

    // The pencil turns into a filled badge when the work is on a list.
    Rectangle {
        id: face
        anchors.fill: parent
        radius: 8
        color: control.listed
            ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.9)
            : Qt.rgba(0, 0, 0, 0.55)
        border.width: 1
        border.color: mouse.containsMouse ? Theme.accent : Theme.glassBorder

        Behavior on color { ColorAnimation { duration: 140 } }

        // The glyph: "+" when not listed, a pencil when listed.
        Text {
            anchors.centerIn: parent
            visible: !control.listed
            text: "+"
            color: "white"
            font.pixelSize: 18
            font.bold: true
        }
        Text {
            anchors.centerIn: parent
            visible: control.listed
            // A pencil glyph: the work is already on a list, so clicking
            // edits rather than adds.
            text: "✎"
            color: "white"
            font.pixelSize: 14
            font.bold: true
        }

        MouseArea {
            id: mouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: {
                // Signed out: offer to sign in rather than a menu whose writes
                // would all fail.
                if (!ListStatusStore.signedIn) {
                    if (control.backend) {
                        const url = control.backend.beginLogin()
                        if (url && url.length > 0) Qt.openUrlExternally(url)
                    }
                    return
                }
                menu.open()
            }
        }
    }

    // A tiny menu anchored under the button, drawn in the overlay so a card's
    // own clipping cannot cut it off.
    Popup {
        id: menu
        parent: face
        x: -menu.width + face.width
        y: face.height + 6
        width: 150
        padding: 6
        modal: false
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

        background: Rectangle {
            color: Theme.surfaceRaised
            radius: 10
            border.width: 1
            border.color: Theme.borderSubtle
        }

        contentItem: Column {
            spacing: 2

            Repeater {
                model: control.offeredStatuses
                delegate: Rectangle {
                    required property var modelData
                    width: menu.width - 12
                    height: 30
                    radius: 7
                    color: rowMouse.containsMouse ? Theme.surfaceHover : "transparent"

                    Text {
                        anchors.left: parent.left
                        anchors.leftMargin: 10
                        anchors.verticalCenter: parent.verticalCenter
                        text: modelData.label
                        color: control.status === modelData.value ? Theme.accent : Theme.ink
                        font.pixelSize: 12
                        font.bold: control.status === modelData.value
                    }
                    MouseArea {
                        id: rowMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            menu.close()
                            ListStatusStore.setStatus(control.animeId, modelData.value)
                        }
                    }
                }
            }

            Rectangle {
                visible: control.listed
                width: menu.width - 12
                height: 1
                color: Theme.borderSubtle
            }
            Rectangle {
                visible: control.listed
                width: menu.width - 12
                height: 30
                radius: 7
                color: removeMouse.containsMouse ? Theme.surfaceHover : "transparent"

                Text {
                    anchors.left: parent.left
                    anchors.leftMargin: 10
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Remove from list"
                    color: Theme.danger
                    font.pixelSize: 12
                }
                MouseArea {
                    id: removeMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        menu.close()
                        ListStatusStore.remove(control.animeId)
                    }
                }
            }
        }
    }
}