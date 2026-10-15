//! Undo and redo by snapshots.
//!
//! A project without audio is small, so each undo step is simply a copy of the
//! project before the edit. Rapid edits of the same kind (dragging the tempo,
//! nudging a fader) are merged into one step.

use std::time::{Duration, Instant};

use crate::Project;

const LIMIT: usize = 200;
/// Edits of the same kind closer together than this become one undo step.
const MERGE_WINDOW: Duration = Duration::from_millis(1200);

#[derive(Default)]
pub struct History {
    undo: Vec<(String, Project)>,
    redo: Vec<(String, Project)>,
    last: Option<(String, Instant)>,
}

impl History {
    /// Call before changing `project`. `label` names the edit for the menu,
    /// e.g. "Move Region"; repeated labels within a moment are merged.
    pub fn record(&mut self, label: &str, project: &Project, now: Instant) {
        let merge = self
            .last
            .as_ref()
            .is_some_and(|(l, at)| l == label && now.duration_since(*at) < MERGE_WINDOW);
        self.last = Some((label.to_owned(), now));
        if merge {
            return;
        }
        self.undo.push((label.to_owned(), project.clone()));
        if self.undo.len() > LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// Restores the project from before the last edit. Returns its label.
    pub fn undo(&mut self, project: &mut Project) -> Option<String> {
        let (label, before) = self.undo.pop()?;
        self.redo.push((label.clone(), std::mem::replace(project, before)));
        self.last = None;
        Some(label)
    }

    pub fn redo(&mut self, project: &mut Project) -> Option<String> {
        let (label, after) = self.redo.pop()?;
        self.undo.push((label.clone(), std::mem::replace(project, after)));
        self.last = None;
        Some(label)
    }

    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|(l, _)| l.as_str())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|(l, _)| l.as_str())
    }

    /// Every step that can be undone, oldest first.
    pub fn undo_labels(&self) -> impl Iterator<Item = &str> {
        self.undo.iter().map(|(l, _)| l.as_str())
    }

    /// Every step that can be redone, the next one first.
    pub fn redo_labels(&self) -> impl Iterator<Item = &str> {
        self.redo.iter().rev().map(|(l, _)| l.as_str())
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_and_redo_restore_snapshots() {
        let mut h = History::default();
        let mut p = Project::default();
        let t0 = Instant::now();

        h.record("Add Track", &p, t0);
        p.add_track(None);
        h.record("Rename", &p, t0 + Duration::from_secs(5));
        p.tracks[0].name = "Vocals".into();

        assert_eq!(h.undo(&mut p).as_deref(), Some("Rename"));
        assert_eq!(p.tracks[0].name, "Audio 1");
        assert_eq!(h.undo(&mut p).as_deref(), Some("Add Track"));
        assert!(p.tracks.is_empty());
        assert_eq!(h.undo(&mut p), None);

        assert_eq!(h.redo_labels().collect::<Vec<_>>(), vec!["Add Track", "Rename"]);
        assert_eq!(h.redo(&mut p).as_deref(), Some("Add Track"));
        assert_eq!(h.undo_labels().collect::<Vec<_>>(), vec!["Add Track"]);
        assert_eq!(h.redo(&mut p).as_deref(), Some("Rename"));
        assert_eq!(p.tracks[0].name, "Vocals");
    }

    #[test]
    fn rapid_edits_of_one_kind_merge() {
        let mut h = History::default();
        let mut p = Project::default();
        let t0 = Instant::now();
        for i in 0..10 {
            h.record("Tempo", &p, t0 + Duration::from_millis(i * 100));
            p.tempo += 1.0;
        }
        assert_eq!(h.undo(&mut p).as_deref(), Some("Tempo"));
        assert_eq!(p.tempo, 120.0, "one undo reverts the whole drag");
        assert_eq!(h.undo(&mut p), None);
    }

    #[test]
    fn a_new_edit_clears_redo() {
        let mut h = History::default();
        let mut p = Project::default();
        let t0 = Instant::now();
        h.record("A", &p, t0);
        p.tempo = 90.0;
        h.undo(&mut p);
        h.record("B", &p, t0 + Duration::from_secs(5));
        assert_eq!(h.redo_label(), None);
    }
}
