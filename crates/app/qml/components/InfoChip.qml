import QtQuick
import Isai

// Icon + label chip used for hardware facts in setup.
Rectangle {
    property alias glyph: icon.glyph
    property alias text: label.text

    implicitWidth: row.implicitWidth + 20
    implicitHeight: 30
    radius: 15
    color: Theme.raised
    border.color: Theme.line

    Row {
        id: row
        anchors.centerIn: parent
        spacing: 7
        Icon { id: icon; size: 15; color: Theme.accent; anchors.verticalCenter: parent.verticalCenter }
        DText { id: label; font.pixelSize: Theme.textSm; anchors.verticalCenter: parent.verticalCenter }
    }
}
