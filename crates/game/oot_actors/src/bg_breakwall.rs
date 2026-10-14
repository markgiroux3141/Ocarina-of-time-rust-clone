//! `Bg_Breakwall` (`ovl_Bg_Breakwall/z_bg_breakwall.c`): a bombable wall. A DynaPoly actor of
//! its object's collision that blocks the way until an explosion (or a hammer's swing: its quad
//! takes `0x48`, `DMG_EXPLOSIVE | DMG_HAMMER_SWING`) hits it: then its collision goes, sixteen
//! fragments fly (`En_A_Obj`'s boulder fragments, params `0x000B`) over puffs of dust
//! (`func_80033480`), its switch flag is set, it sounds (`NA_SE_EV_WALL_BROKEN`, King Dodongo's
//! floor `NA_SE_EV_EXPLOSION`), and it's gone; a set flag keeps it gone. Its four types
//! (`sBombableWallInfo`):
//!
//! - 0, Dodongo's Cavern's entrance: `object_bwall`'s wall; breaking it the first time plays the
//!   cavern's opening (`gDcOpeningCs`, `EVENTCHKINF_B0`, the chime, Player in cutscene mode 49);
//! - 1, a wall (`object_bwall`'s);
//! - 2, King Dodongo's floor (`object_kingdodongo`'s, turned flat: `world.rot.x` 0x4000, its quad
//!   the 2 by 2 square on its floor);
//! - 3, the lava cover in King Dodongo's room (`object_kingdodongo`'s collision, nothing drawn, no
//!   quad): spawned 40 down, it rises 1 a frame to its home (`KREG(80)` 0).
//!
//! It waits for its object (`BgBreakwall_WaitForObject`) before its collision and draw. The quad
//! is placed and registered for hits in its draw (`BgBreakwall_Draw`: the port's `draw_update`,
//! once per game frame), through the actor's model matrix: the wall's 160 wide and 160 tall, 10
//! in front of its face (`sColQuadList`, at scale 0.1). Params with bit 15 set also play the
//! secret's chime (`NA_SE_SY_TRE_BOX_APPEAR`).
//!
//! In the child's game: the shield grave (`hakaana2_scene` room 0) has one, params 0xA001 at
//! (-28, 20, 386): a wall (type 1), flag 1, with the chime; it hides the fairy fountain. Link has
//! no bombs yet (`En_Bom` isn't ported): it stands. The whole overlay is ported.
//!
//! Params: `switchFlag` bits 0 to 5 (set when it breaks; set at init, the wall's gone; the lava
//! cover doesn't read it); bits 6 to 12 unused; `wallType` bits 13 to 14 (`BombableWallType`: 0
//! the cavern's entrance, 1 a wall, 2 King Dodongo's floor, 3 the lava cover, which
//! `Boss_Dodongo` spawns with 0x6000); bit 15 the chime when it breaks (`params < 0`).
//! Flags: reads and sets switch flag `switchFlag`; type 0 reads and sets `EVENTCHKINF_B0` (0xB0,
//! the cavern's opening seen).

use std::sync::Arc;

use eng_collision::collision::CollisionHeader;
use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::step_to_f;
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile, audio_play_actor_sfx2, func_80033480};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::sfx::{NA_SE_EV_WALL_BROKEN, NA_SE_SY_CORRECT_CHIME, NA_SE_SY_TRE_BOX_APPEAR};
use oot_game::collision_check::*;
use oot_game::event_chk_inf;
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::sys_matrix::MtxF;

use crate::en_a_obj::{A_OBJ_BOULDER_FRAGMENT, ACTOR_EN_A_OBJ};

/// `ACTOR_BG_BREAKWALL` (`actor_table.h`: 0x0059).
pub const ACTOR_BG_BREAKWALL: i16 = 0x0059;
/// `OBJECT_BWALL` (`object_table.h`: 0x018A), `OBJECT_KINGDODONGO` (0x0019).
pub const OBJECT_BWALL: i16 = 0x018A;
pub const OBJECT_KINGDODONGO: i16 = 0x0019;
pub const BWALL_OBJECT: &str = "object_bwall";
pub const KINGDODONGO_OBJECT: &str = "object_kingdodongo";

/// `NA_SE_EV_EXPLOSION` (`environmentbank_table.h`: 0x2802).
pub const NA_SE_EV_EXPLOSION: u16 = 0x2802;
/// `EVENTCHKINF_B0` (`save.h`: 0xB0): Dodongo's Cavern's opening seen.
pub const EVENTCHKINF_B0: u16 = 0xB0;
/// `PLAYER_CSACTION_49` (`player.h`: 0x31).
pub const PLAYER_CSACTION_49: u8 = 0x31;

/// `BombableWallType` (`z_bg_breakwall.h`).
pub const BWALL_DC_ENTRANCE: i32 = 0;
pub const BWALL_WALL: i32 = 1;
pub const BWALL_KD_FLOOR: i32 = 2;
pub const BWALL_KD_LAVA_COVER: i32 = 3;

/// `Bg_Breakwall_Profile`: `ACTORCAT_BG`, `ACTOR_FLAG_UPDATE_CULLING_DISABLED`, `gameplay_keep`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_BREAKWALL, name: "Bg_Breakwall", category: ACTORCAT_BG, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: "gameplay_keep", schema: &SCHEMA };

/// `PARAMS_GET_U(params, 0, 6)`: the switch flag.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 0, 6, Kind::SwitchFlag, "set when it breaks; set at init, it's gone (the lava cover doesn't read it)");
/// `(u8)PARAMS_GET_U(params, 13, 2)`: `wallType`.
pub const WALL_TYPE: Field = Field::params(
    "wallType",
    13,
    2,
    Kind::Named(&[(0, "BWALL_DC_ENTRANCE"), (1, "BWALL_WALL"), (2, "BWALL_KD_FLOOR"), (3, "BWALL_KD_LAVA_COVER")]),
    "which wall: its object, display list and collision (`sBombableWallInfo`)",
);
/// `this->dyna.actor.params < 0`: bit 15.
pub const CHIME: Field = Field::params("params < 0", 15, 1, Kind::Named(&[(0, "no chime"), (1, "the secret's chime")]), "the secret's chime as it breaks");

/// Bg_Breakwall's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[SWITCH_FLAG, WALL_TYPE, CHIME],
    flags: &[
        FlagUse::new(Flag::Switch("switchFlag")).reads().sets().note("set as it breaks; set at init, it's gone"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_B0)).reads().sets().note("type 0: Dodongo's Cavern's opening seen"),
    ],
    todo: false,
};

/// `BombableWallInfo`: the object (file), `colHeader`, `dList` and `colType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BombableWallInfo {
    pub object: &'static str,
    pub col_header: &'static str,
    pub dlist: Option<&'static str>,
    pub col_type: i8,
}

/// `sBombableWallInfo`, by type.
pub const BOMBABLE_WALL_INFO: [BombableWallInfo; 4] = [
    BombableWallInfo { object: BWALL_OBJECT, col_header: "object_bwall_Col_000118", dlist: Some("object_bwall_DL_000040"), col_type: 0 },
    BombableWallInfo { object: BWALL_OBJECT, col_header: "object_bwall_Col_000118", dlist: Some("object_bwall_DL_000040"), col_type: 0 },
    BombableWallInfo { object: KINGDODONGO_OBJECT, col_header: "object_kingdodongo_Col_0264A8", dlist: Some("object_kingdodongo_DL_025BD0"), col_type: 1 },
    BombableWallInfo { object: KINGDODONGO_OBJECT, col_header: "object_kingdodongo_Col_025B64", dlist: None, col_type: -1 },
];

/// `sQuadInit`: no material, hit (AC) by Player's and others' `0x48` (explosives, the hammer's
/// swing), no pushing.
pub const QUAD_INIT: ColliderQuadInit = ColliderQuadInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER | AC_TYPE_OTHER, oc_flags1: OC1_NONE, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_QUAD },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0048, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0048, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_NONE,
    },
    quad: [Vec3::ZERO; 4],
};

/// `sColQuadList`: the quads in model space, by `colType`: the wall's face (1600 by 1600, 100 in
/// front) and the floor's square (20 by 20).
pub const COL_QUAD_LIST: [[Vec3; 4]; 2] = [
    [Vec3::new(800.0, 1600.0, 100.0), Vec3::new(-800.0, 1600.0, 100.0), Vec3::new(800.0, 0.0, 100.0), Vec3::new(-800.0, 0.0, 100.0)],
    [Vec3::new(10.0, 0.0, 10.0), Vec3::new(-10.0, 0.0, 10.0), Vec3::new(10.0, 0.0, -10.0), Vec3::new(-10.0, 0.0, -10.0)],
];

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Before the init's `BgBreakwall_SetupAction` (killed at init).
    None,
    WaitForObject,
    Wait,
    LavaCoverMove,
}

pub struct BgBreakwall {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    /// `bombableWallDList`, `colType`, `requiredObjectSlot`.
    pub bombable_wall_dlist: Option<&'static str>,
    pub col_type: i8,
    pub required_object_slot: Option<usize>,
    pub collider: ColliderQuad,
    pub action: Action,
    /// `actor.draw` set (`BgBreakwall_Draw`, once the object is loaded).
    pub drawn: bool,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

impl BgBreakwall {
    /// `(u8)PARAMS_GET_U(params, 13, 2)`: the wall's type.
    pub fn wall_type(&self) -> i32 {
        WALL_TYPE.get(self.actor.params) as i32
    }

    /// `PARAMS_GET_U(params, 0, 6)`: the switch flag.
    pub fn switch_flag(&self) -> i32 {
        SWITCH_FLAG.get(self.actor.params) as i32
    }

    /// `BgBreakwall_Init`: scale 0.1 and the culling volume (`sInitChain`); its type's list and
    /// `colType` (the floor turned flat). A drawn type is gone if its flag is set, else gets its
    /// quad; the lava cover starts 40 down. Then its object's slot (King Dodongo's for types 2 and
    /// 3), gone without one.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = BgBreakwall { actor, bg: BG_ACTOR_MAX, bombable_wall_dlist: None, col_type: -1, required_object_slot: None, collider: ColliderQuad::default(), action: Action::None, drawn: false };
        let wall_type = this.wall_type();
        // sInitChain: ICHAIN_VEC3F_DIV1000(scale, 100), cullingVolumeDistance 4000,
        // cullingVolumeScale 400, cullingVolumeDownward 400.
        this.actor.scale = Vec3::splat(0.1);
        this.actor.culling_volume_distance = 4000.0;
        this.actor.culling_volume_scale = 400.0;
        this.actor.culling_volume_downward = 400.0;
        // DynaPolyActor_Init(&this->dyna, 0): the bg actor is set once the object is loaded.
        let info = BOMBABLE_WALL_INFO[wall_type as usize];
        this.bombable_wall_dlist = info.dlist;
        this.col_type = info.col_type;
        if this.col_type == 1 {
            this.actor.world_rot.x = 0x4000;
        }
        if this.bombable_wall_dlist.is_some() {
            if play.flags.get_switch(this.switch_flag()) {
                this.actor.kill();
                return Box::new(this);
            }
            // ActorShape_Init(&shape, 0.0f, NULL, 0.0f).
            this.actor.shape_y_offset = 0.0;
            // Collider_InitQuad, Collider_SetQuad.
            this.collider = ColliderQuad::new(&QUAD_INIT);
        } else {
            this.actor.world_pos.y -= 40.0;
        }
        this.required_object_slot = play.object_ctx.get_index(if wall_type >= BWALL_KD_FLOOR { OBJECT_KINGDODONGO } else { OBJECT_BWALL });
        if this.required_object_slot.is_none() {
            this.actor.kill();
        } else {
            this.action = Action::WaitForObject;
        }
        Box::new(this)
    }

    fn load_collision(play: &PlayState, info: &BombableWallInfo) -> Option<Arc<CollisionHeader>> {
        crate::obj_kibako2::load_collision(play, info.object, info.col_header)
    }

    /// `BgBreakwall_SpawnFragments`: `count` rings of sixteen `En_A_Obj` fragments (params
    /// `0x000B`), each ring turned a further 0x4000: `actorPosList`'s four rows of four points
    /// through the wall's matrix (its position and `world.rot`, then `pos`), scattered 10 each
    /// way, turned by `actorRotList` and `|world.rot.y|`; on rows 0 and 2 a puff of dust at the
    /// point (`velocity × 200` across, 2 puffs, 650 big growing 150). Each fragment's speed and
    /// `velocity.y` `Rand + accel × 0.6`, its yaw scattered ±1500, its spin (rot x and z) 2000 to
    /// 5500, its parent the wall, its scale `actorScaleList`'s (0.004 by `scaleX`/`scaleY`) ±0.0005.
    /// Returns the last spawned (`NULL` if that spawn failed).
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_fragments(&self, play: &mut PlayState, pos: Vec3, velocity: f32, scale_y: f32, scale_x: f32, count: i32, accel: f32) -> Option<oot_game::actor_ctx::ActorHandle> {
        let mut actor = None;
        let mut angle2: i16 = 0;
        let actor_rot_list: [[i16; 3]; 4] = [[0, 0, 0], [0, 0, 0x4000], [0, 0, -0x4000], [0, 0, 0]];
        let mut actor_scale_list = [Vec3::splat(0.004); 4];
        let actor_pos_list: [[Vec3; 4]; 4] = [
            [Vec3::new(40.0, 15.0, 0.0), Vec3::new(30.0, 57.0, 0.0), Vec3::new(50.0, 57.0, 0.0), Vec3::new(40.0, 70.0, 0.0)],
            [Vec3::new(55.0, -15.0, 0.0), Vec3::new(30.0, -32.0, 0.0), Vec3::new(50.0, -32.0, 0.0), Vec3::new(20.0, -10.0, 0.0)],
            [Vec3::new(-40.0, 14.0, 0.0), Vec3::new(-50.0, 57.0, 0.0), Vec3::new(-30.0, 57.0, 0.0), Vec3::new(-40.0, 70.0, 0.0)],
            [Vec3::new(-55.0, -15.0, 0.0), Vec3::new(-55.0, -32.0, 0.0), Vec3::new(-30.0, -32.0, 0.0), Vec3::new(-20.0, -10.0, 0.0)],
        ];
        for k in (0..4).rev() {
            let s = &mut actor_scale_list[k];
            if k == 0 || k == 3 {
                s.x *= scale_x;
                s.y *= scale_y;
                s.z *= scale_y;
            } else {
                s.x *= scale_y;
                s.y *= scale_x;
                s.z *= scale_x;
            }
        }
        let a = &self.actor;
        let mut i = 0;
        while i < count {
            // ABS(world.rot.y) + angle2, as an s16.
            let angle1 = (a.world_rot.y as i32).abs().wrapping_add(angle2 as i32) as i16;
            let mut m = MtxF::set_translate(a.world_pos.x, a.world_pos.y, a.world_pos.z);
            m.rotate_zyx(a.world_rot.x, a.world_rot.y, a.world_rot.z);
            m.translate(pos.x, pos.y, pos.z);
            for j in (0..4).rev() {
                for k in (0..4).rev() {
                    let actor_pos = m.mult_vec3f(actor_pos_list[j][k]);
                    // The spawn's arguments' Rand_CenteredFloat calls, left to right (IDO).
                    let x = play.rand.centered_float(20.0) + actor_pos.x;
                    let y = play.rand.centered_float(20.0) + actor_pos.y;
                    let z = play.rand.centered_float(20.0) + actor_pos.z;
                    let rot = [actor_rot_list[k][0], actor_rot_list[k][1].wrapping_add(angle1), actor_rot_list[k][2]];
                    actor = play.actor_spawn(ACTOR_EN_A_OBJ, Vec3::new(x, y, z), rot, A_OBJ_BOULDER_FRAGMENT).ok();
                    if j & 1 == 0 {
                        func_80033480(play, actor_pos, velocity * 200.0, 1, 650, 150, 1);
                    }
                    if let Some(h) = actor {
                        let speed = play.rand.zero_one() + accel * 0.6;
                        let velocity_y = play.rand.zero_one() + accel * 0.6;
                        let d_yaw = ((play.rand.zero_one() - 0.5) * 3000.0) as i16;
                        let rot_x = ((play.rand.zero_one() * 3500.0) as i16).wrapping_add(2000);
                        let rot_z = ((play.rand.zero_one() * 3500.0) as i16).wrapping_add(2000);
                        let sx = actor_scale_list[k].x + play.rand.centered_float(0.001);
                        let sy = actor_scale_list[k].y + play.rand.centered_float(0.001);
                        let sz = actor_scale_list[k].z + play.rand.centered_float(0.001);
                        let parent = play.cur_actor;
                        if let Some(f) = play.actors.actor_mut(h) {
                            f.speed_xz = speed;
                            f.velocity.y = velocity_y;
                            f.world_rot.y = f.world_rot.y.wrapping_add(d_yaw);
                            f.world_rot.x = rot_x;
                            f.world_rot.z = rot_z;
                            f.parent = parent;
                            f.scale = Vec3::new(sx, sy, sz);
                        }
                    }
                }
            }
            angle2 = angle2.wrapping_add(0x4000);
            i += 1;
        }
        actor
    }

    /// `BgBreakwall_WaitForObject`: once the object is loaded, the actor's dependency on it, culled
    /// from now on, drawn, its type's collision (`DynaPoly_SetBgActor`); then
    /// `BgBreakwall_LavaCoverMove` for the lava cover, else `BgBreakwall_Wait`.
    fn wait_for_object(&mut self, play: &mut PlayState) {
        let Some(slot) = self.required_object_slot else { return };
        if !play.object_ctx.is_loaded(slot) {
            return;
        }
        let wall_type = self.wall_type();
        // this->dyna.actor.objectSlot = requiredObjectSlot; Actor_SetObjectDependency.
        self.actor.obj_bank_index = Some(slot);
        self.actor.flags &= !ACTOR_FLAG_UPDATE_CULLING_DISABLED;
        self.drawn = true;
        let info = BOMBABLE_WALL_INFO[wall_type as usize];
        if let Some(h) = Self::load_collision(play, &info) {
            self.bg = play.col.dyna.set_bg_actor(h, source(&self.actor), 0);
        }
        self.action = if wall_type == BWALL_KD_LAVA_COVER { Action::LavaCoverMove } else { Action::Wait };
    }

    /// `BgBreakwall_Wait`: hit (`AC_HIT`): the collision gone, one ring of fragments from 55 up
    /// its middle (a floor: 25 in front and 10 down), its flag set, the sound (the floor's
    /// explosion, else the wall broken); the cavern's entrance the first time plays the opening;
    /// bit 15's chime; gone.
    fn wait(&mut self, play: &mut PlayState) {
        if self.collider.base.ac_flags & AC_HIT == 0 {
            return;
        }
        let wall_type = self.wall_type();
        play.col.dyna.delete_bg_actor(self.bg);
        let mut effect_pos = Vec3::ZERO;
        if self.actor.world_rot.x == 0 {
            effect_pos.y = 55.0;
        } else {
            effect_pos.z = 25.0;
            effect_pos.y = -10.0;
        }
        self.spawn_fragments(play, effect_pos, 0.0, 6.4, 5.0, 1, 2.0);
        play.flags.set_switch(self.switch_flag());
        if wall_type == BWALL_KD_FLOOR {
            audio_play_actor_sfx2(play, NA_SE_EV_EXPLOSION);
        } else {
            audio_play_actor_sfx2(play, NA_SE_EV_WALL_BROKEN);
        }
        if wall_type == BWALL_DC_ENTRANCE && !play.save.get_event_chk_inf(EVENTCHKINF_B0) {
            play.save.set_event_chk_inf(EVENTCHKINF_B0);
            // Cutscene_SetScript(play, gDcOpeningCs).
            play.cs_ctx.segment = play.cutscene_script("gDcOpeningCs");
            play.save.cutscene_trigger = 1;
            play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
            play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_49);
        }
        if CHIME.get(self.actor.params) != 0 {
            play.audio.play_sfx_centered(NA_SE_SY_TRE_BOX_APPEAR);
        }
        self.actor.kill();
    }

    /// `BgBreakwall_LavaCoverMove`: up 1 a frame to its home (`KREG(80)` 0).
    fn lava_cover_move(&mut self) {
        let target = self.actor.home_pos.y;
        step_to_f(&mut self.actor.world_pos.y, target, 1.0);
    }

    /// The draw's model matrix (`Actor_Draw`: `Matrix_SetTranslateRotateYXZ` at the position,
    /// `shape.yOffset × scale.y` up, by `shape.rot`; `Matrix_Scale`).
    fn model_matrix(&self) -> MtxF {
        let a = &self.actor;
        let r = a.shape_rot;
        let mut m = MtxF::set_translate_rotate_yxz(a.world_pos.x, a.world_pos.y + a.shape_y_offset * a.scale.y, a.world_pos.z, [r.x, r.y, r.z]);
        m.scale(a.scale.x, a.scale.y, a.scale.z);
        m
    }

    /// `BgBreakwall_Draw`'s quad: `sColQuadList[colType]` through the model matrix
    /// (`Collider_SetQuadVertices`), registered for hits (`CollisionCheck_SetAC`).
    pub fn quad(&self) -> Option<[Vec3; 4]> {
        if self.bombable_wall_dlist.is_none() || self.col_type < 0 {
            return None;
        }
        let m = self.model_matrix();
        Some(COL_QUAD_LIST[self.col_type as usize].map(|v| m.mult_vec3f(v)))
    }
}

impl ActorImpl for BgBreakwall {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BgBreakwall_Update`: the action; the position for `DynaPoly_UpdateContext`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::None => {}
            Action::WaitForObject => self.wait_for_object(play),
            Action::Wait => self.wait(play),
            Action::LavaCoverMove => self.lava_cover_move(),
        }
        if self.bg != BG_ACTOR_MAX && !self.actor.killed {
            play.col.dyna.set_source(self.bg, source(&self.actor));
        }
    }

    /// `BgBreakwall_Destroy`: `DynaPoly_DeleteBgActor` (again, after a break: no effect).
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }

    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }

    /// `BgBreakwall_Draw`'s effect on the game: the quad placed and registered for hits.
    fn draw_update(&mut self, play: &mut PlayState) {
        if !self.drawn || self.actor.killed {
            return;
        }
        if let Some([a, b, c, d]) = self.quad() {
            self.collider.set_vertices(a, b, c, d);
            play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
        }
    }

    /// `BgBreakwall_Draw`: its list opaque (`Gfx_SetupDL_25Opa`); the lava cover draws nothing.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if !self.drawn {
            return;
        }
        let Some(dl) = self.bombable_wall_dlist else { return };
        let object = BOMBABLE_WALL_INFO[self.wall_type() as usize].object;
        out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh(object, dl)), actor_draw_matrix(rs)));
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0).then_some(ColliderMut::Quad(&mut self.collider))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
