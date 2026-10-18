# Feature checklist: Logic Pro × FL Studio → Isai

What the two reference DAWs offer, grouped by area, and where Isai stands.
We add them one by one, roughly top to bottom within each priority group.

**Source:** L = Logic Pro, F = FL Studio, L/F = both.
**Isai:** ✅ done · 🟡 partly · ⬜ not yet

Built-in instruments and effects are listed by *what they do*; Isai builds
its own open-source versions rather than copying products.

---

## 1. Project & file handling

| Feature | Source | Isai |
|---|---|---|
| New project templates | L/F | ✅ |
| Save, Save As, Open, recent projects | L/F | ✅ |
| Autosave and crash recovery | L/F | ✅ |
| Self-contained project folder (audio copied in) | L | ✅ |
| Project alternatives (versions of the same song) | L | ⬜ |
| Automatic backups (restore an earlier save) | L/F | ⬜ |
| Project settings: tempo, time signature, key, sample rate | L/F | 🟡 (sample rate follows the device) |
| Clean up unused audio files / consolidate | L | ⬜ |
| Import/export MIDI files | L/F | ⬜ |
| Import projects from other DAWs (AAF/OMF, DAWproject) | L | ⬜ |

## 2. Transport & time

| Feature | Source | Isai |
|---|---|---|
| Play, stop, go to start | L/F | ✅ |
| Record button and shortcut | L/F | ✅ |
| Bars/beats and time display (LCD) | L/F | ✅ |
| Tempo, time signature, key | L/F | ✅ |
| Tap tempo | L/F | ✅ |
| Metronome with count-in | L/F | ✅ |
| Cycle / loop region | L/F | ✅ |
| Stop twice returns to start | L | ✅ (0 key / palette) |
| Tempo changes over time (tempo track) | L/F | ✅ (tempo lane, G; steps, ramps later) |
| Time signature changes over time | L/F | ✅ (signature lane; bars, grid, metronome follow) |
| Markers (Intro, Verse, Chorus…) | L/F | ✅ (marker lane, M, ⌥←/→, cycle a section) |
| Arrangement sections you can move/copy as blocks | L/F | ⬜ |
| Punch in/out recording (autopunch) | L/F | ⬜ |
| Varispeed (play everything faster/slower) | L | ⬜ |
| Smart Tempo: detect tempo of recordings/imports | L | 🟡 (detect a region's tempo when following) |
| Metronome settings: sound, subdivisions, only while recording | L/F | 🟡 (volume, count-in length) |

## 3. Arrangement / playlist editing

| Feature | Source | Isai |
|---|---|---|
| Multiple tracks, colours, rename, reorder | L/F | ✅ |
| Move regions in time and between tracks, with snap | L/F | ✅ |
| Trim, split, fades, automatic crossfades | L/F | ✅ |
| Zoom and scroll, follow playhead | L/F | ✅ |
| Undo/redo | L/F | ✅ |
| Undo history list (jump back several steps) | L/F | ✅ |
| Copy, cut, paste, duplicate regions (⌘D) | L/F | ✅ |
| Select many regions (marquee, ⇧-click) and edit together | L/F | ✅ |
| Loop a region by dragging its corner | L/F | ✅ |
| Region gain and reverse | L/F | ✅ (plus normalize) |
| Join regions, bounce region in place | L/F | 🟡 (join MIDI, rejoin split audio; bounce later) |
| Track folders / stacks (group tracks, summing) | L/F | ⬜ |
| Track alternatives (swap playlists on a track) | L | ⬜ |
| Take folders and comping (best parts of several takes) | L | ⬜ |
| Patterns: reusable clips placed many times (FL workflow) | F | ⬜ |
| Global tracks: tempo, signature, markers, chord, transposition | L | ⬜ |
| Arranger / song sections | L/F | ⬜ |

## 4. Recording

| Feature | Source | Isai |
|---|---|---|
| Choose audio input and channels | L/F | ✅ |
| Record-arm per track, input monitoring | L/F | ✅ |
| Latency compensation and manual correction | L/F | ✅ |
| Record several tracks at once (multitrack recording) | L/F | ⬜ |
| Cycle recording into take folders | L | ⬜ |
| Low-latency monitoring mode | L | ⬜ |
| Input level meter and clip warning while recording | L/F | 🟡 (meter, no clip warning yet) |
| Built-in tuner | L/F | ✅ (chromatic, guitar, bass; A4 reference) |
| Audio editor for a single recording (Edison-style) | F | ⬜ |

## 5. MIDI & note editing

| Feature | Source | Isai |
|---|---|---|
| MIDI keyboard input | L/F | ✅ |
| MIDI recording (keyboard or computer keys) | L/F | ✅ |
| Software instrument tracks | L/F | ✅ |
| Piano roll: draw, move, stretch, delete notes | L/F | ✅ |
| Velocity and note properties | L/F | 🟡 (velocity lane) |
| Quantize with strength and swing | L/F | ✅ |
| Scale and key highlighting in the piano roll | L/F | ✅ |
| Ghost notes (see other tracks' notes) | F | ✅ |
| Chord stamp, strum, arpeggiate, flam tools | F | ✅ (chords, chords in key, strum, arpeggiate) |
| Slide / portamento notes | F | ⬜ |
| Step sequencer (drum grid) | L/F | ✅ (Isai Drums window, pattern presets, swing) |
| Channel rack (FL-style pattern instruments) | F | ⬜ |
| Event list editor | L/F | ⬜ |
| Score / notation editor | L | ⬜ |
| MIDI effects: arpeggiator, chord trigger, randomizer | L/F | ⬜ |
| Riff/melody generator | F | ⬜ |
| Live Loops / clip launcher grid | L | ⬜ |
| Performance mode (launch clips live) | F | ⬜ |

## 6. Built-in instruments

| Feature | Source | Isai |
|---|---|---|
| Sampler (multi-sample instruments) | L/F | 🟡 (Isai Sampler: one sound across the keys; multi-sample later) |
| Quick sampler / slicer (chop a loop into pads) | L/F | ✅ (even or beat slicing, slices to pattern) |
| Drum machine with kits | L/F | ✅ (synthesised 808, Club, Acoustic, Lo-fi, Trap) |
| Subtractive synth (classic analog style) | L/F | ✅ (Isai Synth) |
| FM synth | L/F | ⬜ |
| Wavetable / additive synth | L/F | ⬜ |
| Piano, electric piano, organ | L/F | ✅ (Isai Piano: 9 tones incl. electric, reed, funk keys) |
| Strings, brass, orchestral | L | ⬜ |
| Bass synth (303-style) | F | ⬜ |
| AI drummer / virtual session players | L | ⬜ |
| Eco/HQ quality modes and voice limits (low-spec) | Isai | ✅ (Eco mode; exports stay full quality) |

## 7. Mixer

| Feature | Source | Isai |
|---|---|---|
| Per-track volume, pan, mute, solo | L/F | ✅ |
| Mixer window with channel strips and big meters | L/F | ✅ |
| Master bus with meter and clip warning | L/F | ✅ |
| Buses / aux sends (shared reverb, delay) | L/F | ✅ |
| Groups and VCAs | L | ⬜ |
| Sidechain routing | L/F | ⬜ |
| Insert effect slots per track | L/F | ✅ |
| Track freeze (save CPU) | L/F | ✅ |
| Plugin delay compensation | L/F | ⬜ |
| Spatial / Dolby Atmos mixing | L | ⬜ |

## 8. Effects

| Feature | Source | Isai |
|---|---|---|
| EQ (parametric, with analyzer) | L/F | ✅ (8 bands, cut slopes, live analyzer) |
| Compressor, multiband compressor | L/F | 🟡 (compressor; multiband later) |
| Limiter / maximizer | L/F | ✅ |
| Reverb (algorithmic) and convolution reverb | L/F | 🟡 (FDN algorithmic; convolution later) |
| Delay (tempo-synced) | L/F | ✅ |
| Chorus, flanger, phaser | L/F | ✅ |
| Distortion / saturation, bitcrusher | L/F | ✅ |
| Gate, de-esser | L/F | ✅ |
| Pitch correction (auto-tune style) | L/F | ✅ (Pitch Correct: key, scale, speed) |
| Vocal transformer / vocoder | L/F | ⬜ |
| Guitar and bass amp simulators, pedalboard | L | ⬜ |
| Gross-Beat style time and volume effects | F | ⬜ |
| Spectrum analyzer, tuner, level meter plugins | L/F | 🟡 (analyzer in the EQ, meters in every editor) |
| Mastering assistant | L/F | ✅ (LUFS targets, EQ + compressor + limiter) |

## 9. Automation

| Feature | Source | Isai |
|---|---|---|
| Volume and pan automation lanes | L/F | ✅ |
| Automate any effect or instrument control | L/F | 🟡 (effect knobs and sends; instruments later) |
| Draw, record (latch/touch) and edit automation | L/F | 🟡 (draw and edit; recording later) |
| Automation clips that move with patterns | F | ⬜ |

## 10. Audio processing

| Feature | Source | Isai |
|---|---|---|
| Time-stretch audio to tempo ("Follow tempo") | L/F | ✅ (WSOLA, per region) |
| Flex Time (move beats inside audio) | L | ⬜ |
| Flex Pitch / Newtone (edit vocal notes) | L/F | ⬜ |
| Stem splitter (vocals, drums, bass, other) | L/F | ✅ (Spleeter 4-stem, MIT; model downloaded once from Deezer) |
| Beat slicing | L/F | ⬜ |
| Normalize, reverse, fade in the audio editor | L/F | ⬜ |

## 11. Plugins & hardware

| Feature | Source | Isai |
|---|---|---|
| Third-party plugins: CLAP, VST3, AU | L/F | ✅ (instruments and effects, their windows, saved in songs; automation and latency compensation later) |
| Plugin manager / scanner | L/F | 🟡 (scan in a separate process; rescan from the menus) |
| MIDI controller learn (map knobs to controls) | L/F | ⬜ |
| Control surface support | L/F | ⬜ |
| Remote control from a phone or tablet | L/F | ⬜ |

## 12. Sounds & browser

| Feature | Source | Isai |
|---|---|---|
| Browser for files, samples, presets | L/F | ✅ (places, favourites, recursive search, drag to tracks) |
| Loop and sample library (free, CC0) | L/F | ⬜ |
| Preview sounds in time with the song | L/F | ✅ (starts on the next bar) |
| Preset/patch system for instruments and effects | L/F | 🟡 (built-in + My presets for piano, synth; EQ presets) |

## 13. Export & sharing

| Feature | Source | Isai |
|---|---|---|
| Export (bounce) the mix to WAV / FLAC | L/F | ✅ |
| MP3 / AAC export | L/F | ⬜ |
| Export stems (each track as a file) | L/F | ✅ |
| Export the cycle region only | L/F | ✅ |
| Loudness normalization on export | L/F | 🟡 (peak normalize to −1 dB; LUFS later) |

## 14. Interface & workflow

| Feature | Source | Isai |
|---|---|---|
| Light/dark themes, styles, accent colours | Isai | ✅ (Studio in the Meadow palette, Midnight, Vivid; painted start screen) |
| Simple/Pro views | L | ✅ |
| Search all commands (⌘K) | Isai | ✅ |
| Keyboard shortcuts | L/F | ✅ |
| Shortcut list and custom key commands | L/F | 🟡 (shortcut list with ?; custom keys later) |
| UI scale (bigger/smaller) | L/F | ✅ (75–200%, Settings › Appearance) |
| Detachable windows / multiple monitors | F | ⬜ |
| Inspector panel (details of the selected track/region) | L | 🟡 (track: instrument, effects, fader, pan, mute/solo; I) |
| Quick help / tooltips everywhere | L | 🟡 |
| Video track (score to picture) | L/F | ⬜ |
| Touch and pen support | F | ⬜ |
| Accessibility (screen readers, full keyboard use) | L | 🟡 |

## 15. Performance (Isai's low-spec focus)

| Feature | Source | Isai |
|---|---|---|
| Hardware check and performance profiles | Isai | ✅ |
| Adjustable buffer, animations, frame rate | L/F | ✅ |
| Only draw what's on screen | Isai | ✅ |
| Crackle detection and suggestions | Isai | ✅ |
| Track freeze, eco modes, disk streaming | L/F | ⬜ |
| Multi-core audio processing | L/F | ⬜ |
