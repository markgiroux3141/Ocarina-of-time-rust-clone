//! `En_Siofuki` (`ovl_En_Siofuki/z_en_siofuki.c`): the water spouts inside Lord Jabu-Jabu (GAME-10
//! milestone 4), DynaPoly actors of `object_siofuki` (`object_siofuki_Col_000D78`,
//! `DYNA_TRANSFORM_POS`) drawn as a column of scrolling water (`object_siofuki_DL_000B70`).
//!
//! - **The column lifts Link** (`func_80AFBE8C`): Link inside its footprint (346 × `scale.x`
//!   across x, 400 × `scale.z` across z) and below its top but not standing on it, his gravity and
//!   fall are cut and he's drawn up to its top (`Math_SmoothStepToF`, 4 a frame at most); walking
//!   against its flow (facing within a quarter turn of the way out from its middle) he's slowed by
//!   how far he is from it and pushed back out (`pushedYaw`, `pushedSpeed` easing to 3), else
//!   halved and pushed on. Out of it again, the push he had is added to his speed once. Standing on
//!   top he splashes (`EffectSsGSplash` type 1, every 10 frames).
//! - **Its height** (`currentHeight`, eased to `targetHeight`: `Math_SmoothStepToF`, 0.8, 3 a frame
//!   at most) bobs 4 up and down every 32 frames (`func_80AFBDC8`); the drawn column is moved to
//!   match (`unk_170`).
//! - **Rising** (type 0, `EN_SIOFUKI_RAISING`): 10 high until its switch flag is set (by a yellow
//!   `Bg_Bdan_Switch`): then 400 high for 300 frames (the timer's ticks), and back down, the flag
//!   cleared.
//! - **Lowering** (type 1, `EN_SIOFUKI_LOWERING`): `shape.rot.x × 40` high; on its switch flag the
//!   attention camera (one-point 5010, 40 frames), 20 frames on it drops to -45 for
//!   `shape.rot.y × 20` frames and comes back, the flag cleared; once its chest flag is set (a chest
//!   opened), at -45 for good.
//!
//! Its size is `shape.rot.z` (`scale.x` = z / 1.73 × 0.1, `scale.z` = z × 0.05); the rotations are
//! zeroed. In room 10 with switch flag 0x1E set it's gone at once. `NA_SE_EV_FOUNTAIN` plays at it
//! while drawn, higher the higher it stands (a lowering one only above -35).
//!
//! The whole overlay is ported.
//!
//! Jabu-Jabu (Master Quest) places five: 0x15C6 (lowering, switch 0x17, chest 6: the boomerang's)
//! in room 1 before door 14 and twice in room 14 (by the boomerang's ledge, 200 high), 0x0EBF
//! (rising, switch 0x3A, room 3) and 0x0F7F (rising, switch 0x3D, room 12).
//!
//! Params: `chestFlag` bits 0 to 5 (lowering: down for good once it's set); `switchFlag` bits 6 to
//! 11; `type` bits 12 to 15 (0 rising, 1 lowering; others kill it). Rotation: `shape.rot.x` the
//! lowering one's height in 40s, `shape.rot.y` how long it stays down in 20-frame units,
//! `shape.rot.z` its width.
//! Flags: reads and clears switch flag `switchFlag`; a lowering one reads chest flag `chestFlag`;
//! reads switch flag 0x1E in room 10.

use std::sync::Arc;

use eng_collision::collision::CollisionHeader;
use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource, DYNA_INTERACT_PLAYER_ON_TOP, DYNA_TRANSFORM_POS};
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{approach_f, smooth_step_to_f};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, Source};
use oot_game::audio::sfx::{NA_SE_EV_FOUNTAIN, SFX_FLAG, SfxPos};
use oot_game::camera::{CAM_ID_MAIN, f_atan2f};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::scene_table::gfx_two_tex_scroll;

/// `ACTOR_EN_SIOFUKI` (`actor_table.h`: 0x015F).
pub const ACTOR_EN_SIOFUKI: i16 = 0x015F;
/// `OBJECT_SIOFUKI` (`object_table.h`: 0x013A).
pub const OBJECT: &str = "object_siofuki";
/// `object_siofuki_Col_000D78`, `object_siofuki_DL_000B70` (`object_siofuki.xml`).
pub const COLLISION: &str = "object_siofuki_Col_000D78";
pub const DL: &str = "object_siofuki_DL_000B70";
/// The bake: the column with its scroll on segment 8.
pub const BAKE: &str = "En_Siofuki/spout";
const SEG_SCROLL: u8 = 0x08;

/// `En_Siofuki_Profile`: `ACTORCAT_BG`, culling off for both, `OBJECT_SIOFUKI`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_SIOFUKI,
    name: "En_Siofuki",
    category: ACTORCAT_BG,
    flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `PARAMS_GET_U((u16)params, 0, 6)`.
pub const CHEST_FLAG: Field = Field::params("chestFlag", 0, 6, Kind::ChestFlag, "a lowering one: down for good once it's set (a chest opened)");
/// `PARAMS_GET_U((u16)params, 6, 6)`.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 6, 6, Kind::SwitchFlag, "raises a rising one, drops a lowering one for a while; cleared when it's back");
/// `PARAMS_GET_U((u16)params, 12, 4)`: `EnSiofukiType`.
pub const TYPE: Field = Field::params("type", 12, 4, Kind::Named(&[(0, "rising (10 high, 400 on its flag)"), (1, "lowering (down on its flag, for good on its chest flag)")]), "how it moves (any other kills it)");
/// `shape.rot.x`: the lowering one's height, 40 a unit.
pub const ROT_X: Field = Field::rot("maxHeight", Source::RotX, 0, 16, Kind::Range(0, 100), "a lowering one's height, 40 a unit");
/// `shape.rot.y`: `activeTime`, the frames down in 20s.
pub const ROT_Y: Field = Field::rot("activeTime", Source::RotY, 0, 16, Kind::Range(0, 100), "how long a lowering one stays down, 20 frames a unit");
/// `shape.rot.z`: its width.
pub const ROT_Z: Field = Field::rot("width", Source::RotZ, 0, 16, Kind::Range(0, 20), "its width (scale.x = z / 1.73 × 0.1, scale.z = z × 0.05); 0 keeps 0.1");

/// En_Siofuki's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[CHEST_FLAG, SWITCH_FLAG, TYPE, ROT_X, ROT_Y, ROT_Z],
    flags: &[
        FlagUse::new(Flag::Switch("switchFlag")).reads().clears().note("raises or drops it; cleared when it's back"),
        FlagUse::new(Flag::Chest("chestFlag")).reads().note("a lowering one: down for good"),
        FlagUse::new(Flag::Switch("0x1E")).reads().note("in room 10: gone"),
    ],
    todo: false,
    objects: &[],
    texts: &[],
};

/// `EnSiofukiType` (`z_en_siofuki.h`).
pub const EN_SIOFUKI_RAISING: i16 = 0;
pub const EN_SIOFUKI_LOWERING: i16 = 1;

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_80AFC34C`: rising, waiting for its flag.
    WaitRaise,
    /// `func_80AFC218`: up (rising) or down (lowering) for the timer.
    Active,
    /// `func_80AFC3C8`: lowering, the attention camera's 20 frames.
    WaitLower,
    /// `func_80AFC478`: lowering, waiting for its flag.
    Wait,
    /// `func_80AFC544`: down for good.
    Down,
}

pub struct EnSiofuki {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    pub action: Action,
    pub timer: i32,
    /// `initPosY`.
    pub init_pos_y: f32,
    /// `unk_170`: the drawn column's offset (model units); `unk_174` its base, 35.
    pub unk_170: f32,
    pub unk_174: f32,
    pub oscillation: f32,
    pub target_height: f32,
    pub current_height: f32,
    pub splash_timer: i32,
    pub apply_speed: bool,
    pub applied_speed: f32,
    pub target_applied_speed: f32,
    pub applied_yaw: i16,
    pub active_time: i16,
    pub max_height: f32,
    pub sfx_flags: u8,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

fn load_collision(play: &PlayState) -> anyhow::Result<Arc<CollisionHeader>> {
    let assets = play.assets.as_ref().ok_or_else(|| anyhow::anyhow!("no asset pack"))?;
    Ok(Arc::new(assets.pack.collision(&keys::collision(OBJECT, COLLISION))?))
}

/// The column's scroll: `Gfx_TwoTexScroll(.., G_TX_RENDERTILE, x, y, 64, 64, 1, x, y, 64, 64)` with
/// `x = gameplayFrames × 15`, `y = gameplayFrames × -15` (u32s).
pub fn scroll(frames: u32) -> Vec<(u32, u32)> {
    let x = frames.wrapping_mul(15);
    let y = frames.wrapping_mul(15).wrapping_neg();
    gfx_two_tex_scroll(0, x, y, 64, 64, 1, x, y, 64, 64)
}

/// `EnSiofuki_Draw`'s mesh (after `Gfx_SetupDL_25Xlu`): the column, its scroll on segment 8.
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: BAKE.into(),
        object: OBJECT.into(),
        segments: vec![(SEG_SCROLL, BakeSegment::Dynamic(scroll(0)))],
        prelude: Vec::new(),
        body: BakeBody::DLists(vec![(OBJECT.into(), DL.into())]),
    }]
}

/// `RAD_TO_BINANG` (`z_math.h`): truncated, as the C's `(s16)(s32)` casts.
fn rad_to_binang(r: f32) -> i16 {
    (r * (32768.0 / std::f32::consts::PI)) as i32 as i16
}

impl EnSiofuki {
    fn ty(&self) -> i16 {
        TYPE.get(self.actor.params)
    }

    fn switch_flag(&self) -> i32 {
        SWITCH_FLAG.get(self.actor.params) as i32
    }

    fn chest_flag(&self) -> i32 {
        CHEST_FLAG.get(self.actor.params) as i32
    }

    /// `EnSiofuki_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = EnSiofuki {
            actor,
            bg: BG_ACTOR_MAX,
            action: Action::Down,
            timer: 0,
            init_pos_y: 0.0,
            unk_170: 0.0,
            unk_174: 0.0,
            oscillation: 0.0,
            target_height: 0.0,
            current_height: 0.0,
            splash_timer: 0,
            apply_speed: false,
            applied_speed: 0.0,
            target_applied_speed: 0.0,
            applied_yaw: 0,
            active_time: 0,
            max_height: 0.0,
            sfx_flags: 0,
        };
        if this.actor.room == 10 && play.flags.get_switch(0x1E) {
            this.actor.kill();
            return Box::new(this);
        }
        // sInitChain: ICHAIN_VEC3F_DIV1000(scale, 100).
        this.actor.scale = Vec3::splat(0.1);
        // DynaPolyActor_Init(DYNA_TRANSFORM_POS), DynaPoly_SetBgActor.
        this.bg = match load_collision(play) {
            Ok(h) => play.col.dyna.set_bg_actor(h, source(&this.actor), DYNA_TRANSFORM_POS),
            Err(e) => {
                log::error!("En_Siofuki: {e:#}");
                BG_ACTOR_MAX
            }
        };
        this.sfx_flags |= 1;
        let ty = this.ty();
        if !(ty == EN_SIOFUKI_RAISING || ty == EN_SIOFUKI_LOWERING) {
            this.actor.kill();
            return Box::new(this);
        }
        this.init_pos_y = this.actor.world_pos.y;
        this.unk_174 = 35.0;
        this.unk_170 = -6058.0 + this.unk_174;
        let r = this.actor.shape_rot;
        let (rx, ry, rz) = (ROT_X.get(r.x), ROT_Y.get(r.y), ROT_Z.get(r.z));
        if rx != 0 {
            this.max_height = rx as f32 * 40.0;
            this.current_height = this.max_height;
        }
        this.active_time = 0;
        if ry != 0 {
            this.active_time = ry;
        }
        if rz != 0 {
            this.actor.scale.x = rz as f32 * (1.0 / 1.73) * 0.1;
            this.actor.scale.z = rz as f32 * 0.5 * 0.1;
        }
        this.actor.world_rot = Default::default();
        this.actor.shape_rot = Default::default();
        if ty == EN_SIOFUKI_RAISING {
            this.current_height = 10.0;
            this.target_height = 10.0;
            this.action = Action::WaitRaise;
        } else if play.flags.get_treasure(this.chest_flag()) {
            this.current_height = -45.0;
            this.target_height = -45.0;
            this.action = Action::Down;
        } else {
            this.target_height = this.current_height;
            this.action = Action::Wait;
        }
        if this.bg != BG_ACTOR_MAX {
            play.col.dyna.set_source(this.bg, source(&this.actor));
        }
        Box::new(this)
    }

    /// `func_80AFBDC8`: the bob (4 up and down every 32 frames), the drawn column's offset, the
    /// height.
    fn func_80afbdc8(&mut self, play: &PlayState) {
        self.oscillation = ((play.gameplay_frames & 0x1F) as f32 / 32.0 * std::f32::consts::PI * 2.0).sin() * 4.0;
        self.unk_170 = self.unk_174 * 10.0 + -6058.0 - self.oscillation * 10.0;
        self.unk_174 = 35.0;
        self.actor.world_pos.y = self.init_pos_y + self.current_height + self.oscillation;
    }

    /// `func_80AFBE8C`: Link in the column lifted and pushed, on top splashing, out of it given the
    /// push once.
    fn func_80afbe8c(&mut self, play: &mut PlayState) {
        let Some(ph) = play.player else { return };
        let Some((ppos, prot_y)) = play.actors.actor(ph).map(|a| (a.world_pos, a.world_rot.y)) else { return };
        let d = ppos - self.actor.world_pos;
        let s = self.actor.scale;
        if d.x > s.x * -346.0 && d.x < s.x * 346.0 && d.z > s.z * -400.0 && d.z < s.z * 400.0 && d.y < 0.0 {
            if self.bg != BG_ACTOR_MAX && play.col.dyna.interact_flag(self.bg, DYNA_INTERACT_PLAYER_ON_TOP) {
                if self.splash_timer <= 0 {
                    play.with_ss(|ss| ss.g_splash_spawn(ppos, None, None, 1, 1));
                    self.splash_timer = 10;
                } else {
                    self.splash_timer -= 1;
                }
                self.apply_speed = false;
                self.applied_speed = 0.0;
                self.target_applied_speed = 0.0;
            } else {
                let dist2d = (d.x * d.x + d.z * d.z).sqrt();
                self.apply_speed = true;
                self.splash_timer = 0;
                let angle = rad_to_binang(f_atan2f(d.x, d.z));
                let d_angle = ((prot_y ^ -0x8000) as i32 - angle as i32) as i16;
                let top = self.actor.world_pos.y;
                if let Some(a) = play.actors.actor_mut(ph) {
                    a.gravity = 0.0;
                    a.velocity.y = 0.0;
                    smooth_step_to_f(&mut a.world_pos.y, top, 0.5, 4.0, 1.0);
                }
                let speed = play.actors.get(ph).and_then(|p| p.as_player()).map_or(0.0, |p| p.player_speed());
                let new_speed = if d_angle < 0x4000 && d_angle > -0x4000 {
                    self.applied_yaw = prot_y ^ -0x8000;
                    let speed_scale = (dist2d / (s.x * 40.0 * 10.0)).clamp(0.0, 1.0);
                    approach_f(&mut self.target_applied_speed, 3.0, 1.0, 1.0);
                    approach_f(&mut self.applied_speed, self.target_applied_speed, 1.0, 0.3 * speed_scale);
                    speed * speed_scale
                } else {
                    self.applied_yaw = prot_y;
                    approach_f(&mut self.target_applied_speed, 3.0, 1.0, 1.0);
                    approach_f(&mut self.applied_speed, self.target_applied_speed, 1.0, 0.1);
                    speed / 2.0
                };
                let (yaw, pushed) = (self.applied_yaw, self.applied_speed);
                if let Some(p) = play.actors.get_mut(ph).and_then(|p| p.as_player_mut()) {
                    p.set_player_speed(new_speed);
                    p.set_pushed(yaw, pushed);
                }
            }
        } else {
            if self.apply_speed {
                let (yaw, pushed) = (self.applied_yaw, self.applied_speed);
                if let Some(p) = play.actors.get_mut(ph).and_then(|p| p.as_player_mut()) {
                    let speed = p.player_speed();
                    p.set_player_speed(pushed + speed);
                    p.set_yaw(yaw);
                }
            }
            self.target_applied_speed = 0.0;
            self.applied_speed = 0.0;
            self.apply_speed = false;
        }
    }

    /// `func_80AFC1D0`: the height eased to its target.
    fn func_80afc1d0(&mut self) {
        smooth_step_to_f(&mut self.current_height, self.target_height, 0.8, 3.0, 0.01);
    }

    /// The three every moving state runs first.
    fn bob_lift_ease(&mut self, play: &mut PlayState) {
        self.func_80afbdc8(play);
        self.func_80afbe8c(play);
        self.func_80afc1d0();
    }

    /// The chest's flag set: down for good.
    fn down_for_good(&mut self) {
        self.current_height = -45.0;
        self.target_height = -45.0;
        self.action = Action::Down;
    }

    /// `func_80AFC218`: the timer out, the flag cleared and back (rising: 10 high; lowering: its
    /// height); else its ticks. A lowering one's chest flag: down for good, the flag cleared.
    fn func_80afc218(&mut self, play: &mut PlayState) {
        self.bob_lift_ease(play);
        self.timer -= 1;
        if self.timer < 0 {
            play.flags.unset_switch(self.switch_flag());
            match self.ty() {
                EN_SIOFUKI_RAISING => {
                    self.target_height = 10.0;
                    self.action = Action::WaitRaise;
                }
                EN_SIOFUKI_LOWERING => {
                    self.target_height = self.max_height;
                    self.action = Action::Wait;
                }
                _ => {}
            }
        } else {
            self.actor.play_sfx_flagged_timer(self.timer);
        }
        if self.ty() == EN_SIOFUKI_LOWERING && play.flags.get_treasure(self.chest_flag()) {
            self.down_for_good();
            play.flags.unset_switch(self.switch_flag());
        }
    }

    /// `func_80AFC34C`: rising, its flag set: 400 high for 300 frames.
    fn func_80afc34c(&mut self, play: &mut PlayState) {
        self.bob_lift_ease(play);
        if play.flags.get_switch(self.switch_flag()) {
            self.target_height = 400.0;
            self.timer = 300;
            self.action = Action::Active;
        }
    }

    /// `func_80AFC3C8`: lowering, 20 frames on down to -45 for `activeTime × 20` frames; the chest
    /// flag: down for good.
    fn func_80afc3c8(&mut self, play: &mut PlayState) {
        self.bob_lift_ease(play);
        self.timer -= 1;
        if self.timer < 0 {
            self.timer = self.active_time as i32 * 20;
            self.target_height = -45.0;
            self.action = Action::Active;
        }
        if play.flags.get_treasure(self.chest_flag()) {
            self.down_for_good();
        }
    }

    /// `func_80AFC478`: lowering, its flag set: the attention camera (5010, 40 frames) and 20
    /// frames' wait; the chest flag: down for good.
    fn func_80afc478(&mut self, play: &mut PlayState) {
        self.bob_lift_ease(play);
        if self.ty() == EN_SIOFUKI_LOWERING {
            if play.flags.get_switch(self.switch_flag()) {
                self.timer = 20;
                self.action = Action::WaitLower;
                let me = play.cur_actor.map(|h| play.cam_actor_of(h, &self.actor));
                play.onepoint_cutscene_init(5010, 40, me, CAM_ID_MAIN);
            }
            if play.flags.get_treasure(self.chest_flag()) {
                self.down_for_good();
            }
        }
    }

    /// `func_80AFC544`: down for good: the bob and the height only.
    fn func_80afc544(&mut self, play: &mut PlayState) {
        self.func_80afbdc8(play);
        self.func_80afc1d0();
    }
}

impl ActorImpl for EnSiofuki {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnSiofuki_Update`: the action; the new transform for `DynaPoly_UpdateContext`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::WaitRaise => self.func_80afc34c(play),
            Action::Active => self.func_80afc218(play),
            Action::WaitLower => self.func_80afc3c8(play),
            Action::Wait => self.func_80afc478(play),
            Action::Down => self.func_80afc544(play),
        }
        if self.bg != BG_ACTOR_MAX {
            play.col.dyna.set_source(self.bg, source(&self.actor));
        }
    }

    /// `EnSiofuki_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        if self.bg != BG_ACTOR_MAX {
            play.col.dyna.delete_bg_actor(self.bg);
        }
    }

    /// `EnSiofuki_Draw`'s sound: `NA_SE_EV_FOUNTAIN` at it (`func_800F436C`), its pitch 1 plus how
    /// high it stands (rising: from 10 to 400; lowering: above -35, to its height).
    fn draw_sfx(&mut self, play: &mut PlayState) {
        if self.actor.killed || self.sfx_flags & 1 == 0 {
            return;
        }
        let Some(h) = play.cur_actor else { return };
        let ratio = match self.ty() {
            EN_SIOFUKI_RAISING => Some((self.current_height - 10.0) / (400.0 - 10.0)),
            EN_SIOFUKI_LOWERING if self.current_height > -35.0 => Some((self.current_height - -35.0) / (self.max_height - -35.0)),
            _ => None,
        };
        if let Some(r) = ratio {
            play.audio.func_800f436c(SfxPos::Actor(h), NA_SE_EV_FOUNTAIN - SFX_FLAG, 1.0 + r);
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.values = vec![self.unk_170];
        rs
    }

    /// `EnSiofuki_Draw`: the column, `unk_170` (model units) along y, its scroll on segment 8.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let unk_170 = rs.values.first().copied().unwrap_or(self.unk_170);
        let m = actor_draw_matrix(rs) * Mat4::from_translation(Vec3::new(0.0, unk_170, 0.0));
        let mut sv = SegmentValues::default();
        sv.read(SEG_SCROLL, &scroll(play.gameplay_frames));
        out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE)), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
    }

    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
