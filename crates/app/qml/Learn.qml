pragma Singleton
import QtQuick

// Plain-English lessons for every effect, instrument, window and control, shown
// by the "?" buttons and the hover tips. Written for people who have never made
// music before. Each topic: a title, what it does, how it works (steps), a tip.
QtObject {
    readonly property var topics: ({
        // ------------------------------------------------------------ effects
        eq: {
            title: "Equalizer (EQ)",
            what: "Makes some pitches louder and some quieter. It is like the bass and treble knobs on a stereo, but with many more controls.",
            how: ["Each dot on the graph is a band: one pitch range you can change.",
                  "Drag a dot up to boost that pitch, down to cut it.",
                  "Drag left or right to choose which pitch the band works on.",
                  "Scroll on a band to make it wide (many pitches) or narrow (one pitch)."],
            tip: "Cut the muddy low middle (about 200–400 Hz) to clear space in a mix. Boost a little around 3–5 kHz to make vocals clearer.",
            example: "A boomy guitar sounds clearer after you cut 250 Hz by a few dB."
        },
        compressor: {
            title: "Compressor",
            what: "Turns down the loudest parts so the quiet and loud parts stay at a similar level. It makes a sound feel steady and punchy.",
            how: ["Threshold: the level where it starts to work. Lower means more of the song is squashed.",
                  "Ratio: how hard it squashes. 4:1 means for every 4 dB over the threshold, only 1 dB comes out.",
                  "Attack: how fast it reacts. Fast catches the first hit; slow lets the punch through.",
                  "Release: how long it takes to let go again.",
                  "Makeup gain: turns the whole sound back up after squashing."],
            tip: "Start with a ratio of 3:1 and turn the threshold down until the needle moves about 3–6 dB.",
            example: "Vocals that jump in and out of the mix sit still after a compressor."
        },
        limiter: {
            title: "Limiter",
            what: "A very strict compressor that stops the sound going above a set loudest level. It stops harsh clipping, and makes a song louder without distortion.",
            how: ["Ceiling: the loudest level allowed. Usually set just below 0 dB.",
                  "Gain: pushes the sound into the ceiling. More gain means louder but more squashed.",
                  "The meter shows how much it is holding the sound back."],
            tip: "Put a limiter last on the master track, with the ceiling at -1 dB, to avoid clipping on streaming services.",
            example: "A quiet mix pushed to a loud level without the peaks crackling."
        },
        reverb: {
            title: "Reverb",
            what: "Adds the sound of a room or a hall. It makes a sound feel like it is in a space, not dry and close.",
            how: ["Size or decay: how big the room is. Longer decay means a bigger hall.",
                  "Mix or wet: how much room sound you hear next to the original.",
                  "Pre-delay: a short gap before the room sound starts, which keeps the sound clear.",
                  "Damping: makes the tail duller, like a room with soft furniture."],
            tip: "Put reverb on a bus and send several tracks to it, so the whole song sounds like it is in one room.",
            example: "A dry vocal with a little reverb sounds like it is on a stage."
        },
        delay: {
            title: "Delay",
            what: "Repeats the sound after a short gap, like an echo in a canyon.",
            how: ["Time: the gap between repeats. Sync it to the tempo for a neat rhythm.",
                  "Feedback: how many repeats you hear before they fade.",
                  "Mix: how loud the echoes are next to the original sound."],
            tip: "A dotted eighth note delay (3/16 of a bar) gives a classic bouncy echo.",
            example: "One short echo on a guitar note makes it feel bigger."
        },
        chorus: {
            title: "Chorus",
            what: "Copies the sound and slightly changes its pitch and timing, so it sounds like several players playing together.",
            how: ["Rate: how fast the wobble is.",
                  "Depth: how far the pitch moves.",
                  "Mix: how much of the copy you hear."],
            tip: "Use a low depth for a subtle shimmer on pads and electric pianos.",
            example: "A plain electric guitar sounds lush and wide after a chorus."
        },
        flanger: {
            title: "Flanger",
            what: "Like chorus but faster and more sweeping. It makes a whooshing jet-plane sound.",
            how: ["Rate: how fast the sweep moves.",
                  "Depth: how strong the sweep is.",
                  "Feedback: how much the sweep repeats, making it more metallic."],
            tip: "Slow the rate right down and turn feedback up for a slow, dramatic sweep.",
            example: "A drum loop with a sweeping whoosh on the cymbals."
        },
        phaser: {
            title: "Phaser",
            what: "Sweeps a set of notches through the sound, for a soft swirling effect.",
            how: ["Rate: how fast the swirl moves.",
                  "Depth: how far the notches sweep.",
                  "Feedback: makes the swirl stronger and more obvious."],
            tip: "Phasers suit keys and pads. Keep depth low on drums.",
            example: "An electric piano that slowly swirls."
        },
        distortion: {
            title: "Distortion",
            what: "Pushes the sound too hard, so it crunches and grows more harmonics. It makes sounds gritty, warm or aggressive.",
            how: ["Drive: how hard the sound is pushed. More drive means more crunch.",
                  "Tone: makes the sound darker or brighter after the crunch.",
                  "Mix or level: balances the gritty sound with the clean one."],
            tip: "A little drive makes a sound warmer. Lots of drive makes it fuzzy.",
            example: "A clean bass with a touch of drive stands out on small speakers."
        },
        bitcrusher: {
            title: "Bitcrusher",
            what: "Makes the sound rougher and lo-fi by cutting its detail, like an old video game or a cheap sampler.",
            how: ["Bits: how many steps the volume has. Fewer bits means rougher.",
                  "Rate: how often the sound is sampled. Lower means more digital grit.",
                  "Mix: how much rough sound you hear."],
            tip: "Use it on one drum or a synth for a retro sound.",
            example: "A hi-hat that sounds like an old computer."
        },
        gate: {
            title: "Noise gate",
            what: "Mutes the sound when it is quiet, so background hiss and room noise disappear between notes.",
            how: ["Threshold: the level below which the sound is muted.",
                  "Attack and release: how fast the gate opens and closes.",
                  "The gate opens when the sound is louder than the threshold."],
            tip: "Set the threshold just above the hiss, so it only closes between phrases.",
            example: "A microphone recording with no hum when the singer is quiet."
        },
        deesser: {
            title: "De-esser",
            what: "Turns down the harsh 'sss' and 'shh' sounds in vocals.",
            how: ["Frequency: the pitch of the harsh sound (usually 4–8 kHz).",
                  "Amount: how much to turn it down.",
                  "It only works when the harsh sound happens, so the rest of the voice stays natural."],
            tip: "Listen with the amount high, then back it off until the 's' sounds feel natural.",
            example: "A vocal with 'sss' sounds that hurt your ears, now softer."
        },
        pitch: {
            title: "Pitch correct",
            what: "Moves sung or played notes to the nearest note in a key, so slightly out-of-tune notes sound in tune.",
            how: ["Key and scale: the notes the song is allowed to use.",
                  "Speed: how fast the correction happens. Fast sounds robotic, slow sounds natural.",
                  "It only changes notes that are close to a scale note."],
            tip: "Set the key first, then a slow speed for natural-sounding vocals.",
            example: "A slightly flat vocal line sits right on the note."
        },
        plugin: {
            title: "Plugin",
            what: "An effect or instrument made by another company, loaded into the app.",
            how: ["Click Open to see its own window with its own controls.",
                  "Its settings are saved with the song.",
                  "If a plugin will not open, it may not be installed or may not support this system."],
            tip: "Plugins have their own controls. Read their manual for what each one does.",
            example: "A piano from another company, played inside Isai."
        },

        // -------------------------------------------------------- instruments
        synth: {
            title: "Synth",
            what: "An instrument that makes sound electronically. You shape the sound with waves, filters and envelopes.",
            how: ["Oscillators make the basic wave. Several oscillators make a thicker sound.",
                  "The filter cuts off some pitches, so the sound can be dark or bright.",
                  "The envelope (ADSR) shapes how a note starts, holds, fades and ends.",
                  "The LFO moves a setting up and down over time, like a slow wobble."],
            tip: "Press a key while turning the filter cutoff to hear how it changes the sound.",
            example: "A soft pad that slowly fades in, made with a slow attack."
        },
        piano: {
            title: "Piano",
            what: "A built-in piano and other keyboard sounds, such as electric piano and organ.",
            how: ["Pick a sound from the tone list.",
                  "Hit keys harder for a louder, brighter note.",
                  "Sustain holds notes after you let go, like the pedal on a real piano."],
            tip: "Use the computer keyboard: A W S E D F and so on play notes when the keyboard mode is on.",
            example: "A soft electric piano chord under a vocal."
        },
        drums: {
            title: "Drum machine",
            what: "A set of drum sounds on a grid. Each row is one drum, and each box is one step in the bar.",
            how: ["Click a box to add a hit. Click again to remove it.",
                  "Each row is one sound: kick, snare, hi-hat and so on.",
                  "Pattern presets give you a starting groove.",
                  "Swing moves every other step slightly late, for a bouncy feel."],
            tip: "Put the kick on beats 1 and 3 and the snare on beats 2 and 4. That is a basic beat.",
            example: "A four-on-the-floor kick with hi-hats on every eighth note."
        },
        sampler: {
            title: "Sampler",
            what: "Plays a recorded sound on the keys. Each key plays the sound at a different pitch.",
            how: ["Load a sound, such as a voice or a drum loop.",
                  "Slices cut the sound into pieces, one per key, so you can play it like a drum kit.",
                  "The root note is the key where the sound plays at its original pitch."],
            tip: "Chop a loop into slices, then play the slices on the keys to make a new rhythm.",
            example: "A vocal chopped into small pieces you play like a keyboard."
        },

        // ------------------------------------------------------------ windows
        mixer: {
            title: "Mixer",
            what: "Shows one channel for each track, like a row of sliders on a mixing desk. It is where you set how loud, how far left or right, and which effects each track has.",
            how: ["Each strip is one track.",
                  "The fader sets how loud the track is.",
                  "Pan moves the track left or right.",
                  "Mute silences it. Solo plays only the soloed tracks.",
                  "Inserts are effects in order from top to bottom. Sends feed a shared bus."],
            tip: "Set each track's loudness first with the faders, then add effects.",
            example: "Drums in the centre and loud, guitar to the left and a little quieter."
        },
        pianoroll: {
            title: "Piano roll",
            what: "A grid for writing notes. Time runs left to right, and pitch runs up and down, like a piano keyboard turned sideways.",
            how: ["Click an empty box to draw a note.",
                  "Drag a note to move it. Drag its right edge to make it longer.",
                  "Right-drag to erase notes.",
                  "Quantize snaps notes to the beat so they line up neatly.",
                  "Scale highlighting shows which notes fit the song's key."],
            tip: "Draw a simple melody with notes from one scale. If a note sounds wrong, it is probably not in the key.",
            example: "A four-note riff that repeats every bar."
        },
        playlist: {
            title: "Arrangement",
            what: "The timeline of the whole song. Each track is a row, and each piece of sound or notes is a region placed along it.",
            how: ["Drag a region to move it in time or to another track.",
                  "Click the scissors to split a region.",
                  "Drag the edges of a region to trim it.",
                  "The ruler at the top shows bars and beats."],
            tip: "Build a song section by section. Copy a region with Cmd+D to repeat it.",
            example: "A chorus region copied to the end of the song."
        },
        inspector: {
            title: "Inspector",
            what: "Shows the selected track's instrument, effects and fader, so you can change them in one place.",
            how: ["Pick a track in the arrangement to see its details here.",
                  "Click Open to see the instrument's own window.",
                  "Effects here run in order from top to bottom."],
            tip: "Hide the inspector with the close button to get more space for the timeline.",
            example: "Selecting a bass track shows its synth and its effects."
        },

        // ------------------------------------------------------------ controls
        fader: {
            title: "Fader (volume)",
            what: "Sets how loud a track is. Higher is louder.",
            how: ["Drag the fader up to make the track louder, down to make it quieter.",
                  "The number shows the level in decibels (dB). 0 dB is the normal level.",
                  "Keep the master below 0 dB to avoid clipping."],
            tip: "Start with every track around -6 dB, then adjust.",
            example: "A quiet vocal raised from -12 dB to -6 dB."
        },
        pan: {
            title: "Pan",
            what: "Moves a sound between the left and right speakers.",
            how: ["Turn the knob left to send the sound left.",
                  "Turn it right to send it right.",
                  "Centre keeps it in the middle, where most lead sounds sit."],
            tip: "Pan drums and guitars slightly apart so they do not fight in the middle.",
            example: "Two guitars, one panned left and one right."
        },
        mute: {
            title: "Mute",
            what: "Silences a track without deleting it.",
            how: ["Click M to mute the track. Click again to hear it.",
                  "A muted track still exists, so you can bring it back."],
            tip: "Mute tracks one at a time to find the one that is too loud.",
            example: "Muting the drums to hear the bass line alone."
        },
        solo: {
            title: "Solo",
            what: "Plays only the soloed tracks and silences the rest. Useful for hearing one part on its own.",
            how: ["Click S to solo a track.",
                  "Solo more tracks to hear them together.",
                  "Click S again to turn it off."],
            tip: "Solo a track to check it for noise or mistakes, then turn solo off before you play the whole song.",
            example: "Soloing the vocal to listen for breaths."
        },
        send: {
            title: "Send",
            what: "Sends part of a track to a shared effect, such as one reverb used by the whole song.",
            how: ["A bus is the shared effect track.",
                  "The send knob sets how much of this track goes to the bus.",
                  "Turning the send up adds more of the shared effect to this track."],
            tip: "Use one reverb bus for the whole song. It sounds more even than a separate reverb on each track.",
            example: "Every track sends a little to one shared room reverb."
        },
        arm: {
            title: "Record arm",
            what: "Gets a track ready to record. Only armed tracks record when you press Record.",
            how: ["Click the red dot on a track to arm it.",
                  "Press Record in the top bar, or R on the keyboard.",
                  "Your input is heard through the speakers while the track is armed."],
            tip: "Plug in your microphone or keyboard first, then arm the track.",
            example: "Arming a voice track before singing."
        },
        metronome: {
            title: "Metronome",
            what: "A click that plays on every beat, to help you keep time while you play or record.",
            how: ["The loud click is the first beat of the bar.",
                  "Turn it on or off with K.",
                  "It only helps you play. It is not recorded into the song."],
            tip: "Use the metronome while you record your first take, then turn it off to listen.",
            example: "Clicks on each beat while you play a drum pattern."
        },
        countin: {
            title: "Count-in",
            what: "Plays a bar of clicks before recording starts, so you know when to begin.",
            how: ["Turn count-in on from the top bar, or with the 1234 button.",
                  "The number of bars is in Settings.",
                  "The playhead waits, then recording starts on the first beat."],
            tip: "Use one bar of count-in for quick ideas, two bars for a careful start.",
            example: "Hearing four clicks, then playing your first note on the one."
        },
        cycle: {
            title: "Cycle (loop)",
            what: "Repeats one section of the song over and over while you work on it.",
            how: ["Press C to turn the cycle on.",
                  "The cycle covers the part of the song between the two markers.",
                  "Recordings in a cycle are kept as separate takes."],
            tip: "Cycle a short section, like a single bar, while you tweak a sound.",
            example: "Looping bars 5 to 8 to fix a drum fill."
        },
        snap: {
            title: "Snap to grid",
            what: "Makes moved regions and notes line up with the beat, so they do not land between beats by accident.",
            how: ["Press N to turn snap on or off.",
                  "The grid size (bar, beat, half beat) sets how small each step is.",
                  "Hold a key while dragging to ignore snap for a moment."],
            tip: "Leave snap on for most edits. Turn it off only when you want a precise, free position.",
            example: "A drum hit snapped exactly to beat 2."
        },
        tempo: {
            title: "Tempo (BPM)",
            what: "How fast the song goes, in beats per minute. 120 BPM means 120 beats each minute.",
            how: ["Click the tempo number and type a new value.",
                  "Tap tempo: tap T in time with the music, and the app works out the BPM.",
                  "Tempo changes along the song are set in the tempo lane (G)."],
            tip: "Most pop songs are 100–130 BPM. Slower songs suit ballads, faster songs suit dance.",
            example: "Setting 90 BPM for a slow, relaxed groove."
        },
        key: {
            title: "Key and scale",
            what: "The home note and the set of notes the song mostly uses. Notes outside the scale can sound wrong.",
            how: ["The key is the home note, such as C or A.",
                  "A scale is the pattern of notes from that home note: major sounds bright, minor sounds sad.",
                  "The piano roll highlights the notes that fit the key."],
            tip: "If you are not sure, use C major or A minor. Their notes are the white keys on a piano.",
            example: "A song in A minor uses only the white keys around A."
        },
        signature: {
            title: "Time signature",
            what: "How many beats are in each bar, and which note gets one beat. 4/4 is the most common.",
            how: ["The top number is how many beats are in a bar.",
                  "The bottom number is which note value counts as one beat (4 means a quarter note).",
                  "Change it along the song in the tempo lane (G)."],
            tip: "4/4 is the easiest to start with. 3/4 feels like a waltz.",
            example: "A waltz in 3/4: one, two, three."
        },
        latency: {
            title: "Latency",
            what: "The short delay between playing a sound and hearing it. Lower feels more responsive.",
            how: ["Buffer size: a smaller buffer means less delay but more risk of crackling.",
                  "Latency correction moves recordings earlier, so they line up with the beat."],
            tip: "If your recordings sound late, increase the correction in Settings.",
            example: "A recording that sounds a little behind the click, fixed by correction."
        },
        bounce: {
            title: "Bounce (export)",
            what: "Turns the song into one finished audio file you can share or play anywhere.",
            how: ["Choose WAV for the best quality, or FLAC for smaller files with the same quality.",
                  "You can export the whole song, the cycle, or each track as a stem."],
            tip: "Export the whole song once, then listen to the file on a phone to check it.",
            example: "A WAV file of the finished song to send to a friend."
        },
        freeze: {
            title: "Freeze",
            what: "Turns a track's instrument and effects into a simple audio file, which saves computer power.",
            how: ["Freeze the track when it is finished.",
                  "A frozen track still plays, and you can change its volume and pan.",
                  "Unfreeze to edit its notes or effects again."],
            tip: "Freeze heavy synth or plugin tracks when your computer starts to struggle.",
            example: "A huge synth pad frozen to save power while you record vocals."
        },
        markers: {
            title: "Markers",
            what: "Names for sections of the song, like Intro, Verse and Chorus, so you can jump between them quickly.",
            how: ["Press M to add a marker at the playhead.",
                  "Alt+Right and Alt+Left jump to the next or last marker.",
                  "Double-click a marker to rename it."],
            tip: "Add markers before you start arranging, so you always know where each section is.",
            example: "Markers at bar 1 (Intro), bar 9 (Verse) and bar 17 (Chorus)."
        }
    })

    /// Plain meanings of words the lessons use. A lesson lists the ones it contains.
    readonly property var terms: ({
        track: "One line of sound in your song. Each track has its own volume, pan and effects.",
        region: "A piece of sound or notes placed on a track.",
        beat: "One pulse of the music, the steady tick you tap your foot to.",
        bar: "A block of beats. In 4/4, a bar has four beats.",
        BPM: "Beats per minute. The tempo: how fast the song goes.",
        Hz: "Hertz: vibrations per second. Higher Hz means a higher pitch. 1000 Hz is written 1 kHz.",
        dB: "Decibel: the unit of loudness. 0 dB is normal, -6 dB is about half as loud, +6 dB is about twice as loud.",
        fader: "The slider that sets how loud a track is.",
        pan: "Where a sound sits, left or right in the speakers.",
        mute: "Silences a track.",
        solo: "Plays only the soloed tracks.",
        bus: "A shared track that several tracks send their sound to, like one reverb for the whole song.",
        send: "How much of a track is sent to a bus.",
        insert: "An effect slot on a track. Effects run in order, from top to bottom.",
        threshold: "The level where an effect starts to work.",
        ratio: "How hard a compressor squashes. 4:1 means 4 dB above the threshold comes out as 1 dB.",
        attack: "How fast an effect reacts when a sound starts.",
        release: "How fast an effect lets go after a sound stops.",
        decay: "How long a sound takes to fade away.",
        feedback: "How much of an echo or sweep feeds back into itself, so it repeats more.",
        drive: "How hard a sound is pushed in distortion. More drive means more crunch.",
        mix: "How much of the effect you hear next to the original sound.",
        wet: "The sound with the effect on. Dry is the original, with no effect.",
        clipping: "Harsh crackling that happens when a sound is too loud for the system to handle.",
        ADSR: "Attack, Decay, Sustain, Release: how a note starts, settles, holds and fades away.",
        LFO: "A slow wave that moves a setting up and down over time, like a wobble.",
        MIDI: "Notes written as data: which key, when it starts, how long, and how hard. An instrument makes the sound.",
        velocity: "How hard a note is played. Harder usually means louder.",
        quantize: "Snaps notes to the beat so they line up neatly.",
        scale: "A set of notes that fit together, like the white keys on a piano.",
        key: "The home note of a song, such as C or A. Notes that fit the key sound right together.",
        loop: "A short piece of music that repeats.",
        sample: "A short recorded sound that you reuse in a song.",
        stem: "One part of a finished song on its own, such as vocals or drums.",
        sync: "Matching an effect to the song's tempo, so it repeats in time with the beat.",
        latency: "The short delay between playing a sound and hearing it."
    })

    /// The lesson list, grouped the way a beginner looks for things.
    readonly property var groups: [
        { title: "Effects", ids: ["eq", "compressor", "limiter", "reverb", "delay", "chorus", "flanger", "phaser",
                                  "distortion", "bitcrusher", "gate", "deesser", "pitch", "plugin"] },
        { title: "Instruments", ids: ["synth", "piano", "drums", "sampler"] },
        { title: "Windows", ids: ["playlist", "mixer", "pianoroll", "inspector"] },
        { title: "Mixing controls", ids: ["fader", "pan", "mute", "solo", "send", "arm"] },
        { title: "Transport and timing", ids: ["metronome", "countin", "cycle", "snap", "tempo", "key", "signature", "latency"] },
        { title: "Working with songs", ids: ["markers", "freeze", "bounce"] }
    ]

    /// The lesson for a topic id, or null when there is none.
    function get(id) {
        return topics[id] || null
    }
}
