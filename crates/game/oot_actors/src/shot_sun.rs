//! `Shot_Sun` (`ovl_Shot_Sun/z_shot_sun.c`): two things in one overlay.
//! - Params 0x40 and 0x41 (low byte): a fairy's spot. Within 150 of it, Link's ocarina is asked
//!   for (`PLAYER_STATE2_23`); taken out (`PLAYER_STATE2_24`), the spot starts free play
//!   (`Message_StartOcarina(OCARINA_ACTION_FREE_PLAY)`), and when it ends (`OCARINA_MODE_04`) the
//!   Sun's Song (0x40) or the Song of Storms (0x41) wakes it: the attention camera on it, the
//!   sparkles (`DEMOKANKYO_SPARKLES`) and `NA_SE_EV_TRE_BOX_APPEAR`, and 50 frames on a big fairy
//!   (`FAIRY_HEAL_BIG`) at its home; the spot goes.
//! - Any other params: Lake Hylia's sun. Within 120 of it from 6:30 to 7:30, a cylinder on the
//!   line from Link's head to the sun takes the player's hits (an arrow shot at the sun): the
//!   fire arrows' chest and their cutscene (`gLakeHyliaFireArrowsCS`), or once they're owned a
//!   large magic jar that stays 6000 frames.
//!
//! The whole overlay is ported. `Item_Etcetera` (the fire arrows) isn't: a placeholder.
//!
//! Params: `params` bits 0 to 7 (0x40: a fairy for the Sun's Song; 0x41: a fairy for the Song
//! of Storms; any other, 1 in the data: Lake Hylia's sun); bits 8 to 15 unused (0xFF in the data).
//! Flags: none (the fire arrows owned is the inventory's).

use eng_collision::math3d::Cylinder16;
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_LOCK_ON_DISABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_UPDATE_DURING_OCARINA, Actor};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile, PLAYER_BODYPART_HEAD};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::audio::ocarina::{OCARINA_SONG_STORMS, OCARINA_SONG_SUNS};
use oot_game::audio::sfx::{NA_SE_EV_TRE_BOX_APPEAR, NA_SE_SY_CORRECT_CHIME, SfxPos};
use oot_game::collision_check::*;
use oot_game::env::clock_time;
use oot_game::message::{OCARINA_ACTION_FREE_PLAY, OCARINA_MODE_04};
use oot_game::play::PlayState;

use crate::player::{STATE2_23, STATE2_24};

/// `ACTOR_SHOT_SUN` (`actor_table.h`: 0x0183).
pub const ACTOR_SHOT_SUN: i16 = 0x0183;

/// `Shot_Sun_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_SHOT_SUN, name: "Shot_Sun", category: ACTORCAT_PROP, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY, object: "gameplay_keep", schema: &SCHEMA };

/// `PARAMS_GET_U(this->actor.params, 0, 8)`: what it is.
pub const TYPE: Field = Field::params(
    "params",
    0,
    8,
    Kind::Named(&[(0x01, "Lake Hylia's sun (any but 0x40, 0x41)"), (0x40, "a fairy's spot: the Sun's Song"), (0x41, "a fairy's spot: the Song of Storms")]),
    "a fairy's spot and its song, or Lake Hylia's sun",
);
/// Bits 8 to 15: 0xFF in the game's data, read by nothing.
pub const HIGH_BYTE: Field = Field::params("params bits 8 to 15", 8, 8, Kind::Unused, "0xFF in the data; read by nothing");

/// Shot_Sun's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[TYPE, HIGH_BYTE], flags: &[], todo: false, objects: &[] };

/// `ACTOR_ITEM_ETCETERA` (`actor_table.h`: 0x010F).
const ACTOR_ITEM_ETCETERA: i16 = 0x010F;

/// `FairySpawnerState`.
pub const SPAWNER_OUT_OF_RANGE: u8 = 0;
pub const SPAWNER_OCARINA_START: u8 = 1;
pub const SPAWNER_OCARINA_PLAYING: u8 = 2;

/// `sCylinderInit` (Lake Hylia's sun).
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: 0, damage: 0x00 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0020, hit_backlash: 0, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 30, height: 60, y_shift: 0, pos: [0; 3] },
};

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    UpdateFairySpawner,
    TriggerFairy,
    SpawnFairy,
    UpdateHyliaSun,
}

pub struct ShotSun {
    pub actor: Actor,
    pub collider: Option<ColliderCylinder>,
    pub hitbox_pos: [i16; 3],
    pub timer: i16,
    pub fairy_spawner_state: u8,
    pub action: Action,
}

impl ShotSun {
    /// `ShotSun_Init`.
    pub fn init(actor: Actor, _play: &mut PlayState) -> Box<dyn ActorImpl> {
        // schema: printed
        log::debug!("{} ---- Ocarina secret spawn!!!!!!!!!!!!!", actor.params);
        let mut this = ShotSun { actor, collider: None, hitbox_pos: [0; 3], timer: 0, fairy_spawner_state: SPAWNER_OUT_OF_RANGE, action: Action::UpdateHyliaSun };
        let params = TYPE.get(this.actor.params);
        if params == 0x40 || params == 0x41 {
            this.fairy_spawner_state = SPAWNER_OUT_OF_RANGE;
            this.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
            this.actor.flags |= ACTOR_FLAG_UPDATE_DURING_OCARINA;
            this.action = Action::UpdateFairySpawner;
            this.actor.flags |= ACTOR_FLAG_LOCK_ON_DISABLED;
        } else {
            this.collider = Some(ColliderCylinder::new(&CYLINDER_INIT));
            this.action = Action::UpdateHyliaSun;
            this.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        }
        Box::new(this)
    }

    /// `ShotSun_SpawnFairy`: after the timer, a big fairy at home (0x40 and 0x41 both
    /// `FAIRY_HEAL_BIG`: `@bug (game)` another params leaves `fairyType` unset, never reached).
    fn spawn_fairy(&mut self, play: &mut PlayState) {
        if self.timer > 0 {
            self.timer -= 1;
            return;
        }
        let fairy_type = crate::en_elf::FAIRY_HEAL_BIG;
        if let Err(e) = play.actor_spawn(crate::en_elf::ACTOR_EN_ELF, self.actor.home_pos, [0, 0, 0], fairy_type) {
            log::warn!("Shot_Sun: the fairy: {e:?}");
        }
        self.actor.kill();
    }

    /// `ShotSun_TriggerFairy`: once the attention camera is on it (`func_8005B198`, the camera's
    /// actor's category) or the timer runs, the sparkles and the chime; the fairy 50 frames on.
    fn trigger_fairy(&mut self, play: &mut PlayState) {
        if play.func_8005b198() == self.actor.category as i32 || self.timer != 0 {
            self.action = Action::SpawnFairy;
            self.timer = 50;
            if let Err(e) = play.actor_spawn(crate::demo_kankyo::ACTOR_DEMO_KANKYO, self.actor.home_pos, [0, 0, 0], crate::demo_kankyo::DEMOKANKYO_SPARKLES) {
                log::warn!("Shot_Sun: the sparkles: {e:?}");
            }
            if let Some(me) = play.cur_actor {
                play.audio.play_sfx_at_pos(SfxPos::Actor(me), NA_SE_EV_TRE_BOX_APPEAR);
            }
        }
    }

    /// `ShotSun_UpdateFairySpawner`.
    fn update_fairy_spawner(&mut self, play: &mut PlayState) {
        let Some((ppos, state2)) = play.player.and_then(|h| play.actors.get(h)).and_then(|p| Some((p.base().world_pos, p.as_player()?.state_flags2()))) else { return };
        let params = TYPE.get(self.actor.params);
        if self.actor.world_pos.distance_squared(ppos) > 150.0 * 150.0 {
            self.fairy_spawner_state = SPAWNER_OUT_OF_RANGE;
            return;
        }
        if self.fairy_spawner_state == SPAWNER_OUT_OF_RANGE {
            if state2 & STATE2_24 == 0 {
                set_player_state2(play, STATE2_23);
                return;
            }
            self.fairy_spawner_state = SPAWNER_OCARINA_START;
        }
        if self.fairy_spawner_state == SPAWNER_OCARINA_START {
            play.message_start_ocarina(OCARINA_ACTION_FREE_PLAY);
            self.fairy_spawner_state = SPAWNER_OCARINA_PLAYING;
        } else if self.fairy_spawner_state == SPAWNER_OCARINA_PLAYING && play.msg_ctx.ocarina_mode == OCARINA_MODE_04 {
            let song = play.msg_ctx.last_played_song;
            if (params == 0x40 && song == OCARINA_SONG_SUNS as u16) || (params == 0x41 && song == OCARINA_SONG_STORMS as u16) {
                self.action = Action::TriggerFairy;
                if let Some(me) = play.cur_actor.map(|h| play.cam_actor_of(h, &self.actor)) {
                    play.onepoint_attention(me);
                }
                self.timer = 0;
            } else {
                self.fairy_spawner_state = SPAWNER_OUT_OF_RANGE;
            }
            self.fairy_spawner_state = SPAWNER_OUT_OF_RANGE;
        }
    }

    /// `ShotSun_UpdateHyliaSun`.
    fn update_hylia_sun(&mut self, play: &mut PlayState) {
        let Some(collider) = self.collider.as_mut() else { return };
        if collider.base.ac_flags & AC_HIT != 0 {
            play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
            log::debug!("SHOT_SUN HIT!!!!!!!");
            // INV_CONTENT(ITEM_ARROW_FIRE): SLOT_ARROW_FIRE is 4.
            if play.save.inventory.items[4] == oot_game::item::ITEM_NONE {
                if let Err(e) = play.actor_spawn(ACTOR_ITEM_ETCETERA, Vec3::new(700.0, -800.0, 7261.0), [0, 0, 0], 7) {
                    log::warn!("Shot_Sun: Item_Etcetera: {e:?}");
                }
                play.cs_ctx.segment = play.cutscene_script("gLakeHyliaFireArrowsCS");
                play.save.cutscene_trigger = 1;
            } else if let Some(h) = crate::en_item00::item_drop_collectible(play, Vec3::new(700.0, -800.0, 7261.0), crate::en_item00::ITEM00_MAGIC_LARGE) {
                if let Some(c) = play.actors.downcast_mut::<crate::en_item00::EnItem00>(h) {
                    c.despawn_timer = 6000;
                    c.actor.speed_xz = 0.0;
                }
            }
            self.actor.kill();
            return;
        }
        let t = play.save.day_time;
        if !(self.actor.xz_dist_to_player > 120.0) && t >= clock_time(6, 30) as u16 && t < clock_time(7, 30) as u16 {
            let head = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map(|p| p.body_part(PLAYER_BODYPART_HEAD)).unwrap_or(Vec3::ZERO);
            let sun = play.env_ctx.sun_pos;
            let pos = [(head.x + sun.x * (1.0 / 6.0)) as i16, (head.y - 30.0 + sun.y * (1.0 / 6.0)) as i16, (head.z + sun.z * (1.0 / 6.0)) as i16];
            self.hitbox_pos = pos;
            let collider = self.collider.as_mut().unwrap();
            collider.set_position(pos);
            play.collision_check_set_ac(&self.actor, 0, collider);
        }
    }
}

/// `player->stateFlags2 |= flag`.
fn set_player_state2(play: &mut PlayState, flag: u32) {
    if let Some(p) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
        p.change_state_flags2(flag, 0);
    }
}

impl ActorImpl for ShotSun {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `ShotSun_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::UpdateFairySpawner => self.update_fairy_spawner(play),
            Action::TriggerFairy => self.trigger_fairy(play),
            Action::SpawnFairy => self.spawn_fairy(play),
            Action::UpdateHyliaSun => self.update_hylia_sun(play),
        }
    }
    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0).then_some(())?;
        self.collider.as_mut().map(ColliderMut::Cylinder)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
