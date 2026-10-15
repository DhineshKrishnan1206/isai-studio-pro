import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Isai

// The mastering assistant: pick where the song is going, let it listen to
// the whole mix, then apply its master chain (an EQ, a gentle compressor and
// a limiter) in one step. Undo takes it all back.
Popup {
    id: dialog
    property var app
    property Item backdrop: null

    readonly property var targets: [
        { value: -14, label: "Streaming", hint: "Spotify, YouTube, Apple Music" },
        { value: -9, label: "Loud", hint: "Club, EDM, hip-hop" },
        { value: -16, label: "Gentle", hint: "Podcasts, acoustic, ballads" }
    ]
    property int target: -14
    property bool applied: false
    readonly property var report: {
        try { return app.masterReport ? JSON.parse(app.masterReport) : null } catch (e) { return null }
    }
    onOpened: applied = false
    onClosed: if (app.mastering) app.cancelMaster()

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: 560
    padding: 22
    modal: true
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    Overlay.modal: Rectangle { color: Qt.rgba(0, 0, 0, 0.45) }
    background: GlassPanel { backdrop: dialog.backdrop; radius: Theme.radiusXl; clarity: 0.25 }
    enter: Transition { NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.fast } }
    exit: Transition { NumberAnimation { property: "opacity"; to: 0; duration: Theme.fast } }

    function fmt(v, unit) { return (v > 0 && unit !== " LUFS" ? "+" : "") + v.toFixed(1) + unit }

    contentItem: ColumnLayout {
        spacing: 14
        RowLayout {
            spacing: 10
            Icon { glyph: Icons.magicWand; size: 18; color: Theme.accent }
            DText { text: "Mastering assistant"; strong: true; font.pixelSize: Theme.textLg }
        }
        DText {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            dim: true
            font.pixelSize: Theme.textSm
            text: "It listens to your whole mix, then sets up the master: a touch of tone, a gentle compressor to hold it together, and a limiter to reach the loudness your song needs, without clipping."
        }

        DText { text: "WHERE IS IT GOING?"; faint: true; font.pixelSize: Theme.textXs; font.weight: Font.DemiBold; font.letterSpacing: 1 }
        RowLayout {
            spacing: 8
            Repeater {
                model: dialog.targets
                delegate: AbstractButton {
                    id: choice
                    required property var modelData
                    readonly property bool on: dialog.target === modelData.value
                    Layout.fillWidth: true
                    implicitHeight: 58
                    hoverEnabled: true
                    onClicked: dialog.target = modelData.value
                    background: Rectangle {
                        radius: Theme.radiusMd
                        color: choice.on ? Theme.accentSoft : choice.hovered ? Theme.hover : Theme.tint(Theme.sunken, 0.6)
                        border.color: choice.on ? Theme.accent : Theme.line
                    }
                    contentItem: Column {
                        leftPadding: 12; topPadding: 9
                        spacing: 2
                        DText { text: choice.modelData.label + "  ·  " + choice.modelData.value + " LUFS"; strong: choice.on; font.pixelSize: Theme.textSm }
                        DText { text: choice.modelData.hint; faint: true; font.pixelSize: Theme.textXs }
                    }
                }
            }
        }

        // Results
        Rectangle {
            visible: !!dialog.report || dialog.app.mastering
            Layout.fillWidth: true
            implicitHeight: dialog.app.mastering ? 70 : resultCol.implicitHeight + 24
            radius: Theme.radiusMd
            color: Theme.tint(Theme.sunken, 0.7)
            border.color: Theme.line
            Row {
                visible: dialog.app.mastering
                anchors.centerIn: parent
                spacing: 10
                Spinner { size: 16 }
                DText { text: "Listening to the whole song…"; dim: true }
            }
            ColumnLayout {
                id: resultCol
                visible: !dialog.app.mastering && !!dialog.report
                x: 14; y: 12
                width: parent.width - 28
                spacing: 10
                GridLayout {
                    columns: 3
                    columnSpacing: 18
                    rowSpacing: 4
                    DText { text: ""; faint: true }
                    DText { text: "NOW"; faint: true; font.pixelSize: Theme.textXs; font.weight: Font.DemiBold }
                    DText { text: "AFTER"; faint: true; font.pixelSize: Theme.textXs; font.weight: Font.DemiBold }
                    DText { text: "Loudness"; dim: true; font.pixelSize: Theme.textSm }
                    DText { text: dialog.report ? dialog.fmt(dialog.report.before.lufs, " LUFS") : ""; font.family: Theme.mono; font.pixelSize: Theme.textSm }
                    DText { text: dialog.report ? dialog.fmt(dialog.report.after.lufs, " LUFS") : ""; font.family: Theme.mono; font.pixelSize: Theme.textSm; color: Theme.success }
                    DText { text: "Peak"; dim: true; font.pixelSize: Theme.textSm }
                    DText { text: dialog.report ? dialog.fmt(dialog.report.before.peak, " dB") : ""; font.family: Theme.mono; font.pixelSize: Theme.textSm; color: dialog.report && dialog.report.before.peak > -0.1 ? Theme.danger : Theme.text }
                    DText { text: dialog.report ? dialog.fmt(dialog.report.after.peak, " dB") : ""; font.family: Theme.mono; font.pixelSize: Theme.textSm }
                    DText { text: "Bass share"; dim: true; font.pixelSize: Theme.textSm }
                    DText { text: dialog.report ? dialog.fmt(dialog.report.before.low, " dB") : ""; font.family: Theme.mono; font.pixelSize: Theme.textSm }
                    DText { text: dialog.report ? dialog.fmt(dialog.report.after.low, " dB") : ""; font.family: Theme.mono; font.pixelSize: Theme.textSm }
                }
                DText { text: "WHAT IT WILL DO"; faint: true; font.pixelSize: Theme.textXs; font.weight: Font.DemiBold; font.letterSpacing: 1 }
                Repeater {
                    model: dialog.report ? dialog.report.changes : []
                    delegate: DText {
                        required property string modelData
                        Layout.fillWidth: true
                        wrapMode: Text.WordWrap
                        text: "•  " + modelData
                        font.pixelSize: Theme.textSm
                    }
                }
                DText {
                    visible: !!dialog.report && dialog.report.target !== dialog.target
                    Layout.fillWidth: true
                    wrapMode: Text.WordWrap
                    text: "You picked a different target: listen again to update this."
                    color: Theme.warning
                    font.pixelSize: Theme.textXs
                }
            }
        }

        DText {
            visible: dialog.applied
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            text: "Done. Play the song to hear it. To compare, turn the master effects off in the mixer (F9); " + (Qt.platform.os === "osx" ? "⌘Z" : "Ctrl+Z") + " takes it all back."
            color: Theme.success
            font.pixelSize: Theme.textSm
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            DText {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                text: "Applying replaces the effects on the master."
                faint: true
                font.pixelSize: Theme.textXs
            }
            DButton { variant: "ghost"; text: "Close"; onClicked: dialog.close() }
            DButton {
                variant: dialog.report && dialog.report.target === dialog.target ? "ghost" : "primary"
                text: dialog.report ? "Listen again" : "Listen and suggest"
                enabled: !dialog.app.mastering
                onClicked: { dialog.applied = false; dialog.app.analyseMaster(dialog.target) }
            }
            DButton {
                visible: !!dialog.report
                variant: "primary"
                text: "Apply to master"
                enabled: !dialog.app.mastering && !dialog.applied
                onClicked: { dialog.app.applyMasterPlan(); dialog.applied = true }
            }
        }
    }
}
