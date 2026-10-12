import QtQuick
import QtQuick.Controls
import com.kitsune.app

Window {
    width: 1280
    height: 800
    visible: true
    title: "Kitsune"
    color: "#0b0d12"

    // The Rust bridge. In QML a `#[qml_element]` QObject is instantiated by
    // its Rust type name.
    Backend {
        id: backend
    }

    Column {
        anchors.centerIn: parent
        spacing: 16

        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: "Kitsune"
            color: "#7c5cff"
            font.pixelSize: 48
            font.bold: true
        }

        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: backend.greet()
            color: "#e6e6e6"
            font.pixelSize: 16
        }

        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: backend.coreSummary()
            color: "#8a8f98"
            font.pixelSize: 13
        }
    }
}