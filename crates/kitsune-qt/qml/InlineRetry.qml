// A retry row for a FAILED LATER PAGE in an infinite-scrolling list.
//
// A first-page failure has nothing to preserve; once pages are already on
// screen, taking over the whole view to report that page 4 failed throws away
// the reader's position and their content. This sits under the grid instead:
// the loaded pages stay, and Retry re-requests only the page that failed.
//
// It renders a COMPACT `ErrorState` so the fox family is consistent, sized for
// a footer slot rather than a full page.
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

    implicitHeight: 180
    height: implicitHeight

    Rectangle {
        id: pill
        anchors.centerIn: parent
        // Wide enough to read, narrow enough not to look like a page banner.
        width: Math.min(560, row.width - 32)
        height: row.implicitHeight - 16
        radius: 14
        color: Theme.glassFillSubtle
        border.width: 1
        border.color: Theme.glassBorder
        clip: true

        ErrorState {
            anchors.fill: parent
            anchors.margins: 8
            compact: true
            showArt: true
            artWidth: 72
            failureMessage: row.message
            title: row.busy ? "Retrying\u2026" : ""
            message: ""
            primaryText: ""
            retryAfter: row.retryIn
            showActions: !row.busy
            onPrimaryClicked: row.retry()
        }
    }
}