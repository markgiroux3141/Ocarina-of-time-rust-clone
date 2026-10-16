//! `En_Ssh` (`ovl_En_Ssh/z_en_ssh.c`): the House of Skulltula's family, cursed: Skulltula-men
//! hanging from the ceiling on their threads, the father (params 0, `ENSSH_FATHER`) bigger and with
//! his own face and legs, the five children (1 to 5). Each is cured (gone, its `En_Sth` there in its
//! place) once Link holds enough Gold Skulltula tokens: a child at `params × 10`, the father at 100.
//!
//! Much of it is the Skulltula's (`En_St`), but it never takes damage:
//! - **On the ceiling** (`EnSsh_Wait`): bobbing (`EnSsh_Bob`: up and down by 0.5 as
//!   `play->state.frames & 8` says) until Link is within 160 across, below it (up to 400) and not
//!   below its floor (`EnSsh_IsCloseToLink`); then it **drops** (`EnSsh_Drop`) at 10 a frame
//!   (`NA_SE_EN_STALTU_DOWN` every 3 frames), and two frames from its hover height (20 for a child,
//!   30 for the father) a white shockwave as it **lands** (`EnSsh_Land`), rising to it.
//! - **Down** (`EnSsh_Idle`): bobbing, laughing every 64 frames, turning to face Link
//!   (`EnSsh_Turn`), and offering to talk within 100 (`Actor_OfferTalk`): a child 0x22; the
//!   father 0x26 (0x27 once heard: `INFTABLE_196`) under 10 tokens, 0x25 (0x24: `INFTABLE_197`)
//!   from 10, 0x29 from 50. Talking to the father sets `EVENTCHKINF_96`; hearing 0x26 or 0x27 sets
//!   `INFTABLE_196`, 0x24 or 0x25 `INFTABLE_197` (`EnSsh_Talk` until the box closes). Link out of
//!   range, it **climbs back** (`EnSsh_Return`, `NA_SE_EN_STALTU_UP`) and waits again. One placed
//!   near its floor starts down (`EnSsh_Start`, `SSH_STATE_GROUND_START`) and stays.
//! - **Its colliders** (`EnSsh_InitColliders`), as the Skulltula's: the body (0) takes arrows,
//!   Din's Fire, the hookshot, the hammer, explosives and Deku Nuts; the legs everything else, the
//!   back (1) with Link behind it, the front (2, metal and hookable) with him in front
//!   (`EnSsh_SetLegsCylinderAC`; a burning Deku Stick hits the body: `EnSsh_CheckBodyStickHit`);
//!   three OC cylinders across its home facing (`EnSsh_SetCylinderOC`); a sphere on its root limb
//!   (`EnSsh_PostLimbDraw`) only registered at 0 health, which it never reaches.
//! - **Hit in front** (`EnSsh_CheckHitFront`, a slash as Link faces it): 8 frames untouchable, and
//!   unless it's swaying, spinning from touching Link or stunned, it **sways** on its thread for
//!   60 frames (`EnSsh_Sway`, run by its draw).
//! - **Hit on the back or the body** (`EnSsh_CheckHitBack`), or the hammer's shock wave
//!   (`actorCtx.unk_02`): no damage; `NA_SE_EN_GOMA_JR_FREEZE` and `NA_SE_VO_ST_DAMAGE`; it's
//!   **stunned** for 120 frames (`EnSsh_SetStunned`: blue, shaking for the last 30, its animation
//!   and action held: `EnSsh_Damaged`), then (unless swaying) **spins** for 30 frames
//!   (`EnSsh_Turn`: by `10000 × spinTimer / 30` a frame, `NA_SE_EN_STALTU_ROLL` and
//!   `NA_SE_VO_ST_ATTACK`), its OC cylinders twice as wide meanwhile
//!   (`EnSsh_UpdateColliderScale`). A back hit also arms it once (`hitCount`).
//! - **Touching Link** (`EnSsh_CheckHitPlayer`, armed or spinning, not stunned): half a heart, a
//!   knockdown (`Actor_SetPlayerKnockbackLargeNoDamage`), a 30-frame spin (unless swaying) during
//!   which its OC cylinders are off (`hitTimer`), disarmed.
//!
//! Drawn as `EnSsh_Draw` does: its eyes on segment 8 (`blinkTex`, by `blinkState`: open but for
//! two frames every 60 to 119), the skeleton (`SkelAnime_DrawOpa`), the father's limbs 4, 5 and 8
//! his own (`object_ssh_DL_0046C0`, `_004080`, `_004DE8`). From baked meshes
//! (docs/adr/0012-actor-bakes.md), one per member kind and eye.
//!
//! Not ported here: the mask reactions (`MaskReaction_GetTextId` with
//! `MASK_REACTION_SET_CURSED_SKULLTULA_MAN`: no masks yet, so 0); the circle shadow
//! (`ActorShadow_DrawCircle`, 30), as for every actor. Not reached: a burning Deku Stick
//! (Player's `unk_860` stays 0), the hammer's shock wave.
//!
//! Params: `params` whole, the family member, 0 to 5 (0 the father, `ENSSH_FATHER`: gone at 100
//! tokens, 0.75 the size, his own face and legs, his texts; 1 to 5 a child, gone at `params × 10`
//! tokens, half the size, text 0x22).
//! Flags: reads `gsTokens`; sets EVENTCHKINF_96 (the father, talked to); reads and sets
//! INFTABLE_196 (texts 0x26, 0x27) and INFTABLE_197 (texts 0x24, 0x25).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::{Cylinder16, Sphere16};
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{cos_s, sin_s, smooth_step_to_f, smooth_step_to_s};
use glam::Vec3;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile, actor_set_player_knockback_large_no_damage, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind};
use oot_game::audio::sfx::*;
use oot_game::collision_check::*;
use oot_game::effect::EffectInit;
use oot_game::effect::blure::{EffectBlureInit, EffectBlureInit1};
use oot_game::npc::{offer_talk, process_talk_request, textbox_is_closing};
use oot_game::pack::{BakeBody, BakeSegment, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::{ANIMMODE_LOOP_INTERP, Anim, SkelAnimeStd};
use oot_game::sys_matrix::{MtxF, binang_to_rad};
use oot_game::{event_chk_inf, inf_table};

/// `ACTOR_EN_SSH` (`actor_table.h`: 0x0188).
pub const ACTOR_EN_SSH: i16 = 0x0188;
pub const OBJECT: &str = "object_ssh";
const SKEL: &str = "object_ssh_Skel_0052E0";

/// `En_Ssh_Profile`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_SSH,
    name: "En_Ssh",
    category: ACTORCAT_NPC,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `this->actor.params`, whole: the family member.
pub const MEMBER: Field = Field::whole(
    "params",
    Kind::Named(&[
        (0, "ENSSH_FATHER: gone at 100 tokens"),
        (1, "a child: gone at 10 tokens"),
        (2, "a child: gone at 20 tokens"),
        (3, "a child: gone at 30 tokens"),
        (4, "a child: gone at 40 tokens"),
        (5, "a child: gone at 50 tokens"),
    ]),
    "which of the cursed family: the father (bigger, his own face, legs and texts) or a child, gone from params × 10 tokens",
);

/// En_Ssh's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[MEMBER],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_96)).sets().note("the father, talked to"),
        FlagUse::new(inf_table!(INFTABLE_196)).reads().sets().note("the father's texts 0x26, 0x27"),
        FlagUse::new(inf_table!(INFTABLE_197)).reads().sets().note("the father's texts 0x24, 0x25"),
    ],
    todo: false,
};

/// `ENSSH_FATHER`.
pub const ENSSH_FATHER: i16 = 0;

/// `stateFlags`: `SSH_STATE_STUNNED`, `SSH_STATE_GROUND_START`, `SSH_STATE_ATTACKED`,
/// `SSH_STATE_SPIN`.
pub const SSH_STATE_STUNNED: u16 = 1 << 0;
pub const SSH_STATE_GROUND_START: u16 = 1 << 2;
pub const SSH_STATE_ATTACKED: u16 = 1 << 3;
pub const SSH_STATE_SPIN: u16 = 1 << 4;

/// `EnSshAnimation`.
pub const SSH_ANIM_UNK0: usize = 0;
pub const SSH_ANIM_UP: usize = 1;
pub const SSH_ANIM_WAIT: usize = 2;
pub const SSH_ANIM_LAND: usize = 3;
pub const SSH_ANIM_DROP: usize = 4;
pub const SSH_ANIM_UNK5: usize = 5;
pub const SSH_ANIM_UNK6: usize = 6;

/// `NA_SE_VO_ST_DAMAGE` (`voicebank_table.h`: 0x6867), `NA_SE_VO_ST_ATTACK` (0x6868).
pub const NA_SE_VO_ST_DAMAGE: u16 = 0x6867;
pub const NA_SE_VO_ST_ATTACK: u16 = 0x6868;

/// `EVENTCHKINF_96` (`save.h`: 0x96): the father talked to.
pub const EVENTCHKINF_96: u16 = 0x96;
/// `INFTABLE_196` (`save.h`: 0x196), `INFTABLE_197` (0x197).
pub const INFTABLE_196: u16 = 0x196;
pub const INFTABLE_197: u16 = 0x197;

/// `TOTAL_EFFECT_COUNT` (`effect.h`).
const TOTAL_EFFECT_COUNT: usize = oot_game::effect::SPARK_COUNT + oot_game::effect::BLURE_COUNT + oot_game::effect::SHIELD_PARTICLE_COUNT;

/// The colliders' ids (`collider_mut`): `colliderCylinders[0..6]`, then `colliderJntSph`.
pub const COL_CYLINDER_BODY: u8 = 0;
pub const COL_CYLINDER_BACK: u8 = 1;
pub const COL_CYLINDER_FRONT: u8 = 2;
pub const COL_JNT_SPH: u8 = 6;

/// `sCylinderInit1`: the body and the legs (AC), 32 by 50, 24 down.
const CYLINDER_INIT1: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HIT6, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_NONE,
    },
    dim: Cylinder16 { radius: 32, height: 50, y_shift: -24, pos: [0, 0, 0] },
};

/// `sColChkInfoInit`: 1 health, immovable.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit2 = CollisionCheckInfoInit2 { health: 1, cyl_radius: 0, cyl_height: 0, cyl_y_shift: 0, mass: MASS_IMMOVABLE };

/// `sCylinderInit2`: the three OC cylinders, 20 by 60, 30 down.
const CYLINDER_INIT2: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HIT6, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 20, height: 60, y_shift: -30, pos: [0, 0, 0] },
};

/// `sJntSphElementsInit`: one sphere on limb 1 (`{0, -240, 0}`, 28), AT `0xFFCFFFFF` for 4.
fn jnt_sph_elements() -> [ColliderJntSphElementInit; 1] {
    [ColliderJntSphElementInit {
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x04 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
            at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
            ac_elem_flags: ACELEM_NONE,
            oc_elem_flags: OCELEM_ON,
        },
        limb: 1,
        model_sphere: Sphere16 { center: [0, -240, 0], radius: 28 },
        scale: 100,
    }]
}

/// `sJntSphInit`.
const JNT_SPH_INIT: ColliderInit =
    ColliderInit { col_type: COL_MATERIAL_HIT6, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_JNTSPH };

/// The body's damage flags (`EnSsh_InitColliders`): `DMG_ARROW | DMG_MAGIC_FIRE | DMG_HOOKSHOT |
/// DMG_HAMMER_SWING | DMG_EXPLOSIVE | DMG_DEKU_NUT` (not the boomerang, unlike `En_St`'s).
pub const BODY_DMG_FLAGS: u32 = DMG_ARROW | DMG_MAGIC_FIRE | DMG_HOOKSHOT | DMG_HAMMER_SWING | DMG_EXPLOSIVE | DMG_DEKU_NUT;
/// The back's: `DMG_DEFAULT` without the body's, nor light and ice magic.
pub const BACK_DMG_FLAGS: u32 = DMG_DEFAULT & !BODY_DMG_FLAGS & !(DMG_MAGIC_LIGHT | DMG_MAGIC_ICE);
/// The front's: `DMG_DEFAULT` without the body's.
pub const FRONT_DMG_FLAGS: u32 = DMG_DEFAULT & !BODY_DMG_FLAGS;

/// The animations `EnSsh_SetAnimation` picks from.
const ANIM_NAMES: [&str; 3] = ["object_ssh_Anim_005BE8", "object_ssh_Anim_000304", "object_ssh_Anim_0055F8"];

/// `EnSsh_SetAnimation`'s `animation`, `playbackSpeed` and `mode` (1 `ANIMMODE_LOOP_INTERP`, 3
/// `ANIMMODE_ONCE_INTERP`), by `EnSshAnimation`: (the animation in `ANIM_NAMES`, speed, mode).
const S_ANIMATIONS: [(usize, f32, u8); 7] = [(0, 1.0, 3), (1, 4.0, 3), (1, 1.0, 1), (2, 1.0, 3), (1, 8.0, 1), (1, 6.0, 1), (0, 2.0, 1)];

/// `EnSsh_Draw`'s `blinkTex`.
const BLINK_TEX: [&str; 3] = ["object_ssh_Tex_0007E0", "object_ssh_Tex_000C60", "object_ssh_Tex_001060"];
const SEG_EYES: u8 = 0x08;

/// `EnSsh_OverrideLimbDraw`'s father's lists: (the limb, 1-based; the list).
const FATHER_LIMBS: [(usize, &str); 3] = [(4, "object_ssh_DL_0046C0"), (5, "object_ssh_DL_004080"), (8, "object_ssh_DL_004DE8")];

/// The mesh of a father (or a child) with eye `eye`.
pub fn bake_name(father: bool, eye: usize) -> String {
    format!("En_Ssh/{}_eye{eye}", if father { "father" } else { "child" })
}

/// The meshes `EnSsh_Draw` draws: the skeleton with each eye on segment 8, the father's with his
/// own limbs 4, 5 and 8.
pub fn bakes() -> Vec<MeshBake> {
    let mut v = Vec::new();
    for father in [true, false] {
        for (eye, tex) in BLINK_TEX.iter().enumerate() {
            let limbs = if father { FATHER_LIMBS.iter().map(|&(l, dl)| LimbOverride { limb: (l - 1) as u8, file: OBJECT.into(), symbol: dl.into() }).collect() } else { Vec::new() };
            v.push(MeshBake {
                name: bake_name(father, eye),
                object: OBJECT.into(),
                segments: vec![(SEG_EYES, BakeSegment::Texture { file: OBJECT.into(), symbol: (*tex).into() })],
                prelude: Vec::new(),
                body: BakeBody::Skeleton { file: OBJECT.into(), symbol: SKEL.into(), limbs },
            });
        }
    }
    v
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Start,
    Wait,
    Drop,
    Land,
    Idle,
    Talk,
    Return,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::Start => "EnSsh_Start",
            Action::Wait => "EnSsh_Wait",
            Action::Drop => "EnSsh_Drop",
            Action::Land => "EnSsh_Land",
            Action::Idle => "EnSsh_Idle",
            Action::Talk => "EnSsh_Talk",
            Action::Return => "EnSsh_Return",
        }
    }
}

/// `DECR`: down by one unless 0; the new value.
fn decr(x: &mut i16) -> i16 {
    if *x != 0 {
        *x -= 1;
    }
    *x
}

/// `BINANG_TO_RAD_ALT`: `((f32)binang / (f32)0x8000) * M_PI`, the product in double precision.
fn binang_to_rad_alt(binang: i16) -> f32 {
    ((binang as f32 / 32768.0) as f64 * std::f64::consts::PI) as f32
}

pub struct EnSsh {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: [Option<Anim>; 3],
    pub action: Action,
    /// `colliderCylinders`: the body, the back, the front (AC), then the three OC ones.
    pub collider_cylinders: [ColliderCylinder; 6],
    pub collider_jnt_sph: ColliderJntSph,
    /// `initialYaw`: its home yaw, which the OC cylinders' row keeps.
    pub initial_yaw: i16,
    /// `maxTurnRate`: grown while stunned, read by `EnSsh_Stunned`'s turn, which never runs.
    pub max_turn_rate: i16,
    /// `unkTimer`: never set (always 0).
    pub unk_timer: i16,
    pub spin_timer: i16,
    /// `hitTimer`: after touching Link, the frames its OC cylinders are off.
    pub hit_timer: i16,
    pub invincibility_timer: i16,
    pub sfx_timer: i16,
    pub stun_timer: i16,
    /// `animTimer`: the landing animation's frames left.
    pub anim_timer: i16,
    pub sway_timer: i16,
    /// `blureIdx`: the spin's trail (`TOTAL_EFFECT_COUNT` when `Effect_Add` found no slot).
    pub blure_idx: usize,
    /// `colliderScale`: 1.5 times its size (0.75 a child, 1.125 the father).
    pub collider_scale: f32,
    /// `floorHeightOffset`: its hover height above its floor (20 a child, 30 the father).
    pub floor_height_offset: f32,
    /// `ceilingPos`: where its thread hangs from (the sway's pivot).
    pub ceiling_pos: Vec3,
    pub sway_angle: i16,
    pub state_flags: u16,
    /// `hitCount`: armed (by a back hit) to hurt Link once.
    pub hit_count: u8,
    pub blink_state: i16,
    pub blink_timer: i16,
}

impl EnSsh {
    fn is_father(&self) -> bool {
        MEMBER.get(self.actor.params) == ENSSH_FATHER
    }

    /// `EnSsh_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let (skeleton, anims) = match play.assets.clone() {
            Some(a) => {
                let s = a.skeleton(OBJECT, SKEL).map_err(|e| log::error!("En_Ssh: {e:#}")).ok();
                (s, ANIM_NAMES.map(|n| a.animation(OBJECT, n).map_err(|e| log::error!("En_Ssh: {e:#}")).ok()))
            }
            None => (None, [None, None, None]),
        };
        let limbs = skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(29);
        let mut s = EnSsh {
            actor,
            skel: SkelAnimeStd::init_flex(limbs, None),
            skeleton,
            anims,
            action: Action::Start,
            collider_cylinders: Default::default(),
            collider_jnt_sph: ColliderJntSph::default(),
            initial_yaw: 0,
            max_turn_rate: 0,
            unk_timer: 0,
            spin_timer: 0,
            hit_timer: 0,
            invincibility_timer: 0,
            sfx_timer: 0,
            stun_timer: 0,
            anim_timer: 0,
            sway_timer: 0,
            blure_idx: TOTAL_EFFECT_COUNT,
            collider_scale: 0.0,
            floor_height_offset: 0.0,
            ceiling_pos: Vec3::ZERO,
            sway_angle: 0,
            state_flags: 0,
            hit_count: 0,
            blink_state: 0,
            blink_timer: 0,
        };
        let tokens = play.save.inventory.gs_tokens;
        if MEMBER.get(s.actor.params) == ENSSH_FATHER {
            if tokens >= 100 {
                s.actor.kill();
                return Box::new(s);
            }
        } else if tokens >= MEMBER.get(s.actor.params).wrapping_mul(10) {
            s.actor.kill();
            return Box::new(s);
        }
        // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 30): the circle shadow isn't ported.
        s.actor.shape_y_offset = 0.0;
        // SkelAnime_Init(&object_ssh_Skel_0052E0, NULL, .., 30); Animation_Change(&object_ssh_Anim_000304,
        // 1, 0, its last frame, ANIMMODE_LOOP_INTERP, 0).
        if let Some(anim) = s.anims[1].clone() {
            let frame_count = anim.last_frame();
            s.skel.change(anim, 1.0, 0.0, frame_count, ANIMMODE_LOOP_INTERP, 0.0);
        }
        s.blure_idx = Self::create_blure_effect(play);
        s.init_colliders();
        s.state_flags = 0;
        s.hit_count = 0;
        s.check_ceiling_pos(play);
        if MEMBER.get(s.actor.params) != ENSSH_FATHER {
            s.set_collider_scale(0.5, 1.0);
        } else {
            s.set_collider_scale(0.75, 1.0);
        }
        s.actor.gravity = 0.0;
        s.initial_yaw = s.actor.world_rot.y;
        s.action = Action::Start;
        Box::new(s)
    }

    /// `EnSsh_SpawnShockwave`: a white shockwave on its floor (100 big, spreading by 220, 8
    /// frames).
    fn spawn_shockwave(&self, play: &mut PlayState) {
        let zero = Vec3::ZERO;
        let pos = Vec3::new(self.actor.world_pos.x, self.actor.floor_height, self.actor.world_pos.z);
        play.with_ss(|ss| ss.blast_spawn_white_shockwave_set_scale(pos, zero, zero, 100, 220, 8));
    }

    /// `EnSsh_CreateBlureEffect`: the spin's trail (`EFFECT_BLURE1`: white from alpha 75 to 0,
    /// `elemDuration` 6, `calcMode` 3), `oot_game::effect::blure`'s.
    fn create_blure_effect(play: &mut PlayState) -> usize {
        // EFFECT_BLURE1: white, alpha 75 to 0, elemDuration 6, unkFlag 0, calcMode 3.
        play.effect_add(EffectInit::Blure(EffectBlureInit::One(EffectBlureInit1 {
            p1_start_color: [255, 255, 255, 75],
            p2_start_color: [255, 255, 255, 75],
            p1_end_color: [255, 255, 255, 0],
            p2_end_color: [255, 255, 255, 0],
            elem_duration: 6,
            unk_flag: 0,
            calc_mode: 3,
        })))
    }

    /// `EnSsh_CheckCeilingPos`: the ceiling straight above (`BgCheck_EntityLineTest1`, 1000 up,
    /// ceilings only, one face); without one, the line's end.
    fn check_ceiling_pos(&mut self, play: &PlayState) -> bool {
        let p = self.actor.world_pos;
        let pos_b = Vec3::new(p.x, p.y + 1000.0, p.z);
        match play.col.entity_line_test(p, pos_b, false, false, true, true) {
            None => {
                self.ceiling_pos = pos_b;
                false
            }
            Some((hit, _poly)) => {
                self.ceiling_pos = hit;
                true
            }
        }
    }

    /// `EnSsh_AddBlureVertex`: the trail's two points (`{834, 834, 0}` and `{834, -584, 0}` times
    /// `colliderScale`) through the matrix `mtx`, added to the trail (nothing with no slot).
    fn add_blure_vertex(&self, play: &mut PlayState, mtx: &MtxF) {
        let s = self.collider_scale;
        let p1 = mtx.mult_vec3f(Vec3::new(834.0 * s, 834.0 * s, 0.0 * s));
        let p2 = mtx.mult_vec3f(Vec3::new(834.0 * s, -584.0 * s, 0.0 * s));
        if let Some(b) = play.effect_blure_mut(self.blure_idx) {
            b.add_vertex(p1, p2);
        }
    }

    /// `EnSsh_AddBlureSpace`.
    fn add_blure_space(&self, play: &mut PlayState) {
        if let Some(b) = play.effect_blure_mut(self.blure_idx) {
            b.add_space();
        }
    }

    /// `EnSsh_InitColliders`: the six cylinders (three `sCylinderInit1`, three `sCylinderInit2`),
    /// the AC ones' damage flags, the front metal and hookable, `DamageTable_Get(2)`, the sphere.
    fn init_colliders(&mut self) {
        let cylinders = [&CYLINDER_INIT1, &CYLINDER_INIT1, &CYLINDER_INIT1, &CYLINDER_INIT2, &CYLINDER_INIT2, &CYLINDER_INIT2];
        for (c, init) in self.collider_cylinders.iter_mut().zip(cylinders) {
            *c = ColliderCylinder::new(init);
        }
        self.collider_cylinders[0].info.ac_dmg_info.dmg_flags = BODY_DMG_FLAGS;
        self.collider_cylinders[1].info.ac_dmg_info.dmg_flags = BACK_DMG_FLAGS;
        self.collider_cylinders[2].base.col_type = COL_MATERIAL_METAL;
        self.collider_cylinders[2].info.ac_elem_flags = ACELEM_ON | ACELEM_HOOKABLE | ACELEM_NO_AT_INFO;
        self.collider_cylinders[2].info.elem_material = ELEM_MATERIAL_UNK2;
        self.collider_cylinders[2].info.ac_dmg_info.dmg_flags = FRONT_DMG_FLAGS;
        self.actor.col_chk_info.set_info2(Some(&crate::en_st::S_DAMAGE_TABLE_PRESET_2), &COL_CHK_INFO_INIT);
        self.collider_jnt_sph = ColliderJntSph::new(&JNT_SPH_INIT, &jnt_sph_elements());
    }

    /// `EnSsh_SetAnimation`: the animation from its start to its last frame (morphing over 6);
    /// returns that last frame.
    fn set_animation(&mut self, anim_index: usize) -> f32 {
        let (a, speed, mode) = S_ANIMATIONS[anim_index];
        let Some(anim) = self.anims[a].clone() else { return 0.0 };
        let frame_count = anim.last_frame();
        self.skel.change(anim, speed, 0.0, frame_count, mode, -6.0);
        frame_count
    }

    /// `EnSsh_SetWaitAnimation`.
    fn set_wait_animation(&mut self) {
        self.set_animation(SSH_ANIM_WAIT);
    }

    /// `EnSsh_SetReturnAnimation`: `NA_SE_EN_STALTU_UP`.
    fn set_return_animation(&mut self, play: &mut PlayState) {
        audio_play_actor_sfx2(play, NA_SE_EN_STALTU_UP);
        self.set_animation(SSH_ANIM_UP);
    }

    /// `EnSsh_SetLandAnimation`: at its hover height above its floor.
    fn set_land_animation(&mut self) {
        self.actor.world_pos.y = self.floor_height_offset + self.actor.floor_height;
        self.anim_timer = self.set_animation(SSH_ANIM_LAND) as i16;
    }

    /// `EnSsh_SetDropAnimation`: down at 10.
    fn set_drop_animation(&mut self) {
        if self.unk_timer == 0 {
            self.anim_timer = self.set_animation(SSH_ANIM_DROP) as i16;
        }
        self.actor.velocity.y = -10.0;
    }

    /// `EnSsh_SetStunned`: unless stunned already, 120 frames of it (`SSH_STATE_ATTACKED`).
    fn set_stunned(&mut self) {
        if self.stun_timer == 0 {
            self.state_flags |= SSH_STATE_ATTACKED;
            self.stun_timer = 120;
            self.actor.color_filter_timer = 0;
        }
    }

    /// `EnSsh_SetColliderScale`: the sphere's radius, and each cylinder's shift and height, times
    /// `scale` (the radius times `scale × radius_mod` too); its scale 0.04, hover height 40 and
    /// `colliderScale` 1.5 times `scale`.
    ///
    /// @bug (game): the colliders' sizes are multiplied from what they are, not set: each spin's
    /// start and end (`EnSsh_UpdateColliderScale`) shrinks the cylinders' shift and height by half
    /// again, their radius by half at the end, and the sphere's radius by half at both (a child's
    /// body from 32 across, 25 tall to 8 and 6 after one spin).
    fn set_collider_scale(&mut self, scale: f32, radius_mod: f32) {
        let e = &mut self.collider_jnt_sph.elements[0];
        let radius = e.dim.model_sphere.radius as f32 * scale;
        e.dim.model_sphere.radius = radius as i32 as i16;
        for c in self.collider_cylinders.iter_mut() {
            let y_shift = c.dim.y_shift as f32 * scale;
            let radius = c.dim.radius as f32 * (scale * radius_mod);
            let height = c.dim.height as f32 * scale;
            c.dim.y_shift = y_shift as i32 as i16;
            c.dim.radius = radius as i32 as i16;
            c.dim.height = height as i32 as i16;
        }
        self.actor.scale = Vec3::splat(0.04 * scale);
        self.floor_height_offset = 40.0 * scale;
        self.collider_scale = scale * 1.5;
    }

    /// `EnSsh_Damaged`: stunned: blue on its first frame (120 frames); counting down (the turn
    /// rate growing towards 10000); at its end (unless swaying) a 30-frame spin,
    /// `NA_SE_EN_STALTU_ROLL` and `NA_SE_VO_ST_ATTACK`.
    fn damaged(&mut self, play: &mut PlayState) -> bool {
        if self.stun_timer == 120 && self.state_flags & SSH_STATE_STUNNED != 0 {
            self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 200, COLORFILTER_BUFFLAG_OPA, self.stun_timer as u16);
        }
        if decr(&mut self.stun_timer) != 0 {
            smooth_step_to_s(&mut self.max_turn_rate, 0x2710, 0xA, 0x3E8, 1);
            false
        } else {
            self.stun_timer = 0;
            self.state_flags &= !SSH_STATE_STUNNED;
            self.spin_timer = 0;
            if self.sway_timer == 0 {
                self.spin_timer = 30;
            }
            audio_play_actor_sfx2(play, NA_SE_EN_STALTU_ROLL);
            audio_play_actor_sfx2(play, NA_SE_VO_ST_ATTACK);
            true
        }
    }

    /// `EnSsh_Turn`: `hitTimer` counts down; spinning by `10000 × spinTimer / 30` a frame while
    /// `spinTimer` counts down; else, neither swaying nor stunned, turning to Link (by 4, at most
    /// 10000).
    fn turn(&mut self) {
        if self.hit_timer != 0 {
            self.hit_timer -= 1;
        }
        if decr(&mut self.spin_timer) != 0 {
            // world.rot.y += 10000.0f * (spinTimer / 30.0f): the sum in float, back to s16.
            self.actor.world_rot.y = (self.actor.world_rot.y as f32 + 10000.0 * (self.spin_timer as f32 / 30.0)) as i32 as i16;
        } else if self.sway_timer == 0 && self.stun_timer == 0 {
            smooth_step_to_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player, 4, 0x2710, 1);
        }
        self.actor.shape_rot.y = self.actor.world_rot.y;
    }

    /// `EnSsh_Stunned`: shaking (±0x7D0) for the stun's last 30 frames.
    ///
    /// @bug (game): the turn away from Link (`yawTowardsPlayer ^ 0x8000` at `maxTurnRate`) needs
    /// `stunTimer` 0, but this runs only while it isn't: it never turns while stunned.
    fn stunned(&mut self) {
        if self.sway_timer == 0 && self.stun_timer == 0 {
            smooth_step_to_s(&mut self.actor.world_rot.y, (self.actor.yaw_towards_player as u16 ^ 0x8000) as i16, 4, self.max_turn_rate, 1);
        }
        self.actor.shape_rot.y = self.actor.world_rot.y;
        if self.stun_timer < 30 {
            if self.stun_timer & 1 != 0 {
                self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x7D0);
            } else {
                self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_sub(0x7D0);
            }
        }
    }

    /// `EnSsh_UpdateYaw`.
    fn update_yaw(&mut self) {
        if self.stun_timer != 0 {
            self.stunned();
        } else {
            self.turn();
        }
    }

    /// `EnSsh_Bob`: the vertical speed towards 0.5, or -0.5 while `play->state.frames & 8`.
    fn bob(&mut self, play: &PlayState) {
        let mut bob_vel = 0.5f32;
        if (play.state_frames & 8) != 0 {
            bob_vel *= -1.0;
        }
        smooth_step_to_f(&mut self.actor.velocity.y, bob_vel, 0.4, 1000.0, 0.0);
    }

    /// `EnSsh_IsCloseToLink`: started down, landing or swaying, always; else Link within 160
    /// across, below it by at most 400, and not below its floor.
    fn is_close_to_link(&self, play: &PlayState) -> bool {
        if self.state_flags & SSH_STATE_GROUND_START != 0 {
            return true;
        }
        if self.unk_timer != 0 {
            return true;
        }
        if self.sway_timer != 0 {
            return true;
        }
        if self.anim_timer != 0 {
            return true;
        }
        if self.actor.xz_dist_to_player > 160.0 {
            return false;
        }
        let player_y = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos.y).unwrap_or(0.0);
        let y_dist = self.actor.world_pos.y - player_y;
        if y_dist < 0.0 || y_dist > 400.0 {
            return false;
        }
        if player_y < self.actor.floor_height {
            return false;
        }
        true
    }

    /// `EnSsh_IsCloseToHome`: two frames' rise from its home height.
    fn is_close_to_home(&self) -> bool {
        let vel = self.actor.velocity.y;
        let next_y = self.actor.world_pos.y + 2.0 * vel;
        next_y >= self.actor.home_pos.y
    }

    /// `EnSsh_IsCloseToGround`: two frames' fall from its hover height.
    fn is_close_to_ground(&self) -> bool {
        let vel = self.actor.velocity.y;
        let next_y = self.actor.world_pos.y + 2.0 * vel;
        (next_y - self.actor.floor_height) <= self.floor_height_offset
    }

    /// `EnSsh_Sway` (run by its draw): swinging on its thread from `ceilingPos` across its facing,
    /// by `sin(swayAngle)` (0x640 on a frame) of a swing shrinking with the timer (`swayTimer / 6`
    /// degrees), leaning by twice the angle.
    fn sway(&mut self) {
        if self.sway_timer != 0 {
            self.sway_angle = self.sway_angle.wrapping_add(0x640);
            self.sway_timer -= 1;
            if self.sway_timer == 0 {
                self.sway_angle = 0;
            }
            let temp = self.sway_timer as f32 * (1.0f32 / 6.0);
            let sway_angle = (temp * (65536.0f32 / 360.0) * sin_s(self.sway_angle)) as i32 as i16;
            let temp = self.actor.world_pos.y - self.ceiling_pos.y;
            let sway_vec_base = Vec3::new(sin_s(sway_angle) * temp, cos_s(sway_angle) * temp, 0.0);
            let c = self.ceiling_pos;
            let mut m = MtxF::set_translate(c.x, c.y, c.z);
            m.rotate_y(binang_to_rad(self.actor.world_rot.y));
            let sway_vec = m.mult_vec3f(sway_vec_base);
            self.actor.shape_rot.z = (-(sway_angle as i32 * 2)) as i16;
            self.actor.world_pos.x = sway_vec.x;
            self.actor.world_pos.z = sway_vec.z;
        }
    }

    /// `EnSsh_CheckBodyStickHit`: a burning Deku Stick (Player's `unk_860`) hits the body, else
    /// the legs.
    fn check_body_stick_hit(&mut self, play: &PlayState) {
        let unk_860 = play.player.and_then(|h| play.actors.downcast::<crate::player::Player>(h)).map(|p| p.unk_860).unwrap_or(0);
        let c = &mut self.collider_cylinders;
        if unk_860 != 0 {
            c[0].info.ac_dmg_info.dmg_flags |= DMG_DEKU_STICK;
            c[1].info.ac_dmg_info.dmg_flags &= !DMG_DEKU_STICK;
            c[2].info.ac_dmg_info.dmg_flags &= !DMG_DEKU_STICK;
        } else {
            c[0].info.ac_dmg_info.dmg_flags &= !DMG_DEKU_STICK;
            c[1].info.ac_dmg_info.dmg_flags |= DMG_DEKU_STICK;
            c[2].info.ac_dmg_info.dmg_flags |= DMG_DEKU_STICK;
        }
    }

    /// `EnSsh_CheckHitPlayer`: armed or spinning, an OC cylinder touched Link: `hitTimer` 30, a
    /// spin as long (unless swaying), `NA_SE_EN_STALTU_ROLL` and `NA_SE_VO_ST_ATTACK`, half a
    /// heart off him, a knockdown at 4 (6 up) away from it, and disarmed.
    ///
    /// @bug (game): spinning but not armed (the spin after a stun by the hammer's shock wave, which
    /// doesn't arm it), the touch takes `hitCount` (a `u8`) from 0 to 255: armed for 255 more.
    fn check_hit_player(&mut self, play: &mut PlayState) -> bool {
        if self.hit_count == 0 && self.spin_timer == 0 {
            return false;
        }
        let mut hit = false;
        for i in 0..3 {
            if self.collider_cylinders[i + 3].base.oc_flags2 & OC2_HIT_PLAYER != 0 {
                self.collider_cylinders[i + 3].base.oc_flags2 &= !OC2_HIT_PLAYER;
                hit = true;
            }
        }
        if !hit {
            return false;
        }
        self.hit_timer = 30;
        if self.sway_timer == 0 {
            self.spin_timer = self.hit_timer;
        }
        audio_play_actor_sfx2(play, NA_SE_EN_STALTU_ROLL);
        audio_play_actor_sfx2(play, NA_SE_VO_ST_ATTACK);
        crate::player::play_damage_player(play, -8);
        actor_set_player_knockback_large_no_damage(play, 4.0, self.actor.yaw_towards_player, 6.0);
        self.hit_count = self.hit_count.wrapping_sub(1);
        true
    }

    /// `EnSsh_CheckHitFront`: the front hit: 8 frames untouchable, and 60 of swaying unless
    /// swaying, `hitTimer` or stunned.
    fn check_hit_front(&mut self) -> bool {
        let ac_flags = self.collider_cylinders[2].base.ac_flags;
        if ac_flags & AC_HIT == 0 {
            false
        } else {
            self.collider_cylinders[2].base.ac_flags &= !AC_HIT;
            self.invincibility_timer = 8;
            if self.sway_timer == 0 && self.hit_timer == 0 && self.stun_timer == 0 {
                self.sway_timer = 60;
            }
            true
        }
    }

    /// `EnSsh_CheckHitBack`: the body or the back hit: 8 frames untouchable, armed (`hitCount`
    /// up to 1), `NA_SE_EN_GOMA_JR_FREEZE` and `NA_SE_VO_ST_DAMAGE` unless stunned already, and
    /// stunned. Always false.
    fn check_hit_back(&mut self, play: &mut PlayState) -> bool {
        let mut hit = false;
        for i in [0, 1] {
            let c = &mut self.collider_cylinders[i];
            if c.base.ac_flags & AC_HIT != 0 {
                c.base.ac_flags &= !AC_HIT;
                hit = true;
            }
        }
        if !hit {
            return false;
        }
        self.invincibility_timer = 8;
        if self.hit_count == 0 {
            self.hit_count += 1;
        }
        if self.stun_timer == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_GOMA_JR_FREEZE);
            audio_play_actor_sfx2(play, NA_SE_VO_ST_DAMAGE);
        }
        self.set_stunned();
        self.state_flags |= SSH_STATE_STUNNED;
        false
    }

    /// `EnSsh_CollisionCheck`: touching Link (not stunned); a hit in front; the hammer's shock
    /// wave (`actorCtx.unk_02`: stunned as by a back hit); a hit on the back. Always false.
    fn collision_check(&mut self, play: &mut PlayState) -> bool {
        if self.stun_timer == 0 {
            self.check_hit_player(play);
        }
        if self.check_hit_front() {
            false
        } else if play.actors.unk_02 != 0 {
            self.invincibility_timer = 8;
            if self.stun_timer == 0 {
                audio_play_actor_sfx2(play, NA_SE_EN_GOMA_JR_FREEZE);
                audio_play_actor_sfx2(play, NA_SE_VO_ST_DAMAGE);
            }
            self.set_stunned();
            self.state_flags |= SSH_STATE_STUNNED;
            false
        } else {
            // Always returns false.
            self.check_hit_back(play)
        }
    }

    /// `EnSsh_SetBodyCylinderAC`.
    fn set_body_cylinder_ac(&mut self, play: &mut PlayState) {
        self.collider_cylinders[0].update(&self.actor);
        play.collision_check_set_ac(&self.actor, 0, &mut self.collider_cylinders[0]);
    }

    /// `EnSsh_SetLegsCylinderAC`: the front's with Link within 90 degrees (0x3FFC) of its facing,
    /// else the back's. (`ABS` of the `s16` difference -0x8000 stays -0x8000 in the `s16`: Link
    /// right behind it by that count is "in front".)
    fn set_legs_cylinder_ac(&mut self, play: &mut PlayState) {
        let angle_towards_link = (self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y) as i32).abs() as i16;
        let i = if (angle_towards_link as i32) < 90 * (0x10000 / 360) { 2 } else { 1 };
        self.collider_cylinders[i].update(&self.actor);
        play.collision_check_set_ac(&self.actor, i as u8, &mut self.collider_cylinders[i]);
    }

    /// `EnSsh_SetCylinderOC`: the three OC cylinders 40 (times `colliderScale`) apart across its
    /// home facing (`initialYaw`), at its position.
    fn set_cylinder_oc(&mut self, play: &mut PlayState) -> bool {
        let mut cyl_offsets = [Vec3::new(40.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(-40.0, 0.0, 0.0)];
        for (i, off) in cyl_offsets.iter_mut().enumerate() {
            let p = self.actor.world_pos;
            *off *= self.collider_scale;
            let mut m = MtxF::set_translate(p.x, p.y, p.z);
            m.rotate_y(binang_to_rad_alt(self.initial_yaw));
            let cyl_pos = m.mult_vec3f(*off);
            let c = &mut self.collider_cylinders[i + 3];
            c.dim.pos = [cyl_pos.x as i32 as i16, cyl_pos.y as i32 as i16, cyl_pos.z as i32 as i16];
            play.collision_check_set_oc(&self.actor, (i + 3) as u8, c);
        }
        true
    }

    /// `EnSsh_SetColliders`: at 0 health (never) the sphere; else the OC cylinders unless
    /// `hitTimer` runs, the AC ones once untouchable no more.
    fn set_colliders(&mut self, play: &mut PlayState) {
        if self.actor.col_chk_info.health == 0 {
            play.collision_check_set_at(&self.actor, COL_JNT_SPH, &mut self.collider_jnt_sph);
            play.collision_check_set_oc(&self.actor, COL_JNT_SPH, &mut self.collider_jnt_sph);
        } else {
            if self.hit_timer == 0 {
                self.set_cylinder_oc(play);
            }
            if decr(&mut self.invincibility_timer) == 0 {
                self.set_body_cylinder_ac(play);
                self.set_legs_cylinder_ac(play);
            }
        }
    }

    /// `EnSsh_Wait`: Link near, it drops; else it bobs.
    fn wait(&mut self, play: &PlayState) {
        if self.is_close_to_link(play) {
            self.set_drop_animation();
            self.action = Action::Drop;
        } else {
            self.bob(play);
        }
    }

    /// `EnSsh_Talk`: bobbing until the box closes.
    fn talk(&mut self, play: &PlayState) {
        self.bob(play);
        if textbox_is_closing(play) {
            self.action = Action::Idle;
        }
    }

    /// `EnSsh_GetTextId`'s part of `EnSsh_Idle`: the mask reaction (none), else the father's by
    /// the tokens and `INFTABLE_196` / `_197`, a child's 0x22.
    pub fn text_id(play: &PlayState, params: i16) -> u16 {
        // MaskReaction_GetTextId(play, MASK_REACTION_SET_CURSED_SKULLTULA_MAN): no masks, so 0.
        let s = &play.save;
        let tokens = s.inventory.gs_tokens;
        if params == ENSSH_FATHER {
            if tokens >= 50 {
                0x29
            } else if tokens >= 10 {
                if s.get_inf_table(INFTABLE_197) { 0x24 } else { 0x25 }
            } else if s.get_inf_table(INFTABLE_196) {
                0x27
            } else {
                0x26
            }
        } else {
            0x22
        }
    }

    /// `EnSsh_Idle`: a talk taken: the father's `EVENTCHKINF_96`, the texts' `INFTABLE_196` and
    /// `_197`. Else the landing's animations to the waiting one; Link out of range, back up; else
    /// a laugh every 64 frames, the bob, and (landed) its text offered within 100.
    fn idle(&mut self, play: &mut PlayState) {
        if process_talk_request(&mut self.actor) {
            self.action = Action::Talk;
            if MEMBER.get(self.actor.params) == ENSSH_FATHER {
                play.save.set_event_chk_inf(EVENTCHKINF_96);
            }
            if self.actor.text_id == 0x26 || self.actor.text_id == 0x27 {
                play.save.set_inf_table(INFTABLE_196);
            }
            if self.actor.text_id == 0x24 || self.actor.text_id == 0x25 {
                play.save.set_inf_table(INFTABLE_197);
            }
        } else {
            if self.unk_timer != 0 && decr(&mut self.unk_timer) == 0 {
                self.set_animation(SSH_ANIM_WAIT);
            }
            if self.anim_timer != 0 && decr(&mut self.anim_timer) == 0 {
                self.set_animation(SSH_ANIM_WAIT);
            }
            if !self.is_close_to_link(play) {
                self.set_return_animation(play);
                self.action = Action::Return;
            } else {
                if decr(&mut self.sfx_timer) == 0 {
                    audio_play_actor_sfx2(play, NA_SE_EN_STALTU_LAUGH);
                    self.sfx_timer = 64;
                }
                self.bob(play);
                if self.unk_timer == 0 && self.anim_timer == 0 {
                    self.actor.text_id = Self::text_id(play, MEMBER.get(self.actor.params));
                    let a = self.actor.clone();
                    offer_talk(play, &a, 100.0);
                }
            }
        }
    }

    /// `EnSsh_Land`: the animations as `EnSsh_Idle`'s; at its hover height, idle; else rising
    /// (towards 2 a frame).
    fn land(&mut self) {
        if self.unk_timer != 0 && decr(&mut self.unk_timer) == 0 {
            self.set_animation(SSH_ANIM_WAIT);
        }
        if self.anim_timer != 0 && decr(&mut self.anim_timer) == 0 {
            self.set_animation(SSH_ANIM_WAIT);
        }
        if (self.actor.floor_height + self.floor_height_offset) <= self.actor.world_pos.y {
            self.action = Action::Idle;
        } else {
            smooth_step_to_f(&mut self.actor.velocity.y, 2.0, 0.6, 1000.0, 0.0);
        }
    }

    /// `EnSsh_Drop`: Link out of range, back up; close to the ground, the shockwave and the
    /// landing; else `NA_SE_EN_STALTU_DOWN` every 3 frames.
    fn drop_(&mut self, play: &mut PlayState) {
        if self.unk_timer != 0 && decr(&mut self.unk_timer) == 0 {
            self.set_animation(SSH_ANIM_DROP);
        }
        if !self.is_close_to_link(play) {
            self.set_return_animation(play);
            self.action = Action::Return;
        } else if self.is_close_to_ground() {
            self.spawn_shockwave(play);
            self.set_land_animation();
            self.action = Action::Land;
        } else if decr(&mut self.sfx_timer) == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_STALTU_DOWN);
            self.sfx_timer = 3;
        }
    }

    /// `EnSsh_Return`: climbing at `4 ×` the climbing animation's progress (again from its start
    /// when it ends); Link back in range, it drops; near its home height, it waits.
    fn return_(&mut self, play: &mut PlayState) {
        let frame_ratio = self.skel.cur_frame / (self.skel.anim_length - 1.0);
        if frame_ratio == 1.0 {
            self.set_return_animation(play);
        }
        if self.is_close_to_link(play) {
            self.set_drop_animation();
            self.action = Action::Drop;
        } else if self.is_close_to_home() {
            self.set_wait_animation();
            self.action = Action::Wait;
        } else {
            self.actor.velocity.y = 4.0 * frame_ratio;
        }
    }

    /// `EnSsh_UpdateColliderScale`: a spin's start doubles the radius; its end halves it back (and
    /// both shrink the rest: `set_collider_scale`'s bug).
    fn update_collider_scale(&mut self) {
        let scale = if MEMBER.get(self.actor.params) != ENSSH_FATHER { 0.5 } else { 0.75 };
        if self.state_flags & SSH_STATE_SPIN != 0 {
            if self.spin_timer == 0 {
                self.state_flags &= !SSH_STATE_SPIN;
                self.set_collider_scale(scale, 1.0);
            }
        } else if self.spin_timer != 0 {
            self.state_flags |= SSH_STATE_SPIN;
            self.set_collider_scale(scale, 2.0);
        }
    }

    /// `EnSsh_Start`: high up, it waits on the ceiling; near its floor, it lands
    /// (`SSH_STATE_GROUND_START`: it stays down). Either runs at once.
    fn start(&mut self, play: &mut PlayState) {
        if !self.is_close_to_ground() {
            self.action = Action::Wait;
            self.wait(play);
        } else {
            self.set_land_animation();
            self.state_flags |= SSH_STATE_GROUND_START;
            self.action = Action::Land;
            self.land();
        }
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Start => self.start(play),
            Action::Wait => self.wait(play),
            Action::Drop => self.drop_(play),
            Action::Land => self.land(),
            Action::Idle => self.idle(play),
            Action::Talk => self.talk(play),
            Action::Return => self.return_(play),
        }
    }

    /// `Actor_Draw`'s model matrix (`Matrix_SetTranslateRotateYXZ`, then the scale).
    fn actor_mtx(&self) -> MtxF {
        let a = &self.actor;
        let mut m = MtxF::set_translate_rotate_yxz(a.world_pos.x, a.world_pos.y + a.shape_y_offset * a.scale.y, a.world_pos.z, [a.shape_rot.x, a.shape_rot.y, a.shape_rot.z]);
        m.scale(a.scale.x, a.scale.y, a.scale.z);
        m
    }
}

impl ActorImpl for EnSsh {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnSsh_Update`: the spin's collider scale; the hits; stunned, the stun's count, else the
    /// animation, the move, the floor and the action; the yaw; the blink; the colliders; the
    /// focus.
    fn update(&mut self, play: &mut PlayState) {
        self.update_collider_scale();
        if self.collision_check(play) {
            // EnSsh_CollisionCheck always returns false, so this never happens.
            return;
        }
        if self.stun_timer != 0 {
            self.damaged(play);
        } else {
            self.skel.update();
            self.actor.update_pos();
            self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
            self.run_action(play);
        }
        self.update_yaw();
        if decr(&mut self.blink_timer) == 0 {
            self.blink_timer = play.rand.s16_offset(60, 60);
        }
        self.blink_state = self.blink_timer;
        if self.blink_state >= 3 {
            self.blink_state = 0;
        }
        self.set_colliders(play);
        self.actor.set_focus(0.0);
    }

    /// `ActorProfile.destroy`: `Effect_Delete` of the trail (no slot: nothing).
    fn destroy(&mut self, play: &mut PlayState) {
        play.effect_delete(self.blure_idx);
    }

    /// `EnSsh_Draw`'s effects on the actor: the Deku Stick's target (`EnSsh_CheckBodyStickHit`),
    /// the sway (`EnSsh_Sway`); limb 1's trail point (`EnSsh_OverrideLimbDraw`, spinning and not
    /// swaying: nothing, no trail); the sphere follows limb 1 (`EnSsh_PostLimbDraw`).
    fn draw_update(&mut self, play: &mut PlayState) {
        if self.actor.killed {
            return;
        }
        self.check_body_stick_hit(play);
        self.sway();
        let model = self.actor_mtx();
        if self.spin_timer != 0 && self.sway_timer == 0 {
            if self.spin_timer >= 2 {
                self.add_blure_vertex(play, &model);
            } else {
                self.add_blure_space(play);
            }
        }
        // SkelAnime_DrawOpa's root limb: Matrix_TranslateRotateZYX(jointTable[0], jointTable[1]).
        let j = &self.skel.joint_table;
        let mut root = model;
        root.translate_rotate_zyx(Vec3::new(j[0][0] as f32, j[0][1] as f32, j[0][2] as f32), j[1]);
        self.collider_jnt_sph.update_spheres(1, &root.to_mat4());
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.switches = vec![self.blink_state.clamp(0, 2) as u32];
        rs
    }

    /// `EnSsh_Draw`: the eyes (`blinkTex[blinkState]` on segment 8), the skeleton.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(joints), [eye]) = (&rs.joints, rs.switches.as_slice()) else { return };
        let Some(skeleton) = &self.skeleton else { return };
        let bones = skeleton.pose(joints);
        let model = oot_game::play::actor_draw_matrix(rs);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(self.is_father(), *eye as usize))), transform: model, bones, params: Default::default() });
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            0..=5 => Some(ColliderMut::Cylinder(&mut self.collider_cylinders[id as usize])),
            COL_JNT_SPH => Some(ColliderMut::JntSph(&mut self.collider_jnt_sph)),
            _ => None,
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
