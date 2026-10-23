import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Shapes
import Isai

// "Isai Synth": the analog-style synthesizer's own window. A calm graphite
// panel: white for values, amber for the LFO, blue for the keys you play.
// Drag the top bar to move it. Every control maps to one engine setting:
//   • OSC A / OSC B: wave, octave or pitch, fine tune, pulse width, A/B mix.
//   • SUB · NOISE · DRIVE: a square sub an octave or two down, noise, and
//     saturation before the filter.
//   • FILTER (low-, high- or band-pass): drag the curve (left/right cutoff,
//     up/down resonance). ENVELOPE: amp or filter, by its points or sliders.
//   • LFO: shape, target, rate, depth and a fade-in delay. UNISON: stacked,
//     detuned copies spread wide. VOICE: poly, mono, legato, glide.
//   • Play with the mouse or the computer keyboard while the window is open
//     (A W S E D F T G Y H U J K O L P ;, Z / X octave, Tab sustain).
Item {
    id: win
    property var app
    /// The synth track (an entry of tracksJson), or null when closed.
    property var track: null
    property Item backdrop: null
    property var activity: null
    property var level: [0, 0]
    signal closeRequested()

    readonly property bool shown: !!track
    readonly property var inst: track ? track.instrument : null
    readonly property var def: {
        try { return JSON.parse(app.instrumentKindsJson || "[]").find(k => k.key === "synth") || null } catch (e) { return null }
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
    function snapshot() {
        const all = {}
        if (def) for (const d of def.params) all[d.key] = v(d.key)
        return all
    }
    function choice(key) { return Math.round(v(key)) }
    /// A setting's position along its control, 0..1 (log settings on a log scale).
    function posOf(key, value) {
        const d = param(key)
        if (!d) return 0
        const x = Math.max(d.min, Math.min(d.max, value))
        if (d.log && d.min > 0) return Math.log(x / d.min) / Math.log(d.max / d.min)
        return (x - d.min) / (d.max - d.min)
    }
    function valueOf(key, pos, whole) {
        const d = param(key)
        const p = Math.max(0, Math.min(1, pos))
        const value = d.log && d.min > 0 ? d.min * Math.pow(d.max / d.min, p) : d.min + p * (d.max - d.min)
        return whole || d.choices.length ? Math.round(value) : value
    }
    function fmt(key, value) {
        const d = param(key)
        if (!d) return ""
        if (d.choices.length) return d.choices[Math.round(value)] || ""
        switch (d.unit) {
        case "%": return Math.round(value) + "%"
        case "ms": return value >= 1000 ? (value / 1000).toFixed(2) + " s" : Math.round(value) + " ms"
        case "s": return value.toFixed(1) + " s"
        case "Hz": return value >= 1000 ? (value / 1000).toFixed(1) + " kHz" : value.toFixed(value < 10 ? 2 : 0) + " Hz"
        case "dB": return (value > 0.05 ? "+" : "") + value.toFixed(1) + " dB"
        case "ct": return (value > 0.05 ? "+" : "") + value.toFixed(0) + " ct"
        case "st": return (value > 0 ? "+" : "") + Math.round(value) + " st"
        default: return value.toFixed(2)
        }
    }

    // Graphite, one light accent, amber for modulation, blue for played keys.
    readonly property color bg: "#0D0E10"
    readonly property color bar: "#101114"
    readonly property color cardBg: "#131417"
    readonly property color cardLine: "#1F2125"
    readonly property color well: "#0A0A0C"
    readonly property color wellLine: "#1A1B1F"
    readonly property color grid: "#17181C"
    readonly property color trackLine: "#26282D"
    readonly property color text: "#E6E7EA"
    readonly property color dim: "#80848D"
    readonly property color faint: "#5D6169"
    readonly property color accent: "#E4E6EA"
    readonly property color mod: "#D2A560"
    readonly property color keyLit: "#4C8DFF"
    function rgba(c, a) { const q = Qt.color(c); return Qt.rgba(q.r, q.g, q.b, a) }
    function css(c, a) { const q = Qt.color(c); return "rgba(" + Math.round(q.r * 255) + "," + Math.round(q.g * 255) + "," + Math.round(q.b * 255) + "," + a + ")" }

    // Laid out at 1120 × 780 and scaled down to fit smaller screens.
    readonly property real fit: parent ? Math.min(1, (parent.width - 16) / 1120, (parent.height - 16) / 780) : 1
    width: 1120
    height: 780
    x: parent ? (parent.width - width) / 2 : 0
    y: parent ? (parent.height - height) / 2 : 0
    visible: opacity > 0
    opacity: shown ? 1 : 0
    scale: fit * (shown ? 1 : 0.97)
    Behavior on opacity { NumberAnimation { duration: Theme.fast } }
    z: 150

    onShownChanged: {
        if (!shown) { presetName = ""; presetValues = null; abStore = ({}); abSlot = "A"; releaseAll(); return }
        reloadUserPresets()
        Qt.callLater(() => typingKeys.forceActiveFocus())
    }
    onTrackChanged: if (!track) live = {}

    // ------------------------------------------------------------ presets
    readonly property var presets: [
        { group: "Bass", items: [
            { name: "Deep Sub", v: { wave: 3, mix: 0, sub: 40, cutoff: 800, resonance: 5, env_amount: 10, attack: 2, decay: 400, sustain: 90, release: 120, mono: 1, volume: -4 } },
            { name: "808", v: { wave: 3, mix: 0, oct_a: 1, glide: 60, mono: 2, attack: 1, decay: 1800, sustain: 0, release: 200, drive: 35, cutoff: 1200, env_amount: 0, volume: -3 } },
            { name: "Acid Bass", v: { wave: 0, mix: 0, cutoff: 300, resonance: 85, env_amount: 70, f_attack: 1, f_decay: 250, f_sustain: 0, attack: 1, decay: 300, sustain: 60, release: 80, mono: 2, glide: 70, drive: 30 } },
            { name: "Reese", v: { wave: 0, wave_b: 0, detune: 25, mix: 50, unison: 2, uni_detune: 30, spread: 40, cutoff: 900, resonance: 20, mono: 1, sustain: 100, release: 150, lfo_target: 1, lfo_rate: 0.3, lfo_depth: 25 } },
            { name: "Wobble", v: { wave: 0, mix: 0, sub: 35, cutoff: 400, resonance: 60, lfo_target: 1, lfo_rate: 2, lfo_depth: 70, mono: 1, sustain: 100, drive: 25 } }
        ] },
        { group: "Lead", items: [
            { name: "Supersaw", v: { wave: 0, wave_b: 0, unison: 6, uni_detune: 35, spread: 90, cutoff: 7000, resonance: 15, env_amount: 20, attack: 5, decay: 400, sustain: 80, release: 400, volume: -9 } },
            { name: "Retro Square", v: { wave: 1, mix: 0, cutoff: 9000, resonance: 5, attack: 2, decay: 200, sustain: 80, release: 150, lfo_target: 0, lfo_rate: 5.5, lfo_depth: 15, lfo_delay: 350, mono: 2, glide: 40 } },
            { name: "Chiptune", v: { wave: 4, pulse_width: 25, mix: 0, cutoff: 18000, env_amount: 0, attack: 1, decay: 120, sustain: 60, release: 60 } },
            { name: "Screamer", v: { wave: 0, wave_b: 1, semi_b: 12, drive: 70, cutoff: 5000, resonance: 40, mono: 2, glide: 50, lfo_target: 0, lfo_depth: 12, lfo_rate: 6, lfo_delay: 250 } }
        ] },
        { group: "Pad", items: [
            { name: "Warm Pad", v: { wave: 0, wave_b: 0, detune: 12, unison: 3, uni_detune: 15, spread: 80, cutoff: 1400, resonance: 10, env_amount: 20, f_attack: 800, f_decay: 2000, f_sustain: 40, attack: 900, decay: 1500, sustain: 80, release: 2500 } },
            { name: "Strings", v: { wave: 0, unison: 4, uni_detune: 22, spread: 100, cutoff: 3200, attack: 400, release: 1500, sustain: 90, lfo_target: 0, lfo_rate: 5, lfo_depth: 6, lfo_delay: 600 } },
            { name: "Glass Pad", v: { wave: 2, wave_b: 3, semi_b: 12, mix: 40, cutoff: 6000, attack: 600, release: 3000, lfo_target: 2, lfo_rate: 0.3, lfo_depth: 20 } },
            { name: "Dark Drone", v: { wave: 0, wave_b: 1, semi_b: -12, cutoff: 500, resonance: 30, attack: 1500, release: 4000, sustain: 100, lfo_target: 1, lfo_rate: 0.1, lfo_depth: 40, noise: 5 } }
        ] },
        { group: "Pluck & keys", items: [
            { name: "Pluck", v: { wave: 0, wave_b: 1, cutoff: 600, resonance: 25, env_amount: 75, f_decay: 180, f_sustain: 0, attack: 1, decay: 350, sustain: 0, release: 250 } },
            { name: "Bell", v: { wave: 3, wave_b: 3, semi_b: 19, mix: 35, cutoff: 12000, env_amount: 0, attack: 1, decay: 1200, sustain: 0, release: 800 } },
            { name: "Brass Stab", v: { wave: 0, wave_b: 0, detune: 10, cutoff: 900, env_amount: 65, f_attack: 60, f_decay: 300, f_sustain: 30, attack: 30, decay: 200, sustain: 80, release: 200 } },
            { name: "Organ", v: { wave: 3, wave_b: 2, semi_b: 12, mix: 45, sub: 30, cutoff: 8000, env_amount: 0, attack: 3, sustain: 100, release: 60 } }
        ] },
        { group: "FX", items: [
            { name: "Wind", v: { wave: 5, mix: 0, filter_type: 2, cutoff: 1200, resonance: 70, lfo_target: 1, lfo_rate: 0.2, lfo_depth: 60, attack: 1500, release: 3000, sustain: 100, env_amount: 0 } },
            { name: "Riser", v: { wave: 5, mix: 0, cutoff: 200, f_attack: 4000, f_sustain: 100, env_amount: 100, attack: 2000, sustain: 100, release: 1000 } },
            { name: "Random Steps", v: { wave: 0, mix: 0, cutoff: 900, resonance: 70, lfo_target: 1, lfo_shape: 4, lfo_rate: 8, lfo_depth: 60, sustain: 100, release: 300 } },
            { name: "Snare Layer", v: { wave: 5, mix: 0, filter_type: 1, cutoff: 1500, env_amount: 0, attack: 1, decay: 180, sustain: 0, release: 120 } },
            { name: "Laser Zap", v: { wave: 1, mix: 0, cutoff: 3000, resonance: 70, env_amount: 100, f_decay: 120, f_sustain: 0, attack: 1, decay: 200, sustain: 0, release: 80, glide: 120, mono: 1 } }
        ] }
    ]
    property var userPresets: []
    function reloadUserPresets() {
        try { userPresets = JSON.parse(app.userPresets("synth") || "[]") } catch (e) { userPresets = [] }
    }
    /// Every preset in browsing order, for ◀ ▶.
    readonly property var allPresets: {
        const out = []
        for (const g of presets) for (const p of g.items) out.push({ name: p.name, group: g.group, values: p.v })
        for (const p of userPresets) out.push({ name: p.name, group: "Mine", values: p.values })
        return out
    }
    property string presetName: ""
    property var presetValues: null
    readonly property string presetGroup: {
        const p = allPresets.find(x => x.name === presetName)
        return p ? p.group : ""
    }
    readonly property bool presetEdited: {
        if (!presetValues || !inst) return false
        for (const k in presetValues) if (Math.abs(v(k) - presetValues[k]) > 1e-3) return true
        return false
    }
    function loadPreset(name, values) {
        const all = {}
        for (const d of def.params) all[d.key] = d.default
        Object.assign(all, values)
        live = {}
        app.setInstrumentParams(track.id, JSON.stringify(all), "Synth Preset: " + name)
        presetName = name
        presetValues = all
    }
    function stepPreset(dir) {
        const list = allPresets
        if (!list.length) return
        const i = list.findIndex(p => p.name === presetName)
        const n = list[(i < 0 ? (dir > 0 ? 0 : list.length - 1) : (i + dir + list.length) % list.length)]
        loadPreset(n.name, n.values)
    }
    function preview(name, values) {
        loadPreset(name, values)
        if (track && app.selectedTrack !== track.id) app.selectTrack(track.id)
        const c = firstKey + 12
        for (const [t, k] of [[0, c], [200, c + 7], [400, c + 12], [600, c + 3]]) schedule(t, k, 100, 180)
        for (const k of [c, c + 3, c + 7]) schedule(820, k, 95, 1200)
    }

    // A / B: two sounds to flip between while tweaking.
    property string abSlot: "A"
    property var abStore: ({})
    function switchAb(slot) {
        if (slot === abSlot || !track) return
        const store = Object.assign({}, abStore)
        store[abSlot] = { values: snapshot(), name: presetName }
        const next = store[slot] || store[abSlot]
        abStore = store
        abSlot = slot
        live = {}
        app.setInstrumentParams(track.id, JSON.stringify(next.values), "Synth " + slot)
        presetName = next.name
        presetValues = next.name ? presetValues : null
    }

    // ------------------------------------------------------------ playing
    property int octave: 3
    readonly property int octaves: 3
    readonly property int firstKey: (octave + 1) * 12
    property bool pedal: false
    function setPedal(down) { if (pedal !== down) { pedal = down; app.sustain(down) } }
    property var down: ({})
    property var downVel: ({})
    function press(key, velocity) {
        if (!track) return
        if (app.selectedTrack !== track.id) app.selectTrack(track.id)
        if (down[key]) return
        const d = Object.assign({}, down); d[key] = true; down = d
        const dv = Object.assign({}, downVel); dv[key] = velocity; downVel = dv
        app.noteOn(key, velocity)
    }
    function lift(key) {
        if (!down[key]) return
        const d = Object.assign({}, down); delete d[key]; down = d
        app.noteOff(key)
    }
    function releaseAll() {
        for (const k in down) lift(Number(k))
        for (const e of queue) if (!e.on) app.noteOff(e.key)
        queue = []
        setPedal(false)
    }
    property var queue: []
    function schedule(afterMs, key, velocity, lengthMs) {
        const t = Date.now() + afterMs
        queue = queue.concat([{ t: t, on: true, key: key, vel: velocity }, { t: t + lengthMs, on: false, key: key }])
    }
    Timer {
        interval: 4
        repeat: true
        running: win.queue.length > 0
        onTriggered: {
            const t = Date.now()
            const due = win.queue.filter(e => e.t <= t)
            if (!due.length) return
            win.queue = win.queue.filter(e => e.t > t)
            for (const e of due) e.on ? win.app.noteOn(e.key, e.vel) : win.app.noteOff(e.key)
        }
    }
    function lit(key) {
        if (down[key]) return true
        return !!activity && !!track && app.selectedTrack === track.id && activity.lit(key)
    }
    function velocityOf(key) {
        if (down[key] && downVel[key] !== undefined) return downVel[key]
        if (activity && activity.vel[key] !== undefined) return activity.vel[key]
        return 100
    }

    readonly property bool typing: shown && typingKeys.activeFocus
    readonly property var typingMap: ({
        [Qt.Key_A]: 0, [Qt.Key_W]: 1, [Qt.Key_S]: 2, [Qt.Key_E]: 3, [Qt.Key_D]: 4, [Qt.Key_F]: 5,
        [Qt.Key_T]: 6, [Qt.Key_G]: 7, [Qt.Key_Y]: 8, [Qt.Key_H]: 9, [Qt.Key_U]: 10, [Qt.Key_J]: 11,
        [Qt.Key_K]: 12, [Qt.Key_O]: 13, [Qt.Key_L]: 14, [Qt.Key_P]: 15, [Qt.Key_Semicolon]: 16
    })
    readonly property var keyLetters: ["A", "W", "S", "E", "D", "F", "T", "G", "Y", "H", "U", "J", "K", "O", "L", "P", ";"]
    Item {
        id: typingKeys
        property var playing: ({})
        Keys.onPressed: e => {
            if (e.isAutoRepeat) { e.accepted = true; return }
            if (e.modifiers & (Qt.ControlModifier | Qt.MetaModifier | Qt.AltModifier)) return
            if (e.key === Qt.Key_Z) win.octave = Math.max(0, win.octave - 1)
            else if (e.key === Qt.Key_X) win.octave = Math.min(7, win.octave + 1)
            else if (e.key === Qt.Key_Tab || e.key === Qt.Key_Backtab) win.setPedal(true)
            else if (e.key === Qt.Key_Space) win.app.togglePlay()
            else if (e.key === Qt.Key_Escape) win.closeRequested()
            else if (win.typingMap[e.key] !== undefined) {
                const note = win.firstKey + win.typingMap[e.key]
                const p = Object.assign({}, playing); p[e.key] = note; playing = p
                win.press(note, 100)
            } else return
            e.accepted = true
        }
        Keys.onReleased: e => {
            if (e.isAutoRepeat) { e.accepted = true; return }
            if (e.key === Qt.Key_Tab || e.key === Qt.Key_Backtab) { win.setPedal(false); e.accepted = true; return }
            const note = playing[e.key]
            if (note === undefined) return
            const p = Object.assign({}, playing); delete p[e.key]; playing = p
            win.lift(note)
            e.accepted = true
        }
        onActiveFocusChanged: if (!activeFocus) { for (const k in playing) win.lift(playing[k]); playing = {}; win.setPedal(false) }
    }

    // ================================================================ body
    Rectangle {
        anchors.fill: parent
        radius: 12
        color: win.bg
        border.color: "#25272B"
        MouseArea { anchors.fill: parent; acceptedButtons: Qt.AllButtons; onPressed: typingKeys.forceActiveFocus(); onWheel: w => w.accepted = true }
    }

    // ------------------------------------------------ top bar
    Rectangle {
        id: topBar
        x: 1; y: 1
        width: parent.width - 2
        height: 46
        radius: 11
        color: win.bar
        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 12; color: win.bar }
        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: "#1B1C20" }
        DragHandler {
            target: win
            // Free to move, as long as part of this bar stays on screen.
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
            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                width: 16; height: 16; radius: 4
                color: "transparent"
                border.color: win.accent
                border.width: 1.5
            }
            DText {
                anchors.verticalCenter: parent.verticalCenter
                text: "Isai Synth"
                font.pixelSize: 13
                font.weight: Font.Bold
                color: win.text
            }
            DText {
                anchors.verticalCenter: parent.verticalCenter
                text: win.track ? "·  " + win.track.name : ""
                font.pixelSize: 11
                color: win.faint
            }
            LearnButton { anchors.verticalCenter: parent.verticalCenter; topic: "synth"; backdrop: win.backdrop; implicitWidth: 24; implicitHeight: 24; iconSize: 13 }
        }

        // Preset field: click for the list, ◀ ▶ to step through.
        Rectangle {
            id: presetField
            anchors.centerIn: parent
            anchors.horizontalCenterOffset: -30
            width: 340; height: 30
            radius: 6
            color: presetArea.containsMouse || presetPopup.opened ? "#121316" : win.well
            border.color: presetPopup.opened ? "#3A3D44" : "#24262B"
            MouseArea {
                id: presetArea
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: { win.reloadUserPresets(); presetPopup.saving = false; presetPopup.opened ? presetPopup.close() : presetPopup.open() }
            }
            Row {
                x: 12
                anchors.verticalCenter: parent.verticalCenter
                spacing: 9
                Icon { anchors.verticalCenter: parent.verticalCenter; glyph: Icons.musicNotes; size: 12; color: win.dim }
                DText {
                    anchors.verticalCenter: parent.verticalCenter
                    width: Math.min(implicitWidth, 170)
                    elide: Text.ElideRight
                    text: win.presetName ? win.presetName + (win.presetEdited ? " *" : "") : "Init"
                    font.pixelSize: 13
                    color: win.text
                }
                Rectangle {
                    visible: win.presetGroup !== ""
                    anchors.verticalCenter: parent.verticalCenter
                    width: groupText.implicitWidth + 12; height: 18; radius: 4
                    color: "#1A1B1F"
                    DText { id: groupText; anchors.centerIn: parent; text: win.presetGroup.toUpperCase(); font.pixelSize: 9; font.weight: Font.Bold; font.letterSpacing: 1; color: "#9A9EA7" }
                }
            }
            Row {
                anchors.right: parent.right; anchors.rightMargin: 4
                anchors.verticalCenter: parent.verticalCenter
                spacing: 2
                SmallButton { glyph: Icons.caretRight; flip: true; tip: "Previous preset"; onClicked: win.stepPreset(-1) }
                SmallButton { glyph: Icons.caretRight; tip: "Next preset"; onClicked: win.stepPreset(1) }
            }
            SynthPresetPopup { id: presetPopup; y: parent.height + 6; x: (parent.width - width) / 2 }
        }
        TextButton {
            anchors.left: presetField.right; anchors.leftMargin: 8
            anchors.verticalCenter: parent.verticalCenter
            label: "Save"
            tip: "Save this sound to My presets"
            onClicked: { win.reloadUserPresets(); presetPopup.open(); presetPopup.startSaving() }
        }

        Row {
            anchors.right: parent.right
            anchors.rightMargin: 12
            anchors.verticalCenter: parent.verticalCenter
            spacing: 6
            TextButton { label: "A"; width: 30; active: win.abSlot === "A"; tip: "Sound A: tweak, then flip to B to compare"; onClicked: win.switchAb("A") }
            TextButton { label: "B"; width: 30; active: win.abSlot === "B"; tip: "Sound B: starts as a copy of A"; onClicked: win.switchAb("B") }
            Item { width: 8; height: 1 }
            AbstractButton {
                id: closeButton
                anchors.verticalCenter: parent.verticalCenter
                width: 28; height: 28
                hoverEnabled: true
                onClicked: win.closeRequested()
                background: Rectangle { radius: 6; color: closeButton.hovered ? "#1D1E22" : "transparent" }
                contentItem: Icon { glyph: Icons.x; size: 12; color: win.dim }
                DToolTip { visible: closeButton.hovered; text: "Close (Esc)" }
            }
        }
    }

    // ------------------------------------------------ cards
    readonly property real colA: 10
    readonly property real colB: 384
    readonly property real colC: 754
    readonly property real wA: 366
    readonly property real wB: 362
    readonly property real wC: 356
    readonly property real top0: 56
    readonly property bool anyPulse: choice("wave") === 4 || choice("wave_b") === 4

    // OSC A
    Card {
        x: win.colA; y: win.top0; width: win.wA; height: 196
        title: "OSC A"
        Chips { anchors.right: parent.right; anchors.rightMargin: 8; y: 5; key: "wave"; labels: ["SAW", "SQR", "TRI", "SIN", "PLS", "NSE"] }
        WaveDisplay {
            x: 10; y: 32; width: parent.width - 20; height: 90
            wave: win.choice("wave"); pulse: win.v("pulse_width")
            tag: (win.choice("oct_a") !== 2 ? win.fmt("oct_a", win.v("oct_a")) + " oct   " : "") + win.fmt("fine_a", win.v("fine_a"))
        }
        Row {
            anchors.horizontalCenter: parent.horizontalCenter
            y: 130
            spacing: 26
            Knob { key: "oct_a"; label: "Octave"; bipolar: true; tip: "Moves oscillator A up or down by whole octaves" }
            Knob { key: "fine_a"; label: "Fine"; bipolar: true; tip: "Fine-tunes oscillator A in cents (100 cents = a semitone)" }
            Knob { key: "pulse_width"; label: "Pulse W"; dimmed: !win.anyPulse; tip: "Width of the Pulse wave (both oscillators): thin and nasal to hollow" }
        }
    }
    // OSC B
    Card {
        x: win.colA; y: win.top0 + 204; width: win.wA; height: 196
        title: "OSC B"
        Chips { anchors.right: parent.right; anchors.rightMargin: 8; y: 5; key: "wave_b"; labels: ["SAW", "SQR", "TRI", "SIN", "PLS", "NSE"] }
        WaveDisplay {
            x: 10; y: 32; width: parent.width - 20; height: 90
            wave: win.choice("wave_b"); pulse: win.v("pulse_width")
            level: Math.max(0.15, win.v("mix") / 100)
            tag: win.fmt("semi_b", win.v("semi_b")) + "   " + win.fmt("detune", win.v("detune"))
        }
        Row {
            anchors.horizontalCenter: parent.horizontalCenter
            y: 130
            spacing: 26
            Knob { key: "semi_b"; label: "Pitch"; bipolar: true; whole: true; tip: "Oscillator B's pitch in semitones (+12 = an octave up, +7 = a fifth)" }
            Knob { key: "detune"; label: "Fine"; bipolar: true; tip: "Detunes oscillator B against A in cents: a few cents make it thick" }
            Knob { key: "mix"; label: "Mix A·B"; tip: "Balance between oscillator A (left) and B (right)"; display: Math.round(100 - win.v("mix")) + " · " + Math.round(win.v("mix")) }
        }
    }
    // SUB · NOISE · DRIVE
    Card {
        x: win.colA; y: win.top0 + 408; width: win.wA; height: 136
        title: "SUB · NOISE · DRIVE"
        Row {
            anchors.horizontalCenter: parent.horizontalCenter
            y: 42
            spacing: 26
            Knob { key: "sub"; label: "Sub"; size: 44; tip: "A square wave below the note, for weight" }
            Knob { key: "sub_oct"; label: "Sub oct"; size: 44; dimmed: win.v("sub") < 0.5; tip: "One or two octaves below the note" }
            Knob { key: "noise"; label: "Noise"; size: 44; tip: "White noise: breath, air, or the hiss of a snare" }
            Knob { key: "drive"; label: "Drive"; size: 44; tip: "Saturation before the filter: warmth to grit" }
        }
    }

    // FILTER
    Card {
        x: win.colB; y: win.top0; width: win.wB; height: 262
        title: "FILTER"
        Chips { anchors.right: parent.right; anchors.rightMargin: 8; y: 5; key: "filter_type"; labels: ["LOW", "HIGH", "BAND"] }
        FilterDisplay { x: 10; y: 32; width: parent.width - 20; height: 150 }
        Row {
            anchors.horizontalCenter: parent.horizontalCenter
            y: 192
            spacing: 22
            Knob {
                key: "cutoff"; label: "Cutoff"
                tip: "Where the filter starts cutting: lower is darker"
                // The LFO's sweep, when it moves the filter.
                modDepth: win.choice("lfo_target") === 1 ? win.v("lfo_depth") / 100 * 3 / Math.log2(18000 / 40) : 0
            }
            Knob { key: "resonance"; label: "Reso"; tip: "Boost at the cutoff: from smooth to whistling" }
            Knob { key: "env_amount"; label: "Env amt"; tip: "How far the filter envelope opens the cutoff on each note" }
            Knob { key: "keytrack"; label: "Key trk"; tip: "Higher notes open the filter more, so they stay as bright as low ones" }
        }
    }
    // ENVELOPE
    Card {
        id: envCard
        x: win.colB; y: win.top0 + 270; width: win.wB; height: 274
        title: "ENVELOPE"
        property string prefix: ""
        Row {
            anchors.right: parent.right; anchors.rightMargin: 8
            y: 5
            spacing: 2
            Rectangle {
                width: envTabs.implicitWidth + 4; height: envTabs.implicitHeight + 4; radius: 6
                color: "#0B0B0D"
                Row {
                    id: envTabs
                    x: 2; y: 2
                    spacing: 2
                    ChipItem { label: "AMP"; on: envCard.prefix === ""; onClicked: envCard.prefix = "" }
                    ChipItem { label: "FILTER"; on: envCard.prefix === "f_"; onClicked: envCard.prefix = "f_" }
                }
            }
        }
        EnvDisplay { x: 10; y: 32; width: parent.width - 20; height: 150; prefix: envCard.prefix }
        Column {
            x: 14; y: 192
            spacing: 4
            EnvSlider { key: envCard.prefix + "attack"; label: "Attack" }
            EnvSlider { key: envCard.prefix + "decay"; label: "Decay" }
            EnvSlider { key: envCard.prefix + "sustain"; label: "Sustain" }
            EnvSlider { key: envCard.prefix + "release"; label: "Release" }
        }
    }

    // LFO
    Card {
        x: win.colC; y: win.top0; width: win.wC; height: 262
        title: "LFO"
        dot: win.mod
        Chips { anchors.right: parent.right; anchors.rightMargin: 8; y: 5; key: "lfo_shape"; labels: ["SIN", "TRI", "SQR", "SAW", "RND"]; tips: ["Sine: smooth", "Triangle: even ramps", "Square: jumps between two values", "Saw: ramps down, jumps back", "Random: a new value each cycle (sample & hold)"] }
        LfoDisplay { x: 10; y: 32; width: parent.width - 20; height: 110 }
        Row {
            x: 12; y: 150
            spacing: 6
            DText { anchors.verticalCenter: parent.verticalCenter; text: "To"; font.pixelSize: 10; color: win.faint; rightPadding: 2 }
            Repeater {
                model: ["PITCH", "FILTER", "VOLUME", "PAN"]
                delegate: TargetChip {
                    required property string modelData
                    required property int index
                    label: modelData
                    on: win.choice("lfo_target") === index
                    onClicked: win.set("lfo_target", index, true)
                }
            }
        }
        Row {
            anchors.horizontalCenter: parent.horizontalCenter
            y: 186
            spacing: 34
            Knob { key: "lfo_rate"; label: "Rate"; color: win.mod; tip: "How fast the LFO moves, in cycles per second" }
            Knob { key: "lfo_depth"; label: "Depth"; color: win.mod; tip: "How much it moves its target. 0 turns the LFO off" }
            Knob { key: "lfo_delay"; label: "Delay"; color: win.mod; dimmed: win.v("lfo_depth") < 0.5; tip: "Fades the LFO in after a note starts: classic delayed vibrato" }
        }
    }
    // UNISON
    Card {
        x: win.colC; y: win.top0 + 270; width: win.wC; height: 134
        title: "UNISON"
        DText {
            anchors.right: parent.right; anchors.rightMargin: 12
            y: 8
            text: (win.choice("unison") + 1) + (win.choice("unison") === 0 ? " voice" : " voices")
            font.pixelSize: 11
            font.weight: Font.DemiBold
            color: "#9A9EA7"
        }
        UnisonDisplay { x: 10; y: 32; width: 130; height: 92 }
        Row {
            x: 154; y: 40
            spacing: 14
            Knob { key: "unison"; label: "Voices"; tip: "Copies of every note, stacked: 7 makes a supersaw" }
            Knob { key: "uni_detune"; label: "Detune"; dimmed: win.choice("unison") === 0; tip: "How far apart the copies are tuned: thicker, then wobblier" }
            Knob { key: "spread"; label: "Spread"; dimmed: win.choice("unison") === 0; tip: "How wide the copies spread from left to right" }
        }
    }
    // VOICE
    Card {
        x: win.colC; y: win.top0 + 412; width: win.wC; height: 132
        title: "VOICE"
        Chips { anchors.right: parent.right; anchors.rightMargin: 8; y: 5; key: "mono"; labels: ["POLY", "MONO", "LEGATO"]; tips: ["Many notes at once", "One note at a time (basses, leads)", "One note; slides between held notes without restarting"] }
        Row {
            x: 18; y: 40
            spacing: 22
            Knob { key: "glide"; label: "Glide"; tip: "Slides the pitch from the last note to the next" }
            Knob { key: "velocity"; label: "Velocity"; tip: "How much playing harder makes it louder and brighter" }
            Knob { key: "volume"; label: "Volume"; bipolar: true; tip: "The synth's output level" }
        }
        // Output level.
        Row {
            anchors.right: parent.right; anchors.rightMargin: 18
            y: 44
            spacing: 3
            Repeater {
                model: 2
                delegate: Rectangle {
                    id: meter
                    required property int index
                    width: 5; height: 66; radius: 2
                    color: "#1D1F23"
                    Rectangle {
                        anchors.bottom: parent.bottom
                        width: parent.width; radius: 2
                        height: parent.height * Math.max(0, Math.min(1, win.level[meter.index] || 0))
                        color: (win.level[meter.index] || 0) > 0.95 ? "#E0685A" : "#B9BCC3"
                    }
                }
            }
        }
        DText { anchors.right: parent.right; anchors.rightMargin: 13; y: 114; text: "OUT"; font.pixelSize: 8; font.letterSpacing: 1; color: win.faint }
    }

    // ------------------------------------------------ keyboard
    Rectangle {
        id: bed
        x: 10; y: 608
        width: parent.width - 20
        height: parent.height - y - 10
        radius: 8
        color: "#0C0D10"
        border.color: win.cardLine
        Column {
            id: side
            x: 10
            anchors.verticalCenter: parent.verticalCenter
            spacing: 8
            TextButton { width: 40; label: "+"; tip: "Octave up (X)"; onClicked: win.octave = Math.min(7, win.octave + 1) }
            DText { anchors.horizontalCenter: parent.horizontalCenter; text: "C" + win.octave; font.pixelSize: 10; font.weight: Font.Bold; color: win.dim }
            TextButton { width: 40; label: "−"; tip: "Octave down (Z)"; onClicked: win.octave = Math.max(0, win.octave - 1) }
            TextButton {
                width: 40; label: "PED"; small: true
                active: win.pedal
                tip: "Sustain pedal (hold Tab)"
                onClicked: win.setPedal(!win.pedal)
            }
        }
        Item {
            id: keys
            x: side.x + side.width + 10
            y: 10
            width: parent.width - x - 10
            height: parent.height - 20
            readonly property int whiteCount: win.octaves * 7 + 1
            readonly property real whiteW: width / whiteCount
            Repeater {
                model: keys.whiteCount
                delegate: SynthKey {
                    required property int index
                    key: win.firstKey + Math.floor(index / 7) * 12 + [0, 2, 4, 5, 7, 9, 11][index % 7]
                    black: false
                    x: index * keys.whiteW
                    width: keys.whiteW - 1.5
                    height: keys.height
                }
            }
            Repeater {
                model: win.octaves * 5
                delegate: SynthKey {
                    required property int index
                    key: win.firstKey + Math.floor(index / 5) * 12 + [1, 3, 6, 8, 10][index % 5]
                    black: true
                    x: (Math.floor(index / 5) * 7 + [0, 1, 3, 4, 5][index % 5] + 1) * keys.whiteW - width / 2 - 0.75
                    width: keys.whiteW * 0.6
                    height: keys.height * 0.6
                    z: 2
                }
            }
        }
    }

    // ================================================================ parts
    component Card: Rectangle {
        id: card
        property string title
        property color dot: win.accent
        radius: 8
        color: win.cardBg
        border.color: win.cardLine
        Rectangle { x: 12; y: 12; width: 6; height: 6; radius: 3; color: card.dot; opacity: 0.9 }
        DText {
            x: 26; y: 7
            text: card.title
            font.pixelSize: 11
            font.weight: Font.Bold
            font.letterSpacing: 0.4
            color: win.text
        }
    }

    component SmallButton: AbstractButton {
        id: sb
        property string glyph
        property bool flip: false
        property string tip
        width: 22; height: 22
        hoverEnabled: true
        background: Rectangle { radius: 5; color: sb.pressed ? "#26282D" : sb.hovered ? "#202125" : "#17181C" }
        contentItem: Item {
            Icon { anchors.centerIn: parent; glyph: sb.glyph; size: 9; color: sb.hovered ? win.text : win.dim; rotation: sb.flip ? 180 : 0 }
        }
        DToolTip { visible: sb.hovered && sb.tip !== ""; text: sb.tip }
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
        background: Rectangle {
            radius: 6
            color: tb.active ? "#1D1E22" : tb.pressed ? "#222327" : tb.hovered ? "#1B1C20" : "#17181C"
            border.color: tb.active ? win.accent : "#24262B"
        }
        contentItem: Item {
            DText {
                id: tbText
                anchors.centerIn: parent
                text: tb.label
                font.pixelSize: tb.small ? 9 : 11
                font.weight: Font.DemiBold
                font.letterSpacing: tb.small ? 1 : 0
                color: tb.active ? win.text : tb.hovered ? win.text : "#A5A9B1"
            }
        }
        DToolTip { visible: tb.hovered && tb.tip !== ""; text: tb.tip }
    }

    component ChipItem: AbstractButton {
        id: ci
        property string label
        property bool on: false
        property string tip
        implicitWidth: ciText.implicitWidth + 14
        implicitHeight: 20
        hoverEnabled: true
        background: Rectangle { radius: 5; color: ci.on ? "#26282D" : ci.hovered ? "#16171A" : "transparent" }
        contentItem: Item {
            DText {
                id: ciText
                anchors.centerIn: parent
                text: ci.label
                font.pixelSize: 10
                font.weight: Font.Bold
                font.letterSpacing: 0.4
                color: ci.on ? "#ECEEF1" : ci.hovered ? "#A5A9B1" : "#6A6E77"
            }
        }
        DToolTip { visible: ci.hovered && ci.tip !== ""; text: ci.tip }
    }

    // A choice between a few options, as a segmented strip.
    component Chips: Rectangle {
        id: chips
        property string key
        property var labels: []
        property var tips: []
        width: chipRow.implicitWidth + 4
        height: 24
        radius: 6
        color: "#0B0B0D"
        Row {
            id: chipRow
            x: 2; y: 2
            spacing: 2
            Repeater {
                model: chips.labels
                delegate: ChipItem {
                    required property string modelData
                    required property int index
                    label: modelData
                    tip: chips.tips[index] || ""
                    on: win.choice(chips.key) === index
                    onClicked: win.set(chips.key, index, true)
                }
            }
        }
    }

    component TargetChip: AbstractButton {
        id: tc
        property string label
        property bool on: false
        implicitWidth: tcText.implicitWidth + 16
        implicitHeight: 22
        hoverEnabled: true
        background: Rectangle {
            radius: 5
            color: tc.on ? "#26282D" : tc.hovered ? "#1D1E22" : "#1A1B1F"
            Rectangle { visible: tc.on; anchors.bottom: parent.bottom; anchors.horizontalCenter: parent.horizontalCenter; width: parent.width - 8; height: 2; radius: 1; color: win.mod }
        }
        contentItem: Item {
            DText { id: tcText; anchors.centerIn: parent; text: tc.label; font.pixelSize: 10; font.weight: Font.Bold; color: tc.on ? "#ECEEF1" : "#80848D" }
        }
    }

    // A knob: thin arc, label and live value underneath. Drag up/down
    // (⇧ fine), scroll, double-click for the default. Hover for what it does.
    component Knob: Item {
        id: knob
        property string key
        property string label
        property string tip
        property color color: win.accent
        property real size: 40
        /// Fills from the centre (for values either side of zero).
        property bool bipolar: false
        /// Whole numbers only (semitones).
        property bool whole: false
        property bool dimmed: false
        /// Text shown instead of the value.
        property string display: ""
        /// Modulation range either side of the value, in knob travel (0..1).
        property real modDepth: 0
        readonly property var def: win.param(key)
        readonly property real value: win.v(key)
        readonly property real pos: win.posOf(key, value)
        readonly property real zero: bipolar ? win.posOf(key, 0) : 0
        readonly property bool hot: area.containsMouse || area.pressed
        width: Math.max(size, 52)
        height: size + 32
        opacity: dimmed ? 0.4 : 1

        Item {
            id: dial
            width: knob.size; height: knob.size
            anchors.horizontalCenter: parent.horizontalCenter
            readonly property real r: knob.size / 2 - 3
            readonly property real c: knob.size / 2
            Shape {
                anchors.fill: parent
                preferredRendererType: Shape.CurveRenderer
                ShapePath {
                    strokeColor: win.trackLine
                    strokeWidth: 2.5
                    fillColor: "transparent"
                    capStyle: ShapePath.RoundCap
                    PathAngleArc { centerX: dial.c; centerY: dial.c; radiusX: dial.r; radiusY: dial.r; startAngle: 135; sweepAngle: 270 }
                }
                ShapePath {
                    strokeColor: knob.modDepth > 0 ? win.mod : "transparent"
                    strokeWidth: 2.5
                    fillColor: "transparent"
                    capStyle: ShapePath.RoundCap
                    PathAngleArc {
                        centerX: dial.c; centerY: dial.c; radiusX: dial.r; radiusY: dial.r
                        startAngle: 135 + 270 * Math.max(0, knob.pos - knob.modDepth)
                        sweepAngle: Math.max(0.01, 270 * (Math.min(1, knob.pos + knob.modDepth) - Math.max(0, knob.pos - knob.modDepth)))
                    }
                }
                ShapePath {
                    strokeColor: knob.color
                    strokeWidth: 2.5
                    fillColor: "transparent"
                    capStyle: ShapePath.RoundCap
                    PathAngleArc {
                        centerX: dial.c; centerY: dial.c; radiusX: dial.r; radiusY: dial.r
                        startAngle: 135 + 270 * Math.min(knob.zero, knob.pos)
                        sweepAngle: Math.max(0.01, 270 * Math.abs(knob.pos - knob.zero))
                    }
                }
            }
            Rectangle {
                anchors.centerIn: parent
                width: knob.size - 14; height: width; radius: width / 2
                color: knob.hot ? "#1E2024" : "#18191D"
                border.color: "#2A2C31"
                Item {
                    anchors.fill: parent
                    rotation: -135 + 270 * knob.pos
                    Rectangle {
                        anchors.horizontalCenter: parent.horizontalCenter
                        y: 3
                        width: 2; height: parent.height * 0.32; radius: 1
                        color: win.text
                    }
                }
            }
        }
        DText {
            anchors.horizontalCenter: parent.horizontalCenter
            y: dial.height + 3
            text: knob.label
            font.pixelSize: 10
            font.weight: Font.DemiBold
            color: knob.hot ? win.text : win.dim
        }
        DText {
            anchors.horizontalCenter: parent.horizontalCenter
            y: dial.height + 16
            text: knob.display !== "" ? knob.display : win.fmt(knob.key, knob.value)
            font.pixelSize: 10
            font.features: { "tnum": 1 }
            color: win.text
        }
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
                win.set(knob.key, win.valueOf(knob.key, startPos + (startY - m.y) / 180 * fine, knob.whole), false)
            }
            onReleased: { win.set(knob.key, knob.value, true); typingKeys.forceActiveFocus() }
            onDoubleClicked: if (knob.def) win.set(knob.key, knob.def.default, true)
            onWheel: w => {
                if (!knob.def) return
                const steps = w.angleDelta.y !== 0 ? w.angleDelta.y / 120 : w.pixelDelta.y / 30
                const n = knob.def.choices.length
                const step = n ? steps / Math.max(1, n - 1) : steps * 0.02
                win.set(knob.key, win.valueOf(knob.key, knob.pos + step, knob.whole), true)
            }
        }
        DToolTip { visible: area.containsMouse && !area.pressed && knob.tip !== ""; text: knob.tip; delay: 600 }
    }

    // One envelope stage as a horizontal slider.
    component EnvSlider: Item {
        id: es
        property string key
        property string label
        readonly property real pos: win.posOf(key, win.v(key))
        width: 334; height: 15
        DText { anchors.verticalCenter: parent.verticalCenter; text: es.label; font.pixelSize: 10; font.weight: Font.DemiBold; color: esArea.containsMouse || esArea.pressed ? win.text : win.dim }
        Rectangle {
            id: esTrack
            x: 66; anchors.verticalCenter: parent.verticalCenter
            width: 200; height: 3; radius: 1.5
            color: win.trackLine
            Rectangle { width: parent.width * es.pos; height: parent.height; radius: 1.5; color: win.accent }
            Rectangle {
                x: parent.width * es.pos - width / 2
                anchors.verticalCenter: parent.verticalCenter
                width: esArea.pressed ? 13 : 11; height: width; radius: width / 2
                color: win.text
                border.color: "#0D0E10"
            }
        }
        DText {
            x: 276; anchors.verticalCenter: parent.verticalCenter
            text: win.fmt(es.key, win.v(es.key))
            font.pixelSize: 10
            color: win.text
        }
        MouseArea {
            id: esArea
            x: esTrack.x - 6; width: esTrack.width + 12; height: parent.height
            hoverEnabled: true
            preventStealing: true
            cursorShape: Qt.SizeHorCursor
            function to(mx) { return win.valueOf(es.key, (mx - 6) / esTrack.width, false) }
            onPressed: m => win.set(es.key, to(m.x), false)
            onPositionChanged: m => { if (pressed) win.set(es.key, to(m.x), false) }
            onReleased: { win.set(es.key, win.v(es.key), true); typingKeys.forceActiveFocus() }
            onDoubleClicked: { const d = win.param(es.key); if (d) win.set(es.key, d.default, true) }
        }
    }

    // Two cycles of a wave, as a filled outline.
    component WaveDisplay: Rectangle {
        id: wd
        property int wave: 0
        property real pulse: 50
        property real level: 1
        property string tag
        radius: 6
        color: win.well
        border.color: win.wellLine
        onWaveChanged: wdCanvas.requestPaint()
        onPulseChanged: wdCanvas.requestPaint()
        onLevelChanged: wdCanvas.requestPaint()
        Canvas {
            id: wdCanvas
            anchors.fill: parent
            anchors.margins: 1
            onWidthChanged: requestPaint()
            onPaint: {
                const ctx = getContext("2d")
                ctx.reset()
                const w = width, h = height, mid = h / 2, amp = (h / 2 - 12) * wd.level
                const x0 = 12, x1 = w - 12
                ctx.strokeStyle = win.css(win.grid, 1)
                ctx.lineWidth = 1
                ctx.beginPath(); ctx.moveTo(x0, mid); ctx.lineTo(x1, mid); ctx.stroke()
                const pw = wd.pulse / 100
                let seed = 11
                const y = t => {
                    switch (wd.wave) {
                    case 0: return 1 - 2 * t
                    case 1: return t < 0.5 ? 0.85 : -0.85
                    case 2: return 1 - 4 * Math.abs(t - 0.5)
                    case 3: return Math.sin(t * Math.PI * 2)
                    case 4: return t < pw ? 0.85 : -0.85
                    default: seed = (seed * 9301 + 49297) % 233280; return (seed / 233280 * 2 - 1) * 0.8
                    }
                }
                const n = wd.wave === 5 ? 140 : 320
                const pts = []
                for (let i = 0; i <= n; ++i) {
                    const t = (i / n * 2) % 1
                    pts.push([x0 + i / n * (x1 - x0), mid - y(i === n ? 0.9999 : t) * amp])
                }
                ctx.beginPath()
                ctx.moveTo(pts[0][0], mid)
                for (const p of pts) ctx.lineTo(p[0], p[1])
                ctx.lineTo(x1, mid)
                ctx.closePath()
                const g = ctx.createLinearGradient(0, mid - amp, 0, mid + amp)
                g.addColorStop(0, win.css(win.accent, 0.16))
                g.addColorStop(0.5, win.css(win.accent, 0.01))
                g.addColorStop(1, win.css(win.accent, 0.16))
                ctx.fillStyle = g
                ctx.fill()
                ctx.beginPath()
                pts.forEach((p, i) => i ? ctx.lineTo(p[0], p[1]) : ctx.moveTo(p[0], p[1]))
                ctx.lineJoin = "round"
                ctx.strokeStyle = win.css(win.accent, 0.95)
                ctx.lineWidth = 1.6
                ctx.stroke()
            }
        }
        DText { anchors.right: parent.right; anchors.rightMargin: 8; y: 6; text: wd.tag; font.pixelSize: 10; font.weight: Font.DemiBold; color: win.faint }
    }

    // The filter's response; drag it: left/right cutoff, up/down resonance.
    component FilterDisplay: Rectangle {
        id: fd
        radius: 6
        color: win.well
        border.color: win.wellLine
        readonly property real fc: win.v("cutoff")
        readonly property real res: win.v("resonance")
        readonly property int type: win.choice("filter_type")
        readonly property real sweep: win.choice("lfo_target") === 1 ? win.v("lfo_depth") / 100 * 3 : 0
        onFcChanged: fdCanvas.requestPaint()
        onResChanged: fdCanvas.requestPaint()
        onTypeChanged: fdCanvas.requestPaint()
        onSweepChanged: fdCanvas.requestPaint()
        function xOf(f) { return 8 + Math.log(Math.max(20, f) / 20) / Math.log(1000) * (width - 16) }
        Canvas {
            id: fdCanvas
            anchors.fill: parent
            onWidthChanged: requestPaint()
            onPaint: {
                const ctx = getContext("2d")
                ctx.reset()
                const w = width, h = height
                ctx.strokeStyle = win.css(win.grid, 1)
                ctx.lineWidth = 1
                for (const f of [100, 1000, 10000]) {
                    const x = fd.xOf(f)
                    ctx.beginPath(); ctx.moveTo(x, 8); ctx.lineTo(x, h - 20); ctx.stroke()
                }
                const q = 0.5 + Math.max(0, Math.min(1, fd.res / 100)) * 9.5
                const db = f => {
                    const r = f / fd.fc
                    const den = Math.sqrt(Math.pow(1 - r * r, 2) + Math.pow(r / q, 2))
                    const mag = fd.type === 1 ? r * r / den : fd.type === 2 ? (r / q) / den : 1 / den
                    return 20 * Math.log10(Math.max(1e-5, mag))
                }
                const zero = h * 0.42
                const Y = d => zero - Math.max(-36, Math.min(24, d)) / 36 * (h - 22 - zero)
                ctx.beginPath(); ctx.moveTo(8, Y(0)); ctx.lineTo(w - 8, Y(0)); ctx.stroke()
                // The LFO's sweep, as a band around the cutoff.
                if (fd.sweep > 0) {
                    const a = fd.xOf(fd.fc / Math.pow(2, fd.sweep)), b = fd.xOf(fd.fc * Math.pow(2, fd.sweep))
                    ctx.fillStyle = win.css(win.mod, 0.08)
                    ctx.fillRect(a, 8, b - a, h - 28)
                    ctx.strokeStyle = win.css(win.mod, 0.45)
                    ctx.beginPath(); ctx.moveTo(a, 8); ctx.lineTo(a, h - 20); ctx.moveTo(b, 8); ctx.lineTo(b, h - 20); ctx.stroke()
                }
                const pts = []
                for (let i = 0; i <= 160; ++i) {
                    const f = 20 * Math.pow(1000, i / 160)
                    pts.push([fd.xOf(f), Y(db(f))])
                }
                ctx.beginPath()
                ctx.moveTo(pts[0][0], h - 20)
                for (const p of pts) ctx.lineTo(p[0], p[1])
                ctx.lineTo(pts[pts.length - 1][0], h - 20)
                ctx.closePath()
                const g = ctx.createLinearGradient(0, 8, 0, h - 20)
                g.addColorStop(0, win.css(win.accent, 0.16))
                g.addColorStop(1, win.css(win.accent, 0.0))
                ctx.fillStyle = g
                ctx.fill()
                ctx.beginPath()
                pts.forEach((p, i) => i ? ctx.lineTo(p[0], p[1]) : ctx.moveTo(p[0], p[1]))
                ctx.lineJoin = "round"
                ctx.strokeStyle = win.css(win.accent, 0.95)
                ctx.lineWidth = 1.6
                ctx.stroke()
                // The cutoff handle.
                const cx = fd.xOf(fd.fc), cy = Y(db(fd.fc))
                ctx.fillStyle = win.css(win.bg, 1)
                ctx.strokeStyle = win.css(win.accent, 1)
                ctx.lineWidth = 1.6
                ctx.beginPath(); ctx.arc(cx, cy, fdArea.pressed || fdArea.containsMouse ? 5.5 : 4.5, 0, Math.PI * 2); ctx.fill(); ctx.stroke()
            }
        }
        Repeater {
            model: [[20, "20"], [100, "100"], [1000, "1k"], [10000, "10k"], [20000, "20k"]]
            delegate: DText {
                required property var modelData
                x: Math.max(6, Math.min(fd.width - implicitWidth - 6, fd.xOf(modelData[0]) - implicitWidth / 2))
                y: fd.height - 16
                text: modelData[1]
                font.pixelSize: 9
                color: win.faint
            }
        }
        DText { x: 10; y: 6; text: win.fmt("cutoff", fd.fc) + "  ·  " + Math.round(fd.res) + "%"; font.pixelSize: 10; font.weight: Font.DemiBold; color: win.faint }
        MouseArea {
            id: fdArea
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.SizeAllCursor
            preventStealing: true
            onContainsMouseChanged: fdCanvas.requestPaint()
            function apply(m) {
                const f = 20 * Math.pow(1000, Math.max(0, Math.min(1, (m.x - 8) / (width - 16))))
                win.set("cutoff", Math.round(Math.max(40, Math.min(18000, f))), false)
                win.set("resonance", Math.round(Math.max(0, Math.min(100, (1 - m.y / height) * 110 - 5))), false)
            }
            onPressed: m => { apply(m); fdCanvas.requestPaint() }
            onPositionChanged: m => { if (pressed) apply(m) }
            onReleased: { win.set("cutoff", win.v("cutoff"), true); win.set("resonance", win.v("resonance"), true); typingKeys.forceActiveFocus(); fdCanvas.requestPaint() }
        }
    }

    // An ADSR envelope; drag its points (attack, decay + sustain, release).
    component EnvDisplay: Rectangle {
        id: ed
        property string prefix
        radius: 6
        color: win.well
        border.color: win.wellLine
        readonly property string ka: prefix + "attack"
        readonly property string kd: prefix + "decay"
        readonly property string ks: prefix + "sustain"
        readonly property string kr: prefix + "release"
        readonly property real pad: 10
        readonly property real topY: 14
        readonly property real bottomY: height - 12
        // Times on a log scale, each stage in its own span; sustain holds a fixed width.
        readonly property real span: (width - 2 * pad - 40) / 3
        function tx(key, ms) { return win.posOf(key, ms) * span }
        function msAt(key, x) { return win.valueOf(key, x / span, false) }
        readonly property real ax: pad + tx(ka, win.v(ka))
        readonly property real dx: ax + tx(kd, win.v(kd))
        readonly property real sy: bottomY - (bottomY - topY) * win.v(ks) / 100
        readonly property real sx: dx + 40
        readonly property real rx: sx + tx(kr, win.v(kr))
        onAxChanged: edCanvas.requestPaint()
        onDxChanged: edCanvas.requestPaint()
        onSyChanged: edCanvas.requestPaint()
        onRxChanged: edCanvas.requestPaint()
        Canvas {
            id: edCanvas
            anchors.fill: parent
            onWidthChanged: requestPaint()
            onPaint: {
                const ctx = getContext("2d")
                ctx.reset()
                const w = width, h = height
                ctx.strokeStyle = win.css(win.grid, 1)
                ctx.lineWidth = 1
                for (let i = 1; i < 8; ++i) { const x = i * w / 8; ctx.beginPath(); ctx.moveTo(x, 6); ctx.lineTo(x, h - 6); ctx.stroke() }
                for (let i = 1; i < 4; ++i) { const y = ed.topY + i * (ed.bottomY - ed.topY) / 4; ctx.beginPath(); ctx.moveTo(6, y); ctx.lineTo(w - 6, y); ctx.stroke() }
                const p0 = [ed.pad, ed.bottomY], p1 = [ed.ax, ed.topY], p2 = [ed.dx, ed.sy], p3 = [ed.sx, ed.sy], p4 = [ed.rx, ed.bottomY]
                const path = () => {
                    ctx.beginPath()
                    ctx.moveTo(p0[0], p0[1])
                    ctx.quadraticCurveTo(p0[0] + (p1[0] - p0[0]) * 0.3, p1[1] + (p0[1] - p1[1]) * 0.1, p1[0], p1[1])
                    ctx.quadraticCurveTo(p1[0] + (p2[0] - p1[0]) * 0.12, p2[1], p2[0], p2[1])
                    ctx.lineTo(p3[0], p3[1])
                    ctx.quadraticCurveTo(p3[0] + (p4[0] - p3[0]) * 0.12, p4[1], p4[0], p4[1])
                }
                path()
                ctx.lineTo(ed.pad, ed.bottomY)
                ctx.closePath()
                const g = ctx.createLinearGradient(0, ed.topY, 0, ed.bottomY)
                g.addColorStop(0, win.css(win.accent, 0.16))
                g.addColorStop(1, win.css(win.accent, 0.0))
                ctx.fillStyle = g
                ctx.fill()
                path()
                ctx.lineJoin = "round"
                ctx.strokeStyle = win.css(win.accent, 0.95)
                ctx.lineWidth = 1.6
                ctx.stroke()
            }
        }
        DText { anchors.right: parent.right; anchors.rightMargin: 8; y: 6; text: ed.prefix === "" ? "VOLUME" : "FILTER CUTOFF"; font.pixelSize: 9; font.weight: Font.Bold; font.letterSpacing: 1; color: win.faint }
        EnvHandle { graph: ed; cx: ed.ax; cy: ed.topY; tip: "Attack: how long it takes to reach full"; onDragged: (x, y) => win.set(ed.ka, ed.msAt(ed.ka, x - ed.pad), false); onDone: win.set(ed.ka, win.v(ed.ka), true) }
        EnvHandle {
            graph: ed
            cx: ed.dx; cy: ed.sy; tip: "Decay (left/right) and sustain level (up/down)"
            onDragged: (x, y) => {
                win.set(ed.kd, ed.msAt(ed.kd, x - ed.ax), false)
                win.set(ed.ks, Math.round(Math.max(0, Math.min(100, (ed.bottomY - y) / (ed.bottomY - ed.topY) * 100))), false)
            }
            onDone: { win.set(ed.kd, win.v(ed.kd), true); win.set(ed.ks, win.v(ed.ks), true) }
        }
        EnvHandle { graph: ed; cx: ed.rx; cy: ed.bottomY; tip: "Release: how long it fades after you let go"; onDragged: (x, y) => win.set(ed.kr, ed.msAt(ed.kr, x - ed.sx), false); onDone: win.set(ed.kr, win.v(ed.kr), true) }
    }

    // A draggable point on an envelope.
    component EnvHandle: Item {
        id: h
        property Item graph
        property real cx
        property real cy
        property string tip
        signal dragged(real x, real y)
        signal done()
        x: cx - 9; y: cy - 9
        width: 18; height: 18
        z: 2
        Rectangle {
            anchors.centerIn: parent
            width: hArea.containsMouse || hArea.pressed ? 11 : 8
            height: width
            radius: width / 2
            color: win.text
            border.color: win.bg
            border.width: 1.5
            Behavior on width { NumberAnimation { duration: Theme.fast } }
        }
        MouseArea {
            id: hArea
            anchors.fill: parent
            anchors.margins: -4
            hoverEnabled: true
            preventStealing: true
            cursorShape: Qt.SizeAllCursor
            onPositionChanged: m => {
                if (!pressed) return
                const p = mapToItem(h.graph, m.x, m.y)
                h.dragged(p.x, p.y)
            }
            onReleased: { h.done(); typingKeys.forceActiveFocus() }
        }
        DToolTip { visible: hArea.containsMouse && !hArea.pressed; text: h.tip }
    }

    // The LFO's shape over a few cycles, fading in by its delay, with a dot
    // running along at its rate.
    component LfoDisplay: Rectangle {
        id: ld
        radius: 6
        color: win.well
        border.color: win.wellLine
        readonly property real rate: win.v("lfo_rate")
        readonly property real depth: win.v("lfo_depth") / 100
        readonly property int shape: win.choice("lfo_shape")
        readonly property real delay: win.v("lfo_delay")
        /// Cycles shown, and the seconds they span.
        readonly property real cycles: Math.min(8, Math.max(2, Math.round(rate * 2)))
        readonly property real seconds: cycles / Math.max(0.05, rate)
        property real phase: 0
        onDepthChanged: ldCanvas.requestPaint()
        onShapeChanged: ldCanvas.requestPaint()
        onDelayChanged: ldCanvas.requestPaint()
        onCyclesChanged: ldCanvas.requestPaint()
        function valueAt(t) {
            const c = t - Math.floor(t)
            switch (shape) {
            case 1: return 1 - 4 * Math.abs(c - 0.5)
            case 2: return c < 0.5 ? 1 : -1
            case 3: return 1 - 2 * c
            case 4: { const n = Math.floor(t); const s = Math.sin(n * 12.9898 + 4.1) * 43758.5453; return (s - Math.floor(s)) * 2 - 1 }
            default: return Math.sin(c * Math.PI * 2)
            }
        }
        Timer {
            running: win.shown && ld.depth > 0
            interval: 40
            repeat: true
            onTriggered: { ld.phase = (ld.phase + 0.04 / ld.seconds) % 1; ldCanvas.requestPaint() }
        }
        Canvas {
            id: ldCanvas
            anchors.fill: parent
            onWidthChanged: requestPaint()
            onPaint: {
                const ctx = getContext("2d")
                ctx.reset()
                const w = width, h = height, x0 = 10, x1 = w - 10, mid = h / 2, amp = (h / 2 - 14)
                ctx.strokeStyle = win.css(win.grid, 1)
                ctx.lineWidth = 1
                for (let i = 1; i < 8; ++i) { const x = x0 + i * (x1 - x0) / 8; ctx.beginPath(); ctx.moveTo(x, 6); ctx.lineTo(x, h - 6); ctx.stroke() }
                ctx.beginPath(); ctx.moveTo(x0, mid); ctx.lineTo(x1, mid); ctx.stroke()
                const fadeAt = t => ld.delay > 0.5 ? Math.min(1, t * ld.seconds * 1000 / ld.delay) : 1
                const n = 240
                const pts = []
                for (let i = 0; i <= n; ++i) {
                    const t = i / n
                    const yv = ld.valueAt(t * ld.cycles + (ld.shape === 4 ? 0 : 0)) * Math.max(0.04, ld.depth) * fadeAt(t)
                    pts.push([x0 + t * (x1 - x0), mid - yv * amp])
                }
                ctx.beginPath()
                ctx.moveTo(x0, mid)
                for (const p of pts) ctx.lineTo(p[0], p[1])
                ctx.lineTo(x1, mid)
                ctx.closePath()
                ctx.fillStyle = win.css(win.mod, 0.08)
                ctx.fill()
                ctx.beginPath()
                pts.forEach((p, i) => i ? ctx.lineTo(p[0], p[1]) : ctx.moveTo(p[0], p[1]))
                ctx.lineJoin = "round"
                ctx.strokeStyle = win.css(win.mod, ld.depth > 0 ? 0.95 : 0.3)
                ctx.lineWidth = 1.6
                ctx.stroke()
                // Where the LFO is now.
                if (ld.depth > 0) {
                    const t = ld.phase
                    const px = x0 + t * (x1 - x0)
                    ctx.strokeStyle = win.css(win.mod, 0.35)
                    ctx.beginPath(); ctx.moveTo(px, 6); ctx.lineTo(px, h - 6); ctx.stroke()
                    ctx.fillStyle = win.css(win.mod, 1)
                    ctx.beginPath(); ctx.arc(px, mid - ld.valueAt(t * ld.cycles) * ld.depth * fadeAt(t) * amp, 3, 0, Math.PI * 2); ctx.fill()
                }
            }
        }
        DText {
            x: 10; y: 6
            text: ld.depth > 0 ? win.fmt("lfo_rate", ld.rate) + (ld.delay > 0.5 ? "  ·  fades in over " + win.fmt("lfo_delay", ld.delay) : "") : "Off: turn up Depth"
            font.pixelSize: 10
            font.weight: Font.DemiBold
            color: win.faint
        }
    }

    // Unison copies: pitch offsets (bars) and where each sits left to right (dots).
    component UnisonDisplay: Rectangle {
        id: ud
        radius: 6
        color: win.well
        border.color: win.wellLine
        readonly property int voices: win.choice("unison") + 1
        readonly property real detune: win.v("uni_detune") / 100
        readonly property real spread: win.v("spread") / 100
        Rectangle { x: parent.width / 2; y: 8; width: 1; height: parent.height - 16; color: win.grid }
        DText { x: 6; y: parent.height - 15; text: "L"; font.pixelSize: 8; color: win.faint }
        DText { anchors.right: parent.right; anchors.rightMargin: 6; y: parent.height - 15; text: "R"; font.pixelSize: 8; color: win.faint }
        Repeater {
            model: ud.voices
            delegate: Item {
                id: uv
                required property int index
                readonly property real t: ud.voices === 1 ? 0 : index / (ud.voices - 1) * 2 - 1
                anchors.fill: parent
                // Pitch: bars spread by detune.
                Rectangle {
                    x: ud.width / 2 + uv.t * ud.detune * (ud.width / 2 - 14) - 1.5
                    y: 12
                    width: 3; height: ud.height - 44; radius: 1.5
                    color: win.accent
                    opacity: 1 - Math.abs(uv.t) * 0.5
                    Behavior on x { NumberAnimation { duration: Theme.fast } }
                }
                // Pan: dots spread by spread.
                Rectangle {
                    x: ud.width / 2 + uv.t * ud.spread * (ud.width / 2 - 20) - 3
                    y: ud.height - 24
                    width: 6; height: 6; radius: 3
                    color: win.dim
                    Behavior on x { NumberAnimation { duration: Theme.fast } }
                }
            }
        }
    }

    // One key: ivory or ebony, blue when it plays (brighter the harder).
    component SynthKey: Item {
        id: k
        property int key
        property bool black
        readonly property bool held: win.lit(key)
        readonly property real vel: held ? win.velocityOf(key) / 127 : 0
        readonly property int offset: key - win.firstKey
        Rectangle {
            anchors.fill: parent
            radius: k.black ? 2.5 : 3
            gradient: Gradient {
                GradientStop { position: 0; color: k.black ? "#111216" : "#CFD1D6" }
                GradientStop { position: 1; color: k.black ? "#26282E" : "#ECEDF0" }
            }
            border.color: k.black ? "#000000" : "#9C9FA6"
            border.width: k.black ? 1 : 0.5
            Rectangle {
                anchors.fill: parent
                radius: parent.radius
                gradient: Gradient {
                    GradientStop { position: 0; color: win.rgba(win.keyLit, 0.55) }
                    GradientStop { position: 1; color: win.rgba(win.keyLit, 1) }
                }
                opacity: k.held ? 0.55 + 0.45 * k.vel : 0
                Behavior on opacity { NumberAnimation { duration: k.held ? 25 : Theme.slow; easing.type: Easing.OutCubic } }
            }
        }
        DText {
            visible: win.typing && k.offset >= 0 && k.offset < win.keyLetters.length
            anchors.horizontalCenter: parent.horizontalCenter
            y: k.black ? parent.height - 18 : parent.height - 32
            text: visible ? win.keyLetters[k.offset] : ""
            font.pixelSize: 9
            font.weight: Font.DemiBold
            color: k.held ? "#FFFFFF" : k.black ? "#80848D" : "#6A6E77"
        }
        DText {
            visible: !k.black && k.key % 12 === 0
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.bottom: parent.bottom; anchors.bottomMargin: 6
            text: "C" + (Math.floor(k.key / 12) - 1)
            font.pixelSize: 8
            color: k.held ? "#FFFFFF" : "#80848D"
        }
        MouseArea {
            anchors.fill: parent
            onPressed: m => { typingKeys.forceActiveFocus(); win.press(k.key, Math.round(40 + 87 * Math.max(0, Math.min(1, m.y / height)))) }
            onReleased: win.lift(k.key)
            onCanceled: win.lift(k.key)
        }
    }

    // Built-in presets, the user's own, and saving the current sound.
    component SynthPresetPopup: Popup {
        id: pop
        padding: 16
        modal: false
        focus: true
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
        onClosed: { saving = false; typingKeys.forceActiveFocus() }
        property bool saving: false
        function startSaving() { saving = true; synthName.text = win.presetName && win.presetGroup === "Mine" ? win.presetName : ""; synthName.forceActiveFocus() }
        background: Rectangle { radius: 8; color: "#141518"; border.color: "#2A2C31" }
        enter: Transition { NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.fast } }
        exit: Transition { NumberAnimation { property: "opacity"; to: 0; duration: Theme.fast } }
        contentItem: Row {
            spacing: 14
            Repeater {
                model: win.presets
                delegate: Column {
                    id: gcol
                    required property var modelData
                    spacing: 1
                    DText { text: gcol.modelData.group.toUpperCase(); font.pixelSize: 9; font.weight: Font.Bold; font.letterSpacing: 1.2; color: win.faint; bottomPadding: 6 }
                    Repeater {
                        model: gcol.modelData.items
                        delegate: SynthPresetRow {
                            required property var modelData
                            name: modelData.name
                            values: modelData.v
                            onChosen: pop.close()
                        }
                    }
                }
            }
            Rectangle { width: 1; height: parent.height; color: "#24262B" }
            Column {
                spacing: 1
                DText { text: "MY PRESETS"; font.pixelSize: 9; font.weight: Font.Bold; font.letterSpacing: 1.2; color: win.faint; bottomPadding: 6 }
                Repeater {
                    model: win.userPresets
                    delegate: SynthPresetRow {
                        required property var modelData
                        name: modelData.name
                        values: modelData.values
                        deletable: true
                        onChosen: pop.close()
                    }
                }
                DText {
                    visible: win.userPresets.length === 0 && !pop.saving
                    width: 140
                    text: "Make a sound you like, then save it here."
                    wrapMode: Text.WordWrap
                    font.pixelSize: 11
                    color: win.faint
                    bottomPadding: 6
                }
                Rectangle {
                    visible: pop.saving
                    width: 140; height: 28
                    radius: 6
                    color: win.well
                    border.color: "#3A3D44"
                    TextInput {
                        id: synthName
                        anchors.fill: parent
                        anchors.margins: 8
                        verticalAlignment: TextInput.AlignVCenter
                        color: win.text
                        font.pixelSize: 12
                        font.family: Theme.font
                        clip: true
                        maximumLength: 40
                        onAccepted: {
                            if (text.trim() === "") return
                            win.app.saveUserPreset("synth", text.trim(), JSON.stringify(win.snapshot()))
                            win.presetName = text.trim()
                            win.presetValues = win.snapshot()
                            win.reloadUserPresets()
                            pop.saving = false
                        }
                        Keys.onEscapePressed: pop.saving = false
                    }
                    DText {
                        visible: synthName.text === ""
                        anchors.left: parent.left; anchors.leftMargin: 8
                        anchors.verticalCenter: parent.verticalCenter
                        text: "Name, then Return"
                        font.pixelSize: 11
                        color: win.faint
                    }
                }
                TextButton {
                    width: 140
                    visible: !pop.saving
                    label: "+  Save current sound"
                    onClicked: pop.startSaving()
                }
            }
        }
    }

    component SynthPresetRow: Item {
        id: row
        property string name
        property var values
        property bool deletable: false
        signal chosen()
        readonly property bool current: win.presetName === name
        width: 140; height: 26
        Rectangle {
            anchors.fill: parent
            radius: 5
            color: row.current ? "#26282D" : rowArea.containsMouse ? "#1B1C20" : "transparent"
            Rectangle { visible: row.current; x: 0; y: 6; width: 2; height: parent.height - 12; radius: 1; color: win.accent }
        }
        MouseArea {
            id: rowArea
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: { win.loadPreset(row.name, row.values); row.chosen() }
        }
        DText {
            anchors.left: parent.left; anchors.leftMargin: 9
            anchors.right: rowButtons.left; anchors.rightMargin: 4
            anchors.verticalCenter: parent.verticalCenter
            text: row.name
            elide: Text.ElideRight
            font.pixelSize: 12
            color: row.current ? win.text : "#B5B8BF"
        }
        Row {
            id: rowButtons
            anchors.right: parent.right; anchors.rightMargin: 3
            anchors.verticalCenter: parent.verticalCenter
            spacing: 2
            opacity: rowArea.containsMouse || hearArea.containsMouse || delArea.containsMouse ? 1 : 0
            Rectangle {
                width: 20; height: 20; radius: 5
                color: hearArea.containsMouse ? "#33363C" : "#202125"
                Icon { anchors.centerIn: parent; glyph: Icons.play; size: 8; color: win.text }
                MouseArea { id: hearArea; anchors.fill: parent; hoverEnabled: true; onClicked: win.preview(row.name, row.values) }
                DToolTip { visible: hearArea.containsMouse; text: "Hear it" }
            }
            Rectangle {
                visible: row.deletable
                width: 20; height: 20; radius: 5
                color: delArea.containsMouse ? "#5A2A2A" : "#202125"
                Icon { anchors.centerIn: parent; glyph: Icons.x; size: 8; color: win.text }
                MouseArea { id: delArea; anchors.fill: parent; hoverEnabled: true; onClicked: { win.app.deleteUserPreset("synth", row.name); win.reloadUserPresets() } }
                DToolTip { visible: delArea.containsMouse; text: "Delete this preset" }
            }
        }
    }
}
