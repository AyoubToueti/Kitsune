// The global top bar: brand on the left, a command/search field in the
// centre, and icon buttons for My List / Filter / Downloads plus the settings
// gear on the right. Rendered once by the window above every page, so it never
// remounts on navigation.
//
// Design: "command bar" — a wide, launcher-style field in the middle with an
// inline Go action, and a compact rail of icon buttons instead of the old
// segmented text pill.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: bar
    z: 1000

    // Which section is active: "list", "filter", "downloads", or "" for none.
    property string activeRoute: ""

    // Whether the history chevrons are usable, derived by the window from its
    // own history stacks.
    property bool canGoBack: false
    property bool canGoForward: false

    // How many downloads are waiting. The window sets this; the badge hides
    // itself when the count is zero.
    property int downloadCount: 0

    signal search(string term)
    signal openHome()
    signal openMyList()
    signal openFilter()
    signal openDownloads()
    signal openSettings()
    signal goBack()
    signal goForward()

    height: 60
    color: Theme.surfaceRaised

    // Brand colours, kept in one place so they are trivial to swap for Theme
    // tokens later.
    QtObject {
        id: brand
        readonly property color red: "#F44949"     // Primary Fox Red
        readonly property color orange: "#FF6B3D"
        readonly property color yellow: "#FEC42D"  // Ear Yellow
        readonly property color cream: "#FEF7E0"   // Cream Highlight
        readonly property color white: "#FFFFFF"
    }

    // Icon geometry, as inline SVG path data. These become data URIs below, so
    // they stay sharp at any DPI and can be tinted per state.
    readonly property string pathSearch: "M15.5 14h-.79l-.28-.27C15.41 12.59 16 11.11 16 9.5 16 5.91 13.09 3 9.5 3S3 5.91 3 9.5 5.91 16 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z"
    readonly property string pathBookmark: "M6 3h12a1 1 0 0 1 1 1v17l-7-4.6L5 21V4a1 1 0 0 1 1-1z"
    readonly property string pathFilter: "M3 5h18l-7 8v6l-4 2v-8L3 5z"
    readonly property string pathDownload: "M5 20h14v-2H5v2zM19 9h-4V3H9v6H5l7 8 7-8z"

    // Build a tinted icon as an inline SVG data URI. Needs the qtsvg
    // image-format plugin at runtime (see flake.nix).
    //
    // The fill is written as "rgb(r,g,b)" rather than a hex string: a '#'
    // inside a data URI starts the fragment and can truncate the SVG before
    // the path is ever parsed.
    function svgIcon(path, c) {
        const r = Math.round(c.r * 255)
        const g = Math.round(c.g * 255)
        const b = Math.round(c.b * 255)
        return "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='rgb("
            + r + "," + g + "," + b + ")'><path d='" + path + "'/></svg>"
    }

    // A full-window backdrop that clears the search field's focus when the
    // reader clicks anywhere else.
    Item {
        id: focusBackdrop
        parent: Overlay.overlay
        anchors.fill: parent
        visible: cmdInput.activeFocus
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

    // Focus the command field from anywhere.
    Shortcut {
        sequences: ["Ctrl+K", "Cmd+K"]
        onActivated: {
            cmdInput.forceActiveFocus()
            cmdInput.selectAll()
        }
    }

    // Escape clears the search focus.
    Shortcut {
        enabled: cmdInput.activeFocus
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
                    width: 50
                    height: 50
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

                Item {
                    id: brandTextItem
                    anchors.verticalCenter: parent.verticalCenter
                    implicitWidth: brandText.implicitWidth
                    implicitHeight: brandText.implicitHeight

                    // Scale animation matching the logo hover effect
                    scale: brandMouse.containsMouse ? 1.05 : 1.0
                    Behavior on scale { NumberAnimation { duration: 150; easing.type: Easing.OutQuad } }

                    // The brand wordmark.
                    //
                    // This previously used Qt5Compat's LinearGradient +
                    // OpacityMask to fill the glyphs with a three-colour
                    // gradient. Two reasons that had to go:
                    //
                    //   1. Qt5Compat is the Qt5 compatibility layer and is
                    //      deprecated in Qt6. It is not part of the Qt kit CI
                    //      installs, which is what broke the Linux smoke run
                    //      with "module Qt5Compat.GraphicalEffects is not
                    //      installed".
                    //   2. Qt 6's Text has no `gradient` property (it existed in
                    //      Qt 5), so there is no drop-in replacement.
                    //
                    // Reproducing a masked gradient without a module means a
                    // ShaderEffect and a shader program of our own -- a lot of
                    // machinery for a 22px wordmark that already animates its
                    // scale on hover. Solid Fox Red it is; `brand.red` is the
                    // first stop of the old gradient anyway, so the mark still
                    // matches the logo.
                    Text {
                        id: brandText
                        text: "Kitsune"
                        font.pixelSize: 22
                        font.bold: true
                        color: brand.red
                    }
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

    // --- Centre: command bar --------------------------------------------
    Rectangle {
        id: cmdContainer
        anchors.centerIn: parent
        width: Math.min(560, parent.width - leftSection.width - rightSection.width - 48)
        height: 40
        radius: 12
        color: Theme.surface
        border.width: 1
        border.color: cmdInput.activeFocus
            ? Theme.accent
            : (cmdHover.hovered
                ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.5)
                : Theme.borderSubtle)

        Behavior on color { ColorAnimation { duration: 150 } }
        Behavior on border.color { ColorAnimation { duration: 150 } }

        HoverHandler { id: cmdHover }

        // A magnifier glyph drawn inline as an SVG data URI. Needs the same
        // qtsvg plugin as the crest.
        Image {
            id: cmdIcon
            anchors.left: parent.left
            anchors.leftMargin: 14
            anchors.verticalCenter: parent.verticalCenter
            width: 15
            height: 15
            source: bar.svgIcon(bar.pathSearch, Theme.inkMuted)
            sourceSize.width: 30
            sourceSize.height: 30
            smooth: true
            opacity: cmdInput.activeFocus ? 1.0 : (cmdHover.hovered ? 0.9 : 0.65)

            Behavior on opacity { NumberAnimation { duration: 150 } }
        }

        TextField {
            id: cmdInput
            anchors.left: cmdIcon.right
            anchors.leftMargin: 10
            anchors.right: cmdActions.left
            anchors.rightMargin: 8
            anchors.verticalCenter: parent.verticalCenter
            verticalAlignment: Text.AlignVCenter
            placeholderText: "Search anime, studios, genres…"
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

        // The right-hand cluster of the field: clear button, shortcut hint and
        // the Go action.
        Row {
            id: cmdActions
            anchors.right: parent.right
            anchors.rightMargin: 5
            anchors.verticalCenter: parent.verticalCenter
            spacing: 6

            // Clear button, shown once there is text.
            Rectangle {
                visible: cmdInput.text.length > 0
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
                        cmdInput.text = ""
                        cmdInput.forceActiveFocus()
                    }
                }
            }

            // Shortcut hint, shown while the box is empty and unfocused.
            Rectangle {
                visible: cmdInput.text.length === 0 && !cmdInput.activeFocus
                implicitWidth: shortcutText.implicitWidth + 8
                implicitHeight: 18
                radius: 4
                color: Theme.fieldFill
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

            // The Go action, styled as the brand gradient.
            Rectangle {
                width: 34
                height: 28
                radius: 7
                opacity: cmdInput.text.trim().length > 0 ? 1.0 : 0.45
                gradient: Gradient {
                    GradientStop { position: 0.0; color: brand.red }
                    GradientStop { position: 1.0; color: brand.orange }
                }

                Behavior on opacity { NumberAnimation { duration: 150 } }

                Text {
                    anchors.centerIn: parent
                    text: "Go"
                    color: brand.white
                    font.pixelSize: 11
                    font.bold: true
                }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    enabled: cmdInput.text.trim().length > 0
                    onClicked: {
                        const term = cmdInput.text.trim()
                        if (term.length > 0) bar.search(term)
                    }
                }
            }
        }
    }

    // --- Right: icon rail + settings -------------------------------------
    Row {
        id: rightSection
        anchors.right: parent.right
        anchors.rightMargin: Theme.pad
        anchors.verticalCenter: parent.verticalCenter
        spacing: 8

        // My List.
        Rectangle {
            id: listBtn
            width: 38
            height: 38
            radius: 12
            color: bar.activeRoute === "list" ? "transparent" : Theme.surface
            border.width: 1
            border.color: bar.activeRoute === "list" ? "transparent" : Theme.borderSubtle
            scale: listMouse.containsMouse ? 1.05 : 1.0

            Behavior on color { ColorAnimation { duration: 150 } }
            Behavior on border.color { ColorAnimation { duration: 150 } }
            Behavior on scale { NumberAnimation { duration: 150; easing.type: Easing.OutQuad } }

            // Active state: the brand gradient.
            Rectangle {
                anchors.fill: parent
                radius: parent.radius
                visible: bar.activeRoute === "list"
                gradient: Gradient {
                    GradientStop { position: 0.0; color: brand.red }
                    GradientStop { position: 1.0; color: brand.orange }
                }
            }

            Image {
                anchors.centerIn: parent
                width: 18
                height: 18
                source: bar.svgIcon(bar.pathBookmark,
                    bar.activeRoute === "list"
                        ? brand.white
                        : (listMouse.containsMouse ? Theme.ink : Theme.inkMuted))
                sourceSize.width: 36
                sourceSize.height: 36
                smooth: true
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

            ToolTip.visible: listMouse.containsMouse
            ToolTip.text: "My List"
            ToolTip.delay: 500
        }

        // Filter.
        Rectangle {
            id: filterBtn
            width: 38
            height: 38
            radius: 12
            color: bar.activeRoute === "filter" ? "transparent" : Theme.surface
            border.width: 1
            border.color: bar.activeRoute === "filter" ? "transparent" : Theme.borderSubtle
            scale: filterMouse.containsMouse ? 1.05 : 1.0

            Behavior on color { ColorAnimation { duration: 150 } }
            Behavior on border.color { ColorAnimation { duration: 150 } }
            Behavior on scale { NumberAnimation { duration: 150; easing.type: Easing.OutQuad } }

            Rectangle {
                anchors.fill: parent
                radius: parent.radius
                visible: bar.activeRoute === "filter"
                gradient: Gradient {
                    GradientStop { position: 0.0; color: brand.red }
                    GradientStop { position: 1.0; color: brand.orange }
                }
            }

            Image {
                anchors.centerIn: parent
                width: 18
                height: 18
                source: bar.svgIcon(bar.pathFilter,
                    bar.activeRoute === "filter"
                        ? brand.white
                        : (filterMouse.containsMouse ? Theme.ink : Theme.inkMuted))
                sourceSize.width: 36
                sourceSize.height: 36
                smooth: true
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

            ToolTip.visible: filterMouse.containsMouse
            ToolTip.text: "Filter"
            ToolTip.delay: 500
        }

        // Downloads, with a badge for anything waiting.
        Rectangle {
            id: downloadsBtn
            width: 38
            height: 38
            radius: 12
            color: bar.activeRoute === "downloads" ? "transparent" : Theme.surface
            border.width: 1
            border.color: bar.activeRoute === "downloads" ? "transparent" : Theme.borderSubtle
            scale: downloadsMouse.containsMouse ? 1.05 : 1.0

            Behavior on color { ColorAnimation { duration: 150 } }
            Behavior on border.color { ColorAnimation { duration: 150 } }
            Behavior on scale { NumberAnimation { duration: 150; easing.type: Easing.OutQuad } }

            Rectangle {
                anchors.fill: parent
                radius: parent.radius
                visible: bar.activeRoute === "downloads"
                gradient: Gradient {
                    GradientStop { position: 0.0; color: brand.red }
                    GradientStop { position: 1.0; color: brand.orange }
                }
            }

            Image {
                anchors.centerIn: parent
                width: 18
                height: 18
                source: bar.svgIcon(bar.pathDownload,
                    bar.activeRoute === "downloads"
                        ? brand.white
                        : (downloadsMouse.containsMouse ? Theme.ink : Theme.inkMuted))
                sourceSize.width: 36
                sourceSize.height: 36
                smooth: true
            }

            // Count badge, tucked into the top-right corner of the button.
            Rectangle {
                id: countBadge
                visible: bar.downloadCount > 0
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.rightMargin: -5
                anchors.topMargin: -5
                width: Math.max(16, countText.implicitWidth + 6)
                height: 16
                radius: 8
                color: brand.yellow
                // Ring the badge in the bar colour so it reads as punched out
                // of the button rather than floating above it.
                border.width: 2
                border.color: Theme.surfaceRaised

                Text {
                    id: countText
                    anchors.centerIn: parent
                    text: bar.downloadCount
                    color: "#17191F"
                    font.pixelSize: 10
                    font.bold: true
                }
            }

            MouseArea {
                id: downloadsMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: {
                    bar.forceActiveFocus()
                    bar.openDownloads()
                }
            }

            ToolTip.visible: downloadsMouse.containsMouse
            ToolTip.text: "Downloads"
            ToolTip.delay: 500
        }

        // Settings gear. Kept round, so it still reads as a different kind of
        // control from the navigation buttons next to it.
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
