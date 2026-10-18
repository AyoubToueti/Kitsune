// The "a page is in flight" row for an infinite-scrolling list.
//
// Deliberately the counterpart of `InlineRetry`: same footer slot, same
// height, so swapping between them does not shift the grid. The spinner is
// what tells the reader a request is actually running -- text alone reads as
// a stuck list.
//
// The glyph is `PlayerIcon`'s `spinner` arc with a rotation animation on the
// Canvas item itself, so the path is drawn once and spun rather than redrawn
// every frame.
import QtQuick

Item {
    id: row

    // Wording, so a page can say "Loading recommendations…" instead of the
    // generic line without duplicating the layout.
    property string message: "Loading more\u2026"

    implicitHeight: 48

    Row {
        anchors.centerIn: parent
        spacing: 10

        PlayerIcon {
            id: spinner
            anchors.verticalCenter: parent.verticalCenter
            kind: "spinner"
            width: 15
            height: 15
            color: Theme.inkMuted

            // `running` follows the theme flag, so reduced motion freezes the
            // arc instead of leaving it spinning.
            RotationAnimation on rotation {
                from: 0
                to: 360
                duration: 900
                loops: Animation.Infinite
                running: Theme.animationsEnabled
            }
        }

        Text {
            anchors.verticalCenter: parent.verticalCenter
            text: row.message
            color: Theme.inkMuted
            font.pixelSize: 13
            font.bold: true
        }
    }
}