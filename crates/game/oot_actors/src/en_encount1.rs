//! `En_Encount1` (`ovl_En_Encount1/z_en_encount1.c`): an enemy spawner (GAME-06 milestone 4).
//!
//! Params: bits 11..15 the kind (`SPAWNER_LEEVER`, `_TEKTITE`, `_STALCHILDREN`, `_WOLFOS`),
//! bits 6..10 how many at once, bits 0..5 how many in all; `world.rot.z` widens its range (120
//! plus 40 each).
//!
//! - **Stalchildren and Wolfos** (`EnEncount1_SpawnStalchildOrWolfos`): in Hyrule Field, by night
//!   (and without the Bunny Hood) and endlessly (10,000), around Link wherever he is: one 200
//!   ahead of him (±20), the next 100 behind; never while he's on dirt, on a bg actor, in the
//!   air or swimming (60 frames' wait after), nor over water deeper than his feet; two at a time
//!   once he's been waited on, and 100 frames between rounds. Elsewhere, within its range. Every
//!   tenth Stalchild after the tenth is bigger (`killCount / 10 * 5`).
//! - **Leevers** (`EnEncount1_SpawnLeevers`): around Link on sand, five places by his facing, a
//!   big one after ten kills.
//! - **Tektites** (`EnEncount1_SpawnTektites`): red ones within its range, every 10 frames.
//!
//! The whole overlay is ported. Its Leevers (`En_Reeba`), Tektites (`En_Tite`) and Wolfos
//! (`En_Wf`) aren't: they spawn as placeholders (a Leever's `aimType` and the big one aren't
//! kept). The debug display (`BREG(0)`) isn't drawn.

use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_LOCK_ON_DISABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, BGCHECKFLAG_GROUND};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorHandle, ActorImpl, ActorProfile};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::surface::SurfaceType;

/// `ACTOR_EN_ENCOUNT1` (`actor_table.h`: 0x00A7).
pub const ACTOR_EN_ENCOUNT1: i16 = 0x00A7;

/// `En_Encount1_Profile`: `ACTORCAT_PROP`, `ACTOR_FLAG_UPDATE_CULLING_DISABLED |
/// ACTOR_FLAG_LOCK_ON_DISABLED`, `OBJECT_GAMEPLAY_KEEP`, no draw.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_EN_ENCOUNT1, name: "En_Encount1", category: ACTORCAT_PROP, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_LOCK_ON_DISABLED, object: "gameplay_keep" };

/// `EnEncount1type`.
pub const SPAWNER_LEEVER: i16 = 0;
pub const SPAWNER_TEKTITE: i16 = 1;
pub const SPAWNER_STALCHILDREN: i16 = 2;
pub const SPAWNER_WOLFOS: i16 = 3;

/// `ACTOR_EN_REEBA`, `ACTOR_EN_TITE`, `ACTOR_EN_WF`, `ACTOR_EN_SKB` (`actor_table.h`).
const ACTOR_EN_REEBA: i16 = 0x001C;
const ACTOR_EN_TITE: i16 = 0x001B;
const ACTOR_EN_WF: i16 = 0x01AF;
const ACTOR_EN_SKB: i16 = crate::en_skb::ACTOR_EN_SKB;
/// `LEEVER_TYPE_SMALL`, `LEEVER_TYPE_BIG` (`z_en_reeba.h`); `TEKTITE_RED` (`z_en_tite.h`).
const LEEVER_TYPE_SMALL: i16 = 0;
const LEEVER_TYPE_BIG: i16 = 2;
const TEKTITE_RED: i16 = -1;
/// `SCENE_HYRULE_FIELD`, `SCENE_HAUNTED_WASTELAND` (`scene_table.h`).
const SCENE_HYRULE_FIELD: u16 = 0x51;
const SCENE_HAUNTED_WASTELAND: u16 = 0x5E;
/// `FLOOR_TYPE_4`, `_7`, `_12`: sand (`bgcheck.h`).
const LEEVER_FLOORS: [u32; 3] = [4, 7, 12];
/// `SURFACE_SFX_OFFSET_DIRT` (`bgcheck.h`: 0).
const SURFACE_SFX_OFFSET_DIRT: u16 = 0;
/// `PLAYER_STATE1_27`: swimming.
const PLAYER_STATE1_27: u32 = 1 << 27;

/// `sLeeverAngles`, `sLeeverDists`.
const LEEVER_ANGLES: [u16; 5] = [0x0000, 0x2710, 0x7148, 0x8EB8, 0xD8F0];
const LEEVER_DISTS: [f32; 5] = [200.0, 170.0, 120.0, 120.0, 170.0];

pub struct EnEncount1 {
    pub actor: Actor,
    pub max_cur_spawns: i16,
    pub cur_num_spawn: i16,
    pub spawn_type: i16,
    pub max_total_spawns: i16,
    pub total_num_spawn: i16,
    pub out_of_range_timer: i16,
    pub field_spawn_timer: i16,
    pub kill_count: i16,
    pub num_leever_spawns: i16,
    pub leever_index: i16,
    pub timer: i16,
    pub reduce_leevers: bool,
    pub spawn_range: f32,
    /// `bigLeever`.
    pub big_leever: Option<ActorHandle>,
}

impl EnEncount1 {
    /// `EnEncount1_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = EnEncount1 {
            actor: actor.clone(),
            max_cur_spawns: 0,
            cur_num_spawn: 0,
            spawn_type: 0,
            max_total_spawns: 0,
            total_num_spawn: 0,
            out_of_range_timer: 0,
            field_spawn_timer: 0,
            kill_count: 0,
            num_leever_spawns: 0,
            leever_index: 0,
            timer: 0,
            reduce_leevers: false,
            spawn_range: 0.0,
            big_leever: None,
        };
        if actor.params <= 0 {
            log::error!("En_Encount1: Input error death! (params {:#06x})", actor.params);
            actor.kill();
            this.actor = actor;
            return Box::new(this);
        }
        let p = actor.params as u16;
        this.spawn_type = ((p >> 11) & 0x1F) as i16;
        this.max_cur_spawns = ((p >> 6) & 0x1F) as i16;
        this.max_total_spawns = (p & 0x3F) as i16;
        this.spawn_range = 120.0 + (40.0 * actor.world_rot.z as f32);
        log::debug!("It's an enemy spawner! {:x}: type {}, at once {}, in all {}, range {}", actor.params, this.spawn_type, this.max_cur_spawns, this.max_total_spawns, this.spawn_range);
        this.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        match this.spawn_type {
            SPAWNER_LEEVER => {
                this.timer = 30;
                this.max_cur_spawns = 5;
                if play.scene_id == SCENE_HAUNTED_WASTELAND {
                    this.reduce_leevers = true;
                    this.max_cur_spawns = 3;
                }
            }
            SPAWNER_TEKTITE => this.max_cur_spawns = 2,
            SPAWNER_STALCHILDREN | SPAWNER_WOLFOS => {
                if play.scene_id == SCENE_HYRULE_FIELD {
                    this.max_total_spawns = 10000;
                }
            }
            _ => {}
        }
        Box::new(this)
    }

    /// What `GET_PLAYER(play)` gives these functions.
    fn player(play: &PlayState) -> Option<(Actor, f32)> {
        let h = play.player?;
        let a = play.actors.actor(h)?.clone();
        Some((a, 0.0))
    }

    /// `EnEncount1_SpawnLeevers`.
    fn spawn_leevers(&mut self, play: &mut PlayState) {
        let Some((player, _)) = Self::player(play) else { return };
        self.out_of_range_timer = 0;
        if !(self.timer == 0 && play.cs_ctx.state == oot_game::cutscene::CS_STATE_IDLE && self.cur_num_spawn <= self.max_cur_spawns && self.cur_num_spawn < 5) {
            return;
        }
        let floor_type = player.floor_poly.map(|p| play.col.floor_type(p)).unwrap_or(0);
        if !LEEVER_FLOORS.contains(&floor_type) {
            self.num_leever_spawns = 0;
            return;
        }
        if self.reduce_leevers && self.actor.xz_dist_to_player > 1300.0 {
            return;
        }
        let spawn_limit = if self.reduce_leevers { 3 } else { 5 };
        while self.cur_num_spawn < self.max_cur_spawns && self.cur_num_spawn < spawn_limit && self.timer == 0 {
            let mut dist = LEEVER_DISTS[self.leever_index as usize];
            let mut angle = (LEEVER_ANGLES[self.leever_index as usize] as i16).wrapping_add(player.shape_rot.y);
            let mut params = LEEVER_TYPE_SMALL;
            if self.kill_count >= 10 && self.big_leever.is_none() {
                self.kill_count = 0;
                self.num_leever_spawns = 0;
                angle = LEEVER_ANGLES[0] as i16;
                dist = LEEVER_DISTS[2];
                params = LEEVER_TYPE_BIG;
            }
            let mut pos = Vec3::new(player.world_pos.x + eng_math::sin_s(angle) * dist, player.floor_height + 120.0, player.world_pos.z + eng_math::cos_s(angle) * dist);
            let (floor_y, _) = play.col.entity_raycast_down(pos);
            if floor_y <= eng_collision::bgcheck::BGCHECK_Y_MIN {
                break;
            }
            pos.y = floor_y;
            match play.actor_spawn_as_child(&mut self.actor, ACTOR_EN_REEBA, pos, [0; 3], params) {
                Ok(h) => {
                    self.cur_num_spawn += 1;
                    // leever->aimType = leeverIndex: En_Reeba isn't ported.
                    self.leever_index += 1;
                    if self.leever_index >= 5 {
                        self.leever_index = 0;
                    }
                    self.num_leever_spawns += 1;
                    if self.num_leever_spawns >= 12 {
                        self.timer = 150;
                        self.num_leever_spawns = 0;
                    }
                    if params != LEEVER_TYPE_SMALL {
                        self.timer = 300;
                        self.big_leever = Some(h);
                    }
                    self.max_cur_spawns = if !self.reduce_leevers { play.rand.zero_float(3.99) as i16 + 2 } else { play.rand.zero_float(2.99) as i16 + 1 };
                }
                Err(e) => {
                    log::debug!("En_Encount1: Cannot spawn! ({e:?})");
                    break;
                }
            }
        }
    }

    /// `EnEncount1_SpawnTektites`: every 10 frames, Link within 100 up or down and the range: a red
    /// Tektite within 25 of it, on the floor.
    fn spawn_tektites(&mut self, play: &mut PlayState) {
        let Some((player, _)) = Self::player(play) else { return };
        if self.timer != 0 {
            return;
        }
        self.timer = 10;
        if (player.world_pos.y - self.actor.world_pos.y).abs() > 100.0 || self.actor.xz_dist_to_player > self.spawn_range {
            self.out_of_range_timer += 1;
            return;
        }
        self.out_of_range_timer = 0;
        if self.cur_num_spawn < self.max_cur_spawns && self.total_num_spawn < self.max_total_spawns {
            let x = self.actor.world_pos.x + play.rand.centered_float(50.0);
            let y = self.actor.world_pos.y + 120.0;
            let z = self.actor.world_pos.z + play.rand.centered_float(50.0);
            let (floor_y, _) = play.col.entity_raycast_down(Vec3::new(x, y, z));
            if floor_y <= eng_collision::bgcheck::BGCHECK_Y_MIN {
                return;
            }
            match play.actor_spawn_as_child(&mut self.actor, ACTOR_EN_TITE, Vec3::new(x, floor_y, z), [0; 3], TEKTITE_RED) {
                Ok(_) => {
                    self.cur_num_spawn += 1;
                    self.total_num_spawn += 1;
                }
                Err(e) => log::debug!("En_Encount1: Cannot spawn! ({e:?})"),
            }
        }
    }

    /// `EnEncount1_SpawnStalchildOrWolfos`.
    fn spawn_stalchild_or_wolfos(&mut self, play: &mut PlayState) {
        let Some((player, _)) = Self::player(play) else { return };
        if play.scene_id != SCENE_HYRULE_FIELD {
            if (player.world_pos.y - self.actor.world_pos.y).abs() > 100.0 || self.actor.xz_dist_to_player > self.spawn_range {
                self.out_of_range_timer += 1;
                return;
            }
        } else if play.save.is_day() {
            // (Player_GetMask == PLAYER_MASK_BUNNY: the masks aren't ported.)
            self.kill_count = 0;
            return;
        }
        self.out_of_range_timer = 0;
        let mut pos = self.actor.world_pos;
        while self.cur_num_spawn < self.max_cur_spawns && self.total_num_spawn < self.max_total_spawns {
            if play.scene_id == SCENE_HYRULE_FIELD {
                let (state1, floor_sfx_offset) =
                    play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map(|p| (p.state_flags1(), p.unk_89e())).unwrap_or((0, SURFACE_SFX_OFFSET_DIRT));
                if floor_sfx_offset == SURFACE_SFX_OFFSET_DIRT
                    || player.floor_bg_id != eng_collision::bgcheck::BGCHECK_SCENE
                    || player.bg_check_flags & BGCHECKFLAG_GROUND == 0
                    || state1 & PLAYER_STATE1_27 != 0
                {
                    self.field_spawn_timer = 60;
                    break;
                }
                if self.field_spawn_timer == 60 {
                    self.max_cur_spawns = 2;
                }
                if self.field_spawn_timer != 0 {
                    self.field_spawn_timer -= 1;
                    break;
                }
                let mut dist = play.rand.centered_float(40.0) + 200.0;
                let mut angle = player.shape_rot.y;
                if self.cur_num_spawn != 0 {
                    angle = angle.wrapping_neg();
                    dist = play.rand.centered_float(40.0) + 100.0;
                }
                pos.x = player.world_pos.x + (eng_math::sin_s(angle) * dist) + play.rand.centered_float(40.0);
                pos.y = player.floor_height + 120.0;
                pos.z = player.world_pos.z + (eng_math::cos_s(angle) * dist) + play.rand.centered_float(40.0);
                let (floor_y, _) = play.col.entity_raycast_down(pos);
                if floor_y <= eng_collision::bgcheck::BGCHECK_Y_MIN {
                    break;
                }
                // depthInWater: Link's `yDistToWater` (BGCHECK_Y_MIN out of water).
                if player.y_dist_to_water != eng_collision::bgcheck::BGCHECK_Y_MIN && floor_y < player.world_pos.y - player.y_dist_to_water {
                    break;
                }
                pos.y = floor_y;
            }
            let (id, params) = if self.spawn_type == SPAWNER_WOLFOS {
                (ACTOR_EN_WF, (0xFF << 8) | 0x00)
            } else {
                let mut params = 0;
                let kc_over_10 = self.kill_count / 10;
                if kc_over_10 > 0 && self.kill_count % 10 == 0 {
                    params = kc_over_10 * 5;
                }
                self.kill_count += 1;
                (ACTOR_EN_SKB, params)
            };
            match play.actor_spawn_as_child(&mut self.actor, id, pos, [0; 3], params) {
                Ok(_) => {
                    self.cur_num_spawn += 1;
                    if self.cur_num_spawn >= self.max_cur_spawns {
                        self.field_spawn_timer = 100;
                    }
                    if play.scene_id != SCENE_HYRULE_FIELD {
                        self.total_num_spawn += 1;
                    }
                }
                Err(e) => {
                    log::debug!("En_Encount1: Cannot spawn! ({e:?})");
                    break;
                }
            }
        }
    }
}

impl ActorImpl for EnEncount1 {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnEncount1_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if self.timer != 0 {
            self.timer -= 1;
        }
        match self.spawn_type {
            SPAWNER_LEEVER => self.spawn_leevers(play),
            SPAWNER_TEKTITE => self.spawn_tektites(play),
            SPAWNER_STALCHILDREN | SPAWNER_WOLFOS => self.spawn_stalchild_or_wolfos(play),
            _ => {}
        }
    }

    /// No draw.
    fn draw(&self, _rs: &RenderState, _play: &PlayState, _view: &ViewInfo, _out: &mut DrawOut) {}

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
