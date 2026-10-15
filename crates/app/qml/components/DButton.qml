import QtQuick
import QtQuick.Controls.Basic
import Isai

// Standard button. variant: "primary" | "secondary" | "ghost" | "danger".
Button {
    id: control

    property string variant: "secondary"
    property string glyph: ""
    property bool large: false

    implicitHeight: large ? 42 : 34
    leftPadding: text === "" ? 0 : (large ? 20 : 14)
    rightPadding: leftPadding
    implicitWidth: text === "" ? implicitHeight : implicitContentWidth + leftPadding + rightPadding
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus
    font.family: Theme.font
    font.pixelSize: large ? Theme.textLg : Theme.textMd
    font.weight: Font.DemiBold

    readonly property color foreground: {
        if (!enabled) return Theme.textFaint
        if (variant === "primary") return Theme.accentText
        if (variant === "danger") return Theme.danger
        return Theme.text
    }
    readonly property color fill: {
        if (variant === "primary")
            return !enabled ? Theme.raised : down ? Qt.darker(Theme.accent, 1.12) : hovered ? Theme.accentHover : Theme.accent
        if (variant === "ghost")
            return down ? Theme.hover : hovered ? Theme.raised : "transparent"
        if (variant === "danger")
            return hovered ? Theme.tint(Theme.danger, 0.14) : Theme.tint(Theme.danger, 0.08)
        return down ? Theme.lineStrong : hovered ? Theme.hover : Theme.raised
    }

    scale: down ? 0.97 : 1
    Behavior on scale { NumberAnimation { duration: Theme.fast; easing.type: Easing.OutCubic } }

    contentItem: Item {
        implicitWidth: row.implicitWidth
        implicitHeight: row.implicitHeight
        Row {
            id: row
            anchors.centerIn: parent
            spacing: 8
            Icon {
                visible: control.glyph !== ""
                glyph: control.glyph
                size: control.large ? 18 : 16
                color: control.foreground
                anchors.verticalCenter: parent.verticalCenter
            }
            DText {
                visible: control.text !== ""
                text: control.text
                font: control.font
                color: control.foreground
                anchors.verticalCenter: parent.verticalCenter
            }
        }
    }

    background: Rectangle {
        radius: control.large ? Theme.radiusMd + 1 : Theme.radiusMd - 1
        color: control.fill
        border.width: control.variant === "secondary" ? 1 : 0
        border.color: Theme.line
        Behavior on color { ColorAnimation { duration: Theme.fast } }

        // Subtle top highlight that gives primary buttons depth.
        Rectangle {
            anchors.fill: parent
            radius: parent.radius
            visible: control.variant === "primary" && control.enabled && Theme.effects
            gradient: Gradient {
                GradientStop { position: 0.0; color: Qt.rgba(1, 1, 1, 0.14) }
                GradientStop { position: 0.6; color: Qt.rgba(1, 1, 1, 0.0) }
            }
        }

        FocusRing { visible: control.visualFocus }
    }
}
