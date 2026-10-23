//! `En_Tp` (`ovl_En_Tp/z_en_tp.c`): the Electric Tailpasaran of Jabu-Jabu's Belly (GAME-10
//! milestone 8): a hard head that rises out of the floor and flies at Link, trailing seven
//! glowing body segments, each its own `En_Tp`. `object_tp`'s `gTailpasaranHeadDL` and
//! `gTailpasaranBodyElemDL`, no skeleton.
//!
//! Master Quest's Jabu-Jabu (`bdan`) places three heads in layers 0 to 3, params 0xFFFF
//! (`ENTP_TYPE_HEAD`), turned (0, 0, 0): room 12's at (-551, -340, -5752), (-766, -340, -5754)
//! and (-662, -340, -5555).
//!
//! - **The head** (params < 0) spawns seven segments at its place (`ENTP_TYPE_BODYELEM`, 0.3
//!   big, each the last one's `child` and the next one's `parent`, `headInstance` the head; the
//!   third lockable and hostile); the head is 1.5 big, its sphere 8 (the segments' 4), its
//!   collider hard (`AC_HARD`: it takes no damage).
//! - **Waiting and rising** (`EnTp_Head_WaitPlayerAndRise`): pointing down (`shape.rot.x`
//!   -0x4000) at its place. Link within 200 across, for 60 frames it lifts its head (0x1F4 a
//!   frame), turns to him (0x5DC), rises to 30 over its place; touching him, or the 60 out, it
//!   **hovers** (`EnTp_Head_Hover`: 40 to 54 frames, speeding to 2.5, 85 over Link, back toward
//!   its place, bobbing, its red glow fading 15 a frame). Link further, it sinks back into its
//!   place, wobbling 6 across. Then it **attacks** (`EnTp_Head_Attack`: 200 frames, or till it
//!   touches him: 30 over Link, at 2.5, turning to him 0x2EE a frame, glowing red 15 more a
//!   frame), then **dives into the floor and comes back up** (`EnTp_Head_SinkIntoGroundAndReappear`:
//!   pitching 0x400 a frame round to straight down, bubbles where it meets the floor on odd
//!   frames (`EffectSsDtBubble`, white in grey); 60 under its place, it and its segments are put
//!   back at its place (they 80 under), and it comes up 1 a frame for 80 frames, the segments'
//!   wobble reset), and waits again. `NA_SE_EN_TAIL_FLY` while away from its place;
//!   `NA_SE_EN_TAIL_CRY` each turn of its spin (`shape.rot.z`, 0x800 a frame). It moves by its
//!   speed (`Actor_MoveXZGravity`), checking walls and the floor (not while diving), turning
//!   0x1F4 off a wall it's heading into.
//! - **A segment** (`EnTp_BodyElem`): with the head flying (`modeVar` set) it takes the place its
//!   parent had last frame (its glow off; under the head's place, not lockable); else 4 under
//!   its parent, swaying across the head's facing, `bodyElemDist` (6 down to 1.5) wide, with the
//!   head's count.
//! - **Hit** (`EnTp_CheckCollide`, health 1, `sDamageTable`, any part): each hurts 8 with an
//!   electric shock (`HIT_SPECIAL_EFFECT_ELECTRIC`) and shocks a sword back
//!   (`HIT_BACKLASH_ELECTRIC`); the head's AT is always on, a segment's only after a melee hit
//!   (`unk_158`, till Link's `PLAYER_STATE1_26`). A Deku Nut stuns the whole of it (80 frames,
//!   `freezeTimer`; blue, `NA_SE_EN_GOMA_JR_FREEZE`); a segment's last health (a stick, the
//!   boomerang, the hammer, a sword) kills it: the head **dies** (`EnTp_Dying`, 13 frames,
//!   `NA_SE_EN_TAIL_DEAD`, every segment dying; then two puffs (`EffectSsDeadDb`) and a drop
//!   from table 5), and each segment after its parent's gone, 2 frames on, leaves a **remnant**
//!   (`ENTP_TYPE_BODYELEMREMNANT`, half its size, drifting at random and fading 20 a frame).
//! - Every part not dying sparks every eighth frame (`EffectSsKiraKira`, small, in its red and
//!   blue).
//!
//! The whole overlay is ported. Its draw (`EnTp_Draw`): the head `gTailpasaranHeadDL` opaque
//! (`Gfx_SetupDL_25Opa`); a segment `gTailpasaranBodyElemDL` translucent, facing the camera
//! (`Matrix_ReplaceRotation`), `gTailpasaranBodyElemTex` on segment 8 and the colour (its red,
//! 0, 255, its alpha) as the primitive colour (the draw's own combiner is replaced by the
//! list's). Its sphere follows the draw (`Collider_UpdateSpheres`: the head's 8 ahead).
//! The head's own `headInstance` is never set (`NULL`); nothing reads it, since the head takes
//! no damage. `@bug (game)`: a Deku Nut's freeze sound for each part above the one hit is played
//! at the one hit (`Actor_PlaySfx(&this->actor, ..)` in the parents' loop): kept.
//!
//! Params: `type` whole: -1 the head (placed), 0 a segment (the head's spawn), 10 a remnant (a
//! dying segment's spawn); 11 and 12 are set as a segment and the head die.
//! Flags: none.

use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{cos_s, sin_s, smooth_step_to_f, smooth_step_to_s, vec3f_yaw};
use glam::Vec3;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ActorHandle, ActorImpl, ActorProfile, audio_play_actor_sfx2, audio_play_actor_sfx_at, math_cos_f};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::audio::sfx::{NA_SE_EN_GOMA_JR_FREEZE, SFX_FLAG, SfxPos};
use oot_game::collision_check::*;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::sys_matrix::MtxF;

use crate::en_bili::DROP_TABLE_5;

/// `ACTOR_EN_TP` (`actor_table.h`: 0x0035).
pub const ACTOR_EN_TP: i16 = 0x0035;
/// `OBJECT_TP` (`object_table.h`: 0x0022).
pub const OBJECT: &str = "object_tp";
pub const OBJECT_TP: i16 = 0x0022;
/// The lists (`object_tp.xml`).
pub const HEAD_DL: &str = "gTailpasaranHeadDL";
pub const BODY_ELEM_DL: &str = "gTailpasaranBodyElemDL";
pub const BODY_ELEM_TEX: &str = "gTailpasaranBodyElemTex";

/// `En_Tp_Profile`: `ACTORCAT_ENEMY`, no flags (the head sets its own); `OBJECT_TP`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_TP, name: "En_Tp", category: ACTORCAT_ENEMY, flags: 0, object: OBJECT, schema: &SCHEMA };

/// `params` (`EnTpType`).
pub const TYPE: Field = Field::whole(
    "type",
    Kind::Named(&[(-1, "head (placed)"), (0, "body segment (the head's)"), (10, "remnant (a dying segment's)"), (11, "body segment dying (set)"), (12, "head dying (set)")]),
    "the head spawns its seven segments; 11 and 12 are written as they die",
);

/// En_Tp's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[TYPE], flags: &[], todo: false, objects: &[], texts: &[] };

/// `EnTpType`.
pub const ENTP_TYPE_HEAD: i16 = -1;
pub const ENTP_TYPE_BODYELEM: i16 = 0;
pub const ENTP_TYPE_BODYELEMREMNANT: i16 = 10;
pub const ENTP_TYPE_BODYELEM_DYING: i16 = 11;
pub const ENTP_TYPE_HEAD_DYING: i16 = 12;

/// `EnTpAction` (`action`: compared by number).
pub const ENTP_ACTION_BODYELEMREMNANT: i32 = 0;
pub const ENTP_ACTION_DYING: i32 = 1;
pub const ENTP_ACTION_BODYELEM: i32 = 2;
pub const ENTP_ACTION_HEAD_WAIT_PLAYER_AND_RISE: i32 = 4;
pub const ENTP_ACTION_HEAD_ATTACK: i32 = 7;
pub const ENTP_ACTION_HEAD_8: i32 = 8;
pub const ENTP_ACTION_HEAD_SINK_INTO_GROUND_AND_REAPPEAR: i32 = 9;

/// `EnTpDamageReaction`.
pub const ENTP_DMG_REACT_NO_EFFECT: u8 = 0x0;
pub const ENTP_DMG_REACT_STUN: u8 = 0x1;
pub const ENTP_DMG_REACT_MELEE_BACKLASH: u8 = 0xE;
pub const ENTP_DMG_REACT_VULNERABLE: u8 = 0xF;

// The sounds (`include/tables/sfx/enemybank_table.h`).
/// `NA_SE_EN_TAIL_FLY` (0x3864).
pub const NA_SE_EN_TAIL_FLY: u16 = 0x3864;
/// `NA_SE_EN_TAIL_CRY` (0x3865).
pub const NA_SE_EN_TAIL_CRY: u16 = 0x3865;
/// `NA_SE_EN_TAIL_DEAD` (0x3866).
pub const NA_SE_EN_TAIL_DEAD: u16 = 0x3866;

/// `NAVI_ENEMY_TAILPASARAN` (`actor.h`: 0x06).
pub const NAVI_ENEMY_TAILPASARAN: u8 = 0x06;
/// `ATTENTION_RANGE_3` (`actor.h`).
pub const ATTENTION_RANGE_3: u8 = 3;
/// `PLAYER_STATE1_26` (`player.h`: 1 << 26).
pub const PLAYER_STATE1_26: u32 = 1 << 26;

/// `sJntSphElementsInit`: one sphere on "limb" 0, 4 round its centre, hurting 8 with an electric
/// shock, hit by Player's `0xFFCFFFFF` with an electric backlash.
pub const JNT_SPH_ELEMENTS_INIT: [ColliderJntSphElementInit; 1] = [ColliderJntSphElementInit {
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_ELECTRIC, damage: 0x08 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_ELECTRIC, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_NONE,
    },
    limb: 0,
    model_sphere: eng_collision::math3d::Sphere16 { center: [0, 0, 0], radius: 4 },
    scale: 100,
}];

/// `sJntSphInit`: `COL_MATERIAL_HIT1`, hurting enemies' AT, hit by Player.
pub const JNT_SPH_INIT: ColliderInit = ColliderInit { col_type: COL_MATERIAL_HIT1, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_JNTSPH };

/// `sDamageTable`.
pub static S_DAMAGE_TABLE: DamageTable = DamageTable {
    table: [
        dmg_entry(0, ENTP_DMG_REACT_STUN),            // Deku nut
        dmg_entry(2, ENTP_DMG_REACT_VULNERABLE),      // Deku stick
        dmg_entry(0, ENTP_DMG_REACT_NO_EFFECT),       // Slingshot
        dmg_entry(0, ENTP_DMG_REACT_NO_EFFECT),       // Explosive
        dmg_entry(1, ENTP_DMG_REACT_VULNERABLE),      // Boomerang
        dmg_entry(0, ENTP_DMG_REACT_NO_EFFECT),       // Normal arrow
        dmg_entry(2, ENTP_DMG_REACT_MELEE_BACKLASH),  // Hammer swing
        dmg_entry(0, ENTP_DMG_REACT_NO_EFFECT),       // Hookshot
        dmg_entry(1, ENTP_DMG_REACT_MELEE_BACKLASH),  // Kokiri sword
        dmg_entry(2, ENTP_DMG_REACT_MELEE_BACKLASH),  // Master sword
        dmg_entry(4, ENTP_DMG_REACT_MELEE_BACKLASH),  // Giant's Knife
        dmg_entry(0, ENTP_DMG_REACT_NO_EFFECT),       // Fire arrow
        dmg_entry(0, ENTP_DMG_REACT_NO_EFFECT),       // Ice arrow
        dmg_entry(0, ENTP_DMG_REACT_NO_EFFECT),       // Light arrow
        dmg_entry(0, ENTP_DMG_REACT_NO_EFFECT),       // Unk arrow 1
        dmg_entry(0, ENTP_DMG_REACT_NO_EFFECT),       // Unk arrow 2
        dmg_entry(0, ENTP_DMG_REACT_NO_EFFECT),       // Unk arrow 3
        dmg_entry(0, ENTP_DMG_REACT_NO_EFFECT),       // Fire magic
        dmg_entry(0, ENTP_DMG_REACT_NO_EFFECT),       // Ice magic
        dmg_entry(0, ENTP_DMG_REACT_NO_EFFECT),       // Light magic
        dmg_entry(0, ENTP_DMG_REACT_NO_EFFECT),       // Shield
        dmg_entry(0, ENTP_DMG_REACT_NO_EFFECT),       // Mirror Ray
        dmg_entry(1, ENTP_DMG_REACT_MELEE_BACKLASH),  // Kokiri spin
        dmg_entry(4, ENTP_DMG_REACT_MELEE_BACKLASH),  // Giant spin
        dmg_entry(2, ENTP_DMG_REACT_MELEE_BACKLASH),  // Master spin
        dmg_entry(2, ENTP_DMG_REACT_MELEE_BACKLASH),  // Kokiri jump
        dmg_entry(8, ENTP_DMG_REACT_MELEE_BACKLASH),  // Giant jump
        dmg_entry(4, ENTP_DMG_REACT_MELEE_BACKLASH),  // Master jump
        dmg_entry(0, ENTP_DMG_REACT_NO_EFFECT),       // Unknown 1
        dmg_entry(0, ENTP_DMG_REACT_NO_EFFECT),       // Unblockable
        dmg_entry(4, ENTP_DMG_REACT_MELEE_BACKLASH),  // Hammer jump
        dmg_entry(0, ENTP_DMG_REACT_NO_EFFECT),       // Unknown 2
    ],
};

/// The body segment's bake's segments: its texture, its colour (`gDPSetPrimColor(0, 0, ..)`, run
/// first, dynamic).
const SEG_TEX: u8 = 0x08;
const SEG_COLOR: u8 = 0x0E;
pub const BAKE_HEAD: &str = "En_Tp/head";
pub const BAKE_BODY_ELEM: &str = "En_Tp/body_elem";

/// `EnTp_Draw`'s meshes: the head, and a segment (its colour dynamic).
pub fn bakes() -> Vec<MeshBake> {
    vec![
        MeshBake { name: BAKE_HEAD.into(), object: OBJECT.into(), segments: vec![], prelude: vec![], body: BakeBody::DLists(vec![(OBJECT.into(), HEAD_DL.into())]) },
        MeshBake {
            name: BAKE_BODY_ELEM.into(),
            object: OBJECT.into(),
            segments: vec![(SEG_TEX, BakeSegment::Texture { file: OBJECT.into(), symbol: BODY_ELEM_TEX.into() }), (SEG_COLOR, BakeSegment::Dynamic(vec![(0xFA00_0000, 0x0000_FFFF), (0xDF00_0000, 0)]))],
            prelude: vec![SEG_COLOR],
            body: BakeBody::DLists(vec![(OBJECT.into(), BODY_ELEM_DL.into())]),
        },
    ]
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Func {
    BodyElem,
    HeadAttack,
    Dying,
    BodyElemRemnant,
    HeadHover,
    HeadWaitPlayerAndRise,
    HeadSinkIntoGroundAndReappear,
}

impl Func {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Func::BodyElem => "EnTp_BodyElem",
            Func::HeadAttack => "EnTp_Head_Attack",
            Func::Dying => "EnTp_Dying",
            Func::BodyElemRemnant => "EnTp_BodyElemRemnant",
            Func::HeadHover => "EnTp_Head_Hover",
            Func::HeadWaitPlayerAndRise => "EnTp_Head_WaitPlayerAndRise",
            Func::HeadSinkIntoGroundAndReappear => "EnTp_Head_SinkIntoGroundAndReappear",
        }
    }
}

pub struct EnTp {
    pub actor: Actor,
    /// `action` (`EnTpAction`).
    pub action: i32,
    /// `modeVar`: the head flying (1); a lockable segment (1); gone (2).
    pub mode_var: i32,
    pub func: Func,
    /// `unk_158`: the last hit's reaction (a melee hit keeps its AT on).
    pub unk_158: u8,
    pub timer: i16,
    pub timer2: i16,
    pub body_elem_alpha: i16,
    pub red_shine_amount: i16,
    pub spark_timer: i16,
    /// `unk_164`: always 0.
    pub unk_164: i16,
    /// `unk_168`: the bob's phase.
    pub unk_168: f32,
    pub body_elem_dist: f32,
    /// `unk_170`: the bob's extra height.
    pub unk_170: f32,
    pub collider: ColliderJntSph,
    /// `headInstance`: a segment's head (`NULL` on the head and on remnants). Set by the head's
    /// init in the C; here found up the parents at a segment's first update (the head has no
    /// handle during its own init).
    pub head_instance: Option<ActorHandle>,
}

/// `SFX_PLAY_AT_POS(&this->actor.projectedPos, sfx)` from the updating actor.
fn sfx_at_projected_pos(play: &mut PlayState, sfx: u16) {
    if let Some(me) = play.cur_actor {
        play.audio.play_sfx_at_pos(SfxPos::Actor(me), sfx);
    }
}

/// Player's `stateFlags1` (0 without him).
fn player_state1(play: &PlayState) -> u32 {
    play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map_or(0, |p| p.state_flags1())
}

/// Player's `world.pos`.
fn player_pos(play: &PlayState) -> Vec3 {
    play.player.and_then(|h| play.actors.actor(h)).map_or(Vec3::ZERO, |a| a.world_pos)
}

impl EnTp {
    /// `EnTp_Init`: see the module's docs.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: lockOnArrowOffset 10.
        actor.target_arrow_offset = 10.0;
        actor.target_mode = ATTENTION_RANGE_3;
        actor.col_chk_info.damage_table = Some(&S_DAMAGE_TABLE);
        actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 0.14);
        actor.col_chk_info.health = 1;
        let mut this = EnTp {
            actor,
            action: 0,
            mode_var: 0,
            func: Func::BodyElem,
            unk_158: 0,
            timer: 0,
            timer2: 0,
            body_elem_alpha: 255,
            red_shine_amount: 0,
            spark_timer: 0,
            unk_164: 0,
            unk_168: 0.0,
            body_elem_dist: 0.0,
            unk_170: 0.0,
            collider: ColliderJntSph::new(&JNT_SPH_INIT, &JNT_SPH_ELEMENTS_INIT),
            head_instance: None,
        };
        let ty = TYPE.get(this.actor.params);
        if ty < 0 {
            this.actor.navi_enemy_id = NAVI_ENEMY_TAILPASARAN;
            this.timer = 0;
            this.collider.base.ac_flags |= AC_HARD;
            this.collider.elements[0].dim.world_sphere.radius = 8;
            this.collider.elements[0].dim.model_sphere.radius = this.collider.elements[0].dim.world_sphere.radius;
            this.head_setup_wait_player_and_rise();
            this.actor.focus_pos = this.actor.world_pos;
            this.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED;
            this.actor.scale = Vec3::splat(1.5);
            let mut prev_child: Option<ActorHandle> = None;
            for i in 0..7i32 {
                let Ok(h) = play.actor_spawn(ACTOR_EN_TP, this.actor.world_pos, [0, 0, 0], ENTP_TYPE_BODYELEM) else { continue };
                // prevChild->actor.child = newChild; newChild->actor.parent = prevChild.
                match prev_child {
                    None => {
                        this.actor.child = Some(h);
                        play.adopt_from_init(h);
                    }
                    Some(p) => {
                        if let Some(prev) = play.actors.actor_mut(p) {
                            prev.child = Some(h);
                        }
                        if let Some(new) = play.actors.actor_mut(h) {
                            new.parent = Some(p);
                        }
                    }
                }
                if let Some(c) = play.actors.downcast_mut::<EnTp>(h) {
                    c.spark_timer = (i + 1) as i16;
                    c.actor.scale = Vec3::splat(0.3);
                    if i == 2 {
                        c.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED;
                        c.mode_var = 1;
                    }
                    c.timer2 = (i * -5) as i16;
                    c.timer = (i * -5) as i16;
                    c.body_elem_dist = 6.0 - (i as f32 * 0.75);
                }
                prev_child = Some(h);
            }
        } else if ty == ENTP_TYPE_BODYELEM {
            this.setup_body_elem();
        } else {
            // ENTP_TYPE_BODYELEMREMNANT (the only other params it's spawned with).
            this.setup_body_elem_remnant(play);
        }
        Box::new(this)
    }

    /// The segment's head (`headInstance`): the top of its parents, found once.
    pub fn head_instance(&mut self, play: &PlayState) -> Option<ActorHandle> {
        if self.head_instance.is_none() && TYPE.get(self.actor.params) == ENTP_TYPE_BODYELEM {
            let mut h = self.actor.parent;
            for _ in 0..16 {
                let Some(a) = h.and_then(|h| play.actors.actor(h)) else { break };
                if a.parent.is_none() {
                    break;
                }
                h = a.parent;
            }
            self.head_instance = h;
        }
        self.head_instance
    }

    /// `EnTp_SetupBodyElem`.
    pub fn setup_body_elem(&mut self) {
        self.action = ENTP_ACTION_BODYELEM;
        self.func = Func::BodyElem;
    }

    /// `EnTp_BodyElem`: see the module's docs.
    fn body_elem(&mut self, play: &mut PlayState) {
        if TYPE.get(self.actor.params) == ENTP_TYPE_BODYELEM_DYING {
            self.action = ENTP_ACTION_DYING;
            if self.actor.parent.is_none() {
                self.setup_dying(play, play.cur_actor, None);
            }
            return;
        }
        if self.mode_var != 0 {
            self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
        }
        let Some(head) = self.head_instance(play) else { return };
        let Some(h) = play.actors.downcast::<EnTp>(head) else { return };
        let (head_mode, head_home_y, head_yaw, head_timer2) = (h.mode_var, h.actor.home_pos.y, h.actor.shape_rot.y, h.timer2);
        let Some(parent) = self.actor.parent.and_then(|p| play.actors.actor(p)) else { return };
        let (parent_prev, parent_y) = (parent.prev_pos, parent.world_pos.y);
        if head_mode != 0 {
            // speed = redShineAmount = velocity.y = unk_168 = 0.
            self.unk_168 = 0.0;
            self.actor.velocity.y = 0.0;
            self.red_shine_amount = 0;
            self.actor.speed_xz = 0.0;
            if self.actor.world_pos.y < head_home_y {
                self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
            }
            self.actor.world_pos = parent_prev;
        } else {
            smooth_step_to_f(&mut self.actor.world_pos.y, parent_y - 4.0, 1.0, 1.0, 0.0);
            let sp36 = head_yaw.wrapping_add(0x4000);
            let a = ((head_timer2 as i32 + self.timer as i32) * 0x7D0) as i16;
            self.actor.world_pos.x = (sin_s(a) * (sin_s(sp36) * self.body_elem_dist)) + self.actor.home_pos.x;
            self.actor.world_pos.z = (sin_s(a) * (cos_s(sp36) * self.body_elem_dist)) + self.actor.home_pos.z;
        }
    }

    /// `EnTp_Head_SetupAttack`: 200 frames.
    pub fn head_setup_attack(&mut self) {
        self.action = ENTP_ACTION_HEAD_ATTACK;
        self.timer = 200;
        self.func = Func::HeadAttack;
    }

    /// `EnTp_Head_Attack`: see the module's docs.
    fn head_attack(&mut self, play: &mut PlayState) {
        let player = play.player;
        smooth_step_to_f(&mut self.actor.world_pos.y, player_pos(play).y + 30.0, 1.0, 0.5, 0.0);
        sfx_at_projected_pos(play, NA_SE_EN_TAIL_FLY - SFX_FLAG);
        if self.collider.base.at_flags & AT_HIT != 0 {
            self.collider.base.at_flags &= !AT_HIT;
            if player.is_some() && self.collider.base.at == player {
                self.timer = 1;
            }
        }
        if self.red_shine_amount < 255 {
            self.red_shine_amount += 15;
        }
        if math_cos_f(self.unk_168) == 0.0 {
            self.unk_170 = 2.0 * play.rand.zero_one();
        }
        self.actor.world_pos.y += math_cos_f(self.unk_168) * (2.0 + self.unk_170);
        self.unk_168 += 0.2;
        smooth_step_to_f(&mut self.actor.speed_xz, 2.5, 0.1, 0.2, 0.0);
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            smooth_step_to_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player, 1, 0x2EE, 0);
            self.actor.shape_rot.y = self.actor.world_rot.y;
        } else {
            self.head_setup_sink_into_ground_and_reappear();
        }
    }

    /// `EnTp_SetupDying`: 2 frames; the head 13, every segment dying (`params`, no health),
    /// `NA_SE_EN_TAIL_DEAD` at it (`me`, its handle). `cur`: the updating segment's handle and
    /// its `child` when the head's set dying from its update (it's out of the arena then, and
    /// in the head's chain); returns whether it's to be written dying itself.
    pub fn setup_dying(&mut self, play: &mut PlayState, me: Option<ActorHandle>, cur: Option<(ActorHandle, Option<ActorHandle>)>) -> bool {
        let mut cur_marked = false;
        self.timer = 2;
        if TYPE.get(self.actor.params) < 0 {
            let mut child = self.actor.child;
            while let Some(c) = child {
                if let Some((h, cur_child)) = cur
                    && h == c
                {
                    cur_marked = true;
                    child = cur_child;
                    continue;
                }
                let Some(a) = play.actors.actor_mut(c) else { break };
                // schema: another actor's params written whole (ENTP_TYPE_BODYELEM_DYING)
                a.params = ENTP_TYPE_BODYELEM_DYING;
                a.col_chk_info.health = 0;
                child = a.child;
            }
            self.timer = 13;
            if let Some(me) = me {
                audio_play_actor_sfx_at(play, me, NA_SE_EN_TAIL_DEAD);
            }
        }
        self.action = ENTP_ACTION_DYING;
        self.func = Func::Dying;
        cur_marked
    }

    /// `EnTp_Dying`: see the module's docs.
    fn dying(&mut self, play: &mut PlayState) {
        let sp78 = Vec3::new(0.0, 0.5, 0.0);
        self.timer = self.timer.wrapping_sub(1);
        if self.timer > 0 {
            return;
        }
        if TYPE.get(self.actor.params) == ENTP_TYPE_HEAD_DYING {
            for _ in 0..2 {
                let w = self.actor.world_pos;
                let x = ((play.rand.zero_one() - 0.5) * 15.0) + w.x;
                let z = ((play.rand.zero_one() - 0.5) * 15.0) + w.z;
                let y = ((play.rand.zero_one() - 0.5) * 5.0) + w.y;
                let pos = Vec3::new(x, y, z);
                play.with_ss(|s| s.dead_db_spawn(pos, sp78, sp78, 100, 0, [255, 255, 255, 255], [0, 0, 255], 1, 9, 1));
            }
            crate::en_item00::item_drop_collectible_random(play, Some(&self.actor), self.actor.world_pos, DROP_TABLE_5);
        } else {
            // ENTP_TYPE_BODYELEM_DYING; OOT_VERSION >= NTSC_1_1: one remnant (two before).
            if let Ok(h) = play.actor_spawn(ACTOR_EN_TP, self.actor.world_pos, [0, 0, 0], ENTP_TYPE_BODYELEMREMNANT)
                && let Some(r) = play.actors.downcast_mut::<EnTp>(h)
            {
                r.actor.scale = Vec3::splat(self.actor.scale.z * 0.5);
                r.red_shine_amount = self.red_shine_amount;
            }
        }
        if let Some(c) = self.actor.child.and_then(|c| play.actors.actor_mut(c)) {
            c.parent = None;
            // schema: another actor's params written whole (ENTP_TYPE_BODYELEM_DYING)
            c.params = ENTP_TYPE_BODYELEM_DYING;
            c.col_chk_info.health = 0;
        }
        self.mode_var = 2;
        self.actor.kill();
    }

    /// `EnTp_SetupBodyElemRemnant`: up to 2.5 off its place and drifting up to 0.75 a frame each
    /// way, at random; no lock-on.
    fn setup_body_elem_remnant(&mut self, play: &mut PlayState) {
        self.action = ENTP_ACTION_BODYELEMREMNANT;
        let r = &mut play.rand;
        self.actor.world_pos.x += (r.zero_one() - 0.5) * 5.0;
        self.actor.world_pos.y += (r.zero_one() - 0.5) * 5.0;
        self.actor.world_pos.z += (r.zero_one() - 0.5) * 5.0;
        self.actor.velocity.x = (r.zero_one() - 0.5) * 1.5;
        self.actor.velocity.y = (r.zero_one() - 0.5) * 1.5;
        self.actor.velocity.z = (r.zero_one() - 0.5) * 1.5;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.func = Func::BodyElemRemnant;
    }

    /// `EnTp_BodyElemRemnant`: drifting, fading 20 a frame; under 20, gone.
    fn body_elem_remnant(&mut self) {
        self.actor.update_pos();
        self.body_elem_alpha -= 20;
        if self.body_elem_alpha < 20 {
            self.body_elem_alpha = 0;
            self.actor.kill();
        }
    }

    /// `EnTp_Head_SetupHover`: 40 to 54 frames.
    pub fn head_setup_hover(&mut self, play: &mut PlayState) {
        self.timer = ((play.rand.zero_one() * 15.0) + 40.0) as i16;
        self.action = ENTP_ACTION_HEAD_8;
        self.func = Func::HeadHover;
    }

    /// `EnTp_Head_Hover`: see the module's docs.
    fn head_hover(&mut self, play: &mut PlayState) {
        let player = play.player;
        smooth_step_to_f(&mut self.actor.speed_xz, 2.5, 0.1, 0.2, 0.0);
        // bodyElemDist is always 0 here.
        smooth_step_to_f(&mut self.actor.world_pos.y, player_pos(play).y + 85.0 + self.body_elem_dist, 1.0, self.actor.speed_xz * 0.25, 0.0);
        sfx_at_projected_pos(play, NA_SE_EN_TAIL_FLY - SFX_FLAG);
        if self.collider.base.at_flags & AT_HIT != 0 {
            self.collider.base.at_flags &= !AT_HIT;
            if player.is_some() && self.collider.base.at == player {
                self.timer2 = 1;
            }
        }
        if self.red_shine_amount != 0 {
            self.red_shine_amount -= 15;
        }
        if math_cos_f(self.unk_168) == 0.0 {
            self.unk_170 = play.rand.zero_one() * 4.0;
        }
        self.actor.world_pos.y += math_cos_f(self.unk_168) * ((self.actor.speed_xz * 0.25) + self.unk_170);
        // unk_164 is always 0.
        self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(self.unk_164);
        self.unk_168 += 0.2;
        if self.timer != 0 {
            self.timer -= 1;
        }
        smooth_step_to_s(&mut self.actor.world_rot.y, vec3f_yaw(self.actor.world_pos, self.actor.home_pos), 1, 0x2EE, 0);
        if self.timer == 0 {
            self.head_setup_attack();
        }
        self.actor.shape_rot.y = self.actor.world_rot.y;
    }

    /// `EnTp_Head_SetupWaitPlayerAndRise`: pointing down, 60 frames, still.
    pub fn head_setup_wait_player_and_rise(&mut self) {
        self.action = ENTP_ACTION_HEAD_WAIT_PLAYER_AND_RISE;
        self.mode_var = 0;
        self.actor.shape_rot.x = -0x4000;
        self.timer = 60;
        self.timer2 = 0;
        self.actor.speed_xz = 0.0;
        self.func = Func::HeadWaitPlayerAndRise;
    }

    /// `EnTp_Head_WaitPlayerAndRise`: see the module's docs.
    fn head_wait_player_and_rise(&mut self, play: &mut PlayState) {
        let player = play.player;
        let ppos = player_pos(play);
        self.timer2 = self.timer2.wrapping_sub(1);
        if self.actor.xz_dist_to_player < 200.0 {
            if self.collider.base.at_flags & AT_HIT != 0 {
                self.collider.base.at_flags &= !AT_HIT;
                if player.is_some() && self.collider.base.at == player {
                    self.timer = 0;
                }
            }
            if self.timer != 0 {
                self.timer -= 1;
                smooth_step_to_s(&mut self.actor.shape_rot.x, 0, 1, 0x1F4, 0);
                smooth_step_to_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player, 1, 0x5DC, 0);
                let sp32 = vec3f_yaw(self.actor.home_pos, ppos).wrapping_add(0x4000);
                smooth_step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y + 30.0, 0.3, 1.0, 0.3);
                // bodyElemDist is always 0 here.
                let a = (self.timer2 as i32 * 0x7D0) as i16;
                self.actor.world_pos.x = (sin_s(a) * (sin_s(sp32) * self.body_elem_dist)) + self.actor.home_pos.x;
                self.actor.world_pos.z = (sin_s(a) * (cos_s(sp32) * self.body_elem_dist)) + self.actor.home_pos.z;
            } else {
                self.actor.shape_rot.x = 0;
                self.mode_var = 1;
                self.head_setup_hover(play);
            }
        } else {
            smooth_step_to_s(&mut self.actor.shape_rot.x, -0x4000, 1, 0x1F4, 0);
            if smooth_step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y, 0.3, 1.5, 0.3) == 0.0 {
                self.timer = 60;
            } else {
                let sp32 = vec3f_yaw(self.actor.home_pos, ppos);
                let a = (self.timer2 as i32 * 0x7D0) as i16;
                self.actor.world_pos.x = (sin_s(a) * (sin_s(sp32) * 6.0)) + self.actor.home_pos.x;
                self.actor.world_pos.z = (sin_s(a) * (cos_s(sp32) * 6.0)) + self.actor.home_pos.z;
            }
        }
        self.actor.shape_rot.y = self.actor.world_rot.y;
        if self.actor.world_pos.y != self.actor.home_pos.y {
            sfx_at_projected_pos(play, NA_SE_EN_TAIL_FLY - SFX_FLAG);
        }
    }

    /// `EnTp_Head_SetupSinkIntoGroundAndReappear`.
    pub fn head_setup_sink_into_ground_and_reappear(&mut self) {
        self.action = ENTP_ACTION_HEAD_SINK_INTO_GROUND_AND_REAPPEAR;
        self.timer = 0;
        self.func = Func::HeadSinkIntoGroundAndReappear;
    }

    /// `EnTp_Head_SinkIntoGroundAndReappear`: see the module's docs.
    fn head_sink_into_ground_and_reappear(&mut self, play: &mut PlayState) {
        // sBubbleAccel, sBubblePrimColor, sBubbleEnvColor.
        const BUBBLE_ACCEL: Vec3 = Vec3::new(0.0, -0.5, 0.0);
        const BUBBLE_PRIM_COLOR: [u8; 4] = [255, 255, 255, 255];
        const BUBBLE_ENV_COLOR: [u8; 4] = [150, 150, 150, 0];
        let mut is_inside_ground = false;
        self.timer2 = self.timer2.wrapping_sub(1);
        if self.timer != 0 || (self.actor.home_pos.y - self.actor.world_pos.y) > 60.0 {
            self.timer = self.timer.wrapping_sub(1);
            if self.timer == 0 {
                // 4. Finished reappearing.
                self.head_setup_wait_player_and_rise();
                let mut child = self.actor.child;
                while let Some(c) = child.and_then(|c| play.actors.downcast_mut::<EnTp>(c)) {
                    c.timer2 = c.timer;
                    child = c.actor.child;
                }
            } else {
                if self.actor.shape_rot.x != -0x4000 {
                    // 2. The head and every segment put back at its place.
                    self.timer = 80;
                    self.actor.velocity.y = 0.0;
                    self.actor.speed_xz = 0.0;
                    self.actor.world_pos = self.actor.home_pos;
                    self.actor.shape_rot.x = -0x4000;
                    let home = self.actor.home_pos;
                    let mut child = self.actor.child;
                    while let Some(c) = child.and_then(|c| play.actors.actor_mut(c)) {
                        c.velocity.y = 0.0;
                        c.speed_xz = 0.0;
                        c.world_pos = home;
                        c.world_pos.y = home.y - 80.0;
                        child = c.child;
                    }
                }
                // 3. Rising back up through the floor.
                self.actor.world_pos.y = self.actor.home_pos.y - self.timer as f32;
            }
        } else {
            // 1. Plunging into the floor.
            if self.actor.shape_rot.x != 0x4000 {
                self.actor.shape_rot.x = self.actor.shape_rot.x.wrapping_sub(0x400);
            }
            if self.red_shine_amount != 0 {
                self.red_shine_amount -= 15;
            }
            self.actor.speed_xz = 2.0 * cos_s(self.actor.shape_rot.x);
            self.actor.velocity.y = sin_s(self.actor.shape_rot.x) * -2.0;
            if (self.actor.world_pos.y - self.actor.floor_height) < 20.0 {
                is_inside_ground = true;
            }
            if self.actor.world_pos.y != self.actor.home_pos.y {
                sfx_at_projected_pos(play, NA_SE_EN_TAIL_FLY - SFX_FLAG);
            }
            if is_inside_ground && play.gameplay_frames & 1 != 0 {
                let mut bubble_pos = self.actor.world_pos;
                bubble_pos.y = self.actor.floor_height;
                let vx = play.rand.centered_float(5.0);
                let vy = (play.rand.zero_one() * 3.5) + 1.5;
                let vz = play.rand.centered_float(5.0);
                let scale = play.rand.s16_offset(100, 50);
                play.with_ss(|s| s.dt_bubble_spawn_custom_color(bubble_pos, Vec3::new(vx, vy, vz), BUBBLE_ACCEL, BUBBLE_PRIM_COLOR, BUBBLE_ENV_COLOR, scale, 20, 0));
            }
        }
    }

    /// `EnTp_CheckCollide`: see the module's docs.
    fn check_collide(&mut self, play: &mut PlayState) {
        if self.collider.base.ac_flags & AC_HIT == 0 || self.action < ENTP_ACTION_BODYELEM {
            return;
        }
        let is_head_instance = TYPE.get(self.actor.params) < 0;
        let mut apply_stun = false;
        self.collider.base.ac_flags &= !AC_HIT;
        self.actor.set_drop_flag_jnt_sph(&self.collider, true);
        let reaction = self.actor.col_chk_info.damage_reaction;
        self.unk_158 = reaction;
        if reaction == ENTP_DMG_REACT_NO_EFFECT {
            return;
        }
        if reaction == ENTP_DMG_REACT_STUN {
            apply_stun = true;
        }
        if !is_head_instance {
            self.actor.apply_damage();
        }
        let buf = if is_head_instance { COLORFILTER_BUFFLAG_OPA } else { COLORFILTER_BUFFLAG_XLU };
        if self.actor.col_chk_info.health == 0 {
            self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
            let Some(head) = self.head_instance(play) else { return };
            // schema: another actor's params read whole (the head's type)
            let head_is_head = play.actors.actor(head).is_some_and(|a| TYPE.get(a.params) < 0);
            // The head isn't the updating actor (it takes no damage): out of the arena for its
            // EnTp_SetupDying, which writes this one too.
            if head_is_head && let Some(mut boxed) = play.actors.take(head) {
                let cur = play.cur_actor.map(|h| (h, self.actor.child));
                let mut marked = false;
                if let Some(h) = boxed.as_any_mut().downcast_mut::<EnTp>() {
                    marked = h.setup_dying(play, Some(head), cur);
                    h.unk_158 = reaction;
                    // schema: another actor's params written whole (ENTP_TYPE_HEAD_DYING)
                    h.actor.params = ENTP_TYPE_HEAD_DYING;
                }
                play.actors.put_back(head, boxed);
                if marked {
                    // schema: params written whole (ENTP_TYPE_BODYELEM_DYING, by the head's walk)
                    self.actor.params = ENTP_TYPE_BODYELEM_DYING;
                    self.actor.col_chk_info.health = 0;
                }
            }
            return;
        }
        if apply_stun {
            self.actor.freeze_timer = 80;
            audio_play_actor_sfx2(play, NA_SE_EN_GOMA_JR_FREEZE);
            self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 255, buf, 80);
        }
        let mut parent = self.actor.parent;
        while let Some(p) = parent {
            let Some(pt) = play.actors.downcast_mut::<EnTp>(p) else { break };
            pt.collider.base.ac_flags &= !AC_HIT;
            if apply_stun {
                pt.actor.freeze_timer = 80;
                pt.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 255, buf, 80);
            }
            parent = pt.actor.parent;
            if apply_stun {
                // @bug (game): Actor_PlaySfx(&this->actor, ..): at this one, once for each parent.
                audio_play_actor_sfx2(play, NA_SE_EN_GOMA_JR_FREEZE);
            }
        }
        let mut child = self.actor.child;
        while let Some(c) = child {
            let Some(ct) = play.actors.downcast_mut::<EnTp>(c) else { break };
            ct.collider.base.ac_flags &= !AC_HIT;
            if apply_stun {
                ct.actor.freeze_timer = 80;
                ct.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 255, buf, 80);
            }
            child = ct.actor.child;
        }
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.func {
            Func::BodyElem => self.body_elem(play),
            Func::HeadAttack => self.head_attack(play),
            Func::Dying => self.dying(play),
            Func::BodyElemRemnant => self.body_elem_remnant(),
            Func::HeadHover => self.head_hover(play),
            Func::HeadWaitPlayerAndRise => self.head_wait_player_and_rise(play),
            Func::HeadSinkIntoGroundAndReappear => self.head_sink_into_ground_and_reappear(play),
        }
    }

    /// Drawn as the head (`gTailpasaranHeadDL`), not a segment.
    pub fn drawn_as_head(&self) -> bool {
        let ty = TYPE.get(self.actor.params);
        ty < 0 || ty == ENTP_TYPE_HEAD_DYING
    }

    /// `Actor_Draw`'s matrix.
    fn draw_mtx(pos: Vec3, rot: [i16; 3], scale: Vec3, y_offset: f32) -> MtxF {
        let mut m = MtxF::set_translate_rotate_yxz(pos.x, pos.y + y_offset * scale.y, pos.z, rot);
        m.scale(scale.x, scale.y, scale.z);
        m
    }
}

impl ActorImpl for EnTp {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnTp_Update`: see the module's docs.
    fn update(&mut self, play: &mut PlayState) {
        if player_state1(play) & PLAYER_STATE1_26 != 0 {
            self.unk_158 = ENTP_DMG_REACT_NO_EFFECT;
        }
        if self.actor.col_chk_info.health != 0 {
            self.check_collide(play);
        }
        self.run_action(play);
        let ty = TYPE.get(self.actor.params);
        if ty < 0 {
            self.actor.move_forward();
            if self.action != ENTP_ACTION_HEAD_SINK_INTO_GROUND_AND_REAPPEAR {
                self.actor.update_bg_check_info(&play.col, 0.0, 15.0, 10.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
            }
            if self.actor.speed_xz != 0.0 && self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
                let v = self.actor.wall_yaw.wrapping_sub(self.actor.world_rot.y);
                if (v as i32).abs() > 0x4000 {
                    if v >= 0 {
                        self.actor.world_rot.y = self.actor.world_rot.y.wrapping_sub(0x1F4);
                    } else {
                        self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(0x1F4);
                    }
                    self.actor.shape_rot.y = self.actor.world_rot.y;
                }
            }
            self.actor.shape_rot.z = self.actor.shape_rot.z.wrapping_add(0x800);
            if self.actor.shape_rot.z == 0 {
                sfx_at_projected_pos(play, NA_SE_EN_TAIL_CRY);
            }
            if self.action >= ENTP_ACTION_BODYELEM {
                play.collision_check_set_at(&self.actor, 0, &mut self.collider);
            }
        }
        if ty != ENTP_TYPE_BODYELEM_DYING {
            self.spark_timer = self.spark_timer.wrapping_sub(1) & 7;
        }
        self.actor.focus_pos = self.actor.world_pos;
        if self.unk_158 == ENTP_DMG_REACT_MELEE_BACKLASH {
            play.collision_check_set_at(&self.actor, 0, &mut self.collider);
        }
        if self.spark_timer & 7 == 0 {
            // sparkVel, sparkAccel (against its velocity), sparkPrimColor (its red), sparkEnvColor.
            let vel = Vec3::ZERO;
            let accel = Vec3::new(-self.actor.velocity.x * 0.25, -self.actor.velocity.y * 0.25, -self.actor.velocity.z * 0.25);
            let prim = [self.red_shine_amount as u8, 0, 255, 255];
            let env = [0, 0, 0, 0];
            let w = self.actor.world_pos;
            let x = ((play.rand.zero_one() - 0.5) * 25.0) + w.x;
            let y = ((play.rand.zero_one() - 0.5) * 20.0) + w.y;
            let z = ((play.rand.zero_one() - 0.5) * 25.0) + w.z;
            let pos = Vec3::new(x, y, z);
            play.with_ss(|s| s.kirakira_spawn_small(pos, vel, accel, prim, env));
        }
        if self.action >= ENTP_ACTION_BODYELEM && self.actor.col_chk_info.health != 0 {
            play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
        }
    }

    /// `EnTp_Destroy`: the collider (nothing to free).
    fn destroy(&mut self, _play: &mut PlayState) {}

    /// `EnTp_Draw`'s write: the sphere under the draw's matrix (`Collider_UpdateSpheres(0, ..)`),
    /// for the head, a segment and a dying segment: the head's 8 ahead (`Matrix_Translate(0, 0,
    /// 8)`), a segment's at its place (`Matrix_ReplaceRotation` keeps the translation).
    fn draw_update(&mut self, _play: &mut PlayState) {
        let ty = TYPE.get(self.actor.params);
        if ty <= 0 || ty == ENTP_TYPE_BODYELEM_DYING {
            let a = &self.actor;
            let mut m = Self::draw_mtx(a.world_pos, [a.shape_rot.x, a.shape_rot.y, a.shape_rot.z], a.scale, a.shape_y_offset);
            if self.drawn_as_head() && self.mode_var != 2 {
                m.translate(0.0, 0.0, 8.0);
            }
            self.collider.update_spheres(0, &m.to_mat4());
        }
    }

    fn render_state(&self) -> RenderState {
        let mut r = RenderState::of(&self.actor);
        r.switches = vec![(self.mode_var != 2) as u32, self.drawn_as_head() as u32, self.red_shine_amount as u16 as u32, self.body_elem_alpha as u16 as u32];
        r
    }

    /// `EnTp_Draw`: see the module's docs.
    fn draw(&self, rs: &RenderState, _play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        let [shown, head, red, alpha] = rs.switches.as_slice() else { return };
        if *shown == 0 {
            return;
        }
        let model = Self::draw_mtx(rs.pos, rs.rot, rs.scale, rs.y_offset).to_mat4();
        if *head != 0 {
            out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(BAKE_HEAD)), model));
        } else {
            // gDPSetPrimColor(0, 0, redShineAmount, 0, 255, bodyElemAlpha).
            let mut sv = SegmentValues::default();
            sv.prim[SEG_COLOR as usize] = Some([*red as u8, 0, 255, *alpha as u8]);
            let m = oot_game::draw::replace_rotation(model, view.billboard);
            out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE_BODY_ELEM)), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
        }
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0).then_some(ColliderMut::JntSph(&mut self.collider))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
