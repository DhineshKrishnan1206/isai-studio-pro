# Where we left off (2026-10-22)

State: clippy clean (`-D warnings`), all tests pass (`cargo test --workspace`).

## Roadmap items completed in this pass

- Backups: File → Restore earlier version… lists `Backups/` (newest first); a restore
  is one undoable edit. `storage::backup_time` reads the time from the file name.
- Alternatives: File → Save as alternative… writes `<folder>/<name>.isai` next to the
  main version and opens it; File → Open alternative… lists the others (through the
  unsaved-work guard). They share the folder's `Audio/`.
- Punch: `Project::punch` (Transport → Punch recording). Finished takes are cut to the
  cycle region (`record_ops::punch_clip`, tested); the take file still holds the whole recording.
- MIDI import: one instrument track per MIDI track (`Project::import_midi`).
- Reduce motion: `app/src/motion.rs` reads macOS `reduceMotion`, Windows
  "Show animations" (UserPreferencesMask bit 0x02) and GNOME `enable-animations`.
  Checked at start and when the window gains focus; "reduced" motion (90 ms) when on.
  Windows and GNOME paths are untested here.

Still not done: multitrack recording (needs several input channels at once in the
engine and device layer), take comping (needs take lanes in the region model and UI),
disk streaming for long audio (needs a reader thread feeding the audio thread), and
DAWproject export/import (needs the format spec and a validator). Real Windows and
Linux testing needs those machines.

## MIDI files (roadmap 2)

- `crates/project/src/midi_file.rs`: Standard MIDI File reader/writer, no new
  dependency. Reads format 0/1, running status, tempo, time signature; refuses
  SMPTE timing; damaged files return errors (tested with every truncation).
- `Project::import_midi` merges a file's tracks into one region on the selected
  instrument track at the playhead; `Project::export_midi` writes a region's
  notes as they play. Menu: Tracks → Import MIDI file… / Export MIDI file….
- Not done: importing each file track onto its own instrument track.

## Project backups (roadmap 1)

- Every save that changes the project keeps the previous `.isai` in
  `<project>/Backups/` as `<name> <unix millis>.isai`; the newest 20 are kept.
  Saving unchanged text adds nothing. `storage::list_backups` lists them, newest first.
- Not done: a restore screen in the app, and the audio is not backed up
  (only the project file).

---

## Earlier: 2026-10-14

State: **builds, launches with a clean log, clippy clean, all 120 tests pass** (`cargo test --workspace`).

## Phase 2, Step 1 (multitrack + projects): ✅ built

- Multiple tracks: add (+ or ⌥⌘N), rename (double-click), colour, mute/solo,
  volume (dB, double-click the readout for 0 dB), pan knob, delete (right-click)
- Regions: select, drag in time and across tracks with snap, delete (Backspace)
- Undo/redo (⌘Z / ⇧⌘Z); rapid fader/tempo moves merge into one step
- Save (⌘S), Save As (⇧⌘S), Open (⌘O), recent projects (chooser + project menu)
- Project = folder: `My Song/My Song.isai` + `Audio/` (imported audio is copied in on save)
- Autosave every 30 s while unsaved, recovery banner on the start screen
- "Save changes?" before New / Open / Recent / dropping a project / quitting
- Window drag on macOS now moves the window directly (was unreliable)

Not yet verified by hand: the native Open/Save dialogs, quitting with unsaved
work, crash recovery, and window dragging. Screenshots look right.

## Phase 2, Step 2 (editing + ⌘K): ✅ built

- Trim region edges (drag the left/right edge), keeps audio in place in time
- Split: ⌘T at the playhead, ⌥-click a region, the scissors button, or right-click
- Fades: drag the corner handles of a selected region (double-click to remove);
  equal-power curves drawn on the region; tiny automatic de-click fades on
  every edge; automatic crossfades where regions overlap on a track
- Zoom: ⌘+scroll at the mouse, ⌘= / ⌘−, toolbar buttons, Z to fit; horizontal
  and vertical scroll; the view follows the playhead; only visible bars/regions draw
- Reorder tracks by dragging their headers
- ⌘K command palette (also the search button in the control bar): every command,
  fuzzy search, plus smart commands from the query ("128 bpm", "bar 17", "3/4", "A minor")
- Dev scenes: `edit` (fades + split + two tracks), `palette` / `palette:<query>`

## Phase 2, Step 3 (recording): ✅ built

- Input device and channels in Settings → Audio, record-arm (R on a track),
  monitoring, latency compensation (2× buffer + manual offset), count-in,
  takes saved as 32-bit float WAV into the project's Audio folder.
- Not verified by hand with a real microphone (macOS permission prompt).

## FEATURES.md #1 Export: ✅ built

- ⌘E, project menu "Export Audio…", or ⌘K "Export audio…"
- Mix or stems (one file per audible track), whole song or cycle region,
  WAV 16/24/32-float or FLAC 16/24, 16-bit is dithered, optional normalize to −1 dB,
  progress + cancel, clip warning, "Show in Finder"
- Code: `engine/src/export.rs` (render + writers), `app/src/export_ops.rs`, `qml/pages/ExportDialog.qml`
- Dev scene: `ISAI_SCENE=export:<wav16|wav24|wav32f|flac16|flac24>:<file>`
- Status bar now tells good news (green check, fades after 4 s) from problems (red, stays)

## FEATURES.md #2 Copy/paste + multi-select: ✅ built

- Selection is a list of region ids (`State.selection`, QML `selectedRegions` JSON);
  `selectedRegion` stays the last-clicked one for single-region commands
- ⇧/⌘-click toggles, drag a box on empty lane space (⇧ adds), ⌘A selects all;
  clicking one region of a group selects just it (unless you drag)
- Drag any selected region and the whole selection moves (clamped to bar 1 and the track list)
- ⌘C / ⌘X / ⌘V (at the playhead, onto the selected track, adds tracks if needed),
  ⌘D duplicates after the group rounded to the snap grid, Backspace deletes all
- Also in the region right-click menu and ⌘K
- Code: `project/src/lib.rs` (Clipboard, regions_in, paste, duplicate_regions, move_regions),
  `app/src/selection_ops.rs`; dev scene `multi`
- Not verified by hand: marquee and group drag with a real mouse

## FEATURES.md #3a Mixer window: ✅ built

- Engine mixes in 512-frame blocks through `engine/src/mixer.rs` (`Mixer`), shared by
  playback and export (so they always match); per-track post-fader peaks
  (`Engine::take_track_peaks`). This is the base for insert effects and buses.
- Floating mixer panel (X, toolbar sliders button, ⌘K): a strip per track (pan, tapered
  fader with stereo meter, M/S), master strip (fader, meter, clip light), and a
  "Channel Effects" column (placeholder until #4). `qml/pages/MixerPanel.qml`, `components/VFader.qml`
- Fade/region/trim drags use `preventStealing` so the timeline no longer scrolls mid-drag
- New **Vivid** style (from the user's Mozart Dribbble reference): pure black, saturated
  warm track gradient, full-height regions, deeper same-hue waveforms
- **Glass**: `components/GlassPanel.qml` blurs the backdrop (ShaderEffectSource +
  MultiEffect, rounded mask needs maskThresholdMin 0.5 / maskSpreadAtMin 1.0) with
  tint, sheen and two glows (Theme.glowA/glowB). Used by the mixer and ⌘K. Solid when
  effects are off. Dev scene `mixer`.
- Mixer is docked along the bottom (aligned with the track area, 12 px margins), resizable
  by dragging its title bar; the arrangement gets `bottomInset` so tracks scroll above it
- **Glass everywhere**: dialogs (Settings, Export, Save changes) blur their page
  (`backdrop` property), popups/menus/dropdowns blur `Theme.backdrop` (window root),
  setup and project chooser use `Ambient.qml` (bg + colour glows) as the backdrop
- **Buses and sends** (#3b): `Project.buses`, `Track.sends` (post-fader, dB), engine
  `PlayBus` + `PlayTrack.sends`, mixed per block in `Mixer` (send levels ramp per block,
  bus meters after track meters). Mixer shows a send knob per bus on each strip
  (`SendKnob.qml`), bus strips (rename/delete on right-click), and a "+ Bus" tile.
  Stems keep their sends (all buses included). Buses are solo-safe.
- Still to do: the floating tool dock

## FEATURES.md #4 First effects: ✅ built

- `engine/src/effects/`: Channel EQ (low cut, low shelf, bell, high shelf; RBJ biquads),
  Compressor (soft knee, linked stereo), Reverb (Freeverb + pre-delay), Delay (tempo sync,
  tone, ping-pong, gliding time), Limiter (1.5 ms look-ahead, sliding-minimum gain, exact ceiling)
- Settings travel in the arrangement (`PlayInsert`); state lives in `EffectBank` on the audio
  thread keyed by insert id, so edits never cut tails. The app builds effects off the audio
  thread (`Audio::sync_effects`, commands `AddEffect`/`RemoveEffect`); export gets a fresh bank.
  Effects are cleared once when the transport goes quiet.
- Inserts on tracks and buses (pre-fader) and on the master. Project: `Insert`, `Chain`,
  `master_inserts`; bus reverb/delay start 100% wet. Stems skip master effects.
- Mixer: effect tags on strips (EQ/COMP/VERB/DLY/LIM), click a strip/bus/master to show its
  chain in Channel Effects (`EffectsPanel.qml`): bypass dot, open for knobs (`ParamKnob.qml`),
  EQ curve (`app.eqCurve`), right-click to move/remove, "Add effect" menu
- Fixed: mixer faders/knobs now move live while dragging (they show a live value)

## FEATURES.md #5 Automation: ✅ built (drawing; recording later)

- Project: `Track.automation: Vec<Lane{target, points[{beat,value}]}>`, `automation_view`;
  targets "volume", "pan", "send:<bus>", "fx:<insert>:<key>"; lanes go away with their bus/effect
- Engine: `PlayLane`/`AutoTarget`; the mixer reads lanes once per block (gains glide),
  effect params via `EffectBank::run_automated`; a send that only exists as automation still routes
- UI: "A" on a track header opens a lane under it (rows have variable height: `rowTops`),
  a picker chooses what it shows (• = has points), click to add, drag points (snapped),
  double-click/⌥-click to delete, Clear. Faders/knobs in headers and the mixer follow
  automation while playing (`Auto.qml` singleton). Dev scene `auto`.
- Not yet: recording automation by moving controls (latch/touch), bus/master automation

## FEATURES.md #6a MIDI, synth, piano roll: ✅ built

- Engine `instruments/`: Isai Synth (2 polyBLEP oscillators, Cytomic SVF low-pass with its
  own envelope, ADSR, 16 voices, stealing), `InstrumentBank` like `EffectBank`,
  `PlayTrack.instrument` + `notes` (samples). Live notes (`LiveNote`) via a queue shared
  by MIDI input threads and the UI; routed to the selected instrument track (`route_live`).
- The start/stop fade now applies to regions only (mixer `Block.fades`); master volume after
  the mix; effects keep running 3 s after everything goes quiet, so tails ring out.
- Project: `Track.instrument`, `Region.midi` (beats: length, offset, notes); MIDI regions use
  the same move/trim/split/copy/duplicate code; `add_instrument_track`, notes API.
- App: `midi_input.rs` opens every MIDI input (midir); ⌘K "Look for MIDI keyboards".
- UI: "+" offers Audio/Instrument track; double-click an instrument lane for a 1-bar region;
  MIDI regions draw their notes; double-click (or P) opens the **Piano Roll** dock (shares
  the bottom with the mixer): keyboard (click to hear), grid with snap, move/resize/delete
  notes, velocity lane, playhead, synth knobs, "Typing" (A W S E D…, Z/X octave).
  Dev scene `midi`.
- Window switcher (FL Studio style) in the track toolbar: Playlist F5, Piano Roll F7, Mixer F9
  (P and X still work). F7 opens the selected MIDI region, else the instrument track's region
  under the playhead, else creates a 4-bar region there (and an instrument track if needed).
- MIDI recording: Record with an instrument track selected (or the piano roll's red dot,
  or R while Typing) captures live notes. The processor tags each live note with the
  playhead (`SetCaptureNotes`, `played` queue → `Engine::take_played_notes`); the app's
  `MidiTake` (record_ops.rs) pairs on/off and, on stop, `Project::add_recorded_notes` makes a
  bar-aligned "Recording" region (one buffer of latency removed). Arming an instrument
  track never opens the microphone. Dev scene `midirec`.
- Quantize: `Project::quantize_notes(region, indices, grid, strength, ends)` on the song grid;
  piano-roll Quantize menu (Q starts, ⇧Q starts+ends, 50%, 25%) uses the roll's snap.
- Piano-roll selection: click / Shift- or ⌘-click / Shift- or ⌘-drag box / ⌘A / Esc;
  group drag (move, transpose, resize) and arrow-key nudges go through
  `Project::transform_notes` (one undo step, returns new indices); Delete →
  `remove_notes`. The arrangement's Delete and ⌘A step aside while the roll owns them.
  Dev scene `midisel`.
- Effect editors v3 (EffectEditor.qml): EQ styled like a modern plugin — per-band filled
  shapes (`Eq::band_curves`, `eq_curves`), lime combined curve, pre (filled) + post (line)
  analyzer (`Meters::pre_scope`, `effect_spectra`, smoothed in QML, Off/Post/Pre+Post),
  Q handles, floating band card, band chips, output drag value. Compressor styled like a
  hardware unit: amber VU needle (GR), metal knobs (static Canvas, rotating pointer),
  transfer curve on a grille, Peak/RMS · Auto · bypass keys. Static layers repaint only
  on resize/param change. MenuPopup forces its layout and flips upward when needed.
- Isai Piano (instruments/piano.rs, PianoWindow.qml): 9 tones (string/FM/bar models), two
  strings per note, stereo by key, body filter, hammer/damper noise, drive, sustain pedal
  (`SUSTAIN_PEDAL` live event; MIDI CC64; Tab / PED button; `app.sustain`), pedal resonance,
  built-in room, Light mode. Window: presets (built-in + user, `user_presets.rs` →
  presets.json), A/B, preview, Soft/Med/Hard + humanize, chord name, scale dots, Chord/Strum/
  Arp, 2–4 octaves, key letters, rising notes, velocity-coloured glow. Engine reports the
  live track's held/struck keys + velocities (`Engine::take_keys`, `instrument_keys`);
  StudioPage's `keyActivity` shares them with the piano roll keyboard.
- Isai Synth v2 (synth.rs, SynthWindow.qml): 2 osc × 6 waves, sub, noise, unison ×7 with
  detune/spread, LP/HP/BP SVF with drive/keytrack and its own ADSR, amp ADSR, glide, Poly/
  Mono/Legato, LFO (sine/tri/square/saw/random, fade-in delay) → pitch/filter/volume/pan,
  osc A octave + fine, sub −1/−2 oct, velocity, sustain pedal. Window: graphite "mono" design
  (white values, amber = LFO, blue = played keys), every control maps to one param with a
  hover tooltip; draggable filter curve + ADSR (graph + sliders, Amp/Filter tab), LFO view
  with live position, unison pitch/pan view, A/B, ◀ ▶ preset stepping, 22 presets + user
  presets; laid out at 1120×780 and scaled to fit. `MAX_INST_PARAMS` is 40.
  Dev scenes `piano`, `synth`.
- Not yet: sampler, drum machine / step sequencer, swing

## Effects v2 + piano roll fixes: ✅ built

- Piano roll: right-click deletes a note, right-drag erases; its own snap menu (Off, Bar,
  1/2, 1/4, 1/8, 1/16, 1/32, triplets), separate from the tracks' snap
- `MAX_PARAMS` is 32. EQ is 8 bands (low cut with 6/12/24/48 dB/oct Butterworth slopes,
  low shelf, 4 bells, high shelf, high cut, output) — **old EQ settings load as flat**.
  Compressor: auto gain, peak/RMS, sidechain low cut. Reverb: 8-line FDN (Hadamard,
  RT60-exact decay, damping, modulation, diffusion, low cut) — old "damping" % now Hz.
  Limiter and compressor report gain reduction.
- `effects/meters.rs`: per-effect atomics (in/out peaks, GR, 2048-sample scope); the app
  keeps an `Arc<Meters>` per effect (`Audio::meters`), QML polls `effectMeters(id)` and
  `effectSpectrum(id, bands)` (`app/src/spectrum.rs` FFT).
- `EffectEditor.qml`: floating glass editor window (double-click an effect, or its sliders
  button): EQ graph with spectrum + draggable band points (scroll = Q/slope, double-click =
  on/off or add a bell), compressor transfer curve with live level + GR meter, limiter GR,
  big knobs for the rest, IN/OUT meters. Dev scenes `fxeq`, `fxcomp`.

## Next: FEATURES.md in order
1. ~~Export~~ ✅
2. ~~Copy / cut / paste / duplicate (⌘D) and multi-select regions~~ ✅
3. ~~Mixer window, buses/sends~~ ✅ (inserts come with effects)
4. ~~EQ, compressor, reverb, delay, limiter~~ ✅
5. ~~Automation~~ ✅ (drawing)
6. MIDI + piano roll ✅, then MIDI recording, quantize, sampler and drum machine
7. Time-stretch ("Follow tempo"), markers, tempo changes

## From tofix.txt

- Window drag when not maximized: fixed for macOS (manual move); needs a check by hand.
- Fade handle: now a round knob on the curve with a length readout; fades
  survived a scripted 3× zoom. Ask the user how they zoomed if it still resets.
- "Changing tempo should speed up / slow down the song": fixed. Audio regions without
  their own tempo are pinned to the tempo they play at (`Project::pin_audio_tempo`, run by
  `set_tempo_at`), so the next tempo change stretches them (WSOLA, `engine::stretch`, made
  in the background). MIDI already followed. Until a stretch is ready, the region plays at its
  recorded speed for a moment.

## Main window
- Plain `Qt.Window` on every platform with the system title bar. Drawing under the macOS
  title bar (`ExpandedClientAreaHint | NoTitleBarBackgroundHint`) left the NSWindow
  unmovable (isMovable = NO): it couldn't be dragged, moved to other Spaces or displays.
  Empty top-bar space still moves the window (`WindowDragArea`: startSystemMove, falling
  back to following the mouse; double-click maximises).

## Tools batch (markers → mastering)
- Markers: `Project.markers`, ruler marker lane (double-click / M, drag, rename, cycle section),
  ⌥←/→. Undo history panel (`History::undo_labels`, `jump_history`). `stop()` = stop, then start.
- Regions: `gain_db`, `reverse`, `loop_to` (span via `region_span`; loops written out in
  `session.arrangement` / `play_notes`), `join_regions`, normalize, and `tempo` (follow the song
  tempo: `region_meter`, `stretch_ratio`; `AudioPool::request_stretch` makes a WSOLA copy with
  `engine::stretch`). Tempo detection: `slicer::tempo`.
- Piano roll: chord stamps (incl. diatonic "in key"), strum, arpeggiate, swing (quantize `swing`),
  scale rows, ghost notes. Shortcuts overlay (?). Tuner (`engine.input_scope` + `tuner.rs` YIN).
- Effects (engine/effects): chorus, flanger, phaser (`modulation.rs`), distortion, bitcrusher
  (`character.rs`), gate, de-esser (LR4 split, `dynamics.rs`), Pitch Correct (`pitch.rs`; starts in
  the song key via `project::parse_key`). Generic knob editor; add menu grouped.
- Instruments: Isai Drums (`drums.rs`, synthesised kits, DrumWindow step sequencer), Isai Sampler
  (`sampler.rs`, Keys/Slices; `Instrument.sample` + `slices`; saved into the project folder;
  `sampler_ops.rs`, `slicer.rs`). `MAX_INST_PARAMS` is 48; `Instrument::render` takes `&PlayInstrument`.
- Browser (BrowserPanel, B): FolderListModel + `find_sounds`; previews via `Engine::preview` (a voice
  after the mix, `sync` waits for the bar); `import_at` drops onto a lane/beat.
- Eco mode (`settings.audio.eco`, `session::eco_params`), CPU % in the status bar. Freeze
  (`Track.frozen`, `freeze_track` renders the track pre-fader with export's renderer).
- Mastering assistant (`mastering.rs`: BS.1770 LUFS, plan EQ/comp/limiter, MasteringDialog).
- Dev scenes: markers, tuner, rolltools, drums, fxdist, sampler, browser, master, midifreeze, stretch.
- Not yet: tempo/time-signature changes over time, third-party plugins (VST3/CLAP/AU), stem splitter.

## Meadow look: start screen, Studio theme, Inspector
- **Palette** (Theme.qml `classicTrackColors`): the saved keys keep their old
  names (blue = lake, green = leaf, yellow = sun, orange = peach, red = clay,
  pink = salmon, violet = dusk, teal = forest). Studio's default accent is
  `forest`. Only Studio uses it; Vivid and Midnight are unchanged.
- **Regions** in Studio (`Theme.tintedRegions`): solid header in the track
  colour with dark text, body `Theme.regionBody(c)`, waveform and notes in
  `Theme.regionInk(c)`. RegionItem's `headInk`/`bodyInk` pick per style.
- **Start screen** (ProjectChooser.qml): fixed forest and cream colours in every
  theme, a random painting from `assets/art/` and a random poster line, Return
  creates the song. `ArtButton` is the cream/outline button used there. The
  Logo has `ink: "light"` to force white ink.
- **Art**: `tools/paint_art.py` (Pillow) paints them; the docstring lists the
  exact commands for the shipped files. build.rs bundles `assets/art`.
- **Track headers**: three rows (name; A R 🎧 M S + dB; fader, pan, meter).
  Lanes are 90 px. Levels come from `app.trackMeters` (TrackArea.meters).
- **Inspector** (InspectorPanel.qml, I key, remembered in QSettings
  "studio/inspectorShown"): reuses EffectsPanel, VFader, PanKnob, MiniToggle.
- **Play button**: compact console button in Studio (no pulse).
- **Website** (docs/): same palette, Young Serif headings, paintings as
  backgrounds. Screenshots come from the dev scenes in `cfg-studio`.

## Tempo and time-signature changes
- **Model**: `Project.tempo_changes` ([TempoChange{beat,bpm}], after the start;
  `tempo` is the opening one) and `signature_changes` ([SignatureChange{bar,
  numerator,denominator}], from bar 2; `numerator/denominator` is the opening
  one and the denominator stays the song's beat unit). `Project::tempo_map()`.
- **engine::time::TempoMap**: steps of tempo, sections of signature;
  beats↔secs↔samples, `bar_beat`, `bar_start`, `next_click`, `meter_at`.
- **Audio thread**: `Command::SetTempo(Box<TempoMap>)` (old one goes back as
  garbage). The metronome schedules each click from `next_click` (no per-sample
  maths); count-in uses the meter at the playhead; previews wait for the next bar.
  `Arrangement.tempo_changes` (samples) gives tempo-synced effects the local BPM.
- **Regions**: audio not following the tempo keeps its speed through changes
  (`region_beats` measures it on the map). Trim/split of audio crossing a change
  still use the tempo at its start (small error; fix if users notice).
- **UI**: `TimeMap` singleton (fed `app.tempoJson`) for bar lines everywhere.
  Ruler lanes (G): signature chips + popup, tempo step line with draggable
  points. The tempo/signature LCD shows the values at the playhead; editing it
  changes that section (`set_tempo_at`, `set_signature_at`).
- Dev scene `tempo`.

## Third-party plugins (VST3, CLAP, Audio Units)
- **engine::plugin**: `ExternalPlugin` trait (process a block with timed
  `NoteEvent`s, `wait` false on the audio thread = `try_lock`, skip the block if
  busy). `PluginEffect` and `PluginInstrument` (turns song notes and live notes
  into events; releases held notes on stop/jump). `EffectKind::Plugin` and
  `InstrumentKind::Plugin` (not in `ALL`, so not in Isai's own menus).
  `PlayInsert.plugin` / `PlayInstrument.plugin` carry the handle; export banks
  build plugin effects with `wait = true`.
- **crates/plugins** (`isai-plugins`): scan (`vst3_host::simple::discover_plugins`),
  `Vst3Plugin` = UI-side owner (window) + `Core` (Arc, `Mutex<Plugin>`, bus
  buffers resized per block within capacity). State as base64 in the song.
- **app/plugin_ops.rs**: catalog cached in `plugins.json` next to settings;
  rescans run `isai --scan-plugins <file>` as a child process (plugins print to
  stdout, so results go to a file). `ensure()` loads every plugin the song uses
  before each arrangement push; `attach()` gives the arrangement its handles;
  `store_states()` before save/autosave. `pollPlugins` runs from a QML Timer
  while scanning or a plugin window is open.
- **Project**: `PluginSlot {format, path, uid, name, vendor, state}` on
  `Insert.plugin` / `Instrument.plugin` (kind "plugin").
- **UI**: plugin effects at the end of each add-effect menu, plugin instruments
  in the new-track menu; clicking a plugin insert or instrument opens its window.
- Dev scene `plugin` (needs Pigments or any instrument plugin installed).
- **CLAP** (`plugins/src/clap.rs`, clack-host): scan of the standard folders and
  `CLAP_PATH`; the started audio processor lives in `Core` (Mutex<Option<Audio>>,
  preallocated ports/buffers/events); `ClapPlugin` (UI thread) owns the instance,
  state via the state extension, GUI embedded in our own NSWindow on macOS or the
  plugin's floating window elsewhere (untested: needs a real CLAP plugin with a GUI).
  Drop takes the processor back and deactivates on the main thread.
- Test plugin: `cargo build -p isai-plugins --example test_clap`, then
  `cargo run -p isai-plugins --example probe -- --clap target/debug/examples/libtest_clap.dylib`;
  in the app: `CLAP_PATH=<folder with it renamed .clap> ISAI_SCENE="plugin:Isai Test Tone"`.
- **Audio Units** (`plugins/src/au.rs`, macOS): Apple's v2 C API through
  objc2-audio-toolbox. Scan = AudioComponentFindNext for aumu/aufx/aumf (no
  loading); uid "type:subt:manu". Float32 non-interleaved stereo; effects pull
  input through a render callback reading boxed `Inputs` (UnsafeCell). MIDI via
  MusicDeviceMIDIEvent. State = ClassInfo as a binary plist. Window = the unit's
  CocoaUI view, else CoreAudioKit's AUGenericView, in `macwin::HostWindow`.
  Test: `cargo run -p isai-plugins --example probe -- --au`; app scene
  `plugin:DLSMusicDevice`.
- Not yet: plugin parameter automation, latency compensation, transport info for CLAP/AU, AUv3-only units.

## Stem splitter
- **crates/stems** (`isai-stems`): Spleeter 4-stem (Deezer, MIT) run natively.
  `model::convert_release` checks Deezer's 4stems.tar.gz against its SHA-256,
  reads the TensorFlow checkpoint (SSTable index + BundleEntryProto, no TF) and
  writes `ISAISTEMS2` (JSON header + f16 weights, 79 MB). `unet.rs`: the U-Net
  (ELU, skip connections from raw conv outputs, decoder act-then-BN, sigmoid
  mask × input) with im2col + matrixmultiply, threads by output rows.
  `lib.rs`: 4096/1024 STFT at 44.1 kHz with a frame of leading silence,
  512-frame segments, ratio masks (out² / Σout²), zero above bin 1024, iSTFT ×2/3,
  streamed out per segment.
- Checks: conv/transposed-conv tests against direct definitions, STFT round trip;
  `cargo run --release -p isai-stems --example split -- 4stems.tar.gz [song.wav]`
  (a bass line → 99.8% Bass, a drum loop → 93% Drums; speech in a mix → Vocals
  correlates 0.90 with the voice).
- **app/stems_ops.rs**: first split downloads with the system `curl` into
  `<app dir>/models/` (progress from the file size), converts, deletes the
  archive. Splits on a thread (cores − 1), writes four float WAVs at 44.1 kHz
  into the project's Audio folder (or `<app dir>/Stems`), then
  `Project::add_stem_tracks` adds coloured tracks below and mutes the original.
  Region menu "Split into Stems…", ⌘K, StemsDialog, status bar progress. Scene `stems`.

## Interface size

- Settings `ui_scale` (percent, 75–200, default 100): a look setting, not part of a profile.
  `controller.rs` exposes it as `uiScale` (`set_int`, clamped).
- **Main.qml**: `root` is laid out at window size ÷ scale and drawn with `scale` from the
  top-left, so every page scales with no per-control changes. Popups attached to the window
  overlay (dropdown menus) stay at 100%.
- Settings › Appearance › "Interface size" (Segmented 75/100/125/150/200%).
- Test: `cargo test -p isai-settings` (default is 100).


## Video for scoring

- Model: `Project.videos` (`VideoClip { id, name, file, start }`, `start` in beats, serde default).
  `add_video` / `remove_video`; save copies each video into `Video/` (`ProjectPaths::video_dir`,
  `resolve_video`), same as audio.
- Controller: `videos_json` (published with the project; `path`, `start` in seconds, `beat`),
  `import_video(url)` at the playhead and `remove_video(id)`, both undoable.
- Player: `crates/app/qml/pages/VideoWindow.qml`, a separate `Window` with Qt Multimedia
  (`MediaPlayer` + `VideoOutput`). It plays the clip under the playhead and follows `app.playing`
  and `app.position`, correcting drift over 150 ms. Muted by default. Docks to the left, right,
  top or bottom of the main window (buttons, or drop it within 28 px of an edge). Keep on top.
- Formats: every platform plays through Qt's FFmpeg backend (`main.rs` sets
  `QT_MEDIA_BACKEND=ffmpeg` before startup). Headless probe (MediaPlayer, FFmpeg 8.1) opened
  MP4, HEVC MP4, MOV, MKV, WebM, AVI, WMV, FLV, MPEG-PS, M2TS, Ogg/VP8, 3GP, MXF and M4V.
  Probe clips: generated with ffmpeg; the probe is in the scratchpad, not the repo.
- macOS Qt: Homebrew's qtmultimedia has no FFmpeg backend, so it was rebuilt from the official
  6.12.0 source with FFmpeg 8.1 (`qt-cmake`, `Qt6ShaderTools_DIR` set, `PKG_CONFIG_PATH` to
  `/opt/homebrew/opt/ffmpeg/lib/pkgconfig`). That build is in `/opt/homebrew/Cellar/qtmultimedia/6.12.0_1`,
  so `brew reinstall qtmultimedia` will replace it with the plain bottle.
- Build: `build.rs` links `Multimedia`; the packaging scripts and the Windows and Linux workflow
  install `qtmultimedia`.
