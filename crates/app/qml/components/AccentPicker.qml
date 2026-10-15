import QtQuick
import QtQuick.Controls.Basic
import Isai

// A row of colour swatches. The first is the style's own accent ("default").
Row {
    id: picker
    property string value: "default"
    signal picked(string value)
    spacing: 8

    Repeater {
        model: ["default", "blue", "violet", "teal", "green", "orange", "pink", "red"]
        delegate: AbstractButton {
            id: swatch
            required property string modelData
            readonly property bool current: picker.value === modelData
            readonly property color colour: modelData === "default"
                ? Theme.accents[Theme.studio ? "blue" : "violet"][Theme.dark ? "dark" : "light"]
                : Theme.accents[modelData][Theme.dark ? "dark" : "light"]

            width: 26; height: 26
            hoverEnabled: true
            focusPolicy: Qt.StrongFocus
            onClicked: picker.picked(modelData)

            background: Rectangle {
                radius: width / 2
                color: swatch.colour
                border.width: swatch.current ? 2 : 0
                border.color: Theme.text
                scale: swatch.hovered ? 1.1 : 1
                Behavior on scale { NumberAnimation { duration: Theme.fast } }
                FocusRing { visible: swatch.visualFocus }
            }
            contentItem: Icon {
                glyph: swatch.modelData === "default" ? Icons.sparkle : Icons.check
                visible: swatch.current || swatch.modelData === "default"
                size: 13
                color: "#FFFFFF"
            }
            DToolTip {
                visible: swatch.hovered
                text: swatch.modelData === "default" ? "Style default"
                      : Theme.accents[swatch.modelData].label
            }
        }
    }
}
