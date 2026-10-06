//! `Obj_Switch` (`ovl_Obj_Switch/z_obj_switch.c`): the dungeons' switches, from
//! `gameplay_dangeon_keep`.
//!
//! `params` (`z_obj_switch.h`): bits 0..2 the type, 4..6 the subtype, 8..13 the switch flag, bit 7
//! frozen (in ice, `Obj_Ice_Poly`).
//! - **Floor** (type 0) and **rusty floor** (1): DynaPoly actors (`gFloorSwitchCol`, moved with
//!   `DYNA_TRANSFORM_POS`) whose height is their `scale.y`, 0.165 up and 0.0165 down. A floor
//!   switch reads what stood on it since its last update (`dyna.interactFlags`): pressed once
//!   (subtype 0) when Link is on it; toggled (1) each time he steps on; held (2) while anything
//!   that presses switches (`ACTOR_FLAG_CAN_PRESS_SWITCHES`) stands on it, released 6 frames after;
//!   held inverted (3, unused) the same with the flag the other way. A rusty one is pressed by the
//!   hammer (its two triangles' AC, `0x40000040`).
//! - **Eye** (2): two triangles that take seeds and arrows (`0x0001F824`) shot from in front (the
//!   colliding actor's `world.rot.y` more than 0x5000 off its own facing); the eye closes over
//!   three frames, gold for once (0), silver for toggle (1), which a second shot opens.
//! - **Crystal** (3) and **targetable crystal** (4): a sphere most of Player's attacks hit
//!   (`0xEFC1FFFE`), red and off, white and on; once (0), toggle (1, the diamond, its textures
//!   scrolling) or synced to its flag (4).
//!
//! Setting the flag starts the attention camera on the switch (`OnePointCutscene_AttentionSetSfx`:
//! the correct chime for once and synced, the chest's appearing for the rest), and the switch
//! waits for the camera to look at it (`func_8005B198`: the switch category) or 100 frames before
//! it moves (`cooldownOn`, `cooldownTimer`).
//!
//! The whole overlay is ported. Not ported: the rumble (`Rumble_Request`, for no actor), the cull
//! zone (`cullingVolume*`: every actor counts as in view), and `Obj_Ice_Poly`, which a frozen
//! switch spawns as a placeholder: the ice would hold the switch (its parent's `freezeTimer` at
//! 40 a frame) until it melts, so here a frozen switch goes off on its second frame (the frozen
//! flag reads as a hit). The crystal's highlight (`func_8002ED80` with 0: only the
//! look-at for the texgen, which the renderer takes from the view) is the renderer's.

use std::sync::Arc;

use eng_collision::collision::CollisionHeader;
use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource, DYNA_INTERACT_ACTOR_SWITCH_PRESSED, DYNA_INTERACT_PLAYER_ON_TOP, DYNA_TRANSFORM_POS};
use eng_collision::math3d::Sphere16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{cos_s, sin_s};
use glam::Vec3;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_SWITCH, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::audio::sfx::{NA_SE_SY_CORRECT_CHIME, NA_SE_SY_TRE_BOX_APPEAR};
use oot_game::collision_check::*;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::scene_table::gfx_two_tex_scroll;
use oot_game::sys_matrix::MtxF;

/// `ACTOR_OBJ_SWITCH`, `ACTOR_OBJ_ICE_POLY` (`actor_table.h`: 0x012A, 0x011E).
pub const ACTOR_OBJ_SWITCH: i16 = 0x012A;
pub const ACTOR_OBJ_ICE_POLY: i16 = 0x011E;

/// `OBJECT_GAMEPLAY_DANGEON_KEEP`: the dungeons' keep, on segment 5.
pub const OBJECT: &str = "gameplay_dangeon_keep";
const COLLISION: &str = "gFloorSwitchCol";

/// `NA_SE_EV_FOOT_SWITCH`, `NA_SE_EV_DIAMOND_SWITCH` (`environmentbank_table.h`: 0x2815, 0x28BA).
pub const NA_SE_EV_FOOT_SWITCH: u16 = 0x2815;
pub const NA_SE_EV_DIAMOND_SWITCH: u16 = 0x28BA;

/// `Obj_Switch_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_OBJ_SWITCH, name: "Obj_Switch", category: ACTORCAT_SWITCH, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT };

/// `ObjSwitchType`.
pub const OBJSWITCH_TYPE_FLOOR: i16 = 0;
pub const OBJSWITCH_TYPE_FLOOR_RUSTY: i16 = 1;
pub const OBJSWITCH_TYPE_EYE: i16 = 2;
pub const OBJSWITCH_TYPE_CRYSTAL: i16 = 3;
pub const OBJSWITCH_TYPE_CRYSTAL_TARGETABLE: i16 = 4;

/// `ObjSwitchSubType`.
pub const OBJSWITCH_SUBTYPE_ONCE: i16 = 0;
pub const OBJSWITCH_SUBTYPE_TOGGLE: i16 = 1;
pub const OBJSWITCH_SUBTYPE_HOLD: i16 = 2;
pub const OBJSWITCH_SUBTYPE_HOLD_INVERTED: i16 = 3;
pub const OBJSWITCH_SUBTYPE_SYNC: i16 = 4;

/// `OBJSWITCH_FROZEN_FLAG`.
pub const OBJSWITCH_FROZEN_FLAG: i16 = 1 << 7;

/// `OBJSWITCH_PARAMS(type, subType, switchFlag)`.
pub const fn objswitch_params(ty: i16, sub_type: i16, switch_flag: i16) -> i16 {
    ty | (sub_type << 4) | (switch_flag << 8)
}

/// `OBJSWITCH_TYPE`: `PARAMS_GET_U(params, 0, 3)`.
pub fn objswitch_type(a: &Actor) -> i16 {
    a.params & 7
}

/// `OBJSWITCH_SUBTYPE`: `PARAMS_GET_U(params, 4, 3)`.
pub fn objswitch_subtype(a: &Actor) -> i16 {
    (a.params >> 4) & 7
}

/// `OBJSWITCH_SWITCH_FLAG`: `PARAMS_GET_U(params, 8, 6)`.
pub fn objswitch_switch_flag(a: &Actor) -> i32 {
    ((a.params >> 8) & 0x3F) as i32
}

/// `OBJSWITCH_FROZEN`: `PARAMS_GET_U(params, 7, 1)`.
pub fn objswitch_frozen(a: &Actor) -> bool {
    (a.params >> 7) & 1 != 0
}

/// `sFocusHeights`, by type.
const FOCUS_HEIGHTS: [f32; 5] = [10.0, 10.0, 0.0, 30.0, 30.0];

/// The floor switch's `scale.y` up (`33.0f / 200.0f`) and down (`33.0f / 2000.0f`), and its step
/// a frame (`99.0f / 2000.0f`).
pub const FLOOR_UP_SCALE_Y: f32 = 33.0 / 200.0;
pub const FLOOR_DOWN_SCALE_Y: f32 = 33.0 / 2000.0;
pub const FLOOR_SCALE_STEP: f32 = 99.0 / 2000.0;

/// `cooldownTimer`'s and `releaseTimer`'s starts, and `disableAcTimer`'s.
const COOLDOWN_FRAMES: i16 = 100;
const RELEASE_FRAMES: i16 = 6;
const DISABLE_AC_FRAMES: i16 = 10;

/// `ATTENTION_RANGE_4` (`actor.h`: 700 / 1050).
const ATTENTION_RANGE_4: u8 = 4;

/// The colliders' ids for `ActorImpl::collider_mut`: `tris.collider` (rusty floor, eye),
/// `jntSph.collider` (crystal). (The C's union: a switch uses one.)
const COLLIDER_TRIS: u8 = 0;
const COLLIDER_JNT_SPH: u8 = 1;

/// A triangle element: `ACELEM_ON` only, no AT, `dmg_flags` taken.
fn tris_element(elem_material: u8, dmg_flags: u32, vtx: [Vec3; 3]) -> ColliderTrisElementInit {
    ColliderTrisElementInit {
        info: ColliderElementInit {
            elem_material,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
            at_elem_flags: ATELEM_NONE,
            ac_elem_flags: ACELEM_ON,
            oc_elem_flags: OCELEM_NONE,
        },
        vtx,
    }
}

/// `sRustyFloorTrisElementsInit`: the top's two halves, 19 up, hit by the hammer
/// (`0x40000040`: `DMG_HAMMER_SWING | DMG_HAMMER_JUMP`).
pub fn rusty_floor_tris_elements() -> [ColliderTrisElementInit; 2] {
    let v = Vec3::new;
    [
        tris_element(ELEM_MATERIAL_UNK0, 0x4000_0040, [v(-20.0, 19.0, -20.0), v(-20.0, 19.0, 20.0), v(20.0, 19.0, 20.0)]),
        tris_element(ELEM_MATERIAL_UNK0, 0x4000_0040, [v(20.0, 19.0, 20.0), v(20.0, 19.0, -20.0), v(-20.0, 19.0, -20.0)]),
    ]
}

/// `sEyeTrisElementsInit`: the eye's two halves, 8.5 in front, hit by the slingshot and the
/// arrows (`0x0001F824`); the first `ELEM_MATERIAL_UNK4`.
pub fn eye_tris_elements() -> [ColliderTrisElementInit; 2] {
    let v = Vec3::new;
    [
        tris_element(ELEM_MATERIAL_UNK4, 0x0001_F824, [v(0.0, 23.0, 8.5), v(-23.0, 0.0, 8.5), v(0.0, -23.0, 8.5)]),
        tris_element(ELEM_MATERIAL_UNK0, 0x0001_F824, [v(0.0, 23.0, 8.5), v(0.0, -23.0, 8.5), v(23.0, 0.0, 8.5)]),
    ]
}

/// `sRustyFloorTrisInit` and `sEyeTrisInit`: AC by Player's attacks only.
const TRIS_INIT: ColliderInit = ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_NONE, shape: COLSHAPE_TRIS };

/// `sCrystalJntSphElementsInit`: one sphere on limb 0, `{0, 300, 0}`, 20, scale 100, hit by
/// `0xEFC1FFFE`, and OC.
pub fn crystal_jnt_sph_elements() -> [ColliderJntSphElementInit; 1] {
    [ColliderJntSphElementInit {
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xEFC1_FFFE, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
            at_elem_flags: ATELEM_NONE,
            ac_elem_flags: ACELEM_ON,
            oc_elem_flags: OCELEM_ON,
        },
        limb: 0,
        model_sphere: Sphere16 { center: [0, 300, 0], radius: 20 },
        scale: 100,
    }]
}

/// `sCrystalJntSphInit`: metal, AC by Player's attacks, OC with everything (`OC2_TYPE_2`).
const JNT_SPH_INIT: ColliderInit =
    ColliderInit { col_type: COL_MATERIAL_METAL, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_JNTSPH };

/// `ObjSwitch_DrawFloor`'s `floorSwitchDLists`, by subtype (the held ones share a list).
const FLOOR_SWITCH_DLISTS: [&str; 4] = ["gFloorSwitch1DL", "gFloorSwitch3DL", "gFloorSwitch2DL", "gFloorSwitch2DL"];
const RUSTY_FLOOR_SWITCH_DL: &str = "gRustyFloorSwitchDL";

/// `ObjSwitch_DrawEye`'s `eyeTextures` (segment 8) by subtype and `eyeTexIndex`, and its
/// `eyeSwitchDLs`. Each texture is a bake of its list (docs/adr/0012-actor-bakes.md).
pub const EYE_TEXTURES: [[&str; 4]; 2] = [
    ["gEyeSwitchGoldOpenTex", "gEyeSwitchGoldOpeningTex", "gEyeSwitchGoldClosingTex", "gEyeSwitchGoldClosedTex"],
    ["gEyeSwitchSilverOpenTex", "gEyeSwitchSilverHalfTex", "gEyeSwitchSilverClosedTex", "gEyeSwitchSilverClosedTex"],
];
const EYE_SWITCH_DLS: [&str; 2] = ["gEyeSwitch1DL", "gEyeSwitch2DL"];
const SEG_EYE_TEX: u8 = 0x08;

/// `ObjSwitch_DrawCrystal`'s lists by subtype (`NULL` for the held ones): the translucent ones
/// (no segments), and the opaque ones, which call segment 8 (`Gfx_TwoTexScroll`) and, the
/// diamond's, a texture on segment 9 (`crystalSubtype1texture`).
const XLU_DLISTS: [Option<&str>; 5] = [Some("gCrystalSwitchCoreXluDL"), Some("gCrystalSwitchDiamondXluDL"), None, None, Some("gCrystalSwitchCoreXluDL")];
const OPA_DLISTS: [Option<&str>; 5] = [Some("gCrystalSwitchCoreOpaDL"), Some("gCrystalSwitchDiamondOpaDL"), None, None, Some("gCrystalSwitchCoreOpaDL")];
const SEG_SCROLL: u8 = 0x08;
const SEG_CRYSTAL_TEX: u8 = 0x09;
/// The segment the bakes take `gDPSetEnvColor(crystalColor, 128)` from (the draw's own command,
/// before the list).
const SEG_ENV: u8 = 0x0B;

/// `crystalSubtype1texture`: `gCrystalSwitchRedTex` off, `gCrystalSwitchBlueTex` on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrystalTex {
    Red,
    Blue,
}

impl CrystalTex {
    pub fn symbol(self) -> &'static str {
        match self {
            CrystalTex::Red => "gCrystalSwitchRedTex",
            CrystalTex::Blue => "gCrystalSwitchBlueTex",
        }
    }
}

fn eye_bake(tex: &str) -> String {
    format!("Obj_Switch/eye/{tex}")
}

fn crystal_opa_bake(dl: &str, tex: Option<CrystalTex>) -> String {
    match tex {
        Some(t) => format!("Obj_Switch/{dl}/{}", t.symbol()),
        None => format!("Obj_Switch/{dl}"),
    }
}

/// `Gfx_TwoTexScroll(gfxCtx, G_TX_RENDERTILE, x1, y1, 0x20, 0x20, 1, x2, y2, 0x20, 0x20)`: the
/// crystal's scroll on segment 8.
pub fn crystal_scroll(x1: u8, y1: u8, x2: u8, y2: u8) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, x1 as u32, y1 as u32, 0x20, 0x20, 1, x2 as u32, y2 as u32, 0x20, 0x20)
}

/// The draws' meshes: the eye's list with each of its textures on segment 8, and the crystal's
/// opaque lists with the scroll on segment 8 and the env colour (the diamond's with each of its
/// textures on segment 9). The floor switches' lists and the crystal's translucent ones bind no
/// segment: they're the pack's meshes of `gameplay_dangeon_keep` (`Gfx_SetupDL_25Opa`, the same
/// list as `Gfx_SetupDL_25Xlu`).
pub fn bakes() -> Vec<MeshBake> {
    let mut v = Vec::new();
    for (dl, texs) in EYE_SWITCH_DLS.iter().zip(EYE_TEXTURES.iter()) {
        let mut seen: Vec<&str> = Vec::new();
        for tex in texs {
            if seen.contains(tex) {
                continue;
            }
            seen.push(tex);
            v.push(MeshBake {
                name: eye_bake(tex),
                object: OBJECT.into(),
                segments: vec![(SEG_EYE_TEX, BakeSegment::Texture { file: OBJECT.into(), symbol: (*tex).into() })],
                prelude: vec![],
                body: BakeBody::DLists(vec![(OBJECT.into(), (*dl).into())]),
            });
        }
    }
    let crystal = |dl: &str, tex: Option<CrystalTex>| {
        let mut segments = vec![(SEG_ENV, BakeSegment::DynamicColor { env: true, prim: false }), (SEG_SCROLL, BakeSegment::Dynamic(crystal_scroll(0, 0, 0, 0)))];
        if let Some(t) = tex {
            segments.push((SEG_CRYSTAL_TEX, BakeSegment::Texture { file: OBJECT.into(), symbol: t.symbol().into() }));
        }
        MeshBake { name: crystal_opa_bake(dl, tex), object: OBJECT.into(), segments, prelude: vec![SEG_ENV], body: BakeBody::DLists(vec![(OBJECT.into(), dl.into())]) }
    };
    v.push(crystal("gCrystalSwitchCoreOpaDL", None));
    v.push(crystal("gCrystalSwitchDiamondOpaDL", Some(CrystalTex::Red)));
    v.push(crystal("gCrystalSwitchDiamondOpaDL", Some(CrystalTex::Blue)));
    v
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    FloorUp,
    FloorPress,
    FloorDown,
    FloorRelease,
    /// `ObjSwitch_EyeInit`: a frozen switch's first frame (`ObjSwitch_EyeFrozenInit`).
    EyeInit,
    EyeOpen,
    EyeClosing,
    EyeClosed,
    EyeOpening,
    CrystalOff,
    CrystalTurnOn,
    CrystalOn,
    CrystalTurnOff,
}

impl Action {
    /// The decomp's name for the action function.
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::FloorUp => "ObjSwitch_FloorUp",
            Action::FloorPress => "ObjSwitch_FloorPress",
            Action::FloorDown => "ObjSwitch_FloorDown",
            Action::FloorRelease => "ObjSwitch_FloorRelease",
            Action::EyeInit => "ObjSwitch_EyeInit",
            Action::EyeOpen => "ObjSwitch_EyeOpen",
            Action::EyeClosing => "ObjSwitch_EyeClosing",
            Action::EyeClosed => "ObjSwitch_EyeClosed",
            Action::EyeOpening => "ObjSwitch_EyeOpening",
            Action::CrystalOff => "ObjSwitch_CrystalOff",
            Action::CrystalTurnOn => "ObjSwitch_CrystalTurnOn",
            Action::CrystalOn => "ObjSwitch_CrystalOn",
            Action::CrystalTurnOff => "ObjSwitch_CrystalTurnOff",
        }
    }
}

pub struct ObjSwitch {
    /// `dyna.actor`.
    pub actor: Actor,
    /// `dyna.bgId`: the floor types' bg actor (`BG_ACTOR_MAX` for the others, which have none).
    pub bg: u16,
    pub action: Action,
    /// `releaseTimer`: the held subtypes' frames before the release.
    pub release_timer: i16,
    /// `disableAcTimer`: the crystal's frames without AC after a hit.
    pub disable_ac_timer: i16,
    /// `cooldownTimer`: the most frames a switch waits for the attention camera.
    pub cooldown_timer: i16,
    /// `cooldownOn`: the flag changed with a camera to wait for.
    pub cooldown_on: bool,
    /// `eyeTexIndex`: 0 open to 3 closed.
    pub eye_tex_index: i16,
    /// `crystalSubtype1texture`.
    pub crystal_subtype1_texture: CrystalTex,
    /// `x1TexScroll`, `y1TexScroll`, `x2TexScroll`, `y2TexScroll`.
    pub x1_tex_scroll: u8,
    pub y1_tex_scroll: u8,
    pub x2_tex_scroll: u8,
    pub y2_tex_scroll: u8,
    /// `crystalColor`.
    pub crystal_color: [u8; 3],
    /// `prevColFlags`: last update's `dyna.interactFlags` (floors) or AC flags (eye, crystal).
    pub prev_col_flags: u8,
    /// `tris.collider` (rusty floor, eye).
    pub tris: ColliderTris,
    /// `jntSph.collider` (crystal).
    pub jnt_sph: ColliderJntSph,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

fn load_collision(play: &PlayState) -> anyhow::Result<Arc<CollisionHeader>> {
    let assets = play.assets.as_ref().ok_or_else(|| anyhow::anyhow!("no asset pack"))?;
    Ok(Arc::new(assets.pack.collision(&keys::collision(OBJECT, COLLISION))?))
}

/// `ObjSwitch_RotateY`.
pub fn rotate_y(src: Vec3, rot_y: i16) -> Vec3 {
    let s = sin_s(rot_y);
    let c = cos_s(rot_y);
    Vec3::new(src.z * s + src.x * c, src.y, src.z * c - src.x * s)
}

impl ObjSwitch {
    fn ty(&self) -> i16 {
        objswitch_type(&self.actor)
    }

    fn sub_type(&self) -> i16 {
        objswitch_subtype(&self.actor)
    }

    fn switch_flag(&self) -> i32 {
        objswitch_switch_flag(&self.actor)
    }

    fn is_floor(&self) -> bool {
        matches!(self.ty(), OBJSWITCH_TYPE_FLOOR | OBJSWITCH_TYPE_FLOOR_RUSTY)
    }

    fn is_crystal(&self) -> bool {
        matches!(self.ty(), OBJSWITCH_TYPE_CRYSTAL | OBJSWITCH_TYPE_CRYSTAL_TARGETABLE)
    }

    /// `dyna.interactFlags` (`DYNA_INTERACT_*`): what stood on it since its last update.
    fn interact_flags(&self, play: &PlayState) -> u8 {
        play.col.dyna.actors.get(self.bg as usize).map_or(0, |a| a.interact_flags.get())
    }

    /// The switch as the one-point cutscenes read it (it's out of the arena in its update).
    fn cam_actor(&self, play: &PlayState) -> Option<oot_game::camera::CamActor> {
        play.cur_actor.map(|h| play.cam_actor_of(h, &self.actor))
    }

    /// `ObjSwitch_InitDynaPoly`: `DynaPolyActor_Init(moveFlag)`, `DynaPoly_SetBgActor` (with the
    /// actor as `Actor_Init` left it: scale 0.01).
    fn init_dyna_poly(&mut self, play: &mut PlayState, move_flag: u32) {
        self.bg = match load_collision(play) {
            Ok(h) => play.col.dyna.set_bg_actor(h, source(&self.actor), move_flag),
            Err(e) => {
                log::error!("Obj_Switch: {e:#}");
                BG_ACTOR_MAX
            }
        };
        if self.bg == BG_ACTOR_MAX {
            log::warn!("Warning : move BG registration failed (../z_obj_switch.c 531)(name {})(arg_data {:#06x})", self.actor.id, self.actor.params);
        }
    }

    /// `ObjSwitch_InitJntSphCollider`: the sphere placed once, through the actor's matrix
    /// (`Matrix_SetTranslateRotateYXZ` at the position plus `shape.yOffset * scale.y`, then
    /// `Matrix_Scale`), `Collider_UpdateSpheres(0)`.
    fn init_jnt_sph_collider(&mut self) {
        let mut c = ColliderJntSph::new(&JNT_SPH_INIT, &crystal_jnt_sph_elements());
        let a = &self.actor;
        let r = a.shape_rot;
        let mut m = MtxF::set_translate_rotate_yxz(a.world_pos.x, a.world_pos.y + a.shape_y_offset * a.scale.y, a.world_pos.z, [r.x, r.y, r.z]);
        m.scale(a.scale.x, a.scale.y, a.scale.z);
        c.update_spheres(0, &m.to_mat4());
        self.jnt_sph = c;
    }

    /// `ObjSwitch_InitTrisCollider`: each vertex turned by `home.rot.y` (`ObjSwitch_RotateY`)
    /// about the position.
    fn init_tris_collider(&mut self, elements: &[ColliderTrisElementInit; 2]) {
        let mut c = ColliderTris::new(&TRIS_INIT, elements);
        for (i, e) in elements.iter().enumerate() {
            let pos = e.vtx.map(|v| rotate_y(v, self.actor.home_rot.y) + self.actor.world_pos);
            c.set_vertices(i, pos[0], pos[1], pos[2]);
        }
        self.tris = c;
    }

    /// `ObjSwitch_SpawnIce`: `Obj_Ice_Poly` as its child, with the switch flag in its params.
    fn spawn_ice(&mut self, play: &mut PlayState) -> bool {
        let (pos, r) = (self.actor.world_pos, self.actor.world_rot);
        let params = (self.switch_flag() << 8) as i16;
        play.actor_spawn_as_child(&mut self.actor, ACTOR_OBJ_ICE_POLY, pos, [r.x, r.y, r.z], params).is_ok()
    }

    /// `ObjSwitch_SetOn`: the flag set (if it wasn't) with the attention camera on the switch:
    /// `cooldownOn`.
    fn set_on(&mut self, play: &mut PlayState) {
        let flag = self.switch_flag();
        if play.flags.get_switch(flag) {
            self.cooldown_on = false;
        } else {
            let sub_type = self.sub_type();
            play.flags.set_switch(flag);
            let sfx = if sub_type == OBJSWITCH_SUBTYPE_ONCE || sub_type == OBJSWITCH_SUBTYPE_SYNC { NA_SE_SY_CORRECT_CHIME } else { NA_SE_SY_TRE_BOX_APPEAR };
            if let Some(me) = self.cam_actor(play) {
                play.onepoint_attention_set_sfx(me, sfx);
            }
            self.cooldown_on = true;
        }
    }

    /// `ObjSwitch_SetOff`: the flag cleared (if it was set); a toggle's with the attention
    /// camera, `cooldownOn`.
    fn set_off(&mut self, play: &mut PlayState) {
        self.cooldown_on = false;
        let flag = self.switch_flag();
        if play.flags.get_switch(flag) {
            play.flags.unset_switch(flag);
            if self.sub_type() == OBJSWITCH_SUBTYPE_TOGGLE {
                if let Some(me) = self.cam_actor(play) {
                    play.onepoint_attention_set_sfx(me, NA_SE_SY_TRE_BOX_APPEAR);
                }
                self.cooldown_on = true;
            }
        }
    }

    /// `ObjSwitch_UpdateTwoTexScrollXY`.
    fn update_two_tex_scroll_xy(&mut self) {
        self.x1_tex_scroll = self.x1_tex_scroll.wrapping_sub(1) & 0x7F;
        self.y1_tex_scroll = self.y1_tex_scroll.wrapping_add(1) & 0x7F;
        self.x2_tex_scroll = self.x2_tex_scroll.wrapping_add(1) & 0x7F;
        self.y2_tex_scroll = self.y2_tex_scroll.wrapping_sub(1) & 0x7F;
    }

    /// `ObjSwitch_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = ObjSwitch {
            actor,
            bg: BG_ACTOR_MAX,
            action: Action::FloorUp,
            release_timer: 0,
            disable_ac_timer: 0,
            cooldown_timer: 0,
            cooldown_on: false,
            eye_tex_index: 0,
            crystal_subtype1_texture: CrystalTex::Red,
            x1_tex_scroll: 0,
            y1_tex_scroll: 0,
            x2_tex_scroll: 0,
            y2_tex_scroll: 0,
            crystal_color: [0; 3],
            prev_col_flags: 0,
            tris: ColliderTris::default(),
            jnt_sph: ColliderJntSph::default(),
        };
        let is_switch_flag_set = play.flags.get_switch(this.switch_flag());
        let ty = this.ty();
        if this.is_floor() {
            this.init_dyna_poly(play, DYNA_TRANSFORM_POS);
        }
        // Actor_ProcessInitChain(sInitChain): ICHAIN_VEC3F_DIV1000(scale, 100) (the cull zone
        // isn't ported).
        this.actor.scale = Vec3::splat(0.1);
        if this.is_floor() {
            this.actor.world_pos.y = this.actor.home_pos.y + 1.0;
        }
        // sFocusHeights[type] (a type past 4 reads past the table in the C: 0 here).
        this.actor.set_focus(FOCUS_HEIGHTS.get(ty as usize).copied().unwrap_or(0.0));
        if ty == OBJSWITCH_TYPE_FLOOR_RUSTY {
            this.init_tris_collider(&rusty_floor_tris_elements());
        } else if ty == OBJSWITCH_TYPE_EYE {
            this.init_tris_collider(&eye_tris_elements());
        } else if this.is_crystal() {
            this.init_jnt_sph_collider();
        }
        if ty == OBJSWITCH_TYPE_CRYSTAL_TARGETABLE {
            this.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
            this.actor.target_mode = ATTENTION_RANGE_4;
        }
        this.actor.col_chk_info.mass = MASS_IMMOVABLE;
        if objswitch_frozen(&this.actor) && !this.spawn_ice(play) {
            log::error!("Error : Ice failed to spawn (../z_obj_switch.c 732)");
            this.actor.params &= !OBJSWITCH_FROZEN_FLAG;
        }
        if objswitch_frozen(&this.actor) {
            this.eye_frozen_init();
        } else if this.is_floor() {
            // @bug (game): this doesn't account for OBJSWITCH_SUBTYPE_HOLD_INVERTED, whose
            // switch position and flag are the other way round.
            if is_switch_flag_set {
                this.floor_down_init();
            } else {
                this.floor_up_init();
            }
        } else if ty == OBJSWITCH_TYPE_EYE {
            if is_switch_flag_set {
                this.eye_closed_init();
            } else {
                this.eye_open_init();
            }
        } else if this.is_crystal() {
            if is_switch_flag_set {
                this.crystal_on_init();
            } else {
                this.crystal_off_init();
            }
        }
        log::debug!("(Dungeon switch)(arg_data {:#06x})", this.actor.params);
        if this.bg != BG_ACTOR_MAX {
            play.col.dyna.set_source(this.bg, source(&this.actor));
        }
        Box::new(this)
    }

    /// `ObjSwitch_FloorUpInit`.
    fn floor_up_init(&mut self) {
        self.actor.scale.y = FLOOR_UP_SCALE_Y;
        self.action = Action::FloorUp;
    }

    /// `ObjSwitch_FloorUp`: a rusty one waits for the hammer; the others for what their
    /// subtype reads of what stood on them.
    fn floor_up(&mut self, play: &mut PlayState) {
        if self.ty() == OBJSWITCH_TYPE_FLOOR_RUSTY {
            if self.tris.base.ac_flags & AC_HIT != 0 {
                self.floor_press_init();
                self.set_on(play);
                self.tris.base.ac_flags &= !AC_HIT;
            } else {
                play.collision_check_set_ac(&self.actor, COLLIDER_TRIS, &mut self.tris);
            }
            return;
        }
        let flags = self.interact_flags(play);
        match self.sub_type() {
            OBJSWITCH_SUBTYPE_ONCE => {
                // DynaPolyActor_IsPlayerOnTop.
                if flags & DYNA_INTERACT_PLAYER_ON_TOP != 0 {
                    self.floor_press_init();
                    self.set_on(play);
                }
            }
            OBJSWITCH_SUBTYPE_TOGGLE => {
                if flags & DYNA_INTERACT_PLAYER_ON_TOP != 0 && self.prev_col_flags & DYNA_INTERACT_PLAYER_ON_TOP == 0 {
                    self.floor_press_init();
                    self.set_on(play);
                }
            }
            OBJSWITCH_SUBTYPE_HOLD => {
                // DynaPolyActor_IsSwitchPressed.
                if flags & DYNA_INTERACT_ACTOR_SWITCH_PRESSED != 0 {
                    self.floor_press_init();
                    self.set_on(play);
                }
            }
            OBJSWITCH_SUBTYPE_HOLD_INVERTED => {
                if flags & DYNA_INTERACT_ACTOR_SWITCH_PRESSED != 0 {
                    self.floor_press_init();
                    self.set_off(play);
                }
            }
            _ => {}
        }
    }

    /// `ObjSwitch_FloorPressInit`.
    fn floor_press_init(&mut self) {
        self.action = Action::FloorPress;
        self.cooldown_timer = COOLDOWN_FRAMES;
    }

    /// `ObjSwitch_FloorPress`: down by 0.0495 a frame once the camera looks at it (or with
    /// nothing to wait for), with the switch's sound at the bottom.
    fn floor_press(&mut self, play: &mut PlayState) {
        if self.sub_type() == OBJSWITCH_SUBTYPE_HOLD_INVERTED || !self.cooldown_on || play.func_8005b198() == self.actor.category as i32 || self.cooldown_timer <= 0 {
            self.actor.scale.y -= FLOOR_SCALE_STEP;
            if self.actor.scale.y <= FLOOR_DOWN_SCALE_Y {
                self.floor_down_init();
                audio_play_actor_sfx2(play, NA_SE_EV_FOOT_SWITCH);
                // Rumble_Request(xyzDistToPlayerSq, 120, 20, 10): not ported.
            }
        }
    }

    /// `ObjSwitch_FloorDownInit`.
    fn floor_down_init(&mut self) {
        self.actor.scale.y = FLOOR_DOWN_SCALE_Y;
        self.release_timer = RELEASE_FRAMES;
        self.action = Action::FloorDown;
    }

    /// `ObjSwitch_FloorDown`.
    fn floor_down(&mut self, play: &mut PlayState) {
        match self.sub_type() {
            OBJSWITCH_SUBTYPE_ONCE => {
                if !play.flags.get_switch(self.switch_flag()) {
                    self.floor_release_init();
                }
            }
            OBJSWITCH_SUBTYPE_TOGGLE => {
                if self.interact_flags(play) & DYNA_INTERACT_PLAYER_ON_TOP != 0 && self.prev_col_flags & DYNA_INTERACT_PLAYER_ON_TOP == 0 {
                    self.floor_release_init();
                    self.set_off(play);
                }
            }
            OBJSWITCH_SUBTYPE_HOLD | OBJSWITCH_SUBTYPE_HOLD_INVERTED => {
                if self.interact_flags(play) & DYNA_INTERACT_ACTOR_SWITCH_PRESSED == 0 && !play.player_in_cs_mode() {
                    if self.release_timer <= 0 {
                        self.floor_release_init();
                        if self.sub_type() == OBJSWITCH_SUBTYPE_HOLD {
                            self.set_off(play);
                        } else {
                            self.set_on(play);
                        }
                    }
                } else {
                    self.release_timer = RELEASE_FRAMES;
                }
            }
            _ => {}
        }
    }

    /// `ObjSwitch_FloorReleaseInit`.
    fn floor_release_init(&mut self) {
        self.action = Action::FloorRelease;
        self.cooldown_timer = COOLDOWN_FRAMES;
    }

    /// `ObjSwitch_FloorRelease`: up by 0.0495 a frame (a toggle and an inverted held switch once
    /// the camera looks at it), with the switch's sound at the top.
    fn floor_release(&mut self, play: &mut PlayState) {
        let sub_type = self.sub_type();
        if (sub_type != OBJSWITCH_SUBTYPE_TOGGLE && sub_type != OBJSWITCH_SUBTYPE_HOLD_INVERTED) || !self.cooldown_on || play.func_8005b198() == self.actor.category as i32 || self.cooldown_timer <= 0
        {
            self.actor.scale.y += FLOOR_SCALE_STEP;
            if self.actor.scale.y >= FLOOR_UP_SCALE_Y {
                self.floor_up_init();
                audio_play_actor_sfx2(play, NA_SE_EV_FOOT_SWITCH);
                // A toggle's Rumble_Request(xyzDistToPlayerSq, 120, 20, 10): not ported.
            }
        }
    }

    /// `ObjSwitch_EyeIsHit`: a new hit (not last frame's too) by an actor facing the eye's
    /// front (its `world.rot.y` more than 0x5000 from the eye's `shape.rot.y`).
    fn eye_is_hit(&self, play: &PlayState) -> bool {
        if self.tris.base.ac_flags & AC_HIT != 0
            && self.prev_col_flags & AC_HIT == 0
            && let Some(colliding) = self.tris.base.ac.and_then(|h| play.actors.actor(h))
        {
            let yaw_diff = colliding.world_rot.y.wrapping_sub(self.actor.shape_rot.y);
            // ABS on the s16, promoted to int: -0x8000 counts as 0x8000.
            if (yaw_diff as i32).abs() > 0x5000 {
                return true;
            }
        }
        false
    }

    /// `ObjSwitch_EyeFrozenInit`.
    fn eye_frozen_init(&mut self) {
        self.action = Action::EyeInit;
    }

    /// `ObjSwitch_EyeInit`.
    fn eye_init(&mut self, play: &mut PlayState) {
        if play.flags.get_switch(self.switch_flag()) {
            self.eye_closed_init();
        } else {
            self.eye_open_init();
        }
    }

    /// `ObjSwitch_EyeOpenInit`.
    fn eye_open_init(&mut self) {
        self.action = Action::EyeOpen;
        self.eye_tex_index = 0;
    }

    /// `ObjSwitch_EyeOpen`.
    fn eye_open(&mut self, play: &mut PlayState) {
        if self.eye_is_hit(play) || objswitch_frozen(&self.actor) {
            self.eye_closing_init();
            self.set_on(play);
            self.actor.params &= !OBJSWITCH_FROZEN_FLAG;
        }
    }

    /// `ObjSwitch_EyeClosingInit`.
    fn eye_closing_init(&mut self) {
        self.action = Action::EyeClosing;
        self.cooldown_timer = COOLDOWN_FRAMES;
    }

    /// `ObjSwitch_EyeClosing`: a texture a frame once the camera looks at it.
    fn eye_closing(&mut self, play: &mut PlayState) {
        if !self.cooldown_on || play.func_8005b198() == self.actor.category as i32 || self.cooldown_timer <= 0 {
            self.eye_tex_index += 1;
            if self.eye_tex_index >= 3 {
                self.eye_closed_init();
                audio_play_actor_sfx2(play, NA_SE_EV_FOOT_SWITCH);
            }
        }
    }

    /// `ObjSwitch_EyeClosedInit`.
    fn eye_closed_init(&mut self) {
        self.action = Action::EyeClosed;
        self.eye_tex_index = 3;
    }

    /// `ObjSwitch_EyeClosed`.
    fn eye_closed(&mut self, play: &mut PlayState) {
        match self.sub_type() {
            OBJSWITCH_SUBTYPE_ONCE => {
                if !play.flags.get_switch(self.switch_flag()) {
                    self.eye_opening_init();
                    self.actor.params &= !OBJSWITCH_FROZEN_FLAG;
                }
            }
            OBJSWITCH_SUBTYPE_TOGGLE => {
                if self.eye_is_hit(play) || objswitch_frozen(&self.actor) {
                    self.eye_opening_init();
                    self.set_off(play);
                    self.actor.params &= !OBJSWITCH_FROZEN_FLAG;
                }
            }
            _ => {}
        }
    }

    /// `ObjSwitch_EyeOpeningInit`.
    fn eye_opening_init(&mut self) {
        self.action = Action::EyeOpening;
        self.cooldown_timer = COOLDOWN_FRAMES;
    }

    /// `ObjSwitch_EyeOpening`.
    fn eye_opening(&mut self, play: &mut PlayState) {
        if self.sub_type() != OBJSWITCH_SUBTYPE_TOGGLE || !self.cooldown_on || play.func_8005b198() == self.actor.category as i32 || self.cooldown_timer <= 0 {
            self.eye_tex_index -= 1;
            if self.eye_tex_index <= 0 {
                self.eye_open_init();
                audio_play_actor_sfx2(play, NA_SE_EV_FOOT_SWITCH);
            }
        }
    }

    /// `ObjSwitch_CrystalOffInit`: black, the red texture.
    fn crystal_off_init(&mut self) {
        self.crystal_color = [0, 0, 0];
        self.crystal_subtype1_texture = CrystalTex::Red;
        self.action = Action::CrystalOff;
    }

    /// `ObjSwitch_CrystalOff`.
    fn crystal_off(&mut self, play: &mut PlayState) {
        let hit = self.jnt_sph.base.ac_flags & AC_HIT != 0;
        match self.sub_type() {
            OBJSWITCH_SUBTYPE_ONCE => {
                if hit && self.disable_ac_timer <= 0 {
                    self.disable_ac_timer = DISABLE_AC_FRAMES;
                    self.set_on(play);
                    self.crystal_turn_on_init();
                }
            }
            OBJSWITCH_SUBTYPE_SYNC => {
                if (hit && self.disable_ac_timer <= 0) || play.flags.get_switch(self.switch_flag()) {
                    self.disable_ac_timer = DISABLE_AC_FRAMES;
                    self.set_on(play);
                    self.crystal_turn_on_init();
                }
            }
            OBJSWITCH_SUBTYPE_TOGGLE => {
                if hit && self.prev_col_flags & AC_HIT == 0 && self.disable_ac_timer <= 0 {
                    self.disable_ac_timer = DISABLE_AC_FRAMES;
                    self.set_on(play);
                    self.crystal_turn_on_init();
                }
                self.update_two_tex_scroll_xy();
            }
            _ => {}
        }
    }

    /// `ObjSwitch_CrystalTurnOnInit`.
    fn crystal_turn_on_init(&mut self) {
        self.action = Action::CrystalTurnOn;
        self.cooldown_timer = COOLDOWN_FRAMES;
    }

    /// `ObjSwitch_CrystalTurnOn`.
    fn crystal_turn_on(&mut self, play: &mut PlayState) {
        if !self.cooldown_on || play.func_8005b198() == self.actor.category as i32 || self.cooldown_timer <= 0 {
            self.crystal_on_init();
            if self.sub_type() == OBJSWITCH_SUBTYPE_TOGGLE {
                self.update_two_tex_scroll_xy();
            }
            audio_play_actor_sfx2(play, NA_SE_EV_DIAMOND_SWITCH);
        }
    }

    /// `ObjSwitch_CrystalOnInit`: white, the blue texture.
    fn crystal_on_init(&mut self) {
        self.crystal_color = [255, 255, 255];
        self.crystal_subtype1_texture = CrystalTex::Blue;
        self.action = Action::CrystalOn;
    }

    /// `ObjSwitch_CrystalOn`.
    fn crystal_on(&mut self, play: &mut PlayState) {
        match self.sub_type() {
            OBJSWITCH_SUBTYPE_ONCE | OBJSWITCH_SUBTYPE_SYNC => {
                if !play.flags.get_switch(self.switch_flag()) {
                    self.crystal_turn_off_init();
                }
            }
            OBJSWITCH_SUBTYPE_TOGGLE => {
                if self.jnt_sph.base.ac_flags & AC_HIT != 0 && self.prev_col_flags & AC_HIT == 0 && self.disable_ac_timer <= 0 {
                    self.disable_ac_timer = DISABLE_AC_FRAMES;
                    self.crystal_turn_off_init();
                    self.set_off(play);
                }
            }
            _ => {}
        }
        self.update_two_tex_scroll_xy();
    }

    /// `ObjSwitch_CrystalTurnOffInit`.
    fn crystal_turn_off_init(&mut self) {
        self.action = Action::CrystalTurnOff;
        self.cooldown_timer = COOLDOWN_FRAMES;
    }

    /// `ObjSwitch_CrystalTurnOff`.
    fn crystal_turn_off(&mut self, play: &mut PlayState) {
        if self.sub_type() != OBJSWITCH_SUBTYPE_TOGGLE || !self.cooldown_on || play.func_8005b198() == self.actor.category as i32 || self.cooldown_timer <= 0 {
            self.crystal_off_init();
            self.update_two_tex_scroll_xy();
            audio_play_actor_sfx2(play, NA_SE_EV_DIAMOND_SWITCH);
        }
    }

    /// `ObjSwitch_DrawFloor`: `Gfx_DrawDListOpa` of the subtype's list.
    fn draw_floor(&self, rs: &RenderState, out: &mut DrawOut) {
        // floorSwitchDLists[subType]: a subtype past 3 reads past the table in the C.
        if let Some(dl) = FLOOR_SWITCH_DLISTS.get(self.sub_type() as usize) {
            crate::gfx_draw_dlist_opa(out, OBJECT, dl, rs);
        }
    }

    /// `ObjSwitch_DrawFloorRusty`.
    fn draw_floor_rusty(&self, rs: &RenderState, out: &mut DrawOut) {
        crate::gfx_draw_dlist_opa(out, OBJECT, RUSTY_FLOOR_SWITCH_DL, rs);
    }

    /// `ObjSwitch_DrawEye`: `Gfx_SetupDL_25Opa`, the texture by `eyeTexIndex` on segment 8, the
    /// subtype's list (a bake per texture).
    fn draw_eye(&self, rs: &RenderState, out: &mut DrawOut) {
        let Some(texs) = EYE_TEXTURES.get(self.sub_type() as usize) else { return };
        let Some(tex) = texs.get(self.eye_tex_index as usize) else { return };
        out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(&eye_bake(tex))), actor_draw_matrix(rs)));
    }

    /// `ObjSwitch_DrawCrystal`: the translucent list, then `Gfx_SetupDL_25Opa`, the toggle's
    /// texture on segment 9, `gDPSetEnvColor(crystalColor, 128)`, the scroll on segment 8 and the
    /// opaque list. (A held subtype's lists are `NULL`: nothing here.)
    fn draw_crystal(&self, rs: &RenderState, out: &mut DrawOut) {
        let sub_type = self.sub_type() as usize;
        let m = actor_draw_matrix(rs);
        if let Some(Some(dl)) = XLU_DLISTS.get(sub_type) {
            out.xlu.push(DrawCmd::new(MeshKey::named(keys::mesh(OBJECT, dl)), m));
        }
        let Some(Some(dl)) = OPA_DLISTS.get(sub_type) else { return };
        let tex = (sub_type as i16 == OBJSWITCH_SUBTYPE_TOGGLE).then_some(self.crystal_subtype1_texture);
        let [r, g, b] = self.crystal_color;
        let mut sv = SegmentValues::default();
        sv.env[SEG_ENV as usize] = Some([r, g, b, 128]);
        sv.read(SEG_SCROLL, &crystal_scroll(self.x1_tex_scroll, self.y1_tex_scroll, self.x2_tex_scroll, self.y2_tex_scroll));
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&crystal_opa_bake(dl, tex))), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
    }
}

impl ActorImpl for ObjSwitch {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `ObjSwitch_Update`: the timers, the action, then by type: a floor keeps this frame's
    /// interact flags; an eye its AC flags, and registers its AC; a crystal counts its AC down
    /// (not while Player's in a cutscene), keeps its AC flags, and registers its AC (when the
    /// count is out) and OC. A floor's new height goes to `DynaPoly_UpdateContext`.
    fn update(&mut self, play: &mut PlayState) {
        if self.release_timer > 0 {
            self.release_timer -= 1;
        }
        if self.cooldown_timer > 0 {
            self.cooldown_timer -= 1;
        }
        match self.action {
            Action::FloorUp => self.floor_up(play),
            Action::FloorPress => self.floor_press(play),
            Action::FloorDown => self.floor_down(play),
            Action::FloorRelease => self.floor_release(play),
            Action::EyeInit => self.eye_init(play),
            Action::EyeOpen => self.eye_open(play),
            Action::EyeClosing => self.eye_closing(play),
            Action::EyeClosed => self.eye_closed(play),
            Action::EyeOpening => self.eye_opening(play),
            Action::CrystalOff => self.crystal_off(play),
            Action::CrystalTurnOn => self.crystal_turn_on(play),
            Action::CrystalOn => self.crystal_on(play),
            Action::CrystalTurnOff => self.crystal_turn_off(play),
        }
        match self.ty() {
            OBJSWITCH_TYPE_FLOOR | OBJSWITCH_TYPE_FLOOR_RUSTY => {
                self.prev_col_flags = self.interact_flags(play);
            }
            OBJSWITCH_TYPE_EYE => {
                self.prev_col_flags = self.tris.base.ac_flags;
                self.tris.base.ac_flags &= !AC_HIT;
                play.collision_check_set_ac(&self.actor, COLLIDER_TRIS, &mut self.tris);
            }
            OBJSWITCH_TYPE_CRYSTAL | OBJSWITCH_TYPE_CRYSTAL_TARGETABLE => {
                if !play.player_in_cs_mode() && self.disable_ac_timer > 0 {
                    self.disable_ac_timer -= 1;
                }
                self.prev_col_flags = self.jnt_sph.base.ac_flags;
                self.jnt_sph.base.ac_flags &= !AC_HIT;
                if self.disable_ac_timer <= 0 {
                    play.collision_check_set_ac(&self.actor, COLLIDER_JNT_SPH, &mut self.jnt_sph);
                }
                play.collision_check_set_oc(&self.actor, COLLIDER_JNT_SPH, &mut self.jnt_sph);
            }
            _ => {}
        }
        if self.bg != BG_ACTOR_MAX {
            play.col.dyna.set_source(self.bg, source(&self.actor));
        }
    }

    /// `ObjSwitch_Draw`: `sDrawFuncs[type]` (a type past 4 reads past the table in the C).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        match self.ty() {
            OBJSWITCH_TYPE_FLOOR => self.draw_floor(rs, out),
            OBJSWITCH_TYPE_FLOOR_RUSTY => self.draw_floor_rusty(rs, out),
            OBJSWITCH_TYPE_EYE => self.draw_eye(rs, out),
            OBJSWITCH_TYPE_CRYSTAL | OBJSWITCH_TYPE_CRYSTAL_TARGETABLE => self.draw_crystal(rs, out),
            _ => {}
        }
    }

    /// `ObjSwitch_Destroy`: the floors' bg actor (the colliders are the actor's own).
    fn destroy(&mut self, play: &mut PlayState) {
        if self.is_floor() {
            play.col.dyna.delete_bg_actor(self.bg);
        }
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            COLLIDER_TRIS => Some(ColliderMut::Tris(&mut self.tris)),
            COLLIDER_JNT_SPH => Some(ColliderMut::JntSph(&mut self.jnt_sph)),
            _ => None,
        }
    }

    /// The floors' `dyna.bgId` (`DynaPoly_UnsetAllInteractFlags` finds no other: they never
    /// call `DynaPoly_SetBgActor`).
    fn dyna_bg_id(&self) -> Option<u16> {
        (self.is_floor() && self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
