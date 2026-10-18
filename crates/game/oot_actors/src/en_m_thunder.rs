//! `En_M_Thunder` (`ovl_En_M_Thunder/z_en_m_thunder.c`): the spin attack's effect, spawned by
//! Player at his waist (`func_80837530`) for the quick spin (stick spun, params' high byte 0) and
//! the charged one (B held after a slash, high byte 2: the magic it costs).
//!
//! - **Its init** (`EnMThunder_Init`): with `PLAYER_STATE2_17` already on (the quick spin, or a
//!   spin from a cutscene) it needs magic (`isMagicAcquired`, the meter idle, the high byte's
//!   magic consumed at once, `MAGIC_CONSUME_NOW`), else it's gone at once with the plain spin's
//!   sounds (`NA_SE_IT_ROLLING_CUT`, `NA_SE_IT_SWORD_SWING_HARD`); with it, it turns Player's
//!   `PLAYER_STATE2_17` off (his sword's quads stop attacking: its cylinder does) and spins.
//!   Otherwise it charges.
//! - **Charging** (`EnMThunder_ChargingSpinAttack`): at Link's waist, his charge (`unk_858`)
//!   copied; from 0.1 the magic shown (`MAGIC_CONSUME_WAIT_PREVIEW`), none or the meter busy
//!   empties it (`EnMThunder_EmptySpinAttack`: gone once the spin starts or the charge stops);
//!   the rumble (logged: no rumble, BACKLOG #60); from 0.15 the charge drawn whole, its
//!   `Eff_Dust` sparkles (its child, params `swordType + 2`) and the scene darkened; the charge's
//!   sound by level (0.1, 0.15, 0.85). Released (`PLAYER_STATE2_17`) at 0.15 or less it's gone
//!   (the plain spin's sounds from 0.1); above, the meter consumes (`MAGIC_STATE_CONSUME_SETUP`)
//!   and it spins: below 0.85 weak (`DMG_SPIN_*`, scale 2 for the Kokiri Sword, else 4), at 0.85
//!   and up strong (`DMG_JUMP_*`, 4 or 8).
//! - **Spinning** (`EnMThunder_SpinAttacking`): the timer from 1 to 0 by 1/16 (16 frames), the
//!   scale to its target, the cylinder's radius 25 × the scale, attacking (`CollisionCheck_SetAT`);
//!   following Link's waist for 8 frames; the trail's alpha and scroll; the charge fading.
//! - **Every frame** the scene darkened by `dimmingIntensity` (`Environment_AdjustLights`) and its
//!   point light (white to blue by the timer, radius 800 × it); its destroy resets the magic
//!   (`Magic_Reset`, if it used any), the light and the darkening.
//! - **Its draw:** the trail (`gSpinAttack1DL`, `2DL` weak, `3DL`, `4DL` strong) at its own
//!   matrix a fiftieth the size, scrolled on segment 8; the charge (`gSpinAttackChargingDL`) along
//!   the sword (Player's `mf_9E0`, by sword type), pulsing (`sSpinChargeScale`), blue or (from
//!   0.85) orange, scrolled on segment 9.
//!
//! Player spawns it from inside his update: its init reads him as he was then
//! (`spawn_from_player`, ADR 0072's way of a spawn from Player).
//!
//! **@bug (game)**, kept: the charge's `Matrix_RotateX(16384.0f)` is in radians (a binary angle's
//! quarter turn meant): about 3.74 radians. The hammer (`PLAYER_MELEEWEAPON_HAMMER`, `swordType`
//! 4) and the Deku Stick (3, only from a cutscene's charge) read past `sSpinAttackDmgFlags` and
//! `sJumpAttackDmgFlags` (see `spin_dmg_flags`) and draw the charge at the hand untransformed.
//!
//! Params: bits 0 to 7 the weapon (`Player_GetMeleeWeaponHeld`: `swordType + 1`), bits 8 to 15
//! the magic it costs (0 the quick spin's, 2 the charged spin's). Not placed: spawned by Player.
//! Flags: none.

use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{smooth_step_to_f, step_to_f};
use glam::{Mat4, Vec3};
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorHandle, ActorImpl, ActorProfile, PLAYER_BODYPART_WAIST};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::audio::sfx::{NA_SE_IT_SWORD_SWING_HARD, SfxPos};
use oot_game::collision_check::*;
use oot_game::lights::{LightInfo, LightNode};
use oot_game::magic::{MAGIC_CONSUME_NOW, MAGIC_CONSUME_WAIT_PREVIEW, MAGIC_STATE_CONSUME_SETUP, MAGIC_STATE_IDLE};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::scene_table::gfx_two_tex_scroll;
use oot_game::sys_matrix::MtxF;

use crate::eff_dust::ACTOR_EFF_DUST;
use crate::player::{Player, STATE1_12, STATE2_17};

/// `ACTOR_EN_M_THUNDER` (`actor_table.h`: 0x0057).
pub const ACTOR_EN_M_THUNDER: i16 = 0x0057;
/// `En_M_Thunder_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_M_THUNDER, name: "En_M_Thunder", category: ACTORCAT_ITEMACTION, flags: 0, object: "gameplay_keep", schema: &SCHEMA };

/// `PARAMS_GET_S(params, 0, 8)`: Player's weapon (`PlayerMeleeWeapon`, `player.h`), `swordType + 1`.
pub const WEAPON: Field = Field::params(
    "PARAMS_GET_S(params, 0, 8)",
    0,
    8,
    Kind::Named(&[(1, "PLAYER_MELEEWEAPON_SWORD_MASTER"), (2, "PLAYER_MELEEWEAPON_SWORD_KOKIRI"), (3, "PLAYER_MELEEWEAPON_SWORD_BIGGORON"), (4, "PLAYER_MELEEWEAPON_DEKU_STICK"), (5, "PLAYER_MELEEWEAPON_HAMMER")]),
    "the weapon spun (Player_GetMeleeWeaponHeld): the trail's size and damage, the charge's place",
);
/// `PARAMS_GET_S(params, 8, 8)`: the magic it costs.
pub const MAGIC: Field = Field::params("PARAMS_GET_S(params, 8, 8)", 8, 8, Kind::Range(0, 0xFF), "the magic it costs (0 the quick spin's, 2 the charged spin's)");

/// En_M_Thunder's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[WEAPON, MAGIC], flags: &[], todo: false };

/// `NA_SE_IT_ROLLING_CUT` (`itembank_table.h`: 0x1823).
pub const NA_SE_IT_ROLLING_CUT: u16 = 0x1823;
/// `NA_SE_IT_ROLLING_CUT_LV1` (`itembank_table.h`: 0x182E).
pub const NA_SE_IT_ROLLING_CUT_LV1: u16 = 0x182E;
/// `NA_SE_IT_ROLLING_CUT_LV2` (`itembank_table.h`: 0x182F).
pub const NA_SE_IT_ROLLING_CUT_LV2: u16 = 0x182F;

/// `EnMThunderSwordType`.
pub const M_THUNDER_SWORD_MASTER: u8 = 0;
pub const M_THUNDER_SWORD_KOKIRI: u8 = 1;
pub const M_THUNDER_SWORD_GIANT: u8 = 2;

/// `EnMThunderAttackStrength`.
pub const M_THUNDER_ATTACK_STRONG: u8 = 0;
pub const M_THUNDER_ATTACK_WEAK: u8 = 1;

/// `sCylinderInit`.
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_ON | AT_TYPE_PLAYER, ac_flags: AC_NONE, oc_flags1: OC1_NONE, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK2,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0001, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 200, height: 200, y_shift: 0, pos: [0; 3] },
};

/// `sSpinAttackDmgFlags`, by `swordType`.
const S_SPIN_ATTACK_DMG_FLAGS: [u32; 3] = [DMG_SPIN_MASTER, DMG_SPIN_KOKIRI, DMG_SPIN_GIANT];
/// `sJumpAttackDmgFlags`, by `swordType`.
const S_JUMP_ATTACK_DMG_FLAGS: [u32; 3] = [DMG_JUMP_MASTER, DMG_JUMP_KOKIRI, DMG_JUMP_GIANT];
/// `sSfxIds` (`EnMThunder_ChargingSpinAttack`), by `attackStrength`.
const S_SFX_IDS: [u16; 4] = [NA_SE_IT_ROLLING_CUT_LV2, NA_SE_IT_ROLLING_CUT_LV1, NA_SE_IT_ROLLING_CUT_LV2, NA_SE_IT_ROLLING_CUT_LV1];
/// `sSpinChargeScale` (`EnMThunder_Draw`).
const S_SPIN_CHARGE_SCALE: [f32; 8] = [0.1, 0.15, 0.2, 0.25, 0.3, 0.25, 0.2, 0.15];

/// `sSpinAttackDmgFlags[swordType]` (`spin`) or `sJumpAttackDmgFlags[swordType]`.
///
/// @bug (game), kept: the hammer's `swordType` (4) and the Deku Stick's (3) are past both
/// arrays: the spin's reads `sJumpAttackDmgFlags[swordType - 3]` (the array after it), the jump's
/// the data after that, assumed here to be `sSfxIds` (`NA_SE_IT_ROLLING_CUT_LV2`'s and `_LV1`'s
/// halves as one word; an open question: the overlay's data order isn't checked).
fn spin_dmg_flags(sword_type: u8, spin: bool) -> u32 {
    let i = sword_type as usize;
    match (spin, i) {
        (true, 0..=2) => S_SPIN_ATTACK_DMG_FLAGS[i],
        (true, 3..=5) => S_JUMP_ATTACK_DMG_FLAGS[i - 3],
        (false, 0..=2) => S_JUMP_ATTACK_DMG_FLAGS[i],
        (false, 3..=4) => ((S_SFX_IDS[(i - 3) * 2] as u32) << 16) | S_SFX_IDS[(i - 3) * 2 + 1] as u32,
        _ => {
            log::warn!("En_M_Thunder: swordType {sword_type} past the damage tables");
            0
        }
    }
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `EnMThunder_ChargingSpinAttack`.
    ChargingSpinAttack,
    /// `EnMThunder_EmptySpinAttack`: no magic for the charge.
    EmptySpinAttack,
    /// `EnMThunder_SpinAttacking`.
    SpinAttacking,
}

/// What Player was when he asked for the spawn (`func_80837530`, inside his update, where the
/// C's init runs): his waist, his shape's yaw and `stateFlags2`. The init's write to his
/// `stateFlags2` comes back in `cleared_state2_17`.
#[derive(Debug, Clone, Copy, Default)]
struct Handover {
    player: Option<(Vec3, i16, u32)>,
    cleared_state2_17: bool,
}

/// `func_80837530`'s `Actor_Spawn(ACTOR_EN_M_THUNDER, waist, 0, 0, 0, params)`, run by Player
/// after his update with what he was at the call. True if the init turned his
/// `PLAYER_STATE2_17` off (Player applies it).
pub fn spawn_from_player(play: &mut PlayState, waist: Vec3, params: i16, shape_yaw: i16, state2: u32) -> bool {
    *play.overlay_static::<Handover>(ACTOR_EN_M_THUNDER) = Handover { player: Some((waist, shape_yaw, state2)), cleared_state2_17: false };
    // schema: the whole params handed on (Player_GetMeleeWeaponHeld | arg2).
    if let Err(e) = play.actor_spawn(ACTOR_EN_M_THUNDER, waist, [0; 3], params) {
        log::debug!("En_M_Thunder: {e:?}");
    }
    std::mem::take(play.overlay_static::<Handover>(ACTOR_EN_M_THUNDER)).cleared_state2_17
}

/// `GET_PLAYER(play)`.
fn player(play: &PlayState) -> Option<&Player> {
    play.player.and_then(|h| play.actors.downcast::<Player>(h))
}

/// `&player->actor.projectedPos`.
fn player_sfx_pos(play: &PlayState) -> SfxPos {
    play.player.map(SfxPos::Actor).unwrap_or(SfxPos::Default)
}

/// `SFX_PLAY_AT_POS(&player->actor.projectedPos, sfx)`.
fn player_sfx(play: &mut PlayState, sfx: u16) {
    let pos = player_sfx_pos(play);
    play.audio.play_sfx_at_pos(pos, sfx);
}

/// `player->stateFlags2 &= ~PLAYER_STATE2_17`.
fn clear_player_state2_17(play: &mut PlayState) {
    if let Some(p) = play.player.and_then(|h| play.actors.downcast_mut::<Player>(h)) {
        p.state2 &= !STATE2_17;
    }
}

pub struct EnMThunder {
    pub actor: Actor,
    pub collider: ColliderCylinder,
    pub light_node: Option<LightNode>,
    pub light_info: LightInfo,
    pub spin_attack_timer: f32,
    pub spin_attack_alpha: f32,
    pub spin_trail_tex_scroll: f32,
    pub spin_charge_percent: f32,
    pub dimming_intensity: f32,
    pub action: Action,
    pub follow_player_timer: u16,
    pub attack_strength: u8,
    pub sword_type: u8,
    pub charge_alpha: u8,
    pub target_scale: u8,
    pub is_using_magic: bool,
    /// Player's `mf_9E0` at this frame's draw (Player draws first: `Actor_DrawAll` by category).
    pub player_mf: Mat4,
}

impl EnMThunder {
    /// `EnMThunder_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let collider = ColliderCylinder::new(&CYLINDER_INIT);
        let p = actor.world_pos;
        let mut this = EnMThunder {
            actor,
            collider,
            light_node: None,
            light_info: LightInfo::point_no_glow(p.x as i32 as i16, p.y as i32 as i16, p.z as i32 as i16, [255, 255, 255], 0),
            spin_attack_timer: 0.0,
            spin_attack_alpha: 0.0,
            spin_trail_tex_scroll: 0.0,
            spin_charge_percent: 0.0,
            dimming_intensity: 0.0,
            action: Action::ChargingSpinAttack,
            follow_player_timer: 0,
            attack_strength: 0,
            sword_type: 0,
            charge_alpha: 0,
            target_scale: 0,
            is_using_magic: false,
            player_mf: Mat4::IDENTITY,
        };
        this.sword_type = (WEAPON.get(this.actor.params) - 1) as u8;
        this.light_node = play.light_ctx.insert_light(this.light_info);
        this.collider.dim.radius = 0;
        this.collider.dim.height = 40;
        this.collider.dim.y_shift = -20;
        this.follow_player_timer = 8;
        this.spin_trail_tex_scroll = 0.0;
        // Player as the spawn's call saw him, or (spawned some other way) as he is.
        let handed = play.overlay_static::<Handover>(ACTOR_EN_M_THUNDER).player.take();
        let from_player = handed.is_some();
        let (waist, shape_yaw, state2) = handed.or_else(|| player(play).map(|p| (p.body_parts_pos[PLAYER_BODYPART_WAIST], p.actor.shape_rot.y, p.state2))).unwrap_or((this.actor.world_pos, 0, 0));
        this.actor.world_pos = waist;
        this.spin_attack_timer = 0.0;
        this.dimming_intensity = 0.0;
        this.actor.shape_rot.y = shape_yaw.wrapping_add(0x8000u16 as i16);
        this.actor.room = -1;
        this.actor.scale = Vec3::splat(0.1);
        this.is_using_magic = false;
        let magic = MAGIC.get(this.actor.params);
        if state2 & STATE2_17 != 0 {
            if !play.save.is_magic_acquired || play.save.magic_state != MAGIC_STATE_IDLE || (magic != 0 && !play.magic_request_change(magic, MAGIC_CONSUME_NOW)) {
                player_sfx(play, NA_SE_IT_ROLLING_CUT);
                player_sfx(play, NA_SE_IT_SWORD_SWING_HARD);
                this.actor.kill();
                return Box::new(this);
            }
            if from_player {
                play.overlay_static::<Handover>(ACTOR_EN_M_THUNDER).cleared_state2_17 = true;
            } else {
                clear_player_state2_17(play);
            }
            this.is_using_magic = true;
            this.collider.info.at_dmg_info.dmg_flags = spin_dmg_flags(this.sword_type, true);
            this.attack_strength = M_THUNDER_ATTACK_WEAK;
            this.target_scale = if this.sword_type == M_THUNDER_SWORD_KOKIRI { 2 } else { 4 };
            this.action = Action::SpinAttacking;
            this.follow_player_timer = 8;
            player_sfx(play, NA_SE_IT_ROLLING_CUT_LV1);
            this.spin_attack_timer = 1.0;
        } else {
            this.action = Action::ChargingSpinAttack;
        }
        this.actor.child = None;
        Box::new(this)
    }

    /// `EnMThunder_AdjustEnvLights`: the scene darkened by `intensity`.
    fn adjust_env_lights(play: &mut PlayState, intensity: f32) {
        play.environment_adjust_lights(intensity, 850.0, 0.2, 0.0);
    }

    /// `EnMThunder_EmptySpinAttack`: no magic for the charge: gone once the spin is released
    /// (the plain spin's sounds for a spin) or the charge stops.
    fn empty_spin_attack(&mut self, play: &mut PlayState) {
        let Some((state1, state2, mwa)) = player(play).map(|p| (p.state1, p.state2, p.melee_weapon_animation)) else { return };
        if state2 & STATE2_17 != 0 {
            if mwa >= play.data.items.mwa("SPIN_ATTACK_1H") {
                player_sfx(play, NA_SE_IT_ROLLING_CUT);
                player_sfx(play, NA_SE_IT_SWORD_SWING_HARD);
            }
            self.actor.kill();
            return;
        }
        if state1 & STATE1_12 == 0 {
            self.actor.kill();
        }
    }

    /// `this->actor.child->parent = NULL`, if it's still there (`child->update != NULL`).
    fn release_child(&mut self, play: &mut PlayState) {
        if let Some(c) = self.actor.child.and_then(|h| play.actors.actor_mut(h))
            && !c.killed
        {
            c.parent = None;
        }
    }

    /// `EnMThunder_ChargingSpinAttack`.
    fn charging_spin_attack(&mut self, play: &mut PlayState) {
        let Some((unk_858, waist, shape_yaw, state1, state2, mwa)) = player(play).map(|p| (p.unk_858, p.body_parts_pos[PLAYER_BODYPART_WAIST], p.actor.shape_rot.y, p.state1, p.state2, p.melee_weapon_animation)) else { return };
        let spin_1h = play.data.items.mwa("SPIN_ATTACK_1H");
        self.spin_charge_percent = unk_858;
        self.actor.world_pos = waist;
        self.actor.shape_rot.y = shape_yaw.wrapping_add(0x8000u16 as i16);
        let magic = MAGIC.get(self.actor.params);
        if !self.is_using_magic && unk_858 >= 0.1 {
            if play.save.magic_state != MAGIC_STATE_IDLE || (magic != 0 && !play.magic_request_change(magic, MAGIC_CONSUME_WAIT_PREVIEW)) {
                self.empty_spin_attack(play);
                self.action = Action::EmptySpinAttack;
                self.charge_alpha = 0;
                self.dimming_intensity = 0.0;
                self.spin_attack_timer = 0.0;
                return;
            }
            self.is_using_magic = true;
        }
        if unk_858 >= 0.1 {
            // Rumble_Request(0.0f, unk_858 * 150, 2, unk_858 * 150): no rumble (BACKLOG #60).
            log::trace!("En_M_Thunder: Rumble_Request(0, {}, 2, {}) not ported", (unk_858 * 150.0) as i32, (unk_858 * 150.0) as i32);
        }
        if state2 & STATE2_17 != 0 {
            self.release_child(play);
            if unk_858 <= 0.15 {
                if unk_858 >= 0.1 && mwa >= spin_1h {
                    player_sfx(play, NA_SE_IT_ROLLING_CUT);
                    player_sfx(play, NA_SE_IT_SWORD_SWING_HARD);
                }
                self.actor.kill();
                return;
            }
            clear_player_state2_17(play);
            if magic != 0 {
                play.save.magic_state = MAGIC_STATE_CONSUME_SETUP;
            }
            if unk_858 < 0.85 {
                self.collider.info.at_dmg_info.dmg_flags = spin_dmg_flags(self.sword_type, true);
                self.attack_strength = M_THUNDER_ATTACK_WEAK;
                self.target_scale = if self.sword_type == M_THUNDER_SWORD_KOKIRI { 2 } else { 4 };
            } else {
                self.collider.info.at_dmg_info.dmg_flags = spin_dmg_flags(self.sword_type, false);
                self.attack_strength = M_THUNDER_ATTACK_STRONG;
                self.target_scale = if self.sword_type == M_THUNDER_SWORD_KOKIRI { 4 } else { 8 };
            }
            self.action = Action::SpinAttacking;
            self.follow_player_timer = 8;
            player_sfx(play, S_SFX_IDS[self.attack_strength as usize]);
            self.spin_attack_timer = 1.0;
            return;
        }
        if state1 & STATE1_12 == 0 {
            // (The C clears the child's parent without the update check.)
            if let Some(c) = self.actor.child.and_then(|h| play.actors.actor_mut(h)) {
                c.parent = None;
            }
            self.actor.kill();
            return;
        }
        if unk_858 > 0.15 {
            self.charge_alpha = 255;
            if self.actor.child.is_none() {
                let (p, yaw) = (self.actor.world_pos, self.actor.shape_rot.y);
                // Actor_SpawnAsChild(.., ACTOR_EFF_DUST, world.pos, 0, shape.rot.y, 0, swordType + 2).
                let params = self.sword_type as i16 + 2;
                if let Err(e) = play.actor_spawn_as_child(&mut self.actor, ACTOR_EFF_DUST, p, [0, yaw, 0], params) {
                    log::debug!("Eff_Dust: {e:?}");
                }
            }
            self.dimming_intensity += (((unk_858 - 0.15) * 1.5) - self.dimming_intensity) * 0.5;
        } else if unk_858 > 0.1 {
            self.charge_alpha = ((unk_858 - 0.1) * 255.0 * 20.0) as i32 as u8;
            self.spin_attack_timer = (unk_858 - 0.1) * 10.0;
        } else {
            self.charge_alpha = 0;
        }
        let pos = player_sfx_pos(play);
        if unk_858 > 0.85 {
            play.audio.audio_play_sword_charge_sfx(pos, 2);
        } else if unk_858 > 0.15 {
            play.audio.audio_play_sword_charge_sfx(pos, 1);
        } else if unk_858 > 0.1 {
            play.audio.audio_play_sword_charge_sfx(pos, 0);
        }
        if play.play_in_cs_mode() {
            self.actor.kill();
        }
    }

    /// `EnMThunder_UpdateSpinAttack`: the charge fades by 40 in the last two frames of following,
    /// the trail scrolls by twice its alpha, the darkening up to the timer by a tenth (down at
    /// once).
    fn update_spin_attack(&mut self) {
        if self.follow_player_timer < 2 {
            self.charge_alpha = if self.charge_alpha < 40 { 0 } else { self.charge_alpha - 40 };
        }
        self.spin_trail_tex_scroll += 2.0 * self.spin_attack_alpha;
        if self.dimming_intensity < self.spin_attack_timer {
            self.dimming_intensity += (self.spin_attack_timer - self.dimming_intensity) * 0.1;
        } else {
            self.dimming_intensity = self.spin_attack_timer;
        }
    }

    /// `EnMThunder_SpinAttacking`.
    fn spin_attacking(&mut self, play: &mut PlayState) {
        if step_to_f(&mut self.spin_attack_timer, 0.0, 1.0 / 16.0) {
            self.actor.kill();
        } else {
            smooth_step_to_f(&mut self.actor.scale.x, self.target_scale as i32 as f32, 0.6, 0.8, 0.0);
            // Actor_SetScale(&this->actor, this->actor.scale.x).
            self.actor.scale = Vec3::splat(self.actor.scale.x);
            self.collider.dim.radius = (self.actor.scale.x * 25.0) as i16;
            self.collider.update(&self.actor);
            play.collision_check_set_at(&self.actor, 0, &mut self.collider);
        }
        if self.follow_player_timer > 0 {
            if let Some(waist) = player(play).map(|p| p.body_parts_pos[PLAYER_BODYPART_WAIST]) {
                self.actor.world_pos.x = waist.x;
                self.actor.world_pos.z = waist.z;
            }
            self.follow_player_timer -= 1;
        }
        self.spin_attack_alpha = if self.spin_attack_timer > 0.6 { 1.0 } else { self.spin_attack_timer * (5.0 / 3.0) };
        self.update_spin_attack();
        if play.play_in_cs_mode() {
            self.actor.kill();
        }
    }
}

pub const BAKE_WEAK: &str = "En_M_Thunder/weak";
pub const BAKE_STRONG: &str = "En_M_Thunder/strong";
pub const BAKE_CHARGE: &str = "En_M_Thunder/charge";
/// The draw's colours (`gDPSetPrimColor`, `gDPSetEnvColor` before each list).
const SEG_COLOR: u8 = 0x0A;
/// The trail's scroll (`gSPSegment(0x08, Gfx_TwoTexScroll(..))`, which the lists call).
const SEG_TRAIL: u8 = 0x08;
/// The charge's scroll (segment 9).
const SEG_CHARGE: u8 = 0x09;

/// The trail's `Gfx_TwoTexScroll(.., G_TX_RENDERTILE, 0xFF - ((u8)(s32)(scroll * 30) & 0xFF), 0,
/// 64, 32, 1, 0xFF - ((u8)(s32)(scroll * 20) & 0xFF), 0, 8, 8)`.
fn trail_scroll(scroll: f32) -> Vec<(u32, u32)> {
    let a = 0xFF - ((scroll * 30.0) as i32 as u8 as u32);
    let b = 0xFF - ((scroll * 20.0) as i32 as u8 as u32);
    gfx_two_tex_scroll(0, a, 0, 64, 32, 1, b, 0, 8, 8)
}

/// The charge's `Gfx_TwoTexScroll(.., G_TX_RENDERTILE, (frames * 5) & 0xFF, 0, 32, 32, 1,
/// (frames * 20) & 0xFF, (frames * chargeTexScroll) & 0xFF, 8, 8)`.
fn charge_scroll(frames: u32, charge_tex_scroll: u32) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, frames.wrapping_mul(5) & 0xFF, 0, 32, 32, 1, frames.wrapping_mul(20) & 0xFF, frames.wrapping_mul(charge_tex_scroll) & 0xFF, 8, 8)
}

/// The draw's lists after `Gfx_SetupDL_25Xlu` (the bake's start; each list sets its own
/// render mode): the trail's two pairs (prim by the draw, their env their own) under segment
/// 8's scroll, and the charge (prim and env by the draw) under segment 9's.
pub fn bakes() -> Vec<MeshBake> {
    let bake = |name: &str, color: BakeSegment, seg: u8, scroll: Vec<(u32, u32)>, lists: &[&str]| MeshBake {
        name: name.into(),
        object: "gameplay_keep".into(),
        segments: vec![(SEG_COLOR, color), (seg, BakeSegment::Dynamic(scroll))],
        prelude: vec![SEG_COLOR],
        body: BakeBody::DLists(lists.iter().map(|l| ("gameplay_keep".to_string(), l.to_string())).collect()),
    };
    vec![
        bake(BAKE_WEAK, BakeSegment::DynamicColor { env: false, prim: true }, SEG_TRAIL, trail_scroll(0.0), &["gSpinAttack1DL", "gSpinAttack2DL"]),
        bake(BAKE_STRONG, BakeSegment::DynamicColor { env: false, prim: true }, SEG_TRAIL, trail_scroll(0.0), &["gSpinAttack3DL", "gSpinAttack4DL"]),
        bake(BAKE_CHARGE, BakeSegment::DynamicColor { env: true, prim: true }, SEG_CHARGE, charge_scroll(0, 20), &["gSpinAttackChargingDL"]),
    ]
}

fn cmd(bake: &str, m: Mat4, sv: SegmentValues) -> DrawCmd {
    DrawCmd { mesh: MeshKey::named(keys::bake(bake)), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } }
}

/// The charge's matrix: Player's `mf_9E0`, then by sword type along the blade, the pulse.
///
/// @bug (game), kept: `Matrix_RotateX(16384.0f, MTXMODE_APPLY)` takes radians.
pub fn charge_matrix(mf_9e0: Mat4, sword_type: u8, spin_charge_scale: f32) -> Mat4 {
    let mut m = MtxF::from_mat4(mf_9e0);
    match sword_type {
        M_THUNDER_SWORD_KOKIRI => {
            m.translate(0.0, 220.0, 0.0);
            m.scale(-0.7, -0.6, -0.4);
            m.rotate_x(16384.0);
        }
        M_THUNDER_SWORD_MASTER => {
            m.translate(0.0, 300.0, -100.0);
            m.scale(-1.2, -1.0, -0.7);
            m.rotate_x(16384.0);
        }
        M_THUNDER_SWORD_GIANT => {
            m.translate(200.0, 350.0, 0.0);
            m.scale(-1.8, -1.4, -0.7);
            m.rotate_x(16384.0);
        }
        _ => {}
    }
    m.scale(1.0, spin_charge_scale, spin_charge_scale);
    m.to_mat4()
}

impl ActorImpl for EnMThunder {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnMThunder_Update`: its action, the darkening, and its light (red and green
    /// `timer * 255`, blue `timer * 100`, radius `timer * 800`).
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::ChargingSpinAttack => self.charging_spin_attack(play),
            Action::EmptySpinAttack => self.empty_spin_attack(play),
            Action::SpinAttacking => self.spin_attacking(play),
        }
        Self::adjust_env_lights(play, self.dimming_intensity);
        let blue_radius = self.spin_attack_timer;
        let red_green = ((blue_radius * 255.0) as i64 as u32 & 0xFF) as u8;
        let p = self.actor.world_pos;
        self.light_info = LightInfo::point_no_glow(p.x as i32 as i16, p.y as i32 as i16, p.z as i32 as i16, [red_green, red_green, (blue_radius * 100.0) as i64 as u32 as u8], (blue_radius * 800.0) as i32 as i16);
        play.light_ctx.set_info(self.light_node, self.light_info);
    }

    /// `EnMThunder_Destroy`: the magic reset if it used any, the darkening off, the light gone.
    fn destroy(&mut self, play: &mut PlayState) {
        if self.is_using_magic {
            play.magic_reset();
        }
        Self::adjust_env_lights(play, 0.0);
        play.light_ctx.remove_light(self.light_node.take());
    }

    /// Player's `mf_9E0` as this frame drew it.
    fn draw_update(&mut self, play: &mut PlayState) {
        if let Some(p) = player(play) {
            self.player_mf = p.mf_9e0;
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.values = vec![self.spin_trail_tex_scroll, self.spin_attack_alpha, self.spin_charge_percent];
        rs.switches = vec![self.attack_strength as u32, self.sword_type as u32, self.charge_alpha as u32];
        rs
    }

    /// `EnMThunder_Draw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (&[scroll, alpha, charge], &[strength, sword_type, charge_alpha]) = (rs.values.as_slice(), rs.switches.as_slice()) else { return };
        let frames = play.gameplay_frames;
        // The trail: the actor's matrix, Matrix_Scale(0.02f, 0.02f, 0.02f, MTXMODE_APPLY).
        let m = oot_game::play::actor_draw_matrix(rs) * Mat4::from_scale(Vec3::splat(0.02));
        let mut sv = SegmentValues::default();
        sv.read(SEG_TRAIL, &trail_scroll(scroll));
        let a = (alpha * 255.0) as i32 as u8;
        match strength as u8 {
            M_THUNDER_ATTACK_STRONG => {
                sv.prim[SEG_COLOR as usize] = Some([255, 255, 170, a]);
                out.xlu.push(cmd(BAKE_STRONG, m, sv));
            }
            M_THUNDER_ATTACK_WEAK => {
                sv.prim[SEG_COLOR as usize] = Some([170, 255, 255, a]);
                out.xlu.push(cmd(BAKE_WEAK, m, sv));
            }
            _ => {}
        }
        // The charge.
        let (scale, prim, env, charge_tex_scroll) = if charge >= 0.85 {
            (S_SPIN_CHARGE_SCALE[(frames & 7) as usize] * 6.0 + 1.0, [255, 255, 170, charge_alpha as u8], [255, 100, 0, 128], 40)
        } else {
            (S_SPIN_CHARGE_SCALE[(frames & 7) as usize] * 2.0 + 1.0, [170, 255, 255, charge_alpha as u8], [0, 100, 255, 128], 20)
        };
        let m = charge_matrix(self.player_mf, sword_type as u8, scale);
        let mut sv = SegmentValues::default();
        sv.prim[SEG_COLOR as usize] = Some(prim);
        sv.env[SEG_COLOR as usize] = Some(env);
        sv.read(SEG_CHARGE, &charge_scroll(frames, charge_tex_scroll));
        out.xlu.push(cmd(BAKE_CHARGE, m, sv));
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

/// The handle of the first `En_M_Thunder` (tests).
pub fn find(play: &PlayState) -> Option<ActorHandle> {
    play.actors.find(ACTOR_EN_M_THUNDER, ACTORCAT_ITEMACTION)
}
