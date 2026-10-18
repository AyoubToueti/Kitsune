// A retry row for a FAILED LATER PAGE in an infinite-scrolling list.
//
// The full-screen RetryOverlay is for a first-page failure, where there is
// nothing to preserve. Once pages are already on screen, taking over the whole
// view to report that page 4 failed throws away the reader's position and
// their content. This sits under the grid instead: the loaded pages stay, and
// Retry re-requests only the page that failed.
//
// Deliberately distinct from a "Loading more" row -- a spinner implies
// progress, and the whole point is that progress has stopped.
import QtQuick
import QtQuick.Layouts

Item {
    id: row

    // The failure reason, already humanised by `RetryState`.
    property string message: ""
    // A retry is in flight; the button is inert and says so.
    property bool busy: false
    // Whole seconds left on a rate-limit lock. `0` means retry is available.
    //
    // Counted here rather than reused from `RetryState`: that backoff belongs
    // to the overlay, and ticking it down would leak into an unrelated page.
    property int retryIn: 0

    // Whether a click would do anything.
    readonly property bool canRetry: !busy && retryIn <= 0

    signal retry()

    implicitHeight: pill.implicitHeight + 24

    Rectangle {
        id: pill
        anchors.centerIn: parent
        // Wide enough to read, narrow enough not to look like a page banner.
        width: Math.min(560, row.width - 32)
        implicitHeight: content.implicitHeight + 20
        radius: 14
        color: Theme.glassFillSubtle
        border.width: 1
        border.color: Theme.glassBorder

        RowLayout {
            id: content
            anchors.centerIn: parent
            width: parent.width - 26
            spacing: 12

            // Warning glyph, matching the overlay's so the two read as the
            // same family of message.
            Rectangle {
                Layout.alignment: Qt.AlignVCenter
                implicitWidth: 22
                implicitHeight: 22
                radius: 11
                color: row.retryIn > 0 ? Qt.rgba(0.49, 0.36, 1, 0.15)
                                       : Qt.rgba(0.90, 0.28, 0.30, 0.15)

                Text {
                    anchors.centerIn: parent
                    text: "!"
                    color: row.retryIn > 0 ? Theme.accentHover : Theme.danger
                    font.pixelSize: 12
                    font.bold: true
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 0

                Text {
                    Layout.fillWidth: true
                    text: "Couldn't load more"
                    color: Theme.ink
                    font.pixelSize: 12
                    font.bold: true
                }

                Text {
                    Layout.fillWidth: true
                    // The reason, not the backend's raw string: `RetryState`
                    // has already turned "HTTP 429 ..." into words.
                    text: row.message
                    color: Theme.inkMuted
                    font.pixelSize: 11
                    elide: Text.ElideRight
                }
            }

            Rectangle {
                Layout.alignment: Qt.AlignVCenter
                implicitWidth: retryLabel.implicitWidth + 28
                implicitHeight: 32
                radius: 16
                color: row.canRetry
                    ? (retryArea.containsMouse ? Theme.accentHover : Theme.accent)
                    : Theme.surfaceHover
                border.width: row.canRetry ? 0 : 1
                border.color: Theme.borderSubtle

                Text {
                    id: retryLabel
                    anchors.centerIn: parent
                    // Tabular figures so the countdown does not jitter the
                    // button's width as the number shrinks.
                    text: row.busy
                        ? "Retrying\u2026"
                        : (row.retryIn > 0 ? ("Retry in " + row.retryIn + "s") : "Retry")
                    color: row.canRetry ? "white" : Theme.inkFaint
                    font.pixelSize: 11
                    font.bold: true
                    font.family: row.retryIn > 0 ? "monospace" : ""
                }

                MouseArea {
                    id: retryArea
                    anchors.fill: parent
                    enabled: row.canRetry
                    hoverEnabled: true
                    // No pointer cursor while inert: the button does nothing.
                    cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
                    onClicked: row.retry()
                }
            }
        }
    }
}