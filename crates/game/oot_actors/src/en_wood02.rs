//! `En_Wood02` (`ovl_En_Wood02/z_en_wood02.c`): the trees and bushes of the fields, and their
//! falling leaves (GAME-06 milestone 4).
//!
//! Params: the low byte is the type (`WoodType`: trees 0 to 10, bushes 11 to 22, leaves 23 and
//! 24), the high byte the drop table (`unk_14C`; 0x80 and up none). A tree with `home.rot.z` set
//! holds a Gold Skulltula (`home.rot.z << 8 | drop`), which it drops when bumped.
//!
//! - **Spawners** (types 3, 6, 8, 13, 15, 19, 21): a group of five more of their kind around
//!   their home (`sSpawnDistance`, `sSpawnAngle`), each spawned as their child once its place
//!   would be in the spawner's view (`EnWood02_SpawnZoneCheck`, `EnWood02_SpawnOffspring`); the
//!   spawner itself stands at the sixth place. A spawned child leaves when it isn't in view
//!   (`ACTOR_FLAG_INSIDE_CULLING_VOLUME` clear): its slot is free again.
//! - **Trees**: hard (the sword bounces, `NA_SE_IT_REFLECTION_WOOD`); bumped by Link's roll
//!   (Player sets `home.rot.y`), a tree drops from its table (or its Gold Skulltula), sways and
//!   sheds four leaves.
//! - **Bushes**: walked through, they drop from their table once and sway.
//! - **Leaves**: drift down, swaying, for 75 frames.
//!
//! The whole overlay is ported. The culling isn't (`Actor_CullingVolumeTest`: every actor is in
//! view), so a spawned tree, once in, stays.

use eng_collision::bgcheck::BGCHECK_Y_MIN;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{approach_f, cos_s, sin_s};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_INSIDE_CULLING_VOLUME, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::audio::sfx::{NA_SE_EV_TREE_SWING, NA_SE_IT_REFLECTION_WOOD};
use oot_game::collision_check::*;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};

/// `ACTOR_EN_WOOD02` (`actor_table.h`: 0x0077).
pub const ACTOR_EN_WOOD02: i16 = 0x0077;
pub const OBJECT: &str = "object_wood02";

/// `En_Wood02_Profile`: `ACTORCAT_PROP`, no flags.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_WOOD02, name: "En_Wood02", category: ACTORCAT_PROP, flags: 0, object: OBJECT };

/// `WoodType`.
pub const WOOD_TREE_CONICAL_LARGE: i16 = 0x00;
pub const WOOD_TREE_CONICAL_MEDIUM: i16 = 0x01;
pub const WOOD_TREE_CONICAL_SMALL: i16 = 0x02;
pub const WOOD_TREE_CONICAL_SPAWNER: i16 = 0x03;
pub const WOOD_TREE_CONICAL_SPAWNED: i16 = 0x04;
pub const WOOD_TREE_OVAL_GREEN: i16 = 0x05;
pub const WOOD_TREE_OVAL_YELLOW_SPAWNER: i16 = 0x06;
pub const WOOD_TREE_OVAL_YELLOW_SPAWNED: i16 = 0x07;
pub const WOOD_TREE_OVAL_GREEN_SPAWNER: i16 = 0x08;
pub const WOOD_TREE_OVAL_GREEN_SPAWNED: i16 = 0x09;
pub const WOOD_TREE_KAKARIKO_ADULT: i16 = 0x0A;
pub const WOOD_BUSH_GREEN_SMALL: i16 = 0x0B;
pub const WOOD_BUSH_GREEN_LARGE: i16 = 0x0C;
pub const WOOD_BUSH_GREEN_SMALL_SPAWNER: i16 = 0x0D;
pub const WOOD_BUSH_GREEN_SMALL_SPAWNED: i16 = 0x0E;
pub const WOOD_BUSH_GREEN_LARGE_SPAWNER: i16 = 0x0F;
pub const WOOD_BUSH_GREEN_LARGE_SPAWNED: i16 = 0x10;
pub const WOOD_BUSH_BLACK_SMALL: i16 = 0x11;
pub const WOOD_BUSH_BLACK_LARGE: i16 = 0x12;
pub const WOOD_BUSH_BLACK_SMALL_SPAWNER: i16 = 0x13;
pub const WOOD_BUSH_BLACK_SMALL_SPAWNED: i16 = 0x14;
pub const WOOD_BUSH_BLACK_LARGE_SPAWNER: i16 = 0x15;
pub const WOOD_BUSH_BLACK_LARGE_SPAWNED: i16 = 0x16;
pub const WOOD_LEAF_GREEN: i16 = 0x17;
pub const WOOD_LEAF_YELLOW: i16 = 0x18;

/// `WoodSpawnType`.
pub const WOOD_SPAWN_NORMAL: u8 = 0;
pub const WOOD_SPAWN_SPAWNED: u8 = 1;
pub const WOOD_SPAWN_SPAWNER: u8 = 2;

/// `WoodDrawType`.
pub const WOOD_DRAW_TREE_CONICAL: u8 = 0;
pub const WOOD_DRAW_TREE_OVAL: u8 = 1;
pub const WOOD_DRAW_TREE_KAKARIKO_ADULT: u8 = 2;
pub const WOOD_DRAW_BUSH_GREEN: u8 = 3;
pub const WOOD_DRAW_4: u8 = 4;
pub const WOOD_DRAW_LEAF_YELLOW: u8 = 5;

/// `ACTOR_EN_SW` (the Gold Skulltula a tree holds).
const ACTOR_EN_SW: i16 = crate::en_sw::ACTOR_EN_SW;

/// `sCylinderInit`: a tree's trunk, hard to the sword (`AC_HARD`), 18 by 60.
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit {
        col_type: COL_MATERIAL_TREE,
        at_flags: AT_NONE,
        ac_flags: AC_ON | AC_HARD | AC_TYPE_PLAYER,
        oc_flags1: OC1_ON | OC1_TYPE_ALL,
        oc_flags2: OC2_TYPE_1,
        shape: COLSHAPE_CYLINDER,
    },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK5,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0FC0_074A, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 18, height: 60, y_shift: 0, pos: [0, 0, 0] },
};

/// `sSpawnDistance`, `sSpawnAngle`: the five offspring's places around the home, and the
/// spawner's own (the sixth).
const SPAWN_DISTANCE: [f32; 6] = [707.0, 525.0, 510.0, 500.0, 566.0, 141.0];
const SPAWN_ANGLE: [u16; 6] = [0x1FFF, 0x4C9E, 0x77F5, 0xA5C9, 0xD6C3, 0xA000];

/// `D_80B3BF54`: the opaque (or only) list by draw type; `D_80B3BF70`: the translucent leaves'
/// list (types 0 to 2; the rest of the table isn't read).
const DLISTS: [&str; 7] =
    ["object_wood02_DL_0078D0", "object_wood02_DL_007CA0", "object_wood02_DL_0080D0", "object_wood02_DL_000090", "object_wood02_DL_000340", "object_wood02_DL_000340", "object_wood02_DL_000700"];
const XLU_DLISTS: [Option<&str>; 6] = [Some("object_wood02_DL_007968"), Some("object_wood02_DL_007D38"), Some("object_wood02_DL_0081A8"), None, None, None];
const LEAF_DL: &str = "object_wood02_DL_000700";

const SEG_COLOR: u8 = 0x0B;

fn xlu_bake(dl: &str) -> String {
    format!("En_Wood02/{dl}")
}
const LEAF_BAKE: &str = "En_Wood02/leaf";

/// The trees' leaves after their env colour, and the leaf after its prim colour.
pub fn bakes() -> Vec<MeshBake> {
    let mut v: Vec<MeshBake> = XLU_DLISTS
        .iter()
        .flatten()
        .map(|dl| MeshBake {
            name: xlu_bake(dl),
            object: OBJECT.into(),
            // Gfx_SetupDL_25Xlu (the bake's start), gDPSetEnvColor(r, g, b, 0).
            segments: vec![(SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: false })],
            prelude: vec![SEG_COLOR],
            body: BakeBody::DLists(vec![(OBJECT.into(), (*dl).into())]),
        })
        .collect();
    v.push(MeshBake {
        name: LEAF_BAKE.into(),
        object: OBJECT.into(),
        // Gfx_SetupDL_25Opa, gDPSetPrimColor(0, 0, r, g, b, 127), Gfx_DrawDListOpa.
        segments: vec![(SEG_COLOR, BakeSegment::DynamicColor { env: false, prim: true })],
        prelude: vec![SEG_COLOR],
        body: BakeBody::DLists(vec![(OBJECT.into(), LEAF_DL.into())]),
    });
    v
}

pub struct EnWood02 {
    pub actor: Actor,
    /// `unk_14C`: the drop table (-1 none), then the sway's countdown (-21 up to -1).
    pub unk_14c: i16,
    /// `unk_14E`: a spawner's five slots (bit 0 spawned, 0x80 its child dropped its Skulltula
    /// or table); a spawned tree's index in its spawner (`[0]`); a leaf's life (`[0]`).
    pub unk_14e: [u8; 5],
    pub spawn_type: u8,
    /// `drawType`: the low nibble the look; a spawner's high nibble its drop table for its
    /// offspring.
    pub draw_type: u8,
    pub collider: ColliderCylinder,
}

impl EnWood02 {
    /// `EnWood02_SpawnZoneCheck`: `pos` through the view (into `projectedPos`, `projectedW`) inside
    /// the spawner's culling volume, its `1 / w` 1000 for a zero `w`.
    fn spawn_zone_check(&mut self, play: &PlayState, pos: Vec3) -> bool {
        let c = play.view_proj * pos.extend(1.0);
        self.actor.projected_pos = c.truncate();
        self.actor.projected_w = c.w;
        let a = &self.actor;
        let inv = if a.projected_w == 0.0 { 1000.0 } else { (1.0 / a.projected_w).abs() };
        let p = a.projected_pos;
        -a.culling_volume_scale < p.z
            && p.z < a.culling_volume_distance + a.culling_volume_scale
            && (p.x.abs() - a.culling_volume_scale) * inv < 1.0
            && (p.y + a.culling_volume_downward) * inv > -1.0
            && (p.y - a.culling_volume_scale) * inv < 1.0
    }

    /// `EnWood02_SpawnOffspring`: each free slot (from the last) whose place is in view gets a
    /// child of the next type (the spawned kind), its drop table the spawner's (or none if the
    /// slot's last child gave its drop), facing its slot's angle.
    fn spawn_offspring(&mut self, play: &mut PlayState) {
        for i in (0..5).rev() {
            if self.unk_14e[i] & 0x7F != 0 {
                continue;
            }
            let extra_rot: i16 = if self.actor.params == WOOD_BUSH_GREEN_LARGE_SPAWNER { 0x4000 } else { 0 };
            let angle = (SPAWN_ANGLE[i] as i16).wrapping_add(self.actor.world_rot.y).wrapping_add(extra_rot);
            let (c, s) = (cos_s(angle), sin_s(angle));
            let pos = Vec3::new(SPAWN_DISTANCE[i] * s + self.actor.home_pos.x, self.actor.home_pos.y, SPAWN_DISTANCE[i] * c + self.actor.home_pos.z);
            if !self.spawn_zone_check(play, pos) {
                continue;
            }
            let params = if self.unk_14e[i] & 0x80 != 0 { (0xFF00u16 as i16) | (self.actor.params + 1) } else { (((self.draw_type & 0xF0) as i16) << 4) | (self.actor.params + 1) };
            let rot = [self.actor.world_rot.x, SPAWN_ANGLE[i] as i16, 0];
            match play.actor_spawn_as_child(&mut self.actor, ACTOR_EN_WOOD02, pos, rot, params) {
                Ok(h) => {
                    let projected = self.actor.projected_pos;
                    if let Some(child) = play.actors.downcast_mut::<EnWood02>(h) {
                        child.unk_14e[0] = i as u8;
                        child.actor.projected_pos = projected;
                    }
                    self.unk_14e[i] |= 1;
                }
                Err(e) => {
                    log::debug!("En_Wood02: offspring {i}: {e:?}");
                    self.unk_14e[i] &= 0x80;
                }
            }
        }
    }

    /// `EnWood02_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut spawn_type = WOOD_SPAWN_NORMAL;
        let mut actor_scale = 1.0;
        let mut unk_14c = ((actor.params as u16 >> 8) & 0xFF) as i16;
        if actor.home_rot.z != 0 {
            actor.home_rot.z = (actor.home_rot.z << 8) | unk_14c;
            unk_14c = -1;
            actor.world_rot.z = 0;
            actor.shape_rot.z = 0;
        } else if unk_14c & 0x80 != 0 {
            unk_14c = -1;
        }
        actor.params &= 0xFF;
        // sInitChain: lockOnArrowOffset 5600.
        actor.target_arrow_offset = 5600.0;
        let collider = ColliderCylinder::new(&CYLINDER_INIT);
        let mut this = EnWood02 { actor, unk_14c, unk_14e: [0; 5], spawn_type: 0, draw_type: 0, collider };
        let a = &mut this.actor;
        let large = |a: &mut Actor| {
            a.culling_volume_distance = 4000.0;
            a.culling_volume_scale = 2000.0;
            a.culling_volume_downward = 2400.0;
        };
        let medium = |a: &mut Actor| {
            a.culling_volume_distance = 4000.0;
            a.culling_volume_scale = 800.0;
            a.culling_volume_downward = 1800.0;
        };
        match a.params {
            WOOD_BUSH_GREEN_LARGE_SPAWNER | WOOD_BUSH_BLACK_LARGE_SPAWNER => {
                spawn_type = 2;
                actor_scale = 1.5;
                large(a);
            }
            WOOD_BUSH_GREEN_LARGE_SPAWNED | WOOD_BUSH_BLACK_LARGE_SPAWNED => {
                spawn_type = 1;
                actor_scale = 1.5;
                large(a);
            }
            WOOD_TREE_CONICAL_LARGE | WOOD_BUSH_GREEN_LARGE | WOOD_BUSH_BLACK_LARGE => {
                actor_scale = 1.5;
                large(a);
            }
            WOOD_TREE_CONICAL_SPAWNER | WOOD_TREE_OVAL_YELLOW_SPAWNER | WOOD_TREE_OVAL_GREEN_SPAWNER | WOOD_BUSH_GREEN_SMALL_SPAWNER | WOOD_BUSH_BLACK_SMALL_SPAWNER => {
                spawn_type = 2;
                medium(a);
            }
            WOOD_TREE_CONICAL_SPAWNED | WOOD_TREE_OVAL_YELLOW_SPAWNED | WOOD_TREE_OVAL_GREEN_SPAWNED | WOOD_BUSH_GREEN_SMALL_SPAWNED | WOOD_BUSH_BLACK_SMALL_SPAWNED => {
                spawn_type = 1;
                medium(a);
            }
            WOOD_TREE_CONICAL_MEDIUM | WOOD_TREE_OVAL_GREEN | WOOD_TREE_KAKARIKO_ADULT | WOOD_BUSH_GREEN_SMALL | WOOD_BUSH_BLACK_SMALL => medium(a),
            WOOD_TREE_CONICAL_SMALL => {
                actor_scale = 0.6;
                a.culling_volume_distance = 4000.0;
                a.culling_volume_scale = 400.0;
                a.culling_volume_downward = 1000.0;
            }
            WOOD_LEAF_GREEN | WOOD_LEAF_YELLOW => {
                this.unk_14e[0] = 0x4B;
                actor_scale = 0.02;
                let a = &mut this.actor;
                a.velocity.x = play.rand.centered_float(6.0);
                a.velocity.z = play.rand.centered_float(6.0);
                a.velocity.y = (play.rand.zero_one() * 1.25) + -3.1;
            }
            _ => {}
        }
        let p = this.actor.params;
        this.draw_type = if p <= WOOD_TREE_CONICAL_SPAWNED {
            WOOD_DRAW_TREE_CONICAL
        } else if p <= WOOD_TREE_OVAL_GREEN_SPAWNED {
            WOOD_DRAW_TREE_OVAL
        } else if p <= WOOD_TREE_KAKARIKO_ADULT {
            WOOD_DRAW_TREE_KAKARIKO_ADULT
        } else if p <= WOOD_BUSH_GREEN_LARGE_SPAWNED {
            WOOD_DRAW_BUSH_GREEN
        } else if p <= WOOD_LEAF_GREEN {
            // The black bushes and the green leaves.
            WOOD_DRAW_4
        } else {
            WOOD_DRAW_LEAF_YELLOW
        };
        this.actor.scale = Vec3::splat(actor_scale);
        this.spawn_type = spawn_type;
        if spawn_type != WOOD_SPAWN_NORMAL {
            let extra_rot: i16 = if this.actor.params == WOOD_BUSH_GREEN_LARGE_SPAWNER { 0x4000 } else { 0 };
            if spawn_type == WOOD_SPAWN_SPAWNER {
                this.draw_type |= (this.unk_14c << 4) as u8;
                this.spawn_offspring(play);
                let angle = (SPAWN_ANGLE[5] as i16).wrapping_add(this.actor.world_rot.y).wrapping_add(extra_rot);
                this.actor.world_pos.x += sin_s(angle) * SPAWN_DISTANCE[5];
                this.actor.world_pos.z += cos_s(angle) * SPAWN_DISTANCE[5];
            } else {
                this.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
            }
            // Snapped to the floor, or gone over the void.
            this.actor.world_pos.y += 200.0;
            // BgCheck_EntityRaycastDown4.
            let (floor_y, _) = play.col.entity_raycast_down(this.actor.world_pos);
            if floor_y > BGCHECK_Y_MIN {
                this.actor.world_pos.y = floor_y;
            } else {
                this.actor.kill();
                return Box::new(this);
            }
        }
        // ActorShape_Init(&shape, 0, NULL, 0).
        this.actor.shape_y_offset = 0.0;
        this.actor.home_rot.y = 0;
        this.actor.col_chk_info.mass = MASS_IMMOVABLE;
        Box::new(this)
    }

    fn is_tree(&self) -> bool {
        self.actor.params <= WOOD_TREE_KAKARIKO_ADULT
    }
}

impl ActorImpl for EnWood02 {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnWood02_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if self.spawn_type == WOOD_SPAWN_SPAWNED && self.actor.parent.is_some() {
            // Out of view: gone, its spawner's slot free (0x80 if its drop was used).
            if self.actor.flags & ACTOR_FLAG_INSIDE_CULLING_VOLUME == 0 {
                let slot = self.unk_14e[0] as usize;
                let v = if self.unk_14c < 0 { 0x80 } else { 0 };
                if let Some(p) = self.actor.parent.and_then(|h| play.actors.downcast_mut::<EnWood02>(h)) {
                    if let Some(s) = p.unk_14e.get_mut(slot) {
                        *s = v;
                    }
                }
                self.actor.kill();
                return;
            }
        } else if self.spawn_type == WOOD_SPAWN_SPAWNER {
            self.spawn_offspring(play);
        }
        if self.is_tree() {
            if self.collider.base.ac_flags & AC_HIT != 0 {
                self.collider.base.ac_flags &= !AC_HIT;
                audio_play_actor_sfx2(play, NA_SE_IT_REFLECTION_WOOD);
            }
            if self.actor.home_rot.y != 0 {
                let mut drops = self.actor.world_pos;
                drops.y += 200.0;
                if self.unk_14c >= 0 && self.unk_14c < 0x64 {
                    crate::en_item00::item_drop_collectible_random(play, Some(&self.actor), drops, self.unk_14c * 16);
                } else if self.actor.home_rot.z != 0 {
                    self.actor.home_rot.z &= 0x1FFF;
                    self.actor.home_rot.z |= 0xE000u16 as i16;
                    let params = self.actor.home_rot.z;
                    let rot_y = self.actor.world_rot.y;
                    if let Err(e) = play.actor_spawn(ACTOR_EN_SW, drops, [0, rot_y, 0], params) {
                        log::debug!("En_Wood02: its Gold Skulltula: {e:?}");
                    }
                    self.actor.home_rot.z = 0;
                }
                // The falling leaves.
                if self.unk_14c >= -1 {
                    let leaves = if self.actor.params == WOOD_TREE_OVAL_YELLOW_SPAWNER || self.actor.params == WOOD_TREE_OVAL_YELLOW_SPAWNED { WOOD_LEAF_YELLOW } else { WOOD_LEAF_GREEN };
                    audio_play_actor_sfx2(play, NA_SE_EV_TREE_SWING);
                    for _ in 0..4 {
                        let yaw = play.rand.centered_float(65535.0) as i32 as i16;
                        if let Err(e) = play.actor_spawn(ACTOR_EN_WOOD02, drops, [0, yaw, 0], leaves) {
                            log::debug!("En_Wood02: a leaf: {e:?}");
                        }
                    }
                }
                self.unk_14c = -0x15;
                self.actor.home_rot.y = 0;
            }
            if self.actor.xz_dist_to_player < 600.0 {
                self.collider.update(&self.actor);
                play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
                play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
            }
        } else if self.actor.params < 0x17 {
            // A bush: Link walking through it (on foot within 20, or riding within 60).
            if self.unk_14c >= -1 {
                let speed = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map(|p| p.speed_xz()).unwrap_or(0.0);
                // (rideActor: Epona isn't ported.)
                if self.actor.xyz_dist_to_player_sq.sqrt() < 20.0 && speed != 0.0 {
                    if self.unk_14c >= 0 && self.unk_14c < 0x64 {
                        let pos = self.actor.world_pos;
                        crate::en_item00::item_drop_collectible_random(play, Some(&self.actor), pos, (self.unk_14c * 16) | (0x8000u16 as i16));
                    }
                    self.unk_14c = -0x15;
                    audio_play_actor_sfx2(play, NA_SE_EV_TREE_SWING);
                }
            }
        } else {
            // A leaf.
            self.unk_14c = self.unk_14c.wrapping_add(1);
            approach_f(&mut self.actor.velocity.x, 0.0, 1.0, 5.0 * 0.01);
            approach_f(&mut self.actor.velocity.z, 0.0, 1.0, 5.0 * 0.01);
            self.actor.update_pos();
            self.actor.shape_rot.z = (sin_s((3000i32.wrapping_mul(self.unk_14c as i32)) as i16) * 16384.0) as i16;
            self.unk_14e[0] = self.unk_14e[0].wrapping_sub(1);
            if self.unk_14e[0] == 0 {
                self.actor.kill();
            }
        }
        // The sway from a bump.
        if self.unk_14c < -1 {
            self.unk_14c += 1;
            let amplitude = sin_s(((self.unk_14c ^ -1) as i32).wrapping_mul(0x3332) as i16) * 250.0;
            let d = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
            self.actor.shape_rot.x = (cos_s(d) * amplitude) as i16;
            self.actor.shape_rot.z = (sin_s(d) * amplitude) as i16;
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.switches = vec![self.draw_type as u32];
        rs
    }

    /// `EnWood02_Draw`: green (50, 170, 70) for the green ovals and leaves, yellow (180, 155, 0)
    /// for the yellow ones, else white; a leaf after its prim colour; a tree's trunk opaque and
    /// its leaves translucent after the env colour; a bush translucent.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let ty = self.actor.params;
        let [r, g, b] = if matches!(ty, WOOD_TREE_OVAL_GREEN_SPAWNER | WOOD_TREE_OVAL_GREEN_SPAWNED | WOOD_TREE_OVAL_GREEN | WOOD_LEAF_GREEN) {
            [50, 170, 70]
        } else if matches!(ty, WOOD_TREE_OVAL_YELLOW_SPAWNER | WOOD_TREE_OVAL_YELLOW_SPAWNED | WOOD_LEAF_YELLOW) {
            [180, 155, 0]
        } else {
            [255, 255, 255]
        };
        let m = actor_draw_matrix(rs);
        let colored = |bake: String, prim: Option<[u8; 4]>, env: Option<[u8; 4]>| {
            let mut sv = SegmentValues::default();
            sv.prim[SEG_COLOR as usize] = prim;
            sv.env[SEG_COLOR as usize] = env;
            DrawCmd { mesh: MeshKey::named(keys::bake(&bake)), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } }
        };
        let draw_type = (rs.switches.first().copied().unwrap_or(0) & 0xF) as usize;
        if ty == WOOD_LEAF_GREEN || ty == WOOD_LEAF_YELLOW {
            out.opa.push(colored(LEAF_BAKE.into(), Some([r, g, b, 127]), None));
        } else if let Some(Some(xlu)) = XLU_DLISTS.get(draw_type) {
            crate::gfx_draw_dlist_opa(out, OBJECT, DLISTS[draw_type], rs);
            out.xlu.push(colored(xlu_bake(xlu), None, Some([r, g, b, 0])));
        } else if let Some(dl) = DLISTS.get(draw_type) {
            out.xlu.push(DrawCmd::new(MeshKey::named(keys::mesh(OBJECT, dl)), m));
        }
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0 && self.is_tree()).then_some(ColliderMut::Cylinder(&mut self.collider))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
