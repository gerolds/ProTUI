// ADR: A `Layer` is one long-running background slot (ambience, music): the same cue keeps playing, another cue crossfades, `None` fades out — and a voice lost to a reload or stop restarts on the next request.
use crate::{AudioError, CueId, Engine, PlayOutcome, Playback, VoiceId};
use kira::backend::Backend;
use std::time::Duration;

/// One background slot. Call `set` with the wanted cue whenever it may have changed (every
/// frame is fine: an unchanged request does no work beyond a voice lookup).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Layer {
    current: Option<(CueId, VoiceId)>,
}

impl Layer {
    pub fn new() -> Self {
        Self::default()
    }

    /// The cue and voice that play in this slot, if any.
    pub fn current(&self) -> Option<(CueId, VoiceId)> {
        self.current
    }

    /// Requests `cue` (a looping cue) for this slot at `now`:
    /// - the same cue while its voice is active: keeps playing, returns `None`;
    /// - another cue while a voice is active: crossfades over `fade` (the old voice stays if
    ///   the new one is dropped); returns the outcome;
    /// - no active voice (first request, or the voice ended through `stop_all`/`replace`):
    ///   fades the cue in over `fade`; returns the outcome;
    /// - `None`: fades the active voice out over `fade`, returns `None`.
    pub fn set<B: Backend>(
        &mut self,
        engine: &mut Engine<B>,
        cue: Option<CueId>,
        fade: Duration,
        now: Duration,
    ) -> Result<Option<PlayOutcome>, AudioError> {
        engine.update(now)?;
        let active = self.current.filter(|(_, voice)| engine.is_active(*voice));
        let Some(cue) = cue else {
            if let Some((_, voice)) = active {
                engine.stop(voice, fade, now)?;
            }
            self.current = None;
            return Ok(None);
        };
        match engine.bank().options(cue).map(|o| o.playback) {
            None => return Err(AudioError::UnknownCue(cue)),
            Some(Playback::Once) => {
                return Err(AudioError::InvalidConfig(
                    "a background layer cue must loop",
                ));
            }
            Some(Playback::Loop { .. }) => {}
        }
        let outcome = match active {
            Some((playing, _)) if playing == cue => return Ok(None),
            Some((_, voice)) => engine.crossfade(voice, cue, fade, now)?,
            None => {
                self.current = None;
                engine.play_with_fade(cue, now, Some(fade))?
            }
        };
        if let Some(voice) = outcome.voice() {
            self.current = Some((cue, voice));
        }
        Ok(Some(outcome))
    }
}
