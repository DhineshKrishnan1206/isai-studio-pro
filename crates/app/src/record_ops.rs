//! The controller's recording side: the live input, arming, and takes.
//!
//! The input only runs while a track is armed, so the microphone (and the
//! system's "microphone in use" indicator) is off the rest of the time.

use std::path::PathBuf;
use std::pin::Pin;
use std::time::{SystemTime, UNIX_EPOCH};

use cxx_qt_lib::QString;
use isai_engine::InputChannels;

use crate::controller::{qlist, qobject::AppController, qs};
use crate::recorder::Recorder;
use isai_engine::instruments::PlayedNote;

/// Notes being recorded from MIDI keyboards or the computer keyboard.
pub struct MidiTake {
    track: u64,
    /// Held keys: when (timeline sample) and how hard they were pressed.
    held: [Option<(u64, u8)>; 128],
    /// Finished notes: start, end (samples), key, velocity.
    notes: Vec<(u64, u64, u8, u8)>,
    scratch: Vec<PlayedNote>,
}

impl MidiTake {
    fn new(track: u64) -> Self {
        Self { track, held: [None; 128], notes: Vec::new(), scratch: Vec::with_capacity(256) }
    }

    fn absorb(&mut self, engine: &mut isai_engine::Engine) {
        engine.take_played_notes(&mut self.scratch);
        for p in self.scratch.drain(..) {
            // Notes only: the sustain pedal isn't recorded (yet).
            if p.note.key > 127 {
                continue;
            }
            let key = usize::from(p.note.key);
            // A second press of a held key ends the first note.
            if let Some((start, vel)) = self.held[key].take() {
                self.notes.push((start, p.pos.max(start), p.note.key, vel));
            }
            if p.note.on && p.note.velocity > 0 {
                self.held[key] = Some((p.pos, p.note.velocity));
            }
        }
    }

    /// Ends any held notes at `pos`.
    fn release_all(&mut self, pos: u64) {
        for (key, held) in self.held.iter_mut().enumerate() {
            if let Some((start, vel)) = held.take() {
                self.notes.push((start, pos.max(start), key as u8, vel));
            }
        }
    }
}

/// Maps a peak level to 0..1 on a -60..0 dB scale.
fn level(peak: f32) -> f64 {
    let db = 20.0 * peak.max(1e-6).log10();
    f64::from(((db + 60.0) / 60.0).clamp(0.0, 1.0))
}

impl AppController {
    pub fn refresh_inputs(mut self: Pin<&mut Self>) {
        let devices = isai_engine::list_input_devices();
        self.as_mut().set_input_device_names(qlist(devices.iter().map(|d| d.name.clone())));
        self.set_input_device_ids(qlist(devices.into_iter().map(|d| d.id)));
    }

    /// Opens or closes the input to match the project: on while an audio
    /// track is armed (instrument tracks record notes, not the input).
    pub(crate) fn sync_input(mut self: Pin<&mut Self>) {
        let st = self.as_mut().st();
        let armed = st.session.as_ref().and_then(|s| s.project.armed_track().filter(|t| t.instrument.is_none()).map(|t| t.monitor));
        // The tuner listens too, without monitoring.
        let armed = armed.or(st.tuner.then_some(false));
        match armed {
            Some(monitor) => {
                if st.input.is_none() {
                    self.as_mut().open_input();
                }
                if let Some(engine) = &mut self.as_mut().st().audio.engine {
                    engine.set_monitor(monitor);
                }
            }
            None => {
                if let Some(engine) = &mut st.audio.engine {
                    engine.set_monitor(false);
                    engine.set_input(None);
                }
                st.input = None;
                self.as_mut().set_input_ok(false);
                self.as_mut().set_input_level_l(0.0);
                self.as_mut().set_input_level_r(0.0);
            }
        }
        self.update_busy();
    }

    pub fn set_tuner(mut self: Pin<&mut Self>, open: bool) {
        self.as_mut().st().tuner = open;
        self.as_mut().set_tuner_open(open);
        self.sync_input();
    }

    pub fn tuner_reading(mut self: Pin<&mut Self>, a4: f64) -> QString {
        let st = self.as_mut().st();
        let Some(engine) = &st.audio.engine else { return QString::default() };
        let rate = engine.info().sample_rate as f32;
        engine.input_scope(&mut st.tuner_samples);
        let samples = &st.tuner_samples;
        let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        let level = crate::controller::meter_scale(peak);
        let json = match crate::tuner::detect(samples, rate) {
            Some(r) => {
                let (name, octave, cents, midi) = crate::tuner::note(r.hz, a4.clamp(400.0, 480.0) as f32);
                serde_json::json!({ "level": level, "hz": r.hz, "note": name, "octave": octave, "cents": cents, "midi": midi, "clarity": r.clarity })
            }
            None => serde_json::json!({ "level": level }),
        };
        qs(&json.to_string())
    }

    /// Closes and reopens the input, e.g. after the user picks another one.
    pub(crate) fn restart_input(mut self: Pin<&mut Self>) {
        let st = self.as_mut().st();
        if st.recorder.is_some() {
            return; // never mid-take
        }
        st.input = None;
        if let Some(engine) = &mut st.audio.engine {
            engine.set_input(None);
        }
        self.sync_input();
    }

    fn open_input(mut self: Pin<&mut Self>) {
        let st = self.as_mut().st();
        let rec = st.settings.recording.clone();
        let Some(engine) = &mut st.audio.engine else { return };
        let select = if rec.stereo { InputChannels::Stereo(rec.channel) } else { InputChannels::Mono(rec.channel) };
        match isai_engine::start_input(rec.input_device.as_deref(), select, engine.info().sample_rate) {
            Ok((input, queue)) => {
                engine.set_input(Some(queue));
                let name = input.device_name.clone();
                st.input = Some(input);
                self.as_mut().set_input_name(qs(&name));
                self.as_mut().set_input_error(QString::default());
                self.set_input_ok(true);
            }
            Err(err) => {
                self.as_mut().set_input_ok(false);
                self.set_input_error(qs(&format!(
                    "Couldn't open the input: {err:#}. On macOS, allow microphone access in System Settings → Privacy & Security."
                )));
            }
        }
    }

    pub fn toggle_arm(mut self: Pin<&mut Self>, track: i32) {
        let id = track.max(0) as u64;
        let armed = self.as_mut().st().session.as_ref().and_then(|s| s.project.track(id).map(|t| t.armed));
        let Some(armed) = armed else { return };
        if let Some(s) = self.as_mut().st().session.as_mut() {
            // Arming isn't an edit to the song, so it isn't an undo step.
            s.project.set_armed(id, !armed);
        }
        self.as_mut().publish_project();
        self.after_arming();
    }

    pub fn toggle_monitor(mut self: Pin<&mut Self>, track: i32) {
        let id = track.max(0) as u64;
        if let Some(s) = self.as_mut().st().session.as_mut() {
            if !s.project.track(id).is_some_and(|t| t.armed) {
                s.project.set_armed(id, true);
            }
            if let Some(t) = s.project.track_mut(id) {
                t.monitor = !t.monitor;
            }
        }
        self.as_mut().publish_project();
        self.after_arming();
    }

    pub fn record(mut self: Pin<&mut Self>) {
        if *self.recording() {
            self.stop_recording();
            return;
        }
        if self.as_mut().st().session.is_none() || self.as_mut().st().recorder.is_some() || self.as_mut().st().midi_take.is_some() {
            return;
        }
        // Record onto the selected instrument track (notes), else the armed
        // track; arm the selected (or a new) track if none.
        let selected = *self.selected_track();
        let target = {
            let s = self.as_mut().st().session.as_mut().expect("checked above");
            let instrument = (selected >= 0)
                .then(|| s.project.track(selected as u64))
                .flatten()
                .filter(|t| t.instrument.is_some())
                .map(|t| t.id);
            match instrument.or_else(|| s.project.armed_track().map(|t| t.id)) {
                Some(id) => {
                    s.project.set_armed(id, true);
                    id
                }
                None => {
                    let id = if selected >= 0 && s.project.track(selected as u64).is_some() {
                        selected as u64
                    } else {
                        let mut new_id = 0;
                        s.edit("New Track", |p| {
                            new_id = p.add_track(None);
                            true
                        });
                        new_id
                    };
                    s.project.set_armed(id, true);
                    id
                }
            }
        };
        self.as_mut().set_selected_track(target as i32);
        self.as_mut().publish_project();
        let notes = self.as_mut().st().session.as_ref().is_some_and(|s| s.project.track(target).is_some_and(|t| t.instrument.is_some()));
        if notes {
            self.start_midi_take(target);
            return;
        }
        self.as_mut().sync_input();
        if !*self.input_ok() {
            let why = self.input_error().to_string();
            self.show_problem(&why);
            return;
        }

        let path = self.as_mut().take_path(target);
        let st = self.as_mut().st();
        let count_in = st.session.as_ref().is_some_and(|s| s.project.count_in);
        let bars = st.settings.metronome.count_in_bars;
        let Some(engine) = &mut st.audio.engine else { return };
        let Some(queue) = engine.take_record_queue() else { return };
        let rate = engine.info().sample_rate;
        st.recorder = Some(Recorder::start(queue, &path, rate));
        st.take_track = Some(target);
        engine.set_recording(true);
        if !engine.is_playing() {
            engine.play(if count_in { bars } else { 0 });
        }
        self.as_mut().set_recording(true);
        self.as_mut().set_playing(true);
        self.as_mut().set_status(QString::default());
        self.update_busy();
    }

    fn start_midi_take(mut self: Pin<&mut Self>, track: u64) {
        self.as_mut().sync_input();
        self.as_mut().route_live();
        let st = self.as_mut().st();
        let count_in = st.session.as_ref().is_some_and(|s| s.project.count_in);
        let bars = st.settings.metronome.count_in_bars;
        let Some(engine) = &mut st.audio.engine else { return };
        let mut take = MidiTake::new(track);
        take.absorb(engine); // drop anything left from before
        take.notes.clear();
        take.held = [None; 128];
        engine.set_capture_notes(true);
        if !engine.is_playing() {
            engine.play(if count_in { bars } else { 0 });
        }
        st.midi_take = Some(take);
        self.as_mut().set_recording(true);
        self.as_mut().set_playing(true);
        self.as_mut().set_status(QString::default());
        self.update_busy();
    }

    /// Turns the notes of a finished MIDI take into a region.
    fn finish_midi_take(mut self: Pin<&mut Self>) {
        let st = self.as_mut().st();
        let Some(mut take) = st.midi_take.take() else { return };
        let Some(engine) = &mut st.audio.engine else { return };
        take.absorb(engine);
        engine.set_capture_notes(false);
        take.release_all(engine.position());
        let rate = f64::from(engine.info().sample_rate);
        // A note reaches us about one buffer after it was played along to.
        let latency = u64::from(engine.info().buffer_size.unwrap_or(512));
        let Some(s) = st.session.as_mut() else { return };
        let meter = s.project.tempo_map();
        let beats = |pos: u64| meter.secs_to_beats(pos.saturating_sub(latency) as f64 / rate);
        let notes: Vec<isai_project::Note> = take
            .notes
            .iter()
            .filter(|n| n.1 > n.0)
            .map(|&(start, end, key, velocity)| {
                let b = beats(start);
                isai_project::Note { start: b, length: beats(end) - b, key, velocity }
            })
            .collect();
        if notes.is_empty() {
            self.show_info("Nothing was played, so nothing was recorded");
            return;
        }
        let mut region = 0;
        s.edit("Record MIDI", |p| {
            region = p.add_recorded_notes(take.track, &notes).unwrap_or(0);
            region != 0
        });
        self.as_mut().push_arrangement();
        self.as_mut().publish_project();
        if region != 0 {
            self.as_mut().set_selection(vec![region]);
        }
    }

    /// Where the next take is written: the project's Audio folder once saved,
    /// otherwise the app's recordings folder (copied in when the project is saved).
    fn take_path(mut self: Pin<&mut Self>, track: u64) -> PathBuf {
        let st = self.as_mut().st();
        let session = st.session.as_ref().expect("recording needs a project");
        let name = session.project.track(track).map_or("Audio".into(), |t| t.name.clone());
        let safe: String = name.chars().map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' { c } else { '_' }).collect();
        let dir = match &session.paths {
            Some(p) => p.audio_dir(),
            None => isai_settings::path()
                .and_then(|p| p.parent().map(|d| d.join("recordings")))
                .unwrap_or_else(std::env::temp_dir),
        };
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
        for n in 1.. {
            let candidate = dir.join(format!("{safe} take {n}.wav"));
            if !candidate.exists() {
                return candidate;
            }
            if n > 999 {
                break;
            }
        }
        dir.join(format!("{safe} take {stamp}.wav"))
    }

    pub(crate) fn stop_recording(mut self: Pin<&mut Self>) {
        self.as_mut().finish_midi_take();
        let st = self.as_mut().st();
        if let Some(engine) = &mut st.audio.engine {
            engine.set_recording(false);
            engine.stop();
        }
        if let Some(rec) = &st.recorder {
            rec.stop();
        }
        self.as_mut().set_recording(false);
        self.as_mut().set_playing(false);
        self.update_busy();
    }

    /// Input meters, and turning a finished take into a region.
    pub(crate) fn poll_recording(mut self: Pin<&mut Self>) {
        let st = self.as_mut().st();
        if let (Some(take), Some(engine)) = (&mut st.midi_take, &mut st.audio.engine) {
            take.absorb(engine);
        }
        if let (Some(engine), Some(_)) = (&st.audio.engine, &st.input) {
            let (l, r) = engine.take_input_peaks();
            st.input_meter = (l.max(st.input_meter.0 * 0.85), r.max(st.input_meter.1 * 0.85));
            let (ml, mr) = st.input_meter;
            self.as_mut().set_input_level_l(level(ml));
            self.as_mut().set_input_level_r(level(mr));
        }

        let st = self.as_mut().st();
        let Some((queue, result)) = st.recorder.as_mut().and_then(|r| r.finished()) else { return };
        st.recorder = None;
        let track = st.take_track.take();
        let Some(engine) = &mut st.audio.engine else { return };
        engine.return_record_queue(queue);
        let rate = engine.info().sample_rate;
        let record_start = engine.record_start();
        // Recording latency: the audio reaches us a little after it was played
        // and sung, so the take is moved earlier by the buffer delays plus the
        // user's correction.
        let buffer = engine.info().buffer_size.unwrap_or(512) as i64;
        let offset_ms = i64::from(st.settings.recording.latency_offset_ms);
        let latency = 2 * buffer + offset_ms * i64::from(rate) / 1000;

        match (result, record_start, track) {
            (Ok(take), Some(start), Some(track)) if take.frames > 0 => {
                let first = start as i64 - latency;
                let secs = |samples: i64| samples as f64 / f64::from(rate);
                let length = secs(take.frames as i64);
                // A take that would start before bar 1 is trimmed instead.
                let (start_secs, offset_secs) = if first >= 0 { (secs(first), 0.0) } else { (0.0, secs(-first)) };
                let file = take.path.display().to_string();
                let name: String = take.path.file_stem().map_or_else(|| "Take".into(), |n| n.to_string_lossy().into_owned());
                st.pool.request(&take.path);
                let mut region_id = 0;
                if let Some(s) = st.session.as_mut() {
                    let beats = s.project.tempo_map().secs_to_beats(start_secs);
                    s.edit("Record", |p| {
                        let Some(id) = p.add_region(track, &file, &name, beats, length) else { return false };
                        region_id = id;
                        // The region keeps its exact position; snapping would move the take.
                        if let Some(r) = p.tracks.iter_mut().flat_map(|t| t.regions.iter_mut()).find(|r| r.id == id) {
                            r.start = beats;
                            r.offset = offset_secs.min(length);
                            r.length = (length - offset_secs).max(0.0);
                        }
                        true
                    });
                }
                self.as_mut().push_arrangement();
                self.as_mut().publish_project();
                self.as_mut().set_selection(vec![region_id]);
                let dropped = self.as_mut().st().audio.engine.as_ref().map_or(0, |e| e.record_dropped());
                if dropped > 0 {
                    self.show_problem("Recorded, but the disk couldn't keep up and some audio was lost");
                }
            }
            (Err(err), _, _) => self.show_problem(&format!("Couldn't save the recording: {err:#}")),
            _ => self.show_problem("Nothing was recorded"),
        }
    }
}
