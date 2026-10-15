//! Converts Deezer's release and splits test sounds (or a WAV file).
//!     cargo run --release -p isai-stems --example split -- 4stems.tar.gz [song.wav]
use std::time::Instant;

use isai_stems::{model, Separator, Weights, SAMPLE_RATE, STEMS};

fn energy(v: &[f32]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>() / v.len().max(1) as f32
}

fn run(sep: &Separator, name: &str, left: &[f32], right: &[f32], save: bool) {
    let t0 = Instant::now();
    let mut stems = vec![(Vec::new(), Vec::new()); 4];
    let threads = std::thread::available_parallelism().map_or(2, |n| n.get());
    sep.separate(left, right, threads, |s, l, r| {
        stems[s].0.extend_from_slice(l);
        stems[s].1.extend_from_slice(r);
    }, |_| {}, || false);
    let secs = left.len() as f32 / SAMPLE_RATE as f32;
    let e: Vec<String> = stems.iter().zip(STEMS).map(|((l, _), n)| format!("{n} {:.1}%", 100.0 * energy(l) / energy(left).max(1e-12))).collect();
    println!("{name}: {secs:.1} s of audio in {:.1} s → {}", t0.elapsed().as_secs_f32(), e.join(", "));
    if save {
        for ((l, r), n) in stems.iter().zip(STEMS) {
            let spec = hound::WavSpec { channels: 2, sample_rate: SAMPLE_RATE, bits_per_sample: 32, sample_format: hound::SampleFormat::Float };
            let mut w = hound::WavWriter::create(format!("{name}-{n}.wav"), spec).unwrap();
            for (a, b) in l.iter().zip(r) {
                w.write_sample(*a).unwrap();
                w.write_sample(*b).unwrap();
            }
            w.finalize().unwrap();
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let t0 = Instant::now();
    let release = std::fs::read(&args[1]).expect("the 4stems.tar.gz");
    let file = model::convert_release(&release).expect("convert");
    println!("converted in {:.1} s: {:.1} MB", t0.elapsed().as_secs_f32(), file.len() as f32 / 1e6);
    let weights = Weights::load(&file).expect("load");
    let sep = Separator::new(&weights).expect("networks");
    let rate = SAMPLE_RATE as f32;
    let n = (rate * 8.0) as usize;
    // A low bass line (55 and 73 Hz with a little harmonic), held notes.
    let bass: Vec<f32> = (0..n).map(|i| {
        let t = i as f32 / rate;
        let f = if ((t * 2.0) as u32).is_multiple_of(2) { 55.0 } else { 73.4 };
        0.4 * (t * f * std::f32::consts::TAU).sin() + 0.1 * (t * f * 2.0 * std::f32::consts::TAU).sin()
    }).collect();
    run(&sep, "bass-line", &bass, &bass, false);
    // A drum loop: noisy snare-like hits and pitched-down kicks, twice a second.
    let mut seed = 7u32;
    let drums: Vec<f32> = (0..n).map(|i| {
        let t = i as f32 / rate;
        let beat = t % 0.5;
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
        let noise = ((seed >> 16) as f32 / 32768.0) - 1.0;
        let kick = (beat * (60.0 + 200.0 * (-beat * 40.0).exp()) * std::f32::consts::TAU).sin() * (-beat * 12.0).exp();
        let snare = if (t % 1.0) >= 0.5 { noise * (-beat * 25.0).exp() * 0.6 } else { 0.0 };
        0.6 * kick + snare
    }).collect();
    run(&sep, "drum-loop", &drums, &drums, false);
    if let Some(path) = args.get(2) {
        let mut r = hound::WavReader::open(path).expect("wav");
        let spec = r.spec();
        let s: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => r.samples::<f32>().map(|x| x.unwrap()).collect(),
            hound::SampleFormat::Int => r.samples::<i32>().map(|x| x.unwrap() as f32 / (1u32 << (spec.bits_per_sample - 1)) as f32).collect(),
        };
        let ch = spec.channels as usize;
        let l: Vec<f32> = s.chunks(ch).map(|c| c[0]).collect();
        let rr: Vec<f32> = s.chunks(ch).map(|c| c[c.len().min(2) - 1]).collect();
        println!("(file at {} Hz; the model expects 44.1 kHz)", spec.sample_rate);
        run(&sep, "song", &l, &rr, true);
    }
}
