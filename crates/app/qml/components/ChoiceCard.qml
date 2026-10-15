import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Isai

// A large selectable option with an icon, title and description.
AbstractButton {
    id: card

    property string glyph
    property string title
    property string description
    property string badge: ""
    property bool selected: false

    hoverEnabled: true
    focusPolicy: Qt.StrongFocus
    padding: 16
    implicitWidth: 300
    implicitHeight: content.implicitHeight + topPadding + bottomPadding

    background: Rectangle {
        radius: Theme.radiusLg - 2
        color: card.selected ? Theme.accentSoft : card.hovered ? Theme.raised : Theme.surface
        border.width: card.selected ? 2 : 1
        border.color: card.selected ? Theme.accent : card.hovered ? Theme.lineStrong : Theme.line
        Behavior on color { ColorAnimation { duration: Theme.fast } }
        Behavior on border.color { ColorAnimation { duration: Theme.fast } }
        FocusRing { visible: card.visualFocus }
    }

    contentItem: RowLayout {
        id: content
        spacing: 14

        Rectangle {
            Layout.preferredWidth: 42
            Layout.preferredHeight: 42
            Layout.alignment: Qt.AlignTop
            radius: Theme.radiusMd + 1
            color: card.selected ? Theme.accent : Theme.raised
            Behavior on color { ColorAnimation { duration: Theme.fast } }
            Icon {
                anchors.centerIn: parent
                glyph: card.glyph
                size: 21
                color: card.selected ? Theme.accentText : Theme.accent
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 4
            RowLayout {
                spacing: 8
                DText {
                    text: card.title
                    strong: true
                    font.pixelSize: Theme.textLg
                }
                Badge {
                    visible: card.badge !== ""
                    text: card.badge
                    tone: "accent"
                }
            }
            DText {
                Layout.fillWidth: true
                text: card.description
                dim: true
                wrapMode: Text.WordWrap
                lineHeight: 1.15
            }
        }

        // Radio indicator
        Rectangle {
            Layout.preferredWidth: 20
            Layout.preferredHeight: 20
            Layout.alignment: Qt.AlignTop
            radius: 10
            color: "transparent"
            border.width: 2
            border.color: card.selected ? Theme.accent : Theme.lineStrong
            Behavior on border.color { ColorAnimation { duration: Theme.fast } }
            Rectangle {
                anchors.centerIn: parent
                width: 10; height: 10; radius: 5
                color: Theme.accent
                scale: card.selected ? 1 : 0
                Behavior on scale { NumberAnimation { duration: Theme.slow; easing.type: Easing.OutBack } }
            }
        }
    }
}
