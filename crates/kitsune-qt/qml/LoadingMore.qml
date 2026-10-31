// The "a page is in flight" row for an infinite-scrolling list.
//
// Deliberately the counterpart of `InlineRetry`: same footer slot, same
// height, so swapping between them does not shift the grid. The thinking fox
// is what tells the reader a request is actually running -- text alone reads
// as a stuck list.
//
// It renders the compact `ErrorState` "loading" kind, so every wait in the app
// (a release search, the next page, a buffering player) wears the same face.
import QtQuick
import com.kitsune.app

Item {
    id: row

    // Wording, so a page can say "Loading recommendations…" instead of the
    // generic line without duplicating the layout.
    property string message: "Loading more\u2026"

    // CONSTANT, and that is load-bearing: the footer's height feeds the
    // view's contentHeight, which drives `atYEnd`, whose handler starts the
    // next load. Matches `InlineRetry`, so swapping the two never shifts.
    implicitHeight: 118

    Rectangle {
        anchors.centerIn: parent
        width: Math.min(560, row.width - 32)
        height: row.implicitHeight - 16
        radius: 14
        color: Theme.glassFillSubtle
        border.width: 1
        border.color: Theme.glassBorder

        ErrorState {
            anchors.fill: parent
            anchors.margins: 4
            compact: true
            kind: "loading"
            showActions: false
            title: row.message
            message: ""
            artWidth: 72
        }
    }
}