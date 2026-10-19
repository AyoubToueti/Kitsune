// The full-screen overlay shown while a card is being dragged into a list.
//
// Three layers: the scrim with its five drop targets, the floating ghost under
// the cursor, and the confirmation toast. All driven by `DragState`, so this
// component holds no drag logic of its own -- it renders and hit-tests.
//
// It does NOT take mouse input. The card's MouseArea holds the grab for the
// whole drag, so events never reach this item; tracking arrives through
// `DragState.move`. That is also why there is no MouseArea here: adding one
// would swallow the release that ends the drag.
import QtQuick
import QtQuick.Layouts
import com.kitsune.app

Item {
    id: overlay

    // Above the page stack and the back/forward cue (z 200), below Qt's
    // `Overlay.overlay` where popups live.
    z: 300

    // The targets, in the order shown. Values are AniList's wire strings, the
    // same vocabulary `ListStatusStore` and `MyListPage` use.
    readonly property var targets: [
        { "value": "current", "label": "Watching" },
        { "value": "planning", "label": "Plan to watch" },
        { "value": "completed", "label": "Completed" },
        { "value": "paused", "label": "On hold" },
        { "value": "dropped", "label": "Dropped" }
    ]

    // The list the dragged work is ALREADY on, or "" when it is on none.
    //
    // Read once here rather than per target: every target would otherwise
    // re-evaluate the store lookup, and the targets only need to compare
    // against the same value. Recomputes automatically when `DragState.animeId`
    // changes, which is what makes a second drag show the right marker.
    readonly property string currentStatus: DragState.active
        ? ListStatusStore.statusFor(DragState.animeId)
        : ""

    // --- Hit testing ------------------------------------------------------
    //
    // Run whenever the cursor moves. `mapFromItem(null, ..)` converts the
    // window-space cursor into each target's own space, so the targets can sit
    // anywhere in the layout without the maths knowing about it.
    function refreshHover() {
        if (!DragState.active) return

        for (let i = 0; i < targetRepeater.count; i++) {
            const target = targetRepeater.itemAt(i)
            if (target === null) continue

            const local = target.mapFromItem(null, DragState.cursorX, DragState.cursorY)
            if (local.x >= 0 && local.x <= target.width
                    && local.y >= 0 && local.y <= target.height) {
                DragState.setHover(target.targetStatus)
                return
            }
        }
        DragState.setHover("")
    }

    onVisibleChanged: if (!visible) DragState.setHover("")

    Connections {
        target: DragState
        function onCursorXChanged() { overlay.refreshHover() }
        function onCursorYChanged() { overlay.refreshHover() }
    }

    // --- Scrim + targets --------------------------------------------------
    Rectangle {
        anchors.fill: parent
        visible: DragState.active
        color: Qt.rgba(0.04, 0.05, 0.07, 0.88)

        ColumnLayout {
            anchors.centerIn: parent
            spacing: 22

            Text {
                Layout.alignment: Qt.AlignHCenter
                text: "Move to list"
                // The overlay is its own dark scrim, so it uses the on-artwork
                // palette rather than the page ink, which goes near-black in
                // light mode.
                color: Theme.onArtwork
                font.pixelSize: 18
                font.bold: true
            }

            Row {
                id: targetRow
                Layout.alignment: Qt.AlignHCenter
                spacing: 16

                Repeater {
                    id: targetRepeater
                    model: overlay.targets

                    delegate: Rectangle {
                        id: target

                        required property var modelData
                        // Read by `refreshHover` -- the delegate's own idea of
                        // which status it stands for.
                        readonly property string targetStatus: modelData.value
                        readonly property bool hovered:
                            DragState.hoverStatus === modelData.value
                        // Whether the work being dragged already sits on this
                        // list. Drives the "Current" marker.
                        readonly property bool isCurrent:
                            overlay.currentStatus === modelData.value

                        width: 132
                        height: 132
                        radius: 16
                        color: hovered
                            ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.22)
                            : Qt.rgba(1, 1, 1, 0.04)
                        border.width: 2
                        border.color: hovered ? Theme.accent : Qt.rgba(1, 1, 1, 0.16)

                        // Scale on hover, anchored at the centre so the target
                        // grows in place rather than shifting the row.
                        scale: hovered ? 1.08 : 1.0
                        transformOrigin: Item.Center

                        Behavior on scale { NumberAnimation { duration: 160; easing.type: Easing.OutCubic } }
                        Behavior on color { ColorAnimation { duration: 160 } }
                        Behavior on border.color { ColorAnimation { duration: 160 } }

                        Column {
                            anchors.centerIn: parent
                            width: parent.width - 20
                            spacing: 6

                            Text {
                                width: parent.width
                                horizontalAlignment: Text.AlignHCenter
                                wrapMode: Text.WordWrap
                                // Labels only: `PlayerIcon`'s header documents
                                // that media glyphs render as colour emoji on
                                // some systems and ignore the theme.
                                text: target.modelData.label
                                color: target.hovered ? Theme.onArtwork : Theme.onArtworkMuted
                                font.pixelSize: 13
                                font.bold: true
                            }

                            // Marks the list the work is already on, so the
                            // reader can see BEFORE dropping that this target
                            // would be a no-op.
                            Rectangle {
                                anchors.horizontalCenter: parent.horizontalCenter
                                visible: target.isCurrent
                                implicitWidth: currentLabel.implicitWidth + 14
                                implicitHeight: 18
                                radius: 9
                                // Solid accent when hovered (the drop is a
                                // no-op), muted otherwise so it reads as
                                // information rather than a call to action.
                                color: target.hovered
                                    ? Theme.accent
                                    : Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.22)
                                border.width: 1
                                border.color: Theme.accent

                                Text {
                                    id: currentLabel
                                    anchors.centerIn: parent
                                    text: "Current"
                                    color: target.hovered ? Theme.onArtwork : Theme.accentHover
                                    font.pixelSize: 9
                                    font.bold: true
                                    font.letterSpacing: 0.4
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // --- Ghost ------------------------------------------------------------
    //
    // An Image rather than a live copy of the card: reparenting a GridView
    // delegate out of its view breaks every binding it had.
    Item {
        id: ghost
        visible: DragState.active && DragState.cover !== ""
        width: 120
        height: 160
        // Centred on the cursor.
        x: DragState.cursorX - width / 2
        y: DragState.cursorY - height / 2
        rotation: 4
        scale: 1.05
        opacity: 0.92

        Rectangle {
            anchors.fill: parent
            anchors.margins: -2
            radius: 12
            color: "transparent"
            border.width: 2
            border.color: Theme.accent
        }

        Image {
            anchors.fill: parent
            source: DragState.cover
            fillMode: Image.PreserveAspectCrop
            asynchronous: true
            sourceSize.width: 240
        }
    }

    // --- Toast ------------------------------------------------------------
    Rectangle {
        id: toast
        visible: DragState.toastVisible
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.margins: 24
        implicitWidth: toastLabel.implicitWidth + 36
        implicitHeight: 44
        radius: 12
        color: Theme.surfaceRaised
        border.width: 1
        border.color: Theme.accent

        // Slide and fade in, matching the mock.
        opacity: DragState.toastVisible ? 1.0 : 0.0
        transform: Translate { y: DragState.toastVisible ? 0 : 24 }
        Behavior on opacity { NumberAnimation { duration: 220 } }
        Behavior on transform { YAnimator { duration: 220; easing.type: Easing.OutCubic } }

        Text {
            id: toastLabel
            anchors.centerIn: parent
            text: DragState.toastMessage
            color: Theme.ink
            font.pixelSize: 13
            font.bold: true
        }
    }
}