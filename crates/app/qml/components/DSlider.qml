import QtQuick
import QtQuick.Controls.Basic
import Isai

Slider {
    id: slider
    hoverEnabled: true
    implicitWidth: 120
    implicitHeight: 24

    background: Rectangle {
        x: slider.leftPadding
        y: slider.topPadding + slider.availableHeight / 2 - height / 2
        width: slider.availableWidth
        height: 4
        radius: 2
        color: Theme.lineStrong
        Rectangle {
            width: slider.visualPosition * parent.width
            height: parent.height
            radius: 2
            color: Theme.accent
        }
    }

    handle: Rectangle {
        x: slider.leftPadding + slider.visualPosition * (slider.availableWidth - width)
        y: slider.topPadding + slider.availableHeight / 2 - height / 2
        width: 14; height: 14; radius: 7
        color: "#FFFFFF"
        border.color: Theme.accent
        border.width: 2
        scale: slider.pressed ? 1.15 : slider.hovered ? 1.08 : 1
        Behavior on scale { NumberAnimation { duration: Theme.fast } }
    }
}
