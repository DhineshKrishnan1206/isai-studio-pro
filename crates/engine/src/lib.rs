//! Real-time audio engine.
//!
//! The engine owns the audio device stream. The UI talks to it only through a
//! lock-free command queue, and reads its state back through atomics, so the
//! audio thread never waits on a lock and never allocates or frees memory.
//!
//! This crate must never depend on any UI code.

pub mod arrangement;
pub mod bench;
pub mod clip;
pub mod device;
pub mod effects;
mod engine;
pub mod export;
pub mod input;
pub mod instruments;
pub mod mixer;
mod processor;
pub mod stretch;
pub mod plugin;
pub mod time;

pub use arrangement::{Arrangement, AutoTarget, PlayBus, PlayLane, PlayRegion, PlayTrack};
pub use clip::{Clip, Peaks};
pub use device::{OutputDevice, list_output_devices};
pub use engine::{Engine, EngineConfig, StreamInfo};
pub use input::{Input, InputChannels, InputDevice, list_input_devices, start_input};
pub use time::{BarBeat, Meter, SigChange, TempoMap, TempoPoint};
