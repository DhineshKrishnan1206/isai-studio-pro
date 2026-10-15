import QtQuick
import Isai

// Keyboard-focus outline. Place inside a control's background.
Rectangle {
    anchors.fill: parent
    anchors.margins: -3
    radius: parent.radius + 3
    color: "transparent"
    border.width: 2
    border.color: Theme.tint(Theme.accent, 0.7)
}
