use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use isai_engine::instruments::LiveNote;

use crate::midi_input::{self, LiveSlot};

struct Keyboard {
    enabled: AtomicBool,
    octave: AtomicU8,
    held: Mutex<HashMap<u16, u8>>,
    octave_held: Mutex<HashSet<u16>>,
    slot: LiveSlot,
}

static KEYBOARD: OnceLock<Arc<Keyboard>> = OnceLock::new();

pub fn install(slot: LiveSlot) {
    KEYBOARD.get_or_init(|| {
        Arc::new(Keyboard {
            enabled: AtomicBool::new(false),
            octave: AtomicU8::new(4),
            held: Mutex::new(HashMap::new()),
            octave_held: Mutex::new(HashSet::new()),
            slot,
        })
    });
    #[cfg(target_os = "macos")]
    if let Some(keyboard) = KEYBOARD.get().cloned() {
        isai_plugins::set_plugin_key_handler(move |key, down| keyboard.event(key, down));
    }
}

pub fn set_state(enabled: bool, octave: i32) {
    let Some(keyboard) = KEYBOARD.get() else {
        return;
    };
    keyboard
        .octave
        .store(octave.clamp(0, 8) as u8, Ordering::Relaxed);
    let was_enabled = keyboard.enabled.swap(enabled, Ordering::Relaxed);
    if was_enabled && !enabled {
        keyboard.release_all();
    }
}

pub fn octave() -> i32 {
    KEYBOARD.get().map_or(4, |keyboard| {
        i32::from(keyboard.octave.load(Ordering::Relaxed))
    })
}

pub fn release_held() {
    if let Some(keyboard) = KEYBOARD.get() {
        keyboard.release_all();
    }
}

impl Keyboard {
    // Only the macOS plugin key handler calls this (the tests too).
    #[cfg(any(target_os = "macos", test))]
    fn event(&self, key: u16, down: bool) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        if matches!(key, 6 | 7) {
            let Ok(mut held) = self.octave_held.lock() else {
                return;
            };
            if down {
                if !held.insert(key) {
                    return;
                }
                let octave = self.octave.load(Ordering::Relaxed);
                let octave = if key == 6 {
                    octave.saturating_sub(1)
                } else {
                    octave.saturating_add(1).min(8)
                };
                self.octave.store(octave, Ordering::Relaxed);
            } else {
                held.remove(&key);
            }
            return;
        }
        let Some(offset) = note_offset(key) else {
            return;
        };
        if down {
            let Ok(mut held) = self.held.lock() else {
                return;
            };
            if held.contains_key(&key) {
                return;
            }
            let note = (self.octave.load(Ordering::Relaxed) + 1) * 12 + offset;
            held.insert(key, note);
            midi_input::send(
                &self.slot,
                LiveNote {
                    on: true,
                    key: note,
                    velocity: 100,
                },
            );
        } else {
            let Ok(mut held) = self.held.lock() else {
                return;
            };
            if let Some(note) = held.remove(&key) {
                midi_input::send(
                    &self.slot,
                    LiveNote {
                        on: false,
                        key: note,
                        velocity: 0,
                    },
                );
            }
        }
    }

    fn release_all(&self) {
        let Ok(mut held) = self.held.lock() else {
            return;
        };
        for (_, note) in held.drain() {
            midi_input::send(
                &self.slot,
                LiveNote {
                    on: false,
                    key: note,
                    velocity: 0,
                },
            );
        }
        if let Ok(mut octave_held) = self.octave_held.lock() {
            octave_held.clear();
        }
    }
}

#[cfg(any(target_os = "macos", test))]
fn note_offset(key: u16) -> Option<u8> {
    match key {
        0 => Some(0),   // A
        13 => Some(1),  // W
        1 => Some(2),   // S
        14 => Some(3),  // E
        2 => Some(4),   // D
        3 => Some(5),   // F
        17 => Some(6),  // T
        5 => Some(7),   // G
        16 => Some(8),  // Y
        4 => Some(9),   // H
        32 => Some(10), // U
        38 => Some(11), // J
        40 => Some(12), // K
        31 => Some(13), // O
        37 => Some(14), // L
        35 => Some(15), // P
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Keyboard, note_offset};
    use crate::midi_input::LiveSlot;
    use isai_engine::instruments::LiveNote;
    use std::collections::{HashMap, HashSet};
    use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
    use std::sync::{Arc, Mutex};

    #[test]
    fn mac_key_codes_match_the_studio_keyboard_layout() {
        let offsets = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        let key_codes = [0, 13, 1, 14, 2, 3, 17, 5, 16, 4, 32, 38, 40, 31, 37, 35];
        for (key, expected) in key_codes.into_iter().zip(offsets) {
            assert_eq!(note_offset(key), Some(expected));
        }
    }

    #[test]
    fn plugin_key_up_releases_the_note_at_its_original_octave() {
        let (producer, mut consumer) = rtrb::RingBuffer::<LiveNote>::new(8);
        let slot: LiveSlot = Arc::new(Mutex::new(Some(Arc::new(Mutex::new(producer)))));
        let keyboard = Keyboard {
            enabled: AtomicBool::new(true),
            octave: AtomicU8::new(4),
            held: Mutex::new(HashMap::new()),
            octave_held: Mutex::new(HashSet::new()),
            slot,
        };

        keyboard.event(0, true);
        keyboard.event(0, true);
        keyboard.event(7, true);
        keyboard.event(0, false);

        assert_eq!(
            consumer.pop().unwrap(),
            LiveNote {
                on: true,
                key: 60,
                velocity: 100
            }
        );
        assert_eq!(
            consumer.pop().unwrap(),
            LiveNote {
                on: false,
                key: 60,
                velocity: 0
            }
        );
        assert_eq!(keyboard.octave.load(Ordering::Relaxed), 5);
    }
}
