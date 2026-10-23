import QtQuick
import QtQuick.Layouts
import Isai

// One slot on a channel strip, as on Logic's channel: a label on the left, its
// value on the right. A lit slot has something in it.
Rectangle {
    id: slot
    property string label
    property string value: ""
    property bool lit: false
    property color tint: Theme.accent

    Layout.fillWidth: true
    implicitHeight: 26
    radius: Theme.radiusSm
    color: lit ? Theme.tint(tint, 0.16) : Theme.raised
    border.color: lit ? Theme.tint(tint, 0.6) : Theme.line

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 9
        anchors.rightMargin: 9
        spacing: 6
        DText {
            text: slot.label
            font.pixelSize: Theme.textXs
            font.weight: Font.DemiBold
            color: slot.lit ? Theme.text : Theme.textDim
        }
        Item { Layout.fillWidth: true }
        DText {
            text: slot.value
            font.pixelSize: Theme.textXs
            font.family: Theme.mono
            dim: true
            elide: Text.ElideRight
            Layout.maximumWidth: 120
        }
    }
}
