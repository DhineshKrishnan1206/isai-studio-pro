//! Tap tempo: tap along to the beat and the project follows.

use std::time::{Duration, Instant};

use isai_engine::Meter;

/// Turns taps on a key or button into a tempo.
#[derive(Default)]
pub struct TapTempo {
    taps: Vec<Instant>,
}

impl TapTempo {
    /// Taps further apart than this start a new measurement.
    const RESET_AFTER: Duration = Duration::from_secs(2);

    /// Records a tap. Returns a tempo once there are at least two taps.
    pub fn tap(&mut self, now: Instant) -> Option<f64> {
        if self.taps.last().is_some_and(|&t| now.duration_since(t) > Self::RESET_AFTER) {
            self.taps.clear();
        }
        self.taps.push(now);
        if self.taps.len() > 6 {
            self.taps.remove(0);
        }
        if self.taps.len() < 2 {
            return None;
        }
        let span = now.duration_since(self.taps[0]).as_secs_f64();
        let avg = span / (self.taps.len() - 1) as f64;
        let bpm = (60.0 / avg).clamp(Meter::MIN_BPM, Meter::MAX_BPM);
        Some((bpm * 10.0).round() / 10.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tap_tempo_averages_taps() {
        let mut tap = TapTempo::default();
        let t0 = Instant::now();
        assert_eq!(tap.tap(t0), None);
        assert_eq!(tap.tap(t0 + Duration::from_millis(500)), Some(120.0));
        assert_eq!(tap.tap(t0 + Duration::from_millis(1000)), Some(120.0));
        // A long pause starts over.
        assert_eq!(tap.tap(t0 + Duration::from_secs(5)), None);
    }
}
