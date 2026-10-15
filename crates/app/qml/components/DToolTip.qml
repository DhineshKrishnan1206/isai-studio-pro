import QtQuick
import QtQuick.Controls.Basic
import Isai

ToolTip {
    id: tip
    delay: 450
    padding: 0
    font.family: Theme.font
    font.pixelSize: Theme.textSm

    contentItem: DText {
        text: tip.text
        font: tip.font
        color: Theme.dark ? "#0A0B0F" : "#FFFFFF"
        leftPadding: 9; rightPadding: 9; topPadding: 5; bottomPadding: 5
    }
    background: Rectangle {
        radius: Theme.radiusSm
        color: Theme.dark ? "#E9EBF2" : "#1A1D26"
    }
    enter: Transition { NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.fast } }
    exit: Transition { NumberAnimation { property: "opacity"; to: 0; duration: Theme.fast } }
}
