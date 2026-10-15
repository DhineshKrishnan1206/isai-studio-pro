//! Selecting many regions, and copy / cut / paste / duplicate.
//!
//! The selection is a list of region ids kept in `State`. QML sees it as a
//! JSON array (`selectedRegions`); `selectedRegion` is the last one clicked,
//! which single-region commands such as Split use.

use std::pin::Pin;

use crate::controller::{qobject::AppController, qs};
use crate::project_ops::id;

impl AppController {
    /// Replaces the selection; the last id becomes the primary region and its
    /// track the selected track.
    pub(crate) fn set_selection(mut self: Pin<&mut Self>, ids: Vec<u64>) {
        let st = self.as_mut().st();
        let track = st
            .session
            .as_ref()
            .zip(ids.last())
            .and_then(|(s, &r)| s.project.find_region(r).map(|(t, _)| t.id as i32));
        let json = serde_json::to_string(&ids).unwrap_or_else(|_| "[]".into());
        let primary = ids.last().map_or(-1, |&r| r as i32);
        let count = ids.len() as i32;
        st.selection = ids;
        if let Some(track) = track {
            self.as_mut().set_selected_track(track);
        }
        self.as_mut().set_selected_region(primary);
        self.as_mut().set_selection_count(count);
        self.as_mut().set_selected_regions(qs(&json));
        self.route_live();
    }

    /// Drops selected regions that no longer exist (after undo, delete, open…).
    pub(crate) fn prune_selection(mut self: Pin<&mut Self>) {
        let st = self.as_mut().st();
        let Some(s) = &st.session else { return };
        let before = st.selection.len();
        let kept: Vec<u64> = st.selection.iter().copied().filter(|&r| s.project.find_region(r).is_some()).collect();
        if kept.len() != before {
            self.set_selection(kept);
        }
    }

    pub(crate) fn selection(self: Pin<&mut Self>) -> Vec<u64> {
        self.st().selection.clone()
    }

    pub fn toggle_region_selection(mut self: Pin<&mut Self>, region: i32) {
        let region = id(region);
        let mut ids = self.as_mut().selection();
        if let Some(i) = ids.iter().position(|&r| r == region) {
            ids.remove(i);
        } else {
            ids.push(region);
        }
        self.set_selection(ids);
    }

    pub fn select_regions_in(mut self: Pin<&mut Self>, from: f64, to: f64, top: i32, bottom: i32, add: bool) {
        let Some(s) = &self.as_mut().st().session else { return };
        let found = s.project.regions_in(from, to, top.max(0) as usize, bottom.max(0) as usize);
        let mut ids = if add { self.as_mut().selection() } else { Vec::new() };
        ids.extend(found.into_iter().filter(|r| !ids.contains(r)).collect::<Vec<_>>());
        self.set_selection(ids);
    }

    pub fn select_all_regions(mut self: Pin<&mut Self>) {
        let Some(s) = &self.as_mut().st().session else { return };
        let all = s.project.all_regions();
        self.set_selection(all);
    }

    pub fn copy_selected(mut self: Pin<&mut Self>) {
        let ids = self.as_mut().selection();
        let st = self.as_mut().st();
        let Some(clip) = st.session.as_ref().and_then(|s| s.project.copy_regions(&ids)) else { return };
        st.clipboard = Some(clip);
        self.set_can_paste(true);
    }

    pub fn cut_selected(mut self: Pin<&mut Self>) {
        self.as_mut().copy_selected();
        let ids = self.as_mut().selection();
        if self.as_mut().edit(if ids.len() == 1 { "Cut Region" } else { "Cut Regions" }, true, |p| p.remove_regions(&ids)) {
            self.set_selection(Vec::new());
        }
    }

    pub fn paste(mut self: Pin<&mut Self>) {
        let at = *self.position_beats();
        let track = id(*self.selected_track());
        let st = self.as_mut().st();
        let (Some(clip), Some(s)) = (st.clipboard.clone(), &st.session) else { return };
        let row = s.project.tracks.iter().position(|t| t.id == track).unwrap_or(0);
        let mut pasted = Vec::new();
        self.as_mut().edit("Paste", true, |p| {
            pasted = p.paste(&clip, at, row);
            !pasted.is_empty()
        });
        self.set_selection(pasted);
    }

    pub fn duplicate_selected(mut self: Pin<&mut Self>) {
        let ids = self.as_mut().selection();
        let mut copies = Vec::new();
        self.as_mut().edit("Duplicate", true, |p| {
            copies = p.duplicate_regions(&ids);
            !copies.is_empty()
        });
        if !copies.is_empty() {
            self.set_selection(copies);
        }
    }
}
