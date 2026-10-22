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
    function removeEntry(animeId) {
        if (!backend) return
        page.entries = page.entries.filter(function (e) {
            return !e.anime || e.anime.id !== animeId
        })
        backend.deleteListEntry(animeId)
    }

    function moveEntry(entryId, animeId, status) {
        if (!backend) return
        page.entries = page.entries.map(function (e) {
            if (e.entryId !== entryId) return e
            const copy = Object.assign({}, e)
            copy.status = status
            return copy
        })
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
                page.loadFailed = false
                page.errorText = ""
            }
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

        function onListChanged() {
            if (page.signedIn) page.backend.userList()
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
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: !page.signedIn

            Rectangle {
                anchors.centerIn: parent
                width: 440
                height: 300
                radius: 16
                color: Theme.surfaceRaised
                border.width: 1
                border.color: Theme.borderSubtle

                ErrorState {
                    anchors.fill: parent
                    anchors.margins: 16
                    kind: "sleeping"
                    artWidth: 110
                    title: page.errorText !== "" ? "Sign-in failed" : "Sign in to sync your list"
                    message: page.errorText !== ""
                        ? "We couldn't complete the sign-in. Please try again."
                        : "Connect your AniList account to track episodes, sync watch states, and organize your collection."
                    primaryText: "Sign in with AniList"
                    onPrimaryClicked: page.signIn()
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

        // --- Search & Sort Bar --------------------------------------------
        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: Theme.pad
            Layout.rightMargin: Theme.pad
            Layout.topMargin: Theme.gap
            visible: page.signedIn && page.entries.length > 0
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
            visible: page.signedIn

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
                // Only show the grid when there is actually something to draw.
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
                        title: page.displayTitle(modelData.anime)
                        score: (modelData.anime && modelData.anime.averageScore) || 0
                        cover: (modelData.anime && modelData.anime.coverImage) || ""
                        format: (modelData.anime && modelData.anime.format) || ""
                        animeStatus: (modelData.anime && modelData.anime.status) || ""
                        onClicked: page.openAnime(animeId, modelData.anime && modelData.anime.provider ? modelData.anime.provider : "anilist")
                    }

                    Rectangle {
                        id: menuButton
                        x: (parent.width - 180) / 2 + 6
                        y: (parent.height - 336) / 2 + 6
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
                                            page.moveEntry(cardRoot.entryId, cardRoot.animeId, modelData.value)
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
                                        page.removeEntry(cardRoot.animeId)
                                    }
                                }
                            }
                        }
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