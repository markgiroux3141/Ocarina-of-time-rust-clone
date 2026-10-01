//! Scenes and rooms as the game uses them: the scene table, each scene's header per layer
//! (spawns, lights, skybox, the keep object, collision) and each room's header and meshes.
//! These are asset-pack records (`crate::pack`); `oot_import` builds them from the ROM, with
//! every room display list interpreted into meshes for the layer's state.

use eng_gfx::DrawList;
use glam::Vec3;

/// `gSaveContext.sceneLayer` values for the four non-cutscene layers (`z64save.h`): the scene's
/// alternate headers 0..3.
pub const LAYER_CHILD_DAY: usize = 0;
pub const LAYER_CHILD_NIGHT: usize = 1;
pub const LAYER_ADULT_DAY: usize = 2;
pub const LAYER_ADULT_NIGHT: usize = 3;
/// The non-cutscene layers, which every scene has (a missing header falls back as
/// `Scene_CommandAlternateHeaderList` does). Cutscene layers (4 and up) are in the pack for the
/// scenes whose alternate header lists name them (docs/adr/0023-navi-and-the-opening.md).
pub const GAME_LAYERS: usize = 4;
/// `SCENE_LAYER_CUTSCENE_FIRST` (`z64save.h`): `Play_Init` picks `4 + (cutsceneIndex & 0xF)`
/// for a `cutsceneIndex` of 0xFFF0 and up.
pub const SCENE_LAYER_CUTSCENE_FIRST: usize = 4;

/// The scene layer the game picks for a non-cutscene entrance (`Play_Init`: `sceneLayer` from
/// `linkAge` and `nightFlag`).
pub fn layer_for(child: bool, night: bool) -> usize {
    match (child, night) {
        (true, false) => LAYER_CHILD_DAY,
        (true, true) => LAYER_CHILD_NIGHT,
        (false, false) => LAYER_ADULT_DAY,
        (false, true) => LAYER_ADULT_NIGHT,
    }
}

/// `Path` (`z64scene.h`): a scene path, `{ u8 count; Vec3s* points; }`, with its points read.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Path {
    pub points: Vec<[i16; 3]>,
}

impl Path {
    /// `path->count`.
    pub fn count(&self) -> usize {
        self.points.len()
    }

    /// `((Vec3s*)SEGMENTED_TO_VIRTUAL(path->points))[i]` as a `Vec3f`.
    pub fn point(&self, i: usize) -> Vec3 {
        let p = self.points[i];
        Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
    }
}

/// `ActorEntry` from the spawn/actor lists (0x10 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ActorEntry {
    pub id: i16,
    pub pos: [i16; 3],
    pub rot: [i16; 3],
    pub params: i16,
}

/// `EnvLightSettings` (`z64environment.h`, 0x16 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EnvLightSettings {
    pub ambient: [u8; 3],
    pub light1_dir: [i8; 3],
    pub light1_color: [u8; 3],
    pub light2_dir: [i8; 3],
    pub light2_color: [u8; 3],
    pub fog_color: [u8; 3],
    /// Blend rate in the top 6 bits (`>> 10`, × 4 per frame), fog near in the low 10.
    pub fog_near_raw: u16,
    pub fog_far: i16,
}

impl EnvLightSettings {
    pub fn fog_near(&self) -> i16 {
        (self.fog_near_raw & 0x3FF) as i16
    }
}

/// `Spawn` from `SCENE_CMD_ID_ENTRANCE_LIST` (`play->setupEntranceList`): for spawn number n
/// (`play->curSpawn`), which player entry of the spawn list to use and the room it starts in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EntranceEntry {
    pub spawn: u8,
    pub room: u8,
}

/// `TransitionActorEntry` from `SCENE_CMD_ID_TRANSITION_ACTOR_LIST` (0x10 bytes): an actor
/// between two rooms (a door, a room-change plane), with each side's room and bg camera.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TransitionActorEntry {
    /// `sides[0]` (front) and `sides[1]` (back): `(room, bgCamIndex)`; room -1 is none.
    pub sides: [(i8, i8); 2],
    pub id: i16,
    pub pos: [i16; 3],
    pub rot_y: i16,
    pub params: i16,
}

/// `TRANSITION_ACTOR_PARAMS_INDEX_SHIFT`: `Actor_SpawnTransitionActors` puts the entry's index
/// in the params' top bits.
pub const TRANSITION_ACTOR_PARAMS_INDEX_SHIFT: u32 = 10;

/// A `DEFINE_ENTRANCE` row of `include/tables/entrance_table.h` (`EntranceInfo`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EntranceInfo {
    /// `ENTR_LINK_HOME_0`.
    pub name: String,
    /// `SCENE_*` value.
    pub scene: u16,
    pub spawn: u8,
    pub continue_bgm: bool,
    pub title_card: bool,
    /// `TRANS_TYPE_*` when entering by this entrance (`ENTRANCE_INFO_END_TRANS_TYPE`) and when
    /// leaving for it (`ENTRANCE_INFO_START_TRANS_TYPE`).
    pub end_trans_type: u8,
    pub start_trans_type: u8,
}

/// `SCmdSkyboxSettings`: skybox id, skybox config and `LIGHT_MODE_*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct SkyboxSettings {
    pub skybox_id: u8,
    pub config: u8,
    pub light_mode: u8,
}

/// `ROOM_SHAPE_TYPE_*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ShapeKind {
    /// `ROOM_SHAPE_TYPE_NORMAL`: OPA/XLU display-list pairs.
    Normal,
    /// `ROOM_SHAPE_TYPE_IMAGE`: one pair plus prerendered backgrounds (`RoomData::backgrounds`).
    Image,
    /// `ROOM_SHAPE_TYPE_CULLABLE`: pairs with a bounding sphere, z-sorted and culled every
    /// frame (`Room_DrawCullable`, `crate::room::cullable_order`).
    Cullable,
}

/// A `DEFINE_SCENE` row of `include/tables/scene_table.h`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SceneEntry {
    /// `SCENE_*` value (the row index).
    pub id: u16,
    /// The scene file, e.g. `spot04_scene`.
    pub file: String,
    /// `SCENE_SPOT04`.
    pub enum_name: String,
    /// `SDC_SPOT04`: the scene draw config (`crate::scene_table`).
    pub draw_config: String,
}

/// `scene_table.h` and `object_table.h`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SceneTable {
    pub scenes: Vec<SceneEntry>,
    /// Object id → file name ("" for `DEFINE_OBJECT_UNSET`).
    pub objects: Vec<String>,
    /// `gEntranceTable`: entrance index → scene and spawn.
    pub entrances: Vec<EntranceInfo>,
    /// `Skybox_Setup`'s room skyboxes (`unk_140 != 0`), baked as `skybox::bake_name`.
    pub room_skyboxes: Vec<crate::skybox::RoomSkybox>,
}

impl SceneTable {
    pub fn by_file(&self, file: &str) -> Option<&SceneEntry> {
        self.scenes.iter().find(|s| s.file == file)
    }

    /// The `OBJECT_*` id of an object file.
    pub fn object_id(&self, file: &str) -> Option<i16> {
        self.objects.iter().position(|o| o == file).map(|i| i as i16)
    }

    /// The room skybox with this `SKYBOX_*` id.
    pub fn room_skybox(&self, id: u8) -> Option<&crate::skybox::RoomSkybox> {
        self.room_skyboxes.iter().find(|s| s.id == id)
    }

    /// An entrance by its `ENTR_*` name.
    pub fn entrance_index(&self, name: &str) -> Option<u16> {
        self.entrances.iter().position(|e| e.name == name).map(|i| i as u16)
    }
}

/// One scene: its entry in the scene table and its header for each game layer.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SceneData {
    pub name: String,
    pub id: u16,
    pub draw_config: String,
    /// `layers[l]` for `gSaveContext.sceneLayer` l: the four game layers, then each cutscene
    /// layer up to the last one the scene's alternate header list names. A layer the scene has
    /// no header for falls back the way `Scene_CommandAlternateHeaderList` does (adult night to
    /// adult day, anything else to the main header), so every entry is filled.
    pub layers: Vec<LayerData>,
}

impl SceneData {
    /// The header `Play_SpawnScene` executes for `gSaveContext.sceneLayer` `layer`, and the
    /// layer it's stored as. Past the scene's alternate header list the C reads whatever follows
    /// the list; the main header stands in (logged).
    pub fn layer(&self, layer: usize) -> (usize, &LayerData) {
        match self.layers.get(layer) {
            Some(l) => (layer, l),
            None => {
                log::error!("{}: no scene layer {layer} (the alternate header list has {}): the main header", self.name, self.layers.len().saturating_sub(1));
                (0, &self.layers[0])
            }
        }
    }
}

/// A scene header as loaded for one layer.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LayerData {
    /// Offset of the header in the scene file (0 = the main header); layers sharing a header
    /// have the same offset.
    pub header_offset: u32,
    /// Record name of the collision header (`CollisionHeader`).
    pub collision: String,
    pub spawns: Vec<ActorEntry>,
    pub light_settings: Vec<EnvLightSettings>,
    pub skybox: SkyboxSettings,
    /// `SCENE_CMD_ID_SPECIAL_FILES`: the keep object's file.
    pub keep_object: Option<String>,
    /// Its `OBJECT_*` id (`objectCtx.subKeepIndex`'s object).
    pub keep_object_id: Option<i16>,
    /// `SCENE_CMD_ID_SPECIAL_FILES`' `cUpElfMsgNum`: 0 for none, else 1 + the index into
    /// `sNaviMsgFiles` of Navi's C-Up texts (`play->cUpElfMsgs`, `elf_message::ElfMessageTables`).
    pub c_up_elf_msg_num: u8,
    /// `SCENE_CMD_ID_ENTRANCE_LIST`: indexed by spawn number (`play->curSpawn`).
    pub entrances: Vec<EntranceEntry>,
    /// `SCENE_CMD_ID_EXIT_LIST`: exit index − 1 → entrance index (`play->setupExitList`).
    pub exits: Vec<u16>,
    /// `SCENE_CMD_ID_TRANSITION_ACTOR_LIST`.
    pub transition_actors: Vec<TransitionActorEntry>,
    /// `SCENE_CMD_ID_PATH_LIST`: `play->setupPathList`, indexed by the number an actor's
    /// params carry (`En_Goroiwa`'s `params & 0xFF`).
    pub paths: Vec<Path>,
    /// `SCENE_CMD_ID_MISC_SETTINGS`' `sceneCamType`: `R_SCENE_CAM_TYPE` (`SCENE_CAM_TYPE_*`).
    pub scene_cam_type: u8,
    /// `SCENE_CMD_ID_SOUND_SETTINGS` (`Scene_CommandSoundSettings`), if the header has it.
    pub sound: Option<SoundSettings>,
    /// `SCENE_CMD_ID_CUTSCENE_DATA`: the pack key of the script `Scene_CommandCutsceneData` puts
    /// in `play->csCtx.segment` (`keys::cutscene`), if the header has one.
    pub cutscene: Option<String>,
    /// Record names of the rooms (`RoomData`), in room-list order.
    pub rooms: Vec<String>,
    /// The day time the meshes were built for (`gSaveContext.dayTime`, see `oot_import`'s
    /// scene import), and anything the build reported.
    pub bake_day_time: u16,
    pub notes: Vec<String>,
}

/// `SCmdSoundSettings` (`SCENE_CMD_SOUND_SETTINGS(specId, natureAmbienceId, seqId)`): the audio
/// spec, the nature ambience (`NatureAmbienceId`) and the music (`NA_BGM_*`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SoundSettings {
    pub spec_id: u8,
    pub nature_ambience_id: u8,
    pub seq_id: u8,
}

/// One room as loaded for a scene layer: its header and its shape's meshes.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RoomData {
    pub file: String,
    pub index: usize,
    /// `SCENE_CMD_ID_ROOM_BEHAVIOR`: behaviorType1, behaviorType2.
    pub behavior: [u8; 2],
    pub echo: u8,
    /// `SCENE_CMD_ID_TIME_SETTINGS`: hour, minute, time speed (0xFF = keep).
    pub time: Option<[u8; 3]>,
    pub skybox_disabled: bool,
    pub sun_moon_disabled: bool,
    pub actors: Vec<ActorEntry>,
    /// `SCENE_CMD_ID_OBJECT_LIST`: the `OBJECT_*` ids the room loads.
    pub objects: Vec<i16>,
    pub shape: Option<ShapeKind>,
    pub entries: Vec<EntryMesh>,
    /// `ROOM_SHAPE_TYPE_IMAGE`: the prerendered backgrounds, decoded at import
    /// (docs/adr/0014-prerendered-backgrounds.md).
    pub backgrounds: Vec<RoomBackground>,
}

/// A prerendered background of an image room: `RoomShapeImageSingle`'s image, or one
/// `RoomShapeImageMultiBgEntry`. The mesh is a screen quad drawn in the orthographic overlay
/// space (`eng_gfx::DrawParams::screen`), in copy mode.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RoomBackground {
    /// `ROOM_SHAPE_IMAGE_AMOUNT_MULTI`: the bg camera this one is for (`bgEntry->bgCamIndex`).
    pub bg_cam_index: Option<u8>,
    pub mesh: DrawList,
}

/// `R_SCENE_CAM_TYPE` values (`SCENE_CAM_TYPE_*`, `z64scene.h`).
pub const SCENE_CAM_TYPE_DEFAULT: u8 = 0x00;
pub const SCENE_CAM_TYPE_FIXED_SHOP_VIEWPOINT: u8 = 0x10;
pub const SCENE_CAM_TYPE_FIXED_TOGGLE_VIEWPOINT: u8 = 0x20;
pub const SCENE_CAM_TYPE_FIXED: u8 = 0x30;
pub const SCENE_CAM_TYPE_FIXED_MARKET: u8 = 0x40;

/// One entry of a room shape, interpreted: the OPA and XLU meshes, and for cullable shapes the
/// bounding sphere.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EntryMesh {
    pub bounds: Option<(Vec3, f32)>,
    pub opa: Option<DrawList>,
    pub xlu: Option<DrawList>,
}

impl RoomData {
    pub fn triangles(&self) -> usize {
        self.entries.iter().flat_map(|e| [&e.opa, &e.xlu]).flatten().map(|d| d.triangle_count()).sum()
    }
}
