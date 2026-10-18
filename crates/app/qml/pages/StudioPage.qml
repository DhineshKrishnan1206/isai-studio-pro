import QtCore
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Dialogs
import QtQuick.Layouts
import Isai

// The main workspace. Without an open project it shows the project chooser;
// with one, the control bar, arrangement and status bar.
Item {
    id: studio
    property var app
    /// The chooser opened on top of an open project ("New project…").
    property bool choosing: false
    /// The floating mixer (X).
    property bool mixerShown: false
    onMixerShownChanged: if (mixerShown) editorShown = false
    /// The piano roll and the MIDI region it edits.
    property bool editorShown: false
    property int editorRegion: -1
    /// The floating tuner.
    property bool tunerShown: false
    /// The sound browser on the left.
    property bool browserShown: false
    /// The Inspector: the selected track's channel, down the left side (I).
    property bool inspectorShown: true
    /// The tempo and time-signature lanes on the ruler (G).
    property bool globalShown: false
    Settings {
        category: "studio"
        property alias inspectorShown: studio.inspectorShown
    }
    onEditorShownChanged: if (editorShown) mixerShown = false
    function openEditor(regionId) { editorRegion = regionId; editorShown = true }

    function openEffectEditor(id, chain) { fxEditId = id; fxEditChain = chain; fxEditTint = Theme.accent }
    // The effect editor window: which effect, and where it is.
    property int fxEditId: -1
    property string fxEditChain: ""
    property color fxEditTint: Theme.accent
    /// The effect being edited, looked up again whenever the project changes.
    readonly property var fxEdit: {
        if (fxEditId < 0) return null
        const find = list => (list || []).find(i => i.id === fxEditId)
        for (const t of arrangement.tracks) { const f = find(t.inserts); if (f) return f }
        try {
            for (const b of JSON.parse(studio.app.busesJson || "[]")) { const f = find(b.inserts); if (f) return f }
            return find(JSON.parse(studio.app.masterInsertsJson || "[]")) || null
        } catch (e) { return null }
    }
    /// Just the tracks: closes the piano roll and the mixer (F5).
    function showPlaylist() { editorShown = false; mixerShown = false }

    /// F7 / the piano-roll button: open the selected MIDI region; else the
    /// instrument track's region under the playhead; else make a 4-bar region
    /// there (and an instrument track first, if there is none).
    function togglePianoRoll() {
        if (editorShown) { editorShown = false; return }
        const a = studio.app
        let tracks = arrangement.tracks
        for (const t of tracks) {
            const r = t.regions.find(x => x.id === a.selectedRegion && x.midi)
            if (r) { openEditor(r.id); return }
        }
        let track = tracks.find(t => t.id === a.selectedTrack && t.instrument) || tracks.find(t => t.instrument)
        if (!track) {
            a.addInstrumentTrack("piano")
            tracks = JSON.parse(a.tracksJson)
            track = tracks[tracks.length - 1]
        }
        a.selectTrack(track.id)
        const pos = a.positionBeats
        const here = track.regions.find(x => x.midi && x.start <= pos && pos < x.start + x.beats)
        if (here) { openEditor(here.id); return }
        const bar = TimeMap.barAt(pos)
        const id = a.addMidiRegion(track.id, TimeMap.barStart(bar), TimeMap.barStart(bar + 4) - TimeMap.barStart(bar))
        if (id >= 0) openEditor(id)
    }
    function openEffect(id) { mixerPanel.openInsert(id) }
    // Which keys the selected instrument is playing, read once and shared by
    // the piano window and the piano roll so their keys light up.
    QtObject {
        id: keyActivity
        property var held: ({})
        /// When each key was last struck (so quick notes still flash).
        property var flash: ({})
        /// How hard each key was last played.
        property var vel: ({})
        property real now: 0
        signal ticked()
        function lit(key) { return !!held[key] || (flash[key] !== undefined && now - flash[key] < 140) }
    }
    Timer {
        running: studio.app.projectOpen && (pianoWindow.shown || synthWindow.shown || drumWindow.shown || samplerWindow.shown || studio.editorShown)
        interval: 33
        repeat: true
        property string lastHeld: ""
        onTriggered: {
            let k
            try { k = JSON.parse(studio.app.instrumentKeys() || "{}") } catch (e) { return }
            const t = Date.now()
            const heldKey = (k.held || []).join(",")
            if (heldKey !== lastHeld) {
                lastHeld = heldKey
                const h = {}
                for (const n of k.held || []) h[n] = true
                keyActivity.held = h
            }
            if ((k.struck || []).length) {
                const f = Object.assign({}, keyActivity.flash)
                for (const n of k.struck) f[n] = t
                keyActivity.flash = f
            }
            if (k.vel && Object.keys(k.vel).length) keyActivity.vel = Object.assign({}, keyActivity.vel, k.vel)
            keyActivity.now = t
            keyActivity.ticked()
        }
    }

    /// The track whose instrument window is open (-1: none).
    property int instTrackId: -1
    readonly property var instTrack: {
        const t = instTrackId >= 0 ? arrangement.tracks.find(x => x.id === instTrackId) : null
        return t && t.instrument ? t : null
    }
    readonly property var instLevel: {
        const i = instTrack ? arrangement.tracks.indexOf(instTrack) : -1
        try { return i >= 0 ? (JSON.parse(app.trackMeters || "[]")[i] || [0, 0]) : [0, 0] } catch (e) { return [0, 0] }
    }
    /// Opens a track's instrument window (piano or synth).
    function openInstrument(trackId) {
        app.selectTrack(trackId)
        const t = arrangement.tracks.find(x => x.id === trackId)
        // A plugin instrument has its own window.
        if (t && t.instrument && t.instrument.plugin) { app.openPlugin(t.instrument.id); return }
        if (t && t.instrument) instTrackId = trackId
    }
    // A plugin effect opens its own window instead of Isai's editor.
    onFxEditChanged: if (fxEdit && fxEdit.plugin) { const id = fxEditId; fxEditId = -1; app.openPlugin(id) }
    // While plugins are being scanned or their windows are open, keep them going.
    Timer {
        interval: 100
        repeat: true
        running: studio.app.pluginScanning || studio.app.pluginWindows > 0
        onTriggered: studio.app.pollPlugins()
    }
    /// For dev scenes.
    readonly property Item roll: pianoRoll
    function openAddEffectMenu() { mixerPanel.openAddMenu() }
    // After recording notes with the piano roll open, show the new recording in it.
    Connections {
        target: studio.app
        function onRecordingChanged() {
            if (studio.app.recording || !studio.editorShown) return
            for (const t of arrangement.tracks) {
                const r = t.regions.find(x => x.id === studio.app.selectedRegion && x.midi)
                if (r) { studio.openEditor(r.id); return }
            }
        }
    }
    signal settingsRequested()

    function importAudio() { fileDialog.open() }
    function zoomForTest(factor) { arrangement.zoomBy(factor, 0) }
    function openPalette(query) { palette.open(); palette.setQuery(query || "") }

    // ------------------------------------------------------- file actions
    // Every action that would close the current project goes through guard(),
    // which asks to save unsaved work first. Actions: "new", "open",
    // "quit", "recover", "recent:<path>", "url:<file url>".
    signal quitApproved()
    property string pendingAction: ""
    property string afterSaveAction: ""

    function guard(action) {
        if (app.projectOpen && app.dirty) {
            pendingAction = action
            confirm.open()
        } else {
            run(action)
        }
    }

    function run(action) {
        if (action === "new") choosing = true
        else if (action === "open") openDialog.open()
        else if (action === "quit") quitApproved()
        else if (action === "recover") app.recoverAutosave()
        else if (action.startsWith("recent:")) app.openRecent(action.slice(7))
        else if (action.startsWith("url:")) app.openProject(action.slice(4))
    }

    /// Save; asks for a location the first time. Runs `then` after saving.
    function save(then) {
        if (app.saveProject()) {
            if (then) run(then)
        } else {
            afterSaveAction = then || ""
            saveDialog.open()
        }
    }

    function saveAs() {
        afterSaveAction = ""
        saveDialog.open()
    }

    ExportDialog {
        id: exportDialog
        app: studio.app
        backdrop: studioContent
    }
    function exportAudio() { exportDialog.open() }

    ConfirmDialog {
        id: confirm
        backdrop: studioContent
        z: 50
        projectName: studio.app.projectName
        onSave: studio.save(studio.pendingAction)
        onDiscard: { studio.app.discardChanges(); studio.run(studio.pendingAction) }
    }

    FileDialog {
        id: fileDialog
        title: "Import audio"
        nameFilters: ["Audio files (*.wav *.flac *.mp3 *.ogg *.m4a *.aac *.aif *.aiff *.caf)", "All files (*)"]
        onAccepted: studio.app.importUrl(selectedFile)
    }

    FileDialog {
        id: openDialog
        title: "Open project"
        nameFilters: ["Isai projects (*.isai)"]
        currentFolder: StandardPaths.writableLocation(StandardPaths.MusicLocation)
        onAccepted: studio.app.openProject(selectedFile)
    }

    FileDialog {
        id: saveDialog
        title: "Save project"
        fileMode: FileDialog.SaveFile
        defaultSuffix: "isai"
        nameFilters: ["Isai projects (*.isai)"]
        currentFolder: StandardPaths.writableLocation(StandardPaths.MusicLocation)
        selectedFile: currentFolder + "/" + studio.app.projectName + ".isai"
        onAccepted: {
            if (studio.app.saveProjectAs(selectedFile) && studio.afterSaveAction !== "")
                studio.run(studio.afterSaveAction)
            studio.afterSaveAction = ""
        }
    }

    ProjectChooser {
        anchors.fill: parent
        visible: !studio.app.projectOpen || studio.choosing
        app: studio.app
        cancellable: studio.app.projectOpen
        onCancelled: studio.choosing = false
        onCreated: studio.choosing = false
        onOpenRequested: studio.guard("open")
        onOpenRecentRequested: path => studio.guard("recent:" + path)
        onRecoverRequested: studio.guard("recover")
    }

    ColumnLayout {
        id: studioContent
        anchors.fill: parent
        spacing: 0
        visible: studio.app.projectOpen && !studio.choosing

        TopBar {
            Layout.fillWidth: true
            app: studio.app
            onImportRequested: studio.importAudio()
            onSettingsRequested: studio.settingsRequested()
            onNewProjectRequested: studio.guard("new")
            onOpenRequested: studio.guard("open")
            onSaveRequested: studio.save("")
            onSaveAsRequested: studio.saveAs()
            onOpenRecentRequested: path => studio.guard("recent:" + path)
            onPaletteRequested: palette.open()
            onExportRequested: studio.exportAudio()
            playKeys: studio.playKeys
            onPlayKeysToggled: studio.togglePlayKeys()
        }

        // Crackle warning slides in under the control bar.
        Item {
            Layout.fillWidth: true
            Layout.preferredHeight: studio.app.crackling ? crackle.implicitHeight + 16 : 0
            clip: true
            Behavior on Layout.preferredHeight { NumberAnimation { duration: Theme.slow; easing.type: Easing.OutCubic } }
            Banner {
                id: crackle
                anchors.left: parent.left; anchors.right: parent.right
                anchors.bottom: parent.bottom
                anchors.margins: 8
                anchors.leftMargin: 16; anchors.rightMargin: 16
                tone: "warning"
                text: "The audio is crackling. The Lightweight profile uses a bigger buffer, which usually fixes it."
                DButton { variant: "primary"; text: "Switch to Lightweight"; onClicked: studio.app.switchToLightweight() }
                DButton { variant: "ghost"; text: "Dismiss"; onClicked: studio.app.dismissCrackle() }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            InspectorPanel {
                visible: studio.inspectorShown
                Layout.preferredWidth: 244
                Layout.fillHeight: true
                app: studio.app
                tracks: arrangement.tracks
                meters: arrangement.meters
                onInstrumentRequested: id => studio.openInstrument(id)
                onEffectEditRequested: (id, chain, tint) => { studio.fxEditId = id; studio.fxEditChain = chain; studio.fxEditTint = tint }
                onCloseRequested: studio.inspectorShown = false
            }

            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true

                TrackArea {
                    id: arrangement
                    anchors.fill: parent
                    app: studio.app
                    mixerShown: studio.mixerShown
                    pianoRollShown: studio.editorShown
                    keysEnabled: studio.letterKeys
                    onPlaylistRequested: studio.showPlaylist()
                    onPianoRollToggled: studio.togglePianoRoll()
                    // Room to scroll the last tracks up above the docked mixer or piano roll.
                    bottomInset: studio.mixerShown ? mixerPanel.height + mixerPanel.margin * 2
                               : studio.editorShown ? pianoRoll.height + pianoRoll.margin * 2 : 0
                    onEditRequested: id => studio.openEditor(id)
                onStemsRequested: (id, name) => stemsDialog.openFor(id, name)
                    onInstrumentRequested: id => studio.openInstrument(id)
                    onImportRequested: studio.importAudio()
                    onMixerToggled: studio.mixerShown = !studio.mixerShown
                    tunerShown: studio.tunerShown
                globalShown: studio.globalShown
                onGlobalToggled: studio.globalShown = !studio.globalShown
                    onTunerToggled: studio.tunerShown = !studio.tunerShown
                    onShortcutsRequested: shortcuts.open()
                    browserShown: studio.browserShown
                    onBrowserToggled: studio.browserShown = !studio.browserShown
                }

                BrowserPanel {
                    app: studio.app
                    shown: studio.browserShown
                    backdrop: arrangement
                    y: 50
                    height: parent.height - y - 12
                    onCloseRequested: studio.browserShown = false
                    onProjectRequested: url => studio.guard("url:" + url)
                }

                PianoRoll {
                    id: pianoRoll
                    app: studio.app
                    tracks: arrangement.tracks
                    regionId: studio.editorRegion
                    shown: studio.editorShown
                    backdrop: arrangement
                    onCloseRequested: studio.editorShown = false
                    onInstrumentRequested: id => studio.openInstrument(id)
                    activity: keyActivity
                }

                PianoWindow {
                    id: pianoWindow
                    app: studio.app
                    track: studio.instTrack && studio.instTrack.instrument.kind === "piano" ? studio.instTrack : null
                    activity: keyActivity
                    level: studio.instLevel
                    backdrop: arrangement
                    onCloseRequested: studio.instTrackId = -1
                }

                SynthWindow {
                    id: synthWindow
                    app: studio.app
                    track: studio.instTrack && studio.instTrack.instrument.kind === "synth" ? studio.instTrack : null
                    backdrop: arrangement
                    activity: keyActivity
                    level: studio.instLevel
                    onCloseRequested: studio.instTrackId = -1
                }

                DrumWindow {
                    id: drumWindow
                    app: studio.app
                    track: studio.instTrack && studio.instTrack.instrument.kind === "drums" ? studio.instTrack : null
                    backdrop: arrangement
                    activity: keyActivity
                    level: studio.instLevel
                    onCloseRequested: studio.instTrackId = -1
                }

                SamplerWindow {
                    id: samplerWindow
                    app: studio.app
                    track: studio.instTrack && studio.instTrack.instrument.kind === "sampler" ? studio.instTrack : null
                    backdrop: arrangement
                    activity: keyActivity
                    level: studio.instLevel
                    onCloseRequested: studio.instTrackId = -1
                }

                EffectEditor {
                    app: studio.app
                    insert: studio.fxEdit
                    chainName: studio.fxEditChain
                    tint: studio.fxEditTint
                    backdrop: arrangement
                    onCloseRequested: studio.fxEditId = -1
                }

                TunerWindow {
                    app: studio.app
                    shown: studio.tunerShown
                    backdrop: arrangement
                    onCloseRequested: studio.tunerShown = false
                }

                MixerPanel {
                    id: mixerPanel
                    onEffectEditRequested: (id, chain, tint) => { studio.fxEditId = id; studio.fxEditChain = chain; studio.fxEditTint = tint }
                    onMasteringRequested: masteringDialog.open()
                    app: studio.app
                    backdrop: arrangement
                    tracks: arrangement.tracks
                    shown: studio.mixerShown
                    onCloseRequested: studio.mixerShown = false
                }

                // Drop audio anywhere on the arrangement to import it.
                Rectangle {
                    anchors.fill: parent
                    anchors.margins: 12
                    visible: drop.containsDrag
                    radius: Theme.radiusLg
                    color: Theme.tint(Theme.accent, 0.12)
                    border.color: Theme.accent
                    border.width: 2
                    DText {
                        anchors.centerIn: parent
                        text: "Drop to import"
                        strong: true
                        font.pixelSize: Theme.textXl
                        color: Theme.accent
                    }
                }
                DropArea {
                    id: drop
                    anchors.fill: parent
                    onDropped: d => {
                        if (!d.hasUrls) return
                        const url = d.urls[0].toString()
                        // A dropped project replaces this one; audio is imported.
                        if (url.toLowerCase().endsWith(".isai")) studio.guard("url:" + url)
                        else if (!arrangement.dropFiles(d.urls, d.x, d.y)) studio.app.importUrl(d.urls[0])
                    }
                }
            }
        }

        StatusBar {
            Layout.fillWidth: true
            app: studio.app
        }
    }

    ShortcutsOverlay { id: shortcuts; backdrop: studioContent }
    // A guide for a new song (hidden once done or skipped).
    StartHere {
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.margins: 24
        z: 5
        app: studio.app
        studio: studio
    }
    MasteringDialog { id: masteringDialog; app: studio.app; backdrop: studioContent }
    StemsDialog { id: stemsDialog; app: studio.app; backdrop: studioContent }
    function splitStems() {
        for (const t of arrangement.tracks) {
            const r = t.regions.find(x => x.id === studio.app.selectedRegion && !x.midi)
            if (r) { stemsDialog.openFor(r.id, r.name); return }
        }
        studio.app.statusOk = false
        studio.app.status = "Select an audio region to split into stems"
    }

    // ------------------------------------------------------ command palette
    CommandPalette {
        backdrop: studioContent
        id: palette
        provider: q => studio.commands(q)
    }

    /// Everything ⌘K can do. Commands built from the query (a tempo, a bar
    /// number, a time signature, a key) come first.
    function commands(q) {
        const mod = Qt.platform.os === "osx" ? "⌘" : "Ctrl+"
        const a = app
        const on = v => v ? " (on)" : " (off)"
        const list = []

        // Smart commands from what was typed
        const tempo = q.match(/^(\d{2,3}(?:\.\d)?)\s*(?:bpm)?$/i)
        if (tempo && +tempo[1] >= 20 && +tempo[1] <= 400)
            list.push({ always: true, section: "Suggested", glyph: Icons.metronome,
                        title: "Set tempo to " + tempo[1] + " BPM", action: () => a.setProjectTempo(+tempo[1]) })
        const bar = q.match(/^(?:bar|go to bar|goto)?\s*(\d{1,4})$/i)
        if (bar && +bar[1] >= 1)
            list.push({ always: true, section: "Suggested", glyph: Icons.skipBack,
                        title: "Go to bar " + bar[1], action: () => a.seekBeats(TimeMap.barStart(+bar[1])) })
        const sig = q.match(/^(\d{1,2})\s*\/\s*(2|4|8|16)$/)
        if (sig)
            list.push({ always: true, section: "Suggested", glyph: Icons.musicNote,
                        title: "Set time signature to " + sig[1] + "/" + sig[2],
                        action: () => a.setTimeSignature(+sig[1], +sig[2]) })
        const key = q.match(/^(?:key\s+)?([a-g])([b#]?)\s*(m|min|minor|maj|major)?$/i)
        if (key && q.length <= 12) {
            const name = key[1].toUpperCase() + key[2] + " " + (key[3] && key[3].toLowerCase().startsWith("m") && key[3].toLowerCase() !== "maj" && key[3].toLowerCase() !== "major" ? "minor" : "major")
            list.push({ always: true, section: "Suggested", glyph: Icons.pianoKeys,
                        title: "Set key to " + name, action: () => a.setProjectKey(name) })
        }

        const add = (section, title, action, hint, glyph, keywords) =>
            list.push({ section, title, action, hint: hint || "", glyph, keywords: keywords || "" })

        add("Transport", a.playing ? "Stop" : "Play", () => a.togglePlay(), "Space", Icons.play, "start pause")
        add("Transport", "Go to beginning", () => a.rewind(), "Return", Icons.skipBack, "rewind start home")
        add("Transport", "Cycle" + on(a.cycle), () => a.toggleCycle(), "C", Icons.repeat, "loop repeat")
        add("Transport", "Cycle selected regions", () => a.cycleSelection(), mod + "U", Icons.repeat, "loop repeat locators set range")
        add("Transport", "Metronome" + on(a.metronome), () => a.toggleMetronome(), "K", Icons.metronome, "click")
        add("Transport", "Count-in" + on(a.countIn), () => a.toggleCountIn(), "", Icons.metronome, "pre-roll 1234")
        add("Transport", "Tap tempo", () => a.tapTempo(), "T", Icons.handTap, "bpm")
        add("Transport", a.recording ? "Stop recording" : "Record", () => a.record(), "R", Icons.microphoneStage, "take capture mic")
        if (a.selectedTrack >= 0)
            add("Tracks", "Arm selected track for recording", () => a.toggleArm(a.selectedTrack), "", Icons.microphoneStage, "record")

        add("Edit", a.undoLabel !== "" ? "Undo " + a.undoLabel : "Undo", () => a.undo(), mod + "Z", Icons.arrowCounterClockwise)
        add("Edit", a.redoLabel !== "" ? "Redo " + a.redoLabel : "Redo", () => a.redo(), "⇧" + mod + "Z", Icons.arrowsClockwise)
        add("Edit", "Split at playhead", () => a.splitAtPlayhead(), mod + "T", Icons.scissors, "cut scissors")
        add("Edit", "Split into stems…", () => studio.splitStems(), "", Icons.waveform, "stems vocals drums bass separate isolate acapella instrumental karaoke remove vocals")
        add("Edit", "Undo history…", () => arrangement.openHistory(), "⌥" + mod + "Z", Icons.clock, "steps back go back")
        add("Transport", "Stop (again: go to the start)", () => a.stop(), "0", Icons.stop, "halt")
        add("Markers", "Add marker at playhead", () => a.addMarker(a.positionBeats), "M", Icons.musicNote, "intro verse chorus section locator")
        add("Markers", "Next marker", () => a.jumpMarker(true), "⌥→", Icons.arrowRight, "section locator go")
        add("Markers", "Previous marker", () => a.jumpMarker(false), "⌥←", Icons.arrowLeft, "section locator go back")
        try {
            for (const m of JSON.parse(a.markersJson || "[]")) {
                add("Markers", "Go to " + m.name, () => a.seekBeats(m.beat), "", Icons.skipBack, "marker section")
                add("Markers", "Cycle " + m.name, () => a.cycleMarker(m.id), "", Icons.repeat, "loop marker section")
            }
        } catch (e) {}
        add("Tools", studio.tunerShown ? "Close tuner" : "Tuner", () => studio.tunerShown = !studio.tunerShown, "", Icons.gauge, "tune guitar bass voice pitch")
        add("Tools", "Keyboard shortcuts", () => shortcuts.open(), "?", Icons.keyboard, "keys help hotkeys")
        add("Tools", "Eco mode" + on(a.eco), () => a.setBool("eco", !a.eco), "", Icons.lightning, "low spec cpu light quality slow computer performance")
        add("Tools", studio.browserShown ? "Close browser" : "Browser", () => studio.browserShown = !studio.browserShown, "B", Icons.folderOpen, "sounds samples files loops find search")
        if (a.selectionCount > 0) {
            add("Edit", "Reverse selected regions", () => a.reverseSelected(), "", Icons.arrowLeft, "backwards")
            add("Edit", "Normalize selected regions", () => a.normalizeSelected(), "", Icons.speakerHigh, "louder gain level")
            if (a.selectionCount > 1) add("Edit", "Join selected regions", () => a.joinSelected(), mod + "J", Icons.copy, "merge glue")
        }
        add("View", studio.mixerShown ? "Hide mixer" : "Show mixer", () => studio.mixerShown = !studio.mixerShown, "F9", Icons.sliders, "faders channel strips volume")
        add("View", studio.editorShown ? "Hide piano roll" : "Piano roll", () => studio.togglePianoRoll(), "F7", Icons.pianoKeys, "midi notes editor pattern")
        add("View", "Playlist (just the tracks)", () => studio.showPlaylist(), "F5", Icons.waveform, "arrangement tracks")
        add("View", studio.globalShown ? "Hide tempo & signature lanes" : "Tempo & signature changes", () => studio.globalShown = !studio.globalShown, "G", Icons.metronome, "tempo track change bpm ritardando accelerando time signature global tracks")
        add("View", studio.inspectorShown ? "Hide inspector" : "Show inspector", () => studio.inspectorShown = !studio.inspectorShown, "I", Icons.sliders, "channel track details fader effects")
        add("Tracks", "New piano track", () => { a.addInstrumentTrack("piano"); studio.openInstrument(a.selectedTrack) }, "", Icons.pianoKeys, "piano instrument midi keys")
        add("Tracks", "New synth track", () => { a.addInstrumentTrack("synth"); studio.openInstrument(a.selectedTrack) }, "", Icons.waveform, "synth instrument midi keys bass lead pad")
        add("Tracks", "New sampler track", () => { a.addInstrumentTrack("sampler"); studio.openInstrument(a.selectedTrack) }, "", Icons.waveform, "sampler slicer chop loop sample one-shot")
        add("Tracks", "New drum track", () => { a.addInstrumentTrack("drums"); studio.openInstrument(a.selectedTrack) }, "", Icons.metronome, "drums beat step sequencer kick snare hats 808 club pattern")
        add("Tracks", "Look for MIDI keyboards", () => a.refreshMidi(), "", Icons.keyboard, "midi controller device input")
        add("Edit", "Copy", () => a.copySelected(), mod + "C", Icons.copy, "regions clipboard")
        add("Edit", "Cut", () => a.cutSelected(), mod + "X", Icons.scissors, "regions clipboard")
        if (a.canPaste) add("Edit", "Paste at playhead", () => a.paste(), mod + "V", Icons.copy, "regions clipboard")
        add("Edit", "Duplicate", () => a.duplicateSelected(), mod + "D", Icons.copy, "regions repeat again")
        add("Edit", "Select all regions", () => a.selectAllRegions(), mod + "A", Icons.command, "everything")
        add("Edit", a.selectionCount > 1 ? "Delete selected regions" : "Delete selected region", () => a.deleteSelected(), "⌫", Icons.trash, "remove")
        add("Edit", "Snap to grid" + on(a.snap), () => a.toggleSnap(), "N", Icons.magnet)
        for (const g of [["bar", "Bar"], ["beat", "Beat"], ["1/2", "½ beat"], ["1/4", "¼ beat"]])
            add("Edit", "Snap grid: " + g[1], () => a.setSnapGrid(g[0]), "", Icons.magnet, "quantize")

        add("Tracks", "New track", () => a.addTrack(), "⌥" + mod + "N", Icons.plus, "add audio")
        add("Tracks", "Import audio…", () => studio.importAudio(), mod + "I", Icons.downloadSimple, "file wav mp3 add")
        if (a.selectedTrack >= 0) {
            add("Tracks", "Mute selected track", () => a.toggleMute(a.selectedTrack), "", Icons.speakerX)
            add("Tracks", "Solo selected track", () => a.toggleSolo(a.selectedTrack), "", Icons.headphones)
            add("Tracks", "Delete selected track", () => a.removeTrack(a.selectedTrack), "", Icons.trash, "remove")
            const sel = arrangement.tracks.find(t => t.id === a.selectedTrack)
            if (sel) add("Tracks", sel.frozen ? "Unfreeze selected track" : "Freeze selected track (save CPU)",
                         () => sel.frozen ? a.unfreezeTrack(sel.id) : a.freezeTrack(sel.id), "", Icons.cpu, "render bounce performance")
        }

        add("View", "Zoom in", () => arrangement.zoomBy(1.5), mod + "=", Icons.zoomIn)
        add("View", "Zoom out", () => arrangement.zoomBy(1 / 1.5), mod + "−", Icons.zoomOut)
        add("View", "Zoom to fit", () => arrangement.zoomToFit(), "Z", Icons.zoomFit, "whole song")
        add("View", "Switch to " + (a.view === "pro" ? "Simple" : "Pro") + " view", () => a.setText("view", a.view === "pro" ? "simple" : "pro"), "", Icons.sliders)

        add("File", "Export audio…", () => studio.exportAudio(), mod + "E", Icons.downloadSimple, "bounce mixdown render stems wav flac")
        add("Tools", "Mastering assistant…", () => masteringDialog.open(), "", Icons.magicWand, "master loudness lufs limiter finish loud streaming")
        add("File", "Save", () => studio.save(""), mod + "S", Icons.downloadSimple)
        add("File", "Save As…", () => studio.saveAs(), "⇧" + mod + "S", Icons.downloadSimple)
        add("File", "Open project…", () => studio.guard("open"), mod + "O", Icons.folderOpen)
        add("File", "New project…", () => studio.guard("new"), mod + "N", Icons.filePlus)
        for (let i = 0; i < Math.min(5, a.recentPaths.length); ++i) {
            const path = a.recentPaths[i]
            if (path !== a.projectPath)
                add("File", "Open recent: " + a.recentNames[i], () => studio.guard("recent:" + path), "", Icons.musicNote)
        }

        add("Appearance", "Dark theme", () => a.setText("theme", "dark"), "", Icons.moon)
        add("Appearance", "Light theme", () => a.setText("theme", "light"), "", Icons.sun)
        add("Appearance", "Follow system theme", () => a.setText("theme", "system"), "", Icons.circleHalf)
        add("Appearance", "Studio style", () => a.setText("uiStyle", "studio"), "", Icons.sliders)
        add("Appearance", "Midnight style", () => a.setText("uiStyle", "midnight"), "", Icons.sparkle)
        add("Appearance", "Vivid style", () => a.setText("uiStyle", "vivid"), "", Icons.palette, "black colourful bright")
        add("Appearance", "Settings…", () => studio.settingsRequested(), mod + ",", Icons.gear, "preferences options")
        // Keep each section together (in the order sections first appear), so headers don't repeat.
        const order = []
        for (const c of list) if (order.indexOf(c.section) < 0) order.push(c.section)
        return list.map((c, i) => [c, i])
                   .sort((x, y) => (order.indexOf(x[0].section) - order.indexOf(y[0].section)) || (x[1] - y[1]))
                   .map(x => x[0])
    }

    // Computer keys play the selected instrument track (A W S E D F T G Y H U J K O L P,
    // Z/X change octave), so a plugin can be played without its own window.
    property bool playKeys: false
    property int playOctave: 4
    readonly property bool letterKeys: keysOn && !playKeys
    readonly property var playKeyMap: ({
        [Qt.Key_A]: 0, [Qt.Key_W]: 1, [Qt.Key_S]: 2, [Qt.Key_E]: 3, [Qt.Key_D]: 4, [Qt.Key_F]: 5,
        [Qt.Key_T]: 6, [Qt.Key_G]: 7, [Qt.Key_Y]: 8, [Qt.Key_H]: 9, [Qt.Key_U]: 10, [Qt.Key_J]: 11,
        [Qt.Key_K]: 12, [Qt.Key_O]: 13, [Qt.Key_L]: 14, [Qt.Key_P]: 15
    })
    // The note each held key started, so releasing it stops that note even after an octave change.
    property var heldPlayKeys: ({})
    function togglePlayKeys() {
        playKeys = !playKeys
        if (playKeys) Qt.callLater(() => playKeysHandler.forceActiveFocus())
        else releaseHeldKeys()
    }
    // Focus goes back to the key handler whenever the window is active again
    // (after a plugin window or another app took the keys).
    Connections {
        target: studio.Window.window
        function onActiveChanged() {
            if (studio.playKeys && studio.Window.window && studio.Window.window.active)
                Qt.callLater(() => playKeysHandler.forceActiveFocus())
        }
    }
    function releaseHeldKeys() {
        for (const k in heldPlayKeys) app.noteOff(heldPlayKeys[k])
        heldPlayKeys = ({})
    }
    Item {
        id: playKeysHandler
        focus: studio.playKeys
        Keys.onPressed: e => {
            if (!studio.playKeys) { e.accepted = false; return }
            e.accepted = true
            if (e.isAutoRepeat) return
            if (e.key === Qt.Key_Z) { studio.playOctave = Math.max(0, studio.playOctave - 1); return }
            if (e.key === Qt.Key_X) { studio.playOctave = Math.min(8, studio.playOctave + 1); return }
            if (studio.playKeyMap[e.key] === undefined) { e.accepted = false; return }
            const note = (studio.playOctave + 1) * 12 + studio.playKeyMap[e.key]
            studio.heldPlayKeys[e.key] = note
            studio.app.noteOn(note, 100)
        }
        Keys.onReleased: e => {
            if (!studio.playKeys || e.isAutoRepeat) return
            if (studio.heldPlayKeys[e.key] === undefined) return
            studio.app.noteOff(studio.heldPlayKeys[e.key])
            delete studio.heldPlayKeys[e.key]
            e.accepted = true
        }
    }

    // Standard DAW keys. They only act while a project is on screen.
    readonly property bool keysOn: studio.visible && studio.app.projectOpen && !studio.choosing && !pianoRoll.typing && !pianoWindow.typing && !synthWindow.typing
    Shortcut { sequence: "Space"; enabled: studio.keysOn; onActivated: studio.app.togglePlay() }
    Shortcut { sequence: "Ctrl+I"; enabled: studio.keysOn; onActivated: studio.importAudio() }
    Shortcut { sequences: ["Return", "Home"]; enabled: studio.keysOn; onActivated: studio.app.rewind() }
    Shortcut { sequence: "C"; enabled: studio.letterKeys; onActivated: studio.app.toggleCycle() }
    Shortcut { sequence: "K"; enabled: studio.letterKeys; onActivated: studio.app.toggleMetronome() }
    Shortcut { sequence: "T"; enabled: studio.letterKeys; onActivated: studio.app.tapTempo() }
    Shortcut { sequence: "N"; enabled: studio.letterKeys; onActivated: studio.app.toggleSnap() }
    Shortcut { sequences: ["Backspace", "Delete"]; enabled: studio.keysOn && !pianoRoll.ownsKeys; onActivated: studio.app.deleteSelected() }
    Shortcut { sequences: [StandardKey.Undo]; enabled: studio.keysOn; onActivated: studio.app.undo() }
    Shortcut { sequences: [StandardKey.Redo, "Ctrl+Shift+Z"]; enabled: studio.keysOn; onActivated: studio.app.redo() }
    Shortcut { sequences: [StandardKey.Save]; enabled: studio.keysOn; onActivated: studio.save("") }
    Shortcut { sequence: "Ctrl+Shift+S"; enabled: studio.keysOn; onActivated: studio.saveAs() }
    Shortcut { sequence: "Ctrl+Alt+N"; enabled: studio.keysOn; onActivated: studio.app.addTrack() }
    Shortcut { sequence: "Ctrl+T"; enabled: studio.keysOn; onActivated: studio.app.splitAtPlayhead() }
    Shortcut { sequence: "X"; enabled: studio.letterKeys; onActivated: studio.mixerShown = !studio.mixerShown }
    // Windows, FL Studio style: F5 playlist, F7 piano roll, F9 mixer
    // (P and X do the same, Logic style).
    Shortcut { sequences: ["F7", "P"]; enabled: studio.letterKeys; onActivated: studio.togglePianoRoll() }
    Shortcut { sequence: "F9"; enabled: studio.keysOn; onActivated: studio.mixerShown = !studio.mixerShown }
    Shortcut { sequence: "F5"; enabled: studio.keysOn; onActivated: studio.showPlaylist() }
    Shortcut { sequences: [StandardKey.Copy]; enabled: studio.keysOn; onActivated: studio.app.copySelected() }
    Shortcut { sequences: [StandardKey.Cut]; enabled: studio.keysOn; onActivated: studio.app.cutSelected() }
    Shortcut { sequences: [StandardKey.Paste]; enabled: studio.keysOn; onActivated: studio.app.paste() }
    Shortcut { sequence: "Ctrl+D"; enabled: studio.keysOn; onActivated: studio.app.duplicateSelected() }
    Shortcut { sequences: [StandardKey.SelectAll]; enabled: studio.keysOn && !studio.editorShown; onActivated: studio.app.selectAllRegions() }
    Shortcut { sequence: "Ctrl+E"; enabled: studio.keysOn; onActivated: studio.exportAudio() }
    Shortcut { sequence: "R"; enabled: studio.letterKeys; onActivated: studio.app.record() }
    Shortcut { sequence: "Ctrl+K"; enabled: studio.keysOn; onActivated: palette.shown ? palette.close() : palette.open() }
    Shortcut { sequence: "0"; enabled: studio.keysOn; onActivated: studio.app.stop() }
    Shortcut { sequence: "B"; enabled: studio.letterKeys; onActivated: studio.browserShown = !studio.browserShown }
    Shortcut { sequence: "M"; enabled: studio.letterKeys; onActivated: studio.app.addMarker(studio.app.positionBeats) }
    Shortcut { sequence: "Alt+Right"; enabled: studio.keysOn; onActivated: studio.app.jumpMarker(true) }
    Shortcut { sequence: "Alt+Left"; enabled: studio.keysOn; onActivated: studio.app.jumpMarker(false) }
    Shortcut { sequence: "Ctrl+J"; enabled: studio.keysOn; onActivated: studio.app.joinSelected() }
    Shortcut { sequence: "Ctrl+U"; enabled: studio.keysOn; onActivated: studio.app.cycleSelection() }
    Shortcut { sequence: "I"; enabled: studio.letterKeys; onActivated: studio.inspectorShown = !studio.inspectorShown }
    Shortcut { sequence: "G"; enabled: studio.letterKeys; onActivated: studio.globalShown = !studio.globalShown }
    Shortcut { sequence: "Ctrl+Alt+Z"; enabled: studio.keysOn; onActivated: arrangement.openHistory() }
    Shortcut { sequences: ["?", "Shift+/"]; enabled: studio.keysOn; onActivated: shortcuts.opened ? shortcuts.close() : shortcuts.open() }
    // These also work from the project chooser.
    Shortcut { sequences: [StandardKey.New]; enabled: studio.visible && !studio.choosing; onActivated: studio.guard("new") }
    Shortcut { sequences: [StandardKey.Open]; enabled: studio.visible; onActivated: studio.guard("open") }
}
