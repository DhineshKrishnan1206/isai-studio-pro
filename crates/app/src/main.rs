mod audio;
mod browser_ops;
mod computer_keys;
mod controller;
mod export_ops;
mod format;
mod mastering;
mod midi_input;
mod motion;
mod plugin_ops;
mod pool;
mod project_ops;
mod record_ops;
mod recorder;
mod sampler_ops;
mod selection_ops;
mod session;
mod shared;
mod slicer;
mod spectrum;
mod stems_ops;
mod tap;
mod tuner;
mod user_presets;
mod waveform;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};

fn main() {
    // `isai --scan-plugins <file>`: write the installed plugins to <file> as JSON
    // and quit. The app runs this as a separate process, so a plugin that crashes
    // while loading can't take it down. (A file, not stdout: plugins print to stdout.)
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--scan-plugins") {
        let json = isai_plugins::scan_json();
        match args.get(i + 1) {
            Some(out) => {
                let _ = std::fs::write(out, json);
            }
            None => println!("{json}"),
        }
        return;
    }
    // Video plays through Qt's FFmpeg backend on every platform, so the same
    // files open everywhere (the OS decoders only cover some containers).
    // SAFETY: nothing else is running yet, so no other thread reads the environment.
    unsafe { std::env::set_var("QT_MEDIA_BACKEND", "ffmpeg") };
    let mut app = QGuiApplication::new();
    if let Some(mut app) = app.as_mut() {
        app.as_mut()
            .set_application_name(&QString::from("Isai Studio Pro"));
        // Where QML's own little preferences (e.g. the tuner's pitch) are kept.
        app.as_mut().set_organization_name(&QString::from("Isai"));
    }
    let mut engine = QQmlApplicationEngine::new();
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from("qrc:/qt/qml/Isai/qml/Main.qml"));
    }
    if let Some(app) = app.as_mut() {
        app.exec();
    }
}
