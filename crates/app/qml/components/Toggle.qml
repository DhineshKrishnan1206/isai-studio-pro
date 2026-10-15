import QtQuick
import QtQuick.Controls.Basic
import Isai

// On/off switch. Listen to `toggled` for user changes.
AbstractButton {
    id: toggle
    checkable: true
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus
    implicitWidth: 40
    implicitHeight: 24

    background: Rectangle {
        radius: height / 2
        color: toggle.checked ? Theme.accent : (toggle.hovered ? Theme.lineStrong : Theme.line)
        Behavior on color { ColorAnimation { duration: Theme.fast } }

        Rectangle {
            width: 18; height: 18; radius: 9
            y: 3
            x: toggle.checked ? parent.width - width - 3 : 3
            color: "#FFFFFF"
            Behavior on x { NumberAnimation { duration: Theme.slow; easing.type: Easing.OutCubic } }
        }
        FocusRing { visible: toggle.visualFocus }
    }
}
