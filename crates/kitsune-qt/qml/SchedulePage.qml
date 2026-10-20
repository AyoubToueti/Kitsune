// Airing schedule: a day strip (7 past + today + 21 future) with the selected
// day's broadcasts listed below. Mirrors the Svelte ScheduleWidget.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: page

    property var backend: null

    signal openAnime(int id)

    // Day strip: `PAST` days before today, `FUTURE` after. Only the selected
    // day is fetched (AniList caps perPage at 50, so a whole window cannot
    // arrive at once).
    readonly property int pastDays: 7
    readonly property int futureDays: 21

    // `dayOffset` is relative to today (negative = past).
    property int selectedOffset: 0
    // Whether a day's broadcasts are in flight, so the list can show a skeleton
    // on the first load rather than an empty area.
    property bool loading: false
    // A day failed to load with nothing on screen to preserve: the list area
    // shows an error block instead of an empty page.
    property bool loadFailed: false
    property string pageError: ""

    ListModel { id: dayModel }
    ListModel { id: airingModel }

    Connections {
        target: page.backend
        function onScheduleReady(json) {
            const items = JSON.parse(json)
            airingModel.clear()
            for (const entry of items) {
                airingModel.append({
                    "id": entry.anime.id,
                    "titlesJson": JSON.stringify(entry.anime.title || {}),
                    "cover": entry.anime.coverImage || "",
                    "episode": entry.episode || 0,
                    "time": formatTime(entry.airingAt),
                })
            }
            page.loading = false
            page.loadFailed = false
            page.pageError = ""
            status.text = items.length + " airing"
            RetryState.reset()
        }
        function onScheduleFailed(message) {
            // With nothing on screen there is nothing to preserve, so the list
            // area becomes an error block. A day switch keeps the previous
            // day's rows, so only the empty case takes over the view.
            if (airingModel.count === 0) {
                page.loading = false
                page.loadFailed = true
                page.pageError = message
            } else {
                page.loading = false
            }
            status.text = ""
            RetryState.reportFailure(message)
        }
    }

    function displayTitle(item) {
        return TitlePreference.title(item)
    }

    // Unix seconds -> local HH:MM.
    function formatTime(secs) {
        const d = new Date(secs * 1000)
        return d.toLocaleTimeString(Qt.locale(), "HH:mm")
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
        page.loading = true
        page.loadFailed = false
        page.pageError = ""
        const from = dayStart(offset)
        const to = from + 24 * 3600
        backend.getSchedule(from, to, 50)
    }

    // Re-run the selected day. Called by the retry overlay.
    function reload() {
        loadDay(page.selectedOffset)
    }

    // Build the strip once. Every day gets a tab before any request.
    Component.onCompleted: {
        for (let i = -pastDays; i <= futureDays; i++) {
            const d = new Date()
            d.setHours(0, 0, 0, 0)
            d.setDate(d.getDate() + i)
            dayModel.append({
                "offset": i,
                "weekday": Qt.locale().dayName(d.getDay(), Locale.ShortFormat),
                "day": d.getDate(),
                "isToday": i === 0,
            })
        }
        loadDay(0)
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
                    text: "Schedule"
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

        // Day strip
        ListView {
            Layout.fillWidth: true
            Layout.preferredHeight: 76
            orientation: ListView.Horizontal
            leftMargin: Theme.pad
            rightMargin: Theme.pad
            spacing: 8
            clip: true
            model: dayModel
            // Start scrolled to today, roughly centred.
            Component.onCompleted: positionViewAtIndex(page.pastDays, ListView.Center)

            delegate: Rectangle {
                required property int offset
                required property string weekday
                required property int day
                required property bool isToday
                width: 62
                height: 62
                anchors.verticalCenter: parent ? parent.verticalCenter : undefined
                radius: 10
                color: offset === page.selectedOffset
                    ? Theme.accent
                    : (isToday ? Theme.surfaceHover : Theme.surfaceRaised)
                border.color: Theme.borderSubtle

                Column {
                    anchors.centerIn: parent
                    spacing: 2
                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: weekday
                        color: offset === page.selectedOffset ? "white" : Theme.inkFaint
                        font.pixelSize: 11
                    }
                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: day
                        color: offset === page.selectedOffset ? "white" : Theme.ink
                        font.pixelSize: 18
                        font.bold: true
                    }
                }

                MouseArea {
                    anchors.fill: parent
                    onClicked: {
                        page.selectedOffset = offset
                        page.loadDay(offset)
                    }
                }
            }
        }

        // First-load placeholder: shown until the selected day's broadcasts
        // arrive. Switching days keeps the previous day's rows visible (the
        // model clears only on the reply), so a skeleton on every load would
        // flash; it is limited to the empty first load.
        ErrorState {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: page.loadFailed
            failureMessage: page.pageError
            retryAfter: RetryState.remaining
            onPrimaryClicked: page.reload()
        }

        RankedListSkeleton {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: Theme.pad
            Layout.rightMargin: Theme.pad
            Layout.topMargin: Theme.gap
            visible: page.loading && airingModel.count === 0 && !page.loadFailed
            count: 7
        }

        // Airing list
        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: airingModel
            leftMargin: Theme.pad
            rightMargin: Theme.pad
            topMargin: Theme.gap
            bottomMargin: Theme.pad
            spacing: 10
            visible: !(page.loading && airingModel.count === 0) && !page.loadFailed

            delegate: Rectangle {
                width: ListView.view.width - Theme.pad * 2
                height: 84
                radius: Theme.radius
                color: rowHover.hovered ? Theme.surfaceHover : Theme.surfaceRaised

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 8
                    spacing: 12

                    Rectangle {
                        Layout.preferredWidth: 48
                        Layout.preferredHeight: 68
                        radius: 8
                        color: Theme.surface
                        clip: true
                        Image {
                            anchors.fill: parent
                            source: model.cover
                            fillMode: Image.PreserveAspectCrop
                            asynchronous: true
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2

                        Text {
                            Layout.fillWidth: true
                            text: TitlePreference.title(JSON.parse(model.titlesJson))
                            color: Theme.ink
                            font.pixelSize: 15
                            font.bold: true
                            elide: Text.ElideRight
                        }
                        Text {
                            text: model.episode > 0 ? ("Episode " + model.episode) : ""
                            color: Theme.inkMuted
                            font.pixelSize: 13
                        }
                    }

                    Text {
                        Layout.alignment: Qt.AlignVCenter
                        text: model.time
                        color: Theme.score
                        font.pixelSize: 14
                        font.bold: true
                    }
                }

                HoverHandler { id: rowHover }
                MouseArea {
                    anchors.fill: parent
                    onClicked: page.openAnime(model.id)
                }
            }
        }
    }
}