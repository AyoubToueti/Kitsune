// Throwaway spike. DELETE once Phase 1 lands for real.
import QtQuick
import QtQuick.Controls

Window {
    width: 640
    height: 360
    visible: true
    title: "Kitsune Qt6 spike"
    color: "#0b0d12"

    Column {
        anchors.centerIn: parent
        spacing: 12

        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: "Kitsune"
            color: "#7c5cff"
            font.pixelSize: 42
            font.bold: true
        }
        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: "Qt6 QML window is up on NixOS."
            color: "#e6e6e6"
            font.pixelSize: 16
        }
    }
}