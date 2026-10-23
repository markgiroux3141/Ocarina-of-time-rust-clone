//! `En_Kakasi2` (`ovl_En_Kakasi2/z_en_kakasi2.c`): Pierre, the scarecrow who comes up out of the
//! ground where the adult plays the scarecrow's song (Lake Hylia's adult layers, and other adult
//! scenes: a hookshot target). `object_ka`'s skeleton.
//!
//! - **Hidden** (`func_80A90264`; not drawn, 8000 under his place, no collider): once Bonooru has
//!   heard the song (`EVENTCHKINF_9C`), with Link within his range across and high, the song
//!   played (`OCARINA_MODE_0B`): his switch flag set, the song's mode back to 4, the chime, the
//!   attention cutscene on him (`OnePointCutscene_Attention`), lock-on still off.
//!   (`DEBUG_FEATURES`' `BREG(1)` brings him up without the song: 0 here.)
//! - **Coming up** (`func_80A904D8`, `func_80A90578`): `NA_SE_EV_COME_UP_DEKU_JR`, the dance
//!   looped, spinning 0x800 a frame, rising (`shape.yOffset` to 0, half the way and at most 500 a
//!   frame) until he's within 100.
//! - **Standing** (`func_80A9062C`, `func_80A906C4`): the animation held, easing to its first
//!   frame, turning back to his yaw. His switch flag set at init, he's there from the start.
//!
//! Drawn (`func_80A90948`) once he's up: the skeleton, no limb overrides. His collider's AC and
//! OC only at `shape.yOffset` 0. The debug display (`BREG(0)`: his place as a cross) isn't drawn.
//! The whole overlay is ported.
//!
//! Params: `switchFlag` bits 0 to 5 (0x3F: none); `spawnRangeY` bits 6 to 13 (the decomp's name;
//! the range **across**: 40 a unit plus 40). `world.rot.z`: `spawnRangeXZ` (the decomp's name;
//! the range **up and down**, 40 a unit plus 40; also his roll, as `Actor_Init` copies it).
//! Flags: reads and sets the switch flag `switchFlag` (set: up from the start); reads
//! EVENTCHKINF_9C (the song heard by Bonooru, `En_Kakasi3`).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_math::{approach_zero_f, smooth_step_to_s};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_LOCK_ON_DISABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_UPDATE_DURING_OCARINA, Actor};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, Source};
use oot_game::audio::sfx::NA_SE_SY_CORRECT_CHIME;
use oot_game::collision_check::*;
use oot_game::event_chk_inf;
use oot_game::message::{OCARINA_MODE_04, OCARINA_MODE_0B};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_ONCE, Anim, SkelAnimeStd};

use crate::en_kakasi::{ACTOR_FLAG_HOOKSHOT_PULLS_PLAYER, NA_SE_EV_KAKASHI_SWING, OBJECT, cam_actor, change_anim, draw_skel, load, skel_render_state};

/// `ACTOR_EN_KAKASI2` (`actor_table.h`: 0x01C9).
pub const ACTOR_EN_KAKASI2: i16 = 0x01C9;

/// `En_Kakasi2_Profile`: `ACTORCAT_PROP`, attention-enabled, never culled, updating during the
/// ocarina, no lock-on; no draw until he's up.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_KAKASI2,
    name: "En_Kakasi2",
    category: ACTORCAT_PROP,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA | ACTOR_FLAG_LOCK_ON_DISABLED,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `PARAMS_GET_U(params, 0, 6)`: the switch flag.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 0, 6, Kind::SwitchFlag, "set when he comes up; set at init, he's up from the start (0x3F: none)");
/// `PARAMS_GET_U(params, 6, 8)`: `spawnRangeY`, the range across (`maxSpawnDistance.x`).
pub const SPAWN_RANGE_Y: Field = Field::params("spawnRangeY", 6, 8, Kind::Range(0, 255), "the range across Link plays within: 40 a unit plus 40 (the decomp's `spawnRangeY`)");
/// `world.rot.z`: `spawnRangeXZ`, the range up and down (`maxSpawnDistance.y`).
pub const SPAWN_RANGE_XZ: Field =
    Field::rot("spawnRangeXZ", Source::RotZ, 0, 16, Kind::Range(-0x8000, 0x7FFF), "the range up and down Link plays within: 40 a unit plus 40 (the decomp's `spawnRangeXZ`); also his roll");

/// En_Kakasi2's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[SWITCH_FLAG, SPAWN_RANGE_Y, SPAWN_RANGE_XZ],
    flags: &[
        FlagUse::new(Flag::Switch("switchFlag")).reads().sets(),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_9C)).reads().note("Bonooru (En_Kakasi3) has heard the scarecrow's song: the song brings him up"),
    ],
    todo: false,
    objects: &[],
    texts: &[],
};

/// `EVENTCHKINF_9C` (`save.h`: 0x9C).
pub const EVENTCHKINF_9C: u16 = 0x9C;
/// `ATTENTION_RANGE_4` (`actor.h`).
const ATTENTION_RANGE_4: u8 = 4;
/// `NA_SE_EV_COME_UP_DEKU_JR` (`environmentbank_table.h`: 0x28BF).
pub const NA_SE_EV_COME_UP_DEKU_JR: u16 = 0x28BF;
/// `BREG(1)` (`regs.h`): a debug register, 0.
const BREG_1: i16 = 0;
/// `BREG(0)`: 0 (the debug display's crosses).
const BREG_0: i16 = 0;

/// `sCylinderInit`.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCFFFFF, hit_special_effect: 0, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCFFFFF, hit_backlash: 0, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON | ACELEM_HOOKABLE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 20, height: 70, y_shift: 0, pos: [0; 3] },
};

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_80A90264`: hidden, waiting for the song.
    Func80a90264,
    /// `func_80A904D8`: coming up: the animation and the sound.
    Func80a904d8,
    /// `func_80A90578`: rising, spinning.
    Func80a90578,
    /// `func_80A9062C`: the animation held.
    Func80a9062c,
    /// `func_80A906C4`: standing.
    Func80a906c4,
}

pub struct EnKakasi2 {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Arc<Skeleton>,
    anim: Anim,
    pub action: Action,
    /// `unk_194`: the frames hidden (the debug display's blink).
    pub unk_194: i16,
    /// `switchFlag` (-1: none).
    pub switch_flag: i16,
    /// `unk_198`: the yaw he turns back to.
    pub unk_198: i16,
    /// `maxSpawnDistance`: x the range across, y up and down.
    pub max_spawn_distance: Vec3,
    pub height: f32,
    pub collider: ColliderCylinder,
    /// `actor.draw` set (`func_80A90948`), not `NULL`.
    pub drawn: bool,
}

impl EnKakasi2 {
    /// `EnKakasi2_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some((skeleton, anim)) = load(play, "En_Kakasi2") else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let mut this = EnKakasi2 {
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), None),
            skeleton,
            anim,
            action: Action::Func80a90264,
            unk_194: 0,
            switch_flag: 0,
            unk_198: 0,
            max_spawn_distance: Vec3::ZERO,
            height: 0.0,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            drawn: false,
            actor,
        };
        this.switch_flag = SWITCH_FLAG.get(this.actor.params);
        let spawn_range_y = SPAWN_RANGE_Y.get(this.actor.params) as f32;
        let spawn_range_xz = SPAWN_RANGE_XZ.get(this.actor.world_rot.z) as f32;
        if this.switch_flag == 0x3F {
            this.switch_flag = -1;
        }
        this.actor.target_mode = ATTENTION_RANGE_4;
        this.max_spawn_distance.x = spawn_range_y * 40.0 + 40.0;
        this.max_spawn_distance.y = spawn_range_xz * 40.0 + 40.0;
        log::debug!("Umeda Sangyo visit! range {spawn_range_y} ({}), z angle {spawn_range_xz} ({}), save {}", this.max_spawn_distance.x, this.max_spawn_distance.y, this.switch_flag);
        this.actor.col_chk_info.mass = MASS_IMMOVABLE;
        this.height = 60.0;
        this.actor.scale = Vec3::splat(0.01);
        this.actor.flags |= ACTOR_FLAG_HOOKSHOT_PULLS_PLAYER;
        this.unk_198 = this.actor.shape_rot.y;
        if this.switch_flag >= 0 && play.flags.get_switch(this.switch_flag as i32) {
            this.come_up_skel();
            this.action = Action::Func80a9062c;
        } else {
            this.action = Action::Func80a90264;
            this.actor.shape_y_offset = -8000.0;
        }
        Box::new(this)
    }

    /// `actor.draw = func_80A90948`, the cylinder, `SkelAnime_InitFlex` (with the dance).
    fn come_up_skel(&mut self) {
        self.drawn = true;
        self.collider = ColliderCylinder::new(&CYLINDER_INIT);
        self.skel = SkelAnimeStd::init_flex(self.skeleton.limbs.len(), Some(self.anim.clone()));
    }

    /// `func_80A90264`: hidden. Bonooru has heard the song and Link's within range, the song
    /// played (`OCARINA_MODE_0B`): the switch flag, the mode back to 4, drawn, the attention
    /// cutscene, the chime.
    fn func_80a90264(&mut self, play: &mut PlayState) {
        self.unk_194 = self.unk_194.wrapping_add(1);
        let player_y = play.player.and_then(|h| play.actors.actor(h)).map_or(0.0, |p| p.world_pos.y);
        let in_range = self.actor.xz_dist_to_player < self.max_spawn_distance.x && (player_y - self.actor.world_pos.y).abs() < self.max_spawn_distance.y;
        if BREG_1 != 0 && in_range {
            self.come_up_skel();
            if let Some(me) = cam_actor(play, &self.actor) {
                play.onepoint_attention(me);
            }
            self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_LOCK_ON_DISABLED;
            play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
            if self.switch_flag >= 0 {
                play.flags.set_switch(self.switch_flag as i32);
            }
            self.action = Action::Func80a904d8;
        } else if in_range && play.save.get_event_chk_inf(EVENTCHKINF_9C) {
            self.unk_194 = 0;
            if play.msg_ctx.ocarina_mode == OCARINA_MODE_0B {
                if self.switch_flag >= 0 {
                    play.flags.set_switch(self.switch_flag as i32);
                }
                log::debug!("En_Kakasi2: SAVE finished {}", self.switch_flag);
                play.msg_ctx.ocarina_mode = OCARINA_MODE_04;
                self.come_up_skel();
                if let Some(me) = cam_actor(play, &self.actor) {
                    play.onepoint_attention(me);
                }
                play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
                self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_LOCK_ON_DISABLED;
                self.action = Action::Func80a904d8;
            }
        }
    }

    /// `func_80A904D8`: the dance looped, `NA_SE_EV_COME_UP_DEKU_JR`.
    fn func_80a904d8(&mut self, play: &mut PlayState) {
        change_anim(&mut self.skel, &self.anim, 1.0, ANIMMODE_LOOP);
        audio_play_actor_sfx2(play, NA_SE_EV_COME_UP_DEKU_JR);
        self.action = Action::Func80a90578;
    }

    /// `func_80A90578`: dancing, spinning 0x800 a frame, rising; within 100 he's up.
    fn func_80a90578(&mut self, play: &mut PlayState) {
        self.skel.update();
        let current_frame = self.skel.cur_frame as i16;
        if current_frame == 11 || current_frame == 17 {
            audio_play_actor_sfx2(play, NA_SE_EV_KAKASHI_SWING);
        }
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x800);
        approach_zero_f(&mut self.actor.shape_y_offset, 0.5, 500.0);
        if self.actor.shape_y_offset > -100.0 {
            self.action = Action::Func80a9062c;
            self.actor.shape_y_offset = 0.0;
        }
    }

    /// `func_80A9062C`: the animation held (speed 0, once).
    fn func_80a9062c(&mut self, _play: &mut PlayState) {
        change_anim(&mut self.skel, &self.anim, 0.0, ANIMMODE_ONCE);
        self.action = Action::Func80a906c4;
    }

    /// `func_80A906C4`: easing to the first frame, turning back to his yaw.
    fn func_80a906c4(&mut self, _play: &mut PlayState) {
        if self.skel.cur_frame != 0.0 {
            approach_zero_f(&mut self.skel.cur_frame, 0.5, 1.0);
        }
        smooth_step_to_s(&mut self.actor.shape_rot.y, self.unk_198, 5, 0xBB8, 0);
        self.skel.update();
    }
}

impl ActorImpl for EnKakasi2 {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnKakasi2_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.actor.world_rot = self.actor.shape_rot;
        self.actor.set_focus(self.height);
        match self.action {
            Action::Func80a90264 => self.func_80a90264(play),
            Action::Func80a904d8 => self.func_80a904d8(play),
            Action::Func80a90578 => self.func_80a90578(play),
            Action::Func80a9062c => self.func_80a9062c(play),
            Action::Func80a906c4 => self.func_80a906c4(play),
        }
        // Actor_MoveXZGravity.
        self.actor.move_forward();
        if self.actor.shape_y_offset == 0.0 {
            self.collider.update(&self.actor);
            play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        }
        if BREG_0 != 0 && !self.drawn {
            // DebugDisplay_AddObject: his place, a grey cross blinking (unk_194 even) or a cyan
            // one. Not ported (no debug display).
            log::trace!("En_Kakasi2: debug display at {}", self.actor.world_pos);
        }
    }
    /// `EnKakasi2_Destroy`: the cylinder. `//! @bug (game)`: `SkelAnime_Free` isn't called.
    fn destroy(&mut self, _play: &mut PlayState) {}
    fn render_state(&self) -> RenderState {
        let mut rs = skel_render_state(&self.actor, &self.skel);
        rs.switches = vec![self.drawn as u32];
        rs
    }
    /// `func_80A90948`, once he's up.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if rs.switches.first().copied().unwrap_or(0) == 0 {
            return;
        }
        draw_skel(&self.skeleton, rs, out);
    }
    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0).then_some(ColliderMut::Cylinder(&mut self.collider))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
