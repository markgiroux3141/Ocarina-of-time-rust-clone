//! `En_G_Switch` (`ovl_En_G_Switch/z_en_g_switch.c`): the silver rupees and their tracker, the
//! shooting gallery's rupees and horseback archery's pots, by `type` (bits 12 to 15).
//!
//! - **The tracker** (`ENGSWITCH_SILVER_TRACKER`, type 0): invisible; its init zeroes the overlay's
//!   count (`sCollectedCount`) and reads how many rupees there are (`silverCount`, bits 6 to 11).
//!   Each rupee taken plays `NA_SE_EV_FIVE_COUNT_LUPY` a note up the major scale (0, 2, 4, 5, 7
//!   semitones, up to the fifth); with all of them taken, the chime (`NA_SE_SY_CORRECT_CHIME`), its
//!   switch flag set, the rupee sound, and it goes. In the Gerudo Training Ground's room 2 it sets
//!   the room's temp clear instead of the flag (and no chime). With its flag set at init it goes.
//! - **A silver rupee** (`ENGSWITCH_SILVER_RUPEE`, type 1): spinning (0x800 a frame), scale 0.03,
//!   700 up (`shape.yOffset`); Link within 30 (`xyzDistToPlayerSq`) takes it: 5 rupees, the count
//!   up, the rupee sound, and it rides over his head (40 up, 60 for an adult) bobbing for 15
//!   frames, spinning 0x3C0 a frame, then goes. With its switch flag set at init (the tracker's,
//!   whose flag the rupees share) it goes.
//! - **A horseback archery pot** (`ENGSWITCH_ARCHERY_POT`, type 2): `object_tsubo`'s pot at 0.25 by
//!   0.45 by 0.25, falling (gravity -3), hit only by arrows (`DMG_ARROW`, a cylinder 24 by 74): 30
//!   shards (`EffectSsKakera`), dust, `NA_SE_EV_POT_BROKEN`, the hit mark and a score (`EffectSsExtra`
//!   type 2), gone 50 frames later. Its object missing, it goes at init.
//! - **A shooting gallery rupee** (`ENGSWITCH_TARGET_RUPEE`, type 3): scale 0.05, spun, moved by
//!   `moveMode` (thrown up, sliding left or right, or out to `targetPos` and back) as the gallery
//!   (`En_Syateki_Itm`, its `parent`) set it up; hit, a burst of 20 small rupees and the gallery's
//!   count up; missed, the gallery told.
//!
//! The whole overlay is ported. Not ported: `En_Syateki_Itm` (the shooting gallery, Phase 20): its
//! writes (`targetState[index]`, `hitCount`) are logged and its rupees' set-up fields (`colorIdx`,
//! `moveMode`, `index`, `delayTimer`, `targetPos`) stay as the init leaves them. The tracker's
//! debug cube (`BREG(0)`) and `BREG(8)`'s forced hits aren't (the registers are 0). @bug (game):
//! the rupees' burst passes binary angles to `Matrix_RotateY`/`_RotateX`, which take radians (the
//! yaw anywhere in ±32767 radians, the pitch about -13000); its spins too (15 to 25 radians a
//! frame).
//!
//! Params: `type` bits 12 to 15 (0 the tracker, 1 a silver rupee, 2 an archery pot, 3 a gallery
//! rupee); `switchFlag` bits 0 to 5 (the tracker's and its rupees': set when all are taken);
//! `silverCount` bits 6 to 11 (the tracker's: how many rupees). Master Quest's Dodongo's Cavern
//! room 2: the tracker 0x0165 (five, flag 0x25), five rupees 0x1165.
//! Flags: the tracker sets switch flag `switchFlag` (or the room's temp clear in the Gerudo
//! Training Ground's room 2); the tracker and the rupees read it at init (set: gone).

use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, UPDBGCHECKINFO_FLAG_2, UPDBGCHECKINFO_FLAG_3, UPDBGCHECKINFO_FLAG_4};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile, func_80033480};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, ObjectNeed};
use oot_game::audio::sfx::{NA_SE_SY_CORRECT_CHIME, NA_SE_SY_GET_RUPY, SfxPos};
use oot_game::collision_check::*;
use oot_game::effect::kakera::KAKERA_COLOR_NONE;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{approach_f, cos_s, sin_s};
use eng_collision::math3d::Cylinder16;

/// `ACTOR_EN_G_SWITCH` (`actor_table.h`: 0x0117).
pub const ACTOR_EN_G_SWITCH: i16 = 0x0117;

/// `EnGSwitchType`.
pub const ENGSWITCH_SILVER_TRACKER: i16 = 0;
pub const ENGSWITCH_SILVER_RUPEE: i16 = 1;
pub const ENGSWITCH_ARCHERY_POT: i16 = 2;
pub const ENGSWITCH_TARGET_RUPEE: i16 = 3;

/// `EnGSwitchMoveMode`.
pub const GSWITCH_NONE: i16 = 0;
pub const GSWITCH_APPEAR: i16 = 1;
pub const GSWITCH_THROW: i16 = 2;
pub const GSWITCH_UNUSED: i16 = 3;
pub const GSWITCH_LEFT: i16 = 4;
pub const GSWITCH_RIGHT: i16 = 5;

/// `GSwitchMoveState`.
pub const MOVE_TARGET: i16 = 0;
pub const MOVE_HOME: i16 = 1;

/// `EN_GSWITCH_EFFECT_COUNT`.
pub const EN_GSWITCH_EFFECT_COUNT: usize = 100;

/// `NA_SE_EV_FIVE_COUNT_LUPY` (`environmentbank_table.h`: 0x28E8).
pub const NA_SE_EV_FIVE_COUNT_LUPY: u16 = 0x28E8;
/// `NA_SE_EV_HIT_SOUND` (`environmentbank_table.h`: 0x28D3).
pub const NA_SE_EV_HIT_SOUND: u16 = 0x28D3;
/// `NA_SE_EV_POT_BROKEN` (`environmentbank_table.h`: 0x2887).
pub const NA_SE_EV_POT_BROKEN: u16 = 0x2887;

/// `SCENE_GERUDO_TRAINING_GROUND` (`scene_table.h`: 0x0B).
pub const SCENE_GERUDO_TRAINING_GROUND: u16 = 0x0B;

/// `OBJECT_TSUBO` (`object_table.h`: 0x012C): the archery pots'.
pub const OBJECT_TSUBO: i16 = 0x012C;
/// `object_tsubo_DL_0017C0` (the pot), `object_tsubo_DL_001960` (its shards).
pub const POT_DL: &str = "object_tsubo_DL_0017C0";
pub const POT_SHARD_DL: &str = "object_tsubo_DL_001960";

/// `EFFECT_HITMARK_WHITE`.
const EFFECT_HITMARK_WHITE: i32 = 0;

/// `DMG_ARROW` (`DMG_ARROW_NORMAL | DMG_ARROW_FIRE | DMG_ARROW_ICE | DMG_ARROW_LIGHT |
/// DMG_ARROW_UNK1 | DMG_ARROW_UNK2 | DMG_ARROW_UNK3`).
pub const DMG_ARROW: u32 = 0x0001_F820;

/// `En_G_Switch_Profile`: `ACTORCAT_PROP`, `ACTOR_FLAG_UPDATE_CULLING_DISABLED |
/// ACTOR_FLAG_DRAW_CULLING_DISABLED`, `OBJECT_GAMEPLAY_KEEP`, no draw (set by type).
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_G_SWITCH,
    name: "En_G_Switch",
    category: ACTORCAT_PROP,
    flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED,
    object: "gameplay_keep",
    schema: &SCHEMA,
};

/// `PARAMS_GET_U(params, 12, 4)`.
pub const TYPE: Field = Field::params(
    "type",
    12,
    4,
    Kind::Named(&[(0, "the silver rupees' tracker"), (1, "a silver rupee"), (2, "a horseback archery pot"), (3, "a shooting gallery rupee")]),
    "what it is",
);
/// `PARAMS_GET_U(params, 0, 6)`.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 0, 6, Kind::SwitchFlag, "the tracker's and its rupees': set when all are taken (the rupees and the tracker gone at init)");
/// `PARAMS_GET_NOMASK(params, 6) & 0x3F`.
pub const SILVER_COUNT: Field = Field::params("silverCount", 6, 6, Kind::Range(0, 63), "the tracker's: how many silver rupees to take");

/// En_G_Switch's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, SWITCH_FLAG, SILVER_COUNT],
    flags: &[
        FlagUse::new(Flag::Switch("switchFlag"))
            .reads()
            .sets()
            .note("the tracker sets it when its rupees are all taken (not in the Gerudo Training Ground's room 2); set at init, the tracker and the rupees are gone"),
        FlagUse::new(Flag::TempClear).sets().note("the tracker's in the Gerudo Training Ground's room 2, instead of its switch flag"),
    ],
    todo: false,
    objects: &[ObjectNeed::new("object_tsubo").when("type", &[2]).note("the archery pot: without it, it goes at init")],
    texts: &[],
};

/// `sCylinderInit`: hit by Player's attacks (`AC_TYPE_PLAYER`, anything but the shield and the
/// mirror's ray), 13 across, 40 high.
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK2,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_NONE,
    },
    dim: Cylinder16 { radius: 13, height: 40, y_shift: 0, pos: [0, 0, 0] },
};

/// The collider's id (`collider_mut`).
pub const COL_CYLINDER: u8 = 0;

/// `sRupeeTextures`: green, blue, red, pink, orange, silver.
pub const RUPEE_TEX: [&str; 6] = ["gRupeeGreenTex", "gRupeeBlueTex", "gRupeeRedTex", "gRupeePinkTex", "gRupeeOrangeTex", "gRupeeSilverTex"];

/// `majorScale`: the tracker's notes.
pub const MAJOR_SCALE: [i8; 5] = [0, 2, 4, 5, 7];

fn rupee_bake(i: usize) -> String {
    format!("En_G_Switch/rupee{i}")
}

/// `EnGSwitch_DrawRupee`'s meshes: `gRupeeDL` with each colour on segment 8.
pub fn bakes() -> Vec<MeshBake> {
    RUPEE_TEX
        .iter()
        .enumerate()
        .map(|(i, t)| MeshBake {
            name: rupee_bake(i),
            object: "gameplay_keep".into(),
            segments: vec![(0x08, BakeSegment::Texture { file: "gameplay_keep".into(), symbol: (*t).into() })],
            prelude: Vec::new(),
            body: BakeBody::DLists(vec![("gameplay_keep".into(), "gRupeeDL".into())]),
        })
        .collect()
}

/// The overlay's statics: `sCollectedCount`.
#[derive(Debug, Default)]
pub struct Statics {
    pub collected_count: i16,
}

/// The statics, zeroed with a fresh overlay (`static s16 sCollectedCount = 0`).
pub fn statics(play: &mut PlayState) -> &mut Statics {
    play.overlay_static::<Statics>(ACTOR_EN_G_SWITCH)
}

// shared: Actor_FreeOverlay
/// `Actor_FreeOverlay` (`z_actor.c`, from `Actor_Delete`): when the last En_G_Switch goes the
/// overlay unloads (`ACTOROVL_ALLOC_NORMAL`) and `sCollectedCount` is 0 again with the next load.
fn actor_free_overlay(play: &mut PlayState) {
    let others = play.actors.all().into_iter().any(|h| play.actors.actor(h).is_some_and(|a| a.id == ACTOR_EN_G_SWITCH));
    if !others {
        play.overlay_statics.remove(&ACTOR_EN_G_SWITCH);
    }
}

/// `EnGSwitchEffect`: a small rupee of a gallery rupee's burst.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EnGSwitchEffect {
    pub pos: Vec3,
    pub scale: i16,
    pub timer: i16,
    pub color_idx: i16,
    pub flag: bool,
    pub velocity: Vec3,
    pub rot: Vec3,
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// None set (an unknown type).
    None,
    /// `EnGSwitch_SilverRupeeTracker`.
    SilverRupeeTracker,
    /// `EnGSwitch_SilverRupeeIdle`.
    SilverRupeeIdle,
    /// `EnGSwitch_WaitForObject`.
    WaitForObject,
    /// `EnGSwitch_SilverRupeeCollected`.
    SilverRupeeCollected,
    /// `EnGSwitch_GalleryRupee`.
    GalleryRupee,
    /// `EnGSwitch_ArcheryPot`.
    ArcheryPot,
    /// `EnGSwitch_Kill`.
    Kill,
}

/// What `actor.draw` is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Draw {
    None,
    /// `EnGSwitch_DrawRupee`.
    Rupee,
    /// `EnGSwitch_DrawPot`.
    Pot,
}

pub struct EnGSwitch {
    pub actor: Actor,
    pub action: Action,
    pub draw: Draw,
    pub type_: i16,
    pub silver_count: i16,
    pub switch_flag: i16,
    pub kill_timer: i16,
    pub color_idx: i16,
    pub broken: bool,
    pub num_effects: i16,
    pub object_id: i16,
    /// `index`: the first or second rupee of a gallery pair.
    pub index: i16,
    /// `delayTimer`: the gallery's wait before the second blue rupee.
    pub delay_timer: i16,
    /// `waitTimer`: a gallery rupee's wait out before it goes back.
    pub wait_timer: i16,
    /// `moveMode`, `moveState`.
    pub move_mode: i16,
    pub move_state: i16,
    /// `noteIndex`: the tracker's next note.
    pub note_index: i16,
    pub target_pos: Vec3,
    /// `requiredObjectSlot`.
    pub required_object_slot: Option<usize>,
    pub collider: ColliderCylinder,
    pub effects: Vec<EnGSwitchEffect>,
}

impl EnGSwitch {
    /// `EnGSwitch_Init`: its type and flag, 100 effect slots, then by type (the module's docs).
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = EnGSwitch {
            type_: TYPE.get(actor.params),
            switch_flag: SWITCH_FLAG.get(actor.params),
            actor,
            action: Action::None,
            draw: Draw::None,
            silver_count: 0,
            kill_timer: 0,
            color_idx: 0,
            broken: false,
            num_effects: EN_GSWITCH_EFFECT_COUNT as i16,
            object_id: 0,
            index: 0,
            delay_timer: 0,
            wait_timer: 0,
            move_mode: GSWITCH_NONE,
            move_state: MOVE_TARGET,
            note_index: 0,
            target_pos: Vec3::ZERO,
            required_object_slot: None,
            collider: ColliderCylinder::default(),
            effects: vec![EnGSwitchEffect::default(); EN_GSWITCH_EFFECT_COUNT],
        };
        log::debug!("En_G_Switch: index {:x}, save {:x}", this.type_, this.switch_flag);
        match this.type_ {
            ENGSWITCH_SILVER_TRACKER => {
                log::debug!("En_G_Switch: parent switch spawn (flag {:x})", this.switch_flag);
                statics(play).collected_count = 0;
                // PARAMS_GET_NOMASK(params, 6) & 0x3F: "ideally PARAMS_GET_U(params, 6, 6)" (the
                // C's comment), the same bits.
                this.silver_count = SILVER_COUNT.get(this.actor.params);
                if play.flags.get_switch(this.switch_flag as i32) {
                    this.actor.kill();
                } else {
                    this.action = Action::SilverRupeeTracker;
                }
            }
            ENGSWITCH_SILVER_RUPEE => {
                log::debug!("En_G_Switch: child switch spawn (flag {:x})", this.switch_flag);
                this.color_idx = 5;
                this.num_effects = 20;
                this.collider = ColliderCylinder::new(&CYLINDER_INIT);
                this.draw = Draw::Rupee;
                this.actor.shape_y_offset = 700.0;
                if play.flags.get_switch(this.switch_flag as i32) {
                    this.actor.kill();
                } else {
                    this.actor.scale = Vec3::splat(0.03);
                    this.action = Action::SilverRupeeIdle;
                }
            }
            ENGSWITCH_ARCHERY_POT => {
                this.actor.gravity = -3.0;
                this.color_idx = play.rand.zero_float(2.99) as i16;
                this.collider = ColliderCylinder::new(&CYLINDER_INIT);
                this.actor.scale = Vec3::new(0.25, 0.45, 0.25);
                this.collider.info.ac_dmg_info.dmg_flags = DMG_ARROW;
                this.object_id = OBJECT_TSUBO;
                this.required_object_slot = play.object_ctx.get_index(this.object_id);
                if this.required_object_slot.is_none() {
                    this.actor.kill();
                    log::warn!("En_G_Switch: an archery pot without object_tsubo");
                }
                this.collider.dim.radius = 24;
                this.collider.dim.height = 74;
                this.collider.dim.y_shift = 0;
                this.action = Action::WaitForObject;
            }
            ENGSWITCH_TARGET_RUPEE => {
                this.actor.shape_y_offset = 700.0;
                this.actor.scale = Vec3::splat(0.05);
                this.collider = ColliderCylinder::new(&CYLINDER_INIT);
                this.draw = Draw::Rupee;
                this.collider.dim.radius = 20;
                this.collider.dim.height = 60;
                this.collider.dim.y_shift = 5;
                this.action = Action::GalleryRupee;
            }
            _ => {}
        }
        Box::new(this)
    }

    /// `EnGSwitch_Break`: the hit mark (white, 700) where it was hit; an archery pot's score
    /// (`EffectSsExtra` type 2, 15 up); a gallery rupee's burst of small rupees 30 up.
    fn break_(&mut self, play: &mut PlayState) {
        let p = self.actor.world_pos;
        let rand_pos = Vec3::new(p.x + play.rand.centered_float(40.0), p.y + 30.0 + play.rand.centered_float(35.0), p.z + play.rand.centered_float(40.0));
        let hit = self.collider.info.ac_dmg_info.hit_pos;
        let hit_pos = Vec3::new(hit[0] as f32, hit[1] as f32, hit[2] as f32);
        let ty = self.type_;
        play.with_ss(|ss| {
            ss.hit_mark_spawn_custom_scale(EFFECT_HITMARK_WHITE, 700, hit_pos);
            if ty == ENGSWITCH_ARCHERY_POT {
                ss.extra_spawn(hit_pos, Vec3::new(0.0, 15.0, 0.0), Vec3::ZERO, 5, 2);
            }
        });
        if ty == ENGSWITCH_TARGET_RUPEE {
            for _ in 0..self.num_effects {
                self.spawn_effects(play, rand_pos, 100, self.color_idx);
            }
        }
    }

    /// `EnGSwitch_WaitForObject`: `object_tsubo` loaded, it's the pot's (drawn, waiting for
    /// arrows).
    fn wait_for_object(&mut self, play: &mut PlayState) {
        if let Some(slot) = self.required_object_slot
            && play.object_ctx.is_loaded(slot)
        {
            self.draw = Draw::Pot;
            self.action = Action::ArcheryPot;
        }
    }

    /// `EnGSwitch_SilverRupeeTracker`: a new rupee taken (under five), its note; all taken, the
    /// chime and the flag (or the Gerudo Training Ground's room 2's clear), the rupee sound, gone.
    fn silver_rupee_tracker(&mut self, play: &mut PlayState) {
        let count = statics(play).collected_count;
        if self.note_index < count && count < 5 {
            log::debug!("En_G_Switch: sound? {}", self.note_index);
            let semitone = MAJOR_SCALE.get(self.note_index as usize).copied().unwrap_or(0);
            play.audio.play_sfx_transposed(SfxPos::Default, NA_SE_EV_FIVE_COUNT_LUPY, semitone);
            self.note_index = count;
        }
        if count >= self.silver_count {
            log::debug!("En_G_Switch: last! {}", self.switch_flag);
            if play.scene_id == SCENE_GERUDO_TRAINING_GROUND && self.actor.room == 2 {
                play.flags.set_temp_clear(self.actor.room);
            } else {
                play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
                play.flags.set_switch(self.switch_flag as i32);
            }
            play.audio.play_sfx_centered(NA_SE_SY_GET_RUPY);
            self.actor.kill();
        }
    }

    /// `EnGSwitch_SilverRupeeIdle`: spinning; Link within 30, taken (5 rupees, the count up, the
    /// sound), over his head, 15 frames to go.
    fn silver_rupee_idle(&mut self, play: &mut PlayState) {
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x800);
        if self.actor.xyz_dist_to_player_sq < 30.0 * 30.0 {
            oot_game::item::rupees_change_by(&mut play.save, 5);
            let st = statics(play);
            st.collected_count = st.collected_count.wrapping_add(1);
            play.audio.play_sfx_centered(NA_SE_SY_GET_RUPY);
            self.actor.world_pos = player_pos(play);
            self.actor.world_pos.y += 40.0;
            if play.save.adult {
                self.actor.world_pos.y += 20.0;
            }
            self.actor.gravity = 0.0;
            self.kill_timer = 15;
            self.action = Action::SilverRupeeCollected;
        }
    }

    /// `EnGSwitch_SilverRupeeCollected`: spinning over Link's head, bobbing smaller each frame;
    /// gone with the timer out.
    fn silver_rupee_collected(&mut self, play: &mut PlayState) {
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x3C0);
        if self.kill_timer == 0 {
            self.actor.kill();
            return;
        }
        let p = player_pos(play);
        self.actor.world_pos = p;
        self.actor.world_pos.y = p.y + 40.0 + (self.kill_timer as f32 * 0.3) * sin_s((self.kill_timer as i32 * 0x3A98) as i16);
        if play.save.adult {
            self.actor.world_pos.y += 20.0;
        }
    }

    /// `((EnSyatekiItm*)this->actor.parent)->targetState[this->index] = state` when the gallery
    /// is there: `En_Syateki_Itm` isn't ported.
    fn tell_gallery(&self, play: &PlayState, what: &str) {
        if self.actor.parent.is_some_and(|h| play.actors.actor(h).is_some()) {
            log::warn!("En_G_Switch: the gallery's targetState[{}] = {what}: En_Syateki_Itm isn't ported", self.index);
        }
    }

    /// `EnGSwitch_GalleryRupee`: spinning; after its delay, moving by `moveMode` (thrown and
    /// falling 50 below its home, sliding past `targetPos`, or out to `targetPos`, 60 frames and
    /// back home: missed, the gallery told, gone); hit, the gallery's count up, the sounds, the
    /// break, 50 frames to go.
    fn gallery_rupee(&mut self, play: &mut PlayState) {
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x3C0);
        if self.delay_timer != 0 {
            return;
        }
        match self.move_mode {
            GSWITCH_THROW => {
                self.actor.move_forward();
                if self.actor.velocity.y < 0.0 && self.actor.world_pos.y < self.actor.home_pos.y - 50.0 {
                    self.actor.velocity.y = 0.0;
                    self.actor.gravity = 0.0;
                    self.tell_gallery(play, "ENSYATEKIHIT_MISS");
                    self.actor.kill();
                }
            }
            GSWITCH_LEFT => {
                self.actor.update_pos();
                if self.actor.velocity.x < 0.0 && self.actor.world_pos.x < self.target_pos.x {
                    self.tell_gallery(play, "ENSYATEKIHIT_MISS");
                    self.actor.kill();
                }
            }
            GSWITCH_RIGHT => {
                self.actor.update_pos();
                if self.actor.world_pos.x > self.target_pos.x {
                    self.tell_gallery(play, "ENSYATEKIHIT_MISS");
                    self.actor.kill();
                }
            }
            _ => match self.move_state {
                MOVE_TARGET => {
                    let p = &mut self.actor.world_pos;
                    if (p.x - self.target_pos.x).abs() > 5.0 || (p.y - self.target_pos.y).abs() > 5.0 {
                        approach_f(&mut p.x, self.target_pos.x, 0.3, 30.0);
                        approach_f(&mut p.y, self.target_pos.y, 0.3, 30.0);
                    } else {
                        self.move_state = MOVE_HOME;
                        self.wait_timer = 60;
                    }
                }
                MOVE_HOME => {
                    if self.wait_timer == 0 {
                        let home = self.actor.home_pos;
                        let p = &mut self.actor.world_pos;
                        if (p.x - home.x).abs() > 5.0 || (p.y - home.y).abs() > 5.0 {
                            approach_f(&mut p.x, home.x, 0.3, 30.0);
                            approach_f(&mut p.y, home.y, 0.3, 30.0);
                        } else {
                            self.tell_gallery(play, "ENSYATEKIHIT_MISS");
                            self.actor.kill();
                        }
                    }
                }
                _ => {}
            },
        }
        // BREG(8): 0.
        if self.collider.base.ac_flags & AC_HIT != 0 {
            self.collider.base.ac_flags &= !AC_HIT;
            if self.actor.parent.is_some_and(|h| play.actors.actor(h).is_some()) {
                log::warn!("En_G_Switch: the gallery's hitCount++, targetState[{}] = ENSYATEKIHIT_HIT: En_Syateki_Itm isn't ported", self.index);
                play.audio.play_sfx_centered(NA_SE_EV_HIT_SOUND);
                play.audio.play_sfx_centered(NA_SE_SY_GET_RUPY);
                self.break_(play);
                self.kill_timer = 50;
                self.broken = true;
                self.action = Action::Kill;
            }
        }
    }

    /// `EnGSwitch_ArcheryPot`: spinning; hit, 30 shards round it, dust, the pot's sound, the
    /// break, 50 frames to go.
    fn archery_pot(&mut self, play: &mut PlayState) {
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x3C0);
        if self.collider.base.ac_flags & AC_HIT == 0 {
            return;
        }
        self.collider.base.ac_flags &= !AC_HIT;
        let this_pos = self.actor.world_pos;
        play.with_ss(|ss| {
            let mut angle: i16 = 0;
            for _ in 0..30 {
                let (sn, cs) = (sin_s(angle), cos_s(angle));
                let mut pos = Vec3::new(sn * 8.0, 10.0 + ss.rand.centered_float(5.0), cs * 8.0);
                let vel = Vec3::new(pos.x / 2.0, 10.0 + ss.rand.zero_one() * 15.0, pos.z / 2.0);
                pos += this_pos;
                let rand = ss.rand.zero_one();
                let phi_s0 = if rand < 0.2 {
                    0x60
                } else if rand < 0.6 {
                    0x40
                } else {
                    0x20
                };
                let scale = (30.0 + ss.rand.zero_one() * 130.0) as i16;
                ss.kakera_spawn(pos, vel, this_pos, -240, phi_s0, 10, 10, 0, scale, 0, 0x20, 60, KAKERA_COLOR_NONE, OBJECT_TSUBO, Some(("object_tsubo", POT_SHARD_DL)));
                angle = angle.wrapping_add(0x4E20);
            }
        });
        func_80033480(play, this_pos, 30.0, 4, 20, 50, 0);
        play.sfx_source_play_sfx_at_fixed_world_pos(this_pos, 40, NA_SE_EV_POT_BROKEN);
        self.break_(play);
        self.kill_timer = 50;
        self.broken = true;
        self.action = Action::Kill;
    }

    /// `EnGSwitch_Kill`: gone with the timer out.
    fn kill(&mut self) {
        if self.kill_timer == 0 {
            self.actor.kill();
        }
    }

    /// `EnGSwitch_SpawnEffects`: the first free slot, a small rupee at `pos` flying out at 20
    /// (a random yaw, about -13000 pitch: radians, @bug (game)), for 30 frames.
    fn spawn_effects(&mut self, play: &mut PlayState, pos: Vec3, scale: i16, color_idx: i16) {
        let n = self.num_effects.max(0) as usize;
        if let Some(e) = self.effects.iter_mut().take(n).find(|e| !e.flag) {
            e.pos = pos;
            e.scale = scale;
            e.color_idx = color_idx;
            e.timer = 30;
            e.rot = Vec3::ZERO;
            let pitch = play.rand.centered_float(1000.0) - 13000.0;
            let yaw = play.rand.centered_float(65535.0);
            // Matrix_RotateY(yaw, MTXMODE_NEW); Matrix_RotateX(pitch, MTXMODE_APPLY);
            // Matrix_MultVec3f(&baseVel, &effect->velocity).
            let m = Mat4::from_rotation_y(yaw) * Mat4::from_rotation_x(pitch);
            e.velocity = m.transform_point3(Vec3::new(0.0, 0.0, 20.0));
            e.flag = true;
        }
    }

    /// `EnGSwitch_UpdateEffects`: each live one tumbling, out along its velocity (falling to -20),
    /// its timer down, then shrinking 2 a frame to under 10, gone.
    fn update_effects(&mut self, play: &mut PlayState) {
        let n = self.num_effects.max(0) as usize;
        for e in self.effects.iter_mut().take(n) {
            if !e.flag {
                continue;
            }
            e.rot.x += play.rand.zero_one() * 10.0 + 15.0;
            e.rot.y += play.rand.zero_one() * 10.0 + 15.0;
            e.rot.z += play.rand.zero_one() * 10.0 + 15.0;
            let temp = e.pos + e.velocity;
            approach_f(&mut e.pos.x, temp.x, 0.3, 30.0);
            approach_f(&mut e.pos.y, temp.y, 0.8, 250.0);
            approach_f(&mut e.pos.z, temp.z, 0.3, 30.0);
            approach_f(&mut e.velocity.y, -20.0, 0.9, 1.0);
            if e.timer != 0 {
                e.timer -= 1;
            } else if e.scale < 10 {
                e.flag = false;
            } else {
                e.scale -= 2;
            }
        }
    }
}

/// Player's `world.pos`.
fn player_pos(play: &PlayState) -> Vec3 {
    play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or_default()
}

/// What the draw reads from the update (`render_state`'s `switches`).
mod rs {
    /// `actor.draw` (0 none, 1 the rupee, 2 the pot).
    pub const DRAW: usize = 0;
    pub const BROKEN: usize = 1;
    pub const COLOR: usize = 2;
    pub const SWITCHES: usize = 3;
}

impl ActorImpl for EnGSwitch {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnGSwitch_Update`: the action, the three timers down; an archery pot moved and its floor
    /// checked; drawn, a gallery rupee's burst, and the cylinder placed and registered unless
    /// it's going or a silver rupee waiting.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::None => {}
            Action::SilverRupeeTracker => self.silver_rupee_tracker(play),
            Action::SilverRupeeIdle => self.silver_rupee_idle(play),
            Action::WaitForObject => self.wait_for_object(play),
            Action::SilverRupeeCollected => self.silver_rupee_collected(play),
            Action::GalleryRupee => self.gallery_rupee(play),
            Action::ArcheryPot => self.archery_pot(play),
            Action::Kill => self.kill(),
        }
        if self.kill_timer != 0 {
            self.kill_timer -= 1;
        }
        if self.wait_timer != 0 {
            self.wait_timer -= 1;
        }
        if self.delay_timer != 0 {
            self.delay_timer -= 1;
        }
        if self.type_ != ENGSWITCH_SILVER_TRACKER && self.type_ != ENGSWITCH_SILVER_RUPEE && self.type_ != ENGSWITCH_TARGET_RUPEE {
            self.actor.move_forward();
            self.actor.update_bg_check_info(&play.col, 50.0, 50.0, 100.0, UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
        }
        if self.draw != Draw::None {
            if self.type_ == ENGSWITCH_TARGET_RUPEE {
                self.update_effects(play);
            }
            if self.action != Action::Kill && self.action != Action::SilverRupeeIdle {
                self.collider.update(&self.actor);
                play.collision_check_set_ac(&self.actor, COL_CYLINDER, &mut self.collider);
            }
        }
    }

    /// `EnGSwitch_Destroy`: the cylinder; the overlay's count goes with the last one.
    fn destroy(&mut self, play: &mut PlayState) {
        actor_free_overlay(play);
    }

    fn render_state(&self) -> RenderState {
        let mut r = RenderState::of(&self.actor);
        let draw = match self.draw {
            Draw::None => 0,
            Draw::Rupee => 1,
            Draw::Pot => 2,
        };
        r.switches = vec![draw, self.broken as u32, self.color_idx.clamp(0, 5) as u32];
        r
    }

    /// `EnGSwitch_DrawRupee` (unbroken: `gRupeeDL` in its colour; `func_8002EBCC`'s highlight is
    /// the mesh's texgen) with a gallery rupee's burst (`EnGSwitch_DrawEffects`); `EnGSwitch_DrawPot`
    /// (unbroken: `object_tsubo_DL_0017C0`).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if rs.switches.len() < rs::SWITCHES {
            return;
        }
        let broken = rs.switches[rs::BROKEN] != 0;
        match rs.switches[rs::DRAW] {
            1 => {
                if !broken {
                    let i = rs.switches[rs::COLOR] as usize;
                    out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(&rupee_bake(i))), actor_draw_matrix(rs)));
                }
                if self.type_ == ENGSWITCH_TARGET_RUPEE {
                    // EnGSwitch_DrawEffects: each live one at its place, its scale / 10000, turned
                    // by its rot (radians) on x, y, z.
                    let n = self.num_effects.max(0) as usize;
                    for e in self.effects.iter().take(n).filter(|e| e.flag) {
                        let s = e.scale as f32 / 10000.0;
                        let m = Mat4::from_translation(e.pos) * Mat4::from_scale(Vec3::splat(s)) * Mat4::from_rotation_x(e.rot.x) * Mat4::from_rotation_y(e.rot.y) * Mat4::from_rotation_z(e.rot.z);
                        let i = e.color_idx.clamp(0, 5) as usize;
                        out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(&rupee_bake(i))), m));
                    }
                }
            }
            2 if !broken => {
                crate::gfx_draw_dlist_opa(out, "object_tsubo", POT_DL, rs);
            }
            _ => {}
        }
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            COL_CYLINDER => Some(ColliderMut::Cylinder(&mut self.collider)),
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
