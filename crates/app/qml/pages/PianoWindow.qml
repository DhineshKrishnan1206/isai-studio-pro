import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Shapes
import Isai

// "Isai Piano": the built-in piano's own window, styled like a compact
// hardware keyboard. Drag its top edge to move it.
//   • Tone (◀ ▶ or click the name), PRESETS (built in and your own), A/B to
//     compare two settings, LIGHT for slow computers.
//   • SOUND, CHARACTER, MODULATION, TREMOLO (the switch: volume or auto-pan),
//     ROOM and OUTPUT knobs.
//   • Play with the mouse (click higher on a key for softer) or the computer
//     keyboard while the window is open: A W S E D F T G Y H U J K O L P ;
//     Z / X octave, 1 2 3 soft / medium / hard, Tab sustain pedal, Space
//     play/stop. Chord, Strum and Arp turn one key into a chord, a strum or
//     an arpeggio in the song's key.
//   • Keys light up for every note the piano plays (computer keyboard, MIDI
//     keyboard or the song), brighter when played harder; the notes rise
//     above the keys as they play, and the display names the chord.
Item {
    id: win
    property var app
    /// The piano track (an entry of tracksJson), or null when closed.
    property var track: null
    property Item backdrop: null
    /// Which keys the selected instrument is playing (see StudioPage's keyActivity).
    property var activity: null
    /// This track's output level [left, right], 0..1.
    property var level: [0, 0]
    signal closeRequested()

    readonly property bool shown: !!track
    readonly property var inst: track ? track.instrument : null
    readonly property var def: {
        try { return JSON.parse(app.instrumentKindsJson || "[]").find(k => k.key === "piano") || null } catch (e) { return null }
    }
    function param(key) { return def ? def.params.find(p => p.key === key) || null : null }
    /// Values being dragged right now (published when the drag ends).
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
    /// Every parameter's current value.
    function snapshot() {
        const all = {}
        if (def) for (const d of def.params) all[d.key] = v(d.key)
        return all
    }
    function apply(values, label) {
        live = {}
        app.setInstrumentParams(track.id, JSON.stringify(values), label)
    }

    // Palette from the hardware this is styled after.
    readonly property color body: "#232838"
    readonly property color bodyDark: "#1A1E2B"
    readonly property color outline: "#3B4258"
    readonly property color label: "#9AA3BC"
    readonly property color faint: "#5D6580"
    readonly property color glow: "#E3E7FF"

    width: 1060
    // Fits small screens: the keyboard gives up the height.
    height: parent ? Math.min(700, parent.height - 16) : 700
    x: (parent.width - width) / 2
    y: Math.max(12, (parent.height - height) / 2 - 20)
    visible: opacity > 0
    opacity: shown ? 1 : 0
    scale: shown ? 1 : 0.97
    Behavior on opacity { NumberAnimation { duration: Theme.fast } }
    Behavior on scale { NumberAnimation { duration: Theme.slow; easing.type: Easing.OutCubic } }
    z: 150

    onShownChanged: {
        if (!shown) {
            presetName = ""; presetValues = null; ab = { A: null, B: null }; abSlot = "A"
            releaseAll()
            return
        }
        Qt.callLater(() => typingKeys.forceActiveFocus())
    }
    onTrackChanged: if (!track) live = {}

    // ------------------------------------------------------------ presets
    // Each starts from the defaults and changes only what it lists.
    readonly property var presets: [
        { group: "Classic", items: [
            { name: "Concert Grand", v: { tone: 0, brightness: 60, decay: 8, release: 300, velocity: 75, space: 25, strings: 55, body: 45 } },
            { name: "Bright Pop", v: { tone: 0, brightness: 88, decay: 5, release: 200, velocity: 65, space: 12, volume: -5, body: 25 } },
            { name: "Dark & Moody", v: { tone: 0, brightness: 15, decay: 10, release: 900, velocity: 80, space: 50, body: 60 } },
            { name: "Intimate Felt", v: { tone: 2, brightness: 25, decay: 6, release: 500, velocity: 50, space: 30, mechanics: 60 } },
            { name: "Upright Studio", v: { tone: 0, brightness: 50, decay: 4, release: 180, velocity: 70, space: 8, stereo: 35, body: 65 } },
            { name: "Honky-tonk Bar", v: { tone: 8, brightness: 70, decay: 5, release: 250, velocity: 75, space: 18, strings: 70 } }
        ] },
        { group: "Cinematic", items: [
            { name: "Cinematic", v: { tone: 0, brightness: 40, decay: 14, release: 1200, velocity: 65, mod_rate: 0.35, mod_depth: 18, space: 72, resonance: 70 } },
            { name: "Dramatic", v: { tone: 0, brightness: 80, decay: 12, release: 700, velocity: 100, space: 48, volume: -3, body: 60 } },
            { name: "Epic Hall", v: { tone: 0, brightness: 55, decay: 16, release: 1500, velocity: 85, space: 95, stereo: 90 } },
            { name: "Dreamy", v: { tone: 2, brightness: 40, decay: 12, release: 1600, velocity: 55, mod_rate: 0.3, mod_depth: 60, space: 85 } },
            { name: "Ambient Felt", v: { tone: 2, brightness: 30, decay: 18, release: 2000, velocity: 45, mod_rate: 0.2, mod_depth: 70, space: 92 } },
            { name: "Suspense", v: { tone: 2, brightness: 12, decay: 14, release: 1300, velocity: 60, trem_rate: 0.8, trem_depth: 25, space: 70 } },
            { name: "Celesta Magic", v: { tone: 7, brightness: 55, decay: 6, release: 600, space: 65, mod_rate: 0.4, mod_depth: 25 } },
            { name: "Music Box", v: { tone: 6, brightness: 60, decay: 5, release: 800, space: 55, stereo: 80 } }
        ] },
        { group: "Electric & cool", items: [
            { name: "Cool Keys", v: { tone: 1, brightness: 35, decay: 5, release: 400, velocity: 70, mod_rate: 0.6, mod_depth: 45, trem_rate: 3.5, trem_depth: 25, autopan: 1, space: 30 } },
            { name: "Vintage Tines", v: { tone: 1, brightness: 55, decay: 6, release: 300, velocity: 80, trem_rate: 4.5, trem_depth: 40, autopan: 1, space: 18, drive: 15 } },
            { name: "Suitcase Tremolo", v: { tone: 1, brightness: 50, decay: 6, release: 300, trem_rate: 5.5, trem_depth: 60, space: 15 } },
            { name: "Jazz Club", v: { tone: 1, brightness: 42, decay: 5, release: 280, velocity: 75, mod_rate: 0.8, mod_depth: 22, space: 22 } },
            { name: "Reed Grit", v: { tone: 3, brightness: 60, decay: 5, release: 250, velocity: 90, drive: 35, trem_rate: 5, trem_depth: 30, space: 15 } },
            { name: "Funky Strings", v: { tone: 4, brightness: 75, decay: 3, release: 120, velocity: 85, drive: 20, space: 8, mod_rate: 1.5, mod_depth: 20 } },
            { name: "Lo-fi Chill", v: { tone: 2, brightness: 20, decay: 5, release: 350, velocity: 40, mod_rate: 1.2, mod_depth: 55, space: 35, volume: -5 } },
            { name: "Neo Soul", v: { tone: 1, brightness: 65, decay: 7, release: 350, velocity: 90, mod_rate: 0.5, mod_depth: 30, trem_rate: 2.5, trem_depth: 20, autopan: 1, space: 25 } },
            { name: "Toy Box", v: { tone: 5, brightness: 65, decay: 4, release: 300, space: 30 } }
        ] }
    ]
    /// The user's own presets ({ name, values }), reloaded when saved or deleted.
    property var userPresets: []
    function reloadUserPresets() {
        try { userPresets = JSON.parse(app.userPresets("piano") || "[]") } catch (e) { userPresets = [] }
    }
    Component.onCompleted: reloadUserPresets()
    property string presetName: ""
    property var presetValues: null
    readonly property bool presetEdited: {
        if (!presetValues || !inst) return false
        for (const k in presetValues) if (Math.abs(v(k) - presetValues[k]) > 1e-3) return true
        return false
    }
    function loadPreset(name, values) {
        const all = {}
        for (const d of def.params) all[d.key] = d.default
        Object.assign(all, values)
        apply(all, "Piano Preset: " + name)
        presetName = name
        presetValues = all
    }
    /// Loads a preset and plays a short phrase so it can be heard.
    function preview(name, values) {
        loadPreset(name, values)
        if (track && app.selectedTrack !== track.id) app.selectTrack(track.id)
        const c = firstKey + 12
        const phrase = [[0, c, 90], [180, c + 4, 80], [360, c + 7, 85], [540, c + 12, 95]]
        for (const [t, k, vel] of phrase) schedule(t, k, vel, 700)
        for (const k of [c, c + 4, c + 7, c + 11]) schedule(820, k, 80, 1400)
    }

    // A/B: two settings to compare. Switching stores the current one.
    property var ab: ({ A: null, B: null })
    property string abSlot: "A"
    function switchAb(slot) {
        if (slot === abSlot) return
        const store = Object.assign({}, ab)
        store[abSlot] = snapshot()
        if (store[slot]) apply(store[slot], "Piano " + slot)
        else store[slot] = snapshot()
        ab = store
        abSlot = slot
    }

    // ------------------------------------------------------------ playing
    property int octave: 3
    property int octaves: 3
    readonly property int firstKey: (octave + 1) * 12
    /// How hard the computer keyboard plays: soft, medium or hard, with a
    /// little random variation when Humanize is on.
    property int strength: 1
    property bool humanize: true
    function keyVelocity() {
        const base = [55, 92, 122][strength]
        return Math.max(1, Math.min(127, base + (humanize ? Math.round((Math.random() - 0.5) * 18) : 0)))
    }
    /// Off, Chord, Strum or Arp.
    property int playMode: 0
    readonly property var playModes: ["Off", "Chord", "Strum", "Arp"]
    property bool showScale: true
    property bool pedal: false
    function setPedal(down) {
        if (pedal === down) return
        pedal = down
        app.sustain(down)
    }

    // The song's key, for scale highlighting and chords that fit it.
    readonly property var scaleInfo: {
        const m = /^([A-G])\s*([#♯b♭]?)\s*(major|minor)?/i.exec(app.keyName || "")
        if (!m) return null
        let root = { C: 0, D: 2, E: 4, F: 5, G: 7, A: 9, B: 11 }[m[1].toUpperCase()]
        if (m[2] === "#" || m[2] === "♯") root += 1
        if (m[2] === "b" || m[2] === "♭") root += 11
        const minor = (m[3] || "major").toLowerCase() === "minor"
        return { root: root % 12, steps: minor ? [0, 2, 3, 5, 7, 8, 10] : [0, 2, 4, 5, 7, 9, 11], name: app.keyName }
    }
    function inScale(key) {
        if (!scaleInfo) return false
        return scaleInfo.steps.indexOf(((key - scaleInfo.root) % 12 + 12) % 12) >= 0
    }
    /// The notes one key plays: itself, or a chord that fits the song's key.
    function voicing(key) {
        if (playMode === 0) return [key]
        if (scaleInfo && inScale(key)) {
            // Stack thirds within the scale.
            const deg = scaleInfo.steps.indexOf(((key - scaleInfo.root) % 12 + 12) % 12)
            const notes = [key]
            for (const jump of [2, 4]) {
                const d = deg + jump
                const step = scaleInfo.steps[d % 7] + Math.floor(d / 7) * 12 - scaleInfo.steps[deg]
                notes.push(key + step)
            }
            return notes
        }
        return [key, key + 4, key + 7]
    }

    /// Mouse/computer-keyboard presses: source key → notes sounding for it.
    property var down: ({})
    function press(key, velocity) {
        if (!track) return
        if (app.selectedTrack !== track.id) app.selectTrack(track.id)
        if (down[key]) return
        const notes = voicing(key)
        const d = Object.assign({}, down); d[key] = notes; down = d
        if (playMode === 3) { arpStart(); return }
        notes.forEach((n, i) => {
            if (playMode === 2 && i > 0) schedule(i * 35, n, velocity, -1, key)
            else app.noteOn(n, velocity)
        })
    }
    function lift(key) {
        const notes = down[key]
        if (!notes) return
        const d = Object.assign({}, down); delete d[key]; down = d
        cancelFor(key)
        if (playMode === 3) { if (Object.keys(down).length === 0) arpStop(); return }
        for (const n of notes) app.noteOff(n)
    }
    function releaseAll() {
        for (const k in down) lift(Number(k))
        for (const e of queue) if (!e.on) app.noteOff(e.key)
        queue = []
        arpStop()
        setPedal(false)
    }

    // A tiny scheduler for strums, previews and the arpeggiator.
    property var queue: []
    function schedule(afterMs, key, velocity, lengthMs, owner) {
        const t = Date.now() + afterMs
        const q = queue.slice()
        q.push({ t: t, on: true, key: key, vel: velocity, owner: owner === undefined ? -1 : owner })
        if (lengthMs > 0) q.push({ t: t + lengthMs, on: false, key: key, owner: -1 })
        queue = q
    }
    function cancelFor(owner) {
        // Unplayed strum notes of a released key never start.
        if (queue.some(e => e.owner === owner)) queue = queue.filter(e => e.owner !== owner)
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
            for (const e of due) {
                if (e.on) win.app.noteOn(e.key, e.vel)
                else win.app.noteOff(e.key)
            }
        }
    }
    // Arpeggiator: up through the held chords over two octaves, in 16ths.
    property int arpIndex: 0
    property int arpNote: -1
    function arpNotes() {
        let all = []
        for (const k in down) all = all.concat(down[k])
        all = all.filter((n, i) => all.indexOf(n) === i).sort((a, b) => a - b)
        return all.concat(all.map(n => n + 12))
    }
    function arpStart() { if (!arpTimer.running) { arpIndex = 0; arpTimer.start(); arpTimer.triggered() } }
    function arpStop() { arpTimer.stop(); if (arpNote >= 0) app.noteOff(arpNote); arpNote = -1 }
    Timer {
        id: arpTimer
        interval: Math.max(40, 60000 / Math.max(30, win.app.tempo) / 4)
        repeat: true
        onTriggered: {
            const notes = win.arpNotes()
            if (win.arpNote >= 0) win.app.noteOff(win.arpNote)
            if (!notes.length) { win.arpNote = -1; return }
            win.arpNote = notes[win.arpIndex % notes.length]
            win.arpIndex++
            win.app.noteOn(win.arpNote, win.keyVelocity())
        }
    }

    // ------------------------------------------------- computer keyboard
    /// True while the window takes the computer keyboard (it has focus).
    readonly property bool typing: shown && typingKeys.activeFocus
    readonly property var typingMap: ({
        [Qt.Key_A]: 0, [Qt.Key_W]: 1, [Qt.Key_S]: 2, [Qt.Key_E]: 3, [Qt.Key_D]: 4, [Qt.Key_F]: 5,
        [Qt.Key_T]: 6, [Qt.Key_G]: 7, [Qt.Key_Y]: 8, [Qt.Key_H]: 9, [Qt.Key_U]: 10, [Qt.Key_J]: 11,
        [Qt.Key_K]: 12, [Qt.Key_O]: 13, [Qt.Key_L]: 14, [Qt.Key_P]: 15, [Qt.Key_Semicolon]: 16
    })
    /// The letter that plays each key (offset from `firstKey`), for labels.
    readonly property var keyLetters: ["A", "W", "S", "E", "D", "F", "T", "G", "Y", "H", "U", "J", "K", "O", "L", "P", ";"]
    Item {
        id: typingKeys
        // Keys pressed on the computer keyboard, by Qt key, so a held note
        // ends on the same pitch even if the octave changes meanwhile.
        property var playing: ({})
        Keys.onPressed: e => {
            if (e.isAutoRepeat) { e.accepted = true; return }
            if (e.modifiers & (Qt.ControlModifier | Qt.MetaModifier | Qt.AltModifier)) return
            if (e.key === Qt.Key_Z) win.octave = Math.max(0, win.octave - 1)
            else if (e.key === Qt.Key_X) win.octave = Math.min(7, win.octave + 1)
            else if (e.key === Qt.Key_1 || e.key === Qt.Key_2 || e.key === Qt.Key_3) win.strength = e.key - Qt.Key_1
            else if (e.key === Qt.Key_Tab || e.key === Qt.Key_Backtab) win.setPedal(true)
            else if (e.key === Qt.Key_Space) win.app.togglePlay()
            else if (e.key === Qt.Key_Escape) win.closeRequested()
            else if (win.typingMap[e.key] !== undefined) {
                const note = win.firstKey + win.typingMap[e.key]
                const p = Object.assign({}, playing); p[e.key] = note; playing = p
                win.press(note, win.keyVelocity())
            } else return
            e.accepted = true
        }
        Keys.onReleased: e => {
            if (e.isAutoRepeat) { e.accepted = true; return }
            if (e.key === Qt.Key_Tab || e.key === Qt.Key_Backtab) { if (!pedalButton.latched) win.setPedal(false); e.accepted = true; return }
            const note = playing[e.key]
            if (note === undefined) return
            const p = Object.assign({}, playing); delete p[e.key]; playing = p
            win.lift(note)
            e.accepted = true
        }
        onActiveFocusChanged: if (!activeFocus) {
            for (const k in playing) win.lift(playing[k])
            playing = {}
            if (!pedalButton.latched) win.setPedal(false)
        }
    }

    // ------------------------------------------------------ lit keys
    function lit(key) {
        if (inDown(key)) return true
        return !!activity && !!track && app.selectedTrack === track.id && activity.lit(key)
    }
    function inDown(key) {
        for (const k in down) if (down[k].indexOf(key) >= 0) return true
        return false
    }
    function velOf(key) {
        if (activity && activity.vel[key] !== undefined) return activity.vel[key]
        return 90
    }
    /// Notes playing outside the keys shown, so the octave buttons can say so.
    readonly property bool litBelow: !!activity && Object.keys(activity.held).some(n => n < firstKey)
    readonly property bool litAbove: !!activity && Object.keys(activity.held).some(n => n > firstKey + octaves * 12)

    // The chord being played, by name.
    readonly property var noteNames: ["C", "C♯", "D", "E♭", "E", "F", "F♯", "G", "A♭", "A", "B♭", "B"]
    property string chordName: ""
    function nameChord(keys) {
        if (!keys.length) return ""
        keys.sort((a, b) => a - b)
        const bass = keys[0]
        if (keys.length === 1) return noteNames[bass % 12] + (Math.floor(bass / 12) - 1)
        const pcs = keys.map(k => k % 12).filter((p, i, a) => a.indexOf(p) === i)
        const shapes = [
            ["", [0, 4, 7]], ["m", [0, 3, 7]], ["dim", [0, 3, 6]], ["aug", [0, 4, 8]],
            ["sus2", [0, 2, 7]], ["sus4", [0, 5, 7]], ["7", [0, 4, 7, 10]], ["maj7", [0, 4, 7, 11]],
            ["m7", [0, 3, 7, 10]], ["m7♭5", [0, 3, 6, 10]], ["dim7", [0, 3, 6, 9]], ["6", [0, 4, 7, 9]],
            ["m6", [0, 3, 7, 9]], ["add9", [0, 2, 4, 7]], ["9", [0, 2, 4, 7, 10]], ["maj9", [0, 2, 4, 7, 11]],
            ["m9", [0, 2, 3, 7, 10]], ["mMaj7", [0, 3, 7, 11]], ["7sus4", [0, 5, 7, 10]], ["5", [0, 7]],
            ["", [0, 4]], ["m", [0, 3]]
        ]
        // Try each note as the root; prefer the bass note.
        const roots = [bass % 12].concat(pcs.filter(p => p !== bass % 12))
        for (const root of roots) {
            const rel = pcs.map(p => (p - root + 12) % 12).sort((a, b) => a - b)
            for (const [suffix, shape] of shapes) {
                if (shape.length === rel.length && shape.every((s, i) => s === rel[i]))
                    return noteNames[root] + suffix + (root !== bass % 12 ? "/" + noteNames[bass % 12] : "")
            }
        }
        return pcs.map(p => noteNames[p]).join(" ")
    }

    // Notes rising above the keys: { key, vel, start, end (0 while held) }.
    property var bars: []
    readonly property real riseSpeed: 70
    function updateLit() {
        const t = Date.now()
        const shownKeys = []
        for (let k = firstKey; k <= firstKey + octaves * 12; ++k) if (lit(k)) shownKeys.push(k)
        // Chord name from everything held (including off-screen notes).
        const all = shownKeys.slice()
        if (activity && track && app.selectedTrack === track.id) for (const k in activity.held) if (all.indexOf(Number(k)) < 0) all.push(Number(k))
        const name = nameChord(all)
        if (name !== "") chordName = name
        // Bars: start new ones, end the ones whose keys stopped, drop the ones gone.
        let changed = false
        const b = bars.filter(x => x.end === 0 || (t - x.end) * riseSpeed / 1000 < trail.height + 4)
        if (b.length !== bars.length) changed = true
        for (const x of b) if (x.end === 0 && shownKeys.indexOf(x.key) < 0) { x.end = t; changed = true }
        for (const k of shownKeys) if (!b.some(x => x.key === k && x.end === 0)) { b.push({ key: k, vel: velOf(k), start: t, end: 0 }); changed = true }
        if (changed) bars = b
        if (bars.length) trail.requestPaint()
    }
    Connections {
        target: win.activity
        enabled: win.shown
        function onTicked() { win.updateLit() }
    }
    onDownChanged: updateLit()
    /// Glow colour for a velocity: cool blue when soft, bright white when hard.
    function glowFor(vel, alpha) {
        const t = Math.max(0, Math.min(1, (vel - 30) / 97))
        return Qt.rgba(0.45 + 0.5 * t, 0.55 + 0.42 * t, 1, alpha)
    }

    // ================================================================ body
    Rectangle {
        anchors.fill: parent
        radius: 34
        gradient: Gradient {
            GradientStop { position: 0; color: "#2A3042" }
            GradientStop { position: 1; color: win.bodyDark }
        }
        border.color: "#12151E"
        // Swallow clicks and scrolls meant for the window.
        MouseArea { anchors.fill: parent; acceptedButtons: Qt.AllButtons; onPressed: typingKeys.forceActiveFocus(); onWheel: w => w.accepted = true }
    }

    // The engraved outline with raised tabs top and bottom (drawn once per size).
    Canvas {
        anchors.fill: parent
        anchors.margins: 14
        onWidthChanged: requestPaint()
        onHeightChanged: requestPaint()
        onPaint: {
            const ctx = getContext("2d")
            ctx.reset()
            const w = width, h = height, r = 24, tab = 8, t0 = w * 0.3, t1 = w * 0.7
            ctx.strokeStyle = "rgba(120,132,170,0.45)"
            ctx.lineWidth = 1.2
            ctx.beginPath()
            ctx.moveTo(r, tab)
            ctx.lineTo(t0 - 18, tab)
            ctx.quadraticCurveTo(t0 - 6, tab, t0, 0.5)
            ctx.lineTo(t1, 0.5)
            ctx.quadraticCurveTo(t1 + 6, tab, t1 + 18, tab)
            ctx.lineTo(w - r, tab)
            ctx.arcTo(w - 0.5, tab, w - 0.5, tab + r, r)
            ctx.lineTo(w - 0.5, h - tab - r)
            ctx.arcTo(w - 0.5, h - tab, w - r, h - tab, r)
            ctx.lineTo(t1 + 18, h - tab)
            ctx.quadraticCurveTo(t1 + 6, h - tab, t1, h - 0.5)
            ctx.lineTo(t0, h - 0.5)
            ctx.quadraticCurveTo(t0 - 6, h - tab, t0 - 18, h - tab)
            ctx.lineTo(r, h - tab)
            ctx.arcTo(0.5, h - tab, 0.5, h - tab - r, r)
            ctx.lineTo(0.5, tab + r)
            ctx.arcTo(0.5, tab, r, tab, r)
            ctx.closePath()
            ctx.stroke()
        }
    }

    // ------------------------------------------------ top: name, close
    Item {
        id: top
        x: 0; y: 10
        width: parent.width
        height: 34
        DragHandler {
            target: win
            // Free to move, as long as part of the top bar stays on screen.
            xAxis.minimum: 120 - win.width; xAxis.maximum: win.parent ? win.parent.width - 120 : 0
            yAxis.minimum: 0; yAxis.maximum: win.parent ? win.parent.height - 50 : 0
        }
        DText {
            anchors.centerIn: parent
            text: "ISAI PIANO" + (win.track ? "  ·  " + win.track.name.toUpperCase() : "")
            font.pixelSize: 11
            font.weight: Font.DemiBold
            font.letterSpacing: 2.5
            color: win.label
        }
        LearnButton {
            anchors.right: parent.right
            anchors.rightMargin: 64
            anchors.verticalCenter: parent.verticalCenter
            implicitWidth: 26; implicitHeight: 26; iconSize: 14
            topic: "piano"
            backdrop: win.backdrop
        }
        AbstractButton {
            id: closeButton
            anchors.right: parent.right
            anchors.rightMargin: 34
            anchors.verticalCenter: parent.verticalCenter
            width: 26; height: 26
            hoverEnabled: true
            onClicked: win.closeRequested()
            background: Rectangle { radius: 13; color: closeButton.hovered ? "#323A50" : "transparent" }
            contentItem: Icon { glyph: Icons.x; size: 13; color: win.label }
            DToolTip { visible: closeButton.hovered; text: "Close (Esc)" }
        }
    }

    // ------------------------------------------------ presets · tone · A/B · light
    Item {
        id: header
        x: 44; y: top.y + top.height + 4
        width: parent.width - 88
        height: 30

        Pill {
            id: presetButton
            x: 0
            width: Math.max(160, presetRow.implicitWidth + 24)
            active: presetPopup.opened
            onClicked: { win.reloadUserPresets(); presetPopup.opened ? presetPopup.close() : presetPopup.open() }
            Row {
                id: presetRow
                anchors.centerIn: parent
                spacing: 6
                Icon { anchors.verticalCenter: parent.verticalCenter; glyph: Icons.sparkle; size: 11; color: win.glow }
                DText {
                    anchors.verticalCenter: parent.verticalCenter
                    text: win.presetName ? win.presetName + (win.presetEdited ? " *" : "") : "PRESETS"
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                    font.letterSpacing: win.presetName ? 0.5 : 1.5
                    color: win.glow
                }
                Icon { anchors.verticalCenter: parent.verticalCenter; glyph: Icons.caretDown; size: 9; color: win.label }
            }
            DToolTip { visible: presetButton.hovered && !presetPopup.opened; text: "Ready-made sounds and your own" + (win.presetEdited ? " · * changed since loading" : "") }
            PresetPopup { id: presetPopup; y: parent.height + 8 }
        }

        // Tone: ◀ name ▶ (click the name for the full list).
        Row {
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: 6
            Pill {
                width: 30
                onClicked: win.set("tone", (Math.round(win.v("tone")) + 8) % 9, true)
                Icon { anchors.centerIn: parent; glyph: Icons.caretRight; rotation: 180; size: 11; color: win.label }
            }
            Pill {
                id: toneButton
                width: 190
                active: tonePopup.opened
                onClicked: tonePopup.opened ? tonePopup.close() : tonePopup.open()
                DText {
                    anchors.centerIn: parent
                    text: win.param("tone") ? win.param("tone").choices[Math.round(win.v("tone"))].toUpperCase() : ""
                    font.pixelSize: 12
                    font.weight: Font.DemiBold
                    font.letterSpacing: 2
                    color: win.glow
                }
                Popup {
                    id: tonePopup
                    y: parent.height + 8
                    x: (parent.width - width) / 2
                    padding: 12
                    focus: false
                    onClosed: typingKeys.forceActiveFocus()
                    background: Rectangle { radius: 14; color: "#F21B1F2C"; border.color: win.outline }
                    contentItem: Grid {
                        columns: 3
                        spacing: 6
                        Repeater {
                            model: win.param("tone") ? win.param("tone").choices : []
                            delegate: Pill {
                                id: toneChoice
                                required property string modelData
                                required property int index
                                width: 120
                                active: Math.round(win.v("tone")) === index
                                onClicked: { win.set("tone", index, true); tonePopup.close() }
                                DText {
                                    anchors.centerIn: parent
                                    text: toneChoice.modelData.toUpperCase()
                                    font.pixelSize: 10
                                    font.weight: Font.DemiBold
                                    font.letterSpacing: 1.5
                                    color: toneChoice.active ? win.glow : win.label
                                }
                            }
                        }
                    }
                }
            }
            Pill {
                width: 30
                onClicked: win.set("tone", (Math.round(win.v("tone")) + 1) % 9, true)
                Icon { anchors.centerIn: parent; glyph: Icons.caretRight; size: 11; color: win.label }
            }
        }

        Row {
            anchors.right: parent.right
            spacing: 6
            Repeater {
                model: ["A", "B"]
                delegate: Pill {
                    id: abButton
                    required property string modelData
                    width: 30
                    active: win.abSlot === modelData
                    onClicked: win.switchAb(modelData)
                    DText { anchors.centerIn: parent; text: abButton.modelData; font.pixelSize: 11; font.weight: Font.DemiBold; color: abButton.active ? win.glow : win.label }
                    DToolTip { visible: abButton.hovered; text: "Compare two settings: switch to " + abButton.modelData + " (the other one is kept)" }
                }
            }
            Item { width: 6; height: 1 }
            Pill {
                id: lightButton
                width: 70
                active: win.v("quality") >= 0.5
                onClicked: win.set("quality", active ? 0 : 1, true)
                Row {
                    anchors.centerIn: parent
                    spacing: 5
                    Icon { anchors.verticalCenter: parent.verticalCenter; glyph: Icons.feather; size: 11; color: lightButton.active ? win.glow : win.label }
                    DText { anchors.verticalCenter: parent.verticalCenter; text: "LIGHT"; font.pixelSize: 10; font.weight: Font.DemiBold; font.letterSpacing: 1; color: lightButton.active ? win.glow : win.label }
                }
                DToolTip { visible: lightButton.hovered; text: lightButton.active ? "Light mode: fewer voices and a simpler tone, for slow computers. Click for full quality." : "Use less CPU (for slow computers)" }
            }
        }
    }

    // ------------------------------------------------ knobs
    Row {
        id: knobRow1
        anchors.horizontalCenter: parent.horizontalCenter
        y: header.y + header.height + 14
        spacing: 30
        Group {
            title: "SOUND"
            Knob { key: "brightness"; label: "Bright" }
            Knob { key: "decay"; label: "Decay" }
            Knob { key: "release"; label: "Release" }
            Knob { key: "velocity"; label: "Dynamics" }
        }
        Group {
            title: "CHARACTER"
            Knob { key: "strings"; label: "Strings" }
            Knob { key: "body"; label: "Body" }
            Knob { key: "mechanics"; label: "Mechanics" }
            Knob { key: "drive"; label: "Drive" }
        }
        Group {
            title: "OUTPUT"
            Knob { key: "volume"; label: "Volume" }
            // This track's level.
            Row {
                height: 46
                spacing: 3
                Repeater {
                    model: 2
                    delegate: Rectangle {
                        id: meterBar
                        required property int index
                        width: 5; height: 46; radius: 2.5
                        color: "#11141C"
                        Rectangle {
                            anchors.bottom: parent.bottom
                            width: parent.width
                            height: parent.height * Math.max(0, Math.min(1, win.level[meterBar.index] || 0))
                            radius: 2.5
                            color: win.glow
                            opacity: 0.85
                        }
                    }
                }
            }
        }
    }
    Row {
        id: knobRow2
        anchors.horizontalCenter: parent.horizontalCenter
        y: knobRow1.y + knobRow1.height + 8
        spacing: 30
        Group {
            title: "MODULATION"
            Knob { key: "mod_rate"; label: "Rate" }
            Knob { key: "mod_depth"; label: "Depth" }
        }
        // Tremolo or auto-pan.
        Item {
            width: 30; height: 96
            AbstractButton {
                id: modeSwitch
                readonly property bool pan: win.v("autopan") >= 0.5
                anchors.horizontalCenter: parent.horizontalCenter
                y: 34
                width: 22; height: 40
                hoverEnabled: true
                onClicked: win.set("autopan", pan ? 0 : 1, true)
                background: Rectangle {
                    radius: 11
                    color: win.bodyDark
                    border.color: modeSwitch.hovered ? "#56607E" : "#3A4258"
                    Rectangle {
                        x: 4
                        y: modeSwitch.pan ? 4 : parent.height - height - 4
                        width: 14; height: 14; radius: 7
                        color: "#C9CEE6"
                        Behavior on y { NumberAnimation { duration: 140; easing.type: Easing.OutCubic } }
                    }
                }
                DToolTip { visible: modeSwitch.hovered; text: modeSwitch.pan ? "Tremolo sweeps left and right (auto-pan) · click for volume pulse" : "Tremolo pulses the volume · click for auto-pan" }
            }
            DText { anchors.horizontalCenter: parent.horizontalCenter; y: modeSwitch.y - 16; text: "PAN"; font.pixelSize: 8; font.letterSpacing: 1; color: modeSwitch.pan ? win.glow : win.faint }
            DText { anchors.horizontalCenter: parent.horizontalCenter; y: modeSwitch.y + modeSwitch.height + 4; text: "TREM"; font.pixelSize: 8; font.letterSpacing: 1; color: modeSwitch.pan ? win.faint : win.glow }
        }
        Group {
            title: "TREMOLO"
            Knob { key: "trem_rate"; label: "Rate" }
            Knob { key: "trem_depth"; label: "Depth" }
        }
        Group {
            title: "ROOM"
            Knob { key: "space"; label: "Space" }
            Knob { key: "resonance"; label: "Resonance" }
            Knob { key: "stereo"; label: "Width" }
        }
    }

    // ------------------------------------------------ playing strip
    Item {
        id: strip
        x: 44
        width: parent.width - 88
        y: knobRow2.y + knobRow2.height + 10
        height: 30

        // Strength (computer keyboard) and humanize.
        Row {
            id: strengthRow
            anchors.verticalCenter: parent.verticalCenter
            spacing: 4
            Repeater {
                model: ["SOFT", "MED", "HARD"]
                delegate: Pill {
                    id: strengthButton
                    required property string modelData
                    required property int index
                    width: 52
                    active: win.strength === index
                    onClicked: win.strength = index
                    DText { anchors.centerIn: parent; text: strengthButton.modelData; font.pixelSize: 9; font.weight: Font.DemiBold; font.letterSpacing: 1; color: strengthButton.active ? win.glow : win.label }
                    DToolTip { visible: strengthButton.hovered; text: "How hard the computer keyboard plays (" + (strengthButton.index + 1) + ")" }
                }
            }
            Pill {
                id: humanizeButton
                width: 78
                active: win.humanize
                onClicked: win.humanize = !win.humanize
                DText { anchors.centerIn: parent; text: "HUMANIZE"; font.pixelSize: 9; font.weight: Font.DemiBold; font.letterSpacing: 1; color: humanizeButton.active ? win.glow : win.label }
                DToolTip { visible: humanizeButton.hovered; text: "Vary how hard each key plays a little, like a real player" }
            }
        }

        // The display: the chord (or note) being played, and the song's key.
        Rectangle {
            anchors.left: strengthRow.right; anchors.leftMargin: 14
            anchors.right: modeRow.left; anchors.rightMargin: 14
            anchors.verticalCenter: parent.verticalCenter
            height: 30
            radius: 8
            color: "#0F121A"
            border.color: "#2A3044"
            DText {
                anchors.centerIn: parent
                text: win.chordName || "—"
                font.family: Theme.mono
                font.pixelSize: 15
                font.weight: Font.DemiBold
                color: win.glow
                opacity: win.chordName ? 1 : 0.4
            }
            DText {
                id: scaleLabel
                anchors.right: parent.right; anchors.rightMargin: 8
                anchors.verticalCenter: parent.verticalCenter
                visible: !!win.scaleInfo
                text: win.scaleInfo ? win.scaleInfo.name : ""
                font.pixelSize: 9
                color: win.showScale ? win.label : win.faint
                MouseArea { id: scaleArea; anchors.fill: parent; anchors.margins: -4; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: win.showScale = !win.showScale }
                DToolTip { visible: scaleArea.containsMouse; text: "Dots mark the notes of the song's key · click to " + (win.showScale ? "hide" : "show") + " them" }
            }
        }

        // Play mode and keyboard size.
        Row {
            id: modeRow
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            spacing: 4
            Repeater {
                model: win.playModes
                delegate: Pill {
                    id: modeButton
                    required property string modelData
                    required property int index
                    width: 52
                    active: win.playMode === index
                    onClicked: { win.releaseAll(); win.playMode = index }
                    DText { anchors.centerIn: parent; text: modeButton.modelData.toUpperCase(); font.pixelSize: 9; font.weight: Font.DemiBold; font.letterSpacing: 1; color: modeButton.active ? win.glow : win.label }
                    DToolTip {
                        visible: modeButton.hovered
                        text: ["Each key plays one note", "Each key plays a chord in the song's key", "Each key strums a chord", "Hold keys to arpeggiate them in 16ths at the song's tempo"][modeButton.index]
                    }
                }
            }
            Item { width: 8; height: 1 }
            Repeater {
                model: [2, 3, 4]
                delegate: Pill {
                    id: octButton
                    required property int modelData
                    width: 44
                    active: win.octaves === modelData
                    onClicked: win.octaves = modelData
                    DText { anchors.centerIn: parent; text: octButton.modelData + " OCT"; font.pixelSize: 8; font.weight: Font.DemiBold; color: octButton.active ? win.glow : win.label }
                }
            }
        }
    }

    // ------------------------------------------------ keyboard
    Rectangle {
        id: bed
        x: 34
        width: parent.width - 68
        y: strip.y + strip.height + 10
        height: parent.height - y - 40
        radius: 22
        color: "#121520"
        border.color: "#0B0D14"

        // Octave buttons and the sustain pedal.
        Column {
            id: sideCol
            x: 10
            anchors.verticalCenter: parent.verticalCenter
            spacing: 6
            OctaveButton { anchors.horizontalCenter: parent.horizontalCenter; glyph: Icons.caretUp; hint: win.litAbove; onClicked: win.octave = Math.min(7, win.octave + 1) }
            DText { anchors.horizontalCenter: parent.horizontalCenter; text: "C" + win.octave; font.family: Theme.mono; font.pixelSize: 10; color: win.label }
            OctaveButton { anchors.horizontalCenter: parent.horizontalCenter; glyph: Icons.caretDown; hint: win.litBelow; onClicked: win.octave = Math.max(0, win.octave - 1) }
            Item { width: 1; height: 8 }
            AbstractButton {
                id: pedalButton
                /// Clicked on (stays down until clicked again), as opposed to held with Tab.
                property bool latched: false
                anchors.horizontalCenter: parent.horizontalCenter
                width: 40; height: 52
                hoverEnabled: true
                onClicked: { latched = !latched; win.setPedal(latched); typingKeys.forceActiveFocus() }
                background: Rectangle {
                    radius: 8
                    gradient: Gradient {
                        GradientStop { position: 0; color: win.pedal ? "#4A5378" : "#2A3044" }
                        GradientStop { position: 1; color: win.pedal ? "#3A4260" : "#1C2030" }
                    }
                    border.color: win.pedal ? win.glow : "#3A4258"
                    Behavior on border.color { ColorAnimation { duration: 120 } }
                }
                contentItem: DText {
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                    text: "PED"
                    font.pixelSize: 9
                    font.weight: Font.DemiBold
                    font.letterSpacing: 1
                    color: win.pedal ? win.glow : win.label
                }
                DToolTip { visible: pedalButton.hovered; text: "Sustain pedal: hold Tab, or click to keep it down (MIDI pedals work too)" }
            }
        }

        // Notes rising above the keys.
        Canvas {
            id: trail
            x: keys.x
            y: 8
            width: keys.width
            height: 54
            onPaint: {
                const ctx = getContext("2d")
                ctx.reset()
                const t = Date.now(), h = height, speed = win.riseSpeed / 1000
                for (const b of win.bars) {
                    const geo = keys.geometry(b.key)
                    if (!geo) continue
                    const bottom = b.end === 0 ? h : h - (t - b.end) * speed
                    const topY = Math.max(0, h - (t - b.start) * speed)
                    if (bottom <= 0 || bottom - topY < 0.5) continue
                    const c = win.glowFor(b.vel, 1)
                    const rgb = Math.round(c.r * 255) + "," + Math.round(c.g * 255) + "," + Math.round(c.b * 255)
                    const g = ctx.createLinearGradient(0, topY, 0, bottom)
                    g.addColorStop(0, "rgba(" + rgb + ",0.12)")
                    g.addColorStop(1, "rgba(" + rgb + ",0.85)")
                    ctx.fillStyle = g
                    const w = geo.black ? geo.w * 0.8 : geo.w * 0.6
                    ctx.beginPath()
                    ctx.roundedRect(geo.x + (geo.w - w) / 2, topY, w, bottom - topY, 3, 3)
                    ctx.fill()
                }
            }
        }

        Item {
            id: keys
            x: sideCol.x + sideCol.width + 12
            y: trail.y + trail.height + 4
            width: parent.width - x - 10
            height: parent.height - y - 8
            readonly property int whiteCount: win.octaves * 7 + 1
            readonly property real whiteW: width / whiteCount
            /// Where a key is drawn (for the rising notes), or null if not shown.
            function geometry(key) {
                const rel = key - win.firstKey
                if (rel < 0 || rel > win.octaves * 12) return null
                const oct = Math.floor(rel / 12), pc = rel % 12
                const white = [0, 2, 4, 5, 7, 9, 11].indexOf(pc)
                if (white >= 0) return { x: (oct * 7 + white) * whiteW, w: whiteW - 3, black: false }
                const slot = [1, 3, 6, 8, 10].indexOf(pc)
                const after = [0, 1, 3, 4, 5][slot]
                const w = whiteW * 0.58
                return { x: (oct * 7 + after + 1) * whiteW - w / 2 - 1.5, w: w, black: true }
            }
            // White keys
            Repeater {
                model: keys.whiteCount
                delegate: Key {
                    required property int index
                    readonly property int octaveIndex: Math.floor(index / 7)
                    key: win.firstKey + octaveIndex * 12 + [0, 2, 4, 5, 7, 9, 11][index % 7]
                    black: false
                    x: index * keys.whiteW
                    width: keys.whiteW - 3
                    height: keys.height
                }
            }
            // Black keys
            Repeater {
                model: win.octaves * 5
                delegate: Key {
                    required property int index
                    readonly property int octaveIndex: Math.floor(index / 5)
                    readonly property int slot: [0, 1, 3, 4, 5][index % 5]
                    key: win.firstKey + octaveIndex * 12 + [1, 3, 6, 8, 10][index % 5]
                    black: true
                    x: (octaveIndex * 7 + slot + 1) * keys.whiteW - width / 2 - 1.5
                    width: keys.whiteW * 0.58
                    height: keys.height * 0.6
                    z: 2
                }
            }
        }
    }

    // Computer keyboard help, under the keys.
    DText {
        anchors.horizontalCenter: parent.horizontalCenter
        y: parent.height - 34
        text: win.typing ? "A W S E D F T G Y H U J K O L P ;  play  ·  Z / X octave  ·  1 2 3 soft–hard  ·  Tab pedal  ·  Space play"
                         : "Click the piano to play it with your computer keyboard"
        font.pixelSize: 10
        color: win.typing ? win.label : win.faint
    }

    // ================================================================ parts
    // A rounded hardware-style button; its children go in the middle.
    component Pill: AbstractButton {
        id: pill
        property bool active: false
        default property alias content: inner.data
        height: 28
        hoverEnabled: true
        background: Rectangle {
            radius: 14
            color: pill.active ? "#3A4260" : pill.down ? "#323A50" : pill.hovered ? "#2C3246" : win.bodyDark
            border.color: pill.active ? Qt.rgba(0.89, 0.91, 1, 0.5) : "#2E3448"
            Behavior on color { ColorAnimation { duration: Theme.fast } }
        }
        contentItem: Item { id: inner }
    }

    // A section: a title between two lines, and its controls.
    component Group: Column {
        id: group
        property string title
        default property alias controls: controlRow.data
        spacing: 10
        Item {
            width: Math.max(controlRow.width, titleText.implicitWidth + 40)
            height: 14
            Rectangle { anchors.verticalCenter: parent.verticalCenter; width: (parent.width - titleText.implicitWidth) / 2 - 10; height: 1; color: win.outline }
            DText {
                id: titleText
                anchors.centerIn: parent
                text: group.title
                font.pixelSize: 11
                font.weight: Font.DemiBold
                font.letterSpacing: 1.5
                color: win.label
            }
            Rectangle { anchors.right: parent.right; anchors.verticalCenter: parent.verticalCenter; width: (parent.width - titleText.implicitWidth) / 2 - 10; height: 1; color: win.outline }
        }
        Row {
            id: controlRow
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: 10
        }
    }

    // A dark knob with a glowing value arc. Drag up/down (⇧ fine), scroll,
    // double-click for the default.
    component Knob: Item {
        id: knob
        property string key
        property string label
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
            if (def.log && def.min > 0) return def.min * Math.pow(def.max / def.min, p)
            return def.min + p * (def.max - def.min)
        }
        function fmt(v) {
            if (!def) return ""
            switch (def.unit) {
            case "%": return Math.round(v) + "%"
            case "ms": return Math.round(v) + " ms"
            case "s": return v.toFixed(1) + " s"
            case "Hz": return v.toFixed(v < 10 ? 2 : 1) + " Hz"
            case "dB": return (v > 0.05 ? "+" : "") + v.toFixed(1) + " dB"
            default: return v.toFixed(2)
            }
        }
        readonly property real pos: posOf(value)
        readonly property bool hot: area.containsMouse || area.pressed
        width: 64; height: 72

        Item {
            id: dial
            width: 46; height: 46
            anchors.horizontalCenter: parent.horizontalCenter
            Shape {
                anchors.fill: parent
                preferredRendererType: Shape.CurveRenderer
                ShapePath {
                    strokeColor: "#11141C"
                    strokeWidth: 4
                    fillColor: "transparent"
                    capStyle: ShapePath.RoundCap
                    PathAngleArc { centerX: 23; centerY: 23; radiusX: 20; radiusY: 20; startAngle: 135; sweepAngle: 270 }
                }
                ShapePath {
                    strokeColor: Qt.rgba(0.89, 0.91, 1, knob.hot ? 0.28 : 0.16)
                    strokeWidth: 8
                    fillColor: "transparent"
                    capStyle: ShapePath.RoundCap
                    PathAngleArc { centerX: 23; centerY: 23; radiusX: 20; radiusY: 20; startAngle: 135; sweepAngle: Math.max(0.01, 270 * knob.pos) }
                }
                ShapePath {
                    strokeColor: win.glow
                    strokeWidth: 2.2
                    fillColor: "transparent"
                    capStyle: ShapePath.RoundCap
                    PathAngleArc { centerX: 23; centerY: 23; radiusX: 20; radiusY: 20; startAngle: 135; sweepAngle: Math.max(0.01, 270 * knob.pos) }
                }
            }
            Rectangle {
                anchors.centerIn: parent
                width: 31; height: 31; radius: 15.5
                gradient: Gradient {
                    GradientStop { position: 0; color: "#2C3244" }
                    GradientStop { position: 1; color: "#151821" }
                }
                border.color: "#0D0F16"
                Rectangle {
                    anchors.fill: parent
                    anchors.margins: 1
                    radius: width / 2
                    color: "transparent"
                    border.color: Qt.rgba(1, 1, 1, 0.06)
                }
                Item {
                    anchors.fill: parent
                    rotation: -135 + 270 * knob.pos
                    Rectangle {
                        anchors.horizontalCenter: parent.horizontalCenter
                        y: 4
                        width: 4; height: 4; radius: 2
                        color: "#FFFFFF"
                    }
                }
            }
        }
        DText {
            anchors.horizontalCenter: parent.horizontalCenter
            y: dial.height + 6
            text: knob.hot ? knob.fmt(knob.value) : knob.label
            font.pixelSize: 11
            font.weight: Font.DemiBold
            color: knob.hot ? win.glow : win.label
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
                win.set(knob.key, knob.valueOf(startPos + (startY - m.y) / 180 * fine), false)
            }
            onReleased: { win.set(knob.key, knob.value, true); typingKeys.forceActiveFocus() }
            onDoubleClicked: if (knob.def) win.set(knob.key, knob.def.default, true)
            onWheel: w => {
                if (!knob.def) return
                const steps = w.angleDelta.y !== 0 ? w.angleDelta.y / 120 : w.pixelDelta.y / 30
                win.set(knob.key, knob.valueOf(knob.pos + steps * 0.02), true)
            }
        }
    }

    // One piano key: press to play (higher on the key plays softer).
    component Key: Item {
        id: k
        property int key
        property bool black
        readonly property bool held: win.lit(key)
        readonly property real vel: held ? win.velOf(key) : 90
        readonly property int offset: key - win.firstKey
        Rectangle {
            anchors.fill: parent
            anchors.topMargin: k.held ? 2 : 0
            radius: k.black ? 6 : 9
            gradient: Gradient {
                GradientStop { position: 0; color: k.black ? (k.held ? "#3A4056" : "#20242F") : (k.held ? "#B9BFD6" : "#E6E9F6") }
                GradientStop { position: 1; color: k.black ? (k.held ? "#2B3042" : "#161922") : (k.held ? "#A9AFC8" : "#CDD2E6") }
            }
            border.color: k.black ? "#0A0C12" : "#9DA3BC"
            // The key's front edge.
            Rectangle {
                anchors.bottom: parent.bottom
                anchors.horizontalCenter: parent.horizontalCenter
                width: parent.width - 4
                height: k.held ? 3 : (k.black ? 8 : 6)
                radius: k.black ? 4 : 6
                color: k.black ? "#2E3445" : "#B3B9D0"
            }
            Behavior on anchors.topMargin { NumberAnimation { duration: 50 } }
            // Glow while the note plays (brighter when harder), fading out after.
            Rectangle {
                anchors.fill: parent
                radius: parent.radius
                gradient: Gradient {
                    GradientStop { position: 0; color: win.glowFor(k.vel, k.black ? 0.8 : 0.6) }
                    GradientStop { position: 1; color: win.glowFor(k.vel, k.black ? 0.35 : 0.15) }
                }
                opacity: k.held ? 1 : 0
                Behavior on opacity { NumberAnimation { duration: k.held ? 40 : 260; easing.type: Easing.OutCubic } }
            }
            Rectangle {
                anchors.horizontalCenter: parent.horizontalCenter
                y: parent.height - (k.black ? 16 : 20)
                width: parent.width * 0.5
                height: 3
                radius: 1.5
                color: k.black ? "#C9CFFF" : "#6E78C8"
                opacity: k.held ? 1 : 0
                Behavior on opacity { NumberAnimation { duration: k.held ? 40 : 260 } }
            }
        }
        // In the song's key: a small dot.
        Rectangle {
            visible: win.showScale && win.inScale(k.key)
            anchors.horizontalCenter: parent.horizontalCenter
            y: k.black ? parent.height - 30 : parent.height - 38
            width: 5; height: 5; radius: 2.5
            color: k.black ? "#7F88B8" : "#8A92C8"
            opacity: 0.8
        }
        // The computer key that plays it.
        DText {
            visible: win.typing && k.offset >= 0 && k.offset < win.keyLetters.length
            anchors.horizontalCenter: parent.horizontalCenter
            y: k.black ? parent.height - 52 : parent.height * 0.6
            text: visible ? win.keyLetters[k.offset] : ""
            font.family: Theme.mono
            font.pixelSize: 10
            font.weight: Font.DemiBold
            color: k.black ? "#8A92B8" : "#5D6590"
        }
        DText {
            visible: !k.black && k.key % 12 === 0
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.bottom: parent.bottom
            anchors.bottomMargin: 12
            text: "C" + (Math.floor(k.key / 12) - 1)
            font.pixelSize: 9
            color: "#7D84A0"
        }
        MouseArea {
            anchors.fill: parent
            onPressed: m => {
                typingKeys.forceActiveFocus()
                // Higher on the key plays softer, near the front harder.
                win.press(k.key, Math.round(35 + 92 * Math.max(0, Math.min(1, m.y / height))))
            }
            onReleased: win.lift(k.key)
            onCanceled: win.lift(k.key)
        }
    }

    component OctaveButton: AbstractButton {
        id: ob
        property string glyph
        /// Notes are playing that way, outside the keys shown.
        property bool hint: false
        width: 26; height: 26
        hoverEnabled: true
        background: Rectangle {
            radius: 13
            color: ob.down ? "#3A4260" : ob.hovered ? "#2A3044" : "#1C2030"
            border.color: ob.hint ? win.glow : "#2E3448"
            Behavior on border.color { ColorAnimation { duration: 150 } }
        }
        contentItem: Icon { glyph: ob.glyph; size: 11; color: ob.hint ? win.glow : win.label }
    }

    // One preset in the list: click to load, ▶ to hear it, × to delete (yours).
    component PresetRow: Item {
        id: row
        property string name
        property var values
        property bool deletable: false
        signal chosen()
        readonly property bool current: win.presetName === name
        width: 170; height: 28
        Rectangle {
            anchors.fill: parent
            radius: 8
            color: row.current ? "#3A4260" : rowArea.containsMouse ? "#2A3044" : "transparent"
            border.color: row.current ? Qt.rgba(0.89, 0.91, 1, 0.4) : "transparent"
        }
        MouseArea {
            id: rowArea
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: { win.loadPreset(row.name, row.values); row.chosen() }
        }
        DText {
            anchors.left: parent.left; anchors.leftMargin: 10
            anchors.right: buttons.left; anchors.rightMargin: 4
            anchors.verticalCenter: parent.verticalCenter
            text: row.name
            elide: Text.ElideRight
            font.pixelSize: 12
            color: row.current ? win.glow : "#C9CFE2"
        }
        Row {
            id: buttons
            anchors.right: parent.right; anchors.rightMargin: 4
            anchors.verticalCenter: parent.verticalCenter
            spacing: 2
            opacity: rowArea.containsMouse || previewArea.containsMouse || deleteArea.containsMouse ? 1 : 0
            Rectangle {
                width: 22; height: 22; radius: 11
                color: previewArea.containsMouse ? "#4A5378" : "#2E3448"
                Icon { anchors.centerIn: parent; glyph: Icons.play; size: 9; color: win.glow }
                MouseArea { id: previewArea; anchors.fill: parent; hoverEnabled: true; onClicked: win.preview(row.name, row.values) }
                DToolTip { visible: previewArea.containsMouse; text: "Hear it" }
            }
            Rectangle {
                visible: row.deletable
                width: 22; height: 22; radius: 11
                color: deleteArea.containsMouse ? "#5A3040" : "#2E3448"
                Icon { anchors.centerIn: parent; glyph: Icons.x; size: 9; color: win.glow }
                MouseArea {
                    id: deleteArea
                    anchors.fill: parent
                    hoverEnabled: true
                    onClicked: { win.app.deleteUserPreset("piano", row.name); win.reloadUserPresets() }
                }
                DToolTip { visible: deleteArea.containsMouse; text: "Delete this preset" }
            }
        }
    }

    // Built-in presets, the user's own, and saving the current sound.
    component PresetPopup: Popup {
        id: pop
        padding: 16
        modal: false
        focus: true
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
        onClosed: { saving = false; typingKeys.forceActiveFocus() }
        property bool saving: false
        function saveAs(name) {
            if (name.trim() === "") return
            win.app.saveUserPreset("piano", name.trim(), JSON.stringify(win.snapshot()))
            win.presetName = name.trim()
            win.presetValues = win.snapshot()
            win.reloadUserPresets()
            saving = false
        }
        background: Rectangle { radius: 16; color: "#F21B1F2C"; border.color: win.outline }
        enter: Transition { NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.fast } }
        exit: Transition { NumberAnimation { property: "opacity"; to: 0; duration: Theme.fast } }
        contentItem: Row {
            spacing: 18
            Repeater {
                model: win.presets
                delegate: Column {
                    id: groupCol
                    required property var modelData
                    spacing: 2
                    DText { text: groupCol.modelData.group.toUpperCase(); font.pixelSize: 9; font.weight: Font.DemiBold; font.letterSpacing: 1.5; color: win.label; bottomPadding: 6 }
                    Repeater {
                        model: groupCol.modelData.items
                        delegate: PresetRow {
                            required property var modelData
                            name: modelData.name
                            values: modelData.v
                            onChosen: pop.close()
                        }
                    }
                }
            }
            // The user's own.
            Column {
                spacing: 2
                DText { text: "MY PRESETS"; font.pixelSize: 9; font.weight: Font.DemiBold; font.letterSpacing: 1.5; color: win.label; bottomPadding: 6 }
                Repeater {
                    model: win.userPresets
                    delegate: PresetRow {
                        required property var modelData
                        name: modelData.name
                        values: modelData.values
                        deletable: true
                        onChosen: pop.close()
                    }
                }
                DText {
                    visible: win.userPresets.length === 0 && !pop.saving
                    width: 170
                    text: "Make a sound you like, then save it here."
                    wrapMode: Text.WordWrap
                    font.pixelSize: 11
                    color: win.faint
                    bottomPadding: 6
                }
                Rectangle {
                    visible: pop.saving
                    width: 170; height: 30
                    radius: 8
                    color: "#11141C"
                    border.color: win.outline
                    TextInput {
                        id: nameInput
                        anchors.fill: parent
                        anchors.margins: 8
                        verticalAlignment: TextInput.AlignVCenter
                        color: win.glow
                        font.pixelSize: 12
                        font.family: Theme.font
                        clip: true
                        maximumLength: 40
                        onAccepted: pop.saveAs(text)
                        Keys.onEscapePressed: pop.saving = false
                    }
                    DText {
                        visible: nameInput.text === ""
                        anchors.left: parent.left; anchors.leftMargin: 8
                        anchors.verticalCenter: parent.verticalCenter
                        text: "Name, then Return"
                        font.pixelSize: 11
                        color: win.faint
                    }
                }
                Pill {
                    width: 170
                    visible: !pop.saving
                    onClicked: { pop.saving = true; nameInput.text = ""; nameInput.forceActiveFocus() }
                    Row {
                        anchors.centerIn: parent
                        spacing: 6
                        Icon { anchors.verticalCenter: parent.verticalCenter; glyph: Icons.plus; size: 11; color: win.glow }
                        DText { anchors.verticalCenter: parent.verticalCenter; text: "SAVE CURRENT SOUND"; font.pixelSize: 9; font.weight: Font.DemiBold; font.letterSpacing: 1; color: win.glow }
                    }
                }
            }
        }
    }
}
