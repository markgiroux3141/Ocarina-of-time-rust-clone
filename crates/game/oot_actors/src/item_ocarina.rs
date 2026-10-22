//! `Item_Ocarina` (`ovl_Item_Ocarina/z_item_ocarina.c`): the Ocarina of Time, drawn as its
//! get-item model (`GetItem_Draw` of `GID_OCARINA_OF_TIME`) at scale 0.1, in four kinds:
//!
//! - **Type 0, thrown in Zelda's escape** (`gHyruleFieldZeldaEscapeCs`, scene layer 5: Impa's
//!   `En_Viewer` spawns it at frame 845, `z_en_viewer.c:378-383`): tossed up at 6 a frame with
//!   gravity -0.3 to -5 (`ItemOcarina_GetThrown`), spinning (0x800 a frame about x, 0xC00 about y,
//!   `ItemOcarina_Fly`); at frame 881 put at (250, 60, 1075) and thrown at (1, -5, -7); at frame
//!   897 into the moat: a ripple and a splash, the stop, gravity -0.1 to -0.5, the spin off,
//!   `NA_SE_EV_BOMB_DROP_WATER`; at frame 906 a second ripple at (274, -60, 907).
//! - **Type 1, thrown in the other cutscene** (Impa's `En_Viewer` outside layer 5, at frame 195):
//!   tossed at (0, 4, 6) with gravity -0.3 to -5 (`func_80B865E0`), spinning as type 0
//!   (`func_80B864EC`); at frame 220 put at (144, 80, 1686) and thrown at (1, 2, -7) with gravity
//!   -0.15.
//! - **Type 2:** nothing (`ItemOcarina_DoNothing`); nothing in the game places it.
//! - **Type 3, in the moat** (Hyrule Field's child layers place 0x0003 at (299, -136, 884)): gone
//!   at init until Zelda has fled (`EVENTCHKINF_ZELDA_FLED_CASTLE`) and once it's taken
//!   (`EVENTCHKINF_43`); otherwise at scale 0.2 with Navi's `Elf_Msg2` 0x3800 spawned just below it
//!   (299, -140, 884; rot (0, 4, 1)). It's offered as `GI_OCARINA_OF_TIME` within 30 across and 50
//!   up or down (`Actor_OfferGetItem`, Player's dive reaches it from the bottom), with a bubble
//!   (`EffectSsBubble_Spawn`, 10 across, scale 0.13) on every frame where `gameplayFrames & 13` is 0
//!   (`ItemOcarina_WaitInWater`). Taken: `EVENTCHKINF_43` and switch 3, no longer drawn, and when
//!   the get-item text closes (`Actor_TextboxIsClosing`) `gHyruleFieldZeldaSongOfTimeCs` starts
//!   (`ItemOcarina_StartSoTCutscene`): its terminator takes Link to the Temple of Time's Song of
//!   Time. The actor stays (not killed), and so does the check: the C sets the script and the
//!   trigger again on every frame the text is closing.
//!
//! The whole overlay is ported. Not drawn: the get-item model's highlights (`func_8002EBCC`,
//! `func_8002ED80`), as for `Item_Etcetera` and `En_Ex_Item`.
//!
//! Params: `type` (the whole params, `ItemOcarina_Init`'s switch): 0 thrown in Zelda's escape
//! (Impa's `En_Viewer`, layer 5), 1 thrown in the other cutscene (Impa's, outside layer 5), 2
//! nothing, 3 in the moat (Hyrule Field); any other value is killed at init.
//! Flags: type 3 reads EVENTCHKINF_ZELDA_FLED_CASTLE (not set: gone) and EVENTCHKINF_43 (set:
//! gone), and sets EVENTCHKINF_43 and switch 3 when taken.

use glam::Vec3;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::sfx::NA_SE_EV_BOMB_DROP_WATER;
use oot_game::event_chk_inf;
use oot_game::get_item::{actor_has_parent, offer_get_item_range};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::save::EVENTCHKINF_ZELDA_FLED_CASTLE;

use crate::elf_msg2::ACTOR_ELF_MSG2;

/// `ACTOR_ITEM_OCARINA` (`actor_table.h`: 0x00F1).
pub const ACTOR_ITEM_OCARINA: i16 = 0x00F1;

/// `Item_Ocarina_Profile`: `ACTORCAT_ITEMACTION`, `ACTOR_FLAG_UPDATE_CULLING_DISABLED`,
/// `OBJECT_GI_OCARINA`.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_ITEM_OCARINA, name: "Item_Ocarina", category: ACTORCAT_ITEMACTION, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: "object_gi_ocarina", schema: &SCHEMA };

/// The whole params: the kind (`ItemOcarina_Init`'s switch).
pub const TYPE: Field = Field::params(
    "params",
    0,
    16,
    Kind::Named(&[(0, "thrown in Zelda's escape"), (1, "thrown (the other cutscene)"), (2, "nothing"), (3, "in the moat")]),
    "0 thrown in Zelda's escape (Impa's En_Viewer), 1 thrown outside layer 5, 2 nothing, 3 in Hyrule Field's moat; others killed",
);

/// `EVENTCHKINF_43` (`save.h`: 0x43): the Ocarina of Time taken from the moat.
pub const EVENTCHKINF_43: u16 = 0x43;
/// The switch flag the ocarina taken sets (`Flags_SetSwitch(play, 3)`).
pub const OCARINA_SWITCH_FLAG: i32 = 3;
/// `gHyruleFieldZeldaSongOfTimeCs` (`spot00_scene.h`): started as the get-item text closes.
pub const SONG_OF_TIME_CS: &str = "gHyruleFieldZeldaSongOfTimeCs";

// `GetItemID` and `GetItemDrawID` (`item.h`).
/// `GI_OCARINA_OF_TIME` (0x0C).
pub const GI_OCARINA_OF_TIME: i16 = 0x0C;
/// `GID_OCARINA_OF_TIME` (0x2E).
pub const GID_OCARINA_OF_TIME: i16 = 0x2E;

/// Item_Ocarina's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_ZELDA_FLED_CASTLE)).reads().note("type 3: not set (Zelda hasn't fled), gone at init"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_43)).reads().sets().note("type 3: the ocarina taken; set, gone at init"),
        FlagUse::new(Flag::Switch("3")).sets().note("type 3: the ocarina taken"),
    ],
    todo: false,
    // OBJECT_GI_OCARINA is the profile's.
    objects: &[],
    texts: &[],
};

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `ItemOcarina_GetThrown` (type 0).
    GetThrown,
    /// `ItemOcarina_Fly` (type 0).
    Fly,
    /// `func_80B865E0` (type 1).
    Throw1,
    /// `func_80B864EC` (type 1).
    Fly1,
    /// `ItemOcarina_DoNothing` (type 2).
    DoNothing,
    /// `ItemOcarina_WaitInWater` (type 3).
    WaitInWater,
    /// `ItemOcarina_StartSoTCutscene` (type 3, taken).
    StartSoTCutscene,
}

pub struct ItemOcarina {
    pub actor: Actor,
    pub action: Action,
    /// `spinRotOffset`.
    pub spin_rot_offset: i16,
    /// `actor.draw`: cleared once taken.
    pub drawn: bool,
}

impl ItemOcarina {
    /// `ItemOcarina_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let params = TYPE.get(actor.params);
        let mut this = ItemOcarina { actor, action: Action::DoNothing, spin_rot_offset: 0, drawn: true };
        // ActorShape_Init(&shape, 0, NULL, 0): no shadow.
        this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::None, 0.0);
        this.actor.scale = Vec3::splat(0.1);
        match params {
            0 => this.action = Action::GetThrown,
            1 => this.action = Action::Throw1,
            2 => this.action = Action::DoNothing,
            3 => {
                this.action = Action::WaitInWater;
                if !play.save.get_event_chk_inf(EVENTCHKINF_ZELDA_FLED_CASTLE) || play.save.get_event_chk_inf(EVENTCHKINF_43) {
                    this.actor.kill();
                    return Box::new(this);
                }
                if let Err(e) = play.actor_spawn(ACTOR_ELF_MSG2, Vec3::new(299.0, -140.0, 884.0), [0, 4, 1], 0x3800) {
                    log::debug!("Item_Ocarina: Elf_Msg2: {e:?}");
                }
                this.actor.scale = Vec3::splat(0.2);
            }
            _ => {
                this.actor.kill();
                return Box::new(this);
            }
        }
        log::debug!("no = {params}");
        this.spin_rot_offset = 0x400;
        Box::new(this)
    }

    /// The flight's shared part (`ItemOcarina_Fly`, `func_80B864EC`): moved, spun, and falling
    /// at `gravity` down to `minVelocityY`.
    fn fly_step(&mut self) {
        let a = &mut self.actor;
        a.update_pos();
        a.shape_rot.x = a.shape_rot.x.wrapping_add(self.spin_rot_offset.wrapping_mul(2));
        a.shape_rot.y = a.shape_rot.y.wrapping_add(self.spin_rot_offset.wrapping_mul(3));
        if a.min_velocity_y < a.velocity.y {
            a.velocity.y += a.gravity;
            if a.velocity.y < a.min_velocity_y {
                a.velocity.y = a.min_velocity_y;
            }
        }
    }

    /// `ItemOcarina_Fly`: Impa's throw over the moat; the frames 881, 897 and 906 of the
    /// cutscene.
    fn fly(&mut self, play: &mut PlayState) {
        self.fly_step();
        let frame = play.cs_ctx.frames;
        if frame == 881 {
            let a = &mut self.actor;
            a.world_pos = Vec3::new(250.0, 60.0, 1075.0);
            a.velocity = Vec3::new(1.0, -5.0, -7.0);
        }
        if frame == 897 {
            let pos = self.actor.world_pos;
            play.with_ss(|s| {
                s.g_ripple_spawn(pos, 100, 500, 0);
                s.g_splash_spawn(pos, None, None, 1, 0);
            });
            let a = &mut self.actor;
            a.velocity = Vec3::ZERO;
            a.gravity = -0.1;
            a.min_velocity_y = -0.5;
            self.spin_rot_offset = 0;
            audio_play_actor_sfx2(play, NA_SE_EV_BOMB_DROP_WATER);
        }
        // Landed in the water.
        if frame == 906 {
            let ripple_pos = Vec3::new(274.0, -60.0, 907.0);
            play.with_ss(|s| s.g_ripple_spawn(ripple_pos, 100, 500, 0));
        }
    }

    /// `ItemOcarina_GetThrown`: tossed up.
    fn get_thrown(&mut self) {
        let a = &mut self.actor;
        a.gravity = -0.3;
        a.min_velocity_y = -5.0;
        a.velocity = Vec3::new(0.0, 6.0, 0.0);
        self.action = Action::Fly;
    }

    /// `func_80B864EC`: type 1's flight; at frame 220 put at (144, 80, 1686) and thrown again.
    fn fly1(&mut self, play: &mut PlayState) {
        self.fly_step();
        if play.cs_ctx.frames == 220 {
            let a = &mut self.actor;
            a.world_pos = Vec3::new(144.0, 80.0, 1686.0);
            a.velocity = Vec3::new(1.0, 2.0, -7.0);
            a.gravity = -0.15;
            a.min_velocity_y = -5.0;
        }
    }

    /// `func_80B865E0`: type 1 tossed.
    fn throw1(&mut self) {
        let a = &mut self.actor;
        a.gravity = -0.3;
        a.min_velocity_y = -5.0;
        a.velocity = Vec3::new(0.0, 4.0, 6.0);
        self.action = Action::Fly1;
    }

    /// `ItemOcarina_StartSoTCutscene`: the get-item text closing starts
    /// `gHyruleFieldZeldaSongOfTimeCs`.
    fn start_sot_cutscene(&mut self, play: &mut PlayState) {
        if oot_game::npc::textbox_is_closing(play) {
            play.cs_ctx.segment = play.cutscene_script(SONG_OF_TIME_CS);
            play.save.cutscene_trigger = 1;
        }
    }

    /// `ItemOcarina_WaitInWater`: taken (`EVENTCHKINF_43`, switch 3, undrawn, on to the
    /// cutscene), or offered with its bubbles.
    fn wait_in_water(&mut self, play: &mut PlayState) {
        if actor_has_parent(&self.actor) {
            play.save.set_event_chk_inf(EVENTCHKINF_43);
            play.flags.set_switch(OCARINA_SWITCH_FLAG);
            self.action = Action::StartSoTCutscene;
            self.drawn = false;
        } else {
            let a = self.actor.clone();
            offer_get_item_range(play, &a, GI_OCARINA_OF_TIME, 30.0, 50.0);
            if play.gameplay_frames & 13 == 0 {
                let pos = self.actor.world_pos;
                play.with_ss(|s| s.bubble_spawn(pos, 0.0, 0.0, 10.0, 0.13));
            }
        }
    }
}

impl ActorImpl for ItemOcarina {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `ItemOcarina_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::GetThrown => self.get_thrown(),
            Action::Fly => self.fly(play),
            Action::Throw1 => self.throw1(),
            Action::Fly1 => self.fly1(play),
            Action::DoNothing => {}
            Action::WaitInWater => self.wait_in_water(play),
            Action::StartSoTCutscene => self.start_sot_cutscene(play),
        }
    }

    /// `ItemOcarina_Draw`: `GetItem_Draw(GID_OCARINA_OF_TIME)` at `Actor_Draw`'s matrix.
    fn draw(&self, rs: &RenderState, play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        if !self.drawn {
            return;
        }
        let Some(a) = &play.assets else { return };
        // func_8002EBCC, func_8002ED80: the highlights, not drawn.
        oot_game::draw::get_item_draw(&a.items, GID_OCARINA_OF_TIME, actor_draw_matrix(rs), play.gameplay_frames, view, out);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_bubble_frames() {
        // gameplayFrames & 13 == 0: 0, 2, 16, 18, 32, ...
        let frames: Vec<u32> = (0..34).filter(|f| f & 13 == 0).collect();
        assert_eq!(frames, vec![0, 2, 16, 18, 32]);
    }
}
