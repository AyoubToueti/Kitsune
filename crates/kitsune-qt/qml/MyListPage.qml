// The reader's own list, mirroring the Svelte /list route: a row of status
// tabs with counts over a poster grid, each card carrying a menu to move or
// remove the entry.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: page

    readonly property string pageRoute: "list"

    property var backend: null
    property bool signedIn: false
    property bool loading: false
    property string errorText: ""

    // The list failed to load: the grid area becomes an error block rather
    // than an empty collection.
    property bool loadFailed: false

    property var entries: []
    property string activeTab: ""
    property string searchText: ""
    readonly property string normalizedSearchText: searchText.trim().toLowerCase()
    
    property string sortKey: ""
    property bool sortReversed: false

    readonly property bool canReverse: sortKey !== ""

    readonly property var sortOptions: [
        { "value": "", "label": "Default Order" },
        { "value": "added", "label": "Recently Added" },
        { "value": "title", "label": "Title" }
    ]

    // `provider` is the id's namespace, forwarded so the detail page fetches
    // from the source that issued the id.
    signal openAnime(int id, string provider)

    readonly property var tabs: [
        { value: "", label: "All" },
        { value: "current", label: "Watching" },
        { value: "planning", label: "Plan to Watch" },
        { value: "completed", label: "Completed" },
        { value: "paused", label: "On-Hold" },
        { value: "dropped", label: "Dropped" }
    ]

    readonly property var listStatuses: [
        { value: "current", label: "Watching" },
        { value: "completed", label: "Completed" },
        { value: "paused", label: "On hold" },
        { value: "dropped", label: "Dropped" },
        { value: "planning", label: "Plan to watch" }
    ]

    function displayTitle(item) {
        return TitlePreference.title(item)
    }

    readonly property var statusCounts: {
        const counts = { "": entries.length }
        for (let i = 0; i < entries.length; i++) {
            const st = entries[i].status
            if (st) counts[st] = (counts[st] || 0) + 1
        }
        return counts
    }

    function countFor(value) {
        return statusCounts[value] || 0
    }

    readonly property var filteredEntries: {
        let list = entries

        if (activeTab !== "") {
            list = list.filter(function (e) { return e.status === activeTab })
        }

        const term = normalizedSearchText
        if (term !== "") {
            list = list.filter(function (e) {
                return displayTitle(e.anime).toLowerCase().indexOf(term) !== -1
            })
        }

        if (sortKey !== "") {
            list = list.slice()
            const direction = sortReversed ? -1 : 1

            if (sortKey === "added") {
                list.sort(function (a, b) {
                    const x = a.addedAt || 0
                    const y = b.addedAt || 0
                    return (y - x) * direction
                })
            } else if (sortKey === "title") {
                list.sort(function (a, b) {
                    const x = displayTitle(a.anime).toLowerCase()
                    const y = displayTitle(b.anime).toLowerCase()
                    if (x === y) return 0
                    return (x < y ? -1 : 1) * direction
                })
            }
        }

        return list
    }

    function signIn() {
        if (!backend) return
        const url = backend.beginLogin()
        if (url && url.length > 0) Qt.openUrlExternally(url)
    }

    // `animeId`, not the list-entry id: the list is now keyed on the work, so
    // an entry that was never pushed (it has no AniList entry id at all) can
    // still be removed.
    //
    // `provider` is REQUIRED by both invokables. Omitting it threw a TypeError
    // on the arity check and dropped the write entirely -- the same defect
    // that silenced Continue Watching. The namespace is part of the record's
    // identity, so it must travel with the id.
    function removeEntry(animeId, provider) {
        if (!backend) return
        const namespace = provider || "anilist"
        page.entries = page.entries.filter(function (e) {
            return !e.anime || e.anime.id !== animeId
                || (e.anime.provider || "anilist") !== namespace
        })
        backend.deleteListEntry(animeId, namespace)
    }

    function moveEntry(entryId, animeId, status, provider) {
        if (!backend) return
        page.entries = page.entries.map(function (e) {
            if (e.entryId !== entryId) return e
            const copy = Object.assign({}, e)
            copy.status = status
            return copy
        })
        backend.setListEntry(animeId, status, 0, provider || "anilist")
    }

    Connections {
        target: page.backend

        function onAuthStatusReady(signedInNow) {
            page.signedIn = signedInNow
            // Read the list EITHER WAY. It is a local file, so a reader who
            // has never connected AniList -- or is offline -- still has one.
            // Clearing it here is what made the page claim an empty library
            // and demand a sign-in just to look at their own collection.
            page.loading = true
            page.backend.userList()
        }

        function onUserListReady(json) {
            page.entries = JSON.parse(json) || []
            page.loading = false
            page.loadFailed = false
            page.errorText = ""
            RetryState.reset()
        }

        function onUserListFailed(message) {
            page.loading = false
            page.loadFailed = true
            page.errorText = message
            RetryState.reportFailure(message)
        }

        // NOT gated on `signedIn`: the list is a local file, and a write made
        // while signed out still changes it. Gating here left a local add or
        // remove invisible until the page was left and re-entered.
        function onListChanged() {
            page.backend.userList()
        }

        function onAuthError(message) {
            page.errorText = message
        }
    }

    function reload() {
        if (!backend || !page.signedIn) return
        page.loading = true
        page.loadFailed = false
        page.errorText = ""
        backend.userList()
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
        //
        // A NOTE, not a gate. The list below is local, so a reader without an
        // account can still browse, search and organise it; signing in only
        // adds syncing, which this strip is what offers.
        Item {
            Layout.fillWidth: true
            Layout.preferredHeight: page.signedIn ? 0 : 120
            visible: !page.signedIn

            Rectangle {
                anchors.centerIn: parent
                width: parent.width - Theme.pad * 2
                height: 96
                radius: 12
                color: Theme.surfaceRaised
                border.width: 1
                border.color: Theme.borderSubtle

                Row {
                    anchors.fill: parent
                    anchors.margins: 16
                    spacing: 16

                    Column {
                        width: parent.width - signInBtn.width - 16
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 4

                        Text {
                            text: page.errorText !== "" ? "Sign-in failed" : "Not connected to AniList"
                            color: Theme.ink
                            font.pixelSize: 14
                            font.bold: true
                        }
                        Text {
                            width: parent.width
                            wrapMode: Text.WordWrap
                            text: page.errorText !== ""
                                ? "We couldn't complete the sign-in. Please try again."
                                : "Your list is saved on this device and works offline. Sign in to sync it with AniList."
                            color: Theme.inkMuted
                            font.pixelSize: 12
                        }
                    }

                    Rectangle {
                        id: signInBtn
                        anchors.verticalCenter: parent.verticalCenter
                        width: signInLabel.implicitWidth + 28
                        height: 34
                        radius: 17
                        color: signInMouse.containsMouse ? Theme.accentHover : Theme.accent

                        Behavior on color { ColorAnimation { duration: 120 } }

                        Text {
                            id: signInLabel
                            anchors.centerIn: parent
                            text: "Sign in"
                            color: "white"
                            font.pixelSize: 12
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

        // --- Search & Sort Bar --------------------------------------------
        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: Theme.pad
            Layout.rightMargin: Theme.pad
            Layout.topMargin: Theme.gap
            visible: page.entries.length > 0
            spacing: 8

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 38
                radius: 19
                color: Theme.glassFillSubtle
                border.width: 1
                border.color: listSearch.activeFocus ? Theme.accent : Theme.glassBorder

                TextInput {
                    id: listSearch
                    anchors.fill: parent
                    anchors.leftMargin: 16
                    anchors.rightMargin: 40
                    verticalAlignment: TextInput.AlignVCenter
                    color: Theme.ink
                    font.pixelSize: 12
                    selectionColor: Theme.accent
                    selectedTextColor: "white"
                    clip: true
                    onTextChanged: page.searchText = text

                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        visible: listSearch.text.length === 0
                        text: "Search your list\u2026"
                        color: Theme.inkFaint
                        font.pixelSize: 12
                    }
                }

                Item {
                    anchors.right: parent.right
                    anchors.rightMargin: 12
                    anchors.verticalCenter: parent.verticalCenter
                    width: 18
                    height: 18
                    visible: listSearch.text.length > 0

                    Text {
                        anchors.centerIn: parent
                        text: "\u2715"
                        color: clearArea.containsMouse ? Theme.ink : Theme.inkFaint
                        font.pixelSize: 12
                    }

                    MouseArea {
                        id: clearArea
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: listSearch.text = ""
                    }
                }
            }

            Rectangle {
                id: sortGroup
                Layout.preferredHeight: 38
                implicitWidth: sortInner.implicitWidth + 10
                radius: 12
                color: Theme.glassFillSubtle
                border.width: 1
                border.color: Theme.glassBorder

                Row {
                    id: sortInner
                    anchors.centerIn: parent
                    spacing: 4

                    // Reverse toggle with flip animation
                    Rectangle {
                        width: 32
                        height: 32
                        radius: 9
                        opacity: page.canReverse ? 1.0 : 0.45
                        color: reverseMouse.containsMouse ? Theme.surfaceHover : "transparent"

                        Text {
                            id: reverseIcon
                            anchors.centerIn: parent
                            text: "\u21C5"
                            color: page.sortReversed ? Theme.accent : Theme.inkMuted
                            font.pixelSize: 15
                            rotation: page.sortReversed ? 180 : 0

                            Behavior on rotation {
                                NumberAnimation {
                                    duration: 250
                                    easing.type: Easing.InOutQuad
                                }
                            }
                        }

                        MouseArea {
                            id: reverseMouse
                            anchors.fill: parent
                            enabled: page.canReverse
                            hoverEnabled: true
                            cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
                            onClicked: page.sortReversed = !page.sortReversed
                        }
                    }

                    Rectangle {
                        width: 1
                        height: 18
                        anchors.verticalCenter: parent.verticalCenter
                        color: Theme.borderSubtle
                    }

                    ThemeSelect {
                        anchors.verticalCenter: parent.verticalCenter
                        preferredWidth: 150
                        borderless: true
                        accentText: true
                        options: page.sortOptions
                        value: page.sortKey
                        onSelected: (v) => {
                            page.sortKey = v
                            if (v === "") page.sortReversed = false
                        }
                    }
                }
            }
        }

        // --- Grid View Container ------------------------------------------
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            // Error case: server error fox, message classified from page.errorText.
            ErrorState {
                anchors.fill: parent
                visible: page.loadFailed
                failureMessage: page.errorText
                retryAfter: RetryState.remaining
                onPrimaryClicked: page.reload()
            }

            PosterGridSkeleton {
                anchors.fill: parent
                anchors.leftMargin: Theme.pad
                anchors.rightMargin: Theme.pad
                anchors.topMargin: Theme.gap
                visible: page.loading && !page.loadFailed
                count: 12
                cellWidth: 200
                cellHeight: 350
            }

            GridView {
                id: grid
                anchors.fill: parent
                anchors.leftMargin: Theme.pad
                anchors.rightMargin: Theme.pad
                anchors.topMargin: Theme.gap
                anchors.bottomMargin: Theme.pad
                visible: !page.loading && !page.loadFailed && page.filteredEntries.length > 0
                clip: true
                cellWidth: 200
                cellHeight: 350
                model: page.filteredEntries

                delegate: Item {
                    id: cardRoot
                    required property var modelData
                    width: grid.cellWidth
                    height: grid.cellHeight

                    readonly property int entryId: modelData.entryId
                    readonly property int animeId: modelData.anime ? modelData.anime.id : 0
                    readonly property string entryStatus: modelData.status || ""

                    PosterCard {
                        anchors.centerIn: parent
                        width: 180
                        height: 336
                        animeId: cardRoot.animeId
                        // Without this the card's own quick-add asked the
                        // store about the AniList work sharing this number.
                        animeProvider: modelData.anime && modelData.anime.provider
                            ? modelData.anime.provider : "anilist"
                        title: page.displayTitle(modelData.anime)
                        score: (modelData.anime && modelData.anime.averageScore) || 0
                        cover: (modelData.anime && modelData.anime.coverImage) || ""
                        format: (modelData.anime && modelData.anime.format) || ""
                        animeStatus: (modelData.anime && modelData.anime.status) || ""
                        onClicked: page.openAnime(animeId, modelData.anime && modelData.anime.provider ? modelData.anime.provider : "anilist")
                    }
                }
            }

            // Empty case: notFound fox with context-aware message. Never overlaps
            // the error fox because of the `!page.loadFailed` guard.
            ErrorState {
                anchors.fill: parent
                anchors.margins: 24
                visible: !page.loading && !page.loadFailed && page.filteredEntries.length === 0
                kind: "notFound"
                artWidth: 180
                showActions: false
                title: page.normalizedSearchText !== ""
                    ? "No matches found"
                    : (page.activeTab === ""
                        ? "Nothing here yet"
                        : "Nothing in this list")
                message: page.normalizedSearchText !== ""
                    ? ("No titles match \u201c" + page.searchText.trim() + "\u201d"
                        + (page.activeTab === "" ? "." : " in this list."))
                    : (page.activeTab === ""
                        ? "You haven't added any anime to your list yet. Browse the catalog and add some titles to get started."
                        : "No titles in this list yet.")
            }
        }
    }
}