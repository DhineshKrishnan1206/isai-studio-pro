//! Live readings from a running effect, for its editor: input and output
//! levels, gain reduction, and the latest audio for a spectrum analyzer.
//!
//! The audio thread only stores into atomics, so it never waits; the UI
//! reads (and resets) them whenever it redraws.

use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering::Relaxed};

/// Samples kept for the analyzer (a power of two, for the FFT).
pub const SCOPE: usize = 2048;

pub struct Meters {
    /// Peak levels since the last read (f32 bits), left and right.
    input: [AtomicU32; 2],
    output: [AtomicU32; 2],
    /// Deepest gain reduction since the last read, in dB (f32 bits, ≤ 0).
    reduction: AtomicU32,
    /// The latest output (mono), as a ring of f32 bits, and the input that
    /// made it (same positions), for a before/after analyzer.
    scope: Vec<AtomicU32>,
    pre: Vec<AtomicU32>,
    scope_pos: AtomicUsize,
}

impl Default for Meters {
    fn default() -> Self {
        Self {
            input: [AtomicU32::new(0), AtomicU32::new(0)],
            output: [AtomicU32::new(0), AtomicU32::new(0)],
            reduction: AtomicU32::new(0f32.to_bits()),
            scope: (0..SCOPE).map(|_| AtomicU32::new(0)).collect(),
            pre: (0..SCOPE).map(|_| AtomicU32::new(0)).collect(),
            scope_pos: AtomicUsize::new(0),
        }
    }
}

fn raise(a: &AtomicU32, v: f32) {
    let _ = a.fetch_update(Relaxed, Relaxed, |old| (v > f32::from_bits(old)).then_some(v.to_bits()));
}

fn peak(s: &[f32]) -> f32 {
    s.iter().fold(0.0f32, |p, x| p.max(x.abs()))
}

impl Meters {
    /// Audio thread: before processing a block (`scope`: keep its audio too).
    pub fn record_input(&self, l: &[f32], r: &[f32], scope: bool) {
        raise(&self.input[0], peak(l));
        raise(&self.input[1], peak(r));
        if scope {
            // Same positions the output will take; `record_output` moves on.
            let mut pos = self.scope_pos.load(Relaxed);
            for (a, b) in l.iter().zip(r) {
                self.pre[pos].store(((a + b) * 0.5).to_bits(), Relaxed);
                pos = (pos + 1) % SCOPE;
            }
        }
    }

    /// Audio thread: after processing a block. `reduction` is the effect's
    /// gain reduction in dB (0 for effects that don't reduce).
    pub fn record_output(&self, l: &[f32], r: &[f32], reduction: f32, scope: bool) {
        raise(&self.output[0], peak(l));
        raise(&self.output[1], peak(r));
        let _ = self.reduction.fetch_update(Relaxed, Relaxed, |old| (reduction < f32::from_bits(old)).then_some(reduction.to_bits()));
        if scope {
            let mut pos = self.scope_pos.load(Relaxed);
            for (a, b) in l.iter().zip(r) {
                self.scope[pos].store(((a + b) * 0.5).to_bits(), Relaxed);
                pos = (pos + 1) % SCOPE;
            }
            self.scope_pos.store(pos, Relaxed);
        }
    }

    /// UI: (input L, input R, output L, output R, reduction dB) since the
    /// last call, then resets them.
    pub fn take(&self) -> ([f32; 4], f32) {
        let take = |a: &AtomicU32| f32::from_bits(a.swap(0, Relaxed));
        let levels = [take(&self.input[0]), take(&self.input[1]), take(&self.output[0]), take(&self.output[1])];
        let reduction = f32::from_bits(self.reduction.swap(0f32.to_bits(), Relaxed));
        (levels, reduction)
    }

    /// UI: the latest `SCOPE` output samples, oldest first.
    pub fn scope(&self) -> Vec<f32> {
        self.ring(&self.scope)
    }

    /// UI: the input matching `scope()`.
    pub fn pre_scope(&self) -> Vec<f32> {
        self.ring(&self.pre)
    }

    fn ring(&self, ring: &[AtomicU32]) -> Vec<f32> {
        let pos = self.scope_pos.load(Relaxed);
        (0..SCOPE).map(|i| f32::from_bits(ring[(pos + i) % SCOPE].load(Relaxed))).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_peaks_until_read_and_the_latest_audio() {
        let m = Meters::default();
        m.record_input(&[0.5, -0.8], &[0.1, 0.2], false);
        m.record_output(&[0.25; 4], &[0.25; 4], -3.0, true);
        m.record_output(&[0.1; 4], &[0.1; 4], -1.0, true);
        let (levels, gr) = m.take();
        assert_eq!(levels, [0.8, 0.2, 0.25, 0.25]);
        assert_eq!(gr, -3.0, "deepest reduction");
        assert_eq!(m.take().0, [0.0; 4], "reset after reading");
        let scope = m.scope();
        assert_eq!(scope.len(), SCOPE);
        assert_eq!(scope[SCOPE - 1], 0.1);
    }
}
