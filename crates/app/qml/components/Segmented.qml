import QtQuick
import QtQuick.Controls.Basic
import Isai

// A row of mutually exclusive options with a sliding highlight.
// options: [{ value: ..., label: "...", glyph: optional }]
Rectangle {
    id: seg

    property var options: []
    property var value
    property int minItemWidth: 56
    signal activated(var value)

    readonly property int currentIndex: {
        for (let i = 0; i < options.length; ++i)
            if (options[i].value === value) return i
        return -1
    }

    implicitWidth: row.implicitWidth + 6
    implicitHeight: 34
    radius: Theme.radiusMd
    color: Theme.sunken
    border.color: Theme.line

    Rectangle {
        id: thumb
        // Mentioning repeater.count re-evaluates this once the items exist.
        readonly property Item target: repeater.count > 0 && seg.currentIndex >= 0 ? repeater.itemAt(seg.currentIndex) : null
        visible: target !== null
        x: target ? row.x + target.x : 0
        y: 3
        width: target ? target.width : 0
        height: seg.height - 6
        radius: Theme.radiusMd - 3
        color: Theme.dark ? Theme.hover : Theme.surface
        border.color: Theme.dark ? Theme.lineStrong : Theme.line
        Behavior on x { NumberAnimation { duration: Theme.slow; easing.type: Easing.OutCubic } }
        Behavior on width { NumberAnimation { duration: Theme.slow; easing.type: Easing.OutCubic } }
    }

    Row {
        id: row
        x: 3
        anchors.verticalCenter: parent.verticalCenter
        Repeater {
            id: repeater
            model: seg.options
            delegate: AbstractButton {
                id: item
                required property var modelData
                required property int index
                readonly property bool current: seg.currentIndex === index

                width: Math.max(seg.minItemWidth, inner.implicitWidth + 22)
                height: seg.height - 6
                hoverEnabled: true
                focusPolicy: Qt.StrongFocus
                onClicked: seg.activated(modelData.value)

                contentItem: Item {
                    Row {
                        id: inner
                        anchors.centerIn: parent
                        spacing: 6
                        Icon {
                            visible: item.modelData.glyph !== undefined
                            glyph: item.modelData.glyph || ""
                            size: 15
                            color: label.color
                            anchors.verticalCenter: parent.verticalCenter
                        }
                        DText {
                            id: label
                            text: item.modelData.label
                            font.weight: item.current ? Font.DemiBold : Font.Normal
                            color: item.current || item.hovered ? Theme.text : Theme.textDim
                            anchors.verticalCenter: parent.verticalCenter
                        }
                    }
                }
                background: Item {
                    FocusRing { visible: item.visualFocus; radius: Theme.radiusMd - 3 }
                }
            }
        }
    }
}
