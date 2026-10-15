import QtQuick
import Isai

// Small pill label. tone: "accent" | "success" | "warning" | "danger" | "neutral".
Rectangle {
    id: badge
    property alias text: label.text
    property string tone: "neutral"

    readonly property color toneColor: tone === "accent" ? Theme.accent
                                     : tone === "success" ? Theme.success
                                     : tone === "warning" ? Theme.warning
                                     : tone === "danger" ? Theme.danger
                                     : Theme.textDim

    implicitWidth: label.implicitWidth + 14
    implicitHeight: 20
    radius: 10
    color: Theme.tint(toneColor, Theme.dark ? 0.16 : 0.12)

    DText {
        id: label
        anchors.centerIn: parent
        font.pixelSize: Theme.textXs
        font.weight: Font.DemiBold
        color: badge.toneColor
    }
}
