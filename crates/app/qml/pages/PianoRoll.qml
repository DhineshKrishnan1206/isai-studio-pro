import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Isai

// The piano roll: a glass pane docked along the bottom (like the mixer) for
// editing one MIDI region's notes.
//   • Click the grid to add a note. Click a note to select it; Shift- or
//     ⌘/Ctrl-click adds to the selection; Shift- or ⌘/Ctrl-drag on empty grid
//     draws a selection box; ⌘A selects all, Esc clears.
//   • Drag a note to move it (with every selected note), drag its right edge
//     to change length. Arrow keys nudge: ←/→ by the grid, ↑/↓ a semitone,
//     Shift+↑/↓ an octave. Delete removes the selection.
//   • Right-click a note to delete it (right-drag erases every note you pass
//     over); ⌥-click works too.
//   • The snap menu sets the grid notes snap to (separate from the tracks').
//   • Quantize (Q) moves the notes onto that grid; the menu has gentler
//     options and swing.
//   • Tools: draw whole chords with one click (or chords that fit the song's
//     key), strum or arpeggiate chords, and show the key's notes and other
//     regions' notes (ghosts) behind the grid.
//   • The record button (or R while typing) records what you play on a MIDI
//     keyboard or the computer keyboard onto this track.
//   • Drag the bars in the velocity lane to set how hard each note plays.
//   • Click the keyboard to hear a note. "Typing" plays notes from the
//     computer keyboard: A W S E D F T G Y H U J K…, Z/X change octave.
Item {
    id: roll
    property var app
    property var tracks: []
    property int regionId: -1
    property bool shown: false
    /// What the frosted glass blurs.
    property Item backdrop: null
    signal closeRequested()
    /// Open the track's instrument window.
    signal instrumentRequested(int trackId)
    /// Which keys the selected instrument is playing, to light up the keyboard.
    property var activity: null
    readonly property bool listening: !!activity && !!track && app.selectedTrack === track.id

    readonly property var track: tracks.find(t => t.regions.some(r => r.id === regionId)) || null
    readonly property var region: track ? track.regions.find(r => r.id === regionId) || null : null
    readonly property var midi: region ? region.midi : null
    readonly property var notes: midi ? midi.notes : []
    readonly property color tint: track ? Theme.trackColor(track.color) : Theme.accent
    /// The bar length where the region starts (bars can change length along the song).
    readonly property real barBeats: TimeMap.barLenAt(region ? region.start : 0)

    property real dockHeight: 420
    readonly property int margin: 12
    x: margin
    width: parent.width - margin * 2
    height: Math.max(260, Math.min(parent.height * 0.75, dockHeight))
    y: parent.height - height - margin
    visible: opacity > 0 && !!region
    opacity: shown ? 1 : 0
    Behavior on opacity { NumberAnimation { duration: Theme.fast } }
    z: 50

    property real pxPerBeat: 48
    readonly property int keyHeight: 13
    readonly property int keyboardWidth: 56
    readonly property real lengthBeats: midi ? midi.length : 4
    /// Room to draw past the region's end (it grows to fit new notes).
    readonly property real gridBeats: lengthBeats + barBeats * 2
    property bool typing: false
    property bool showInstrument: true

    // The piano roll's own snap, in beats (0 = off). Triplets divide a beat in three.
    readonly property var snapChoices: [
        { label: "Off", beats: 0 },
        { label: "Bar", beats: -1 },
        { label: "1/2 note", beats: 2 },
        { label: "1/4 (beat)", beats: 1 },
        { label: "1/8", beats: 0.5 },
        { label: "1/16", beats: 0.25 },
        { label: "1/32", beats: 0.125 },
        { label: "1/8 triplet", beats: 1 / 3 },
        { label: "1/16 triplet", beats: 1 / 6 }
    ]
    property int snapIndex: 5
    function step() {
        const b = snapChoices[snapIndex].beats
        return b < 0 ? barBeats : b
    }
    /// Length new notes get: the last one drawn or resized (or the snap step).
    property real lastLength: 1
    onSnapIndexChanged: if (step() > 0) lastLength = Math.max(step(), lastLength < step() ? step() : lastLength)
    /// Snaps a region-relative position to the grid (counted from bar 1).
    function snapRel(rel) {
        const s = step()
        if (!region || s <= 0) return rel
        return Math.round((region.start + rel) / s) * s - region.start
    }
    function floorRel(rel) {
        const s = step()
        if (!region || s <= 0) return rel
        return Math.floor((region.start + rel) / s + 1e-6) * s - region.start
    }
    // ------------------------------------------------------ selection
    /// Selected notes, by index into `notes`.
    property var selection: []
    /// True when the piano roll should get Delete instead of the arrangement.
    readonly property bool ownsKeys: shown && !typing && selection.length > 0
    // Edits elsewhere (undo, recording) can shrink the list.
    onNotesChanged: if (selection.some(i => i >= notes.length)) selection = selection.filter(i => i < notes.length)
    function isSelected(i) { return selection.indexOf(i) >= 0 }
    function toggleSelected(i) {
        selection = isSelected(i) ? selection.filter(x => x !== i) : selection.concat([i])
    }
    function selectAll() { selection = notes.map((_, i) => i) }

    // A group drag in progress: "move" or "resize", and how far (beats, keys).
    property string dragMode: ""
    property real dragDx: 0
    property int dragDk: 0
    property real dragDl: 0
    /// Applies a move/transpose/resize to the selected notes as one edit.
    function transformSelected(dx, dk, dl) {
        if (!selection.length || (dx === 0 && dk === 0 && dl === 0)) return
        const moved = app.transformNotes(regionId, selection.join(","), dx, dk, dl)
        selection = moved === "" ? [] : moved.split(",").map(Number)
    }
    function deleteSelected() {
        if (!selection.length) return
        const picked = selection.join(",")
        selection = []
        app.removeNotes(regionId, picked)
    }
    function nudgeStep() { return step() > 0 ? step() : 0.25 }

    /// The grid quantize uses: the snap step, or 1/16 when snap is off.
    function quantizeGrid() { return step() > 0 ? step() : 0.25 }
    function quantize(strength, ends, swing) {
        if (!region) return
        // Quantizing re-sorts the notes, so the selection would point at the wrong ones.
        const picked = selection.join(",")
        selection = []
        app.quantizeNotes(regionId, picked, quantizeGrid(), strength, ends, swing || 0)
    }

    // ------------------------------------------------------ chords & tools
    /// What a click on the grid draws: a note, or a chord built on it.
    property string drawChord: "note"
    readonly property var chordShapes: ({
        major: [0, 4, 7], minor: [0, 3, 7], seventh: [0, 4, 7, 10], maj7: [0, 4, 7, 11],
        min7: [0, 3, 7, 10], sus2: [0, 2, 7], sus4: [0, 5, 7], power: [0, 7, 12], dim: [0, 3, 6]
    })
    readonly property var chordNames: ({
        note: "Notes", major: "Major chords", minor: "Minor chords", seventh: "7th chords", maj7: "Maj7 chords",
        min7: "Min7 chords", sus2: "Sus2 chords", sus4: "Sus4 chords", power: "Power chords", dim: "Diminished chords",
        key3: "Chords in key", key7: "7th chords in key"
    })
    /// The song's key as pitch classes ("C major", "F♯ minor"…).
    readonly property var scale: {
        const m = (app.keyName || "C major").match(/^([A-Ga-g])([#♯b♭]?)\s*(.*)$/)
        if (!m) return [0, 2, 4, 5, 7, 9, 11]
        let root = { c: 0, d: 2, e: 4, f: 5, g: 7, a: 9, b: 11 }[m[1].toLowerCase()]
        if (m[2] === "#" || m[2] === "♯") root += 1
        if (m[2] === "b" || m[2] === "♭") root -= 1
        const steps = /min/i.test(m[3]) ? [0, 2, 3, 5, 7, 8, 10] : [0, 2, 4, 5, 7, 9, 11]
        return steps.map(s => (s + root + 12) % 12)
    }
    function inScale(key) { return scale.indexOf(key % 12) >= 0 }
    /// The keys a click at `key` draws.
    function chordKeys(key) {
        if (drawChord === "note") return [key]
        if (drawChord === "key3" || drawChord === "key7") {
            // Stack every other note of the scale, starting from the clicked one.
            const pc = key % 12
            const degree = scale.indexOf(pc)
            if (degree < 0) return chordShapes.major.map(i => key + i)
            const count = drawChord === "key7" ? 4 : 3
            const out = []
            for (let i = 0; i < count; ++i) {
                const d = degree + i * 2
                const pcNext = scale[d % 7]
                out.push(key - pc + pcNext + 12 * Math.floor(d / 7))
            }
            return out
        }
        return chordShapes[drawChord].map(i => key + i)
    }
    function strum(down) {
        const picked = selection.join(",")
        const out = app.strumNotes(regionId, picked, Math.max(1 / 32, quantizeGrid() / 4), down)
        selection = out === "" ? [] : out.split(",").map(Number)
    }
    function arpeggiate(pattern) {
        const picked = selection.join(",")
        const out = app.arpeggiateNotes(regionId, picked, quantizeGrid(), pattern)
        selection = out === "" ? [] : out.split(",").map(Number)
    }
    property bool showScale: true
    property bool showGhosts: true
    /// Notes from the other MIDI regions (this track's and others'), placed
    /// on this region's timeline: [[start, length, key]…].
    readonly property var ghosts: {
        if (!showGhosts || !region) return []
        const out = []
        const from = region.start - midi.offset, to = region.start + gridBeats
        for (const t of tracks)
            for (const r of t.regions) {
                if (!r.midi || r.id === region.id || r.start >= to || r.start + r.beats <= region.start) continue
                for (const n of r.midi.notes) {
                    const rel = n[0] - r.midi.offset
                    if (rel < 0 || rel >= r.midi.length) continue
                    const at = r.start + rel - region.start
                    if (at + n[1] > 0 && at < gridBeats && out.length < 1500) out.push([at, n[1], n[2]])
                }
            }
        return out
    }
    function recordHere() {
        if (!track) return
        if (!app.recording) app.selectTrack(track.id)
        app.record()
    }

    /// The note under (x, y) on the grid, or -1.
    function noteAt(x, y) {
        const key = keyAt(y), beat = x / pxPerBeat
        for (let i = notes.length - 1; i >= 0; --i) {
            const n = notes[i], rel = n[0] - midi.offset
            if (n[2] === key && beat >= rel && beat < rel + n[1]) return i
        }
        return -1
    }
    function keyY(key) { return (127 - key) * keyHeight }
    function keyAt(y) { return Math.max(0, Math.min(127, 127 - Math.floor(y / keyHeight))) }
    function isBlack(key) { return [1, 3, 6, 8, 10].indexOf(key % 12) >= 0 }
    function keyName(key) { return ["C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B"][key % 12] + (Math.floor(key / 12) - 1) }

    /// Scrolls so the notes (or middle C) are in view.
    function centre() {
        const keys = notes.map(n => n[2])
        const mid = keys.length ? (Math.min(...keys) + Math.max(...keys)) / 2 : 60
        grid.contentY = Math.max(0, Math.min(grid.contentHeight - grid.height, keyY(mid) - grid.height / 2))
        grid.contentX = 0
    }
    onRegionIdChanged: { selection = []; Qt.callLater(centre) }
    onShownChanged: if (shown) Qt.callLater(centre)

    // Hear a note while clicking or dragging.
    property int sounding: -1
    function preview(key, velocity) {
        if (sounding === key) return
        if (sounding >= 0) app.noteOff(sounding)
        sounding = key
        app.noteOn(key, velocity || 100)
    }
    function stopPreview() {
        if (sounding >= 0) app.noteOff(sounding)
        sounding = -1
    }

    GlassPanel {
        anchors.fill: parent
        backdrop: roll.backdrop
        // Swallow clicks so they don't reach the arrangement underneath.
        MouseArea { anchors.fill: parent; acceptedButtons: Qt.AllButtons; onWheel: w => w.accepted = true }

        // ------------------------------------------------ title bar
        Item {
            id: titleBar
            width: parent.width
            height: 40
            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.SizeVerCursor
                preventStealing: true
                property real startY
                property real startHeight
                onPressed: m => { startY = mapToItem(null, 0, m.y).y; startHeight = roll.height }
                onPositionChanged: m => { if (pressed) roll.dockHeight = startHeight - (mapToItem(null, 0, m.y).y - startY) }
                onDoubleClicked: roll.dockHeight = 420
            }
            Rectangle {
                anchors.horizontalCenter: parent.horizontalCenter
                y: 5
                width: 36; height: 4; radius: 2
                color: Theme.tint(Theme.text, 0.18)
            }
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 14
                anchors.rightMargin: 8
                spacing: 8
                Icon { glyph: Icons.pianoKeys; size: 15; color: roll.tint }
                DText { text: "Piano Roll"; strong: true }
                DText {
                    text: roll.region ? roll.region.name + " · " + roll.track.name : ""
                    faint: true
                    font.pixelSize: Theme.textXs
                }
                DText {
                    text: roll.notes.length + (roll.notes.length === 1 ? " note" : " notes")
                          + (roll.selection.length ? " · " + roll.selection.length + " selected" : "")
                    faint: true
                    font.pixelSize: Theme.textXs
                }
                Item { Layout.fillWidth: true }
                AbstractButton {
                    id: toolsButton
                    implicitWidth: tRow.implicitWidth + 16
                    implicitHeight: 28
                    hoverEnabled: true
                    onClicked: toolsMenu.opened ? toolsMenu.close() : toolsMenu.open()
                    background: Rectangle {
                        radius: Theme.radiusMd
                        color: toolsMenu.opened ? Theme.hover : roll.drawChord !== "note" ? Theme.tint(roll.tint, 0.18) : toolsButton.hovered ? Theme.raised : "transparent"
                        border.color: roll.drawChord !== "note" ? Theme.tint(roll.tint, 0.6) : Theme.line
                    }
                    contentItem: Item {
                        Row {
                            id: tRow
                            anchors.centerIn: parent
                            spacing: 5
                            Icon { glyph: Icons.musicNotes; size: 13; color: roll.tint; anchors.verticalCenter: parent.verticalCenter }
                            DText { text: roll.drawChord === "note" ? "Tools" : roll.chordNames[roll.drawChord]; font.pixelSize: Theme.textSm; anchors.verticalCenter: parent.verticalCenter }
                            Icon { glyph: Icons.caretDown; size: 10; color: Theme.textFaint; anchors.verticalCenter: parent.verticalCenter }
                        }
                    }
                    DToolTip { visible: toolsButton.hovered && !toolsMenu.opened; text: "Draw chords, strum, arpeggiate, and show the key and other regions' notes" }
                    MenuPopup {
                        id: toolsMenu
                        y: parent.height + 4
                        width: 270
                        items: {
                            const tick = k => (roll.drawChord === k ? "✓  " : "     ") + roll.chordNames[k]
                            const draw = k => ({ label: tick(k), action: () => roll.drawChord = k })
                            const sel = roll.selection.length ? "SELECTED" : "ALL"
                            return [
                                { header: "A CLICK DRAWS" },
                                draw("note"), draw("key3"), draw("key7"), draw("major"), draw("minor"),
                                draw("seventh"), draw("maj7"), draw("min7"), draw("sus4"), draw("power"),
                                { separator: true },
                                { header: "CHORDS (" + sel + " NOTES)" },
                                { label: "Strum up", enabled: roll.notes.length > 1, action: () => roll.strum(false) },
                                { label: "Strum down", enabled: roll.notes.length > 1, action: () => roll.strum(true) },
                                { label: "Arpeggiate up", enabled: roll.notes.length > 1, action: () => roll.arpeggiate("up") },
                                { label: "Arpeggiate down", enabled: roll.notes.length > 1, action: () => roll.arpeggiate("down") },
                                { label: "Arpeggiate up and down", enabled: roll.notes.length > 1, action: () => roll.arpeggiate("updown") },
                                { separator: true },
                                { header: "SHOW" },
                                { label: (roll.showScale ? "✓  " : "     ") + "Notes in " + (roll.app.keyName || "the key"), action: () => roll.showScale = !roll.showScale },
                                { label: (roll.showGhosts ? "✓  " : "     ") + "Other regions' notes", action: () => roll.showGhosts = !roll.showGhosts }
                            ]
                        }
                    }
                }
                Icon { glyph: Icons.magnet; size: 14; color: roll.step() > 0 ? roll.tint : Theme.textFaint }
                Dropdown {
                    implicitWidth: 128
                    implicitHeight: 28
                    font.pixelSize: Theme.textSm
                    model: roll.snapChoices.map(c => c.label)
                    currentIndex: roll.snapIndex
                    onActivated: i => roll.snapIndex = i
                    DToolTip { visible: parent.hovered && !parent.popup.visible; text: "Snap notes to" }
                }
                AbstractButton {
                    id: quantizeButton
                    implicitWidth: qRow.implicitWidth + 16
                    implicitHeight: 28
                    hoverEnabled: true
                    enabled: roll.notes.length > 0
                    onClicked: quantizeMenu.opened ? quantizeMenu.close() : quantizeMenu.open()
                    background: Rectangle {
                        radius: Theme.radiusMd
                        color: quantizeButton.down || quantizeMenu.opened ? Theme.hover : quantizeButton.hovered ? Theme.raised : "transparent"
                        border.color: Theme.line
                    }
                    contentItem: Item {
                        Row {
                            id: qRow
                            anchors.centerIn: parent
                            spacing: 5
                            Icon { glyph: Icons.magicWand; size: 13; color: quantizeButton.enabled ? roll.tint : Theme.textFaint; anchors.verticalCenter: parent.verticalCenter }
                            DText { text: "Quantize"; font.pixelSize: Theme.textSm; faint: !quantizeButton.enabled; anchors.verticalCenter: parent.verticalCenter }
                            Icon { glyph: Icons.caretDown; size: 10; color: Theme.textFaint; anchors.verticalCenter: parent.verticalCenter }
                        }
                    }
                    DToolTip { visible: quantizeButton.hovered && !quantizeMenu.opened; text: "Line notes up with the grid (Q)" }
                    MenuPopup {
                        id: quantizeMenu
                        y: parent.height + 4
                        width: 250
                        items: [
                            { header: (roll.selection.length ? "SELECTED NOTES" : "ALL NOTES") + " TO " + (roll.step() > 0 ? roll.snapChoices[roll.snapIndex].label.toUpperCase() : "1/16") },
                            { label: "Note starts", shortcut: "Q", action: () => roll.quantize(1, false) },
                            { label: "Starts and ends", shortcut: "⇧Q", action: () => roll.quantize(1, true) },
                            { label: "Halfway (keeps the feel)", action: () => roll.quantize(0.5, false) },
                            { label: "A little (25%)", action: () => roll.quantize(0.25, false) },
                            { separator: true },
                            { header: "SWING (EVERY SECOND STEP LATE)" },
                            { label: "Light swing (55%)", action: () => roll.quantize(1, false, 0.1) },
                            { label: "Swing (60%)", action: () => roll.quantize(1, false, 0.2) },
                            { label: "Heavy swing (66%, triplet feel)", action: () => roll.quantize(1, false, 1 / 3) }
                        ]
                    }
                }
                Rectangle { Layout.preferredWidth: 1; Layout.preferredHeight: 18; color: Theme.line }
                AbstractButton {
                    id: recButton
                    implicitWidth: 28
                    implicitHeight: 28
                    hoverEnabled: true
                    enabled: roll.app.audioOk
                    onClicked: roll.recordHere()
                    background: Rectangle {
                        radius: width / 2
                        color: roll.app.recording ? Theme.tint(Theme.record, 0.25) : recButton.hovered ? Theme.raised : "transparent"
                    }
                    contentItem: Item {
                        Rectangle {
                            anchors.centerIn: parent
                            width: 11; height: 11; radius: 5.5
                            color: recButton.enabled ? Theme.record : Theme.textFaint
                            SequentialAnimation on opacity {
                                running: roll.app.recording && Theme.motionMs > 0
                                loops: Animation.Infinite
                                NumberAnimation { from: 1; to: 0.35; duration: 600 }
                                NumberAnimation { from: 0.35; to: 1; duration: 600 }
                                onRunningChanged: if (!running) parent.opacity = 1
                            }
                        }
                    }
                    DToolTip {
                        visible: recButton.hovered
                        text: roll.app.recording ? "Stop recording" : "Record notes onto this track from a MIDI keyboard or the computer keyboard (R)"
                    }
                }
                Rectangle { Layout.preferredWidth: 1; Layout.preferredHeight: 18; color: Theme.line }
                IconButton {
                    implicitWidth: 26; implicitHeight: 26; iconSize: 14
                    glyph: Icons.zoomOut
                    tip: "Zoom out"
                    onClicked: roll.pxPerBeat = Math.max(12, roll.pxPerBeat / 1.4)
                }
                IconButton {
                    implicitWidth: 26; implicitHeight: 26; iconSize: 14
                    glyph: Icons.zoomIn
                    tip: "Zoom in"
                    onClicked: roll.pxPerBeat = Math.min(240, roll.pxPerBeat * 1.4)
                }
                Rectangle { Layout.preferredWidth: 1; Layout.preferredHeight: 18; color: Theme.line }
                IconButton {
                    implicitWidth: 30; implicitHeight: 26; iconSize: 15
                    glyph: Icons.keyboard
                    active: roll.typing
                    tip: roll.typing ? "Computer keyboard plays notes (A W S E D…, Z/X octave). Click to stop."
                                     : "Play notes from the computer keyboard"
                    onClicked: { roll.typing = !roll.typing; if (roll.typing) typingKeys.forceActiveFocus() }
                }
                IconButton {
                    implicitWidth: 30; implicitHeight: 26; iconSize: 15
                    glyph: Icons.sliders
                    active: roll.showInstrument
                    tip: roll.showInstrument ? "Hide the instrument's controls" : "Show the instrument's controls"
                    onClicked: roll.showInstrument = !roll.showInstrument
                }
                IconButton {
                    implicitWidth: 26; implicitHeight: 26; iconSize: 14
                    glyph: Icons.x
                    tip: "Close the piano roll (F7)"
                    onClicked: { roll.typing = false; roll.closeRequested() }
                }
            }
            Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.line }
        }

        RowLayout {
            anchors.fill: parent
            anchors.topMargin: titleBar.height + 6
            anchors.margins: 10
            spacing: 8

            // ------------------------------------------------ keyboard + grid
            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true

                // Keyboard (scrolls with the grid vertically)
                Item {
                    id: keyboard
                    width: roll.keyboardWidth
                    height: grid.height
                    clip: true
                    Item {
                        y: -grid.contentY
                        width: parent.width
                        height: 128 * roll.keyHeight
                        Repeater {
                            model: 128
                            delegate: Rectangle {
                                required property int index
                                readonly property int key: 127 - index
                                readonly property bool black: roll.isBlack(key)
                                y: index * roll.keyHeight
                                width: black ? keyboard.width * 0.62 : keyboard.width
                                height: roll.keyHeight
                                readonly property bool lit: roll.sounding === key || (roll.listening && roll.activity.lit(key))
                                color: lit ? roll.tint : black ? "#1A1A1D" : "#ECECEF"
                                Behavior on color { ColorAnimation { duration: 90 } }
                                border.color: Theme.tint("#000000", 0.25)
                                DText {
                                    visible: parent.key % 12 === 0
                                    anchors.right: parent.right; anchors.rightMargin: 4
                                    anchors.verticalCenter: parent.verticalCenter
                                    text: roll.keyName(parent.key)
                                    font.pixelSize: 9
                                    color: "#444448"
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    onPressed: roll.preview(parent.key, 100)
                                    onReleased: roll.stopPreview()
                                }
                            }
                        }
                    }
                }

                Flickable {
                    id: grid
                    x: roll.keyboardWidth + 2
                    width: parent.width - x
                    height: parent.height - velocity.height - 6
                    contentWidth: Math.max(width, roll.gridBeats * roll.pxPerBeat)
                    contentHeight: 128 * roll.keyHeight
                    clip: true
                    boundsBehavior: Flickable.StopAtBounds
                    flickableDirection: Flickable.HorizontalAndVerticalFlick
                    ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }
                    ScrollBar.horizontal: ScrollBar { policy: ScrollBar.AsNeeded }

                    // Rows: darker for black keys, a line under each C.
                    Repeater {
                        model: 128
                        delegate: Rectangle {
                            required property int index
                            readonly property int key: 127 - index
                            y: index * roll.keyHeight
                            width: grid.contentWidth
                            height: roll.keyHeight
                            color: roll.showScale && roll.inScale(key) ? Theme.tint(roll.tint, roll.isBlack(key) ? 0.10 : 0.07)
                                   : roll.isBlack(key) ? Theme.tint("#000000", Theme.dark ? 0.28 : 0.08) : "transparent"
                            Rectangle {
                                visible: parent.key % 12 === 0
                                anchors.bottom: parent.bottom
                                width: parent.width; height: 1
                                color: Theme.tint(Theme.lineStrong, 0.8)
                            }
                        }
                    }
                    // Beat and bar lines (visible part only).
                    Repeater {
                        model: Math.ceil(grid.width / roll.pxPerBeat) + 2
                        delegate: Rectangle {
                            required property int index
                            readonly property int beat: Math.floor(grid.contentX / roll.pxPerBeat) + index
                            readonly property bool bar: TimeMap.isBarLine((roll.region ? roll.region.start : 0) + beat - (roll.midi ? roll.midi.offset : 0))
                            x: beat * roll.pxPerBeat
                            width: 1
                            height: grid.contentHeight
                            color: bar ? Theme.tint(Theme.lineStrong, 0.9) : Theme.tint(Theme.line, 0.9)
                        }
                    }
                    // Past the region's end
                    Rectangle {
                        x: roll.lengthBeats * roll.pxPerBeat
                        width: grid.contentWidth - x
                        height: grid.contentHeight
                        color: Theme.tint("#000000", 0.25)
                        Rectangle { width: 2; height: parent.height; color: roll.tint; opacity: 0.6 }
                    }

                    // Ghost notes: other regions' notes, faint, behind everything.
                    Repeater {
                        model: roll.ghosts
                        delegate: Rectangle {
                            required property var modelData
                            x: modelData[0] * roll.pxPerBeat
                            y: roll.keyY(modelData[2]) + 2
                            width: Math.max(3, modelData[1] * roll.pxPerBeat - 1)
                            height: roll.keyHeight - 4
                            radius: 2
                            color: Theme.tint(Theme.text, 0.10)
                            border.color: Theme.tint(Theme.text, 0.22)
                        }
                    }

                    // Empty grid: left-click adds a note; right-click (and right-drag)
                    // erases notes. Right-clicks on a note fall through to here.
                    MouseArea {
                        id: gridMouse
                        width: grid.contentWidth
                        height: grid.contentHeight
                        preventStealing: true
                        acceptedButtons: Qt.LeftButton | Qt.RightButton
                        cursorShape: erasing ? Qt.ForbiddenCursor : boxing ? Qt.ArrowCursor : Qt.CrossCursor
                        property bool erasing: false
                        // Selection box (Shift- or ⌘/Ctrl-drag).
                        property bool boxing: false
                        property real bx0
                        property real by0
                        property real bx1
                        property real by1
                        property var before: []
                        function erase(x, y) {
                            const i = roll.noteAt(x, y)
                            if (i < 0) return
                            roll.selection = roll.selection.filter(s => s !== i).map(s => s > i ? s - 1 : s)
                            roll.app.removeNote(roll.regionId, i)
                        }
                        function boxSelect() {
                            const x0 = Math.min(bx0, bx1) / roll.pxPerBeat, x1 = Math.max(bx0, bx1) / roll.pxPerBeat
                            const y0 = Math.min(by0, by1), y1 = Math.max(by0, by1)
                            const hit = []
                            roll.notes.forEach((n, i) => {
                                const rel = n[0] - roll.midi.offset, top = roll.keyY(n[2])
                                if (rel < x1 && rel + n[1] > x0 && top < y1 && top + roll.keyHeight > y0) hit.push(i)
                            })
                            roll.selection = before.concat(hit.filter(i => before.indexOf(i) < 0))
                        }
                        onPressed: m => {
                            if (m.button === Qt.RightButton) {
                                erasing = true
                                erase(m.x, m.y)
                                return
                            }
                            if (m.modifiers & (Qt.ShiftModifier | Qt.ControlModifier)) {
                                boxing = true
                                bx0 = bx1 = m.x
                                by0 = by1 = m.y
                                before = roll.selection
                                return
                            }
                            // A click on empty grid clears the selection and adds a note (or a chord).
                            roll.selection = []
                            const key = roll.keyAt(m.y)
                            const start = Math.max(0, roll.floorRel(m.x / roll.pxPerBeat))
                            roll.preview(key, 100)
                            if (roll.drawChord === "note") {
                                roll.app.addNote(roll.regionId, start, roll.lastLength, key, 100)
                            } else {
                                const chord = roll.chordKeys(key).filter(k => k <= 127).map(k => [start, roll.lastLength, k, 100])
                                const added = roll.app.addNotes(roll.regionId, JSON.stringify(chord))
                                roll.selection = added === "" ? [] : added.split(",").map(Number)
                            }
                        }
                        onPositionChanged: m => {
                            if (erasing) erase(m.x, m.y)
                            else if (boxing) { bx1 = m.x; by1 = m.y; boxSelect() }
                        }
                        onReleased: { erasing = false; boxing = false; roll.stopPreview() }
                    }

                    // The selection box
                    Rectangle {
                        visible: gridMouse.boxing
                        x: Math.min(gridMouse.bx0, gridMouse.bx1)
                        y: Math.min(gridMouse.by0, gridMouse.by1)
                        width: Math.abs(gridMouse.bx1 - gridMouse.bx0)
                        height: Math.abs(gridMouse.by1 - gridMouse.by0)
                        z: 4
                        color: Theme.tint(roll.tint, 0.14)
                        border.color: roll.tint
                        radius: 2
                    }

                    // Notes
                    Repeater {
                        model: roll.notes.length
                        delegate: NoteBox {
                            required property int index
                            noteIndex: index
                            note: roll.notes[index]
                        }
                    }

                    // Playhead
                    Rectangle {
                        visible: roll.region && roll.app.positionBeats >= roll.region.start
                        x: roll.region ? (roll.app.positionBeats - roll.region.start) * roll.pxPerBeat : 0
                        width: 2
                        height: grid.contentHeight
                        color: Theme.playhead
                        opacity: 0.9
                    }
                }

                // Velocity lane
                Item {
                    id: velocity
                    x: grid.x
                    width: grid.width
                    anchors.bottom: parent.bottom
                    height: 54
                    clip: true
                    Rectangle {
                        anchors.fill: parent
                        radius: Theme.radiusSm
                        color: Theme.tint(Theme.sunken, 0.5)
                        border.color: Theme.tint(Theme.line, 0.8)
                    }
                    DText {
                        x: -grid.x + 4
                        anchors.verticalCenter: parent.verticalCenter
                        text: "VEL"
                        font.pixelSize: 9
                        faint: true
                    }
                    Item {
                        x: -grid.contentX
                        width: grid.contentWidth
                        height: parent.height
                        Repeater {
                            model: roll.notes.length
                            delegate: Item {
                                id: bar
                                required property int index
                                readonly property var note: roll.notes[index]
                                property int live: -1
                                readonly property int vel: live >= 0 ? live : note[3]
                                x: (note[0] - roll.midi.offset) * roll.pxPerBeat
                                width: 9
                                height: velocity.height
                                Rectangle {
                                    anchors.bottom: parent.bottom
                                    anchors.bottomMargin: 3
                                    x: 1; width: 3
                                    height: (velocity.height - 8) * bar.vel / 127
                                    radius: 1.5
                                    color: roll.tint
                                    Rectangle {
                                        anchors.horizontalCenter: parent.horizontalCenter
                                        width: 8; height: 8; radius: 4
                                        y: -4
                                        color: roll.tint
                                    }
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    anchors.leftMargin: -3; anchors.rightMargin: -3
                                    preventStealing: true
                                    cursorShape: Qt.SizeVerCursor
                                    function velAt(y) { return Math.round(Math.max(1, Math.min(127, (1 - (y - 3) / (velocity.height - 8)) * 127))) }
                                    onPressed: m => bar.live = velAt(m.y)
                                    onPositionChanged: m => { if (pressed) bar.live = velAt(m.y) }
                                    onReleased: {
                                        const n = bar.note, vel = bar.live, i = bar.index
                                        bar.live = -1
                                        if (vel !== n[3])
                                            roll.app.updateNote(roll.regionId, i, n[0] - roll.midi.offset, n[1], n[2], vel)
                                    }
                                }
                                DToolTip { visible: bar.live >= 0; delay: 0; text: "Velocity " + bar.vel }
                            }
                        }
                    }
                }
            }

            // ------------------------------------------------ instrument
            Rectangle {
                Layout.preferredWidth: 1
                Layout.fillHeight: true
                visible: roll.showInstrument
                color: Theme.line
            }
            ColumnLayout {
                id: instrumentPanel
                visible: roll.showInstrument && !!roll.track && !!roll.track.instrument
                Layout.preferredWidth: 244
                Layout.maximumWidth: 244
                Layout.fillHeight: true
                spacing: 6
                readonly property var def: {
                    try {
                        const kinds = JSON.parse(roll.app.instrumentKindsJson || "[]")
                        return roll.track && roll.track.instrument ? kinds.find(k => k.key === roll.track.instrument.kind) : null
                    } catch (e) { return null }
                }
                RowLayout {
                    spacing: 6
                    Icon { glyph: Icons.waveform; size: 14; color: roll.tint }
                    DText {
                        text: roll.track && roll.track.instrument ? roll.track.instrument.label : ""
                        strong: true
                        font.pixelSize: Theme.textSm
                    }
                }
                DButton {
                    visible: !!roll.track && !!roll.track.instrument
                    Layout.fillWidth: true
                    implicitHeight: 30
                    glyph: roll.track && roll.track.instrument && roll.track.instrument.kind === "synth" ? Icons.waveform : Icons.pianoKeys
                    text: roll.track && roll.track.instrument ? "Open " + roll.track.instrument.label : ""
                    onClicked: roll.instrumentRequested(roll.track.id)
                }
                DText {
                    Layout.fillWidth: true
                    text: roll.app.midiInputs !== "" ? "MIDI in: " + roll.app.midiInputs : "No MIDI keyboard found. Use Typing, or plug one in and choose Look for MIDI keyboards in ⌘K."
                    wrapMode: Text.WordWrap
                    faint: true
                    font.pixelSize: Theme.textXs
                }
                Flickable {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    contentHeight: knobs.implicitHeight
                    clip: true
                    boundsBehavior: Flickable.StopAtBounds
                    GridLayout {
                        id: knobs
                        width: parent.width
                        columns: 4
                        columnSpacing: 0
                        rowSpacing: 4
                        Repeater {
                            model: instrumentPanel.def ? instrumentPanel.def.params : []
                            delegate: ParamKnob {
                                required property var modelData
                                Layout.alignment: Qt.AlignHCenter
                                def: modelData
                                value: roll.track.instrument.values[modelData.key]
                                tint: roll.tint
                                onMoved: (v, done) => roll.app.setInstrumentParam(roll.track.id, modelData.key, v, done)
                            }
                        }
                    }
                }
            }
        }
    }

    // Computer keyboard as a piano (while "Typing" is on).
    property int octave: 4
    readonly property var typingMap: ({
        [Qt.Key_A]: 0, [Qt.Key_W]: 1, [Qt.Key_S]: 2, [Qt.Key_E]: 3, [Qt.Key_D]: 4, [Qt.Key_F]: 5,
        [Qt.Key_T]: 6, [Qt.Key_G]: 7, [Qt.Key_Y]: 8, [Qt.Key_H]: 9, [Qt.Key_U]: 10, [Qt.Key_J]: 11,
        [Qt.Key_K]: 12, [Qt.Key_O]: 13, [Qt.Key_L]: 14, [Qt.Key_P]: 15
    })
    Item {
        id: typingKeys
        focus: roll.typing
        Keys.onPressed: e => {
            if (!roll.typing || e.isAutoRepeat) { e.accepted = roll.typing; return }
            if (e.key === Qt.Key_R) roll.recordHere()
            else if (e.key === Qt.Key_Space) roll.app.togglePlay()
            else if (e.key === Qt.Key_Z) roll.octave = Math.max(0, roll.octave - 1)
            else if (e.key === Qt.Key_X) roll.octave = Math.min(8, roll.octave + 1)
            else if (roll.typingMap[e.key] !== undefined) roll.app.noteOn((roll.octave + 1) * 12 + roll.typingMap[e.key], 100)
            e.accepted = true
        }
        Keys.onReleased: e => {
            if (!roll.typing || e.isAutoRepeat) return
            if (roll.typingMap[e.key] !== undefined) roll.app.noteOff((roll.octave + 1) * 12 + roll.typingMap[e.key])
            e.accepted = true
        }
    }

    // One note: click to select, drag to move (with the rest of the
    // selection), drag the right edge to resize.
    component NoteBox: Rectangle {
        id: box
        property int noteIndex
        property var note
        readonly property real rel: note[0] - roll.midi.offset
        readonly property bool outside: rel < 0 || rel >= roll.lengthBeats
        readonly property bool selected: roll.selection.indexOf(noteIndex) >= 0
        // Where the note shows during a group drag.
        readonly property bool moving: selected && roll.dragMode !== ""
        readonly property real shownStart: rel + (moving && roll.dragMode === "move" ? roll.dragDx : 0)
        readonly property real shownLength: Math.max(1 / 64, note[1] + (moving && roll.dragMode === "resize" ? roll.dragDl : 0))
        readonly property int shownKey: Math.max(0, Math.min(127, note[2] + (moving && roll.dragMode === "move" ? roll.dragDk : 0)))

        x: shownStart * roll.pxPerBeat
        y: roll.keyY(shownKey) + 1
        width: Math.max(4, shownLength * roll.pxPerBeat - 1)
        height: roll.keyHeight - 2
        radius: 3
        color: selected ? Qt.lighter(roll.tint, 1.35)
                        : Qt.rgba(roll.tint.r, roll.tint.g, roll.tint.b, 0.45 + 0.55 * note[3] / 127)
        border.width: selected ? 1.5 : 1
        border.color: selected ? "#FFFFFF" : Qt.darker(roll.tint, 1.6)
        opacity: outside ? 0.35 : 1
        z: moving ? 3 : 2

        MouseArea {
            anchors.fill: parent
            preventStealing: true
            hoverEnabled: true
            // Left only: right-clicks reach the grid underneath, which erases.
            acceptedButtons: Qt.LeftButton
            property string mode: ""
            property real pressX
            property real pressY
            cursorShape: mouseX > box.width - 6 ? Qt.SizeHorCursor : pressed ? Qt.ClosedHandCursor : Qt.OpenHandCursor
            onPressed: m => {
                mode = ""
                if (m.modifiers & Qt.AltModifier) {
                    const i = box.noteIndex
                    roll.selection = roll.selection.filter(s => s !== i).map(s => s > i ? s - 1 : s)
                    roll.app.removeNote(roll.regionId, i)
                    return
                }
                if (m.modifiers & (Qt.ShiftModifier | Qt.ControlModifier)) {
                    roll.toggleSelected(box.noteIndex)
                    // Unselecting a note doesn't start a drag.
                    if (!roll.isSelected(box.noteIndex)) return
                } else if (!box.selected) {
                    roll.selection = [box.noteIndex]
                }
                mode = m.x > box.width - 6 ? "resize" : "move"
                const p = mapToItem(grid.contentItem, m.x, m.y)
                pressX = p.x; pressY = p.y
                roll.preview(box.note[2], box.note[3])
            }
            onPositionChanged: m => {
                if (!pressed || mode === "") return
                const p = mapToItem(grid.contentItem, m.x, m.y)
                if (roll.dragMode === "" && Math.abs(p.x - pressX) < 3 && Math.abs(p.y - pressY) < 3) return
                roll.dragMode = mode
                const dx = (p.x - pressX) / roll.pxPerBeat
                if (mode === "move") {
                    // Snap the grabbed note; the rest of the selection keeps its spacing.
                    const d = roll.snapRel(box.rel + dx) - box.rel
                    const earliest = Math.min(...roll.selection.map(i => roll.notes[i][0] - roll.midi.offset))
                    roll.dragDx = Math.max(d, -earliest)
                    roll.dragDk = -Math.round((p.y - pressY) / roll.keyHeight)
                    roll.preview(Math.max(0, Math.min(127, box.note[2] + roll.dragDk)), box.note[3])
                } else {
                    const end = roll.snapRel(box.rel + box.note[1] + dx)
                    const len = Math.max(roll.step() > 0 ? roll.step() : 1 / 32, end - box.rel)
                    roll.dragDl = len - box.note[1]
                }
            }
            onReleased: {
                roll.stopPreview()
                const dragged = roll.dragMode, dx = roll.dragDx, dk = roll.dragDk, dl = roll.dragDl
                const newLength = box.note[1] + dl
                roll.dragMode = ""
                roll.dragDx = 0; roll.dragDk = 0; roll.dragDl = 0
                mode = ""
                if (dragged === "") return
                if (dragged === "resize") roll.lastLength = newLength
                roll.transformSelected(dragged === "move" ? dx : 0, dragged === "move" ? dk : 0, dragged === "resize" ? dl : 0)
            }
        }
    }

    // Keys for the selection (not while the computer keyboard plays notes).
    readonly property bool editKeys: shown && !typing
    Shortcut { sequences: ["Backspace", "Delete"]; enabled: roll.ownsKeys; onActivated: roll.deleteSelected() }
    Shortcut { sequences: [StandardKey.SelectAll]; enabled: roll.editKeys && roll.notes.length > 0; onActivated: roll.selectAll() }
    Shortcut { sequence: "Escape"; enabled: roll.ownsKeys; onActivated: roll.selection = [] }
    Shortcut { sequence: "Left"; enabled: roll.ownsKeys; onActivated: roll.transformSelected(-roll.nudgeStep(), 0, 0) }
    Shortcut { sequence: "Right"; enabled: roll.ownsKeys; onActivated: roll.transformSelected(roll.nudgeStep(), 0, 0) }
    Shortcut { sequence: "Up"; enabled: roll.ownsKeys; onActivated: roll.transformSelected(0, 1, 0) }
    Shortcut { sequence: "Down"; enabled: roll.ownsKeys; onActivated: roll.transformSelected(0, -1, 0) }
    Shortcut { sequence: "Shift+Up"; enabled: roll.ownsKeys; onActivated: roll.transformSelected(0, 12, 0) }
    Shortcut { sequence: "Shift+Down"; enabled: roll.ownsKeys; onActivated: roll.transformSelected(0, -12, 0) }

    Shortcut { sequence: "Q"; enabled: roll.shown && !roll.typing && roll.notes.length > 0; onActivated: roll.quantize(1, false) }
    Shortcut { sequence: "Shift+Q"; enabled: roll.shown && !roll.typing && roll.notes.length > 0; onActivated: roll.quantize(1, true) }
}
