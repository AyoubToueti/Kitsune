// Placeholder rows for a ranked title list: the home page's Top 100 preview and
// the full Top Rated page. Mirrors the `TopPage` row delegate (rank number, a
// 50x72 portrait, a title line and a facts line, then a score on the right) at
// the same 88px height and 8px spacing, so the real rows swap in without a
// shift.
import QtQuick
import QtQuick.Layouts

Column {
    id: list

    // How many placeholder rows to draw. The home preview wants fewer than the
    // full page.
    property int count: 10

    spacing: 8

    Repeater {
        model: list.count

        delegate: Rectangle {
            width: list.width
            height: 88
            radius: Theme.radius
            color: Theme.surfaceRaised

            RowLayout {
                anchors.fill: parent
                anchors.margins: 8
                spacing: 14

                // Rank number.
                Skeleton {
                    Layout.preferredWidth: 44
                    Layout.preferredHeight: 26
                    Layout.alignment: Qt.AlignVCenter
                    radius: 6
                }

                // Portrait.
                Skeleton {
                    Layout.preferredWidth: 50
                    Layout.preferredHeight: 72
                    radius: 8
                }

                // Title + facts.
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 8
                    Skeleton {
                        Layout.fillWidth: true
                        Layout.preferredWidth: 0
                        Layout.maximumWidth: 320
                        height: 14
                        radius: 4
                    }
                    Skeleton {
                        Layout.preferredWidth: 160
                        height: 12
                        radius: 4
                    }
                }

                // Score.
                Skeleton {
                    Layout.preferredWidth: 48
                    Layout.preferredHeight: 20
                    Layout.alignment: Qt.AlignVCenter
                    radius: 6
                }
            }
        }
    }
}