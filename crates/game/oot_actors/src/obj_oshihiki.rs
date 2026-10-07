//! `Obj_Oshihiki` (`ovl_Obj_Oshihiki/z_obj_oshihiki.c`): the push block, a DynaPoly actor of
//! `gameplay_dangeon_keep` (`gPushBlockCol`, `DYNA_TRANSFORM_POS`: it carries Link) drawn with
//! `gPushBlockDL`, its texture on segment 8 by size and its colour by scene.
//!
//! Params: bits 0..3 the type (`PushBlockType`: small, medium, large, huge, each "start on" or
//! "start off"), bits 6..7 the colour's column, bits 8..15 a switch flag (0..0x3F: with it set
//! the "start on" kinds aren't there, with it clear the "start off" ones).
//!
//! Player at its wall adds his push (2) or pull (-2) to `dyna.unk_150`, `unk_158` his yaw
//! (`func_8002DFA4`). Resting (`ObjOshihiki_OnScene`, `_OnActor`), with its 10-frame wait over,
//! strong enough (`Player_GetStrength` for the large and huge kinds) and no wall within the next
//! 20 along it (`ObjOshihiki_CheckWall`), it slides 20 that way (`ObjOshihiki_Push`: 0.5 faster a
//! frame up to 2, `NA_SE_EV_ROCK_SLIDE`), then waits 10, clearing Player's `PLAYER_STATE2_4` each
//! time it stops taking the push. With no floor under it any more (the highest of its bottom's
//! corners and middle, `BgCheck_EntityRaycastDown6`) it falls (`ObjOshihiki_Fall`, gravity -1) and
//! lands with `NA_SE_EV_BLOCK_BOUND` and the floor's footstep. A block resting on another moves
//! with it (`ObjOshihiki_MoveWithBlockUnder`, which the C runs from its draw: here in
//! `draw_update`, ADR 0037), and presses a switch it rests on (`DynaPolyActor_SetSwitchPressed`).
//!
//! The Master Quest Deku Tree's is room 3's, spawned by `Obj_Makeoshihiki` (params 0xFFC0: small,
//! colour 3, no flag). The whole overlay is ported; the cull zone (`cullingVolume*`) isn't, for
//! any actor.

use eng_collision::bgcheck::{BGCHECK_Y_MIN, PolyId};
use eng_collision::dyna::{BG_ACTOR_MAX, BGCHECK_SCENE, BgActorSource, DYNA_INTERACT_ACTOR_ON_TOP, DYNA_INTERACT_ACTOR_SWITCH_PRESSED, DYNA_TRANSFORM_POS};
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{cos_s, sin_s, step_to_f};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorHandle, ActorImpl, ActorProfile, audio_play_actor_sfx2, dyna_poly_get_actor};
use oot_game::audio::sfx::{NA_SE_PL_WALK_GROUND, SFX_FLAG};
use oot_game::collision_check::MASS_IMMOVABLE;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::player_lib::{PLAYER_STR_BRACELET, PLAYER_STR_SILVER_G, player_get_strength};
use oot_game::surface::SurfaceType;

/// `ACTOR_OBJ_OSHIHIKI` (`actor_table.h`: 0x00FF).
pub const ACTOR_OBJ_OSHIHIKI: i16 = 0x00FF;
pub const OBJECT: &str = "gameplay_dangeon_keep";
const COLLISION: &str = "gPushBlockCol";
const DLIST: &str = "gPushBlockDL";

/// `NA_SE_EV_ROCK_SLIDE`, `NA_SE_EV_BLOCK_BOUND` (`environmentbank_table.h`: 0x280A, 0x2835).
pub const NA_SE_EV_ROCK_SLIDE: u16 = 0x280A;
pub const NA_SE_EV_BLOCK_BOUND: u16 = 0x2835;

/// `ACTOR_OBJ_SWITCH` (`actor_table.h`: 0x012A).
const ACTOR_OBJ_SWITCH: i16 = 0x012A;
/// `PLAYER_STATE2_4` (Player's: pushing or pulling a block that takes it).
const PLAYER_STATE2_4: u32 = 1 << 4;

/// `Obj_Oshihiki_Profile`: `ACTOR_FLAG_UPDATE_CULLING_DISABLED`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_OBJ_OSHIHIKI, name: "Obj_Oshihiki", category: ACTORCAT_PROP, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT };

/// `PushBlockType` (`z_obj_oshihiki.h`).
pub const PUSHBLOCK_SMALL_START_ON: i16 = 0;
pub const PUSHBLOCK_MEDIUM_START_ON: i16 = 1;
pub const PUSHBLOCK_LARGE_START_ON: i16 = 2;
pub const PUSHBLOCK_HUGE_START_ON: i16 = 3;
pub const PUSHBLOCK_SMALL_START_OFF: i16 = 4;
pub const PUSHBLOCK_MEDIUM_START_OFF: i16 = 5;
pub const PUSHBLOCK_LARGE_START_OFF: i16 = 6;
pub const PUSHBLOCK_HUGE_START_OFF: i16 = 7;

/// `stateFlags` (`z_obj_oshihiki.h`).
pub const PUSHBLOCK_ON_SCENE: u16 = 1 << 0;
pub const PUSHBLOCK_SETUP_ON_SCENE: u16 = 1 << 1;
pub const PUSHBLOCK_ON_ACTOR: u16 = 1 << 2;
pub const PUSHBLOCK_SETUP_ON_ACTOR: u16 = 1 << 3;
pub const PUSHBLOCK_PUSH: u16 = 1 << 4;
pub const PUSHBLOCK_SETUP_PUSH: u16 = 1 << 5;
pub const PUSHBLOCK_FALL: u16 = 1 << 6;
pub const PUSHBLOCK_SETUP_FALL: u16 = 1 << 7;
pub const PUSHBLOCK_MOVE_UNDER: u16 = 1 << 8;

/// `sScales`, by type.
pub const SCALES: [f32; 8] = [1.0 / 10.0, 1.0 / 6.0, 1.0 / 5.0, 1.0 / 3.0, 1.0 / 10.0, 1.0 / 6.0, 1.0 / 5.0, 1.0 / 3.0];

/// `sColors`, by scene (`sSceneIds`' order) and params bits 6..7.
pub const COLORS: [[[u8; 3]; 4]; 9] = [
    [[110, 86, 40], [110, 86, 40], [110, 86, 40], [110, 86, 40]],         // deku tree
    [[106, 120, 110], [104, 80, 20], [0, 0, 0], [0, 0, 0]],               // dodongos cavern
    [[142, 99, 86], [72, 118, 96], [0, 0, 0], [0, 0, 0]],                 // forest temple
    [[210, 150, 80], [210, 170, 80], [0, 0, 0], [0, 0, 0]],               // fire temple
    [[102, 144, 182], [176, 167, 100], [100, 167, 100], [117, 97, 96]],   // water temple
    [[232, 210, 176], [232, 210, 176], [232, 210, 176], [232, 210, 176]], // spirit temple
    [[135, 125, 95], [135, 125, 95], [135, 125, 95], [135, 125, 95]],     // shadow temple
    [[255, 255, 255], [255, 255, 255], [255, 255, 255], [255, 255, 255]], // ganons castle
    [[232, 210, 176], [232, 210, 176], [232, 210, 176], [232, 210, 176]], // gerudo training grounds
];

/// `sSceneIds` (`scene_table.h`): the Deku Tree, Dodongo's Cavern, the Forest, Fire, Water,
/// Spirit and Shadow Temples, Ganon's Tower, the Gerudo Training Ground.
pub const SCENE_IDS: [u16; 9] = [0x00, 0x01, 0x03, 0x04, 0x05, 0x06, 0x07, 0x0A, 0x0B];

/// `sColCheckPoints`: the bottom face's corners and middle, 1.01 up.
pub const COL_CHECK_POINTS: [Vec3; 5] = [Vec3::new(29.99, 1.01, -29.99), Vec3::new(-29.99, 1.01, -29.99), Vec3::new(-29.99, 1.01, 29.99), Vec3::new(29.99, 1.01, 29.99), Vec3::new(0.0, 1.01, 0.0)];

/// `sFaceVtx`, `sFaceDirection`: the four corners of the block's middle cross-section across
/// the push, each pulled 1 in.
pub const FACE_VTX: [(f32, f32); 4] = [(-30.0, 0.0), (30.0, 0.0), (-30.0, 60.0), (30.0, 60.0)];
pub const FACE_DIRECTION: [(f32, f32); 4] = [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)];

/// The texture `ObjOshihiki_SetTexture` binds on segment 8.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Texture {
    /// `gPushBlockSilverTex`: the small and medium kinds.
    Silver,
    /// `gPushBlockBaseTex`: the large.
    Base,
    /// `gPushBlockGrayTex`: the huge.
    Gray,
}

impl Texture {
    pub fn symbol(self) -> &'static str {
        match self {
            Texture::Silver => "gPushBlockSilverTex",
            Texture::Base => "gPushBlockBaseTex",
            Texture::Gray => "gPushBlockGrayTex",
        }
    }
}

/// The segment the draw's `gDPSetEnvColor` is baked on (a free one; the colour is dynamic).
const SEG_ENV: u8 = 0x0B;
/// `gSPSegment(POLY_OPA_DISP++, 0x08, SEGMENTED_TO_VIRTUAL(this->texture))`.
const SEG_TEX: u8 = 0x08;

/// The bake of `gPushBlockDL` with `tex` on segment 8 and the env colour dynamic.
pub fn bake_name(tex: Texture) -> String {
    format!("Obj_Oshihiki/{}", tex.symbol())
}

/// The draw's meshes: `gPushBlockDL` with each texture, the env colour dynamic.
pub fn bakes() -> Vec<MeshBake> {
    [Texture::Silver, Texture::Base, Texture::Gray]
        .into_iter()
        .map(|t| MeshBake {
            name: bake_name(t),
            object: OBJECT.into(),
            segments: vec![(SEG_ENV, BakeSegment::DynamicColor { env: true, prim: false }), (SEG_TEX, BakeSegment::Texture { file: OBJECT.into(), symbol: t.symbol().into() })],
            prelude: vec![SEG_ENV],
            body: BakeBody::DLists(vec![(OBJECT.into(), DLIST.into())]),
        })
        .collect()
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `ObjOshihiki_OnScene`: resting on the scene's floor.
    OnScene,
    /// `ObjOshihiki_OnActor`: on a DynaPoly actor's (or about to land).
    OnActor,
    /// `ObjOshihiki_Push`.
    Push,
    /// `ObjOshihiki_Fall`.
    Fall,
}

pub struct ObjOshihiki {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    pub action: Option<Action>,
    pub state_flags: u16,
    pub timer: i16,
    pub yaw_sin: f32,
    pub yaw_cos: f32,
    pub push_speed: f32,
    pub push_dist: f32,
    pub direction: f32,
    pub floor_bg_ids: [u16; 5],
    pub floor_polys: [Option<PolyId>; 5],
    pub floor_heights: [f32; 5],
    pub highest_floor: i16,
    pub cant_move: bool,
    /// `blockUnder`.
    pub block_under: Option<ActorHandle>,
    pub under_dist_x: f32,
    pub under_dist_z: f32,
    pub texture: Option<Texture>,
    pub color: [u8; 3],
    /// Whether this frame's draw moved it with the block under it (the draw's
    /// `Matrix_Translate`).
    pub moved_with_under: bool,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

/// `ObjOshihiki_RotateXZ`.
pub fn rotate_xz(input: Vec3, sn: f32, cs: f32) -> Vec3 {
    Vec3::new(input.z * sn + input.x * cs, input.y, input.z * cs - input.x * sn)
}

/// Clears Player's `PLAYER_STATE2_4` (`player->stateFlags2 &= ~PLAYER_STATE2_4`).
fn clear_player_state2_4(play: &mut PlayState) {
    if let Some(p) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
        p.change_state_flags2(0, PLAYER_STATE2_4);
    }
}

impl ObjOshihiki {
    /// `PARAMS_GET_U(params, 0, 4)`: the type.
    pub fn block_type(&self) -> i16 {
        (self.actor.params as u16 & 0xF) as i16
    }

    /// `PARAMS_GET_U(params, 8, 6)`: the switch flag.
    fn switch_flag(&self) -> i32 {
        ((self.actor.params as u16 >> 8) & 0x3F) as i32
    }

    /// `ObjOshihiki_Init`: its collision for a known type (`ObjOshihiki_CheckType`); gone for a
    /// "start on" kind with its flag set or a "start off" one with it clear; the type's scale
    /// and texture, immovable, the scene's colour, the floors reset, resting on an actor
    /// (`ObjOshihiki_SetupOnActor`: falling with gravity -1 until it finds a floor).
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = ObjOshihiki {
            actor,
            // @bug (game): an unknown type leaves `dyna.bgId` as the zeroed actor's 0 (another
            // bg actor's); no placement in the game has one.
            bg: 0,
            action: None,
            state_flags: 0,
            timer: 0,
            yaw_sin: 0.0,
            yaw_cos: 0.0,
            push_speed: 0.0,
            push_dist: 0.0,
            direction: 0.0,
            floor_bg_ids: [BGCHECK_SCENE; 5],
            floor_polys: [None; 5],
            floor_heights: [0.0; 5],
            highest_floor: 0,
            cant_move: false,
            block_under: None,
            under_dist_x: 0.0,
            under_dist_z: 0.0,
            texture: None,
            color: [0; 3],
            moved_with_under: false,
        };
        this.check_type(play);
        let flag_param = (this.actor.params as u16 >> 8) & 0xFF;
        if flag_param <= 0x3F {
            if play.flags.get_switch(this.switch_flag()) {
                if matches!(this.block_type(), PUSHBLOCK_SMALL_START_ON | PUSHBLOCK_MEDIUM_START_ON | PUSHBLOCK_LARGE_START_ON | PUSHBLOCK_HUGE_START_ON) {
                    this.actor.kill();
                    return Box::new(this);
                }
            } else if matches!(this.block_type(), PUSHBLOCK_SMALL_START_OFF | PUSHBLOCK_MEDIUM_START_OFF | PUSHBLOCK_LARGE_START_OFF | PUSHBLOCK_HUGE_START_OFF) {
                this.actor.kill();
                return Box::new(this);
            }
        }
        this.set_scale();
        this.set_texture();
        // sInitChain: cullingVolumeDistance 1800, cullingVolumeScale 500, cullingVolumeDownward
        // 1500 (the cull zone isn't ported).
        this.actor.col_chk_info.mass = MASS_IMMOVABLE;
        this.set_color(play);
        this.reset_floors();
        this.setup_on_actor();
        log::debug!("(dungeon keep push/pull block)(arg_data 0x{:04x})", this.actor.params);
        play.col.dyna.set_source(this.bg, source(&this.actor));
        Box::new(this)
    }

    /// `ObjOshihiki_CheckType`: `ObjOshihiki_InitDynapoly(gPushBlockCol, DYNA_TRANSFORM_POS)` for
    /// the eight types.
    fn check_type(&mut self, play: &mut PlayState) {
        if (0..8).contains(&self.block_type()) {
            // ObjOshihiki_InitDynapoly: DynaPolyActor_Init(moveFlag), the collision.
            self.bg = match crate::obj_kibako2::load_collision(play, OBJECT, COLLISION) {
                Some(h) => play.col.dyna.set_bg_actor(h, source(&self.actor), DYNA_TRANSFORM_POS),
                None => BG_ACTOR_MAX,
            };
            if self.bg == BG_ACTOR_MAX {
                log::warn!("Warning : move BG registration failed (../z_obj_oshihiki.c 280)(name {})(arg_data 0x{:04x})", self.actor.id, self.actor.params);
            }
        } else {
            log::error!("Error : type cannot be determined (../z_obj_oshihiki.c 444)(arg_data 0x{:04x})", self.actor.params);
        }
    }

    /// `ObjOshihiki_SetScale`: `sScales[type]`. `@bug (game)`: types 8 to 15 read past the table
    /// into `sColors`' bytes as a float (no placement has one).
    fn set_scale(&mut self) {
        let t = self.block_type() as usize;
        let s = match SCALES.get(t) {
            Some(&s) => s,
            None => {
                let bytes: Vec<u8> = COLORS.iter().flatten().flatten().copied().collect();
                let o = (t - SCALES.len()) * 4;
                f32::from_be_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]])
            }
        };
        self.actor.scale = Vec3::splat(s);
    }

    /// `ObjOshihiki_SetTexture`: silver for the small and medium kinds, the base texture for the
    /// large, grey for the huge (none for an unknown type).
    fn set_texture(&mut self) {
        self.texture = match self.block_type() {
            PUSHBLOCK_SMALL_START_ON | PUSHBLOCK_MEDIUM_START_ON | PUSHBLOCK_SMALL_START_OFF | PUSHBLOCK_MEDIUM_START_OFF => Some(Texture::Silver),
            PUSHBLOCK_LARGE_START_ON | PUSHBLOCK_LARGE_START_OFF => Some(Texture::Base),
            PUSHBLOCK_HUGE_START_ON | PUSHBLOCK_HUGE_START_OFF => Some(Texture::Gray),
            _ => None,
        };
    }

    /// `ObjOshihiki_SetColor`: `sColors[scene][params bits 6..7]`, white in a scene not listed.
    fn set_color(&mut self, play: &PlayState) {
        let idx = ((self.actor.params as u16 >> 6) & 3) as usize;
        match SCENE_IDS.iter().position(|&s| s == play.scene_id) {
            Some(i) => self.color = COLORS[i][idx],
            None => {
                log::error!("Error : scene_data_ID cannot be determined. (../z_obj_oshihiki.c 579)");
                self.color = [255; 3];
            }
        }
    }

    /// `ObjOshihiki_StrongEnough`: not when it can't move (`Obj_Makeoshihiki`'s); the small and
    /// medium always, the large with the bracelet, the huge with the silver gauntlets.
    fn strong_enough(&self, play: &PlayState) -> bool {
        if self.cant_move {
            return false;
        }
        let strength = player_get_strength(&play.save);
        match self.block_type() {
            PUSHBLOCK_SMALL_START_ON | PUSHBLOCK_MEDIUM_START_ON | PUSHBLOCK_SMALL_START_OFF | PUSHBLOCK_MEDIUM_START_OFF => true,
            PUSHBLOCK_LARGE_START_ON | PUSHBLOCK_LARGE_START_OFF => strength >= PLAYER_STR_BRACELET,
            PUSHBLOCK_HUGE_START_ON | PUSHBLOCK_HUGE_START_OFF => strength >= PLAYER_STR_SILVER_G,
            _ => false,
        }
    }

    /// `ObjOshihiki_ResetFloors`.
    fn reset_floors(&mut self) {
        self.floor_bg_ids = [BGCHECK_SCENE; 5];
    }

    /// `ObjOshihiki_GetBlockUnder`: the push block it rests on (its highest floor a push block's,
    /// level with it).
    fn get_block_under(&self, play: &PlayState) -> Option<ActorHandle> {
        let bg = self.floor_bg_ids[self.highest_floor as usize];
        if bg != BGCHECK_SCENE && (self.actor.floor_height - self.actor.world_pos.y).abs() < 0.001 {
            let h = dyna_poly_get_actor(&play.actors, &play.col, bg)?;
            if play.actors.actor(h).is_some_and(|a| a.id == ACTOR_OBJ_OSHIHIKI) {
                return Some(h);
            }
        }
        None
    }

    /// `ObjOshihiki_UpdateInitPos`: `home` stepped by 20s towards where it is now, in x and z.
    fn update_init_pos(&mut self) {
        let a = &mut self.actor;
        if a.home_pos.x < a.world_pos.x {
            while (a.world_pos.x - a.home_pos.x) >= 20.0 {
                a.home_pos.x += 20.0;
            }
        } else {
            while (a.home_pos.x - a.world_pos.x) >= 20.0 {
                a.home_pos.x -= 20.0;
            }
        }
        if a.home_pos.z < a.world_pos.z {
            while (a.world_pos.z - a.home_pos.z) >= 20.0 {
                a.home_pos.z += 20.0;
            }
        } else {
            while (a.home_pos.z - a.world_pos.z) >= 20.0 {
                a.home_pos.z -= 20.0;
            }
        }
    }

    /// `ObjOshihiki_NoSwitchPress`: false on a blue floor switch already pressed for its own flag
    /// (an inverse one already up), which it keeps down.
    fn no_switch_press(&self, dyna: Option<ActorHandle>, play: &PlayState) -> bool {
        let Some(d) = dyna.and_then(|h| play.actors.actor(h)) else { return true };
        if d.id == ACTOR_OBJ_SWITCH {
            let dyna_switch_flag = ((d.params as u16 >> 8) & 0x3F) as i32;
            match d.params & 0x33 {
                // Normal blue switch.
                0x20 => {
                    if dyna_switch_flag == self.switch_flag() && play.flags.get_switch(dyna_switch_flag) {
                        return false;
                    }
                }
                // Inverse blue switch.
                0x30 => {
                    if dyna_switch_flag == self.switch_flag() && !play.flags.get_switch(dyna_switch_flag) {
                        return false;
                    }
                }
                _ => {}
            }
        }
        true
    }

    /// `ObjOshihiki_SetFloors`: the floor under each of the bottom's corners and middle (scaled,
    /// turned along the push, at last frame's height: `prevPos.y`), its own collision skipped
    /// (`BgCheck_EntityRaycastDown6`, `chkDist` 0).
    fn set_floors(&mut self, play: &PlayState) {
        for i in 0..5 {
            let p = COL_CHECK_POINTS[i];
            let off = Vec3::new(p.x * (self.actor.scale.x * 10.0), p.y * (self.actor.scale.y * 10.0), p.z * (self.actor.scale.z * 10.0));
            let mut pt = rotate_xz(off, self.yaw_sin, self.yaw_cos);
            pt.x += self.actor.world_pos.x;
            pt.y += self.actor.prev_pos.y;
            pt.z += self.actor.world_pos.z;
            let (y, poly) = play.col.entity_raycast_down6(pt, self.bg, 0.0);
            self.floor_heights[i] = y;
            self.floor_polys[i] = poly;
            self.floor_bg_ids[i] = poly.map_or(BGCHECK_SCENE, |p| p.bg);
        }
    }

    /// `ObjOshihiki_GetHighestFloor`: the highest; one on the scene's ties win over an actor's.
    fn get_highest_floor(&self) -> i16 {
        let mut highest = 0usize;
        for i in 1..self.floor_heights.len() {
            if self.floor_heights[i] > self.floor_heights[highest] {
                highest = i;
            } else if self.floor_bg_ids[i] == BGCHECK_SCENE && (self.floor_heights[i] - self.floor_heights[highest]) > -0.001 {
                highest = i;
            }
        }
        highest as i16
    }

    /// `ObjOshihiki_SetGround`.
    fn set_ground(&mut self, play: &PlayState) {
        self.reset_floors();
        self.set_floors(play);
        self.highest_floor = self.get_highest_floor();
        self.actor.floor_height = self.floor_heights[self.highest_floor as usize];
    }

    /// `ObjOshihiki_CheckFloor`: on (or under) its floor: set on it.
    fn check_floor(&mut self, play: &PlayState) -> bool {
        self.set_ground(play);
        if (self.actor.floor_height - self.actor.world_pos.y) >= -0.001 {
            self.actor.world_pos.y = self.actor.floor_height;
            return true;
        }
        false
    }

    /// `ObjOshihiki_CheckGround`: fallen out of the world, it's gone; else `CheckFloor`'s test
    /// on the floor `SetGround` found.
    fn check_ground(&mut self) -> bool {
        if self.actor.world_pos.y <= BGCHECK_Y_MIN + 10.0 {
            log::warn!("Warning : Push/pull block fell too much (../z_obj_oshihiki.c 809)(arg_data 0x{:04x})", self.actor.params);
            self.actor.kill();
            return false;
        }
        if (self.actor.floor_height - self.actor.world_pos.y) >= -0.001 {
            self.actor.world_pos.y = self.actor.floor_height;
            return true;
        }
        false
    }

    /// `ObjOshihiki_CheckWall`: a wall within the next 20 (less 0.5) along `angle` past its face,
    /// on the side of `direction`'s sign: four lines from its middle cross-section's corners
    /// (pulled 1 in), its own collision skipped (`BgCheck_EntityLineTest3`).
    fn check_wall(&self, play: &PlayState, angle: i16, direction: f32) -> bool {
        let sign = if direction >= 0.0 { 1.0 } else { -1.0 };
        let max_dist = sign * (300.0 * self.actor.scale.x + 20.0 - 0.5);
        let sn = sin_s(angle);
        let cs = cos_s(angle);
        for i in 0..4 {
            let off = Vec3::new(FACE_VTX[i].0 * self.actor.scale.x * 10.0 + FACE_DIRECTION[i].0, FACE_VTX[i].1 * self.actor.scale.y * 10.0 + FACE_DIRECTION[i].1, 0.0);
            let face_vtx = rotate_xz(off, sn, cs) + self.actor.world_pos;
            let face_vtx_next = Vec3::new(face_vtx.x + max_dist * sn, face_vtx.y, face_vtx.z + max_dist * cs);
            if play.col.entity_line_test3(face_vtx, face_vtx_next, true, false, false, true, self.bg, 0.0).is_some() {
                return true;
            }
        }
        false
    }

    /// `ObjOshihiki_MoveWithBlockUnder` (from the draw): a block under it that's taking a push
    /// with no wall in its way becomes `blockUnder`; while `blockUnder` slides, it moves along
    /// by the same, its `home` stepped after it. Clears `blockUnder` once that stops.
    fn move_with_block_under(&mut self, play: &PlayState) -> bool {
        if let Some(bu) = self.get_block_under(play)
            && let Some(b) = play.actors.downcast::<ObjOshihiki>(bu)
            && b.state_flags & PUSHBLOCK_SETUP_PUSH != 0
            && !self.check_wall(play, play.col.dyna.unk_158(b.bg), b.direction)
        {
            self.block_under = Some(bu);
        }
        if self.state_flags & PUSHBLOCK_MOVE_UNDER != 0
            && let Some(bu) = self.block_under
        {
            match play.actors.downcast::<ObjOshihiki>(bu) {
                Some(b) if b.state_flags & PUSHBLOCK_PUSH != 0 => {
                    self.under_dist_x = b.actor.world_pos.x - b.actor.prev_pos.x;
                    self.under_dist_z = b.actor.world_pos.z - b.actor.prev_pos.z;
                    self.actor.world_pos.x += self.under_dist_x;
                    self.actor.world_pos.z += self.under_dist_z;
                    self.update_init_pos();
                    return true;
                }
                Some(b) if b.state_flags & PUSHBLOCK_SETUP_PUSH != 0 => {}
                // Not pushed any more (or gone: the C would read freed memory).
                _ => self.block_under = None,
            }
        }
        false
    }

    /// `ObjOshihiki_SetupOnScene`: still, no gravity.
    fn setup_on_scene(&mut self) {
        self.state_flags |= PUSHBLOCK_SETUP_ON_SCENE;
        self.actor.gravity = 0.0;
        self.actor.velocity = Vec3::ZERO;
        self.action = Some(Action::OnScene);
    }

    /// `ObjOshihiki_OnScene`: with its wait over and a push or pull on it (`dyna.unk_150`):
    /// strong enough and no wall that way, it's pushed (`direction` the push's sign); otherwise
    /// the push isn't taken (Player's `PLAYER_STATE2_4` cleared, `unk_150` zeroed).
    fn on_scene(&mut self, play: &mut PlayState) {
        self.state_flags |= PUSHBLOCK_ON_SCENE;
        let unk_150 = play.col.dyna.unk_150(self.bg);
        if self.timer <= 0 && unk_150.abs() > 0.001 {
            if self.strong_enough(play) && !self.check_wall(play, play.col.dyna.unk_158(self.bg), unk_150) {
                self.direction = unk_150;
                self.setup_push();
            } else {
                clear_player_state2_4(play);
                play.col.dyna.set_unk_150(self.bg, 0.0);
            }
        } else {
            clear_player_state2_4(play);
            play.col.dyna.set_unk_150(self.bg, 0.0);
        }
    }

    /// `ObjOshihiki_SetupOnActor`: still, gravity -1.
    fn setup_on_actor(&mut self) {
        self.state_flags |= PUSHBLOCK_SETUP_ON_ACTOR;
        self.actor.velocity = Vec3::ZERO;
        self.actor.gravity = -1.0;
        self.action = Some(Action::OnActor);
    }

    /// `ObjOshihiki_OnActor`: moving under gravity: on its floor, the scene's makes it rest there;
    /// a DynaPoly actor's has it on top and pressing it, and takes a push as `OnScene` does (not
    /// off a blue switch pressed for its own flag). Off its floor: falling, unless the floor's
    /// actor moves (`DYNA_TRANSFORM_POS`), which carries it.
    fn on_actor(&mut self, play: &mut PlayState) {
        self.state_flags |= PUSHBLOCK_ON_ACTOR;
        self.actor.move_forward();
        if self.check_floor(play) {
            let bg_id = self.floor_bg_ids[self.highest_floor as usize];
            if bg_id == BGCHECK_SCENE {
                self.setup_on_scene();
            } else {
                match dyna_poly_get_actor(&play.actors, &play.col, bg_id) {
                    Some(d) => {
                        // DynaPolyActor_SetActorOnTop, DynaPolyActor_SetSwitchPressed.
                        play.col.dyna.set_interact_flag(bg_id, DYNA_INTERACT_ACTOR_ON_TOP);
                        play.col.dyna.set_interact_flag(bg_id, DYNA_INTERACT_ACTOR_SWITCH_PRESSED);
                        let unk_150 = play.col.dyna.unk_150(self.bg);
                        if self.timer <= 0 && unk_150.abs() > 0.001 {
                            if self.strong_enough(play) && self.no_switch_press(Some(d), play) && !self.check_wall(play, play.col.dyna.unk_158(self.bg), unk_150) {
                                self.direction = unk_150;
                                self.setup_push();
                            } else {
                                clear_player_state2_4(play);
                                play.col.dyna.set_unk_150(self.bg, 0.0);
                            }
                        } else {
                            clear_player_state2_4(play);
                            play.col.dyna.set_unk_150(self.bg, 0.0);
                        }
                    }
                    None => self.setup_on_scene(),
                }
            }
        } else {
            let bg_id = self.floor_bg_ids[self.highest_floor as usize];
            if bg_id == BGCHECK_SCENE {
                self.setup_fall(play);
            } else {
                let moving = dyna_poly_get_actor(&play.actors, &play.col, bg_id).is_some() && play.col.dyna.actors.get(bg_id as usize).is_some_and(|a| a.move_flags & DYNA_TRANSFORM_POS != 0);
                if moving {
                    play.col.dyna.set_interact_flag(bg_id, DYNA_INTERACT_ACTOR_ON_TOP);
                    play.col.dyna.set_interact_flag(bg_id, DYNA_INTERACT_ACTOR_SWITCH_PRESSED);
                    self.actor.world_pos.y = self.actor.floor_height;
                } else {
                    self.setup_fall(play);
                }
            }
        }
    }

    /// `ObjOshihiki_SetupPush`: no gravity.
    fn setup_push(&mut self) {
        self.state_flags |= PUSHBLOCK_SETUP_PUSH;
        self.action = Some(Action::Push);
        self.actor.gravity = 0.0;
    }

    /// `ObjOshihiki_Push`: 0.5 faster a frame (up to 2) towards 20 from `home` along the push
    /// (`direction`'s sign); off its floor on the way, it falls from there; at 20,
    /// `NA_SE_EV_BLOCK_BOUND` against a wall ahead, `home` here, a 10-frame wait, at rest. Each
    /// stop clears Player's `PLAYER_STATE2_4` and the push. `NA_SE_EV_ROCK_SLIDE` every frame.
    fn push(&mut self, play: &mut PlayState) {
        self.push_speed += 0.5;
        self.state_flags |= PUSHBLOCK_PUSH;
        self.push_speed = self.push_speed.min(2.0);
        let stop_flag = step_to_f(&mut self.push_dist, 20.0, self.push_speed);
        let push_dist_signed = if self.direction >= 0.0 { 1.0 } else { -1.0 } * self.push_dist;
        self.actor.world_pos.x = self.actor.home_pos.x + push_dist_signed * self.yaw_sin;
        self.actor.world_pos.z = self.actor.home_pos.z + push_dist_signed * self.yaw_cos;
        if !self.check_floor(play) {
            self.actor.home_pos.x = self.actor.world_pos.x;
            self.actor.home_pos.z = self.actor.world_pos.z;
            clear_player_state2_4(play);
            play.col.dyna.set_unk_150(self.bg, 0.0);
            self.push_dist = 0.0;
            self.push_speed = 0.0;
            self.setup_fall(play);
        } else if stop_flag {
            if self.check_wall(play, play.col.dyna.unk_158(self.bg), play.col.dyna.unk_150(self.bg)) {
                audio_play_actor_sfx2(play, NA_SE_EV_BLOCK_BOUND);
            }
            self.actor.home_pos.x = self.actor.world_pos.x;
            self.actor.home_pos.z = self.actor.world_pos.z;
            clear_player_state2_4(play);
            play.col.dyna.set_unk_150(self.bg, 0.0);
            self.push_dist = 0.0;
            self.push_speed = 0.0;
            self.timer = 10;
            if self.floor_bg_ids[self.highest_floor as usize] == BGCHECK_SCENE {
                self.setup_on_scene();
            } else {
                self.setup_on_actor();
            }
        }
        audio_play_actor_sfx2(play, NA_SE_EV_ROCK_SLIDE - SFX_FLAG);
    }

    /// `ObjOshihiki_SetupFall`: still, gravity -1, the floor under it found.
    fn setup_fall(&mut self, play: &PlayState) {
        self.state_flags |= PUSHBLOCK_SETUP_FALL;
        self.actor.velocity = Vec3::ZERO;
        self.actor.gravity = -1.0;
        self.set_ground(play);
        self.action = Some(Action::Fall);
    }

    /// `ObjOshihiki_Fall`: a push on it now is dropped (and Player's `PLAYER_STATE2_4`); falling to
    /// the floor `SetupFall` found, it lands at rest with `NA_SE_EV_BLOCK_BOUND` and the floor's
    /// footstep (`NA_SE_PL_WALK_GROUND` + `SurfaceType_GetSfxOffset`).
    fn fall(&mut self, play: &mut PlayState) {
        self.state_flags |= PUSHBLOCK_FALL;
        if play.col.dyna.unk_150(self.bg).abs() > 0.001 {
            play.col.dyna.set_unk_150(self.bg, 0.0);
            clear_player_state2_4(play);
        }
        self.actor.move_forward();
        if self.check_ground() {
            if self.floor_bg_ids[self.highest_floor as usize] == BGCHECK_SCENE {
                self.setup_on_scene();
            } else {
                self.setup_on_actor();
            }
            audio_play_actor_sfx2(play, NA_SE_EV_BLOCK_BOUND);
            let floor = self.floor_polys[self.highest_floor as usize];
            let offset = play.audio.tables.surface_sfx_id(floor.map(|p| play.col.sfx_type(p)).unwrap_or(0));
            audio_play_actor_sfx2(play, NA_SE_PL_WALK_GROUND.wrapping_add(offset));
        }
    }
}

impl ActorImpl for ObjOshihiki {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `ObjOshihiki_Update`: the frame's state flags cleared (`PUSHBLOCK_MOVE_UNDER` set for the
    /// draw), the wait down, `world.rot.y` the push's yaw (`dyna.unk_158`) and its sine and
    /// cosine, then the action; the new transform for `DynaPoly_UpdateContext`.
    fn update(&mut self, play: &mut PlayState) {
        self.state_flags &=
            !(PUSHBLOCK_SETUP_FALL | PUSHBLOCK_FALL | PUSHBLOCK_SETUP_PUSH | PUSHBLOCK_PUSH | PUSHBLOCK_SETUP_ON_ACTOR | PUSHBLOCK_ON_ACTOR | PUSHBLOCK_SETUP_ON_SCENE | PUSHBLOCK_ON_SCENE);
        self.state_flags |= PUSHBLOCK_MOVE_UNDER;
        if self.timer > 0 {
            self.timer -= 1;
        }
        self.actor.world_rot.y = play.col.dyna.unk_158(self.bg);
        self.yaw_sin = sin_s(self.actor.world_rot.y);
        self.yaw_cos = cos_s(self.actor.world_rot.y);
        match self.action {
            Some(Action::OnScene) => self.on_scene(play),
            Some(Action::OnActor) => self.on_actor(play),
            Some(Action::Push) => self.push(play),
            Some(Action::Fall) => self.fall(play),
            None => {}
        }
        play.col.dyna.set_source(self.bg, source(&self.actor));
    }

    /// `ObjOshihiki_Draw`'s state: `ObjOshihiki_MoveWithBlockUnder` (the block moved with the
    /// one under it), `PUSHBLOCK_MOVE_UNDER` cleared; the new transform for the next frame's
    /// `DynaPoly_UpdateContext`.
    fn draw_update(&mut self, play: &mut PlayState) {
        self.moved_with_under = self.move_with_block_under(play);
        self.state_flags &= !PUSHBLOCK_MOVE_UNDER;
        if self.moved_with_under {
            play.col.dyna.set_source(self.bg, source(&self.actor));
        }
    }

    /// `ObjOshihiki_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }

    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.values = vec![if self.moved_with_under { self.under_dist_x } else { 0.0 }, if self.moved_with_under { self.under_dist_z } else { 0.0 }];
        rs.switches = vec![self.texture.map_or(0, |t| t as u32 + 1)];
        rs
    }

    /// `ObjOshihiki_Draw`: moved with the block under it, `Matrix_Translate(underDist × 10)` in
    /// its model space; `gPushBlockDL` with its texture on segment 8 and its colour as the env
    /// colour (the debug ROM's `mREG(13..15)` outside the scenes it lists: Ganon's Tower isn't
    /// one, `@bug (game)`; those regs are 0).
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let tex = match rs.switches.first() {
            Some(1) => Texture::Silver,
            Some(2) => Texture::Base,
            Some(3) => Texture::Gray,
            _ => return,
        };
        // Actor_Draw's matrix is from where the block was before `draw_update` moved it (the
        // state captured here is after): back there, then the draw's own translate (the same
        // place for the small block's scale of 0.1).
        let (dx, dz) = (rs.values.first().copied().unwrap_or(0.0), rs.values.get(1).copied().unwrap_or(0.0));
        let m = if dx != 0.0 || dz != 0.0 {
            let mut before = rs.clone();
            before.pos -= Vec3::new(dx, 0.0, dz);
            actor_draw_matrix(&before) * Mat4::from_translation(Vec3::new(dx * 10.0, 0.0, dz * 10.0))
        } else {
            actor_draw_matrix(rs)
        };
        // DEBUG_FEATURES: the colour in these scenes, else mREG(13), mREG(14), mREG(15).
        let [r, g, b] = match play.scene_id {
            0x00 | 0x01 | 0x03 | 0x04 | 0x05 | 0x06 | 0x07 | 0x0B => self.color,
            _ => [0, 0, 0],
        };
        let mut sv = SegmentValues::default();
        sv.env[SEG_ENV as usize] = Some([r, g, b, 255]);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(tex))), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
