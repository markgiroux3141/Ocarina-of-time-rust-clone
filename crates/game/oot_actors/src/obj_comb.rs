//! `Obj_Comb` (`ovl_Obj_Comb/z_obj_comb.c`): a beehive (`gameplay_field_keep`'s `gFieldBeehiveDL`),
//! hanging from Zora's Domain's walls (three, params 0xFF02) and elsewhere.
//!
//! It waits (`ObjComb_Wait`) with one sphere (15 across at a tenth of the model's size, round its
//! origin) taking Player's attacks (`0x4001FFFE`). The hammer, an arrow, a seed or a Deku Stick
//! only shake it (`unk_1B0` set to 1500, its swing's size, which falls by 50 a frame); anything else
//! (the sword, a bomb, the boomerang) breaks it (`ObjComb_Break`: 31 pieces of
//! `gFieldBeehiveFragmentDL` in a spiral, a turn of 0x4E20 apart, out and up from its middle, dust
//! from 10 below it, `func_80033480`) and it drops its item (`ObjComb_ChooseItemDrop`): a piece of
//! heart once (its collectible flag), anything else one time in two. Each frame its swing turns
//! (`unk_1B2` by 0x2EE0) and tips it about x (`ObjComb_Update`): drawn tipped about a point 118
//! above its origin in the model (`ObjComb_Draw`), where the sphere follows
//! (`Collider_UpdateSpheres`).
//!
//! The whole overlay is ported. `@bug (game)`: `ObjComb_ChooseItemDrop`'s `(params > 0) ||
//! (params < ITEM00_MAX)` is always true (`&&` meant): a drop of 0x1A and up still drops one time
//! in two, `Item_DropCollectible` given an `ITEM00_*` past the list.
//!
//! Params: `drop` bits 0 to 4 (the collectible it drops, `ITEM00_*`: Zora's Domain's 2, a red
//! rupee; a piece of heart once); bits 5 to 7 unused; `collectibleFlag` bits 8 to 13 (a piece of
//! heart's flag); bits 14 and 15 unused. `rot.x` its tilt (the grottos' hives hang tilted: its
//! swing is about it), `rot.z` its roll.
//! Flags: reads the collectible flag (a piece of heart already taken: no drop); the `En_Item00`
//! it drops reads and sets it.

use eng_math::{binang_to_rad, cos_s, sin_s};
use glam::{Mat4, Vec3};
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile, func_80033480};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, Source};
use oot_game::collision_check::*;
use oot_game::effect::kakera::KAKERA_COLOR_NONE;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

/// `ACTOR_OBJ_COMB` (`actor_table.h`: 0x019E).
pub const ACTOR_OBJ_COMB: i16 = 0x019E;
/// `OBJECT_GAMEPLAY_FIELD_KEEP` (`object_table.h`: 0x0002).
pub const OBJECT: &str = "gameplay_field_keep";
const OBJECT_GAMEPLAY_FIELD_KEEP: i16 = 0x0002;
/// The hive and its piece.
pub const DL: &str = "gFieldBeehiveDL";
pub const FRAGMENT_DL: &str = "gFieldBeehiveFragmentDL";

/// `Obj_Comb_Profile`: `ACTORCAT_PROP`, `FLAGS` 0.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_OBJ_COMB, name: "Obj_Comb", category: ACTORCAT_PROP, flags: 0, object: OBJECT, schema: &SCHEMA };

/// `PARAMS_GET_U(params, 0, 5)`: the drop.
pub const DROP: Field = Field::params("drop", 0, 5, Kind::Drop, "the collectible it drops as it breaks (a piece of heart once; anything else one time in two)");
/// Bits 5 to 7: read by nothing.
pub const UNUSED_5: Field = Field::params("unused bits 5 to 7", 5, 3, Kind::Unused, "read by nothing");
/// `PARAMS_GET_U(params, 8, 6)`: a piece of heart's collectible flag.
pub const COLLECTIBLE_FLAG: Field = Field::params("collectibleFlag", 8, 6, Kind::CollectibleFlag, "a piece of heart's flag (set: no drop); read with no other drop");
/// Bits 14 and 15: read by nothing.
pub const UNUSED_14: Field = Field::params("unused bits 14 and 15", 14, 2, Kind::Unused, "read by nothing (set in the data)");

/// `home.rot.x`: its tilt, which its swing swings about (the grottos' hang tilted).
pub const ROT_X: Field = Field::rot("home.rot.x", Source::RotX, 0, 16, Kind::Rotation, "its tilt about x (its swing swings about it)");
/// `shape.rot.z`: its roll, drawn.
pub const ROT_Z: Field = Field::rot("shape.rot.z", Source::RotZ, 0, 16, Kind::Rotation, "its roll");

/// Obj_Comb's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[DROP, UNUSED_5, COLLECTIBLE_FLAG, UNUSED_14, ROT_X, ROT_Z],
    flags: &[FlagUse::new(Flag::Collectible("collectibleFlag")).reads().note("a piece of heart already taken: no drop (the En_Item00 it drops sets it)")],
    todo: false,
    objects: &[],
    texts: &[],
};

/// `ITEM00_HEART_PIECE` (`z_en_item00.h`: 0x06).
const ITEM00_HEART_PIECE: i16 = 0x06;
/// `ITEM00_MAX` (`z_en_item00.h`: 0x1A).
const ITEM00_MAX: i16 = 0x1A;
/// The hive's swing on a light hit (`unk_1B0`), its fall a frame, its turn a frame.
const SHAKE: i16 = 1500;
/// The model's height of the point it tips about.
const PIVOT: f32 = 118.0;

/// `sJntSphElementsInit`: Player's attacks (`0x4001FFFE`), the sphere 15 round the origin.
const JNT_SPH_ELEMENTS: [ColliderJntSphElementInit; 1] = [ColliderJntSphElementInit {
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x4001_FFFE, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    limb: 0,
    model_sphere: eng_collision::math3d::Sphere16 { center: [0, 0, 0], radius: 15 },
    scale: 100,
}];

/// `sJntSphInit`.
const JNT_SPH_INIT: ColliderInit =
    ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_PLAYER, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_JNTSPH };

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `ObjComb_Wait`.
    Wait,
}

pub struct ObjComb {
    pub actor: Actor,
    pub action: Action,
    pub collider: ColliderJntSph,
    /// `unk_1B0`: the swing's size.
    pub unk_1b0: i16,
    /// `unk_1B2`: the swing's phase.
    pub unk_1b2: i16,
}

impl ObjComb {
    /// `ObjComb_Init`: `sInitChain` (scale 0.1; culling volume 1100, 100, 900), the sphere, waiting.
    pub fn init(actor: Actor, _play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = ObjComb { actor, action: Action::Wait, collider: ColliderJntSph::new(&JNT_SPH_INIT, &JNT_SPH_ELEMENTS), unk_1b0: 0, unk_1b2: 0 };
        this.actor.scale = Vec3::splat(0.1);
        this.actor.culling_volume_distance = 1100.0;
        this.actor.culling_volume_scale = 100.0;
        this.actor.culling_volume_downward = 900.0;
        this.setup_wait();
        Box::new(this)
    }

    /// `ObjComb_SetupWait`.
    fn setup_wait(&mut self) {
        self.action = Action::Wait;
    }

    /// `ObjComb_Break`: 31 pieces (each a turn of 0x4E20 on, up to 10 out, from 10.5 below to 10.5
    /// above), thrown out and up; small ones light (gravity -200/256, 40), big ones heavy (-340,
    /// 20); then dust 10 below it.
    pub fn break_(&self, play: &mut PlayState) {
        let wp = self.actor.world_pos;
        play.with_ss(|ss| {
            let mut angle: i16 = 0;
            for i in 0..31 {
                angle = angle.wrapping_add(0x4E20);
                let rand1 = ss.rand.zero_one() * 10.0;
                let pos1 = Vec3::new(sin_s(angle) * rand1, (i - 15) as f32 * 0.7, cos_s(angle) * rand1);
                let pos = pos1 + wp;
                let velocity = Vec3::new(
                    (ss.rand.zero_one() - 0.5) + pos1.x * 0.5,
                    (ss.rand.zero_one() - 0.5) + pos1.y * 0.6,
                    (ss.rand.zero_one() - 0.5) + pos1.z * 0.5,
                );
                let scale = (ss.rand.zero_one() * 72.0 + 25.0) as i16;
                let (gravity, arg6) = if scale < 40 {
                    (-200, 40)
                } else if scale < 70 {
                    (-280, 30)
                } else {
                    (-340, 20)
                };
                let rand2 = ss.rand.zero_one();
                let arg5 = if rand2 < 0.1 {
                    96
                } else if rand2 < 0.8 {
                    64
                } else {
                    32
                };
                ss.kakera_spawn(pos, velocity, pos, gravity, arg5, arg6, 4, 0, scale, 0, 0, 80, KAKERA_COLOR_NONE, OBJECT_GAMEPLAY_FIELD_KEEP, Some((OBJECT, FRAGMENT_DL)));
            }
        });
        let pos = Vec3::new(wp.x, wp.y - 10.0, wp.z);
        func_80033480(play, pos, 40.0, 6, 70, 60, 1);
    }

    /// `ObjComb_ChooseItemDrop`: a piece of heart unless its flag is set (with the flag); anything
    /// else one time in two.
    pub fn choose_item_drop(&self, play: &mut PlayState) {
        let mut params = DROP.get(self.actor.params);
        // @bug (game): always true (`||` for `&&`).
        if params > 0 || params < ITEM00_MAX {
            let flag = COLLECTIBLE_FLAG.get(self.actor.params);
            if params == ITEM00_HEART_PIECE {
                if play.flags.get_collectible(flag as i32) {
                    params = -1;
                } else {
                    params |= flag << 8;
                }
            } else if play.rand.zero_one() < 0.5 {
                params = -1;
            }
            if params >= 0 {
                crate::en_item00::item_drop_collectible(play, self.actor.world_pos, params);
            }
        }
    }

    /// `ObjComb_Wait`: the swing dying down; hit by the hammer, an arrow, a seed or a stick, a
    /// shake; by anything else, broken, its drop, gone. Else open to attacks; while here, to
    /// pushes.
    fn wait(&mut self, play: &mut PlayState) {
        self.unk_1b0 -= 50;
        if self.unk_1b0 < 0 {
            self.unk_1b0 = 0;
        }
        if self.collider.base.ac_flags & AC_HIT != 0 {
            self.collider.base.ac_flags &= !AC_HIT;
            let dmg_flags = self.collider.elements[0].info.ac_hit_elem.map(|h| h.at_dmg_info.dmg_flags).unwrap_or(0);
            if dmg_flags & (DMG_HAMMER | DMG_ARROW | DMG_SLINGSHOT | DMG_DEKU_STICK) != 0 {
                self.unk_1b0 = SHAKE;
            } else {
                self.break_(play);
                self.choose_item_drop(play);
                self.actor.kill();
            }
        } else {
            play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
        }
        if !self.actor.killed {
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        }
    }

    /// `ObjComb_Draw`'s matrix: tipped about the model's point 118 up, scaled.
    pub fn draw_matrix(pos: Vec3, rot: [i16; 3], scale: Vec3) -> Mat4 {
        // BINANG_TO_RAD.
        let r = binang_to_rad;
        Mat4::from_translation(Vec3::new(pos.x, pos.y + PIVOT * scale.y, pos.z))
            * Mat4::from_rotation_y(r(rot[1]))
            * Mat4::from_rotation_x(r(rot[0]))
            * Mat4::from_rotation_z(r(rot[2]))
            * Mat4::from_translation(Vec3::new(0.0, -(scale.y * PIVOT), 0.0))
            * Mat4::from_scale(scale)
    }
}

impl ActorImpl for ObjComb {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `ObjComb_Update`: the swing on, the action, tipped about x by it.
    fn update(&mut self, play: &mut PlayState) {
        self.unk_1b2 = self.unk_1b2.wrapping_add(0x2EE0);
        match self.action {
            Action::Wait => self.wait(play),
        }
        self.actor.shape_rot.x = (sin_s(self.unk_1b2) * self.unk_1b0 as f32 + self.actor.home_rot.x as f32) as i32 as i16;
    }

    /// `ObjComb_Draw`'s `Collider_UpdateSpheres(0, &collider)` under its matrix.
    fn draw_update(&mut self, _play: &mut PlayState) {
        let r = self.actor.shape_rot;
        let m = Self::draw_matrix(self.actor.world_pos, [r.x, r.y, r.z], self.actor.scale);
        self.collider.update_spheres(0, &m);
    }

    fn render_state(&self) -> RenderState {
        RenderState::of(&self.actor)
    }

    /// `ObjComb_Draw`: `Gfx_SetupDL_25Opa`, `gFieldBeehiveDL` under its matrix.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let m = Self::draw_matrix(rs.pos, rs.rot, rs.scale);
        out.opa.push(eng_gfx::DrawCmd::new(eng_gfx::MeshKey::named(oot_game::pack::keys::mesh(OBJECT, DL)), m));
    }

    /// `ObjComb_Destroy`: `Collider_DestroyJntSph`.
    fn destroy(&mut self, _play: &mut PlayState) {}

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0).then_some(ColliderMut::JntSph(&mut self.collider))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
