//! Third-party plugins (VST3, CLAP, Audio Units), as the engine sees them.
//!
//! The app loads and owns the plugin; the engine only calls
//! [`ExternalPlugin::process`] through a shared handle. That keeps every
//! plugin format (and its SDK) out of the engine crate.
//!
//! On the audio thread a plugin is never waited for: if the app is busy with
//! it (opening its window, saving its state), that block passes the sound
//! through unchanged, or is silent for an instrument, instead of stalling the
//! audio. Exports render offline and do wait.

use std::fmt;
use std::sync::Arc;

use crate::instruments::{LiveNote, PlayNote};
use crate::mixer::{BLOCK, SILENT};

/// A note starting (velocity 1..=127) or ending (velocity 0), `offset` frames into the block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoteEvent {
    pub offset: u32,
    pub key: u8,
    pub velocity: u8,
}

/// What a plugin needs to know about the song while it plays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transport {
    pub bpm: f64,
    pub playing: bool,
    pub sample_rate: f32,
}

/// A loaded plugin. Implemented by the app for each plugin format.
pub trait ExternalPlugin: Send + Sync {
    /// Runs one block (at most `BLOCK` frames): `l` and `r` hold the input and
    /// receive the output. With `wait` false (the audio thread), returns false
    /// without touching the audio if the plugin is busy elsewhere.
    fn process(
        &self,
        l: &mut [f32],
        r: &mut [f32],
        events: &[NoteEvent],
        transport: &Transport,
        wait: bool,
    ) -> bool;

    /// Stops every sounding note (the transport stopped or jumped).
    fn all_notes_off(&self) {}
}

/// A shared handle to a loaded plugin.
#[derive(Clone)]
pub struct PluginRef(pub Arc<dyn ExternalPlugin>);

impl fmt::Debug for PluginRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PluginRef")
    }
}

/// A plugin used as an insert effect.
pub struct PluginEffect {
    plugin: Option<PluginRef>,
    /// Wait for a busy plugin (offline export) instead of passing the sound through.
    wait: bool,
}

impl PluginEffect {
    pub fn new(plugin: Option<PluginRef>, wait: bool) -> Self {
        Self { plugin, wait }
    }

    pub fn process(&mut self, transport: &Transport, l: &mut [f32], r: &mut [f32]) {
        if let Some(p) = &self.plugin {
            p.0.process(l, r, &[], transport, self.wait);
        }
    }
}

/// Most note starts and ends sent to a plugin in one block.
const MAX_EVENTS: usize = 512;

/// A plugin used as an instrument: turns the song's notes and live notes
/// into timed note events for each block.
pub struct PluginInstrument {
    plugin: Option<PluginRef>,
    wait: bool,
    events: Vec<NoteEvent>,
    /// Notes sounding from the song: (key, end position).
    held: Vec<(u8, u64)>,
    /// Keys held from live playing.
    live_held: u128,
    /// Index of the next song note to start.
    next: usize,
    /// The previous frame's timeline position (SILENT: stopped).
    last_pos: u64,
    scratch_l: Vec<f32>,
    scratch_r: Vec<f32>,
    struck: u128,
    velocities: [u8; 128],
}

impl PluginInstrument {
    /// Allocates its buffers: call off the audio thread.
    pub fn new(plugin: Option<PluginRef>, wait: bool) -> Self {
        Self {
            plugin,
            wait,
            events: Vec::with_capacity(MAX_EVENTS),
            held: Vec::with_capacity(128),
            live_held: 0,
            next: 0,
            last_pos: SILENT,
            scratch_l: vec![0.0; BLOCK],
            scratch_r: vec![0.0; BLOCK],
            struck: 0,
            velocities: [0; 128],
        }
    }

    fn push(&mut self, offset: usize, key: u8, velocity: u8) {
        if self.events.len() < MAX_EVENTS {
            self.events.push(NoteEvent {
                offset: offset as u32,
                key: key.min(127),
                velocity,
            });
        }
        if velocity > 0 {
            self.struck |= 1 << key.min(127);
            self.velocities[usize::from(key.min(127))] = velocity;
        }
    }

    fn release_song_notes(&mut self, offset: usize) {
        while let Some((key, _)) = self.held.pop() {
            self.push(offset, key, 0);
        }
    }

    /// Plays `notes` (sorted by start) at this block's timeline `positions`,
    /// plus `live` notes at its start, adding the plugin's sound into `l` / `r`.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        notes: &[PlayNote],
        positions: &[u64],
        live: &[LiveNote],
        transport: &Transport,
        l: &mut [f32],
        r: &mut [f32],
    ) {
        let n = positions.len().min(BLOCK).min(l.len()).min(r.len());
        self.events.clear();
        for note in live {
            let bit = 1u128 << note.key.min(127);
            if note.on {
                self.live_held |= bit;
                self.push(0, note.key, note.velocity.max(1));
            } else {
                self.live_held &= !bit;
                self.push(0, note.key, 0);
            }
        }
        for (i, &pos) in positions[..n].iter().enumerate() {
            if pos == SILENT {
                if self.last_pos != SILENT {
                    self.release_song_notes(i);
                }
                self.last_pos = SILENT;
                continue;
            }
            // Started, looped or jumped: let go of what was playing and find our place.
            if self.last_pos == SILENT || pos != self.last_pos + 1 {
                self.release_song_notes(i);
                self.next = notes.partition_point(|x| x.start < pos);
            }
            let mut k = 0;
            while k < self.held.len() {
                if self.held[k].1 <= pos {
                    let (key, _) = self.held.swap_remove(k);
                    self.push(i, key, 0);
                } else {
                    k += 1;
                }
            }
            while self.next < notes.len() && notes[self.next].start <= pos {
                let note = notes[self.next];
                self.next += 1;
                if note.end > pos && self.held.len() < self.held.capacity() {
                    self.held.push((note.key, note.end));
                    self.push(i, note.key, note.velocity.max(1));
                }
            }
            self.last_pos = pos;
        }

        let Some(plugin) = &self.plugin else { return };
        let (sl, sr) = (&mut self.scratch_l[..n], &mut self.scratch_r[..n]);
        sl.fill(0.0);
        sr.fill(0.0);
        if plugin.0.process(sl, sr, &self.events, transport, self.wait) {
            for (o, s) in l[..n].iter_mut().zip(sl.iter()) {
                *o += *s;
            }
            for (o, s) in r[..n].iter_mut().zip(sr.iter()) {
                *o += *s;
            }
        }
    }

    pub fn keys(&mut self) -> (u128, u128) {
        let held = self
            .held
            .iter()
            .fold(self.live_held, |m, (k, _)| m | (1 << k));
        (held, std::mem::take(&mut self.struck))
    }

    pub fn velocities(&self) -> &[u8; 128] {
        &self.velocities
    }

    pub fn reset(&mut self) {
        self.held.clear();
        self.live_held = 0;
        self.last_pos = SILENT;
        if let Some(p) = &self.plugin {
            p.0.all_notes_off();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Records the events it receives and outputs a constant.
    #[derive(Default)]
    struct Probe {
        events: Mutex<Vec<NoteEvent>>,
    }

    impl ExternalPlugin for Probe {
        fn process(
            &self,
            l: &mut [f32],
            r: &mut [f32],
            events: &[NoteEvent],
            _: &Transport,
            _: bool,
        ) -> bool {
            self.events.lock().unwrap().extend_from_slice(events);
            l.fill(0.5);
            r.fill(0.25);
            true
        }
    }

    const T: Transport = Transport {
        bpm: 120.0,
        playing: true,
        sample_rate: 48_000.0,
    };

    #[test]
    fn notes_become_timed_events_and_end_when_playback_jumps() {
        let probe = Arc::new(Probe::default());
        let mut inst = PluginInstrument::new(Some(PluginRef(probe.clone())), false);
        let notes = [
            PlayNote {
                start: 10,
                end: 20,
                key: 60,
                velocity: 100,
            },
            PlayNote {
                start: 15,
                end: 100,
                key: 64,
                velocity: 90,
            },
        ];
        let (mut l, mut r) = (vec![0.0; 32], vec![0.0; 32]);
        let positions: Vec<u64> = (0..32).collect();
        inst.render(&notes, &positions, &[], &T, &mut l, &mut r);
        let ev = probe.events.lock().unwrap().clone();
        assert_eq!(
            ev,
            vec![
                NoteEvent {
                    offset: 10,
                    key: 60,
                    velocity: 100
                },
                NoteEvent {
                    offset: 15,
                    key: 64,
                    velocity: 90
                },
                NoteEvent {
                    offset: 20,
                    key: 60,
                    velocity: 0
                },
            ]
        );
        assert_eq!((l[0], r[0]), (0.5, 0.25), "the plugin's sound is added");
        // Jumping back (a cycle) releases the held note first.
        probe.events.lock().unwrap().clear();
        let back: Vec<u64> = (0..4).collect();
        inst.render(&notes, &back, &[], &T, &mut l[..4], &mut r[..4]);
        assert_eq!(
            probe.events.lock().unwrap()[0],
            NoteEvent {
                offset: 0,
                key: 64,
                velocity: 0
            }
        );
        // Live notes arrive at the block start.
        probe.events.lock().unwrap().clear();
        let live = [LiveNote {
            on: true,
            key: 72,
            velocity: 80,
        }];
        inst.render(&notes, &[SILENT; 4], &live, &T, &mut l[..4], &mut r[..4]);
        assert_eq!(
            probe.events.lock().unwrap()[0],
            NoteEvent {
                offset: 0,
                key: 72,
                velocity: 80
            }
        );
        assert_eq!(inst.keys().0, 1 << 72);
    }
}
