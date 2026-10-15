import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Isai

// The start screen: choose a template, set tempo, time signature and key,
// and create the project.
Rectangle {
    id: chooser
    property var app
    /// True when opened from an existing project, so it can be cancelled.
    property bool cancellable: false
    signal cancelled()
    signal created()
    signal openRequested()
    signal openRecentRequested(string path)
    signal recoverRequested()

    color: Theme.bg

    // Starting points. Each just sets sensible defaults; you can change anything.
    readonly property var templates: [
        { key: "empty",  glyph: Icons.filePlus,        title: "Empty Project", text: "Start from a blank timeline.",              tempo: 120, num: 4, den: 4, songKey: "C major", metronome: false },
        { key: "beat",   glyph: Icons.waveform,        title: "Beat",          text: "Hip-hop and trap tempo, metronome on.",     tempo: 90,  num: 4, den: 4, songKey: "A minor", metronome: true },
        { key: "song",   glyph: Icons.musicNote,       title: "Song",          text: "Pop and rock: 4/4 with a count-in.",        tempo: 110, num: 4, den: 4, songKey: "G major", metronome: true },
        { key: "waltz",  glyph: Icons.pianoKeys,       title: "Ballad",        text: "A gentle 6/8 feel for slower songs.",       tempo: 72,  num: 6, den: 8, songKey: "D major", metronome: true },
        { key: "voice",  glyph: Icons.microphoneStage, title: "Voice & Podcast", text: "Spoken word; no metronome.",               tempo: 120, num: 4, den: 4, songKey: "C major", metronome: false }
    ]
    property string selected: "empty"
    property var current: templates.find(t => t.key === selected)

    property real tempo: 120
    property int num: 4
    property int den: 4
    property string songKey: "C major"

    function pick(t) {
        selected = t.key
        tempo = t.tempo; num = t.num; den = t.den; songKey = t.songKey
    }

    function create() {
        app.newProject(nameField.text, tempo, num, den, songKey)
        if (current.metronome) app.toggleMetronome()
        if (current.key === "song") app.toggleCountIn()
        nameField.text = ""
        created()
    }

    // Colour behind everything: what the glass panels blur.
    Ambient { id: ambient; anchors.fill: parent }

    // Top strip: brand and actions
    Rectangle {
        id: header
        width: parent.width
        height: 64
        color: Theme.tint(Theme.panel, Theme.glass ? 0.55 : 1)
        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.line }
        WindowDragArea { anchors.fill: parent }
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 16
            anchors.rightMargin: 16
            Logo { variant: "lockup"; size: 34 }
            Item { Layout.fillWidth: true }
            DButton {
                glyph: Icons.folderOpen
                text: "Open…"
                onClicked: chooser.openRequested()
                DToolTip { visible: parent.hovered; text: "Open a project (" + (Qt.platform.os === "osx" ? "⌘O" : "Ctrl+O") + ")" }
            }
            DButton {
                visible: chooser.cancellable
                variant: "ghost"
                text: "Cancel"
                onClicked: chooser.cancelled()
            }
        }
    }

    RowLayout {
        anchors.top: header.bottom
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.margins: 24
        spacing: 24

        // Templates
        Item {
            Layout.fillHeight: true
            Layout.fillWidth: true

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 8
                spacing: 16
                DText { text: "New Project"; font.pixelSize: Theme.textXxl; font.weight: Font.DemiBold }
                DText { text: "Choose a starting point. You can change everything later."; dim: true }
                Banner {
                    Layout.fillWidth: true
                    visible: chooser.app.recoveryName !== ""
                    tone: "info"
                    glyph: Icons.lifebuoy
                    text: "“" + chooser.app.recoveryName + "” wasn't saved when Isai last closed (autosaved "
                          + chooser.app.recoveryAge + ")."
                    DButton { variant: "primary"; text: "Recover"; onClicked: chooser.recoverRequested() }
                    DButton { variant: "ghost"; text: "Discard"; onClicked: chooser.app.discardAutosave() }
                }

                GridLayout {
                    Layout.fillWidth: true
                    Layout.topMargin: 8
                    columns: Math.max(1, Math.floor((parent.width + 12) / 252))
                    columnSpacing: 12
                    rowSpacing: 12
                    Repeater {
                        model: chooser.templates
                        delegate: AbstractButton {
                            id: tile
                            required property var modelData
                            readonly property bool chosen: chooser.selected === modelData.key
                            Layout.fillWidth: true
                            Layout.preferredHeight: 132
                            hoverEnabled: true
                            focusPolicy: Qt.StrongFocus
                            onClicked: chooser.pick(modelData)
                            onDoubleClicked: { chooser.pick(modelData); chooser.create() }

                            background: GlassPanel {
                                backdrop: ambient
                                radius: Theme.radiusLg
                                glows: false
                                Rectangle {
                                    anchors.fill: parent
                                    radius: Theme.radiusLg
                                    color: tile.chosen ? Theme.accentSoft : tile.hovered ? Theme.tint(Theme.hover, 0.5) : "transparent"
                                    border.width: tile.chosen ? 2 : 0
                                    border.color: Theme.accent
                                    Behavior on color { ColorAnimation { duration: Theme.fast } }
                                }
                                FocusRing { visible: tile.visualFocus }
                            }
                            contentItem: ColumnLayout {
                                spacing: 6
                                Rectangle {
                                    Layout.preferredWidth: 38; Layout.preferredHeight: 38
                                    radius: Theme.radiusMd
                                    color: tile.chosen ? Theme.accent : Theme.raised
                                    Icon {
                                        anchors.centerIn: parent
                                        glyph: tile.modelData.glyph
                                        size: 20
                                        color: tile.chosen ? Theme.accentText : Theme.accent
                                    }
                                }
                                Item { Layout.preferredHeight: 2 }
                                DText { text: tile.modelData.title; strong: true }
                                DText {
                                    Layout.fillWidth: true
                                    text: tile.modelData.text
                                    dim: true
                                    font.pixelSize: Theme.textSm
                                    wrapMode: Text.WordWrap
                                }
                            }
                            padding: 16
                        }
                    }
                }
                // Recent projects
                DText {
                    visible: chooser.app.recentPaths.length > 0
                    Layout.topMargin: 18
                    text: "RECENT"
                    faint: true
                    font.pixelSize: Theme.textXs
                    font.weight: Font.DemiBold
                    font.letterSpacing: 1.2
                }
                ListView {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    spacing: 2
                    model: chooser.app.recentPaths.length
                    delegate: AbstractButton {
                        id: recentRow
                        required property int index
                        readonly property string path: chooser.app.recentPaths[index]
                        width: ListView.view.width
                        height: 44
                        hoverEnabled: true
                        onClicked: chooser.openRecentRequested(path)
                        background: Rectangle {
                            radius: Theme.radiusMd
                            color: recentRow.hovered ? Theme.tint(Theme.raised, 0.6) : "transparent"
                        }
                        contentItem: RowLayout {
                            spacing: 12
                            Icon { Layout.leftMargin: 10; glyph: Icons.musicNote; size: 18; color: Theme.accent }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 0
                                DText { text: chooser.app.recentNames[recentRow.index]; strong: true }
                                DText {
                                    Layout.fillWidth: true
                                    text: recentRow.path
                                    faint: true
                                    font.pixelSize: Theme.textXs
                                    elide: Text.ElideMiddle
                                }
                            }
                        }
                    }
                }
                Item { Layout.fillHeight: true; visible: chooser.app.recentPaths.length === 0 }
            }
        }

        // Project details
        GlassPanel {
            Layout.preferredWidth: 340
            Layout.fillHeight: true
            backdrop: ambient

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 24
                spacing: 6

                DText { text: "PROJECT"; faint: true; font.pixelSize: Theme.textXs; font.weight: Font.DemiBold; font.letterSpacing: 1.2 }

                DText { Layout.topMargin: 10; text: "Name"; dim: true; font.pixelSize: Theme.textSm }
                TextField {
                    id: nameField
                    Layout.fillWidth: true
                    implicitHeight: 34
                    placeholderText: "Untitled"
                    font.family: Theme.font
                    font.pixelSize: Theme.textMd
                    color: Theme.text
                    placeholderTextColor: Theme.textFaint
                    selectionColor: Theme.accent
                    leftPadding: 10
                    background: Rectangle {
                        radius: Theme.radiusMd
                        color: Theme.sunken
                        border.color: nameField.activeFocus ? Theme.accent : Theme.line
                    }
                    onAccepted: chooser.create()
                }

                DText { Layout.topMargin: 12; text: "Tempo"; dim: true; font.pixelSize: Theme.textSm }
                RowLayout {
                    spacing: 8
                    DSlider {
                        Layout.fillWidth: true
                        from: 40; to: 220; stepSize: 1
                        value: chooser.tempo
                        onMoved: chooser.tempo = Math.round(value)
                    }
                    DText {
                        Layout.preferredWidth: 64
                        horizontalAlignment: Text.AlignRight
                        text: chooser.tempo.toFixed(0) + " BPM"
                        font.family: Theme.mono
                    }
                }

                DText { Layout.topMargin: 12; text: "Time signature"; dim: true; font.pixelSize: Theme.textSm }
                Segmented {
                    minItemWidth: 46
                    options: ["2/4", "3/4", "4/4", "6/8", "12/8"].map(s => ({ value: s, label: s }))
                    value: chooser.num + "/" + chooser.den
                    onActivated: v => { const p = v.split("/"); chooser.num = parseInt(p[0]); chooser.den = parseInt(p[1]) }
                }

                DText { Layout.topMargin: 12; text: "Key"; dim: true; font.pixelSize: Theme.textSm }
                RowLayout {
                    spacing: 8
                    Dropdown {
                        id: tonic
                        implicitWidth: 110
                        model: ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"]
                        currentIndex: Math.max(0, model.indexOf(chooser.songKey.split(" ")[0]))
                        onActivated: chooser.songKey = model[currentIndex] + " " + chooser.songKey.split(" ")[1]
                    }
                    Segmented {
                        minItemWidth: 64
                        options: [{ value: "major", label: "Major" }, { value: "minor", label: "Minor" }]
                        value: chooser.songKey.split(" ")[1]
                        onActivated: v => chooser.songKey = chooser.songKey.split(" ")[0] + " " + v
                    }
                }

                Item { Layout.fillHeight: true }

                DText {
                    Layout.fillWidth: true
                    wrapMode: Text.WordWrap
                    faint: true
                    font.pixelSize: Theme.textSm
                    text: "Audio runs at your device's sample rate (" + (chooser.app.sampleRate / 1000).toFixed(1) + " kHz)."
                }
                DButton {
                    Layout.fillWidth: true
                    Layout.topMargin: 8
                    variant: "primary"
                    large: true
                    glyph: Icons.plus
                    text: "Create project"
                    onClicked: chooser.create()
                }
            }
        }
    }
}
