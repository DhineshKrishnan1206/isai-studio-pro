import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Isai

// The control bar: project, transport, LCD, metronome, and quick actions.
Rectangle {
    id: bar
    property var app
    signal importRequested()
    signal importVideoRequested()
    signal settingsRequested()
    signal newProjectRequested()
    signal openRequested()
    signal saveRequested()
    signal saveAsRequested()
    signal openRecentRequested(string path)
    signal paletteRequested()
    signal exportRequested()
    property bool playKeys: false
    signal playKeysToggled()

    readonly property bool pro: app.view === "pro"

    implicitHeight: 64
    color: Theme.panel

    // Empty parts of the bar move the window. Declared first so every
    // control below keeps its own clicks and drags (e.g. dragging the tempo).
    WindowDragArea { anchors.fill: parent }

    Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.line }

    component Separator: Rectangle {
        Layout.preferredWidth: 1
        Layout.preferredHeight: 26
        Layout.leftMargin: 4
        Layout.rightMargin: 4
        color: Theme.line
    }

    // Left: project
    RowLayout {
        anchors.left: parent.left
        anchors.leftMargin: 16
        anchors.verticalCenter: parent.verticalCenter
        width: Math.max(0, transport.x - anchors.leftMargin - 16)
        spacing: 12
        Logo { size: 40 }
        Separator {}
        // Project name. A dot marks unsaved changes. File commands live in the menu bar.
        Item {
            id: projectButton
            Layout.fillWidth: true
            Layout.maximumWidth: implicitWidth
            implicitHeight: 34
            implicitWidth: projectRow.implicitWidth
            HoverHandler { id: projectHover }
            RowLayout {
                id: projectRow
                anchors.fill: parent
                spacing: 6
                DText {
                    Layout.leftMargin: 8
                    Layout.fillWidth: true
                    text: bar.app.projectName
                    strong: true
                    elide: Text.ElideRight
                }
                Rectangle {
                    visible: bar.app.dirty
                    implicitWidth: 7; implicitHeight: 7; radius: 3.5
                    color: Theme.textDim
                }
            }
            DToolTip {
                visible: projectHover.hovered
                text: bar.app.projectPath !== "" ? bar.app.projectPath + (bar.app.dirty ? " · unsaved changes" : "")
                                                 : "Not saved yet"
            }
        }
        Item { Layout.fillWidth: true }
    }

    // Centre: transport and LCD
    Rectangle {
        id: transport
        anchors.centerIn: parent
        width: transportRow.implicitWidth + 16
        height: 52
        radius: Theme.studio ? Theme.radiusLg : 26
        color: Theme.sunken
        border.color: Theme.line

        RowLayout {
            id: transportRow
            anchors.centerIn: parent
            spacing: 4

            IconButton {
                glyph: Icons.skipBack
                filled: true
                tip: "Go to beginning (Return)"
                onClicked: bar.app.rewind()
            }
            PlayButton {
                playing: bar.app.playing
                enabled: bar.app.audioOk
                onClicked: bar.app.togglePlay()
            }
            // Record: red, and pulsing while a take is being recorded.
            AbstractButton {
                id: recordButton
                implicitWidth: 34
                implicitHeight: 34
                hoverEnabled: true
                enabled: bar.app.audioOk
                onClicked: bar.app.record()
                background: Rectangle {
                    radius: width / 2
                    color: bar.app.recording ? Theme.tint(Theme.record, 0.25) : recordButton.hovered ? Theme.raised : "transparent"
                    Behavior on color { ColorAnimation { duration: Theme.fast } }
                }
                contentItem: Item {
                    Rectangle {
                        anchors.centerIn: parent
                        width: 13; height: 13; radius: 6.5
                        color: !recordButton.enabled ? Theme.textFaint : Theme.record
                        SequentialAnimation on opacity {
                            running: bar.app.recording && Theme.motionMs > 0
                            loops: Animation.Infinite
                            NumberAnimation { from: 1; to: 0.35; duration: 600 }
                            NumberAnimation { from: 0.35; to: 1; duration: 600 }
                            onRunningChanged: if (!running) parent.opacity = 1
                        }
                    }
                }
                DToolTip { visible: recordButton.hovered; text: bar.app.recording ? "Stop recording (R or Space)" : "Record (R)" }
            }
            IconButton {
                glyph: Icons.repeat
                active: bar.app.cycle
                activeColor: Theme.loopOn
                tip: bar.app.cycle ? "Cycle is on (C): the song repeats the loop you set" : "Cycle (C): repeat one section of the song over and over"
                onClicked: bar.app.toggleCycle()
            }

            TransportLcd {
                Layout.leftMargin: 6
                Layout.rightMargin: 6
                app: bar.app
            }

            IconButton {
                glyph: Icons.metronome
                active: bar.app.metronome
                tip: bar.app.metronome ? "Metronome is on (K): a click on every beat" : "Metronome (K): a click on every beat to help you keep time"
                onClicked: bar.app.toggleMetronome()
            }
            IconButton {
                glyph: Icons.keyboard
                active: bar.playKeys
                tip: bar.playKeys ? "Computer keys play notes (click to stop)" : "Play notes with the computer keyboard (A–L, Z/X octave)"
                onClicked: bar.playKeysToggled()
            }
            IconButton {
                label: "1234"
                active: bar.app.countIn
                tip: bar.app.countIn
                     ? "Count-in is on: " + bar.app.countInBars + (bar.app.countInBars === 1 ? " bar" : " bars") + " of clicks before recording starts"
                     : "Count-in: play a bar of clicks before you start recording"
                onClicked: bar.app.toggleCountIn()
            }
        }
    }

    // Right: monitoring and actions
    RowLayout {
        anchors.right: parent.right
        anchors.rightMargin: 12
        anchors.verticalCenter: parent.verticalCenter
        spacing: 8

        Rectangle {
            visible: bar.pro && bar.app.audioOk && bar.width > 1320
            implicitWidth: cpuRow.implicitWidth + 16
            implicitHeight: 28
            radius: 14
            color: Theme.raised
            border.color: Theme.line
            readonly property real load: bar.app.cpuLoad
            Row {
                id: cpuRow
                anchors.centerIn: parent
                spacing: 6
                Icon { glyph: Icons.cpu; size: 14; color: parent.parent.load > 0.7 ? Theme.danger : parent.parent.load > 0.4 ? Theme.warning : Theme.success; anchors.verticalCenter: parent.verticalCenter }
                DText { text: Math.round(bar.app.cpuLoad * 100) + "%"; font.family: Theme.mono; font.pixelSize: Theme.textSm; anchors.verticalCenter: parent.verticalCenter }
            }
            DToolTip { visible: cpuHover.hovered; text: "Audio CPU. Above 70% you may hear crackles." }
            HoverHandler { id: cpuHover }
        }


        IconButton {
            glyph: bar.app.volume < 0.01 ? Icons.speakerX : bar.app.volume < 0.5 ? Icons.speakerLow : Icons.speakerHigh
            tip: "Mute"
            property real lastVolume: 1
            onClicked: {
                if (bar.app.volume > 0.01) { lastVolume = bar.app.volume; bar.app.setVolumeLevel(0) }
                else bar.app.setVolumeLevel(lastVolume)
            }
        }
        DSlider {
            // Hidden on narrower windows, where the transport needs the room.
            visible: bar.width > 1380
            implicitWidth: 96
            from: 0; to: 1.5
            value: bar.app.volume
            onMoved: bar.app.setVolumeLevel(value)
        }

        Separator {}

        DButton {
            glyph: Icons.downloadSimple
            text: "Import"
            onClicked: bar.importRequested()
            DToolTip { visible: parent.hovered; text: "Import audio (⌘I)" }
        }
        DButton {
            glyph: Icons.fileAudio
            text: "Video"
            onClicked: bar.importVideoRequested()
            DToolTip { visible: parent.hovered; text: "Import a video for scoring" }
        }
        IconButton {
            glyph: Icons.magnifyingGlass
            tip: "Search commands (" + (Qt.platform.os === "osx" ? "⌘K" : "Ctrl+K") + ")"
            onClicked: bar.paletteRequested()
        }
        IconButton {
            glyph: Icons.gear
            tip: "Settings"
            onClicked: bar.settingsRequested()
        }
    }
}
