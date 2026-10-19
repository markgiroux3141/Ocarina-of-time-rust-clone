//! `Bg_Spot16_Bombstone` (`ovl_Bg_Spot16_Bombstone/z_bg_spot16_bombstone.c`): the boulder over
//! Dodongo's Cavern's mouth on Death Mountain Trail, and the pieces it breaks into.
//!
//! The boulder (`params` 0xFF) is `object_spot16_obj`'s `gDodongosCavernRock3DL` at scale 0.4,
//! gone at init once its switch flag is set. It has two colliders, both placed once at init and
//! never moved: a sphere of radius 120 round its middle, 50 up (`COL_MATERIAL_HARD`: the sword
//! bounces off; it pushes Link (OC) and is immovable), and a cylinder of radius 190 and 80 tall,
//! offset (10, 0, 50) from it (not turned by its yaw), which only an explosion hits
//! (`DMG_EXPLOSIVE`). Each frame it pushes Link back out of the half behind its face
//! (`func_808B56BC`), and it watches for a Bomb Flower's bomb Link carries (`func_808B57E0`): it
//! remembers the bomb, and once that bomb is below y 1400 within 400 (on x and z) of (-1579,
//! -790) with its fuse running, it starts one-point cutscene 4180 for the fuse's time plus 20 (the
//! shot of the cavern's mouth).
//!
//! A blast on the cylinder breaks it (`func_808B5950`): five pieces (params 0 to 4), one-point
//! 4180 for 50 frames, its switch flag and `EVENTCHKINF_23`; it isn't drawn any more and, for 60
//! frames, puffs of dust rise where it was from a table (`D_808B5EB0`, `func_800287AC`), the chime
//! (`NA_SE_SY_CORRECT_CHIME`) on the 56th, gone after the 60th.
//!
//! A piece (`params` 0 to 5) flies from the boulder by a table (`D_808B5DD8`: its speed, its
//! rise, its size, its spin, its direction, how high it starts), drawn with `object_bombiwa`'s
//! `object_bombiwa_DL_0009E0` once that object is loaded (gone without it in the room). It falls
//! (gravity -1); on a wall or landing it breaks into fragments (`EffectSsKakera`) and dust,
//! `NA_SE_EV_ROCK_BROKEN`, and is gone (60 frames at most). Piece 0 spawns a sixth (params 5)
//! where it breaks.
//!
//! The whole overlay is ported. The debug register `mREG(64)` (1: the pieces spawned again) is 0
//! and isn't read.
//!
//! Params: `type` bits 0 to 7 (0xFF the boulder, the trail's placement 0x04FF; 0 to 5 a piece,
//! spawned by the boulder and piece 0; any other is gone at init: "arg_data is strange"); the
//! init keeps only these bits in `params`. `switchFlag` bits 8 to 13 (the boulder's: set when it
//! breaks, and set at init it's gone); bits 14 and 15 unused (set by the 0xFFFF of layers 5 to
//! 7: switch flag 0x3F there; layers 5 and 6 don't load its object, layer 7 does).
//! Flags: the boulder reads and sets switch flag `switchFlag`; it sets EVENTCHKINF_23 as it
//! breaks.

use eng_math::{cos_s, sin_s};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, BGCHECKFLAG_GROUND, BGCHECKFLAG_WALL, UPDBGCHECKINFO_FLAG_0, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_EXPLOSIVE, ACTORCAT_PROP, ActorHandle, ActorImpl, ActorProfile, func_80033480};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, ObjectNeed};
use oot_game::audio::sfx::{NA_SE_EV_ROCK_BROKEN, NA_SE_SY_CORRECT_CHIME};
use oot_game::camera::CAM_ID_MAIN;
use oot_game::collision_check::*;
use oot_game::effect::kakera::KAKERA_COLOR_NONE;
use oot_game::event_chk_inf;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

use crate::en_bombf::{ACTOR_EN_BOMBF, EnBombf};

/// `ACTOR_BG_SPOT16_BOMBSTONE` (`actor_table.h`: 0x00CD).
pub const ACTOR_BG_SPOT16_BOMBSTONE: i16 = 0x00CD;
/// `OBJECT_SPOT16_OBJ` (`object_table.h`: 0x00C2): the boulder's.
pub const OBJECT: &str = "object_spot16_obj";
/// `gDodongosCavernRock3DL`: the boulder.
pub const BOULDER_DL: &str = "gDodongosCavernRock3DL";
/// `OBJECT_BOMBIWA` (`object_table.h`: 0x0163): the pieces'.
pub const OBJECT_BOMBIWA: i16 = 0x0163;
pub const BOMBIWA_OBJECT: &str = "object_bombiwa";
/// `object_bombiwa_DL_0009E0`: a piece, and its fragments.
pub const DEBRIS_DL: &str = "object_bombiwa_DL_0009E0";

/// `EVENTCHKINF_23` (`save.h`: 0x23): the cavern's boulder blown up.
pub const EVENTCHKINF_23: u16 = 0x23;
/// `PLAYER_STATE1_CARRYING_ACTOR` (`player.h`: 1 << 11).
const PLAYER_STATE1_CARRYING_ACTOR: u32 = 1 << 11;
/// The one-point cutscene of the cavern's mouth (`OnePointCutscene_Init(play, 4180, ..)`).
pub const ONEPOINT_BOULDER: i16 = 4180;

/// `Bg_Spot16_Bombstone_Profile`: `ACTORCAT_PROP`, `ACTOR_FLAG_UPDATE_CULLING_DISABLED`,
/// `OBJECT_SPOT16_OBJ`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_SPOT16_BOMBSTONE, name: "Bg_Spot16_Bombstone", category: ACTORCAT_PROP, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `PARAMS_GET_U(params, 0, 8)` (`func_808B4C30`): the boulder or a piece.
pub const TYPE: Field = Field::params(
    "type",
    0,
    8,
    Kind::Named(&[(0, "piece 0 (spawns piece 5)"), (1, "piece 1"), (2, "piece 2"), (3, "piece 3"), (4, "piece 4"), (5, "piece 5"), (0xFF, "the boulder")]),
    "the boulder (0xFF) or a piece of it (0 to 5, spawned as it breaks)",
);
/// `PARAMS_GET_U(params, 8, 6)` (`func_808B4C30`): `switchFlag`.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 8, 6, Kind::SwitchFlag, "the boulder's: set when it breaks; set at init, it's gone");
/// Bits 14 and 15: set by the 0xFFFF of layers 5 to 7, read by nothing.
pub const HIGH_BITS: Field = Field::params("bits 14 and 15", 14, 2, Kind::Unused, "set by layers 5 to 7's 0xFFFF; read by nothing");

/// Bg_Spot16_Bombstone's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, SWITCH_FLAG, HIGH_BITS],
    flags: &[
        FlagUse::new(Flag::Switch("switchFlag")).reads().sets().note("the boulder: set as it breaks; set at init, it's gone"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_23)).sets().note("the boulder broken"),
    ],
    todo: false,
    // `func_808B4E58`: the pieces (0 to 5) ask for OBJECT_BOMBIWA and are gone without it; the
    // boulder (0xFF) draws from its own object, but its pieces, spawned as it breaks, need it there
    objects: &[ObjectNeed::new("object_bombiwa").when("type", &[0, 1, 2, 3, 4, 5]).note("the pieces' model")],
    texts: &[],
};

/// `params` 0xFF: the boulder whole.
const BOULDER: i16 = 0xFF;

/// `D_808B5DD8`'s values as written (`s16`).
const fn h(v: u16) -> i16 {
    v as i16
}

/// `D_808B5DD8`, by piece: `speed`, `velocity.y`, the scale (× 1/600), the spin on x and on z
/// (`unk_210`, `unk_212`), `world.rot.y`, how far up it starts, `shape.rot`.
pub const D_808B5DD8: [[i16; 10]; 6] = [
    [h(0x0008), h(0x0004), h(0x0046), h(0x07D0), h(0xFCE0), h(0x0000), h(0x0064), h(0x0000), h(0x0000), h(0x0000)],
    [h(0x0006), h(0x0003), h(0x0032), h(0x00C8), h(0x0A28), h(0xC350), h(0x005A), h(0x0000), h(0x0000), h(0x0000)],
    [h(0x0005), h(0x0003), h(0x0028), h(0xF63C), h(0x0190), h(0x30B0), h(0x0032), h(0x0000), h(0x0000), h(0x0000)],
    [h(0x0003), h(0x0001), h(0x003C), h(0x0258), h(0xFF9C), h(0xAFC8), h(0x0032), h(0x0000), h(0x0000), h(0x0000)],
    [h(0x0003), h(0x0001), h(0x0028), h(0xF2B8), h(0xFF9C), h(0x6590), h(0x001E), h(0x0000), h(0x0000), h(0x0000)],
    [h(0x0006), h(0x0009), h(0x0028), h(0x0000), h(0x0BB8), h(0xD8F0), h(0x001E), h(0x0000), h(0x0000), h(0x0000)],
];

/// `sJntSphElementsInit`: hit by Player's weapons but explosions (`0x4FC1FFF6`), pushing; the
/// model sphere (0, 50, 0) 288 at scale 100 (never used: the init sets the world sphere).
pub const JNT_SPH_ELEMENTS: [ColliderJntSphElementInit; 1] = [ColliderJntSphElementInit {
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x4FC1_FFF6, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    limb: 0,
    model_sphere: eng_collision::math3d::Sphere16 { center: [0, 50, 0], radius: 288 },
    scale: 100,
}];

/// `sJntSphInit`: hard (the sword bounces off), hit by Player's, pushing every type.
pub const JNT_SPH_INIT: ColliderInit = ColliderInit { col_type: COL_MATERIAL_HARD, at_flags: AT_NONE, ac_flags: AC_ON | AC_HARD | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_JNTSPH };

/// `sCylinderInit`: 190 by 80, offset (10, 0, 50); only explosions hit it (`DMG_EXPLOSIVE`).
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_NONE, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0008, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_NONE,
    },
    dim: eng_collision::math3d::Cylinder16 { radius: 190, height: 80, y_shift: 0, pos: [10, 0, 50] },
};

/// `D_808B5EB0`: the dust after the blast, by frame (`unk_154`): the frame, the offset across
/// (x), up and along (z) the boulder's facing, the scale, its step and the life
/// (`func_800287AC`).
pub const D_808B5EB0: [[i16; 7]; 26] = [
    [h(0x0000), h(0x000A), h(0x003C), h(0xFFF6), h(0x0104), h(0x01E0), h(0x0007)],
    [h(0x0000), h(0x0000), h(0x0032), h(0x0000), h(0x0104), h(0x017C), h(0x000D)],
    [h(0x0001), h(0x001E), h(0x0014), h(0x0014), h(0x00A0), h(0x0104), h(0x000A)],
    [h(0x0002), h(0x0014), h(0x0028), h(0x0014), h(0x00C8), h(0x0096), h(0x0007)],
    [h(0x0003), h(0xFFD8), h(0x0064), h(0x001E), h(0x00D2), h(0x0064), h(0x0007)],
    [h(0x0004), h(0x000A), h(0x0078), h(0x000A), h(0x00A0), h(0x0028), h(0x000A)],
    [h(0x0005), h(0xFFA7), h(0x006E), h(0x0041), h(0x0096), h(0x0118), h(0x0007)],
    [h(0x0006), h(0xFFC4), h(0x0070), h(0x006D), h(0x006E), h(0x0082), h(0x0007)],
    [h(0x0007), h(0xFFF2), h(0x0050), h(0x007F), h(0x008C), h(0x00D2), h(0x0007)],
    [h(0x0008), h(0x0014), h(0x0032), h(0x001E), h(0x008C), h(0x00C8), h(0x0007)],
    [h(0x0009), h(0xFFEC), h(0x0014), h(0x0028), h(0x00C8), h(0x00B4), h(0x0007)],
    [h(0x000A), h(0x001E), h(0x0028), h(0xFFE2), h(0x0064), h(0x0078), h(0x0007)],
    [h(0x000A), h(0xFF92), h(0x001D), h(0x0078), h(0x008C), h(0x0118), h(0x000A)],
    [h(0x000B), h(0x001E), h(0x0014), h(0x001E), h(0x0050), h(0x00C8), h(0x0006)],
    [h(0x000C), h(0x0028), h(0x001E), h(0xFFE2), h(0x00AA), h(0x010E), h(0x0011)],
    [h(0x000D), h(0xFFD8), h(0x0032), h(0xFFEC), h(0x008C), h(0x0056), h(0x0006)],
    [h(0x000D), h(0x0032), h(0x0032), h(0x0032), h(0x0096), h(0x00C8), h(0x000A)],
    [h(0x000E), h(0x0028), h(0x0028), h(0xFFD8), h(0x003C), h(0x00A0), h(0x0006)],
    [h(0x000F), h(0xFFE2), h(0x0014), h(0x0032), h(0x00AA), h(0x012C), h(0x0006)],
    [h(0x0010), h(0x0028), h(0x0032), h(0x0014), h(0x00AA), h(0x0078), h(0x0007)],
    [h(0x0010), h(0x001E), h(0x0032), h(0x003C), h(0x0096), h(0x00C8), h(0x0006)],
    [h(0x0011), h(0x000A), h(0x000A), h(0x0000), h(0x0082), h(0x00DC), h(0x0006)],
    [h(0x0012), h(0x000A), h(0x0028), h(0x0014), h(0x00B4), h(0x00DC), h(0x000B)],
    [h(0x0013), h(0x000A), h(0x0005), h(0x0000), h(0x006E), h(0x0046), h(0x0011)],
    [h(0x0013), h(0xFFEC), h(0x0032), h(0xFFE2), h(0x0096), h(0x00C8), h(0x0006)],
    [h(0x0014), h(0x0050), h(0x0032), h(0x0000), h(0x0096), h(0x00C8), h(0x0008)],
];

/// `sVelocity`, `sAcceleration`: the dust's.
const S_VELOCITY: Vec3 = Vec3::ZERO;
const S_ACCELERATION: Vec3 = Vec3::new(0.0, 0.4, 0.0);

/// `D_808B6074`: the fragments' sizes (× `scale.x` × 3).
pub const D_808B6074: [f32; 5] = [66.0, 51.0, 48.0, 36.0, 21.0];
/// `D_808B6088`: the pieces the boulder spawns.
const D_808B6088: [i16; 5] = [0, 1, 2, 3, 4];

/// Where the watch looks for the falling bomb (`func_808B57E0`): below y 1400, within 400 on x and z
/// of (-1579, -790).
pub const WATCH_MAX_Y: f32 = 1400.0;
pub const WATCH_CENTER_XZ: (f32, f32) = (-1579.0, -790.0);
pub const WATCH_RADIUS: f32 = 400.0;

/// The overlay's statics (docs/adr/0059-castle-towns-actors.md): `sPlayerBomb` (the Bomb Flower
/// bomb Link was seen carrying) and `sTimer` (the watch's cutscene's time left).
#[derive(Debug, Default)]
pub struct Statics {
    pub player_bomb: Option<ActorHandle>,
    pub timer: i16,
}

/// The statics, zeroed with a fresh overlay (`static EnBombf* sPlayerBomb = NULL`,
/// `static s16 sTimer = 0`).
pub fn statics(play: &mut PlayState) -> &mut Statics {
    play.overlay_static::<Statics>(ACTOR_BG_SPOT16_BOMBSTONE)
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// None set (gone at init).
    None,
    /// `func_808B5950`: the boulder, whole.
    Boulder,
    /// `func_808B5A94`: the boulder broken: its dust, the chime.
    Broken,
    /// `func_808B5B04`: a piece waiting for `object_bombiwa`.
    WaitForObject,
    /// `func_808B5B6C`: a piece flying.
    Fly,
}

/// The colliders' ids (`collider_mut`).
pub const COL_JNT_SPH: u8 = 0;
pub const COL_CYLINDER: u8 = 1;

pub struct BgSpot16Bombstone {
    pub actor: Actor,
    pub action: Action,
    /// `dList`: the boulder's or a piece's (`None` before the init sets it).
    pub dlist: Option<&'static str>,
    /// `actor.draw` set.
    pub drawn: bool,
    /// `unk_154`: frames, counted up first thing in each update.
    pub unk_154: i16,
    pub switch_flag: i16,
    /// `unk_158`: the next row of `D_808B5EB0`.
    pub unk_158: i16,
    pub sin_rotation: f32,
    pub cos_rotation: f32,
    pub collider_jnt_sph: ColliderJntSph,
    pub collider_cylinder: ColliderCylinder,
    /// `unk_210`, `unk_212`: a piece's spin on x and z, a frame.
    pub unk_210: i16,
    pub unk_212: i16,
    pub required_object_slot: Option<usize>,
}

impl BgSpot16Bombstone {
    /// `BgSpot16Bombstone_Init`: the params split (`func_808B4C30`), then the boulder's init or a
    /// piece's; gone if that fails or the type is strange.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = BgSpot16Bombstone {
            actor,
            action: Action::None,
            dlist: None,
            drawn: false,
            unk_154: 0,
            switch_flag: 0,
            unk_158: 0,
            sin_rotation: 0.0,
            cos_rotation: 0.0,
            collider_jnt_sph: ColliderJntSph::default(),
            collider_cylinder: ColliderCylinder::default(),
            unk_210: 0,
            unk_212: 0,
            required_object_slot: None,
        };
        this.func_808b4c30();
        let should_live = match this.ty() {
            BOULDER => this.func_808b4d9c(play),
            0..=5 => this.func_808b4e58(play),
            ty => {
                log::debug!("Error : arg_data is strange (../z_bg_spot16_bombstone.c 668)(arg_data {ty:#06x})");
                false
            }
        };
        if !should_live {
            this.actor.kill();
            return Box::new(this);
        }
        log::debug!("Spot16 obj Bomb Stone (scaleX {})(arg_data {:#06x})", this.actor.scale.x, this.ty());
        Box::new(this)
    }

    /// `this->actor.params`, which the init cut to its low byte.
    pub fn ty(&self) -> i16 {
        TYPE.get(self.actor.params)
    }

    /// `func_808B4C30`: `switchFlag` from bits 8 to 13, `params` its low byte.
    fn func_808b4c30(&mut self) {
        self.switch_flag = SWITCH_FLAG.get(self.actor.params);
        self.actor.params = TYPE.get(self.actor.params); // schema: the C's write, the type kept
    }

    /// `func_808B4C4C`: the sphere, its world sphere set here once: 50 above the boulder, radius
    /// 120 (`Collider_UpdateSpheres` is never called).
    fn func_808b4c4c(&mut self) {
        let mut c = ColliderJntSph::new(&JNT_SPH_INIT, &JNT_SPH_ELEMENTS);
        let p = self.actor.world_pos;
        let ws = &mut c.elements[0].dim.world_sphere;
        ws.center = [p.x as i16, (p.y + 50.0) as i16, p.z as i16];
        ws.radius = 120;
        self.collider_jnt_sph = c;
    }

    /// `func_808B4D04`: the cylinder, its offset plus the boulder's position (`(s16)`), set once.
    fn func_808b4d04(&mut self) {
        let mut c = ColliderCylinder::new(&CYLINDER_INIT);
        let p = self.actor.world_pos;
        c.dim.pos[0] = c.dim.pos[0].wrapping_add(p.x as i16);
        c.dim.pos[1] = c.dim.pos[1].wrapping_add(p.y as i16);
        c.dim.pos[2] = c.dim.pos[2].wrapping_add(p.z as i16);
        self.collider_cylinder = c;
    }

    /// `func_808B4D9C`: the boulder's init. Gone with its switch flag; else the culling volume
    /// (`sInitChainBoulder`), scale 0.4, immovable, its colliders, its facing's sine and cosine,
    /// `gDodongosCavernRock3DL`, drawn, waiting (`func_808B5934`).
    fn func_808b4d9c(&mut self, play: &mut PlayState) -> bool {
        if play.flags.get_switch(self.switch_flag as i32) {
            log::debug!("Spot16 obj Bomb Stone destroyed");
            return false;
        }
        // sInitChainBoulder: cullingVolumeDistance 2000, cullingVolumeScale 500,
        // cullingVolumeDownward 1000.
        self.actor.culling_volume_distance = 2000.0;
        self.actor.culling_volume_scale = 500.0;
        self.actor.culling_volume_downward = 1000.0;
        self.actor.scale = Vec3::splat(0.4);
        self.actor.col_chk_info.mass = MASS_IMMOVABLE;
        self.func_808b4c4c();
        self.func_808b4d04();
        self.sin_rotation = sin_s(self.actor.shape_rot.y);
        self.cos_rotation = cos_s(self.actor.shape_rot.y);
        self.dlist = Some(BOULDER_DL);
        self.func_808b5934();
        true
    }

    /// `func_808B4E58`: a piece's init (`sInitChainDebris`: gravity -1, at most 10 down a frame,
    /// the culling volume), its row of `D_808B5DD8`: speed, rise, scale, spin, direction, 50 out
    /// that way and its height up from where it was spawned, `shape.rot`. `object_bombiwa`'s
    /// list; gone if that object isn't in the room, else waiting for it (`func_808B5AF0`).
    fn func_808b4e58(&mut self, play: &mut PlayState) -> bool {
        let scale_factor = 1.0f32 / 600.0;
        let sin_cos_pos_factor = 50.0f32;
        let a = &mut self.actor;
        // sInitChainDebris: gravity -1, minVelocityY -10, cullingVolumeDistance 1000,
        // cullingVolumeScale 200, cullingVolumeDownward 1000.
        a.gravity = -1.0;
        a.min_velocity_y = -10.0;
        a.culling_volume_distance = 1000.0;
        a.culling_volume_scale = 200.0;
        a.culling_volume_downward = 1000.0;
        let row = D_808B5DD8[TYPE.get(a.params) as usize];
        a.speed_xz = row[0] as f32;
        a.velocity.y = row[1] as f32;
        a.scale = Vec3::splat(row[2] as f32 * scale_factor);
        // (f32) then back to s16.
        self.unk_210 = row[3];
        self.unk_212 = row[4];
        let a = &mut self.actor;
        a.world_rot.y = row[5];
        let sin_value = sin_s(a.world_rot.y);
        let cos_value = cos_s(a.world_rot.y);
        a.world_pos.x = (sin_value * sin_cos_pos_factor) + a.home_pos.x;
        a.world_pos.y = row[6] as f32 + a.home_pos.y;
        a.world_pos.z = (cos_value * sin_cos_pos_factor) + a.home_pos.z;
        a.shape_rot.x = row[7];
        a.shape_rot.y = row[8];
        a.shape_rot.z = row[9];
        self.dlist = Some(DEBRIS_DL);
        self.required_object_slot = play.object_ctx.get_index(OBJECT_BOMBIWA);
        if self.required_object_slot.is_none() {
            log::debug!("Error : Bank danger! (arg_data {:#06x})(../z_bg_spot16_bombstone.c 589)", self.ty());
            return false;
        }
        self.func_808b5af0();
        true
    }

    /// `BgSpot16Bombstone_SpawnDust`: three puffs of dust (`func_80033480`) 50 above it,
    /// `scale.x × 150` across, `scale.x × 250` big growing 160, lit.
    fn spawn_dust(&self, play: &mut PlayState) {
        let scale_x1 = self.actor.scale.x * 150.0;
        let scale_x2 = (self.actor.scale.x * 250.0) as i16;
        let p = self.actor.world_pos;
        let world = Vec3::new(p.x, p.y + 50.0, p.z);
        func_80033480(play, world, scale_x1, 2, scale_x2, 0xA0, 1);
    }

    /// `func_808B5240`: the rows of `D_808B5EB0` whose frame has come (`unk_154`), each a puff
    /// of brown dust (`func_800287AC`) at its offset turned by the boulder's facing.
    fn func_808b5240(&mut self, play: &mut PlayState) {
        let actor_position = self.actor.world_pos;
        loop {
            if self.unk_158 as u16 as usize >= D_808B5EB0.len() || self.unk_154 < D_808B5EB0[self.unk_158 as usize][0] {
                break;
            }
            let row = D_808B5EB0[self.unk_158 as usize];
            let temp_unk2 = row[1] as f32;
            let temp_unk6 = row[3] as f32;
            let position = Vec3::new(
                ((self.sin_rotation * temp_unk6) + (temp_unk2 * self.cos_rotation)) + actor_position.x,
                row[2] as f32 + actor_position.y,
                ((self.cos_rotation * temp_unk6) - (temp_unk2 * self.sin_rotation)) + actor_position.z,
            );
            play.with_ss(|ss| ss.func_800287ac(position, S_VELOCITY, S_ACCELERATION, row[4], row[5], row[6]));
            self.unk_158 += 1;
        }
    }

    /// `BgSpot16Bombstone_SpawnFragments`: piece 0 spawns piece 5 where it is and throws the last
    /// two sizes of `D_808B6074`; the others all five: fragments (`EffectSsKakera`, `object_bombiwa`'s
    /// list) within 4 across and 8 to 13 up, flying out up to 8 each way and up at 0 to 14 plus
    /// 1.3 times its fall.
    fn spawn_fragments(&self, play: &mut PlayState) {
        let velocity_y_multiplier = 1.3f32;
        let mut index: usize;
        if self.ty() == 0 {
            let p = self.actor.world_pos;
            let _ = play.actor_spawn(ACTOR_BG_SPOT16_BOMBSTONE, p, [0; 3], 5);
            index = 3;
        } else {
            index = 0;
        }
        let a = &self.actor;
        let (world, scale_x, vy) = (a.world_pos, a.scale.x, a.velocity.y);
        if index < D_808B6074.len() {
            play.with_ss(|ss| loop {
                let x = ((ss.rand.zero_one() - 0.5) * 8.0) + world.x;
                let y = ((ss.rand.zero_one() * 5.0) + world.y) + 8.0;
                let z = ((ss.rand.zero_one() - 0.5) * 8.0) + world.z;
                let pos = Vec3::new(x, y, z);
                let vx = (ss.rand.zero_one() - 0.5) * 16.0;
                // (Rand_ZeroOne() * 14.0) in double, plus the float product.
                let vy = ((ss.rand.zero_one() as f64 * 14.0) + (vy.abs() * velocity_y_multiplier) as f64) as f32;
                let vz = (ss.rand.zero_one() - 0.5) * 16.0;
                let velocity = Vec3::new(vx, vy, vz);
                let scale = (D_808B6074[index] * scale_x * 3.0) as i16;
                ss.kakera_spawn(pos, velocity, world, -420, 0x31, 0xF, 0xF, 0, scale, 2, 0x40, 160, KAKERA_COLOR_NONE, OBJECT_BOMBIWA, Some((BOMBIWA_OBJECT, DEBRIS_DL)));
                index += 1;
                if index == D_808B6074.len() {
                    break;
                }
            });
        }
    }

    /// `func_808B561C`: the five pieces (`D_808B6088`) where the boulder is, until a spawn fails.
    fn func_808b561c(&self, play: &mut PlayState) {
        let pos = self.actor.world_pos;
        for &p in &D_808B6088 {
            if play.actor_spawn(ACTOR_BG_SPOT16_BOMBSTONE, pos, [0; 3], p).is_err() {
                break;
            }
        }
    }

    /// `func_808B56BC`: Link within 130 (x and z) and -10 to 160 above, in the half behind the
    /// boulder's face (the yaw to him more than 0x3FFF off its facing), is put back onto the face's
    /// plane along its facing. The yaw difference isn't wrapped: `ABS` of the two `s16`s' `s32`
    /// difference, less 0x3FFF, as an `s16`.
    fn func_808b56bc(&self, play: &mut PlayState) {
        let a = &self.actor;
        if a.xz_dist_to_player < 130.0 && a.y_dist_to_player < 160.0 && a.y_dist_to_player >= -10.0 {
            let yaw_diff = a.yaw_towards_player as i32 - a.shape_rot.y as i32;
            let abs_yaw_diff = yaw_diff.abs();
            let adjusted_yaw_diff = (abs_yaw_diff - 0x3FFF) as i16;
            if adjusted_yaw_diff > 0 {
                let sin_value = sin_s(adjusted_yaw_diff) * a.xz_dist_to_player;
                if sin_value >= 0.0 {
                    let (s, c) = (self.sin_rotation, self.cos_rotation);
                    if let Some(p) = play.player.and_then(|h| play.actors.actor_mut(h)) {
                        p.world_pos.x += sin_value * s;
                        p.world_pos.z += sin_value * c;
                    }
                } else {
                    log::debug!("Error Can't correct (../z_bg_spot16_bombstone.c 935)(arg_data {:#06x})(hosei_angY {adjusted_yaw_diff:x})", self.ty());
                }
            }
        }
    }

    /// `func_808B57E0`: the watch. `sTimer` counts down. A bomb remembered: gone (its update
    /// `NULL`), forgotten; else with `sTimer` run out, below y 1400 within 400 (x and z) of (-1579,
    /// -790), still a bomb (params `BOMBFLOWER_BODY`) and its fuse running: one-point 4180 for
    /// the fuse plus 20 (`sTimer`). None remembered: a Bomb Flower's bomb Link carries (an
    /// explosive of `ACTOR_EN_BOMBF`) is.
    fn func_808b57e0(&mut self, play: &mut PlayState) {
        let st = statics(play);
        if st.timer > 0 {
            st.timer -= 1;
        }
        let (player_bomb, s_timer) = (st.player_bomb, st.timer);
        if let Some(h) = player_bomb {
            let Some(bomb) = play.actors.downcast::<EnBombf>(h).filter(|b| !b.actor.killed) else {
                statics(play).player_bomb = None;
                return;
            };
            let p = bomb.actor.world_pos;
            let (dx, dz) = (p.x + 1579.0, p.z + 790.0);
            // Math3D_Dist1DSq(x + 1579, z + 790) < SQ(400).
            if s_timer <= 0 && p.y < WATCH_MAX_Y && (dx * dx + dz * dz) < WATCH_RADIUS * WATCH_RADIUS && crate::en_bombf::TYPE.get(bomb.actor.params) == 0 {
                let fuse = bomb.timer;
                if fuse > 0 {
                    let t = fuse + 20;
                    statics(play).timer = t;
                    play.onepoint_cutscene_init(ONEPOINT_BOULDER, t, None, CAM_ID_MAIN);
                }
            }
        } else {
            let carried = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).filter(|p| p.state_flags1() & PLAYER_STATE1_CARRYING_ACTOR != 0).and_then(|p| p.held_actor());
            if let Some(held) = carried
                && play.actors.actor(held).is_some_and(|a| a.category == ACTORCAT_EXPLOSIVE && a.id == ACTOR_EN_BOMBF)
            {
                statics(play).player_bomb = Some(held);
            }
        }
    }

    /// `func_808B5934`: drawn, waiting whole.
    fn func_808b5934(&mut self) {
        self.drawn = true;
        self.action = Action::Boulder;
    }

    /// `func_808B5950`: the boulder whole. Link pushed out, the watch; a blast on its cylinder:
    /// the five pieces, one-point 4180 for 50, its switch flag and `EVENTCHKINF_23`, broken
    /// (`func_808B5A78`). Else its cylinder set for hits, its sphere for pushing and hits.
    fn func_808b5950(&mut self, play: &mut PlayState) {
        self.func_808b56bc(play);
        self.func_808b57e0(play);
        if self.collider_cylinder.base.ac_flags & AC_HIT != 0 {
            self.collider_cylinder.base.ac_flags &= !AC_HIT;
            self.func_808b561c(play);
            play.onepoint_cutscene_init(ONEPOINT_BOULDER, 50, None, CAM_ID_MAIN);
            play.flags.set_switch(self.switch_flag as i32);
            play.save.set_event_chk_inf(EVENTCHKINF_23);
            self.func_808b5a78();
        } else {
            play.collision_check_set_ac(&self.actor, COL_CYLINDER, &mut self.collider_cylinder);
            play.collision_check_set_oc(&self.actor, COL_JNT_SPH, &mut self.collider_jnt_sph);
            play.collision_check_set_ac(&self.actor, COL_JNT_SPH, &mut self.collider_jnt_sph);
        }
        // mREG(64) == 1 (debug): the pieces again. The register is 0.
    }

    /// `func_808B5A78`: the frame count and the dust's row from 0, not drawn, broken.
    fn func_808b5a78(&mut self) {
        self.unk_154 = 0;
        self.unk_158 = 0;
        self.drawn = false;
        self.action = Action::Broken;
    }

    /// `func_808B5A94`: the dust's rows; the chime on frame 56; gone after 60.
    fn func_808b5a94(&mut self, play: &mut PlayState) {
        self.func_808b5240(play);
        if self.unk_154 == 56 {
            play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
        }
        if self.unk_154 > 60 {
            self.actor.kill();
        }
    }

    /// `func_808B5AF0`: a piece waiting for its object, not drawn.
    fn func_808b5af0(&mut self) {
        self.action = Action::WaitForObject;
        self.drawn = false;
    }

    /// `func_808B5B04`: once `object_bombiwa` is loaded, flying (`func_808B5B58`) and drawn.
    fn func_808b5b04(&mut self, play: &mut PlayState) {
        if self.required_object_slot.is_some_and(|s| play.object_ctx.is_loaded(s)) {
            self.func_808b5b58();
            self.drawn = true;
        }
    }

    /// `func_808B5B58`: the frame count from 0, flying.
    fn func_808b5b58(&mut self) {
        self.unk_154 = 0;
        self.action = Action::Fly;
    }

    /// `func_808B5B6C`: a piece flies and spins; gone after 60 frames; on a wall, or on the ground
    /// falling (last frame's checks), its fragments, its dust, `NA_SE_EV_ROCK_BROKEN` (20 frames
    /// where it is) and gone. Else its floor and walls checked (17.5 up, 35 across).
    fn func_808b5b6c(&mut self, play: &mut PlayState) {
        let a = &mut self.actor;
        a.move_forward();
        a.shape_rot.x = a.shape_rot.x.wrapping_add(self.unk_210);
        a.shape_rot.z = a.shape_rot.z.wrapping_add(self.unk_212);
        if self.unk_154 > 60 {
            self.actor.kill();
            return;
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 || (self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 && self.actor.velocity.y < 0.0) {
            self.spawn_fragments(play);
            self.spawn_dust(play);
            play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.world_pos, 20, NA_SE_EV_ROCK_BROKEN);
            self.actor.kill();
            return;
        }
        self.actor.update_bg_check_info(&play.col, 17.5, 35.0, 0.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
    }

    /// The object of the list drawn: the boulder's own, or `object_bombiwa` (`gSPSegment(6, ..)`).
    fn dlist_object(&self) -> &'static str {
        if self.ty() == BOULDER { OBJECT } else { BOMBIWA_OBJECT }
    }
}

impl ActorImpl for BgSpot16Bombstone {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BgSpot16Bombstone_Update`: `unk_154` up, then the action.
    fn update(&mut self, play: &mut PlayState) {
        self.unk_154 = self.unk_154.wrapping_add(1);
        match self.action {
            Action::None => {}
            Action::Boulder => self.func_808b5950(play),
            Action::Broken => self.func_808b5a94(play),
            Action::WaitForObject => self.func_808b5b04(play),
            Action::Fly => self.func_808b5b6c(play),
        }
    }

    /// `BgSpot16Bombstone_Destroy`: the boulder's colliders (nothing to free).
    fn destroy(&mut self, _play: &mut PlayState) {}

    /// `BgSpot16Bombstone_Draw`: `Gfx_SetupDL_25Opa`, its list (a piece's with `object_bombiwa` on
    /// segment 6).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if !self.drawn {
            return;
        }
        let Some(dl) = self.dlist else { return };
        crate::gfx_draw_dlist_opa(out, self.dlist_object(), dl, rs);
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        if self.ty() != BOULDER {
            return None;
        }
        match id {
            COL_JNT_SPH => Some(ColliderMut::JntSph(&mut self.collider_jnt_sph)),
            COL_CYLINDER => Some(ColliderMut::Cylinder(&mut self.collider_cylinder)),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_trail_s_placement_is_the_boulder_on_switch_4() {
        assert_eq!(TYPE.get(0x04FF), BOULDER);
        assert_eq!(SWITCH_FLAG.get(0x04FF), 4);
    }

    #[test]
    fn the_tables_read_as_signed() {
        assert_eq!(D_808B5DD8[1][5], -15536);
        assert_eq!(D_808B5EB0[6][1], -89);
    }
}
