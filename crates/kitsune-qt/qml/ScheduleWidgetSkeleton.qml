// The embedded airing-schedule placeholder, shown while a day's broadcasts are
// in flight. Mirrors `ScheduleWidget.qml`: a fixed-width day strip (so the strip
// keeps its size and the rows below do not jump) over a few row placeholders,
// each matching a broadcast row's shape (a thumbnail, two text lines, and a
// time on the right).
import QtQuick

Item {
    id: widget

    implicitHeight: content.implicitHeight

    Column {
        id: content
        width: parent.width
        spacing: 12

        // --- Day strip ---------------------------------------------------
        // The strip is static in the real widget (every day gets a tab before
        // any request), so it is drawn solid here rather than as placeholders;
        // only the day's rows load.
        ListView {
            width: parent.width
            height: 56
            orientation: ListView.Horizontal
            spacing: 8
            clip: true
            interactive: false
            model: 29

            delegate: Rectangle {
                width: 76
                height: 56
                radius: 10
                color: index === 7 ? Theme.accent : Theme.surfaceRaised
                border.width: 1
                border.color: index === 7 ? Theme.accent : Theme.borderSubtle
            }
        }

        // --- The day's broadcasts ---------------------------------------
        // Six row placeholders, matching the embedded widget's row limit.
        Repeater {
            model: 6
            delegate: Item {
                width: widget.width
                height: 42

                Row {
                    anchors.fill: parent
                    anchors.margins: 6
                    spacing: 12

                    // Episode thumbnail.
                    Skeleton {
                        width: 56
                        height: 32
                        radius: 4
                        anchors.verticalCenter: parent.verticalCenter
                    }

                    // Title + episode line.
                    Column {
                        width: parent.width - 140
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 6
                        Skeleton { width: parent.width * 0.6; height: 11; radius: 4 }
                        Skeleton { width: parent.width * 0.35; height: 10; radius: 4 }
                    }
                }
            }
        }
    }
}