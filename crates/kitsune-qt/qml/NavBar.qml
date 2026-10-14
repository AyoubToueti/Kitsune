// The global top bar, shared by every route. Mirrors `NavBar.svelte`:
// a brand mark, a centered search box, a segmented My List / Filter pill, and
// a settings button. Lives above the StackView in main.qml so it never scrolls
// away and every page draws beneath it.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: bar

    // Which segmented pill is active: "list", "filter", or "" for neither.
    property string activeRoute: ""

    // Emitted when the reader submits the search box.
    signal search(string term)
    signal openHome()
    signal openMyList()
    signal openFilter()
    signal openSettings()

    height: 60
    color: Theme.surfaceRaised

    // Hairline bottom border, so the bar reads as a distinct surface.
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: 1
        color: Theme.borderSubtle
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.pad
        anchors.rightMargin: Theme.pad
        spacing: 16

        // --- Brand --------------------------------------------------------
        RowLayout {
            spacing: 8

            Rectangle {
                width: 32
                height: 32
                radius: 9
                gradient: Gradient {
                    GradientStop { position: 0.0; color: "#ff2a5f" }
                    GradientStop { position: 1.0; color: "#ff7e40" }
                }
                Text {
                    anchors.centerIn: parent
                    text: "K"
                    color: "white"
                    font.pixelSize: 18
                    font.bold: true
                }
            }
            Text {
                text: "Kitsune"
                color: Theme.ink
                font.pixelSize: 20
                font.bold: true
            }

            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: bar.openHome()
            }
        }

        // --- Search -------------------------------------------------------
        Rectangle {
            Layout.fillWidth: true
            Layout.maximumWidth: 420
            height: 36
            radius: 18
            color: Theme.surfaceHover
            border.width: 1
            border.color: searchField.activeFocus ? Theme.accent : Theme.borderSubtle

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 8
                spacing: 8

                Text {
                    text: "⌕"
                    color: searchField.activeFocus ? Theme.accent : Theme.inkMuted
                    font.pixelSize: 18
                }

                TextField {
                    id: searchField
                    Layout.fillWidth: true
                    placeholderText: "Search anime…"
                    color: Theme.ink
                    placeholderTextColor: Theme.inkFaint
                    font.pixelSize: 13
                    background: Item {}
                    onAccepted: {
                        const term = text.trim()
                        if (term.length > 0) bar.search(term)
                    }
                }

                Text {
                    visible: searchField.text.length === 0
                    text: "⌘K"
                    color: Theme.inkFaint
                    font.pixelSize: 10
                    font.bold: true
                }
            }
        }

        Item { Layout.fillWidth: true }

        // --- Segmented My List / Filter pill ------------------------------
        Rectangle {
            width: 184
            height: 36
            radius: 18
            color: Theme.surface
            border.width: 1
            border.color: Theme.borderSubtle

            // The sliding indicator, shown only when one segment is active.
            Rectangle {
                id: indicator
                visible: bar.activeRoute === "list" || bar.activeRoute === "filter"
                width: (parent.width / 2) - 4
                height: parent.height - 8
                y: 4
                x: bar.activeRoute === "filter" ? (parent.width / 2) : 4
                radius: 14
                color: Theme.accent
                Behavior on x { NumberAnimation { duration: 200; easing.type: Easing.OutCubic } }
            }

            Row {
                anchors.fill: parent

                Item {
                    width: parent.width / 2
                    height: parent.height
                    Text {
                        anchors.centerIn: parent
                        text: "My List"
                        color: bar.activeRoute === "list" ? "white" : Theme.inkMuted
                        font.pixelSize: 12
                        font.bold: true
                    }
                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: bar.openMyList()
                    }
                }

                Item {
                    width: parent.width / 2
                    height: parent.height
                    Text {
                        anchors.centerIn: parent
                        text: "Filter"
                        color: bar.activeRoute === "filter" ? "white" : Theme.inkMuted
                        font.pixelSize: 12
                        font.bold: true
                    }
                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: bar.openFilter()
                    }
                }
            }
        }

        // --- Settings -----------------------------------------------------
        Rectangle {
            width: 36
            height: 36
            radius: 18
            color: settingsMouse.containsMouse ? Theme.surfaceHover : Theme.surfaceRaised
            border.width: 1
            border.color: settingsMouse.containsMouse ? Theme.accent : Theme.borderSubtle

            Text {
                anchors.centerIn: parent
                text: "⚙"
                color: settingsMouse.containsMouse ? Theme.accent : Theme.inkMuted
                font.pixelSize: 17
            }
            MouseArea {
                id: settingsMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: bar.openSettings()
            }
        }
    }
}