import QtQuick
import QtQuick.Layouts
import Isai

// Inline message with an icon and optional actions.
// tone: "info" | "warning" | "danger" | "success".
Rectangle {
    id: banner
    property string tone: "info"
    property string glyph: tone === "success" ? Icons.checkCircle : tone === "info" ? Icons.info : Icons.warning
    property alias text: message.text
    default property alias actions: actionRow.data

    readonly property color toneColor: tone === "warning" ? Theme.warning
                                     : tone === "danger" ? Theme.danger
                                     : tone === "success" ? Theme.success
                                     : Theme.accent

    implicitHeight: layout.implicitHeight + 20
    radius: Theme.radiusMd
    color: Theme.tint(toneColor, Theme.dark ? 0.12 : 0.08)
    border.color: Theme.tint(toneColor, 0.35)

    RowLayout {
        id: layout
        anchors.fill: parent
        anchors.leftMargin: 14
        anchors.rightMargin: 10
        spacing: 10
        Icon { glyph: banner.glyph; color: banner.toneColor; size: 18 }
        DText {
            id: message
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
        }
        Row { id: actionRow; spacing: 6 }
    }
}
