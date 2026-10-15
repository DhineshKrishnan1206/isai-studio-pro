# Roadmap

Each phase ends with a release that is useful on its own.

## Phase 0: Audio engine ✅ in progress

- [x] Real-time engine with a lock-free command queue
- [x] Load and resample WAV, FLAC, MP3, OGG and AAC files
- [x] Play, stop, loop, seek, volume and click-free start/stop
- [x] Level meters, audio CPU load and dropout detection
- [x] First-launch setup with hardware detection, benchmark and profiles
- [x] Settings window; every option saved to `settings.toml`
- [ ] Test on real Windows and Linux machines, including a low-spec one
- [ ] Follow the OS "reduce motion" setting automatically
- [x] Choose the name: Isai Studio Pro

## Phase 1: Sound & rhythm basics ✅

- [x] Audio output to speakers, headphones or an interface
- [x] Master clock in bars and beats; tempo (BPM), time signature and key; tap tempo
- [x] Metronome with accented downbeats, count-in (1–2 bars) and adjustable volume
- [x] Transport: play, stop, go to start
- [x] New Project screen with templates
- [x] Bars & beats ruler, playhead, grid, snap and cycle region
- [ ] Stop pressed twice jumps back to the start (DAW convention)

## Phase 2: Tracks, clips and recording

Projects
- [x] Save and open projects (`.isai` folder); autosave and crash recovery; recent projects
- [x] Undo and redo for every edit

Tracks and clips
- [x] Multiple audio tracks: add, rename, colour, delete
- [x] Reorder tracks by dragging
- [x] Clips (regions): move within and across tracks, with snap
- [x] Trim clip edges, split with scissors, fade in/out and crossfades
- [x] Timeline zoom and scroll

Playback and basic mixing
- [x] Summing engine: any number of tracks played together, with headroom
- [x] Per-track volume fader, pan knob, mute (M) and solo (S)
- [ ] Master bus with level meter and clip warning

Recording
- [x] Audio input: choose microphone or interface inputs
- [x] Record-arm per track, input monitoring, record with count-in
- [x] Latency compensation so recordings land on the beat

Sharing
- [x] Export (bounce) the song, the cycle region or stems to WAV and FLAC (MP3 later)

- [x] ⌘K command palette (search every command; type "128 bpm", "bar 17", "3/4", "A minor")

Also in this phase: UI scale, keyboard shortcut list.

## Phase 3: Arrangement extras

- [ ] Markers (Intro, Verse, Chorus…) and arrangement sections
- [ ] Tempo and time-signature changes along the timeline
- [ ] Time-stretch audio to the project tempo

## Phase 4: MIDI and virtual instruments

- [ ] MIDI keyboard input (all devices)
- [ ] Software instrument tracks; record MIDI from a keyboard
- [ ] Piano roll: draw, move, stretch and delete notes; velocity editing
- [ ] Quantize (snap notes to the beat), with strength and swing
- [ ] Built-in instruments: sampler, synth, drum machine (eco/HQ modes)
- [ ] Third-party plugins: CLAP, then VST3 (and AU on macOS)

## Phase 5: Mixing desk and effects

- [ ] Mixer view: channel strips, tall faders, meters, mute/solo
- [ ] Effect slots per track; built-in EQ, compressor, reverb, delay
- [ ] Buses and sends (e.g. one shared reverb)
- [ ] Automation lanes: draw volume, pan and effect changes over time
- [ ] Track freeze: render heavy tracks to audio to save CPU

## Phase 6: Logic-style extras

- [ ] Stem splitting (separate vocals, drums, bass, other)
- [ ] Tempo detection for imported audio
- [ ] AI drummer
- [ ] Free loop and sample library (CC0)
