mod audio;
mod browser_ops;
mod controller;
mod export_ops;
mod format;
mod mastering;
mod midi_input;
mod pool;
mod project_ops;
mod record_ops;
mod recorder;
mod selection_ops;
mod sampler_ops;
mod session;
mod slicer;
mod spectrum;
mod tap;
mod tuner;
mod shared;
mod waveform;
mod user_presets;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};

fn main() {
    let mut app = QGuiApplication::new();
    if let Some(mut app) = app.as_mut() {
        app.as_mut().set_application_name(&QString::from("Isai Studio Pro"));
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
