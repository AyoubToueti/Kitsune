// Vector glyphs for the player chrome.
//
// Drawn on a Canvas rather than set in a font: the media characters (U+23EE,
// U+23F8, ...) render as colour emoji on many systems, so their colour, size
// and baseline differ per platform and they ignore the theme. A few lines of
// path code is cheaper than shipping an icon font and gives one consistent look.
import QtQuick

Canvas {
    id: icon

    // play | pause | stop | prev | next | rewind | forward |
    // volume | mute | gear | fullscreen | exit-fullscreen | close |
    // copy | download | spinner | info | chevron-right | chevron-left |
    // eye-off
    property string kind: "play"
    property color color: Theme.inkMuted

    width: 18
    height: 18

    onKindChanged: requestPaint()
    onColorChanged: requestPaint()
    onWidthChanged: requestPaint()
    onHeightChanged: requestPaint()

    onPaint: {
        var ctx = getContext("2d")
        ctx.clearRect(0, 0, width, height)

        var w = width
        var h = height
        ctx.fillStyle = color
        ctx.strokeStyle = color
        ctx.lineWidth = Math.max(1.4, w * 0.09)
        ctx.lineJoin = "round"
        ctx.lineCap = "round"

        switch (kind) {
        case "play":
            ctx.beginPath()
            ctx.moveTo(w * 0.30, h * 0.20)
            ctx.lineTo(w * 0.80, h * 0.50)
            ctx.lineTo(w * 0.30, h * 0.80)
            ctx.closePath()
            ctx.fill()
            break
        case "pause":
            ctx.fillRect(w * 0.30, h * 0.22, w * 0.13, h * 0.56)
            ctx.fillRect(w * 0.57, h * 0.22, w * 0.13, h * 0.56)
            break
        case "stop":
            ctx.fillRect(w * 0.28, h * 0.28, w * 0.44, h * 0.44)
            break
        case "prev":
            ctx.fillRect(w * 0.20, h * 0.26, w * 0.10, h * 0.48)
            ctx.beginPath()
            ctx.moveTo(w * 0.82, h * 0.24)
            ctx.lineTo(w * 0.38, h * 0.50)
            ctx.lineTo(w * 0.82, h * 0.76)
            ctx.closePath()
            ctx.fill()
            break
        case "next":
            ctx.beginPath()
            ctx.moveTo(w * 0.18, h * 0.24)
            ctx.lineTo(w * 0.62, h * 0.50)
            ctx.lineTo(w * 0.18, h * 0.76)
            ctx.closePath()
            ctx.fill()
            ctx.fillRect(w * 0.70, h * 0.26, w * 0.10, h * 0.48)
            break
        case "rewind":
            ctx.beginPath()
            ctx.moveTo(w * 0.50, h * 0.24)
            ctx.lineTo(w * 0.16, h * 0.50)
            ctx.lineTo(w * 0.50, h * 0.76)
            ctx.closePath()
            ctx.fill()
            ctx.beginPath()
            ctx.moveTo(w * 0.86, h * 0.24)
            ctx.lineTo(w * 0.52, h * 0.50)
            ctx.lineTo(w * 0.86, h * 0.76)
            ctx.closePath()
            ctx.fill()
            break
        case "forward":
            ctx.beginPath()
            ctx.moveTo(w * 0.14, h * 0.24)
            ctx.lineTo(w * 0.48, h * 0.50)
            ctx.lineTo(w * 0.14, h * 0.76)
            ctx.closePath()
            ctx.fill()
            ctx.beginPath()
            ctx.moveTo(w * 0.50, h * 0.24)
            ctx.lineTo(w * 0.84, h * 0.50)
            ctx.lineTo(w * 0.50, h * 0.76)
            ctx.closePath()
            ctx.fill()
            break
        case "volume":
            ctx.beginPath()
            ctx.moveTo(w * 0.14, h * 0.38)
            ctx.lineTo(w * 0.28, h * 0.38)
            ctx.lineTo(w * 0.42, h * 0.22)
            ctx.lineTo(w * 0.42, h * 0.78)
            ctx.lineTo(w * 0.28, h * 0.62)
            ctx.lineTo(w * 0.14, h * 0.62)
            ctx.closePath()
            ctx.fill()
            ctx.beginPath()
            ctx.arc(w * 0.44, h * 0.50, w * 0.16, -Math.PI / 3, Math.PI / 3)
            ctx.stroke()
            ctx.beginPath()
            ctx.arc(w * 0.44, h * 0.50, w * 0.30, -Math.PI / 3, Math.PI / 3)
            ctx.stroke()
            break
        case "mute":
            ctx.beginPath()
            ctx.moveTo(w * 0.12, h * 0.38)
            ctx.lineTo(w * 0.26, h * 0.38)
            ctx.lineTo(w * 0.40, h * 0.22)
            ctx.lineTo(w * 0.40, h * 0.78)
            ctx.lineTo(w * 0.26, h * 0.62)
            ctx.lineTo(w * 0.12, h * 0.62)
            ctx.closePath()
            ctx.fill()
            ctx.beginPath()
            ctx.moveTo(w * 0.58, h * 0.36)
            ctx.lineTo(w * 0.84, h * 0.64)
            ctx.moveTo(w * 0.84, h * 0.36)
            ctx.lineTo(w * 0.58, h * 0.64)
            ctx.stroke()
            break
        case "gear":
            var cx = w * 0.5
            var cy = h * 0.5
            var rIn = w * 0.19
            var rOut = w * 0.40
            ctx.beginPath()
            ctx.arc(cx, cy, rIn, 0, Math.PI * 2)
            ctx.stroke()
            for (var i = 0; i < 8; i++) {
                var a = (Math.PI * 2 / 8) * i
                ctx.beginPath()
                ctx.moveTo(cx + Math.cos(a) * rIn, cy + Math.sin(a) * rIn)
                ctx.lineTo(cx + Math.cos(a) * rOut, cy + Math.sin(a) * rOut)
                ctx.stroke()
            }
            break
        case "fullscreen":
            var m = w * 0.24
            var s = w * 0.20
            ctx.beginPath()
            ctx.moveTo(m, m + s)
            ctx.lineTo(m, m)
            ctx.lineTo(m + s, m)
            ctx.moveTo(w - m - s, m)
            ctx.lineTo(w - m, m)
            ctx.lineTo(w - m, m + s)
            ctx.moveTo(w - m, h - m - s)
            ctx.lineTo(w - m, h - m)
            ctx.lineTo(w - m - s, h - m)
            ctx.moveTo(m + s, h - m)
            ctx.lineTo(m, h - m)
            ctx.lineTo(m, h - m - s)
            ctx.stroke()
            break
        case "exit-fullscreen":
            var n = w * 0.24
            var t = w * 0.20
            ctx.beginPath()
            ctx.moveTo(n, n + t)
            ctx.lineTo(n + t, n + t)
            ctx.lineTo(n + t, n)
            ctx.moveTo(w - n - t, n)
            ctx.lineTo(w - n - t, n + t)
            ctx.lineTo(w - n, n + t)
            ctx.moveTo(w - n, h - n - t)
            ctx.lineTo(w - n - t, h - n - t)
            ctx.lineTo(w - n - t, h - n)
            ctx.moveTo(n + t, h - n)
            ctx.lineTo(n + t, h - n - t)
            ctx.lineTo(n, h - n - t)
            ctx.stroke()
            break
        case "close":
            ctx.beginPath()
            ctx.moveTo(w * 0.28, h * 0.28)
            ctx.lineTo(w * 0.72, h * 0.72)
            ctx.moveTo(w * 0.72, h * 0.28)
            ctx.lineTo(w * 0.28, h * 0.72)
            ctx.stroke()
            break
        case "copy":
            ctx.strokeRect(w * 0.14, h * 0.14, w * 0.48, h * 0.48)
            ctx.strokeRect(w * 0.38, h * 0.38, w * 0.48, h * 0.48)
            break
        case "download":
            ctx.beginPath()
            ctx.moveTo(w * 0.5, h * 0.18)
            ctx.lineTo(w * 0.5, h * 0.62)
            ctx.moveTo(w * 0.30, h * 0.44)
            ctx.lineTo(w * 0.5, h * 0.64)
            ctx.lineTo(w * 0.70, h * 0.44)
            ctx.moveTo(w * 0.22, h * 0.80)
            ctx.lineTo(w * 0.78, h * 0.80)
            ctx.stroke()
            break
        case "spinner":
            ctx.beginPath()
            ctx.arc(w * 0.5, h * 0.5, w * 0.36, 0, Math.PI * 1.5)
            ctx.stroke()
            break
        case "info":
            // Circled "i": the dot and the stem.
            ctx.beginPath()
            ctx.arc(w * 0.5, h * 0.5, w * 0.36, 0, Math.PI * 2)
            ctx.stroke()
            ctx.beginPath()
            ctx.arc(w * 0.5, h * 0.30, w * 0.055, 0, Math.PI * 2)
            ctx.fill()
            ctx.beginPath()
            ctx.moveTo(w * 0.5, h * 0.44)
            ctx.lineTo(w * 0.5, h * 0.72)
            ctx.stroke()
            break
        case "chevron-right":
            ctx.beginPath()
            ctx.moveTo(w * 0.38, h * 0.24)
            ctx.lineTo(w * 0.66, h * 0.5)
            ctx.lineTo(w * 0.38, h * 0.76)
            ctx.stroke()
            break
        case "chevron-left":
            ctx.beginPath()
            ctx.moveTo(w * 0.62, h * 0.24)
            ctx.lineTo(w * 0.34, h * 0.5)
            ctx.lineTo(w * 0.62, h * 0.76)
            ctx.stroke()
            break
        case "eye-off":
            // A crossed-out eye: "this episode has been watched". Two arcs
            // form the eye, a dot the pupil, and a slash crosses the whole.
            ctx.beginPath()
            ctx.moveTo(w * 0.12, h * 0.5)
            ctx.quadraticCurveTo(w * 0.5, h * 0.16, w * 0.88, h * 0.5)
            ctx.quadraticCurveTo(w * 0.5, h * 0.84, w * 0.12, h * 0.5)
            ctx.closePath()
            ctx.stroke()
            ctx.beginPath()
            ctx.arc(w * 0.5, h * 0.5, w * 0.12, 0, Math.PI * 2)
            ctx.fill()
            ctx.beginPath()
            ctx.moveTo(w * 0.16, h * 0.82)
            ctx.lineTo(w * 0.84, h * 0.18)
            ctx.stroke()
            break
        }
    }
}