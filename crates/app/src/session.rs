//! The open project with everything around it: where it is saved, its undo
//! history, and whether it has unsaved changes.
//!
//! Plain Rust, so the editing rules are tested without Qt or an audio device.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use isai_engine::effects::PlayInsert;
use isai_engine::instruments::{PlayInstrument, PlayNote};
use isai_engine::{Arrangement, AutoTarget, PlayBus, PlayLane, PlayRegion, PlayTrack};
use isai_project::{EffectKind, History, Insert, InstrumentKind, Project, ProjectPaths, Track, db_to_gain};
use serde_json::json;

use crate::pool::{AudioPool, AudioState};

/// Fade applied to every region edge so cuts never click.
const DECLICK_SECS: f64 = 0.003;

/// Lowers an instrument's settings to its lightest quality, for Eco mode.
fn eco_params(kind: InstrumentKind, params: &mut [f32; isai_project::MAX_INST_PARAMS]) {
    let set = |params: &mut [f32; isai_project::MAX_INST_PARAMS], key: &str, f: &dyn Fn(f32) -> f32| {
        if let Some(i) = kind.params().iter().position(|d| d.key == key) {
            params[i] = f(params[i]);
        }
    };
    match kind {
        InstrumentKind::Piano => set(params, "quality", &|_| 1.0),
        // Unison is a choice index: 1 = two voices.
        InstrumentKind::Synth => set(params, "unison", &|v| v.min(1.0)),
        _ => {}
    }
}

/// Where two regions on a track overlap, the earlier one fades out while the
/// later one fades in across the overlap, like an automatic crossfade.
fn crossfade_overlaps(regions: &mut [PlayRegion]) {
    regions.sort_by_key(|r| r.start);
    for i in 1..regions.len() {
        let (before, after) = regions.split_at_mut(i);
        let (prev, next) = (&mut before[i - 1], &mut after[0]);
        let prev_end = prev.start + prev.len;
        if prev_end > next.start {
            let overlap = (prev_end - next.start).min(next.len).min(prev.len);
            prev.fade_out = prev.fade_out.max(overlap);
            next.fade_in = next.fade_in.max(overlap);
        }
    }
}

pub struct Session {
    pub project: Project,
    /// `None` until the project is saved for the first time.
    pub paths: Option<ProjectPaths>,
    pub history: History,
    /// Changes since the last save.
    pub dirty: bool,
}

/// A chain of effects as JSON: [{ id, kind, label, enabled, values: { key: value } }].
fn inserts_json(chain: &[Insert]) -> serde_json::Value {
    chain
        .iter()
        .map(|i| {
            let label = i.label();
            let values: serde_json::Map<_, _> = i
                .effect_kind()
                .map(|k| k.params().iter().zip(i.values()).map(|(d, v)| (d.key.to_owned(), json!(v))).collect())
                .unwrap_or_default();
            // Plugins: their name and maker; their controls are in their own window.
            let plugin = i.plugin.as_ref().map(|p| json!({ "name": p.name, "vendor": p.vendor }));
            json!({ "id": i.id, "kind": i.kind, "label": label, "enabled": i.enabled, "values": values, "plugin": plugin })
        })
        .collect()
}

/// Every built-in instrument with its parameters, like `effect_kinds_json`.
pub fn instrument_kinds_json() -> String {
    let kinds: Vec<_> = InstrumentKind::ALL
        .iter()
        .map(|k| json!({ "key": k.key(), "label": k.label(), "params": params_json(k.params()) }))
        .collect();
    serde_json::Value::Array(kinds).to_string()
}

fn params_json(params: &[isai_engine::effects::ParamDef]) -> Vec<serde_json::Value> {
    params
        .iter()
        .map(|d| {
            json!({
                "key": d.key, "label": d.label, "min": d.min, "max": d.max,
                "default": d.default, "unit": d.unit, "log": d.log, "choices": d.choices,
            })
        })
        .collect()
}

/// Every built-in effect with its parameters, for the effect editor.
pub fn effect_kinds_json() -> String {
    let kinds: Vec<_> = EffectKind::ALL
        .iter()
        .map(|k| json!({ "key": k.key(), "label": k.label(), "params": params_json(k.params()) }))
        .collect();
    serde_json::Value::Array(kinds).to_string()
}

impl Session {
    pub fn new(project: Project, paths: Option<ProjectPaths>) -> Self {
        Self { project, paths, history: History::default(), dirty: false }
    }

    /// Applies an edit as one undoable step. `f` returns whether it changed
    /// anything; edits that change nothing leave no undo step.
    pub fn edit(&mut self, label: &str, f: impl FnOnce(&mut Project) -> bool) -> bool {
        let mut next = self.project.clone();
        if !f(&mut next) || next == self.project {
            return false;
        }
        self.history.record(label, &self.project, Instant::now());
        self.project = next;
        self.dirty = true;
        true
    }

    pub fn undo(&mut self) -> Option<String> {
        let label = self.history.undo(&mut self.project)?;
        self.dirty = true;
        Some(label)
    }

    pub fn redo(&mut self) -> Option<String> {
        let label = self.history.redo(&mut self.project)?;
        self.dirty = true;
        Some(label)
    }

    /// The real location of a region's audio file.
    pub fn resolve(&self, file: &str) -> PathBuf {
        isai_project::resolve(self.paths.as_ref(), file)
    }

    /// Stretches that following regions need at the current tempo: (file, ratio).
    pub fn stretches(&self) -> Vec<(PathBuf, f64)> {
        self.project
            .tracks
            .iter()
            .flat_map(|t| &t.regions)
            .filter(|r| r.midi.is_none() && r.tempo.is_some())
            .map(|r| (self.resolve(&r.file), self.project.stretch_ratio(r)))
            .filter(|(_, ratio)| (ratio - 1.0).abs() > 1e-4)
            .collect()
    }

    pub fn audio_paths(&self) -> Vec<PathBuf> {
        self.project.audio_files().iter().map(|f| self.resolve(f)).collect()
    }

    /// Saves to `paths` (the current location, or a new one for Save As).
    pub fn save_to(&mut self, paths: ProjectPaths) -> anyhow::Result<()> {
        let saved = isai_project::save(&self.project, &paths, self.paths.as_ref())?;
        self.project = saved;
        self.paths = Some(paths);
        self.dirty = false;
        Ok(())
    }

    /// What the engine should play. Regions whose audio isn't loaded yet are
    /// left out until it is. With `eco`, instruments play in their lightest
    /// quality (the piano's Light mode, at most two unison voices on the synth).
    pub fn arrangement(&self, pool: &AudioPool, eco: bool) -> Arrangement {
        let rate = pool.rate();
        // Positions go through the tempo map; an audio region's own length is
        // measured at the tempo where it starts.
        let map = self.project.tempo_map();
        let secs = |s: f64| (s * f64::from(rate)).round().max(0.0) as u64;
        // A few milliseconds of fade on every edge, so cutting into a waveform
        // never clicks.
        let declick = secs(DECLICK_SECS);
        let bus_index = |id: u64| self.project.buses.iter().position(|b| b.id == id);
        let inserts = |chain: &[Insert]| -> Vec<PlayInsert> {
            chain
                .iter()
                .filter_map(|i| Some(PlayInsert { plugin: None, id: i.id, kind: i.effect_kind()?, enabled: i.enabled, params: i.values() }))
                .collect()
        };
        let tracks = self
            .project
            .tracks
            .iter()
            .map(|track| {
                let mut regions: Vec<PlayRegion> = Vec::new();
                for r in track.regions.iter().filter(|r| r.midi.is_none()) {
                    let path = self.resolve(&r.file);
                    let Some(source) = pool.ready(&path) else { continue };
                    // Following the tempo: the stretched copy, once it's made.
                    let ratio = self.project.stretch_ratio(r);
                    let (audio, scale) = match pool.stretched(&path, ratio) {
                        Some(stretched) if (ratio - 1.0).abs() > 1e-4 => (stretched, ratio),
                        _ => (source, 1.0),
                    };
                    let full = secs(r.length * scale);
                    let gain = 10f32.powf(r.gain_db / 20.0);
                    // A looped region plays its audio again and again until its loop end.
                    let beats = self.project.region_beats(r);
                    let span = self.project.region_span(r);
                    let local = self.project.region_meter(r);
                    let mut pass = 0.0;
                    while pass < span - 1e-9 {
                        // Audio not following the tempo keeps its speed through tempo changes.
                        let len = if r.tempo.is_none() {
                            let from = r.start + pass;
                            secs(map.beats_to_secs(from + (span - pass).min(beats)) - map.beats_to_secs(from))
                        } else {
                            secs(local.beats_to_secs((span - pass).min(beats)) * scale)
                        }
                        .min(full);
                        if len == 0 {
                            break;
                        }
                        // A shorter last pass of a reversed region still starts from the end of the audio.
                        let offset = secs(r.offset * scale) + if r.reverse { full - len } else { 0 };
                        regions.push(PlayRegion {
                            audio: Arc::clone(audio),
                            start: map.beats_to_samples(r.start + pass, rate),
                            offset,
                            len,
                            fade_in: secs(r.fade_in * scale).max(declick).min(len / 2),
                            fade_out: secs(r.fade_out * scale).max(declick).min(len / 2),
                            gain,
                            reverse: r.reverse,
                        });
                        pass += beats.max(1e-6);
                    }
                }
                crossfade_overlaps(&mut regions);
                let automation = self.play_lanes(track, rate);
                let automated = |bus: usize| automation.iter().any(|l| l.target == AutoTarget::Send(bus));
                let mut sends: Vec<(usize, f32)> = track
                    .sends
                    .iter()
                    .filter_map(|s| Some((bus_index(s.bus)?, db_to_gain(s.level_db))))
                    .filter(|&(bus, g)| g > 0.0 || automated(bus))
                    .collect();
                // A send that only exists as automation still needs a path to its bus.
                for lane in &automation {
                    if let AutoTarget::Send(bus) = lane.target
                        && !sends.iter().any(|s| s.0 == bus)
                    {
                        sends.push((bus, 0.0));
                    }
                }
                let mut play = PlayTrack {
                    gain: track.gain() * self.project.group_gain(track),
                    pan: track.pan,
                    audible: self.project.audible(track),
                    sends,
                    inserts: inserts(&track.inserts),
                    automation,
                    instrument: track.instrument.as_ref().and_then(|i| {
                        let kind = i.instrument_kind()?;
                        let mut params = i.values();
                        if eco {
                            eco_params(kind, &mut params);
                        }
                        let mut play = PlayInstrument::new(i.id, kind, params);
                        // The sampler's sound (once loaded) and its slices in samples.
                        play.sample = i.sample.as_ref().and_then(|f| pool.ready(&self.resolve(f)).cloned());
                        play.slices = i.slices.iter().map(|&s| (s * f64::from(rate)).round() as u64).collect();
                        Some(play)
                    }),
                    notes: if track.instrument.is_some() { self.play_notes(track, rate) } else { Vec::new() },
                    regions,
                };
                // Frozen (and its audio loaded): just the rendered file, with no
                // instrument or effects running. Fader, pan and sends stay live.
                if let Some(audio) = track.frozen.as_ref().and_then(|f| pool.ready(&self.resolve(f))) {
                    let len = audio.frames() as u64;
                    play.regions = vec![PlayRegion {
                        audio: Arc::clone(audio),
                        start: 0,
                        offset: 0,
                        len,
                        fade_in: 0,
                        fade_out: declick.min(len / 2),
                        gain: 1.0,
                        reverse: false,
                    }];
                    play.instrument = None;
                    play.notes = Vec::new();
                    play.inserts = Vec::new();
                    play.automation.retain(|l| !matches!(l.target, AutoTarget::Param { .. }));
                }
                play
            })
            .collect();
        let buses = self
            .project
            .buses
            .iter()
            .map(|b| PlayBus { gain: db_to_gain(b.volume_db), pan: b.pan, audible: !b.mute, inserts: inserts(&b.inserts) })
            .collect();
        let tempo_changes = map.tempos().skip(1).map(|(beat, bpm)| (map.beats_to_samples(beat, rate), bpm as f32)).collect();
        Arrangement { tracks, buses, master: inserts(&self.project.master_inserts), bpm: self.project.tempo as f32, tempo_changes }
    }

    /// Every note an instrument track plays, in samples, sorted by start.
    /// Notes outside a region's visible part are left out; a note running
    /// past the region's end is cut there.
    fn play_notes(&self, track: &Track, rate: u32) -> Vec<PlayNote> {
        let map = self.project.tempo_map();
        let meter = &map;
        let mut notes: Vec<PlayNote> = track
            .regions
            .iter()
            .filter_map(|r| Some((r, r.midi.as_ref()?)))
            .flat_map(|(r, m)| {
                // Each pass of a looped region plays the notes again.
                let span = self.project.region_span(r);
                let passes = (span / m.length.max(1e-6) - 1e-9).ceil().max(1.0) as usize;
                (0..passes).flat_map(move |k| {
                    let pass = r.start + k as f64 * m.length;
                    let pass_end = (pass + m.length).min(r.start + span);
                    m.notes.iter().filter(|n| n.start >= m.offset && n.start < m.offset + m.length).filter_map(move |n| {
                        let start = pass + (n.start - m.offset);
                        if start >= pass_end {
                            return None;
                        }
                        let end = (start + n.length).min(pass_end);
                        Some(PlayNote {
                            start: meter.beats_to_samples(start, rate),
                            end: meter.beats_to_samples(end, rate),
                            key: n.key,
                            velocity: n.velocity,
                        })
                    })
                })
            })
            .collect();
        notes.sort_by_key(|n| (n.start, n.key));
        notes
    }

    /// A track's automation lanes for the engine, with points in samples.
    fn play_lanes(&self, track: &Track, rate: u32) -> Vec<PlayLane> {
        let meter = self.project.tempo_map();
        track
            .automation
            .iter()
            .filter(|l| !l.points.is_empty())
            .filter_map(|lane| {
                let mut parts = lane.target.split(':');
                let target = match (parts.next()?, parts.next(), parts.next()) {
                    ("volume", None, _) => AutoTarget::Volume,
                    ("pan", None, _) => AutoTarget::Pan,
                    ("send", Some(bus), None) => {
                        let bus: u64 = bus.parse().ok()?;
                        AutoTarget::Send(self.project.buses.iter().position(|b| b.id == bus)?)
                    }
                    ("fx", Some(insert), Some(key)) => {
                        let insert: u64 = insert.parse().ok()?;
                        let kind = track.inserts.iter().find(|i| i.id == insert)?.effect_kind()?;
                        AutoTarget::Param { insert, index: kind.params().iter().position(|d| d.key == key)? }
                    }
                    _ => return None,
                };
                let points = lane.points.iter().map(|p| (meter.beats_to_samples(p.beat, rate), p.value)).collect();
                Some(PlayLane { target, points })
            })
            .collect()
    }

    /// What a track's automation can control, for the lane picker:
    /// [{ key, label, min, max, value, points: [[beat, value]…] }].
    fn auto_targets_json(&self, track: &Track) -> serde_json::Value {
        let mut keys: Vec<(String, String, &str)> = vec![("volume".into(), "Volume".into(), "dB"), ("pan".into(), "Pan".into(), "pan")];
        for b in &self.project.buses {
            keys.push((format!("send:{}", b.id), format!("Send to {}", b.name), "dB"));
        }
        for i in &track.inserts {
            if let Some(kind) = i.effect_kind() {
                for d in kind.params().iter().filter(|d| d.choices.is_empty()) {
                    keys.push((format!("fx:{}:{}", i.id, d.key), format!("{}: {}", kind.label(), d.label), d.unit));
                }
            }
        }
        keys.into_iter()
            .filter_map(|(key, label, unit)| {
                let (min, max, value) = self.project.target_range(track, &key)?;
                let points: Vec<_> = track
                    .automation
                    .iter()
                    .find(|l| l.target == key)
                    .map(|l| l.points.iter().map(|p| json!([p.beat, p.value])).collect())
                    .unwrap_or_default();
                Some(json!({ "key": key, "label": label, "unit": unit, "min": min, "max": max, "value": value, "points": points }))
            })
            .collect()
    }

    /// The master's effects as JSON.
    pub fn master_inserts_json(&self) -> String {
        inserts_json(&self.project.master_inserts).to_string()
    }

    /// Markers as JSON: [{ id, beat, name }], sorted by beat.
    pub fn markers_json(&self) -> String {
        let markers: Vec<_> = self.project.markers.iter().map(|m| json!({ "id": m.id, "beat": m.beat, "name": m.name })).collect();
        serde_json::Value::Array(markers).to_string()
    }

    /// Buses as JSON for the mixer.
    pub fn buses_json(&self) -> String {
        let buses: Vec<_> = self
            .project
            .buses
            .iter()
            .map(|b| {
                json!({
                    "id": b.id, "name": b.name, "color": b.color, "volume": b.volume_db,
                    "pan": b.pan, "mute": b.mute, "inserts": inserts_json(&b.inserts),
                })
            })
            .collect();
        serde_json::Value::Array(buses).to_string()
    }

    /// Tracks and regions as JSON for the QML arrangement view.
    pub fn tracks_json(&self, pool: &AudioPool) -> String {
        let tracks: Vec<_> = self
            .project
            .tracks
            .iter()
            .map(|t| {
                let regions: Vec<_> = t
                    .regions
                    .iter()
                    .map(|r| {
                        let path = self.resolve(&r.file);
                        let state = match pool.get(&path) {
                            _ if r.midi.is_some() => "ready",
                            Some(AudioState::Ready(_)) => "ready",
                            Some(AudioState::Failed(_)) | None => "missing",
                            Some(AudioState::Loading) => "loading",
                        };
                        let midi = r.midi.as_ref().map(|m| {
                            json!({
                                "length": m.length,
                                "offset": m.offset,
                                "notes": m.notes.iter().map(|n| json!([n.start, n.length, n.key, n.velocity])).collect::<Vec<_>>(),
                            })
                        });
                        json!({
                            "id": r.id,
                            "name": r.name,
                            "start": r.start,
                            "beats": self.project.region_beats(r),
                            // With its loop, if it loops.
                            "span": self.project.region_span(r),
                            "gain": r.gain_db,
                            "reverse": r.reverse,
                            "fadeIn": self.project.region_meter(r).secs_to_beats(r.fade_in),
                            "fadeOut": self.project.region_meter(r).secs_to_beats(r.fade_out),
                            "tempo": r.tempo,
                            "offset": r.offset,
                            "length": r.length,
                            "source": if r.midi.is_some() { String::new() } else { path.display().to_string() },
                            "state": state,
                            "midi": midi,
                        })
                    })
                    .collect();
                json!({
                    "id": t.id,
                    "name": t.name,
                    "frozen": t.frozen.is_some(),
                    "folder": t.folder,
                    "group": t.group,
                    "color": t.color,
                    "volume": t.volume_db,
                    "pan": t.pan,
                    "mute": t.mute,
                    "solo": t.solo,
                    "armed": t.armed,
                    "monitor": t.monitor,
                    "audible": self.project.audible(t),
                    "inserts": inserts_json(&t.inserts),
                    "instrument": t.instrument.as_ref().map(|i| {
                        let kind = i.instrument_kind();
                        let values: serde_json::Map<_, _> = kind
                            .map(|k| k.params().iter().zip(i.values()).map(|(d, v)| (d.key.to_owned(), json!(v))).collect())
                            .unwrap_or_default();
                        let sample = i.sample.as_ref().map(|f| {
                            let path = self.resolve(f);
                            let (state, secs) = match pool.get(&path) {
                                Some(AudioState::Ready(c)) => ("ready", c.duration_secs()),
                                Some(AudioState::Loading) => ("loading", 0.0),
                                _ => ("missing", 0.0),
                            };
                            let name = path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                            json!({ "source": path.display().to_string(), "name": name, "state": state, "seconds": secs })
                        });
                        json!({
                            "id": i.id, "kind": i.kind,
                            "label": i.plugin.as_ref().map_or_else(|| kind.map_or("Instrument", |k| k.label()).to_owned(), |p| p.name.clone()),
                            "values": values,
                            "sample": sample, "slices": i.slices,
                            "plugin": i.plugin.as_ref().map(|p| json!({ "name": p.name, "vendor": p.vendor })),
                        })
                    }),
                    "automationView": t.automation_view,
                    "autoTargets": self.auto_targets_json(t),
                    // Send level in dB per bus id.
                    "sends": t.sends.iter().map(|s| (s.bus.to_string(), json!(s.level_db))).collect::<serde_json::Map<_, _>>(),
                    "regions": regions,
                })
            })
            .collect();
        serde_json::Value::Array(tracks).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> Session {
        Session::new(Project::default(), None)
    }

    #[test]
    fn edits_mark_dirty_and_can_be_undone() {
        let mut s = session();
        assert!(s.edit("Add Track", |p| {
            p.add_track(None);
            true
        }));
        assert!(s.dirty);
        assert_eq!(s.project.tracks.len(), 1);
        assert_eq!(s.undo().as_deref(), Some("Add Track"));
        assert!(s.project.tracks.is_empty());
        assert_eq!(s.redo().as_deref(), Some("Add Track"));
        assert_eq!(s.project.tracks.len(), 1);
    }

    #[test]
    fn edits_that_change_nothing_leave_no_undo_step() {
        let mut s = session();
        assert!(!s.edit("Nothing", |p| p.remove_track(42)));
        assert!(!s.dirty);
        assert!(s.history.undo_label().is_none());
    }

    #[test]
    fn overlapping_regions_crossfade() {
        let audio = Arc::new(isai_engine::Clip { name: String::new(), sample_rate: 100, left: vec![1.0; 100], right: vec![1.0; 100] });
        let region = |start| PlayRegion { audio: audio.clone(), start, offset: 0, len: 100, fade_in: 1, fade_out: 1, gain: 1.0, reverse: false };
        let mut regions = vec![region(80), region(0)];
        crossfade_overlaps(&mut regions);
        assert_eq!(regions[0].start, 0);
        assert_eq!(regions[0].fade_out, 20);
        assert_eq!(regions[1].fade_in, 20);
    }

    #[test]
    fn loops_repeat_audio_and_notes() {
        let mut s = session();
        let mut synth = 0;
        s.edit("Setup", |p| {
            synth = p.add_instrument_track(InstrumentKind::Synth);
            let r = p.add_midi_region(synth, 0.0, 4.0).unwrap();
            p.add_note(r, 1.0, 1.0, 60, 100);
            p.set_region_loop(r, 10.0)
        });
        let track = s.project.track(synth).unwrap().clone();
        let starts: Vec<u64> = s.play_notes(&track, 100).iter().map(|n| n.start).collect();
        // 120 BPM at 100 samples a second: a beat is 50 samples.
        assert_eq!(starts, vec![50, 250, 450], "three passes, the last one cut short");
    }

    #[test]
    fn eco_mode_lightens_instruments() {
        let mut s = session();
        s.edit("Setup", |p| {
            let piano = p.add_instrument_track(InstrumentKind::Piano);
            let synth = p.add_instrument_track(InstrumentKind::Synth);
            p.set_instrument_param(synth, "unison", 6.0);
            p.set_instrument_param(piano, "quality", 0.0)
        });
        let pool = AudioPool::new(48_000);
        let value = |a: &Arrangement, track: usize, key: &str| {
            let i = a.tracks[track].instrument.as_ref().unwrap();
            i.params[i.kind.params().iter().position(|d| d.key == key).unwrap()]
        };
        let full = s.arrangement(&pool, false);
        assert_eq!((value(&full, 0, "quality"), value(&full, 1, "unison")), (0.0, 6.0));
        let eco = s.arrangement(&pool, true);
        assert_eq!((value(&eco, 0, "quality"), value(&eco, 1, "unison")), (1.0, 1.0));
    }

    #[test]
    fn regions_wait_for_their_audio() {
        let mut s = session();
        s.edit("Import", |p| {
            let t = p.add_track(None);
            p.add_region(t, "/nowhere/a.wav", "a", 0.0, 1.0);
            true
        });
        let pool = AudioPool::new(48_000);
        let arrangement = s.arrangement(&pool, false);
        assert_eq!(arrangement.tracks.len(), 1);
        assert!(arrangement.tracks[0].regions.is_empty(), "not loaded yet");

        let json: serde_json::Value = serde_json::from_str(&s.tracks_json(&pool)).unwrap();
        assert_eq!(json[0]["name"], "Audio 1");
        assert_eq!(json[0]["regions"][0]["state"], "missing");
        assert_eq!(json[0]["regions"][0]["beats"], 2.0);
    }
}
