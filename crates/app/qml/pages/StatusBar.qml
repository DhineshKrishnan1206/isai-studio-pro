import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Isai

Rectangle {
    id: bar
    property var app
    implicitHeight: 34
    color: Theme.panel

    Rectangle { anchors.top: parent.top; width: parent.width; height: 1; color: Theme.line }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 14
        anchors.rightMargin: 14
        spacing: 10

        // Good news fades away on its own; problems stay until dismissed.
        Timer {
            running: bar.app.statusOk && bar.app.status !== ""
            interval: 4000
            onTriggered: bar.app.clearStatus()
        }

        // State, most important first: loading, error, no audio, ready.
        Spinner { visible: bar.app.loading || bar.app.stemRegion >= 0; size: 14 }
        DText {
            visible: bar.app.stemRegion >= 0
            text: bar.app.stemStage + " " + Math.round(Math.max(0, bar.app.stemProgress) * 100) + "%"
            dim: true
            font.pixelSize: Theme.textSm
        }
        Rectangle {
            visible: !bar.app.loading && bar.app.status === "" && bar.app.audioOk
            width: 7; height: 7; radius: 3.5
            color: bar.app.recording ? Theme.record : bar.app.playing ? Theme.accent : Theme.success
        }
        Icon {
            visible: !bar.app.loading && (bar.app.status !== "" || !bar.app.audioOk)
            glyph: bar.app.status === "" ? Icons.speakerX : bar.app.statusOk ? Icons.checkCircle : Icons.warning
            size: 15
            color: bar.app.status === "" ? Theme.warning : bar.app.statusOk ? Theme.success : Theme.danger
        }
        DText {
            Layout.fillWidth: true
            elide: Text.ElideRight
            font.pixelSize: Theme.textSm
            dim: true
            text: bar.app.recording ? "Recording" + (bar.app.inputName !== "" ? " from " + bar.app.inputName : "")
                : bar.app.loading ? "Importing audio…"
                : bar.app.status !== "" ? bar.app.status
                : !bar.app.audioOk ? (bar.app.audioError || "Sound is off")
                : bar.app.playing ? "Playing"
                : "Ready"
        }
        DButton {
            visible: !bar.app.loading && bar.app.status === "" && !bar.app.audioOk
            variant: "ghost"
            implicitHeight: 24
            text: "Retry"
            onClicked: bar.app.retryAudio()
        }
        IconButton {
            visible: bar.app.status !== ""
            implicitWidth: 24; implicitHeight: 24
            iconSize: 14
            glyph: Icons.x
            tip: "Dismiss"
            onClicked: bar.app.clearStatus()
        }

        // Eco mode is on: click to switch it off.
        AbstractButton {
            id: ecoBadge
            visible: bar.app.eco
            implicitWidth: ecoText.implicitWidth + 16
            implicitHeight: 20
            hoverEnabled: true
            onClicked: bar.app.setBool("eco", false)
            background: Rectangle { radius: 10; color: Theme.tint(Theme.success, ecoBadge.hovered ? 0.3 : 0.18) }
            contentItem: Item { DText { id: ecoText; anchors.centerIn: parent; text: "ECO"; font.pixelSize: 10; font.weight: Font.Bold; font.letterSpacing: 1; color: Theme.success } }
            DToolTip { visible: ecoBadge.hovered; text: "Eco mode: instruments in their lightest quality. Click to turn it off." }
        }
        // How busy the audio is (the share of each buffer's time it takes).
        DText {
            visible: bar.app.audioOk && (bar.app.view === "pro" || bar.app.cpuLoad > 0.6)
            text: "CPU " + Math.round(bar.app.cpuLoad * 100) + "%"
            font.family: Theme.mono
            font.pixelSize: Theme.textSm
            color: bar.app.cpuLoad > 0.8 ? Theme.danger : bar.app.cpuLoad > 0.6 ? Theme.warning : Theme.textFaint
        }
        DText {
            visible: bar.app.view === "pro" && bar.app.audioOk
            text: bar.app.deviceName + "  ·  " + bar.app.sampleRate + " Hz  ·  "
                  + (bar.app.actualBuffer > 0 ? bar.app.actualBuffer + " buffer" : "default buffer")
            faint: true
            font.pixelSize: Theme.textSm
        }
        LevelMeter {
            leftLevel: bar.app.meterL
            rightLevel: bar.app.meterR
            meterWidth: 120
        }
    }
}
