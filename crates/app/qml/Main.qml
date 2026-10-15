import QtQuick
import QtQuick.Controls.Basic
import Isai

ApplicationWindow {
    id: win

    width: 1240
    height: 780
    minimumWidth: 860
    minimumHeight: 540
    visible: true
    title: "Isai Studio Pro"
    color: Theme.bg
    font.family: Theme.font
    font.pixelSize: Theme.textMd

    // A standard window with the system's own title bar, so moving it, other
    // displays and desktops, full screen and window snapping all just work.
    // (Drawing under the macOS title bar left the window impossible to drag.)
    flags: Qt.Window

    property bool ready: false
    property bool quitting: false

    // Ask to save unsaved work before the window closes.
    onClosing: close => {
        if (!quitting && app.projectOpen && app.dirty) {
            close.accepted = false
            studio.guard("quit")
        }
    }

    AppController { id: app }

    // Settings drive the design tokens.
    Binding {
        target: Theme; property: "dark"
        value: app.theme === "dark"
               || (app.theme === "system" && Application.styleHints.colorScheme !== Qt.ColorScheme.Light)
    }
    Binding {
        target: Theme; property: "motionMs"
        value: app.animations === "full" ? 200 : app.animations === "reduced" ? 90 : 0
    }
    Binding { target: Theme; property: "effects"; value: app.effects }
    Binding { target: Theme; property: "style"; value: app.uiStyle || "studio" }
    Binding { target: Theme; property: "accentChoice"; value: app.accent || "default" }
    Binding { target: TimeMap; property: "json"; value: app.tempoJson }

    // Poll the engine only while something is moving, at the user's frame rate.
    Timer {
        interval: app.uiFps > 0 ? Math.round(1000 / app.uiFps) : 16
        repeat: true
        running: app.busy && app.uiFps > 0
        onTriggered: app.tick()
    }
    FrameAnimation {
        running: app.busy && app.uiFps === 0
        onTriggered: app.tick()
    }
    // Crash-recovery copy of unsaved work, every 30 seconds.
    Timer {
        interval: 30000
        repeat: true
        running: app.projectOpen && app.dirty
        onTriggered: app.autosave()
    }

    // Everything lives in `root` so the whole window can be captured as an image.
    Rectangle {
        id: root
        anchors.fill: parent
        color: Theme.bg
        Component.onCompleted: Theme.backdrop = root
        Behavior on color { ColorAnimation { duration: Theme.slow } }

        StudioPage {
            id: studio
            anchors.fill: parent
            app: app
            visible: opacity > 0
            opacity: win.ready && !app.needsSetup ? 1 : 0
            enabled: !app.needsSetup
            Behavior on opacity { NumberAnimation { duration: Theme.slow } }
            onSettingsRequested: settingsDialog.open()
            onQuitApproved: { win.quitting = true; win.close() }
        }

        SetupPage {
            id: setup
            anchors.fill: parent
            app: app
            visible: opacity > 0
            opacity: win.ready && app.needsSetup ? 1 : 0
            Behavior on opacity { NumberAnimation { duration: Theme.slow } }
        }

        SettingsDialog { id: settingsDialog; app: app; backdrop: app.needsSetup ? setup : studio }
    }

    Shortcut {
        sequences: [StandardKey.Preferences, "Ctrl+,"]
        enabled: !app.needsSetup
        onActivated: settingsDialog.open()
    }

    Component.onCompleted: {
        app.start(win.contentItem.GraphicsInfo.api === GraphicsInfo.Software)
        win.ready = true
        if (app.devScene !== "") Qt.callLater(win.showScene)
    }

    // ------------------------------------------------------------------ dev
    // ISAI_SCENE picks a screen and ISAI_SCREENSHOT saves it as an image, so the
    // design can be reviewed without clicking through the app.
    //   setup1..setup4   a first-launch setup step
    //   chooser          the New Project screen
    //   studio           the main screen (with a demo project)
    //   edit             studio, plus fades, a split and a second track
    //   settings0..4     a Settings section
    function showScene() {
        const scene = app.devScene
        if (scene.startsWith("setup")) {
            const step = parseInt(scene.slice(5) || "1") - 1
            if (step > 0) {
                stepTimer.step = step
                stepTimer.start()
            }
        }
        if ((scene === "studio" || scene.startsWith("export") || scene.startsWith("edit") || scene === "multi" || scene === "mixer" || scene.startsWith("fx") || scene === "auto" || scene.startsWith("midi") || scene === "piano" || scene === "synth" || scene === "markers" || scene === "tuner" || scene === "rolltools" || scene === "drums" || scene === "sampler" || scene === "browser" || scene === "master" || scene === "stretch" || scene === "cycle" || scene === "tempo" || scene === "stems" || scene.startsWith("plugin") || scene.startsWith("palette") || scene.startsWith("settings")) && !app.projectOpen)
            app.newProject("Demo Song", 120, 4, 4, "C major")
        if (scene.startsWith("settings")) {
            settingsDialog.section = parseInt(scene.slice(8) || "0")
            settingsDialog.open()
        }
        if (scene.startsWith("edit") || scene === "multi" || scene === "mixer" || scene.startsWith("fx") || scene === "auto" || scene.startsWith("midi") || scene === "piano" || scene === "synth" || scene === "markers" || scene === "tuner" || scene === "rolltools" || scene === "drums" || scene === "sampler") editTimer.start()
        if (scene.startsWith("palette")) paletteTimer.start()
        if (scene === "browser") Qt.callLater(() => studio.browserShown = true)
        if (scene === "master") masterTimer.start()
        if (scene === "stretch") stretchTimer.start()
        if (scene === "cycle") cycleTimer.start()
        if (scene === "tempo") tempoTimer.start()
        if (scene.startsWith("plugin")) pluginTimer.start()
        if (scene === "stems") stemsTimer.start()
        if (scene === "export") Qt.callLater(studio.exportAudio)
        if (scene.startsWith("export:")) exportRunTimer.start()
        if (app.devScreenshot !== "") shotTimer.start()
    }
    // "export:<format>:<file>": export the mix there, then show the dialog's result.
    Timer {
        id: exportRunTimer
        interval: 1200
        onTriggered: {
            const parts = app.devScene.split(":")
            studio.exportAudio()
            app.exportAudio("file://" + parts.slice(2).join(":"), false, false, parts[1], true)
        }
    }
    Timer { id: fxAddTimer; interval: 900; onTriggered: studio.openAddEffectMenu() }
    // Plays a little melody (one note per tick), then stops recording.
    Timer {
        id: recTimer
        interval: 250
        repeat: true
        property int n: 0
        onTriggered: {
            const keys = [60, 62, 64, 65, 67, 69, 71, 72]
            if (n > 0 && n <= keys.length) app.noteOff(keys[n - 1])
            if (n < keys.length) app.noteOn(keys[n], 100)
            else { stop(); app.record(); studio.togglePianoRoll() }
            n++
        }
    }
    // "palette" or "palette:<query>": the ⌘K palette, optionally with text typed.
    Timer {
        id: paletteTimer
        interval: 600
        onTriggered: studio.openPalette(app.devScene.indexOf(":") > 0 ? app.devScene.slice(app.devScene.indexOf(":") + 1) : "")
    }
    // "edit": fades on the first region, a split, and a second track.
    Timer {
        id: editTimer
        interval: 1000
        onTriggered: {
            const tracks = JSON.parse(app.tracksJson || "[]")
            if (tracks.length === 0 || tracks[0].regions.length === 0) { restart(); return }
            const r = tracks[0].regions[0]
            const t0 = tracks[0]
            app.setRegionFades(r.id, 2, 3, true)
            app.selectRegion(r.id)
            app.seekBeats(16)
            app.splitAtPlayhead()
            app.addTrack()
            if (app.devScene === "edit-zoom") { app.selectRegion(r.id); zoomTimer.start() }
            // "mixer": the mixer open while playing, so the meters move.
            if (app.devScene === "mixer" || app.devScene.startsWith("fx")) {
                app.setTrackVolume(t0.id, -4.5, true)
                app.setTrackPan(t0.id, -0.4, true)
                app.addBus()
                app.renameBus(JSON.parse(app.busesJson)[0].id, "Reverb")
                app.setSend(t0.id, JSON.parse(app.busesJson)[0].id, -8, true)
                const eqId = app.addInsert("track", t0.id, "eq")
                for (const [k, v] of [["lc_on", 1], ["lc_freq", 80], ["b2_on", 1], ["b2_freq", 900], ["b2_gain", 5], ["b2_q", 1.4],
                                      ["b4_on", 1], ["b4_freq", 4500], ["b4_gain", -3.5], ["hs_on", 1], ["hs_gain", 3]])
                    app.setInsertParam(eqId, k, v, true)
                const compId = app.addInsert("track", t0.id, "compressor")
                app.setInsertParam(compId, "threshold", -30, true)
                app.addInsert("bus", JSON.parse(app.busesJson)[0].id, "reverb")
                app.addInsert("master", -1, "limiter")
                app.selectTrack(t0.id)
                studio.openEffect(eqId)
                studio.mixerShown = true
                if (app.devScene === "fxeq") studio.openEffectEditor(eqId, "demo-song · track")
                if (app.devScene === "fxcomp") studio.openEffectEditor(compId, "demo-song · track")
                // "fxadd": the Add effect menu, low in the docked mixer.
                if (app.devScene === "fxadd") fxAddTimer.start()
                app.seekBeats(0)
                app.togglePlay()
            }
            // "midi": an instrument track with a chord progression, piano roll open, playing.
            // "midirec": record notes played from code onto a new instrument track.
            if (app.devScene === "midirec") {
                app.addInstrumentTrack("synth")
                const tr = JSON.parse(app.tracksJson)
                app.selectTrack(tr[tr.length - 1].id)
                app.seekBeats(0)
                app.record()
                recTimer.start()
            }
            // "markers": markers, a looped region, gain and reverse, history open.
            if (app.devScene === "markers") {
                app.addMarker(0); app.addMarker(16); app.addMarker(32)
                const tr = JSON.parse(app.tracksJson)
                const parts = tr[0].regions
                app.setRegionGain(parts[0].id, 3.5, true)
                app.selectRegion(parts[1].id)
                app.reverseSelected()
                app.addInstrumentTrack("synth")
                const tr2 = JSON.parse(app.tracksJson)
                const inst = tr2[tr2.length - 1]
                const region = app.addMidiRegion(inst.id, 0, 4)
                for (let i = 0; i < 8; ++i) app.addNote(region, i * 0.5, 0.4, [60, 64, 67, 72][i % 4], 100)
                app.setRegionLoop(region, 16)
                app.selectRegion(region)
                app.seekBeats(18)
                return
            }
            // "tuner": the tuner open.
            if (app.devScene === "tuner") {
                studio.tunerShown = true
                return
            }
            // "rolltools": chords drawn in key, strummed, ghost notes from another track.
            if (app.devScene === "rolltools") {
                app.setProjectKey("A minor")
                app.addInstrumentTrack("piano")
                let tr = JSON.parse(app.tracksJson)
                const bass = app.addMidiRegion(tr[tr.length - 1].id, 0, 8)
                for (let i = 0; i < 4; ++i) app.addNote(bass, i * 2, 1.5, [45, 41, 48, 43][i], 100)
                app.addInstrumentTrack("synth")
                tr = JSON.parse(app.tracksJson)
                const region = app.addMidiRegion(tr[tr.length - 1].id, 0, 8)
                studio.openEditor(region)
                studio.roll.drawChord = "key3"
                for (const [b, k] of [[0, 57], [2, 53], [4, 60], [6, 55]])
                    app.addNotes(region, JSON.stringify(studio.roll.chordKeys(k).map(x => [b, 1.75, x, 100])))
                app.strumNotes(region, "", 0.0625, false)
                return
            }
            // "drums": a drum track with a groove, its window open.
            if (app.devScene === "drums") {
                app.addInstrumentTrack("drums")
                const dt = app.selectedTrack
                const region = app.addMidiRegion(dt, 0, 4)
                const rows = { 36: "x.........x.....", 38: "........X.......", 42: "x.xox.xox.xxx.xo", 46: "..............x." }
                const notes = []
                for (const k in rows) for (let i = 0; i < 16; ++i) if (rows[k][i] !== ".") notes.push([i * 0.25, 0.25, Number(k), rows[k][i] === "X" ? 127 : rows[k][i] === "o" ? 60 : 100])
                app.addNotes(region, JSON.stringify(notes))
                app.selectRegion(region)
                studio.openInstrument(dt)
                return
            }
            // "sampler": the demo song in a sampler, sliced at its beats.
            if (app.devScene === "sampler") {
                const source = t0.regions[0].source
                app.addInstrumentTrack("sampler")
                const st = app.selectedTrack
                app.loadSample(st, "file://" + source)
                app.setInstrumentParam(st, "mode", 1, true)
                studio.openInstrument(st)
                sliceTimer.track = st
                sliceTimer.start()
                return
            }
            // "fxdist": a distortion's editor (the generic knob layout).
            if (app.devScene === "fxdist") {
                const id = app.addInsert("track", t0.id, "distortion")
                studio.openEffectEditor(id, "demo-song · track")
                return
            }
            // "synth": a synth track with its window open.
            if (app.devScene === "synth") {
                app.addInstrumentTrack("synth")
                studio.openInstrument(app.selectedTrack)
                return
            }
            // "piano": a piano track with its window open.
            if (app.devScene === "piano") {
                app.addInstrumentTrack("piano")
                studio.openInstrument(app.selectedTrack)
                return
            }
            if (app.devScene === "midi" || app.devScene === "midisel" || app.devScene === "midifreeze") {
                app.addInstrumentTrack("synth")
                const tracks2 = JSON.parse(app.tracksJson)
                const inst = tracks2[tracks2.length - 1]
                const region = app.addMidiRegion(inst.id, 0, 8)
                const chords = [[60, 64, 67], [57, 60, 64], [53, 57, 60], [55, 59, 62]]
                chords.forEach((c, i) => c.forEach(k => app.addNote(region, i * 2, 1.75, k, 80 + 10 * (k % 3))))
                for (let i = 0; i < 16; ++i) app.addNote(region, i * 0.5, 0.4, [72, 76, 79, 76][i % 4], 70 + (i % 4) * 12)
                // "midifreeze": the synth track frozen (rendered in the background).
                if (app.devScene === "midifreeze") { app.freezeTrack(inst.id); return }
                studio.openEditor(region)
                // "midisel": the first chord selected and moved up a tone and right a beat.
                if (app.devScene === "midisel") {
                    studio.roll.selection = [0, 1, 2]
                    studio.roll.transformSelected(1, 2, 0)
                    return
                }
                app.seekBeats(0)
                app.togglePlay()
            }
            // "auto": a volume fade drawn on the first track, lane open, playing.
            if (app.devScene === "auto") {
                for (const [b, v] of [[0, -30], [8, 0], [24, -6], [36, -60]]) app.addAutoPoint(t0.id, "volume", b, v)
                app.setAutomationView(t0.id, "volume")
                app.selectTrack(t0.id)
                app.seekBeats(4)
                app.togglePlay()
            }
            // "multi": duplicate everything (⌘A, ⌘D), copy, paste onto the new track.
            if (app.devScene === "multi") {
                app.selectAllRegions()
                app.duplicateSelected()
                app.copySelected()
                const t = JSON.parse(app.tracksJson)
                app.selectTrack(t[t.length - 1].id)
                app.seekBeats(4)
                app.paste()
            }
        }
    }
    Timer { id: zoomTimer; interval: 400; onTriggered: studio.zoomForTest(3) }
    // Splits the first region into stems, then logs how it went.
    Timer {
        id: stemsTimer
        interval: 1500
        onTriggered: {
            const r = JSON.parse(app.tracksJson)[0].regions[0]
            app.splitStems(r.id)
            stemsWatch.start()
        }
    }
    Timer {
        id: stemsWatch
        interval: 500
        repeat: true
        onTriggered: {
            if (app.stemRegion >= 0) { console.log("stems", app.stemStage, app.stemProgress.toFixed(2)); return }
            stop()
            console.log("stems done:", JSON.parse(app.tracksJson).map(t => t.name + (t.mute ? " (muted)" : "")).join(" | "), "·", app.status)
            shotTimer.interval = 1500
            shotTimer.restart()
        }
    }
    // Waits for the plugin scan, adds Pigments with a chord and plays it.
    Timer {
        id: pluginTimer
        interval: 500
        repeat: true
        onTriggered: {
            if (app.pluginScanning) return
            stop()
            const list = JSON.parse(app.pluginsJson || "[]")
            // "plugin:<name>" picks a plugin by name; otherwise Pigments or any instrument.
            const want = app.devScene.split(":")[1] || "Pigments"
            const p = list.find(x => x.name === want) || list.find(x => x.instrument)
            if (!p) return
            const track = app.addPluginTrack(p.index)
            const region = app.addMidiRegion(track, 0, 8)
            for (const key of [60, 64, 67]) app.addNote(region, 0, 6, key, 100)
            app.seekBeats(0)
            app.togglePlay()
            pluginMeter.start()
        }
    }
    Timer { id: pluginMeter; interval: 1200; onTriggered: console.log("plugin-meter", app.meterL.toFixed(3), app.meterR.toFixed(3), app.trackMeters, "windows", app.pluginWindows, app.status) }
    // A tempo change at bar 5 and 3/4 from bar 9, with the lanes shown, playing from bar 8.
    Timer { id: tempoTimer; interval: 1500; onTriggered: { app.addTempoChange(16, 90); app.addSignatureChange(9, 3, 4); studio.globalShown = true; app.seekBeats(29) } }
    Timer { id: cycleTimer; interval: 1500; onTriggered: { app.splitAtPlayhead(); app.seekBeats(20); app.selectAllRegions(); app.cycleSelection() } }
    Timer {
        id: stretchTimer
        interval: 1500
        onTriggered: {
            const r = JSON.parse(app.tracksJson)[0].regions[0]
            const bpm = app.detectRegionTempo(r.id)
            console.log("DETECT", bpm, "beats", r.beats)
            app.setRegionTempo(r.id, bpm > 0 ? bpm : 120)
            app.setProjectTempo(100)
            stretchDone.start()
        }
    }
    Timer { id: stretchDone; interval: 2500; onTriggered: { const r = JSON.parse(app.tracksJson)[0].regions[0]; console.log("AFTER beats", r.beats, "tempo", r.tempo, "loading", app.loading) } }
    Timer { id: masterTimer; interval: 1500; onTriggered: { app.analyseMaster(-14); masterDone.start() } }
    // Popups don't show in screenshots, so print the report instead.
    Timer { id: masterDone; interval: 3000; onTriggered: console.log("MASTER", app.masterReport) }
    Timer { id: sliceTimer; property int track: -1; interval: 1500; onTriggered: app.sliceSample(track, "beats", 16) }
    Timer {
        id: stepTimer
        property int step: 1
        interval: 1400
        onTriggered: setup.go(step)
    }
    Timer {
        id: shotTimer
        interval: app.devScene === "midirec" ? 5000 : app.devScene.startsWith("plugin") ? 30000 : app.devScene === "stems" ? 240000 : 2800
        onTriggered: {
            root.grabToImage(result => {
                result.saveToFile(app.devScreenshot)
                Qt.quit()
            })
            // An idle window draws no frames, and the grab waits for one.
            win.requestUpdate()
        }
    }
}
