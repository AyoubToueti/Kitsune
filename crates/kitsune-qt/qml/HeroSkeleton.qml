// The home page's spotlight placeholder, shown while the hero carousel's data
// is still in flight. Mirrors `HeroCarousel`'s layout (a 360px banner with a
// left-aligned spotlight column and dot controls) so the real carousel swaps in without a shift.
import QtQuick
import QtQuick.Layouts
import com.kitsune.app

Item {
    id: hero

    implicitHeight: 360

    Rectangle {
        anchors.fill: parent
        color: Theme.surface
        clip: true

        // Backdrop artwork placeholder, filling the banner behind the column.
        Skeleton {
            anchors.fill: parent
        }

        // 1. Horizontal vignette overlay matching HeroCarousel
        Rectangle {
            anchors.fill: parent
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0.00; color: Theme.heroFadeStart }
                GradientStop { position: 0.32; color: Theme.heroFadeStart }
                GradientStop { position: 0.48; color: Theme.heroFadeMid }
                GradientStop { position: 0.60; color: "transparent" }
                GradientStop { position: 0.80; color: "transparent" }
                GradientStop { position: 1.00; color: Theme.heroFadeStart }
            }
        }

        // 2. Vertical vignette overlay matching HeroCarousel
        Rectangle {
            anchors.fill: parent
            gradient: Gradient {
                GradientStop { position: 0.00; color: "transparent" }
                GradientStop { position: 0.80; color: "transparent" }
                GradientStop { position: 1.00; color: Theme.heroFadeStart }
            }
        }

        // Spotlight column: kicker, title, score & facts row, genre chips, synopsis lines,
        // spacer, and button CTA -- matching ColumnLayout in HeroCarousel.
        ColumnLayout {
            anchors.left: parent.left
            anchors.leftMargin: 36
            anchors.right: parent.right
            anchors.rightMargin: parent.width * 0.64
            anchors.top: parent.top
            anchors.topMargin: 24
            anchors.bottom: parent.bottom
            anchors.bottomMargin: 48
            spacing: 6

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
                    width: parent.width * 0.85
                    height: 24
                    radius: 6
                }

                Skeleton {
                    width: parent.width * 0.55
                    height: 24
                    radius: 6
                }
            }

            // Score & facts row (score pill + format, duration, year, episodes).
            RowLayout {
                spacing: 10

                // Score pill (★ 85)
                Skeleton {
                    implicitWidth: 48
                    implicitHeight: 22
                    radius: 6
                }

                // Inline facts
                Row {
                    spacing: 10
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

            // Synopsis: 4 lines matching maximumLineCount: 4.
            Column {
                Layout.fillWidth: true
                spacing: 6

                Skeleton { width: parent.width; height: 12; radius: 4 }
                Skeleton { width: parent.width; height: 12; radius: 4 }
                Skeleton { width: parent.width * 0.90; height: 12; radius: 4 }
                Skeleton { width: parent.width * 0.60; height: 12; radius: 4 }
            }

            Item { Layout.fillHeight: true }

            // Action button ("View Details").
            Skeleton {
                implicitWidth: 128
                implicitHeight: 36
                radius: 18
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