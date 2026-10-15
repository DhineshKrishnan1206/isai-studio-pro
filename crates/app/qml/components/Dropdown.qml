import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Effects
import QtQuick.Layouts
import Isai

ComboBox {
    id: box

    implicitWidth: 260
    implicitHeight: 34
    hoverEnabled: true
    font.family: Theme.font
    font.pixelSize: Theme.textMd

    contentItem: DText {
        leftPadding: 12
        rightPadding: 32
        text: box.displayText
        elide: Text.ElideRight
    }

    indicator: Icon {
        x: box.width - width - 10
        y: (box.height - height) / 2
        glyph: Icons.caretDown
        size: 14
        color: Theme.textDim
        rotation: box.popup.visible ? 180 : 0
        Behavior on rotation { NumberAnimation { duration: Theme.slow; easing.type: Easing.OutCubic } }
    }

    background: Rectangle {
        radius: Theme.radiusMd - 1
        color: box.hovered ? Theme.hover : Theme.raised
        border.color: box.popup.visible ? Theme.accent : Theme.line
        Behavior on color { ColorAnimation { duration: Theme.fast } }
        FocusRing { visible: box.visualFocus }
    }

    delegate: ItemDelegate {
        id: option
        required property var modelData
        required property int index
        width: ListView.view ? ListView.view.width : box.width
        height: 34
        hoverEnabled: true
        highlighted: box.highlightedIndex === index

        contentItem: RowLayout {
            spacing: 8
            DText {
                Layout.fillWidth: true
                text: option.modelData
                elide: Text.ElideRight
                font.weight: box.currentIndex === option.index ? Font.DemiBold : Font.Normal
            }
            Icon {
                visible: box.currentIndex === option.index
                glyph: Icons.check
                size: 15
                color: Theme.accent
            }
        }
        background: Rectangle {
            radius: Theme.radiusSm
            color: option.highlighted ? Theme.hover : "transparent"
        }
    }

    popup: Popup {
        y: box.height + 6
        width: box.width
        padding: 5
        implicitHeight: Math.min(list.contentHeight + 10, 320)

        contentItem: ListView {
            id: list
            clip: true
            implicitHeight: contentHeight
            model: box.popup.visible ? box.delegateModel : null
            currentIndex: box.highlightedIndex
            boundsBehavior: Flickable.StopAtBounds
        }

        background: GlassPanel { backdrop: Theme.backdrop; radius: Theme.radiusMd + 1; clarity: 0.3; glows: false }

        enter: Transition {
            NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.fast }
            NumberAnimation { property: "y"; from: box.height; to: box.height + 6; duration: Theme.slow; easing.type: Easing.OutCubic }
        }
        exit: Transition { NumberAnimation { property: "opacity"; to: 0; duration: Theme.fast } }
    }
}
