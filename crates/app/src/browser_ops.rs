//! The controller's sound browser side: previewing files, importing them
//! onto a track, and finding sounds in a folder.

use std::path::{Path, PathBuf};
use std::pin::Pin;

use cxx_qt_lib::{QString, QUrl};
use serde_json::json;

use crate::controller::{qobject::AppController, qs};
use crate::project_ops::{id, url_path};

/// Files the browser shows: audio the app can open, and projects.
pub(crate) const SOUND_EXTENSIONS: [&str; 10] = ["wav", "flac", "mp3", "ogg", "m4a", "aac", "aif", "aiff", "caf", "isai"];
const MAX_RESULTS: usize = 300;
/// Folders deeper than this aren't searched (a whole disk would take too long).
const MAX_DEPTH: usize = 6;

impl AppController {
    pub fn preview_file(mut self: Pin<&mut Self>, url: &QUrl, sync: bool) {
        let Some(path) = url_path(url) else { return };
        if path.extension().is_some_and(|e| e == isai_project::EXTENSION) {
            return;
        }
        let st = self.as_mut().st();
        // Only the project's audio, pending imports and this preview stay decoded.
        let mut keep = st.session.as_ref().map(crate::session::Session::audio_paths).unwrap_or_default();
        keep.extend(st.pending_imports.iter().map(|(p, _)| p.clone()));
        keep.push(path.clone());
        st.pool.retain(&keep);
        st.pool.request(&path);
        match st.pool.ready(&path).cloned() {
            Some(clip) => {
                st.preview_pending = None;
                if let Some(engine) = &mut st.audio.engine {
                    engine.preview(Some(clip), sync);
                }
            }
            None => st.preview_pending = Some((path, sync)),
        }
        self.as_mut().set_previewing(true);
        self.update_busy();
    }

    pub fn stop_preview(mut self: Pin<&mut Self>) {
        let st = self.as_mut().st();
        st.preview_pending = None;
        if let Some(engine) = &mut st.audio.engine {
            engine.preview(None, false);
        }
        self.set_previewing(false);
    }

    /// Keeps `previewing` up to date while a preview plays.
    pub(crate) fn poll_preview(mut self: Pin<&mut Self>) {
        let st = self.as_mut().st();
        let on = st.preview_pending.is_some() || st.audio.engine.as_ref().is_some_and(|e| e.previewing());
        if on != *self.previewing() {
            self.set_previewing(on);
        }
    }

    pub fn import_at(mut self: Pin<&mut Self>, url: &QUrl, track: i32, beats: f64) {
        let Some(path) = url_path(url) else { return };
        if path.extension().is_some_and(|e| e == isai_project::EXTENSION) {
            self.open_path(&path);
            return;
        }
        let target = (track >= 0).then(|| (id(track), beats.max(0.0)));
        let st = self.as_mut().st();
        st.pool.request(&path);
        st.pending_imports.push((path, target));
        self.as_mut().set_loading(true);
        self.update_busy();
    }

    pub fn find_sounds(&self, folder: &QUrl, query: &QString) -> QString {
        let Some(root) = url_path(folder) else { return qs("[]") };
        let words: Vec<String> = query.to_string().to_lowercase().split_whitespace().map(str::to_owned).collect();
        let mut found = Vec::new();
        search(&root, &words, 0, &mut found);
        let items: Vec<_> = found
            .iter()
            .map(|p| {
                let name = p.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                let folder = p.parent().and_then(|d| d.strip_prefix(&root).ok()).map_or_else(String::new, |d| d.display().to_string());
                let kind = if p.extension().is_some_and(|e| e == "isai") { "project" } else { "audio" };
                json!({ "name": name, "path": p.display().to_string(), "folder": folder, "kind": kind })
            })
            .collect();
        qs(&serde_json::Value::Array(items).to_string())
    }
}

fn is_sound(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| SOUND_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

/// Sound files under `dir` whose names hold every word, depth first, sorted by name.
fn search(dir: &Path, words: &[String], depth: usize, out: &mut Vec<PathBuf>) {
    if depth > MAX_DEPTH || out.len() >= MAX_RESULTS {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if out.len() >= MAX_RESULTS {
            return;
        }
        let name = path.file_name().map_or_else(String::new, |n| n.to_string_lossy().to_lowercase());
        // Skip hidden folders and app bundles: they hold nothing to browse.
        if name.starts_with('.') || name.ends_with(".app") {
            continue;
        }
        if path.is_dir() {
            search(&path, words, depth + 1, out);
        } else if is_sound(&path) && words.iter().all(|w| name.contains(w.as_str())) {
            out.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_finds_matching_sounds_in_subfolders() {
        let dir = std::env::temp_dir().join(format!("isai-browse-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("Drums/Kicks")).unwrap();
        std::fs::create_dir_all(dir.join(".hidden")).unwrap();
        for f in ["Drums/Kicks/Big Kick.wav", "Drums/snare.flac", "notes.txt", ".hidden/kick.wav", "Song.isai"] {
            std::fs::write(dir.join(f), b"").unwrap();
        }
        let mut found = Vec::new();
        search(&dir, &["kick".into()], 0, &mut found);
        assert_eq!(found, vec![dir.join("Drums/Kicks/Big Kick.wav")], "hidden folders are skipped");
        found.clear();
        search(&dir, &[], 0, &mut found);
        assert_eq!(found.len(), 3, "audio and projects, not text: {found:?}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
