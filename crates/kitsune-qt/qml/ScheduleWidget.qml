// The home page's embedded airing schedule: a day strip (7 past + today + 21
// future) over the selected day's broadcasts. Mirrors `ScheduleWidget.svelte`.
//
// Embedded, so it does NOT scroll its own list -- the home page scrolls. Only
// the day strip scrolls horizontally.
//
// Uses `getScheduleTagged`: the home page never unmounts, so an untagged reply
// from a standalone schedule page would be mistaken for this widget's.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: widget

    property var backend: null
    // The caller's own label, so a reply can be matched to this widget.
    property string tag: "home-schedule"

    signal openAnime(int id)

    readonly property int pastDays: 7
    readonly property int futureDays: 21
    // How many of the day's broadcasts the embedded widget shows.
    readonly property int rowLimit: 6

    // `dayOffset` is relative to today (negative = past).
    property int selectedOffset: 0

    // The selected day's broadcasts, in the order the provider returned them.
    property var airing: []
    property bool loading: false
    property string errorText: ""

    ListModel { id: dayModel }

    // Sized from the inner column, so the height tracks exactly what is shown
    // (the strip, then 0..`rowLimit` rows, with the Column's own spacing). A
    // hand-computed constant under-counted the gaps and let the next section's
    // heading overlap the last row.
    implicitHeight: content.implicitHeight

    function displayTitle(item) {
        return TitlePreference.title(item)
    }

    // Unix seconds -> "hh:mm AM/PM" in the viewer's timezone.
    function formatTime(secs) {
        const d = new Date(secs * 1000)
        let h = d.getHours()
        const m = d.getMinutes()
        const ampm = h < 12 ? "AM" : "PM"
        h = h % 12
        if (h === 0) h = 12
        return (h < 10 ? "0" + h : h) + ":" + (m < 10 ? "0" + m : m) + " " + ampm
    }

    // Midnight local for the day `offset` days from today, as unix seconds.
    function dayStart(offset) {
        const d = new Date()
        d.setHours(0, 0, 0, 0)
        d.setDate(d.getDate() + offset)
        return Math.floor(d.getTime() / 1000)
    }

    function loadDay(offset) {
        if (!backend) return
        loading = true
        errorText = ""
        const from = dayStart(offset)
        backend.getScheduleTagged(tag, from, from + 24 * 3600, 50)
    }

    Connections {
        target: widget.backend

        function onTaggedScheduleReady(t, json) {
            if (t !== widget.tag) return
            widget.airing = JSON.parse(json) || []
            widget.loading = false
        }
        function onTaggedScheduleFailed(t, message) {
            if (t !== widget.tag) return
            widget.errorText = message
            widget.loading = false
        }
    }

    // Build the strip once, then load today. Every day gets a tab before any
    // request, so the strip keeps a stable width.
    Component.onCompleted: {
        for (let i = -pastDays; i <= futureDays; i++) {
            const d = new Date()
            d.setHours(0, 0, 0, 0)
            d.setDate(d.getDate() + i)
            dayModel.append({
                "offset": i,
                "weekday": Qt.locale().dayName(d.getDay(), Locale.ShortFormat),
                "shortDate": Qt.locale().toString(d, "MMM d"),
                "isToday": i === 0,
            })
        }
        loadDay(0)
    }

    Column {
        id: content
        width: parent.width
        spacing: 12

        // --- Day strip --------------------------------------------------
        ListView {
            id: strip
            width: parent.width
            height: 56
            orientation: ListView.Horizontal
            spacing: 8
            clip: true
            model: dayModel
            // Start scrolled to today, roughly centred.
            Component.onCompleted: positionViewAtIndex(widget.pastDays, ListView.Center)

            delegate: Rectangle {
                required property int offset
                required property string weekday
                required property string shortDate
                required property bool isToday

                width: 76
                height: 56
                radius: 10
                color: offset === widget.selectedOffset
                    ? Theme.accent
                    : (isToday ? Theme.surfaceHover : Theme.surfaceRaised)
                border.width: 1
                border.color: offset === widget.selectedOffset
                    ? Theme.accent : Theme.borderSubtle

                Column {
                    anchors.centerIn: parent
                    spacing: 1
                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: weekday
                        color: offset === widget.selectedOffset ? "white" : Theme.inkMuted
                        font.pixelSize: 12
                        font.bold: true
                    }
                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: shortDate
                        color: offset === widget.selectedOffset ? "#d9ffffff" : Theme.inkFaint
                        font.pixelSize: 10
                    }
                }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        widget.selectedOffset = offset
                        widget.loadDay(offset)
                    }
                }
            }
        }

        // --- The day's broadcasts ---------------------------------------
        // Placeholder rows while the day loads, so the widget reads as loading
        // rather than empty. Each mirrors a broadcast row (a fixed-width time
        // column then a title line).
        Repeater {
            model: widget.loading ? Math.min(widget.rowLimit, 6) : 0

            delegate: Item {
                width: widget.width
                height: 42

                RowLayout {
                    anchors.fill: parent
                    spacing: 16

                    Skeleton {
                        Layout.preferredWidth: 76
                        Layout.preferredHeight: 12
                        Layout.alignment: Qt.AlignVCenter
                        radius: 4
                    }
                    Skeleton {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 12
                        Layout.alignment: Qt.AlignVCenter
                        radius: 4
                    }
                }
            }
        }

        ErrorState {
            visible: widget.errorText !== ""
            width: parent.width
            compact: true
            showArt: false
            failureMessage: widget.errorText
            title: "Could not load that day"
            message: ""
            primaryText: "Retry"
            onPrimaryClicked: widget.loadDay(widget.selectedOffset)
        }

        ErrorState {
            visible: !widget.loading && widget.errorText === ""
                && widget.airing.length === 0
            width: parent.width
            compact: true
            showArt: false
            showActions: false
            kind: "sleeping"
            title: "Nothing airing this day"
            message: ""
        }

        Repeater {
            model: Math.min(widget.airing.length, widget.rowLimit)

            delegate: Rectangle {
                required property int index
                width: widget.width
                height: 42
                color: rowMouse.containsMouse ? Theme.surfaceHover : "transparent"

                RowLayout {
                    anchors.fill: parent
                    spacing: 16

                    // Time, fixed width so the titles line up.
                    Text {
                        Layout.preferredWidth: 76
                        text: widget.formatTime(widget.airing[index].airingAt)
                        color: Theme.inkFaint
                        font.pixelSize: 12
                    }

                    Text {
                        Layout.fillWidth: true
                        text: widget.displayTitle(widget.airing[index].anime)
                        color: rowMouse.containsMouse ? Theme.accent : Theme.ink
                        font.pixelSize: 14
                        elide: Text.ElideRight
                    }

                    Rectangle {
                        visible: widget.airing[index].episode !== undefined
                            && widget.airing[index].episode !== null
                        radius: 6
                        color: Theme.surfaceHover
                        implicitWidth: epText.implicitWidth + 16
                        implicitHeight: 24

                        Text {
                            id: epText
                            anchors.centerIn: parent
                            text: "Episode " + (widget.airing[index].episode || 0)
                            color: Theme.inkMuted
                            font.pixelSize: 11
                        }
                    }
                }

                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    height: 1
                    color: Theme.borderSubtle
                }

                MouseArea {
                    id: rowMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: widget.openAnime(widget.airing[index].anime.id)
                }
            }
        }
    }
}