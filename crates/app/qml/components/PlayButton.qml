import QtQuick
import QtQuick.Controls.Basic
import Isai

// The big round play/stop button, with a soft pulse while playing.
AbstractButton {
    id: button
    property bool playing: false

    implicitWidth: 42
    implicitHeight: 42
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus

    background: Item {
        Rectangle {
            id: pulse
            anchors.centerIn: parent
            width: parent.width; height: parent.height; radius: width / 2
            color: Theme.playActive
            opacity: 0
            visible: Theme.motionMs > 0
            SequentialAnimation on scale {
                running: button.playing && pulse.visible
                loops: Animation.Infinite
                NumberAnimation { from: 1; to: 1.45; duration: 1400; easing.type: Easing.OutCubic }
            }
            SequentialAnimation on opacity {
                running: button.playing && pulse.visible
                loops: Animation.Infinite
                NumberAnimation { from: 0.35; to: 0; duration: 1400; easing.type: Easing.OutCubic }
            }
        }
        Rectangle {
            anchors.fill: parent
            radius: width / 2
            readonly property color base: button.playing ? Theme.playActive : Theme.playIdle
            color: !button.enabled ? Theme.raised
                 : button.down ? Qt.darker(base, 1.12)
                 : button.hovered ? Qt.lighter(base, 1.1) : base
            border.width: Theme.studio && !button.playing ? 1 : 0
            border.color: Theme.lineStrong
            scale: button.down ? 0.94 : 1
            Behavior on scale { NumberAnimation { duration: Theme.fast } }
            Behavior on color { ColorAnimation { duration: Theme.fast } }
            FocusRing { visible: button.visualFocus }
        }
    }

    contentItem: Icon {
        glyph: button.playing ? Icons.stop : Icons.play
        filled: true
        size: 19
        color: !button.enabled ? Theme.textFaint : button.playing ? "#FFFFFF" : Theme.playIdleIcon
        // Optical centring: the play triangle looks off-centre without a nudge.
        leftPadding: button.playing ? 0 : 2
    }

    DToolTip {
        visible: button.hovered
        text: button.playing ? "Stop (Space)" : "Play (Space)"
    }
}
