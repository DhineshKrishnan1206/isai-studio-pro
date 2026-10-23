//! Time-stretching: making audio longer or shorter without changing its pitch,
//! so a loop recorded at one tempo plays in time at another.
//!
//! WSOLA: overlapping windowed grains are copied to the output at a steady
//! spacing, while the place each grain is read from moves at the stretched
//! speed, nudged a little each time to where it lines up best with the
//! previous one (so waveforms join without phasey seams). The search runs on
//! a downsampled copy, which keeps it quick enough for slow computers.

use crate::clip::Clip;

/// Makes `clip` `ratio` times as long (2.0 = twice as long, half the speed),
/// keeping its pitch.
pub fn stretch(clip: &Clip, ratio: f64) -> Clip {
    let ratio = ratio.clamp(0.25, 4.0);
    let len = clip.frames();
    if (ratio - 1.0).abs() < 1e-4 || len < 64 {
        return Clip {
            name: clip.name.clone(),
            sample_rate: clip.sample_rate,
            left: clip.left.clone(),
            right: clip.right.clone(),
        };
    }
    let rate = clip.sample_rate as f64;
    // Grains of about 40 ms, overlapping four times.
    let n = ((0.04 * rate) as usize / 4 * 4).max(64);
    let hs = n / 4;
    let ha = hs as f64 / ratio;
    let tol = n / 4;
    let window: Vec<f32> = (0..n)
        .map(|i| (0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / n as f64).cos()) as f32)
        .collect();

    // A quarter-rate mono copy to search on.
    const DOWN: usize = 4;
    let mono: Vec<f32> = (0..len / DOWN)
        .map(|i| {
            (0..DOWN)
                .map(|k| clip.left[i * DOWN + k] + clip.right[i * DOWN + k])
                .sum::<f32>()
        })
        .collect();
    let at = |i: isize| -> f32 {
        if i >= 0 && (i as usize) < mono.len() {
            mono[i as usize]
        } else {
            0.0
        }
    };

    let out_len = (len as f64 * ratio).round() as usize;
    let total = out_len + n;
    let (mut l, mut r, mut norm) = (
        vec![0.0f32; total],
        vec![0.0f32; total],
        vec![0.0f32; total],
    );
    let mut prev: isize = 0;
    let mut k = 0usize;
    while k * hs < out_len {
        let nominal = (k as f64 * ha).round() as isize;
        let pos = if k == 0 {
            0
        } else {
            // Where the last grain would naturally have continued.
            let natural = prev + hs as isize;
            let span = (n / 2 / DOWN) as isize;
            let mut best = (nominal, f32::MIN);
            let mut d = -(tol as isize);
            while d <= tol as isize {
                let cand = nominal + d;
                if cand >= 0 && (cand as usize) < len {
                    let (c, nat) = (cand / DOWN as isize, natural / DOWN as isize);
                    let mut score = 0.0f32;
                    for j in 0..span {
                        score += at(c + j) * at(nat + j);
                    }
                    if score > best.1 {
                        best = (cand, score);
                    }
                }
                d += DOWN as isize;
            }
            best.0
        };
        let start = k * hs;
        for (i, w) in window.iter().enumerate() {
            let src = pos as usize + i;
            if src >= len {
                break;
            }
            l[start + i] += clip.left[src] * w;
            r[start + i] += clip.right[src] * w;
            norm[start + i] += w;
        }
        prev = pos;
        k += 1;
    }
    for ((a, b), w) in l.iter_mut().zip(r.iter_mut()).zip(&norm) {
        if *w > 1e-3 {
            *a /= w;
            *b /= w;
        }
    }
    l.truncate(out_len);
    r.truncate(out_len);
    Clip {
        name: clip.name.clone(),
        sample_rate: clip.sample_rate,
        left: l,
        right: r,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(hz: f32, secs: f32) -> Clip {
        let s: Vec<f32> = (0..(secs * 48_000.0) as usize)
            .map(|i| 0.5 * (i as f32 * hz / 48_000.0 * std::f32::consts::TAU).sin())
            .collect();
        Clip {
            name: "t".into(),
            sample_rate: 48_000,
            left: s.clone(),
            right: s,
        }
    }

    fn hz(s: &[f32]) -> f32 {
        s.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count() as f32
            / (s.len() as f32 / 48_000.0)
    }

    #[test]
    fn stretching_changes_length_but_not_pitch() {
        for ratio in [0.5, 0.8, 1.25, 2.0] {
            let out = stretch(&sine(440.0, 1.0), ratio);
            assert_eq!(
                out.frames(),
                (48_000.0 * ratio).round() as usize,
                "length ×{ratio}"
            );
            let mid = &out.left[out.frames() / 8..out.frames() * 7 / 8];
            assert!(
                (hz(mid) - 440.0).abs() < 6.0,
                "pitch at ×{ratio}: {}",
                hz(mid)
            );
            // Smooth joins: no sample jumps much more than a 440 Hz sine's steepest step.
            let steepest = mid
                .windows(2)
                .map(|w| (w[1] - w[0]).abs())
                .fold(0.0f32, f32::max);
            assert!(steepest < 0.05, "a seam at ×{ratio}: {steepest}");
        }
    }

    #[test]
    fn the_same_length_is_a_copy() {
        let c = sine(100.0, 0.1);
        assert_eq!(stretch(&c, 1.0).left, c.left);
    }
}
