//! `En_Po_Relay` (`ovl_En_Po_Relay/z_en_po_relay.c`): Dampé's ghost, who races Link through his
//! grave (`hakasitarelay_scene`, every layer: room 0 at (-63, -530, -2177), params 0x0007).
//!
//! - **Only one** (`sAlreadySpawned`, the overlay's static): a second at init is killed. He belongs
//!   to no room (`room` -1, `EnPoRelay_SetupIdle`): he stays as Link and he cross the rooms.
//! - **Waiting** (`EnPoRelay_Idle`): turning towards Link (0x100 a frame); Link within 250 is
//!   offered his text at once (`ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED`): 0x5041 ("Heh heh heh, young
//!   man! Are you fast on your feet? ... Let's have a race! Follow me if you dare!", the scene's
//!   prefix 0x5000 on 65).
//! - **The start** (`EnPoRelay_Talk`, `EnPoRelay_SetupRace`): as that box closes, his text is
//!   0x502F (the race's time) from now on, the timer starts counting up (`Interface_SetTimer(0)`),
//!   he notes whether Link has the hookshot (`hookshotSlotFull`), laughs (`NA_SE_EN_PO_LAUGH`) and
//!   can't be locked on to.
//! - **The race** (`EnPoRelay_Race`): along his 28 points (`sPathPoints`), turning towards the next
//!   (`Math_SmoothStepToS` by 2, 0x1000 to 0x100), on to the next within 40 of it. Reaching point 8
//!   sets the switch 0x35 (the first door, `Bg_Relay_Objects`, rises), point 16 sets 0x36, point
//!   24 sets 0x37; past point 27 the race ends. His speed (`Math_ApproachF` by half, 1.5 at most a
//!   frame) up to point 22: Link's own speed (1.2 or 1.4 times it in three squares where Link takes
//!   another way that meets his later; within 150, 1 or 1.2 times; within 300, 0.8 or 1 times; the
//!   larger with the hookshot), else 3.5 (4.5 with the hookshot); plus 1, and 0.02 for each unit
//!   Link is within 250. From point 23, 3.5. Every 32 frames at the soonest, 3% a frame, he drops
//!   a flame (`En_Honotrap` `HONOTRAP_TYPE_FLAME_DROP`) up to point 22: beside him (30 to his left,
//!   right or at him, by `Rand_ZeroOne`), thrown back and out. He spins as the 32 frames run.
//! - **The end** (`EnPoRelay_EndRace`): turned to face -0x4000 (0x800 a frame), still. Link within
//!   150 in front of him (0x3000) can talk (A): 0x502F, the time ("The time of this race was
//!   ...!!"), then on its event (`EnPoRelay_Talk2`) 0x502D (the first race: "As a reward, I'm going
//!   to give you my treasure ... the Hookshot") or 0x502E (Link had the hookshot at the start:
//!   "You're back, young man... Was the present I gave to you fun and useful?"). Link already in
//!   room 5 (past him) instead: he's gone and the timer off.
//! - **Disappearing** (`EnPoRelay_DisappearAndReward`): the timer off; three columns of blue
//!   flames (`EffectSsDeadDb`) for 7 frames, `NA_SE_EN_EXTINCT`, rising 10 a frame and shrinking
//!   (0.001 a frame from 0.01). Gone, the reward: **the first race** (no hookshot at its start)
//!   sets room 4's temporary clear flag (`Flags_SetTempClear(play, 4)`: room 4's chest, `En_Box`
//!   0x1100, the hookshot, appears) and the high score (`HIGH_SCORE(HS_DAMPE_RACE)`) to the time;
//!   **a later race** keeps the better time and drops, where he is on the floor, the piece of
//!   heart (`Item_DropCollectible2` with his collectible flag, hovering) for a time of 60 seconds or
//!   less while its collectible flag isn't set; else a red rupee (`En_Item00` 2).
//!
//! **Drawn** as `EnPoRelay_Draw` does: the skeleton (`gDampeSkel`, `gDampeFloatAnim` looping) with
//! his eye on segment 8 (`sEyesTextures`, cycling every frame) from `En_Tk`'s bakes of the same
//! skeleton (`en_tk::bake_name`); at limb 14 the lantern (`gDampeLanternDL`) in `lightColor` (a
//! light orange to white, `Rand_ZeroOne` each drawn frame) at alpha 128, and his point light there
//! (1200 up the limb, radius 200, in `lightColor`); at limb 8 his halo (`gDampeHaloDL`). The circle
//! shadow (42) is the shape's. He bobs 8 up and down (`bobTimer`, 32 frames) 45 above his next
//! point's height (`EnPoRelay_CorrectY`).
//!
//! The whole overlay is ported. Not ported (the framework's): the point light's shading
//! (`Lights_BindAll`'s lights aren't drawn). `Item_DropCollectible2` is `en_item00.rs`'s.
//!
//! Params: `collectibleFlag` bits 0 to 5 (the piece of heart's collectible flag, 7 as placed; the
//! init writes the params down to them, `params &= 0x3F`).
//! Flags: sets the switches 0x35, 0x36 and 0x37 at his points 8, 16 and 24 (the race's doors,
//! `Bg_Relay_Objects`); sets room 4's temporary clear flag after the first race (the hookshot's
//! chest); reads the collectible flag `collectibleFlag` (the piece of heart, which `En_Item00`
//! sets); reads and sets `HIGH_SCORE(HS_DAMPE_RACE)`; reads the hookshot (`INV_CONTENT`).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{approach_f, cos_s, scaled_step_to_s, sin_s, smooth_step_to_s, step_to_f, vec3f_yaw};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, TextLine};
use oot_game::audio::sfx::{NA_SE_EN_EXTINCT, SFX_FLAG};
use oot_game::collision_check::*;
use oot_game::item::{ITEM_HOOKSHOT, ITEM_NONE};
use oot_game::lights::{LightInfo, LightNode};
use oot_game::message::{HS_DAMPE_RACE, TEXT_STATE_EVENT};
use oot_game::npc::{actor_set_text_with_prefix, offer_talk, process_talk_request, textbox_is_closing};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::save::TIMER_STATE_OFF;
use oot_game::skelanime_std::{Anim, SkelAnimeStd};

use crate::bg_treemouth::actor_is_facing_and_near_player;
use crate::en_honotrap::{ACTOR_EN_HONOTRAP, HONOTRAP_TYPE_FLAME_DROP};
use crate::en_item00::{ACTOR_EN_ITEM00, ITEM00_HEART_PIECE, ITEM00_RUPEE_RED, item_drop_collectible2};

/// `ACTOR_EN_PO_RELAY` (`actor_table.h`: 0x0122).
pub const ACTOR_EN_PO_RELAY: i16 = 0x0122;
/// `OBJECT_TK` (`object_table.h`: 0x0089).
pub const OBJECT: &str = "object_tk";

/// `ACTOR_FLAG_IGNORE_QUAKE` (`actor.h`: 1 << 12).
pub const ACTOR_FLAG_IGNORE_QUAKE: u32 = 1 << 12;

/// `FLAGS`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_PO_RELAY,
    name: "En_Po_Relay",
    category: ACTORCAT_NPC,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_IGNORE_QUAKE | ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `this->actor.params & 0x3F` (`EnPoRelay_Init` keeps only these bits): the piece of heart's
/// collectible flag.
pub const COLLECTIBLE_FLAG: Field = Field::params("collectibleFlag", 0, 6, Kind::CollectibleFlag, "the piece of heart's: dropped after a later race of 60 seconds or less until it's set");

/// En_Po_Relay's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[COLLECTIBLE_FLAG],
    flags: &[
        FlagUse::new(Flag::Switch("0x35")).sets().note("the first door: set as he reaches his point 8"),
        FlagUse::new(Flag::Switch("0x36")).sets().note("the second door: set at his point 16"),
        FlagUse::new(Flag::Switch("0x37")).sets().note("the third door: set at his point 24"),
        FlagUse::new(Flag::TempClear).sets().note("room 4's, after the first race (no hookshot at its start): the hookshot's chest appears"),
        FlagUse::new(Flag::Collectible("collectibleFlag")).reads().note("the piece of heart (set by En_Item00 as it's taken): dropped only while unset"),
        FlagUse::new(Flag::Other("HIGH_SCORE(HS_DAMPE_RACE): the save's highScores[6], the race's best in seconds")).reads().sets().note("the first race's time; a later race's when better"),
        FlagUse::new(Flag::Other("INV_CONTENT(ITEM_HOOKSHOT)")).reads().note("at the race's start: the first race or a later one"),
    ],
    todo: false,
    objects: &[],
    texts: &[
        TextLine::new(0x5041).cond("Link within 250 of him, before the race").note("Heh heh heh, young man! Let's have a race! (starts it as it closes)"),
        TextLine::new(0x502F).cond("talked to at the race's end").note("the race's time; goes on to 0x502D or 0x502E"),
        TextLine::new(0x502D).cond("the first race (no hookshot at its start)").note("as a reward, my treasure: the Hookshot"),
        TextLine::new(0x502E).cond("a later race (the hookshot at its start)").note("you're back, young man... was the present fun and useful?"),
    ],
};

/// `NAVI_ENEMY_DAMPES_GHOST` (`actor.h`: 0x4F).
pub const NAVI_ENEMY_DAMPES_GHOST: u8 = 0x4F;

/// The sounds (`enemybank_table.h`).
pub const NA_SE_EN_PO_FLY: u16 = 0x3871;
pub const NA_SE_EN_PO_AWAY: u16 = 0x3872;
pub const NA_SE_EN_PO_LAUGH: u16 = 0x38EC;

/// The base text ids (`Actor_SetTextWithPrefix`, the grave's prefix 0x5000): his greeting (65 in
/// the C), the race's time, the later race's end and the first's.
// schema: 0x5041 with the scene's prefix
pub const MSG_GREETING: i16 = 65;
// schema: 0x502F with the scene's prefix
pub const MSG_TIME: i16 = 0x2F;
// schema: 0x502E with the scene's prefix
pub const MSG_LATER_RACE: i16 = 0x2E;
// schema: 0x502D with the scene's prefix
pub const MSG_FIRST_RACE: i16 = 0x2D;

/// `sPathPoints`: his way through the grave.
pub const PATH_POINTS: [[i16; 3]; 28] = [
    [-60, -530, -2950],
    [390, -500, -2950],
    [390, -500, -3850],
    [210, -530, -3850],
    [210, -610, -4390],
    [570, -570, -4390],
    [570, -570, -4840],
    [210, -570, -4840],
    [210, -570, -5170],
    [210, -570, -5470],
    [570, -570, -5470],
    [570, -570, -5200],
    [1260, -610, -5200],
    [1650, -670, -4840],
    [1650, -720, -4480],
    [2010, -730, -4480],
    [2010, -730, -4240],
    [2010, -730, -3580],
    [1650, -700, -3580],
    [1650, -660, -3130],
    [2190, -660, -3130],
    [2190, -590, -2590],
    [2460, -560, -2590],
    [2900, -410, -2190],
    [2894, -410, -1922],
    [2890, -410, -1670],
    [2890, -360, -1540],
    [2990, -360, -1540],
];

/// What `sPathPoints[28]` reads, past the array (`EnPoRelay_Race` after its last point): the next
/// data, `En_Po_Relay_Profile`'s first six bytes (the id 0x0122, the category 4 and a pad byte,
/// the top half of `FLAGS`).
const PATH_POINT_PAST_END: [i16; 3] = [ACTOR_EN_PO_RELAY, 0x0400, (PROFILE.flags >> 16) as i16];

/// The doors' switch flags, set as he reaches the point before `pathPoint` 9, 17 and 25.
pub const SWITCH_FIRST_DOOR: i32 = 0x35;
pub const SWITCH_SECOND_DOOR: i32 = 0x36;
pub const SWITCH_THIRD_DOOR: i32 = 0x37;

/// `sCylinderInit`.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0000, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 30, height: 52, y_shift: 0, pos: [0, 0, 0] },
};

/// `sDisappearParticlesVelocity`, `sDisappearParticlesAccel`.
const DISAPPEAR_PARTICLES_VELOCITY: Vec3 = Vec3::new(0.0, 1.5, 0.0);
const DISAPPEAR_PARTICLES_ACCEL: Vec3 = Vec3::new(0.0, 0.0, 0.0);
/// `sLanternLightOffset`: the light, from the lantern limb.
const LANTERN_LIGHT_OFFSET: Vec3 = Vec3::new(0.0, 1200.0, 0.0);

/// `DampeEyes`.
pub const DAMPE_EYES_FULLY_OPEN: u16 = 0;
pub const DAMPE_EYES_HALF_OPEN: u16 = 1;
pub const DAMPE_EYES_CLOSED: u16 = 2;
pub const DAMPE_EYES_MAX: u16 = 3;

/// `LIMB_OBJECT_TK_00BDCC` (the lantern), `LIMB_OBJECT_TK_00BD84` (the halo): 1-based.
pub const LIMB_LANTERN: usize = 14;
pub const LIMB_HALO: usize = 8;

const SKEL: &str = "gDampeSkel";
const FLOAT_ANIM: &str = "gDampeFloatAnim";

/// The lantern's `gDPSetEnvColor`, run before `gDampeLanternDL`.
const SEG_ENV: u8 = 0x0E;
/// `gDampeLanternDL` in the env colour.
pub const LANTERN_BAKE: &str = "En_Po_Relay/lantern";
/// `gDampeHaloDL`.
pub const HALO_BAKE: &str = "En_Po_Relay/halo";

/// The lantern's and the halo's bakes (the skeleton's are `En_Tk`'s: `en_tk::bake_name`).
pub fn bakes() -> Vec<MeshBake> {
    vec![
        MeshBake {
            name: LANTERN_BAKE.into(),
            object: OBJECT.into(),
            segments: vec![(SEG_ENV, BakeSegment::DynamicColor { env: true, prim: false })],
            prelude: vec![SEG_ENV],
            body: BakeBody::DLists(vec![(OBJECT.into(), "gDampeLanternDL".into())]),
        },
        MeshBake { name: HALO_BAKE.into(), object: OBJECT.into(), segments: vec![], prelude: vec![], body: BakeBody::DLists(vec![(OBJECT.into(), "gDampeHaloDL".into())]) },
    ]
}

/// The overlay's statics: `sAlreadySpawned`.
#[derive(Debug, Default)]
pub struct Statics {
    pub already_spawned: bool,
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Idle,
    Race,
    EndRace,
    Talk,
    Talk2,
    DisappearAndReward,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::Idle => "EnPoRelay_Idle",
            Action::Race => "EnPoRelay_Race",
            Action::EndRace => "EnPoRelay_EndRace",
            Action::Talk => "EnPoRelay_Talk",
            Action::Talk2 => "EnPoRelay_Talk2",
            Action::DisappearAndReward => "EnPoRelay_DisappearAndReward",
        }
    }
}

pub struct EnPoRelay {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    pub action: Action,
    /// `hookshotSlotFull`: Link had the hookshot at the race's start.
    pub hookshot_slot_full: bool,
    /// `bobTimer`: the bob's phase (32 down to 1).
    pub bob_timer: u8,
    pub action_timer: i16,
    /// `pathPoint`: the point he's making for.
    pub path_point: i16,
    pub yaw_towards_path_point: i16,
    /// `textId`: his text from now on.
    pub text_id: u16,
    pub eye_texture_idx: u16,
    pub light_color: [u8; 4],
    pub light_node: Option<LightNode>,
    pub light_info: LightInfo,
    pub collider: ColliderCylinder,
}

/// An `f32` passed as an `s16` (`Lights_PointNoGlowSetInfo`'s position).
fn to_s16(f: f32) -> i16 {
    f as i32 as i16
}

/// `EnPoRelay_Vec3sToVec3f` of `sPathPoints[i]` (past the end, what follows the array).
pub fn path_point(i: i16) -> Vec3 {
    let p = PATH_POINTS.get(i as usize).copied().unwrap_or(PATH_POINT_PAST_END);
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

/// `Math3D_PointInSquare2D`.
fn point_in_square_2d(upper_left_x: f32, lower_right_x: f32, upper_left_y: f32, lower_right_y: f32, x: f32, y: f32) -> bool {
    x >= upper_left_x && x <= lower_right_x && y >= upper_left_y && y <= lower_right_y
}

/// `Math_Vec3f_DistXZ`.
fn dist_xz(a: Vec3, b: Vec3) -> f32 {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    (dx * dx + dz * dz).sqrt()
}

/// `Actor_Draw`'s translation with `ACTOR_FLAG_IGNORE_QUAKE` (as `En_Poh`'s).
fn actor_model(rs: &RenderState, flags: u32, quake: Vec3) -> Mat4 {
    let mut m = actor_draw_matrix(rs);
    if flags & ACTOR_FLAG_IGNORE_QUAKE != 0 {
        m.w_axis += quake.extend(0.0);
    }
    m
}

impl EnPoRelay {
    /// `EnPoRelay_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: naviEnemyId NAVI_ENEMY_DAMPES_GHOST, lockOnArrowOffset 1500.
        actor.navi_enemy_id = NAVI_ENEMY_DAMPES_GHOST;
        actor.target_arrow_offset = 1500.0;
        actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 42.0);
        let (mut skeleton, mut anim): (Option<Arc<Skeleton>>, Option<Anim>) = (None, None);
        if let Some(a) = play.assets.clone() {
            skeleton = a.skeleton(OBJECT, SKEL).map_err(|e| log::error!("En_Po_Relay: {e:#}")).ok();
            anim = a.animation(OBJECT, FLOAT_ANIM).map_err(|e| log::error!("En_Po_Relay: {e:#}")).ok();
        }
        // SkelAnime_InitFlex(.., &gDampeSkel, &gDampeFloatAnim, .., LIMB_OBJECT_TK_00BE40_MAX): the
        // float looped.
        let skel = SkelAnimeStd::init_flex(skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(17), anim);
        let collider = ColliderCylinder::new(&CYLINDER_INIT);
        let h = actor.home_pos;
        let light_info = LightInfo::point_no_glow(to_s16(h.x), to_s16(h.y), to_s16(h.z), [255, 255, 255], 200);
        let light_node = play.light_ctx.insert_light(light_info);
        let mut this = EnPoRelay {
            actor,
            skel,
            skeleton,
            action: Action::Idle,
            hookshot_slot_full: false,
            bob_timer: 0,
            action_timer: 0,
            path_point: 0,
            yaw_towards_path_point: 0,
            text_id: 0,
            eye_texture_idx: 0,
            light_color: [0, 0, 0, 255],
            light_node,
            light_info,
            collider,
        };
        let st = play.overlay_static::<Statics>(ACTOR_EN_PO_RELAY);
        if st.already_spawned {
            this.actor.kill();
        } else {
            st.already_spawned = true;
            actor_set_text_with_prefix(play.scene_id, &mut this.actor, MSG_GREETING);
            this.text_id = this.actor.text_id;
            this.setup_idle();
        }
        this.actor.params &= 0x3F;
        Box::new(this)
    }

    /// `EnPoRelay_SetupIdle`.
    fn setup_idle(&mut self) {
        self.bob_timer = 32;
        self.path_point = 0;
        // Does not despawn upon room transitions.
        self.actor.room = -1;
        self.actor.shape_rot.y = 0;
        self.actor.world_rot.y = -0x8000;
        self.actor.col_chk_info.mass = MASS_HEAVY;
        self.action = Action::Idle;
    }

    /// `EnPoRelay_SetupRace`: the timer from 0, the hookshot noted, laughing.
    pub fn setup_race(&mut self, play: &mut PlayState) {
        let vec = path_point(self.path_point);
        let d = self.actor.shape_rot.y.wrapping_sub(self.actor.world_rot.y).wrapping_sub(-0x8000);
        // ((s16)(..) >> 0xB) % 32U: the shift's int taken as unsigned.
        self.action_timer = (((d as i32) >> 0xB) as u32 % 32) as i16;
        play.interface_set_timer(0);
        self.hookshot_slot_full = play.save.inv_content(ITEM_HOOKSHOT) != ITEM_NONE;
        self.yaw_towards_path_point = vec3f_yaw(self.actor.world_pos, vec);
        self.actor.flags |= ACTOR_FLAG_LOCK_ON_DISABLED;
        audio_play_actor_sfx2(play, NA_SE_EN_PO_LAUGH);
        self.action = Action::Race;
    }

    /// `EnPoRelay_SetupEndRace`.
    fn setup_end_race(&mut self) {
        self.actor.world_rot.y = self.actor.home_rot.y.wrapping_add(0xC000u16 as i16);
        self.actor.flags &= !ACTOR_FLAG_LOCK_ON_DISABLED;
        self.actor.speed_xz = 0.0;
        self.action = Action::EndRace;
    }

    /// `EnPoRelay_CorrectY`: home 45 above his point's height (2 a frame), bobbing 8.
    fn correct_y(&mut self) {
        let i = if self.path_point as usize >= PATH_POINTS.len() { PATH_POINTS.len() as i16 - 1 } else { self.path_point };
        step_to_f(&mut self.actor.home_pos.y, path_point(i).y + 45.0, 2.0);
        self.actor.world_pos.y = sin_s((self.bob_timer as i32 * 0x800) as i16) * 8.0 + self.actor.home_pos.y;
    }

    fn player(play: &PlayState) -> Option<&Actor> {
        play.player.and_then(|h| play.actors.actor(h))
    }

    /// `EnPoRelay_Idle`: Link within 250 is offered his text at once.
    fn idle(&mut self, play: &mut PlayState) {
        scaled_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 0x100);
        if process_talk_request(&mut self.actor) {
            self.actor.flags &= !ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
            self.action = Action::Talk;
        } else if self.actor.xz_dist_to_player < 250.0 {
            self.actor.flags |= ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
            self.actor.text_id = self.text_id;
            let a = self.actor.clone();
            offer_talk(play, &a, 250.0);
        }
        self.actor.play_sfx_flagged(NA_SE_EN_PO_FLY - SFX_FLAG);
    }

    /// `EnPoRelay_Talk`: his greeting closing starts the race.
    fn talk(&mut self, play: &mut PlayState) {
        scaled_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 0x100);
        if textbox_is_closing(play) {
            actor_set_text_with_prefix(play.scene_id, &mut self.actor, MSG_TIME);
            self.text_id = self.actor.text_id;
            self.setup_race(play);
        }
        self.actor.play_sfx_flagged(NA_SE_EN_PO_FLY - SFX_FLAG);
    }

    /// `EnPoRelay_Race`.
    fn race(&mut self, play: &mut PlayState) {
        let (player_pos, player_speed) = Self::player(play).map(|p| (p.world_pos, p.speed_xz)).unwrap_or_default();
        if self.action_timer != 0 {
            self.action_timer -= 1;
        }
        // 3% chance of throwing a flame every 32 frames (1.6 seconds); failing, it checks again
        // the next frame without waiting the 32 frames.
        if self.action_timer == 0 && play.rand.zero_one() < 0.03 {
            self.action_timer = 32;
            if self.path_point < 23 {
                let r = play.rand.zero_one() * 3.0;
                let multiplier: f32 = if r < 1.0 {
                    1.0
                } else if r < 2.0 {
                    -1.0
                } else {
                    0.0
                };
                let speed_xz = 30.0 * multiplier;
                let p = self.actor.world_pos;
                let pos = Vec3::new(cos_s(self.yaw_towards_path_point) * speed_xz + p.x, p.y, sin_s(self.yaw_towards_path_point) * speed_xz + p.z);
                // (yawTowardsPathPoint + 0x8000) - (0x2000 * multiplier): an int less a float,
                // passed as an s16.
                let yaw = ((self.yaw_towards_path_point as i32 + 0x8000) as f32 - 8192.0 * multiplier) as i32 as i16;
                if let Err(e) = play.actor_spawn(ACTOR_EN_HONOTRAP, pos, [0, yaw, 0], HONOTRAP_TYPE_FLAME_DROP) {
                    log::debug!("En_Po_Relay: En_Honotrap: {e:?}");
                }
            }
        }
        smooth_step_to_s(&mut self.actor.world_rot.y, self.yaw_towards_path_point, 2, 0x1000, 0x100);
        self.actor.shape_rot.y = self.actor.world_rot.y.wrapping_add(self.action_timer.wrapping_mul(0x800)).wrapping_add(-0x8000);
        if self.path_point < 23 {
            // @bug (game) Dampé's speed is directly proportional to the player's speed when less
            // than 300 units away from the player and not in the branching paths, so if the
            // player's speed is negative (by HESSing or similar), Dampé will also move backwards,
            // away from the next path point rather than towards it.
            let mut speed_xz;
            // If the player travels along a different path to Dampé that converges later.
            if point_in_square_2d(660.0, 840.0, -4480.0, -3760.0, player_pos.x, player_pos.z)
                || point_in_square_2d(1560.0, 1740.0, -4030.0, -3670.0, player_pos.x, player_pos.z)
                || point_in_square_2d(1580.0, 2090.0, -3030.0, -2500.0, player_pos.x, player_pos.z)
            {
                speed_xz = if self.hookshot_slot_full { player_speed * 1.4 } else { player_speed * 1.2 };
            } else if self.actor.xz_dist_to_player < 150.0 {
                speed_xz = if self.hookshot_slot_full { player_speed * 1.2 } else { player_speed };
            } else if self.actor.xz_dist_to_player < 300.0 {
                speed_xz = if self.hookshot_slot_full { player_speed } else { player_speed * 0.8 };
            } else if self.hookshot_slot_full {
                speed_xz = 4.5;
            } else {
                speed_xz = 3.5;
            }
            let multiplier = (250.0 - self.actor.xz_dist_to_player).max(0.0);
            speed_xz += multiplier * 0.02 + 1.0;
            approach_f(&mut self.actor.speed_xz, speed_xz, 0.5, 1.5);
        } else {
            approach_f(&mut self.actor.speed_xz, 3.5, 0.5, 1.5);
        }
        let mut vec = path_point(self.path_point);
        if dist_xz(self.actor.world_pos, vec) < 40.0 {
            self.path_point += 1;
            // @bug (game) past his last point this reads sPathPoints[28], beyond the array (what
            // follows it: see PATH_POINT_PAST_END); only the yaw below takes it, unused from here.
            vec = path_point(self.path_point);
            if self.path_point as usize == PATH_POINTS.len() {
                self.setup_end_race();
            } else if self.path_point == 9 {
                // The first door.
                play.flags.set_switch(SWITCH_FIRST_DOOR);
            } else if self.path_point == 17 {
                // The second door.
                play.flags.set_switch(SWITCH_SECOND_DOOR);
            } else if self.path_point == 25 {
                // The third door.
                play.flags.set_switch(SWITCH_THIRD_DOOR);
            }
        }
        self.yaw_towards_path_point = vec3f_yaw(self.actor.world_pos, vec);
        self.actor.play_sfx_flagged(NA_SE_EN_PO_AWAY - SFX_FLAG);
    }

    /// `EnPoRelay_EndRace`: waiting at the end, facing -0x4000; Link in room 5 has passed him.
    fn end_race(&mut self, play: &mut PlayState) {
        scaled_step_to_s(&mut self.actor.shape_rot.y, -0x4000, 0x800);
        if process_talk_request(&mut self.actor) {
            self.action = Action::Talk2;
        } else if play.room_ctx.cur.num == 5 {
            self.actor.kill();
            play.save.timer_state = TIMER_STATE_OFF;
        } else if actor_is_facing_and_near_player(&self.actor, 150.0, 0x3000) {
            self.actor.text_id = self.text_id;
            let a = self.actor.clone();
            offer_talk(play, &a, 250.0);
        }
        self.actor.play_sfx_flagged(NA_SE_EN_PO_FLY - SFX_FLAG);
    }

    /// `EnPoRelay_Talk2`: the time's box's event goes on to the first race's end or a later's;
    /// that closing, he goes.
    fn talk2(&mut self, play: &mut PlayState) {
        scaled_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 0x100);
        if play.message_state() == TEXT_STATE_EVENT {
            if play.message_should_advance() {
                if self.hookshot_slot_full {
                    actor_set_text_with_prefix(play.scene_id, &mut self.actor, MSG_LATER_RACE);
                } else {
                    actor_set_text_with_prefix(play.scene_id, &mut self.actor, MSG_FIRST_RACE);
                }
                self.text_id = self.actor.text_id;
                play.continue_textbox(self.actor.text_id);
            }
        } else if textbox_is_closing(play) {
            play.save.timer_state = TIMER_STATE_OFF;
            self.action_timer = 0;
            self.action = Action::DisappearAndReward;
        }
        self.actor.play_sfx_flagged(NA_SE_EN_PO_FLY - SFX_FLAG);
    }

    /// `EnPoRelay_DisappearAndReward`.
    fn disappear_and_reward(&mut self, play: &mut PlayState) {
        self.action_timer += 1;
        let p = self.actor.world_pos;
        if self.action_timer < 8 {
            let cam = play.cam_dir_yaw();
            let mut vec = Vec3::ZERO;
            if self.action_timer < 5 {
                let a = ((self.action_timer as i32 * 0x1000) - 0x4000) as i16;
                vec.y = sin_s(a) * 23.0 + (p.y + 40.0);
                let multiplier = cos_s(a) * 23.0;
                vec.x = sin_s(cam.wrapping_add(0x4800)) * multiplier + p.x;
                vec.z = cos_s(cam.wrapping_add(0x4800)) * multiplier + p.z;
            } else {
                vec.y = p.y + 40.0 + 15.0 * (self.action_timer - 5) as f32;
                vec.x = sin_s(cam.wrapping_add(0x4800)) * 23.0 + p.x;
                vec.z = cos_s(cam.wrapping_add(0x4800)) * 23.0 + p.z;
            }
            let scale = self.action_timer * 10 + 80;
            let spawn = |play: &mut PlayState, v: Vec3| play.with_ss(|s| s.dead_db_spawn(v, DISAPPEAR_PARTICLES_VELOCITY, DISAPPEAR_PARTICLES_ACCEL, scale, 0, [255, 255, 255, 255], [0, 0, 255], 1, 9, 1));
            spawn(play, vec);
            vec.x = (p.x + p.x) - vec.x;
            vec.z = (p.z + p.z) - vec.z;
            spawn(play, vec);
            vec.x = p.x;
            vec.z = p.z;
            spawn(play, vec);
            if self.action_timer == 1 {
                audio_play_actor_sfx2(play, NA_SE_EN_EXTINCT);
            }
        }
        if step_to_f(&mut self.actor.scale.x, 0.0, 0.001) {
            if self.hookshot_slot_full {
                let pos_at_ground = Vec3::new(self.actor.world_pos.x, self.actor.floor_height, self.actor.world_pos.z);
                let hs = HS_DAMPE_RACE as usize;
                let seconds = play.save.timer_seconds as i32;
                if seconds < play.save.high_scores[hs] {
                    play.save.high_scores[hs] = seconds;
                }
                let flag = COLLECTIBLE_FLAG.get(self.actor.params);
                if !play.flags.get_collectible(flag as i32) && seconds <= 60 {
                    item_drop_collectible2(play, pos_at_ground, (flag << 8) + (0x4000 | ITEM00_HEART_PIECE));
                } else if let Err(e) = play.actor_spawn(ACTOR_EN_ITEM00, pos_at_ground, [0; 3], ITEM00_RUPEE_RED) {
                    log::debug!("En_Po_Relay: En_Item00: {e:?}");
                }
            } else {
                play.flags.set_temp_clear(4);
                play.save.high_scores[HS_DAMPE_RACE as usize] = play.save.timer_seconds as i32;
            }
            self.actor.kill();
        }
        self.actor.scale.y = self.actor.scale.x;
        self.actor.scale.z = self.actor.scale.x;
        self.actor.world_pos.y += 10.0;
    }

    /// The limb matrices of the skeleton at `joints`.
    fn bones(&self, joints: &[[i16; 3]]) -> Vec<Mat4> {
        match &self.skeleton {
            Some(s) => s.pose_override(joints, |_, _, _| Mat4::IDENTITY),
            None => Vec::new(),
        }
    }
}

impl ActorImpl for EnPoRelay {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnPoRelay_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.skel.update();
        match self.action {
            Action::Idle => self.idle(play),
            Action::Race => self.race(play),
            Action::EndRace => self.end_race(play),
            Action::Talk => self.talk(play),
            Action::Talk2 => self.talk2(play),
            Action::DisappearAndReward => self.disappear_and_reward(play),
        }
        self.actor.move_forward();
        self.correct_y();
        self.actor.update_bg_check_info(&play.col, 0.0, 27.0, 60.0, UPDBGCHECKINFO_FLAG_2);
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        self.actor.set_focus(50.0);
        if self.bob_timer != 0 {
            self.bob_timer -= 1;
        }
        if self.bob_timer == 0 {
            self.bob_timer = 32;
        }
        self.eye_texture_idx += 1;
        if self.eye_texture_idx == DAMPE_EYES_MAX {
            self.eye_texture_idx = DAMPE_EYES_FULLY_OPEN;
        }
    }

    /// `EnPoRelay_PostLimbDraw`'s changes: the lantern's colour (`Rand_ZeroOne`: a light orange
    /// to white) and his light there, 1200 up the lantern limb.
    fn draw_update(&mut self, play: &mut PlayState) {
        let bones = self.bones(&self.skel.joint_table);
        let Some(&b) = bones.get(LIMB_LANTERN - 1) else { return };
        let rand = play.rand.zero_one();
        self.light_color[0] = ((rand * 30.0) as i16 + 225) as u8;
        self.light_color[1] = ((rand * 100.0) as i16 + 155) as u8;
        self.light_color[2] = ((rand * 160.0) as i16 + 95) as u8;
        let rs = RenderState::of(&self.actor);
        let m = actor_model(&rs, self.actor.flags, play.game_camera.quake_offset) * b;
        let v = m.transform_point3(LANTERN_LIGHT_OFFSET);
        let c = self.light_color;
        self.light_info = LightInfo::point_no_glow(to_s16(v.x), to_s16(v.y), to_s16(v.z), [c[0], c[1], c[2]], 200);
        play.light_ctx.set_info(self.light_node, self.light_info);
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.switches = vec![self.eye_texture_idx as u32];
        rs
    }

    /// `EnPoRelay_Draw`, with `EnPoRelay_PostLimbDraw`'s lantern and halo.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        let eye = rs.switches.first().copied().unwrap_or(0).min(DAMPE_EYES_MAX as u32 - 1) as usize;
        let model = actor_model(rs, self.actor.flags, play.game_camera.quake_offset);
        let bones = self.bones(&joints.rot);
        if bones.is_empty() {
            return;
        }
        let lantern = bones.get(LIMB_LANTERN - 1).map(|b| model * *b);
        let halo = bones.get(LIMB_HALO - 1).map(|b| model * *b);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&crate::en_tk::bake_name(eye))), transform: model, bones, params: Default::default() });
        if let Some(m) = lantern {
            let c = self.light_color;
            let mut sv = SegmentValues::default();
            sv.env[SEG_ENV as usize] = Some([c[0], c[1], c[2], 128]);
            out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(LANTERN_BAKE)), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
        }
        if let Some(m) = halo {
            out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(HALO_BAKE)), m));
        }
    }

    /// `EnPoRelay_Destroy`: his light out. The last one gone, the overlay unloads
    /// (`ACTOROVL_ALLOC_NORMAL`): `sAlreadySpawned` is false again at its next load.
    fn destroy(&mut self, play: &mut PlayState) {
        play.light_ctx.remove_light(self.light_node.take());
        let others = play.actors.all().into_iter().any(|h| play.actors.actor(h).is_some_and(|a| a.id == ACTOR_EN_PO_RELAY));
        if !others {
            play.overlay_statics.remove(&ACTOR_EN_PO_RELAY);
        }
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
