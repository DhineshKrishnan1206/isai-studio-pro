import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Dialogs
import QtQuick.Shapes
import Isai

// "Isai Sampler": play any sound from the keyboard, or chop a loop into slices.
//   • Load a sound (or drop an audio file on the window).
//   • Keys: the sound plays across the keyboard from its root key. Drag the
//     start and end lines on the waveform; turn Loop on to hold notes.
//   • Slices: the sound is cut into pieces, one per key from C2. Even or
//     Detect beats cut it for you; drag a line to move it, double-click the
//     waveform to add one, right-click a line to remove it, click a slice to
//     hear it. Make pattern writes a MIDI region that plays the slices in order.
Item {
    id: win
    property var app
    property var track: null
    property Item backdrop: null
    property var activity: null
    property var level: [0, 0]
    signal closeRequested()

    readonly property bool shown: !!track
    readonly property var inst: track ? track.instrument : null
    readonly property var sample: inst ? inst.sample : null
    readonly property bool ready: !!sample && sample.state === "ready" && sample.seconds > 0
    readonly property var slices: inst ? inst.slices || [] : []
    readonly property bool sliceMode: Math.round(v("mode")) === 1
    readonly property int firstSliceKey: 36
    readonly property var def: {
        try { return JSON.parse(app.instrumentKindsJson || "[]").find(k => k.key === "sampler") || null } catch (e) { return null }
    }
    function param(key) { return def ? def.params.find(p => p.key === key) || null : null }
    property var live: ({})
    function v(key) {
        if (live[key] !== undefined) return live[key]
        if (inst && inst.values[key] !== undefined) return inst.values[key]
        const d = param(key)
        return d ? d.default : 0
    }
    function set(key, value, done) {
        const l = Object.assign({}, live)
        if (done) delete l[key]
        else l[key] = value
        live = l
        app.setInstrumentParam(track.id, key, value, done)
    }
    function noteName(k) { return ["C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B"][k % 12] + (Math.floor(k / 12) - 1) }
    function audition(key) {
        if (!track) return
        if (app.selectedTrack !== track.id) app.selectTrack(track.id)
        app.noteOn(key, 110)
        noteOffTimer.key = key
        noteOffTimer.restart()
    }
    Timer { id: noteOffTimer; property int key: 0; interval: 400; onTriggered: win.app.noteOff(key) }

    readonly property color bg: "#0D0E10"
    readonly property color cardBg: "#131417"
    readonly property color cardLine: "#1F2125"
    readonly property color well: "#0A0A0C"
    readonly property color text: "#E6E7EA"
    readonly property color dim: "#80848D"
    readonly property color faint: "#5D6169"
    readonly property color trackLine: "#26282D"
    readonly property color hit: "#4C8DFF"
    readonly property color amber: "#D2A560"

    readonly property real fit: parent ? Math.min(1, (parent.width - 16) / 1060, (parent.height - 16) / 660) : 1
    width: 1060
    height: 660
    x: parent ? (parent.width - width) / 2 : 0
    y: parent ? (parent.height - height) / 2 : 0
    visible: opacity > 0
    opacity: shown ? 1 : 0
    scale: fit * (shown ? 1 : 0.97)
    Behavior on opacity { NumberAnimation { duration: Theme.fast } }
    z: 150
    onShownChanged: if (!shown) live = {}

    FileDialog {
        id: loadDialog
        title: "Load a sound into the sampler"
        nameFilters: ["Audio files (*.wav *.flac *.mp3 *.ogg *.m4a *.aac *.aif *.aiff *.caf)", "All files (*)"]
        onAccepted: win.app.loadSample(win.track.id, selectedFile)
    }

    Rectangle {
        anchors.fill: parent
        radius: 12
        color: win.bg
        border.color: "#25272B"
        MouseArea { anchors.fill: parent; acceptedButtons: Qt.AllButtons; onWheel: w => w.accepted = true }
    }
    DropArea {
        anchors.fill: parent
        onDropped: d => { if (d.hasUrls && win.track) win.app.loadSample(win.track.id, d.urls[0]) }
    }

    // ------------------------------------------------ top bar
    Rectangle {
        x: 1; y: 1
        width: parent.width - 2
        height: 46
        radius: 11
        color: "#101114"
        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 12; color: parent.color }
        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: "#1B1C20" }
        DragHandler {
            target: win
            readonly property real ox: (win.width - win.width * win.fit) / 2
            readonly property real oy: (win.height - win.height * win.fit) / 2
            xAxis.minimum: 120 - win.width * win.fit - ox
            xAxis.maximum: win.parent ? win.parent.width - 120 - ox : 0
            yAxis.minimum: -oy
            yAxis.maximum: win.parent ? win.parent.height - 46 * win.fit - oy : 0
        }
        Row {
            x: 14
            anchors.verticalCenter: parent.verticalCenter
            spacing: 9
            Rectangle { anchors.verticalCenter: parent.verticalCenter; width: 16; height: 16; radius: 4; color: "transparent"; border.color: win.text; border.width: 1.5 }
            DText { anchors.verticalCenter: parent.verticalCenter; text: "Isai Sampler"; font.pixelSize: 13; font.weight: Font.Bold; color: win.text }
            DText { anchors.verticalCenter: parent.verticalCenter; text: win.track ? "·  " + win.track.name : ""; font.pixelSize: 11; color: win.faint }
        }
        Row {
            anchors.centerIn: parent
            spacing: 8
            TextButton { label: win.sample ? "Load another…" : "Load a sound…"; onClicked: loadDialog.open(); tip: "Any audio file: a note, a drum hit, a vocal, a loop" }
            DText {
                anchors.verticalCenter: parent.verticalCenter
                width: Math.min(implicitWidth, 260)
                elide: Text.ElideMiddle
                text: !win.sample ? "" : win.sample.state === "loading" ? "Loading " + win.sample.name + "…" : win.sample.state === "missing" ? win.sample.name + " (missing)" : win.sample.name
                font.pixelSize: 12
                color: win.sample && win.sample.state === "missing" ? "#E0685A" : win.text
            }
            Rectangle { width: 1; height: 20; color: "#24262B"; anchors.verticalCenter: parent.verticalCenter }
            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                width: modeRow.implicitWidth + 4; height: 26; radius: 6
                color: "#0B0B0D"
                Row {
                    id: modeRow
                    x: 2; y: 2
                    spacing: 2
                    Repeater {
                        model: ["Keys", "Slices"]
                        delegate: TextButton {
                            required property string modelData
                            required property int index
                            small: true
                            label: modelData
                            active: Math.round(win.v("mode")) === index
                            tip: index === 0 ? "Play the sound across the keyboard" : "One key per slice, from C2"
                            onClicked: win.set("mode", index, true)
                        }
                    }
                }
            }
        }
        AbstractButton {
            id: closeButton
            anchors.right: parent.right; anchors.rightMargin: 12
            anchors.verticalCenter: parent.verticalCenter
            width: 28; height: 28
            hoverEnabled: true
            onClicked: win.closeRequested()
            background: Rectangle { radius: 6; color: closeButton.hovered ? "#1D1E22" : "transparent" }
            contentItem: Icon { glyph: Icons.x; size: 12; color: win.dim }
        }
    }

    // ------------------------------------------------ waveform
    Rectangle {
        id: waveCard
        x: 10; y: 56
        width: parent.width - 20
        height: 230
        radius: 8
        color: win.cardBg
        border.color: win.cardLine

        Rectangle {
            id: wave
            x: 10; y: 10
            width: parent.width - 20
            height: parent.height - 20
            radius: 6
            color: win.well
            border.color: "#1A1B1F"
            clip: true
            readonly property real secs: win.ready ? win.sample.seconds : 1
            function xAt(s) { return s / secs * width }
            function secsAt(x) { return Math.max(0, Math.min(secs, x / width * secs)) }

            WaveformView {
                visible: win.ready
                anchors.fill: parent
                anchors.margins: 6
                color: Qt.rgba(0.85, 0.86, 0.9, 0.55)
                columnWidth: 2
                source: win.sample ? win.sample.source : ""
                offset: 0
                length: wave.secs
                reverse: Math.round(win.v("reverse")) === 1
            }
            Column {
                visible: !win.ready
                anchors.centerIn: parent
                spacing: 8
                DText { anchors.horizontalCenter: parent.horizontalCenter; text: win.sample && win.sample.state === "loading" ? "Loading…" : "Drop a sound here"; font.pixelSize: 18; color: win.dim }
                DText { anchors.horizontalCenter: parent.horizontalCenter; text: "or use Load a sound… above"; font.pixelSize: 11; color: win.faint }
            }

            // Keys mode: dimmed outside start–end, the loop shaded amber.
            Item {
                anchors.fill: parent
                visible: win.ready && !win.sliceMode
                readonly property real sx: win.v("start") / 100 * wave.width
                readonly property real ex: win.v("end") / 100 * wave.width
                readonly property real lx: win.v("loop_start") / 100 * wave.width
                Rectangle { width: parent.sx; height: parent.height; color: Qt.rgba(0, 0, 0, 0.55) }
                Rectangle { x: parent.ex; width: wave.width - parent.ex; height: parent.height; color: Qt.rgba(0, 0, 0, 0.55) }
                Rectangle {
                    visible: Math.round(win.v("loop")) === 1
                    x: Math.max(parent.sx, parent.lx); width: Math.max(0, parent.ex - x); height: parent.height
                    color: Qt.rgba(0.82, 0.65, 0.38, 0.12)
                    border.color: Qt.rgba(0.82, 0.65, 0.38, 0.5)
                }
                Marker { key: "start"; x: parent.sx; label: "START"; tint: win.text }
                Marker { key: "end"; x: parent.ex; label: "END"; tint: win.text; flip: true }
                Marker { key: "loop_start"; x: parent.lx; label: "LOOP"; tint: win.amber; visible: Math.round(win.v("loop")) === 1; low: true }
            }

            // Slices mode: a line per slice, a key name on each.
            Item {
                id: sliceLayer
                anchors.fill: parent
                visible: win.ready && win.sliceMode
                property int dragging: -1
                property real dragSecs: 0
                function at(i) { return i === dragging ? dragSecs : win.slices[i] }
                MouseArea {
                    anchors.fill: parent
                    acceptedButtons: Qt.LeftButton
                    onClicked: m => {
                        // Hear the slice under the click.
                        const s = wave.secsAt(m.x)
                        let i = 0
                        while (i + 1 < win.slices.length && win.slices[i + 1] <= s) ++i
                        if (win.slices.length) win.audition(win.firstSliceKey + i)
                    }
                    onDoubleClicked: m => win.app.setSlices(win.track.id, JSON.stringify(win.slices.concat([wave.secsAt(m.x)])), true)
                }
                Repeater {
                    model: win.slices.length
                    delegate: Item {
                        id: line
                        required property int index
                        readonly property bool hot: lineArea.containsMouse || sliceLayer.dragging === index
                        x: wave.xAt(sliceLayer.at(index)) - 6
                        width: 12
                        height: wave.height
                        Rectangle { x: 5; width: 2; height: parent.height; color: line.hot ? "#FFFFFF" : win.hit }
                        Rectangle {
                            x: 6; y: 4
                            width: keyText.implicitWidth + 10; height: 18; radius: 4
                            color: line.hot ? "#FFFFFF" : win.hit
                            DText { id: keyText; anchors.centerIn: parent; text: win.noteName(win.firstSliceKey + line.index); font.pixelSize: 10; font.weight: Font.Bold; color: "#0A0A0C" }
                        }
                        MouseArea {
                            id: lineArea
                            anchors.fill: parent
                            anchors.leftMargin: -3; anchors.rightMargin: -3
                            hoverEnabled: true
                            preventStealing: true
                            acceptedButtons: Qt.LeftButton | Qt.RightButton
                            cursorShape: Qt.SizeHorCursor
                            onPressed: m => {
                                if (m.button === Qt.RightButton) {
                                    const left = win.slices.filter((_, i) => i !== line.index)
                                    win.app.setSlices(win.track.id, JSON.stringify(left), true)
                                    return
                                }
                                sliceLayer.dragging = line.index
                                sliceLayer.dragSecs = win.slices[line.index]
                            }
                            onPositionChanged: m => { if (pressed && sliceLayer.dragging === line.index) sliceLayer.dragSecs = wave.secsAt(mapToItem(wave, m.x, 0).x) }
                            onReleased: {
                                if (sliceLayer.dragging !== line.index) return
                                const moved = win.slices.slice()
                                moved[line.index] = sliceLayer.dragSecs
                                sliceLayer.dragging = -1
                                win.app.setSlices(win.track.id, JSON.stringify(moved), true)
                            }
                        }
                        DToolTip { visible: lineArea.containsMouse; text: "Slice " + (line.index + 1) + " · key " + win.noteName(win.firstSliceKey + line.index) + " · drag to move, right-click to remove" }
                    }
                }
            }
        }
    }

    // ------------------------------------------------ controls
    Row {
        id: controls
        x: 10; y: waveCard.y + waveCard.height + 8
        spacing: 8
        height: 150

        Card {
            width: win.sliceMode ? 300 : 200
            title: win.sliceMode ? "SLICES" : "PLAYBACK"
            Column {
                visible: win.sliceMode
                x: 12; y: 32
                spacing: 8
                Row {
                    spacing: 4
                    DText { anchors.verticalCenter: parent.verticalCenter; text: "Up to"; font.pixelSize: 11; color: win.dim; rightPadding: 4 }
                    Repeater {
                        model: ["4", "8", "16", "32"]
                        delegate: TextButton {
                            required property string modelData
                            required property int index
                            small: true
                            width: 34
                            label: modelData
                            active: Math.round(win.v("slice_count")) === index
                            onClicked: win.set("slice_count", index, true)
                        }
                    }
                }
                Row {
                    spacing: 6
                    readonly property int count: [4, 8, 16, 32][Math.round(win.v("slice_count"))] || 8
                    TextButton { label: "Even"; tip: "Cut into equal pieces"; enabled: win.ready; onClicked: win.app.sliceSample(win.track.id, "even", parent.count) }
                    TextButton { label: "Detect beats"; tip: "Cut where each hit starts"; enabled: win.ready; onClicked: win.app.sliceSample(win.track.id, "beats", parent.count) }
                }
                TextButton {
                    label: "Make pattern"
                    tip: "A MIDI region that plays the slices in order: move its notes to remix the loop"
                    enabled: win.ready && win.slices.length > 0
                    onClicked: win.app.slicesToPattern(win.track.id)
                }
            }
            Column {
                visible: !win.sliceMode
                x: 12; y: 34
                spacing: 10
                Toggle2 { label: "Loop while held"; key: "loop" }
                Toggle2 { label: "Reverse"; key: "reverse" }
                DText { text: "Root: " + win.noteName(Math.round(win.v("root"))) + " plays it as recorded"; font.pixelSize: 10; color: win.faint; width: 176; wrapMode: Text.WordWrap }
            }
        }
        Card {
            width: 172
            title: "PITCH"
            Row {
                x: 8; y: 34
                Knob { key: "root"; label: "Root"; whole: true; display: win.noteName(Math.round(win.v("root"))); visible: !win.sliceMode }
                Knob { key: "tune"; label: "Tune"; bipolar: true; whole: true }
                Knob { key: "fine"; label: "Fine"; bipolar: true; visible: win.sliceMode }
            }
        }
        Card {
            width: 312
            title: "ENVELOPE"
            Row {
                x: 8; y: 34
                Knob { key: "attack"; label: "Attack" }
                Knob { key: "decay"; label: "Decay" }
                Knob { key: "sustain"; label: "Sustain" }
                Knob { key: "release"; label: "Release" }
            }
        }
        Card {
            width: win.width - 20 - 8 * 3 - (win.sliceMode ? 300 : 200) - 172 - 312
            title: "TONE & OUTPUT"
            Row {
                x: 8; y: 34
                Knob { key: "cutoff"; label: "Cutoff" }
                Knob { key: "velocity"; label: "Velocity" }
                Knob { key: "volume"; label: "Volume"; bipolar: true }
            }
        }
    }

    // ------------------------------------------------ keyboard
    Rectangle {
        x: 10
        y: controls.y + controls.height + 8
        width: parent.width - 20
        height: parent.height - y - 10
        radius: 8
        color: "#0C0D10"
        border.color: win.cardLine
        Item {
            id: keys
            x: 10; y: 10
            width: parent.width - 20
            height: parent.height - 20
            readonly property int first: win.sliceMode ? 36 : 48
            readonly property int octaves: 3
            readonly property real whiteW: width / (octaves * 7 + 1)
            Repeater {
                model: keys.octaves * 7 + 1
                delegate: KeyItem {
                    required property int index
                    key: keys.first + Math.floor(index / 7) * 12 + [0, 2, 4, 5, 7, 9, 11][index % 7]
                    black: false
                    x: index * keys.whiteW
                    width: keys.whiteW - 1.5
                    height: keys.height
                }
            }
            Repeater {
                model: keys.octaves * 5
                delegate: KeyItem {
                    required property int index
                    key: keys.first + Math.floor(index / 5) * 12 + [1, 3, 6, 8, 10][index % 5]
                    black: true
                    x: (Math.floor(index / 5) * 7 + [0, 1, 3, 4, 5][index % 5] + 1) * keys.whiteW - width / 2 - 0.75
                    width: keys.whiteW * 0.6
                    height: keys.height * 0.6
                    z: 2
                }
            }
        }
    }

    Keys.onEscapePressed: closeRequested()

    // ================================================================ parts
    component Card: Rectangle {
        id: card
        property string title
        height: 150
        radius: 8
        color: win.cardBg
        border.color: win.cardLine
        DText { x: 12; y: 10; text: card.title; font.pixelSize: 9; font.weight: Font.Bold; font.letterSpacing: 1; color: win.faint }
    }

    // A start / end / loop line on the waveform, dragged sideways.
    component Marker: Item {
        id: mk
        property string key
        property string label
        property color tint
        property bool flip: false
        property bool low: false
        width: 12
        height: parent.height
        transform: Translate { x: -6 }
        Rectangle { x: 5; width: 2; height: parent.height; color: mk.tint }
        Rectangle {
            x: mk.flip ? 6 - width : 6
            y: mk.low ? parent.height - 22 : 4
            width: mkText.implicitWidth + 10; height: 18; radius: 4
            color: mk.tint
            DText { id: mkText; anchors.centerIn: parent; text: mk.label; font.pixelSize: 9; font.weight: Font.Bold; color: "#0A0A0C" }
        }
        MouseArea {
            anchors.fill: parent
            anchors.leftMargin: -4; anchors.rightMargin: -4
            preventStealing: true
            cursorShape: Qt.SizeHorCursor
            onPositionChanged: m => {
                if (!pressed) return
                const x = mapToItem(wave, m.x, 0).x
                win.set(mk.key, Math.max(0, Math.min(100, x / wave.width * 100)), false)
            }
            onReleased: win.set(mk.key, win.v(mk.key), true)
        }
    }

    component Toggle2: AbstractButton {
        id: tg
        property string label
        property string key
        readonly property bool on: Math.round(win.v(key)) === 1
        implicitWidth: tgRow.implicitWidth
        implicitHeight: 20
        hoverEnabled: true
        onClicked: win.set(key, on ? 0 : 1, true)
        contentItem: Row {
            id: tgRow
            spacing: 8
            Rectangle {
                width: 30; height: 16; radius: 8
                color: tg.on ? win.text : "#26282D"
                Rectangle { x: tg.on ? 16 : 2; y: 2; width: 12; height: 12; radius: 6; color: tg.on ? "#0A0A0C" : "#80848D"; Behavior on x { NumberAnimation { duration: 90 } } }
            }
            DText { anchors.verticalCenter: parent.verticalCenter; text: tg.label; font.pixelSize: 11; color: tg.hovered ? win.text : "#B5B8BF" }
        }
    }

    component KeyItem: Item {
        id: k
        property int key
        property bool black
        readonly property bool held: !!win.activity && !!win.track && win.app.selectedTrack === win.track.id && win.activity.lit(key)
        readonly property int slice: key - win.firstSliceKey
        readonly property bool hasSlice: win.sliceMode && slice >= 0 && slice < win.slices.length
        Rectangle {
            anchors.fill: parent
            radius: 3
            gradient: Gradient {
                GradientStop { position: 0; color: k.black ? "#111216" : "#CFD1D6" }
                GradientStop { position: 1; color: k.black ? "#26282E" : "#ECEDF0" }
            }
            Rectangle {
                anchors.fill: parent
                radius: 3
                color: win.hit
                opacity: k.held ? 0.85 : 0
                Behavior on opacity { NumberAnimation { duration: k.held ? 20 : 220 } }
            }
        }
        DText {
            visible: k.hasSlice
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.bottom: parent.bottom; anchors.bottomMargin: k.black ? 6 : 20
            text: k.slice + 1
            font.pixelSize: 9
            font.weight: Font.Bold
            color: k.black ? win.hit : "#2F5DB0"
        }
        DText {
            visible: !k.black && k.key % 12 === 0
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.bottom: parent.bottom; anchors.bottomMargin: 5
            text: win.noteName(k.key)
            font.pixelSize: 8
            color: "#80848D"
        }
        MouseArea {
            anchors.fill: parent
            onPressed: { if (win.track && win.app.selectedTrack !== win.track.id) win.app.selectTrack(win.track.id); win.app.noteOn(k.key, 105) }
            onReleased: win.app.noteOff(k.key)
            onCanceled: win.app.noteOff(k.key)
        }
    }

    component TextButton: AbstractButton {
        id: tb
        property string label
        property bool active: false
        property bool small: false
        property string tip
        implicitWidth: Math.max(28, tbText.implicitWidth + 20)
        height: tb.small ? 22 : 28
        hoverEnabled: true
        opacity: enabled ? 1 : 0.4
        background: Rectangle {
            radius: 6
            color: tb.active ? "#26282D" : tb.pressed ? "#222327" : tb.hovered ? "#1B1C20" : "#17181C"
            border.color: tb.active ? win.text : "#24262B"
        }
        contentItem: Item {
            DText {
                id: tbText
                anchors.centerIn: parent
                text: tb.label
                font.pixelSize: tb.small ? 10 : 11
                font.weight: Font.DemiBold
                color: tb.active || tb.hovered ? win.text : "#A5A9B1"
            }
        }
        DToolTip { visible: tb.hovered && tb.tip !== ""; text: tb.tip }
    }

    component Knob: Item {
        id: knob
        property string key
        property string label
        property bool bipolar: false
        property bool whole: false
        property string display: ""
        readonly property var def: win.param(key)
        readonly property real value: win.v(key)
        function posOf(v) {
            if (!def) return 0
            const x = Math.max(def.min, Math.min(def.max, v))
            if (def.log && def.min > 0) return Math.log(x / def.min) / Math.log(def.max / def.min)
            return (x - def.min) / (def.max - def.min)
        }
        function valueOf(p) {
            p = Math.max(0, Math.min(1, p))
            const out = def.log && def.min > 0 ? def.min * Math.pow(def.max / def.min, p) : def.min + p * (def.max - def.min)
            return whole ? Math.round(out) : out
        }
        function fmt(v) {
            if (display !== "") return display
            if (!def) return ""
            switch (def.unit) {
            case "dB": return (v > 0.05 ? "+" : "") + v.toFixed(1) + " dB"
            case "st": return (v > 0 ? "+" : "") + Math.round(v) + " st"
            case "ct": return (v > 0 ? "+" : "") + Math.round(v) + " ct"
            case "%": return Math.round(v) + "%"
            case "ms": return v >= 1000 ? (v / 1000).toFixed(2) + " s" : Math.round(v) + " ms"
            case "Hz": return v >= 1000 ? (v / 1000).toFixed(1) + " kHz" : Math.round(v) + " Hz"
            default: return v.toFixed(1)
            }
        }
        readonly property real pos: posOf(value)
        readonly property real zero: bipolar && def ? posOf(def.min < 0 && def.max > 0 ? 0 : def.default) : 0
        readonly property bool hot: area.containsMouse || area.pressed
        width: 76; height: 100
        Item {
            id: dial
            y: 4
            width: 42; height: 42
            anchors.horizontalCenter: parent.horizontalCenter
            Shape {
                anchors.fill: parent
                preferredRendererType: Shape.CurveRenderer
                ShapePath {
                    strokeColor: win.trackLine; strokeWidth: 2.5; fillColor: "transparent"; capStyle: ShapePath.RoundCap
                    PathAngleArc { centerX: 21; centerY: 21; radiusX: 18; radiusY: 18; startAngle: 135; sweepAngle: 270 }
                }
                ShapePath {
                    strokeColor: win.text; strokeWidth: 2.5; fillColor: "transparent"; capStyle: ShapePath.RoundCap
                    PathAngleArc {
                        centerX: 21; centerY: 21; radiusX: 18; radiusY: 18
                        startAngle: 135 + 270 * Math.min(knob.zero, knob.pos)
                        sweepAngle: Math.max(0.01, 270 * Math.abs(knob.pos - knob.zero))
                    }
                }
            }
            Rectangle {
                anchors.centerIn: parent
                width: 28; height: 28; radius: 14
                color: knob.hot ? "#1E2024" : "#18191D"
                border.color: "#2A2C31"
                Item {
                    anchors.fill: parent
                    rotation: -135 + 270 * knob.pos
                    Rectangle { anchors.horizontalCenter: parent.horizontalCenter; y: 3; width: 2; height: 9; radius: 1; color: win.text }
                }
            }
        }
        DText { anchors.horizontalCenter: parent.horizontalCenter; y: 52; text: knob.label; font.pixelSize: 10; font.weight: Font.DemiBold; color: knob.hot ? win.text : win.dim }
        DText { anchors.horizontalCenter: parent.horizontalCenter; y: 66; text: knob.fmt(knob.value); font.pixelSize: 10; color: win.text }
        MouseArea {
            id: area
            anchors.fill: dial
            hoverEnabled: true
            preventStealing: true
            cursorShape: Qt.SizeVerCursor
            property real startY
            property real startPos
            onPressed: m => { startY = m.y; startPos = knob.pos }
            onPositionChanged: m => {
                if (!pressed || !knob.def) return
                const fine = m.modifiers & Qt.ShiftModifier ? 0.25 : 1
                win.set(knob.key, knob.valueOf(startPos + (startY - m.y) / 180 * fine), false)
            }
            onReleased: win.set(knob.key, knob.value, true)
            onDoubleClicked: if (knob.def) win.set(knob.key, knob.def.default, true)
        }
    }
}
