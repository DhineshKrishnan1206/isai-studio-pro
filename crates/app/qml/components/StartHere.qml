import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import QtCore
import Isai

// "Start here": a short guide for a new song. Each step is ticked off by doing it,
// so the guide follows what you actually do. Hidden once every step is done, or
// when you skip it (remembered on this computer).
Rectangle {
    id: guide
    /// The app controller and the studio page.
    property var app
    property var studio

    property bool played: false
    readonly property real startTempo: 120

    Connections {
        target: guide.app
        function onPlayingChanged() { if (guide.app.playing) guide.played = true }
    }

    readonly property bool hasEffect: {
        try {
            return JSON.parse(guide.app.tracksJson || "[]").some(t => t.inserts && t.inserts.length > 0)
        } catch (e) { return false }
    }

    readonly property var steps: [
        { title: "Add a track",
          text: "Tracks are the lines of sound in your song. Click Add drum track to make one.",
          done: guide.app.trackCount > 0,
          action: "Add drum track", run: () => guide.app.addInstrumentTrack("drums") },
        { title: "Press Play",
          text: "Play starts the song from the beginning, so you can hear your track.",
          done: guide.played,
          action: "Play", run: () => guide.app.togglePlay() },
        { title: "Change the tempo",
          text: "Tempo is how fast the song goes. Click the number 120.0 at the top and type a new one.",
          done: Math.abs(guide.app.tempo - guide.startTempo) > 0.5,
          action: "", run: null },
        { title: "Open the mixer",
          text: "The mixer has one fader per track. Faders set how loud each track is.",
          done: guide.studio ? guide.studio.mixerShown : false,
          action: "Open mixer", run: () => guide.studio.mixerShown = true },
        { title: "Add an effect",
          text: "Effects change the sound. Pick a track in the mixer and add an Equalizer. Click the i button to learn what it does.",
          done: guide.hasEffect,
          action: "", run: null }
    ]

    readonly property int current: {
        for (let i = 0; i < steps.length; ++i) if (!steps[i].done) return i
        return steps.length
    }
    readonly property bool finished: current >= steps.length

    visible: !settings.dismissed && !!app && app.projectOpen
    width: 320
    implicitHeight: column.implicitHeight + 32
    radius: Theme.radiusXl
    color: Theme.raised
    border.color: Theme.line

    readonly property var settings: Settings { category: "startHere"; property bool dismissed: false }

    ColumnLayout {
        id: column
        anchors.fill: parent
        anchors.margins: 16
        spacing: 10

        RowLayout {
            Layout.fillWidth: true
            DText {
                text: guide.finished ? "Start here · all done" : "Start here · step " + (guide.current + 1) + " of " + guide.steps.length
                faint: true
                font.pixelSize: Theme.textXs
                font.weight: Font.DemiBold
                font.letterSpacing: 1
                Layout.fillWidth: true
            }
            IconButton {
                glyph: Icons.x
                tip: "Hide this guide"
                onClicked: guide.settings.dismissed = true
            }
        }

        DText {
            visible: !guide.finished
            Layout.fillWidth: true
            text: guide.finished ? "" : guide.steps[guide.current].title
            strong: true
            font.pixelSize: Theme.textLg
        }
        DText {
            visible: !guide.finished
            Layout.fillWidth: true
            text: guide.finished ? "" : guide.steps[guide.current].text
            wrapMode: Text.WordWrap
            dim: true
        }
        DText {
            visible: guide.finished
            Layout.fillWidth: true
            text: "You have made the start of a song. Click the i button on any control to learn what it does."
            wrapMode: Text.WordWrap
            dim: true
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            DButton {
                visible: !guide.finished && guide.steps[guide.current].action !== ""
                text: guide.finished ? "" : guide.steps[guide.current].action
                onClicked: guide.steps[guide.current].run()
            }
            Item { Layout.fillWidth: true }
            DButton {
                visible: guide.finished
                text: "Done"
                onClicked: guide.settings.dismissed = true
            }
            DButton {
                visible: !guide.finished
                text: "Skip guide"
                onClicked: guide.settings.dismissed = true
            }
        }
    }
}
