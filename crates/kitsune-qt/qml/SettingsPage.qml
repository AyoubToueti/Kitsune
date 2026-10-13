// The reader's preferences. Mirrors the Svelte SettingsDrawer: player, download
// dir, resolution preference, min seeders, auto-launch threshold, theme.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: page

    property var backend: null
    // The loaded settings object, kept locally while editing.
    property var current: null

    signal back()

    Connections {
        target: page.backend
        function onSettingsReady(json) {
            page.current = JSON.parse(json)
            // Reflect into the controls.
            if (page.current) {
                playerField.text = page.current.player || ""
                downloadField.text = page.current.download_dir || ""
                seedersBox.value = page.current.min_seeders || 0
                fractionSlider.value = page.current.ready_fraction || 0.05
                askSwitch.checked = page.current.ask_every_time !== false
            }
            status.text = "Loaded"
        }
        function onSettingsFailed(message) {
            status.text = "Failed: " + message
        }
    }

    Component.onCompleted: {
        if (backend) backend.getSettings()
    }

    // Save the edited fields back as a whole Settings object.
    function save() {
        if (!page.current) return
        const s = page.current
        s.player = playerField.text
        s.download_dir = downloadField.text.length > 0 ? downloadField.text : null
        s.min_seeders = Math.round(seedersBox.value)
        s.ready_fraction = fractionSlider.value
        s.ask_every_time = askSwitch.checked
        backend.setSettings(JSON.stringify(s))
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // Header
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 64
            color: Theme.surface

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Theme.pad
                anchors.rightMargin: Theme.pad

                Text {
                    text: "Settings"
                    color: Theme.accent
                    font.pixelSize: 24
                    font.bold: true
                }
                Text {
                    id: status
                    Layout.fillWidth: true
                    color: Theme.inkFaint
                    font.pixelSize: 13
                }
            }
        }

        Flickable {
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: width
            contentHeight: form.height + Theme.gap + Theme.pad
            clip: true

            ColumnLayout {
                id: form
                width: parent.width - Theme.pad * 2
                x: Theme.pad
                y: Theme.gap
                spacing: 18

                // External player
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 6
                    Text { text: "External player"; color: Theme.ink; font.pixelSize: 14; font.bold: true }
                    TextField {
                        id: playerField
                        Layout.fillWidth: true
                        color: Theme.ink
                        placeholderTextColor: Theme.inkFaint
                        placeholderText: "mpv"
                        background: Rectangle { radius: 8; color: Theme.surfaceRaised; border.color: Theme.borderSubtle }
                    }
                }

                // Download directory
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 6
                    Text { text: "Download directory"; color: Theme.ink; font.pixelSize: 14; font.bold: true }
                    TextField {
                        id: downloadField
                        Layout.fillWidth: true
                        color: Theme.ink
                        placeholderTextColor: Theme.inkFaint
                        placeholderText: "(system temp directory)"
                        background: Rectangle { radius: 8; color: Theme.surfaceRaised; border.color: Theme.borderSubtle }
                    }
                }

                // Minimum seeders
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 6
                    Text { text: "Minimum seeders"; color: Theme.ink; font.pixelSize: 14; font.bold: true }
                    SpinBox {
                        id: seedersBox
                        from: 0
                        to: 1000
                    }
                }

                // Auto-launch threshold
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 6
                    Text {
                        text: "Auto-launch threshold: " + Math.round(fractionSlider.value * 100) + "%"
                        color: Theme.ink
                        font.pixelSize: 14
                        font.bold: true
                    }
                    Slider {
                        id: fractionSlider
                        Layout.fillWidth: true
                        from: 0.01
                        to: 1.0
                        stepSize: 0.01
                    }
                }

                // Ask every time
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 10
                    Switch { id: askSwitch }
                    Text { text: "Ask which player each time"; color: Theme.ink; font.pixelSize: 14 }
                }

                NavButton {
                    label: "Save"
                    onClicked: page.save()
                }
            }
        }
    }
}