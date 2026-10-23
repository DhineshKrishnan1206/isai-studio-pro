import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Isai

// The Inspector (I): the selected track's details down the left side, like a
// console channel: its instrument, its effects, and a big fader with meters.
Rectangle {
    id: inspector
    property var app
    property var tracks: []
    property var meters: []
    /// What the info button's lesson card blurs behind it (the studio page).
    property Item backdrop: null
    signal instrumentRequested(int id)
    signal effectEditRequested(int id, string chain, color tint)
    signal closeRequested()

    readonly property int index: tracks.findIndex(t => t.id === app.selectedTrack)
    readonly property var track: index >= 0 ? tracks[index] : null
    readonly property color tint: track ? Theme.trackColor(track.color) : Theme.accent
    readonly property var level: index >= 0 && meters[index] ? meters[index] : [0, 0]

    color: Theme.panel
    Rectangle { anchors.right: parent.right; width: 1; height: parent.height; color: Theme.line }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 12
        anchors.rightMargin: 13
        spacing: 10

        RowLayout {
            DText { text: "INSPECTOR"; faint: true; font.pixelSize: Theme.textXs; font.weight: Font.DemiBold; font.letterSpacing: 1.2 }
            LearnButton { topic: "inspector"; backdrop: inspector.backdrop; implicitWidth: 24; implicitHeight: 24; iconSize: 13 }
            Item { Layout.fillWidth: true }
            IconButton {
                glyph: Icons.x
                iconSize: 13
                implicitWidth: 22; implicitHeight: 22
                tip: "Hide the inspector (I)"
                onClicked: inspector.closeRequested()
            }
        }

        DText {
            visible: !inspector.track
            Layout.fillWidth: true
            text: "Select a track to see its instrument, effects and fader here."
            dim: true
            wrapMode: Text.WordWrap
            font.pixelSize: Theme.textSm
        }

        // The track
        RowLayout {
            visible: !!inspector.track
            spacing: 9
            Rectangle { width: 4; height: 32; radius: 2; color: inspector.tint }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 1
                DText { Layout.fillWidth: true; text: inspector.track ? inspector.track.name : ""; strong: true; elide: Text.ElideRight }
                DText {
                    text: !inspector.track ? "" : inspector.track.instrument ? "Instrument · " + inspector.track.instrument.label : "Audio track"
                    dim: true
                    font.pixelSize: Theme.textXs
                }
            }
        }
        DButton {
            visible: !!inspector.track && !!inspector.track.instrument
            Layout.fillWidth: true
            glyph: Icons.pianoKeys
            text: inspector.track && inspector.track.instrument ? "Open " + inspector.track.instrument.label : ""
            onClicked: inspector.instrumentRequested(inspector.track.id)
        }

        // The channel's slots, top to bottom as on Logic's channel strip.
        ColumnLayout {
            visible: !!inspector.track
            Layout.fillWidth: true
            spacing: 4
            ChannelSlot {
                label: "EQ"
                lit: !!inspector.track && inspector.track.inserts.some(i => String(i.kind || "").toLowerCase().indexOf("eq") >= 0)
                tint: inspector.tint
                value: lit ? "On" : ""
            }
            ChannelSlot {
                label: "Instrument"
                lit: !!inspector.track && !!inspector.track.instrument
                tint: inspector.tint
                value: lit ? inspector.track.instrument.label : "None"
            }
            ChannelSlot {
                label: "Audio FX"
                lit: !!inspector.track && inspector.track.inserts.length > 0
                tint: inspector.tint
                value: inspector.track ? inspector.track.inserts.length + (inspector.track.inserts.length === 1 ? " effect" : " effects") : ""
            }
            ChannelSlot {
                label: "Sends"
                lit: !!inspector.track && Object.keys(inspector.track.sends || {}).length > 0
                tint: inspector.tint
                value: inspector.track ? Object.keys(inspector.track.sends || {}).length + " bus" : ""
            }
            ChannelSlot {
                label: "Output"
                lit: true
                tint: inspector.tint
                value: "Stereo Out"
            }
        }

        EffectsPanel {
            visible: !!inspector.track
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumHeight: 120
            app: inspector.app
            chainType: "track"
            chainId: inspector.track ? inspector.track.id : -1
            chainName: inspector.track ? inspector.track.name + " · track" : ""
            inserts: inspector.track ? inspector.track.inserts : []
            tint: inspector.tint
            onEditRequested: id => inspector.effectEditRequested(id, chainName, inspector.tint)
        }
        Item { visible: !inspector.track; Layout.fillHeight: true }

        // Channel strips: the selected track, then the master output, side by side.
        RowLayout {
            Layout.fillWidth: true
            spacing: 8
        Rectangle {
            visible: !!inspector.track
            Layout.fillWidth: true
            Layout.preferredHeight: Math.min(300, Math.max(200, inspector.height * 0.38))
            radius: Theme.radiusMd
            color: Theme.surface
            border.color: Theme.line

            RowLayout {
                anchors.fill: parent
                anchors.margins: 12
                spacing: 14
                VFader {
                    Layout.fillHeight: true
                    db: inspector.track ? Auto.live(inspector.track, "volume", inspector.app.positionBeats, inspector.app.playing, inspector.track.volume) : 0
                    tint: inspector.tint
                    leftLevel: inspector.level[0]
                    rightLevel: inspector.level[1]
                    onMoved: (d, done) => inspector.app.setTrackVolume(inspector.track.id, d, done)
                }
                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    spacing: 8
                    DText { text: "Volume"; faint: true; font.pixelSize: Theme.textXs }
                    DText {
                        readonly property real v: inspector.track ? inspector.track.volume : 0
                        text: v <= -60 ? "-∞ dB" : (v > 0 ? "+" : "") + v.toFixed(1) + " dB"
                        font.family: Theme.mono
                        font.pixelSize: Theme.textMd
                    }
                    DText { Layout.topMargin: 6; text: "Pan"; faint: true; font.pixelSize: Theme.textXs }
                    PanKnob {
                        value: inspector.track ? Auto.live(inspector.track, "pan", inspector.app.positionBeats, inspector.app.playing, inspector.track.pan) : 0
                        tint: inspector.tint
                        onMoved: (v, done) => inspector.app.setTrackPan(inspector.track.id, v, done)
                    }
                    Item { Layout.fillHeight: true }
                    Row {
                        spacing: 4
                        MiniToggle {
                            label: "M"
                            on: !!inspector.track && inspector.track.mute
                            onColor: Theme.accent
                            tip: "Mute"
                            onClicked: inspector.app.toggleMute(inspector.track.id)
                        }
                        MiniToggle {
                            label: "S"
                            on: !!inspector.track && inspector.track.solo
                            onColor: Theme.loopOn
                            tip: "Solo"
                            onClicked: inspector.app.toggleSolo(inspector.track.id)
                        }
                    }
                }
            }
        }

        // Master output: the Stereo Out channel's fader.
        Rectangle {
            Layout.preferredWidth: 84
            Layout.preferredHeight: Math.min(300, Math.max(200, inspector.height * 0.38))
            radius: Theme.radiusMd
            color: Theme.surface
            border.color: Theme.line

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 10
                spacing: 8
                DText { Layout.fillWidth: true; text: "Stereo Out"; faint: true; font.pixelSize: Theme.textXs; elide: Text.ElideRight }
                VFader {
                    Layout.fillHeight: true
                    Layout.alignment: Qt.AlignHCenter
                    db: 20 * Math.log10(Math.max(0.0001, inspector.app.volume))
                    tint: Theme.accent
                    onMoved: (d, done) => inspector.app.setVolumeLevel(Math.pow(10, d / 20))
                }
                DText {
                    Layout.alignment: Qt.AlignHCenter
                    text: inspector.app.volume < 0.001 ? "-∞ dB" : (20 * Math.log10(inspector.app.volume)).toFixed(1) + " dB"
                    font.family: Theme.mono
                    font.pixelSize: Theme.textXs
                }
            }
        }
        }
    }
}
