import QtQuick
import QtQuick.Layouts
import Isai

// One setting: title and description on the left, its control on the right.
Item {
    id: row

    property string title
    property string description: ""
    property bool divider: true
    default property alias control: slot.data

    Layout.fillWidth: true
    implicitHeight: Math.max(texts.implicitHeight, slot.implicitHeight) + 28

    RowLayout {
        anchors.fill: parent
        anchors.topMargin: 14
        anchors.bottomMargin: 14
        spacing: 20

        ColumnLayout {
            id: texts
            Layout.fillWidth: true
            spacing: 3
            DText { text: row.title; strong: true }
            DText {
                Layout.fillWidth: true
                visible: row.description !== ""
                text: row.description
                dim: true
                font.pixelSize: Theme.textSm
                wrapMode: Text.WordWrap
            }
        }

        Item {
            id: slot
            Layout.alignment: Qt.AlignVCenter | Qt.AlignRight
            implicitWidth: childrenRect.width
            implicitHeight: childrenRect.height
        }
    }

    Rectangle {
        visible: row.divider
        anchors.bottom: parent.bottom
        width: parent.width
        height: 1
        color: Theme.line
    }
}
