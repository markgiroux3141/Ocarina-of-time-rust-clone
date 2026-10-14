//! `En_Tk` (`ovl_En_Tk/z_en_tk.c`): Dampé the gravekeeper, who runs "Dampé's Heart-Pounding
//! Gravedigging Tour" in the graveyard on the child's evenings (`spot02_scene` layer 1, at
//! (-476, 61, 434)).
//!
//! He's there only for a child in `SCENE_GRAVEYARD` from just after 18:00 to just before 21:00
//! (`EnTk_Init`: `dayTime <= CLOCK_TIME(18, 0)` or `>= CLOCK_TIME(21, 0)` kills him, so 18:00
//! exactly too).
//!
//! - **He walks his round** (`EnTk_Walk`): the scene's path 0 (`EnTk_Orient`: turning towards
//!   its next point by 1000 at most a frame; within √10 of it, on to the next, round again after
//!   the last), stepping forward only on two stretches of his walk (`EnTk_Step`: frames 10 to 21
//!   and 36 to 47, at up to 2 a frame), `NA_SE_EN_MORIBLIN_WALK` at its frames 0 and 25. He
//!   doesn't follow Link: he stops (`EnTk_RestAnim`) when Link is within 100 and in front of his
//!   face (`EnTk_CheckFacingPlayer`: 0x1554 either side), or after 240 to 479 frames.
//! - **He rests** (`EnTk_Rest`) 60 to 119 frames, his head turned towards Link (by
//!   `headRot`, 1270 to 10730), then walks on. Link in front of him keeps him resting and offers
//!   to talk within 60 (`Npc_UpdateTalking`, the collider's 30 + 30).
//! - **His talk** (`EnTk_GetTextId`, `EnTk_UpdateTalkState`): 0x5018 ("Hey kid! ...") the first
//!   time, 0x5019 ("Do you want me to dig here?") once `INFTABLE_D9` is set. At the choice: "No"
//!   (choice 1) gives 0x0084; "Yes" with fewer than 10 rupees 0x0085; either sets `INFTABLE_D9`
//!   and ends at its box's event. "Yes" with 10 or more: the box paused (`MSGMODE_PAUSED`), 10
//!   rupees taken (`Rupees_ChangeBy(-10)`), `INFTABLE_D9`, and he **digs where he stands**
//!   (`NPC_TALK_STATE_ACTION`, then `EnTk_DigAnim` and `EnTk_Dig` the next frame).
//! - **The dig** (`EnTk_Dig`): a dig spot (`En_It`) within 40 across and not more than 10 above
//!   his floor when he starts (`EnTk_CheckNextSpot`, from the prop list; not the spot he last dug
//!   on until he's walked off it: `EnTk_CheckCurrentSpot`) makes it a good dig. At the dig's frame
//!   32: `NA_SE_EV_DIG_UP`, and on a good dig a prize (`EnTk_ChooseReward`) dropped 40 from him
//!   (`Item_DropCollectible`): a green, blue or red rupee, or a purple one, which the first time
//!   is the piece of heart instead (`ITEMGETINF_1C`). Ten frames on, `NA_SE_SY_TRE_BOX_APPEAR`
//!   (the chime for the piece of heart; `NA_SE_SY_ERROR` for nothing). At the dig's end the box
//!   closes, or with nothing found 0x501A ("Nope, nothing here!") opens. The prizes come in
//!   rounds: eight greens, four blues, two reds and one purple, drawn at random (40%, 30%, 20%,
//!   10%) while their kind lasts, else the cheapest left; after all fifteen, a green starts the
//!   next round.
//! - **The dirt** (`EnTkEff_*`): from frame 32 to 39 of a dig, a puff a frame at his shovel's
//!   blade (`v3f_304`), his own list of 20, rising and growing for 12 frames, drawn as
//!   `EnTkEff_Draw` does (`gDampeEff1DL`, `gDampeEff2DL` facing the camera, `gDust8Tex` to
//!   `gDust1Tex` as it fades).
//!
//! Drawn as `EnTk_Draw` does: the dirt, his eyes on segment 8 (`sEyesSegments`, blinking:
//! `EnTk_UpdateEyes`), the skeleton with his head turned by `headRot` on limb 16
//! (`EnTk_OverrideLimbDraw`, which also reads `h_21E`, the animation's yaw of limbs 15 and 16),
//! the focus at limb 16 and the shovel (`gDampeShovelDL`, `func_80B1D200`) at limb 14, with the
//! blade's point (0, 0, 4600) on it (`EnTk_PostLimbDraw`). From baked meshes
//! (docs/adr/0012-actor-bakes.md), one per eye and per dust frame. The circle shadow
//! (`ActorShadow_DrawCircle`, 24) is the usual stand-in.
//!
//! Faithful bugs: `EnTk_Rest` reads `v1` uninitialised when he starts walking (taken as 0 here);
//! `EnTk_Dig` gives `Matrix_RotateY` his yaw in binary angle units as radians, so the prize lands
//! 40 from him in a direction that isn't his facing; text 0x5028 (never his) would set
//! `INFTABLE_D8`.
//!
//! Not ported here: the mask reactions (`MaskReaction_GetTextId(play, MASK_REACTION_SET_DAMPE)`:
//! no masks yet, so none; BACKLOG #39).
//!
//! Params: `params` whole, as an `s16` (`EnTk_Orient`: below 0 he never turns to his path; 0 and
//! up he walks the scene's path 0, whatever the value). Placed as 0.
//! Flags: reads and sets INFTABLE_D9 (heard the offer: 0x5019 instead of 0x5018); sets
//! INFTABLE_D8 only for text 0x5028, which isn't his (so never); reads and sets ITEMGETINF_1C (the
//! piece of heart dug up).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{sin_s, smooth_step_to_s};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, Actor, UPDBGCHECKINFO_FLAG_0, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ACTORCAT_PROP, ActorHandle, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind};
use oot_game::audio::sfx::{NA_SE_SY_CORRECT_CHIME, NA_SE_SY_ERROR, NA_SE_SY_TRE_BOX_APPEAR};
use oot_game::collision_check::*;
use oot_game::gbi::setup_dl;
use oot_game::item::rupees_change_by;
use oot_game::message::*;
use oot_game::npc::{NpcTrack, process_talk_request, talk_update};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, Rand, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::*;
use oot_game::sys_matrix::MtxF;
use oot_game::{inf_table, item_get_inf};

use crate::en_it::ACTOR_EN_IT;
use crate::en_item00::{ITEM00_HEART_PIECE, ITEM00_RUPEE_BLUE, ITEM00_RUPEE_GREEN, ITEM00_RUPEE_PURPLE, ITEM00_RUPEE_RED, item_drop_collectible};

/// `ACTOR_EN_TK` (`actor_table.h`: 0x0085).
pub const ACTOR_EN_TK: i16 = 0x0085;
/// `OBJECT_TK` (`object_table.h`: 0x0089).
pub const OBJECT: &str = "object_tk";

/// `En_Tk_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_TK, name: "En_Tk", category: ACTORCAT_NPC, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY, object: OBJECT, schema: &SCHEMA };

/// `this->actor.params < 0` (`EnTk_Orient`): the whole params as an `s16`.
pub const PARAMS: Field = Field::whole("params", Kind::Range(-0x8000, 0x7FFF), "below 0: he never turns to his path; 0 and up: he walks the scene's path 0");

/// En_Tk's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[PARAMS],
    flags: &[
        FlagUse::new(inf_table!(INFTABLE_D9)).reads().sets().note("heard his offer: 0x5019 instead of 0x5018"),
        FlagUse::new(inf_table!(INFTABLE_D8)).sets().note("text 0x5028 only, which isn't his: never"),
        FlagUse::new(item_get_inf!(ITEMGETINF_1C)).reads().sets().note("the piece of heart dug up"),
    ],
    todo: false,
};

/// `SCENE_GRAVEYARD` (`scene_table.h`: 0x53).
pub const SCENE_GRAVEYARD: u16 = 0x53;
/// `INFTABLE_D8`, `INFTABLE_D9` (`save.h`: 0xD8, 0xD9).
pub const INFTABLE_D8: u16 = 0xD8;
pub const INFTABLE_D9: u16 = 0xD9;
/// `ITEMGETINF_1C` (`save.h`: 0x1C): the graveyard's piece of heart dug up.
pub const ITEMGETINF_1C: u16 = 0x1C;
/// `NA_SE_EV_DIG_UP` (`environmentbank_table.h`: 0x28C8).
pub const NA_SE_EV_DIG_UP: u16 = 0x28C8;
/// `NA_SE_EN_MORIBLIN_WALK` (`enemybank_table.h`: 0x38B8).
pub const NA_SE_EN_MORIBLIN_WALK: u16 = 0x38B8;

/// `NPC_TALK_STATE_*` (`actor.h`).
pub const NPC_TALK_STATE_IDLE: i16 = 0;
pub const NPC_TALK_STATE_TALKING: i16 = 1;
pub const NPC_TALK_STATE_ACTION: i16 = 2;

/// `ATTENTION_RANGE_6`.
const ATTENTION_RANGE_6: u8 = 6;

/// `rewardParams` (`EnTk_Dig`), by `currentReward`.
pub const REWARD_PARAMS: [i16; 5] = [ITEM00_RUPEE_GREEN, ITEM00_RUPEE_BLUE, ITEM00_RUPEE_RED, ITEM00_RUPEE_PURPLE, ITEM00_HEART_PIECE];
/// The dig's frame where the prize comes out.
pub const DIG_REWARD_FRAME: f32 = 32.0;

// gDampeSkel's limbs, 1-based as `EnTk_OverrideLimbDraw` sees them (the C's comments).
const LIMB_NECK: usize = 14;
const LIMB_HEAD: usize = 15;
const LIMB_JAW: usize = 16;

/// `sCylinderInit`.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: 0, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0000, hit_backlash: 0, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 30, height: 52, y_shift: 0, pos: [0; 3] },
};

/// `sColChkInfoInit`.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit2 = CollisionCheckInfoInit2 { health: 0, cyl_radius: 0, cyl_height: 0, cyl_y_shift: 0, mass: MASS_IMMOVABLE };

const SKEL: &str = "gDampeSkel";
const DIG_ANIM: &str = "gDampeDigAnim";
const WALK_ANIM: &str = "gDampeWalkAnim";
const REST_ANIM: &str = "gDampeRestAnim";
/// `sEyesSegments`.
const EYE_TEXTURES: [&str; 3] = ["gDampeEyeOpenTex", "gDampeEyeHalfTex", "gDampeEyeClosedTex"];
/// `dustTextures` (`EnTkEff_Draw`), by `imageIdx`.
const DUST_TEXTURES: [&str; 8] = ["gDust8Tex", "gDust7Tex", "gDust6Tex", "gDust5Tex", "gDust4Tex", "gDust3Tex", "gDust2Tex", "gDust1Tex"];

/// The dirt's segments: the texture (8, as `gDampeEff2DL` loads it), its colours, its setup.
const SEG_TEX: u8 = 0x08;
const SEG_SETUP: u8 = 0x09;
const SEG_COLOR: u8 = 0x0A;
/// `gDPSetEnvColor(100, 60, 20, 0)`, the dirt's.
const DUST_ENV: [u8; 4] = [100, 60, 20, 0];
/// `gDPSetPrimColor(0, 0, 170, 130, 90, alpha)`, the dirt's.
const DUST_PRIM: [u8; 3] = [170, 130, 90];

/// The skeleton drawn with eye `eye`.
pub fn bake_name(eye: usize) -> String {
    format!("En_Tk/eye{eye}")
}

/// A puff of dirt drawn with `dustTextures[image]`.
pub fn dust_bake_name(image: usize) -> String {
    format!("En_Tk/dust{image}")
}

/// The meshes `EnTk_Draw` draws: the skeleton with each eye; `EnTkEff_Draw`'s puff with each
/// dust texture (`Gfx_SetupDL_25Xlu`, then `SETUPDL_0`, whose modes replace it; its colours).
pub fn bakes() -> Vec<MeshBake> {
    let mut v: Vec<MeshBake> = (0..EYE_TEXTURES.len())
        .map(|eye| MeshBake {
            name: bake_name(eye),
            object: OBJECT.into(),
            segments: vec![(0x08, BakeSegment::Texture { file: OBJECT.into(), symbol: EYE_TEXTURES[eye].into() })],
            prelude: Vec::new(),
            body: BakeBody::Skeleton { file: OBJECT.into(), symbol: SKEL.into(), limbs: Vec::new() },
        })
        .collect();
    let mut setup = setup_dl::setup_dl_0();
    setup.end();
    v.extend((0..DUST_TEXTURES.len()).map(|image| MeshBake {
        name: dust_bake_name(image),
        object: OBJECT.into(),
        segments: vec![
            (SEG_TEX, BakeSegment::Texture { file: "gameplay_keep".into(), symbol: DUST_TEXTURES[image].into() }),
            (SEG_SETUP, BakeSegment::Commands(setup.0.clone())),
            (SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true }),
        ],
        prelude: vec![SEG_SETUP, SEG_COLOR],
        body: BakeBody::DLists(vec![(OBJECT.into(), "gDampeEff1DL".into()), (OBJECT.into(), "gDampeEff2DL".into())]),
    }));
    v
}

/// `EnTkEff`: a puff of dirt.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EnTkEff {
    pub active: u8,
    pub time_left: u8,
    pub time_total: u8,
    pub size: f32,
    pub growth: f32,
    pub pos: Vec3,
    pub speed: Vec3,
    pub accel: Vec3,
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Rest,
    Walk,
    Dig,
}

struct Anims {
    dig: Anim,
    walk: Anim,
    rest: Anim,
}

/// `DECR`: down by one unless 0; the new value.
fn decr(x: &mut i16) -> i16 {
    if *x != 0 {
        *x -= 1;
    }
    *x
}

pub struct EnTk {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Arc<Skeleton>,
    anims: Anims,
    pub action: Action,
    pub collider: ColliderCylinder,
    /// `interactInfo`: only its `talkState` is used.
    pub interact_info: NpcTrack,
    pub valid_dig_here: u8,
    /// `rewardCount`: this round's greens, blues, reds and purples.
    pub reward_count: [u8; 4],
    pub current_spot: Option<ActorHandle>,
    pub current_reward: i32,
    pub blink_cycles: i16,
    pub reward_timer: i16,
    pub action_countdown: i16,
    /// `h_21E`: the animation's yaw of the head and jaw (limbs 15 and 16), from the last draw.
    pub h_21e: i16,
    pub eye_texture_idx: i16,
    pub blink_countdown: i16,
    pub head_rot: i16,
    pub current_waypoint: i16,
    /// `v3f_304`: the shovel's blade, from the last draw.
    pub v3f_304: Vec3,
    pub eff: [EnTkEff; 20],
}

impl EnTk {
    /// `EnTk_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = (|| -> anyhow::Result<_> {
            Ok((assets.skeleton(OBJECT, SKEL)?, Anims { dig: assets.animation(OBJECT, DIG_ANIM)?, walk: assets.animation(OBJECT, WALK_ANIM)?, rest: assets.animation(OBJECT, REST_ANIM)? }))
        })();
        let (skeleton, anims) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Tk: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        // SkelAnime_InitFlex(.., NULL, .., 18), Animation_Change(gDampeRestAnim, 1, 0, its last
        // frame, ANIMMODE_LOOP, 0): Animation_PlayLoop's.
        let mut this = EnTk {
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(anims.rest.clone())),
            skeleton,
            anims,
            action: Action::Rest,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            interact_info: NpcTrack::default(),
            valid_dig_here: 0,
            reward_count: [0; 4],
            current_spot: None,
            current_reward: 0,
            blink_cycles: 0,
            reward_timer: 0,
            action_countdown: 0,
            h_21e: 0,
            eye_texture_idx: 0,
            blink_countdown: 0,
            head_rot: 0,
            current_waypoint: 0,
            v3f_304: Vec3::ZERO,
            eff: [EnTkEff::default(); 20],
            actor,
        };
        // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 24).
        this.actor.shape_y_offset = 0.0;
        this.actor.col_chk_info.set_info2(None, &COL_CHK_INFO_INIT);
        let t = play.save.day_time as i32;
        if t <= oot_game::env::clock_time(18, 0) || t >= oot_game::env::clock_time(21, 0) || play.save.adult || play.scene_id != SCENE_GRAVEYARD {
            this.actor.kill();
            return Box::new(this);
        }
        this.actor.scale = Vec3::splat(0.01);
        this.actor.target_mode = ATTENTION_RANGE_6;
        this.actor.gravity = -0.1;
        this.current_reward = -1;
        this.current_spot = None;
        this.action = Action::Rest;
        Box::new(this)
    }

    /// `EnTkEff_Create`: in the first slot not active.
    fn eff_create(&mut self, pos: Vec3, speed: Vec3, accel: Vec3, duration: u8, size: f32, growth: f32) {
        if let Some(eff) = self.eff.iter_mut().find(|e| e.active != 1) {
            eff.size = size;
            eff.growth = growth;
            eff.time_total = duration;
            eff.time_left = duration;
            eff.active = 1;
            eff.pos = pos;
            eff.accel = accel;
            eff.speed = speed;
        }
    }

    /// `EnTkEff_Update`.
    fn eff_update(&mut self, rand: &mut Rand) {
        for eff in self.eff.iter_mut() {
            if eff.active != 0 {
                eff.time_left = eff.time_left.wrapping_sub(1);
                if eff.time_left == 0 {
                    eff.active = 0;
                }
                eff.accel.x = rand.zero_one() * 0.4 - 0.2;
                eff.accel.z = rand.zero_one() * 0.4 - 0.2;
                eff.pos += eff.speed;
                eff.speed += eff.accel;
                eff.size += eff.growth;
            }
        }
    }

    /// `EnTkEff_CreateDflt`: still, rising by 0.3 and up to `y_accel_max` more.
    fn eff_create_dflt(&mut self, rand: &mut Rand, pos: Vec3, duration: u8, size: f32, growth: f32, y_accel_max: f32) {
        let speed = Vec3::ZERO;
        let mut accel = Vec3::new(0.0, 0.3, 0.0);
        accel.y += rand.zero_one() * y_accel_max;
        self.eff_create(pos, speed, accel, duration, size, growth);
    }

    /// `EnTk_RestAnim`.
    fn rest_anim(&mut self, play: &mut PlayState) {
        let last = self.anims.rest.last_frame();
        self.skel.change(self.anims.rest.clone(), 1.0, 0.0, last, ANIMMODE_LOOP, -10.0);
        self.action_countdown = play.rand.s16_offset(60, 60);
        self.actor.speed_xz = 0.0;
    }

    /// `EnTk_WalkAnim`. Its end frame is the rest's (`Animation_GetLastFrame(&gDampeRestAnim)`),
    /// which `ANIMMODE_LOOP` replaces with the walk's own anyway.
    fn walk_anim(&mut self, play: &mut PlayState) {
        let last = self.anims.rest.last_frame();
        self.skel.change(self.anims.walk.clone(), 1.0, 0.0, last, ANIMMODE_LOOP, -10.0);
        self.action_countdown = play.rand.s16_offset(240, 240);
    }

    /// `EnTk_DigAnim`: a good dig if a spot is here.
    fn dig_anim(&mut self, play: &mut PlayState) {
        let last = self.anims.dig.last_frame();
        self.skel.change(self.anims.dig.clone(), 1.0, 0.0, last, ANIMMODE_LOOP, -10.0);
        if self.check_next_spot(play) >= 0 {
            self.valid_dig_here = 1;
        }
    }

    /// `EnTk_UpdateEyes`: a blink of the three eyes, two or three times, every 30 to 59 frames.
    fn update_eyes(&mut self, rand: &mut Rand) {
        if decr(&mut self.blink_countdown) == 0 {
            self.eye_texture_idx += 1;
            if self.eye_texture_idx > 2 {
                self.blink_cycles -= 1;
                if self.blink_cycles < 0 {
                    self.blink_countdown = rand.s16_offset(30, 30);
                    self.blink_cycles = 2;
                    if rand.zero_one() > 0.5 {
                        self.blink_cycles += 1;
                    }
                }
                self.eye_texture_idx = 0;
            }
        }
    }

    /// `EnTk_CheckFacingPlayer`: Link within 100 and within 0x1554 of where his face points.
    pub fn check_facing_player(&self) -> bool {
        if self.actor.xyz_dist_to_player_sq > 100.0 * 100.0 {
            return false;
        }
        let v0 = self.actor.shape_rot.y.wrapping_sub(self.h_21e).wrapping_sub(self.head_rot);
        let v1 = self.actor.yaw_towards_player.wrapping_sub(v0);
        (v1 as i32).abs() < 0x1554
    }

    /// `Actor_WorldDistXZToActor`.
    fn dist_xz(a: &Actor, b: &Actor) -> f32 {
        let (dx, dz) = (b.world_pos.x - a.world_pos.x, b.world_pos.z - a.world_pos.z);
        (dx * dx + dz * dz).sqrt()
    }

    /// `EnTk_CheckNextSpot`: the first `En_It` in the prop list, other than the current spot,
    /// within 40 across and not more than 10 above his floor: it's the current spot now, and its
    /// params (0x0D05) returned; else -1.
    pub fn check_next_spot(&mut self, play: &PlayState) -> i32 {
        for &h in play.actors.category(ACTORCAT_PROP) {
            let Some(prop) = play.actors.actor(h) else { continue };
            if prop.id != ACTOR_EN_IT {
                continue;
            }
            if Some(h) == self.current_spot {
                continue;
            }
            let dy = prop.world_pos.y - self.actor.floor_height;
            let dxz = Self::dist_xz(&self.actor, prop);
            if dxz > 40.0 || dy > 10.0 {
                continue;
            }
            self.current_spot = Some(h);
            // schema: En_It's params, which its init sets to 0x0D05 (no placement field)
            return prop.params as i32;
        }
        -1
    }

    /// `EnTk_CheckCurrentSpot`: the current spot forgotten once he's off it.
    fn check_current_spot(&mut self, play: &PlayState) {
        if let Some(h) = self.current_spot {
            let off = match play.actors.actor(h) {
                Some(spot) => {
                    let dy = spot.world_pos.y - self.actor.floor_height;
                    let dxz = Self::dist_xz(&self.actor, spot);
                    dxz > 40.0 || dy > 10.0
                }
                // (The C's pointer would dangle; the spots are never deleted.)
                None => true,
            };
            if off {
                self.current_spot = None;
            }
        }
    }

    /// `EnTk_Step`: his speed on the walk's stepping frames (36 to 47, 10 to 21), a half sine up
    /// to 2; his footstep at frames 0 and 25.
    fn step(&mut self, play: &mut PlayState) -> f32 {
        const STEP_FRAMES: [f32; 2] = [36.0, 10.0];
        if self.skel.cur_frame == 0.0 || self.skel.cur_frame == 25.0 {
            audio_play_actor_sfx2(play, NA_SE_EN_MORIBLIN_WALK);
        }
        if !self.skel.is(WALK_ANIM) {
            return 0.0;
        }
        let mut a1 = self.skel.cur_frame;
        let Some(&step) = STEP_FRAMES.iter().find(|&&s| a1 < s + 12.0 && a1 >= s) else { return 0.0 };
        a1 = (32768.0 / 12.0) * (a1 - step);
        sin_s(a1 as i32 as i16) * 2.0
    }

    /// `EnTk_Orient`: turned towards path 0's current point; on to the next within √10 (false).
    fn orient(&mut self, play: &PlayState) -> bool {
        if PARAMS.get(self.actor.params) < 0 {
            return true;
        }
        let Some(path) = play.setup_path_list().first() else {
            log::warn!("En_Tk: the scene has no path 0");
            return true;
        };
        let point = path.point(self.current_waypoint as usize);
        let dx = point.x - self.actor.world_pos.x;
        let dz = point.z - self.actor.world_pos.z;
        smooth_step_to_s(&mut self.actor.shape_rot.y, oot_game::sys_matrix::rad_to_binang(oot_game::camera::f_atan2f(dx, dz)), 10, 1000, 1);
        self.actor.world_rot = self.actor.shape_rot;
        if dx * dx + dz * dz < 10.0 {
            self.current_waypoint += 1;
            if self.current_waypoint as usize >= path.count() {
                self.current_waypoint = 0;
            }
            false
        } else {
            true
        }
    }

    /// `EnTk_GetTextId`: 0x5019 once `INFTABLE_D9` is set, else 0x5018.
    pub fn get_text_id(play: &PlayState) -> u16 {
        // MaskReaction_GetTextId(play, MASK_REACTION_SET_DAMPE): no masks, so 0 (BACKLOG #39).
        if play.save.get_inf_table(INFTABLE_D9) { 0x5019 } else { 0x5018 }
    }

    /// `EnTk_UpdateTalkState`.
    fn update_talk_state(play: &mut PlayState, thisx: &mut Actor) -> i16 {
        let mut talk_state = NPC_TALK_STATE_TALKING;
        match play.message_state() {
            TEXT_STATE_CLOSING => {
                // @bug (game) 0x5028 ("I am the boss of the carpenters ...") isn't his text, so
                // INFTABLE_D8 is never set here.
                if thisx.text_id == 0x5028 {
                    play.save.set_inf_table(INFTABLE_D8);
                }
                talk_state = NPC_TALK_STATE_IDLE;
            }
            TEXT_STATE_CHOICE => {
                if play.message_should_advance() && (thisx.text_id == 0x5018 || thisx.text_id == 0x5019) {
                    if play.msg_ctx.choice_index == 1 {
                        // "Thanks a lot!"
                        thisx.text_id = 0x0084;
                    } else if play.save.rupees < 10 {
                        // "You don't have enough Rupees!"
                        thisx.text_id = 0x0085;
                    } else {
                        play.msg_ctx.msg_mode = MSGMODE_PAUSED;
                        rupees_change_by(&mut play.save, -10);
                        play.save.set_inf_table(INFTABLE_D9);
                        return NPC_TALK_STATE_ACTION;
                    }
                    play.continue_textbox(thisx.text_id);
                    play.save.set_inf_table(INFTABLE_D9);
                }
            }
            TEXT_STATE_EVENT => {
                if play.message_should_advance() && (thisx.text_id == 0x0084 || thisx.text_id == 0x0085) {
                    play.with_msg(|m, f| m.close_textbox(f.audio));
                    talk_state = NPC_TALK_STATE_IDLE;
                }
            }
            _ => {}
        }
        talk_state
    }

    /// `Npc_UpdateTalking(.., collider.dim.radius + 30, EnTk_GetTextId, EnTk_UpdateTalkState)`.
    fn npc_update_talking(&mut self, play: &mut PlayState) {
        let mut talk_state = self.interact_info.talk_state;
        let range = self.collider.dim.radius as f32 + 30.0;
        talk_update(play, &mut self.actor, &mut talk_state, range, |play, _| Self::get_text_id(play), Self::update_talk_state);
        self.interact_info.talk_state = talk_state;
    }

    /// `EnTk_ChooseReward`: a kind at random (under 0.4 green, 0.7 blue, 0.9 red, else purple)
    /// while this round has it left, else the cheapest left; all fifteen given, a green starts a
    /// new round.
    pub fn choose_reward(&mut self, rand: &mut Rand) -> i32 {
        const LIMITS: [u8; 4] = [8, 4, 2, 1];
        let luck = rand.zero_one();
        // The 0.7 and 0.9 are doubles in the C.
        let reward = if luck < 0.4 {
            0
        } else if (luck as f64) < 0.7 {
            1
        } else if (luck as f64) < 0.9 {
            2
        } else {
            3
        };
        if self.reward_count[reward] < LIMITS[reward] {
            self.reward_count[reward] += 1;
            return reward as i32;
        }
        for r in 0..4 {
            if self.reward_count[r] < LIMITS[r] {
                self.reward_count[r] += 1;
                return r as i32;
            }
        }
        self.reward_count = [1, 0, 0, 0];
        0
    }

    /// `EnTk_DigEff`: from the dig's frame 32 to 39, a puff of dirt at the blade (within 6
    /// across and 4 up or down).
    fn dig_eff(&mut self, rand: &mut Rand) {
        if self.skel.cur_frame >= 32.0 && self.skel.cur_frame < 40.0 {
            let x = (rand.zero_one() - 0.5) * 12.0 + self.v3f_304.x;
            let y = (rand.zero_one() - 0.5) * 8.0 + self.v3f_304.y;
            let z = (rand.zero_one() - 0.5) * 12.0 + self.v3f_304.z;
            self.eff_create_dflt(rand, Vec3::new(x, y, z), 12, 0.2, 0.1, 0.0);
        }
    }

    /// `EnTk_Rest`.
    fn rest(&mut self, play: &mut PlayState) {
        let face = |s: &Self| s.actor.yaw_towards_player.wrapping_sub(s.actor.shape_rot.y.wrapping_sub(s.h_21e));
        let v1: i16;
        if self.interact_info.talk_state != NPC_TALK_STATE_IDLE {
            v1 = face(self);
            if self.interact_info.talk_state == NPC_TALK_STATE_ACTION {
                self.dig_anim(play);
                self.interact_info.talk_state = NPC_TALK_STATE_IDLE;
                self.action = Action::Dig;
                return;
            }
            self.npc_update_talking(play);
        } else if self.check_facing_player() {
            v1 = face(self);
            self.action_countdown = 0;
            self.npc_update_talking(play);
        } else if process_talk_request(&mut self.actor) {
            v1 = face(self);
            self.action_countdown = 0;
            self.interact_info.talk_state = NPC_TALK_STATE_TALKING;
        } else if decr(&mut self.action_countdown) == 0 {
            self.walk_anim(play);
            self.action = Action::Walk;
            // @bug (game) v1 is uninitialised past this branch: taken as 0.
            v1 = 0;
        } else {
            v1 = 0;
        }
        let a1 = (-(v1 as i32)).clamp(1270, 10730) as i16;
        smooth_step_to_s(&mut self.head_rot, a1, 6, 1000, 1);
    }

    /// `EnTk_Walk`.
    fn walk(&mut self, play: &mut PlayState) {
        if self.interact_info.talk_state == NPC_TALK_STATE_ACTION {
            self.dig_anim(play);
            self.interact_info.talk_state = NPC_TALK_STATE_IDLE;
            self.action = Action::Dig;
        } else {
            self.actor.speed_xz = self.step(play);
            self.orient(play);
            smooth_step_to_s(&mut self.head_rot, 0, 6, 1000, 1);
            self.check_current_spot(play);
            decr(&mut self.action_countdown);
            if self.check_facing_player() || self.action_countdown == 0 {
                self.rest_anim(play);
                self.action = Action::Rest;
            }
        }
    }

    /// `EnTk_Dig`.
    fn dig(&mut self, play: &mut PlayState) {
        self.dig_eff(&mut play.rand);
        if self.skel.cur_frame == DIG_REWARD_FRAME {
            // What's gonna come out?
            audio_play_actor_sfx2(play, NA_SE_EV_DIG_UP);
            self.reward_timer = 0;
            if self.valid_dig_here == 1 {
                let reward_pos = self.reward_pos();
                self.current_reward = self.choose_reward(&mut play.rand);
                if self.current_reward == 3 {
                    // The purple rupee is the piece of heart on the first grand prize.
                    if !play.save.get_item_get_inf(ITEMGETINF_1C) {
                        play.save.set_item_get_inf(ITEMGETINF_1C);
                        self.current_reward = 4;
                    }
                }
                item_drop_collectible(play, reward_pos, REWARD_PARAMS[self.current_reward as usize]);
            }
        }
        if self.skel.cur_frame >= DIG_REWARD_FRAME && self.reward_timer == 10 {
            // The prize's sound shortly after.
            if self.valid_dig_here == 0 {
                audio_play_actor_sfx2(play, NA_SE_SY_ERROR);
            } else if self.current_reward == 4 {
                play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
            } else {
                audio_play_actor_sfx2(play, NA_SE_SY_TRE_BOX_APPEAR);
            }
        }
        self.reward_timer = self.reward_timer.wrapping_add(1);
        if self.skel.on_frame(self.skel.end_frame) {
            if self.current_reward < 0 {
                // "Nope, nothing here!"
                play.start_textbox(0x501A, None);
            } else {
                play.with_msg(|m, f| m.close_textbox(f.audio));
            }
            self.rest_anim(play);
            self.current_reward = -1;
            self.valid_dig_here = 0;
            self.action = Action::Rest;
        }
    }

    /// Where `EnTk_Dig` drops the prize: (0, 0, -40) turned by `Matrix_RotateY(shape.rot.y,
    /// MTXMODE_NEW)`, from his place.
    pub fn reward_pos(&self) -> Vec3 {
        let mut m = MtxF::IDENTITY;
        // @bug (game) Matrix_RotateY takes radians; his yaw goes in as its binary angle, so the
        // prize lands 40 from him at sin and cos of that many radians, not behind him.
        m.rotate_y(self.actor.shape_rot.y as f32);
        m.mult_vec3f(Vec3::new(0.0, 0.0, -40.0)) + self.actor.world_pos
    }

    /// The limb matrices `EnTk_Draw` draws with: the jaw (limb 16) turned by `head_rot`
    /// (`EnTk_OverrideLimbDraw`).
    fn pose(&self, joints: &[[i16; 3]], head_rot: i16) -> Vec<Mat4> {
        self.skeleton.pose_override(joints, |limb, _pos, rot| {
            if limb == LIMB_JAW {
                rot[1] = rot[1].wrapping_add(head_rot);
            }
            Mat4::IDENTITY
        })
    }

    /// `EnTkEff_Draw`: each active puff facing the camera, its alpha and dust frame by the time
    /// it has left.
    fn eff_draw(&self, view: &ViewInfo, out: &mut DrawOut) {
        for eff in self.eff.iter().filter(|e| e.active != 0) {
            let alpha = (eff.time_left as f32 * (255.0 / eff.time_total as f32)) as i16;
            let image = (eff.time_left as f32 * (DUST_TEXTURES.len() as f32 / eff.time_total as f32)) as i16;
            let m = oot_game::draw::replace_rotation(Mat4::from_translation(eff.pos), view.billboard) * Mat4::from_scale(Vec3::new(eff.size, eff.size, 1.0));
            let mut sv = SegmentValues::default();
            sv.prim[SEG_COLOR as usize] = Some([DUST_PRIM[0], DUST_PRIM[1], DUST_PRIM[2], alpha as u8]);
            sv.env[SEG_COLOR as usize] = Some(DUST_ENV);
            let image = (image.max(0) as usize).min(DUST_TEXTURES.len() - 1);
            out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(&dust_bake_name(image))), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
        }
    }
}

impl ActorImpl for EnTk {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnTk_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        self.skel.update();
        // Actor_MoveXZGravity.
        self.actor.move_forward();
        self.actor.update_bg_check_info(&play.col, 40.0, 10.0, 0.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
        match self.action {
            Action::Rest => self.rest(play),
            Action::Walk => self.walk(play),
            Action::Dig => self.dig(play),
        }
        self.eff_update(&mut play.rand);
        self.update_eyes(&mut play.rand);
    }
    /// What `EnTk_Draw` writes back: `h_21E` (`EnTk_OverrideLimbDraw`, limbs 15 and 16), the
    /// focus on the jaw and the blade's point (0, 0, 4600) on limb 14 (`EnTk_PostLimbDraw`).
    fn draw_update(&mut self, _play: &mut PlayState) {
        let joints = &self.skel.joint_table;
        let (Some(head), Some(jaw)) = (joints.get(LIMB_HEAD), joints.get(LIMB_JAW)) else { return };
        self.h_21e = head[1].wrapping_add(jaw[1]);
        let bones = self.pose(joints, self.head_rot);
        let m = actor_draw_matrix(&RenderState::of(&self.actor));
        if let Some(b) = bones.get(LIMB_JAW - 1) {
            self.actor.focus_pos = (m * *b).transform_point3(Vec3::ZERO);
        }
        if let Some(b) = bones.get(LIMB_NECK - 1) {
            self.v3f_304 = (m * *b).transform_point3(Vec3::new(0.0, 0.0, 4600.0));
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.angles = vec![self.head_rot];
        rs.switches = vec![self.eye_texture_idx.clamp(0, 2) as u32];
        rs
    }
    /// `EnTk_Draw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        self.eff_draw(view, out);
        let Some(joints) = &rs.joints else { return };
        let (Some(&head_rot), Some(&eye)) = (rs.angles.first(), rs.switches.first()) else { return };
        let bones = self.pose(&joints.rot, head_rot);
        let model = actor_draw_matrix(rs);
        // func_80B1D200 at limb 14 (EnTk_PostLimbDraw).
        let shovel = bones.get(LIMB_NECK - 1).map(|b| model * *b);
        let eye = (eye as usize).min(EYE_TEXTURES.len() - 1);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(eye))), transform: model, bones, params: Default::default() });
        if let Some(m) = shovel {
            out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh(OBJECT, "gDampeShovelDL")), m));
        }
        // ActorShadow_DrawCircle (shadowScale 24): the circle shadow's stand-in.
        let (floor, _) = play.col.entity_raycast_down(rs.pos + Vec3::Y * 20.0);
        let shadow = Mat4::from_translation(Vec3::new(rs.pos.x, floor + 0.3, rs.pos.z)) * Mat4::from_scale(Vec3::new(24.0, 1.0, 24.0));
        out.xlu.push(DrawCmd::new(MeshKey::named(oot_game::play::builtin::SHADOW), shadow));
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
