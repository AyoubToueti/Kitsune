// On-demand release availability report.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Popup {
    id: popup

    property var backend: null
    property var release: null
    property string token: ""
    property bool loading: false
    property string error: ""
    property var report: null

    modal: true
    closePolicy: Popup.CloseOnEscape
    anchors.centerIn: parent
    width: Math.min(560, parent ? parent.width - 64 : 560)
    height: Math.min(500, parent ? parent.height - 64 : 500)
    padding: 0

    function check(releaseObject) {
        release = releaseObject
        report = null
        error = ""
        loading = true
        token = String(Date.now()) + "-" + Math.floor(Math.random() * 100000)
        open()
        if (backend) backend.probeRelease(token, JSON.stringify(releaseObject))
    }

    function badgeColor() {
        if (!report) return Theme.inkMuted
        if (report.badge === "green") return Theme.healthGreen
        if (report.badge === "yellow") return Theme.healthYellow
        return Theme.healthRed
    }

    function verdict() {
        if (!report) return "Checking availability"
        if (report.badge === "green") return "Healthy"
        if (report.badge === "yellow") return "Uncertain"
        return "No confirmation"
    }

    function advice() {
        if (!report) return "Asking the torrent trackers and swarm for current data."
        if (report.badge === "green") return "Metadata resolved and trackers report peers. This release should be usable."
        if (report.badge === "yellow") return "Some evidence was found, but availability is not fully confirmed."
        return "No tracker or metadata response was confirmed. Downloads may stall."
    }

    Connections {
        target: popup.backend
        function onReleaseProbeReady(resultToken, json) {
            if (resultToken !== popup.token) return
            popup.loading = false
            popup.report = JSON.parse(json)
        }
        function onReleaseProbeFailed(resultToken, message) {
            if (resultToken !== popup.token) return
            popup.loading = false
            popup.error = message
        }
    }

    background: GlassPanel {
        strong: true
        radius: 14
    }

    contentItem: ColumnLayout {
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 62
            Layout.leftMargin: 18
            Layout.rightMargin: 12
            spacing: 10

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2
                Text {
                    Layout.fillWidth: true
                    text: "Availability check"
                    color: Theme.ink
                    font.pixelSize: 15
                    font.bold: true
                    elide: Text.ElideRight
                }
                Text {
                    Layout.fillWidth: true
                    text: popup.release ? (popup.release.title || "") : ""
                    color: Theme.inkMuted
                    font.pixelSize: 10
                    elide: Text.ElideRight
                }
            }

            Text {
                text: "\u2715"
                color: closeMouse.containsMouse ? Theme.accent : Theme.inkMuted
                font.pixelSize: 14
                MouseArea {
                    id: closeMouse
                    anchors.fill: parent
                    anchors.margins: -6
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: popup.close()
                }
            }
        }

        Rectangle { Layout.fillWidth: true; height: 1; color: Theme.separator }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 18
            spacing: 12

            RowLayout {
                Layout.fillWidth: true
                spacing: 12
                Rectangle {
                    width: 14; height: 14; radius: 7
                    color: popup.badgeColor()
                }
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2
                    Text {
                        text: popup.verdict()
                        color: popup.badgeColor()
                        font.pixelSize: 16
                        font.bold: true
                    }
                    Text {
                        Layout.fillWidth: true
                        text: popup.advice()
                        color: Theme.inkMuted
                        font.pixelSize: 11
                        wrapMode: Text.WordWrap
                    }
                }
            }

            Text {
                visible: popup.loading
                text: "Checking trackers and torrent metadata\u2026"
                color: Theme.inkMuted
                font.pixelSize: 12
            }
            Text {
                visible: popup.error.length > 0
                text: popup.error
                color: Theme.danger
                font.pixelSize: 11
                wrapMode: Text.WordWrap
            }

            RowLayout {
                visible: popup.report !== null
                Layout.fillWidth: true
                spacing: 18
                Text {
                    text: "Seeders  " + ((popup.report && popup.report.probe && popup.report.probe.scrape)
                        ? popup.report.probe.scrape.seeders : "\u2014")
                    color: Theme.ink
                    font.pixelSize: 11
                }
                Text {
                    text: "Leechers  " + ((popup.report && popup.report.probe && popup.report.probe.scrape)
                        ? popup.report.probe.scrape.leechers : "\u2014")
                    color: Theme.ink
                    font.pixelSize: 11
                }
                Text {
                    text: "Trackers  " + ((popup.report && popup.report.probe && popup.report.probe.scrape)
                        ? popup.report.probe.scrape.trackerUrl : "none")
                    color: Theme.inkFaint
                    font.pixelSize: 10
                    elide: Text.ElideMiddle
                    Layout.fillWidth: true
                }
            }

            RowLayout {
                visible: popup.report !== null
                Layout.fillWidth: true
                spacing: 8
                Text {
                    text: "Metadata"
                    color: Theme.inkMuted
                    font.pixelSize: 11
                }
                Text {
                    text: popup.report && popup.report.probe && popup.report.probe.metadata
                        ? (popup.report.probe.metadata.resolved ? "Resolved" : "Not resolved")
                        : "Not checked"
                    color: popup.report && popup.report.probe && popup.report.probe.metadata
                        && popup.report.probe.metadata.resolved
                        ? Theme.healthGreen : Theme.healthRed
                    font.pixelSize: 11
                    font.bold: true
                }
            }

            Text {
                visible: popup.report !== null
                text: "The result is a snapshot. Tracker counts can change after this check."
                color: Theme.inkFaint
                font.pixelSize: 10
                wrapMode: Text.WordWrap
            }

            Item { Layout.fillHeight: true }

            RowLayout {
                Layout.fillWidth: true
                spacing: 8
                Item { Layout.fillWidth: true }
                Rectangle {
                    width: 80; height: 32; radius: 8
                    color: refreshMouse.containsMouse ? Theme.accent : "transparent"
                    border.width: 1; border.color: Theme.accent
                    Text {
                        anchors.centerIn: parent
                        text: "Refresh"
                        color: refreshMouse.containsMouse ? "white" : Theme.ink
                        font.pixelSize: 11
                    }
                    MouseArea {
                        id: refreshMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: popup.check(popup.release)
                    }
                }
            }
        }
    }
}