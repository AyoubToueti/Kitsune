// A small add-to-list control for a poster card's hover overlay.
//
// Shows a "+" when the work is not on the reader's list, and -- when it is --
// the icon of the list it is saved to (play for Watching, a check for
// Completed, a calendar for Plan to watch, pause for On hold, a minus for
// Dropped). Clicking either opens a tiny menu of the main lists (Watching /
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

    // Resolution-independent SVG icons as data URIs, so the glyph stays sharp
    // at any scale and the two states (unlisted / listed) never depend on a
    // font shipping a particular character.
    function getSvgIcon(name, color) {
        var fill = color === undefined ? "#ffffff" : Theme.hex(color)
        var path = ""
        if (name === "play")
            path = "M8 5v14l11-7z"
        else if (name === "check")
            path = "M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"
        else if (name === "calendar")
            path = "M19 4h-1V2h-2v2H8V2H6v2H5c-1.11 0-1.99.9-1.99 2L3 20c0 1.1.89 2 2 2h14c1.1 0 2-.9 2-2V6c0-1.1-.9-2-2-2zm0 16H5V10h14v10zm0-12H5V6h14v2z"
        else if (name === "pause")
            path = "M6 19h4V5H6v14zm8-14v14h4V5h-4z"
        else if (name === "minus")
            path = "M19 13H5v-2h14v2z"
        else // "plus"
            path = "M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"

        var xml = "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='" + fill + "'><path d='" + path + "'/></svg>"
        return "data:image/svg+xml," + encodeURIComponent(xml)
    }

    // The glyph for a list status, so a listed work shows *where* it is saved
    // (play / check / calendar / pause / minus) instead of a generic pencil.
    function statusIconName(value) {
        if (value === "current") return "play"
        if (value === "completed") return "check"
        if (value === "planning") return "calendar"
        if (value === "paused") return "pause"
        if (value === "dropped") return "minus"
        return "plus"
    }

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

        // The glyph: "+" when not listed; when listed, the icon of the list
        // the work is saved to, so the button says WHERE it is rather than
        // merely that it is somewhere.
        Image {
            anchors.centerIn: parent
            width: 16
            height: 16
            visible: !control.listed
            source: control.getSvgIcon("plus")
            sourceSize.width: 32
            sourceSize.height: 32
            smooth: true
        }
        Image {
            anchors.centerIn: parent
            width: 15
            height: 15
            visible: control.listed
            source: control.getSvgIcon(control.statusIconName(control.status))
            sourceSize.width: 30
            sourceSize.height: 30
            smooth: true
        }

        // The status in words, so the glyph is never the only cue. Shown on
        // hover, where the button would otherwise be ambiguous.
        ToolTip.visible: control.listed && mouse.containsMouse
        ToolTip.text: ListStatusStore.labelFor(control.status)
        ToolTip.delay: 200

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