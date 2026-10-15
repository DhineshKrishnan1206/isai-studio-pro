import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Isai

// The start screen: a painting (tools/paint_art.py) with the choices over its
// darker left side: new song from a template, open, recent songs, recovery.
// Its colours are fixed (forest and cream), like a poster, in every theme.
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

    readonly property color cream: "#F6ECD6"
    readonly property color forest: "#04241A"
    color: forest

    // Starting points. Each just sets sensible defaults; you can change anything.
    readonly property var templates: [
        { key: "empty",  title: "Empty",         dot: "#7DB862", tempo: 120, num: 4, den: 4, songKey: "C major", metronome: false },
        { key: "beat",   title: "Beat",          dot: "#CD6138", tempo: 90,  num: 4, den: 4, songKey: "A minor", metronome: true },
        { key: "song",   title: "Song",          dot: "#F5DC53", tempo: 110, num: 4, den: 4, songKey: "G major", metronome: true },
        { key: "waltz",  title: "Ballad · 6/8",  dot: "#F4B271", tempo: 72,  num: 6, den: 8, songKey: "D major", metronome: true },
        { key: "voice",  title: "Voice & podcast", dot: "#56A096", tempo: 120, num: 4, den: 4, songKey: "C major", metronome: false }
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

    readonly property var art: ["meadow-field.jpg", "meadow-lake.jpg", "meadow-dusk.jpg"]
    // A different painting and line each time the app opens.
    readonly property int artIndex: Math.floor(Math.random() * art.length)
    readonly property var lines: [
        "start with\none note.\nthen another.",
        "every song\nstarts\nsomewhere.",
        "hum it first.\nthe rest\nfollows.",
        "the quiet\nbetween notes\nis music too.",
        "play it\nonce more,\nslower."
    ]
    readonly property string line: lines[Math.floor(Math.random() * lines.length)]
    readonly property string greeting: {
        const h = new Date().getHours()
        return h < 5 ? "still up?" : h < 12 ? "good morning." : h < 17 ? "good afternoon." : "good evening."
    }

    Image {
        anchors.fill: parent
        source: Theme.artDir + chooser.art[chooser.artIndex]
        sourceSize: Qt.size(1600, 1000)
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        opacity: status === Image.Ready ? 1 : 0
        Behavior on opacity { NumberAnimation { duration: Theme.slow } }
    }
    // Deep forest on the left, where the text sits; the painting shows on the right.
    Rectangle {
        anchors.fill: parent
        gradient: Gradient {
            orientation: Gradient.Horizontal
            GradientStop { position: 0.0; color: Qt.rgba(0.0, 0.13, 0.08, 0.95) }
            GradientStop { position: 0.40; color: Qt.rgba(0.0, 0.13, 0.08, 0.84) }
            GradientStop { position: 0.66; color: Qt.rgba(0.0, 0.13, 0.08, 0.0) }
        }
    }
    WindowDragArea { anchors.left: parent.left; anchors.right: parent.right; height: 56 }
    // Return starts a new song from the chosen template (also from the name field).
    Shortcut { sequences: ["Return", "Enter"]; enabled: chooser.visible; onActivated: chooser.create() }

    // The line, poster style, on the painting.
    Text {
        visible: chooser.width > 980
        anchors.right: parent.right
        anchors.rightMargin: Math.round(parent.width * 0.1)
        anchors.verticalCenter: parent.verticalCenter
        text: chooser.line
        horizontalAlignment: Text.AlignRight
        font.family: Theme.serif
        font.pixelSize: 22
        lineHeight: 1.1
        color: chooser.cream
        style: Text.Raised
        styleColor: Qt.rgba(0, 0.16, 0.1, 0.35)
    }

    Flickable {
        id: scroller
        anchors.fill: parent
        anchors.topMargin: 56
        anchors.bottomMargin: 44
        contentHeight: column.implicitHeight + 24
        clip: true
        boundsBehavior: Flickable.StopAtBounds

        ColumnLayout {
            id: column
            x: Math.max(32, Math.min(80, chooser.width * 0.055))
            width: Math.min(580, chooser.width * 0.52)
            spacing: 0

            RowLayout {
                spacing: 12
                Logo { variant: "mark"; size: 42; ink: "light" }
                Text {
                    text: "studio pro"
                    font.family: Theme.font
                    font.pixelSize: 14
                    font.letterSpacing: 2
                    color: chooser.cream
                    opacity: 0.7
                    Layout.alignment: Qt.AlignBottom
                    Layout.bottomMargin: 4
                }
            }

            Text {
                Layout.topMargin: 34
                text: chooser.greeting + "\nwhat are we making?"
                font.family: Theme.serif
                font.pixelSize: 38
                lineHeight: 1.08
                color: chooser.cream
            }
            Text {
                Layout.topMargin: 10
                text: "Pick up where you left off, or start something new."
                font.family: Theme.font
                font.pixelSize: 15
                color: chooser.cream
                opacity: 0.72
            }

            Banner {
                Layout.fillWidth: true
                Layout.topMargin: 20
                visible: chooser.app.recoveryName !== ""
                tone: "info"
                glyph: Icons.lifebuoy
                text: "“" + chooser.app.recoveryName + "” wasn't saved when Isai last closed (autosaved "
                      + chooser.app.recoveryAge + ")."
                DButton { variant: "primary"; text: "Recover"; onClicked: chooser.recoverRequested() }
                DButton { variant: "ghost"; text: "Discard"; onClicked: chooser.app.discardAutosave() }
            }

            RowLayout {
                Layout.topMargin: 26
                spacing: 12
                ArtButton {
                    primary: true
                    glyph: Icons.plus
                    text: "New song"
                    hint: "↵"
                    onClicked: chooser.create()
                }
                ArtButton {
                    glyph: Icons.folderOpen
                    text: "Open…"
                    hint: Qt.platform.os === "osx" ? "⌘O" : "Ctrl+O"
                    onClicked: chooser.openRequested()
                }
                ArtButton {
                    visible: chooser.cancellable
                    text: "Back to song"
                    onClicked: chooser.cancelled()
                }
            }

            // ---- New song settings
            Text {
                Layout.topMargin: 34
                text: "START FROM"
                font.family: Theme.font
                font.pixelSize: 11
                font.weight: Font.DemiBold
                font.letterSpacing: 1.8
                color: chooser.cream
                opacity: 0.55
            }
            Flow {
                Layout.fillWidth: true
                Layout.topMargin: 12
                spacing: 8
                Repeater {
                    model: chooser.templates
                    delegate: AbstractButton {
                        id: chip
                        required property var modelData
                        readonly property bool chosen: chooser.selected === modelData.key
                        implicitHeight: 34
                        implicitWidth: chipRow.implicitWidth + 28
                        hoverEnabled: true
                        focusPolicy: Qt.StrongFocus
                        onClicked: chooser.pick(modelData)
                        onDoubleClicked: { chooser.pick(modelData); chooser.create() }
                        background: Rectangle {
                            radius: 17
                            color: chip.chosen ? Qt.rgba(0.96, 0.93, 0.84, 0.16) : chip.hovered ? Qt.rgba(0.96, 0.93, 0.84, 0.08) : "transparent"
                            border.width: 1
                            border.color: chip.chosen ? chooser.cream : Qt.rgba(0.96, 0.93, 0.84, 0.25)
                            FocusRing { visible: chip.visualFocus }
                        }
                        contentItem: Item {
                            Row {
                                id: chipRow
                                anchors.centerIn: parent
                                spacing: 8
                                Rectangle { width: 9; height: 9; radius: 4.5; color: chip.modelData.dot; anchors.verticalCenter: parent.verticalCenter }
                                Text {
                                    text: chip.modelData.title
                                    font.family: Theme.font
                                    font.pixelSize: 13
                                    color: chooser.cream
                                    anchors.verticalCenter: parent.verticalCenter
                                }
                            }
                        }
                    }
                }
            }

            // Name, tempo, signature, key
            Rectangle {
                Layout.fillWidth: true
                Layout.topMargin: 14
                implicitHeight: details.implicitHeight + 28
                radius: 14
                color: Qt.rgba(0.0, 0.1, 0.06, 0.55)
                border.color: Qt.rgba(0.96, 0.93, 0.84, 0.14)
                GridLayout {
                    id: details
                    anchors.fill: parent
                    anchors.margins: 14
                    columns: 2
                    columnSpacing: 14
                    rowSpacing: 10

                    Text { text: "Name"; font.family: Theme.font; font.pixelSize: 12; color: chooser.cream; opacity: 0.65 }
                    TextField {
                        id: nameField
                        Layout.fillWidth: true
                        implicitHeight: 32
                        placeholderText: "Untitled"
                        font.family: Theme.font
                        font.pixelSize: Theme.textMd
                        color: chooser.cream
                        placeholderTextColor: Qt.rgba(0.96, 0.93, 0.84, 0.4)
                        selectionColor: "#2FA38A"
                        leftPadding: 10
                        background: Rectangle {
                            radius: 8
                            color: Qt.rgba(0, 0, 0, 0.25)
                            border.color: nameField.activeFocus ? chooser.cream : Qt.rgba(0.96, 0.93, 0.84, 0.18)
                        }
                    }

                    Text { text: "Tempo"; font.family: Theme.font; font.pixelSize: 12; color: chooser.cream; opacity: 0.65 }
                    RowLayout {
                        spacing: 10
                        DSlider {
                            Layout.fillWidth: true
                            from: 40; to: 220; stepSize: 1
                            value: chooser.tempo
                            onMoved: chooser.tempo = Math.round(value)
                        }
                        Text {
                            Layout.preferredWidth: 64
                            horizontalAlignment: Text.AlignRight
                            text: chooser.tempo.toFixed(0) + " BPM"
                            font.family: Theme.mono
                            font.pixelSize: 12
                            color: chooser.cream
                        }
                    }

                    Text { text: "Time"; font.family: Theme.font; font.pixelSize: 12; color: chooser.cream; opacity: 0.65 }
                    Segmented {
                        minItemWidth: 44
                        options: ["2/4", "3/4", "4/4", "6/8", "12/8"].map(s => ({ value: s, label: s }))
                        value: chooser.num + "/" + chooser.den
                        onActivated: v => { const p = v.split("/"); chooser.num = parseInt(p[0]); chooser.den = parseInt(p[1]) }
                    }

                    Text { text: "Key"; font.family: Theme.font; font.pixelSize: 12; color: chooser.cream; opacity: 0.65 }
                    RowLayout {
                        spacing: 8
                        Dropdown {
                            implicitWidth: 96
                            model: ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"]
                            currentIndex: Math.max(0, model.indexOf(chooser.songKey.split(" ")[0]))
                            onActivated: chooser.songKey = model[currentIndex] + " " + chooser.songKey.split(" ")[1]
                        }
                        Segmented {
                            minItemWidth: 60
                            options: [{ value: "major", label: "Major" }, { value: "minor", label: "Minor" }]
                            value: chooser.songKey.split(" ")[1]
                            onActivated: v => chooser.songKey = chooser.songKey.split(" ")[0] + " " + v
                        }
                    }
                }
            }

            // ---- Recent songs
            Text {
                visible: chooser.app.recentPaths.length > 0
                Layout.topMargin: 30
                text: "RECENT"
                font.family: Theme.font
                font.pixelSize: 11
                font.weight: Font.DemiBold
                font.letterSpacing: 1.8
                color: chooser.cream
                opacity: 0.55
            }
            Repeater {
                model: Math.min(6, chooser.app.recentPaths.length)
                delegate: AbstractButton {
                    id: recentRow
                    required property int index
                    readonly property string path: chooser.app.recentPaths[index]
                    Layout.fillWidth: true
                    Layout.topMargin: index === 0 ? 10 : 2
                    implicitHeight: 56
                    hoverEnabled: true
                    onClicked: chooser.openRecentRequested(path)
                    background: Rectangle {
                        radius: 12
                        color: recentRow.hovered ? Qt.rgba(0.96, 0.93, 0.84, 0.09) : "transparent"
                    }
                    contentItem: RowLayout {
                        spacing: 14
                        // A small piece of a painting, so each song has its own little cover.
                        Rectangle {
                            Layout.leftMargin: 8
                            Layout.preferredWidth: 52
                            Layout.preferredHeight: 38
                            radius: 7
                            clip: true
                            color: chooser.forest
                            border.color: Qt.rgba(0.96, 0.93, 0.84, 0.22)
                            Image {
                                anchors.fill: parent
                                anchors.margins: 1
                                source: Theme.artDir + chooser.art[recentRow.index % chooser.art.length]
                                sourceSize: Qt.size(160, 100)
                                fillMode: Image.PreserveAspectCrop
                                horizontalAlignment: [Image.AlignLeft, Image.AlignHCenter, Image.AlignRight][recentRow.index % 3]
                                asynchronous: true
                            }
                        }
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 1
                            Text {
                                text: chooser.app.recentNames[recentRow.index]
                                font.family: Theme.font
                                font.pixelSize: 14
                                font.weight: Font.DemiBold
                                color: chooser.cream
                            }
                            Text {
                                Layout.fillWidth: true
                                text: recentRow.path
                                font.family: Theme.font
                                font.pixelSize: 12
                                color: chooser.cream
                                opacity: 0.5
                                elide: Text.ElideMiddle
                            }
                        }
                    }
                }
            }
        }
    }

    Text {
        anchors.left: parent.left
        anchors.leftMargin: column.x
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 18
        text: "offline · no account · free forever"
        font.family: Theme.font
        font.pixelSize: 12
        font.letterSpacing: 1
        color: chooser.cream
        opacity: 0.55
    }
    Text {
        anchors.right: parent.right
        anchors.rightMargin: 24
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 18
        text: "audio at " + (chooser.app.sampleRate / 1000).toFixed(1) + " kHz"
        font.family: Theme.font
        font.pixelSize: 12
        font.letterSpacing: 1
        color: chooser.cream
        opacity: 0.7
    }
}
