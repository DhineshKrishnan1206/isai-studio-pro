//! Lists the installed plugins, then loads one and plays a note through it.
//!     cargo run -p isai-plugins --example probe -- [name]
use isai_engine::plugin::{NoteEvent, Transport};

fn main() {
    let found = isai_plugins::scan();
    for p in &found {
        println!("{:<5} {:<24} {:<22} {:<28} instrument={} gui={}", p.format.label(), p.name, p.vendor, p.category, p.instrument, p.has_gui);
    }
    let want = std::env::args().nth(1).unwrap_or_default().to_lowercase();
    let Some(info) = found.iter().find(|p| !want.is_empty() && p.name.to_lowercase().contains(&want)) else { return };
    println!("\nloading {} …", info.name);
    let t0 = std::time::Instant::now();
    let plugin = match isai_plugins::load(info, 48_000.0) {
        Ok(p) => p,
        Err(e) => return println!("failed: {e}"),
    };
    println!("loaded in {:?}; editor: {}", t0.elapsed(), plugin.has_editor());
    let handle = plugin.handle();
    let transport = Transport { bpm: 120.0, playing: true, sample_rate: 48_000.0 };
    let (mut l, mut r) = (vec![0.0f32; 512], vec![0.0f32; 512]);
    let mut peak = 0.0f32;
    for block in 0..200 {
        l.fill(0.0);
        r.fill(0.0);
        let events: Vec<NoteEvent> = match block {
            0 => vec![NoteEvent { offset: 0, key: 60, velocity: 110 }],
            120 => vec![NoteEvent { offset: 0, key: 60, velocity: 0 }],
            _ => vec![],
        };
        let ok = handle.0.process(&mut l, &mut r, &events, &transport, true);
        peak = l.iter().chain(&r).fold(peak, |m, s| m.max(s.abs()));
        if block % 40 == 0 {
            println!("block {block:>3}: processed={ok} peak so far={peak:.4}");
        }
    }
    let state = plugin.save_state().map_or(0, |s| s.len());
    println!("state: {state} bytes");
}
