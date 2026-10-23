//! MIDI keyboards: every connected MIDI input is opened, and its notes go
//! straight to the audio thread (to the selected instrument track).

use std::sync::{Arc, Mutex};

use isai_engine::instruments::LiveNote;

/// Where live notes go: the running engine's queue (replaced when the
/// engine restarts). Shared with the MIDI input threads.
pub type LiveSlot = Arc<Mutex<Option<Arc<Mutex<rtrb::Producer<LiveNote>>>>>>;

/// Sends a note to the engine, if one is running. Never blocks for long:
/// the locks are only held to push one item.
pub fn send(slot: &LiveSlot, note: LiveNote) {
    let target = slot.lock().ok().and_then(|s| s.clone());
    if let Some(queue) = target
        && let Ok(mut q) = queue.lock()
    {
        let _ = q.push(note);
    }
}

/// Note-on/off and the sustain pedal (controller 64) from a raw MIDI
/// message (any channel); `None` for anything else.
pub fn parse(msg: &[u8]) -> Option<LiveNote> {
    let (&status, key, velocity) = (msg.first()?, *msg.get(1)?, *msg.get(2)?);
    match status & 0xF0 {
        0x90 if velocity > 0 => Some(LiveNote {
            on: true,
            key: key & 0x7F,
            velocity,
        }),
        0x90 | 0x80 => Some(LiveNote {
            on: false,
            key: key & 0x7F,
            velocity: 0,
        }),
        0xB0 if key == 64 => Some(LiveNote {
            on: velocity >= 64,
            key: isai_engine::instruments::SUSTAIN_PEDAL,
            velocity: 0,
        }),
        _ => None,
    }
}

/// Open MIDI inputs. Dropping this closes them.
pub struct MidiInputs {
    _connections: Vec<midir::MidiInputConnection<()>>,
    pub names: Vec<String>,
}

impl MidiInputs {
    /// Opens every MIDI input currently connected.
    pub fn open_all(slot: &LiveSlot) -> Self {
        let mut connections = Vec::new();
        let mut names = Vec::new();
        let ports = midir::MidiInput::new("Isai")
            .map(|m| m.ports())
            .unwrap_or_default();
        for port in ports {
            let Ok(input) = midir::MidiInput::new("Isai") else {
                continue;
            };
            let name = input
                .port_name(&port)
                .unwrap_or_else(|_| "MIDI input".into());
            let slot = slot.clone();
            let connected = input.connect(
                &port,
                "isai-in",
                move |_, msg, _| {
                    if let Some(note) = parse(msg) {
                        send(&slot, note);
                    }
                },
                (),
            );
            if let Ok(c) = connected {
                connections.push(c);
                names.push(name);
            }
        }
        Self {
            _connections: connections,
            names,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_note_on_and_off_on_any_channel() {
        assert_eq!(
            parse(&[0x93, 60, 100]),
            Some(LiveNote {
                on: true,
                key: 60,
                velocity: 100
            })
        );
        assert_eq!(
            parse(&[0x90, 60, 0]),
            Some(LiveNote {
                on: false,
                key: 60,
                velocity: 0
            }),
            "velocity 0 is off"
        );
        assert_eq!(
            parse(&[0x81, 62, 40]),
            Some(LiveNote {
                on: false,
                key: 62,
                velocity: 0
            })
        );
        assert_eq!(
            parse(&[0xB0, 7, 100]),
            None,
            "other controllers are ignored for now"
        );
        assert_eq!(
            parse(&[0xB0, 64, 127]),
            Some(LiveNote {
                on: true,
                key: isai_engine::instruments::SUSTAIN_PEDAL,
                velocity: 0
            }),
            "sustain pedal"
        );
        assert_eq!(
            parse(&[0xB2, 64, 0]),
            Some(LiveNote {
                on: false,
                key: isai_engine::instruments::SUSTAIN_PEDAL,
                velocity: 0
            })
        );
        assert_eq!(parse(&[0x90]), None);
    }
}
