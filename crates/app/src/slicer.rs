//! Chopping a sample into slices (evenly, or where its beats start), and
//! guessing its tempo.

/// `count` equal slices of `frames` samples: where each starts.
pub fn even(frames: usize, count: usize) -> Vec<usize> {
    let count = count.max(1);
    (0..count).map(|i| i * frames / count).collect()
}

/// Where the hits in a sound start (drum hits, plucked notes, syllables),
/// at most `count` of them, earliest first. Always starts with 0.
///
/// It compares the energy of each short window with the one before it and
/// keeps the biggest jumps, at least 50 ms apart.
pub fn onsets(mono: &[f32], rate: u32, count: usize) -> Vec<usize> {
    const HOP: usize = 256;
    let windows = mono.len() / HOP;
    if windows < 4 || count <= 1 {
        return vec![0];
    }
    let energy: Vec<f32> = mono
        .chunks(HOP)
        .map(|w| w.iter().map(|s| s * s).sum::<f32>() / w.len() as f32)
        .collect();
    // How much louder each window is than the recent past (log scale, only rises).
    let mut flux: Vec<(usize, f32)> = (1..energy.len())
        .map(|i| {
            let before = energy[i.saturating_sub(3)..i]
                .iter()
                .cloned()
                .fold(0.0f32, f32::max);
            let rise = (energy[i] + 1e-7).log10() - (before + 1e-7).log10();
            (i, if energy[i] > 1e-6 { rise.max(0.0) } else { 0.0 })
        })
        .collect();
    flux.sort_by(|a, b| b.1.total_cmp(&a.1));
    let gap = (rate as usize / 20 / HOP).max(1);
    let mut picked: Vec<usize> = Vec::new();
    for (i, strength) in flux {
        if picked.len() + 1 >= count || strength < 0.3 {
            break;
        }
        if picked.iter().all(|&p| p.abs_diff(i) >= gap) && i >= gap {
            picked.push(i);
        }
    }
    // Move each onset back to the quiet just before its window, so the hit isn't clipped.
    let mut out: Vec<usize> = picked.into_iter().map(|i| (i - 1) * HOP).collect();
    out.push(0);
    out.sort_unstable();
    out.dedup();
    out
}

/// The tempo of a rhythmic sound (a drum loop, a song), in BPM between 85
/// and 170, if it has a clear beat.
///
/// It builds an envelope of how suddenly the sound gets louder, then finds
/// the spacing at which that envelope best repeats.
pub fn tempo(mono: &[f32], rate: u32) -> Option<f64> {
    const HOP: usize = 512;
    let energy: Vec<f32> = mono
        .chunks(HOP)
        .map(|w| w.iter().map(|s| s * s).sum::<f32>())
        .collect();
    if energy.len() < 64 {
        return None;
    }
    let flux: Vec<f32> = energy
        .windows(2)
        .map(|w| ((w[1] + 1e-9).ln() - (w[0] + 1e-9).ln()).max(0.0))
        .collect();
    let mean = flux.iter().sum::<f32>() / flux.len() as f32;
    let f: Vec<f32> = flux.iter().map(|x| x - mean).collect();
    let hops_per_sec = rate as f64 / HOP as f64;
    let lag_of = |bpm: f64| 60.0 / bpm * hops_per_sec;
    let (lo, hi) = (lag_of(180.0).floor() as usize, lag_of(60.0).ceil() as usize);
    if hi + 2 >= f.len() {
        return None;
    }
    let ac = |lag: usize| -> f32 {
        f.iter().zip(&f[lag..]).map(|(a, b)| a * b).sum::<f32>() / (f.len() - lag) as f32
    };
    let scores: Vec<(usize, f32)> = (lo..=hi).map(|lag| (lag, ac(lag))).collect();
    let &(best, score) = scores.iter().max_by(|a, b| a.1.total_cmp(&b.1))?;
    if score <= 0.0 {
        return None;
    }
    // Sharpen between whole lags.
    let at = |l: usize| {
        scores
            .iter()
            .find(|(x, _)| *x == l)
            .map_or(score, |(_, s)| *s)
    };
    let (a, b, c) = (at(best - 1), score, at(best + 1));
    let den = a - 2.0 * b + c;
    let shift = if den.abs() > 1e-12 {
        (0.5 * (a - c) / den).clamp(-0.5, 0.5)
    } else {
        0.0
    };
    let mut bpm = 60.0 * hops_per_sec / (best as f64 + f64::from(shift));
    while bpm < 85.0 {
        bpm *= 2.0;
    }
    while bpm > 170.0 {
        bpm /= 2.0;
    }
    Some((bpm * 10.0).round() / 10.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn even_slices_split_the_sample_equally() {
        assert_eq!(even(1000, 4), vec![0, 250, 500, 750]);
        assert_eq!(even(10, 0), vec![0]);
    }

    #[test]
    fn tempo_finds_the_beat_of_a_click_track() {
        for bpm in [92.0f64, 125.0, 140.0] {
            let rate = 48_000u32;
            let beat = (60.0 / bpm * f64::from(rate)) as usize;
            let mut click = vec![0.0f32; rate as usize * 8];
            for (i, s) in click.iter_mut().enumerate() {
                let since = i % beat;
                // A louder hit on every fourth beat, like a real bar.
                let level = if (i / beat).is_multiple_of(4) {
                    0.9
                } else {
                    0.5
                };
                *s = (since as f32 * 0.25).sin() * (-(since as f32) / 800.0).exp() * level;
            }
            let found = tempo(&click, rate).expect("a clear beat");
            assert!((found - bpm).abs() < 1.5, "{bpm} BPM read as {found}");
        }
        assert_eq!(tempo(&[0.0; 1000], 48_000), None, "too short");
    }

    #[test]
    fn onsets_find_the_hits_of_a_drum_loop() {
        // Four decaying clicks a quarter of a second apart, over a little noise.
        let rate = 48_000;
        let mut seed = 7u32;
        let mut loop_ = vec![0.0f32; rate as usize];
        for (i, s) in loop_.iter_mut().enumerate() {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
            *s = ((seed >> 16) as f32 / 65_536.0 - 0.5) * 0.002;
            let since = i % 12_000;
            *s += (since as f32 * 0.3).sin() * (-(since as f32) / 1500.0).exp() * 0.8;
        }
        let found = onsets(&loop_, rate, 8);
        assert_eq!(found.len(), 4, "{found:?}");
        for (f, want) in found.iter().zip([0usize, 12_000, 24_000, 36_000]) {
            assert!(f.abs_diff(want) <= 512, "{found:?}");
        }
        assert_eq!(onsets(&loop_, rate, 2).len(), 2, "no more than asked for");
    }
}
