import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Effects
import Isai

// A small floating grid of choices, e.g. time signatures or keys.
// options: [{ value, label }]; emits `picked(value)` and closes.
Popup {
    id: popup
    property var options: []
    property var current
    property int columns: 4
    property string title: ""
    signal picked(var value)

    padding: 10
    modal: false
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    background: GlassPanel { backdrop: Theme.backdrop; radius: Theme.radiusLg; clarity: 0.3 }
    enter: Transition { NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.fast } }
    exit: Transition { NumberAnimation { property: "opacity"; to: 0; duration: Theme.fast } }

    contentItem: Column {
        spacing: 8
        DText {
            visible: popup.title !== ""
            text: popup.title
            faint: true
            font.pixelSize: Theme.textXs
            font.weight: Font.DemiBold
            font.letterSpacing: 1
        }
        Grid {
            columns: popup.columns
            spacing: 4
            Repeater {
                model: popup.options
                delegate: AbstractButton {
                    id: cell
                    required property var modelData
                    readonly property bool selected: popup.current === modelData.value
                    width: 64; height: 30
                    hoverEnabled: true
                    onClicked: { popup.picked(modelData.value); popup.close() }
                    background: Rectangle {
                        radius: Theme.radiusSm
                        color: cell.selected ? Theme.accent : cell.hovered ? Theme.hover : Theme.raised
                    }
                    contentItem: DText {
                        text: cell.modelData.label
                        horizontalAlignment: Text.AlignHCenter
                        font.family: Theme.mono
                        font.pixelSize: Theme.textSm
                        color: cell.selected ? Theme.accentText : Theme.text
                    }
                }
            }
        }
    }
}
