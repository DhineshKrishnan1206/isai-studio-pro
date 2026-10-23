import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Isai

// The start screen. Each starting point has its own 16-step beat: the stage
// plays the chosen one at that song's tempo, and each card shows its pattern.
// The page reveals in steps, with the ArtCraft-style square buttons over a
// faint dot grid. Colours are fixed (the site's dark), in every theme.
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

    // The site's dark palette. Qt reads 8-digit hex as AARRGGBB, so the
    // translucent tones are written as explicit alpha colours.
    readonly property color paper: "#121316"
    readonly property color card: "#0B0B0C"
    readonly property color raised: "#1C1D21"
    readonly property color ink: "#F2F1EE"
    readonly property color invertBg: "#FFFFFF"
    readonly property color invertFg: "#0B0B0C"
    readonly property color mute: Qt.rgba(0.949, 0.945, 0.933, 0.6)
    readonly property color faint: Qt.rgba(0.949, 0.945, 0.933, 0.38)
    readonly property color line: Qt.rgba(0.949, 0.945, 0.933, 0.15)
    readonly property color lineStrong: Qt.rgba(0.949, 0.945, 0.933, 0.4)

    FontLoader { id: fArchivo; source: "qrc:/qt/qml/Isai/assets/fonts/Archivo.ttf" }
    FontLoader { id: fMono; source: "qrc:/qt/qml/Isai/assets/fonts/GeistMono.ttf" }
    readonly property string sans: fArchivo.name !== "" ? fArchivo.name : Theme.font
    readonly property string mono: fMono.name !== "" ? fMono.name : Theme.mono

    // Starting points. Each sets the tempo, time and key, and its own beat:
    // four lanes (kick, snare, hats, bass) of 16 steps, x = a hit.
    readonly property var templates: [
        { key: "empty", title: "Empty", dot: "#2FD39C", tempo: 120, num: 4, den: 4, songKey: "C major", metronome: false,
          lanes: ["x...x...x...x...", "................", "................", "................"] },
        { key: "beat", title: "Beat", dot: "#F27141", tempo: 90, num: 4, den: 4, songKey: "A minor", metronome: true,
          lanes: ["x...x...x...x...", "....x.......x...", "x.x.x.x.x.x.x.x.", "..x...x...x...x."] },
        { key: "song", title: "Song", dot: "#F5C543", tempo: 110, num: 4, den: 4, songKey: "G major", metronome: true,
          lanes: ["x.....x..x.....x.", "....x.......x...", "x.x.x.x.x.x.x.x.", "x...x...x.x.x..."] },
        { key: "waltz", title: "Ballad · 6/8", dot: "#8B5CF6", tempo: 72, num: 6, den: 8, songKey: "D major", metronome: true,
          lanes: ["x.....x.....x...", "...x.....x......", "x.x.x.x.x.x.x.x.", "x.....x.....x..."] },
        { key: "voice", title: "Voice & podcast", dot: "#EC5D92", tempo: 120, num: 4, den: 4, songKey: "C major", metronome: false,
          lanes: ["x...x...x...x...", "................", "..x...x...x..x..", "................"] }
    ]
    property string selected: "empty"
    property var current: templates.find(t => t.key === selected)

    function create() {
        const t = current
        app.newProject(nameField.text, t.tempo, t.num, t.den, t.songKey)
        if (t.metronome) app.toggleMetronome()
        if (t.key === "song") app.toggleCountIn()
        nameField.text = ""
        created()
    }

    color: paper

    // The playhead: 16 sixteenth-note steps per bar, at the chosen tempo.
    property real step: 0
    NumberAnimation on step {
        from: 0; to: 16
        duration: Math.round(240000 / chooser.current.tempo)
        loops: Animation.Infinite
        running: chooser.splashDone && Theme.motionMs > 0
    }
    readonly property int activeCol: Math.floor(step) % 16

    // The splash shows while the app gets ready; then the page reveals in steps.
    property bool splashDone: false
    property int stage: 0
    Timer { running: true; interval: 900; onTriggered: chooser.splashDone = true }
    Timer {
        running: chooser.splashDone && chooser.stage < 6
        interval: 120
        repeat: true
        onTriggered: chooser.stage++
    }

    // A faint dot grid along the edges, fading out toward the middle. Drawn once per size.
    Canvas {
        id: dots
        anchors.fill: parent
        onWidthChanged: requestPaint()
        onHeightChanged: requestPaint()
        onPaint: {
            const ctx = getContext("2d")
            ctx.reset()
            ctx.fillStyle = "rgba(242, 241, 238, 0.14)"
            for (let x = 20; x < width; x += 28) {
                for (let y = 20; y < height; y += 28) {
                    if (Math.min(x, width - x, y, height - y) < 110) {
                        ctx.beginPath()
                        ctx.arc(x, y, 1.2, 0, Math.PI * 2)
                        ctx.fill()
                    }
                }
            }
        }
    }

    Rectangle {
        id: splash
        anchors.fill: parent
        color: chooser.paper
        visible: opacity > 0
        opacity: chooser.splashDone ? 0 : 1
        Behavior on opacity { NumberAnimation { duration: Theme.slow } }
        Column {
            anchors.centerIn: parent
            spacing: 18
            Logo { anchors.horizontalCenter: parent.horizontalCenter; variant: "mark"; size: 44; ink: "light" }
            Text {
                anchors.horizontalCenter: parent.horizontalCenter
                text: "ISAI STUDIO PRO"
                font.family: chooser.mono
                font.pixelSize: 11
                font.letterSpacing: 1.6
                color: chooser.mute
            }
            Spinner { anchors.horizontalCenter: parent.horizontalCenter; color: chooser.mute }
        }
    }

    WindowDragArea { anchors.left: parent.left; anchors.right: parent.right; height: 46 }
    // Return creates the song (also from the name field).
    Shortcut { sequences: ["Return", "Enter"]; enabled: chooser.visible && chooser.splashDone; onActivated: chooser.create() }

    // A block that rises and fades in once its stage has come.
    component Revealed: Item {
        id: rv
        property int at: 0
        readonly property bool on: chooser.stage > at
        opacity: on ? 1 : 0
        Behavior on opacity { NumberAnimation { duration: 900; easing.type: Easing.OutCubic } }
        transform: Translate {
            y: rv.on ? 0 : 28
            Behavior on y { NumberAnimation { duration: 900; easing.type: Easing.OutCubic } }
        }
    }

    // The site's button: square, mono, bold, uppercase. Filled ones dim on hover;
    // outlined ones invert.
    component SiteButton: AbstractButton {
        id: sb
        property bool primary: false
        implicitHeight: 48
        implicitWidth: buttonText.implicitWidth + 48
        hoverEnabled: true
        focusPolicy: Qt.StrongFocus
        opacity: sb.primary && sb.hovered ? 0.8 : 1
        Behavior on opacity { NumberAnimation { duration: 150 } }
        background: Rectangle {
            color: sb.primary || sb.hovered ? chooser.invertBg : "transparent"
            border.width: sb.primary ? 0 : 1
            border.color: sb.hovered && !sb.primary ? "transparent" : chooser.lineStrong
            FocusRing { visible: sb.visualFocus }
            Behavior on color { ColorAnimation { duration: 120 } }
        }
        contentItem: Text {
            id: buttonText
            text: sb.text
            font.family: chooser.mono
            font.pixelSize: 11
            font.weight: Font.Bold
            font.capitalization: Font.AllUppercase
            font.letterSpacing: 1.3
            color: sb.primary || sb.hovered ? chooser.invertFg : chooser.ink
            Behavior on color { ColorAnimation { duration: 120 } }
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
    }

    // A small uppercase label above a block.
    component Kicker: Text {
        font.family: chooser.mono
        font.pixelSize: 11
        font.capitalization: Font.AllUppercase
        font.letterSpacing: 1.6
        color: chooser.mute
    }

    // A pattern in miniature: four lanes of 16 steps.
    component PatternGrid: Item {
        id: pg
        property var lanes: []
        property color tint: chooser.ink
        property real cell: 5
        property real gap: 2
        implicitWidth: 16 * (cell + gap)
        implicitHeight: 4 * (cell + gap)
        Repeater {
            model: 64
            delegate: Rectangle {
                required property int index
                readonly property int col: index % 16
                readonly property int row: Math.floor(index / 16)
                readonly property bool hit: pg.lanes.length > row && pg.lanes[row].charAt(col) === "x"
                x: col * (pg.cell + pg.gap)
                y: row * (pg.cell + pg.gap)
                width: pg.cell
                height: pg.cell
                color: hit ? pg.tint : chooser.line
            }
        }
    }

    // A starting point: its name, tempo, and its own beat in miniature.
    component TemplateCard: AbstractButton {
        id: tc
        property var modelData
        readonly property bool chosen: chooser.selected === modelData.key
        hoverEnabled: true
        focusPolicy: Qt.StrongFocus
        implicitHeight: 124
        onClicked: chooser.selected = modelData.key
        onDoubleClicked: { chooser.selected = modelData.key; chooser.create() }
        background: Rectangle {
            color: tc.chosen ? chooser.raised : chooser.card
            border.width: tc.chosen ? 2 : 1
            border.color: tc.chosen ? chooser.ink : tc.hovered ? chooser.mute : chooser.line
            FocusRing { visible: tc.visualFocus }
            Behavior on border.color { ColorAnimation { duration: 120 } }
        }
        contentItem: ColumnLayout {
            anchors.fill: parent
            anchors.margins: 14
            spacing: 12
            RowLayout {
                Layout.fillWidth: true
                spacing: 8
                Rectangle { Layout.preferredWidth: 7; Layout.preferredHeight: 7; color: tc.modelData.dot }
                Text {
                    Layout.fillWidth: true
                    text: tc.modelData.title
                    font.family: chooser.sans
                    font.pixelSize: 14
                    font.weight: Font.DemiBold
                    color: chooser.ink
                    elide: Text.ElideRight
                }
            }
            PatternGrid {
                lanes: tc.modelData.lanes
                tint: tc.modelData.dot
            }
            Text {
                text: tc.modelData.tempo + " BPM"
                font.family: chooser.mono
                font.pixelSize: 11
                color: chooser.mute
            }
        }
    }

    // ---------------------------------------------------------------- the page
    GridLayout {
        id: page
        width: Math.min(1120, chooser.width - 56)
        x: (chooser.width - width) / 2
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: -20
        visible: chooser.splashDone
        columns: chooser.width > 960 ? 2 : 1
        columnSpacing: 48
        rowSpacing: 36

        // ------------------------------------------------ left: the words and the name
        ColumnLayout {
            Layout.fillWidth: true
            Layout.preferredWidth: 1
            Layout.alignment: Qt.AlignTop
            spacing: 0

            Revealed {
                at: 0
                Layout.fillWidth: true
                implicitHeight: intro.implicitHeight
                ColumnLayout {
                    id: intro
                    width: parent.width
                    spacing: 0
                    Kicker { text: "ISAI · STUDIO PRO" }
                    Text {
                        Layout.topMargin: 12
                        text: "New song"
                        font.family: chooser.sans
                        font.pixelSize: 52
                        font.weight: Font.Bold
                        color: chooser.ink
                    }
                    Text {
                        Layout.topMargin: 8
                        text: "Pick a starting beat, name it, and start playing."
                        font.family: chooser.sans
                        font.pixelSize: 15
                        color: chooser.mute
                    }
                }
            }

            // Work from the last run that wasn't saved.
            Revealed {
                at: 1
                Layout.fillWidth: true
                Layout.topMargin: 26
                visible: chooser.app.recoveryName !== ""
                implicitHeight: recoverBox.implicitHeight
                Rectangle {
                    id: recoverBox
                    width: parent.width
                    implicitHeight: recoverRow.implicitHeight + 28
                    color: chooser.card
                    border.color: chooser.lineStrong
                    RowLayout {
                        id: recoverRow
                        anchors.fill: parent
                        anchors.margins: 14
                        spacing: 12
                        Text {
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                            text: "“" + chooser.app.recoveryName + "” wasn't saved last time (autosaved " + chooser.app.recoveryAge + ")."
                            font.family: chooser.sans
                            font.pixelSize: 14
                            color: chooser.ink
                        }
                        SiteButton { primary: true; text: "Recover"; onClicked: chooser.recoverRequested() }
                        SiteButton { text: "Discard"; onClicked: chooser.app.discardAutosave() }
                    }
                }
            }

            Revealed {
                at: 2
                Layout.fillWidth: true
                Layout.topMargin: 36
                implicitHeight: nameBlock.implicitHeight
                ColumnLayout {
                    id: nameBlock
                    width: parent.width
                    spacing: 10
                    Kicker { text: "Name" }
                    TextField {
                        id: nameField
                        Layout.fillWidth: true
                        implicitHeight: 52
                        placeholderText: chooser.current.title + " song"
                        font.family: chooser.sans
                        font.pixelSize: 17
                        color: chooser.ink
                        placeholderTextColor: chooser.faint
                        selectionColor: chooser.invertBg
                        selectedTextColor: chooser.invertFg
                        leftPadding: 18
                        background: Rectangle {
                            color: chooser.card
                            border.color: nameField.activeFocus ? chooser.ink : chooser.lineStrong
                            Behavior on border.color { ColorAnimation { duration: 120 } }
                        }
                    }
                }
            }

            Revealed {
                at: 4
                Layout.fillWidth: true
                Layout.topMargin: 26
                implicitHeight: buttonRow.implicitHeight
                RowLayout {
                    id: buttonRow
                    width: parent.width
                    spacing: 12
                    SiteButton { primary: true; text: "Create song"; onClicked: chooser.create() }
                    SiteButton { text: "Open…"; onClicked: chooser.openRequested() }
                    SiteButton { visible: chooser.cancellable; text: "Back"; onClicked: chooser.cancelled() }
                    Item { Layout.fillWidth: true }
                }
            }

            Revealed {
                at: 5
                Layout.fillWidth: true
                Layout.topMargin: 36
                visible: chooser.app.recentPaths.length > 0
                implicitHeight: recentBlock.implicitHeight
                ColumnLayout {
                    id: recentBlock
                    width: parent.width
                    spacing: 0
                    Kicker { Layout.bottomMargin: 10; text: "Recent" }
                    Repeater {
                        model: Math.min(3, chooser.app.recentPaths.length)
                        delegate: AbstractButton {
                            id: recentRow
                            required property int index
                            readonly property string path: chooser.app.recentPaths[index]
                            Layout.fillWidth: true
                            implicitHeight: 44
                            hoverEnabled: true
                            focusPolicy: Qt.StrongFocus
                            onClicked: chooser.openRecentRequested(path)
                            background: Rectangle {
                                color: recentRow.hovered ? chooser.invertBg : "transparent"
                                border.width: recentRow.hovered ? 0 : 1
                                border.color: chooser.line
                                FocusRing { visible: recentRow.visualFocus }
                                Behavior on color { ColorAnimation { duration: 120 } }
                            }
                            contentItem: Text {
                                leftPadding: 16
                                verticalAlignment: Text.AlignVCenter
                                text: chooser.app.recentNames[recentRow.index]
                                font.family: chooser.sans
                                font.pixelSize: 15
                                font.weight: Font.DemiBold
                                color: recentRow.hovered ? chooser.invertFg : chooser.ink
                                elide: Text.ElideRight
                                Behavior on color { ColorAnimation { duration: 120 } }
                            }
                        }
                    }
                }
            }
        }

        // ------------------------------------------------ right: the stage, playing the chosen beat
        Revealed {
            at: 1
            Layout.fillWidth: true
            Layout.preferredWidth: 1
            Layout.alignment: Qt.AlignTop
            implicitHeight: stageBox.implicitHeight
            Rectangle {
                id: stageBox
                width: parent.width
                implicitHeight: stageCol.implicitHeight + 40
                color: chooser.card
                border.color: chooser.lineStrong

                ColumnLayout {
                    id: stageCol
                    anchors.fill: parent
                    anchors.margins: 20
                    spacing: 16

                    RowLayout {
                        Layout.fillWidth: true
                        Text {
                            text: "NOW PLAYING · " + chooser.current.title.toUpperCase()
                            font.family: chooser.mono
                            font.pixelSize: 11
                            font.letterSpacing: 1.4
                            color: chooser.mute
                        }
                        Item { Layout.fillWidth: true }
                        Text {
                            text: chooser.current.num + "/" + chooser.current.den + " · " + chooser.current.songKey.toUpperCase()
                            font.family: chooser.mono
                            font.pixelSize: 11
                            color: chooser.mute
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 10
                        Text {
                            text: chooser.current.tempo
                            font.family: chooser.sans
                            font.pixelSize: 64
                            font.weight: Font.Bold
                            color: chooser.ink
                        }
                        Text {
                            Layout.alignment: Qt.AlignBottom
                            Layout.bottomMargin: 14
                            text: "BPM"
                            font.family: chooser.mono
                            font.pixelSize: 12
                            font.letterSpacing: 1.4
                            color: chooser.mute
                        }
                    }

                    // The grid: the playhead sweeps across at the chosen tempo.
                    Item {
                        id: stageGrid
                        Layout.fillWidth: true
                        implicitHeight: 4 * 44
                        readonly property real cellW: width / 16
                        Repeater {
                            model: 64
                            delegate: Rectangle {
                                required property int index
                                readonly property int col: index % 16
                                readonly property int row: Math.floor(index / 16)
                                readonly property bool hit: chooser.current.lanes[row].charAt(col) === "x"
                                readonly property bool hot: hit && col === chooser.activeCol
                                x: col * stageGrid.cellW + 3
                                y: row * 44 + 4
                                width: stageGrid.cellW - 6
                                height: 36
                                radius: 3
                                color: hot ? chooser.ink : hit ? chooser.current.dot : chooser.line
                                opacity: hit ? 1 : 0.7
                                Behavior on color { ColorAnimation { duration: 80 } }
                            }
                        }
                        Rectangle {
                            x: chooser.step * stageGrid.cellW
                            y: -6
                            width: 2
                            height: stageGrid.height + 12
                            color: chooser.ink
                            opacity: 0.85
                        }
                    }
                }
            }
        }

        // ------------------------------------------------ the starting points, across the bottom
        Revealed {
            at: 3
            Layout.fillWidth: true
            Layout.columnSpan: chooser.width > 960 ? 2 : 1
            implicitHeight: pickBlock.implicitHeight
            ColumnLayout {
                id: pickBlock
                width: parent.width
                spacing: 12
                Kicker { text: "Start from" }
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 10
                    Repeater {
                        model: chooser.templates
                        delegate: TemplateCard {
                            modelData: chooser.templates[index]
                            Layout.fillWidth: true
                        }
                    }
                }
            }
        }
    }

    // The footer: what the app is, and the chosen starting point's settings.
    Text {
        anchors.left: parent.left
        anchors.leftMargin: 24
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 18
        text: "offline · no account · free forever"
        font.family: chooser.mono
        font.pixelSize: 11
        color: chooser.mute
    }
    Text {
        anchors.right: parent.right
        anchors.rightMargin: 24
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 18
        text: chooser.current.tempo + " BPM · " + chooser.current.num + "/" + chooser.current.den + " · " + chooser.current.songKey.toUpperCase()
        font.family: chooser.mono
        font.pixelSize: 11
        color: chooser.mute
    }
}
