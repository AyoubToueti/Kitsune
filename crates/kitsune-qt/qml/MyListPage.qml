// The reader's own list, mirroring the Svelte /list route: a row of status
// tabs with counts over a poster grid, each card carrying a menu to move or
// remove the entry.
//
// Signed out, it shows a sign-in banner rather than an error: "nothing to
// show" is the correct state for a reader with no account.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: page

    // Which navbar pill this page lights up. Read by the window to derive the
    // active route, so re-opening this page does not replay the slide.
    readonly property string pageRoute: "list"

    property var backend: null
    property bool signedIn: false
    property bool loading: false
    property string errorText: ""

    // The reader's whole list, as the backend returned it.
    property var entries: []
    // The active tab's status wire value, or "" for All.
    property string activeTab: ""

    signal openAnime(int id)

    // The tabs, in the order they are offered. Labels are the reader's words;
    // values are AniList's own enum members (the wire format).
    readonly property var tabs: [
        { value: "", label: "All" },
        { value: "current", label: "Watching" },
        { value: "planning", label: "Plan to Watch" },
        { value: "completed", label: "Completed" },
        { value: "paused", label: "On-Hold" },
        { value: "dropped", label: "Dropped" }
    ]

    // The statuses a card's menu offers, matching the detail page's.
    readonly property var listStatuses: [
        { value: "current", label: "Watching" },
        { value: "completed", label: "Completed" },
        { value: "paused", label: "On hold" },
        { value: "dropped", label: "Dropped" },
        { value: "planning", label: "Plan to watch" }
    ]

    function displayTitle(item) {
        const t = item.title || {}
        return t.userPreferred || t.english || t.romaji || t.native || "(untitled)"
    }

    // The entries the active tab shows. Reading `entries` and `activeTab` here
    // makes them dependencies of every binding that calls this.
    function visibleEntries() {
        if (page.activeTab === "") return page.entries
        return page.entries.filter(function (e) { return e.status === page.activeTab })
    }

    // How many entries carry `value`, or the total for "".
    function countFor(value) {
        if (value === "") return page.entries.length
        let n = 0
        for (const e of page.entries) if (e.status === value) n++
        return n
    }

    function signIn() {
        if (!backend) return
        const url = backend.beginLogin()
        if (url && url.length > 0) Qt.openUrlExternally(url)
    }

    // Optimistically drop a row, then let the backend confirm (it re-reads on
    // `listChanged`).
    function removeEntry(entryId) {
        if (!backend) return
        page.entries = page.entries.filter(function (e) { return e.entryId !== entryId })
        backend.deleteListEntry(entryId)
    }

    // Optimistically move a row between lists, then write it.
    function moveEntry(entryId, animeId, status) {
        if (!backend) return
        page.entries = page.entries.map(function (e) {
            if (e.entryId !== entryId) return e
            const copy = {}
            for (const k in e) copy[k] = e[k]
            copy.status = status
            return copy
        })
        // Progress is deliberately omitted: changing the list must not reset
        // how far the reader has got.
        backend.setListEntry(animeId, status, 0)
    }

    Connections {
        target: page.backend

        function onAuthStatusReady(signedInNow) {
            page.signedIn = signedInNow
            if (signedInNow) {
                page.loading = true
                page.backend.userList()
            } else {
                page.entries = []
                page.loading = false
            }
        }

        function onUserListReady(json) {
            page.entries = JSON.parse(json) || []
            page.loading = false
            page.errorText = ""
        }

        function onUserListFailed(message) {
            page.loading = false
            page.errorText = message
        }

        function onListChanged() {
            // Any successful write re-reads, so the grid is never stale.
            if (page.signedIn) page.backend.userList()
        }

        function onAuthError(message) {
            page.errorText = message
        }
    }

    Component.onCompleted: {
        if (backend) backend.authStatus()
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // --- Header -------------------------------------------------------
        Column {
            Layout.fillWidth: true
            Layout.leftMargin: Theme.pad
            Layout.rightMargin: Theme.pad
            Layout.topMargin: Theme.pad
            spacing: 2

            Text {
                text: "My List"
                color: Theme.ink
                font.pixelSize: 20
                font.bold: true
            }
            Text {
                text: "Manage your anime library and track progress"
                color: Theme.inkMuted
                font.pixelSize: 12
            }
        }

        // --- Signed out banner --------------------------------------------
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: !page.signedIn

            Rectangle {
                anchors.centerIn: parent
                width: 440
                height: 250
                radius: 16
                color: Theme.surfaceRaised
                border.width: 1
                border.color: Theme.borderSubtle

                Column {
                    anchors.centerIn: parent
                    width: parent.width - 48
                    spacing: 10

                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: "◎"
                        color: Theme.accent
                        font.pixelSize: 34
                    }
                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: "Sign in to sync your list"
                        color: Theme.ink
                        font.pixelSize: 16
                        font.bold: true
                    }
                    Text {
                        width: parent.width
                        horizontalAlignment: Text.AlignHCenter
                        text: "Connect your AniList account to track episodes, sync watch states, and organize your collection."
                        color: Theme.inkMuted
                        font.pixelSize: 12
                        wrapMode: Text.WordWrap
                    }

                    Item { width: 1; height: 6 }

                    Rectangle {
                        anchors.horizontalCenter: parent.horizontalCenter
                        width: 190
                        height: 40
                        radius: 10
                        color: signInMouse.containsMouse ? Theme.accentHover : Theme.accent

                        Text {
                            anchors.centerIn: parent
                            text: "Sign in with AniList"
                            color: "white"
                            font.pixelSize: 13
                            font.bold: true
                        }
                        MouseArea {
                            id: signInMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: page.signIn()
                        }
                    }
                }
            }
        }

        // --- Tabs ---------------------------------------------------------
        Flow {
            Layout.fillWidth: true
            Layout.leftMargin: Theme.pad
            Layout.rightMargin: Theme.pad
            Layout.topMargin: Theme.gap
            visible: page.signedIn
            spacing: 6

            Repeater {
                model: page.tabs

                delegate: Rectangle {
                    required property var modelData
                    readonly property bool active: page.activeTab === modelData.value
                    readonly property int count: page.countFor(modelData.value)

                    height: 34
                    radius: 10
                    color: active ? Theme.accent : Theme.surfaceRaised
                    border.width: active ? 0 : 1
                    border.color: Theme.borderSubtle
                    implicitWidth: tabRow.implicitWidth + 24

                    Row {
                        id: tabRow
                        anchors.centerIn: parent
                        spacing: 8

                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            text: modelData.label
                            color: active ? "white" : Theme.inkMuted
                            font.pixelSize: 12
                            font.bold: true
                        }
                        Rectangle {
                            anchors.verticalCenter: parent.verticalCenter
                            visible: !page.loading
                            radius: 5
                            color: active ? "#33ffffff" : Theme.surfaceHover
                            implicitWidth: countText.implicitWidth + 10
                            implicitHeight: 18

                            Text {
                                id: countText
                                anchors.centerIn: parent
                                text: String(count)
                                color: active ? "white" : Theme.inkFaint
                                font.pixelSize: 10
                                font.bold: true
                            }
                        }
                    }

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: page.activeTab = modelData.value
                    }
                }
            }
        }

        // --- Grid ---------------------------------------------------------
        GridView {
            id: grid
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: Theme.pad
            Layout.rightMargin: Theme.pad
            Layout.topMargin: Theme.gap
            Layout.bottomMargin: Theme.pad
            visible: page.signedIn
            clip: true
            cellWidth: 200
            cellHeight: 350
            model: page.visibleEntries()

            delegate: Item {
                id: cardRoot
                required property var modelData
                width: 180
                height: 336

                // Captured so the menu rows (declared below) can reach the
                // entry without a nested Repeater shadowing `modelData`.
                readonly property int entryId: modelData.entryId
                readonly property int animeId: modelData.anime.id
                readonly property string entryStatus: modelData.status

                PosterCard {
                    anchors.left: parent.left
                    width: 180
                    title: page.displayTitle(modelData.anime)
                    score: modelData.anime.averageScore || 0
                    cover: modelData.anime.coverImage || ""
                    format: modelData.anime.format || ""
                    onClicked: page.openAnime(animeId)
                }

                // ⋮ menu, top-LEFT (the card's score pill owns the top-right).
                Rectangle {
                    id: menuButton
                    x: 6
                    y: 6
                    width: 26
                    height: 26
                    radius: 13
                    color: menuMouse.containsMouse ? Theme.surfaceHover : "#cc0b0d12"
                    border.width: 1
                    border.color: Theme.borderSubtle

                    Text {
                        anchors.centerIn: parent
                        text: "⋮"
                        color: Theme.ink
                        font.pixelSize: 16
                        font.bold: true
                    }

                    MouseArea {
                        id: menuMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: itemMenu.open()
                    }
                }

                Popup {
                    id: itemMenu
                    parent: menuButton
                    x: 0
                    y: menuButton.height + 4
                    width: 176
                    padding: 6
                    modal: false
                    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

                    background: Rectangle {
                        color: Theme.surfaceRaised
                        radius: 12
                        border.width: 1
                        border.color: Theme.borderSubtle
                    }

                    // The five statuses as explicit rows. A Repeater would be
                    // tidier but its `modelData` would shadow this delegate's,
                    // hiding the entry the rows need.
                    contentItem: Column {
                        spacing: 2

                        Repeater {
                            model: page.listStatuses
                            delegate: Rectangle {
                                required property var modelData
                                width: itemMenu.width - 12
                                height: 30
                                radius: 8
                                color: optMouse.containsMouse ? Theme.surfaceHover : "transparent"

                                Text {
                                    anchors.left: parent.left
                                    anchors.leftMargin: 10
                                    anchors.verticalCenter: parent.verticalCenter
                                    // `modelData` here is the STATUS; the
                                    // entry is reached through the captured
                                    // `cardRoot` properties.
                                    text: modelData.label
                                    color: modelData.value === cardRoot.entryStatus
                                        ? Theme.accent : Theme.ink
                                    font.pixelSize: 13
                                }

                                MouseArea {
                                    id: optMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: {
                                        itemMenu.close()
                                        page.moveEntry(cardRoot.entryId,
                                            cardRoot.animeId, modelData.value)
                                    }
                                }
                            }
                        }

                        Rectangle {
                            width: itemMenu.width - 12
                            height: 1
                            color: Theme.borderSubtle
                        }

                        Rectangle {
                            width: itemMenu.width - 12
                            height: 30
                            radius: 8
                            color: rmMouse.containsMouse ? Theme.surfaceHover : "transparent"

                            Text {
                                anchors.left: parent.left
                                anchors.leftMargin: 10
                                anchors.verticalCenter: parent.verticalCenter
                                text: "Remove from list"
                                color: Theme.danger
                                font.pixelSize: 13
                            }

                            MouseArea {
                                id: rmMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    itemMenu.close()
                                    page.removeEntry(cardRoot.entryId)
                                }
                            }
                        }
                    }
                }
            }
        }

        // --- Empty / error ------------------------------------------------
        Text {
            Layout.fillWidth: true
            Layout.topMargin: 24
            Layout.leftMargin: Theme.pad
            Layout.rightMargin: Theme.pad
            visible: page.signedIn && !page.loading
                && page.visibleEntries().length === 0
            text: page.activeTab === ""
                ? "You haven't added any anime to your list yet."
                : "No titles in this list."
            color: Theme.inkMuted
            font.pixelSize: 13
            horizontalAlignment: Text.AlignHCenter
        }

        Text {
            Layout.fillWidth: true
            Layout.leftMargin: Theme.pad
            Layout.bottomMargin: Theme.pad
            visible: page.errorText !== ""
            text: page.errorText
            color: Theme.danger
            font.pixelSize: 12
        }
    }
}