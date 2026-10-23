//! Presets people save themselves, per instrument or effect kind, kept in
//! `presets.json` next to the settings file:
//! `{ "piano": [ { "name": "…", "values": { key: value } } ] }`.

use std::path::PathBuf;

use serde_json::{Map, Value, json};

fn file() -> Option<PathBuf> {
    isai_settings::path().and_then(|p| p.parent().map(|d| d.join("presets.json")))
}

fn load() -> Map<String, Value> {
    file()
        .and_then(|f| std::fs::read_to_string(f).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn store(all: &Map<String, Value>) -> anyhow::Result<()> {
    let f = file().ok_or_else(|| anyhow::anyhow!("no settings folder"))?;
    if let Some(dir) = f.parent() {
        std::fs::create_dir_all(dir)?;
    }
    // Write beside it, then swap, so a crash never leaves half a file.
    let tmp = f.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(all)?)?;
    std::fs::rename(tmp, f)?;
    Ok(())
}

/// The saved presets for `kind`, as a JSON array.
pub fn list(kind: &str) -> String {
    load()
        .get(kind)
        .cloned()
        .unwrap_or_else(|| json!([]))
        .to_string()
}

/// Saves (or replaces, by name) a preset.
pub fn save(kind: &str, name: &str, values: &str) -> anyhow::Result<()> {
    let values: Value = serde_json::from_str(values)?;
    let mut all = load();
    let list = all.entry(kind.to_owned()).or_insert_with(|| json!([]));
    let items = list
        .as_array_mut()
        .ok_or_else(|| anyhow::anyhow!("presets file is damaged"))?;
    items.retain(|p| p.get("name").and_then(Value::as_str) != Some(name));
    items.push(json!({ "name": name, "values": values }));
    store(&all)
}

pub fn delete(kind: &str, name: &str) -> anyhow::Result<()> {
    let mut all = load();
    if let Some(items) = all.get_mut(kind).and_then(Value::as_array_mut) {
        items.retain(|p| p.get("name").and_then(Value::as_str) != Some(name));
    }
    store(&all)
}
