import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: bar
    z: 1000

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

    // Fullscreen backdrop to clear focus when clicking anywhere outside the search bar
    Item {
        id: focusBackdrop
        parent: Overlay.overlay
        anchors.fill: parent
        visible: searchField.activeFocus
        z: 999

        MouseArea {
            anchors.fill: parent
            propagateComposedEvents: true
            onPressed: function(mouse) {
                bar.forceActiveFocus()
                mouse.accepted = false
            }
        }
    }

    // Global keyboard shortcut to focus search
    Shortcut {
        sequences: ["Ctrl+K", "Cmd+K"]
        onActivated: {
            searchField.forceActiveFocus()
            searchField.selectAll()
        }
    }

    // Escape keyboard shortcut to clear search focus
    Shortcut {
        enabled: searchField.activeFocus
        sequence: "Escape"
        onActivated: bar.forceActiveFocus()
    }

    // Hairline bottom border
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: 1
        color: Theme.borderSubtle
    }

    // --- Left Section: Brand ------------------------------------------
    Row {
        id: leftSection
        anchors.left: parent.left
        anchors.leftMargin: Theme.pad
        anchors.verticalCenter: parent.verticalCenter
        spacing: 10

        Item {
            width: brandRow.implicitWidth
            height: brandRow.implicitHeight

            Row {
                id: brandRow
                spacing: 10

                Rectangle {
                    width: 34
                    height: 34
                    radius: 10
                    scale: brandMouse.containsMouse ? 1.05 : 1.0
                    gradient: Gradient {
                        GradientStop { position: 0.0; color: "#ff2a5f" }
                        GradientStop { position: 1.0; color: "#ff7e40" }
                    }

                    Behavior on scale { NumberAnimation { duration: 150; easing.type: Easing.OutQuad } }

                    Text {
                        anchors.centerIn: parent
                        text: "K"
                        color: "white"
                        font.pixelSize: 18
                        font.bold: true
                    }
                }

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Kitsune"
                    color: brandMouse.containsMouse ? Theme.accent : Theme.ink
                    font.pixelSize: 20
                    font.bold: true

                    Behavior on color { ColorAnimation { duration: 150 } }
                }
            }

            MouseArea {
                id: brandMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: {
                    bar.forceActiveFocus()
                    bar.openHome()
                }
            }
        }
    }

    // --- Center Section: Search Bar -----------------------------------
    Rectangle {
        id: searchContainer
        anchors.centerIn: parent
        width: Math.min(420, parent.width - leftSection.width - rightSection.width - 48)
        height: 36
        radius: 18
        color: searchField.activeFocus ? Theme.surface : Theme.surfaceHover
        border.width: 1
        border.color: searchField.activeFocus ? Theme.accent : Theme.borderSubtle

        Behavior on color { ColorAnimation { duration: 150 } }
        Behavior on border.color { ColorAnimation { duration: 150 } }

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 12
            anchors.rightMargin: 8
            spacing: 8

            Text {
                Layout.alignment: Qt.AlignVCenter
                verticalAlignment: Text.AlignVCenter
                text: "⌕"
                color: searchField.activeFocus ? Theme.accent : Theme.inkMuted
                font.pixelSize: 16

                Behavior on color { ColorAnimation { duration: 150 } }
            }

            TextField {
                id: searchField
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignVCenter
                verticalAlignment: Text.AlignVCenter
                placeholderText: "Search anime…"
                color: Theme.ink
                placeholderTextColor: Theme.inkFaint
                font.pixelSize: 13
                selectByMouse: true
                background: Item {}
                onAccepted: {
                    const term = text.trim()
                    if (term.length > 0) bar.search(term)
                }
                Keys.onEscapePressed: bar.forceActiveFocus()
            }

            // Clear Search Button
            Rectangle {
                Layout.alignment: Qt.AlignVCenter
                visible: searchField.text.length > 0
                width: 20
                height: 20
                radius: 10
                color: clearMouse.containsMouse ? Theme.borderSubtle : "transparent"

                Text {
                    anchors.centerIn: parent
                    text: "✕"
                    color: Theme.inkMuted
                    font.pixelSize: 11
                }

                MouseArea {
                    id: clearMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        searchField.text = ""
                        searchField.forceActiveFocus()
                    }
                }
            }

            // Shortcut Hint Badge
            Rectangle {
                Layout.alignment: Qt.AlignVCenter
                visible: searchField.text.length === 0 && !searchField.activeFocus
                implicitWidth: shortcutText.implicitWidth + 8
                implicitHeight: 18
                radius: 4
                color: Theme.surface
                border.width: 1
                border.color: Theme.borderSubtle

                Text {
                    id: shortcutText
                    anchors.centerIn: parent
                    text: "⌘K"
                    color: Theme.inkFaint
                    font.pixelSize: 10
                    font.bold: true
                }
            }
        }
    }

    // --- Right Section: Segmented Pill & Settings ----------------------
    Row {
        id: rightSection
        anchors.right: parent.right
        anchors.rightMargin: Theme.pad
        anchors.verticalCenter: parent.verticalCenter
        spacing: 16

        // Segmented My List / Filter pill
        Rectangle {
            width: 184
            height: 36
            radius: 18
            color: Theme.surface
            border.width: 1
            border.color: Theme.borderSubtle

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
                        color: bar.activeRoute === "list" ? "white" : (listMouse.containsMouse ? Theme.ink : Theme.inkMuted)
                        font.pixelSize: 12
                        font.bold: true

                        Behavior on color { ColorAnimation { duration: 150 } }
                    }

                    MouseArea {
                        id: listMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            bar.forceActiveFocus()
                            bar.openMyList()
                        }
                    }
                }

                Item {
                    width: parent.width / 2
                    height: parent.height

                    Text {
                        anchors.centerIn: parent
                        text: "Filter"
                        color: bar.activeRoute === "filter" ? "white" : (filterMouse.containsMouse ? Theme.ink : Theme.inkMuted)
                        font.pixelSize: 12
                        font.bold: true

                        Behavior on color { ColorAnimation { duration: 150 } }
                    }

                    MouseArea {
                        id: filterMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            bar.forceActiveFocus()
                            bar.openFilter()
                        }
                    }
                }
            }
        }

        // Settings
        Rectangle {
            width: 36
            height: 36
            radius: 18
            color: settingsMouse.containsMouse ? Theme.surfaceHover : Theme.surface
            border.width: 1
            border.color: settingsMouse.containsMouse ? Theme.accent : Theme.borderSubtle

            Behavior on color { ColorAnimation { duration: 150 } }
            Behavior on border.color { ColorAnimation { duration: 150 } }

            Text {
                id: settingsIcon
                anchors.centerIn: parent
                text: "⚙"
                color: settingsMouse.containsMouse ? Theme.accent : Theme.inkMuted
                font.pixelSize: 17
                rotation: settingsMouse.containsMouse ? 45 : 0

                Behavior on color { ColorAnimation { duration: 150 } }
                Behavior on rotation { NumberAnimation { duration: 250; easing.type: Easing.OutQuad } }
            }

            MouseArea {
                id: settingsMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: {
                    bar.forceActiveFocus()
                    bar.openSettings()
                }
            }

            ToolTip.visible: settingsMouse.containsMouse
            ToolTip.text: "Settings"
            ToolTip.delay: 500
        }
    }
}