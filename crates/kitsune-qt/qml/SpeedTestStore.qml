// The shared internet-speed-test state.
//
// One measurement serves every surface that shows it (the Settings diagnostics
// card and the watch modal's compact readout), so the result is held here
// rather than in each page. The backend allows only one run at a time; this
// store mirrors that in `running` so every button disables together.
pragma Singleton
import QtQuick

// An `Item` rather than `QtObject` for the same reason as `ListStatusStore`:
// the `Connections` block below is a child, and `QtObject` has no default
// property to hold one.
Item {
    id: store

    // The Backend, set once by the window.
    property var backend: null

    // Whether a measurement is in flight.
    property bool running: false
    // The latest progress: { phase, percent, latencyMs, downloadMbps }.
    property var progress: null
    // The finished result: { latencyMs, jitterMs, downloadMbps,
    // bytesDownloaded, durationMs, verdict }, or null until one completes.
    property var result: null
    // The last failure, cleared when a new run starts.
    property string error: ""

    // Whether to show MiB/s instead of Mbps, from the saved settings. Refreshed
    // when the backend reports settings, so toggling it in Settings updates the
    // result without a restart.
    property bool binaryUnits: false

    function setBackend(b) {
        backend = b
        if (b) b.getSettings()
    }

    // Start a measurement. Ignores the click while one is already running, so
    // a double-click cannot queue a second request the backend would reject.
    function run() {
        if (!backend || running) return
        running = true
        progress = null
        error = ""
        backend.runSpeedTest()
    }

    // A one-line verdict for the result, or "" before one exists.
    function verdictText() {
        if (!result) return ""
        if (result.verdict === "good") return "Your connection is fast enough for smooth streaming."
        if (result.verdict === "ok") return "Your connection should stream at standard quality."
        if (result.verdict === "poor") return "Your connection is slow; expect buffering at high quality."
        return ""
    }

    // The download figure, formatted in the reader's chosen unit, or "" before
    // a result exists.
    //
    // The measurement is always decimal megabits (that is what the engine
    // reports and how connections are sold). MiB/s is derived here rather than
    // in the backend so the one stored result serves both displays:
    //   bits/s = Mbps * 1e6
    //   MiB/s  = bits/s / 8 / 2^20 = Mbps * 0.11920928955078125
    function downloadText() {
        if (!result || result.downloadMbps === undefined) return ""
        if (store.binaryUnits) {
            const mib = result.downloadMbps * 0.11920928955078125
            return mib.toFixed(1) + " MiB/s"
        }
        return result.downloadMbps.toFixed(1) + " Mbps"
    }

    // Latency in milliseconds, rounded, or "" before a result exists.
    function latencyText() {
        if (!result || result.latencyMs === undefined) return ""
        return Math.round(result.latencyMs) + " ms"
    }

    Connections {
        target: store.backend

        function onSpeedTestProgress(json) {
            store.progress = JSON.parse(json) || null
        }
        function onSpeedTestReady(json) {
            store.running = false
            store.progress = null
            store.result = JSON.parse(json) || null
        }
        function onSpeedTestFailed(message) {
            store.running = false
            store.progress = null
            store.error = message
        }

        // The unit choice lives in Settings; read it whenever settings arrive,
        // which covers both startup and a save from the settings popup.
        function onSettingsReady(json) {
            const s = JSON.parse(json)
            if (s === null) return
            store.binaryUnits = s.useBinaryUnits === true
        }
    }
}