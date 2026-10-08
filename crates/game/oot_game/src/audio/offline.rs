//! Not in the C: the audio side of a headless run. After each game frame the play state's
//! `GameOp`s go to an offline renderer (`eng_audio::Renderer`), which runs the frame's VI
//! retraces (`R_UPDATE_RATE`: 3 per 20 Hz frame, 2 in the pause menu) and gives back what the
//! game reads next
//! (docs/adr/0026-the-games-audio.md). What the AI played is the run's sound.

use eng_audio::{AudioData, Renderer};

use super::{AudioSide, GameAudio};
use crate::play::PlayState;

pub struct OfflineAudio {
    pub renderer: Renderer,
    /// Keep what the AI plays (`renderer.out`); off, it's dropped every frame.
    pub keep_output: bool,
}

impl AudioSide for OfflineAudio {
    fn hand_over(&mut self, audio: &mut GameAudio) {
        let ops = audio.take_ops();
        let view = self.renderer.game_frame(&ops, audio.update_rate as u64);
        audio.set_view(view);
        if !self.keep_output {
            self.renderer.out.clear();
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl OfflineAudio {
    /// The audio side, booted (`Audio_Init`: `AudioLoad_Init` on spec 0).
    pub fn new(data: &AudioData, keep_output: bool) -> OfflineAudio {
        OfflineAudio { renderer: Renderer::new(data), keep_output }
    }

    /// After a game frame: its ops to the audio side, the frame's retraces, the view back.
    pub fn frame(&mut self, play: &mut PlayState) {
        self.hand_over(&mut play.audio);
    }

    /// `n` game frames with no input, each handed over.
    pub fn run_idle(&mut self, play: &mut PlayState, n: u32) {
        for _ in 0..n {
            play.tick_with(crate::play::scripted_input(Default::default(), Default::default()));
            self.frame(play);
        }
    }

    /// The sequence player `player` plays, if it's on: its `seqId`.
    pub fn playing(&self, player: usize) -> Option<u8> {
        let sp = &self.renderer.ctx.seq_players[player];
        sp.enabled.then_some(sp.seq_id)
    }
}
