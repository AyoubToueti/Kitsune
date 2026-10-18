// The centred "request failed, retry?" card, shown over whatever page is up
// when a load fails. Bound to the `RetryState` singleton, so any page can raise
// it by reporting a failure and the window can re-run the page on Retry.
//
// A scrim dims the page behind it; the card carries the message and a Retry
// button that is disabled (with a countdown) while a rate-limit lock is active.
import QtQuick
import QtQuick.Layouts

Item {
    id: overlay

    // The scrim covers the whole window, so a click anywhere behind the card is
    // swallowed rather than reaching the page under it.
    Rectangle {
        anchors.fill: parent
        color: Qt.rgba(0, 0, 0, 0.55)

        MouseArea { anchors.fill: parent }
    }

    Rectangle {
        anchors.centerIn: parent
        width: 380
        implicitHeight: card.implicitHeight + 48
        radius: 16
        color: Theme.surfaceRaised
        border.width: 1
        border.color: Theme.borderSubtle

        ColumnLayout {
            id: card
            anchors.centerIn: parent
            width: parent.width - 48
            spacing: 14

            // A warning glyph, so the card reads as a problem at a glance.
            Text {
                Layout.alignment: Qt.AlignHCenter
                text: "!"
                color: Theme.danger
                font.pixelSize: 30
                font.bold: true
            }

            Text {
                Layout.alignment: Qt.AlignHCenter
                Layout.fillWidth: true
                text: RetryState.headline
                color: Theme.ink
                font.pixelSize: 17
                font.bold: true
                wrapMode: Text.WordWrap
                horizontalAlignment: Text.AlignHCenter
            }

            // The reason, in words a reader understands rather than the
            // backend's own string ("provider returned HTTP 429: ..."). The
            // rate-limit case reads the LIVE countdown, so the number here and
            // the number on the button tick down together.
            Text {
                Layout.fillWidth: true
                text: RetryState.detail
                color: Theme.inkMuted
                font.pixelSize: 12
                wrapMode: Text.WordWrap
                horizontalAlignment: Text.AlignHCenter
            }

            Item { Layout.preferredHeight: 2 }

            // The Retry button. Disabled (dimmed) while locked, and its label
            // carries the countdown so the reader knows when it unlocks.
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 42
                radius: 10
                color: RetryState.locked
                    ? Theme.surfaceHover
                    : (retryMouse.containsMouse ? Theme.accentHover : Theme.accent)
                border.width: RetryState.locked ? 1 : 0
                border.color: Theme.borderSubtle

                Text {
                    anchors.centerIn: parent
                    text: RetryState.locked
                        ? ("Retry in " + RetryState.remaining + "s")
                        : "Retry"
                    color: RetryState.locked ? Theme.inkFaint : "white"
                    font.pixelSize: 14
                    font.bold: true
                }

                MouseArea {
                    id: retryMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    // No pointer cursor while locked: the button does nothing.
                    cursorShape: RetryState.locked ? Qt.ArrowCursor : Qt.PointingHandCursor
                    onClicked: RetryState.requestRetry()
                }
            }
        }
    }
}