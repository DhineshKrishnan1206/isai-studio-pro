import QtQuick
import QtQuick.Controls.Basic
import Isai

// Square icon-only button with a tooltip. `active` shows it as switched on.
Button {
    id: control

    property string glyph
    /// Short text shown instead of an icon, e.g. "1234" for count-in.
    property string label: ""
    property bool filled: false
    property bool active: false
    property color activeColor: Theme.accent
    property string tip: ""
    property real iconSize: 18

    implicitWidth: label === "" ? 34 : Math.max(34, labelMetrics.width + 16)
    implicitHeight: 34
    padding: 0
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus

    readonly property color foreground: !control.enabled ? Theme.textFaint
                                      : control.active ? control.activeColor
                                      : control.hovered ? Theme.text : Theme.textDim

    contentItem: Item {
        Icon {
            anchors.centerIn: parent
            visible: control.label === ""
            glyph: control.glyph
            filled: control.filled || control.active
            size: control.iconSize
            color: control.foreground
        }
        DText {
            anchors.centerIn: parent
            visible: control.label !== ""
            text: control.label
            font.family: Theme.mono
            font.pixelSize: Theme.textXs
            font.weight: Font.DemiBold
            color: control.foreground
        }
    }

    background: Rectangle {
        radius: Theme.radiusMd - 1
        color: control.active ? Theme.tint(control.activeColor, Theme.dark ? 0.18 : 0.14)
             : control.down ? Theme.hover
             : control.hovered ? Theme.raised : "transparent"
        Behavior on color { ColorAnimation { duration: Theme.fast } }
        FocusRing { visible: control.visualFocus }
    }

    TextMetrics { id: labelMetrics; font.family: Theme.mono; font.pixelSize: Theme.textXs; text: control.label }

    DToolTip {
        visible: control.hovered && control.tip !== ""
        text: control.tip
    }
}
