//! Splitting a song into vocals, drums, bass and other, with Deezer's
//! Spleeter 4-stem model (code and model MIT-licensed), run natively.
//!
//! The model is not shipped with Isai: the app downloads Deezer's release on
//! first use, checks it against Deezer's checksum and converts it
//! ([`model::convert_release`]).
//!
//! It follows Spleeter's own separation: a 4096-sample STFT (hop 1024) at
//! 44.1 kHz, magnitudes of the lowest 1024 bins in 512-frame segments through
//! one U-Net per stem, then each stem's share of the mix (its estimate
//! squared over the sum of all four) applied to the full STFT (bins above
//! 1024 get nothing), and the inverse STFT. Segments are processed one at a
//! time and their sound handed out as soon as it's final, so a long song never
//! sits in memory four times over.

pub mod model;
mod unet;

use std::sync::Arc;

use realfft::RealFftPlanner;
use realfft::num_complex::Complex32;

pub use model::Weights;
use unet::{Tensor, UNet};

pub const SAMPLE_RATE: u32 = 44_100;
pub const STEMS: [&str; 4] = ["Vocals", "Drums", "Bass", "Other"];

const FRAME: usize = 4096;
const HOP: usize = 1024;
const BINS: usize = FRAME / 2 + 1;
const T: usize = 512;
const F: usize = 1024;
const EPS: f32 = 1e-10;
/// Hann² windows overlapping by a quarter add up to 1.5.
const WINDOW_COMPENSATION: f32 = 2.0 / 3.0;

/// The four networks, ready to run.
pub struct Separator {
    nets: Vec<UNet>,
}

impl Separator {
    pub fn new(weights: &Weights) -> Result<Self, String> {
        Ok(Self {
            nets: (0..4)
                .map(|i| UNet::load(weights, i))
                .collect::<Result<_, _>>()?,
        })
    }

    /// Splits stereo audio at 44.1 kHz. `out(stem, left, right)` receives each
    /// stem's audio in order, a piece at a time. `progress` gets 0..1; when
    /// `cancel` returns true it stops and returns false.
    pub fn separate(
        &self,
        left: &[f32],
        right: &[f32],
        threads: usize,
        out: impl FnMut(usize, &[f32], &[f32]),
        progress: impl Fn(f32),
        cancel: impl Fn() -> bool,
    ) -> bool {
        separate_with(
            left,
            right,
            out,
            progress,
            cancel,
            &|mag: &Tensor, stem: usize| self.nets[stem].forward(mag, threads),
        )
    }
}

/// The separation, with each stem's spectrogram estimated by `estimate`.
fn separate_with(
    left: &[f32],
    right: &[f32],
    mut out: impl FnMut(usize, &[f32], &[f32]),
    progress: impl Fn(f32),
    cancel: impl Fn() -> bool,
    estimate: &dyn Fn(&Tensor, usize) -> Tensor,
) -> bool {
    {
        let n = left.len().min(right.len());
        let padded = n + FRAME; // a frame of silence in front, as Spleeter does
        let frames = padded.div_ceil(HOP);
        let segments = frames.div_ceil(T).max(1);
        let sample = |ch: usize, p: usize| -> f32 {
            if p < FRAME || p - FRAME >= n {
                0.0
            } else if ch == 0 {
                left[p - FRAME]
            } else {
                right[p - FRAME]
            }
        };

        let window: Vec<f32> = (0..FRAME)
            .map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / FRAME as f32).cos())
            .collect();
        let mut planner = RealFftPlanner::<f32>::new();
        let fwd: Arc<dyn realfft::RealToComplex<f32>> = planner.plan_fft_forward(FRAME);
        let inv: Arc<dyn realfft::ComplexToReal<f32>> = planner.plan_fft_inverse(FRAME);
        let mut time = fwd.make_input_vec();
        let mut freq = fwd.make_output_vec();

        // Overlap-add buffers per stem and channel, covering padded positions [base, base + len).
        let seg_len = T * HOP;
        let acc_len = seg_len + FRAME;
        let mut acc = vec![vec![vec![0.0f32; acc_len]; 2]; 4];
        let mut base = 0usize;
        let mut emitted = FRAME; // output starts after the leading silence

        for s in 0..segments {
            if cancel() {
                return false;
            }
            // STFT of this segment's frames, both channels: [ch][t][bin].
            let mut spec = vec![vec![Complex32::new(0.0, 0.0); T * BINS]; 2];
            let mut mag = Tensor::zeros(T, F, 2);
            for t in 0..T {
                let frame = s * T + t;
                if frame >= frames {
                    break;
                }
                for (ch, spec) in spec.iter_mut().enumerate() {
                    for (i, v) in time.iter_mut().enumerate() {
                        *v = sample(ch, frame * HOP + i) * window[i];
                    }
                    if fwd.process(&mut time, &mut freq).is_err() {
                        return false;
                    }
                    spec[t * BINS..(t + 1) * BINS].copy_from_slice(&freq);
                    for (f, bin) in freq.iter().take(F).enumerate() {
                        mag.d[(t * F + f) * 2 + ch] = bin.norm();
                    }
                }
            }

            // Each stem's estimate, then its share of the mix.
            let mut est: Vec<Tensor> = Vec::with_capacity(4);
            for i in 0..4 {
                if cancel() {
                    return false;
                }
                est.push(estimate(&mag, i));
                progress((s * 4 + i + 1) as f32 / (segments * 4) as f32);
            }
            let mut total = vec![EPS; T * F * 2];
            for e in &est {
                for (t, v) in total.iter_mut().zip(&e.d) {
                    *t += v * v;
                }
            }

            for (stem, e) in est.iter().enumerate() {
                for t in 0..T {
                    let frame = s * T + t;
                    if frame >= frames {
                        break;
                    }
                    for ch in 0..2 {
                        for f in 0..BINS {
                            let mask = if f < F {
                                let k = (t * F + f) * 2 + ch;
                                (e.d[k] * e.d[k] + EPS / 4.0) / total[k]
                            } else {
                                0.0
                            };
                            freq[f] = spec[ch][t * BINS + f] * mask;
                        }
                        // The inverse needs real DC and Nyquist bins.
                        freq[0].im = 0.0;
                        freq[BINS - 1].im = 0.0;
                        if inv.process(&mut freq, &mut time).is_err() {
                            return false;
                        }
                        let at = frame * HOP - base;
                        let buf = &mut acc[stem][ch];
                        for (i, v) in time.iter().enumerate() {
                            buf[at + i] += v * window[i] / FRAME as f32;
                        }
                    }
                }
            }

            // Everything before the next segment's first frame is final.
            let done_upto = ((s + 1) * T * HOP).min(padded);
            if done_upto > emitted {
                let (from, to) = (emitted - base, done_upto - base);
                for (stem, bufs) in acc.iter().enumerate() {
                    let l: Vec<f32> = bufs[0][from..to]
                        .iter()
                        .map(|v| v * WINDOW_COMPENSATION)
                        .collect();
                    let r: Vec<f32> = bufs[1][from..to]
                        .iter()
                        .map(|v| v * WINDOW_COMPENSATION)
                        .collect();
                    out(stem, &l, &r);
                }
                emitted = done_upto;
            }
            let shift = ((s + 1) * T * HOP).saturating_sub(base).min(acc_len);
            for bufs in &mut acc {
                for b in bufs.iter_mut() {
                    b.drain(..shift);
                    b.resize(acc_len, 0.0);
                }
            }
            base += shift;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// With every stem estimating the same (nothing), each gets a quarter of
    /// the mix, so a quarter comes back: the STFT round trip is exact.
    #[test]
    fn the_stft_round_trip_rebuilds_the_mix() {
        let n = 3 * T * HOP / 2 + 777;
        let left: Vec<f32> = (0..n).map(|i| (i as f32 * 0.013).sin() * 0.5).collect();
        let right: Vec<f32> = (0..n).map(|i| (i as f32 * 0.007).cos() * 0.3).collect();
        let (mut l0, mut r0) = (Vec::new(), Vec::new());
        let ok = separate_with(
            &left,
            &right,
            |stem, l, r| {
                if stem == 0 {
                    l0.extend_from_slice(l);
                    r0.extend_from_slice(r);
                }
            },
            |_| {},
            || false,
            &|mag: &Tensor, _| Tensor::zeros(mag.h, mag.w, mag.c),
        );
        assert!(ok);
        assert_eq!(l0.len(), n);
        // These tones are far below bin 1024 (~11 kHz), where the masks apply.
        // (Their abrupt start and end aren't: skip a frame at each edge.)
        for i in (FRAME..n - FRAME).step_by(997) {
            assert!(
                (l0[i] * 4.0 - left[i]).abs() < 1e-3,
                "left {i}: {} vs {}",
                l0[i] * 4.0,
                left[i]
            );
            assert!((r0[i] * 4.0 - right[i]).abs() < 1e-3, "right {i}");
        }
    }
}
