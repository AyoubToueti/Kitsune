// The global top bar: brand on the left, a search box in the centre, and the
// My List / Filter pill plus the settings gear on the right. Rendered once by
// the window above every page, so it never remounts on navigation.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: bar
    z: 1000

    // Which segmented pill is active: "list", "filter", or "" for neither.
    property string activeRoute: ""

    // Whether the history chevrons are usable, derived by the window from its
    // own history stacks.
    property bool canGoBack: false
    property bool canGoForward: false

    signal search(string term)
    signal openHome()
    signal openMyList()
    signal openFilter()
    signal openSettings()
    signal goBack()
    signal goForward()

    height: 60
    color: Theme.surfaceRaised

    // A full-window backdrop that clears the search field's focus when the
    // reader clicks anywhere else.
    Item {
        id: focusBackdrop
        parent: Overlay.overlay
        anchors.fill: parent
        visible: searchField.activeFocus
        z: 999

        MouseArea {
            anchors.fill: parent
            propagateComposedEvents: true
            onPressed: function (mouse) {
                bar.forceActiveFocus()
                mouse.accepted = false
            }
        }
    }

    // Focus the search box from anywhere.
    Shortcut {
        sequences: ["Ctrl+K", "Cmd+K"]
        onActivated: {
            searchField.forceActiveFocus()
            searchField.selectAll()
        }
    }

    // Escape clears the search focus.
    Shortcut {
        enabled: searchField.activeFocus
        sequence: "Escape"
        onActivated: bar.forceActiveFocus()
    }

    // Hairline bottom border.
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: 1
        color: Theme.borderSubtle
    }

    // --- Left: brand ----------------------------------------------------
    Row {
        id: leftSection
        anchors.left: parent.left
        anchors.leftMargin: Theme.pad
        anchors.verticalCenter: parent.verticalCenter
        spacing: 12

        Item {
            width: brandRow.implicitWidth
            height: brandRow.implicitHeight

            Row {
                id: brandRow
                spacing: 10

                // The crest, embedded from `assets/logo.svg` (see build.rs).
                // Vector, so it stays sharp at any size or DPI; it needs the
                // qtsvg image-format plugin at runtime (see flake.nix).
                Item {
                    width: 60
                    height: 60
                    anchors.verticalCenter: parent.verticalCenter
                    scale: brandMouse.containsMouse ? 1.05 : 1.0

                    Behavior on scale { NumberAnimation { duration: 150; easing.type: Easing.OutQuad } }

                    Image {
                        anchors.fill: parent
                        source: "qrc:/qt/qml/com/kitsune/app/assets/logo.svg"
                        // 2x the display size, so a HiDPI screen still gets a
                        // crisp raster from the vector.
                        sourceSize.width: 120
                        sourceSize.height: 120
                        fillMode: Image.PreserveAspectFit
                        smooth: true
                    }
                }

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Kitsune"
                    color: brandMouse.containsMouse ? Theme.accent : Theme.ink
                    font.pixelSize: 22
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

        // History chevrons. Dimmed (and inert) when there is nowhere to go, so
        // the control reads as unavailable rather than broken. These mirror the
        // mouse back/forward buttons and the Alt+Left/Right shortcuts.
        Row {
            anchors.verticalCenter: parent.verticalCenter
            spacing: 4

            Rectangle {
                width: 30
                height: 30
                radius: 8
                color: bar.canGoBack
                    ? (backMouse.containsMouse ? Theme.surfaceHover : "transparent")
                    : "transparent"

                Text {
                    anchors.centerIn: parent
                    text: "\u2039"
                    color: bar.canGoBack ? Theme.ink : Theme.inkFaint
                    font.pixelSize: 22
                    font.bold: true
                    opacity: bar.canGoBack ? 1.0 : 0.35
                }

                MouseArea {
                    id: backMouse
                    anchors.fill: parent
                    enabled: bar.canGoBack
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: bar.goBack()
                }
            }

            Rectangle {
                width: 30
                height: 30
                radius: 8
                color: bar.canGoForward
                    ? (fwdMouse.containsMouse ? Theme.surfaceHover : "transparent")
                    : "transparent"

                Text {
                    anchors.centerIn: parent
                    text: "\u203A"
                    color: bar.canGoForward ? Theme.ink : Theme.inkFaint
                    font.pixelSize: 22
                    font.bold: true
                    opacity: bar.canGoForward ? 1.0 : 0.35
                }

                MouseArea {
                    id: fwdMouse
                    anchors.fill: parent
                    enabled: bar.canGoForward
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: bar.goForward()
                }
            }
        }
    }

    // --- Centre: search -------------------------------------------------
    Rectangle {
        id: searchContainer
        anchors.centerIn: parent
        width: Math.min(420, parent.width - leftSection.width - rightSection.width - 48)
        height: 36
        radius: 18
        color: searchField.activeFocus
            ? Theme.surface
            : (searchHover.hovered ? Theme.surfaceHover : Qt.rgba(1, 1, 1, 0.03))
        border.width: 1
        border.color: searchField.activeFocus
            ? Theme.accent
            : (searchHover.hovered
                ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.5)
                : Theme.borderSubtle)

        Behavior on color { ColorAnimation { duration: 150 } }
        Behavior on border.color { ColorAnimation { duration: 150 } }

        HoverHandler { id: searchHover }

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 12
            anchors.rightMargin: 8
            spacing: 8

            // A magnifier glyph drawn inline as an SVG data URI. URL-encoded so
            // the `#`/`<` cannot break the URI. Needs the same qtsvg plugin.
            Image {
                Layout.alignment: Qt.AlignVCenter
                width: 14
                height: 14
                source: "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='white'><path d='M15.5 14h-.79l-.28-.27C15.41 12.59 16 11.11 16 9.5 16 5.91 13.09 3 9.5 3S3 5.91 3 9.5 5.91 16 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z'/></svg>"
                sourceSize.width: 28
                sourceSize.height: 28
                smooth: true
                opacity: searchField.activeFocus ? 1.0 : (searchHover.hovered ? 0.85 : 0.65)

                Behavior on opacity { NumberAnimation { duration: 150 } }
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

            // Clear button, shown once there is text.
            Rectangle {
                Layout.alignment: Qt.AlignVCenter
                visible: searchField.text.length > 0
                width: 20
                height: 20
                radius: 10
                color: clearMouse.containsMouse ? Theme.borderSubtle : "transparent"

                Text {
                    anchors.centerIn: parent
                    text: "\u2715"
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

            // Shortcut hint, shown while the box is empty and unfocused.
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
                    text: "\u2318K"
                    color: Theme.inkFaint
                    font.pixelSize: 10
                    font.bold: true
                }
            }
        }
    }

    // --- Right: segmented pill + settings -------------------------------
    Row {
        id: rightSection
        anchors.right: parent.right
        anchors.rightMargin: Theme.pad
        anchors.verticalCenter: parent.verticalCenter
        spacing: 16

        // The My List / Filter pill, with an indicator that slides to the
        // active half.
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
                        color: bar.activeRoute === "list"
                            ? "white"
                            : (listMouse.containsMouse ? Theme.ink : Theme.inkMuted)
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
                        color: bar.activeRoute === "filter"
                            ? "white"
                            : (filterMouse.containsMouse ? Theme.ink : Theme.inkMuted)
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

        // Settings gear.
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
                text: "\u2699"
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