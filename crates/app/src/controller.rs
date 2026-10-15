//! The bridge between the QML interface and the Rust engine and settings.
//!
//! QML reads plain properties and calls invokables; all logic lives here.
//! While something is moving, QML calls `tick()` at the user's chosen frame
//! rate; when nothing moves, the timer stops and the app uses no CPU.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
        include!("cxx-qt-lib/qurl.h");
        type QUrl = cxx_qt_lib::QUrl;
    }

    #[auto_cxx_name]
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        // First-launch setup
        #[qproperty(bool, needs_setup)]
        #[qproperty(QString, setup_notice)]
        #[qproperty(bool, checking)]
        #[qproperty(QString, cpu_name)]
        #[qproperty(i32, cores)]
        #[qproperty(f64, ram_gb)]
        #[qproperty(i32, voices)]
        #[qproperty(QString, recommended)]
        // Settings
        #[qproperty(QString, profile)]
        #[qproperty(QString, view)]
        #[qproperty(QString, theme)]
        #[qproperty(QString, ui_style)]
        #[qproperty(QString, accent)]
        #[qproperty(QString, animations)]
        #[qproperty(i32, ui_fps)]
        #[qproperty(i32, meter_hz)]
        #[qproperty(QString, waveform_detail)]
        #[qproperty(bool, effects)]
        /// Eco mode: instruments in their lightest quality while you work.
        #[qproperty(bool, eco)]
        #[qproperty(i32, buffer_size)]
        #[qproperty(QString, device_id)]
        #[qproperty(bool, suggest_lighter)]
        #[qproperty(QString, settings_path)]
        #[qproperty(QStringList, device_names)]
        #[qproperty(QStringList, device_ids)]
        // Audio engine
        #[qproperty(bool, audio_ok)]
        #[qproperty(QString, audio_error)]
        #[qproperty(QString, device_name)]
        #[qproperty(i32, sample_rate)]
        #[qproperty(i32, actual_buffer)]
        #[qproperty(f64, cpu_load)]
        #[qproperty(bool, crackling)]
        // Transport
        #[qproperty(bool, loading)]
        #[qproperty(f64, position)]
        #[qproperty(QString, position_text)]
        #[qproperty(bool, playing)]
        // Tracks: a JSON array the arrangement view renders (see session.rs)
        #[qproperty(QString, tracks_json)]
        /// Buses for the mixer, as a JSON array.
        #[qproperty(QString, buses_json)]
        /// The master's effects, as a JSON array (same shape as a track's "inserts").
        #[qproperty(QString, master_inserts_json)]
        /// Every built-in effect and its parameters, as JSON (fixed).
        #[qproperty(QString, effect_kinds_json)]
        /// Every built-in instrument and its parameters, as JSON (fixed).
        #[qproperty(QString, instrument_kinds_json)]
        /// Connected MIDI keyboards, comma-separated ("" when none).
        #[qproperty(QString, midi_inputs)]
        #[qproperty(i32, track_count)]
        #[qproperty(i32, selected_track)]
        #[qproperty(i32, selected_region)]
        /// Every selected region id, as a JSON array; `selected_region` is the last one clicked.
        #[qproperty(QString, selected_regions)]
        #[qproperty(i32, selection_count)]
        /// True when there are copied regions to paste.
        #[qproperty(bool, can_paste)]
        // Project file
        #[qproperty(bool, dirty)]
        #[qproperty(QString, project_path)]
        #[qproperty(QString, undo_label)]
        #[qproperty(QString, redo_label)]
        /// The undo history: { "undo": [labels, oldest first], "redo": [next first] }.
        #[qproperty(QString, history_json)]
        /// Markers: [{ id, beat, name }], sorted by beat.
        #[qproperty(QString, markers_json)]
        /// The tempo and time-signature map: {"tempos": [[beat, bpm]…], "sigs": [[beat, bar, num, den]…]}.
        #[qproperty(QString, tempo_json)]
        /// Installed plugins: [{ index, name, vendor, format, instrument, category }].
        #[qproperty(QString, plugins_json)]
        #[qproperty(bool, plugin_scanning)]
        /// How many plugin windows are open.
        #[qproperty(i32, plugin_windows)]
        /// The region being split into stems (-1: none), how far along (0..1), and what it's doing.
        #[qproperty(i32, stem_region)]
        #[qproperty(f64, stem_progress)]
        #[qproperty(QString, stem_stage)]
        /// The tuner is open (and listening to the input).
        #[qproperty(bool, tuner_open)]
        /// The browser is playing a preview.
        #[qproperty(bool, previewing)]
        #[qproperty(QStringList, recent_paths)]
        #[qproperty(QStringList, recent_names)]
        // Export
        #[qproperty(bool, exporting)]
        #[qproperty(f64, export_progress)]
        /// The track being frozen (-1 when none), and how far along.
        #[qproperty(i32, freezing_track)]
        /// The mastering assistant is measuring the mix; its last report (JSON,
        /// "" until there is one): { target, before, after, changes }.
        #[qproperty(bool, mastering)]
        #[qproperty(QString, master_report)]
        #[qproperty(f64, freeze_progress)]
        /// Folder of the last export, for "Show in Finder".
        #[qproperty(QString, export_folder)]
        // Recording
        #[qproperty(bool, recording)]
        #[qproperty(bool, input_ok)]
        #[qproperty(QString, input_name)]
        #[qproperty(QString, input_error)]
        #[qproperty(f64, input_level_l)]
        #[qproperty(f64, input_level_r)]
        #[qproperty(QStringList, input_device_names)]
        #[qproperty(QStringList, input_device_ids)]
        #[qproperty(QString, input_device_id)]
        #[qproperty(bool, input_stereo)]
        #[qproperty(i32, input_channel)]
        #[qproperty(i32, latency_offset_ms)]
        /// Name of an autosaved project that can be recovered ("" if none).
        #[qproperty(QString, recovery_name)]
        #[qproperty(QString, recovery_age)]
        // Project and musical time
        #[qproperty(bool, project_open)]
        #[qproperty(QString, project_name)]
        #[qproperty(f64, tempo)]
        #[qproperty(i32, time_sig_num)]
        #[qproperty(i32, time_sig_den)]
        #[qproperty(QString, key_name)]
        /// Playhead in beats (one note of the time signature's denominator).
        #[qproperty(f64, position_beats)]
        /// Playhead as "bar beat division tick", e.g. "5 2 1   0".
        #[qproperty(QString, bar_text)]
        #[qproperty(f64, length_beats)]
        #[qproperty(bool, metronome)]
        #[qproperty(bool, count_in)]
        #[qproperty(bool, counting_in)]
        #[qproperty(bool, cycle)]
        #[qproperty(f64, cycle_start)]
        #[qproperty(f64, cycle_end)]
        #[qproperty(bool, snap)]
        #[qproperty(QString, snap_value)]
        #[qproperty(i32, click_volume)]
        #[qproperty(i32, count_in_bars)]
        #[qproperty(f64, volume)]
        #[qproperty(f64, meter_l)]
        #[qproperty(f64, meter_r)]
        /// Each track's level for the mixer: a JSON array of [left, right], 0..1 on the
        /// same scale as `meter_l`, in track order.
        #[qproperty(QString, track_meters)]
        /// The master went over 0 dB since the last `clear_clip`.
        #[qproperty(bool, master_clip)]
        #[qproperty(QString, status)]
        /// True when `status` is good news (saved, exported) rather than a problem.
        #[qproperty(bool, status_ok)]
        /// True while something changes over time and QML should keep calling `tick`.
        #[qproperty(bool, busy)]
        /// Developer tools: `ISAI_SCREENSHOT=out.png` saves a picture of the window
        /// and quits; `ISAI_SCENE` picks what to show first (see Main.qml).
        #[qproperty(QString, dev_screenshot)]
        #[qproperty(QString, dev_scene)]
        type AppController = super::AppControllerRust;

        /// Call once when the window is ready. `software_rendering` is true when
        /// Qt draws without a GPU, which forces the lightest visuals.
        #[qinvokable]
        fn start(self: Pin<&mut Self>, software_rendering: bool);
        #[qinvokable]
        fn tick(self: Pin<&mut Self>);

        #[qinvokable]
        fn prepare_audio(self: Pin<&mut Self>);
        /// Saves the choices made so far, so setup resumes if the app closes.
        #[qinvokable]
        fn save_setup_progress(self: Pin<&mut Self>);
        #[qinvokable]
        fn finish_setup(self: Pin<&mut Self>);
        #[qinvokable]
        fn rerun_setup(self: Pin<&mut Self>);

        #[qinvokable]
        fn set_text(self: Pin<&mut Self>, key: &QString, value: &QString);
        #[qinvokable]
        fn set_int(self: Pin<&mut Self>, key: &QString, value: i32);
        #[qinvokable]
        fn set_bool(self: Pin<&mut Self>, key: &QString, value: bool);
        #[qinvokable]
        fn refresh_devices(self: Pin<&mut Self>);
        #[qinvokable]
        fn retry_audio(self: Pin<&mut Self>);
        #[qinvokable]
        fn test_tone(self: Pin<&mut Self>);
        #[qinvokable]
        fn switch_to_lightweight(self: Pin<&mut Self>);
        #[qinvokable]
        fn dismiss_crackle(self: Pin<&mut Self>);

        /// Imports an audio file onto a new track at the playhead.
        #[qinvokable]
        fn import_url(self: Pin<&mut Self>, url: &QUrl);
        #[qinvokable]
        fn add_track(self: Pin<&mut Self>);
        #[qinvokable]
        fn remove_track(self: Pin<&mut Self>, id: i32);
        #[qinvokable]
        fn rename_track(self: Pin<&mut Self>, id: i32, name: &QString);
        #[qinvokable]
        fn set_track_color(self: Pin<&mut Self>, id: i32, color: &QString);
        #[qinvokable]
        fn toggle_mute(self: Pin<&mut Self>, id: i32);
        #[qinvokable]
        fn toggle_solo(self: Pin<&mut Self>, id: i32);
        /// While dragging, call with `commit` false; once more with true on release.
        #[qinvokable]
        fn set_track_volume(self: Pin<&mut Self>, id: i32, db: f64, commit: bool);
        #[qinvokable]
        fn set_track_pan(self: Pin<&mut Self>, id: i32, pan: f64, commit: bool);
        #[qinvokable]
        fn select_track(self: Pin<&mut Self>, id: i32);
        #[qinvokable]
        fn select_region(self: Pin<&mut Self>, id: i32);
        #[qinvokable]
        fn move_region(self: Pin<&mut Self>, id: i32, start: f64, track: i32);
        #[qinvokable]
        fn delete_selected(self: Pin<&mut Self>);
        #[qinvokable]
        fn add_bus(self: Pin<&mut Self>);
        #[qinvokable]
        fn remove_bus(self: Pin<&mut Self>, id: i32);
        #[qinvokable]
        fn rename_bus(self: Pin<&mut Self>, id: i32, name: &QString);
        #[qinvokable]
        fn set_bus_volume(self: Pin<&mut Self>, id: i32, db: f64, commit: bool);
        #[qinvokable]
        fn set_bus_pan(self: Pin<&mut Self>, id: i32, pan: f64, commit: bool);
        #[qinvokable]
        fn toggle_bus_mute(self: Pin<&mut Self>, id: i32);
        /// How much of a track goes to a bus, in dB.
        #[qinvokable]
        fn set_send(self: Pin<&mut Self>, track: i32, bus: i32, db: f64, commit: bool);
        /// Adds an effect to a chain: `chain` is "track", "bus" or "master"
        /// (`owner` is the track or bus id). Returns the new effect's id, or -1.
        #[qinvokable]
        fn add_insert(self: Pin<&mut Self>, chain: &QString, owner: i32, kind: &QString) -> i32;
        #[qinvokable]
        fn remove_insert(self: Pin<&mut Self>, id: i32);
        /// Sets several of an effect's parameters at once ({ key: value } JSON),
        /// as one undo step named `label` (e.g. loading a preset).
        #[qinvokable]
        fn set_insert_params(self: Pin<&mut Self>, id: i32, values: &QString, label: &QString);
        /// Adds an instrument track ("synth") and selects it.
        #[qinvokable]
        fn add_instrument_track(self: Pin<&mut Self>, kind: &QString);
        /// Adds an empty MIDI region; returns its id, or -1.
        #[qinvokable]
        fn add_midi_region(self: Pin<&mut Self>, track: i32, start: f64, length: f64) -> i32;
        /// Notes, by region id and index; `start` is in beats from the region's start.
        #[qinvokable]
        fn add_note(self: Pin<&mut Self>, region: i32, start: f64, length: f64, key: i32, velocity: i32) -> i32;
        #[qinvokable]
        fn update_note(self: Pin<&mut Self>, region: i32, index: i32, start: f64, length: f64, key: i32, velocity: i32) -> i32;
        #[qinvokable]
        fn remove_note(self: Pin<&mut Self>, region: i32, index: i32);
        /// Moves/transposes/resizes several notes together (one undo step).
        /// `indices` is comma-separated; returns their new indices the same way.
        #[qinvokable]
        fn transform_notes(self: Pin<&mut Self>, region: i32, indices: &QString, d_start: f64, d_key: i32, d_length: f64) -> QString;
        #[qinvokable]
        fn remove_notes(self: Pin<&mut Self>, region: i32, indices: &QString);
        /// Moves notes toward a grid (in beats). `indices` is a comma-separated
        /// list, or "" for every note; `strength` is 0..1.
        /// `swing` 0..0.5 delays every second grid step (1/3 is a triplet feel).
        #[qinvokable]
        fn quantize_notes(self: Pin<&mut Self>, region: i32, indices: &QString, grid: f64, strength: f64, ends: bool, swing: f64);
        /// Adds notes ([[start, length, key, velocity]…] JSON, starts in beats
        /// from the region's start) as one edit; returns their indices, comma-separated.
        #[qinvokable]
        fn add_notes(self: Pin<&mut Self>, region: i32, notes: &QString) -> QString;
        /// Strums the chords among `indices` (every note when ""): `spread` beats
        /// between neighbouring notes, upwards or `down`. Returns the new indices.
        #[qinvokable]
        fn strum_notes(self: Pin<&mut Self>, region: i32, indices: &QString, spread: f64, down: bool) -> QString;
        /// Turns chords into arpeggios of `step` beats ("up", "down", "updown").
        #[qinvokable]
        fn arpeggiate_notes(self: Pin<&mut Self>, region: i32, indices: &QString, step: f64, pattern: &QString) -> QString;
        /// Adds a marker at `beats` (snapped); returns its id.
        #[qinvokable]
        fn add_marker(self: Pin<&mut Self>, beats: f64) -> i32;
        #[qinvokable]
        fn move_marker(self: Pin<&mut Self>, id: i32, beats: f64);
        #[qinvokable]
        fn rename_marker(self: Pin<&mut Self>, id: i32, name: &QString);
        #[qinvokable]
        fn remove_marker(self: Pin<&mut Self>, id: i32);
        /// Moves the playhead to the next marker (`forward`) or the previous one.
        #[qinvokable]
        fn jump_marker(self: Pin<&mut Self>, forward: bool);
        /// Cycles the section a marker starts, up to the next marker.
        #[qinvokable]
        fn cycle_marker(self: Pin<&mut Self>, id: i32);
        /// A region's volume change in dB; `commit` false while dragging.
        #[qinvokable]
        fn set_region_gain(self: Pin<&mut Self>, id: i32, db: f64, commit: bool);
        /// Plays the selected audio regions backwards (or forwards again).
        #[qinvokable]
        fn reverse_selected(self: Pin<&mut Self>);
        /// Turns the selected audio regions up or down so their loudest peak is at −1 dB.
        #[qinvokable]
        fn normalize_selected(self: Pin<&mut Self>);
        /// Repeats a region's contents until `end` beats (at or before its end: no loop).
        #[qinvokable]
        fn set_region_loop(self: Pin<&mut Self>, id: i32, end: f64);
        /// The tempo an audio region sounds like it's at, in BPM (0 when there's no clear beat).
        #[qinvokable]
        fn detect_region_tempo(self: Pin<&mut Self>, region: i32) -> f64;
        /// Makes an audio region follow the song's tempo, its audio being at
        /// `bpm` (time-stretched to fit); 0 stops it following.
        #[qinvokable]
        fn set_region_tempo(self: Pin<&mut Self>, region: i32, bpm: f64);
        /// Joins the selected regions (on one track) into one.
        #[qinvokable]
        fn join_selected(self: Pin<&mut Self>);
        /// Undoes (negative) or redoes (positive) several steps at once.
        #[qinvokable]
        fn jump_history(self: Pin<&mut Self>, steps: i32);
        /// Stops; when already stopped, goes back to the start.
        #[qinvokable]
        fn stop(self: Pin<&mut Self>);
        /// Opens or closes the tuner; while open it listens to the input.
        #[qinvokable]
        fn set_tuner(self: Pin<&mut Self>, open: bool);
        /// What the tuner hears, with A4 at `a4` Hz: { level, hz, note, octave,
        /// cents, clarity } (no "hz" when there is no clear pitch).
        #[qinvokable]
        fn tuner_reading(self: Pin<&mut Self>, a4: f64) -> QString;
        #[qinvokable]
        fn set_instrument_param(self: Pin<&mut Self>, track: i32, key: &QString, value: f64, commit: bool);
        /// Sets several instrument parameters ({ key: value } JSON) as one undo step.
        #[qinvokable]
        fn set_instrument_params(self: Pin<&mut Self>, track: i32, values: &QString, label: &QString);
        /// Plays a note on the selected instrument track (computer keyboard, piano roll).
        #[qinvokable]
        fn note_on(self: Pin<&mut Self>, key: i32, velocity: i32);
        #[qinvokable]
        fn note_off(self: Pin<&mut Self>, key: i32);
        /// The sustain pedal on the selected instrument track (down or up).
        #[qinvokable]
        fn sustain(self: Pin<&mut Self>, down: bool);
        /// Keys the selected instrument is playing, for lighting up keyboards:
        /// { "held": [notes], "struck": [notes struck since the last call] }.
        #[qinvokable]
        fn instrument_keys(self: Pin<&mut Self>) -> QString;
        /// Presets the user saved for an instrument or effect kind ("piano"…):
        /// a JSON array of { name, values }.
        #[qinvokable]
        fn user_presets(self: &AppController, kind: &QString) -> QString;
        /// Saves a preset ({ key: value } JSON), replacing one with the same name.
        #[qinvokable]
        fn save_user_preset(self: Pin<&mut Self>, kind: &QString, name: &QString, values: &QString);
        #[qinvokable]
        fn delete_user_preset(self: Pin<&mut Self>, kind: &QString, name: &QString);
        /// Plays an audio file by itself (the browser). With `sync` while
        /// playing, it starts on the next bar.
        #[qinvokable]
        fn preview_file(self: Pin<&mut Self>, url: &QUrl, sync: bool);
        #[qinvokable]
        fn stop_preview(self: Pin<&mut Self>);
        /// Imports an audio file onto a track at `beats` (a new track when `track` is -1
        /// or an instrument track).
        #[qinvokable]
        fn import_at(self: Pin<&mut Self>, url: &QUrl, track: i32, beats: f64);
        /// Sound files under a folder whose names contain `query` (any order of
        /// words), searching subfolders: [{ name, path, folder, kind }], at most 300.
        #[qinvokable]
        fn find_sounds(self: &Self, folder: &QUrl, query: &QString) -> QString;
        /// Loads an audio file into a track's sampler.
        #[qinvokable]
        fn load_sample(self: Pin<&mut Self>, track: i32, url: &QUrl);
        /// Chops the sampler's sound into slices: "even", or "beats" (where hits
        /// start), up to `count` of them.
        #[qinvokable]
        fn slice_sample(self: Pin<&mut Self>, track: i32, how: &QString, count: i32);
        /// Slice start points in seconds ([…] JSON); `commit` false while dragging.
        #[qinvokable]
        fn set_slices(self: Pin<&mut Self>, track: i32, slices: &QString, commit: bool);
        /// Writes a MIDI region that plays every slice in order, in time. Returns its id.
        #[qinvokable]
        fn slices_to_pattern(self: Pin<&mut Self>, track: i32) -> i32;
        /// Looks for MIDI keyboards again (after plugging one in).
        #[qinvokable]
        fn refresh_midi(self: Pin<&mut Self>);
        /// Shows an automation lane under a track ("" hides it). Not an undo step.
        #[qinvokable]
        fn set_automation_view(self: Pin<&mut Self>, track: i32, target: &QString);
        /// Adds a point to a lane; returns its index, or -1.
        #[qinvokable]
        fn add_auto_point(self: Pin<&mut Self>, track: i32, target: &QString, beat: f64, value: f64) -> i32;
        #[qinvokable]
        fn move_auto_point(self: Pin<&mut Self>, track: i32, target: &QString, index: i32, beat: f64, value: f64);
        #[qinvokable]
        fn remove_auto_point(self: Pin<&mut Self>, track: i32, target: &QString, index: i32);
        #[qinvokable]
        fn clear_lane(self: Pin<&mut Self>, track: i32, target: &QString);
        /// An effect's live levels since the last call: { in: [l, r], out: [l, r], gr }
        /// (levels 0..1 on the meter scale, gr = gain reduction in dB ≤ 0).
        #[qinvokable]
        fn effect_meters(self: Pin<&mut Self>, id: i32) -> QString;
        /// The spectrum of an effect's output: `bands` levels in dB, 20 Hz–20 kHz (log spaced).
        #[qinvokable]
        fn effect_spectrum(self: Pin<&mut Self>, id: i32, bands: i32) -> QString;
        /// Before and after spectra of an EQ: { "pre": [...], "post": [...] }.
        #[qinvokable]
        fn effect_spectra(self: Pin<&mut Self>, id: i32, bands: i32) -> QString;
        /// The EQ curve for these settings ({ key: value } JSON): `points` gains
        /// in dB, at frequencies spaced evenly on a log scale from 20 Hz to 20 kHz.
        #[qinvokable]
        fn eq_curve(self: &AppController, values: &QString, points: i32) -> QString;
        /// Like `eq_curve`, plus each band's own curve:
        /// { "total": [...], "bands": [[...] or null × 8] } (low cut, low shelf,
        /// bells 1–4, high shelf, high cut).
        #[qinvokable]
        fn eq_curves(self: &AppController, values: &QString, points: i32) -> QString;
        /// Turns an effect on or off (bypass).
        #[qinvokable]
        fn toggle_insert(self: Pin<&mut Self>, id: i32);
        #[qinvokable]
        fn move_insert(self: Pin<&mut Self>, id: i32, to: i32);
        #[qinvokable]
        fn set_insert_param(self: Pin<&mut Self>, id: i32, key: &QString, value: f64, commit: bool);
        /// ⇧-click: adds a region to the selection, or takes it out.
        #[qinvokable]
        fn toggle_region_selection(self: Pin<&mut Self>, id: i32);
        /// Marquee: selects regions over beats `from..to` on track rows `top..=bottom`.
        #[qinvokable]
        fn select_regions_in(self: Pin<&mut Self>, from: f64, to: f64, top: i32, bottom: i32, add: bool);
        #[qinvokable]
        fn select_all_regions(self: Pin<&mut Self>);
        #[qinvokable]
        fn copy_selected(self: Pin<&mut Self>);
        #[qinvokable]
        fn cut_selected(self: Pin<&mut Self>);
        /// Pastes at the playhead, onto the selected track.
        #[qinvokable]
        fn paste(self: Pin<&mut Self>);
        #[qinvokable]
        fn duplicate_selected(self: Pin<&mut Self>);
        /// Moves a region's left (`left` true) or right edge to `beats`.
        #[qinvokable]
        fn trim_region(self: Pin<&mut Self>, id: i32, left: bool, beats: f64);
        #[qinvokable]
        fn split_region(self: Pin<&mut Self>, id: i32, beats: f64);
        /// Splits the selected region, or every region under the playhead on the
        /// selected track, at the playhead (⌘T).
        #[qinvokable]
        fn split_at_playhead(self: Pin<&mut Self>);
        /// Fade lengths in beats; `commit` false while dragging.
        #[qinvokable]
        fn set_region_fades(self: Pin<&mut Self>, id: i32, fade_in: f64, fade_out: f64, commit: bool);
        #[qinvokable]
        fn move_track(self: Pin<&mut Self>, id: i32, index: i32);
        #[qinvokable]
        fn undo(self: Pin<&mut Self>);
        #[qinvokable]
        fn redo(self: Pin<&mut Self>);

        /// Saves to the current file. Returns false when the project has never
        /// been saved, so QML should ask for a location (Save As).
        #[qinvokable]
        fn save_project(self: Pin<&mut Self>) -> bool;
        #[qinvokable]
        fn save_project_as(self: Pin<&mut Self>, url: &QUrl) -> bool;
        #[qinvokable]
        fn open_project(self: Pin<&mut Self>, url: &QUrl);
        #[qinvokable]
        fn open_recent(self: Pin<&mut Self>, path: &QString);
        #[qinvokable]
        fn recover_autosave(self: Pin<&mut Self>);
        #[qinvokable]
        fn discard_autosave(self: Pin<&mut Self>);
        /// Writes a crash-recovery copy if there are unsaved changes.
        #[qinvokable]
        fn autosave(self: Pin<&mut Self>);
        /// Arms a track for recording (only one at a time), or disarms it.
        #[qinvokable]
        fn toggle_arm(self: Pin<&mut Self>, id: i32);
        /// Hear the live input through the armed track.
        #[qinvokable]
        fn toggle_monitor(self: Pin<&mut Self>, id: i32);
        /// Starts recording on the armed track (arming one if needed), or stops.
        #[qinvokable]
        fn record(self: Pin<&mut Self>);
        #[qinvokable]
        fn refresh_inputs(self: Pin<&mut Self>);
        /// Exports the mix (to the file `url`) or stems (into the folder `url`).
        #[qinvokable]
        fn export_audio(self: Pin<&mut Self>, url: &QUrl, stems: bool, cycle_only: bool, format: &QString, normalize: bool);
        #[qinvokable]
        fn cancel_export(self: Pin<&mut Self>);
        /// Renders a track to audio in the background and plays that instead,
        /// which saves processing power (its fader, pan and sends still work).
        #[qinvokable]
        fn freeze_track(self: Pin<&mut Self>, track: i32);
        #[qinvokable]
        fn unfreeze_track(self: Pin<&mut Self>, track: i32);
        /// Measures the mix and works out a master chain for `target` LUFS
        /// (in the background; the result arrives in `master_report`).
        #[qinvokable]
        fn analyse_master(self: Pin<&mut Self>, target: f64);
        /// Puts the suggested EQ, compressor and limiter on the master
        /// (replacing its effects), as one undo step.
        #[qinvokable]
        fn apply_master_plan(self: Pin<&mut Self>);
        #[qinvokable]
        fn cancel_master(self: Pin<&mut Self>);
        /// The user chose not to keep unsaved changes (closing or replacing the project).
        #[qinvokable]
        fn discard_changes(self: Pin<&mut Self>);
        #[qinvokable]
        fn toggle_play(self: Pin<&mut Self>);
        #[qinvokable]
        fn rewind(self: Pin<&mut Self>);
        #[qinvokable]
        fn seek_beats(self: Pin<&mut Self>, beats: f64);
        /// The position rounded to the grid (unchanged when snap is off).
        #[qinvokable]
        fn snap_beats(self: &Self, beats: f64) -> f64;

        #[qinvokable]
        fn new_project(self: Pin<&mut Self>, name: &QString, tempo: f64, num: i32, den: i32, key: &QString);
        #[qinvokable]
        fn close_project(self: Pin<&mut Self>);
        #[qinvokable]
        fn set_project_tempo(self: Pin<&mut Self>, bpm: f64);
        #[qinvokable]
        fn tap_tempo(self: Pin<&mut Self>);
        #[qinvokable]
        fn set_time_signature(self: Pin<&mut Self>, num: i32, den: i32);
        /// Adds a tempo change at a position (snapped), or changes the one there.
        #[qinvokable]
        fn add_tempo_change(self: Pin<&mut Self>, beat: f64, bpm: f64);
        /// Moves the tempo change at `from` to `to` (snapped) with a new tempo.
        #[qinvokable]
        fn move_tempo_change(self: Pin<&mut Self>, from: f64, to: f64, bpm: f64);
        #[qinvokable]
        fn remove_tempo_change(self: Pin<&mut Self>, beat: f64);
        /// Adds a time-signature change at the start of a bar (bar 1: the opening one).
        #[qinvokable]
        fn add_signature_change(self: Pin<&mut Self>, bar: i32, num: i32, den: i32);
        #[qinvokable]
        fn remove_signature_change(self: Pin<&mut Self>, bar: i32);
        /// Scans this computer for plugins again (in the background).
        #[qinvokable]
        fn rescan_plugins(self: Pin<&mut Self>);
        /// Checks on a scan and keeps plugin windows responsive (call while either is going).
        #[qinvokable]
        fn poll_plugins(self: Pin<&mut Self>);
        /// Adds installed plugin `index` to a chain and opens it; returns the insert id.
        #[qinvokable]
        fn add_plugin_effect(self: Pin<&mut Self>, chain_type: &QString, chain_id: i32, index: i32) -> i32;
        /// Adds a track playing installed plugin `index` and opens it; returns the track id.
        #[qinvokable]
        fn add_plugin_track(self: Pin<&mut Self>, index: i32) -> i32;
        /// Opens a plugin's own window (by its insert or instrument id).
        #[qinvokable]
        fn open_plugin(self: Pin<&mut Self>, id: i32);
        /// Whether the stem model is downloaded (else the first split downloads it).
        #[qinvokable]
        fn stem_model_ready(self: &AppController) -> bool;
        /// Splits an audio region into vocals, drums, bass and other on new tracks.
        #[qinvokable]
        fn split_stems(self: Pin<&mut Self>, region: i32);
        #[qinvokable]
        fn cancel_stems(self: Pin<&mut Self>);
        #[qinvokable]
        fn set_project_key(self: Pin<&mut Self>, key: &QString);
        #[qinvokable]
        fn toggle_metronome(self: Pin<&mut Self>);
        #[qinvokable]
        fn toggle_count_in(self: Pin<&mut Self>);
        #[qinvokable]
        fn toggle_cycle(self: Pin<&mut Self>);
        #[qinvokable]
        fn set_cycle_range(self: Pin<&mut Self>, start: f64, end: f64);
        /// Cycles exactly the selected regions (or the selected track's), ⌘U.
        #[qinvokable]
        fn cycle_selection(self: Pin<&mut Self>);
        #[qinvokable]
        fn toggle_snap(self: Pin<&mut Self>);
        #[qinvokable]
        fn set_snap_grid(self: Pin<&mut Self>, value: &QString);
        #[qinvokable]
        fn set_volume_level(self: Pin<&mut Self>, volume: f64);
        #[qinvokable]
        fn clear_status(self: Pin<&mut Self>);
        #[qinvokable]
        fn clear_clip(self: Pin<&mut Self>);
    }
}

use std::path::PathBuf;
use std::pin::Pin;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};
use isai_settings::{Animations, Detail, Hardware, Profile, Settings, Style, Theme, View};

use crate::audio::Audio;
use crate::format::format_time;
use crate::pool::AudioPool;
use crate::session::Session;
use crate::tap::TapTempo;

/// Late audio callbacks during playback before we suggest a lighter profile.
/// One at startup is normal and shouldn't nag the user.
const CRACKLE_THRESHOLD: u64 = 3;
/// How long to keep ticking after a test tone so its level shows on the meters.
const TONE_TICK: Duration = Duration::from_millis(1200);

#[derive(Default)]
pub(crate) struct State {
    pub settings: Settings,
    /// Installed and loaded third-party plugins.
    pub plugins: crate::plugin_ops::Plugins,
    /// A region being split into stems.
    pub stems: Option<crate::stems_ops::StemJob>,
    /// The open song's tempo map, kept for turning playhead samples into beats.
    pub tempo_map: isai_engine::TempoMap,
    hardware: Hardware,
    bench: Option<mpsc::Receiver<u32>>,
    pub audio: Audio,
    /// The open project, if any.
    pub session: Option<Session>,
    pub pool: AudioPool,
    /// Files being decoded for import, each becoming a new track when ready.
    /// With the track and beat to place each on (`None`: a new track at the playhead).
    pub pending_imports: Vec<(PathBuf, Option<(u64, f64)>)>,
    /// A browser preview waiting for its file to load (path, in time with the song).
    pub preview_pending: Option<(PathBuf, bool)>,
    meter: (f32, f32),
    /// Smoothed per-track levels, and a scratch list for reading them.
    track_meter: Vec<(f32, f32)>,
    track_peaks: Vec<(f32, f32)>,
    meter_at: Option<Instant>,
    tone_until: Option<Instant>,
    overruns_seen: u64,
    crackles: u64,
    crackle_dismissed: bool,
    pub tap: TapTempo,
    pub input: Option<isai_engine::Input>,
    pub recorder: Option<crate::recorder::Recorder>,
    /// The track the current take lands on.
    pub take_track: Option<u64>,
    /// Notes being recorded onto an instrument track.
    pub midi_take: Option<crate::record_ops::MidiTake>,
    pub input_meter: (f32, f32),
    pub export: Option<crate::export_ops::ExportJob>,
    pub freeze: Option<crate::export_ops::FreezeJob>,
    pub master_job: Option<crate::export_ops::MasterJob>,
    /// The assistant's last suggestion, ready to apply.
    pub master_plan: Option<crate::mastering::Plan>,
    /// Selected region ids, in the order they were selected.
    pub selection: Vec<u64>,
    pub clipboard: Option<isai_project::Clipboard>,
    /// Where live notes go (the running engine), shared with MIDI input threads.
    pub live_slot: crate::midi_input::LiveSlot,
    pub midi: Option<crate::midi_input::MidiInputs>,
    /// The track live notes were last routed to.
    pub live_track: Option<Option<usize>>,
    /// The tuner is open, so the input stays open too.
    pub tuner: bool,
    /// Scratch space for the tuner's samples.
    pub tuner_samples: Vec<f32>,
}

#[derive(Default)]
pub struct AppControllerRust {
    needs_setup: bool,
    setup_notice: QString,
    checking: bool,
    cpu_name: QString,
    cores: i32,
    ram_gb: f64,
    voices: i32,
    recommended: QString,

    profile: QString,
    view: QString,
    theme: QString,
    ui_style: QString,
    accent: QString,
    animations: QString,
    ui_fps: i32,
    meter_hz: i32,
    waveform_detail: QString,
    effects: bool,
    eco: bool,
    buffer_size: i32,
    device_id: QString,
    suggest_lighter: bool,
    settings_path: QString,
    device_names: QStringList,
    device_ids: QStringList,

    audio_ok: bool,
    audio_error: QString,
    device_name: QString,
    sample_rate: i32,
    actual_buffer: i32,
    cpu_load: f64,
    crackling: bool,

    loading: bool,
    position: f64,
    position_text: QString,
    playing: bool,
    tracks_json: QString,
    buses_json: QString,
    master_inserts_json: QString,
    effect_kinds_json: QString,
    instrument_kinds_json: QString,
    midi_inputs: QString,
    track_count: i32,
    selected_track: i32,
    selected_region: i32,
    selected_regions: QString,
    selection_count: i32,
    can_paste: bool,
    dirty: bool,
    project_path: QString,
    undo_label: QString,
    redo_label: QString,
    history_json: QString,
    markers_json: QString,
    tempo_json: QString,
    plugins_json: QString,
    plugin_scanning: bool,
    plugin_windows: i32,
    stem_region: i32,
    stem_progress: f64,
    stem_stage: QString,
    tuner_open: bool,
    previewing: bool,
    recent_paths: QStringList,
    recent_names: QStringList,
    recovery_name: QString,
    recovery_age: QString,
    exporting: bool,
    export_progress: f64,
    freezing_track: i32,
    mastering: bool,
    master_report: QString,
    freeze_progress: f64,
    export_folder: QString,
    recording: bool,
    input_ok: bool,
    input_name: QString,
    input_error: QString,
    input_level_l: f64,
    input_level_r: f64,
    input_device_names: QStringList,
    input_device_ids: QStringList,
    input_device_id: QString,
    input_stereo: bool,
    input_channel: i32,
    latency_offset_ms: i32,
    project_open: bool,
    project_name: QString,
    tempo: f64,
    time_sig_num: i32,
    time_sig_den: i32,
    key_name: QString,
    position_beats: f64,
    bar_text: QString,
    length_beats: f64,
    metronome: bool,
    count_in: bool,
    counting_in: bool,
    cycle: bool,
    cycle_start: f64,
    cycle_end: f64,
    snap: bool,
    snap_value: QString,
    click_volume: i32,
    count_in_bars: i32,
    volume: f64,
    meter_l: f64,
    meter_r: f64,
    track_meters: QString,
    master_clip: bool,
    status: QString,
    status_ok: bool,
    busy: bool,
    dev_screenshot: QString,
    dev_scene: QString,

    pub(crate) state: State,
}

pub(crate) fn qs(s: &str) -> QString {
    QString::from(s)
}

pub(crate) fn qlist(items: impl IntoIterator<Item = String>) -> QStringList {
    let mut list = QStringList::default();
    for item in items {
        list.append(QString::from(item.as_str()));
    }
    list
}

/// `[[l,r],…]` on the meter scale, rounded so idle meters don't keep changing.
fn track_meters_json(levels: &[(f32, f32)]) -> String {
    let mut out = String::from("[");
    for (i, (l, r)) in levels.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!("[{:.3},{:.3}]", meter_scale(*l), meter_scale(*r)));
    }
    out.push(']');
    out
}

/// Maps a peak level to 0..1 on a -60..0 dB scale, which matches how loud it sounds.
pub(crate) fn meter_scale(level: f32) -> f64 {
    let db = 20.0 * level.max(1e-6).log10();
    f64::from(((db + 60.0) / 60.0).clamp(0.0, 1.0))
}

impl qobject::AppController {
    pub(crate) fn st(self: Pin<&mut Self>) -> &mut State {
        &mut self.rust_mut().get_mut().state
    }

    // ---------------------------------------------------------------- startup

    pub fn start(mut self: Pin<&mut Self>, software_rendering: bool) {
        self.as_mut().set_volume(1.0);
        self.as_mut().set_freezing_track(-1);
        self.as_mut().set_stem_region(-1);
        self.as_mut().set_stem_progress(-1.0);
        let env = |key| std::env::var(key).unwrap_or_default();
        self.as_mut().set_dev_screenshot(qs(&env("ISAI_SCREENSHOT")));
        self.as_mut().set_dev_scene(qs(&env("ISAI_SCENE")));
        self.as_mut().set_effect_kinds_json(qs(&crate::session::effect_kinds_json()));
        self.as_mut().set_instrument_kinds_json(qs(&crate::session::instrument_kinds_json()));
        // The installed plugins: from the last scan, or scanned now in the background.
        self.as_mut().st().plugins.start();
        self.as_mut().publish_plugins();
        self.as_mut().refresh_midi();
        let mut hardware = Hardware::detect();
        if software_rendering {
            hardware.gpu = Some("software renderer".into());
        }
        self.as_mut().set_cpu_name(qs(&hardware.cpu_name));
        self.as_mut().set_cores(hardware.cores as i32);
        self.as_mut().set_ram_gb(f64::from(hardware.ram_gb));
        self.as_mut().st().hardware = hardware;
        if let Some(path) = isai_settings::path() {
            self.as_mut().set_settings_path(qs(&path.display().to_string()));
        }
        self.as_mut().refresh_devices();

        match isai_settings::load() {
            // Setup was started but not finished: carry on from the saved choices.
            Ok(Some(settings)) if !settings.setup_done => {
                self.as_mut().st().settings = settings;
                self.begin_setup(None);
            }
            Ok(Some(settings)) => {
                self.as_mut().st().settings = settings;
                self.as_mut().publish_settings();
                self.as_mut().publish_recent();
                self.as_mut().ensure_engine();
                self.as_mut().check_recovery();
                // `isai song.wav` starts a project with that file; `isai song.isai` opens it.
                if let Some(arg) = std::env::args_os().nth(1) {
                    let path = PathBuf::from(arg);
                    if path.extension().is_some_and(|e| e == isai_project::EXTENSION) {
                        self.as_mut().open_path(&path);
                    } else {
                        self.as_mut().new_project(&qs("Untitled"), 120.0, 4, 4, &qs("C major"));
                        self.as_mut().import_path(path);
                    }
                }
            }
            Ok(None) => self.begin_setup(None),
            Err(err) => self.begin_setup(Some(format!(
                "Your settings couldn't be read, so let's set things up again. ({err:#})"
            ))),
        }
    }

    fn begin_setup(mut self: Pin<&mut Self>, notice: Option<String>) {
        self.as_mut().set_setup_notice(qs(notice.as_deref().unwrap_or("")));
        self.as_mut().set_needs_setup(true);
        self.as_mut().set_checking(true);
        self.as_mut().publish_settings();

        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(isai_engine::bench::voice_capacity(Duration::from_secs(1)));
        });
        self.as_mut().st().bench = Some(rx);
        self.update_busy();
    }

    pub fn prepare_audio(self: Pin<&mut Self>) {
        self.ensure_engine();
    }

    pub fn save_setup_progress(mut self: Pin<&mut Self>) {
        let settings = &mut self.as_mut().st().settings;
        settings.setup_done = false;
        settings.sync_profile();
        self.save();
    }

    pub fn finish_setup(mut self: Pin<&mut Self>) {
        let settings = &mut self.as_mut().st().settings;
        settings.setup_done = true;
        settings.sync_profile();
        self.as_mut().set_needs_setup(false);
        self.as_mut().save();
        self.as_mut().publish_settings();
        self.as_mut().publish_recent();
        self.ensure_engine();
    }

    pub fn rerun_setup(self: Pin<&mut Self>) {
        self.begin_setup(None);
    }

    // --------------------------------------------------------------- settings

    pub fn set_text(mut self: Pin<&mut Self>, key: &QString, value: &QString) {
        let value = value.to_string();
        let s = &mut self.as_mut().st().settings;
        match key.to_string().as_str() {
            "profile" => {
                if let Some(p) = Profile::from_key(&value) {
                    s.apply_profile(p);
                }
            }
            "view" => s.view = View::from_key(&value).unwrap_or(s.view),
            "theme" => s.theme = Theme::from_key(&value).unwrap_or(s.theme),
            "uiStyle" => s.style = Style::from_key(&value).unwrap_or(s.style),
            "accent" => s.accent = value,
            "animations" => {
                s.visual.animations = Animations::from_key(&value).unwrap_or(s.visual.animations);
            }
            "waveformDetail" => {
                s.visual.waveform_detail = Detail::from_key(&value).unwrap_or(s.visual.waveform_detail);
            }
            "deviceId" => s.audio.device_id = (!value.is_empty()).then_some(value),
            "inputDevice" => s.recording.input_device = (!value.is_empty()).then_some(value),
            other => eprintln!("unknown text setting {other}"),
        }
        self.settings_changed();
    }

    pub fn set_int(mut self: Pin<&mut Self>, key: &QString, value: i32) {
        if key.to_string() == "latencyOffset" {
            self.as_mut().st().settings.recording.latency_offset_ms = value.clamp(-500, 500);
            self.settings_changed();
            return;
        }
        let value = value.max(0) as u32;
        let s = &mut self.as_mut().st().settings;
        match key.to_string().as_str() {
            "uiFps" => s.visual.ui_fps = value,
            "meterHz" => s.visual.meter_hz = value.max(1),
            "bufferSize" => s.audio.buffer_size = value.max(32),
            "clickVolume" => s.metronome.volume = value.min(150),
            "countInBars" => s.metronome.count_in_bars = value.clamp(1, 4),
            "inputChannel" => s.recording.channel = value.min(63) as u16,
            other => eprintln!("unknown number setting {other}"),
        }
        self.settings_changed();
    }

    pub fn set_bool(mut self: Pin<&mut Self>, key: &QString, value: bool) {
        let s = &mut self.as_mut().st().settings;
        match key.to_string().as_str() {
            "effects" => s.visual.effects = value,
            "suggestLighter" => s.helpers.suggest_lighter_profile = value,
            "inputStereo" => s.recording.stereo = value,
            "eco" => s.audio.eco = value,
            other => eprintln!("unknown on/off setting {other}"),
        }
        self.settings_changed();
    }

    fn settings_changed(mut self: Pin<&mut Self>) {
        self.as_mut().st().settings.sync_profile();
        // During setup nothing is saved until the user clicks Finish.
        if !*self.needs_setup() {
            self.as_mut().save();
        }
        self.as_mut().publish_settings();
        if !*self.needs_setup() || self.as_mut().st().audio.engine.is_some() {
            self.as_mut().ensure_engine();
        }
        // e.g. the click volume changed
        self.as_mut().push_project_to_engine();
        // e.g. Eco mode switched
        self.as_mut().push_arrangement();
        // e.g. a different input or channel: reopen it if it's in use.
        self.restart_input();
    }

    pub(crate) fn save(mut self: Pin<&mut Self>) {
        if let Err(err) = isai_settings::save(&self.as_mut().st().settings) {
            self.show_problem(&format!("Couldn't save settings: {err:#}"));
        }
    }

    fn publish_settings(mut self: Pin<&mut Self>) {
        let s = self.as_mut().st().settings.clone();
        self.as_mut().set_click_volume(s.metronome.volume as i32);
        self.as_mut().set_count_in_bars(s.metronome.count_in_bars as i32);
        self.as_mut().set_profile(qs(s.profile.key()));
        self.as_mut().set_view(qs(s.view.key()));
        self.as_mut().set_theme(qs(s.theme.key()));
        self.as_mut().set_ui_style(qs(s.style.key()));
        self.as_mut().set_accent(qs(&s.accent));
        self.as_mut().set_animations(qs(s.visual.animations.key()));
        self.as_mut().set_ui_fps(s.visual.ui_fps as i32);
        self.as_mut().set_meter_hz(s.visual.meter_hz as i32);
        self.as_mut().set_waveform_detail(qs(s.visual.waveform_detail.key()));
        self.as_mut().set_effects(s.visual.effects);
        self.as_mut().set_eco(s.audio.eco);
        self.as_mut().set_buffer_size(s.audio.buffer_size as i32);
        self.as_mut().set_device_id(qs(s.audio.device_id.as_deref().unwrap_or("")));
        self.as_mut().set_suggest_lighter(s.helpers.suggest_lighter_profile);
        self.as_mut().set_input_device_id(qs(s.recording.input_device.as_deref().unwrap_or("")));
        self.as_mut().set_input_stereo(s.recording.stereo);
        self.as_mut().set_input_channel(i32::from(s.recording.channel));
        self.as_mut().set_latency_offset_ms(s.recording.latency_offset_ms);
    }

    pub fn refresh_devices(mut self: Pin<&mut Self>) {
        self.as_mut().refresh_inputs();
        let devices = isai_engine::list_output_devices();
        self.as_mut().set_device_names(qlist(devices.iter().map(|d| d.name.clone())));
        self.as_mut().set_device_ids(qlist(devices.into_iter().map(|d| d.id)));
    }

    // ------------------------------------------------------------------ audio

    fn ensure_engine(mut self: Pin<&mut Self>) {
        let config = Audio::config_for(&self.as_mut().st().settings);
        if self.as_mut().st().audio.ensure(config) {
            self.as_mut().on_engine_started();
        }
        self.publish_audio();
    }

    pub fn retry_audio(mut self: Pin<&mut Self>) {
        self.as_mut().refresh_devices();
        if self.as_mut().st().audio.retry() {
            self.as_mut().on_engine_started();
        }
        self.publish_audio();
    }

    fn publish_audio(mut self: Pin<&mut Self>) {
        let (ok, error, info) = {
            let audio = &self.as_mut().st().audio;
            (
                audio.engine.is_some(),
                audio.error.clone().unwrap_or_default(),
                audio.engine.as_ref().map(|e| e.info().clone()),
            )
        };
        self.as_mut().set_audio_ok(ok);
        self.as_mut().set_audio_error(qs(&error));
        if let Some(info) = info {
            self.as_mut().set_device_name(qs(&info.device_name));
            self.as_mut().set_sample_rate(info.sample_rate as i32);
            self.as_mut().set_actual_buffer(info.buffer_size.map_or(0, |n| n as i32));
        }
    }

    /// A new engine starts empty, so hand it the current clip and transport state.
    fn on_engine_started(mut self: Pin<&mut Self>) {
        let volume = *self.volume() as f32;
        self.as_mut().push_project_to_engine();
        let st = self.as_mut().st();
        st.overruns_seen = 0;
        st.crackles = 0;
        let Some(engine) = &mut st.audio.engine else { return };
        engine.set_volume(volume);
        // Audio is decoded at the device's rate; a new rate means decoding again.
        let rate = engine.info().sample_rate;
        st.pool.set_rate(rate);
        self.as_mut().push_arrangement();
        // Any input queue belonged to the old engine.
        self.as_mut().restart_input();
        self.update_busy();
    }

    pub fn test_tone(mut self: Pin<&mut Self>) {
        let st = self.as_mut().st();
        if let Some(engine) = &mut st.audio.engine {
            engine.test_tone();
            st.tone_until = Some(Instant::now() + TONE_TICK);
        }
        self.update_busy();
    }

    pub fn switch_to_lightweight(mut self: Pin<&mut Self>) {
        self.as_mut().st().settings.apply_profile(Profile::Lightweight);
        self.as_mut().set_crackling(false);
        self.settings_changed();
    }

    pub fn dismiss_crackle(mut self: Pin<&mut Self>) {
        self.as_mut().st().crackle_dismissed = true;
        self.set_crackling(false);
    }

    // ------------------------------------------------------------------- tick

    pub fn tick(mut self: Pin<&mut Self>) {
        self.as_mut().poll_bench();
        self.as_mut().poll_audio();
        self.as_mut().poll_recording();
        self.as_mut().poll_export();
        self.as_mut().poll_freeze();
        self.as_mut().poll_master();
        self.as_mut().poll_stems();
        self.as_mut().poll_engine();
        self.as_mut().poll_preview();
        self.update_busy();
    }

    fn poll_bench(mut self: Pin<&mut Self>) {
        let st = self.as_mut().st();
        let Some(voices) = st.bench.as_ref().and_then(|rx| rx.try_recv().ok()) else { return };
        st.bench = None;
        st.hardware.voices = voices;
        // Only a recommendation: the user picks the profile themselves.
        let recommended = st.hardware.recommend();
        self.as_mut().set_voices(voices as i32);
        self.as_mut().set_recommended(qs(recommended.key()));
        self.set_checking(false);
    }

    fn poll_engine(mut self: Pin<&mut Self>) {
        let meter_hz = self.as_mut().st().settings.visual.meter_hz.max(1);
        let suggest = {
            let s = &self.as_mut().st().settings;
            s.helpers.suggest_lighter_profile && s.profile != Profile::Lightweight
        };
        let now = Instant::now();
        self.as_mut().route_live();
        let st = self.as_mut().st();
        let Some(engine) = &mut st.audio.engine else { return };
        engine.collect_garbage();

        let playing = engine.is_playing();
        let position = engine.position();
        let load = f64::from(engine.load());

        let due = st
            .meter_at
            .is_none_or(|t| now.duration_since(t).as_secs_f64() >= 1.0 / f64::from(meter_hz));
        let levels = if due {
            let elapsed = st.meter_at.map_or(1.0, |t| now.duration_since(t).as_secs_f32());
            let (l, r) = engine.take_peaks();
            // Fall back smoothly instead of dropping straight to zero.
            let decay = (-elapsed / 0.25).exp();
            st.meter = (l.max(st.meter.0 * decay), r.max(st.meter.1 * decay));
            st.meter_at = Some(now);
            // Tracks, then buses.
            let tracks = st.session.as_ref().map_or(0, |s| s.project.tracks.len() + s.project.buses.len());
            engine.take_track_peaks(tracks, &mut st.track_peaks);
            st.track_meter.resize(tracks, (0.0, 0.0));
            for (m, p) in st.track_meter.iter_mut().zip(&st.track_peaks) {
                *m = (p.0.max(m.0 * decay), p.1.max(m.1 * decay));
            }
            Some((st.meter, l.max(r) > 1.0))
        } else {
            None
        };

        let overruns = engine.overruns();
        if playing {
            st.crackles += overruns.saturating_sub(st.overruns_seen);
        }
        st.overruns_seen = overruns;
        let crackling = suggest && !st.crackle_dismissed && st.crackles >= CRACKLE_THRESHOLD;
        let counting_in = engine.counting_in();
        let rate = f64::from(engine.info().sample_rate);
        // Where the playhead is, in beats and bars, and the tempo and signature there.
        let here = st.session.as_ref().map(|_| {
            let secs = position as f64 / rate;
            let beats = st.tempo_map.secs_to_beats(secs);
            (secs, beats, st.tempo_map.bar_beat(beats).to_string(), st.tempo_map.has_changes().then(|| st.tempo_map.meter_at(beats)))
        });

        self.as_mut().set_playing(playing);
        self.as_mut().set_cpu_load(load);
        if counting_in != *self.counting_in() {
            self.as_mut().set_counting_in(counting_in);
        }
        if let Some((secs, beats, bar, local)) = here {
            self.as_mut().set_position(secs);
            self.as_mut().set_position_text(qs(&format_time(secs)));
            self.as_mut().set_position_beats(beats);
            self.as_mut().set_bar_text(qs(&bar));
            if let Some(m) = local {
                self.as_mut().show_meter(m);
            }
        }
        if let Some(((l, r), clipped)) = levels {
            self.as_mut().set_meter_l(meter_scale(l));
            self.as_mut().set_meter_r(meter_scale(r));
            if clipped && !*self.master_clip() {
                self.as_mut().set_master_clip(true);
            }
            let json = track_meters_json(&self.as_mut().st().track_meter);
            if json != self.track_meters().to_string() {
                self.as_mut().set_track_meters(qs(&json));
            }
        }
        if crackling != *self.crackling() {
            self.set_crackling(crackling);
        }
    }

    pub(crate) fn update_busy(mut self: Pin<&mut Self>) {
        let st = self.as_mut().st();
        let tone = st.tone_until.is_some_and(|t| Instant::now() < t);
        let metering = st.meter.0.max(st.meter.1) > 0.001;
        let busy = st.bench.is_some()
            || st.recorder.is_some()
            || st.midi_take.is_some()
            || st.export.is_some()
            || st.freeze.is_some()
            || st.master_job.is_some()
            || st.stems.is_some()
            || st.input.is_some()
            || st.tuner
            || st.pool.is_loading()
            || !st.pending_imports.is_empty()
            || st.preview_pending.is_some()
            || st.audio.engine.as_ref().is_some_and(|e| e.previewing())
            || tone
            || metering
            || st.audio.engine.as_ref().is_some_and(|e| e.is_playing());
        let busy = busy || *self.playing();
        if busy != *self.busy() {
            self.set_busy(busy);
        }
    }
}
