// A single anime poster: cover art, a score badge, and a clamped title.
// Mirrors the Svelte `AnimeCard` / `TrendingCard`.
import QtQuick
import QtQuick.Controls

Item {
    id: card

    // Plain properties rather than a model role, so the delegate stays
    // independent of how the caller supplies data.
    property string title
    property real score
    property url cover
    property string format

    signal clicked()

    implicitWidth: 200
    implicitHeight: width * 1.5 + 56

    Rectangle {
        id: poster
        width: parent.width
        height: parent.width * 1.5
        radius: Theme.radius
        color: Theme.surfaceRaised
        clip: true

        Image {
            anchors.fill: parent
            source: card.cover
            fillMode: Image.PreserveAspectCrop
            asynchronous: true
            // A cover that fails to load leaves the raised rectangle, which
            // reads as an intentional placeholder rather than a broken glyph.
            sourceSize.width: 400
        }

        // A soft scrim so the badge stays legible over a light poster.
        Rectangle {
            anchors.fill: parent
            gradient: Gradient {
                GradientStop { position: 0.6; color: "transparent" }
                GradientStop { position: 1.0; color: "#99000000" }
            }
        }

        // Score badge, top-right (matches the Svelte card).
        Rectangle {
            visible: card.score > 0
            anchors.top: parent.top
            anchors.right: parent.right
            anchors.margins: 8
            radius: 6
            color: "#cc0b0d12"
            implicitWidth: scoreLabel.implicitWidth + 14
            implicitHeight: scoreLabel.implicitHeight + 6

            Text {
                id: scoreLabel
                anchors.centerIn: parent
                text: "★ " + card.score
                color: Theme.score
                font.pixelSize: 13
                font.bold: true
            }
        }

        // Format chip, bottom-left.
        Rectangle {
            visible: card.format.length > 0
            anchors.left: parent.left
            anchors.bottom: parent.bottom
            anchors.margins: 8
            radius: 6
            color: "#cc7c5cff"
            implicitWidth: formatLabel.implicitWidth + 14
            implicitHeight: formatLabel.implicitHeight + 6

            Text {
                id: formatLabel
                anchors.centerIn: parent
                text: card.format
                color: "white"
                font.pixelSize: 11
                font.bold: true
            }
        }

        HoverHandler { id: hover }

        MouseArea {
            anchors.fill: parent
            onClicked: card.clicked()
        }
    }

    Text {
        anchors.top: poster.bottom
        anchors.topMargin: 8
        width: parent.width
        text: card.title
        color: hover.hovered ? Theme.ink : Theme.inkMuted
        font.pixelSize: 14
        elide: Text.ElideRight
        maximumLineCount: 2
        wrapMode: Text.WordWrap
    }
}