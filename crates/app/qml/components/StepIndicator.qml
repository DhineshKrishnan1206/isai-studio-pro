import QtQuick
import QtQuick.Layouts
import Isai

// Numbered progress steps joined by lines. Finished steps show a check mark.
RowLayout {
    id: steps
    property var labels: []
    property int current: 0
    spacing: 0

    Repeater {
        model: steps.labels
        delegate: RowLayout {
            id: step
            required property string modelData
            required property int index
            readonly property bool done: index < steps.current
            readonly property bool active: index === steps.current
            Layout.fillWidth: index < steps.labels.length - 1
            spacing: 8

            Rectangle {
                Layout.preferredWidth: 26
                Layout.preferredHeight: 26
                radius: 13
                color: step.done ? Theme.accent : step.active ? Theme.accentSoft : "transparent"
                border.width: step.done ? 0 : 2
                border.color: step.active ? Theme.accent : Theme.lineStrong
                Behavior on color { ColorAnimation { duration: Theme.slow } }

                Icon {
                    anchors.centerIn: parent
                    visible: step.done
                    glyph: Icons.check
                    size: 14
                    color: Theme.accentText
                }
                DText {
                    anchors.centerIn: parent
                    visible: !step.done
                    text: step.index + 1
                    font.pixelSize: Theme.textSm
                    font.weight: Font.DemiBold
                    color: step.active ? Theme.accent : Theme.textFaint
                }
            }

            DText {
                text: step.modelData
                font.pixelSize: Theme.textSm
                strong: step.active
                color: step.active ? Theme.text : step.done ? Theme.textDim : Theme.textFaint
            }

            // Connector to the next step
            Item {
                visible: step.index < steps.labels.length - 1
                Layout.fillWidth: true
                Layout.leftMargin: 4
                Layout.rightMargin: 12
                implicitHeight: 2
                Rectangle { anchors.fill: parent; radius: 1; color: Theme.line }
                Rectangle {
                    height: parent.height
                    radius: 1
                    width: step.done ? parent.width : 0
                    color: Theme.accent
                    Behavior on width { NumberAnimation { duration: Theme.slow * 2; easing.type: Easing.OutCubic } }
                }
            }
        }
    }
}
