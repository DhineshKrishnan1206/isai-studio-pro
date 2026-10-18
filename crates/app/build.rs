use std::fs;
use std::path::Path;

use cxx_qt_build::{CxxQtBuilder, QmlFile, QmlModule};

/// Every .qml file under `qml/`, so new components are picked up automatically.
fn qml_files(dir: &Path, out: &mut Vec<String>) {
    let mut entries: Vec<_> = fs::read_dir(dir).expect("qml dir").flatten().collect();
    entries.sort_by_key(|e| e.path());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            qml_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "qml") {
            out.push(path.to_string_lossy().into_owned());
        }
    }
}

fn main() {
    let mut files = Vec::new();
    qml_files(Path::new("qml"), &mut files);
    println!("cargo::rerun-if-changed=qml");

    // Theme and Icons are singletons: `Theme.accent`, `Icons.play`.
    let singletons = ["qml/Theme.qml", "qml/Icons.qml", "qml/Auto.qml", "qml/TimeMap.qml", "qml/Learn.qml"];
    let qml: Vec<QmlFile> = files
        .iter()
        .map(|f| QmlFile::from(f.as_str()).singleton(singletons.contains(&f.as_str())))
        .collect();

    // Fonts, brand images and backdrop art are compiled into the binary.
    let assets: Vec<String> = ["assets/fonts", "assets/brand", "assets/art"]
        .iter()
        .flat_map(|dir| fs::read_dir(dir).expect("assets dir").flatten())
        // Only files: brand/source/ holds the original artwork, not shipped.
        .filter(|e| e.path().is_file())
        .map(|e| e.path().to_string_lossy().into_owned())
        .collect();

    CxxQtBuilder::new_qml_module(QmlModule::new("Isai").qml_files(qml))
        .qt_module("Quick")
        .qrc_resources(assets)
        .files(["src/controller.rs", "src/waveform.rs"])
        .build();
}
