// DownloadsPage.qml
// Permanently downloaded episodes, grouped by anime and then by torrent.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Effects
import com.kitsune.app

Item {
    id: page

    readonly property string pageRoute: "downloads"

    property var backend: null
    property var records: []
    property string activeFilter: "all"
    property string sortMode: "status"     // status | added | finished | name | size
    property bool sortReversed: false
    property bool groupByAnime: true
    property string searchQuery: ""
    property string libraryPath: ""
    property var selectedIds: ({})
    property var expandedGroups: ({})
    property var expandedAnime: ({})

    // Tree-line layout (grouped mode) + sticky header backdrop.
    // Theme-aware properties:
    readonly property color stickyBackdrop: Theme.surface
    readonly property int treeTrunkX: 28
    readonly property int treeGutter: 48
    readonly property int cardGap: 10
    readonly property color treeLineColor: Theme.dark ? Qt.rgba(1, 1, 1, 0.16) : Qt.rgba(0, 0, 0, 0.08)

    // Live progress snapshots, refreshed every poll.
    property var liveGroups: ({})
    property var liveRecords: ({})

    function asRecords(value) {
        if (Array.isArray(value)) return value
        if (value && typeof value === "object") return [value]
        return []
    }

    function structuralSignature(arr) {
        const parts = []
        for (let i = 0; i < arr.length; i++) {
            const r = arr[i]
            parts.push(String(r.id)
                + "|" + (r.status || "")
                + "|" + (r.priority || "")
                + "|" + (r.fileCount || 0)
                + "|" + (r.restoring ? 1 : 0)
                + "|" + (r.error || "")
                + "|" + (r.title || "")
                + "|" + (r.coverImage ? 1 : 0)
                + "|" + (r.episode || 0)
                + "|" + (r.fileIdx || 0)
                + "|" + (r.totalBytes || 0))
        }
        parts.sort()
        return parts.join(";")
    }

    function toTimestamp(v) {
        if (!v) return 0
        if (typeof v === "number") return v > 0 ? v : 0
        const t = Date.parse(v)
        return isNaN(t) ? 0 : t
    }

    function groupKeyOf(record) {
        return String(record.groupKey
            || record.infoHash
            || ("record:" + record.id))
    }

    function animeKeyOf(record) {
        const id = record.animeId || 0
        if (id > 0)
            return String(record.provider || "unknown").toLowerCase() + ":" + id
        return "title:" + String(record.title || "unknown").trim().toLowerCase()
    }

    function computeLiveGroups(arr) {
        const map = ({})
        for (let i = 0; i < arr.length; i++) {
            const r = arr[i]
            const key = groupKeyOf(r)
            let g = map[key]
            if (!g) {
                g = map[key] = {
                    progressBytes: 0, totalBytes: 0,
                    speedMbps: 0, peersLive: 0, etaSeconds: 0,
                    error: "", restoring: false,
                    downloading: false, queued: false, paused: false,
                    failedCount: 0, status: "complete",
                    fraction: 0, percent: 0
                }
            }
            if (r.status === "downloading") g.downloading = true
            if (r.status === "queued") g.queued = true
            if (r.status === "paused") g.paused = true
            if (r.status === "failed") g.failedCount += 1
            if (r.restoring) g.restoring = true
            g.progressBytes += r.progressBytes || 0
            g.totalBytes += r.totalBytes || 0
            g.speedMbps = Math.max(g.speedMbps, r.speedMbps || 0)
            g.peersLive = Math.max(g.peersLive, r.peersLive || 0)
            if ((r.etaSeconds || 0) > g.etaSeconds) g.etaSeconds = r.etaSeconds
            if (!g.error && r.error) g.error = r.error
        }
        for (const key in map) {
            const g = map[key]
            if (g.downloading) g.status = "downloading"
            else if (g.queued) g.status = "queued"
            else if (g.paused) g.status = "paused"
            else if (g.failedCount > 0) g.status = "failed"
            else g.status = "complete"
            g.fraction = g.status === "complete"
                ? 1
                : (g.totalBytes > 0
                    ? Math.min(Math.max(g.progressBytes / g.totalBytes, 0), 1)
                    : 0)
            g.percent = Math.round(g.fraction * 100)
        }
        return map
    }

    function computeLiveRecords(arr) {
        const map = ({})
        for (let i = 0; i < arr.length; i++) {
            const r = arr[i]
            const rec = {
                status: r.status || "",
                progressBytes: r.progressBytes || 0,
                totalBytes: r.totalBytes || 0,
                fraction: 0,
                percent: 0
            }
            rec.fraction = rec.status === "complete"
                ? 1
                : (rec.totalBytes > 0
                    ? Math.min(Math.max(rec.progressBytes / rec.totalBytes, 0), 1)
                    : 0)
            rec.percent = Math.round(rec.fraction * 100)
            map[r.id] = rec
        }
        return map
    }

    function replaceRecords(value) {
        const next = asRecords(value)
        const newSig = structuralSignature(next)
        const oldSig = structuralSignature(records)

        liveGroups = computeLiveGroups(next)
        liveRecords = computeLiveRecords(next)

        if (newSig === oldSig) return

        preserveScroll()
        records = next
    }

    function preserveScroll() {
        const y = list.contentY
        Qt.callLater(function () {
            if (list.moving || list.flicking) return
            const maxY = Math.max(0, list.contentHeight - list.height)
            list.contentY = Math.min(y, maxY)
        })
    }

    function groupKey(record) {
        return groupKeyOf(record)
    }

    readonly property var groups: {
        const map = ({})
        const order = []
        for (let i = 0; i < records.length; i++) {
            const record = records[i]
            const key = groupKeyOf(record)
            if (!map[key]) {
                map[key] = {
                    "key": key,
                    "animeKey": animeKeyOf(record),
                    "records": [],
                    "title": record.title || "Unknown anime",
                    "releaseTitle": record.releaseTitle || "",
                    "coverImage": record.coverImage || "",
                    "priority": record.priority || "normal",
                    "searchText": ""
                }
                order.push(key)
            }
            map[key].records.push(record)
        }

        const out = []
        for (let k = 0; k < order.length; k++) {
            const group = map[order[k]]
            group.records.sort(function (a, b) {
                return (a.episode || 0) - (b.episode || 0)
                    || (a.fileIdx || 0) - (b.fileIdx || 0)
            })
            aggregate(group)
            out.push(group)
        }
        return out
    }

    function aggregate(group) {
        const rows = group.records
        let downloading = false
        let queued = false
        let paused = false
        let failed = 0
        let complete = 0
        let progressBytes = 0
        let totalBytes = 0
        let firstLiveId = null
        let firstCompleteId = null
        let addedAt = 0
        let finishedAt = 0

        const searchParts = [group.title || "", group.releaseTitle || ""]

        for (let i = 0; i < rows.length; i++) {
            const r = rows[i]
            if (r.status === "downloading") downloading = true
            if (r.status === "queued") queued = true
            if (r.status === "paused") paused = true
            if (r.status === "failed") failed += 1
            if (r.status === "complete") {
                complete += 1
                if (firstCompleteId === null) firstCompleteId = r.id
            } else if (firstLiveId === null) {
                firstLiveId = r.id
            }

            progressBytes += r.progressBytes || 0
            totalBytes += r.totalBytes || 0

            const a = toTimestamp(r.addedAt || r.addedDate
                || r.createdAt || r.timestamp)
            if (a > 0 && (addedAt === 0 || a < addedAt)) addedAt = a
            if (r.status === "complete") {
                const f = toTimestamp(r.finishedAt || r.finishedDate
                    || r.completedAt || r.completedDate)
                if (f > finishedAt) finishedAt = f
            }

            if (r.episodeTitle) searchParts.push(r.episodeTitle)
            if (r.fileName) searchParts.push(r.fileName)
            if (r.episode) {
                searchParts.push("episode " + r.episode)
                searchParts.push("ep " + r.episode)
            }
        }

        group.downloading = downloading
        group.queued = queued
        group.paused = paused
        group.failedCount = failed
        group.completeCount = complete
        group.progressBytes = progressBytes
        group.totalBytes = totalBytes
        group.firstLiveId = firstLiveId
        group.firstCompleteId = firstCompleteId
        group.actionId = firstLiveId !== null ? firstLiveId : rows[0].id
        group.priority = rows[0].priority || "normal"
        group.fileCount = Math.max(rows[0].fileCount || 0, rows.length)
        group.addedAt = addedAt
        group.finishedAt = finishedAt
        group.searchText = searchParts.join(" ").toLowerCase()

        if (downloading) group.status = "downloading"
        else if (queued) group.status = "queued"
        else if (paused) group.status = "paused"
        else if (failed > 0) group.status = "failed"
        else group.status = "complete"
    }

    function statusRank(status) {
        switch (status) {
        case "downloading": return 0
        case "queued":      return 1
        case "paused":      return 2
        case "failed":      return 3
        case "complete":    return 4
        default:            return 5
        }
    }

    readonly property bool sortIsAscending: {
        const defaultAscending = (sortMode === "status" || sortMode === "name")
        return sortReversed ? !defaultAscending : defaultAscending
    }

    function baseCompareAscending(a, b) {
        switch (sortMode) {
        case "status": {
            const rankDiff = statusRank(a.status) - statusRank(b.status)
            if (rankDiff !== 0) return rankDiff
            return (a.addedAt || 0) - (b.addedAt || 0)
        }
        case "added":
            return (a.addedAt || 0) - (b.addedAt || 0)
        case "finished":
            return (a.finishedAt || 0) - (b.finishedAt || 0)
        case "name":
            return (a.title || "").localeCompare(b.title || "")
        case "size":
            return (a.totalBytes || 0) - (b.totalBytes || 0)
        default:
            return 0
        }
    }

    function compareGroups(a, b) {
        const base = baseCompareAscending(a, b)
        return sortIsAscending ? base : -base
    }

    function applySort(mode) {
        if (sortMode === mode) {
            sortReversed = !sortReversed
        } else {
            sortMode = mode
            sortReversed = false
        }
    }

    readonly property var filtered: {
        let result
        if (activeFilter === "all") {
            result = groups.slice()
        } else {
            result = groups.filter(function (g) { return g.status === activeFilter })
        }

        const q = searchQuery.trim().toLowerCase()
        if (q.length > 0) {
            result = result.filter(function (g) {
                return g.searchText.indexOf(q) >= 0
            })
        }

        if (groupByAnime) {
            result.sort(compareGroups)
            const animeRank = ({})
            let rank = 0
            for (let i = 0; i < result.length; i++) {
                const key = result[i].animeKey
                if (animeRank[key] === undefined) {
                    animeRank[key] = rank
                    rank += 1
                }
            }
            result.sort(function (a, b) {
                const diff = animeRank[a.animeKey] - animeRank[b.animeKey]
                return diff !== 0 ? diff : compareGroups(a, b)
            })
        } else {
            result.sort(compareGroups)
        }
        return result
    }

    readonly property var animeMeta: {
        const map = ({})
        for (let i = 0; i < filtered.length; i++) {
            const g = filtered[i]
            let m = map[g.animeKey]
            if (!m) {
                const anchor = (g.records && g.records.length > 0) ? g.records[0] : null
                m = map[g.animeKey] = {
                    key: g.animeKey,
                    title: g.title || "Unknown anime",
                    coverImage: g.coverImage || "",
                    animeId: anchor ? (anchor.animeId || 0) : 0,
                    provider: anchor ? (anchor.provider || "anilist") : "anilist",
                    count: 0,
                    downloading: 0, queued: 0, paused: 0,
                    failed: 0, completeCount: 0, activeCount: 0,
                    progressBytes: 0, totalBytes: 0,
                    status: "complete", fraction: 0
                }
            }
            m.count += 1
            if (g.status === "downloading") m.downloading += 1
            else if (g.status === "queued") m.queued += 1
            else if (g.status === "paused") m.paused += 1
            else if (g.status === "failed") m.failed += 1
            else if (g.status === "complete") m.completeCount += 1
            m.progressBytes += g.progressBytes || 0
            m.totalBytes += g.totalBytes || 0
        }
        for (const key in map) {
            const m = map[key]
            m.activeCount = m.count - m.completeCount
            if (m.downloading > 0) m.status = "downloading"
            else if (m.queued > 0) m.status = "queued"
            else if (m.paused > 0) m.status = "paused"
            else if (m.failed > 0) m.status = "failed"
            else m.status = "complete"
            m.fraction = m.status === "complete"
                ? 1
                : (m.totalBytes > 0
                    ? Math.min(Math.max(m.progressBytes / m.totalBytes, 0), 1)
                    : 0)
        }
        return map
    }

    function animeCount(gs) {
        const keys = ({})
        for (let i = 0; i < gs.length; i++) keys[gs[i].animeKey] = true
        return Object.keys(keys).length
    }

    function animeIsOpen(key) {
        if (!groupByAnime) return true
        if (expandedAnime[key] !== undefined) return expandedAnime[key]
        if (activeFilter !== "all" || searchQuery.trim().length > 0) return true
        if (Object.keys(animeMeta).length <= 1) return true
        const meta = animeMeta[key]
        return meta ? meta.activeCount > 0 : true
    }

    function toggleAnime(key) {
        const next = Object.assign({}, expandedAnime)
        next[key] = !animeIsOpen(key)
        preserveScroll()
        expandedAnime = next
    }

    readonly property bool anyActive: {
        for (let i = 0; i < groups.length; i++) {
            if (groups[i].status === "downloading"
                    || groups[i].status === "queued") return true
        }
        return false
    }

    readonly property var filters: [
        { value: "all", label: "All" },
        { value: "downloading", label: "Downloading" },
        { value: "queued", label: "Queued" },
        { value: "paused", label: "Paused" },
        { value: "complete", label: "Completed" },
        { value: "failed", label: "Failed" }
    ]

    readonly property var sortModes: [
        { value: "status",   label: "Status" },
        { value: "added",    label: "Added" },
        { value: "finished", label: "Finished" },
        { value: "name",     label: "Name" },
        { value: "size",     label: "Size" }
    ]

    function countFor(value) {
        if (value === "all") return groups.length
        let n = 0
        for (let i = 0; i < groups.length; i++)
            if (groups[i].status === value) n += 1
        return n
    }

    function groupFor(key) {
        for (let i = 0; i < groups.length; i++)
            if (groups[i].key === key) return groups[i]
        return null
    }

    function label(record) {
        const ep = record.episode ? ("Episode " + record.episode) : "Episode"
        if (record.episodeTitle && record.episodeTitle.length > 0)
            return ep + " \u00b7 " + record.episodeTitle
        return ep
    }

    function groupLabel(group) {
        if (group.records.length === 1) return label(group.records[0])
        const episodes = []
        for (const r of group.records)
            if (r.episode) episodes.push(r.episode)
        if (episodes.length > 1) {
            if (episodes[0] === episodes[episodes.length - 1])
                return "Episode " + episodes[0]
            return "Episodes " + episodes[0] + "\u2013" + episodes[episodes.length - 1]
        }
        return group.records.length + " episode files"
    }

    function fileSummary(group) {
        const noun = group.fileCount === 1 ? "file" : "files"
        let text = group.fileCount + " " + noun
        if (group.records.length > 1 || group.completeCount > 0)
            text += " \u00b7 " + group.completeCount + " of "
                + group.records.length + " selected done"
        return text
    }

    function formatSize(bytes) {
        if (!bytes || bytes <= 0) return "\u2014"
        const units = ["B", "KiB", "MiB", "GiB", "TiB"]
        let v = bytes, u = 0
        while (v >= 1024 && u < units.length - 1) { v /= 1024; u++ }
        return v.toFixed(v < 10 && u > 0 ? 1 : 0) + " " + units[u]
    }

    function formatSpeed(mbps) {
        if (!mbps || mbps <= 0) return "\u2014"
        return mbps.toFixed(1) + " MiB/s"
    }

    function formatEta(seconds) {
        if (seconds === null || seconds === undefined || seconds <= 0) return "\u2014"
        const h = Math.floor(seconds / 3600)
        const m = Math.floor((seconds % 3600) / 60)
        const s = Math.floor(seconds % 60)
        if (h > 0) return "ETA " + h + "h " + m + "m"
        if (m > 0) return "ETA " + m + "m " + s + "s"
        return "ETA " + s + "s"
    }

    function statusLabelFor(status) {
        switch (status) {
        case "downloading": return "Downloading"
        case "queued": return "Queued"
        case "paused": return "Paused"
        case "complete": return "Completed"
        case "failed": return "Failed"
        default: return status
        }
    }

    function statusColorFor(status) {
        switch (status) {
        case "complete": return Theme.score
        case "failed": return Theme.danger
        case "paused": return Theme.healthYellow
        default: return Theme.accent
        }
    }

    function foxKindFor(status) {
        switch (status) {
        case "downloading": return "running"
        case "queued": return "running"
        case "paused": return "sleeping"
        case "failed": return "serverError"
        case "complete": return "victory"
        default: return "sleeping"
        }
    }

    function fillColorFor(status) {
        switch (status) {
        case "complete": return Theme.score
        case "failed": return Theme.danger
        case "paused": return Theme.healthYellow
        default: return Theme.accent
        }
    }

    function priorityLabelFor(priority) {
        switch (priority) {
        case "high": return "High"
        case "low": return "Low"
        default: return "Normal"
        }
    }

    function toggleSelected(key) {
        const next = Object.assign({}, selectedIds)
        if (next[key]) delete next[key]
        else next[key] = true
        selectedIds = next
    }

    function toggleExpanded(key) {
        const next = Object.assign({}, expandedGroups)
        if (next[key]) delete next[key]
        else next[key] = true
        expandedGroups = next
    }

    function selectedList() {
        return Object.keys(selectedIds)
    }

    function selectedGroups() {
        const out = []
        for (const key of selectedList()) {
            const group = groupFor(key)
            if (group) out.push(group)
        }
        return out
    }

    readonly property bool allVisibleSelected: {
        const items = filtered
        if (items.length === 0) return false
        for (let i = 0; i < items.length; i++) {
            if (selectedIds[items[i].key] !== true) return false
        }
        return true
    }

    function toggleSelectAllVisible() {
        const items = filtered
        if (items.length === 0) return
        const next = Object.assign({}, selectedIds)
        if (allVisibleSelected) {
            for (let i = 0; i < items.length; i++) delete next[items[i].key]
        } else {
            for (let i = 0; i < items.length; i++) next[items[i].key] = true
        }
        selectedIds = next
    }

    readonly property bool bulkHasPausable: {
        for (const key of Object.keys(selectedIds)) {
            const g = groupFor(key)
            if (!g) continue
            if (g.status === "downloading" || g.status === "queued") return true
        }
        return false
    }

    readonly property bool bulkHasResumable: {
        for (const key of Object.keys(selectedIds)) {
            const g = groupFor(key)
            if (!g) continue
            if (g.status === "paused") return true
        }
        return false
    }

    function bulkPause() {
        if (!backend) return
        for (const group of selectedGroups()) {
            if (group.status === "downloading" || group.status === "queued")
                backend.pauseDownload(String(group.actionId))
        }
        selectedIds = ({})
    }

    function bulkResume() {
        if (!backend) return
        for (const group of selectedGroups()) {
            if (group.status === "paused")
                backend.resumeDownload(String(group.actionId))
        }
        selectedIds = ({})
    }

    function bulkPriority(priority) {
        if (!backend) return
        for (const group of selectedGroups())
            backend.setDownloadPriority(String(group.actionId), priority)
    }

    function bulkRemove() {
        if (!backend) return
        for (const group of selectedGroups())
            backend.removeDownload(String(group.actionId), group.completeCount > 0)
        selectedIds = ({})
    }

    function cyclePriority(group) {
        if (!backend) return
        const next = group.priority === "high"
            ? "normal" : (group.priority === "normal" ? "low" : "high")
        backend.setDownloadPriority(String(group.actionId), next)
    }

    function play(record) {
        openAnimeDownloads(record, record.id)
    }

    function playGroup(group) {
        if (!group.firstCompleteId) return
        openAnimeDownloads(group.records[0], group.firstCompleteId)
    }

    function openAnimeDownloads(anchor, selectedId) {
        if (!backend || !anchor) return
        const animeKey = animeKeyOf(anchor)
        const rows = []
        for (let i = 0; i < groups.length; i++) {
            const g = groups[i]
            if (g.animeKey !== animeKey) continue
            for (let j = 0; j < g.records.length; j++) {
                const r = g.records[j]
                if (r.status !== "complete") continue
                rows.push({
                    recordId: r.id,
                    name: r.fileName || "",
                    episode: r.episode || 0,
                    episodeLabel: r.episode
                        ? label(r)
                        : (r.episodeTitle || r.releaseTitle || r.fileName
                            || "Downloaded file"),
                    lengthBytes: r.totalBytes || 0,
                    releaseTitle: r.releaseTitle || ""
                })
            }
        }
        if (rows.length === 0) return
        rows.sort(function (a, b) {
            return (a.episode || 0) - (b.episode || 0)
                || String(a.name).localeCompare(String(b.name))
        })
        for (let i = 0; i < rows.length; i++) rows[i].idx = i

        openDownloads(JSON.stringify({
            animeId: anchor.animeId || 0,
            provider: anchor.provider || "anilist",
            title: anchor.title || "Downloads",
            coverImage: anchor.coverImage || "",
            selectedId: selectedId || rows[0].recordId,
            files: rows
        }))
    }

    function openGroupFolder(group) {
        if (!backend || !group.firstCompleteId) return
        backend.openDownloadFolder(String(group.firstCompleteId))
    }

    function togglePause(group, status) {
        if (!backend) return
        if (status === "paused") backend.resumeDownload(String(group.actionId))
        else backend.pauseDownload(String(group.actionId))
    }

    function manageFiles(group) {
        if (!backend) return
        filesDialog.openFor(group.key, group.releaseTitle || group.title)
    }

    signal openDownloads(string payload)
    signal openAnime(int id, string provider)

    Component.onCompleted: {
        if (!backend) return
        libraryPath = backend.libraryDir()
        backend.restoreDownloads()
        backend.getDownloads()
        backend.pollDownloads()
    }

    onSearchQueryChanged: preserveScroll()
    onSortModeChanged: preserveScroll()
    onSortReversedChanged: preserveScroll()
    onGroupByAnimeChanged: preserveScroll()
    onActiveFilterChanged: preserveScroll()

    Connections {
        target: page.backend
        function onDownloadsReady(json) {
            page.replaceRecords(JSON.parse(json))
            if (page.backend) page.libraryPath = page.backend.libraryDir()
        }
        function onDownloadProgress(json) {
            page.replaceRecords(JSON.parse(json))
        }
        function onDownloadCompleted(json) {
            const record = JSON.parse(json)
            if (record !== null) page.backend.pollDownloads()
        }
        function onDownloadStarted(json) {
            page.backend.getDownloads()
        }
        function onEpisodeDownloadFailed(id, message) {
            page.errorText = message
        }
    }

    property string errorText: ""

    Timer {
        interval: 1000
        repeat: true
        running: page.anyActive
        onTriggered: if (page.backend) page.backend.pollDownloads()
    }

    DownloadFilesDialog {
        id: filesDialog
        backend: page.backend
        onFilesApplied: if (page.backend) page.backend.getDownloads()
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Theme.pad
        spacing: 14

        // ---- Page header ----
        RowLayout {
            Layout.fillWidth: true
            spacing: 16

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 3
                Text {
                    text: "Downloads"
                    color: Theme.ink
                    font.pixelSize: 27
                    font.weight: Font.DemiBold
                }
                Text {
                    Layout.fillWidth: true
                    text: {
                        const total = page.groups.length
                        const shown = page.filtered.length
                        const totalAnime = page.animeCount(page.groups)
                        const shownAnime = page.animeCount(page.filtered)
                        const isFiltered = page.activeFilter !== "all"
                            || page.searchQuery.trim().length > 0
                        const countText = isFiltered && shown !== total
                            ? shown + " of " + total + " downloads in "
                                + shownAnime + " anime"
                            : total + (total === 1 ? " download" : " downloads")
                                + " in " + totalAnime + " anime"
                        return countText
                            + "  \u00b7  " + page.countFor("complete") + " completed"
                            + "  \u00b7  " + (page.libraryPath.length > 0
                                ? page.libraryPath : "No library directory configured")
                    }
                    color: Theme.inkMuted
                    font.pixelSize: 11
                    elide: Text.ElideMiddle
                }
            }

            ActionButton {
                Layout.alignment: Qt.AlignVCenter
                label: "Open library"
                icon: "\u2197"
                tone: Theme.accent
                onActivated: if (page.backend) page.backend.openLibraryFolder()
            }
        }

        // ---- Filters ----
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 46
            radius: 12
            color: Theme.surfaceSunken
            border.width: 1
            border.color: Theme.borderSubtle

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 7
                anchors.rightMargin: 14
                spacing: 4

                Repeater {
                    model: page.filters
                    delegate: Rectangle {
                        required property var modelData
                        readonly property bool active: page.activeFilter === modelData.value
                        implicitWidth: filterRow.implicitWidth + 22
                        implicitHeight: 32
                        radius: 8
                        color: active ? Theme.accent
                            : (filterMouse.containsMouse ? Theme.glassFillHover : "transparent")

                        RowLayout {
                            id: filterRow
                            anchors.centerIn: parent
                            spacing: 6
                            Text {
                                text: modelData.label
                                color: active ? "#17110d" : Theme.inkMuted
                                font.pixelSize: 11
                                font.weight: active ? Font.DemiBold : Font.Medium
                            }
                            Rectangle {
                                implicitWidth: Math.max(18, countText.implicitWidth + 8)
                                implicitHeight: 18
                                radius: 9
                                color: active ? Qt.rgba(0, 0, 0, 0.16)
                                    : Qt.rgba(Theme.ink.r, Theme.ink.g, Theme.ink.b, 0.055)
                                Text {
                                    id: countText
                                    anchors.centerIn: parent
                                    text: page.countFor(modelData.value)
                                    color: active ? "#17110d" : Theme.inkFaint
                                    font.pixelSize: 9
                                    font.bold: true
                                }
                            }
                        }
                        MouseArea {
                            id: filterMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                if (page.activeFilter === modelData.value) {
                                    page.toggleSelectAllVisible()
                                } else {
                                    page.activeFilter = modelData.value
                                }
                            }
                        }
                    }
                }

                Item { Layout.fillWidth: true }
                Text {
                    text: "Click active tab to select all"
                    color: Theme.inkFaint
                    font.pixelSize: 10
                }
            }
        }

        // ---- Search + sort ----
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 42
            radius: 10
            color: Theme.surfaceSunken
            border.width: 1
            border.color: Theme.borderSubtle

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 6
                anchors.rightMargin: 6
                spacing: 6

                // Search
                Rectangle {
                    Layout.fillWidth: true
                    Layout.minimumWidth: 120
                    Layout.preferredHeight: 30
                    radius: 7
                    color: searchInput.activeFocus
                        ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.08)
                        : Theme.fieldFill
                    border.width: 1
                    border.color: searchInput.activeFocus
                        ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.55)
                        : Theme.borderSubtle

                    TextInput {
                        id: searchInput
                        anchors.fill: parent
                        anchors.leftMargin: 10
                        anchors.rightMargin: 26
                        verticalAlignment: TextInput.AlignVCenter
                        color: Theme.ink
                        font.pixelSize: 11
                        selectByMouse: true
                        clip: true
                        onTextChanged: page.searchQuery = text
                        Keys.onEscapePressed: {
                            text = ""
                            page.searchQuery = ""
                        }
                    }

                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.left: parent.left
                        anchors.leftMargin: 10
                        visible: searchInput.text.length === 0
                        text: "Search title, episode, or filename\u2026"
                        color: Theme.inkFaint
                        font.pixelSize: 11
                    }

                    Rectangle {
                        anchors.right: parent.right
                        anchors.rightMargin: 6
                        anchors.verticalCenter: parent.verticalCenter
                        width: 18
                        height: 18
                        radius: 4
                        visible: searchInput.text.length > 0
                        color: clearMouse.containsMouse ? Theme.glassFillHover : "transparent"
                        Text {
                            anchors.centerIn: parent
                            text: "\u00d7"
                            color: Theme.inkMuted
                            font.pixelSize: 14
                        }
                        MouseArea {
                            id: clearMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                searchInput.text = ""
                                page.searchQuery = ""
                            }
                        }
                    }
                }

                ActionButton {
                    label: page.groupByAnime ? "Grouped" : "Flat"
                    icon: "\u2261"
                    prominent: page.groupByAnime
                    quiet: !page.groupByAnime
                    tone: Theme.accent
                    implicitHeight: 30
                    onActivated: page.groupByAnime = !page.groupByAnime
                }

                Text {
                    Layout.leftMargin: 4
                    text: "Sort:"
                    color: Theme.inkFaint
                    font.pixelSize: 10
                }

                Repeater {
                    model: page.sortModes
                    delegate: Rectangle {
                        required property var modelData
                        readonly property bool active: page.sortMode === modelData.value
                        Layout.preferredWidth: sortRow.implicitWidth + 16
                        Layout.preferredHeight: 26
                        radius: 6
                        color: active ? Theme.accent
                            : (sortMouse.containsMouse ? Theme.glassFillHover : "transparent")

                        Row {
                            id: sortRow
                            anchors.centerIn: parent
                            spacing: 4
                            Text {
                                anchors.verticalCenter: parent.verticalCenter
                                text: modelData.label
                                color: active ? "#17110d" : Theme.inkMuted
                                font.pixelSize: 10
                                font.weight: active ? Font.DemiBold : Font.Medium
                            }
                            Text {
                                anchors.verticalCenter: parent.verticalCenter
                                visible: active
                                text: page.sortIsAscending ? "\u25b2" : "\u25bc"
                                color: "#17110d"
                                font.pixelSize: 7
                            }
                        }
                        MouseArea {
                            id: sortMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: page.applySort(modelData.value)
                        }
                    }
                }
            }
        }

        // ---- Bulk actions ----
        Rectangle {
            Layout.fillWidth: true
            visible: Object.keys(page.selectedIds).length > 0
            implicitHeight: visible ? 48 : 0
            radius: 10
            color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.07)
            border.width: 1
            border.color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.28)

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 14
                anchors.rightMargin: 8
                spacing: 7
                Text {
                    text: Object.keys(page.selectedIds).length + " selected"
                    color: Theme.accent
                    font.pixelSize: 11
                    font.bold: true
                }
                Item { Layout.fillWidth: true }
                ActionButton {
                    visible: page.bulkHasPausable
                    label: "Pause"
                    implicitHeight: 30
                    onActivated: page.bulkPause()
                }
                ActionButton {
                    visible: page.bulkHasResumable
                    label: "Resume"
                    implicitHeight: 30
                    onActivated: page.bulkResume()
                }
                ActionButton { label: "High priority"; implicitHeight: 30; onActivated: page.bulkPriority("high") }
                ActionButton {
                    label: "Remove"
                    tone: Theme.danger
                    implicitHeight: 30
                    onActivated: page.bulkRemove()
                }
            }
        }

        // ---- Download cards ----
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ListView {
                id: list
                anchors.fill: parent
                clip: true
                spacing: page.groupByAnime ? 0 : 10
                visible: page.filtered.length > 0
                model: page.filtered
                boundsBehavior: Flickable.StopAtBounds
                section.property: page.groupByAnime ? "animeKey" : ""
                section.labelPositioning: page.groupByAnime
                    ? (ViewSection.InlineLabels | ViewSection.CurrentLabelAtStart)
                    : ViewSection.InlineLabels

                section.delegate: Item {
                    id: animeHeader
                    readonly property var meta: page.animeMeta[section] || ({
                        title: section, coverImage: "", count: 0,
                        completeCount: 0, activeCount: 0,
                        status: "complete", fraction: 0
                    })
                    readonly property bool open: page.animeIsOpen(section)

                    width: list.width
                    height: 74

                    Rectangle {
                        anchors.fill: parent
                        color: page.stickyBackdrop
                    }

                    Rectangle {
                        visible: animeHeader.open
                        x: page.treeTrunkX - 1
                        y: sectionCard.height
                        width: 2
                        height: animeHeader.height - sectionCard.height
                        color: page.treeLineColor
                    }

                    Rectangle {
                        id: sectionCard
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        height: 64
                        radius: 12
                        color: animeHeaderMouse.containsMouse ? Theme.surfaceHover : Theme.surfaceRaised
                        border.width: 1
                        border.color: animeHeaderMouse.containsMouse
                            ? Theme.glassBorder : Theme.borderSubtle
                    }

                    MouseArea {
                        id: animeHeaderMouse
                        anchors.fill: sectionCard
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: page.toggleAnime(section)
                    }

                    RowLayout {
                        anchors.fill: sectionCard
                        anchors.leftMargin: 11
                        anchors.rightMargin: 11
                        anchors.topMargin: 9
                        anchors.bottomMargin: 9
                        spacing: 11

                        // Cover image is now clickable to open Anime Page
                        Rectangle {
                            id: sectionCoverContainer
                            Layout.alignment: Qt.AlignVCenter
                            width: 34
                            height: 46
                            radius: 7
                            color: Theme.surfaceSunken
                            border.width: 1
                            border.color: sectionCoverMouse.containsMouse && (meta.animeId || 0) > 0 
                                ? Theme.accent : Theme.glassBorder
                            clip: true

                            Image {
                                id: sectionCover
                                anchors.fill: parent
                                anchors.margins: 1
                                visible: false
                                source: meta.coverImage
                                fillMode: Image.PreserveAspectCrop
                                asynchronous: true
                                cache: true
                                sourceSize.width: 68
                                sourceSize.height: 92
                            }

                            MultiEffect {
                                source: sectionCover
                                anchors.fill: sectionCover
                                maskEnabled: true
                                maskSource: sectionMask
                                visible: meta.coverImage.length > 0 && sectionCover.status === Image.Ready
                            }

                            Item {
                                id: sectionMask
                                anchors.fill: sectionCover
                                layer.enabled: true
                                visible: false
                                Rectangle {
                                    anchors.fill: parent
                                    radius: 6
                                    color: "white"
                                }
                            }

                            Text {
                                visible: meta.coverImage.length === 0
                                    || sectionCover.status !== Image.Ready
                                anchors.centerIn: parent
                                text: (meta.title || "?").charAt(0).toUpperCase()
                                color: Theme.inkFaint
                                font.pixelSize: 15
                                font.weight: Font.DemiBold
                            }

                            // Hover overlay to indicate clickability
                            Rectangle {
                                anchors.fill: parent
                                radius: parent.radius
                                visible: sectionCoverMouse.containsMouse && (meta.animeId || 0) > 0
                                color: Qt.rgba(0, 0, 0, 0.5)
                                Text {
                                    anchors.centerIn: parent
                                    text: "\u2197" // External link arrow
                                    color: "#ffffff"
                                    font.pixelSize: 14
                                }
                            }

                            MouseArea {
                                id: sectionCoverMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: (meta.animeId || 0) > 0 ? Qt.PointingHandCursor : Qt.ArrowCursor
                                onClicked: (mouse) => {
                                    if ((meta.animeId || 0) > 0) {
                                        page.openAnime(meta.animeId, meta.provider || "anilist")
                                    }
                                }
                                ToolTip.visible: containsMouse && (meta.animeId || 0) > 0
                                ToolTip.delay: 300
                                ToolTip.text: "Open anime page"
                            }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            Layout.alignment: Qt.AlignVCenter
                            spacing: 3

                            Text {
                                Layout.fillWidth: true
                                text: meta.title
                                color: Theme.ink
                                font.pixelSize: 15
                                font.weight: Font.DemiBold
                                elide: Text.ElideRight
                            }
                            Text {
                                Layout.fillWidth: true
                                text: meta.count + (meta.count === 1 ? " download" : " downloads")
                                    + (meta.completeCount > 0
                                        ? "  \u00b7  " + meta.completeCount + " completed" : "")
                                    + (meta.activeCount > 0 && meta.completeCount > 0
                                        ? "  \u00b7  " + meta.activeCount + " unfinished" : "")
                                color: Theme.inkMuted
                                font.pixelSize: 10
                                elide: Text.ElideRight
                            }
                        }

                        Rectangle {
                            Layout.alignment: Qt.AlignVCenter
                            implicitWidth: animeStatusRow.implicitWidth + 16
                            implicitHeight: 22
                            radius: 11
                            color: Qt.rgba(page.statusColorFor(meta.status).r,
                                page.statusColorFor(meta.status).g,
                                page.statusColorFor(meta.status).b, 0.09)
                            RowLayout {
                                id: animeStatusRow
                                anchors.centerIn: parent
                                spacing: 6
                                Rectangle {
                                    width: 5; height: 5; radius: 2.5
                                    color: page.statusColorFor(meta.status)
                                }
                                Text {
                                    text: page.statusLabelFor(meta.status)
                                    color: page.statusColorFor(meta.status)
                                    font.pixelSize: 10
                                    font.weight: Font.DemiBold
                                }
                            }
                        }

                        Text {
                            Layout.alignment: Qt.AlignVCenter
                            text: animeHeader.open ? "\u25be" : "\u25b8"
                            color: animeHeaderMouse.containsMouse
                                ? Theme.accent : Theme.inkMuted
                            font.pixelSize: 14
                        }
                    }
                }

                delegate: Item {
                    id: card
                    required property var modelData
                    required property int index
                    readonly property var group: modelData
                    readonly property var live: page.liveGroups[group.key] || ({
                        status: group.status,
                        progressBytes: 0, totalBytes: 0,
                        speedMbps: 0, peersLive: 0, etaSeconds: 0,
                        error: "", restoring: false,
                        fraction: 0, percent: 0
                    })
                    readonly property bool expanded: page.expandedGroups[group.key] === true
                    readonly property bool selected: page.selectedIds[group.key] === true
                    readonly property string animeKey: group.animeKey || page.animeKeyOf(group)
                    readonly property bool animeOpen: page.animeIsOpen(animeKey)
                    readonly property bool tree: page.groupByAnime
                    readonly property bool isLastInAnime: index >= page.filtered.length - 1
                        || (page.filtered[index + 1].animeKey
                            || page.animeKeyOf(page.filtered[index + 1])) !== animeKey
                    readonly property int gutter: tree ? page.treeGutter : 0
                    readonly property int gapBelow: tree ? page.cardGap : 0
                    readonly property int elbowY: 38

                    width: list.width
                    height: animeOpen ? cardBody.height + gapBelow : 0
                    visible: animeOpen

                    Rectangle {
                        visible: card.tree
                        x: page.treeTrunkX - 1
                        y: 0
                        width: 2
                        height: card.isLastInAnime ? card.elbowY : card.height
                        color: page.treeLineColor
                    }
                    Rectangle {
                        visible: card.tree
                        x: page.treeTrunkX - 1
                        y: card.elbowY - 1
                        width: page.treeGutter - page.treeTrunkX - 3
                        height: 2
                        color: page.treeLineColor
                    }
                    Rectangle {
                        visible: card.tree
                        x: page.treeTrunkX - 3
                        y: card.elbowY - 3
                        width: 6; height: 6; radius: 3
                        color: page.statusColorFor(card.live.status)
                    }

                    Rectangle {
                    id: cardBody
                    x: card.gutter
                    width: card.width - card.gutter
                    height: content.implicitHeight + 34
                    radius: 13
                    color: cardMouse.containsMouse ? Theme.surfaceHover : Theme.surfaceRaised
                    border.width: selected ? 2 : 1
                    border.color: selected ? Theme.accent
                        : (live.status === "failed"
                            ? Qt.rgba(Theme.danger.r, Theme.danger.g, Theme.danger.b, 0.42)
                            : (cardMouse.containsMouse ? Theme.glassBorder : Theme.borderSubtle))

                    MouseArea {
                        id: cardMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        acceptedButtons: Qt.LeftButton | Qt.MiddleButton
                        cursorShape: Qt.PointingHandCursor
                        onClicked: (mouse) => {
                            if (mouse.button === Qt.MiddleButton) {
                                page.toggleSelected(group.key)
                            } else if (mouse.button === Qt.LeftButton) {
                                if (group.completeCount > 0) {
                                    page.playGroup(group)
                                } else {
                                    page.toggleExpanded(group.key)
                                }
                            }
                        }
                    }

                    Rectangle {
                        anchors.left: parent.left
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom
                        anchors.margins: 8
                        width: 3
                        radius: 1.5
                        color: page.statusColorFor(live.status)
                        opacity: selected || live.status === "downloading" ? 1 : 0.65
                    }

                    ColumnLayout {
                        id: content
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.margins: 12
                        anchors.leftMargin: 18
                        spacing: 10

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 12

                            // Selection + expander
                            ColumnLayout {
                                Layout.alignment: Qt.AlignTop
                                Layout.topMargin: 4
                                spacing: 8
                                Rectangle {
                                    Layout.alignment: Qt.AlignHCenter
                                    width: 17
                                    height: 17
                                    radius: 5
                                    color: card.selected ? Theme.accent : "transparent"
                                    border.width: 1.5
                                    border.color: card.selected ? Theme.accent : Theme.inkFaint
                                    Text {
                                        anchors.centerIn: parent
                                        visible: card.selected
                                        text: "\u2713"
                                        color: "#17110d"
                                        font.pixelSize: 10
                                        font.bold: true
                                    }
                                    MouseArea {
                                        anchors.fill: parent
                                        anchors.margins: -4
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: page.toggleSelected(group.key)
                                    }
                                }
                                Rectangle {
                                    Layout.alignment: Qt.AlignHCenter
                                    width: 20
                                    height: 20
                                    radius: 6
                                    color: expandMouse.containsMouse ? Theme.glassFillHover : "transparent"
                                    Text {
                                        anchors.centerIn: parent
                                        text: card.expanded ? "\u25be" : "\u25b8"
                                        color: expandMouse.containsMouse ? Theme.accent : Theme.inkMuted
                                        font.pixelSize: 12
                                    }
                                    MouseArea {
                                        id: expandMouse
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: page.toggleExpanded(group.key)
                                    }
                                }
                            }

                            // Cover
                            Rectangle {
                                id: coverContainer
                                Layout.alignment: Qt.AlignTop
                                width: 62
                                height: 84
                                radius: 9
                                color: Theme.surfaceSunken
                                border.width: 1
                                border.color: Theme.glassBorder
                                
                                Image {
                                    id: coverArt
                                    anchors.fill: parent
                                    anchors.margins: 1
                                    visible: false
                                    source: group.coverImage
                                    fillMode: Image.PreserveAspectCrop
                                    asynchronous: true
                                    cache: true
                                    sourceSize.width: 124
                                    sourceSize.height: 168
                                }

                                MultiEffect {
                                    source: coverArt
                                    anchors.fill: coverArt
                                    maskEnabled: true
                                    maskSource: maskItem
                                    visible: group.coverImage.length > 0 && coverArt.status === Image.Ready
                                }

                                Item {
                                    id: maskItem
                                    anchors.fill: coverArt
                                    layer.enabled: true
                                    visible: false
                                    Rectangle {
                                        anchors.fill: parent
                                        radius: 8
                                        color: "white"
                                    }
                                }
                                
                                Text {
                                    visible: group.coverImage.length === 0 || coverArt.status !== Image.Ready
                                    anchors.centerIn: parent
                                    text: (group.title || "?").charAt(0).toUpperCase()
                                    color: Theme.inkFaint
                                    font.pixelSize: 24
                                    font.weight: Font.DemiBold
                                }
                                
                                // Play overlay on hover
                                Rectangle {
                                    anchors.fill: parent
                                    radius: parent.radius
                                    visible: group.completeCount > 0 && cardMouse.containsMouse
                                    color: Qt.rgba(0, 0, 0, 0.4)
                                    Text {
                                        anchors.centerIn: parent
                                        text: "\u25b6"
                                        color: "#ffffff"
                                        font.pixelSize: 24
                                    }
                                }
                            }

                            // Primary information and progress
                            ColumnLayout {
                                Layout.fillWidth: true
                                Layout.alignment: Qt.AlignTop
                                spacing: 7

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 8
                                    Text {
                                        Layout.fillWidth: true
                                        text: group.title
                                        color: Theme.ink
                                        font.pixelSize: 15
                                        font.weight: Font.DemiBold
                                        elide: Text.ElideRight
                                    }
                                    Rectangle {
                                        implicitWidth: statusRow.implicitWidth + 16
                                        implicitHeight: 22
                                        radius: 11
                                        color: Qt.rgba(page.statusColorFor(live.status).r,
                                            page.statusColorFor(live.status).g,
                                            page.statusColorFor(live.status).b, 0.09)
                                        RowLayout {
                                            id: statusRow
                                            anchors.centerIn: parent
                                            spacing: 6
                                            Rectangle {
                                                width: 5; height: 5; radius: 2.5
                                                color: page.statusColorFor(live.status)
                                            }
                                            Text {
                                                text: live.restoring
                                                    ? "Restoring\u2026"
                                                    : page.statusLabelFor(live.status)
                                                color: page.statusColorFor(live.status)
                                                font.pixelSize: 10
                                                font.weight: Font.DemiBold
                                            }
                                        }
                                    }
                                    ActionButton {
                                        label: page.priorityLabelFor(group.priority)
                                        quiet: true
                                        implicitHeight: 22
                                        tone: Theme.inkMuted
                                        onActivated: page.cyclePriority(group)
                                    }
                                }

                                Text {
                                    Layout.fillWidth: true
                                    text: page.groupLabel(group)
                                        + (group.releaseTitle.length > 0 ? "  \u00b7  " + group.releaseTitle : "")
                                    color: Theme.inkMuted
                                    font.pixelSize: 11
                                    elide: Text.ElideRight
                                }

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 12
                                    Text {
                                        text: page.fileSummary(group)
                                        color: Theme.inkFaint
                                        font.pixelSize: 10
                                    }
                                    Text {
                                        visible: live.status === "downloading"
                                        text: page.formatSpeed(live.speedMbps)
                                        color: live.speedMbps > 0 ? Theme.healthGreen : Theme.inkFaint
                                        font.pixelSize: 10
                                        font.bold: live.speedMbps > 0
                                    }
                                    Text {
                                        visible: live.status === "downloading"
                                        text: live.peersLive + " peers  \u00b7  "
                                            + page.formatEta(live.etaSeconds)
                                        color: Theme.inkFaint
                                        font.pixelSize: 10
                                    }
                                    Item { Layout.fillWidth: true }
                                    Text {
                                        text: page.formatSize(live.progressBytes)
                                            + " / " + page.formatSize(live.totalBytes)
                                        color: Theme.inkFaint
                                        font.pixelSize: 10
                                    }
                                }

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 10
                                    Rectangle {
                                        Layout.fillWidth: true
                                        Layout.alignment: Qt.AlignVCenter
                                        height: 6
                                        radius: 3
                                        color: Theme.separator
                                        Rectangle {
                                            height: parent.height
                                            radius: parent.radius
                                            width: parent.width * live.fraction
                                            color: live.status === "downloading"
                                                ? "transparent"
                                                : page.fillColorFor(live.status)
                                            Behavior on width { NumberAnimation { duration: 280; easing.type: Easing.Linear } }

                                            // Download gradient overlay
                                            Rectangle {
                                                anchors.fill: parent
                                                radius: parent.radius
                                                visible: live.status === "downloading"
                                                gradient: Gradient {
                                                    orientation: Gradient.Horizontal
                                                    GradientStop { position: 0.0; color: "#F44949" }
                                                    GradientStop { position: 0.5; color: "#FEC42D" }
                                                    GradientStop { position: 1.0; color: "#FEF7E0" }
                                                }
                                            }
                                        }
                                    }
                                    Text {
                                        Layout.minimumWidth: 34
                                        horizontalAlignment: Text.AlignRight
                                        text: live.percent + "%"
                                        color: page.statusColorFor(live.status)
                                        font.pixelSize: 11
                                        font.bold: true
                                    }
                                }
                            }

                            // Fox illustration
                            ErrorState {
                                Layout.alignment: Qt.AlignVCenter
                                Layout.preferredWidth: 92
                                Layout.preferredHeight: 72
                                kind: page.foxKindFor(live.status)
                                animated: true
                                showArt: true
                                showText: false
                                showActions: false
                                compact: true
                                artWidth: 92
                                maxContentWidth: 92
                            }
                        }

                        Text {
                            Layout.fillWidth: true
                            visible: live.status === "failed" && live.error.length > 0
                            text: live.error
                            color: Theme.danger
                            font.pixelSize: 11
                            wrapMode: Text.WordWrap
                        }

                        // Actions footer
                        RowLayout {
                            Layout.fillWidth: true
                            Layout.leftMargin: 32
                            spacing: 7
                            
                            ActionButton {
                                visible: live.status !== "complete"
                                label: live.status === "paused" ? "Resume"
                                    : (live.status === "failed" ? "Retry" : "Pause")
                                tone: Theme.accent
                                prominent: live.status === "paused" || live.status === "failed"
                                onActivated: page.togglePause(group, live.status)
                            }
                            ActionButton {
                                label: "Files"
                                onActivated: page.manageFiles(group)
                            }
                            ActionButton {
                                visible: group.completeCount > 0
                                label: "Open folder"
                                onActivated: page.openGroupFolder(group)
                            }
                            Item { Layout.fillWidth: true }
                            ActionButton {
                                id: removeButton
                                property bool armed: false
                                label: armed ? "Confirm remove" : "Remove"
                                tone: Theme.danger
                                quiet: !armed
                                onActivated: {
                                    if (!page.backend) return
                                    if (!armed) {
                                        armed = true
                                        disarmTimer.restart()
                                        return
                                    }
                                    page.backend.removeDownload(String(group.actionId), group.completeCount > 0)
                                    armed = false
                                }
                                Timer {
                                    id: disarmTimer
                                    interval: 4000
                                    onTriggered: removeButton.armed = false
                                }
                            }
                        }

                        Rectangle {
                            Layout.fillWidth: true
                            Layout.leftMargin: 32
                            Layout.preferredHeight: 1
                            visible: card.expanded
                            color: Theme.separator
                        }

                        Repeater {
                            model: card.expanded ? group.records : []
                            delegate: Rectangle {
                                required property var modelData
                                readonly property var rec: modelData
                                readonly property var liveRec: page.liveRecords[rec.id] || ({
                                    status: rec.status,
                                    progressBytes: rec.progressBytes || 0,
                                    totalBytes: rec.totalBytes || 0,
                                    fraction: 0, percent: 0
                                })

                                Layout.fillWidth: true
                                Layout.leftMargin: 32
                                implicitHeight: 48
                                radius: 8
                                color: fileMouse.containsMouse ? Theme.glassFillHover : Theme.surfaceSunken

                                // Click expanded row to play
                                MouseArea {
                                    id: fileMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: liveRec.status === "complete" ? Qt.PointingHandCursor : Qt.ArrowCursor
                                    onClicked: {
                                        if (liveRec.status === "complete") {
                                            page.play(rec)
                                        }
                                    }
                                }

                                RowLayout {
                                    anchors.fill: parent
                                    anchors.leftMargin: 12
                                    anchors.rightMargin: 8
                                    spacing: 10
                                    Rectangle {
                                        width: 6; height: 6; radius: 3
                                        color: page.statusColorFor(liveRec.status)
                                    }
                                    ColumnLayout {
                                        Layout.fillWidth: true
                                        spacing: 2
                                        Text {
                                            Layout.fillWidth: true
                                            text: page.label(rec)
                                            color: Theme.ink
                                            font.pixelSize: 11
                                            font.weight: Font.DemiBold
                                            elide: Text.ElideRight
                                        }
                                        Text {
                                            Layout.fillWidth: true
                                            text: (rec.fileName || "")
                                                + "  \u00b7  " + page.statusLabelFor(liveRec.status)
                                                + (liveRec.status !== "complete"
                                                    ? "  \u00b7  " + liveRec.percent + "%"
                                                    : "  \u00b7  " + page.formatSize(liveRec.totalBytes))
                                            color: Theme.inkFaint
                                            font.pixelSize: 9
                                            elide: Text.ElideMiddle
                                        }
                                    }
                                    Rectangle {
                                        visible: liveRec.status !== "complete"
                                        width: 90; height: 4; radius: 2
                                        color: Theme.separator
                                        Rectangle {
                                            height: parent.height
                                            radius: parent.radius
                                            width: parent.width * liveRec.fraction
                                            color: liveRec.status === "downloading"
                                                ? "transparent"
                                                : page.statusColorFor(liveRec.status)

                                            Rectangle {
                                                anchors.fill: parent
                                                radius: parent.radius
                                                visible: liveRec.status === "downloading"
                                                gradient: Gradient {
                                                    orientation: Gradient.Horizontal
                                                    GradientStop { position: 0.0; color: "#F44949" }
                                                    GradientStop { position: 0.5; color: "#FEC42D" }
                                                    GradientStop { position: 1.0; color: "#FEF7E0" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    } // cardBody
                }
            }

            ErrorState {
                anchors.centerIn: parent
                visible: page.filtered.length === 0
                kind: "sleeping"
                artWidth: 180
                maxContentWidth: 360
                title: page.searchQuery.trim().length > 0
                    ? "No matches"
                    : (page.groups.length === 0 ? "No downloads yet" : "Nothing in this filter")
                message: page.searchQuery.trim().length > 0
                    ? "Try a different search term."
                    : (page.groups.length === 0
                        ? "Use the download button on a release to keep an episode."
                        : "Switch the filter to see the other downloads.")
                showActions: false
            }
        }

        Text {
            Layout.fillWidth: true
            visible: page.errorText.length > 0
            text: page.errorText
            color: Theme.danger
            font.pixelSize: 11
            wrapMode: Text.WordWrap
        }
    }
}