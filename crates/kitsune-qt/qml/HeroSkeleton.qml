// The home page's spotlight placeholder, shown while the hero carousel's data
// is still in flight. Mirrors `HeroCarousel`'s layout (a 360px banner with a
// left-aligned spotlight column and dot controls) so the real carousel swaps in without a shift.
import QtQuick
import QtQuick.Layouts

Item {
    id: hero

    implicitHeight: 360

    Rectangle {
        anchors.fill: parent
        radius: 16
        color: Theme.surfaceRaised
        border.width: 1
        border.color: Theme.glassBorder
        clip: true

        // Backdrop artwork placeholder, filling the banner behind the column.
        Skeleton {
            anchors.fill: parent
            radius: 16
        }

        // Spotlight column: kicker, title, score & facts row, genre chips, synopsis lines,
        // spacer, and button CTA -- matching ColumnLayout in HeroCarousel.
        ColumnLayout {
            anchors.left: parent.left
            anchors.leftMargin: 36
            anchors.right: parent.right
            anchors.rightMargin: parent.width * 0.28
            anchors.top: parent.top
            anchors.topMargin: 24
            anchors.bottom: parent.bottom
            anchors.bottomMargin: 52
            spacing: 8

            // Spotlight rank badge ("#1 SPOTLIGHT").
            Skeleton {
                implicitWidth: 100
                implicitHeight: 22
                radius: 6
            }

            // Title: two stacked bars simulating title text up to 2 lines.
            Column {
                Layout.fillWidth: true
                spacing: 6

                Skeleton {
                    width: parent.width * 0.70
                    height: 28
                    radius: 6
                }

                Skeleton {
                    width: parent.width * 0.45
                    height: 28
                    radius: 6
                }
            }

            // Score & facts row (score pill + format, duration, year, episodes).
            RowLayout {
                spacing: 12

                // Score pill (★ 85)
                Skeleton {
                    implicitWidth: 48
                    implicitHeight: 22
                    radius: 6
                }

                // Inline facts
                Row {
                    spacing: 12
                    Skeleton { width: 28; height: 14; radius: 4 }
                    Skeleton { width: 36; height: 14; radius: 4 }
                    Skeleton { width: 32; height: 14; radius: 4 }
                    Skeleton { width: 44; height: 14; radius: 4 }
                }
            }

            // Genre chip row.
            Row {
                spacing: 6
                Skeleton { width: 60; height: 22; radius: 8 }
                Skeleton { width: 72; height: 22; radius: 8 }
                Skeleton { width: 52; height: 22; radius: 8 }
            }

            // Synopsis: two lines matching maximumLineCount: 2.
            Column {
                Layout.fillWidth: true
                spacing: 6

                Skeleton { width: parent.width; height: 12; radius: 4 }
                Skeleton { width: parent.width * 0.75; height: 12; radius: 4 }
            }

            Item { Layout.fillHeight: true }

            // Action button ("View Details").
            Skeleton {
                implicitWidth: 136
                implicitHeight: 38
                radius: 19
            }
        }

        // Expanding pill indicators along the bottom-left.
        Row {
            anchors.left: parent.left
            anchors.leftMargin: 36
            anchors.bottom: parent.bottom
            anchors.bottomMargin: 18
            spacing: 6

            Repeater {
                model: 5
                delegate: Skeleton {
                    required property int index
                    width: index === 0 ? 24 : 8
                    height: 8
                    radius: 4
                    opacity: index === 0 ? 1.0 : 0.5
                }
            }
        }
    }
}