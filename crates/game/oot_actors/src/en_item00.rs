//! `En_Item00` (`src/code/z_en_item00.c`): the collectibles: rupees, recovery hearts, heart
//! pieces and containers, ammo, magic, keys. `params & 0xFF` is the `Item00Type`, bits 8..13 a
//! collectible flag, bit 15 "collected at once".
//!
//! A drop (`Item_DropCollectible`, `Item_DropCollectibleRandom`, from bushes and rocks) pops
//! up, falls and bounces (`func_8001E304`, `func_8001E1C8`), then spins where it lands
//! (`func_8001DFC8`) until Link walks within 30 of it or its 220 frames run out (blinking for
//! the last 40). Touched, it's given (`Item_Give`) and bounces over Link's head for 15 frames
//! (`EnItem00_Collected`).
//!
//! The items Player holds up (sticks, nuts, seeds, magic, keys, heart pieces and containers,
//! shields and tunics) are offered to Player instead (`Actor_OfferGetItemNearby`, docs/adr/0019); Player's
//! get-item interrupt takes one by becoming its `parent`, and the item goes (`Actor_HasParent`).
//!
//! Drawn as `EnItem00_Draw` does: `gRupeeDL` with the rupee's colour on segment 8, or
//! `gItemDropDL` with the item's picture after `Gfx_SetupDL_66` (docs/adr/0012-actor-bakes.md),
//! the heart piece's and container's lists, and `GetItem_Draw` (`oot_game::draw`) for the placed
//! recovery hearts (once `OBJECT_GI_HEART` is loaded), the shields and the tunics.
//!
//! Not ported: the circle shadow, the sparkles (`EffectSsKiraKira`, whose random numbers are
//! still drawn), and the healing fairy's sound (`EffectSsDeadSound`, an effect).

use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{cos_s, sin_s, smooth_step_to_f, smooth_step_to_s};
use glam::Vec3;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_MISC, ActorHandle, ActorImpl, ActorProfile};
use oot_game::collision_check::*;
use oot_game::get_item::{actor_has_parent, offer_get_item, offer_get_item_range};
use oot_game::item::*;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};

pub const ACTOR_EN_ITEM00: i16 = 0x0015;
/// `ACTOR_EN_ELF` and `FAIRY_HEAL_TIMED`.
const ACTOR_EN_ELF: i16 = 0x0018;
const FAIRY_HEAL_TIMED: i16 = 0x02;

/// `En_Item00_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_ITEM00, name: "En_Item00", category: ACTORCAT_MISC, flags: 0, object: "gameplay_keep" };

// `Item00Type` (actor.h).
pub const ITEM00_RUPEE_GREEN: i16 = 0x00;
pub const ITEM00_RUPEE_BLUE: i16 = 0x01;
pub const ITEM00_RUPEE_RED: i16 = 0x02;
pub const ITEM00_RECOVERY_HEART: i16 = 0x03;
pub const ITEM00_BOMBS_A: i16 = 0x04;
pub const ITEM00_ARROWS_SINGLE: i16 = 0x05;
pub const ITEM00_HEART_PIECE: i16 = 0x06;
pub const ITEM00_HEART_CONTAINER: i16 = 0x07;
pub const ITEM00_ARROWS_SMALL: i16 = 0x08;
pub const ITEM00_ARROWS_MEDIUM: i16 = 0x09;
pub const ITEM00_ARROWS_LARGE: i16 = 0x0A;
pub const ITEM00_BOMBS_B: i16 = 0x0B;
pub const ITEM00_NUTS: i16 = 0x0C;
pub const ITEM00_STICK: i16 = 0x0D;
pub const ITEM00_MAGIC_LARGE: i16 = 0x0E;
pub const ITEM00_MAGIC_SMALL: i16 = 0x0F;
pub const ITEM00_SEEDS: i16 = 0x10;
pub const ITEM00_SMALL_KEY: i16 = 0x11;
pub const ITEM00_FLEXIBLE: i16 = 0x12;
pub const ITEM00_RUPEE_ORANGE: i16 = 0x13;
pub const ITEM00_RUPEE_PURPLE: i16 = 0x14;
pub const ITEM00_SHIELD_DEKU: i16 = 0x15;
pub const ITEM00_SHIELD_HYLIAN: i16 = 0x16;
pub const ITEM00_TUNIC_ZORA: i16 = 0x17;
pub const ITEM00_TUNIC_GORON: i16 = 0x18;
pub const ITEM00_BOMBS_SPECIAL: i16 = 0x19;
pub const ITEM00_NONE: i16 = 0xFF;

/// The objects `GetItem_Draw` needs loaded (`object_table.h`'s `OBJECT_GI_HEART`,
/// `OBJECT_GI_SHIELD_1`, `OBJECT_GI_SHIELD_2`, `OBJECT_GI_CLOTHES`), by file.
const OBJECT_GI_HEART: &str = "object_gi_heart";
const OBJECT_GI_SHIELD_1: &str = "object_gi_shield_1";
const OBJECT_GI_SHIELD_2: &str = "object_gi_shield_2";
const OBJECT_GI_CLOTHES: &str = "object_gi_clothes";

/// `Object_GetSlot(&play->objectCtx, object)` for an object file.
fn object_bank(play: &PlayState, file: &str) -> Option<usize> {
    let id = play.assets.as_ref()?.scenes.objects.iter().position(|o| o == file)?;
    play.object_ctx.get_index(id as i16)
}

/// `sRupeeTex`.
const RUPEE_TEX: [&str; 5] = ["gRupeeGreenTex", "gRupeeBlueTex", "gRupeeRedTex", "gRupeePinkTex", "gRupeeOrangeTex"];
/// `sItemDropTex`.
const ITEM_DROP_TEX: [&str; 12] = [
    "gDropRecoveryHeartTex",
    "gDropBombTex",
    "gDropArrows1Tex",
    "gDropArrows2Tex",
    "gDropArrows3Tex",
    "gDropBombTex",
    "gDropDekuNutTex",
    "gDropDekuStickTex",
    "gDropMagicLargeTex",
    "gDropMagicSmallTex",
    "gDropDekuSeedsTex",
    "gDropKeySmallTex",
];

/// The segment the draws bind the texture to.
const SEG_TEX: u8 = 0x08;
/// `gItemDropDL` multiplies in segment 1's matrix, `play->billboardMtx`: the bake takes the
/// identity (an N64 `Mtx`), and the draw applies the billboard (`ViewInfo::billboard`).
const SEG_BILLBOARD: u8 = 0x01;

pub(crate) fn identity_mtx() -> Vec<u8> {
    let mut b = vec![0u8; 64];
    for i in [0, 5, 10, 15] {
        b[i * 2 + 1] = 1;
    }
    b
}
/// The segment holding `sSetupDL[SETUPDL_66]`.
const SEG_SETUP: u8 = 0x0D;

fn rupee_bake(i: usize) -> String {
    format!("En_Item00/rupee{i}")
}
fn drop_bake(i: usize) -> String {
    format!("En_Item00/drop{i}")
}

/// `EnItem00_DrawRupee`'s and `EnItem00_DrawCollectible`'s meshes.
pub fn bakes() -> Vec<MeshBake> {
    let mut v = Vec::new();
    for (i, t) in RUPEE_TEX.iter().enumerate() {
        v.push(MeshBake {
            name: rupee_bake(i),
            object: "gameplay_keep".into(),
            segments: vec![(SEG_TEX, BakeSegment::Texture { file: "gameplay_keep".into(), symbol: (*t).into() })],
            prelude: Vec::new(),
            body: BakeBody::DLists(vec![("gameplay_keep".into(), "gRupeeDL".into())]),
        });
    }
    // SETUPDL_66: gsSPTexture(0xFFFF, 0xFFFF, 0, G_TX_RENDERTILE, G_ON),
    // gsDPSetCombineMode(G_CC_DECALRGBA, G_CC_DECALRGBA),
    // gsDPSetOtherMode(G_AD_NOTPATTERN | G_CD_MAGICSQ | G_CK_NONE | G_TC_FILT | G_TF_BILERP |
    //                  G_TT_NONE | G_TL_TILE | G_TD_CLAMP | G_TP_PERSP | G_CYC_1CYCLE | G_PM_NPRIMITIVE,
    //                  G_AC_THRESHOLD | G_ZS_PIXEL | Z_CMP | Z_UPD | CVG_DST_FULL | ZMODE_OPA |
    //                  CVG_X_ALPHA | ALPHA_CVG_SEL | G_RM_PASS | GBL_c2(G_BL_CLR_IN, G_BL_0, G_BL_CLR_IN, G_BL_1)),
    // gsSPLoadGeometryMode(G_ZBUFFER | G_SHADE | G_SHADING_SMOOTH).
    let mut setup = oot_game::gbi::Dl::default();
    setup.pipe_sync();
    setup.0.push((0xD700_0002, 0xFFFF_FFFF));
    use oot_game::gbi::{ac, cc_ab, cc_c, cc_d};
    let decal = [cc_ab::ZERO, cc_ab::ZERO, cc_c::ZERO, cc_d::TEXEL0, ac::ZERO, ac::ZERO, ac::ZERO, ac::TEXEL0];
    setup.combine_lerp(decal, decal);
    setup.0.push((0xEF08_2C10, 0x0F0A_3231));
    setup.0.push((0xD900_0000, 0x0020_0005));
    setup.end();
    for (i, t) in ITEM_DROP_TEX.iter().enumerate() {
        v.push(MeshBake {
            name: drop_bake(i),
            object: "gameplay_keep".into(),
            segments: vec![
                (SEG_TEX, BakeSegment::Texture { file: "gameplay_keep".into(), symbol: (*t).into() }),
                (SEG_SETUP, BakeSegment::Commands(setup.0.clone())),
                (SEG_BILLBOARD, BakeSegment::Bytes(identity_mtx())),
            ],
            prelude: vec![SEG_SETUP],
            body: BakeBody::DLists(vec![("gameplay_keep".into(), "gItemDropDL".into())]),
        });
    }
    v
}

/// `sCylinderInit`.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_NONE, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_type: ELEM_MATERIAL_UNK0,
        toucher: ColliderElementDamageInfoAT { dmg_flags: 0, effect: 0, damage: 0 },
        bumper: ColliderElementDamageInfoACInit { dmg_flags: 0x10, effect: 0, defense: 0 },
        toucher_flags: ATELEM_NONE | ATELEM_SFX_NORMAL,
        bumper_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_NONE,
    },
    dim: eng_collision::math3d::Cylinder16 { radius: 10, height: 30, y_shift: 0, pos: [0; 3] },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_8001DFC8`: resting (spinning).
    Rest,
    /// `func_8001E1C8`: bouncing.
    Bounce,
    /// `func_8001E304`: popping up out of what dropped it.
    Pop,
    /// `EnItem00_Collected`: over Link's head.
    Collected,
}

pub struct EnItem00 {
    pub actor: Actor,
    pub action: Action,
    pub collectible_flag: i16,
    pub get_item_id: i16,
    /// `despawnTimer`: frames left (-1: placed, never despawns).
    pub despawn_timer: i16,
    pub scale: f32,
    /// `unk_154`: frames before it can be taken; `unk_156`, `unk_158`: the blink.
    pub unk_154: i16,
    pub unk_156: i16,
    pub unk_158: i16,
    pub collider: ColliderCylinder,
}

impl EnItem00 {
    /// `EnItem00_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut y_offset = 980.0;
        let spawn_param_8000 = actor.params as u16 & 0x8000 != 0;
        let collectible_flag = (actor.params & 0x3F00) >> 8;
        actor.params &= 0xFF;
        let mut e = EnItem00 {
            actor,
            action: Action::Rest,
            collectible_flag,
            get_item_id: GI_NONE,
            despawn_timer: 0,
            scale: 0.0,
            unk_154: 0,
            unk_156: 0,
            unk_158: 1,
            collider: ColliderCylinder::default(),
        };
        if play.flags.get_collectible(collectible_flag as i32) {
            e.actor.kill();
            return Box::new(e);
        }
        // sInitChain: targetArrowOffset 2000.
        e.actor.target_arrow_offset = 2000.0;
        e.collider = ColliderCylinder::new(&CYLINDER_INIT);
        let set_scale = |e: &mut EnItem00, s: f32| {
            e.actor.scale = Vec3::splat(s);
            e.scale = s;
        };
        match e.actor.params {
            ITEM00_RUPEE_GREEN | ITEM00_RUPEE_BLUE | ITEM00_RUPEE_RED => {
                set_scale(&mut e, 0.015);
                y_offset = 750.0;
            }
            ITEM00_SMALL_KEY => {
                e.unk_158 = 0;
                set_scale(&mut e, 0.03);
                y_offset = 350.0;
            }
            ITEM00_HEART_PIECE => {
                e.unk_158 = 0;
                y_offset = 650.0;
                set_scale(&mut e, 0.02);
            }
            ITEM00_RECOVERY_HEART => {
                e.actor.home_rot.z = play.rand.centered_float(65535.0) as i32 as i16;
                y_offset = 430.0;
                set_scale(&mut e, 0.02);
            }
            ITEM00_HEART_CONTAINER => {
                y_offset = 430.0;
                e.unk_158 = 0;
                set_scale(&mut e, 0.02);
            }
            ITEM00_ARROWS_SINGLE => {
                y_offset = 400.0;
                set_scale(&mut e, 0.02);
            }
            ITEM00_ARROWS_SMALL | ITEM00_ARROWS_MEDIUM | ITEM00_ARROWS_LARGE => {
                set_scale(&mut e, 0.035);
                y_offset = 250.0;
            }
            ITEM00_BOMBS_A | ITEM00_BOMBS_B | ITEM00_NUTS | ITEM00_STICK | ITEM00_MAGIC_SMALL | ITEM00_SEEDS | ITEM00_BOMBS_SPECIAL => {
                set_scale(&mut e, 0.03);
                y_offset = 320.0;
            }
            ITEM00_MAGIC_LARGE => {
                set_scale(&mut e, (0.045 - 1e-10) as f32);
                y_offset = 320.0;
            }
            ITEM00_RUPEE_ORANGE => {
                set_scale(&mut e, (0.045 - 1e-10) as f32);
                y_offset = 750.0;
            }
            ITEM00_RUPEE_PURPLE => {
                set_scale(&mut e, 0.03);
                y_offset = 750.0;
            }
            ITEM00_FLEXIBLE => {
                y_offset = 500.0;
                set_scale(&mut e, 0.01);
            }
            ITEM00_SHIELD_DEKU | ITEM00_SHIELD_HYLIAN | ITEM00_TUNIC_ZORA | ITEM00_TUNIC_GORON => {
                // objBankIndex = Object_GetSlot(OBJECT_GI_SHIELD_1 / _2 / OBJECT_GI_CLOTHES),
                // Actor_SetObjectDependency. Without the object the index is -1 and
                // Actor_UpdateAll kills the item (here at once).
                let file = match e.actor.params {
                    ITEM00_SHIELD_DEKU => OBJECT_GI_SHIELD_1,
                    ITEM00_SHIELD_HYLIAN => OBJECT_GI_SHIELD_2,
                    _ => OBJECT_GI_CLOTHES,
                };
                match object_bank(play, file) {
                    Some(bank) => e.actor.obj_bank_index = Some(bank),
                    None => {
                        log::debug!("En_Item00: {file} isn't loaded");
                        e.actor.kill();
                    }
                }
                set_scale(&mut e, 0.5);
                y_offset = 0.0;
                e.actor.world_rot.x = 0x4000;
            }
            _ => {}
        }
        e.unk_156 = 0;
        // ActorShape_Init(yOffset, ActorShadow_DrawCircle, shadowScale): no shadow drawn.
        e.actor.shape_y_offset = y_offset;
        e.actor.focus_pos = e.actor.world_pos;
        e.get_item_id = GI_NONE;
        if !spawn_param_8000 {
            e.action = Action::Rest;
            e.despawn_timer = -1;
            return Box::new(e);
        }
        e.despawn_timer = 15;
        e.unk_154 = 35;
        e.actor.speed_xz = 0.0;
        e.actor.velocity.y = 0.0;
        e.actor.gravity = 0.0;
        let mut get_item_id = GI_NONE;
        match e.actor.params {
            ITEM00_RUPEE_GREEN => {
                item_give(&mut play.save, Some(&mut play.audio), ITEM_RUPEE_GREEN);
            }
            ITEM00_RUPEE_BLUE => {
                item_give(&mut play.save, Some(&mut play.audio), ITEM_RUPEE_BLUE);
            }
            ITEM00_RUPEE_RED => {
                item_give(&mut play.save, Some(&mut play.audio), ITEM_RUPEE_RED);
            }
            ITEM00_RUPEE_PURPLE => {
                item_give(&mut play.save, Some(&mut play.audio), ITEM_RUPEE_PURPLE);
            }
            ITEM00_RUPEE_ORANGE => {
                item_give(&mut play.save, Some(&mut play.audio), ITEM_RUPEE_GOLD);
            }
            ITEM00_RECOVERY_HEART => {
                item_give(&mut play.save, Some(&mut play.audio), ITEM_RECOVERY_HEART);
            }
            ITEM00_FLEXIBLE => {
                health_change_by(&mut play.save, Some(&mut play.audio), 0x70);
            }
            ITEM00_BOMBS_A | ITEM00_BOMBS_B => {
                item_give(&mut play.save, Some(&mut play.audio), ITEM_BOMBS_5);
            }
            ITEM00_ARROWS_SINGLE => {
                item_give(&mut play.save, Some(&mut play.audio), ITEM_BOW);
            }
            ITEM00_ARROWS_SMALL => {
                item_give(&mut play.save, Some(&mut play.audio), ITEM_ARROWS_5);
            }
            ITEM00_ARROWS_MEDIUM => {
                item_give(&mut play.save, Some(&mut play.audio), ITEM_ARROWS_10);
            }
            ITEM00_ARROWS_LARGE => {
                item_give(&mut play.save, Some(&mut play.audio), ITEM_ARROWS_30);
            }
            ITEM00_SMALL_KEY => {
                item_give(&mut play.save, Some(&mut play.audio), ITEM_SMALL_KEY);
            }
            // @bug (game): the large magic jar's get-item is the small one's, and the other
            // way round.
            ITEM00_MAGIC_LARGE => get_item_id = GI_MAGIC_JAR_SMALL,
            ITEM00_MAGIC_SMALL => get_item_id = GI_MAGIC_JAR_LARGE,
            ITEM00_SEEDS => get_item_id = GI_DEKU_SEEDS_5,
            ITEM00_NUTS => get_item_id = GI_DEKU_NUTS_5,
            ITEM00_STICK => get_item_id = GI_DEKU_STICKS_1,
            _ => {}
        }
        if get_item_id != GI_NONE && !actor_has_parent(&e.actor) {
            // Actor_OfferGetItemNearby(&this->actor, play, getItemId), from inside whoever dropped it. The
            // item isn't in the actor context yet, so it can't be the offering actor here; when
            // Player dropped it (func_8083E4C4) the offer is wiped at the end of Player's update
            // anyway, so skipping it changes nothing.
            if play.cur_actor != play.player {
                log::debug!("En_Item00: an offer from a collected-at-once drop ({get_item_id:#x}) isn't made");
            }
        }
        e.action = Action::Collected;
        e.collected(play);
        Box::new(e)
    }

    /// `func_8001DFC8`: resting: rupees and hearts spin, and the despawn timer ends it.
    fn func_8001dfc8(&mut self) {
        let p = self.actor.params;
        if p <= ITEM00_RUPEE_RED || (p == ITEM00_RECOVERY_HEART && self.despawn_timer < 0) || p == ITEM00_HEART_PIECE {
            self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(960);
        } else if p >= ITEM00_SHIELD_DEKU && p != ITEM00_BOMBS_SPECIAL {
            if self.despawn_timer == -1 {
                if smooth_step_to_s(&mut self.actor.shape_rot.x, self.actor.world_rot.x.wrapping_sub(0x4000), 2, 3000, 1500) == 0 {
                    self.despawn_timer = -2;
                }
            } else if smooth_step_to_s(&mut self.actor.shape_rot.x, self.actor.world_rot.x.wrapping_neg().wrapping_sub(0x4000), 2, 3000, 1500) == 0 {
                self.despawn_timer = -1;
            }
            smooth_step_to_s(&mut self.actor.world_rot.x, 0, 2, 2500, 500);
        }
        if p == ITEM00_HEART_PIECE {
            self.actor.shape_y_offset = sin_s(self.actor.shape_rot.y) * 150.0 + 850.0;
        }
        smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 1.0, 0.5, 0.0);
        let keeps = p == ITEM00_SMALL_KEY || p == ITEM00_HEART_PIECE || p == ITEM00_HEART_CONTAINER;
        if self.unk_154 == 0 && !keeps {
            self.unk_154 = -1;
        }
        if self.despawn_timer == 0 && !keeps {
            self.actor.kill();
        }
        if self.actor.gravity != 0.0 && self.actor.bg_check_flags & BGCHECKFLAG_GROUND == 0 {
            self.action = Action::Bounce;
        }
    }

    /// `func_8001E1C8`: bounce until the fall is slower than 2.
    fn func_8001e1c8(&mut self, play: &mut PlayState) {
        if self.actor.params <= ITEM00_RUPEE_RED {
            self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(960);
        }
        if play.gameplay_frames & 1 != 0 {
            // EffectSsKiraKira_SpawnSmall at a random point around it (not drawn).
            for _ in 0..3 {
                play.rand.centered_float(10.0);
            }
        }
        if self.actor.bg_check_flags & (BGCHECKFLAG_GROUND | BGCHECKFLAG_GROUND_TOUCH) != 0 {
            let original_velocity = self.actor.velocity.y;
            if original_velocity > -2.0 {
                self.action = Action::Rest;
                self.actor.velocity.y = 0.0;
            } else {
                self.actor.velocity.y = original_velocity * -0.8;
                self.actor.bg_check_flags &= !BGCHECKFLAG_GROUND;
            }
        }
    }

    /// `func_8001E304`: popping out and falling (a recovery heart floats down, swaying).
    fn func_8001e304(&mut self, play: &mut PlayState) {
        self.despawn_timer += 1;
        if self.actor.params == ITEM00_RECOVERY_HEART && self.actor.velocity.y < 0.0 {
            self.actor.speed_xz = 0.0;
            self.actor.gravity = -0.4;
            if self.actor.velocity.y < -1.5 {
                self.actor.velocity.y = -1.5;
            }
            self.actor.home_rot.z = self.actor.home_rot.z.wrapping_add(((self.actor.velocity.y + 3.0) * 1000.0) as i32 as i16);
            self.actor.world_pos.x += cos_s(self.actor.yaw_towards_player) * (-3.0 * cos_s(self.actor.home_rot.z));
            self.actor.world_pos.z += sin_s(self.actor.yaw_towards_player) * (-3.0 * cos_s(self.actor.home_rot.z));
        }
        if self.actor.params <= ITEM00_RUPEE_RED {
            self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(960);
        } else if self.actor.params >= ITEM00_SHIELD_DEKU && self.actor.params != ITEM00_BOMBS_SPECIAL {
            self.actor.world_rot.x = self.actor.world_rot.x.wrapping_sub(700);
            self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(400);
            self.actor.shape_rot.x = self.actor.world_rot.x.wrapping_sub(0x4000);
        }
        if self.actor.velocity.y <= 2.0 {
            let rot_offset = self.actor.shape_rot.z as u16 as i32 + 10000;
            if rot_offset < 65535 {
                self.actor.shape_rot.z = self.actor.shape_rot.z.wrapping_add(10000);
            } else {
                self.actor.shape_rot.z = -1;
            }
        }
        if play.gameplay_frames & 1 == 0 {
            // EffectSsKiraKira_SpawnSmall (not drawn).
            for _ in 0..3 {
                play.rand.zero_one();
            }
        }
        if self.actor.bg_check_flags & (BGCHECKFLAG_GROUND | BGCHECKFLAG_GROUND_TOUCH) != 0 {
            self.action = Action::Rest;
            self.actor.shape_rot.z = 0;
            self.actor.velocity.y = 0.0;
            self.actor.speed_xz = 0.0;
        }
    }

    /// `EnItem00_Collected`: bouncing over Link's head until the timer runs out.
    fn collected(&mut self, play: &mut PlayState) {
        if self.get_item_id != GI_NONE {
            if !actor_has_parent(&self.actor) {
                let actor = self.actor.clone();
                offer_get_item_range(play, &actor, self.get_item_id, 50.0, 80.0);
                self.despawn_timer += 1;
            } else {
                self.get_item_id = GI_NONE;
            }
        }
        if self.despawn_timer == 0 {
            self.actor.kill();
            return;
        }
        let Some(p) = play.player.and_then(|h| play.actors.actor(h)) else { return };
        self.actor.world_pos = p.world_pos;
        if self.actor.params <= ITEM00_RUPEE_RED {
            self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(960);
        } else if self.actor.params == ITEM00_RECOVERY_HEART {
            self.actor.shape_rot.y = 0;
        }
        self.actor.world_pos.y += 40.0 + sin_s((self.despawn_timer as i32 * 15000) as i16) * (self.despawn_timer as f32 * 0.3);
        if play.save.adult {
            self.actor.world_pos.y += 20.0;
        }
    }

    /// Any DynaPoly actor moved this frame (`EnItem00_Update`'s `D_80157D94`).
    fn dyna_moved(play: &PlayState) -> bool {
        play.col.dyna.actors.iter().any(|b| b.cur.pos != b.prev.pos)
    }
}

impl ActorImpl for EnItem00 {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnItem00_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if self.despawn_timer > 0 {
            self.despawn_timer -= 1;
        }
        if self.despawn_timer > 0 && self.despawn_timer < 41 && self.unk_154 <= 0 {
            self.unk_156 = self.despawn_timer;
        }
        match self.action {
            Action::Rest => self.func_8001dfc8(),
            Action::Bounce => self.func_8001e1c8(play),
            Action::Pop => self.func_8001e304(play),
            Action::Collected => self.collected(play),
        }
        if self.actor.killed {
            return;
        }
        smooth_step_to_f(&mut self.actor.scale.x, self.scale, 0.1, self.scale * 0.1, 0.0);
        self.actor.scale.z = self.actor.scale.x;
        self.actor.scale.y = self.actor.scale.x;
        if self.actor.gravity != 0.0 {
            let mut sp3a = false;
            let mut moved = false;
            if self.actor.bg_check_flags & (BGCHECKFLAG_GROUND | BGCHECKFLAG_GROUND_TOUCH) != 0 {
                moved = Self::dyna_moved(play);
            } else {
                sp3a = true;
                self.actor.move_forward();
            }
            if sp3a || moved {
                self.actor.update_bg_check_info(&play.col, 10.0, 15.0, 15.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
                if self.actor.floor_height <= -10000.0 {
                    self.actor.kill();
                    return;
                }
            }
        }
        self.collider.update(&self.actor);
        play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
        let p = self.actor.params;
        if matches!(p, ITEM00_SHIELD_DEKU | ITEM00_SHIELD_HYLIAN | ITEM00_TUNIC_ZORA | ITEM00_TUNIC_GORON) {
            self.actor.shape_y_offset = (cos_s(self.actor.shape_rot.x) * 37.0).abs();
        }
        if self.unk_154 > 0 {
            return;
        }
        if !(self.actor.xz_dist_to_player <= 30.0 && self.actor.y_dist_to_player >= -50.0 && self.actor.y_dist_to_player <= 50.0) && !actor_has_parent(&self.actor) {
            return;
        }
        // (No game over.)
        let (save, audio) = (&mut play.save, &mut play.audio);
        let get_item_id = match p {
            ITEM00_RUPEE_GREEN => {
                item_give(save, Some(&mut *audio), ITEM_RUPEE_GREEN);
                GI_NONE
            }
            ITEM00_RUPEE_BLUE => {
                item_give(save, Some(&mut *audio), ITEM_RUPEE_BLUE);
                GI_NONE
            }
            ITEM00_RUPEE_RED => {
                item_give(save, Some(&mut *audio), ITEM_RUPEE_RED);
                GI_NONE
            }
            ITEM00_RUPEE_PURPLE => {
                item_give(save, Some(&mut *audio), ITEM_RUPEE_PURPLE);
                GI_NONE
            }
            ITEM00_RUPEE_ORANGE => {
                item_give(save, Some(&mut *audio), ITEM_RUPEE_GOLD);
                GI_NONE
            }
            ITEM00_STICK => GI_DEKU_STICKS_1,
            ITEM00_NUTS => GI_DEKU_NUTS_5,
            ITEM00_RECOVERY_HEART => {
                item_give(save, Some(&mut *audio), ITEM_RECOVERY_HEART);
                GI_NONE
            }
            ITEM00_FLEXIBLE => {
                health_change_by(save, Some(&mut *audio), 0x70);
                GI_NONE
            }
            ITEM00_BOMBS_A | ITEM00_BOMBS_B => {
                item_give(save, Some(&mut *audio), ITEM_BOMBS_5);
                GI_NONE
            }
            ITEM00_ARROWS_SINGLE => {
                item_give(save, Some(&mut *audio), ITEM_BOW);
                GI_NONE
            }
            ITEM00_ARROWS_SMALL => {
                item_give(save, Some(&mut *audio), ITEM_ARROWS_5);
                GI_NONE
            }
            ITEM00_ARROWS_MEDIUM => {
                item_give(save, Some(&mut *audio), ITEM_ARROWS_10);
                GI_NONE
            }
            ITEM00_ARROWS_LARGE => {
                item_give(save, Some(&mut *audio), ITEM_ARROWS_30);
                GI_NONE
            }
            ITEM00_SEEDS => GI_DEKU_SEEDS_5,
            ITEM00_SMALL_KEY => GI_SMALL_KEY,
            ITEM00_HEART_PIECE => GI_HEART_PIECE,
            ITEM00_HEART_CONTAINER => GI_HEART_CONTAINER,
            ITEM00_MAGIC_LARGE => GI_MAGIC_JAR_LARGE,
            ITEM00_MAGIC_SMALL => GI_MAGIC_JAR_SMALL,
            ITEM00_SHIELD_DEKU => GI_SHIELD_DEKU,
            ITEM00_SHIELD_HYLIAN => GI_SHIELD_HYLIAN,
            ITEM00_TUNIC_ZORA => GI_TUNIC_ZORA,
            ITEM00_TUNIC_GORON => GI_TUNIC_GORON,
            _ => GI_NONE,
        };
        if get_item_id != GI_NONE && !actor_has_parent(&self.actor) {
            let actor = self.actor.clone();
            offer_get_item(play, &actor, get_item_id);
        }
        // What Player holds up goes once Player has it (its parent).
        if matches!(p, ITEM00_HEART_PIECE | ITEM00_HEART_CONTAINER | ITEM00_SMALL_KEY | ITEM00_SHIELD_DEKU | ITEM00_SHIELD_HYLIAN | ITEM00_TUNIC_ZORA | ITEM00_TUNIC_GORON) {
            if actor_has_parent(&self.actor) {
                play.flags.set_collectible(self.collectible_flag as i32);
                self.actor.kill();
            }
            return;
        }
        if !(p <= ITEM00_RUPEE_RED || p == ITEM00_RUPEE_ORANGE) && get_item_id != GI_NONE {
            if actor_has_parent(&self.actor) {
                play.flags.set_collectible(self.collectible_flag as i32);
                self.actor.kill();
            }
            return;
        }
        if p <= ITEM00_RUPEE_RED || p == ITEM00_RUPEE_ORANGE {
            play.audio.play_sfx_centered(oot_game::audio::sfx::NA_SE_SY_GET_RUPY);
        } else {
            play.audio.play_sfx_centered(oot_game::audio::sfx::NA_SE_SY_GET_ITEM);
        }
        play.flags.set_collectible(self.collectible_flag as i32);
        self.despawn_timer = 15;
        self.unk_154 = 35;
        self.actor.shape_rot.z = 0;
        self.actor.speed_xz = 0.0;
        self.actor.velocity.y = 0.0;
        self.actor.gravity = 0.0;
        self.actor.scale = Vec3::splat(self.scale);
        self.get_item_id = GI_NONE;
        self.action = Action::Collected;
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0).then_some(ColliderMut::Cylinder(&mut self.collider))
    }

    /// `EnItem00_Draw`'s state change: a placed recovery heart waits for `OBJECT_GI_HEART`
    /// (`despawnTimer` -1), then depends on it (-2) and is drawn with `GetItem_Draw`.
    fn draw_update(&mut self, play: &mut PlayState) {
        if self.actor.params == ITEM00_RECOVERY_HEART && self.despawn_timer == -1 && self.unk_156 & self.unk_158 == 0 {
            if let Some(bank) = object_bank(play, OBJECT_GI_HEART)
                && play.object_ctx.is_loaded(bank)
            {
                self.actor.obj_bank_index = Some(bank);
                self.despawn_timer = -2;
            }
        }
    }

    /// The blink (`unk_156 & unk_158`), whether a recovery heart is a placed one (and one ready
    /// for `GetItem_Draw`), and the game frame (the get-item models' scroll).
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.switches = vec![(self.unk_156 & self.unk_158 != 0) as u32, (self.despawn_timer < 0) as u32, (self.despawn_timer == -2) as u32];
        rs
    }

    /// `EnItem00_Draw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        if rs.switches.first().copied().unwrap_or(0) != 0 {
            return;
        }
        let m = actor_draw_matrix(rs);
        let p = self.actor.params;
        let placed = rs.switches.get(1).copied().unwrap_or(0) != 0;
        let heart_ready = rs.switches.get(2).copied().unwrap_or(0) != 0;
        let get_item_draw = |out: &mut DrawOut, draw_id: i16, m: glam::Mat4| {
            if let Some(a) = &play.assets {
                oot_game::draw::get_item_draw(&a.items, draw_id, m, play.gameplay_frames, view, out);
            }
        };
        match p {
            ITEM00_RUPEE_GREEN | ITEM00_RUPEE_BLUE | ITEM00_RUPEE_RED | ITEM00_RUPEE_ORANGE | ITEM00_RUPEE_PURPLE => {
                // EnItem00_DrawRupee (func_8002EBCC's highlight is the mesh's texgen).
                let i = if p <= ITEM00_RUPEE_RED { p } else { p - 0x10 } as usize;
                out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(&rupee_bake(i))), m));
            }
            ITEM00_HEART_PIECE => out.xlu.push(DrawCmd::new(MeshKey::named(keys::mesh("gameplay_keep", "gHeartPieceInteriorDL")), m)),
            ITEM00_HEART_CONTAINER => {
                out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh("gameplay_keep", "gHeartPieceExteriorDL")), m));
                out.xlu.push(DrawCmd::new(MeshKey::named(keys::mesh("gameplay_keep", "gHeartContainerInteriorDL")), m));
            }
            // A placed recovery heart: GetItem_Draw(GID_RECOVERY_HEART) at 16 times the scale,
            // once its object is loaded.
            ITEM00_RECOVERY_HEART if placed => {
                if heart_ready {
                    get_item_draw(out, GID_RECOVERY_HEART, m * glam::Mat4::from_scale(Vec3::splat(16.0)));
                }
            }
            ITEM00_RECOVERY_HEART
            | ITEM00_BOMBS_A
            | ITEM00_BOMBS_B
            | ITEM00_BOMBS_SPECIAL
            | ITEM00_ARROWS_SINGLE
            | ITEM00_ARROWS_SMALL
            | ITEM00_ARROWS_MEDIUM
            | ITEM00_ARROWS_LARGE
            | ITEM00_NUTS
            | ITEM00_STICK
            | ITEM00_MAGIC_LARGE
            | ITEM00_MAGIC_SMALL
            | ITEM00_SEEDS
            | ITEM00_SMALL_KEY => {
                // EnItem00_DrawCollectible.
                let mut i = p - 3;
                if p == ITEM00_BOMBS_SPECIAL {
                    i = 1;
                } else if p >= ITEM00_ARROWS_SMALL {
                    i -= 3;
                }
                out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(&drop_bake(i as usize))), m * view.billboard));
            }
            ITEM00_SHIELD_DEKU => get_item_draw(out, GID_SHIELD_DEKU, m),
            ITEM00_SHIELD_HYLIAN => get_item_draw(out, GID_SHIELD_HYLIAN, m),
            ITEM00_TUNIC_ZORA => get_item_draw(out, GID_TUNIC_ZORA, m),
            ITEM00_TUNIC_GORON => get_item_draw(out, GID_TUNIC_GORON, m),
            // ITEM00_FLEXIBLE: nothing.
            _ => {}
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// `func_8001F404`: what a drop becomes for Link's age, health and items, or -1 for nothing
/// (bombs, arrows, seeds and magic need the bombs, the bow, the slingshot and the meter; a
/// recovery heart at full health is a green rupee).
pub fn func_8001f404(play: &PlayState, mut drop_id: i16) -> i16 {
    if play.save.adult {
        if drop_id == ITEM00_SEEDS {
            drop_id = ITEM00_ARROWS_SMALL;
        } else if drop_id == ITEM00_STICK {
            drop_id = ITEM00_RUPEE_GREEN;
        }
    } else if matches!(drop_id, ITEM00_ARROWS_SMALL | ITEM00_ARROWS_MEDIUM | ITEM00_ARROWS_LARGE) {
        drop_id = ITEM00_SEEDS;
    }
    let s = &play.save;
    if (matches!(drop_id, ITEM00_BOMBS_A | ITEM00_BOMBS_SPECIAL | ITEM00_BOMBS_B) && s.inv_content(ITEM_BOMB) == ITEM_NONE)
        || (matches!(drop_id, ITEM00_ARROWS_SMALL | ITEM00_ARROWS_MEDIUM | ITEM00_ARROWS_LARGE) && s.inv_content(ITEM_BOW) == ITEM_NONE)
        || (matches!(drop_id, ITEM00_MAGIC_LARGE | ITEM00_MAGIC_SMALL) && s.magic_level == 0)
        || (drop_id == ITEM00_SEEDS && s.inv_content(ITEM_SLINGSHOT) == ITEM_NONE)
    {
        return -1;
    }
    if drop_id == ITEM00_RECOVERY_HEART && play.save.health_capacity == play.save.health {
        return ITEM00_RUPEE_GREEN;
    }
    drop_id
}

/// The spawned drop's start: popping up at `vy`, 2 across, a random direction, from scale 0,
/// 220 frames; it belongs to no room (keys and hearts aside).
fn setup_pop(play: &mut PlayState, h: ActorHandle, vy: f32, yaw: i16) {
    if let Some(e) = play.actors.downcast_mut::<EnItem00>(h) {
        e.actor.velocity.y = vy;
        e.actor.speed_xz = 2.0;
        e.actor.gravity = -0.9;
        e.actor.world_rot.y = yaw;
        e.actor.scale = Vec3::ZERO;
        e.action = Action::Pop;
        e.despawn_timer = 220;
        if !matches!(e.actor.params, ITEM00_SMALL_KEY | ITEM00_HEART_PIECE | ITEM00_HEART_CONTAINER) {
            e.actor.room = -1;
        }
        e.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
    }
}

/// `Item_DropCollectible`: drop `params`'s item (`0x4000`: falling instead of popping up;
/// `0x8000`: taken at once).
pub fn item_drop_collectible(play: &mut PlayState, spawn_pos: Vec3, params: i16) -> Option<ActorHandle> {
    let param4000 = params & 0x4000 != 0;
    let param8000 = params as u16 & 0x8000 != 0;
    let param3f00 = params & 0x3F00;
    let mut params = params & 0x3FFF;
    if params & 0xFF == ITEM00_FLEXIBLE && !param4000 {
        // A healing fairy (En_Elf), and its sound (EffectSsDeadSound: not ported).
        return play.actor_spawn(ACTOR_EN_ELF, spawn_pos + Vec3::Y * 40.0, [0; 3], FAIRY_HEAL_TIMED).ok();
    }
    if !param8000 {
        params = func_8001f404(play, params & 0xFF);
    }
    if params == -1 {
        return None;
    }
    let h = play.actor_spawn(ACTOR_EN_ITEM00, spawn_pos, [0; 3], params | if param8000 { i16::MIN } else { 0 } | param3f00).ok()?;
    if !param8000 {
        let yaw = play.rand.centered_float(65536.0) as i32 as i16;
        setup_pop(play, h, if !param4000 { 8.0 } else { -2.0 }, yaw);
    }
    Some(h)
}

/// `Item_DropCollectibleRandom`: a drop from table `params >> 4` (`sItemDropIds`, a random one
/// of its 16), as many as `sDropQuantities` says. `ITEM00_FLEXIBLE` gives what Link needs most.
pub fn item_drop_collectible_random(play: &mut PlayState, from_actor: Option<&Actor>, spawn_pos: Vec3, params: i16) {
    let Some(assets) = play.assets.clone() else { return };
    let tables = &assets.item_drops;
    let mut drop_table_index = (play.rand.zero_one() * 16.0) as i16;
    let param8000 = params as u16 & 0x8000 != 0;
    let mut params = params & 0x7FFF;
    // fromActor->dropFlag, which picks a fixed table and entry, isn't kept (0).
    let _ = from_actor;
    let id_at = |params: i16, i: i16| tables.ids.get((params + i) as usize).copied().unwrap_or(ITEM00_NONE as u8);
    let mut drop_id = id_at(params, drop_table_index) as i16;
    if drop_id == ITEM00_FLEXIBLE {
        let s = &play.save;
        if s.health <= 0x10 {
            // One heart or less: a healing fairy.
            let _ = play.actor_spawn(ACTOR_EN_ELF, spawn_pos + Vec3::Y * 40.0, [0; 3], FAIRY_HEAL_TIMED);
            return;
        } else if s.health <= 0x30 {
            params = 0xB * 0x10;
            drop_table_index = 0;
            drop_id = ITEM00_RECOVERY_HEART;
        } else if s.health <= 0x50 {
            params = 0xA * 0x10;
            drop_table_index = 0;
            drop_id = ITEM00_RECOVERY_HEART;
        } else if s.magic_level != 0 && s.magic == 0 {
            params = 0xA * 0x10;
            drop_table_index = 0;
            drop_id = ITEM00_MAGIC_LARGE;
        } else if s.magic_level != 0 && s.magic <= (s.magic_level >> 1) {
            params = 0xA * 0x10;
            drop_table_index = 0;
            drop_id = ITEM00_MAGIC_SMALL;
        } else if !s.adult && s.ammo(ITEM_SLINGSHOT) < 6 {
            params = 0xA * 0x10;
            drop_table_index = 0;
            drop_id = ITEM00_SEEDS;
        } else if s.adult && s.ammo(ITEM_BOW) < 6 {
            params = 0xA * 0x10;
            drop_table_index = 0;
            drop_id = ITEM00_ARROWS_MEDIUM;
        } else if s.ammo(ITEM_BOMB) < 6 {
            params = 0xD * 0x10;
            drop_table_index = 0;
            drop_id = ITEM00_BOMBS_A;
        } else if s.rupees < 11 {
            params = 0xA * 0x10;
            drop_table_index = 0;
            drop_id = ITEM00_RUPEE_RED;
        } else {
            return;
        }
    }
    if drop_id == ITEM00_NONE {
        return;
    }
    let mut drop_quantity = tables.quantities.get((params + drop_table_index) as usize).copied().unwrap_or(0) as i16;
    while drop_quantity > 0 {
        if !param8000 {
            // dropId is a u8: func_8001F404's -1 is ITEM00_NONE.
            drop_id = func_8001f404(play, drop_id) as u8 as i16;
            if drop_id != ITEM00_NONE
                && let Ok(h) = play.actor_spawn(ACTOR_EN_ITEM00, spawn_pos, [0; 3], drop_id)
            {
                let yaw = (play.rand.zero_one() * 40000.0) as i32 as i16;
                setup_pop(play, h, 8.0, yaw);
            }
        } else {
            item_drop_collectible(play, spawn_pos, params | i16::MIN);
        }
        drop_quantity -= 1;
    }
}
