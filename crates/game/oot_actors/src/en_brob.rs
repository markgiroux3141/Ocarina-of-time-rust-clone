//! `En_Brob` (`ovl_En_Brob/z_en_brob.c`): the Flobbery muscle block of Jabu-Jabu's Belly
//! (GAME-10 milestone 4), a pillar of flesh that rises out of the floor as Link nears and
//! shocks him on touch. `object_brob`'s `gBrobSkel` (a flex skeleton of 9 limbs) and its
//! collision `gBrobCol` (`DynaPolyActor_Init(0)`), solid only while it's down.
//!
//! Master Quest's Jabu-Jabu (`bdan`) places six, the same in layers 0 to 3: room 1's three, half
//! size (params 0x01FF), at (-172, -340, -1553) turned 9102, (-233, -330, -1707) and (-177, -340,
//! -1858) turned -6918; room 2's one, full size (0x00FF), at (0, -400, -3715); room 4's two
//! (0x00FF) at (-1353, -20, -1740) and (-1113, -40, -1711). The unused cutscene layer (4) has two
//! 0x00FF in room 4 and two 0x0014 (full size, 20/30 of its height) in room 5.
//!
//! - **Down** (`EnBrob_Idle`: its model at its home, `modelOffsetY` 0; its collision on; no
//!   colliders): Link standing on it is thrown off (`Actor_SetPlayerKnockbackLargeNoDamage`, 5 at
//!   1 up) and it rises; Link within 300 across, it **rises** (`EnBrob_MoveUp`: `gBrobMoveUpAnim`
//!   once, its collision off, the model up 1000 a frame to frame 8, 250 to 12, down 250 after;
//!   hurting and pushing, not hittable). After a boomerang's stun it can't for 200 frames (blue
//!   while over 80 of them).
//! - **Up** (`EnBrob_Wobble`: `gBrobWobbleAnim` looped, a 5-frame morph, `modelOffsetY` 8000,
//!   `NA_SE_EN_BROB_WAVE` at frames 6 and 15): hurting (8, `HIT_SPECIAL_EFFECT_ELECTRIC`: a shock)
//!   and hittable; after 1200 frames, with Link over 500 away, it **sinks**
//!   (`EnBrob_MoveDown`: `gBrobMoveUpAnim` backward, `modelOffsetY` 8250 and down 1000 a frame to
//!   frame 8, 250 to 12, up 250 after; pushing only) and is down again.
//! - **Its shock** (`EnBrob_Shock`: `gBrobShockAnim` looped): its AT touching Link, or a Kokiri
//!   Sword's slash (`DMG_SLASH_KOKIRI`): ten loops of four bolts (`EffectSsLightning`, white in
//!   pale cyan, `scale.y × 8000` big, 4 frames, 1 bolt) at its four sides (`scale.x × 5500` out,
//!   every other frame at the corners), random heights up its body; then up again. Touching
//!   Link while rising, unless the hit was blocked (`AT_BOUNCED`), it throws him off instead.
//! - **Stunned** (`EnBrob_Stunned`): a boomerang (`DMG_BOOMERANG`) on either cylinder:
//!   `gBrobStunnedAnim` once (a 5-frame morph), blue (`COLORFILTER_BUFFLAG_OPA`) 80 frames,
//!   `NA_SE_EN_GOMA_JR_FREEZE`, its model down 125 and 1250 a frame to frame 8, pushing only;
//!   then down (200 frames before it rises again).
//! - Its two cylinders (`sCylinderInit` scaled): the body, `8000 × scale.x` across and
//!   `12000 × scale.y` high from the floor, at limb 3 (`EnBrob_PostLimbDraw`), and the top,
//!   `11000 × scale.y` high from `5000 × scale.y` down, at limb 8 (7 up). Health 0, immovable.
//!
//! The whole overlay is ported. Its draw (`EnBrob_Draw`, `SkelAnime_DrawFlexOpa`) is `gBrobSkel`
//! `modelOffsetY` up (model units); the cylinders' places, from the limbs' matrices, are set in
//! `draw_update`.
//!
//! Params: `size` bits 8 to 15 (0 full size, scale 0.01; any other half, 0.005); `height` bits 0
//! to 7 (0xFF its own height, else `scale.y` times `height / 30`, twice that at half size;
//! `params &= 0xFF` at init).
//! Flags: none.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource, DYNA_INTERACT_PLAYER_ON_TOP};
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ActorImpl, ActorProfile, actor_set_player_knockback_large_no_damage, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::audio::sfx::NA_SE_EN_GOMA_JR_FREEZE;
use oot_game::collision_check::*;
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_ONCE, Anim, LimbDraw, SkelAnimeStd, draw_opa_pose};

/// `ACTOR_EN_BROB` (`actor_table.h`: 0x00B6).
pub const ACTOR_EN_BROB: i16 = 0x00B6;
/// `OBJECT_BROB` (`object_table.h`: 0x00AC).
pub const OBJECT: &str = "object_brob";
pub const OBJECT_BROB: i16 = 0x00AC;
/// `gBrobSkel` (`object_brob.xml`): a flex skeleton of 9 limbs (`jointTable[10]`).
pub const SKEL: &str = "gBrobSkel";
/// `gBrobCol`.
pub const COLLISION: &str = "gBrobCol";

/// The animations (`object_brob.xml`).
pub const ANIM_MOVE_UP: &str = "gBrobMoveUpAnim";
pub const ANIM_WOBBLE: &str = "gBrobWobbleAnim";
pub const ANIM_STUNNED: &str = "gBrobStunnedAnim";
pub const ANIM_SHOCK: &str = "gBrobShockAnim";

/// `En_Brob_Profile`: `ACTORCAT_ENEMY`, attention-enabled (cleared at init), hostile,
/// `OBJECT_BROB`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_BROB, name: "En_Brob", category: ACTORCAT_ENEMY, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE, object: OBJECT, schema: &SCHEMA };

/// `PARAMS_GET_U(params, 8, 8)`: full size (0) or half.
pub const SIZE: Field = Field::params("size", 8, 8, Kind::Range(0, 255), "0 full size (scale 0.01), any other half (0.005)");
/// `PARAMS_GET_U(params, 0, 8)`: its height.
pub const HEIGHT: Field = Field::params("height", 0, 8, Kind::Range(0, 255), "0xFF its own height; else scale.y times height / 30 (twice that at half size)");

/// En_Brob's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[SIZE, HEIGHT], flags: &[], todo: false, objects: &[], texts: &[] };

/// `NA_SE_EN_BROB_WAVE` (`enemybank_table.h`: 0x394A).
pub const NA_SE_EN_BROB_WAVE: u16 = 0x394A;

/// The limbs `EnBrob_PostLimbDraw` reads (1-based): the body's cylinder's and the top's.
pub const LIMB_BODY: usize = 3;
pub const LIMB_TOP: usize = 8;

/// `sCylinderInit`: `COL_MATERIAL_HIT0`; hurting 8 with an electric shock (`ATELEM_SFX_NONE`),
/// hit by Player's `0xFFCFFFFF` with an electric backlash, pushing as type 1; 8000 by 11000,
/// 5000 down (scaled at init).
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HIT0, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK1,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_ELECTRIC, damage: 0x08 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_ELECTRIC, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 8000, height: 11000, y_shift: -5000, pos: [0; 3] },
};

/// `sColChkInfoInit`: health 0, 60 by 120, immovable.
pub const COL_CHK_INFO_INIT: CollisionCheckInfoInit = CollisionCheckInfoInit { health: 0, cyl_radius: 60, cyl_height: 120, mass: MASS_IMMOVABLE };

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Idle,
    MoveUp,
    Wobble,
    Stunned,
    MoveDown,
    Shock,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::Idle => "EnBrob_Idle",
            Action::MoveUp => "EnBrob_MoveUp",
            Action::Wobble => "EnBrob_Wobble",
            Action::Stunned => "EnBrob_Stunned",
            Action::MoveDown => "EnBrob_MoveDown",
            Action::Shock => "EnBrob_Shock",
        }
    }
}

struct Anims {
    move_up: Anim,
    wobble: Anim,
    stunned: Anim,
    shock: Anim,
}

impl Anims {
    fn load(play: &PlayState) -> Option<Anims> {
        let a = play.assets.clone()?;
        let get = |s: &str| a.animation(OBJECT, s).map_err(|e| log::error!("En_Brob: {e:#}")).ok();
        Some(Anims { move_up: get(ANIM_MOVE_UP)?, wobble: get(ANIM_WOBBLE)?, stunned: get(ANIM_STUNNED)?, shock: get(ANIM_SHOCK)? })
    }
}

pub struct EnBrob {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: Option<Anims>,
    /// `actionFunc` (`NULL` until `EnBrob_SetupIdle` at init: `None`).
    pub action: Option<Action>,
    pub timer: i16,
    pub model_offset_y: i16,
    /// `colliders`: the body's (limb 3) and the top's (limb 8).
    pub colliders: [ColliderCylinder; 2],
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

/// `s16 -= f32` (`this->modelOffsetY -= 125.0f`): the sum a float, truncated to an `s16`.
fn add_f(v: i16, f: f32) -> i16 {
    ((v as f32) + f) as i32 as i16
}

impl EnBrob {
    /// `EnBrob_Init`: see the module's docs.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let anims = Anims::load(play);
        let skeleton = play.assets.clone().and_then(|a| a.skeleton(OBJECT, SKEL).map_err(|e| log::error!("En_Brob: {e:#}")).ok());
        // SkelAnime_InitFlex(.., &gBrobSkel, &gBrobMoveUpAnim, .., 10).
        let skel = SkelAnimeStd::init_flex(skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(9), anims.as_ref().map(|a| a.move_up.clone()));
        // DynaPolyActor_Init(&this->dyna, 0), gBrobCol (its transform set again below, with the
        // scale).
        let bg = match crate::obj_kibako2::load_collision(play, OBJECT, COLLISION) {
            Some(h) => play.col.dyna.set_bg_actor(h, source(&actor), 0),
            None => BG_ACTOR_MAX,
        };
        actor.col_chk_info.set_info(None, &COL_CHK_INFO_INIT);
        let height = HEIGHT.get(actor.params) as f32;
        if SIZE.get(actor.params) == 0 {
            actor.scale = Vec3::splat(0.01);
            // schema: params &= 0xFF, the height alone kept
            actor.params &= 0xFF;
            if HEIGHT.get(actor.params) != 0xFF {
                actor.scale.y *= height * (1.0 / 30.0);
            }
        } else {
            actor.scale = Vec3::splat(0.005);
            // schema: params &= 0xFF, the height alone kept
            actor.params &= 0xFF;
            if HEIGHT.get(actor.params) != 0xFF {
                actor.scale.y *= height * (2.0 / 30.0);
            }
        }
        let mut colliders = [ColliderCylinder::new(&CYLINDER_INIT), ColliderCylinder::new(&CYLINDER_INIT)];
        let s = actor.scale;
        // dim.radius *= scale.x, and the rest (s16 = f32: truncated).
        colliders[0].dim.radius = (colliders[0].dim.radius as f32 * s.x) as i32 as i16;
        colliders[0].dim.height = (s.y * 12000.0) as i32 as i16;
        colliders[0].dim.y_shift = 0;
        colliders[1].dim.radius = (colliders[1].dim.radius as f32 * s.x) as i32 as i16;
        colliders[1].dim.height = (colliders[1].dim.height as f32 * s.y) as i32 as i16;
        colliders[1].dim.y_shift = (colliders[1].dim.y_shift as f32 * s.y) as i32 as i16;
        actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        let mut this = EnBrob { actor, bg, skel, skeleton, anims, action: None, timer: 0, model_offset_y: 0, colliders };
        play.col.dyna.set_source(this.bg, source(&this.actor));
        this.setup_idle(play);
        Box::new(this)
    }

    fn anim(&self, f: impl Fn(&Anims) -> &Anim) -> Option<Anim> {
        self.anims.as_ref().map(|a| f(a).clone())
    }

    /// `Animation_Change(.., anim, speed, start, end, mode, morphFrames)` (the end the
    /// animation's last frame when `end` is `None`).
    fn change(&mut self, f: impl Fn(&Anims) -> &Anim, speed: f32, start: Option<f32>, end: Option<f32>, mode: u8, morph: f32) {
        if let Some(a) = self.anim(f) {
            let last = a.last_frame();
            self.skel.change(a, speed, start.unwrap_or(0.0), end.unwrap_or(last), mode, morph);
        }
    }

    /// `EnBrob_SetupIdle`: its collision on, 200 frames' wait after a stun (else none), the
    /// model at home.
    pub fn setup_idle(&mut self, play: &mut PlayState) {
        play.col.dyna.set_collision_disabled(self.bg, false);
        self.timer = if self.action == Some(Action::Stunned) { 200 } else { 0 };
        self.model_offset_y = 0;
        self.action = Some(Action::Idle);
    }

    /// `EnBrob_SetupMoveUp`: `gBrobMoveUpAnim` once, its collision off, the model 1000 up.
    pub fn setup_move_up(&mut self, play: &mut PlayState) {
        if let Some(a) = self.anim(|a| &a.move_up) {
            self.skel.play_once(a);
        }
        play.col.dyna.set_collision_disabled(self.bg, true);
        self.model_offset_y = 1000;
        self.action = Some(Action::MoveUp);
    }

    /// `EnBrob_SetupWobble`: `gBrobWobbleAnim` looped (a 5-frame morph), the model 8000 up, 1200
    /// frames.
    pub fn setup_wobble(&mut self) {
        self.change(|a| &a.wobble, 1.0, None, None, ANIMMODE_LOOP, -5.0);
        self.model_offset_y = 8000;
        self.timer = 1200;
        self.action = Some(Action::Wobble);
    }

    /// `EnBrob_SetupStunned`: `gBrobStunnedAnim` once (a 5-frame morph), the model 125 down,
    /// blue 80 frames, `NA_SE_EN_GOMA_JR_FREEZE`.
    pub fn setup_stunned(&mut self, play: &mut PlayState) {
        self.change(|a| &a.stunned, 1.0, None, None, ANIMMODE_ONCE, -5.0);
        self.model_offset_y = add_f(self.model_offset_y, -125.0);
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 255, COLORFILTER_BUFFLAG_OPA, 80);
        audio_play_actor_sfx2(play, NA_SE_EN_GOMA_JR_FREEZE);
        self.action = Some(Action::Stunned);
    }

    /// `EnBrob_SetupMoveDown`: `gBrobMoveUpAnim` backward from its last frame (a 5-frame morph),
    /// the model 8250 up.
    pub fn setup_move_down(&mut self) {
        let last = self.anim(|a| &a.move_up).map(|a| a.last_frame());
        self.change(|a| &a.move_up, -1.0, last, Some(0.0), ANIMMODE_ONCE, -5.0);
        self.model_offset_y = 8250;
        self.action = Some(Action::MoveDown);
    }

    /// `EnBrob_SetupShock`: `gBrobShockAnim` looped (a 5-frame morph), ten loops.
    pub fn setup_shock(&mut self) {
        self.change(|a| &a.shock, 1.0, None, None, ANIMMODE_LOOP, -5.0);
        self.timer = 10;
        self.action = Some(Action::Shock);
    }

    /// `EnBrob_Idle`: see the module's docs.
    fn idle(&mut self, play: &mut PlayState) {
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.timer == 0 {
            // DynaPolyActor_IsPlayerOnTop.
            if play.col.dyna.interact_flag(self.bg, DYNA_INTERACT_PLAYER_ON_TOP) {
                actor_set_player_knockback_large_no_damage(play, 5.0, self.actor.yaw_towards_player, 1.0);
                self.setup_move_up(play);
            } else if self.actor.xz_dist_to_player < 300.0 {
                self.setup_move_up(play);
            }
        } else if self.timer >= 81 {
            self.actor.color_filter_timer = 80;
        }
    }

    /// `EnBrob_MoveUp`: the model up 1000 a frame to frame 8, 250 to 12, down 250 after; done,
    /// wobbling.
    fn move_up(&mut self) {
        if self.skel.update() {
            self.setup_wobble();
        } else {
            let cur_frame = self.skel.cur_frame;
            if cur_frame < 8.0 {
                self.model_offset_y = add_f(self.model_offset_y, 1000.0);
            } else if cur_frame < 12.0 {
                self.model_offset_y = add_f(self.model_offset_y, 250.0);
            } else {
                self.model_offset_y = add_f(self.model_offset_y, -250.0);
            }
        }
    }

    /// `EnBrob_Wobble`: `NA_SE_EN_BROB_WAVE` at frames 6 and 15; after 1200 frames with Link over
    /// 500 away, sinking.
    fn wobble(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.skel.on_frame(6.0) || self.skel.on_frame(15.0) {
            audio_play_actor_sfx2(play, NA_SE_EN_BROB_WAVE);
        }
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.timer == 0 && self.actor.xz_dist_to_player > 500.0 {
            self.setup_move_down();
        }
    }

    /// `EnBrob_Stunned`: the animation (the model down 1250 a frame to frame 8), blue; then down.
    fn stunned(&mut self, play: &mut PlayState) {
        if self.skel.update() {
            self.setup_idle(play);
        } else if self.skel.cur_frame < 8.0 {
            self.model_offset_y = add_f(self.model_offset_y, -1250.0);
        }
        self.actor.color_filter_timer = 80;
    }

    /// `EnBrob_MoveDown`: the model down 1000 a frame to frame 8, 250 to 12, up 250 after (the
    /// animation's frame counting down); done, down.
    fn move_down(&mut self, play: &mut PlayState) {
        if self.skel.update() {
            self.setup_idle(play);
        } else {
            let cur_frame = self.skel.cur_frame;
            if cur_frame < 8.0 {
                self.model_offset_y = add_f(self.model_offset_y, -1000.0);
            } else if cur_frame < 12.0 {
                self.model_offset_y = add_f(self.model_offset_y, -250.0);
            } else {
                self.model_offset_y = add_f(self.model_offset_y, 250.0);
            }
        }
    }

    /// `EnBrob_Shock`: see the module's docs.
    fn shock(&mut self, play: &mut PlayState) {
        // primColor, envColor.
        const PRIM_COLOR: [u8; 4] = [255, 255, 255, 255];
        const ENV_COLOR: [u8; 4] = [200, 255, 255, 255];
        self.skel.update();
        if self.skel.on_frame(0.0) && self.timer != 0 {
            self.timer -= 1;
        }
        let s = self.actor.scale;
        let mut dist1 = if play.gameplay_frames % 2 != 0 { 0.0 } else { s.x * 5500.0 };
        let mut dist2 = s.x * 5500.0;
        let w = self.actor.world_pos;
        for i in 0..4 {
            let (x, z) = if i % 2 != 0 {
                (w.x + dist1, w.z + dist2)
            } else {
                let p = (w.x + dist2, w.z + dist1);
                dist1 = -dist1;
                dist2 = -dist2;
                p
            };
            let y = (((play.rand.zero_one() * 15000.0) + 1000.0) * s.y) + w.y;
            let scale = (s.y * 8000.0) as i32 as i16;
            let yaw = (play.rand.zero_one() * 65536.0) as i32 as i16;
            let pos = Vec3::new(x, y, z);
            play.with_ss(|ss| ss.lightning_spawn(pos, PRIM_COLOR, ENV_COLOR, scale, yaw, 4, 1));
        }
        if self.timer == 0 {
            self.setup_wobble();
        }
    }

    /// The pose and the cylinders' places `EnBrob_PostLimbDraw` takes from limbs 3 and 8 (the
    /// top's 7 up): `Actor_Draw`'s matrix, `modelOffsetY` up, the skeleton.
    pub fn limb_positions(&self) -> Option<(Vec3, Vec3)> {
        let skeleton = self.skeleton.as_ref()?;
        let model = actor_draw_matrix(&RenderState::of(&self.actor)) * Mat4::from_translation(Vec3::new(0.0, self.model_offset_y as f32, 0.0));
        let (mut body, mut top) = (Vec3::ZERO, Vec3::ZERO);
        draw_opa_pose(
            skeleton,
            &self.skel.joint_table,
            |_, _, _| LimbDraw::Default(Mat4::IDENTITY),
            |limb, m| {
                // Matrix_Get(&mtx): mtx.xw, mtx.yw, mtx.zw.
                let t = (model * m).w_axis;
                if limb == LIMB_BODY {
                    body = Vec3::new(t.x, t.y, t.z);
                } else if limb == LIMB_TOP {
                    top = Vec3::new(t.x, t.y, t.z);
                }
            },
        );
        Some((body, top))
    }
}

impl ActorImpl for EnBrob {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnBrob_Update`: a boomerang on either cylinder stuns it; its AT touching Link or a
    /// Kokiri Sword's slash shocks (rising, it throws Link off unless the hit was blocked); the
    /// action; up (not stunned) hurting, up and not rising hittable, up or stunned pushing.
    fn update(&mut self, play: &mut PlayState) {
        let ac_hits = [self.colliders[0].base.ac_flags & AC_HIT != 0, self.colliders[1].base.ac_flags & AC_HIT != 0];
        let hit_by = |c: &ColliderCylinder, hit: bool, dmg: u32| hit && c.info.ac_hit_elem.is_some_and(|h| h.at_dmg_info.dmg_flags & dmg != 0);
        if hit_by(&self.colliders[0], ac_hits[0], DMG_BOOMERANG) || hit_by(&self.colliders[1], ac_hits[1], DMG_BOOMERANG) {
            for c in &mut self.colliders {
                c.base.at_flags &= !(AT_HIT | AT_BOUNCED);
                c.base.ac_flags &= !AC_HIT;
            }
            self.setup_stunned(play);
        } else if self.colliders[0].base.at_flags & AT_HIT != 0
            || self.colliders[1].base.at_flags & AT_HIT != 0
            || hit_by(&self.colliders[0], ac_hits[0], DMG_SLASH_KOKIRI)
            || hit_by(&self.colliders[1], ac_hits[1], DMG_SLASH_KOKIRI)
        {
            if self.action == Some(Action::MoveUp) && self.colliders[0].base.at_flags & AT_BOUNCED == 0 && self.colliders[1].base.at_flags & AT_BOUNCED == 0 {
                actor_set_player_knockback_large_no_damage(play, 5.0, self.actor.yaw_towards_player, 1.0);
            } else if self.action != Some(Action::MoveUp) {
                self.setup_shock();
            }
            for c in &mut self.colliders {
                c.base.at_flags &= !(AT_HIT | AT_BOUNCED);
                c.base.ac_flags &= !AC_HIT;
            }
        }
        match self.action {
            Some(Action::Idle) => self.idle(play),
            Some(Action::MoveUp) => self.move_up(),
            Some(Action::Wobble) => self.wobble(play),
            Some(Action::Stunned) => self.stunned(play),
            Some(Action::MoveDown) => self.move_down(play),
            Some(Action::Shock) => self.shock(play),
            None => {}
        }
        if self.action != Some(Action::Idle) && self.action != Some(Action::MoveDown) {
            let [c0, c1] = &mut self.colliders;
            if self.action != Some(Action::Stunned) {
                play.collision_check_set_at(&self.actor, 0, c0);
                play.collision_check_set_at(&self.actor, 1, c1);
                if self.action != Some(Action::MoveUp) {
                    play.collision_check_set_ac(&self.actor, 0, c0);
                    play.collision_check_set_ac(&self.actor, 1, c1);
                }
            }
            play.collision_check_set_oc(&self.actor, 0, c0);
            play.collision_check_set_oc(&self.actor, 1, c1);
        }
    }

    /// `EnBrob_Destroy`: its collision taken out (`DynaPoly_DeleteBgActor`), the cylinders.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }

    /// `EnBrob_PostLimbDraw`'s writes: the body's cylinder at limb 3, the top's at limb 8, 7 up
    /// (truncated to `s16`).
    fn draw_update(&mut self, _play: &mut PlayState) {
        if let Some((body, top)) = self.limb_positions() {
            self.colliders[0].dim.pos = [body.x as i16, body.y as i16, body.z as i16];
            self.colliders[1].dim.pos = [top.x as i16, (top.y + 7.0) as i16, top.z as i16];
        }
    }

    fn render_state(&self) -> RenderState {
        let mut r = RenderState::of(&self.actor);
        r.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        r.values = vec![self.model_offset_y as f32];
        r
    }

    /// `EnBrob_Draw`: `gBrobSkel` (`SkelAnime_DrawFlexOpa`), `modelOffsetY` up.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(joints), Some(skeleton)) = (&rs.joints, &self.skeleton) else { return };
        let Some(&offset_y) = rs.values.first() else { return };
        let bones = draw_opa_pose(skeleton, &joints.rot, |_, _, _| LimbDraw::Default(Mat4::IDENTITY), |_, _| {});
        // Matrix_Translate(0.0f, this->modelOffsetY, 0.0f, MTXMODE_APPLY).
        let model = actor_draw_matrix(rs) * Mat4::from_translation(Vec3::new(0.0, offset_y, 0.0));
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::mesh(OBJECT, SKEL)), transform: model, bones, params: Default::default() });
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        self.colliders.get_mut(id as usize).map(ColliderMut::Cylinder)
    }
    fn dyna_bg_id(&self) -> Option<u16> {
        Some(self.bg)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
