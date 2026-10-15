import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Shapes
import Isai

// "Isai Drums": a step sequencer for a drum track. Each row is a drum, each
// column a 16th note of one bar; the steps are the notes of a MIDI region on
// the track (so the piano roll, copy/paste and looping all work on them too).
//   • Click a step to add a hit, click it again to remove it; drag to paint.
//     Shift-click for an accent, ⌥-click for a soft hit.
//   • Click a drum's name to hear it and pick it for the knobs on the right.
//   • Bars: how long the pattern is; the numbered tabs pick which bar you edit.
//   • Patterns fill the bar with a starting groove; Swing pushes off-beats late.
Item {
    id: win
    property var app
    /// The drum track (an entry of tracksJson), or null when closed.
    property var track: null
    property Item backdrop: null
    property var activity: null
    property var level: [0, 0]
    signal closeRequested()

    readonly property bool shown: !!track
    readonly property var inst: track ? track.instrument : null
    readonly property var def: {
        try { return JSON.parse(app.instrumentKindsJson || "[]").find(k => k.key === "drums") || null } catch (e) { return null }
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

    // Graphite, like the synth: white for values, blue for hits, amber for accents.
    readonly property color bg: "#0D0E10"
    readonly property color cardBg: "#131417"
    readonly property color cardLine: "#1F2125"
    readonly property color well: "#0A0A0C"
    readonly property color text: "#E6E7EA"
    readonly property color dim: "#80848D"
    readonly property color faint: "#5D6169"
    readonly property color trackLine: "#26282D"
    readonly property color hit: "#4C8DFF"
    readonly property color accent: "#D2A560"

    readonly property real fit: parent ? Math.min(1, (parent.width - 16) / 1060, (parent.height - 16) / 640) : 1
    width: 1060
    height: 640
    x: parent ? (parent.width - width) / 2 : 0
    y: parent ? (parent.height - height) / 2 : 0
    visible: opacity > 0
    opacity: shown ? 1 : 0
    scale: fit * (shown ? 1 : 0.97)
    Behavior on opacity { NumberAnimation { duration: Theme.fast } }
    z: 150

    // ------------------------------------------------------------ pieces
    readonly property var pieces: [
        { name: "Kick", key: 36, id: "kick" }, { name: "Snare", key: 38, id: "snare" },
        { name: "Clap", key: 39, id: "clap" }, { name: "Rim", key: 37, id: "rim" },
        { name: "Closed hat", key: 42, id: "chh" }, { name: "Open hat", key: 46, id: "ohh" },
        { name: "Low tom", key: 43, id: "ltom" }, { name: "High tom", key: 48, id: "htom" },
        { name: "Crash", key: 49, id: "crash" }, { name: "Cowbell", key: 56, id: "cowbell" }
    ]
    property int picked: 0

    // ------------------------------------------------------------ the pattern region
    property int regionId: -1
    readonly property var region: {
        if (!track) return null
        return track.regions.find(r => r.id === regionId && r.midi) || null
    }
    readonly property real barBeats: TimeMap.barLenAt(region ? region.start : app.positionBeats)
    readonly property real stepBeats: 0.25
    readonly property int steps: Math.round(barBeats / stepBeats)
    readonly property int bars: region ? Math.max(1, Math.round(region.beats / barBeats)) : 1
    property int page: 0
    onBarsChanged: if (page >= bars) page = bars - 1
    onShownChanged: if (shown) Qt.callLater(pickRegion)
    onTrackChanged: if (track) Qt.callLater(pickRegion)
    /// Edits the selected MIDI region on this track, else its first one, else
    /// makes a one-bar region at the playhead's bar.
    function pickRegion() {
        if (!track) return
        const sel = track.regions.find(r => r.id === app.selectedRegion && r.midi)
        const first = track.regions.find(r => r.midi)
        const r = sel || first
        if (r) { regionId = r.id; return }
        const at = TimeMap.barStart(TimeMap.barAt(app.positionBeats))
        regionId = app.addMidiRegion(track.id, at, barBeats)
        page = 0
    }
    /// Notes in the region by step on the current page: { "key:step": [index, velocity] }.
    readonly property var cells: {
        const out = {}
        if (!region) return out
        const notes = region.midi.notes, offset = region.midi.offset
        const from = page * barBeats
        notes.forEach((n, i) => {
            const rel = n[0] - offset - from
            const step = Math.round(rel / stepBeats)
            if (step >= 0 && step < steps && Math.abs(rel - step * stepBeats) < stepBeats * 0.3)
                out[n[2] + ":" + step] = [i, n[3]]
        })
        return out
    }
    function toggle(key, step, velocity, want) {
        if (!region) return
        const cell = cells[key + ":" + step]
        if (cell && want !== true) app.removeNote(region.id, cell[0])
        else if (!cell && want !== false) app.addNote(region.id, page * barBeats + step * stepBeats, stepBeats, key, velocity)
    }
    function setBars(n) {
        if (!region) return
        app.trimRegion(region.id, false, region.start + n * barBeats)
    }
    function audition(key) {
        if (!track) return
        if (app.selectedTrack !== track.id) app.selectTrack(track.id)
        app.noteOn(key, 110)
        Qt.callLater(() => app.noteOff(key))
    }
    /// The step the playhead is on, when it's inside the current page.
    readonly property int playingStep: {
        if (!region || !app.playing) return -1
        const rel = app.positionBeats - region.start - page * barBeats
        if (region.span !== undefined && app.positionBeats >= region.start + region.span) return -1
        const inBar = ((rel % (bars * barBeats)) + bars * barBeats) % (bars * barBeats)
        return inBar < barBeats ? Math.floor(inBar / stepBeats) : -1
    }

    // ------------------------------------------------------------ patterns
    // Steps per drum for one bar of 16ths. "x" = hit, "X" = accent, "o" = soft.
    readonly property var patterns: [
        { name: "Rock", rows: { 36: "x.......x.x.....", 38: "....X.......X...", 42: "x.x.x.x.x.x.x.x." } },
        { name: "House", rows: { 36: "X...X...X...X...", 39: "....x.......x...", 42: "..x...x...x...x.", 46: "..o...o...o...o." } },
        { name: "Hip-hop", rows: { 36: "x......x..x.....", 38: "....X.......X...", 42: "x.x.x.x.x.x.x.xo" } },
        { name: "Trap", rows: { 36: "x.........x.....", 38: "........X.......", 42: "x.xox.xox.xxx.xo", 46: "..............x." } },
        { name: "Reggaeton", rows: { 36: "x...x...x...x...", 38: "...x..x....x..x.", 42: "x.x.x.x.x.x.x.x." } },
        { name: "Breakbeat", rows: { 36: "x.x.......x.....", 38: "....X..o.o..X..o", 42: "x.x.x.x.x.x.x.x." } },
        { name: "Disco", rows: { 36: "x...x...x...x...", 38: "....x.......x...", 46: "..x...x...x...x.", 42: "x...x...x...x..." } },
        { name: "Bossa nova", rows: { 36: "x..x..x.x..x..x.", 37: "x..x...x..x..x..", 42: "x.x.x.x.x.x.x.x." } }
    ]
    function loadPattern(p) {
        if (!region) return
        // Clear this bar, then write the pattern in (two undo steps).
        const mine = Object.keys(cells).map(k => cells[k][0])
        if (mine.length) app.removeNotes(region.id, mine.join(","))
        const notes = []
        for (const key in p.rows) {
            const row = p.rows[key]
            for (let s = 0; s < Math.min(steps, row.length); ++s) {
                const c = row[s]
                if (c === ".") continue
                notes.push([page * barBeats + s * stepBeats, stepBeats, Number(key), c === "X" ? 127 : c === "o" ? 60 : 100])
            }
        }
        if (notes.length) app.addNotes(region.id, JSON.stringify(notes))
    }
    function clearBar() {
        const mine = Object.keys(cells).map(k => cells[k][0])
        if (region && mine.length) app.removeNotes(region.id, mine.join(","))
    }

    // ================================================================ body
    Rectangle {
        anchors.fill: parent
        radius: 12
        color: win.bg
        border.color: "#25272B"
        MouseArea { anchors.fill: parent; acceptedButtons: Qt.AllButtons; onWheel: w => w.accepted = true }
    }

    // ------------------------------------------------ top bar
    Rectangle {
        id: topBar
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
            DText { anchors.verticalCenter: parent.verticalCenter; text: "Isai Drums"; font.pixelSize: 13; font.weight: Font.Bold; color: win.text }
            DText { anchors.verticalCenter: parent.verticalCenter; text: win.track ? "·  " + win.track.name : ""; font.pixelSize: 11; color: win.faint }
        }
        Row {
            anchors.centerIn: parent
            spacing: 8
            TextButton {
                label: "Patterns ▾"
                tip: "Fill this bar with a starting groove"
                onClicked: patternMenu.opened ? patternMenu.close() : patternMenu.open()
                MenuPopup {
                    id: patternMenu
                    y: parent.height + 4
                    width: 200
                    items: win.patterns.map(p => ({ label: p.name, action: () => win.loadPattern(p) }))
                        .concat([{ separator: true }, { label: "Clear this bar", danger: true, action: () => win.clearBar() }])
                }
            }
            Rectangle { width: 1; height: 20; color: "#24262B"; anchors.verticalCenter: parent.verticalCenter }
            DText { anchors.verticalCenter: parent.verticalCenter; text: "Bars"; font.pixelSize: 11; color: win.faint }
            Repeater {
                model: [1, 2, 4]
                delegate: TextButton {
                    required property int modelData
                    width: 30
                    label: modelData
                    active: win.bars === modelData
                    tip: "Make the pattern " + modelData + (modelData === 1 ? " bar" : " bars") + " long"
                    onClicked: win.setBars(modelData)
                }
            }
            Rectangle { width: 1; height: 20; color: "#24262B"; anchors.verticalCenter: parent.verticalCenter }
            TextButton {
                label: win.app.playing ? "■  Stop" : "▶  Play pattern"
                tip: "Loop this pattern (Space plays from the playhead)"
                onClicked: {
                    if (win.app.playing) { win.app.togglePlay(); return }
                    if (!win.region) return
                    win.app.setCycleRange(win.region.start, win.region.start + win.region.span)
                    win.app.seekBeats(win.region.start)
                    win.app.togglePlay()
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
            DToolTip { visible: closeButton.hovered; text: "Close (Esc)" }
        }
    }

    // ------------------------------------------------ the grid
    Rectangle {
        id: gridCard
        x: 10; y: 56
        width: parent.width - 20 - side.width - 8
        height: parent.height - y - 10
        radius: 8
        color: win.cardBg
        border.color: win.cardLine

        readonly property real headerW: 118
        readonly property real cellW: (width - headerW - 24) / win.steps
        readonly property real rowH: (height - 70) / win.pieces.length

        // Bar tabs
        Row {
            x: 12; y: 10
            spacing: 4
            DText { anchors.verticalCenter: parent.verticalCenter; text: "BAR"; font.pixelSize: 9; font.weight: Font.Bold; font.letterSpacing: 1; color: win.faint; rightPadding: 4 }
            Repeater {
                model: win.bars
                delegate: TextButton {
                    required property int index
                    width: 28; small: true
                    label: index + 1
                    active: win.page === index
                    onClicked: win.page = index
                }
            }
        }
        DText {
            anchors.right: parent.right; anchors.rightMargin: 12
            y: 14
            text: "Click a step to add a hit  ·  ⇧ accent  ·  ⌥ soft  ·  drag to paint"
            font.pixelSize: 10
            color: win.faint
        }

        // Step numbers
        Repeater {
            model: win.steps
            delegate: DText {
                required property int index
                x: gridCard.headerW + 12 + index * gridCard.cellW
                y: 40
                width: gridCard.cellW
                horizontalAlignment: Text.AlignHCenter
                text: index % 4 === 0 ? (index / 4 + 1) : "·"
                font.pixelSize: 10
                font.weight: index % 4 === 0 ? Font.Bold : Font.Normal
                color: win.playingStep === index ? win.text : win.faint
            }
        }

        // Rows
        Repeater {
            model: win.pieces
            delegate: Item {
                id: row
                required property var modelData
                required property int index
                readonly property bool lit: !!win.activity && win.activity.lit(modelData.key)
                x: 12
                y: 58 + index * gridCard.rowH
                width: gridCard.width - 24
                height: gridCard.rowH
                // Name: hear it and pick it.
                Rectangle {
                    width: gridCard.headerW - 8
                    height: parent.height - 4
                    y: 2
                    radius: 5
                    color: win.picked === row.index ? "#26282D" : nameArea.containsMouse ? "#1B1C20" : "transparent"
                    Rectangle {
                        x: 6; anchors.verticalCenter: parent.verticalCenter
                        width: 6; height: 6; radius: 3
                        color: row.lit ? win.hit : "#2C2E33"
                        Behavior on color { ColorAnimation { duration: row.lit ? 20 : 250 } }
                    }
                    DText {
                        x: 18; anchors.verticalCenter: parent.verticalCenter
                        text: row.modelData.name
                        font.pixelSize: 11
                        font.weight: win.picked === row.index ? Font.DemiBold : Font.Normal
                        color: win.picked === row.index ? win.text : "#B5B8BF"
                    }
                    MouseArea {
                        id: nameArea
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onPressed: { win.picked = row.index; win.audition(row.modelData.key) }
                    }
                }
                // Steps
                Repeater {
                    model: win.steps
                    delegate: Rectangle {
                        id: cell
                        required property int index
                        readonly property var note: win.cells[row.modelData.key + ":" + index]
                        readonly property int vel: note ? note[1] : 0
                        readonly property bool beatStart: index % 4 === 0
                        x: gridCard.headerW + index * gridCard.cellW + 1
                        y: 3
                        width: gridCard.cellW - 3
                        height: row.height - 6
                        radius: 4
                        color: note ? (vel >= 120 ? win.accent : win.hit)
                             : win.playingStep === index ? "#24262B"
                             : Math.floor(index / 4) % 2 === 0 ? "#1A1B1F" : "#16171A"
                        opacity: note ? 0.45 + 0.55 * vel / 127 : 1
                        border.color: win.playingStep === index ? "#3A3D44" : "transparent"
                    }
                }
                // One mouse area over the row's steps, so a drag paints across them.
                MouseArea {
                    x: gridCard.headerW
                    width: gridCard.cellW * win.steps
                    height: parent.height
                    preventStealing: true
                    property bool adding: true
                    property int last: -1
                    function stepAt(mx) { return Math.max(0, Math.min(win.steps - 1, Math.floor(mx / gridCard.cellW))) }
                    function velocity(m) { return m.modifiers & Qt.ShiftModifier ? 127 : m.modifiers & Qt.AltModifier ? 60 : 100 }
                    onPressed: m => {
                        const s = stepAt(m.x)
                        adding = !win.cells[row.modelData.key + ":" + s]
                        last = s
                        win.picked = row.index
                        win.toggle(row.modelData.key, s, velocity(m), adding)
                        if (adding) win.audition(row.modelData.key)
                    }
                    onPositionChanged: m => {
                        if (!pressed) return
                        const s = stepAt(m.x)
                        if (s === last) return
                        last = s
                        win.toggle(row.modelData.key, s, velocity(m), adding)
                    }
                }
            }
        }
    }

    // ------------------------------------------------ side panel
    Rectangle {
        id: side
        x: parent.width - width - 10; y: 56
        width: 286
        height: parent.height - y - 10
        radius: 8
        color: win.cardBg
        border.color: win.cardLine

        DText { x: 14; y: 12; text: "KIT"; font.pixelSize: 9; font.weight: Font.Bold; font.letterSpacing: 1; color: win.faint }
        Flow {
            x: 14; y: 30
            width: parent.width - 28
            spacing: 4
            Repeater {
                model: ["808", "Club", "Acoustic", "Lo-fi", "Trap"]
                delegate: TextButton {
                    required property string modelData
                    required property int index
                    label: modelData
                    active: Math.round(win.v("kit")) === index
                    onClicked: { win.set("kit", index, true); win.audition(36) }
                }
            }
        }
        Row {
            x: 10; y: 98
            spacing: 6
            Knob { key: "volume"; label: "Volume"; bipolar: true }
            Knob { key: "drive"; label: "Drive" }
            Item {
                width: 74; height: 74
                Knob {
                    id: swingKnob
                    anchors.horizontalCenter: parent.horizontalCenter
                    key: ""
                    label: "Swing"
                    manual: true
                    manualValue: win.swing
                    display: Math.round(50 + win.swing * 50) + "%"
                    onMoved: (v, done) => { win.swing = v; if (done) win.applySwing() }
                }
            }
        }
        Rectangle { x: 14; y: 186; width: parent.width - 28; height: 1; color: win.cardLine }
        DText {
            x: 14; y: 198
            text: win.pieces[win.picked].name.toUpperCase()
            font.pixelSize: 9; font.weight: Font.Bold; font.letterSpacing: 1
            color: win.text
        }
        DText { anchors.right: parent.right; anchors.rightMargin: 14; y: 198; text: "click a name to pick"; font.pixelSize: 9; color: win.faint }
        Grid {
            x: 10; y: 220
            columns: 2
            rowSpacing: 4
            columnSpacing: 30
            readonly property string pieceId: win.pieces[win.picked].id
            Knob { key: parent.pieceId + "_level"; label: "Level"; bipolar: true }
            Knob { key: parent.pieceId + "_tune"; label: "Tune"; bipolar: true; whole: true }
            Knob { key: parent.pieceId + "_decay"; label: "Length" }
            Knob { key: parent.pieceId + "_pan"; label: "Pan"; bipolar: true }
        }
        // Output level.
        Row {
            anchors.bottom: parent.bottom; anchors.bottomMargin: 16
            x: 14
            spacing: 3
            Repeater {
                model: 2
                delegate: Rectangle {
                    id: meter
                    required property int index
                    width: side.width - 28; height: 4; radius: 2
                    color: "#1D1F23"
                    Rectangle {
                        width: parent.width * Math.max(0, Math.min(1, win.level[meter.index] || 0))
                        height: parent.height; radius: 2
                        color: (win.level[meter.index] || 0) > 0.95 ? "#E0685A" : "#B9BCC3"
                    }
                }
            }
        }
    }

    /// Swing for this pattern: 0 straight, 0.33 a triplet feel (applied to the notes on release).
    property real swing: 0
    function applySwing() {
        if (region) app.quantizeNotes(region.id, "", stepBeats, 1, false, swing * 0.5)
    }

    Keys.onEscapePressed: closeRequested()

    // ================================================================ parts
    component TextButton: AbstractButton {
        id: tb
        property string label
        property bool active: false
        property bool small: false
        property string tip
        implicitWidth: Math.max(28, tbText.implicitWidth + 20)
        height: tb.small ? 22 : 28
        hoverEnabled: true
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

    // A knob for one instrument setting (or, with `manual`, any 0..1 value).
    component Knob: Item {
        id: knob
        property string key
        property string label
        property bool bipolar: false
        property bool whole: false
        property bool manual: false
        property real manualValue: 0
        property string display: ""
        signal moved(real value, bool done)
        readonly property var def: manual ? null : win.param(key)
        readonly property real value: manual ? manualValue : win.v(key)
        function posOf(v) {
            if (manual) return Math.max(0, Math.min(1, v / 0.66))
            if (!def) return 0
            const x = Math.max(def.min, Math.min(def.max, v))
            if (def.log && def.min > 0) return Math.log(x / def.min) / Math.log(def.max / def.min)
            return (x - def.min) / (def.max - def.min)
        }
        function valueOf(p) {
            p = Math.max(0, Math.min(1, p))
            if (manual) return p * 0.66
            const out = def.log && def.min > 0 ? def.min * Math.pow(def.max / def.min, p) : def.min + p * (def.max - def.min)
            return whole || def.choices.length ? Math.round(out) : out
        }
        function fmt(v) {
            if (display !== "") return display
            if (!def) return ""
            switch (def.unit) {
            case "dB": return (v > 0.05 ? "+" : "") + v.toFixed(1) + " dB"
            case "st": return (v > 0 ? "+" : "") + Math.round(v) + " st"
            case "%": return Math.round(v) + "%"
            case "×": return "×" + v.toFixed(2)
            default: return Math.abs(v) < 0.02 ? "C" : (v < 0 ? "L" : "R") + Math.round(Math.abs(v) * 100)
            }
        }
        readonly property real pos: posOf(value)
        readonly property real zero: bipolar && def ? posOf(def.min < 0 && def.max > 0 ? 0 : def.default) : 0
        readonly property bool hot: area.containsMouse || area.pressed
        width: 74; height: 74
        Item {
            id: dial
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
        DText { anchors.horizontalCenter: parent.horizontalCenter; y: 45; text: knob.label; font.pixelSize: 10; font.weight: Font.DemiBold; color: knob.hot ? win.text : win.dim }
        DText { anchors.horizontalCenter: parent.horizontalCenter; y: 58; text: knob.fmt(knob.value); font.pixelSize: 10; color: win.text }
        MouseArea {
            id: area
            anchors.fill: dial
            hoverEnabled: true
            preventStealing: true
            cursorShape: Qt.SizeVerCursor
            property real startY
            property real startPos
            function apply(v, done) { if (knob.manual) knob.moved(v, done); else win.set(knob.key, v, done) }
            onPressed: m => { startY = m.y; startPos = knob.pos }
            onPositionChanged: m => {
                if (!pressed || (!knob.def && !knob.manual)) return
                const fine = m.modifiers & Qt.ShiftModifier ? 0.25 : 1
                apply(knob.valueOf(startPos + (startY - m.y) / 180 * fine), false)
            }
            onReleased: apply(knob.value, true)
            onDoubleClicked: apply(knob.manual ? 0 : knob.def.default, true)
        }
    }
}
