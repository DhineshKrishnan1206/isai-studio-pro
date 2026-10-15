import QtQuick
import QtQuick.Controls.Basic
import Isai

// A button for the painted start screen: cream when primary, otherwise a
// see-through outline. Its colours are fixed, because the painting behind it
// looks the same in light and dark mode.
AbstractButton {
    id: control
    property bool primary: false
    property string glyph: ""
    /// A shortcut shown after the label, like "⌘N".
    property string hint: ""

    readonly property color cream: "#F6ECD6"
    readonly property color ink: primary ? "#0A2A1E" : cream

    implicitHeight: 44
    implicitWidth: row.implicitWidth + 36
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus
    scale: down ? 0.97 : 1
    Behavior on scale { NumberAnimation { duration: Theme.fast; easing.type: Easing.OutCubic } }

    background: Rectangle {
        radius: 12
        color: control.primary ? (control.hovered ? "#FFF6E4" : control.cream)
                               : Qt.rgba(0.96, 0.93, 0.84, control.hovered ? 0.16 : 0.08)
        border.width: control.primary ? 0 : 1
        border.color: Qt.rgba(0.96, 0.93, 0.84, 0.22)
        Behavior on color { ColorAnimation { duration: Theme.fast } }
        FocusRing { visible: control.visualFocus }
    }
    contentItem: Item {
        Row {
            id: row
            anchors.centerIn: parent
            spacing: 9
            Icon {
                visible: control.glyph !== ""
                glyph: control.glyph
                size: 16
                color: control.ink
                anchors.verticalCenter: parent.verticalCenter
            }
            Text {
                text: control.text
                font.family: Theme.font
                font.pixelSize: 14
                font.weight: Font.DemiBold
                color: control.ink
                anchors.verticalCenter: parent.verticalCenter
            }
            Text {
                visible: control.hint !== ""
                text: control.hint
                font.family: Theme.mono
                font.pixelSize: 11
                color: control.ink
                opacity: 0.6
                anchors.verticalCenter: parent.verticalCenter
            }
        }
    }
}
