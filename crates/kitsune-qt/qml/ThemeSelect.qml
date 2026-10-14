// A themed dropdown. Native QML `ComboBox` popups are drawn by the platform
// and cannot be styled, so this renders its own listbox with the app's surface
// and accent -- mirroring `Select.svelte`.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    // `options` is a list of { value, label } objects.
    property var options: []
    property string value: ""
    property string placeholder: ""
    property int preferredWidth: 150
    // When true the trigger draws no border/background of its own, so it can
    // sit inside a container that owns the border (the filter page's sort
    // group, which wraps a reverse button and this select in one pill).
    property bool borderless: false
    // When true the chosen label is drawn in the accent colour, as the filter
    // page's sort control does.
    property bool accentText: false

    signal selected(string value)

    implicitWidth: preferredWidth
    implicitHeight: 36

    function currentLabel() {
        for (let i = 0; i < options.length; i++) {
            if (options[i].value === value) return options[i].label
        }
        if (placeholder.length > 0) return placeholder
        return options.length > 0 ? options[0].label : ""
    }

    Rectangle {
        id: trigger
        anchors.fill: parent
        radius: root.borderless ? 0 : 18
        color: root.borderless ? "transparent" : Theme.surfaceHover
        border.width: root.borderless ? 0 : 1
        border.color: popup.visible ? Theme.accent : Theme.borderSubtle

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 14
            anchors.rightMargin: 12
            spacing: 6

            Text {
                Layout.fillWidth: true
                text: root.currentLabel()
                color: root.accentText ? Theme.accent : Theme.ink
                font.pixelSize: 13
                font.bold: root.accentText
                elide: Text.ElideRight
            }
            Text {
                text: popup.visible ? "▴" : "▾"
                color: root.accentText ? Theme.accent : Theme.inkFaint
                font.pixelSize: 11
            }
        }

        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: popup.visible ? popup.close() : popup.open()
        }
    }

    Popup {
        id: popup
        y: trigger.height + 4
        width: Math.max(trigger.width, 170)
        padding: 4
        modal: false
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

        background: Rectangle {
            radius: 10
            color: Theme.surfaceRaised
            border.width: 1
            border.color: Theme.borderSubtle
        }

        contentItem: ListView {
            implicitHeight: Math.min(contentHeight, 300)
            model: root.options
            clip: true

            delegate: Rectangle {
                required property var modelData

                width: popup.availableWidth
                height: 34
                radius: 6
                color: rowMouse.containsMouse ? Theme.surfaceHover : "transparent"

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.left: parent.left
                    anchors.leftMargin: 10
                    text: modelData.label
                    color: modelData.value === root.value ? Theme.accent : Theme.ink
                    font.pixelSize: 13
                    font.bold: modelData.value === root.value
                }

                MouseArea {
                    id: rowMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        root.value = modelData.value
                        root.selected(modelData.value)
                        popup.close()
                    }
                }
            }
        }
    }
}