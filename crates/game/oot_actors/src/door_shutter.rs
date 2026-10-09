//! `Door_Shutter` (`ovl_Door_Shutter/z_door_shutter.c`): the dungeons' sliding doors, Phantom
//! Ganon's bars and the stone slab that shuts Gohma's room. A transition actor (it spawns from
//! the scene's transition-actor list, with its index in the params' top bits).
//!
//! Params: the type in bits 6..9 (`DoorShutterType`), a switch flag in bits 0..5. The type and
//! the side Link stands on decide how the door behaves (`DoorShutter_SetupDoor`, run again each
//! time the door closes): plain, barred until the room is cleared or a switch flag is set,
//! unopenable from behind, locked (a small key, the boss key). Seen from its "front" room (the
//! one it's in), a door barred from the back is a plain door.
//!
//! The style (which object and display lists) comes from the scene (`sSceneInfo`: the Deku
//! Tree's `object_ydan_objects`) or the type (the boss door, Phantom Ganon's bars, Gohma's
//! slab). The door waits for its object, then for Player: standing within 50 through it,
//! inside its sides, and facing it, Player can open it (`doorType` `PLAYER_DOORTYPE_SLIDING`).
//! Player's A press (`Player_ActionHandler_1`) sets `isActive`, and the door slides up 200,
//! changes the camera (`Camera_ChangeDoorCam`), and slides down once Link is past it: the old
//! room goes (`Room_FinishRoomChange`), and a door barred on Link's side slams shut faster and
//! holds him a moment (`Player_SetCsActionWithHaltedActors` 2, then 7).
//!
//! Drawn as `DoorShutter_Draw` draws it, from the pack's meshes: the door's list (only when the
//! view's eye is on Link's side of it), the bars sliding down over it, the Jabu Jabu door's
//! eight sections, the boss door's texture (a bake per dungeon), and a locked door's chains
//! (`Actor_DrawDoorLock`).
//!
//! The Gohma slab's fall writes `Boss_Goma`'s sub camera, which isn't ported: its quake uses the
//! main camera (logged). Rumble (`Rumble_Request`, `Rumble_Override`) isn't ported.

use std::f32::consts::PI;
use std::sync::Arc;

use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use eng_gfx::{DrawCmd, MeshKey};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, BGCHECKFLAG_GROUND, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_DOOR, ActorImpl, ActorProfile, DOORLOCK_BOSS, DOORLOCK_NORMAL, DOORLOCK_NORMAL_SPIRIT, actor_draw_door_lock, actor_spawn_floor_dust_ring, audio_play_actor_sfx2};
use oot_game::audio::sfx::*;
use oot_game::camera::CAM_ID_MAIN;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::quake::QUAKE_TYPE_3;
use oot_game::scene::TRANSITION_ACTOR_PARAMS_INDEX_SHIFT;
use oot_game::sys_matrix::MtxF;

use crate::player::{PLAYER_DOORTYPE_SLIDING, Player, STATE1_6, STATE1_7, STATE1_10, STATE1_11, STATE1_28};

/// `ACTOR_DOOR_SHUTTER` (`actor_table.h`: 0x002E).
pub const ACTOR_DOOR_SHUTTER: i16 = 0x002E;

/// `Door_Shutter_Profile`: `FLAGS` `ACTOR_FLAG_UPDATE_CULLING_DISABLED`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_DOOR_SHUTTER, name: "Door_Shutter", category: ACTORCAT_DOOR, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: "gameplay_keep" };

// `DoorShutterType`.
pub const SHUTTER: u8 = 0x00;
pub const SHUTTER_FRONT_CLEAR: u8 = 0x01;
pub const SHUTTER_FRONT_SWITCH: u8 = 0x02;
pub const SHUTTER_BACK_LOCKED: u8 = 0x03;
pub const SHUTTER_PG_BARS: u8 = 0x04;
pub const SHUTTER_BOSS: u8 = 0x05;
pub const SHUTTER_GOHMA_BLOCK: u8 = 0x06;
pub const SHUTTER_FRONT_SWITCH_BACK_CLEAR: u8 = 0x07;
pub const SHUTTER_KEY_LOCKED: u8 = 0x0B;

// `DoorShutterGfxType`.
pub const DOORSHUTTER_GFX_DEKU_TREE_1: u8 = 0;
pub const DOORSHUTTER_GFX_DEKU_TREE_2: u8 = 1;
pub const DOORSHUTTER_GFX_DODONGOS_CAVERN: u8 = 2;
pub const DOORSHUTTER_GFX_JABU_JABU: u8 = 3;
pub const DOORSHUTTER_GFX_PHANTOM_GANON_BARS: u8 = 4;
pub const DOORSHUTTER_GFX_GOHMA_BLOCK: u8 = 5;
pub const DOORSHUTTER_GFX_SPIRIT_TEMPLE: u8 = 6;
pub const DOORSHUTTER_GFX_BOSS_DOOR: u8 = 7;
pub const DOORSHUTTER_GFX_GENERIC: u8 = 8;
pub const DOORSHUTTER_GFX_FIRE_TEMPLE_1: u8 = 9;
pub const DOORSHUTTER_GFX_FIRE_TEMPLE_2: u8 = 10;
pub const DOORSHUTTER_GFX_GANONS_TOWER: u8 = 11;
pub const DOORSHUTTER_GFX_WATER_TEMPLE_1: u8 = 12;
pub const DOORSHUTTER_GFX_WATER_TEMPLE_2: u8 = 13;
pub const DOORSHUTTER_GFX_SHADOW_TEMPLE_1: u8 = 14;
pub const DOORSHUTTER_GFX_SHADOW_TEMPLE_2: u8 = 15;
pub const DOORSHUTTER_GFX_ICE_CAVERN: u8 = 16;
pub const DOORSHUTTER_GFX_GERUDO_TRAINING_GROUND: u8 = 17;
pub const DOORSHUTTER_GFX_GANONS_CASTLE: u8 = 18;
pub const DOORSHUTTER_GFX_ROYAL_FAMILYS_TOMB: u8 = 19;

// `DoorShutterStyleType` (-1, `DOORSHUTTER_STYLE_FROM_SCENE`, is `None` in `S_TYPE_STYLES`).
pub const DOORSHUTTER_STYLE_PHANTOM_GANON: u8 = 0;
pub const DOORSHUTTER_STYLE_GOHMA_BLOCK: u8 = 1;
pub const DOORSHUTTER_STYLE_DEKU_TREE: u8 = 2;
pub const DOORSHUTTER_STYLE_DODONGOS_CAVERN: u8 = 3;
pub const DOORSHUTTER_STYLE_JABU_JABU: u8 = 4;
pub const DOORSHUTTER_STYLE_FOREST_TEMPLE: u8 = 5;
pub const DOORSHUTTER_STYLE_BOSS_DOOR: u8 = 6;
pub const DOORSHUTTER_STYLE_GENERIC: u8 = 7;
pub const DOORSHUTTER_STYLE_FIRE_TEMPLE: u8 = 8;
pub const DOORSHUTTER_STYLE_GANONS_TOWER: u8 = 9;
pub const DOORSHUTTER_STYLE_SPIRIT_TEMPLE: u8 = 10;
pub const DOORSHUTTER_STYLE_WATER_TEMPLE: u8 = 11;
pub const DOORSHUTTER_STYLE_SHADOW_TEMPLE: u8 = 12;
pub const DOORSHUTTER_STYLE_ICE_CAVERN: u8 = 13;
pub const DOORSHUTTER_STYLE_GERUDO_TRAINING_GROUND: u8 = 14;
pub const DOORSHUTTER_STYLE_GANONS_CASTLE: u8 = 15;
pub const DOORSHUTTER_STYLE_ROYAL_FAMILYS_TOMB: u8 = 16;

// `OBJECT_*` (`object_table.h`).
const OBJECT_GAMEPLAY_KEEP: i16 = 0x0001;
const OBJECT_GOMA: i16 = 0x001C;
const OBJECT_DDAN_OBJECTS: i16 = 0x002B;
const OBJECT_HIDAN_OBJECTS: i16 = 0x002C;
const OBJECT_YDAN_OBJECTS: i16 = 0x0036;
const OBJECT_GND: i16 = 0x0037;
const OBJECT_MENKURI_OBJECTS: i16 = 0x004D;
const OBJECT_MIZU_OBJECTS: i16 = 0x0059;
const OBJECT_ICE_OBJECTS: i16 = 0x006B;
const OBJECT_BDAN_OBJECTS: i16 = 0x0096;
const OBJECT_BDOOR: i16 = 0x00B0;
const OBJECT_GANON_OBJECTS: i16 = 0x0139;
const OBJECT_JYA_DOOR: i16 = 0x016D;
const OBJECT_DEMO_KEKKAI: i16 = 0x0179;
const OBJECT_HAKA_DOOR: i16 = 0x0187;
const OBJECT_OUKE_HAKA: i16 = 0x018F;

// `SCENE_*` (`scene_table.h`).
const SCENE_DEKU_TREE: u16 = 0x00;
const SCENE_DODONGOS_CAVERN: u16 = 0x01;
const SCENE_JABU_JABU: u16 = 0x02;
const SCENE_FOREST_TEMPLE: u16 = 0x03;
const SCENE_FIRE_TEMPLE: u16 = 0x04;
const SCENE_WATER_TEMPLE: u16 = 0x05;
const SCENE_SPIRIT_TEMPLE: u16 = 0x06;
const SCENE_SHADOW_TEMPLE: u16 = 0x07;
const SCENE_BOTTOM_OF_THE_WELL: u16 = 0x08;
const SCENE_ICE_CAVERN: u16 = 0x09;
const SCENE_GANONS_TOWER: u16 = 0x0A;
const SCENE_GERUDO_TRAINING_GROUND: u16 = 0x0B;
const SCENE_INSIDE_GANONS_CASTLE: u16 = 0x0D;
const SCENE_DODONGOS_CAVERN_BOSS: u16 = 0x12;
const SCENE_FOREST_TEMPLE_BOSS: u16 = 0x14;
const SCENE_FIRE_TEMPLE_BOSS: u16 = 0x15;
const SCENE_WATER_TEMPLE_BOSS: u16 = 0x16;
const SCENE_SPIRIT_TEMPLE_BOSS: u16 = 0x17;
const SCENE_SHADOW_TEMPLE_BOSS: u16 = 0x18;
const SCENE_GANONDORF_BOSS: u16 = 0x19;
const SCENE_ROYAL_FAMILYS_TOMB: u16 = 0x41;

/// `EVENTCHKINF_BEGAN_GOHMA_BATTLE` (`save.h`).
const EVENTCHKINF_BEGAN_GOHMA_BATTLE: u16 = 0x70;
/// `DUNGEON_BOSS_KEY` (`item.h`): the boss key's bit in `dungeonItems`.
const DUNGEON_BOSS_KEY: u32 = 0;
/// `RESPAWN_MODE_DOWN`.
const RESPAWN_MODE_DOWN: usize = 0;
/// `PLAYER_PARAMS(PLAYER_START_MODE_MOVE_FORWARD_SLOW, PLAYER_START_BG_CAM_DEFAULT)`.
const RESPAWN_PARAMS: i16 = 0x0DFF;
/// `PLAYER_CSACTION_2` and `PLAYER_CSACTION_7`: Link held, surprised, and let go.
const PLAYER_CSACTION_2: u8 = 2;
const PLAYER_CSACTION_7: u8 = 7;

/// `sStyleInfo`: `(objectId, gfxType1, gfxType2)` per style.
const S_STYLE_INFO: [(i16, u8, u8); 17] = [
    (OBJECT_GND, DOORSHUTTER_GFX_PHANTOM_GANON_BARS, DOORSHUTTER_GFX_PHANTOM_GANON_BARS),
    (OBJECT_GOMA, DOORSHUTTER_GFX_GOHMA_BLOCK, DOORSHUTTER_GFX_GOHMA_BLOCK),
    (OBJECT_YDAN_OBJECTS, DOORSHUTTER_GFX_DEKU_TREE_1, DOORSHUTTER_GFX_DEKU_TREE_2),
    (OBJECT_DDAN_OBJECTS, DOORSHUTTER_GFX_DODONGOS_CAVERN, DOORSHUTTER_GFX_DODONGOS_CAVERN),
    (OBJECT_BDAN_OBJECTS, DOORSHUTTER_GFX_JABU_JABU, DOORSHUTTER_GFX_JABU_JABU),
    (OBJECT_GAMEPLAY_KEEP, DOORSHUTTER_GFX_GENERIC, DOORSHUTTER_GFX_GENERIC),
    (OBJECT_BDOOR, DOORSHUTTER_GFX_BOSS_DOOR, DOORSHUTTER_GFX_BOSS_DOOR),
    (OBJECT_GAMEPLAY_KEEP, DOORSHUTTER_GFX_GENERIC, DOORSHUTTER_GFX_GENERIC),
    (OBJECT_HIDAN_OBJECTS, DOORSHUTTER_GFX_FIRE_TEMPLE_1, DOORSHUTTER_GFX_FIRE_TEMPLE_2),
    (OBJECT_GANON_OBJECTS, DOORSHUTTER_GFX_GANONS_TOWER, DOORSHUTTER_GFX_GANONS_TOWER),
    (OBJECT_JYA_DOOR, DOORSHUTTER_GFX_SPIRIT_TEMPLE, DOORSHUTTER_GFX_SPIRIT_TEMPLE),
    (OBJECT_MIZU_OBJECTS, DOORSHUTTER_GFX_WATER_TEMPLE_1, DOORSHUTTER_GFX_WATER_TEMPLE_2),
    (OBJECT_HAKA_DOOR, DOORSHUTTER_GFX_SHADOW_TEMPLE_1, DOORSHUTTER_GFX_SHADOW_TEMPLE_2),
    (OBJECT_ICE_OBJECTS, DOORSHUTTER_GFX_ICE_CAVERN, DOORSHUTTER_GFX_ICE_CAVERN),
    (OBJECT_MENKURI_OBJECTS, DOORSHUTTER_GFX_GERUDO_TRAINING_GROUND, DOORSHUTTER_GFX_GERUDO_TRAINING_GROUND),
    (OBJECT_DEMO_KEKKAI, DOORSHUTTER_GFX_GANONS_CASTLE, DOORSHUTTER_GFX_GANONS_CASTLE),
    (OBJECT_OUKE_HAKA, DOORSHUTTER_GFX_ROYAL_FAMILYS_TOMB, DOORSHUTTER_GFX_ROYAL_FAMILYS_TOMB),
];

/// `DoorShutterGfxInfo`: the door's and the bars' lists (`(file, symbol)`), how far up the
/// bars open, the bars' z offset, and how far to the sides and up or down Player can be to
/// open the door.
#[derive(Debug, Clone, Copy)]
pub struct GfxInfo {
    pub door_dl: (&'static str, &'static str),
    pub bars_dl: Option<(&'static str, &'static str)>,
    pub bars_open_offset_y: u8,
    pub bars_offset_z: u8,
    pub range_sides: u8,
    pub range_y: u8,
}

const fn gfx(door_dl: (&'static str, &'static str), bars_dl: Option<(&'static str, &'static str)>, bars_open_offset_y: u8, bars_offset_z: u8, range_sides: u8, range_y: u8) -> GfxInfo {
    GfxInfo { door_dl, bars_dl, bars_open_offset_y, bars_offset_z, range_sides, range_y }
}

const BARS: Option<(&str, &str)> = Some(("gameplay_keep", "gDoorMetalBarsDL"));

/// `sGfxInfo`, as gc-eu-mq-dbg builds it (`OOT_VERSION >= NTSC_1_1`: the Jabu Jabu, Phantom
/// Ganon, Gohma and boss doors' `rangeSides` 50).
pub const S_GFX_INFO: [GfxInfo; 20] = [
    gfx(("object_ydan_objects", "gDTDungeonDoor1DL"), BARS, 130, 12, 20, 15),
    gfx(("object_ydan_objects", "gDTDungeonDoor2DL"), BARS, 130, 12, 20, 15),
    gfx(("object_ddan_objects", "gDodongoDoorDL"), Some(("object_ddan_objects", "gDodongoBarsDL")), 240, 14, 70, 15),
    gfx(("object_bdan_objects", "gJabuDoorSection1DL"), Some(("object_bdan_objects", "gJabuWebDoorDL")), 0, 110, 50, 15),
    gfx(("object_gnd", "gPhantomGanonBarsDL"), None, 130, 12, 50, 15),
    gfx(("object_goma", "gGohmaDoorDL"), None, 130, 12, 50, 15),
    gfx(("object_jya_door", "gSpiritDoorDL"), Some(("object_jya_door", "gJyaDoorMetalBarsDL")), 240, 14, 50, 15),
    gfx(("object_bdoor", "gBossDoorDL"), None, 130, 12, 50, 15),
    gfx(("gameplay_keep", "gDungeonDoorDL"), BARS, 130, 12, 20, 15),
    gfx(("object_hidan_objects", "gFireTempleDoorFrontDL"), BARS, 130, 12, 20, 15),
    gfx(("object_hidan_objects", "gFireTempleDoorBackDL"), BARS, 130, 12, 20, 15),
    gfx(("object_ganon_objects", "object_ganon_objects_DL_0000C0"), BARS, 130, 12, 20, 15),
    gfx(("object_mizu_objects", "gObjectMizuObjectsDoorShutterDL_005D90"), BARS, 130, 12, 20, 15),
    gfx(("object_mizu_objects", "gObjectMizuObjectsDoorShutterDL_007000"), BARS, 130, 12, 20, 15),
    gfx(("object_haka_door", "object_haka_door_DL_002620"), BARS, 130, 12, 20, 15),
    gfx(("object_haka_door", "object_haka_door_DL_003890"), BARS, 130, 12, 20, 15),
    gfx(("object_ice_objects", "object_ice_objects_DL_001D10"), BARS, 130, 12, 20, 15),
    gfx(("object_menkuri_objects", "gGTGDoorDL"), BARS, 130, 12, 20, 15),
    gfx(("object_demo_kekkai", "gGanonsCastleDoorDL"), BARS, 130, 12, 20, 15),
    gfx(("object_ouke_haka", "object_ouke_haka_DL_0000C0"), BARS, 130, 12, 20, 15),
];

/// `sTypeStyles`: the style per type, `None` for `DOORSHUTTER_STYLE_FROM_SCENE`.
const S_TYPE_STYLES: [Option<u8>; 12] = [
    None,
    None,
    None,
    None,
    Some(DOORSHUTTER_STYLE_PHANTOM_GANON),
    Some(DOORSHUTTER_STYLE_BOSS_DOOR),
    Some(DOORSHUTTER_STYLE_GOHMA_BLOCK),
    None,
    Some(DOORSHUTTER_STYLE_PHANTOM_GANON),
    None,
    None,
    None,
];

/// `sSceneInfo`: the style of the doors in each scene, the last row for any other.
const S_SCENE_INFO: [(i32, u8); 18] = [
    (SCENE_DEKU_TREE as i32, DOORSHUTTER_STYLE_DEKU_TREE),
    (SCENE_DODONGOS_CAVERN as i32, DOORSHUTTER_STYLE_DODONGOS_CAVERN),
    (SCENE_DODONGOS_CAVERN_BOSS as i32, DOORSHUTTER_STYLE_DODONGOS_CAVERN),
    (SCENE_JABU_JABU as i32, DOORSHUTTER_STYLE_JABU_JABU),
    (SCENE_FOREST_TEMPLE as i32, DOORSHUTTER_STYLE_FOREST_TEMPLE),
    (SCENE_FIRE_TEMPLE as i32, DOORSHUTTER_STYLE_FIRE_TEMPLE),
    (SCENE_GANONS_TOWER as i32, DOORSHUTTER_STYLE_GANONS_TOWER),
    (SCENE_GANONDORF_BOSS as i32, DOORSHUTTER_STYLE_GANONS_TOWER),
    (SCENE_SPIRIT_TEMPLE as i32, DOORSHUTTER_STYLE_SPIRIT_TEMPLE),
    (SCENE_SPIRIT_TEMPLE_BOSS as i32, DOORSHUTTER_STYLE_SPIRIT_TEMPLE),
    (SCENE_WATER_TEMPLE as i32, DOORSHUTTER_STYLE_WATER_TEMPLE),
    (SCENE_SHADOW_TEMPLE as i32, DOORSHUTTER_STYLE_SHADOW_TEMPLE),
    (SCENE_BOTTOM_OF_THE_WELL as i32, DOORSHUTTER_STYLE_SHADOW_TEMPLE),
    (SCENE_ICE_CAVERN as i32, DOORSHUTTER_STYLE_ICE_CAVERN),
    (SCENE_GERUDO_TRAINING_GROUND as i32, DOORSHUTTER_STYLE_GERUDO_TRAINING_GROUND),
    (SCENE_INSIDE_GANONS_CASTLE as i32, DOORSHUTTER_STYLE_GANONS_CASTLE),
    (SCENE_ROYAL_FAMILYS_TOMB as i32, DOORSHUTTER_STYLE_ROYAL_FAMILYS_TOMB),
    (-1, DOORSHUTTER_STYLE_GENERIC),
];

// `DoorShutterBossDoorTexIndex`.
const DOORSHUTTER_BOSSDOORTEX_0: u8 = 0;
const DOORSHUTTER_BOSSDOORTEX_FIRE: u8 = 1;
const DOORSHUTTER_BOSSDOORTEX_WATER: u8 = 2;
const DOORSHUTTER_BOSSDOORTEX_SHADOW: u8 = 3;
const DOORSHUTTER_BOSSDOORTEX_GANON: u8 = 4;
const DOORSHUTTER_BOSSDOORTEX_FOREST: u8 = 5;
const DOORSHUTTER_BOSSDOORTEX_SPIRIT: u8 = 6;

/// `sBossDoorInfo`: `(dungeonSceneId, bossSceneId, texIndex)`, the last row for any other.
const S_BOSS_DOOR_INFO: [(i32, i32, u8); 7] = [
    (SCENE_FIRE_TEMPLE as i32, SCENE_FIRE_TEMPLE_BOSS as i32, DOORSHUTTER_BOSSDOORTEX_FIRE),
    (SCENE_WATER_TEMPLE as i32, SCENE_WATER_TEMPLE_BOSS as i32, DOORSHUTTER_BOSSDOORTEX_WATER),
    (SCENE_SHADOW_TEMPLE as i32, SCENE_SHADOW_TEMPLE_BOSS as i32, DOORSHUTTER_BOSSDOORTEX_SHADOW),
    (SCENE_GANONS_TOWER as i32, SCENE_GANONDORF_BOSS as i32, DOORSHUTTER_BOSSDOORTEX_GANON),
    (SCENE_FOREST_TEMPLE as i32, SCENE_FOREST_TEMPLE_BOSS as i32, DOORSHUTTER_BOSSDOORTEX_FOREST),
    (SCENE_SPIRIT_TEMPLE as i32, SCENE_SPIRIT_TEMPLE_BOSS as i32, DOORSHUTTER_BOSSDOORTEX_SPIRIT),
    (-1, -1, DOORSHUTTER_BOSSDOORTEX_0),
];

/// `sJabuDoorDLists`, in `object_bdan_objects`.
const S_JABU_DOOR_DLISTS: [&str; 8] =
    ["gJabuDoorSection1DL", "gJabuDoorSection2DL", "gJabuDoorSection7DL", "gJabuDoorSection4DL", "gJabuDoorSection5DL", "gJabuDoorSection4DL", "gJabuDoorSection3DL", "gJabuDoorSection2DL"];

/// `sBossDoorTextures`, in `object_bdoor`, by `DoorShutterBossDoorTexIndex`.
const S_BOSS_DOOR_TEXTURES: [&str; 7] = ["gBossDoorDefaultTex", "gBossDoorFireTex", "gBossDoorWaterTex", "gBossDoorShadowTex", "gBossDoorGanonsCastleTex", "gBossDoorForestTex", "gBossDoorSpiritTex"];

/// The bake of the boss door with texture `index` on segment 8.
fn boss_door_bake_name(index: usize) -> String {
    format!("Door_Shutter/boss/{index}")
}

/// The meshes `DoorShutter_Draw` draws that the pack's plain meshes don't cover: the boss door
/// with each dungeon's texture (`gSPSegment(0x08, sBossDoorTextures[bossDoorTexIndex])`).
pub fn bakes() -> Vec<MeshBake> {
    S_BOSS_DOOR_TEXTURES
        .iter()
        .enumerate()
        .map(|(i, tex)| MeshBake {
            name: boss_door_bake_name(i),
            object: "object_bdoor".into(),
            segments: vec![(0x08, BakeSegment::Texture { file: "object_bdoor".into(), symbol: (*tex).into() })],
            prelude: Vec::new(),
            body: BakeBody::DLists(vec![("object_bdoor".into(), "gBossDoorDL".into())]),
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `DoorShutter_WaitForObject`.
    WaitForObject,
    /// `DoorShutter_WaitClear`.
    WaitClear,
    /// `DoorShutter_Unopenable`.
    Unopenable,
    /// `DoorShutter_Idle`.
    Idle,
    /// `DoorShutter_BarAndWaitSwitchFlag`.
    BarAndWaitSwitchFlag,
    /// `DoorShutter_UnbarredCheckSwitchFlag`.
    UnbarredCheckSwitchFlag,
    /// `DoorShutter_Open`.
    Open,
    /// `DoorShutter_Unbar`.
    Unbar,
    /// `DoorShutter_Close`.
    Close,
    /// `DoorShutter_JabuDoorClose`.
    JabuDoorClose,
    /// `DoorShutter_WaitPlayerSurprised`.
    WaitPlayerSurprised,
    /// `DoorShutter_GohmaBlockFall`.
    GohmaBlockFall,
    /// `DoorShutter_GohmaBlockBounce`.
    GohmaBlockBounce,
    /// `DoorShutter_PhantomGanonBarsRaise`.
    PhantomGanonBarsRaise,
}

pub struct DoorShutter {
    /// `dyna.actor`.
    pub actor: Actor,
    /// `dyna.bgId` (`BG_ACTOR_MAX` until the Gohma slab or the bars register one: the doors
    /// themselves have no collision).
    pub bg: u16,
    /// `SLIDING_DOOR_ACTOR_BASE`'s `isActive`: Player set it opening the door (and the bars'
    /// and slab's counters).
    pub is_active: i16,
    /// `jabuDoorClosedAmount`: 0 open to 100 closed.
    pub jabu_door_closed_amount: i16,
    pub boss_door_tex_index: i16,
    pub door_type: u8,
    pub style_type: u8,
    pub gfx_type: u8,
    /// `requiredObjectSlot`.
    pub required_object_slot: Option<usize>,
    /// `unlockTimer`: non-zero while the door is locked, counting down as it unlocks.
    pub unlock_timer: i8,
    pub action_timer: i8,
    /// `barsClosedAmount`: 0 unbarred to 1 barred.
    pub bars_closed_amount: f32,
    pub action: Action,
}

impl DoorShutter {
    /// `DoorShutter_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: scale 1 (the culling volume isn't ported).
        actor.scale = Vec3::ONE;
        // Unused: shape.yOffset is still 0 here.
        actor.home_pos.z = actor.shape_y_offset;
        // DynaPolyActor_Init(&this->dyna, 0): bgId -1.
        let door_type = ((actor.params as u16 >> 6) & 0xF) as u8;
        let mut d = DoorShutter {
            actor,
            bg: BG_ACTOR_MAX,
            is_active: 0,
            jabu_door_closed_amount: 0,
            boss_door_tex_index: 0,
            door_type,
            style_type: 0,
            gfx_type: 0,
            required_object_slot: None,
            unlock_timer: 0,
            action_timer: 0,
            bars_closed_amount: 0.0,
            action: Action::WaitForObject,
        };
        let scene = play.scene_id as i32;
        let style_type = match S_TYPE_STYLES.get(door_type as usize).copied().flatten() {
            None => S_SCENE_INFO[..S_SCENE_INFO.len() - 1].iter().find(|e| e.0 == scene).unwrap_or(&S_SCENE_INFO[S_SCENE_INFO.len() - 1]).1,
            Some(DOORSHUTTER_STYLE_BOSS_DOOR) => {
                let e = S_BOSS_DOOR_INFO[..S_BOSS_DOOR_INFO.len() - 1].iter().find(|e| e.0 == scene || e.1 == scene).unwrap_or(&S_BOSS_DOOR_INFO[S_BOSS_DOOR_INFO.len() - 1]);
                d.boss_door_tex_index = e.2 as i16;
                DOORSHUTTER_STYLE_BOSS_DOOR
            }
            // DOORSHUTTER_STYLE_PHANTOM_GANON, DOORSHUTTER_STYLE_GOHMA_BLOCK.
            Some(s) => {
                d.actor.room = -1;
                s
            }
        };
        match play.object_ctx.get_index(S_STYLE_INFO[style_type as usize].0) {
            Some(slot) => d.required_object_slot = Some(slot),
            None => {
                d.actor.kill();
                return Box::new(d);
            }
        }
        d.setup_action(Action::WaitForObject);
        d.style_type = style_type;
        if d.door_type == SHUTTER_KEY_LOCKED || d.door_type == SHUTTER_BOSS {
            if !play.flags.get_switch(d.switch_flag()) {
                d.unlock_timer = 10;
            }
            d.actor.set_focus(60.0);
        } else if style_type == DOORSHUTTER_STYLE_JABU_JABU {
            // Actor_SetScale(0.1); cullingVolumeScale 200 (not ported).
            d.actor.scale = Vec3::splat(0.1);
            d.jabu_door_closed_amount = 100;
            d.actor.set_focus(0.0);
        } else {
            d.actor.set_focus(60.0);
        }
        Box::new(d)
    }

    /// `DOORSHUTTER_GET_SWITCH_FLAG`.
    pub fn switch_flag(&self) -> i32 {
        (self.actor.params & 0x3F) as i32
    }

    /// `GET_TRANSITION_ACTOR_INDEX`.
    pub fn transition_index(&self) -> usize {
        (self.actor.params as u16 >> TRANSITION_ACTOR_PARAMS_INDEX_SHIFT) as usize
    }

    /// `DoorShutter_SetupAction`.
    fn setup_action(&mut self, action: Action) {
        self.action = action;
        self.action_timer = 0;
    }

    /// `DoorShutter_SetupDoor`: the action for the door's type and the side Player is on; true
    /// if the door is barred.
    pub fn setup_door(&mut self, play: &mut PlayState) -> bool {
        let Some(entry) = play.transi_actors.get(self.transition_index()).copied() else { return false };
        let mut front_room = entry.sides[0].0;
        let mut door_type = self.door_type;
        let style_info = S_STYLE_INFO[self.style_type as usize];
        if door_type != SHUTTER_KEY_LOCKED {
            if front_room == entry.sides[1].0 {
                // Player in front.
                let d = self.actor.shape_rot.y.wrapping_sub(self.actor.yaw_towards_player);
                if (d as i32).abs() < 0x4000 {
                    front_room = -1;
                }
            }
            if front_room == self.actor.room {
                door_type = if door_type == SHUTTER_FRONT_SWITCH_BACK_CLEAR {
                    // The back's clear becomes the front's.
                    SHUTTER_FRONT_CLEAR
                } else if door_type == SHUTTER_BOSS {
                    SHUTTER_BACK_LOCKED
                } else {
                    SHUTTER
                };
            }
        }
        self.gfx_type = if door_type == SHUTTER { style_info.1 } else { style_info.2 };
        if door_type == SHUTTER_FRONT_CLEAR {
            if !play.flags.get_clear(self.actor.room) {
                self.setup_action(Action::WaitClear);
                self.bars_closed_amount = 1.0;
                return true;
            }
        } else if door_type == SHUTTER_FRONT_SWITCH || door_type == SHUTTER_FRONT_SWITCH_BACK_CLEAR {
            if !play.flags.get_switch(self.switch_flag()) {
                self.setup_action(Action::BarAndWaitSwitchFlag);
                self.bars_closed_amount = 1.0;
                return true;
            } else {
                self.setup_action(Action::UnbarredCheckSwitchFlag);
                return false;
            }
        } else if door_type == SHUTTER_BACK_LOCKED {
            self.setup_action(Action::Unopenable);
            return false;
        }
        self.setup_action(Action::Idle);
        false
    }

    /// `DoorShutter_WaitForObject`.
    fn wait_for_object(&mut self, play: &mut PlayState) {
        let Some(slot) = self.required_object_slot.filter(|&s| play.object_ctx.is_loaded(s)) else { return };
        self.actor.obj_bank_index = Some(slot);
        if self.door_type == SHUTTER_PG_BARS || self.door_type == SHUTTER_GOHMA_BLOCK {
            // DynaPoly for the kinds that have collision. (Actor_SetObjectDependency.)
            self.gfx_type = S_STYLE_INFO[self.style_type as usize].1;
            let (file, symbol) = if self.door_type == SHUTTER_GOHMA_BLOCK { ("object_goma", "gGohmaDoorCol") } else { ("object_gnd", "gPhantomGanonBarsCol") };
            self.bg = match play.assets.as_ref().map(|a| a.pack.collision(&keys::collision(file, symbol))) {
                Some(Ok(h)) => play.col.dyna.set_bg_actor(Arc::new(h), self.source(), 0),
                Some(Err(e)) => {
                    log::error!("Door_Shutter: {file} {symbol}: {e:#}");
                    BG_ACTOR_MAX
                }
                None => BG_ACTOR_MAX,
            };
            if self.door_type == SHUTTER_GOHMA_BLOCK {
                self.actor.velocity.y = 0.0;
                self.actor.gravity = -2.0;
                audio_play_actor_sfx2(play, NA_SE_EV_SLIDE_DOOR_CLOSE);
                self.setup_action(Action::GohmaBlockFall);
            } else {
                self.setup_action(Action::PhantomGanonBarsRaise);
                self.is_active = 7;
            }
        } else {
            self.setup_door(play);
        }
    }

    /// The transform `DynaPoly_UpdateContext` reads from the actor.
    fn source(&self) -> BgActorSource {
        let r = self.actor.shape_rot;
        BgActorSource { pos: self.actor.world_pos, shape_rot: [r.x, r.y, r.z], scale: self.actor.scale, shape_y_offset: self.actor.shape_y_offset }
    }

    /// `DoorShutter_GetPlayerDistance`: Player's distance forwards from the door (negative
    /// behind it), or `MAXFLOAT` when he's beyond `max_dist_sides` to a side or `max_dist_y`
    /// above or below.
    fn get_player_distance(&self, player_pos: Vec3, offset_y: f32, max_dist_sides: f32, max_dist_y: f32) -> f32 {
        let rel = self.actor.world_to_actor_coords(Vec3::new(player_pos.x, player_pos.y + offset_y, player_pos.z));
        if rel.x.abs() > max_dist_sides || rel.y.abs() > max_dist_y { f32::MAX } else { rel.z }
    }

    /// `DoorShutter_GetPlayerSide`: 1 when Player is near and facing the door's front, -1 its
    /// back, else 0.
    fn get_player_side(&self, play: &PlayState) -> i8 {
        if play.player_in_cs_mode() {
            return 0;
        }
        let Some(p) = play.player.and_then(|h| play.actors.downcast::<Player>(h)) else { return 0 };
        let g = S_GFX_INFO[self.gfx_type as usize];
        let offset_y = if self.gfx_type != DOORSHUTTER_GFX_JABU_JABU { 0.0 } else { 80.0 };
        let dist = self.get_player_distance(p.actor.world_pos, offset_y, g.range_sides as f32, g.range_y as f32);
        if dist.abs() < 50.0 {
            let mut yaw_diff = p.actor.shape_rot.y.wrapping_sub(self.actor.shape_rot.y);
            // In front, facing the door is facing the opposite way to it: centre on 0.
            if dist > 0.0 {
                yaw_diff = (-0x8000i32 as i16).wrapping_sub(yaw_diff);
            }
            if (yaw_diff as i32).abs() < 0x3000 {
                return if dist >= 0.0 { 1 } else { -1 };
            }
        }
        0
    }

    /// `player->naviTextId = id`.
    fn set_navi_text_id(play: &mut PlayState, id: i16) {
        if let Some(p) = play.player.and_then(|h| play.actors.downcast_mut::<Player>(h)) {
            p.navi_text_id = id;
        }
    }

    /// `OnePointCutscene_Attention` on the door (out of the arena: it passes itself).
    fn attention_on_self(&self, play: &mut PlayState) {
        if let Some(me) = play.cur_actor {
            let a = play.cam_actor_of(me, &self.actor);
            play.onepoint_attention(a);
        }
    }

    /// `DoorShutter_WaitClear`: barred until the room is cleared (its temporary clear flag is
    /// made permanent).
    fn wait_clear(&mut self, play: &mut PlayState) {
        if play.flags.get_clear(self.actor.room) || play.flags.get_temp_clear(self.actor.room) {
            play.flags.set_clear(self.actor.room);
            self.setup_action(Action::Unbar);
            self.attention_on_self(play);
            if let Some(a) = play.player.and_then(|h| play.cam_actor(h)) {
                play.onepoint_attention(a);
            }
            self.action_timer = -100;
        } else if self.get_player_side(play) != 0 {
            Self::set_navi_text_id(play, -0x202);
        }
    }

    /// `DoorShutter_Idle`: Player may open the door; a locked door takes a small key (or needs
    /// the boss key), and its switch flag is set when it opens.
    fn idle(&mut self, play: &mut PlayState) {
        if self.is_active != 0 {
            self.setup_action(Action::Open);
            self.actor.velocity.y = 0.0;
            if self.unlock_timer != 0 {
                play.flags.set_switch(self.switch_flag());
                if self.door_type != SHUTTER_BOSS {
                    let m = play.save.map_index as usize;
                    if let Some(k) = play.save.inventory.dungeon_keys.get_mut(m) {
                        *k -= 1;
                    }
                    audio_play_actor_sfx2(play, NA_SE_EV_CHAIN_KEY_UNLOCK);
                } else {
                    audio_play_actor_sfx2(play, NA_SE_EV_CHAIN_KEY_UNLOCK_B);
                }
            }
        } else {
            let door_direction = self.get_player_side(play);
            if door_direction != 0 {
                let m = play.save.map_index as usize;
                if self.unlock_timer != 0 {
                    if self.door_type == SHUTTER_BOSS {
                        // CHECK_DUNGEON_ITEM(DUNGEON_BOSS_KEY, mapIndex).
                        let items = play.save.inventory.dungeon_items.get(m).copied().unwrap_or(0) as u32;
                        if items & (1 << DUNGEON_BOSS_KEY) == 0 {
                            Self::set_navi_text_id(play, -0x204);
                            return;
                        }
                    } else if play.save.inventory.dungeon_keys.get(m).copied().unwrap_or(0) <= 0 {
                        Self::set_navi_text_id(play, -0x203);
                        return;
                    }
                }
                let me = play.cur_actor;
                let unlocking = self.unlock_timer != 0;
                if let Some(p) = play.player.and_then(|h| play.actors.downcast_mut::<Player>(h)) {
                    if unlocking {
                        p.door_timer = 10;
                    }
                    p.door_type = PLAYER_DOORTYPE_SLIDING;
                    p.door_direction = door_direction;
                    p.door_actor = me;
                }
            }
        }
    }

    /// `DoorShutter_InitOpeningDoorCam`: the door camera, held longer when the door will bar
    /// behind Player.
    fn init_opening_door_cam(&mut self, play: &mut PlayState) {
        // The category is never changed from ACTORCAT_DOOR: always taken.
        if self.actor.category == ACTORCAT_DOOR {
            let saved_gfx_type = self.gfx_type;
            let mut door_cam_timer2 = 15;
            if self.setup_door(play) {
                door_cam_timer2 = 32;
            }
            // The action, gfxType and barsClosedAmount back as they were.
            self.setup_action(Action::Open);
            self.gfx_type = saved_gfx_type;
            self.bars_closed_amount = 0.0;
            let bg_cam_index = play.player.and_then(|h| play.actors.downcast::<Player>(h)).map(|p| p.sliding_door_bg_cam_index).unwrap_or(0);
            let data = play.data.clone();
            let me = play.cur_actor;
            play.game_camera.change_door_cam(&data.camera, &play.col, me, bg_cam_index, 12, door_cam_timer2, 10);
        }
    }

    /// `DoorShutter_UpdateOpening`: the door slides up 200 (the Jabu Jabu door shrinks open);
    /// the first frame plays the sound and changes the camera. True once open.
    fn update_opening(&mut self, play: &mut PlayState) -> bool {
        if self.gfx_type != DOORSHUTTER_GFX_JABU_JABU {
            if self.actor.velocity.y == 0.0 {
                audio_play_actor_sfx2(play, NA_SE_EV_SLIDE_DOOR_OPEN);
                self.init_opening_door_cam(play);
            }
            eng_math::step_to_f(&mut self.actor.velocity.y, 15.0, 3.0);
            let target = self.actor.home_pos.y + 200.0;
            let step = self.actor.velocity.y;
            if eng_math::step_to_f(&mut self.actor.world_pos.y, target, step) {
                return true;
            }
        } else {
            if self.jabu_door_closed_amount == 100 {
                audio_play_actor_sfx2(play, NA_SE_EV_BUYODOOR_OPEN);
                self.init_opening_door_cam(play);
            }
            if eng_math::step_to_s(&mut self.jabu_door_closed_amount, 0, 10) {
                return true;
            }
        }
        false
    }

    /// `DoorShutter_UpdateBarsClosed`: the bars towards `target` (1 barred, 0 unbarred) by 0.2
    /// a frame, the sound as they start. True when there.
    fn update_bars_closed(&mut self, play: &mut PlayState, target: f32) -> bool {
        if self.bars_closed_amount == 1.0 - target {
            let sfx = match (self.gfx_type != DOORSHUTTER_GFX_JABU_JABU, target == 1.0) {
                (true, true) => NA_SE_EV_METALDOOR_CLOSE,
                (true, false) => NA_SE_EV_METALDOOR_OPEN,
                (false, true) => NA_SE_EV_BUYOSHUTTER_CLOSE,
                (false, false) => NA_SE_EV_BUYOSHUTTER_OPEN,
            };
            audio_play_actor_sfx2(play, sfx);
        }
        eng_math::step_to_f(&mut self.bars_closed_amount, target, 0.2)
    }

    /// `DoorShutter_BarAndWaitSwitchFlag`.
    fn bar_and_wait_switch_flag(&mut self, play: &mut PlayState) {
        if self.update_bars_closed(play, 1.0) {
            if play.flags.get_switch(self.switch_flag()) {
                self.setup_action(Action::Unbar);
                self.attention_on_self(play);
                self.action_timer = -100;
            } else if self.get_player_side(play) != 0 {
                Self::set_navi_text_id(play, if play.scene_id == SCENE_JABU_JABU { -0x20B } else { -0x202 });
            }
        }
    }

    /// `DoorShutter_UnbarredCheckSwitchFlag`: unbarred, the switch flag is checked again.
    fn unbarred_check_switch_flag(&mut self, play: &mut PlayState) {
        if self.is_active == 0 && !play.flags.get_switch(self.switch_flag()) {
            self.setup_action(Action::BarAndWaitSwitchFlag);
        } else {
            self.idle(play);
        }
    }

    /// `DoorShutter_Open`: once unlocked and the room behind it loaded, it opens; once Player is
    /// more than 50 away (20 for the boss door), it closes.
    fn open(&mut self, play: &mut PlayState) {
        // DECR(unlockTimer) == 0.
        if self.unlock_timer != 0 {
            self.unlock_timer -= 1;
        }
        if self.unlock_timer == 0 && play.room_ctx.status == 0 && self.update_opening(play) {
            let dist = if self.door_type == SHUTTER_BOSS { 20.0 } else { 50.0 };
            if self.actor.xz_dist_to_player > dist {
                if self.setup_door(play) {
                    // Barred behind Player: closes faster.
                    self.actor.velocity.y = 30.0;
                }
                if self.gfx_type != DOORSHUTTER_GFX_JABU_JABU {
                    audio_play_actor_sfx2(play, NA_SE_EV_SLIDE_DOOR_CLOSE);
                    self.setup_action(Action::Close);
                } else {
                    audio_play_actor_sfx2(play, NA_SE_EV_BUYODOOR_CLOSE);
                    if (self.door_type == SHUTTER_FRONT_SWITCH || self.door_type == SHUTTER_FRONT_SWITCH_BACK_CLEAR) && !play.flags.get_switch(self.switch_flag()) {
                        audio_play_actor_sfx2(play, NA_SE_EV_BUYOSHUTTER_CLOSE);
                    }
                    self.setup_action(Action::JabuDoorClose);
                }
            }
        }
    }

    /// `DoorShutter_Unbar`: after the attention camera has turned to the door (or 100 odd
    /// frames), the bars slide away.
    fn unbar(&mut self, play: &mut PlayState) {
        if self.action_timer != 0 {
            if self.action_timer < 0 {
                if play.state_frames % 2 != 0 {
                    self.action_timer += 1;
                }
                // func_8005B198: the category the attention camera attended.
                if self.actor.category as i32 == play.onepoint.d_8011d3ac || self.action_timer == 0 {
                    self.action_timer = 5;
                }
            } else {
                self.action_timer -= 1;
            }
        } else if self.update_bars_closed(play, 0.0) {
            if !(self.door_type == SHUTTER || self.door_type == SHUTTER_FRONT_CLEAR) {
                self.setup_action(Action::UnbarredCheckSwitchFlag);
            } else {
                self.setup_action(Action::Idle);
            }
            play.audio.func_800f5b58();
        }
    }

    /// `DoorShutter_SetupClosed`: the door is shut. The room behind Player goes, the respawn
    /// point is set, and a door barred on his side holds him a moment.
    fn setup_closed(&mut self, play: &mut PlayState) {
        let room = self.actor.room;
        let player_pos = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos);
        if self.actor.room >= 0 {
            if let (Some(pp), Some(entry)) = (player_pos, play.transi_actors.get(self.transition_index()).copied()) {
                let rel = self.actor.world_to_actor_coords(pp);
                self.actor.room = entry.sides[if rel.z < 0.0 { 0 } else { 1 }].0;
            }
            if room != self.actor.room {
                // Player went back: the rooms swap.
                std::mem::swap(&mut play.room_ctx.cur, &mut play.room_ctx.prev);
                play.room_ctx.active_buf_page ^= 1;
            }
            play.room_change_done();
            play.setup_respawn_point(RESPAWN_MODE_DOWN, RESPAWN_PARAMS);
        }
        self.is_active = 0;
        self.actor.velocity.y = 0.0;
        let carrying = play.player.and_then(|h| play.actors.downcast::<Player>(h)).is_some_and(|p| p.state1 & STATE1_11 != 0);
        if self.setup_door(play) && !carrying {
            // Barred behind Player.
            self.setup_action(Action::WaitPlayerSurprised);
            play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_2);
        }
    }

    /// `DoorShutter_Close`: the door slides down to its home; landing, it thuds (with dust when
    /// it slammed), shakes the camera, and is shut.
    fn close(&mut self, play: &mut PlayState) {
        if self.actor.velocity.y < 20.0 {
            eng_math::step_to_f(&mut self.actor.velocity.y, 20.0, 8.0);
        }
        let (home_y, step) = (self.actor.home_pos.y, self.actor.velocity.y);
        if eng_math::step_to_f(&mut self.actor.world_pos.y, home_y, step) {
            if self.actor.velocity.y > 20.0 {
                self.actor.floor_height = self.actor.home_pos.y;
                let (a, pos) = (self.actor.clone(), self.actor.world_pos);
                actor_spawn_floor_dust_ring(play, &a, pos, 45.0, 10, 8.0, 500, 10, false);
            }
            audio_play_actor_sfx2(play, NA_SE_EV_STONE_BOUND);
            quake(play, CAM_ID_MAIN, QUAKE_TYPE_3, -32536, 2, 10);
            // Rumble_Request(xyzDistToPlayerSq, 180, 20, 100): no rumble.
            self.setup_closed(play);
        }
    }

    /// `DoorShutter_JabuDoorClose`.
    fn jabu_door_close(&mut self, play: &mut PlayState) {
        if eng_math::step_to_s(&mut self.jabu_door_closed_amount, 100, 10) {
            self.setup_closed(play);
        }
    }

    /// `DoorShutter_WaitPlayerSurprised`: 32 frames, then Player is let go.
    fn wait_player_surprised(&mut self, play: &mut PlayState) {
        let t = self.action_timer;
        self.action_timer = t.wrapping_add(1);
        if t > 30 {
            play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_7);
            self.setup_door(play);
        }
    }

    /// `DoorShutter_GohmaBlockFall`: the slab falls; landing before Gohma's battle began, it
    /// thuds, shakes her sub camera and raises a ring of dust.
    fn gohma_block_fall(&mut self, play: &mut PlayState) {
        self.actor.move_forward();
        let own_bg = (self.bg != BG_ACTOR_MAX).then_some(self.bg);
        self.actor.update_bg_check_info_of(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2, own_bg);
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.setup_action(Action::GohmaBlockBounce);
            if !play.save.get_event_chk_inf(EVENTCHKINF_BEGAN_GOHMA_BATTLE) {
                self.is_active = 10;
                audio_play_actor_sfx2(play, NA_SE_EV_STONE_BOUND);
                // ((BossGoma*)this->dyna.actor.parent)->subCamId. With no Queen Gohma for a parent
                // (a test's slab: the C would read through NULL), the main camera's.
                let sub_cam_id = match self.actor.parent.and_then(|h| play.actors.downcast::<crate::boss_goma::BossGoma>(h)) {
                    Some(goma) => goma.sub_cam_id,
                    None => {
                        log::warn!("Door_Shutter: the Gohma slab has no Boss_Goma parent: its quake goes to the main camera");
                        CAM_ID_MAIN
                    }
                };
                Self::request_quake_and_rumble(play, 2, 10, sub_cam_id);
                let (a, pos) = (self.actor.clone(), self.actor.world_pos);
                actor_spawn_floor_dust_ring(play, &a, pos, 70.0, 20, 8.0, 500, 10, true);
            }
        }
    }

    /// `DoorShutter_GohmaBlockBounce`: a bounce too small to see.
    fn gohma_block_bounce(&mut self) {
        if self.is_active != 0 {
            self.is_active -= 1;
            let bounce_factor = (self.is_active as f32 * 250.0 / 100.0).sin();
            self.actor.shape_y_offset = self.is_active as f32 * 3.0 / 10.0 * bounce_factor;
        }
    }

    /// `DoorShutter_PhantomGanonBarsRaise`.
    fn phantom_ganon_bars_raise(&mut self) {
        // DECR(isActive).
        if self.is_active != 0 {
            self.is_active -= 1;
        }
        let target_offset_y = if self.is_active % 2 != 0 { -3.0 } else { 0.0 };
        eng_math::smooth_step_to_f(&mut self.actor.world_pos.y, -34.0 + target_offset_y, 1.0, 20.0, 0.0);
    }

    /// `DoorShutter_RequestQuakeAndRumble`.
    fn request_quake_and_rumble(play: &mut PlayState, quake_y: i16, quake_duration: i16, cam_id: i16) {
        // (Rumble_Override(0.0f, 180, 20, 100) between the request and its settings: no rumble.)
        quake(play, cam_id, QUAKE_TYPE_3, 20000, quake_y, quake_duration);
    }

    /// `DoorShutter_ShouldDraw`: the view's eye on Player's side of the door (always in a
    /// cutscene).
    fn should_draw(&self, rs: &RenderState, play: &PlayState, view: &ViewInfo) -> bool {
        if play.player_in_cs_mode() {
            return true;
        }
        let yaw = rs.rot[1];
        let to_eye = eng_math::atan2_s(view.eye.z - rs.pos.z, view.eye.x - rs.pos.x);
        let rel_eye = (to_eye.wrapping_sub(yaw) as i32).abs();
        let to_player = rs.angles.get(rs::YAW_TOWARDS_PLAYER).copied().unwrap_or(0);
        let rel_player = (to_player.wrapping_sub(yaw) as i32).abs();
        !((rel_player < 0x4000 && rel_eye > 0x4000) || (rel_player > 0x4000 && rel_eye < 0x4000))
    }

    /// `DoorShutter_DrawJabuJabuDoor`: the eight sections round the door's centre.
    fn draw_jabu_jabu_door(base: &MtxF, closed_amount: f32, out: &mut DrawOut) {
        let mut angle = 0.0f32;
        let y_scale = closed_amount * 0.01;
        for (i, dl) in S_JABU_DOOR_DLISTS.iter().enumerate() {
            let mut m = *base;
            m.rotate_z(angle);
            if i % 2 == 0 {
                m.translate(0.0, 800.0, 0.0);
            } else if i == 1 || i == 7 {
                m.translate(0.0, 848.52, 0.0);
            } else {
                m.translate(0.0, 989.94, 0.0);
            }
            if closed_amount != 100.0 {
                m.scale(1.0, y_scale, 1.0);
            }
            out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh("object_bdan_objects", dl)), m.to_mat4()));
            angle -= 2.0 * PI / S_JABU_DOOR_DLISTS.len() as f32;
        }
    }
}

/// `Quake_Request(Play_GetCamera(play, cam_id), ty)`, then `Quake_SetSpeed(i, speed)`,
/// `Quake_SetPerturbations(i, y, 0, 0, 0)` and `Quake_SetDuration(i, duration)`.
fn quake(play: &mut PlayState, cam_id: i16, ty: u32, speed: i16, y: i16, duration: i16) {
    let i = play.quake_request(cam_id, ty);
    play.quake_set_speed(i, speed);
    play.quake_set_perturbations(i, y, 0, 0, 0);
    play.quake_set_duration(i, duration);
}

/// Indices into the render state's extras.
mod rs {
    /// `angles`: `yawTowardsPlayer`.
    pub const YAW_TOWARDS_PLAYER: usize = 0;
    /// `values`: `barsClosedAmount`, `jabuDoorClosedAmount`.
    pub const BARS_CLOSED_AMOUNT: usize = 0;
    pub const JABU_DOOR_CLOSED_AMOUNT: usize = 1;
    /// `switches`: `gfxType`, `unlockTimer`, whether the object is in (`objectSlot ==
    /// requiredObjectSlot`), and the room the door was in.
    pub const GFX_TYPE: usize = 0;
    pub const UNLOCK_TIMER: usize = 1;
    pub const OBJECT_READY: usize = 2;
    pub const ROOM: usize = 3;
}

impl ActorImpl for DoorShutter {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `DoorShutter_Update`: nothing while Player talks, is dead, gets an item or is held
    /// (`PLAYER_STATE1_28`), but waiting for the object.
    fn update(&mut self, play: &mut PlayState) {
        let state1 = play.player.and_then(|h| play.actors.downcast::<Player>(h)).map(|p| p.state1).unwrap_or(0);
        if state1 & (STATE1_6 | STATE1_7 | STATE1_10 | STATE1_28) == 0 || self.action == Action::WaitForObject {
            match self.action {
                Action::WaitForObject => self.wait_for_object(play),
                Action::WaitClear => self.wait_clear(play),
                Action::Unopenable => {}
                Action::Idle => self.idle(play),
                Action::BarAndWaitSwitchFlag => self.bar_and_wait_switch_flag(play),
                Action::UnbarredCheckSwitchFlag => self.unbarred_check_switch_flag(play),
                Action::Open => self.open(play),
                Action::Unbar => self.unbar(play),
                Action::Close => self.close(play),
                Action::JabuDoorClose => self.jabu_door_close(play),
                Action::WaitPlayerSurprised => self.wait_player_surprised(play),
                Action::GohmaBlockFall => self.gohma_block_fall(play),
                Action::GohmaBlockBounce => self.gohma_block_bounce(),
                Action::PhantomGanonBarsRaise => self.phantom_ganon_bars_raise(),
            }
        }
        if self.bg != BG_ACTOR_MAX {
            play.col.dyna.set_source(self.bg, self.source());
        }
    }
    /// `DoorShutter_Destroy`: its bg actor goes, and the transition-actor entry can spawn
    /// again (`id *= -1`).
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
        if self.actor.room >= 0
            && let Some(t) = play.transi_actors.get_mut(self.transition_index())
        {
            t.id = -t.id;
        }
    }
    fn render_state(&self) -> RenderState {
        let mut s = RenderState::of(&self.actor);
        s.angles = vec![self.actor.yaw_towards_player];
        s.values = vec![self.bars_closed_amount, self.jabu_door_closed_amount as f32];
        let ready = self.actor.obj_bank_index.is_some() && self.actor.obj_bank_index == self.required_object_slot;
        s.switches = vec![self.gfx_type as u32, self.unlock_timer as u8 as u32, ready as u32, self.actor.room as u8 as u32];
        s
    }
    /// `DoorShutter_Draw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        // @bug (game): the door draws before its object is in when that object is the profile's
        // (gameplay_keep); here an actor that never ran its update has no object slot yet, so
        // it doesn't.
        if rs.switches.get(rs::OBJECT_READY).copied().unwrap_or(0) == 0 {
            return;
        }
        if !(self.style_type == DOORSHUTTER_STYLE_PHANTOM_GANON || self.should_draw(rs, play, view)) {
            return;
        }
        let gfx_type = rs.switches[rs::GFX_TYPE] as u8;
        let unlock_timer = rs.switches[rs::UNLOCK_TIMER] as u8 as i8;
        let room = rs.switches[rs::ROOM] as u8 as i8;
        let bars_closed_amount = rs.values[rs::BARS_CLOSED_AMOUNT];
        let jabu_closed_amount = rs.values[rs::JABU_DOOR_CLOSED_AMOUNT];
        let g = S_GFX_INFO[gfx_type as usize];
        let mut m = MtxF::from_mat4(actor_draw_matrix(rs));
        if gfx_type == DOORSHUTTER_GFX_JABU_JABU {
            Self::draw_jabu_jabu_door(&m, jabu_closed_amount, out);
            if bars_closed_amount != 0.0 {
                let scale = (jabu_closed_amount * 0.01) * bars_closed_amount;
                // (gDPSetEnvColor alpha 255 * scale: no purpose.)
                m.translate(0.0, 0.0, g.bars_offset_z as f32);
                m.scale(scale, scale, scale);
                if let Some((file, sym)) = g.bars_dl {
                    out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh(file, sym)), m.to_mat4()));
                }
            }
        } else {
            let mut door_mesh = keys::mesh(g.door_dl.0, g.door_dl.1);
            if g.bars_dl.is_some() {
                let entry = play.transi_actors.get(self.transition_index());
                let same_room = entry.is_some_and(|t| t.sides[0].0 == t.sides[1].0);
                if play.room_ctx.prev.num >= 0 || same_room {
                    let yaw = eng_math::atan2_s(rs.pos.z - view.eye.z, rs.pos.x - view.eye.x);
                    if (rs.rot[1].wrapping_sub(yaw) as i32).abs() < 0x4000 {
                        m.rotate_y(PI);
                    }
                } else if entry.is_some_and(|t| room == t.sides[0].0) {
                    m.rotate_y(PI);
                }
            } else if self.door_type == SHUTTER_BOSS {
                door_mesh = keys::bake(&boss_door_bake_name(self.boss_door_tex_index as usize));
            }
            out.opa.push(DrawCmd::new(MeshKey::named(door_mesh), m.to_mat4()));
            if bars_closed_amount != 0.0
                && let Some((file, sym)) = g.bars_dl
            {
                m.translate(0.0, g.bars_open_offset_y as f32 * (1.0 - bars_closed_amount), g.bars_offset_z as f32);
                out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh(file, sym)), m.to_mat4()));
            }
        }
        if unlock_timer != 0 {
            m.scale(0.01, 0.01, 0.025);
            let ty = if self.door_type == SHUTTER_BOSS {
                DOORLOCK_BOSS
            } else if gfx_type == DOORSHUTTER_GFX_SPIRIT_TEMPLE {
                DOORLOCK_NORMAL_SPIRIT
            } else {
                DOORLOCK_NORMAL
            };
            actor_draw_door_lock(out, &m, unlock_timer as i32, ty);
        }
    }
    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
