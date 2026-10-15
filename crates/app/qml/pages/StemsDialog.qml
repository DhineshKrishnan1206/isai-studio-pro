import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Isai

// Splitting an audio region into vocals, drums, bass and other. The first
// time, it explains the one-off model download (Deezer's Spleeter, MIT).
// While it runs it shows progress; it can be closed and the split goes on.
Popup {
    id: dialog
    property var app
    property Item backdrop: null
    property int regionId: -1
    property string regionName: ""
    readonly property bool running: app.stemRegion >= 0
    readonly property bool needsModel: !app.stemModelReady()

    function openFor(id, name) { regionId = id; regionName = name; open() }

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: 480
    padding: 22
    modal: true
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    Overlay.modal: Rectangle { color: Qt.rgba(0, 0, 0, 0.45) }
    background: GlassPanel { backdrop: dialog.backdrop; radius: Theme.radiusXl; clarity: 0.25 }
    enter: Transition { NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.fast } }
    exit: Transition { NumberAnimation { property: "opacity"; to: 0; duration: Theme.fast } }

    contentItem: ColumnLayout {
        spacing: 14
        RowLayout {
            spacing: 10
            Icon { glyph: Icons.waveform; size: 18; color: Theme.accent }
            DText { text: "Split into stems"; strong: true; font.pixelSize: Theme.textLg }
        }
        DText {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            dim: true
            font.pixelSize: Theme.textSm
            text: "Separates “" + dialog.regionName + "” into four new tracks: vocals, drums, bass and other. The original track is muted, so you can solo, remix or remove any part."
        }
        Rectangle {
            visible: dialog.needsModel && !dialog.running
            Layout.fillWidth: true
            implicitHeight: dl.implicitHeight + 24
            radius: Theme.radiusMd
            color: Theme.tint(Theme.sunken, 0.7)
            border.color: Theme.line
            DText {
                id: dl
                x: 12; y: 12
                width: parent.width - 24
                wrapMode: Text.WordWrap
                font.pixelSize: Theme.textSm
                text: "The first time, Isai downloads the separation model: 146 MB from Deezer's Spleeter project (MIT licence), checked and kept as 79 MB. After that it works offline."
            }
        }
        ColumnLayout {
            visible: dialog.running
            Layout.fillWidth: true
            spacing: 6
            RowLayout {
                DText { text: dialog.app.stemStage; font.pixelSize: Theme.textSm }
                Item { Layout.fillWidth: true }
                DText { text: Math.round(Math.max(0, dialog.app.stemProgress) * 100) + "%"; font.family: Theme.mono; font.pixelSize: Theme.textSm; dim: true }
            }
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: 6
                radius: 3
                color: Theme.sunken
                Rectangle {
                    width: parent.width * Math.max(0, Math.min(1, dialog.app.stemProgress))
                    height: parent.height
                    radius: 3
                    color: Theme.accent
                    Behavior on width { NumberAnimation { duration: Theme.fast } }
                }
            }
            DText {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                faint: true
                font.pixelSize: Theme.textXs
                text: "You can close this window and keep working; the new tracks appear when it's done."
            }
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            Item { Layout.fillWidth: true }
            DButton {
                visible: dialog.running
                variant: "danger"
                text: "Stop"
                onClicked: dialog.app.cancelStems()
            }
            DButton { variant: "ghost"; text: dialog.running ? "Close" : "Cancel"; onClicked: dialog.close() }
            DButton {
                visible: !dialog.running
                variant: "primary"
                text: dialog.needsModel ? "Download and split" : "Split"
                onClicked: dialog.app.splitStems(dialog.regionId)
            }
        }
    }
}
