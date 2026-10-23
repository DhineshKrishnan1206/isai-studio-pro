# Roadmap

Isai is early development. [FEATURES.md](FEATURES.md) is the detailed feature
inventory; this roadmap orders the work that best improves trust and usability
for a first public release. Priorities may change as people test the app.

## 1. Release confidence

- [x] Keep the audio engine independent from the UI and test the workspace in CI.
- [x] Build Windows and macOS trial artifacts from GitHub Actions.
- [x] Add a Linux x86_64 archive and a CI launch/screenshot smoke test.
- [ ] Test recording, playback, device reconnect, project recovery and plugins on
  real Windows and Linux machines, including a low-spec PC.
- [x] Restore earlier saved versions from the project's `Backups/` folder (File menu).
- [x] Follow the operating system's reduce-motion setting automatically (macOS checked; Windows and GNOME Linux read but not tested on those systems).

## 2. Move projects between tools

- [x] Import and export standard MIDI files (one region per import; see FEATURES.md).
- [ ] Import and export DAWproject, starting with a clearly documented subset.
- [x] Add project alternatives so experiments do not overwrite a song's main mix.

## 3. Polish core workflows

- [ ] Multitrack recording, punch in/out and take comping.
- [ ] Improve accessibility and keyboard-only operation across the app.
- [ ] Stream long audio files from disk instead of loading every file fully into
  memory.
- [ ] Keep improving low-spec performance using real project benchmarks.

## 4. Expand only where users need it

Use the remaining items in [FEATURES.md](FEATURES.md) to choose future work.
Prefer finishing an end-to-end workflow over adding isolated feature-parity
items; keep the engine real-time safe and preserve projects across upgrades.

The package workflow currently creates ad-hoc signed macOS trial builds and a
Linux archive that requires a system Qt 6.9+ runtime. These are development
artifacts, not yet polished installers or Developer ID signed/notarized public
releases.
