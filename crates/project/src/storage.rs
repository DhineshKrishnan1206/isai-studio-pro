//! Saving and opening projects.
//!
//! A project is a folder, so it can be zipped, backed up or moved to another
//! computer in one piece:
//!
//! ```text
//! My Song/
//! ├── My Song.isai     the project (readable text, RON format)
//! └── Audio/           every audio file the project uses
//! ```
//!
//! Audio imported before the first save is referenced where it lies; saving
//! copies it into `Audio/`, so the project never depends on files elsewhere.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, bail};
use serde::{Deserialize, Serialize};

use crate::{FORMAT_VERSION, Project};

pub const EXTENSION: &str = "isai";
pub const AUDIO_DIR: &str = "Audio";
pub const VIDEO_DIR: &str = "Video";

/// Where a project lives on disk.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectPaths {
    /// The project folder.
    pub dir: PathBuf,
    /// The `.isai` file inside it.
    pub file: PathBuf,
}

impl ProjectPaths {
    /// Paths for a project the user is saving as `chosen` (e.g. from a Save
    /// dialog: `~/Music/My Song.isai`). The project gets its own folder next
    /// to it: `~/Music/My Song/My Song.isai`.
    pub fn for_new(chosen: &Path) -> Self {
        let stem = chosen
            .file_stem()
            .map_or_else(|| "Untitled".into(), |s| s.to_string_lossy().into_owned());
        let parent = chosen.parent().unwrap_or(Path::new("."));
        // If they picked a spot inside a folder already named after the song, use it.
        let dir = if parent
            .file_name()
            .is_some_and(|n| n.to_string_lossy() == stem)
        {
            parent.to_path_buf()
        } else {
            parent.join(&stem)
        };
        let file = dir.join(format!("{stem}.{EXTENSION}"));
        Self { dir, file }
    }

    /// Paths for an existing `.isai` file.
    pub fn for_file(file: &Path) -> Self {
        let dir = file.parent().unwrap_or(Path::new(".")).to_path_buf();
        Self {
            dir,
            file: file.to_path_buf(),
        }
    }

    pub fn audio_dir(&self) -> PathBuf {
        self.dir.join(AUDIO_DIR)
    }

    pub fn video_dir(&self) -> PathBuf {
        self.dir.join(VIDEO_DIR)
    }

    /// The real location of a video clip's file.
    pub fn resolve_video(&self, file: &str) -> PathBuf {
        let path = Path::new(file);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.video_dir().join(path)
        }
    }

    /// The real location of a region's audio file.
    pub fn resolve(&self, file: &str) -> PathBuf {
        resolve(Some(self), file)
    }
}

/// The real location of an audio file, for a saved project or not.
pub fn resolve(paths: Option<&ProjectPaths>, file: &str) -> PathBuf {
    let path = Path::new(file);
    match paths {
        Some(p) if !path.is_absolute() => p.audio_dir().join(path),
        _ => path.to_path_buf(),
    }
}

/// Saves `project` to `paths`, first copying any audio that lives outside the
/// project folder into `Audio/`. Returns the project as saved (with audio
/// paths made relative), which the caller should keep using.
pub fn save(
    project: &Project,
    paths: &ProjectPaths,
    previous: Option<&ProjectPaths>,
) -> anyhow::Result<Project> {
    let audio_dir = paths.audio_dir();
    fs::create_dir_all(&audio_dir)
        .with_context(|| format!("couldn't create {}", audio_dir.display()))?;

    let mut saved = project.clone();
    saved.version = FORMAT_VERSION;
    for track in &mut saved.tracks {
        for region in track.regions.iter_mut().filter(|r| r.midi.is_none()) {
            let source = resolve(previous, &region.file);
            if source.parent() == Some(audio_dir.as_path()) {
                region.file = file_name(&source);
                continue;
            }
            region.file = copy_into(&source, &audio_dir)?;
        }
        // A frozen track's audio, and a sampler's sound, travel with the project too.
        if let Some(frozen) = track.frozen.as_mut() {
            let source = resolve(previous, frozen);
            *frozen = if source.parent() == Some(audio_dir.as_path()) {
                file_name(&source)
            } else {
                copy_into(&source, &audio_dir)?
            };
        }
        if let Some(sample) = track.instrument.as_mut().and_then(|i| i.sample.as_mut()) {
            let source = resolve(previous, sample);
            *sample = if source.parent() == Some(audio_dir.as_path()) {
                file_name(&source)
            } else {
                copy_into(&source, &audio_dir)?
            };
        }
    }

    if !saved.videos.is_empty() {
        let video_dir = paths.video_dir();
        fs::create_dir_all(&video_dir)
            .with_context(|| format!("couldn't create {}", video_dir.display()))?;
        for video in &mut saved.videos {
            let source = match previous {
                Some(p) if !Path::new(&video.file).is_absolute() => p.resolve_video(&video.file),
                _ => PathBuf::from(&video.file),
            };
            video.file = if source.parent() == Some(video_dir.as_path()) {
                file_name(&source)
            } else {
                copy_into(&source, &video_dir)?
            };
        }
    }

    let text = ron::ser::to_string_pretty(&saved, ron::ser::PrettyConfig::default())?;
    keep_backup(paths, &text)?;
    write_atomically(&paths.file, &text)?;
    Ok(saved)
}

/// Folder inside the project where earlier saved versions are kept.
pub const BACKUP_DIR: &str = "Backups";
/// How many earlier versions to keep; the oldest are deleted past this.
pub const KEEP_BACKUPS: usize = 20;

/// Before a save replaces the project file, keeps the version on disk as a
/// backup, unless it is already the same text (saving twice in a row keeps one).
fn keep_backup(paths: &ProjectPaths, new_text: &str) -> anyhow::Result<()> {
    let Ok(old) = fs::read_to_string(&paths.file) else {
        return Ok(());
    };
    if old == new_text {
        return Ok(());
    }
    let dir = paths.dir.join(BACKUP_DIR);
    fs::create_dir_all(&dir).with_context(|| format!("couldn't create {}", dir.display()))?;
    let stem = project_stem(paths);
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    // Two saves in the same millisecond still get their own names.
    let target = (0..)
        .map(|n| {
            if n == 0 {
                dir.join(format!("{stem} {millis}.{EXTENSION}"))
            } else {
                dir.join(format!("{stem} {millis} ({n}).{EXTENSION}"))
            }
        })
        .find(|p| !p.exists())
        .unwrap_or_default();
    fs::write(&target, old).with_context(|| format!("couldn't back up to {}", target.display()))?;
    prune_backups(paths);
    Ok(())
}

/// Deletes all but the newest `KEEP_BACKUPS` backups. Failures are ignored: a
/// backup that can't be removed is not worth failing a save over.
fn prune_backups(paths: &ProjectPaths) {
    for old in list_backups(paths).into_iter().skip(KEEP_BACKUPS) {
        let _ = fs::remove_file(old);
    }
}

/// The project's backups, newest first.
pub fn list_backups(paths: &ProjectPaths) -> Vec<PathBuf> {
    let stem = project_stem(paths);
    let prefix = format!("{stem} ");
    let suffix = format!(".{EXTENSION}");
    let Ok(entries) = fs::read_dir(paths.dir.join(BACKUP_DIR)) else {
        return Vec::new();
    };
    let mut found: Vec<(u128, PathBuf)> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter_map(|p| {
            let name = p.file_name()?.to_str()?.to_owned();
            let rest = name.strip_prefix(&prefix)?.strip_suffix(&suffix)?;
            // The time comes first; a " (n)" tag for same-millisecond saves follows it.
            let millis = rest.split(' ').next()?.parse::<u128>().ok()?;
            Some((millis, p))
        })
        .collect();
    found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
    found.into_iter().map(|(_, p)| p).collect()
}

/// When a backup was made (milliseconds since 1970), from its file name.
pub fn backup_time(backup: &Path) -> Option<u128> {
    let stem = backup.file_stem()?.to_str()?;
    stem.rsplit(' ').find_map(|part| part.parse::<u128>().ok())
}

fn project_stem(paths: &ProjectPaths) -> String {
    paths
        .file
        .file_stem()
        .map_or_else(|| "Untitled".into(), |s| s.to_string_lossy().into_owned())
}

/// Opens a project file.
pub fn load(file: &Path) -> anyhow::Result<(Project, ProjectPaths)> {
    let text =
        fs::read_to_string(file).with_context(|| format!("couldn't read {}", file.display()))?;
    let project: Project = ron::from_str(&text).context("this isn't a valid Isai project file")?;
    if project.version > FORMAT_VERSION {
        bail!("this project was made with a newer version of Isai; please update to open it");
    }
    Ok((project, ProjectPaths::for_file(file)))
}

/// Copies `source` into `dir`, keeping its name unless a different file
/// already has it. Returns the name used.
fn copy_into(source: &Path, dir: &Path) -> anyhow::Result<String> {
    let name = file_name(source);
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) => (s.to_owned(), format!(".{e}")),
        None => (name.clone(), String::new()),
    };
    let mut candidate = name.clone();
    for n in 2.. {
        let target = dir.join(&candidate);
        if !target.exists() {
            fs::copy(source, &target)
                .with_context(|| format!("couldn't copy {} into the project", source.display()))?;
            return Ok(candidate);
        }
        if same_size(source, &target) {
            // Already copied by an earlier save.
            return Ok(candidate);
        }
        candidate = format!("{stem} {n}{ext}");
    }
    unreachable!()
}

fn same_size(a: &Path, b: &Path) -> bool {
    match (fs::metadata(a), fs::metadata(b)) {
        (Ok(a), Ok(b)) => a.len() == b.len(),
        _ => false,
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map_or_else(|| "audio".into(), |n| n.to_string_lossy().into_owned())
}

/// Writes next to the target and renames over it, so a crash mid-save can
/// never leave a half-written project.
fn write_atomically(path: &Path, text: &str) -> anyhow::Result<()> {
    let tmp = path.with_extension(format!("{EXTENSION}.tmp"));
    fs::write(&tmp, text).with_context(|| format!("couldn't write {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("couldn't save {}", path.display()))?;
    Ok(())
}

/// A periodic copy of the open project, kept in the app's own folder, so work
/// can be recovered after a crash, even for projects never saved.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Autosave {
    /// Seconds since 1970, when the copy was made.
    pub saved_at: u64,
    /// The project's file, if it has been saved before.
    pub file: Option<String>,
    pub project: Project,
}

impl Autosave {
    const FILE: &str = "autosave.isai";

    pub fn new(project: &Project, paths: Option<&ProjectPaths>) -> Self {
        let saved_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        Self {
            saved_at,
            file: paths.map(|p| p.file.display().to_string()),
            project: project.clone(),
        }
    }

    pub fn write(&self, app_dir: &Path) -> anyhow::Result<()> {
        fs::create_dir_all(app_dir)?;
        let text = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())?;
        write_atomically(&app_dir.join(Self::FILE), &text)
    }

    pub fn read(app_dir: &Path) -> Option<Self> {
        let text = fs::read_to_string(app_dir.join(Self::FILE)).ok()?;
        ron::from_str(&text).ok()
    }

    /// Removes the copy, e.g. after a clean save or when the user discards it.
    pub fn clear(app_dir: &Path) {
        let _ = fs::remove_file(app_dir.join(Self::FILE));
    }

    pub fn paths(&self) -> Option<ProjectPaths> {
        self.file
            .as_deref()
            .map(|f| ProjectPaths::for_file(Path::new(f)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use isai_engine::Meter;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("isai-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn new_projects_get_their_own_folder() {
        let p = ProjectPaths::for_new(Path::new("/music/My Song.isai"));
        assert_eq!(p.dir, PathBuf::from("/music/My Song"));
        assert_eq!(p.file, PathBuf::from("/music/My Song/My Song.isai"));
        // Choosing a spot already inside the song's folder doesn't nest it again.
        let p = ProjectPaths::for_new(Path::new("/music/My Song/My Song.isai"));
        assert_eq!(p.dir, PathBuf::from("/music/My Song"));
    }

    #[test]
    fn saving_copies_audio_in_and_round_trips() {
        let root = temp_dir("save");
        let outside = root.join("drums.wav");
        fs::write(&outside, b"fake audio").unwrap();

        let mut project = Project::new("Song", Meter::default(), "A minor");
        let t = project.add_track(None);
        project.add_region(t, &outside.display().to_string(), "drums", 4.0, 1.5);

        let paths = ProjectPaths::for_new(&root.join("Song.isai"));
        let saved = save(&project, &paths, None).unwrap();
        assert_eq!(saved.tracks[0].regions[0].file, "drums.wav");
        assert!(paths.audio_dir().join("drums.wav").exists());

        let (loaded, loaded_paths) = load(&paths.file).unwrap();
        assert_eq!(loaded, saved);
        assert_eq!(loaded_paths, paths);
        assert_eq!(
            loaded_paths.resolve("drums.wav"),
            paths.audio_dir().join("drums.wav")
        );

        // Saving again doesn't duplicate the audio.
        save(&loaded, &loaded_paths, Some(&loaded_paths)).unwrap();
        assert_eq!(fs::read_dir(paths.audio_dir()).unwrap().count(), 1);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn saving_copies_videos_into_the_video_folder() {
        let root = temp_dir("video");
        let outside = root.join("scene.mov");
        fs::write(&outside, b"fake video").unwrap();

        let mut project = Project::new("Score", Meter::default(), "C major");
        let track = project.add_video_track("Video");
        project.add_video(&outside.display().to_string(), "scene", 8.0, 12.0, track);
        let paths = ProjectPaths::for_new(&root.join("Score.isai"));
        let saved = save(&project, &paths, None).unwrap();
        assert_eq!(saved.videos[0].file, "scene.mov");
        assert!(paths.video_dir().join("scene.mov").exists());

        let (loaded, loaded_paths) = load(&paths.file).unwrap();
        assert_eq!(loaded, saved);
        assert_eq!(
            loaded_paths.resolve_video("scene.mov"),
            paths.video_dir().join("scene.mov")
        );

        // Saving again doesn't duplicate the video.
        save(&loaded, &loaded_paths, Some(&loaded_paths)).unwrap();
        assert_eq!(fs::read_dir(paths.video_dir()).unwrap().count(), 1);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn different_files_with_the_same_name_are_kept_apart() {
        let root = temp_dir("names");
        fs::create_dir_all(root.join("a")).unwrap();
        fs::create_dir_all(root.join("b")).unwrap();
        fs::write(root.join("a/take.wav"), b"one").unwrap();
        fs::write(root.join("b/take.wav"), b"second").unwrap();

        let mut project = Project::default();
        let t = project.add_track(None);
        project.add_region(
            t,
            &root.join("a/take.wav").display().to_string(),
            "a",
            0.0,
            1.0,
        );
        project.add_region(
            t,
            &root.join("b/take.wav").display().to_string(),
            "b",
            4.0,
            1.0,
        );
        let saved = save(&project, &ProjectPaths::for_new(&root.join("P.isai")), None).unwrap();
        let files: Vec<_> = saved.tracks[0]
            .regions
            .iter()
            .map(|r| r.file.clone())
            .collect();
        assert_eq!(files, vec!["take.wav", "take 2.wav"]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn autosave_round_trips_and_clears() {
        let dir = temp_dir("autosave");
        let project = Project::new("Draft", Meter::default(), "C major");
        Autosave::new(&project, None).write(&dir).unwrap();
        let back = Autosave::read(&dir).unwrap();
        assert_eq!(back.project, project);
        assert!(back.file.is_none());
        Autosave::clear(&dir);
        assert!(Autosave::read(&dir).is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn saving_keeps_the_previous_version_as_a_backup() {
        let root = temp_dir("backups");
        let paths = ProjectPaths::for_new(&root.join("Song.isai"));
        let mut project = Project::new("Song", Meter::default(), "C major");

        save(&project, &paths, None).unwrap();
        assert!(
            list_backups(&paths).is_empty(),
            "the first save has nothing to back up"
        );

        project.tempo = 140.0;
        save(&project, &paths, None).unwrap();
        let backups = list_backups(&paths);
        assert_eq!(backups.len(), 1);
        assert!(backups[0].starts_with(paths.dir.join(BACKUP_DIR)));
        let (old, _) = load(&backups[0]).unwrap();
        assert_eq!(
            old.tempo, 120.0,
            "the backup holds the version before the save"
        );

        // Saving the same text again doesn't add a copy.
        save(&project, &paths, None).unwrap();
        assert_eq!(list_backups(&paths).len(), 1);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn backups_are_pruned_to_the_newest() {
        let root = temp_dir("prune");
        let paths = ProjectPaths::for_new(&root.join("Song.isai"));
        let mut project = Project::new("Song", Meter::default(), "C major");
        // One change per save, so every save keeps a backup.
        for n in 0..(KEEP_BACKUPS + 5) {
            project.tempo = 60.0 + n as f64;
            save(&project, &paths, None).unwrap();
        }
        assert_eq!(list_backups(&paths).len(), KEEP_BACKUPS);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_files_from_the_future() {
        let dir = temp_dir("future");
        let p = Project {
            version: FORMAT_VERSION + 1,
            ..Default::default()
        };
        let file = dir.join("x.isai");
        fs::write(&file, ron::ser::to_string(&p).unwrap()).unwrap();
        assert!(load(&file).is_err());
        let _ = fs::remove_dir_all(&dir);
    }
}
