// On-demand release availability report.
//
// Compact "fox strip" redesign: the kitsune from ErrorState carries the
// verdict beside a dense one-strip summary, with the snapshot note and a gold
// Refresh pinned to the footer. The layout prevents reflow during the loading
// state by swapping text for skeleton placeholders in-place.
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
    width: Math.min(520, parent ? parent.width - 64 : 520)
    height: contentItem.implicitHeight
    padding: 0

    enter: Transition {
        NumberAnimation { property: "opacity"; from: 0; to: 1; duration: 180 }
        NumberAnimation { property: "scale"; from: 0.96; to: 1; duration: 240; easing.type: Easing.OutBack }
    }
    exit: Transition {
        NumberAnimation { property: "opacity"; from: 1; to: 0; duration: 140 }
    }

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
        if (popup.error.length > 0) return Theme.danger
        if (popup.loading || !popup.report) return Theme.accent
        if (popup.report.badge === "green") return Theme.healthGreen
        if (popup.report.badge === "yellow") return Theme.healthYellow
        return Theme.healthRed
    }

    // Which fox answers the door. Art only -- text and buttons live in the
    // strip beside it, so the ErrorState instance carries no copy of its own.
    readonly property string foxKind: {
        if (popup.error.length > 0) return "serverError"
        if (popup.loading || popup.report === null) return "loading"
        if (popup.report.badge === "green") return "victory"
        if (popup.report.badge === "yellow") return "rateLimited"
        return "notFound"
    }

    function verdict() {
        if (popup.error.length > 0) return "Check failed"
        if (popup.loading || !popup.report) return "Checking availability"
        if (popup.report.badge === "green") return "Healthy"
        if (popup.report.badge === "yellow") return "Uncertain"
        return "No confirmation"
    }

    function advice() {
        if (popup.error.length > 0) return popup.error
        if (popup.loading || !popup.report) return "Asking the torrent trackers and swarm for current data."
        if (popup.report.badge === "green") return "Metadata resolved and trackers report peers. This release should be usable."
        if (popup.report.badge === "yellow") return "Some evidence was found, but availability is not fully confirmed."
        return "No tracker or metadata response was confirmed. Downloads may stall."
    }

    function scrape() {
        return popup.report && popup.report.probe ? (popup.report.probe.scrape || null) : null
    }

    function seeders() {
        const s = scrape()
        return (s && s.seeders !== undefined) ? s.seeders : "\u2014"
    }

    function leechers() {
        const s = scrape()
        return (s && s.leechers !== undefined) ? s.leechers : "\u2014"
    }

    function trackerUrl() {
        const s = scrape()
        return (s && s.trackerUrl && s.trackerUrl.length > 0) ? s.trackerUrl : "none responded"
    }

    function metadataResolved() {
        return !!(popup.report && popup.report.probe && popup.report.probe.metadata
            && popup.report.probe.metadata.resolved)
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

        // --- Header ------------------------------------------------------

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

        // --- Fox strip ---------------------------------------------------
        // The fox on the left, verdict and numbers on the right.

        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 18
            Layout.rightMargin: 18
            Layout.topMargin: 16
            spacing: 14

            Item {
                Layout.preferredWidth: 118
                Layout.preferredHeight: 92

                // Soft verdict-tinted glow behind the fox.
                Rectangle {
                    anchors.centerIn: parent
                    width: 126
                    height: 88
                    radius: 44
                    color: popup.badgeColor()
                    opacity: 0.10
                }

                // Bouncing wrapper for the loading state
                Item {
                    anchors.fill: parent
                    y: 0
                    SequentialAnimation on y {
                        running: popup.loading && popup.visible
                        loops: Animation.Infinite
                        NumberAnimation { from: 0; to: -6; duration: 600; easing.type: Easing.InOutQuad }
                        NumberAnimation { from: -6; to: 0; duration: 600; easing.type: Easing.InOutQuad }
                    }

                    ErrorState {
                        anchors.fill: parent
                        kind: popup.foxKind
                        artWidth: 112
                        showText: false
                        showActions: false
                    }
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 4

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    Rectangle {
                        Layout.alignment: Qt.AlignVCenter
                        width: 8; height: 8; radius: 4
                        color: popup.badgeColor()

                        // The dot breathes while the probe is in flight.
                        SequentialAnimation on opacity {
                            running: popup.loading && popup.visible
                            loops: Animation.Infinite
                            NumberAnimation { from: 1; to: 0.3; duration: 550; easing.type: Easing.InOutQuad }
                            NumberAnimation { from: 0.3; to: 1; duration: 550; easing.type: Easing.InOutQuad }
                        }
                    }

                    Text {
                        Layout.fillWidth: true
                        text: popup.verdict()
                        color: popup.badgeColor()
                        font.pixelSize: 15
                        font.bold: true
                        elide: Text.ElideRight
                    }
                }

                Text {
                    Layout.fillWidth: true
                    text: popup.advice()
                    color: popup.error.length > 0 ? Theme.danger : Theme.inkMuted
                    font.pixelSize: 11
                    wrapMode: Text.WordWrap
                    Layout.minimumHeight: 32 // Prevents layout jump when text length changes
                }

                // --- Mini stats -------------------------------------------

                RowLayout {
                    visible: (popup.report !== null || popup.loading) && popup.error.length === 0
                    Layout.fillWidth: true
                    Layout.topMargin: 6
                    spacing: 12

                    // SEEDERS
                    ColumnLayout {
                        spacing: 1
                        Text {
                            text: "SEEDERS"
                            color: Theme.inkFaint
                            font.pixelSize: 9
                            font.bold: true
                        }
                        Item {
                            Layout.preferredWidth: 30
                            Layout.preferredHeight: 16
                            Rectangle {
                                visible: popup.loading
                                width: 24
                                height: 14
                                radius: 3
                                color: Theme.surfaceRaised
                                anchors.verticalCenter: parent.verticalCenter
                                SequentialAnimation on opacity {
                                    running: popup.loading && popup.visible
                                    loops: Animation.Infinite
                                    NumberAnimation { from: 0.4; to: 1; duration: 800 }
                                    NumberAnimation { from: 1; to: 0.4; duration: 800 }
                                }
                            }
                            Text {
                                visible: !popup.loading && popup.report !== null && popup.error.length === 0
                                text: String(popup.seeders())
                                color: popup.badgeColor()
                                font.pixelSize: 13
                                font.bold: true
                                anchors.verticalCenter: parent.verticalCenter
                            }
                        }
                    }

                    Rectangle {
                        Layout.alignment: Qt.AlignVCenter
                        width: 1; height: 16
                        color: Theme.separator
                    }

                    // LEECHERS
                    ColumnLayout {
                        spacing: 1
                        Text {
                            text: "LEECHERS"
                            color: Theme.inkFaint
                            font.pixelSize: 9
                            font.bold: true
                        }
                        Item {
                            Layout.preferredWidth: 30
                            Layout.preferredHeight: 16
                            Rectangle {
                                visible: popup.loading
                                width: 18
                                height: 14
                                radius: 3
                                color: Theme.surfaceRaised
                                anchors.verticalCenter: parent.verticalCenter
                                SequentialAnimation on opacity {
                                    running: popup.loading && popup.visible
                                    loops: Animation.Infinite
                                    NumberAnimation { from: 0.4; to: 1; duration: 800 }
                                    NumberAnimation { from: 1; to: 0.4; duration: 800 }
                                }
                            }
                            Text {
                                visible: !popup.loading && popup.report !== null && popup.error.length === 0
                                text: String(popup.leechers())
                                color: Theme.ink
                                font.pixelSize: 13
                                font.bold: true
                                anchors.verticalCenter: parent.verticalCenter
                            }
                        }
                    }

                    Rectangle {
                        Layout.alignment: Qt.AlignVCenter
                        width: 1; height: 16
                        color: Theme.separator
                    }

                    // METADATA
                    ColumnLayout {
                        spacing: 1
                        Text {
                            text: "METADATA"
                            color: Theme.inkFaint
                            font.pixelSize: 9
                            font.bold: true
                        }
                        Item {
                            Layout.preferredWidth: 60
                            Layout.preferredHeight: 16
                            Rectangle {
                                visible: popup.loading
                                width: 48
                                height: 14
                                radius: 3
                                color: Theme.surfaceRaised
                                anchors.verticalCenter: parent.verticalCenter
                                SequentialAnimation on opacity {
                                    running: popup.loading && popup.visible
                                    loops: Animation.Infinite
                                    NumberAnimation { from: 0.4; to: 1; duration: 800 }
                                    NumberAnimation { from: 1; to: 0.4; duration: 800 }
                                }
                            }
                            Text {
                                visible: !popup.loading && popup.report !== null && popup.error.length === 0
                                text: popup.metadataResolved() ? "Resolved" : "Not resolved"
                                color: {
                                    if (popup.metadataResolved()) return Theme.healthGreen
                                    if (popup.report && popup.report.badge === "yellow") return Theme.healthYellow
                                    return Theme.healthRed
                                }
                                font.pixelSize: 13
                                font.bold: true
                                anchors.verticalCenter: parent.verticalCenter
                            }
                        }
                    }

                    Item { Layout.fillWidth: true }
                }

                // --- Tracker chip -----------------------------------------

                Rectangle {
                    visible: (popup.report !== null || popup.loading) && popup.error.length === 0
                    Layout.fillWidth: true
                    Layout.topMargin: 8
                    Layout.preferredHeight: 26
                    radius: 8
                    color: Theme.surfaceRaised
                    border.width: 1
                    border.color: Theme.borderSubtle

                    // Skeleton
                    Rectangle {
                        visible: popup.loading
                        anchors.fill: parent
                        anchors.margins: 2
                        radius: 6
                        color: Theme.surface
                        SequentialAnimation on opacity {
                            running: popup.loading && popup.visible
                            loops: Animation.Infinite
                            NumberAnimation { from: 0.4; to: 1; duration: 800 }
                            NumberAnimation { from: 1; to: 0.4; duration: 800 }
                        }
                    }

                    // Text
                    Text {
                        visible: !popup.loading && popup.report !== null && popup.error.length === 0
                        anchors.fill: parent
                        anchors.leftMargin: 10
                        anchors.rightMargin: 10
                        verticalAlignment: Text.AlignVCenter
                        text: popup.trackerUrl()
                        color: Theme.inkMuted
                        font.pixelSize: 10
                        elide: Text.ElideMiddle
                    }
                }
            }
        }

        // --- Footer ------------------------------------------------------

        Rectangle { Layout.fillWidth: true; height: 1; color: Theme.separator; Layout.topMargin: 16 }

        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 18
            Layout.rightMargin: 18
            Layout.topMargin: 12
            Layout.bottomMargin: 14
            spacing: 12

            Text {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignVCenter
                text: "The result is a snapshot. Tracker counts can change after this check."
                color: Theme.inkFaint
                font.pixelSize: 10
                wrapMode: Text.WordWrap
                Layout.minimumHeight: 28
            }

            Rectangle {
                Layout.preferredWidth: 92
                Layout.preferredHeight: 34
                radius: 10
                border.width: 1
                
                // Disable hover effects while loading
                border.color: (refreshMouse.containsMouse && !popup.loading) ? "#FFD166" : "#FFC53D"
                scale: (refreshMouse.containsMouse && !popup.loading) ? 1.03 : 1
                
                // Lock/Dim the button visually while loading
                opacity: popup.loading ? 0.4 : 1.0
                Behavior on opacity {
                    NumberAnimation { duration: 150 }
                }
                Behavior on scale {
                    NumberAnimation { duration: 130; easing.type: Easing.OutBack }
                }
                
                gradient: Gradient {
                    GradientStop { position: 0; color: "#FFC53D" }
                    GradientStop { position: 1; color: "#F5A623" }
                }

                Text {
                    anchors.centerIn: parent
                    text: "\u21BB Refresh"
                    color: "#241A05"
                    font.pixelSize: 11
                    font.bold: true
                }

                MouseArea {
                    id: refreshMouse
                    anchors.fill: parent
                    // Disable interaction while loading
                    enabled: !popup.loading
                    hoverEnabled: !popup.loading
                    cursorShape: Qt.PointingHandCursor
                    onClicked: popup.check(popup.release)
                }
            }
        }
    }
}