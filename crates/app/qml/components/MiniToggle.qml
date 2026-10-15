import QtQuick
import QtQuick.Controls.Basic
import Isai

// Small square on/off button: mute, solo, record-arm, monitor.
AbstractButton {
    id: toggle
    property string label
    property string glyph: ""
    property bool on: false
    property color onColor: Theme.accent
    property string tip
    implicitWidth: 22
    implicitHeight: 20
    hoverEnabled: true
    background: Rectangle {
        radius: Theme.radiusSm
        color: toggle.on ? toggle.onColor : toggle.hovered ? Theme.hover : Theme.raised
        border.color: toggle.on ? toggle.onColor : Theme.line
    }
    contentItem: Item {
        DText {
            anchors.centerIn: parent
            visible: toggle.glyph === ""
            text: toggle.label
            font.pixelSize: Theme.textXs
            font.weight: Font.DemiBold
            color: toggle.on ? "#FFFFFF" : Theme.textDim
        }
        Icon {
            anchors.centerIn: parent
            visible: toggle.glyph !== ""
            glyph: toggle.glyph
            size: 12
            color: toggle.on ? "#FFFFFF" : Theme.textDim
        }
    }
    DToolTip { visible: toggle.hovered; text: toggle.tip }
}
