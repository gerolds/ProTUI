# Sound and music

Read this before you add sound effects, ambience or music, or change how the game opens an audio device. The audio
crate's API is in [crates/terminal-audio/README.md](../../crates/terminal-audio/README.md). This page holds findings
from earlier probes on this machine and in a sibling project (Runner). Mixing, loudness, music and what the host
shows without a device are reference findings, not this game's decisions; the design in `design/` decides what the game
plays. Audio-thread, decoding, timing and manifest-validation lines are engineering rules.

## Library

- `kira` 0.12 on its `cpal` backend plays everything. It has tracks (buses) with volume per track, effects on
  tracks, tweens for fades, a clock, and a custom `Sound` trait. With no output device,
  `AudioManager::<DefaultBackend>::new` returns an error instead of crashing. It recovers when the device changes.
- Compared: `rodio` lacks groups and fades and printed device errors to the terminal, which breaks a full-screen
  view. An own mixer on `cpal` was estimated at 400–800 lines.
- The workspace pins kira with default features off and `cpal` on. That drops `symphonia` (MPL-2.0); WAV is decoded
  with `hound` and handed to kira as frames. About 38 crates were measured for kira on Windows with only WAV.
- `kira::backend::Renderer` mixes without a device. An offline render through it is the same mix the device plays.

## Mixing

- One track per bus (for example effects, ambience, interface, music), each with its own volume.
- A cap on voices playing at once; a play beyond it is dropped. Per sound: a cap on copies with a steal rule
  (oldest stops, or the new play is dropped), a cooldown, small random volume and pitch changes per play.
- Ducking: while a sound marked to duck plays, a named bus dips by a set number of decibels with attack and release
  times.
- A soft limiter on the main output keeps loud moments from clipping.
- Keep the play rules (cooldown, caps, steal, ducking) in plain code on a clock passed in. Tests then run them on a
  fake clock, and a sound log shows the same choices with or without a device.

## No device

- No output device must never stop the game; the host continues silent. Device errors never go to the terminal's
  error output, which would break the full-screen view. The sibling project showed why in a status line or the
  problem strip; where to show it is a `design/` choice.
- A sound log (`--log-sounds <file>` or similar) writes one line per request: time, name, file, or why it was
  dropped, and whether a device was found. Agents check timing and choices there.

## Timing

- Sound waits for the frame that shows its event. Queue each request with the time of the thing it belongs to and
  release it when the drawn timeline reaches that time. In the sibling project a sound before its picture read as wrong.
- Decoding, file reads and bank building happen off the frame path, at start or on reload, never per frame.

## Manifest

- Code and content name a sound by a stable name (`group/name`, lowercase letters, digits, `-`), never by a file.
- A manifest file lists every name with its bus, files and play settings. Each entry's comment says what the sound
  is like and how long it is, so a person, an agent or a tool can make the file.
- Strict TOML ([content.md](content.md)): unknown and missing keys are refused. Start-up refuses a manifest that
  lacks a name the code uses, and lists the names.
- Files: 16- or 24-bit PCM or 32-bit float WAV at 22.05, 44.1 or 48 kHz, mono or stereo. A file that does not read is
  an error with its path; never a silent substitute.
- Sample data in this repository is neutral: a sine tone named `ui/click` is enough for a test.

## Making sounds

- Synthesis recipes in code give a starter set: one recipe per name, effects mono 44.1 kHz, ambience loops mono
  22.05 kHz of 12–16 s. A new name needs a recipe, or the tool stops with its name. Any file may later be replaced
  by hand.
- Agents cannot listen. Check every sound by an offline render plus numbers: peak and RMS in dBFS, count of
  non-finite samples, length, and the sound log. Report what you measured, never how it sounds.
- Reference loudness from a sibling-project probe: sustained voices near -10 dBFS, percussive peaks near -6 dBFS at
  full velocity; music mixed at about -16 to -21 dBFS RMS with peaks below 0 dBFS. Targets are a `design/` choice.
- Listening is the user's check. Ask for it with the exact command that plays the sound.

## Music findings (reference)

- Unconstrained random notes sound generic. Trial pieces with chords chosen at random each bar and themes of a few
  notes were judged "generic ambience, not particularly memorable" by the user. Working systems limit note choice
  hard: Spore filters random notes into scales and modes, Rain World adds and removes fixed layers.
- Authored material with constrained variation worked better: written tunes, written chord progressions, forms such
  as A A B A whose first statement plays exactly as written, riffs that follow the chords, drum step patterns,
  swing. A later statement varies with a set probability (transpose, invert, ornament, octave). A random chord walk
  suits ambience only.
- Randomness works best as one axis from locked to free: at one end a loop repeats, in between it now and then
  changes a note. Every state stays reproducible from a seed plus the controls.
- In the probes a few strong controls (for example mood, tension, density, brightness, space, variation) worked
  better than many settings; switching a situation took effect at the next bar.
- For a creative tool, the lesson was to ask first what the user does with their hands. A probe built for agents to
  write pieces was the wrong tool; the user wanted to compose inside the app by play: mouse buttons with visible
  handles, keys as optional shortcuts.
- Write what plays as text: one line per bar (chord, controls, active layers) and one per note (time, layer, pitch,
  velocity, cause). Agents check their work in that log and in rendered WAV files.
- Options measured: `fundsp` 0.23 for synthesised voices (drone, pad, bell, pluck, strings, wind, noise);
  `rustysynth` 1.3.8 plays a SoundFont such as GeneralUser GS (about 30 MB, free to bundle, sounds like general
  MIDI). A probe rendered 20–60× real time with synth voices and 70–90× with the SoundFont; the music cost 2–5 % of
  one core. A debug build at opt-level 0 rendered at 0.6× real time; opt-level 2 for dev builds reached 22×.
- SoundFont voices sat 3–10 dB below synth voices until each layer got its own level from solo renders.
- A live custom kira `Sound` runs on the audio thread. It must never allocate, lock or block while playing. Parse,
  check and build a piece on another thread and swap it in lock-free; a broken file is refused with a reason and
  the old piece keeps playing. Remove non-finite samples and keep a limiter on the output.
- Sources: [kira Sound](https://docs.rs/kira/0.12.6/kira/sound/trait.Sound.html),
  [kira Renderer](https://docs.rs/kira/0.12.6/kira/backend/struct.Renderer.html),
  [fundsp](https://github.com/SamiPerttu/fundsp), [rustysynth](https://github.com/sinshu/rustysynth),
  [GeneralUser GS licence](https://github.com/mrbumpy409/GeneralUser-GS/blob/main/documentation/LICENSE.txt),
  [Rain World threat](https://rainworld.miraheze.org/wiki/Threat),
  [Laurie Spiegel interview](http://econtact.ca/10_2/SpiegelLa_KD.html).

## Sound generation findings (reference)

- The minimal synth kit in `terminal-audio` (naive oscillators, simple filters, comb reverb) sounded canned to a
  user in texture and instrument quality. Playback (kira) is not the limit; generation is. Pipelines worth comparing
  by ear: the current kit, fundsp DSP, SF2 samples (rustysynth), Surge XT driven from Python `pedalboard`, and
  physically modelled foley with CC0 transients. Build an A/B listening order in `scratch/` and let the user choose.
- Agent music tooling (discussed, not built): agents can already write scores in text formats they were trained on;
  what they lack is perception and a cheap human feedback loop. A minimal "DAW for agents" = a declarative text
  score with stable bar/part ids and seeded constrained variation; a renderer around an existing synth back end
  (by section and stem); an analyser that returns text (realised chords, density, repetition, loudness, spectral
  balance, masking between stems, harshness flags) plus piano-roll and spectrogram PNGs; optional learned "ears"
  (text-audio embedding similarity to approved references, an audio-capable model asked a specific question); and
  a feedback file of the user's timestamped comments mapped to score ids. The taste anchor (approved references and
  verdicts) is the core; without it agent music drifts generic. Build it only after a back end sounds good.

## Remote terminals

Native playback happens on the machine that runs the process. A remote terminal session does not carry sound to the
viewer. The proposed [Terminal Audio Protocol](https://github.com/kovidgoyal/kitty/discussions/9507) is not a
portable baseline.
