import QtQuick
import QtQuick.Layouts
import Isai

// "Save changes?" before closing or replacing a project with unsaved work.
// Emits save / discard; Cancel just closes.
Item {
    id: dialog
    /// What the frosted glass blurs: the page behind the dialog.
    property Item backdrop: null
    property string projectName: ""
    property bool shown: false
    signal save()
    signal discard()

    function open() { shown = true }
    function close() { shown = false }

    anchors.fill: parent
    visible: opacity > 0
    opacity: shown ? 1 : 0
    z: 100
    Behavior on opacity { NumberAnimation { duration: Theme.fast } }

    Shortcut { sequence: "Escape"; enabled: dialog.shown; onActivated: dialog.close() }

    Rectangle {
        anchors.fill: parent
        color: Theme.overlay
        MouseArea { anchors.fill: parent }
    }

    GlassPanel {
        backdrop: dialog.backdrop
        clarity: 0.3
        radius: Theme.radiusXl
        anchors.centerIn: parent
        width: 440
        height: content.implicitHeight + 48
        scale: dialog.shown ? 1 : 0.96
        Behavior on scale { NumberAnimation { duration: Theme.slow; easing.type: Easing.OutCubic } }

        ColumnLayout {
            id: content
            anchors.fill: parent
            anchors.margins: 24
            spacing: 10
            RowLayout {
                spacing: 12
                Icon { glyph: Icons.warning; size: 26; color: Theme.warning }
                DText {
                    Layout.fillWidth: true
                    text: "Save changes to “" + dialog.projectName + "”?"
                    strong: true
                    font.pixelSize: Theme.textLg
                    wrapMode: Text.WordWrap
                }
            }
            DText {
                Layout.fillWidth: true
                text: "If you don't save, your changes since the last save will be lost."
                dim: true
                wrapMode: Text.WordWrap
            }
            RowLayout {
                Layout.topMargin: 10
                spacing: 8
                DButton { variant: "danger"; text: "Don't Save"; onClicked: { dialog.close(); dialog.discard() } }
                Item { Layout.fillWidth: true }
                DButton { variant: "ghost"; text: "Cancel"; onClicked: dialog.close() }
                DButton { variant: "primary"; text: "Save"; onClicked: { dialog.close(); dialog.save() } }
            }
        }
    }
}
