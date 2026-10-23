//! Standard MIDI files (`.mid`): reading notes out of one, and writing a MIDI
//! region's notes into one. Only what a DAW needs is read: note on/off,
//! running status, tempo and time signature, track names. Everything else is
//! skipped. Files with SMPTE timing are refused.

use std::collections::HashMap;

use anyhow::{Context as _, bail};

use crate::Note;

/// Ticks per beat written by [`write`]. Divides evenly into 1/2, 1/4, 1/8 and
/// 1/16 beats and into triplets' 1/3-beat grid to within a tick.
pub const TICKS_PER_BEAT: u16 = 480;

/// The shortest note read or written, in beats (a 64th).
const MIN_NOTE: f64 = 1.0 / 64.0;

/// One track of a file that has notes in it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Track {
    pub name: String,
    /// Starts are in beats from the start of the file.
    pub notes: Vec<Note>,
}

/// What a MIDI file holds, in the terms a project needs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Song {
    /// The name the file gives itself: its first track name.
    pub name: String,
    /// Tracks with notes, in file order.
    pub tracks: Vec<Track>,
    /// The first tempo in the file, if it has one.
    pub bpm: Option<f64>,
    /// The first time signature in the file, as (numerator, denominator).
    pub meter: Option<(u32, u32)>,
}

/// Reads a Standard MIDI File. Notes that are never switched off end where
/// their track ends.
pub fn read(bytes: &[u8]) -> anyhow::Result<Song> {
    let mut r = Reader { bytes, pos: 0 };
    if r.take(4)? != b"MThd" {
        bail!("this isn't a MIDI file");
    }
    let header_len = r.u32()? as usize;
    if header_len < 6 {
        bail!("the MIDI file header is damaged");
    }
    let _format = r.u16()?;
    let tracks = r.u16()?;
    let division = r.u16()?;
    r.take(header_len - 6)?;
    if division & 0x8000 != 0 {
        bail!("MIDI files with SMPTE timing aren't supported");
    }
    if division == 0 {
        bail!("the MIDI file has no timing");
    }
    let ppq = f64::from(division);

    let mut song = Song::default();
    let mut seen = 0;
    while seen < tracks && !r.done() {
        let id = r.take(4)?;
        let len = r.u32()? as usize;
        let body = r.take(len)?;
        if id != b"MTrk" {
            continue; // Unknown chunks are skipped, as the format allows.
        }
        seen += 1;
        let track = read_track(body, ppq, &mut song)?;
        if song.name.is_empty() {
            song.name = track.name.clone();
        }
        if !track.notes.is_empty() {
            song.tracks.push(track);
        }
    }
    Ok(song)
}

/// Reads one `MTrk` chunk's events.
fn read_track(body: &[u8], ppq: f64, song: &mut Song) -> anyhow::Result<Track> {
    let mut r = Reader {
        bytes: body,
        pos: 0,
    };
    let mut track = Track::default();
    let mut tick: u64 = 0;
    // The last channel status byte, for running status.
    let mut status: u8 = 0;
    // Notes switched on and not yet off, by (channel, key): (start tick, velocity).
    let mut open: HashMap<(u8, u8), Vec<(u64, u8)>> = HashMap::new();
    let mut spans: Vec<(u64, u64, u8, u8)> = Vec::new();

    while !r.done() {
        tick += u64::from(r.vlq()?);
        let first = r.u8()?;
        if first == 0xFF {
            let kind = r.u8()?;
            let len = r.vlq()? as usize;
            let data = r.take(len)?;
            status = 0;
            match kind {
                0x03 if track.name.is_empty() => {
                    track.name = String::from_utf8_lossy(data).into_owned()
                }
                0x51 if data.len() == 3 && song.bpm.is_none() => {
                    let micros = u32::from_be_bytes([0, data[0], data[1], data[2]]);
                    if micros > 0 {
                        song.bpm = Some(60_000_000.0 / f64::from(micros));
                    }
                }
                0x58 if data.len() >= 2 && song.meter.is_none() => {
                    song.meter = Some((u32::from(data[0]), 1u32 << data[1].min(7)));
                }
                0x2F => break,
                _ => {}
            }
            continue;
        }
        if first == 0xF0 || first == 0xF7 {
            let len = r.vlq()? as usize;
            r.take(len)?;
            status = 0;
            continue;
        }
        // A channel message. A data byte here means the status is repeated.
        let data1 = if first & 0x80 != 0 {
            if first >= 0xF0 {
                // Other system messages have no channel; skip their data.
                status = 0;
                continue;
            }
            status = first;
            r.u8()?
        } else {
            if status == 0 {
                bail!("the MIDI file uses running status before any status byte");
            }
            first
        };
        let kind = status & 0xF0;
        let channel = status & 0x0F;
        let data2 = if kind == 0xC0 || kind == 0xD0 {
            0
        } else {
            r.u8()?
        };
        if data1 > 127 || data2 > 127 {
            bail!("the MIDI file has a damaged note event");
        }
        match kind {
            0x90 if data2 > 0 => open
                .entry((channel, data1))
                .or_default()
                .push((tick, data2)),
            0x80 | 0x90 => {
                let stack = open.entry((channel, data1)).or_default();
                if !stack.is_empty() {
                    let (start, velocity) = stack.remove(0);
                    spans.push((start, tick, data1, velocity));
                }
            }
            _ => {}
        }
    }

    // Notes still held at the end of the track end there.
    for ((_, key), stack) in open {
        for (start, velocity) in stack {
            spans.push((start, tick, key, velocity));
        }
    }
    spans.sort_by_key(|&(start, _, key, _)| (start, key));
    track.notes = spans
        .into_iter()
        .map(|(start, end, key, velocity)| Note {
            start: start as f64 / ppq,
            length: (end.saturating_sub(start) as f64 / ppq).max(MIN_NOTE),
            key: key.min(127),
            velocity: velocity.clamp(1, 127),
        })
        .collect();
    Ok(track)
}

/// Writes notes as a Standard MIDI File (format 1): a first track with the
/// name, tempo and time signature, then the notes on channel 1.
pub fn write(notes: &[Note], name: &str, bpm: f64, meter: (u32, u32)) -> Vec<u8> {
    let ppq = f64::from(TICKS_PER_BEAT);
    let tick = |beats: f64| (beats * ppq).round().max(0.0) as u64;

    // Events sorted by time; at the same time, note-offs come before note-ons,
    // so a repeated note ends before it starts again.
    let mut events: Vec<(u64, u8, Vec<u8>)> = Vec::new();
    for note in notes {
        let on = tick(note.start);
        let off = tick(note.start + note.length).max(on + 1);
        let key = note.key.min(127);
        events.push((on, 1, vec![0x90, key, note.velocity.clamp(1, 127)]));
        events.push((off, 0, vec![0x80, key, 0]));
    }
    events.sort_by_key(|(t, order, _)| (*t, *order));
    let notes_track = track_chunk(events.into_iter().map(|(t, _, bytes)| (t, bytes)).collect());

    let micros = (60_000_000.0 / bpm.max(1.0)).round() as u32;
    let denominator_power = meter.1.max(1).trailing_zeros() as u8;
    let name = &name.as_bytes()[..name.len().min(255)];
    let mut name_event = vec![0xFF, 0x03, name.len() as u8];
    name_event.extend_from_slice(name);
    let meta_track = track_chunk(vec![
        (0, name_event),
        (
            0,
            vec![
                0xFF,
                0x51,
                0x03,
                (micros >> 16) as u8,
                (micros >> 8) as u8,
                micros as u8,
            ],
        ),
        (
            0,
            vec![
                0xFF,
                0x58,
                0x04,
                meter.0.clamp(1, 255) as u8,
                denominator_power,
                24,
                8,
            ],
        ),
    ]);

    let mut out = Vec::new();
    out.extend_from_slice(b"MThd");
    out.extend_from_slice(&6u32.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes()); // format 1: tracks play together
    out.extend_from_slice(&2u16.to_be_bytes());
    out.extend_from_slice(&TICKS_PER_BEAT.to_be_bytes());
    out.extend_from_slice(&meta_track);
    out.extend_from_slice(&notes_track);
    out
}

/// Encodes events (absolute tick, bytes) as an `MTrk` chunk, with the end-of-track event.
fn track_chunk(events: Vec<(u64, Vec<u8>)>) -> Vec<u8> {
    let mut data = Vec::new();
    let mut last = 0u64;
    for (time, bytes) in events {
        push_vlq(&mut data, (time - last) as u32);
        data.extend_from_slice(&bytes);
        last = time;
    }
    data.extend_from_slice(&[0x00, 0xFF, 0x2F, 0x00]);
    let mut chunk = b"MTrk".to_vec();
    chunk.extend_from_slice(&(data.len() as u32).to_be_bytes());
    chunk.extend_from_slice(&data);
    chunk
}

/// Appends a variable-length quantity (7 bits per byte, high bit = more follow).
fn push_vlq(out: &mut Vec<u8>, mut value: u32) {
    let mut bytes = [0u8; 4];
    let mut n = 0;
    loop {
        bytes[n] = (value & 0x7F) as u8;
        n += 1;
        value >>= 7;
        if value == 0 || n == 4 {
            break;
        }
    }
    for i in (0..n).rev() {
        out.push(bytes[i] | if i > 0 { 0x80 } else { 0 });
    }
}

/// Bounds-checked reads: a cut-short file is an error, never a panic.
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn done(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    fn take(&mut self, n: usize) -> anyhow::Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|&e| e <= self.bytes.len())
            .context("the MIDI file is cut short")?;
        let slice = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn u8(&mut self) -> anyhow::Result<u8> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> anyhow::Result<u16> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> anyhow::Result<u32> {
        let b = self.take(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// A variable-length quantity, at most four bytes.
    fn vlq(&mut self) -> anyhow::Result<u32> {
        let mut value = 0u32;
        for _ in 0..4 {
            let b = self.u8()?;
            value = (value << 7) | u32::from(b & 0x7F);
            if b & 0x80 == 0 {
                return Ok(value);
            }
        }
        bail!("the MIDI file has a damaged number")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(start: f64, length: f64, key: u8, velocity: u8) -> Note {
        Note {
            start,
            length,
            key,
            velocity,
        }
    }

    #[test]
    fn notes_survive_a_round_trip() {
        let notes = vec![
            note(0.0, 1.0, 60, 100),
            note(0.5, 0.25, 64, 90),
            note(1.0, 0.5, 60, 20),
            note(1.0, 2.0, 67, 127),
        ];
        let bytes = write(&notes, "Lead", 96.0, (3, 4));
        let song = read(&bytes).unwrap();
        assert_eq!(song.bpm.map(|b| (b * 100.0).round()), Some(9600.0));
        assert_eq!(song.meter, Some((3, 4)));
        assert_eq!(song.tracks.len(), 1);
        assert_eq!(song.name, "Lead");
        assert_eq!(song.tracks[0].name, "");
        assert_eq!(song.tracks[0].notes, notes);
    }

    #[test]
    fn running_status_is_understood() {
        // Two note-ons with the status byte written once, then the note-offs.
        let mut track = vec![
            0x00, 0x90, 60, 100, 0x00, 62, 100, 0x60, 0x80, 60, 0, 0x00, 62, 0,
        ];
        track.extend_from_slice(&[0x00, 0xFF, 0x2F, 0x00]);
        let mut file = b"MThd".to_vec();
        file.extend_from_slice(&6u32.to_be_bytes());
        file.extend_from_slice(&[0, 0, 0, 1, 0, 96]);
        file.extend_from_slice(b"MTrk");
        file.extend_from_slice(&(track.len() as u32).to_be_bytes());
        file.extend_from_slice(&track);
        let song = read(&file).unwrap();
        let keys: Vec<u8> = song.tracks[0].notes.iter().map(|n| n.key).collect();
        assert_eq!(keys, vec![60, 62]);
        assert!((song.tracks[0].notes[0].length - 1.0).abs() < 1e-9);
    }

    #[test]
    fn notes_left_on_end_where_the_track_ends() {
        // A note on at 0, then the track ends a beat later with the note still held.
        let mut track = vec![0x00, 0x90, 60, 80, 0x60];
        track.extend_from_slice(&[0xFF, 0x2F, 0x00]);
        let mut file = b"MThd".to_vec();
        file.extend_from_slice(&6u32.to_be_bytes());
        file.extend_from_slice(&[0, 0, 0, 1, 0, 96]);
        file.extend_from_slice(b"MTrk");
        file.extend_from_slice(&(track.len() as u32).to_be_bytes());
        file.extend_from_slice(&track);
        let song = read(&file).unwrap();
        assert_eq!(song.tracks[0].notes.len(), 1);
        assert!((song.tracks[0].notes[0].length - 1.0).abs() < 1e-9);
    }

    #[test]
    fn damaged_files_are_errors_not_panics() {
        assert!(read(b"not midi at all").is_err());
        let bytes = write(&[note(0.0, 1.0, 60, 100)], "x", 120.0, (4, 4));
        for cut in 0..bytes.len() {
            // Every truncation either parses or is refused; none may panic.
            let _ = read(&bytes[..cut]);
        }
    }

    #[test]
    fn variable_length_numbers_round_trip() {
        for value in [0u32, 1, 127, 128, 8191, 8192, 0x0FFF_FFFF] {
            let mut out = Vec::new();
            push_vlq(&mut out, value);
            let mut r = Reader {
                bytes: &out,
                pos: 0,
            };
            assert_eq!(r.vlq().unwrap(), value);
            assert!(r.done());
        }
    }
}
