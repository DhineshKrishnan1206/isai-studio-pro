//! Prints how many synth voices this machine can render in real time on one core.
//! Run with: cargo run -p isai-engine --release --example bench

fn main() {
    let voices = isai_engine::bench::voice_capacity(std::time::Duration::from_secs(1));
    println!("{voices} voices in real time on one core");
}
