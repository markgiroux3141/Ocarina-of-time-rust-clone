//! Player movement, ported from `ovl_player_actor/z_player.c` (the decomp commit in use still
//! calls most of these `func_808xxxxx`; each port names its original).
//!
//! Scope: standing, turning in place, walking/running with the walk/run blend, rolling on A
//! (including bonking into walls), running off ledges (auto-jump or fall), air control,
//! landing (including fall-damage stagger and landing rolls), and the head/torso look and
//! lean rotations. Everything that needs items, targeting, water, cutscenes, other actors,
//! ledge hanging or climbing is left out. The interrupt checks for those are stubbed to
//! "not taken", and the list is in the spike 03 findings.
//!
//! Naming: fields keep the decomp's `unk_XXX` names where the decomp has no name yet, so the
//! port can be diffed against the C. File-scope statics (`sControlStickMagnitude` ...) live in
//! `PlayerStatics`.

#![allow(non_snake_case)] // fields keep the decomp's unk_XXX names

use eng_collision::bgcheck::{BGCHECK_Y_MIN, CollisionContext, PolyId, udist_plane_to_pos};
use eng_collision::dyna::BGCHECK_SCENE;
use eng_input::pad::{BTN_A, BTN_Z, Input, stick_to_mag_angle};
use eng_math::*;
use glam::Vec3;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTOR_PLAYER, ACTORCAT_NPC, ACTORCAT_PLAYER, ActorContext, ActorHandle, ActorImpl, PLAYER_BODYPART_WAIST, PlayerIface};
use oot_game::camera::CAM_ID_MAIN;
use oot_game::cutscene::CsCmdActorCue;
use oot_game::play_scene::{PlayIo, SCENE_GANONS_TOWER_COLLAPSE_EXTERIOR, SCENE_SHADOW_TEMPLE, SCENE_KOKIRI_FOREST};
use oot_game::save::{RESPAWN_MODE_DOWN, RESPAWN_MODE_RETURN};
use oot_game::scene::EntranceInfo;
use oot_game::transition::{TRANS_TRIGGER_OFF, TRANS_TRIGGER_START, TRANS_TYPE_FADE_BLACK, TRANS_TYPE_FADE_BLACK_FAST, TRANS_TYPE_FADE_WHITE};
use std::cell::RefCell;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::data::{AgeProperties, AnimId, GameData, Regs};
use oot_game::player_lib::Blinker;
use oot_game::skelanime::{ANIMMODE_LOOP, ANIMMODE_ONCE, SkelAnime};
use oot_game::surface::{SurfaceType, WALL_FLAG_0, WALL_FLAG_1, WALL_FLAG_2, WALL_FLAG_3, WALL_FLAG_CRAWLSPACE_1, WALL_FLAG_CRAWLSPACE_2, WALL_FLAG_6};
use oot_game::target::{ACTOR_FLAG_LOCK_ON_DISABLED, TargetView};
use oot_game::collision_check::{self as cc, ColliderCylinder, ColliderCylinderInit, ColliderElementInit, ColliderInit, ColliderMut, ColliderQuad, ColliderQuadInit, ColliderElementDamageInfoAT, ColliderElementDamageInfoACInit};
use eng_collision::math3d::Cylinder16;

// stateFlags1
pub const STATE1_0: u32 = 1 << 0; // going through an exit
pub const STATE1_2: u32 = 1 << 2;
pub const STATE1_3: u32 = 1 << 3;
/// `PLAYER_STATE1_9`: the bow's or slingshot's string drawn back (`func_8083442C`, `func_808351D4`).
pub const STATE1_9: u32 = 1 << 9;
pub const STATE1_8: u32 = 1 << 8; // item change pending
/// `sItemButtons`: B, C-Left, C-Down, C-Right (`Player_GetItemOnButton`'s 0 to 3).
const S_ITEM_BUTTONS: [u16; 4] = [eng_input::pad::BTN_B, eng_input::pad::BTN_CLEFT, eng_input::pad::BTN_CDOWN, eng_input::pad::BTN_CRIGHT];
/// `PLAYER_MASK_NONE` (`player.h`).
const PLAYER_MASK_NONE: u8 = 0;
/// `PLAYER_ITEM_CHG_13` (`z_player.c`'s `ItemChangeType`): the bottle's and boomerang's change.
const PLAYER_ITEM_CHG_13: i32 = 13;
pub const STATE1_10: u32 = 1 << 10; // getting an item
pub const STATE1_24: u32 = 1 << 24;
pub const STATE1_4: u32 = 1 << 4; // locked on (Player_CheckHostileLockOn)
pub const STATE1_7: u32 = 1 << 7; // dead
pub const STATE1_15: u32 = 1 << 15; // Z pressed this lock-on
pub const STATE1_23: u32 = 1 << 23; // riding
pub const STATE1_25: u32 = 1 << 25; // boomerang in flight
pub const STATE1_6: u32 = 1 << 6;
pub const STATE1_11: u32 = 1 << 11; // holding an actor
pub const STATE1_12: u32 = 1 << 12;
pub const STATE1_13: u32 = 1 << 13;
pub const STATE1_14: u32 = 1 << 14;
pub const STATE1_16: u32 = 1 << 16;
pub const STATE1_17: u32 = 1 << 17;
pub const STATE1_18: u32 = 1 << 18; // jumping
pub const STATE1_19: u32 = 1 << 19; // freefall
pub const STATE1_20: u32 = 1 << 20;
pub const STATE1_21: u32 = 1 << 21;
pub const STATE1_22: u32 = 1 << 22; // shielding
pub const STATE1_26: u32 = 1 << 26;
pub const STATE1_27: u32 = 1 << 27; // swimming
pub const STATE1_28: u32 = 1 << 28;
pub const STATE1_29: u32 = 1 << 29;
pub const STATE1_30: u32 = 1 << 30;
pub const STATE1_31: u32 = 1 << 31;
// stateFlags2
pub const STATE2_0: u32 = 1 << 0;
pub const STATE2_10: u32 = 1 << 10; // under the water surface
pub const STATE2_11: u32 = 1 << 11; // diving from the surface
pub const STATE2_1: u32 = 1 << 1;
pub const STATE2_13: u32 = 1 << 13; // keep the target (Switch Z-targeting)
pub const STATE2_21: u32 = 1 << 21;
pub const STATE2_2: u32 = 1 << 2;
pub const STATE2_3: u32 = 1 << 3;
/// `PLAYER_STATE2_4`: pushing or pulling a block that takes it (the block clears it when it stops).
pub const STATE2_4: u32 = 1 << 4;
pub const STATE2_5: u32 = 1 << 5; // facing follows currentYaw
pub const STATE2_6: u32 = 1 << 6;
pub const STATE2_8: u32 = 1 << 8;
pub const STATE2_9: u32 = 1 << 9;
pub const STATE2_12: u32 = 1 << 12;
pub const STATE2_14: u32 = 1 << 14;
pub const STATE2_16: u32 = 1 << 16;
pub const STATE2_18: u32 = 1 << 18;
pub const STATE2_19: u32 = 1 << 19; // side hop / backflip
pub const STATE2_22: u32 = 1 << 22;
pub const STATE2_26: u32 = 1 << 26;
pub const STATE2_27: u32 = 1 << 27;
pub const STATE2_28: u32 = 1 << 28; // idle fidget
pub const STATE2_17: u32 = 1 << 17; // spin attack
pub const STATE2_30: u32 = 1 << 30; // stab lunge
pub const STATE2_7: u32 = 1 << 7;
pub const STATE2_31: u32 = 1 << 31;
/// `PLAYER_STATE2_29`: not drawn (`Player_Draw`), while Link browses a shop's shelves
/// (`En_Ossan`).
pub const STATE2_29: u32 = 1 << 29;
// stateFlags3
pub const STATE3_1: u32 = 1 << 1;
pub const STATE3_3: u32 = 1 << 3;
pub const STATE3_4: u32 = 1 << 4;
pub const STATE3_7: u32 = 1 << 7;

// PlayerDoorType.
pub const PLAYER_DOORTYPE_AJAR: i8 = -1;
pub const PLAYER_DOORTYPE_NONE: i8 = 0;
pub const PLAYER_DOORTYPE_HANDLE: i8 = 1;
pub const PLAYER_DOORTYPE_SLIDING: i8 = 2;
pub const PLAYER_DOORTYPE_FAKE: i8 = 3;

/// What Player does to the play state, the camera and other actors during its update, applied
/// in order right after it (Player is out of the actor context while it updates; the C calls
/// these in place).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayRequest {
    /// `Player_RequestCameraSetting`: `Camera_RequestSetting` on the main camera, unless the scene's camera
    /// is fixed (`Play_CamIsNotFixed`), where `CAM_SET_SCENE_TRANSITION` only changes the
    /// interface alpha.
    CamSetting(i16),
    /// `Camera_RequestSetting(Play_GetCamera(play, CAM_ID_MAIN), setting)`.
    ChangeSetting(i16),
    /// `Camera_ChangeDoorCam(mainCam, door, bgCamIndex, 0, timer1, timer2, timer3)`.
    DoorCam { door: ActorHandle, bg_cam_index: i16, timers: [i16; 3] },
    /// `Camera_SetFinishedFlag(mainCam)`.
    CamDone,
    /// `Room_RequestNewRoom`: load a room.
    RoomLoad(i8),
    /// `Room_FinishRoomChange`: the previous room goes.
    RoomChangeDone,
    /// The door's `openAnim` and `playerIsOpening = true`.
    OpenDoor { door: ActorHandle, open_anim: u8 },
    /// `slidingDoor->isActive = true`: Player opens a sliding door (`Door_Shutter`).
    SlidingDoorActive(ActorHandle),
    /// `doorActor->room = play->roomCtx.curRoom.num`, and its double's.
    DoorRoom { door: ActorHandle },
    /// `actor->flags |= ACTOR_FLAG_TALK`: Player accepted the actor's talk offer (the actor's
    /// `Actor_TalkOfferAccepted` sees it in its update this frame).
    TalkRequest(ActorHandle),
    /// `actor->textId = text_id` (an ajar door's 0xD0).
    SetTextId { actor: ActorHandle, text_id: u16 },
    /// `Message_StartTextbox(play, textId, actor)`.
    StartTextbox { text_id: u16, actor: Option<ActorHandle> },
    /// `Interface_SetDoAction(play, action)`.
    DoAction(u16),
    /// `interactedActor->parent = &this->actor` (`Player_ActionHandler_2`): Player took the actor's
    /// item (its `Actor_HasParent` is true from now).
    SetParent(ActorHandle),
    /// `chest->unk_1F4 = v`: Player opens the chest, with the long (1) or short (-1) animation.
    ChestOpen { chest: ActorHandle, unk_1f4: i16 },
    /// `Camera_SetCameraData(Play_GetCamera(play, CAM_ID_MAIN), 4, NULL, NULL, data2, 0, 0)`.
    SetCameraData { data2: i16 },
    /// `Item_DropCollectible(play, &pos, params)` (`func_8083E4C4`).
    DropCollectible { pos: Vec3, params: i16 },
    /// `Item_Give(play, item)`, in its place among the requests (after `Message_StartTextbox`,
    /// whose heart piece text counts the pieces before this one).
    ItemGive(u8),
    /// `Interface_SetNaviCall(play, naviCallState)`.
    NaviCall(u16),
    /// `func_8084DFF4`'s sound for item `get_item_id`, after its `Item_Give` (a heart piece's
    /// fanfare counts the pieces with this one): `Audio_PlayFanfare`, or a sound effect for a
    /// rupee or a heart.
    GetItemFanfare(i16),
    /// `func_800F6964(arg)`: every player fades out (the secret hole's exit).
    FadeOutAllSeq(u16),
    /// A sound, in its order among Player's calls.
    Sfx(PlayerSfx),
    /// `OnePointCutscene_Init(play, cs_id, timer, actor, parent)`, the actor Player himself
    /// (`player`) or NULL.
    OnePointCutscene { cs_id: i16, timer: i16, player: bool, parent: i16 },
    /// `play->gameOverCtx.state = state` (`func_80836448`, `func_80843AE8`).
    GameOverState(u16),
    /// `Letterbox_SetSizeTarget(size)`.
    LetterboxSizeTarget(i32),
    /// `Player_SpawnFairy(play, this, &pos, &offset, FAIRY_REVIVE_DEATH)`: the bottled fairy that
    /// revives Link, at `pos` (the offset applied).
    SpawnReviveFairy(Vec3),
    /// `func_8083819C`'s burnt Deku Shield: `Actor_Spawn(ACTOR_ITEM_SHIELD, pos, params 1)` and
    /// `Message_StartTextbox(play, 0x305F, NULL)` (`Inventory_DeleteEquipment` ran in place).
    BurnDekuShield(Vec3),
    /// The death's and the revival's audio calls.
    Audio(PlayerAudio),
    /// `CollisionCheck_SpawnShieldParticles` (`metal`: with its light) or the particles of
    /// `CollisionCheck_SpawnShieldParticlesWood` (its sound is a `Sfx`): the sword on a wall
    /// (`func_80842DF4`).
    ShieldParticles { pos: Vec3, metal: bool },
    /// `func_80832630`: a hit's freeze, `actorCtx.freezeFlashTimer` 1 unless one runs.
    FreezeFlash,
    /// `sBloodFuncs[kind]` at `pos` (`func_8002F9EC`'s `CollisionCheck_BlueBlood`).
    Blood { kind: u8, pos: Vec3 },
    /// `Player_RequestQuake(play, speed, y, duration)`: a decaying quake (type 3) on the main
    /// camera (`crate::quake`).
    Quake { speed: i32, y: i32, duration: i32 },
    /// `TitleCard_Clear(play, &play->actorCtx.titleCtx)` (`Player_UseItem`'s cutscene items).
    TitleCardClear,
    /// `this->heldActor = Actor_SpawnAsChild(&play->actorCtx, &this->actor, play, ACTOR_EN_ARROW,
    /// world.pos, 0, shape.rot.y, 0, params)` (`func_8083442C`): the seed or arrow held, Player's
    /// child. Applied by Player's update, which sets `heldActor`.
    SpawnHeldArrow { pos: Vec3, yaw: i16, params: i16 },
    /// `heldActor->parent = NULL`: Player lets go of what he held (`Player_DetachHeldActor`,
    /// `func_808350A4`'s shot).
    ReleaseHeld(ActorHandle),
    /// `Actor_Spawn(&play->actorCtx, play, ACTOR_EN_ARROW, pos, rot, params)`: a Deku nut thrown
    /// (`Player_Action_8084E604`).
    SpawnArrow { pos: Vec3, rot: [i16; 3], params: i16 },
    /// `Camera_RequestMode(Play_GetCamera(play, CAM_ID_MAIN), mode)` made inside Player's update
    /// (`func_8083AD4C`), which used its result already (`Player::camera_request_mode`).
    CamRequestMode(i16),
}

/// The audio calls of Player's death and revival.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayerAudio {
    /// `Audio_SetBgmVolumeOffDuringFanfare`.
    BgmVolumeOffDuringFanfare,
    /// `Audio_SetBgmVolumeOnDuringFanfare`.
    BgmVolumeOnDuringFanfare,
    /// `Audio_StopBgmAndFanfare(fadeOutDuration)`.
    StopBgmAndFanfare(u16),
    /// `Audio_PlayFanfare(seqId)`.
    PlayFanfare(u16),
}

/// `PlayerHitResponseType` (`player.h`): `func_80837C0C`'s reactions.
pub const PLAYER_HIT_RESPONSE_NONE: i32 = 0;
pub const PLAYER_HIT_RESPONSE_KNOCKBACK_LARGE: i32 = 1;
pub const PLAYER_HIT_RESPONSE_KNOCKBACK_SMALL: i32 = 2;
pub const PLAYER_HIT_RESPONSE_FROZEN: i32 = 3;
pub const PLAYER_HIT_RESPONSE_ELECTRIFIED: i32 = 4;

/// `PlayerKnockbackType` (`player.h`): what `Actor_SetPlayerKnockback` asks for.
pub const PLAYER_KNOCKBACK_NONE: u8 = 0;
pub const PLAYER_KNOCKBACK_SMALL: u8 = 1;
pub const PLAYER_KNOCKBACK_LARGE: u8 = 2;
pub const PLAYER_KNOCKBACK_LARGE_ELECTRIFIED: u8 = 3;

/// `PLAYER_TUNIC_GORON`, `PLAYER_SHIELD_DEKU` (`player.h`).
const PLAYER_TUNIC_GORON: u8 = 1;
const PLAYER_SHIELD_DEKU: u8 = 1;
const PLAYER_SHIELD_HYLIAN: u8 = 2;
/// `ROOM_ENV_HOT` (`room.h`): a hot room's `environmentType` (`behaviorType2`).
const ROOM_ENV_HOT: u8 = 3;
/// `SCENE_SPIRIT_TEMPLE_BOSS` (`scene_table.h`).
const SCENE_SPIRIT_TEMPLE_BOSS: u16 = 0x17;

/// `D_80854398`: the string drawn back, by `unk_860`: `NA_SE_IT_BOW_DRAW`, `NA_SE_IT_SLING_DRAW`,
/// `NA_SE_IT_HOOKSHOT_READY` (`itembank_table.h`: 0x1807, 0x1821, 0x1827).
const D_80854398: [u16; 3] = [0x1807, 0x1821, 0x1827];
/// `D_808543DC`: let go with nothing loaded: `NA_SE_IT_BOW_FLICK`, `NA_SE_IT_SLING_FLICK` (0x1830,
/// 0x1835).
const D_808543DC: [u16; 2] = [0x1830, 0x1835];
/// `NA_SE_IT_HOOKSHOT_RECEIVE` (`itembank_table.h`: 0x1828).
const NA_SE_IT_HOOKSHOT_RECEIVE: u16 = 0x1828;
/// `D_808543CC` and `D_808543D4`: the bow's and the hookshot's raise from walking, and their wait.
const D_808543CC: [&str; 2] = ["link_bow_walk2ready", "link_hook_walk2ready"];
const D_808543D4: [&str; 2] = ["link_bow_bow_wait", "link_hook_wait"];
/// `ACTOR_EN_ARROW` (`actor_table.h`: 0x0016).
pub const ACTOR_EN_ARROW: i16 = 0x0016;
/// `ArrowType` (`z_en_arrow.h`).
pub const ARROW_NORMAL_HORSE: i16 = 1;
pub const ARROW_NORMAL: i16 = 2;
pub const ARROW_FIRE: i16 = 3;
pub const ARROW_SEED: i16 = 9;
pub const ARROW_NUT: i16 = 10;
/// `ROOM_TYPE_INDOORS` (`room.h`): `roomCtx.curRoom.type` (the room's `behaviorType1`).
const ROOM_TYPE_INDOORS: u8 = 2;
/// `UNK6AE_ROT_*` (`player.h`).
const UNK6AE_ROT_FOCUS_X: u16 = 1 << 0;
const UNK6AE_ROT_FOCUS_Y: u16 = 1 << 1;
const UNK6AE_ROT_UPPER_X: u16 = 1 << 6;
const UNK6AE_ROT_UPPER_Z: u16 = 1 << 8;
/// `CAM_STATE_LOCK_MODE` (`camera.h`): the main camera takes no mode request.
const CAM_STATE_LOCK_MODE: i16 = 1 << 5;
/// `BowSlingshotStringData` (`Player_PostLimbDrawGameplay`, `sBowSlingshotStringData`): the
/// adult's bow string and the child's slingshot string, and where they hang from the right hand.
const BOW_SLINGSHOT_STRING: [(&str, &str, Vec3); 2] =
    [("object_link_boy", "gLinkAdultBowStringDL", Vec3::new(0.0, -360.4, 0.0)), ("object_link_child", "gLinkChildSlingshotStringDL", Vec3::new(606.0, 236.0, 0.0))];

/// The ice round frozen Link (`Player_Draw` under `PLAYER_STATE2_14`): `gEffIceFragment3DL` with
/// `Gfx_TwoTexScroll` on segment 8 and `gDPSetEnvColor(0, 50, 100, 255)`.
const ICE_BAKE: &str = "Player/ice";
const SEG_ICE_SCROLL: u8 = 0x08;
const SEG_ICE_ENV: u8 = 0x0C;

/// `Player_Draw`'s ice scroll at `gameplay_frames`: `Gfx_TwoTexScroll(G_TX_RENDERTILE, 0,
/// (0 - gameplayFrames) % 128, 32, 32, 1, 0, (gameplayFrames * -2) % 128, 32, 32)` (u32 arithmetic).
fn ice_scroll(gameplay_frames: u32) -> Vec<(u32, u32)> {
    let y1 = 0u32.wrapping_sub(gameplay_frames) % 128;
    let y2 = gameplay_frames.wrapping_mul((-2i32) as u32) % 128;
    oot_game::scene_table::gfx_two_tex_scroll(0, 0, y1, 32, 32, 1, 0, y2, 32, 32)
}

/// Player's own bakes (docs/adr/0012-actor-bakes.md): the frozen ice.
pub fn bakes() -> Vec<oot_game::pack::MeshBake> {
    use oot_game::pack::{BakeBody, BakeSegment, MeshBake};
    vec![MeshBake {
        name: ICE_BAKE.into(),
        object: "gameplay_keep".into(),
        segments: vec![(SEG_ICE_ENV, BakeSegment::Commands(vec![(0xFB00_0000, 0x0032_64FF), (0xDF00_0000, 0)])), (SEG_ICE_SCROLL, BakeSegment::Dynamic(ice_scroll(0)))],
        prelude: vec![SEG_ICE_ENV],
        body: BakeBody::DLists(vec![("gameplay_keep".into(), "gEffIceFragment3DL".into())]),
    }]
}

/// Player's sounds: what its `Player_PlaySfx`-style calls ask the audio for, at Player
/// (`&this->actor.projectedPos`) unless said.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayerSfx {
    /// `Sfx_PlaySfxCentered`: no position.
    NoPos(u16),
    /// `Player_PlaySfx`: `Audio_PlaySfxGeneral` at Player, the default scales.
    Actor(u16),
    /// `func_800F4010`: a footstep at speed `f32`.
    Footstep(u16, f32),
    /// `func_800F4138`, `func_800F4190`.
    F4138(u16, f32),
    F4190(u16),
    /// `Audio_SetCodeReverb`: the floor's echo.
    CodeReverb(i8),
    /// `Audio_StopSfxById`.
    StopById(u16),
    /// `Audio_SetBaseFilter`: the underwater filter (and the bubbling).
    BaseFilter(u8),
}

/// `SurfaceMaterial` (`bgcheck.h`): the ones Player names.
const SURFACE_MATERIAL_SAND: u16 = 1;
/// `SURFACE_MATERIAL_WOOD`, `SURFACE_MATERIAL_DIRT_SOFT` (`bgcheck.h`).
const SURFACE_MATERIAL_WOOD: u16 = 10;
const SURFACE_MATERIAL_DIRT_SOFT: u16 = 11;
const SURFACE_MATERIAL_WATER_SHALLOW: u16 = 4;
const SURFACE_MATERIAL_WATER_DEEP: u16 = 5;
/// `PLAYER_BOOTS_IRON` (`player.h`).
const PLAYER_BOOTS_IRON: u8 = 1;
use oot_game::audio::sfx::*;

// floor properties (FLOOR_PROPERTY_*)
const FLOOR_PROPERTY_5: u32 = 5;
const FLOOR_PROPERTY_12: u32 = 12;
const FLOOR_TYPE_4: u32 = 4;
const FLOOR_TYPE_10: u32 = 10;
const FLOOR_TYPE_11: u32 = 11;
const FLOOR_TYPE_12: u32 = 12;
const FLOOR_EFFECT_2: u32 = 2;
/// `ENTR_RETURN_GREAT_FAIRYS_FOUNTAIN_SPELLS`, `ENTR_RETURN_GROTTO` (`scene.h`).
const ENTR_RETURN_GREAT_FAIRYS_FOUNTAIN_SPELLS: u16 = 0x7FF9;
const ENTR_RETURN_GROTTO: u16 = 0x7FFF;
const FLOOR_PROPERTY_6: u32 = 6;
const FLOOR_PROPERTY_7: u32 = 7;
const FLOOR_PROPERTY_8: u32 = 8;
const FLOOR_PROPERTY_9: u32 = 9;
const FLOOR_TYPE_6: u32 = 6;
const FLOOR_TYPE_8: u32 = 8;
const FLOOR_TYPE_7: u32 = 7;
const FLOOR_TYPE_9: u32 = 9;
const FLOOR_TYPE_2: u32 = 2;
const FLOOR_TYPE_3: u32 = 3;
/// `SCENE_FOREST_TEMPLE` (`scene_table.h`): the Forest Temple.
const SCENE_FOREST_TEMPLE: u16 = 0x03;

/// `PLAYER_ANIMGROUP_*` indices used here (group 0 = wait, 1 = walk, 2 = run, ...).
pub mod group {
    pub const WAIT: usize = 0;
    /// `PLAYER_ANIMGROUP_defense`, `_defense_wait`, `_defense_end` (`player.h`).
    pub const DEFENSE: usize = 0x14;
    pub const DEFENSE_WAIT: usize = 0x15;
    pub const DEFENSE_END: usize = 0x16;
    /// `PLAYER_ANIMGROUP_pull_start`, `_pulling`, `_pull_end` (`player.h`).
    pub const PULL_START: usize = 0x23;
    pub const PULLING: usize = 0x24;
    pub const PULL_END: usize = 0x25;
    pub const WALK: usize = 1;
    pub const RUN: usize = 2;
    pub const DAMAGE_RUN: usize = 3;
    pub const HEAVY_RUN: usize = 4;
    pub const LANDING: usize = 14;
    pub const SHORT_LANDING: usize = 15;
    pub const ROLL: usize = 16;
    pub const ROLL_BONK: usize = 17;
    pub const END_WALK_A: usize = 18;
    pub const END_WALK_B: usize = 19;
    pub const TURN: usize = 26;
    pub const TARGET_EXIT_R: usize = 27;
    pub const TARGET_EXIT_L: usize = 28;
    pub const PARALLEL_BACKWALK: usize = 31;
    pub const TARGET_WAIT_L: usize = 5;
    pub const TARGET_WAIT_R: usize = 6;
    pub const TARGET_ENTER: usize = 7;
    pub const SHORT_LANDING_: usize = 15;
    pub const PARALLEL_SIDEWALK: usize = 23;
    pub const SIDESTEP_R: usize = 24;
    pub const SIDESTEP_L: usize = 25;
    /// Climb up from a hang (`av1.actionVar1` > 0).
    pub const HANG_CLIMB_UP: usize = 38;
    /// Mid-air ledge grab.
    pub const HANG_GRAB: usize = 39;
    /// Hanging loop after a mid-air grab.
    pub const HANG_WAIT: usize = 40;
    /// Climb up after a mid-air grab (`av1.actionVar1` < 0).
    pub const HANG_GRAB_CLIMB_UP: usize = 41;
}

/// The action function in `this->func_674`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `Player_Action_Idle`: standing (idle, landing, end of roll).
    StandingStill,
    /// `Player_Action_80842180`: walking and running.
    Run,
    /// `Player_Action_TurnInPlace`: turning in place.
    Turn,
    /// `Player_Action_Roll`: rolling.
    Roll,
    /// `Player_Action_8084411C`: in the air (jumping or falling).
    Midair,
    /// `Player_Action_8084BBE4`: hanging from a ledge.
    Hang,
    /// `Player_Action_8084BDFC`: climbing up from a hang.
    ClimbUp,
    /// `Player_Action_80845668`: stepping or jumping up onto a ledge from the ground.
    ClimbLedge,
    /// `Player_Action_80840450`: standing locked on to a target.
    TargetIdle,
    /// `Player_Action_808407CC`: standing in parallel mode (Z with nothing to target).
    ParallelIdle,
    /// `Player_Action_8084227C`: running forward while targeting.
    TargetRun,
    /// `Player_Action_8084193C`: sidestepping around a target.
    Sidestep,
    /// `Player_Action_808423EC`: stepping back from a locked target.
    TargetBackwalk,
    /// `Player_Action_8084251C`: braking after the step back.
    TargetBackBrake,
    /// `Player_Action_80840DE4`: walking sideways in parallel mode.
    ParallelWalk,
    /// `Player_Action_808414F8`: walking/running backwards in parallel mode.
    ParallelBackwalk,
    /// `Player_Action_8084170C`: braking out of a parallel back run.
    ParallelBackBrake,
    /// `Player_Action_808417FC`: end of that brake.
    ParallelBackBrakeEnd,
    /// `Player_Action_808502D0`: a melee attack (sword slash).
    Attack,
    /// `Player_Action_8084D610`: treading water.
    Swim,
    /// `Player_Action_8084D84C`: swimming forward.
    SwimMove,
    /// `Player_Action_8084DAB4`: swimming while targeting.
    SwimTarget,
    /// `Player_Action_8084DC48`: diving.
    Dive,
    /// `Player_Action_8084E1EC`: surfacing after a dive.
    Surface,
    /// `Player_Action_80845CA4`: walking through an exit, or in from an entrance.
    ExitWalk,
    /// `Player_Action_8084F88C`: falling into a void.
    VoidFall,
    /// `Player_Action_WaitForPutAway`: putting the held item away, then the pending action (`func_A74`).
    ItemPutAway,
    /// `Player_Action_8084BF1C`: on a ladder or a climbable wall.
    Climb,
    /// `Player_Action_8084C5F8`: stepping off a ladder at its top or bottom.
    ClimbEnd,
    /// `Player_Action_80845EF8`: opening a door and walking through it.
    DoorOpen,
    /// `Player_Action_Talk`: talking, until the message box closes.
    Talk,
    /// `Player_Action_8084E6D4`: getting an item: a chest's opening, then the item held up over Link's
    /// head with its text.
    GetItem,
    /// `Player_Action_8084C760`: in a crawlspace, crawling.
    Crawl,
    /// `Player_Action_8084C81C`: climbing out of a crawlspace.
    CrawlExit,
    /// `Player_Action_8084370C`: staggering from a hit.
    Damaged,
    /// `Player_Action_8084377C`: knocked down, in the air until the landing.
    KnockedDown,
    /// `Player_Action_80843954`: lying on the ground after a knockdown.
    Down,
    /// `Player_Action_80843A38`: getting up.
    GetUp,
    /// `Player_Action_CsAction`: in a cutscene mode (`csMode`).
    Cutscene,
    /// `Player_Action_8084FB10`: frozen in ice (`PLAYER_HIT_RESPONSE_FROZEN`), then the thaw.
    Frozen,
    /// `Player_Action_8084FBF4`: electrified (`PLAYER_HIT_RESPONSE_ELECTRIFIED`).
    Electrified,
    /// `Player_Action_8084E30C`: hit while swimming.
    SwimDamaged,
    /// `Player_Action_80843CEC`: dying (or revived) on the ground.
    Dying,
    /// `Player_Action_8084E368`: dying (or revived) while swimming.
    DyingInWater,
    /// `Player_Action_80843188`: guarding with the shield (R), the shield aimed with the stick.
    Guard,
    /// `Player_Action_808435C4`: pushed back by a hit on the shield.
    GuardHit,
    /// `Player_Action_808505DC`: the sword's rebound off something hard (`func_80842D20`).
    Rebound,
    /// `Player_Action_8084B78C`: holding on to a wall to push or pull (A at a `WALL_FLAG_6` wall).
    PushWait,
    /// `Player_Action_8084B898`: pushing.
    Push,
    /// `Player_Action_8084B9E4`: pulling.
    Pull,
    /// `Player_Action_8084B1D8`: first person: C-Up's look (`unk_6AD` 1) or aiming an item
    /// (`unk_6AD` 2).
    FirstPerson,
    /// `Player_Action_8084E604`: throwing a Deku nut.
    ThrowNut,
}

/// `D_80854870`: the push's slips (`ANIMSFX_DATA(ANIMSFX_TYPE_FLOOR, 3)`, `-(.., 21)`).
const D_80854870: [(u16, i16); 2] = [(NA_SE_PL_SLIP, 0x1000 | 3), (NA_SE_PL_SLIP, -(0x1000 | 21))];
/// `D_80854878`: the pull's (frames 4 and 24).
const D_80854878: [(u16, i16); 2] = [(NA_SE_PL_SLIP, 0x1000 | 4), (NA_SE_PL_SLIP, -(0x1000 | 24))];
/// `D_80854880`: where the pull looks for the floor behind Link: 26 up, 40 back.
const D_80854880: Vec3 = Vec3::new(0.0, 26.0, -40.0);
/// `ACTOR_BG_HEAVY_BLOCK` (`actor_table.h`: 0x0092).
const ACTOR_BG_HEAVY_BLOCK: i16 = 0x0092;
/// `SPEED_MODE_LINEAR` (`Player_GetMovementSpeedAndYaw`'s `speedMode`).
const SPEED_MODE_LINEAR: f32 = 0.0;

/// `func_A74`: what `Player_Action_WaitForPutAway` runs once the item is away.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum A74 {
    /// `func_8083A3B0`: start climbing.
    ClimbStart,
    /// `Player_SetupTalk`: start talking.
    Talk,
    /// `func_8083A434`: the get-item action.
    GetItem,
    /// `func_8083A40C`: the crawl.
    Crawl,
    /// `func_8083A388`: holding on to the wall to push or pull (`Player_Action_8084B78C`).
    PushWait,
}

/// Player's upper-body action (`this->upperActionFunc`), run from `Player_UpdateUpperBody`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpperAction {
    /// `func_8083485C`: nothing in hand (only the shield check).
    Default,
    /// `Player_UpperAction_Sword`: a sword in hand (shield check, pending item change).
    Sword,
    /// `Player_UpperAction_ChangeHeldItem`: an item change playing on `skelAnime2`.
    Change,
    /// `func_80834B5C`: the shield held up while Z-targeting, until R is let go.
    ShieldUp,
    /// `func_80834BD4`: the shield's recoil from a blocked hit, back to up.
    ShieldHit,
    /// `func_80834C74`: the shield coming down.
    ShieldDown,
    /// `func_8083501C`: the bow, the slingshot or the hookshot in hand, lowered.
    Bow,
    /// `func_808351D4`: raised, the string drawn back.
    BowDrawn,
    /// `func_808353D8`: just shot, the next shot or lowering.
    BowShot,
    /// `func_80835588`: lowering (`link_bow_bow_shoot_end`).
    BowLower,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::StandingStill => "Player_Action_Idle",
            Action::Run => "Player_Action_80842180",
            Action::Turn => "Player_Action_TurnInPlace",
            Action::Roll => "Player_Action_Roll",
            Action::Midair => "Player_Action_8084411C",
            Action::Hang => "Player_Action_8084BBE4",
            Action::ClimbUp => "Player_Action_8084BDFC",
            Action::ClimbLedge => "Player_Action_80845668",
            Action::TargetIdle => "Player_Action_80840450",
            Action::ParallelIdle => "Player_Action_808407CC",
            Action::TargetRun => "Player_Action_8084227C",
            Action::Sidestep => "Player_Action_8084193C",
            Action::TargetBackwalk => "Player_Action_808423EC",
            Action::TargetBackBrake => "Player_Action_8084251C",
            Action::ParallelWalk => "Player_Action_80840DE4",
            Action::ParallelBackwalk => "Player_Action_808414F8",
            Action::ParallelBackBrake => "Player_Action_8084170C",
            Action::ParallelBackBrakeEnd => "Player_Action_808417FC",
            Action::Attack => "Player_Action_808502D0",
            Action::Swim => "Player_Action_8084D610",
            Action::SwimMove => "Player_Action_8084D84C",
            Action::SwimTarget => "Player_Action_8084DAB4",
            Action::Dive => "Player_Action_8084DC48",
            Action::Surface => "Player_Action_8084E1EC",
            Action::ExitWalk => "Player_Action_80845CA4",
            Action::VoidFall => "Player_Action_8084F88C",
            Action::ItemPutAway => "Player_Action_WaitForPutAway",
            Action::Climb => "Player_Action_8084BF1C",
            Action::ClimbEnd => "Player_Action_8084C5F8",
            Action::DoorOpen => "Player_Action_80845EF8",
            Action::Talk => "Player_Action_Talk",
            Action::GetItem => "Player_Action_8084E6D4",
            Action::Crawl => "Player_Action_8084C760",
            Action::CrawlExit => "Player_Action_8084C81C",
            Action::Damaged => "Player_Action_8084370C",
            Action::KnockedDown => "Player_Action_8084377C",
            Action::Down => "Player_Action_80843954",
            Action::GetUp => "Player_Action_80843A38",
            Action::Cutscene => "Player_Action_CsAction",
            Action::Frozen => "Player_Action_8084FB10",
            Action::Electrified => "Player_Action_8084FBF4",
            Action::SwimDamaged => "Player_Action_8084E30C",
            Action::Dying => "Player_Action_80843CEC",
            Action::DyingInWater => "Player_Action_8084E368",
            Action::Guard => "Player_Action_80843188",
            Action::GuardHit => "Player_Action_808435C4",
            Action::Rebound => "Player_Action_808505DC",
            Action::PushWait => "Player_Action_8084B78C",
            Action::Push => "Player_Action_8084B898",
            Action::Pull => "Player_Action_8084B9E4",
            Action::FirstPerson => "Player_Action_8084B1D8",
            Action::ThrowNut => "Player_Action_8084E604",
        }
    }
}

/// File-scope statics of `z_player.c` that Player's update shares between functions.
#[derive(Debug, Clone, Default)]
pub struct PlayerStatics {
    /// `sControlStickMagnitude`: stick magnitude 0..60 (`Lib_GetControlStickData`).
    pub stick_mag: f32,
    /// `sControlStickAngle`: stick angle relative to up.
    pub stick_angle: i16,
    /// `sControlStickWorldYaw`: stick direction in world space (camera input yaw + stick angle).
    pub stick_world_yaw: i16,
    /// `sUpperBodyIsBusy`: upper-body item action result (always 0 without items).
    pub item_action: i32,
    /// `sFloorType`: floor type under Player.
    pub floor_type: u32,
    /// `sWaterSpeedFactor`: animation/speed scale, 0.5 underwater and 1 otherwise.
    pub speed_scale: f32,
    /// `sTouchedWallFlags`: wall flags of the touched wall.
    pub wall_flags: u32,
    /// `sYDistToFloor`: height above the floor.
    pub floor_dist: f32,
    /// `sPrevFloorProperty`: floor property.
    pub floor_property: u32,
    /// `sShapeYawToTouchedWall`: |facing - wall normal|.
    pub wall_facing_diff: i32,
    /// `sWorldYawToTouchedWall`: |movement yaw - wall normal|.
    pub wall_move_diff: i32,
    /// `sFloorShapePitch`: floor slope along the facing direction.
    pub facing_slope: i16,
    /// `sUseHeldItem` / `sHeldItemButtonIsHeldDown`: B pressed / held with the weapon already in hand this frame.
    pub use_held_item: bool,
    pub held_item_button_is_held_down: bool,
    /// `D_80858AA0`: the animation's `moveFlags` when a cutscene mode starts.
    pub d_80858aa0: i32,
    /// The main camera's mode after Player's own `Camera_RequestMode` calls this update (they're
    /// made on the camera after it, `PlayRequest::CamRequestMode`); `None` before any.
    pub cam_mode: Option<i16>,
}

/// Per-frame environment the update needs.
pub struct Env<'a> {
    pub data: &'a GameData,
    pub col: &'a CollisionContext,
    /// `Camera_GetInputDirYaw(GET_ACTIVE_CAM(play))`.
    pub cam_input_yaw: i16,
    /// `play->gameplayFrames` (face alternation).
    pub gameplay_frames: u32,
    /// The other actors (Player is taken out of the actor context while it updates), and last
    /// frame's target context.
    pub actors: &'a ActorContext,
    pub target: TargetView,
    /// What Player writes to the play state and the save (exits, voids, respawn points).
    pub io: &'a RefCell<PlayIo>,
    /// `play->exitList`, and `gEntranceTable`.
    pub exits: &'a [u16],
    pub entrances: &'a [EntranceInfo],
    /// `transiActorCtx.list`, and `roomCtx.prevRoom.num`.
    pub transi_actors: &'a [oot_game::scene::TransitionActorEntry],
    pub prev_room: i8,
    /// `Play_GetCamera(play, CAM_ID_MAIN)->stateFlags`, as the last camera update left it.
    pub cam_state_flags: i16,
    /// `Message_GetState(&play->msgCtx)`, as the last `Message_Update` left it.
    pub msg_state: u8,
    /// `play->roomCtx.curRoom.behaviorType1`.
    pub room_behavior_type1: u8,
    /// Player's own handle (`&this->actor`; Player is out of `actors` while it updates).
    pub me: Option<ActorHandle>,
    /// `Camera_GetCamDirYaw(GET_ACTIVE_CAM(play))`, as the last camera update left it.
    pub cam_dir_yaw: i16,
    /// `sGetItemTable` (`table/items`; empty without the pack).
    pub items: &'a oot_game::item::ItemTables,
    /// The game's audio tables (`sSurfaceMaterialToSfxOffset` for the floor's footsteps).
    pub audio: &'a oot_game::audio::AudioGameTables,
    /// `play->csCtx`'s `state`, `frames` and `linkAction`, and `play->sceneId`.
    pub cs_state: u8,
    pub cs_frames: u16,
    pub cs_link_action: Option<CsCmdActorCue>,
    pub scene_id: u16,
    /// `play->gameOverCtx.state`, and `play->roomCtx.curRoom.environmentType` (`behaviorType2`).
    pub game_over_state: u16,
    pub room_behavior_type2: u8,
    /// `play->activeCamId`.
    pub active_cam_id: i16,
    /// The main camera as this update began: its setting's modes (`sCameraSettings[setting]`'s
    /// `validModes`, `unk_00`) and its mode, for `Camera_CheckValidMode` and `Camera_RequestMode`.
    pub main_cam_valid_modes: u32,
    pub main_cam_mode: i16,
    /// `R_SCENE_CAM_TYPE`.
    pub scene_cam_type: u8,
}

impl Env<'_> {
    /// An actor by handle (what `focusActor` holds).
    pub fn target(&self, h: ActorHandle) -> Option<&Actor> {
        self.actors.actor(h)
    }
}

#[derive(Debug, Clone)]
pub struct Player {
    pub actor: Actor,
    pub skel: SkelAnime,
    pub action: Action,
    pub regs: Regs,
    pub age: AgeProperties,
    pub adult: bool,
    /// `modelAnimType` (0: nothing in hand).
    pub model_anim_type: usize,
    pub state1: u32,
    pub state2: u32,
    pub state3: u32,
    pub linear_velocity: f32,
    pub current_yaw: i16,
    pub target_yaw: i16,
    pub action_var1: i8,
    /// `underwaterTimer`: the frames spent under water (to 300).
    pub underwater_timer: i16,
    /// Multipurpose timer.
    pub action_var2: i16,
    /// Rolling index into the stick history below.
    pub control_stick_data_index: u8,
    /// Stick direction history (8 sectors), -1 when the stick is below 55.
    pub control_stick_spin_angles: [i8; 4],
    /// Stick direction relative to facing (0 forward, 1 left, 2 back, 3 right), -1 below 55.
    pub control_stick_directions: [i8; 4],
    pub prev_control_stick_magnitude: f32,
    pub prev_control_stick_angle: i16,
    /// Walk start blend (0 → 1).
    pub unk_864: f32,
    /// Walk/run cycle phase in walk-animation frames (0..29).
    pub unk_868: f32,
    pub unk_870: f32,
    /// `unk_86C`: the guard's lock-on weight (1 locked on to an enemy as the guard starts).
    pub unk_86c: f32,
    /// `rightHandType == PLAYER_MODELTYPE_RH_SHIELD` through `Player_SetModelsForHoldingShield`:
    /// the shield in the right hand (the sheath without it), until the models are set again.
    pub holding_shield: bool,
    pub unk_874: f32,
    /// Current run speed limit (`R_RUN_SPEED_LIMIT`, reduced when running into walls).
    pub unk_880: f32,
    /// Floor slope along `currentYaw` and across it.
    pub floor_pitch: i16,
    pub floor_pitch_alt: i16,
    /// Walk climb/descend blend.
    pub unk_89C: i16,
    pub unk_6C2: i16,
    pub unk_6C4: f32,
    /// Which of the look/lean rotations were driven this frame (`func_80847298` decays the rest).
    pub unk_6AE_rot_flags: u16,
    pub idle_type: i8,
    pub unk_6AD: u8,
    /// Head: z (headLimbRot.x), y (headLimbRot.y), x (headLimbRot.z). Upper body: x (upperLimbRot.x), y (upperLimbRot.y), z (upperLimbRot.z).
    pub upper_limb_yaw_secondary: i16,
    pub head_limb_rot_x: i16,
    pub head_limb_rot_y: i16,
    pub head_limb_rot_z: i16,
    pub upper_limb_rot_x: i16,
    pub upper_limb_rot_y: i16,
    pub upper_limb_rot_z: i16,
    pub unk_87C: i16,
    pub turn_rate: i16,
    pub unk_890: u8,
    pub floor_property: u32,
    pub floor_type_timer: u8,
    pub prev_floor_type: u32,
    pub fall_start_height: i16,
    pub fall_distance: i16,
    pub wall_height: f32,
    pub wall_distance: f32,
    pub ledge_climb_type: u8,
    pub ledge_climb_delay_timer: u8,
    pub hover_boots_timer: u8,
    pub pushed_speed: f32,
    pub pushed_yaw: i16,
    pub melee_weapon_state: i8,
    /// `focusActor`: the targeted actor.
    pub focus_actor: Option<ActorHandle>,
    /// What `Player_UpdateCamAndSeqModes` asked of the camera this update (applied by
    /// `ActorImpl::update`, which has the play state).
    pub cam_request: Option<(i16, Option<ActorHandle>)>,
    /// `zTargetActiveTimer`: Z-target timer.
    pub z_target_active_timer: i16,
    /// `bodyPartsPos[PLAYER_BODYPART_HEAD]` from the last draw (also `actor.focus.pos`).
    pub head_pos: Vec3,
    /// `skelAnime2`: the upper-body animation (item changes), merged into `skel` by `Player_UpdateUpperBody`.
    pub skel2: SkelAnime,
    /// `func_82C`.
    pub upper: UpperAction,
    /// `heldItemAction`, `itemAction` (`PLAYER_IA_*`), `heldItemId` (the item in hand or being
    /// changed to, `ITEM_*`), `heldItemButton` (the button it was used from: 0 B, 1 to 3 the C
    /// buttons).
    pub held_item_ap: i32,
    pub item_ap: i32,
    pub held_item_id: u8,
    pub held_item_button: i8,
    /// `currentMask` (`PLAYER_MASK_*`): set by `Player_UseItem`'s mask branch, which no button
    /// reaches yet (masks aren't drawn).
    pub current_mask: u8,
    /// `modelGroup`, `nextModelGroup` (PLAYER_MODELGROUP_*).
    pub model_group: usize,
    pub next_model_group: usize,
    /// `currentShield` (`PLAYER_SHIELD_*`: 0 none, 1 Deku, 2 Hylian, 3 Mirror), `currentTunic`,
    /// `currentBoots`, `currentSwordItemId` (`B_BTN_ITEM`): from the save's equipment
    /// (`Player_SetEquipmentData`).
    pub current_shield: u8,
    pub current_tunic: u8,
    pub current_boots: u8,
    /// `floorSfxOffset`: the floor's footstep sound (`SurfaceType_GetSfxOffset`, an offset from
    /// `NA_SE_PL_WALK_GROUND`), and `prevFloorSfxOffset`, the last frame's.
    pub floor_sfx_offset: u16,
    pub prev_floor_sfx_offset: u16,
    pub current_sword_item_id: u8,
    /// `getItemId`: what `interactRangeActor` offers this frame (`Actor_OfferGetItem`): positive to
    /// take at once, negative a chest's (opened on A), `GI_NONE` something to pick up. While
    /// getting an item, what's being got.
    pub get_item_id: i16,
    /// `interactRangeActor`, `getItemDirection`: the actor offering, and how squarely (reset to
    /// 0x6000 each update).
    pub interact_range_actor: Option<ActorHandle>,
    pub get_item_direction: i16,
    /// `unk_862`: the draw id plus one of the item held up (0: none; `Player_DrawGetItem`).
    pub unk_862: i16,
    /// `unk_860`: the burning Deku Stick's timer (210 lit at a torch, `Obj_Syokudai`), among
    /// other things (the bow's -1, the slingshot's -2, the hookshot's -3, the fishing rod's);
    /// `Player_InitItemAction` zeroes it. `En_St`, `Obj_Syokudai` and `Bg_Ydan_Sp` read it.
    pub unk_860: i16,
    /// `unk_85C`: the Deku Stick's length (1 whole, 0.5 broken, shrinking to 0 as it burns out;
    /// `Player_InitDekuStickIA`), and `unk_858` (the bow string's and the fishing rod's).
    pub unk_85c: f32,
    pub unk_858: f32,
    /// `unk_834`: the bow's and slingshot's timer, zeroed when the item change ends.
    pub unk_834: i16,
    /// The save's `AMMO()` and the explosives out (`actorLists[ACTORCAT_EXPLOSIVE].length`) as
    /// `Player_UseItem` reads them, refreshed at the start of Player's update (see `ammo`).
    pub ammo_view: [i8; 16],
    pub explosive_count: usize,
    /// `leftHandPos`: the left hand limb's origin in the last draw.
    pub left_hand_pos: Vec3,
    /// `play->gameplayFrames` at the last update (the held-up item's spin when drawn).
    pub gameplay_frames: u32,
    pub item_change_type: usize,
    pub upper_anim_interp_weight: f32,
    pub unk_844: i8,
    pub unk_845: i8,
    /// `meleeWeaponAnimation` (PLAYER_MWA_*).
    pub melee_weapon_animation: usize,
    /// `cylinder`: the body (OC against everything, AC from enemies). Collider id 0.
    pub cylinder: ColliderCylinder,
    /// `meleeWeaponQuads`: the sword's two AT quads, set from the draw. Collider ids 1 and 2.
    pub melee_weapon_quads: [ColliderQuad; 2],
    /// `shieldQuad`: collider id 3, registered while shielding (`Player_UpdateShieldCollider`).
    pub shield_quad: ColliderQuad,
    /// `shieldMf`: the shield's matrix from the last draw, in the right hand or on the back.
    pub shield_mf: glam::Mat4,
    /// `meleeWeaponInfo`: the sword's tip and base last frame, for the blur and the two quads.
    pub melee_weapon_info: [WeaponInfo; 3],
    /// `bodyPartsPos` (`PLAYER_BODYPART_*`), from the last draw.
    pub body_parts_pos: [Vec3; BODYPART_MAX],
    /// `actor.shape.feetPos`: the feet (`FOOT_LEFT`, `FOOT_RIGHT`), from the last draw
    /// (`Actor_SetFeetPos` in `Player_PostLimbDrawGameplay`).
    pub feet_pos: [Vec3; 2],
    /// `targetActor`, `targetActorDistance`, `exchangeItemId`: who offered to talk this frame
    /// (`Actor_OfferTalkExchange`). Accepting it (the talk interrupt) comes with the message box.
    pub target_actor: Option<ActorHandle>,
    pub target_actor_distance: f32,
    pub exchange_item_id: u8,
    pub s: PlayerStatics,
    pub input: Input,
    /// `faceChange.face` blink timer; `actor.shape.face`.
    pub blinker: Blinker,
    pub face: usize,
    rng: u32,
    /// Diagnostics: things the port saw but does not implement (ledge grab, climbing...).
    pub notes: Vec<String>,
    /// The last frame's foot IK per leg (left, right), when it ran.
    pub legs: Option<[oot_game::footik::LegResult; 2]>,
    pub frame: u32,
    /// `unk_450`: where an exit or entrance walk heads; `unk_45C`: a door's second target.
    pub unk_450: Vec3,
    pub unk_45C: Vec3,
    pub door_timer: i16,
    /// `doorType` (`PLAYER_DOORTYPE_*`), `doorDirection` (1 in front, -1 behind) and
    /// `doorActor`: the door that offered to open this frame (`EnDoor_Idle`), reset at the end
    /// of every update.
    pub door_type: i8,
    pub door_direction: i8,
    pub door_actor: Option<ActorHandle>,
    /// `unk_447`: the door type of the sliding door being walked through (set, never read).
    pub unk_447: i8,
    /// `cv.slidingDoorBgCamIndex`: the bg camera of the room a sliding door leads into
    /// (`Door_Shutter` hands it to `Camera_ChangeDoorCam` as it opens).
    pub sliding_door_bg_cam_index: i16,
    /// What the update asks of the play state (`PlayRequest`).
    pub play_requests: Vec<PlayRequest>,
    /// `unk_A84`: the height the void check measures falls from.
    pub unk_A84: i16,
    /// `csMode`, `prevCsMode`: the cutscene mode (`Player_SetCsActionWithHaltedActors`) and the one whose start ran.
    pub cs_mode: u8,
    pub prev_cs_mode: u8,
    /// `cueId`: the `linkAction` cue being followed; `csActor`: the actor a mode is about;
    /// `doorBgCamIndex`: 1 when `Player_SetCsActionWithHaltedActors` set the mode (Link is then held).
    pub cue_id: u16,
    pub cs_actor: Option<ActorHandle>,
    pub door_bg_cam_index: i16,
    /// `func_A74`.
    pub func_a74: Option<A74>,
    /// `unk_3C4`: the DynaPoly actor whose wall Link holds on to (`Player_ActionHandler_5`), or
    /// none for the scene's own wall.
    pub unk_3c4: Option<ActorHandle>,
    /// Start mode 0 (`Player_StartMode_Nothing`): `update` is a no-op and `draw` is NULL.
    pub inert: bool,
    /// `naviTextId`: what Navi says when C-Up talks to her (her update sets it; negative: she
    /// speaks at once), cleared at the end of every update.
    pub navi_text_id: i16,
    /// `naviActor`: Navi (`Player_SpawnFairy`).
    pub navi_actor: Option<ActorHandle>,
    /// `actor.shape.shadowDraw` is `ActorShadow_DrawFeet` (`Player_Init`), not `NULL` (Link
    /// asleep in the opening).
    pub shadow_feet: bool,
    /// `textboxBtnCooldownTimer`: frames after a talk that A, B and C-Up are ignored.
    pub textbox_btn_cooldown_timer: u8,
    /// `putAwayCooldownTimer`: frames before "Put Away" shows on the A button.
    pub put_away_cooldown_timer: u8,
    /// `invincibilityTimer`: no damage while non-zero; positive counts down after a hit (and
    /// is visible), negative counts up (the roll's, when Link's cylinder is also AT).
    pub invincibility_timer: i8,
    /// `damageFlickerAnimCounter`: the hit flash's phase (`Player_Draw` steps it while the
    /// invincibility is visible; `Player_SetIntangibility` resets it).
    pub damage_flicker_anim_counter: u8,
    /// The hit flash's red fog this frame (`Player_Draw`'s `Gfx_SetFog2` far plane), or none.
    pub damage_flash_far: Option<i32>,
    /// `bodyShockTimer`, `unk_892`: the electric shock's sparks (`Player_UpdateBodyShock`).
    pub body_shock_timer: u8,
    pub unk_892: u8,
    /// `bodyIsBurning`, `bodyFlameTimers`: each body part's flame (`Player_UpdateBodyBurn`).
    pub body_is_burning: bool,
    pub body_flame_timers: [u8; BODYPART_MAX],
    /// `knockbackDamage` .. `knockbackYVelocity`: the knockback an actor asked for this frame (`Actor_SetPlayerKnockback`):
    /// the extra damage, the kind (1 a push, 2 a knockdown, 3 a shock), the yaw, the speed and
    /// the upward speed. `knockbackType` is cleared at the end of every update.
    pub knockback_damage: u8,
    pub knockback_type: u8,
    pub knockback_rot: i16,
    pub knockback_speed: f32,
    pub knockback_y_velocity: f32,
    /// `unk_A86` (the Ganon fight's delayed damage; never set here), `unk_A87` (frames without
    /// damage).
    pub unk_A86: i8,
    pub unk_A87: u8,
    /// `unk_A73`: 4 on a shot (`func_808350A4`) or a boomerang's throw, counted down by
    /// `Player_UpdateCommon`; `EnArrow_Shoot` lets a seed or an arrow fly only while it's set.
    pub unk_A73: u8,
    /// `heldActor`: the seed or arrow in hand (`En_Arrow`, Player's child).
    pub held_actor: Option<ActorHandle>,
    /// `unk_836`: the bow's and slingshot's upper-body state (`func_808351D4`), zeroed with each
    /// upper-body action.
    pub unk_836: i8,
    /// `play->roomCtx.curRoom.type` as this update began (`func_8083C61C` reads it from
    /// `Player_UseItem`, which the item change reaches without the play state).
    pub room_type_view: u8,
}

/// `sSkeletonBaseTransl`: the root translation Link's animations are authored around.
const BASE_TRANSL: [i16; 3] = [-57, 3377, 0];
/// `D_8085456C`: head-look floor probe offset.
const HEAD_PROBE: Vec3 = Vec3::new(0.0, 100.0, 40.0);

/// `sCueToCsActionMap`: each `linkAction` cue's cutscene mode (negative: without moving Link to the
/// cue's start). A cue past the table reads as 0 (the C reads past it).
#[rustfmt::skip]
const S_CUE_TO_CS_ACTION_MAP: [i8; 78] = [
    0,  3,  3,  5,   4,   8,   9,   13, 14, 15, 16, 17, 18, -22, 23, 24, 25,  26, 27,  28,  29, 31, 32, 33, 34, -35,
    30, 36, 38, -39, -40, -41, 42,  43, 45, 46, 0,  0,  0,  67,  48, 47, -50, 51, -52, -53, 54, 55, 56, 57, 58, 59,
    60, 61, 62, 63,  64,  -65, -66, 68, 11, 69, 70, 71, 8,  8,   72, 73, 78,  79, 80,  89,  90, 91, 92, 77, 19, 94,
];

pub fn s_cue_to_cs_action_map(action: u16) -> i8 {
    S_CUE_TO_CS_ACTION_MAP.get(action as usize).copied().unwrap_or(0)
}

fn abs16(v: i16) -> i32 {
    (v as i32).abs()
}

impl Player {
    /// `Player_Init` / `Player_InitCommon` for a plain spawn standing at `pos`.
    pub fn new(data: &GameData, adult: bool, pos: Vec3, yaw: i16) -> Player {
        let age = data.ages[if adult { 0 } else { 1 }];
        let regs = data.regs[if adult { 0 } else { 1 }].clone();
        let model_anim_type = 0;
        let skel = SkelAnime::new_link(data, data.player_anim(group::WAIT, model_anim_type), BASE_TRANSL);
        let mut actor = Actor::new(pos, yaw);
        PROFILE.apply(&mut actor);
        actor.gravity = regs.reg(68) as f32 / 100.0;
        let mut p = Player {
            actor,
            skel,
            action: Action::StandingStill,
            unk_880: regs.run_speed_limit(),
            regs,
            age,
            adult,
            model_anim_type,
            state1: 0,
            state2: 0,
            state3: 0,
            linear_velocity: 0.0,
            current_yaw: yaw,
            target_yaw: 0,
            action_var1: 0,
            underwater_timer: 0,
            action_var2: 0,
            control_stick_data_index: 0,
            control_stick_spin_angles: [-1; 4],
            control_stick_directions: [-1; 4],
            prev_control_stick_magnitude: 0.0,
            prev_control_stick_angle: 0,
            unk_864: 0.0,
            unk_868: 0.0,
            unk_870: 0.0,
            unk_86c: 0.0,
            holding_shield: false,
            unk_874: 0.0,
            floor_pitch: 0,
            floor_pitch_alt: 0,
            unk_89C: 0,
            unk_6C2: 0,
            unk_6C4: 0.0,
            unk_6AE_rot_flags: 0,
            idle_type: 0,
            unk_6AD: 0,
            upper_limb_yaw_secondary: 0,
            head_limb_rot_x: 0,
            head_limb_rot_y: 0,
            head_limb_rot_z: 0,
            upper_limb_rot_x: 0,
            upper_limb_rot_y: 0,
            upper_limb_rot_z: 0,
            unk_87C: 0,
            turn_rate: 0,
            unk_890: 0,
            floor_property: 0,
            floor_type_timer: 0,
            prev_floor_type: 0,
            fall_start_height: pos.y as i16,
            fall_distance: 0,
            wall_height: 0.0,
            wall_distance: 0.0,
            ledge_climb_type: 0,
            ledge_climb_delay_timer: 0,
            hover_boots_timer: 0,
            pushed_speed: 0.0,
            pushed_yaw: 0,
            melee_weapon_state: 0,
            focus_actor: None,
            cam_request: None,
            legs: None,
            z_target_active_timer: 0,
            head_pos: pos + Vec3::Y * 50.0,
            skel2: SkelAnime::new_link(data, data.player_anim(group::WAIT, 0), BASE_TRANSL),
            upper: UpperAction::Default,
            held_item_ap: 0,
            item_ap: 0,
            // Player_Init: heldItemId = ITEM_NONE, then Player_UseItem(ITEM_NONE) leaves no item
            // in hand (PLAYER_IA_NONE, the default model group).
            held_item_id: oot_game::item::ITEM_NONE,
            held_item_button: 0,
            current_mask: 0,
            model_group: data.items.model_group("DEFAULT"),
            next_model_group: data.items.model_group("DEFAULT"),
            // A plain start is the map select's file's (SaveContext::debug): the Kokiri Sword and
            // the Deku Shield for the child, the Master Sword and the Hylian Shield for the adult,
            // the Kokiri tunic and boots. Player_Init takes the save's (set_equipment_data).
            current_shield: if adult { 2 } else { 1 },
            current_tunic: 0,
            current_boots: 0,
            floor_sfx_offset: 0,
            prev_floor_sfx_offset: 0,
            current_sword_item_id: if adult { oot_game::item::ITEM_SWORD_MASTER } else { oot_game::item::ITEM_SWORD_KOKIRI },
            get_item_id: 0,
            interact_range_actor: None,
            get_item_direction: 0x6000,
            unk_862: 0,
            unk_860: 0,
            unk_85c: 0.0,
            unk_858: 0.0,
            unk_834: 0,
            ammo_view: [0; 16],
            explosive_count: 0,
            left_hand_pos: pos,
            gameplay_frames: 0,
            item_change_type: 0,
            upper_anim_interp_weight: 0.0,
            unk_844: 0,
            unk_845: 0,
            melee_weapon_animation: 0,
            cylinder: ColliderCylinder::new(&D_80854624),
            melee_weapon_quads: [ColliderQuad::new(&D_80854650), ColliderQuad::new(&D_80854650)],
            shield_quad: ColliderQuad::new(&D_808546A0),
            shield_mf: glam::Mat4::IDENTITY,
            melee_weapon_info: [WeaponInfo::default(); 3],
            body_parts_pos: [pos; BODYPART_MAX],
            feet_pos: [pos; 2],
            target_actor: None,
            target_actor_distance: f32::MAX,
            exchange_item_id: 0,
            s: PlayerStatics { speed_scale: 1.0, ..Default::default() },
            input: Input::default(),
            blinker: Blinker::default(),
            face: 0,
            rng: 0x1234_5678,
            notes: Vec::new(),
            frame: 0,
            unk_450: Vec3::ZERO,
            unk_45C: Vec3::ZERO,
            door_timer: 0,
            door_type: PLAYER_DOORTYPE_NONE,
            unk_447: 0,
            sliding_door_bg_cam_index: 0,
            door_direction: 0,
            door_actor: None,
            play_requests: Vec::new(),
            unk_A84: pos.y as i16,
            cs_mode: 0,
            prev_cs_mode: 0,
            cue_id: 0,
            cs_actor: None,
            door_bg_cam_index: 0,
            func_a74: None,
            unk_3c4: None,
            inert: false,
            navi_text_id: 0,
            navi_actor: None,
            shadow_feet: true,
            textbox_btn_cooldown_timer: 0,
            put_away_cooldown_timer: 0,
            invincibility_timer: 0,
            damage_flicker_anim_counter: 0,
            damage_flash_far: None,
            body_shock_timer: 0,
            unk_892: 0,
            body_is_burning: false,
            body_flame_timers: [0; BODYPART_MAX],
            knockback_damage: 0,
            knockback_type: 0,
            knockback_rot: 0,
            knockback_speed: 0.0,
            knockback_y_velocity: 0.0,
            unk_A86: 0,
            unk_A87: 0,
            unk_A73: 0,
            held_actor: None,
            unk_836: 0,
            room_type_view: 0,
        };
        // A plain start (the tests' and the sandbox's): standing still (`func_80853080`).
        p.func_80853080(data);
        p
    }

    /// `Player_Init` for a spawn from the scene's spawn list: the respawn point, then the
    /// start mode in `params` bits 8..11 (`sStartModeFuncs`).
    pub fn init(base: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let adult = play.save.adult;
        let data = play.data.clone();
        let mut p = Player::new(&data, adult, base.world_pos, base.shape_rot.y);
        p.set_equipment_data(&data, &play.save);
        let flags = p.actor.flags;
        p.actor = Actor { flags, gravity: p.actor.gravity, ..base };
        // thisx->room = -1: Player belongs to no room.
        p.actor.room = -1;
        p.current_yaw = p.actor.world_rot.y;
        let respawn_flag = play.save.respawn_flag;
        if respawn_flag != 0 {
            if respawn_flag == -3 {
                p.actor.params = play.save.respawn[RESPAWN_MODE_RETURN].player_params;
            } else {
                let mode = if respawn_flag < 0 { RESPAWN_MODE_DOWN } else { respawn_flag as usize - 1 };
                if respawn_flag > 0 {
                    let r = play.save.respawn[mode];
                    p.actor.world_pos = r.pos;
                    p.actor.home_pos = r.pos;
                    p.actor.prev_pos = r.pos;
                    p.fall_start_height = r.pos.y as i16;
                    p.actor.shape_rot.y = r.yaw;
                    p.current_yaw = r.yaw;
                    p.actor.params = r.player_params;
                }
                play.flags.temp_swch = play.save.respawn[mode].temp_swch_flags & 0xFF_FFFF;
                play.flags.temp_collect = play.save.respawn[mode].temp_collect_flags;
            }
        }
        // func_80845C68(play, respawnFlag == 2): the void-out respawn point is here.
        if respawn_flag != 2 {
            let (pos, yaw) = (p.actor.world_pos, p.actor.shape_rot.y);
            let mut io = play.take_io();
            io.setup_respawn_point(RESPAWN_MODE_DOWN, 0xDFF, pos, yaw);
            io.save.respawn[RESPAWN_MODE_DOWN].data = 0;
            io.save.respawn[RESPAWN_MODE_DOWN].player_params = (p.actor.params & 0xFF) | 0xD00;
            play.put_io(io);
        } else {
            play.save.respawn[RESPAWN_MODE_DOWN].data = 0;
        }
        play.save.respawn[RESPAWN_MODE_DOWN].data = 1;
        p.unk_A84 = p.actor.world_pos.y as i16;
        let mut init_mode = ((p.actor.params as u16 & 0xF00) >> 8) as u8;
        // The warp songs' and blue warps' starts are mode 13 in a cutscene layer.
        if (init_mode == 5 || init_mode == 6) && play.save.cutscene_index >= 0xFFF0 {
            init_mode = 13;
        }
        p.start_mode(play, init_mode);
        // GAMEMODE_NORMAL or GAMEMODE_END_CREDITS (3).
        if init_mode != 0 && (play.save.game_mode == oot_game::save::GAMEMODE_NORMAL || play.save.game_mode == 3) {
            // Player_SpawnFairy(play, this, &world.pos, &sNaviSpawnPosOffset, FAIRY_NAVI): Navi.
            let pos = p.get_relative_position(p.actor.world_pos, Vec3::new(0.0, 50.0, 0.0));
            match play.actor_spawn(crate::en_elf::ACTOR_EN_ELF, pos, [0; 3], crate::en_elf::FAIRY_NAVI) {
                Ok(h) => p.navi_actor = Some(h),
                Err(e) => log::debug!("Navi: {e:?}"),
            }
        }
        Box::new(p)
    }

    /// `Player_SetEquipmentData`: the shield, tunic and boots worn, and B's item, from the save;
    /// then the model group again for the item in hand (`Player_SetModelGroup`, which takes the
    /// shield into account). (`Player_SetBootData`: the Kokiri boots' registers are Player's
    /// already.)
    pub fn set_equipment_data(&mut self, data: &GameData, save: &oot_game::save::SaveContext) {
        use oot_game::item::{EQUIP_TYPE_BOOTS, EQUIP_TYPE_SHIELD, EQUIP_TYPE_TUNIC};
        // csMode 0x56 (a cutscene's equipment) never comes up.
        self.current_shield = save.cur_equip_value(EQUIP_TYPE_SHIELD) as u8;
        self.current_tunic = (save.cur_equip_value(EQUIP_TYPE_TUNIC) as u8).wrapping_sub(1);
        self.current_boots = (save.cur_equip_value(EQUIP_TYPE_BOOTS) as u8).wrapping_sub(1);
        self.current_sword_item_id = save.b_btn_item();
        let g = self.action_to_model_group(data, self.held_item_ap);
        self.player_set_model_group(data, g);
    }

    /// `sStartModeFuncs[initMode]`.
    fn start_mode(&mut self, play: &mut PlayState, mode: u8) {
        let data = play.data.clone();
        match mode {
            // Player_StartMode_Nothing: no update, no draw.
            0 => self.inert = true,
            // Player_StartMode_Idle.
            13 => {
                if self.set_starting_movement(&data, &play.col, 180.0) {
                    self.action_var2 = -20;
                }
            }
            // Player_StartMode_MoveForwardSlow.
            8..=12 | 14 => {
                self.linear_velocity = 2.0;
                play.save.entrance_speed = 2.0;
                if self.set_starting_movement(&data, &play.col, 120.0) {
                    self.action_var2 = -15;
                }
            }
            // Player_StartMode_MoveForward.
            15 => {
                if play.save.entrance_speed < 0.1 {
                    play.save.entrance_speed = 0.1;
                }
                self.linear_velocity = play.save.entrance_speed;
                if self.set_starting_movement(&data, &play.col, 800.0) {
                    self.action_var2 = (-80.0 / self.linear_velocity) as i16;
                    if self.action_var2 < -20 {
                        self.action_var2 = -20;
                    }
                }
            }
            // 1..=7: the Master Sword pedestal, warp songs, blue warps, the jump down into
            // Kokiri Forest's opening and so on: not ported. Standing still.
            m => self.note(format!("start mode {m} (sStartModeFuncs) not ported: standing")),
        }
    }

    /// `Player_SetStartingMovement`: the entrance walk `dist` ahead, or swimming if the spawn is in deep
    /// water. True for the walk.
    fn set_starting_movement(&mut self, data: &GameData, col: &CollisionContext, dist: f32) -> bool {
        if let Some(y) = col.water_surface(self.actor.world_pos.x, self.actor.world_pos.z, col.water_room)
            && y - self.actor.world_pos.y >= self.age.unk_24
        {
            // Player_Action_8084D7C4 (swimming in from the entrance) isn't ported: tread water.
            self.note("entrance into deep water: Player_Action_8084D7C4 not ported, treading water");
            self.func_80838F18(data);
            self.state1 |= STATE1_27 | STATE1_29;
            return false;
        }
        let yaw = self.actor.shape_rot.y;
        self.func_80838E70(data, dist, yaw);
        self.state1 |= STATE1_29;
        true
    }

    /// `func_80838E70`: walk `dist` towards `yaw` (`Player_Action_80845CA4`).
    fn func_80838E70(&mut self, data: &GameData, dist: f32, yaw: i16) {
        self.setup_action(data, Action::ExitWalk, 0);
        self.func_80832440();
        self.action_var1 = 1;
        self.action_var2 = 1;
        self.unk_450.x = sin_s(yaw) * dist + self.actor.world_pos.x;
        self.unk_450.z = cos_s(yaw) * dist + self.actor.world_pos.z;
        // Player_AnimPlayOnce(play, this, Player_GetIdleAnim(this)): LinkAnimation_PlayOnce.
        let a = self.anim(data, group::WAIT);
        self.skel.play_once(data, a);
    }

    fn note(&mut self, s: impl Into<String>) {
        let s = s.into();
        if self.notes.last() != Some(&s) {
            log::debug!("player: {s}");
            self.notes.push(s);
        }
    }

    /// `Rand_ZeroOne` (`qrand.c`).
    fn rand_zero_one(&mut self) -> f32 {
        self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        f32::from_bits((self.rng >> 9) | 0x3F80_0000) - 1.0
    }

    fn anim(&self, data: &GameData, g: usize) -> AnimId {
        data.player_anim(g, self.model_anim_type)
    }

    pub fn grounded(&self) -> bool {
        self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0
    }

    // ================================================================================
    // Per-frame update

    /// `Player_Update` → `Player_UpdateCommon`, movement path. Call once per game frame
    /// with the frame's input; then `finish_frame` (the AnimTaskQueue update).
    pub fn update(&mut self, env: &Env, input: Input) {
        self.frame += 1;
        self.gameplay_frames = env.gameplay_frames;
        self.input = input;
        self.ammo_view = env.io.borrow().save.inventory.ammo;
        self.explosive_count = env.actors.category(oot_game::actor_ctx::ACTORCAT_EXPLOSIVE).len();
        self.room_type_view = env.room_behavior_type1;
        self.s.cam_mode = None;
        let data = env.data;
        // Player_Update: what Player held is gone (update == NULL): Player_DetachHeldActor.
        if self.held_actor.is_some_and(|h| env.target(h).is_none_or(|a| a.killed)) {
            self.detach_held_actor(data);
        }
        self.actor.prev_pos = self.actor.home_pos;

        // An offering actor that went (update == NULL) offers nothing.
        if self.interact_range_actor.is_some_and(|h| Some(h) != env.me && env.target(h).is_none_or(|a| a.killed)) {
            self.interact_range_actor = None;
        }
        if self.unk_A73 != 0 {
            self.unk_A73 -= 1;
        }
        if self.textbox_btn_cooldown_timer != 0 {
            self.textbox_btn_cooldown_timer -= 1;
        }
        if self.unk_A87 != 0 {
            self.unk_A87 -= 1;
        }
        if self.invincibility_timer < 0 {
            self.invincibility_timer += 1;
        } else if self.invincibility_timer > 0 {
            self.invincibility_timer -= 1;
        }
        if self.unk_890 != 0 {
            self.unk_890 -= 1;
        }
        self.update_interface(env);
        self.update_z_targeting(env);
        if self.held_item_ap == env.data.items.ap("DEKU_STICK") && self.unk_860 != 0 {
            self.update_burning_deku_stick(env);
        } else if self.held_item_ap == env.data.items.ap("FISHING_POLE") && self.unk_860 < 0 {
            self.unk_860 += 1;
        }
        if self.body_shock_timer != 0 {
            self.update_body_shock(env);
        }
        if self.body_is_burning {
            self.update_body_burn(env);
        }

        scaled_step_to_s(&mut self.unk_6C2, 0, 400);
        // FaceChange_UpdateBlinking(this->faceChange.face, 20, 80, 6) and the face alternation.
        let blink = self.blinker.step();
        self.face = blink + if env.gameplay_frames & 32 != 0 { 0 } else { 3 };
        // (currentMask == PLAYER_MASK_BUNNY: Player_UpdateBunnyEars; the masks aren't drawn.)
        if self.func_8002dd6c() {
            self.func_8084ff7c();
        }

        if self.skel.move_flags & 0x80 == 0 {
            // Not on ice and no hover boots: speed and direction come straight from Player.
            self.actor.speed_xz = self.linear_velocity;
            self.actor.world_rot.y = self.current_yaw;
            self.actor.update_velocity();
            if self.pushed_speed != 0.0 {
                self.actor.velocity.x += self.pushed_speed * sin_s(self.pushed_yaw);
                self.actor.velocity.z += self.pushed_speed * cos_s(self.pushed_yaw);
            }
            self.actor.update_pos();
            self.process_scene_collision(env);
        }
        if self.pushed_speed != 0.0 {
            step_to_f(&mut self.pushed_speed, 0.0, 1.0);
        }

        if !self.in_blocking_cs_mode(env) && self.state2 & STATE2_18 == 0 {
            self.func_8083D53C(env);
            if self.actor.category == ACTORCAT_PLAYER && env.io.borrow().save.health == 0 {
                // Out of health: let go of a ledge or a wall, else die once on the ground or in
                // the water.
                if self.state1 & (STATE1_13 | STATE1_14 | STATE1_21) != 0 {
                    self.func_80832440();
                    self.func_80837B9C(data);
                } else if self.grounded() || self.state1 & STATE1_27 != 0 {
                    let anim = if self.func_808332B8() {
                        data.anim("link_swimer_swim_down")
                    } else if self.body_shock_timer != 0 {
                        data.anim("link_normal_electric_shock_end")
                    } else {
                        data.anim("link_derth_rebirth")
                    };
                    self.func_80836448(env, anim);
                }
            } else {
                let trigger_start = env.io.borrow().transition.trigger == TRANS_TRIGGER_START;
                if self.actor.parent.is_none() && (trigger_start || self.unk_A87 != 0 || !self.func_808382DC(env)) {
                    self.func_8083AA10(env);
                } else {
                    self.fall_start_height = self.actor.world_pos.y as i16;
                }
                // (Player_DetectRumbleSecrets: the rumble.)
            }
        }

        // A script running: its cue's mode (6) or held still (0x31), unless riding, grabbed or
        // under water.
        if env.cs_state != oot_game::cutscene::CS_STATE_IDLE && self.cs_mode != 6 && self.state1 & STATE1_23 == 0 && self.state2 & STATE2_7 == 0 {
            if env.cs_link_action.is_some_and(|l| s_cue_to_cs_action_map(l.action) != 0) {
                self.set_cs_action_with_halted_actors(6);
                self.zero_speed_xz();
            } else if self.cs_mode == 0 && self.state2 & STATE2_10 == 0 && env.cs_state != oot_game::cutscene::CS_STATE_STOP {
                self.set_cs_action_with_halted_actors(0x31);
                self.zero_speed_xz();
            }
        }
        if self.cs_mode != 0 {
            if self.cs_mode != 7 || self.state1 & (STATE1_13 | STATE1_14 | STATE1_21 | STATE1_26) == 0 {
                self.unk_6AD = 3;
            } else if self.action != Action::Cutscene {
                self.func_80852944(env);
            }
        } else {
            self.prev_cs_mode = 0;
        }

        self.func_8083D6EC();

        if self.focus_actor.is_none() && self.navi_text_id == 0 {
            self.state2 &= !(STATE2_1 | STATE2_21);
        }

        self.state1 &= !(STATE1_12 | STATE1_22 | (1 << 1) | (1 << 9));
        self.state2 &= !(STATE2_0 | STATE2_2 | STATE2_3 | STATE2_5 | STATE2_6 | STATE2_8 | (1 << 9) | STATE2_12 | STATE2_14 | STATE2_16 | STATE2_22 | STATE2_26);
        self.state3 &= !STATE3_4;

        self.func_80847298();
        self.process_control_stick(env);

        // sWaterSpeedFactor: 0.5 while swimming.
        self.s.speed_scale = if self.state1 & STATE1_27 != 0 { 0.5 } else { 1.0 };

        self.s.use_held_item = false;
        self.s.held_item_button_is_held_down = false;
        if self.state3 & (1 << 2) == 0 {
            self.run_action(env);
        }

        self.cam_request = self.update_cam_and_seq_modes();
        if self.skel.move_flags & 8 != 0 {
            let s = if self.skel.move_flags & 4 != 0 { 1.0 } else { self.age.translation_scale };
            self.skel.request_move_actor(s);
        }
        self.update_shape_yaw(env);
        let _ = data;
        self.actor.home_pos = self.actor.world_pos;
    }

    /// `Player_UpdateCamAndSeqModes`'s camera half: the mode Player asks the main camera for
    /// (`Camera_RequestMode`), with the actor it passes to `Camera_SetViewParam(camera, 8, ...)`,
    /// or `None` in first person (`PLAYER_STATE1_20`); `seq_mode` is its sequence half.
    /// Hookshot, `Player_Action_8084377C`, and the bow, slingshot and boomerang aren't ported, so
    /// their modes never come up.
    pub fn update_cam_and_seq_modes(&self) -> Option<(i16, Option<ActorHandle>)> {
        use oot_game::camera::*;
        if self.cs_mode != 0 {
            return Some((CAM_MODE_NORMAL, None));
        }
        if self.state1 & STATE1_20 != 0 {
            return None;
        }
        let mut target = None;
        // (Flying with the hookshot, CAM_MODE_HOOKSHOT_FLY: the hookshot isn't ported.)
        let mode = if self.action == Action::KnockedDown {
            CAM_MODE_STILL
        } else if self.state2 & STATE2_8 != 0 {
            CAM_MODE_PUSH_PULL
        } else if let Some(t) = self.focus_actor {
            target = Some(t);
            if self.actor.flags & ACTOR_FLAG_TALK == ACTOR_FLAG_TALK {
                CAM_MODE_TALK
            } else if self.state1 & STATE1_16 != 0 {
                if self.state1 & STATE1_25 != 0 { CAM_MODE_FOLLOW_BOOMERANG } else { CAM_MODE_Z_TARGET_FRIENDLY }
            } else {
                CAM_MODE_Z_TARGET_UNFRIENDLY
            }
        } else if self.state1 & STATE1_12 != 0 {
            CAM_MODE_CHARGE
        } else if self.state1 & STATE1_25 != 0 {
            CAM_MODE_FOLLOW_BOOMERANG
        } else if self.state1 & (STATE1_13 | STATE1_14) != 0 {
            // Player_FriendlyLockOnOrParallel.
            if self.state1 & (STATE1_16 | STATE1_17 | STATE1_30) != 0 { CAM_MODE_Z_LEDGE_HANG } else { CAM_MODE_LEDGE_HANG }
        } else if self.state1 & (STATE1_17 | STATE1_30) != 0 {
            if self.func_8002dd78() || self.func_808334b4() {
                CAM_MODE_Z_AIM
            } else if self.state1 & STATE1_21 != 0 {
                CAM_MODE_Z_WALL_CLIMB
            } else {
                CAM_MODE_Z_PARALLEL
            }
        } else if self.state1 & (STATE1_18 | STATE1_21) != 0 {
            if self.action == Action::ClimbLedge || self.state1 & STATE1_21 != 0 { CAM_MODE_WALL_CLIMB } else { CAM_MODE_JUMP }
        } else if self.state1 & STATE1_19 != 0 {
            CAM_MODE_FREE_FALL
        } else if self.melee_weapon_state != 0
            // PLAYER_MWA_FORWARD_SLASH_1H (0) .. PLAYER_MWA_SPIN_ATTACK_1H (24).
            && self.melee_weapon_animation < 24
        {
            CAM_MODE_STILL
        } else {
            CAM_MODE_NORMAL
        };
        Some((mode, target))
    }

    /// `Player_UpdateCamAndSeqModes`' sequence half: `SEQ_MODE_STILL` standing still in the
    /// normal camera mode or in first person, else `SEQ_MODE_DEFAULT` (`SEQ_MODE_ENEMY` with an
    /// enemy near, `targetCtx.bgmEnemy`, which nothing sets yet). Player isn't riding
    /// (`PLAYER_STATE1_23`; Epona isn't ported).
    pub fn seq_mode(&self) -> u8 {
        use oot_game::audio::{SEQ_MODE_DEFAULT, SEQ_MODE_STILL};
        if self.cs_mode != 0 {
            return SEQ_MODE_DEFAULT;
        }
        match self.update_cam_and_seq_modes() {
            None => SEQ_MODE_STILL,
            Some((oot_game::camera::CAM_MODE_NORMAL, _)) if self.linear_velocity == 0.0 && self.state1 & STATE1_23 == 0 => SEQ_MODE_STILL,
            Some(_) => SEQ_MODE_DEFAULT,
        }
    }

    /// `AnimTaskQueue_Update`, run after all actors (only Player here) have updated.
    pub fn finish_frame(&mut self) {
        self.skel.run_queue(&mut self.actor);
    }

    /// The draw-time part of Player that changes its joint table: `func_8008F87C` (foot IK) for
    /// both legs, as `Player_OverrideLimbDrawGameplayCommon` runs it for `PLAYER_LIMB_L_THIGH`
    /// and `PLAYER_LIMB_R_THIGH`. Call after `finish_frame` (Player_Draw follows every update).
    pub fn apply_foot_ik(&mut self, data: &GameData, col: &CollisionContext) -> [oot_game::footik::LegResult; 2] {
        // Player_OverrideLimbDrawGameplayCommon, PLAYER_LIMB_ROOT: child root scaling, minus unk_6C4.
        let j0 = self.skel.joint[0];
        let mut root = glam::Vec3::new(j0[0] as f32, j0[1] as f32, j0[2] as f32);
        if !self.adult {
            let mf = self.skel.move_flags;
            if mf & 4 == 0 || mf & 1 != 0 {
                root.x *= 0.64;
                root.z *= 0.64;
            }
            if mf & 4 == 0 || mf & 2 != 0 {
                root.y *= 0.64;
            }
        }
        root.y -= self.unk_6C4;
        let r = self.actor.shape_rot;
        let actor = oot_game::footik::actor_matrix(self.actor.world_pos, self.actor.shape_y_offset, [r.x, r.y, r.z]);
        let rig = &data.rigs[if self.adult { 0 } else { 1 }];
        let legs = [("L_THIGH", "L_SHIN", "L_FOOT"), ("R_THIGH", "R_SHIN", "R_FOOT")];
        let legs = legs.map(|(t, s, f)| {
            oot_game::footik::solve_leg(&data.foot_ik, rig, col, self.adult, actor, root, &mut self.skel.joint, self.unk_6C4, data.limb(t), data.limb(s), data.limb(f))
        });
        // Player_PostLimbDraw's bodyPartsPos[PLAYER_BODYPART_HEAD]: the head limb's origin (the
        // look rotations of the upper body are left out here).
        let head = oot_game::footik::limb_matrix(rig, actor, root, &self.skel.joint, data.limb("HEAD"));
        self.head_pos = head.transform_point3(Vec3::ZERO);
        legs
    }

    fn run_action(&mut self, env: &Env) {
        match self.action {
            Action::StandingStill => self.action_idle(env),
            Action::Cutscene => self.action_cs_action(env),
            Action::Run => self.action_80842180(env),
            Action::Turn => self.action_turn_in_place(env),
            Action::Roll => self.action_roll(env),
            Action::Midair => self.action_8084411c(env),
            Action::Hang => self.action_8084bbe4(env),
            Action::ClimbUp => self.action_8084bdfc(env),
            Action::ClimbLedge => self.action_80845668(env),
            Action::TargetIdle => self.action_80840450(env),
            Action::ParallelIdle => self.action_808407cc(env),
            Action::TargetRun => self.action_8084227c(env),
            Action::Sidestep => self.action_8084193c(env),
            Action::TargetBackwalk => self.action_808423ec(env),
            Action::TargetBackBrake => self.action_8084251c(env),
            Action::ParallelWalk => self.action_80840de4(env),
            Action::ParallelBackwalk => self.action_808414f8(env),
            Action::ParallelBackBrake => self.action_8084170c(env),
            Action::ParallelBackBrakeEnd => self.action_808417fc(env),
            Action::Attack => self.action_808502d0(env),
            Action::Swim => self.action_8084d610(env),
            Action::SwimMove => self.action_8084d84c(env),
            Action::SwimTarget => self.action_8084dab4(env),
            Action::Dive => self.action_8084dc48(env),
            Action::Surface => self.action_8084e1ec(env),
            Action::ExitWalk => self.action_80845ca4(env),
            Action::VoidFall => self.action_8084f88c(env),
            Action::ItemPutAway => self.action_wait_for_put_away(env),
            Action::Climb => self.action_8084bf1c(env),
            Action::ClimbEnd => self.action_8084c5f8(env),
            Action::DoorOpen => self.action_80845ef8(env),
            Action::Talk => self.action_talk(env),
            Action::GetItem => self.action_8084e6d4(env),
            Action::Crawl => self.action_8084c760(env),
            Action::CrawlExit => self.action_8084c81c(env),
            Action::Damaged => self.action_8084370c(env),
            Action::KnockedDown => self.action_8084377c(env),
            Action::Down => self.action_80843954(env),
            Action::GetUp => self.action_80843a38(env),
            Action::Frozen => self.action_8084fb10(env),
            Action::Electrified => self.action_8084fbf4(env),
            Action::SwimDamaged => self.action_8084e30c(env),
            Action::Dying => self.action_80843cec(env),
            Action::DyingInWater => self.action_8084e368(env),
            Action::Guard => self.action_80843188(env),
            Action::GuardHit => self.action_808435c4(env),
            Action::Rebound => self.action_808505dc(env),
            Action::PushWait => self.action_8084b78c(env),
            Action::Push => self.action_8084b898(env),
            Action::Pull => self.action_8084b9e4(env),
            Action::FirstPerson => self.action_8084b1d8(env),
            Action::ThrowNut => self.action_8084e604(env),
        }
    }

    /// `func_8083D6EC`: gravity and terminal velocity (water/sand parts not modelled).
    fn func_8083D6EC(&mut self) {
        self.actor.min_velocity_y = -20.0;
        self.actor.gravity = self.regs.reg(68) as f32 / 100.0;
    }

    /// `Player_ProcessControlStick`: stick processing, once per frame.
    fn process_control_stick(&mut self, env: &Env) {
        self.prev_control_stick_magnitude = self.s.stick_mag;
        self.prev_control_stick_angle = self.s.stick_angle;
        let (mag, ang) = stick_to_mag_angle(&self.input);
        self.s.stick_mag = mag;
        self.s.stick_angle = ang;
        self.s.stick_world_yaw = env.cam_input_yaw.wrapping_add(ang);
        self.control_stick_data_index = (self.control_stick_data_index + 1) % 4;
        let (v1, v0) = if self.s.stick_mag < 55.0 {
            (-1i8, -1i8)
        } else {
            (
                ((self.s.stick_angle.wrapping_add(0x2000) as u16) >> 9) as i8,
                (((self.s.stick_world_yaw.wrapping_sub(self.actor.shape_rot.y)).wrapping_add(0x2000) as u16) >> 14) as i8,
            )
        };
        self.control_stick_spin_angles[self.control_stick_data_index as usize] = v1;
        self.control_stick_directions[self.control_stick_data_index as usize] = v0;
    }

    fn stick_dir(&self) -> i8 {
        self.control_stick_directions[self.control_stick_data_index as usize]
    }

    // ================================================================================
    // Collision (Player_ProcessSceneCollision)

    /// `Player_PosVsWallLineTest`: line from Player's position (raised by `off.y`) to a point `off`
    /// ahead in facing space; walls only, one-sided.
    fn pos_vs_wall_line_test(&self, env: &Env, off: Vec3) -> Option<(Vec3, PolyId)> {
        let a = Vec3::new(self.actor.world_pos.x, self.actor.world_pos.y + off.y, self.actor.world_pos.z);
        let b = self.get_relative_position(self.actor.world_pos, off);
        env.col.entity_line_test(a, b, true, false, false, true)
    }

    /// `Player_GetRelativePosition`: `base` + `off` rotated by the facing yaw.
    fn get_relative_position(&self, base: Vec3, off: Vec3) -> Vec3 {
        let (c, s) = (cos_s(self.actor.shape_rot.y), sin_s(self.actor.shape_rot.y));
        Vec3::new(base.x + (off.x * c + off.z * s), base.y + off.y, base.z + (off.z * c - off.x * s))
    }

    /// `Player_ProcessSceneCollision`: Player's bg check and everything derived from the floor and wall.
    fn process_scene_collision(&mut self, env: &Env) {
        let col = env.col;
        let mut spc7 = 0u8;
        self.s.floor_property = self.floor_property;
        let (radius, wall_h, ceil_h) = if self.state2 & STATE2_18 != 0 {
            (10.0, 15.0, 30.0)
        } else {
            (self.age.wall_radius, 26.0, self.age.ceiling_check_height)
        };
        let all = UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_1 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4 | UPDBGCHECKINFO_FLAG_5;
        let mut flags = if self.state1 & (STATE1_29 | STATE1_31) != 0 {
            if self.state1 & STATE1_31 != 0 {
                self.actor.bg_check_flags &= !BGCHECKFLAG_GROUND;
                UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4 | UPDBGCHECKINFO_FLAG_5
            } else if self.state1 & STATE1_0 != 0 && (self.unk_A84 as i32 - self.actor.world_pos.y as i32) >= 100 {
                UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4 | UPDBGCHECKINFO_FLAG_5
            } else if self.state1 & STATE1_0 == 0 && matches!(self.action, Action::DoorOpen | Action::ExitWalk) {
                // Through a door (Player_Action_80845EF8, Player_Action_80845CA4): no walls.
                self.actor.bg_check_flags &= !(BGCHECKFLAG_WALL | BGCHECKFLAG_PLAYER_WALL_INTERACT);
                UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4 | UPDBGCHECKINFO_FLAG_5
            } else {
                all
            }
        } else {
            all
        };
        // PLAYER_STATE3_0.
        if self.state3 & (1 << 0) != 0 {
            flags &= !(UPDBGCHECKINFO_FLAG_1 | UPDBGCHECKINFO_FLAG_2);
        }
        if flags & UPDBGCHECKINFO_FLAG_2 != 0 {
            self.state3 |= STATE3_4;
        }
        self.actor.update_bg_check_info(col, wall_h, radius, ceil_h, flags);
        // DynaPolyActor_UpdateCarriedActorRotY: a rotating platform turns Player's currentYaw too.
        self.current_yaw = self.current_yaw.wrapping_add(self.actor.carried_yaw);
        if self.actor.bg_check_flags & BGCHECKFLAG_CEILING != 0 {
            self.actor.velocity.y = 0.0;
        }
        self.s.floor_dist = self.actor.world_pos.y - self.actor.floor_height;
        if let Some(fp) = self.actor.floor_poly {
            self.floor_property = col.floor_property(fp);
            self.prev_floor_sfx_offset = self.floor_sfx_offset;
            if self.actor.bg_check_flags & BGCHECKFLAG_WATER != 0 {
                self.floor_sfx_offset = if self.actor.y_dist_to_water < 20.0 { SURFACE_MATERIAL_WATER_SHALLOW } else { SURFACE_MATERIAL_WATER_DEEP };
            } else if self.state2 & STATE2_9 != 0 {
                self.floor_sfx_offset = SURFACE_MATERIAL_SAND;
            } else {
                // floorSfxOffset is a sfxType, but SurfaceType_GetSfxOffset returns a sfxId (the decomp's
                // note): NA_SE_PL_WALK_* - SFX_FLAG, the offset from NA_SE_PL_WALK_GROUND.
                self.floor_sfx_offset = env.audio.surface_sfx_id(col.sfx_type(fp));
            }
            if self.actor.category == ACTORCAT_PLAYER {
                self.sfx(PlayerSfx::CodeReverb(col.echo(fp) as i8));
                if self.actor.floor_bg_id == eng_collision::bgcheck::BGCHECK_SCENE {
                    // (Environment_ChangeLightSetting: not ported here.)
                } else {
                    col.dyna.set_interact_flag(self.actor.floor_bg_id, eng_collision::dyna::DYNA_INTERACT_PLAYER_ABOVE);
                }
            }
        }
        let floor = self.actor.floor_poly;
        self.handle_exits_and_voids(env, floor);

        self.actor.bg_check_flags &= !BGCHECKFLAG_PLAYER_WALL_INTERACT;
        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
            let probe = Vec3::new(0.0, 18.0, self.age.wall_radius + 10.0);
            if self.state2 & STATE2_18 == 0 {
                if let Some((_, poly)) = self.pos_vs_wall_line_test(env, probe) {
                    self.actor.bg_check_flags |= BGCHECKFLAG_PLAYER_WALL_INTERACT;
                    if self.actor.wall_poly != Some(poly) {
                        self.actor.wall_poly = Some(poly);
                        let n = col.poly(poly).normal;
                        self.actor.wall_yaw = atan2_s(n[2] as f32, n[0] as f32);
                    }
                }
            }
            let wall_back = self.actor.wall_yaw.wrapping_add(-0x8000i32 as i16);
            let sp9a = self.actor.shape_rot.y.wrapping_sub(wall_back);
            self.s.wall_flags = self.actor.wall_poly.map(|w| col.wall_flags(w)).unwrap_or(0);
            self.s.wall_facing_diff = abs16(sp9a);
            let sp9a = self.current_yaw.wrapping_sub(wall_back);
            self.s.wall_move_diff = abs16(sp9a);
            let k = self.s.wall_move_diff as f32 * 0.00008;
            let rsl = self.regs.run_speed_limit();
            if !self.grounded() || k >= 1.0 {
                self.unk_880 = rsl;
            } else {
                self.unk_880 = (rsl * k).max(0.1);
            }
            if self.actor.bg_check_flags & BGCHECKFLAG_PLAYER_WALL_INTERACT != 0 && self.s.wall_facing_diff < 0x3000 {
                if let Some(wp) = self.actor.wall_poly {
                    spc7 = self.wall_height_class(env, wp, probe);
                }
            }
        } else {
            self.unk_880 = self.regs.run_speed_limit();
            self.ledge_climb_delay_timer = 0;
            self.wall_height = 0.0;
        }
        if spc7 == self.ledge_climb_type {
            if self.linear_velocity != 0.0 && self.ledge_climb_delay_timer < 100 {
                self.ledge_climb_delay_timer += 1;
            }
        } else {
            self.ledge_climb_type = spc7;
            self.ledge_climb_delay_timer = 0;
        }

        if self.grounded() {
            self.s.floor_type = self.actor.floor_poly.map(|p| col.floor_type(p)).unwrap_or(0);
            if !self.update_hover_boots() {
                if self.actor.floor_bg_id != eng_collision::bgcheck::BGCHECK_SCENE {
                    // DynaPoly_SetPlayerOnTop.
                    col.dyna.set_interact_flag(self.actor.floor_bg_id, eng_collision::dyna::DYNA_INTERACT_PLAYER_ON_TOP);
                }
                if let Some(fp) = self.actor.floor_poly {
                    let n = col.poly_normal(fp);
                    let inv_ny = 1.0 / n.y;
                    let (s, c) = (sin_s(self.current_yaw), cos_s(self.current_yaw));
                    self.floor_pitch = atan2_s(1.0, (-(n.x * s) - (n.z * c)) * inv_ny);
                    self.floor_pitch_alt = atan2_s(1.0, (-(n.x * c) - (n.z * s)) * inv_ny);
                    let (s, c) = (sin_s(self.actor.shape_rot.y), cos_s(self.actor.shape_rot.y));
                    self.s.facing_slope = atan2_s(1.0, (-(n.x * s) - (n.z * c)) * inv_ny);
                    // Player_HandleSlopes: slippery slopes (FLOOR_EFFECT_1) are not in the course.
                    if col.floor_effect(fp) == 1 {
                        self.note("slippery slope (Player_HandleSlopes) not modelled");
                    }
                }
            }
        } else {
            self.update_hover_boots();
        }
        if self.prev_floor_type == self.s.floor_type {
            self.floor_type_timer = self.floor_type_timer.saturating_add(1);
        } else {
            self.prev_floor_type = self.s.floor_type;
            self.floor_type_timer = 0;
        }
    }

    /// The wall-top probe in `Player_ProcessSceneCollision`: sets `wallHeight` and returns the climb class
    /// (`spC7`: 1 step, 2..4 increasingly tall ledges).
    fn wall_height_class(&mut self, env: &Env, wp: PolyId, probe: Vec3) -> u8 {
        let col = env.col;
        let wall = col.poly(wp);
        if abs16(wall.normal[1]) >= 600 {
            return 0;
        }
        let n = col.poly_normal(wp);
        self.wall_distance = udist_plane_to_pos(n, wall.dist as f32, self.actor.world_pos);
        let k = self.wall_distance + 10.0;
        let top_probe = Vec3::new(self.actor.world_pos.x - k * n.x, self.actor.world_pos.y + self.age.wall_probe_height, self.actor.world_pos.z - k * n.z);
        let (top, ground) = col.entity_raycast_down(top_probe);
        self.wall_height = top - self.actor.world_pos.y;
        let ceiling = col
            .check_ceiling(eng_collision::bgcheck::IGNORE_ENTITY, self.actor.world_pos, (top - self.actor.world_pos.y) + 20.0)
            .is_some();
        if self.wall_height < 18.0 || ceiling {
            self.wall_height = 399.96002;
            return 0;
        }
        let probe2 = Vec3::new(probe.x, (top + 5.0) - self.actor.world_pos.y, probe.z);
        if let Some((_, p2)) = self.pos_vs_wall_line_test(env, probe2) {
            let n2 = col.poly(p2).normal;
            let d = self.actor.wall_yaw.wrapping_sub(atan2_s(n2[2] as f32, n2[0] as f32));
            if abs16(d) < 0x4000 && col.wall_flags(p2) & WALL_FLAG_1 == 0 {
                self.wall_height = 399.96002;
                return 0;
            }
        }
        if col.wall_flags(wp) & WALL_FLAG_0 != 0 {
            return 0;
        }
        if self.age.unk_1C <= self.wall_height {
            let steep = ground.map(|g| abs16(col.poly(g).normal[1]) > 28000).unwrap_or(false);
            if steep {
                if self.age.unk_14 <= self.wall_height {
                    4
                } else if self.age.unk_18 <= self.wall_height {
                    3
                } else {
                    2
                }
            } else {
                0
            }
        } else {
            1
        }
    }

    /// `Player_UpdateHoverBoots`: hover boots bookkeeping; returns true when airborne.
    fn update_hover_boots(&mut self) -> bool {
        // Kokiri boots: the timer never counts.
        self.hover_boots_timer = 0;
        if self.grounded() {
            self.hover_boots_timer = 19;
            return false;
        }
        self.s.floor_type = 0;
        self.floor_pitch = 0;
        self.floor_pitch_alt = 0;
        self.s.facing_slope = 0;
        true
    }

    /// `func_8083AA10`: leaving the ground. Starts an auto-jump off a ledge when running
    /// forward fast, otherwise a fall (or a ledge grab, which is not modelled).
    fn func_8083AA10(&mut self, env: &Env) {
        let data = env.data;
        self.fall_distance = self.fall_start_height.wrapping_sub(self.actor.world_pos.y as i32 as i16);
        if self.state1 & (STATE1_27 | STATE1_29) == 0 && !self.grounded() {
            if self.func_80838FB8(env) {
                return;
            }
            if self.s.floor_property == FLOOR_PROPERTY_8 {
                self.actor.world_pos.x = self.actor.prev_pos.x;
                self.actor.world_pos.z = self.actor.prev_pos.z;
                return;
            }
            if self.state3 & STATE3_1 == 0 && self.skel.move_flags & 0x80 == 0 && self.action != Action::Midair {
                if self.s.floor_property == FLOOR_PROPERTY_7 || self.melee_weapon_state != 0 {
                    self.actor.world_pos = self.actor.prev_pos;
                    self.zero_speed_xz();
                    return;
                }
                if self.hover_boots_timer != 0 {
                    self.actor.velocity.y = 1.0;
                    self.s.floor_property = FLOOR_PROPERTY_9;
                    return;
                }
                let sp5c = self.current_yaw.wrapping_sub(self.actor.shape_rot.y);
                self.setup_action(data, Action::Midair, 1);
                self.func_80832440();
                if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_LEAVE != 0
                    && self.state1 & STATE1_27 == 0
                    && self.s.floor_property != FLOOR_PROPERTY_6
                    && self.s.floor_property != FLOOR_PROPERTY_9
                    && self.s.floor_dist > 20.0
                    && self.melee_weapon_state == 0
                    && abs16(sp5c) < 0x2000
                    && self.linear_velocity > 3.0
                {
                    // FLOOR_PROPERTY_11 jump into water: no water.
                    self.func_8083A4A8(data);
                    return;
                }
                if self.s.floor_property == FLOOR_PROPERTY_9 || self.s.floor_dist <= self.age.ledge_grab_min_drop || !self.func_8083A6AC(env) {
                    let a = data.anim("link_normal_landing_wait");
                    self.skel.play_loop(data, a);
                }
            }
        } else {
            self.fall_start_height = self.actor.world_pos.y as i32 as i16;
        }
    }

    /// `func_8083A6AC`: grab the ledge Player just walked off: a line back towards the previous
    /// position finds the cliff face, and Player hangs from its top (`func_8083A5C4`).
    fn func_8083A6AC(&mut self, env: &Env) -> bool {
        let data = env.data;
        if self.actor.y_dist_to_water < -80.0 && abs16(self.floor_pitch) < 2730 && abs16(self.floor_pitch_alt) < 2730 {
            let d = Vec3::new(self.actor.prev_pos.x - self.actor.world_pos.x, 0.0, self.actor.prev_pos.z - self.actor.world_pos.z);
            let len = (d.x * d.x + d.z * d.z).sqrt();
            let k = if len != 0.0 { 5.0 / len } else { 0.0 };
            let b = Vec3::new(self.actor.prev_pos.x + d.x * k, self.actor.world_pos.y, self.actor.prev_pos.z + d.z * k);
            if let Some((_, poly)) = env.col.entity_line_test(self.actor.world_pos, b, true, false, false, true)
                && abs16(env.col.poly(poly).normal[1]) < 600
            {
                let n = env.col.poly_normal(poly);
                let sp54 = udist_plane_to_pos(n, env.col.poly(poly).dist as f32, self.actor.world_pos);
                let climbable = self.s.floor_property == FLOOR_PROPERTY_6 || env.col.wall_flags(poly) & WALL_FLAG_3 != 0;
                let anim = if climbable { data.anim("link_normal_Fclimb_startB") } else { data.anim("link_normal_fall") };
                self.func_8083A5C4(data, env, poly, sp54, anim);
                if climbable {
                    // Down onto a climbable wall below the edge.
                    self.setup_wait_for_put_away(data, env, A74::ClimbStart);
                    self.current_yaw = self.current_yaw.wrapping_add(i16::MIN);
                    self.actor.shape_rot.y = self.current_yaw;
                    self.state1 |= STATE1_21;
                    self.start_anim_movement(0x9F);
                    self.action_var2 = -1;
                    self.action_var1 = 1;
                } else {
                    self.state1 |= STATE1_13;
                    self.state1 &= !STATE1_17;
                }
                self.play_sfx(NA_SE_PL_SLIPDOWN);
                self.play_voice_sfx(NA_SE_VO_LI_HANG);
                return true;
            }
        }
        false
    }

    /// `func_8083A5C4`: start hanging from the top edge of `wall`, `dist` away from its plane.
    fn func_8083A5C4(&mut self, data: &GameData, env: &Env, wall: PolyId, dist: f32, anim: AnimId) {
        let n = env.col.poly_normal(wall);
        self.setup_action(data, Action::Hang, 0);
        self.func_80832564(data);
        self.skel.play_once(data, anim);
        self.actor.world_pos.x -= (dist + 1.0) * n.x;
        self.actor.world_pos.z -= (dist + 1.0) * n.z;
        self.current_yaw = atan2_s(n.z, n.x);
        self.actor.shape_rot.y = self.current_yaw;
        self.func_80832224();
        self.reset_anim_movement();
    }

    /// `func_80832224`.
    fn func_80832224(&mut self) {
        self.zero_speed_xz();
        self.unk_6AD = 0;
    }

    /// `Player_ApplyAnimMovementScaledByAge`: apply the root motion accumulated so far to the actor (`flags` 1: x/z,
    /// 2: y, 4: y unscaled), then fold the root yaw into the facing.
    fn apply_anim_movement_scaled_by_age(&mut self, flags: u16) {
        self.skel.move_flags = flags;
        self.skel.prev_transl = self.skel.base_transl;
        let mut pos = self.skel.update_translation(self.actor.shape_rot.y);
        if flags & 1 != 0 {
            if !self.adult {
                pos.x *= 0.64;
                pos.z *= 0.64;
            }
            self.actor.world_pos.x += pos.x * self.actor.scale.x;
            self.actor.world_pos.z += pos.z * self.actor.scale.z;
        }
        if flags & 2 != 0 {
            if flags & 4 == 0 {
                pos.y *= self.age.translation_scale;
            }
            self.actor.world_pos.y += pos.y * self.actor.scale.y;
        }
        self.apply_yaw_from_anim();
    }

    /// `func_80837B60`: bake the whole root offset (the hanging body) into the position.
    fn func_80837B60(&mut self) {
        self.skel.prev_transl = self.skel.joint[0];
        self.apply_anim_movement_scaled_by_age(3);
    }

    /// `func_80837B9C`: let go and fall.
    fn func_80837B9C(&mut self, data: &GameData) {
        self.setup_action(data, Action::Midair, 0);
        let a = data.anim("link_normal_landing_wait");
        self.skel.play_loop(data, a);
        self.action_var2 = 1;
        if self.unk_6AD != 3 {
            self.unk_6AD = 0;
        }
    }

    /// `Player_Action_8084BBE4`: hanging from a ledge. Any stick direction climbs up; A lets go.
    fn action_8084bbe4(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_6;
        let fall = data.anim("link_normal_fall");
        if self.skel.update(data) {
            let anim = if self.action_var1 > 0 { data.anim("link_normal_fall_wait") } else { self.anim(data, group::HANG_WAIT) };
            self.skel.play_loop(data, anim);
        } else if self.action_var1 == 0 {
            let f = if self.skel.animation == fall { 11.0 } else { 1.0 };
            if self.skel.on_frame(f) {
                self.play_floor_sfx(NA_SE_PL_WALK_GROUND);
                self.action_var1 = if self.skel.animation == fall { 1 } else { -1 };
            }
        }
        scaled_step_to_s(&mut self.actor.shape_rot.y, self.current_yaw, 0x800);
        if self.action_var1 != 0 {
            if self.control_stick_spin_angles[self.control_stick_data_index as usize] >= 0 {
                let anim = if self.action_var1 > 0 { self.anim(data, group::HANG_CLIMB_UP) } else { self.anim(data, group::HANG_GRAB_CLIMB_UP) };
                self.func_8083A9B8(data, anim);
                return;
            }
            // `actor.shape.feetFloorFlag` (feet touching a floor, set at draw time) also lets go;
            // not modelled.
            if self.input.cur.held(BTN_A) {
                self.func_80837B60();
                self.linear_velocity = if self.action_var1 < 0 { -0.8 } else { 0.8 };
                self.func_80837B9C(data);
                self.state1 &= !(STATE1_13 | STATE1_14);
            }
        }
    }

    /// `func_8083A9B8`: start climbing up from the hang.
    fn func_8083A9B8(&mut self, data: &GameData, anim: AnimId) {
        self.setup_action(data, Action::ClimbUp, 0);
        self.skel.play_once_set_speed(data, anim, 1.3);
    }

    /// `Player_Action_8084BDFC`: climbing up; at the end the animation's x/z root motion is applied
    /// and Player stands on the ledge.
    fn action_8084bdfc(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_6;
        if self.skel.update(data) {
            self.apply_anim_movement_scaled_by_age(1);
            self.func_8083C0E8(data);
            return;
        }
        if self.skel.on_frame(self.skel.end_frame - 6.0) {
            self.play_landing_sfx();
        } else if self.skel.on_frame(self.skel.end_frame - 34.0) {
            self.state1 &= !(STATE1_13 | STATE1_14);
            self.play_sfx(NA_SE_PL_CLIMB_CLIFF);
            self.play_voice_sfx(NA_SE_VO_LI_CLIMB_END);
        }
    }

    /// `Player_ActionHandler_12` (interrupt 12): climb onto the wall in front when it's ledge height
    /// (`ledgeClimbType` ≥ 2) after pushing into it for 6 frames or on A; hop up a low step
    /// (`ledgeClimbType` = 1) after 3 frames.
    fn action_handler_12(&mut self, env: &Env) -> bool {
        let data = env.data;
        let swimming = self.state1 & STATE1_27 != 0;
        if self.state1 & STATE1_11 == 0 && self.ledge_climb_type >= 2 && (!swimming || self.age.unk_14 > self.wall_height) {
            if swimming {
                // func_808332B8: swimming with Kokiri boots.
                if self.actor.y_dist_to_water < 50.0 {
                    if self.ledge_climb_type < 2 || self.wall_height > self.age.unk_10 {
                        return false;
                    }
                } else {
                    return false;
                }
            } else if !self.grounded() || (self.age.unk_14 <= self.wall_height && swimming) {
                return false;
            }
            // The WALL_FLAG_6 prompt case applies to dynamic collision only (wallBgId != BGCHECK_SCENE).
            if self.ledge_climb_delay_timer >= 6 || self.input.press.held(BTN_A) {
                self.setup_action(data, Action::ClimbLedge, 0);
                self.state1 |= STATE1_18;
                let mut sp34 = self.wall_height;
                let anim;
                if self.age.unk_14 <= sp34 {
                    anim = data.anim("link_normal_250jump_start");
                    self.linear_velocity = 1.0;
                } else {
                    let n = self.actor.wall_poly.map(|w| env.col.poly_normal(w)).unwrap_or(Vec3::ZERO);
                    let sp24 = self.wall_distance + 0.5;
                    self.state1 |= STATE1_14;
                    let k = self.age.translation_scale;
                    if swimming {
                        anim = data.anim("link_swimer_swim_15step_up");
                        sp34 -= 60.0 * k;
                        self.state1 &= !STATE1_27;
                    } else if self.age.unk_18 <= sp34 {
                        anim = data.anim("link_normal_150step_up");
                        sp34 -= 59.0 * k;
                    } else {
                        anim = data.anim("link_normal_100step_up");
                        sp34 -= 41.0 * k;
                    }
                    self.actor.shape_y_offset -= sp34 * 100.0;
                    self.actor.world_pos.x -= sp24 * n.x;
                    self.actor.world_pos.y += self.wall_height;
                    self.actor.world_pos.z -= sp24 * n.z;
                    self.func_80832224();
                }
                self.actor.bg_check_flags |= BGCHECKFLAG_GROUND;
                self.skel.play_once_set_speed(data, anim, 1.3);
                self.skel.disable_queue();
                self.current_yaw = self.actor.wall_yaw.wrapping_add(i16::MIN);
                self.actor.shape_rot.y = self.current_yaw;
                return true;
            }
        } else if self.grounded() && self.ledge_climb_type == 1 && self.ledge_climb_delay_timer >= 3 {
            // func_808389E8: hop up a low step.
            let vy = self.wall_height * 0.08 + 5.5;
            let a = data.anim("link_normal_jump");
            self.func_80838940(data, Some(a), vy, NA_SE_VO_LI_SWORD_N);
            self.linear_velocity = 2.5;
            return true;
        }
        false
    }

    // ================================================================================
    // Climbing (ladders, vines and climbable walls)

    /// `Player_SetupWaitForPutAway`: the action `f` once the held item is put away (`Player_Action_WaitForPutAway`).
    fn setup_wait_for_put_away(&mut self, data: &GameData, env: &Env, f: A74) -> bool {
        let _ = env;
        self.func_a74 = Some(f);
        self.setup_action(data, Action::ItemPutAway, 0);
        self.state2 |= STATE2_6;
        self.put_away_held_item(data)
    }

    /// `Player_PutAwayHeldItem`: put a held item away (`Player_UseItem(ITEM_NONE)`).
    fn put_away_held_item(&mut self, data: &GameData) -> bool {
        if self.held_item_ap >= data.items.ap("FISHING_POLE") {
            self.use_item(data, oot_game::item::ITEM_NONE);
            true
        } else {
            false
        }
    }

    /// `Player_Action_WaitForPutAway`: the item change plays; when it's done (or there was none), the
    /// pending action.
    fn action_wait_for_put_away(&mut self, env: &Env) {
        self.state2 |= STATE2_5 | STATE2_6;
        self.skel.update(env.data);
        // (Holding an actor with no item to get: Player doesn't hold actors yet.)
        if !self.update_upper_body(env) {
            match self.func_a74 {
                Some(A74::ClimbStart) => self.func_8083A3B0(env.data),
                Some(A74::Talk) => self.setup_talk(env.data),
                Some(A74::GetItem) => self.func_8083A434(env.data),
                Some(A74::Crawl) => self.func_8083A40C(env.data),
                Some(A74::PushWait) => self.func_8083A388(env.data),
                None => {}
            }
        }
    }

    /// `func_8083A3B0`: climbing, keeping `av2.actionVar2` and `av1.actionVar1`.
    fn func_8083A3B0(&mut self, data: &GameData) {
        let (sp1c, sp18) = (self.action_var2, self.action_var1);
        self.setup_action_preserve_anim_movement(data, Action::Climb, 0);
        self.actor.velocity.y = 0.0;
        self.action_var2 = sp1c;
        self.action_var1 = sp18;
    }

    /// `Player_ActionHandler_5` (interrupt 5): at a wall Link faces: climbing it (`func_8083EC18`),
    /// entering a crawlspace (`Player_TryEnteringCrawlspace`), or, standing on the ground at a
    /// `WALL_FLAG_6` wall 39 or more high, "Grab" (`PLAYER_STATE2_0`) and on A holding on to it to
    /// push or pull (`func_8083F72C`), `unk_3C4` the wall's DynaPoly actor (none for the scene's).
    /// A `Bg_Heavy_Block`'s wall needs the gold gauntlets (`Player_GetStrength`) and is lifted
    /// (`func_8083A0F4`): Player's lift isn't ported, so that logs and returns as the C does.
    fn action_handler_5(&mut self, env: &Env) -> bool {
        if self.state1 & STATE1_11 == 0 && self.actor.bg_check_flags & BGCHECKFLAG_PLAYER_WALL_INTERACT != 0 && self.s.wall_facing_diff < 0x3000 {
            let flags = self.s.wall_flags;
            if (self.linear_velocity > 0.0 && self.func_8083EC18(env, flags)) || self.try_entering_crawlspace(env, flags) {
                return true;
            }
            if !self.func_808332B8() && (self.linear_velocity == 0.0 || self.state2 & STATE2_2 == 0) && flags & WALL_FLAG_6 != 0 && self.grounded() && self.wall_height >= 39.0 {
                self.state2 |= STATE2_0;
                if self.input.cur.held(BTN_A) {
                    let wall_bg = self.wall_bg_id();
                    let wall_poly_actor = if wall_bg != BGCHECK_SCENE { oot_game::actor_ctx::dyna_poly_get_actor(env.actors, env.col, wall_bg) } else { None };
                    if wall_bg != BGCHECK_SCENE && wall_poly_actor.is_some() {
                        let h = wall_poly_actor.unwrap();
                        if env.actors.actor(h).is_some_and(|a| a.id == ACTOR_BG_HEAVY_BLOCK) {
                            if oot_game::player_lib::player_get_strength(&env.io.borrow().save) < oot_game::player_lib::PLAYER_STR_GOLD_G {
                                return false;
                            }
                            // Player_SetupWaitForPutAway(func_8083A0F4), PLAYER_STATE1_CARRYING_ACTOR,
                            // interactRangeActor the block, getItemId GI_NONE, the yaw off the wall,
                            // func_80832224: Player's lift isn't ported.
                            self.note("lifting a Bg_Heavy_Block (func_8083A0F4) not ported");
                            return true;
                        }
                        self.unk_3c4 = wall_poly_actor;
                    } else {
                        self.unk_3c4 = None;
                    }
                    let anim = env.data.anim("link_normal_push_wait");
                    self.func_8083F72C(env, anim);
                    return true;
                }
            }
        }
        false
    }

    /// `this->actor.wallBgId`: the bg id of the wall Link last touched (`BGCHECK_SCENE` for the
    /// scene's own).
    fn wall_bg_id(&self) -> u16 {
        self.actor.wall_poly.map_or(BGCHECK_SCENE, |p| p.bg)
    }

    /// `func_8083F72C`: holding on to the wall (`Player_Action_8084B78C`, after the held item is
    /// put away: `func_8083A388`), `anim` played once, still, facing the wall.
    fn func_8083F72C(&mut self, env: &Env, anim: AnimId) {
        let data = env.data;
        if !self.setup_wait_for_put_away(data, env, A74::PushWait) {
            self.setup_action(data, Action::PushWait, 0);
        }
        self.skel.play_once(data, anim);
        self.func_80832224();
        self.current_yaw = self.actor.wall_yaw.wrapping_add(i16::MIN);
        self.actor.shape_rot.y = self.current_yaw;
    }

    /// `func_8083A388`.
    fn func_8083A388(&mut self, data: &GameData) {
        self.setup_action(data, Action::PushWait, 0);
    }

    /// `func_8083F524`: kept at the wall, 26 up, `wallCheckRadius` + 5 off it, looking 30 ahead
    /// (`func_8083F360`).
    fn func_8083F524(&mut self, env: &Env) -> bool {
        let r = self.age.wall_radius + 5.0;
        self.func_8083F360(env, 26.0, r, 30.0, 0.0)
    }

    /// `func_8083F9D0`: still holding on? At a wall, with the block taking the push
    /// (`PLAYER_STATE2_4`) or A held, and the wall still `unk_3C4`'s (the scene's for none): true
    /// while the block takes it (nothing to decide), false to read the stick. Otherwise Link lets
    /// go (`gPlayerAnim_link_normal_push_wait_end`, standing) and it's true.
    fn func_8083F9D0(&mut self, env: &Env) -> bool {
        if self.actor.bg_check_flags & BGCHECKFLAG_PLAYER_WALL_INTERACT != 0 && (self.state2 & STATE2_4 != 0 || self.input.cur.held(BTN_A)) {
            let wall_bg = self.wall_bg_id();
            let wall_poly_actor = if wall_bg != BGCHECK_SCENE { oot_game::actor_ctx::dyna_poly_get_actor(env.actors, env.col, wall_bg) } else { None };
            // &wallPolyActor->actor == this->unk_3C4 (NULL's actor is NULL).
            if wall_poly_actor == self.unk_3c4 {
                return self.state2 & STATE2_4 != 0;
            }
        }
        let data = env.data;
        self.func_80839FFC(data);
        self.skel.play_once(data, data.anim("link_normal_push_wait_end"));
        self.state2 &= !STATE2_4;
        true
    }

    /// `func_8083FAB8`: pushing (`Player_Action_8084B898`), `gPlayerAnim_link_normal_push_start`.
    fn func_8083FAB8(&mut self, data: &GameData) {
        self.setup_action(data, Action::Push, 0);
        self.state2 |= STATE2_4;
        self.skel.play_once(data, data.anim("link_normal_push_start"));
    }

    /// `func_8083FB14`: pulling (`Player_Action_8084B9E4`), `PLAYER_ANIMGROUP_pull_start`.
    fn func_8083FB14(&mut self, data: &GameData) {
        self.setup_action(data, Action::Pull, 0);
        self.state2 |= STATE2_4;
        self.skel.play_once(data, data.player_anim(group::PULL_START, self.model_anim_type));
    }

    /// `func_8083FFB8`: the stick along Link's facing: its speed times the cosine of its angle
    /// off his yaw; 1 forwards (push), -1 backwards (pull), 0 with the stick still or square
    /// across.
    fn func_8083FFB8(&self, arg1: &mut f32, arg2: i16) -> i32 {
        let temp1 = arg2.wrapping_sub(self.actor.shape_rot.y);
        // ABS(temp1) as a u16, then Math_CosS's s16.
        let temp2 = (temp1 as i32).unsigned_abs() as u16;
        let temp3 = cos_s(temp2 as i16);
        *arg1 *= temp3;
        if *arg1 != 0.0 {
            if temp3 > 0.0 { 1 } else { -1 }
        } else {
            0
        }
    }

    /// `func_8084B840`: the push (`arg2` 2) or pull (-2) on the wall's DynaPoly actor
    /// (`func_8002DFA4`: added to its `unk_150`, `unk_158` Link's `world.rot.y`).
    fn func_8084B840(&self, env: &Env, arg2: f32) {
        let wall_bg = self.wall_bg_id();
        if wall_bg != BGCHECK_SCENE {
            // DynaPoly_GetActor, then func_8002DFA4 (nothing for a bg id not in use).
            env.col.dyna.func_8002DFA4(wall_bg, arg2, self.actor.world_rot.y);
        }
    }

    /// `Player_Action_8084B78C`: holding on to the wall (`PLAYER_STATE2_0`, `_6`, `_8`: the push
    /// camera, `CAM_MODE_PUSH_PULL`). Once the animation is done, unless he lets go
    /// (`func_8083F9D0`), the stick along his facing pushes (`func_8083FAB8`) or pulls
    /// (`func_8083FB14`).
    fn action_8084b78c(&mut self, env: &Env) {
        self.state2 |= STATE2_0 | STATE2_6 | STATE2_8;
        self.func_8083F524(env);
        if self.skel.update(env.data) && !self.func_8083F9D0(env) {
            let (_, mut speed_target, yaw_target) = self.get_movement_speed_and_yaw(env, SPEED_MODE_LINEAR);
            let temp = self.func_8083FFB8(&mut speed_target, yaw_target);
            if temp > 0 {
                self.func_8083FAB8(env.data);
            } else if temp < 0 {
                self.func_8083FB14(env.data);
            }
        }
    }

    /// `Player_Action_8084B898`: pushing: `gPlayerAnim_link_normal_push_start`, then
    /// `gPlayerAnim_link_normal_pushing` looped (`av2.actionVar2` 1); `NA_SE_VO_LI_PUSH` on the start's
    /// frame 11, the floor's slips on frames 3 and 21 (`D_80854870`). Unless he lets go: the stick
    /// back pulls, still ends (`gPlayerAnim_link_normal_push_end`), forwards pushes on
    /// (`PLAYER_STATE2_4`). While the push is taken, 2 on the block (`func_8084B840`) and Link
    /// moving at 2.
    fn action_8084b898(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_0 | STATE2_6 | STATE2_8;
        // func_80832CB0.
        if self.skel.update(data) {
            self.skel.play_loop(data, data.anim("link_normal_pushing"));
            self.action_var2 = 1;
        } else if self.action_var2 == 0 && self.skel.on_frame(11.0) {
            self.play_voice_sfx(NA_SE_VO_LI_PUSH);
        }
        self.process_anim_sfx_list(&D_80854870);
        self.func_8083F524(env);
        if !self.func_8083F9D0(env) {
            let (_, mut speed_target, yaw_target) = self.get_movement_speed_and_yaw(env, SPEED_MODE_LINEAR);
            let temp = self.func_8083FFB8(&mut speed_target, yaw_target);
            if temp < 0 {
                self.func_8083FB14(data);
            } else if temp == 0 {
                let anim = data.anim("link_normal_push_end");
                self.func_8083F72C(env, anim);
            } else {
                self.state2 |= STATE2_4;
            }
        }
        if self.state2 & STATE2_4 != 0 {
            self.func_8084B840(env, 2.0);
            self.linear_velocity = 2.0;
        }
    }

    /// `Player_Action_8084B9E4`: pulling: `PLAYER_ANIMGROUP_pull_start`, then `_pulling` looped
    /// (`av2.actionVar2` 1); `NA_SE_VO_LI_PUSH` on the start's frame 11, then the floor's slips on
    /// the loop's frames 4 and 24 (`D_80854878`). Unless he lets go: the stick forwards pushes,
    /// still ends (`PLAYER_ANIMGROUP_pull_end`), back pulls on (`PLAYER_STATE2_4`). While the pull
    /// is taken: with a floor 40 behind him within 20 of his feet (`func_8083973C` at 26 up) and
    /// no wall between (`BgCheck_EntityLineTest1` 26 up), -2 on the block (`func_8084B840`); else
    /// the pull stops (`PLAYER_STATE2_4` cleared).
    fn action_8084b9e4(&mut self, env: &Env) {
        let data = env.data;
        let anim = data.player_anim(group::PULLING, self.model_anim_type);
        self.state2 |= STATE2_0 | STATE2_6 | STATE2_8;
        // func_80832CB0.
        if self.skel.update(data) {
            self.skel.play_loop(data, anim);
            self.action_var2 = 1;
        } else if self.action_var2 == 0 {
            if self.skel.on_frame(11.0) {
                self.play_voice_sfx(NA_SE_VO_LI_PUSH);
            }
        } else {
            self.process_anim_sfx_list(&D_80854878);
        }
        self.func_8083F524(env);
        if !self.func_8083F9D0(env) {
            let (_, mut speed_target, yaw_target) = self.get_movement_speed_and_yaw(env, SPEED_MODE_LINEAR);
            let temp1 = self.func_8083FFB8(&mut speed_target, yaw_target);
            if temp1 > 0 {
                self.func_8083FAB8(data);
            } else if temp1 == 0 {
                let anim = data.player_anim(group::PULL_END, self.model_anim_type);
                self.func_8083F72C(env, anim);
            } else {
                self.state2 |= STATE2_4;
            }
        }
        if self.state2 & STATE2_4 != 0 {
            // func_8083973C: the point behind (sp5C) and the floor under it.
            let sp5c = self.get_relative_position(self.actor.world_pos, D_80854880);
            let temp2 = env.col.entity_raycast_down(sp5c).0 - self.actor.world_pos.y;
            if temp2.abs() < 20.0 {
                let sp44 = Vec3::new(self.actor.world_pos.x, sp5c.y, self.actor.world_pos.z);
                if env.col.entity_line_test(sp44, sp5c, true, false, false, true).is_none() {
                    self.func_8084B840(env, -2.0);
                    return;
                }
            }
            self.state2 &= !STATE2_4;
        }
    }

    /// `Player_TryEnteringCrawlspace`: a child at a crawlspace's wall (`WALL_FLAG_CRAWLSPACE_1`, `_5`), within 8 of the
    /// line through the middle of its triangle: A says "Enter" (`PLAYER_STATE2_DO_ACTION_ENTER`). On A,
    /// into the crawlspace (`PLAYER_STATE2_CRAWLING`): Link lined up with the triangle's middle at
    /// his distance from the wall, facing it, `gPlayerAnim_link_child_tunnel_start` moving him
    /// in (`moveFlags` 0x9D), and the crawl once the item is away (`func_8083A40C`).
    fn try_entering_crawlspace(&mut self, env: &Env, arg2: u32) -> bool {
        if self.adult || self.state1 & STATE1_27 != 0 || arg2 & (WALL_FLAG_CRAWLSPACE_1 | WALL_FLAG_CRAWLSPACE_2) == 0 {
            return false;
        }
        let Some(wall) = self.actor.wall_poly else { return false };
        let col = env.col;
        let v = col.poly_vertices(wall);
        let (mut sp4c, mut phi_f2, mut sp44, mut phi_f12) = (v[0].x, v[0].x, v[0].z, v[0].z);
        for p in &v[1..] {
            if sp4c > p.x {
                sp4c = p.x;
            } else if phi_f2 < p.x {
                phi_f2 = p.x;
            }
            if sp44 > p.z {
                sp44 = p.z;
            } else if phi_f12 < p.z {
                phi_f12 = p.z;
            }
        }
        let sp4c = (sp4c + phi_f2) * 0.5;
        let sp44 = (sp44 + phi_f12) * 0.5;
        let n = col.poly_normal(wall);
        let d = ((self.actor.world_pos.x - sp4c) * n.z) - ((self.actor.world_pos.z - sp44) * n.x);
        if d.abs() < 8.0 {
            self.state2 |= STATE2_16;
            if self.input.press.held(BTN_A) {
                let data = env.data;
                let sp30 = self.wall_distance;
                self.setup_wait_for_put_away(data, env, A74::Crawl);
                self.state2 |= STATE2_18;
                self.current_yaw = self.actor.wall_yaw.wrapping_add(i16::MIN);
                self.actor.shape_rot.y = self.current_yaw;
                self.actor.world_pos.x = sp4c + sp30 * n.x;
                self.actor.world_pos.z = sp44 + sp30 * n.z;
                self.func_80832224();
                self.actor.prev_pos = self.actor.world_pos;
                self.skel.play_once(data, data.anim("link_child_tunnel_start"));
                self.start_anim_movement(0x9D);
                return true;
            }
        }
        false
    }

    /// `func_8083A40C`: the crawl (`Player_Action_8084C760`).
    fn func_8083A40C(&mut self, data: &GameData) {
        self.setup_action_preserve_anim_movement(data, Action::Crawl, 0);
    }

    /// `Player_Action_8084C760`: in a crawlspace. `tunnel_start` plays out first, its root motion moving
    /// Link in; then the stick's forward tilt is the speed (`rel.stick_y` × 0.03, backwards
    /// too) along `currentYaw`, and `Camera_Subj4` keeps Link on the crawlspace's line. At
    /// either end's wall, the way out (`Player_TryLeavingCrawlspace`). The crawl's sounds: `D_808548B4`
    /// (`Player_ProcessAnimSfxList`).
    fn action_8084c760(&mut self, env: &Env) {
        self.state2 |= STATE2_6;
        if self.skel.update(env.data) {
            if self.state1 & STATE1_0 == 0 {
                if self.skel.move_flags != 0 {
                    self.skel.move_flags = 0;
                    return;
                }
                if !self.try_leaving_crawlspace(env) {
                    self.linear_velocity = self.input.rel.stick_y as f32 * 0.03;
                }
            }
            return;
        }
        self.process_anim_sfx_list(env.audio.player_anim_sfx("D_808548B4"));
    }

    /// `Player_TryLeavingCrawlspace`: crawling into a crawlspace's wall (`WALL_FLAG_CRAWLSPACE_1`, `_5`) head first
    /// (backwards, feet first): out, with `tunnel_end` facing away from the wall, or
    /// `tunnel_start` played backwards, each with its one-point cutscene (9601, 9602: the
    /// camera's spline up and out of the crawlspace).
    fn try_leaving_crawlspace(&mut self, env: &Env) -> bool {
        let data = env.data;
        if self.linear_velocity != 0.0 && self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 && self.s.wall_flags & (WALL_FLAG_CRAWLSPACE_1 | WALL_FLAG_CRAWLSPACE_2) != 0 {
            let mut temp = self.actor.shape_rot.y.wrapping_sub(self.actor.wall_yaw);
            if self.linear_velocity < 0.0 {
                temp = temp.wrapping_add(i16::MIN);
            }
            if abs16(temp) > 0x4000 {
                self.setup_action(data, Action::CrawlExit, 0);
                if self.linear_velocity > 0.0 {
                    self.actor.shape_rot.y = self.actor.wall_yaw.wrapping_add(i16::MIN);
                    self.skel.play_once(data, data.anim("link_child_tunnel_end"));
                    self.start_anim_movement(0x9D);
                    self.play_requests.push(PlayRequest::OnePointCutscene { cs_id: 9601, timer: 999, player: false, parent: CAM_ID_MAIN });
                } else {
                    self.actor.shape_rot.y = self.actor.wall_yaw;
                    self.play_backwards(data, data.anim("link_child_tunnel_start"));
                    self.start_anim_movement(0x9D);
                    self.play_requests.push(PlayRequest::OnePointCutscene { cs_id: 9602, timer: 999, player: false, parent: CAM_ID_MAIN });
                }
                self.current_yaw = self.actor.shape_rot.y;
                self.zero_speed_xz();
                return true;
            }
        }
        false
    }

    /// `Player_Action_8084C81C`: out of the crawlspace; standing once the animation ends
    /// (`func_8083C0E8`). Its sounds: `D_808548D8`.
    fn action_8084c81c(&mut self, env: &Env) {
        self.state2 |= STATE2_6;
        if self.skel.update(env.data) {
            self.func_8083C0E8(env.data);
            self.state2 &= !STATE2_18;
            return;
        }
        self.process_anim_sfx_list(env.audio.player_anim_sfx("D_808548D8"));
    }

    // ================================================================================
    // Damage (func_808382DC) and the knockdown

    /// `Player_InBlockingCsMode` (magic isn't ported).
    fn in_blocking_cs_mode(&self, env: &Env) -> bool {
        self.state1 & (STATE1_7 | STATE1_29) != 0
            || self.cs_mode != 0
            || env.io.borrow().transition.trigger == TRANS_TRIGGER_START
            || self.state1 & STATE1_0 != 0
            || self.state3 & STATE3_7 != 0
    }

    /// `Player_SetIntangibility`: `timer` frames of invincibility, unless the roll's is running.
    fn set_intangibility(&mut self, timer: i8) {
        if self.invincibility_timer >= 0 {
            self.invincibility_timer = timer;
            self.damage_flicker_anim_counter = 0;
        }
    }

    /// `Player_SetInvulnerability`: at most `timer`.
    fn set_invulnerability(&mut self, timer: i8) {
        if self.invincibility_timer > timer {
            self.invincibility_timer = timer;
        }
        self.damage_flicker_anim_counter = 0;
    }

    /// `func_80837B18`: `Health_ChangeBy(damage)` unless invincible; false once Link is out of
    /// health.
    fn func_80837B18(&mut self, env: &Env, damage: i32) -> bool {
        if self.invincibility_timer != 0 || self.actor.category != ACTORCAT_PLAYER {
            return true;
        }
        // Health_ChangeBy's recovery sound (for a gain), through Player's requests.
        if damage > 0 {
            self.sfx(PlayerSfx::NoPos(NA_SE_SY_HP_RECOVER));
        }
        oot_game::item::health_change_by(&mut env.io.borrow_mut().save, None, damage as i16)
    }

    /// `Player_InflictDamage`: true when it took the last of Link's health.
    fn player_inflict_damage(&mut self, env: &Env, damage: i32) -> bool {
        if !self.in_blocking_cs_mode(env) && !self.func_80837B18(env, damage) {
            self.state2 &= !STATE2_7;
            return true;
        }
        false
    }

    /// `func_808382BC`: at least 20 frames of invincibility.
    fn func_808382BC(&mut self) {
        if self.invincibility_timer >= 0 && self.invincibility_timer < 20 {
            self.invincibility_timer = 20;
        }
    }

    /// `func_808382DC`: what hurts Link this frame; true when something did (then there's no
    /// ledge check, `func_8083AA10`):
    /// - being crushed, a void floor (`FLOOR_TYPE_9`) or `PLAYER_STATE2_31`: the respawn or
    ///   the void-out;
    /// - the knockback an actor asked for (`knockbackType`, `Actor_SetPlayerKnockback`), even while invincible
    ///   for kind 2 and up;
    /// - a hit on the body cylinder (`AC_HIT`);
    /// - a hurting wall or floor (`func_80042108`, the hot floors `FLOOR_TYPE_2`, `_3`).
    ///
    /// A hit on the shield (its quad `AC_BOUNCED`) is blocked instead: Link is pushed back at 18
    /// with the guard's recoil (`Player_Action_808435C4`; on the upper body outside the guard,
    /// `func_80834BD4`), and a fire hit burns a Deku Shield away (`func_8083819C`).
    ///
    /// `unk_A86`'s damage is never set. The rumble isn't ported.
    fn func_808382DC(&mut self, env: &Env) -> bool {
        use oot_game::actor::BGCHECKFLAG_CRUSHED;
        if self.unk_A86 != 0 {
            if !self.in_blocking_cs_mode(env) {
                self.player_inflict_damage(env, -16);
                self.unk_A86 = 0;
            }
            return true;
        }
        // Player_GetHeight (not riding).
        let height = if self.adult { 68.0 } else { 44.0 };
        let sp68 = (height - 8.0) < self.unk_6C4 * self.actor.scale.y;
        if sp68 || self.actor.bg_check_flags & BGCHECKFLAG_CRUSHED != 0 || self.s.floor_type == FLOOR_TYPE_9 || self.state2 & STATE2_31 != 0 {
            self.play_voice_sfx(NA_SE_VO_LI_DAMAGE_S);
            let mut io = env.io.borrow_mut();
            if sp68 {
                let (pos, yaw) = (self.actor.world_pos, self.actor.shape_rot.y);
                io.trigger_respawn(pos, yaw);
                io.set_transition_for_next_entrance(env.entrances);
            } else {
                // The Forest Temple's checkerboard ceiling room and the Shadow Temple's falling
                // spikes respawn Link at a set place.
                let special = match (io.scene_id, io.room) {
                    (SCENE_FOREST_TEMPLE, 15) => Some(Vec3::new(1992.0, 403.0, -3432.0)),
                    (SCENE_SHADOW_TEMPLE, 10) => Some(Vec3::new(1200.0, -1343.0, 3850.0)),
                    _ => None,
                };
                if let Some(pos) = special {
                    let (p, yaw) = (self.actor.world_pos, self.actor.shape_rot.y);
                    io.setup_respawn_point(RESPAWN_MODE_DOWN, 0xDFF, p, yaw);
                    io.save.respawn[RESPAWN_MODE_DOWN].pos = pos;
                    io.save.respawn[RESPAWN_MODE_DOWN].yaw = 0;
                }
                io.trigger_void_out();
            }
            drop(io);
            self.play_voice_sfx(NA_SE_VO_LI_TAKEN_AWAY);
            // (play->haltAllActors = true: not modelled.)
            self.sfx(PlayerSfx::NoPos(NA_SE_OC_ABYSS));
            return true;
        }
        if self.knockback_type != PLAYER_KNOCKBACK_NONE && (self.knockback_type >= PLAYER_KNOCKBACK_LARGE || self.invincibility_timer == 0) {
            // knockbackResponse, by knockbackType.
            const KNOCKBACK_RESPONSE: [i32; 3] = [PLAYER_HIT_RESPONSE_KNOCKBACK_SMALL, PLAYER_HIT_RESPONSE_KNOCKBACK_LARGE, PLAYER_HIT_RESPONSE_KNOCKBACK_LARGE];
            self.func_80838280(env);
            if self.knockback_type == PLAYER_KNOCKBACK_LARGE_ELECTRIFIED {
                self.body_shock_timer = 40;
            }
            self.actor.col_chk_info.damage = self.actor.col_chk_info.damage.wrapping_add(self.knockback_damage);
            let (kind, speed, vy, yaw) = (KNOCKBACK_RESPONSE[self.knockback_type as usize - 1], self.knockback_speed, self.knockback_y_velocity, self.knockback_rot);
            self.func_80837C0C(env, kind, speed, vy, yaw, 20);
            return true;
        }
        // sp64: the shield's AC_BOUNCED, or the roll's block. @bug (game): that one tests the
        // attacking collider's u8 atFlags against 0x20000000, so it never holds.
        let sp64 = self.shield_quad.base.ac_flags & cc::AC_BOUNCED != 0;
        if sp64 {
            let data = env.data;
            // (Player_RequestRumble: the rumble isn't ported.)
            if !self.is_child_with_hylian_shield() {
                if self.invincibility_timer >= 0 {
                    let sp54 = self.action == Action::Guard;
                    if !self.func_808332B8() {
                        self.setup_action(data, Action::GuardHit, 0);
                    }
                    self.action_var1 = sp54 as _;
                    let two = self.holds_two_handed_weapon(data) as usize;
                    if !sp54 {
                        self.set_upper_action_func(UpperAction::ShieldHit);
                        // D_808543BC, D_808543B4.
                        let anim = if self.unk_870 < 0.5 { ["link_anchor_defense_hit", "link_anchor_defense_long_hitR"][two] } else { ["link_anchor_defense_hit", "link_anchor_defense_long_hitL"][two] };
                        let a = data.anim(anim);
                        self.skel2.play_once(data, a);
                    } else {
                        // D_808543C4.
                        let a = data.anim(["link_normal_defense_hit", "link_fighter_defense_long_hit"][two]);
                        self.skel.play_once(data, a);
                    }
                }
                if self.state1 & (STATE1_13 | STATE1_14 | STATE1_21) == 0 {
                    self.linear_velocity = -18.0;
                    self.current_yaw = self.actor.shape_rot.y;
                }
            }
            if self.shield_quad.info.ac_hit_elem.is_some_and(|h| h.at_dmg_info.hit_special_effect == cc::HIT_SPECIAL_EFFECT_FIRE) {
                self.func_8083819C(env);
            }
            return false;
        }
        if self.unk_A87 != 0
            || self.invincibility_timer > 0
            || self.state1 & STATE1_26 != 0
            || self.cs_mode != 0
            || self.melee_weapon_quads[0].base.at_flags & cc::AT_HIT != 0
            || self.melee_weapon_quads[1].base.at_flags & cc::AT_HIT != 0
        {
            return false;
        }
        if self.cylinder.base.ac_flags & cc::AC_HIT != 0 {
            if self.cylinder.base.ac.and_then(|h| env.target(h)).is_some_and(|a| a.flags & ACTOR_FLAG_SFX_FOR_PLAYER_BODY_HIT != 0) {
                self.play_sfx(NA_SE_PL_BODY_HIT);
            }
            let ac_pos = self.cylinder.base.ac.and_then(|h| env.target(h)).map(|a| a.world_pos).unwrap_or(self.actor.world_pos);
            let effect = self.actor.col_chk_info.ac_hit_special_effect;
            let sp4c = if self.state1 & STATE1_27 != 0 {
                PLAYER_HIT_RESPONSE_NONE
            } else {
                match effect {
                    cc::HIT_SPECIAL_EFFECT_ICE => PLAYER_HIT_RESPONSE_FROZEN,
                    cc::HIT_SPECIAL_EFFECT_ELECTRIC => PLAYER_HIT_RESPONSE_ELECTRIFIED,
                    cc::HIT_SPECIAL_EFFECT_KNOCKBACK => PLAYER_HIT_RESPONSE_KNOCKBACK_LARGE,
                    _ => {
                        self.func_80838280(env);
                        PLAYER_HIT_RESPONSE_NONE
                    }
                }
            };
            // Actor_WorldYawTowardActor(ac, &this->actor).
            let yaw = vec3f_yaw(ac_pos, self.actor.world_pos);
            self.func_80837C0C(env, sp4c, 4.0, 5.0, yaw, 20);
            return true;
        }
        if self.invincibility_timer != 0 {
            return false;
        }
        // D_808544F4: in the Goron tunic, the frames on a hot floor (FLOOR_TYPE_2, _3) before it
        // hurts; without it, it hurts at once.
        const D_808544F4: [u8; 2] = [120, 60];
        let sp48 = self.s.floor_type.wrapping_sub(FLOOR_TYPE_2);
        let sp48 = (sp48 <= FLOOR_TYPE_3 - FLOOR_TYPE_2).then_some(sp48 as usize);
        let col = env.col;
        let wall_hurts = self.actor.wall_poly.is_some_and(|w| col.flag27(w));
        let floor_hurts = self.actor.floor_poly.is_some_and(|f| col.flag27(f));
        // PLAYER_TUNIC_GORON is 1.
        if wall_hurts
            || sp48.is_some_and(|i| floor_hurts && self.floor_type_timer >= D_808544F4[i])
            || sp48.is_some_and(|i| self.current_tunic != 1 || self.floor_type_timer >= D_808544F4[i])
        {
            self.floor_type_timer = 0;
            self.actor.col_chk_info.damage = 4;
            let yaw = self.actor.shape_rot.y;
            self.func_80837C0C(env, 0, 4.0, 5.0, yaw, 20);
            return true;
        }
        false
    }

    /// `D_808544B0`: the staggers, by (strong hit) × 4 + (from behind) × 2 + (locked on).
    const D_808544B0: [&'static str; 8] = [
        "link_normal_front_shit",
        "link_normal_front_shitR",
        "link_normal_back_shit",
        "link_normal_back_shitR",
        "link_normal_front_hit",
        "link_anchor_front_hitR",
        "link_normal_back_hit",
        "link_anchor_back_hitR",
    ];

    /// `func_80837C0C`: hurt by `colChkInfo.damage`, `arg6` frames of invincibility, and the
    /// reaction (`arg2`, `PLAYER_HIT_RESPONSE_*`):
    /// - frozen (`Player_Action_8084FB10`) or electrified (`Player_Action_8084FBF4`);
    /// - swimming, the swimming hit (`Player_Action_8084E30C`);
    /// - knocked down (`Player_Action_8084377C`) for the knockbacks, in the air, hanging or
    ///   climbing, at `arg3` and `arg4` from `arg5` (the small knockback with its own);
    /// - otherwise a stagger (`Player_Action_8084370C`), or only a flinch when running fast
    ///   (`unk_890`).
    ///
    /// Out of health it takes no reaction (a fall in the air). The rumble isn't ported.
    fn func_80837C0C(&mut self, env: &Env, arg2: i32, arg3: f32, arg4: f32, mut arg5: i16, arg6: i8) {
        let data = env.data;
        let mut sp2c = None;
        if self.state1 & STATE1_13 != 0 {
            self.func_80837B60();
        }
        self.unk_890 = 0;
        self.play_sfx(NA_SE_PL_DAMAGE);
        let damage = self.actor.col_chk_info.damage as i32;
        if !self.func_80837B18(env, -damage) {
            self.state2 &= !STATE2_7;
            if !self.grounded() && self.state1 & STATE1_27 == 0 {
                self.func_80837B9C(data);
            }
            return;
        }
        self.set_intangibility(arg6);
        if arg2 == PLAYER_HIT_RESPONSE_FROZEN {
            self.setup_action(data, Action::Frozen, 0);
            sp2c = Some(data.anim("link_normal_ice_down"));
            self.func_80832224();
            // (Player_RequestRumble(this, 255, 10, 40, 0).)
            self.play_sfx(NA_SE_PL_FREEZE_S);
            self.play_voice_sfx(NA_SE_VO_LI_FREEZE);
        } else if arg2 == PLAYER_HIT_RESPONSE_ELECTRIFIED {
            self.setup_action(data, Action::Electrified, 0);
            // (Player_RequestRumble(this, 255, 80, 150, 0).) Player_AnimPlayLoopAdjusted.
            let a = data.anim("link_normal_electric_shock");
            self.skel.play_loop_set_speed(data, a, 2.0 / 3.0);
            self.func_80832224();
            self.action_var2 = 20;
        } else {
            if !self.func_80837C0C_hit(env, arg2, arg3, arg4, &mut arg5, &mut sp2c) {
                // The running flinch: no reaction.
                return;
            }
        }
        self.func_80832564(env.data);
        self.state1 |= STATE1_26;
        if let Some(a) = sp2c {
            // Player_AnimPlayOnceAdjusted.
            self.skel.play_once_set_speed(data, a, 2.0 / 3.0);
        }
    }

    /// `func_80837C0C`'s reactions other than frozen and electrified: the swimming hit, the
    /// knockdown or the stagger, then the facing. False for the running flinch (`unk_890` 20 and
    /// nothing else: the C returns there).
    fn func_80837C0C_hit(&mut self, env: &Env, arg2: i32, arg3: f32, arg4: f32, arg5: &mut i16, sp2c: &mut Option<AnimId>) -> bool {
        let data = env.data;
        *arg5 = arg5.wrapping_sub(self.actor.shape_rot.y);
        let arg5 = *arg5;
        if self.state1 & STATE1_27 != 0 {
            self.setup_action(data, Action::SwimDamaged, 0);
            // (Player_RequestRumble(this, 180, 20, 50, 0).)
            self.linear_velocity = 4.0;
            self.actor.velocity.y = 0.0;
            *sp2c = Some(data.anim("link_swimer_swim_hit"));
            self.play_voice_sfx(NA_SE_VO_LI_DAMAGE_S);
        } else if arg2 == PLAYER_HIT_RESPONSE_KNOCKBACK_LARGE || arg2 == PLAYER_HIT_RESPONSE_KNOCKBACK_SMALL || !self.grounded() || self.state1 & (STATE1_13 | STATE1_14 | STATE1_21) != 0 {
            self.setup_action(data, Action::KnockedDown, 0);
            self.state3 |= STATE3_1;
            self.func_80832224();
            if arg2 == PLAYER_HIT_RESPONSE_KNOCKBACK_SMALL {
                self.action_var2 = 4;
                self.actor.speed_xz = 3.0;
                self.linear_velocity = 3.0;
                self.actor.velocity.y = 6.0;
                // Player_AnimChangeFreeze.
                let a = self.anim(data, group::DAMAGE_RUN);
                self.skel.change(data, a, 1.0, 0.0, 0.0, ANIMMODE_ONCE, 0.0);
                self.play_voice_sfx(NA_SE_VO_LI_DAMAGE_S);
            } else {
                self.actor.speed_xz = arg3;
                self.linear_velocity = arg3;
                self.actor.velocity.y = arg4;
                *sp2c = Some(if abs16(arg5) > 0x4000 { data.anim("link_normal_front_downA") } else { data.anim("link_normal_back_downA") });
                // (NA_SE_VO_BL_DOWN for a dead non-Player: Link is the Player.)
                self.play_voice_sfx(NA_SE_VO_LI_FALL_L);
            }
            self.hover_boots_timer = 0;
            self.actor.bg_check_flags &= !BGCHECKFLAG_GROUND;
        } else {
            if self.linear_velocity > 4.0 && self.state1 & STATE1_4 == 0 {
                self.unk_890 = 20;
                // (Player_RequestRumble(this, 120, 20, 10, 0).)
                self.play_voice_sfx(NA_SE_VO_LI_DAMAGE_S);
                return false;
            }
            self.setup_action(data, Action::Damaged, 0);
            self.func_80833C3C();
            let mut i = 0;
            if self.actor.col_chk_info.damage >= 5 {
                self.linear_velocity = 23.0;
                i += 4;
            }
            if abs16(arg5) <= 0x4000 {
                i += 2;
            }
            if self.state1 & STATE1_4 != 0 {
                i += 1;
            }
            *sp2c = Some(data.anim(Self::D_808544B0[i]));
            self.play_voice_sfx(NA_SE_VO_LI_DAMAGE_S);
        }
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(arg5);
        self.current_yaw = self.actor.shape_rot.y;
        self.actor.world_rot.y = self.actor.shape_rot.y;
        if abs16(arg5) > 0x4000 {
            self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(i16::MIN);
        }
        true
    }

    /// `Player_Action_8084370C`: staggering; standing again at the end (`func_80839F90`).
    fn action_8084370c(&mut self, env: &Env) {
        self.decelerate_to_zero();
        let sp1c = self.try_action_interrupt(env, 16.0);
        if sp1c != 0 && (self.skel.update(env.data) || sp1c > 0) {
            self.func_80839F90(env.data);
        }
    }

    /// `Player_Action_8084377C`: knocked down. While another knockback comes in the air (`knockbackType`),
    /// it takes its yaw and speed. On the ground at the animation's end: after kind 2's
    /// four frames, standing (`func_80853080`); else, once nothing hits any more, lying down
    /// (`Player_Action_80843954`) with `front_downB` or `back_downB`; the bounce's sound on touching down.
    fn action_8084377c(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5 | STATE2_6;
        self.func_808382BC();
        if self.state1 & STATE1_29 == 0 && self.action_var2 == 0 && self.knockback_type != 0 {
            let temp = self.actor.shape_rot.y.wrapping_sub(self.knockback_rot);
            self.actor.shape_rot.y = self.knockback_rot;
            self.current_yaw = self.knockback_rot;
            self.linear_velocity = self.knockback_speed;
            if abs16(temp) > 0x4000 {
                self.actor.shape_rot.y = self.knockback_rot.wrapping_add(i16::MIN);
            }
            if self.actor.velocity.y < 0.0 {
                self.actor.gravity = 0.0;
                self.actor.velocity.y = 0.0;
            }
        }
        if self.skel.update(data) && self.grounded() {
            if self.action_var2 != 0 {
                self.action_var2 -= 1;
                if self.action_var2 == 0 {
                    self.func_80853080(data);
                }
            } else if self.state1 & STATE1_29 != 0 || (self.cylinder.base.ac_flags & cc::AC_HIT == 0 && self.knockback_type == 0) {
                if self.state1 & STATE1_29 != 0 {
                    self.action_var2 += 1;
                } else {
                    self.setup_action(data, Action::Down, 0);
                    self.state1 |= STATE1_26;
                }
                let a = if self.current_yaw != self.actor.shape_rot.y { data.anim("link_normal_front_downB") } else { data.anim("link_normal_back_downB") };
                self.skel.play_once(data, a);
                self.play_voice_sfx(NA_SE_VO_LI_FREEZE);
            }
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 {
            self.play_floor_sfx(NA_SE_PL_BOUND);
        }
    }

    /// `Player_Action_80843954`: lying down until the slide stops, then getting up (`Player_Action_80843A38`)
    /// with `front_down_wake` or `back_down_wake`.
    fn action_80843954(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5 | STATE2_6;
        self.func_808382BC();
        self.decelerate_to_zero();
        if self.skel.update(data) && self.linear_velocity == 0.0 {
            if self.state1 & STATE1_29 != 0 {
                self.action_var2 += 1;
            } else {
                self.setup_action(data, Action::GetUp, 0);
                self.state1 |= STATE1_26;
            }
            let a = if self.current_yaw != self.actor.shape_rot.y { data.anim("link_normal_front_down_wake") } else { data.anim("link_normal_back_down_wake") };
            self.skel.play_once_set_speed(data, a, 2.0 / 3.0);
            self.current_yaw = self.actor.shape_rot.y;
        }
    }

    /// `Player_Action_80843A38`: getting up; standing at the end (`func_80839F90`), with
    /// `D_808545DC`'s sounds.
    fn action_80843a38(&mut self, env: &Env) {
        self.state2 |= STATE2_5;
        self.func_808382BC();
        if self.state1 & STATE1_29 != 0 {
            self.skel.update(env.data);
        } else {
            let sp24 = self.try_action_interrupt(env, 16.0);
            if sp24 != 0 && (self.skel.update(env.data) || sp24 > 0) {
                self.func_80839F90(env.data);
            }
        }
        self.process_anim_sfx_list(env.audio.player_anim_sfx("D_808545DC"));
    }

    // ================================================================================
    // Burning and the shock (Player_UpdateBodyBurn, Player_UpdateBodyShock)

    /// `func_80838280`: a fire hit sets Link burning; Link cries out.
    fn func_80838280(&mut self, env: &Env) {
        if self.actor.col_chk_info.ac_hit_special_effect == cc::HIT_SPECIAL_EFFECT_FIRE {
            self.func_8083821C(env);
        }
        self.play_voice_sfx(NA_SE_VO_LI_FALL_L);
    }

    /// `func_8083821C`: every body part catches fire, for 0 to 199 (`Rand_S16Offset(0, 200)`).
    fn func_8083821C(&mut self, env: &Env) {
        let mut io = env.io.borrow_mut();
        for t in self.body_flame_timers.iter_mut() {
            *t = io.rand.s16_offset(0, 200) as u8;
        }
        self.body_is_burning = true;
    }

    /// `func_8083819C`: a Deku Shield worn burns away: thrown off as `Item_Shield`, out of the
    /// equipment (`Inventory_DeleteEquipment`, which runs `Player_SetEquipmentData`), with its
    /// text.
    fn func_8083819C(&mut self, env: &Env) {
        if self.current_shield == PLAYER_SHIELD_DEKU {
            self.play_requests.push(PlayRequest::BurnDekuShield(self.actor.world_pos));
            let mut io = env.io.borrow_mut();
            oot_game::item::inventory_delete_equipment(&mut io.save, oot_game::item::EQUIP_TYPE_SHIELD);
            let save = io.save.clone();
            drop(io);
            self.set_equipment_data(env.data, &save);
        }
    }

    /// `Player_UpdateBodyShock`: the shock's sparks (`EffectSsFhgFlash_SpawnShock`, which then
    /// jump about Link's body parts themselves), every so often, each with its crackle
    /// (`NA_SE_PL_SPARK`).
    fn update_body_shock(&mut self, env: &Env) {
        self.body_shock_timer -= 1;
        self.unk_892 = self.unk_892.wrapping_add(self.body_shock_timer);
        if self.unk_892 > 20 {
            let mut shock_scale = self.body_shock_timer as i32 * 2;
            self.unk_892 -= 20;
            if shock_scale > 40 {
                shock_scale = 40;
            }
            let mut io = env.io.borrow_mut();
            let part = io.rand.zero_float(BODYPART_MAX as f32 - 0.1) as usize;
            let bp = self.body_parts_pos[part];
            let p = self.actor.world_pos;
            let x = (io.rand.centered_float(5.0) + bp.x) - p.x;
            let y = (io.rand.centered_float(5.0) + bp.y) - p.y;
            let z = (io.rand.centered_float(5.0) + bp.z) - p.z;
            io.ss().fhg_flash_spawn_shock(env.me, Vec3::new(x, y, z), shock_scale as i16, oot_game::effect::fhg_flash::FHGFLASH_SHOCK_PLAYER);
            drop(io);
            self.actor.play_sfx_flagged2(NA_SE_PL_SPARK - SFX_FLAG);
        }
    }

    /// `Player_UpdateBodyBurn`: the flames burn down (faster running, slowly in the Goron Tunic,
    /// at once with `PLAYER_STATE2_3`), with the torch's roar and half a heart's quarter off
    /// every 8 frames while any burns (every frame in Twinrova's room). A Deku Shield burns away
    /// first (`func_8083819C`). Each burning part has its flame this frame
    /// (`EffectSsFireTail_SpawnFlameOnPlayer`), as big and bright as its timer.
    fn update_body_burn(&mut self, env: &Env) {
        let sp54 = if self.current_tunic == PLAYER_TUNIC_GORON { 20 } else { (self.linear_velocity * 0.4) as i32 + 1 };
        let sp58 = if self.state2 & STATE2_3 != 0 { 100 } else { 0 };
        let mut spawned_flame = false;
        self.func_8083819C(env);
        for (i, t) in self.body_flame_timers.iter_mut().enumerate() {
            let timer_step = sp58 + sp54;
            if *t as i32 <= timer_step {
                *t = 0;
            } else {
                spawned_flame = true;
                *t = (*t as i32 - timer_step) as u8;
                let flame_scale = if *t as f32 > 20.0 { ((*t as f32 - 20.0) * 0.01).clamp(0.199_999_99, 0.2) } else { *t as f32 * 0.01 };
                let flame_intensity = ((*t as f32 - 25.0) * 0.02).clamp(0.0, 1.0);
                if let Some(me) = env.me {
                    let pos = self.body_parts_pos[i];
                    env.io.borrow_mut().ss().fire_tail_spawn_flame(me, self.actor.velocity, pos, flame_scale, i as i16, flame_intensity);
                }
            }
        }
        if spawned_flame {
            self.play_sfx(NA_SE_EV_TORCH - SFX_FLAG);
            let dmg_cooldown = if env.scene_id == SCENE_SPIRIT_TEMPLE_BOSS { 0 } else { 7 };
            if dmg_cooldown & env.gameplay_frames == 0 {
                self.player_inflict_damage(env, -1);
            }
        } else {
            self.body_is_burning = false;
        }
    }

    /// `Player_UpdateBurningDekuStick`: a burning stick (`unk_860` counting down from 210). Burnt
    /// away (`unk_85C` 0), it's put away. At 0: one stick less, the stick gone (`unk_85C` 0, and
    /// `unk_860` 1 so the next frame puts it away); over 200 the flame grows over 10 frames;
    /// under 20 the stick shrinks with it. The flame is a dust puff at the tip each frame
    /// (`func_8002836C`: rising at 0.5 a frame, yellow in red, up to 200 big, 8 frames).
    fn update_burning_deku_stick(&mut self, env: &Env) {
        // D_808547A4, D_808547B0, D_808547BC, D_808547C0.
        const VELOCITY: Vec3 = Vec3::new(0.0, 0.5, 0.0);
        const ACCEL: Vec3 = Vec3::new(0.0, 0.5, 0.0);
        const PRIM: [u8; 4] = [255, 255, 100, 255];
        const ENV: [u8; 4] = [255, 50, 0, 0];
        if self.unk_85c == 0.0 {
            self.use_item(env.data, oot_game::item::ITEM_NONE);
            return;
        }
        let mut temp = 1.0f32;
        // DECR(unk_860): the new value, held at 0.
        if self.unk_860 != 0 {
            self.unk_860 -= 1;
        }
        if self.unk_860 == 0 {
            self.change_ammo(env, oot_game::item::ITEM_DEKU_STICK, -1);
            self.unk_860 = 1;
            temp = 0.0;
            self.unk_85c = temp;
        } else if self.unk_860 > 200 {
            temp = (210 - self.unk_860) as f32 / 10.0;
        } else if self.unk_860 < 20 {
            temp = self.unk_860 as f32 / 20.0;
            self.unk_85c = temp;
        }
        let tip = self.melee_weapon_info[0].tip;
        env.io.borrow_mut().ss().func_8002836c(tip, VELOCITY, ACCEL, PRIM, ENV, (temp * 200.0) as i16, 0, 8);
    }

    // ================================================================================
    // Frozen, electrified, the swimming hit

    /// `func_80832594`: struggling out of the ice: `actionVar2` grows with the stick's spin and
    /// 5 per A or B; true once past `arg2`.
    fn func_80832594(&mut self, arg1: i32, arg2: i32) -> bool {
        let control_stick_angle_diff = self.prev_control_stick_angle.wrapping_sub(self.s.stick_angle);
        let k = ((control_stick_angle_diff as i32).abs() as f32 * self.s.stick_mag.abs() * 2.541_580_2e-6) as i16;
        self.action_var2 = self.action_var2.wrapping_add((arg1 as i16).wrapping_add(k));
        if self.input.press.held(BTN_A) || self.input.press.held(eng_input::pad::BTN_B) {
            self.action_var2 = self.action_var2.wrapping_add(5);
        }
        self.action_var2 as i32 > arg2
    }

    /// `Player_Action_8084FB10`: frozen. A quarter heart off every 4 frames until Link struggles
    /// free (`func_80832594`), shattering the ice (`EffectSsIcePiece_SpawnBurst`, an effect, not
    /// ported); then standing at the animation's end, invulnerable for 20 frames.
    fn action_8084fb10(&mut self, env: &Env) {
        if self.action_var1 >= 0 {
            if self.action_var1 < 6 {
                self.action_var1 += 1;
            }
            if self.func_80832594(1, 100) {
                self.action_var1 = -1;
                self.play_sfx(NA_SE_PL_ICE_BROKEN);
            } else {
                self.state2 |= STATE2_14;
            }
            if env.gameplay_frames % 4 == 0 {
                self.player_inflict_damage(env, -1);
            }
        } else if self.skel.update(env.data) {
            self.func_80839F90(env.data);
            self.set_invulnerability(-20);
        }
    }

    /// `Player_Action_8084FBF4`: electrified for 20 frames, a quarter heart off at the start of
    /// each 25 (`func_80837B18`: out of health, the count stops), the sparks going and the cry.
    fn action_8084fbf4(&mut self, env: &Env) {
        self.skel.update(env.data);
        self.func_808382BC();
        if self.action_var2 % 25 != 0 || self.func_80837B18(env, -1) {
            // DECR(this->av2.actionVar2) == 0.
            if self.action_var2 != 0 {
                self.action_var2 -= 1;
            }
            if self.action_var2 == 0 {
                self.func_80839F90(env.data);
            }
        }
        self.body_shock_timer = 40;
        self.actor.play_sfx_flagged2((NA_SE_VO_LI_TAKEN_AWAY - SFX_FLAG).wrapping_add(self.age.climb.unk_92));
    }

    /// `Player_Action_8084E30C`: hit while swimming; treading water again at the animation's end.
    fn action_8084e30c(&mut self, env: &Env) {
        self.func_8084B000();
        if self.skel.update(env.data) {
            self.func_80838F18(env.data);
        }
        let yaw = self.actor.shape_rot.y;
        self.linear_velocity = self.func_8084AEEC(self.linear_velocity, 0.0, yaw);
    }

    // ================================================================================
    // Death and the revival

    /// `func_80836448`: Link dies with `anim` (swimming: `Player_Action_8084E368`, else
    /// `Player_Action_80843CEC`; `PLAYER_STATE1_DEAD`). The music goes; a bottled fairy is used up
    /// and revives him (`GAMEOVER_REVIVE_START`), else it's the game over
    /// (`GAMEOVER_DEATH_START`, its fanfare). The camera's one-point cutscene 9806 and the
    /// letterbox follow.
    fn func_80836448(&mut self, env: &Env, anim: AnimId) {
        use oot_game::game_over::{GAMEOVER_DEATH_START, GAMEOVER_REVIVE_START};
        let data = env.data;
        let cond = self.func_808332B8();
        self.func_80832564(data);
        self.setup_action(data, if cond { Action::DyingInWater } else { Action::Dying }, 0);
        self.state1 |= STATE1_7;
        self.skel.play_once(data, anim);
        if anim == data.anim("link_derth_rebirth") {
            self.skel.end_frame = 84.0;
        }
        self.func_80832224();
        self.play_voice_sfx(NA_SE_VO_LI_DOWN);
        if self.actor.category == ACTORCAT_PLAYER {
            self.play_requests.push(PlayRequest::Audio(PlayerAudio::BgmVolumeOffDuringFanfare));
            let mut io = env.io.borrow_mut();
            if oot_game::item::inventory_consume_fairy(&mut io.save) {
                drop(io);
                self.play_requests.push(PlayRequest::GameOverState(GAMEOVER_REVIVE_START));
                self.action_var1 = 1;
            } else {
                io.save.seq_id = oot_game::audio::NA_BGM_DISABLED as u8;
                io.save.nature_ambience_id = oot_game::audio::NATURE_ID_DISABLED;
                drop(io);
                self.play_requests.push(PlayRequest::GameOverState(GAMEOVER_DEATH_START));
                self.play_requests.push(PlayRequest::Audio(PlayerAudio::StopBgmAndFanfare(0)));
                self.play_requests.push(PlayRequest::Audio(PlayerAudio::PlayFanfare(oot_game::audio::NA_BGM_GAME_OVER)));
            }
            self.play_requests.push(PlayRequest::OnePointCutscene { cs_id: 9806, timer: if cond { 120 } else { 60 }, player: true, parent: CAM_ID_MAIN });
            self.play_requests.push(PlayRequest::LetterboxSizeTarget(32));
        }
    }

    /// `func_80843AE8`, once the death's animation has ended:
    /// - reviving (`actionVar1`): the fairy comes out (`Player_SpawnFairy`, `FAIRY_REVIVE_DEATH`)
    ///   with its sound and the camera's one-point cutscene 9908; 60 frames later Link gets up
    ///   (`gPlayerAnim_link_derth_rebirth` from frame 99) with 20 hearts' worth to count in
    ///   (`healthAccumulator` 0x140), and once counted he stands, invulnerable for 20 frames;
    /// - dead: the game over waits a second for its menu (`GAMEOVER_DEATH_DELAY_MENU`).
    fn func_80843AE8(&mut self, env: &Env) {
        use oot_game::game_over::{GAMEOVER_DEATH_DELAY_MENU, GAMEOVER_DEATH_WAIT_GROUND};
        let data = env.data;
        if self.action_var2 != 0 {
            if self.action_var2 > 0 {
                self.action_var2 -= 1;
                if self.action_var2 == 0 {
                    if self.state1 & STATE1_27 != 0 {
                        let a = data.anim("link_swimer_swim_wait");
                        let last = data.anims[a].last_frame();
                        self.skel.change(data, a, 1.0, 0.0, last, ANIMMODE_ONCE, -16.0);
                    } else {
                        let a = data.anim("link_derth_rebirth");
                        let last = data.anims[a].last_frame();
                        self.skel.change(data, a, 1.0, 99.0, last, ANIMMODE_ONCE, 0.0);
                    }
                    env.io.borrow_mut().save.health_accumulator = 0x140;
                    self.action_var2 = -1;
                }
            } else if env.io.borrow().save.health_accumulator == 0 {
                self.state1 &= !STATE1_7;
                if self.state1 & STATE1_27 != 0 {
                    self.func_80838F18(data);
                } else {
                    self.func_80853080(data);
                }
                self.unk_A87 = 20;
                self.set_invulnerability(-20);
                self.play_requests.push(PlayRequest::Audio(PlayerAudio::BgmVolumeOnDuringFanfare));
            }
        } else if self.action_var1 != 0 {
            self.action_var2 = 60;
            // D_808545E4: 5 in front.
            let pos = self.get_relative_position(self.actor.world_pos, Vec3::new(0.0, 0.0, 5.0));
            self.play_requests.push(PlayRequest::SpawnReviveFairy(pos));
            self.play_sfx(NA_SE_EV_FIATY_HEAL - SFX_FLAG);
            self.play_requests.push(PlayRequest::OnePointCutscene { cs_id: 9908, timer: 125, player: true, parent: CAM_ID_MAIN });
        } else if env.game_over_state == GAMEOVER_DEATH_WAIT_GROUND {
            self.play_requests.push(PlayRequest::GameOverState(GAMEOVER_DEATH_DELAY_MENU));
        }
    }

    /// `Player_Action_80843CEC`: dying on the ground. Without the Goron Tunic, a hot room, a
    /// void floor or a hot floor set Link burning again. At the animation's end, `func_80843AE8`;
    /// before it, the fall's sounds (`D_808545F0`, or the shock's bound at frame 88).
    fn action_80843cec(&mut self, env: &Env) {
        if self.current_tunic != PLAYER_TUNIC_GORON {
            let hot_floor = self.s.floor_type.wrapping_sub(FLOOR_TYPE_2) <= FLOOR_TYPE_3 - FLOOR_TYPE_2;
            let floor_hurts = self.actor.floor_poly.is_some_and(|f| env.col.flag27(f));
            if env.room_behavior_type2 == ROOM_ENV_HOT || self.s.floor_type == FLOOR_TYPE_9 || (hot_floor && !floor_hurts) {
                self.func_8083821C(env);
            }
        }
        self.decelerate_to_zero();
        if self.skel.update(env.data) {
            if self.actor.category == ACTORCAT_PLAYER {
                self.func_80843AE8(env);
            }
            return;
        }
        if self.skel.animation == env.data.anim("link_derth_rebirth") {
            self.process_anim_sfx_list(env.audio.player_anim_sfx("D_808545F0"));
        } else if self.skel.animation == env.data.anim("link_normal_electric_shock_end") && self.skel.on_frame(88.0) {
            self.play_floor_sfx(NA_SE_PL_BOUND);
        }
    }

    /// `Player_Action_8084E368`: dying while swimming; `func_80843AE8` at the animation's end.
    fn action_8084e368(&mut self, env: &Env) {
        self.func_8084B000();
        if self.skel.update(env.data) {
            self.func_80843AE8(env);
        }
        let yaw = self.actor.shape_rot.y;
        self.linear_velocity = self.func_8084AEEC(self.linear_velocity, 0.0, yaw);
    }

    /// `func_8083EC18`: onto the wall in front if it's 79 tall and climbable: vines and
    /// climbable walls (`WALL_FLAG_3`) from where Link is, a ladder (`WALL_FLAG_1`) lined up to
    /// its rungs, or a ladder's top (`WALL_FLAG_2`) turning round to climb down.
    fn func_8083EC18(&mut self, env: &Env, arg2: u32) -> bool {
        let data = env.data;
        let col = env.col;
        if self.wall_height < 79.0 {
            return false;
        }
        // Kokiri boots.
        if !(self.state1 & STATE1_27 == 0 || self.actor.y_dist_to_water < self.age.unk_2C) {
            return false;
        }
        let Some(wall) = self.actor.wall_poly else { return false };
        let sp8c: i8 = if arg2 & WALL_FLAG_3 != 0 { 2 } else { 0 };
        if !(sp8c != 0 || arg2 & WALL_FLAG_1 != 0 || col.wall_flags(wall) & WALL_FLAG_2 != 0) {
            return false;
        }
        let (mut phi_f20, mut phi_f12) = (0.0f32, 0.0f32);
        let (sp80, sp7c);
        if sp8c != 0 {
            sp80 = self.actor.world_pos.x;
            sp7c = self.actor.world_pos.z;
        } else {
            // The wall triangle's horizontal middle, and its lowest point.
            let v = col.poly_vertices(wall);
            let (mut x0, mut x1, mut z0, mut z1) = (v[0].x, v[0].x, v[0].z, v[0].z);
            phi_f20 = v[0].y;
            for p in &v[1..] {
                if x0 > p.x {
                    x0 = p.x;
                } else if x1 < p.x {
                    x1 = p.x;
                }
                if z0 > p.z {
                    z0 = p.z;
                } else if z1 < p.z {
                    z1 = p.z;
                }
                if phi_f20 > p.y {
                    phi_f20 = p.y;
                }
            }
            sp80 = (x0 + x1) * 0.5;
            sp7c = (z0 + z1) * 0.5;
            let n = col.poly_normal(wall);
            phi_f12 = ((self.actor.world_pos.x - sp80) * n.z) - ((self.actor.world_pos.z - sp7c) * n.x);
            let sp48 = self.actor.world_pos.y - phi_f20;
            // The height to the next rung, 15 apart, in double precision as written.
            const RUNG: f64 = 15.000000223517418;
            phi_f20 = ((((sp48 as f64 / RUNG) + 0.5) as i32 as f32) as f64 * RUNG - sp48 as f64) as f32;
            phi_f12 = phi_f12.abs();
        }
        if phi_f12 >= 8.0 {
            return false;
        }
        let n = col.poly_normal(wall);
        let mut sp34 = self.wall_distance;
        self.setup_wait_for_put_away(data, env, A74::ClimbStart);
        self.state1 |= STATE1_21;
        self.state1 &= !STATE1_27;
        let anim;
        if sp8c != 0 || arg2 & WALL_FLAG_1 != 0 {
            self.action_var1 = sp8c;
            if sp8c != 0 {
                anim = if self.grounded() { data.anim("link_normal_Fclimb_startA") } else { data.anim("link_normal_Fclimb_hold2upL") };
                sp34 = (self.age.wall_radius - 1.0) - sp34;
            } else {
                anim = self.age.climb.unk_A4;
                sp34 -= 1.0;
            }
            self.action_var2 = -2;
            self.actor.world_pos.y += phi_f20;
            self.current_yaw = self.actor.wall_yaw.wrapping_add(i16::MIN);
            self.actor.shape_rot.y = self.current_yaw;
        } else {
            anim = self.age.climb.unk_A8;
            self.action_var2 = -4;
            self.current_yaw = self.actor.wall_yaw;
            self.actor.shape_rot.y = self.current_yaw;
        }
        self.actor.world_pos.x = (sp34 * n.x) + sp80;
        self.actor.world_pos.z = (sp34 * n.z) + sp7c;
        self.func_80832224();
        self.actor.prev_pos = self.actor.world_pos;
        self.skel.play_once(data, anim);
        self.start_anim_movement(0x9F);
        true
    }

    /// `func_8083F360`: keep to the wall: a line from `arg4` to `arg3` ahead at height `arg1`;
    /// on the wall, face it and stand `arg2` off it. True while there's a wall.
    fn func_8083F360(&mut self, env: &Env, arg1: f32, arg2: f32, arg3: f32, arg4: f32) -> bool {
        let (c, s) = (cos_s(self.actor.shape_rot.y), sin_s(self.actor.shape_rot.y));
        let y = self.actor.world_pos.y + arg1;
        let a = Vec3::new(self.actor.world_pos.x + arg4 * s, y, self.actor.world_pos.z + arg4 * c);
        let b = Vec3::new(self.actor.world_pos.x + arg3 * s, y, self.actor.world_pos.z + arg3 * c);
        let hit = env.col.entity_line_test(a, b, true, false, false, true);
        self.actor.wall_poly = hit.map(|h| h.1);
        if let Some((p, poly)) = hit {
            self.actor.bg_check_flags |= BGCHECKFLAG_PLAYER_WALL_INTERACT;
            self.s.wall_flags = env.col.wall_flags(poly);
            let n = env.col.poly_normal(poly);
            let t = atan2_s(-n.z, -n.x);
            scaled_step_to_s(&mut self.actor.shape_rot.y, t, 800);
            self.current_yaw = self.actor.shape_rot.y;
            self.actor.world_pos.x = p.x - sin_s(self.actor.shape_rot.y) * arg2;
            self.actor.world_pos.z = p.z - cos_s(self.actor.shape_rot.y) * arg2;
            return true;
        }
        self.actor.bg_check_flags &= !BGCHECKFLAG_PLAYER_WALL_INTERACT;
        false
    }

    /// `func_8083FBC0`: let go on A, or when the wall isn't climbable any more.
    fn func_8083FBC0(&mut self, env: &Env) -> bool {
        let f = self.s.wall_flags;
        let flag2 = self.actor.wall_poly.is_some_and(|w| env.col.wall_flags(w) & WALL_FLAG_2 != 0);
        if !self.input.press.held(BTN_A) && self.actor.bg_check_flags & BGCHECKFLAG_PLAYER_WALL_INTERACT != 0 && (f & WALL_FLAG_3 != 0 || f & WALL_FLAG_1 != 0 || flag2) {
            return false;
        }
        self.func_8083FB7C(env.data);
        self.play_voice_sfx(NA_SE_VO_LI_AUTO_JUMP);
        true
    }

    /// `func_8083FB7C`: off the wall, falling.
    fn func_8083FB7C(&mut self, data: &GameData) {
        self.state1 &= !(STATE1_21 | STATE1_27);
        self.func_80837B9C(data);
        self.linear_velocity = -0.4;
    }

    /// `func_8083973C`: the floor below a point `off` ahead of Link (`Player_GetRelativePosition`,
    /// `BgCheck_EntityRaycastDown3`).
    fn func_8083973C(&self, env: &Env, off: Vec3) -> f32 {
        let p = self.get_relative_position(self.actor.world_pos, off);
        env.col.entity_raycast_down(p).0
    }

    /// `func_8083F070`: step off the ladder (`Player_Action_8084C5F8`).
    fn func_8083F070(&mut self, data: &GameData, anim: AnimId) {
        self.setup_action_preserve_anim_movement(data, Action::ClimbEnd, 0);
        self.skel.play_once_set_speed(data, anim, 4.0 / 3.0);
    }

    /// `LinkAnimation_Change(anim, -1, lastFrame, 0, ANIMMODE_ONCE, 0)`: an animation played
    /// backwards (climbing down).
    fn play_backwards(&mut self, data: &GameData, anim: AnimId) {
        let last = data.anims[anim].last_frame();
        self.skel.change(data, anim, -1.0, last, 0.0, ANIMMODE_ONCE, 0.0);
    }

    /// `Player_Action_8084BF1C`: climbing. Up and down one rung per animation (sideways on vines), at a
    /// speed from the stick; off the top onto the ledge (vines) or the ladder's top step, off
    /// the bottom near the floor.
    fn action_8084bf1c(&mut self, env: &Env) {
        let data = env.data;
        let mut sp84 = self.input.rel.stick_y as i32;
        let mut sp80 = self.input.rel.stick_x as i32;
        self.fall_start_height = self.actor.world_pos.y as i32 as i16;
        self.state2 |= STATE2_6;
        let mut phi_f0 = if self.action_var1 != 0 && sp84.abs() < sp80.abs() {
            sp84 = 0;
            sp80.abs() as f32 * 0.0325
        } else {
            sp80 = 0;
            sp84.abs() as f32 * 0.05
        };
        phi_f0 = phi_f0.clamp(1.0, 3.35);
        let phi_f2 = if self.skel.play_speed >= 0.0 { 1.0 } else { -1.0 };
        self.skel.play_speed = phi_f2 * phi_f0;
        if self.action_var2 >= 0 {
            // (A DynaPoly wall carries Link along: no climbable dyna walls are ported.)
            self.actor.update_bg_check_info(env.col, 26.0, 6.0, self.age.ceiling_check_height, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_1 | UPDBGCHECKINFO_FLAG_2);
            let r = self.age.unk_3C;
            self.func_8083F360(env, 26.0, r, 50.0, -20.0);
        }
        if (self.action_var2 < 0 || !self.func_8083FBC0(env)) && self.skel.update(data) {
            if self.action_var2 < 0 {
                self.action_var2 = (self.action_var2 as i32).abs() as i16 & 1;
                return;
            }
            let c = self.age.climb;
            if sp84 != 0 {
                let mut sp68 = (self.action_var1 as i32 + self.action_var2 as i32) as usize;
                if sp84 > 0 {
                    // D_8085488C = { 0, unk_40, 26 }: the ledge above.
                    let temp_f0 = self.func_8083973C(env, Vec3::new(0.0, self.age.unk_40, 26.0));
                    if self.actor.world_pos.y < temp_f0 {
                        if self.action_var1 != 0 {
                            self.actor.world_pos.y = temp_f0;
                            self.state1 &= !STATE1_21;
                            if let Some(w) = self.actor.wall_poly {
                                let r = self.age.unk_3C;
                                self.func_8083A5C4(data, env, w, r, data.anim("link_normal_jump_climb_up_free"));
                            }
                            self.current_yaw = self.current_yaw.wrapping_add(i16::MIN);
                            self.actor.shape_rot.y = self.current_yaw;
                            self.func_8083A9B8(data, data.anim("link_normal_jump_climb_up_free"));
                            self.state1 |= STATE1_14;
                        } else {
                            let a = c.unk_CC[self.action_var2 as usize & 1];
                            self.func_8083F070(data, a);
                        }
                    } else {
                        self.skel.prev_transl = c.unk_4A[sp68.min(3)];
                        self.skel.play_once(data, c.unk_AC[sp68.min(3)]);
                    }
                } else if (self.actor.world_pos.y - self.actor.floor_height) < 15.0 {
                    if self.action_var1 != 0 {
                        self.func_8083FB7C(data);
                    } else {
                        if self.action_var2 != 0 {
                            self.skel.prev_transl = c.unk_44;
                        }
                        let a = c.unk_C4[self.action_var2 as usize & 1];
                        self.func_8083F070(data, a);
                        self.action_var2 = 1;
                    }
                } else {
                    sp68 ^= 1;
                    self.skel.prev_transl = c.unk_62[sp68.min(3)];
                    let a1 = c.unk_AC[sp68.min(3)];
                    self.play_backwards(data, a1);
                }
                self.action_var2 ^= 1;
            } else if self.action_var1 != 0 && sp80 != 0 {
                let a2 = c.unk_BC[self.action_var2 as usize & 1];
                if sp80 > 0 {
                    self.skel.prev_transl = c.unk_7A[self.action_var2 as usize & 1];
                    self.skel.play_once(data, a2);
                } else {
                    self.skel.prev_transl = c.unk_86[self.action_var2 as usize & 1];
                    self.play_backwards(data, a2);
                }
            } else {
                self.state2 |= STATE2_12;
            }
            return;
        }
        if self.action_var2 < 0 {
            let f = |p: &Self, frames: &[f32]| frames.iter().any(|&fr| p.skel.on_frame(fr));
            if (self.action_var2 == -2 && f(self, &[14.0, 29.0])) || (self.action_var2 == -4 && f(self, &[22.0, 35.0, 49.0, 55.0])) {
                self.func_8084BEE4();
            }
            return;
        }
        if self.skel.on_frame(if self.skel.play_speed > 0.0 { 20.0 } else { 0.0 }) {
            self.func_8084BEE4();
        }
    }

    /// `func_8084BEE4`: a hand or foot on the ladder, or on the wall.
    fn func_8084BEE4(&mut self) {
        self.play_sfx(if self.action_var1 != 0 { NA_SE_PL_WALK_WALL } else { NA_SE_PL_WALK_LADDER });
    }

    /// `Player_Action_8084C5F8`: the step off a ladder, standing at its end.
    fn action_8084c5f8(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_6;
        let temp = self.try_action_interrupt(env, 4.0);
        if temp == 0 {
            self.state1 &= !STATE1_21;
            return;
        }
        if temp > 0 || self.skel.update(data) {
            self.func_8083C0E8(data);
            self.state1 &= !STATE1_21;
            return;
        }
        // D_80854898, D_808548A0: the steps' frames.
        let mut sp38 = [10.0, 20.0];
        if self.action_var2 != 0 {
            self.process_anim_sfx_list(env.audio.player_anim_sfx("D_808548A8"));
            sp38 = [40.0, 50.0];
        }
        if self.skel.on_frame(sp38[0]) || self.skel.on_frame(sp38[1]) {
            let p = self.actor.world_pos + Vec3::new(0.0, 20.0, 0.0);
            let (h, poly) = env.col.entity_raycast_down(p);
            if h != 0.0 {
                // SurfaceType_GetMaterial: the floor's type, not its sound's id.
                self.floor_sfx_offset = poly.map(|g| env.col.sfx_type(g) as u16).unwrap_or(0);
                self.play_landing_sfx();
            }
        }
    }

    /// `Player_Action_80845668`: stepping up onto a ledge (100/150 step-up animations, the model
    /// offset `shape.yOffset` easing back to 0), or the tall-ledge jump that ends in a mid-air grab.
    fn action_80845668(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5;
        let sp3c = self.skel.update(data);
        if self.skel.animation == data.anim("link_normal_250jump_start") {
            self.linear_velocity = 1.0;
            if self.skel.on_frame(8.0) {
                let mut t = self.wall_height.min(self.age.wall_probe_height);
                t *= if self.state1 & STATE1_27 != 0 { 0.085 } else { 0.072 };
                if !self.adult {
                    t += 1.0;
                }
                self.func_80838940(data, None, t, NA_SE_VO_LI_AUTO_JUMP);
                self.action_var2 = -1;
            }
        } else {
            let temp2 = self.try_action_interrupt(env, 4.0);
            if temp2 == 0 {
                self.state1 &= !(STATE1_14 | STATE1_18);
                return;
            }
            if sp3c || temp2 > 0 {
                self.func_8083C0E8(data);
                self.state1 &= !(STATE1_14 | STATE1_18);
                return;
            }
            let a = self.skel.animation;
            let temp3 = if a == data.anim("link_swimer_swim_15step_up") {
                if self.skel.on_frame(30.0) {
                    self.func_8083D0A8(env, 10.0);
                }
                50.0
            } else if a == data.anim("link_normal_150step_up") {
                30.0
            } else if a == data.anim("link_normal_100step_up") {
                16.0
            } else {
                0.0
            };
            if self.skel.on_frame(temp3) {
                self.play_landing_sfx();
                self.play_voice_sfx(NA_SE_VO_LI_CLIMB_END);
            }
            if a == data.anim("link_normal_100step_up") || self.skel.cur_frame > 5.0 {
                if self.action_var2 == 0 {
                    self.play_jumping_sfx();
                    self.action_var2 = 1;
                }
                step_to_f(&mut self.actor.shape_y_offset, 0.0, 150.0);
            }
        }
    }

    /// `func_8083A4A8`: the automatic jump off a ledge.
    fn func_8083A4A8(&mut self, data: &GameData) {
        let yaw_diff = self.current_yaw.wrapping_sub(self.actor.shape_rot.y);
        let anim = if abs16(yaw_diff) < 0x1000 && self.linear_velocity > 4.0 {
            data.anim("link_normal_run_jump")
        } else {
            data.anim("link_normal_jump")
        };
        let r = &self.regs;
        let vy = if self.linear_velocity > r.ireg(66) as f32 / 100.0 {
            r.ireg(67) as f32 / 100.0
        } else {
            (r.ireg(68) as f32 / 100.0) + ((r.ireg(69) as f32 * self.linear_velocity) / 1000.0)
        };
        self.func_80838940(data, Some(anim), vy, NA_SE_VO_LI_AUTO_JUMP);
        self.action_var2 = 1;
    }

    /// `func_80838940`: start a jump with vertical speed `vy`: the jump's sound and Link's
    /// voice `sfx_id`.
    fn func_80838940(&mut self, data: &GameData, anim: Option<AnimId>, vy: f32, sfx_id: u16) {
        self.setup_action(data, Action::Midair, 1);
        if let Some(a) = anim {
            self.skel.play_once_set_speed(data, a, 2.0 / 3.0);
        }
        self.actor.velocity.y = vy * self.s.speed_scale;
        self.hover_boots_timer = 0;
        self.actor.bg_check_flags &= !BGCHECKFLAG_GROUND;
        self.play_jumping_sfx();
        self.play_voice_sfx(sfx_id);
        self.state1 |= STATE1_18;
    }

    // ================================================================================
    // Action setup and shared helpers

    /// `Player_SetupAction`: switch action function. A change of item (the shield held, `itemAction`
    /// -1) goes back to the held item's models (`func_8008EC70`) unless `flags` 1 keeps the
    /// shield; without `flags` 1 the upper body goes back to the held item's action
    /// (`func_80834644`) and the shield down.
    fn setup_action(&mut self, data: &GameData, action: Action, flags: i32) -> bool {
        if self.action == action {
            return false;
        }
        self.action = action;
        if self.item_ap != self.held_item_ap && (flags & 1 == 0 || self.state1 & STATE1_22 == 0) {
            self.func_8008ec70(data);
        }
        if flags & 1 == 0 && self.state1 & STATE1_11 == 0 {
            self.func_80834644(data);
            self.state1 &= !STATE1_22;
        }
        self.finish_anim_movement();
        self.state1 &= !(STATE1_2 | STATE1_6 | STATE1_26 | STATE1_28 | STATE1_29 | STATE1_31);
        self.state2 &= !(STATE2_19 | STATE2_27 | STATE2_28);
        self.state3 &= !(STATE3_1 | STATE3_3 | STATE3_7);
        self.action_var1 = 0;
        self.action_var2 = 0;
        self.idle_type = 0;
        self.func_808326F0();
        true
    }

    /// `func_80834644`: the upper body back to the held item's action (an item change in progress
    /// finished: `Player_FinishItemChange`), nothing held up (`Player_DetachHeldActor`), no change
    /// pending.
    fn func_80834644(&mut self, data: &GameData) {
        if self.upper == UpperAction::Change {
            self.finish_item_change(data);
        }
        let f = self.upper_for(data, self.held_item_ap);
        self.set_upper_action_func(f);
        self.unk_834 = 0;
        self.idle_type = 0;
        self.detach_held_actor(data);
        self.state1 &= !STATE1_8;
    }

    /// `Player_IsChildWithHylianShield`.
    fn is_child_with_hylian_shield(&self) -> bool {
        !self.adult && self.current_shield == PLAYER_SHIELD_HYLIAN
    }

    /// `Player_HoldsTwoHandedWeapon`: the Biggoron's Sword through the hammer.
    fn holds_two_handed_weapon(&self, data: &GameData) -> bool {
        let it = &data.items;
        (it.ap("SWORD_BIGGORON")..=it.ap("HAMMER")).contains(&self.held_item_ap)
    }

    /// `Player_CheckHostileLockOn`.
    fn check_hostile_lock_on(&self) -> bool {
        self.state1 & STATE1_4 != 0
    }

    /// `Player_SetModelsForHoldingShield`: guarding with nothing else in hand (or the item in
    /// hand being the one held), the shield goes into the right hand (`PLAYER_MODELTYPE_RH_SHIELD`,
    /// the sheath without it: `SHEATH_18` to `_16`, `_19` to `_17`), with the guard's animation
    /// type (`PLAYER_ANIMTYPE_2`) and no item action (-1).
    fn set_models_for_holding_shield(&mut self, data: &GameData) {
        if self.state1 & STATE1_22 != 0 && (self.item_ap < 0 || self.item_ap == self.held_item_ap) && !self.holds_two_handed_weapon(data) && !self.is_child_with_hylian_shield() {
            self.holding_shield = true;
            self.model_anim_type = 2;
            self.item_ap = -1;
        }
    }

    /// `func_8008EC70`: back to the held item's action and models.
    fn func_8008ec70(&mut self, data: &GameData) {
        self.item_ap = self.held_item_ap;
        let g = self.action_to_model_group(data, self.held_item_ap);
        self.player_set_model_group(data, g);
        self.unk_6AD = 0;
    }

    /// `func_808346C4`: the shield's upper-body action (`func_80834B5C`); the animation raising it
    /// from the stance's lead foot (`unk_870`: `D_808543A4`, `D_808543AC`).
    fn func_808346c4(&mut self, data: &GameData) -> AnimId {
        self.set_upper_action_func(UpperAction::ShieldUp);
        self.detach_held_actor(data);
        let two = self.holds_two_handed_weapon(data);
        let name = match (self.unk_870 < 0.5, two) {
            (true, false) => "link_anchor_waitR2defense",
            (true, true) => "link_anchor_waitR2defense_long",
            (false, false) => "link_anchor_waitL2defense",
            (false, true) => "link_anchor_waitL2defense_long",
        };
        data.anim(name)
    }

    /// `func_80834758`: R while Z-targeting raises the shield on the upper body (not riding, not
    /// in a cutscene's hold, nothing being changed, a shield worn and not the child's Hylian
    /// Shield): its animation at its last frame, `NA_SE_IT_SHIELD_POSTURE`.
    fn func_80834758(&mut self, env: &Env) -> bool {
        let data = env.data;
        if self.state1 & (STATE1_22 | STATE1_23 | STATE1_29) == 0
            && self.held_item_ap == self.item_ap
            && self.current_shield != 0
            && !self.is_child_with_hylian_shield()
            && self.is_z_targeting()
            && self.input.cur.held(eng_input::pad::BTN_R)
        {
            let anim = self.func_808346c4(data);
            let frame = data.anims[anim].last_frame();
            self.skel2.change(data, anim, 1.0, frame, frame, ANIMMODE_ONCE, 0.0);
            self.play_sfx(NA_SE_IT_SHIELD_POSTURE);
            return true;
        }
        false
    }

    /// `func_80834894`: lowering the shield: the raise played backwards (`func_80834C74`), the
    /// held item's models back, `NA_SE_IT_SHIELD_REMOVE`.
    fn func_80834894(&mut self, data: &GameData) {
        self.set_upper_action_func(UpperAction::ShieldDown);
        if self.item_ap < 0 {
            self.func_8008ec70(data);
        }
        self.skel2.reverse();
        self.play_sfx(NA_SE_IT_SHIELD_REMOVE);
    }

    /// `func_80834B5C`: the shield up while R is held, else coming down.
    fn func_80834b5c(&mut self, env: &Env) -> bool {
        let data = env.data;
        self.skel2.update(data);
        if !self.input.cur.held(eng_input::pad::BTN_R) {
            self.func_80834894(data);
        } else {
            self.state1 |= STATE1_22;
            self.set_models_for_holding_shield(data);
        }
        true
    }

    /// `func_80834BD4`: the hit's recoil on the upper body; once played, the shield up again.
    fn func_80834bd4(&mut self, env: &Env) -> bool {
        let data = env.data;
        if self.skel2.update(data) {
            let anim = self.func_808346c4(data);
            let frame = data.anims[anim].last_frame();
            self.skel2.change(data, anim, 1.0, frame, frame, ANIMMODE_ONCE, 0.0);
        }
        self.state1 |= STATE1_22;
        self.set_models_for_holding_shield(data);
        true
    }

    /// `func_80834C74`: the shield coming down; once down (or with B pressed), the held item's
    /// upper-body action again, on the wait loop.
    fn func_80834c74(&mut self, env: &Env) -> bool {
        let data = env.data;
        self.s.use_held_item = self.s.held_item_button_is_held_down;
        if self.s.use_held_item || self.skel2.update(data) {
            let f = self.upper_for(data, self.held_item_ap);
            self.set_upper_action_func(f);
            let a = self.anim(data, group::WAIT);
            self.skel2.play_loop(data, a);
            self.idle_type = 0;
            self.run_upper(env);
            return false;
        }
        true
    }

    /// `Player_ActionHandler_11`: R guards (a shield worn; the child's Hylian Shield always,
    /// another only when not Z-targeting): the guard's first frame (its animation at the end),
    /// `NA_SE_IT_SHIELD_POSTURE`; the lock-on weight `unk_86C` set as it starts.
    fn action_handler_11(&mut self, env: &Env) -> bool {
        let data = env.data;
        if self.current_shield != 0
            && self.input.cur.held(eng_input::pad::BTN_R)
            && (self.is_child_with_hylian_shield() || (!self.friendly_lock_on_or_parallel() && self.focus_actor.is_none()))
        {
            self.func_80832318();
            self.detach_held_actor(data);
            if self.setup_action(data, Action::Guard, 0) {
                self.state1 |= STATE1_22;
                let anim = if !self.is_child_with_hylian_shield() {
                    self.set_models_for_holding_shield(data);
                    self.anim(data, group::DEFENSE)
                } else {
                    data.anim("clink_normal_defense_ALL")
                };
                if anim != self.skel.animation {
                    if self.check_hostile_lock_on() {
                        self.unk_86c = 1.0;
                    } else {
                        self.unk_86c = 0.0;
                        self.func_80833C3C();
                    }
                    self.upper_limb_rot_x = 0;
                    self.upper_limb_rot_y = 0;
                    self.upper_limb_rot_z = 0;
                }
                let frame = data.anims[anim].last_frame();
                self.skel.change(data, anim, 1.0, frame, frame, ANIMMODE_ONCE, 0.0);
                if self.is_child_with_hylian_shield() {
                    // ANIM_FLAG_DISABLE_CHILD_ROOT_ADJUSTMENT.
                    self.start_anim_movement(4);
                }
                self.play_sfx(NA_SE_IT_SHIELD_POSTURE);
            }
            return true;
        }
        false
    }

    /// `Player_Action_80843188`: guarding. Once the guard's animation is up, the defence's wait
    /// loops; the stick tilts the shield (the focus's pitch up to 3500, the upper body's yaw),
    /// relative to the camera. B stabs from behind it (`func_808428D8`), the stab's sword active
    /// until its frame 2 with the walls' recoil (`func_80842DF4`). Climbing, talking or picking
    /// up interrupts it (`func_80842964`); letting go of R ends it: the guard's end, the held
    /// item's models back, `NA_SE_IT_SHIELD_REMOVE`.
    fn action_80843188(&mut self, env: &Env) {
        let data = env.data;
        if self.skel.update(data) {
            if !self.is_child_with_hylian_shield() {
                let a = self.anim(data, group::DEFENSE_WAIT);
                self.skel.play_loop(data, a);
            }
            self.action_var2 = 1;
            self.action_var1 = 0;
        }
        if !self.is_child_with_hylian_shield() {
            self.state1 |= STATE1_22;
            self.update_upper_body(env);
            self.state1 &= !STATE1_22;
        }
        self.decelerate_to_zero();
        if self.action_var2 != 0 {
            let sp54 = self.input.rel.stick_y as f32 * 100.0;
            let sp50 = self.input.rel.stick_x as f32 * -120.0;
            let sp4e = self.actor.shape_rot.y.wrapping_sub(env.cam_input_yaw);
            let sp40 = cos_s(sp4e);
            let mut sp4c = ((sin_s(sp4e) * sp50) + (sp54 * sp40)) as i32 as i16;
            let sp40 = cos_s(sp4e);
            let sp4a = ((sp50 * sp40) - (sin_s(sp4e) * sp54)) as i32 as i16;
            if sp4c > 3500 {
                sp4c = 3500;
            }
            let mut sp48 = ((sp4c as i32 - self.actor.focus_rot.x as i32).abs() as f32 * 0.25) as i32 as i16;
            if sp48 < 100 {
                sp48 = 100;
            }
            let mut sp46 = ((sp4a as i32 - self.upper_limb_rot_y as i32).abs() as f32 * 0.25) as i32 as i16;
            if sp46 < 50 {
                sp46 = 50;
            }
            scaled_step_to_s(&mut self.actor.focus_rot.x, sp4c, sp48);
            self.upper_limb_rot_x = self.actor.focus_rot.x;
            scaled_step_to_s(&mut self.upper_limb_rot_y, sp4a, sp46);
            if self.action_var1 != 0 {
                if !self.func_80842df4(env) {
                    if self.skel.cur_frame < 2.0 {
                        self.func_80833A20(env, 1);
                    }
                } else {
                    self.action_var2 = 1;
                    self.action_var1 = 0;
                }
            } else if !self.func_80842964(env) {
                if self.action_handler_11(env) {
                    self.func_808428d8(env);
                } else {
                    self.state1 &= !STATE1_22;
                    self.func_80832318();
                    if self.is_child_with_hylian_shield() {
                        self.func_8083A060(data);
                        let a = data.anim("clink_normal_defense_ALL");
                        let last = data.anims[a].last_frame();
                        self.skel.change(data, a, 1.0, last, 0.0, ANIMMODE_ONCE, 0.0);
                        self.start_anim_movement(4);
                    } else {
                        if self.item_ap < 0 {
                            self.func_8008ec70(data);
                        }
                        let a = self.anim(data, group::DEFENSE_END);
                        self.func_8083A098(data, a);
                    }
                    self.play_sfx(NA_SE_IT_SHIELD_REMOVE);
                    return;
                }
            } else {
                return;
            }
        }
        self.state1 |= STATE1_22;
        self.set_models_for_holding_shield(data);
        // UNK6AE_ROT_FOCUS_X | UNK6AE_ROT_UPPER_X | UNK6AE_ROT_UPPER_Y.
        self.unk_6AE_rot_flags |= 0x01 | 0x40 | 0x80;
    }

    /// `func_80842964`: climbing on (`Player_ActionHandler_13`), talking or picking up
    /// (`Player_ActionHandler_2`) interrupts the guard.
    fn func_80842964(&mut self, env: &Env) -> bool {
        self.action_handler_13(env) || self.action_handler_talk(env) || self.action_handler_2(env)
    }

    /// `func_808428D8`: B with a sword stabs from the guard (`link_normal_defense_kiru`,
    /// `PLAYER_MWA_STAB_1H`), towards where the upper body aims.
    fn func_808428d8(&mut self, env: &Env) -> bool {
        let data = env.data;
        if !self.is_child_with_hylian_shield() && Self::melee_weapon(self.held_item_ap) != 0 && self.s.use_held_item {
            let a = data.anim("link_normal_defense_kiru");
            self.skel.play_once(data, a);
            self.action_var1 = 1;
            self.melee_weapon_animation = data.items.mwa("STAB_1H");
            self.current_yaw = self.actor.shape_rot.y.wrapping_add(self.upper_limb_rot_y);
            return true;
        }
        false
    }

    /// `Player_Action_808435C4`: pushed back by a hit on the shield. Outside the guard, the
    /// upper body's recoil (`func_80834BD4`) plays to the shield up again, or to an interrupt,
    /// then the targeting stance; in the guard, the body's recoil, then the guard again at its
    /// end.
    fn action_808435c4(&mut self, env: &Env) {
        let data = env.data;
        self.decelerate_to_zero();
        if self.action_var1 == 0 {
            self.s.item_action = self.update_upper_body(env) as i32;
            if self.upper == UpperAction::ShieldUp || self.try_action_interrupt_upper(env, 4.0) >= 1 {
                self.setup_action(data, Action::TargetIdle, 1);
            }
        } else {
            let r = self.try_action_interrupt(env, 4.0);
            if r != 0 && (r >= 1 || self.skel.update(data)) {
                self.setup_action(data, Action::Guard, 1);
                self.state1 |= STATE1_22;
                self.set_models_for_holding_shield(data);
                let anim = self.anim(data, group::DEFENSE);
                let frames = data.anims[anim].last_frame();
                self.skel.change(data, anim, 1.0, frames, frames, ANIMMODE_ONCE, 0.0);
            }
        }
    }

    /// `Player_TryActionInterrupt` on the upper body's animation (`upperSkelAnime`).
    fn try_action_interrupt_upper(&mut self, env: &Env, arg3: f32) -> i32 {
        if (self.skel2.end_frame - arg3) <= self.skel2.cur_frame {
            if self.try_action_handler_list(env, Self::S_ACTION_HANDLER_LIST_IDLE, true) {
                return 0;
            }
            if self.get_movement_speed_and_yaw(env, 0.018).0 {
                return 1;
            }
        }
        -1
    }

    /// `func_80842D20`: the sword's rebound (`Player_Action_808505DC`; not out of the guard):
    /// `D_808545CC`'s rebound by the weapon's length and the lock-on; pushed back at 18.
    fn func_80842d20(&mut self, env: &Env) {
        let data = env.data;
        if self.action != Action::Guard {
            self.func_80832440();
            self.setup_action(data, Action::Rebound, 0);
            let sp28 = if self.check_hostile_lock_on() { 2 } else { 0 };
            let names = ["link_fighter_rebound", "link_fighter_rebound_long", "link_fighter_reboundR", "link_fighter_rebound_longR"];
            let a = data.anim(names[self.holds_two_handed_weapon(data) as usize + sp28]);
            // Player_AnimPlayOnceAdjusted.
            self.skel.play_once_set_speed(data, a, 2.0 / 3.0);
        }
        // (Player_RequestRumble: the rumble isn't ported.)
        self.linear_velocity = -18.0;
        self.func_80842cf0(env);
    }

    /// `func_80842A88`: one Deku Stick less, and what's left of it put away.
    fn func_80842a88(&mut self, env: &Env) {
        self.change_ammo(env, oot_game::item::ITEM_DEKU_STICK, -1);
        self.use_item(env.data, oot_game::item::ITEM_NONE);
    }

    /// `func_80842AC4`: a Deku Stick longer than half breaks on a hit: with sticks left, its far
    /// half flies off backwards from the right hand (`EffectSsStick_Spawn`), half of it stays
    /// (`unk_85C` 0.5) as it's put away, one stick less, `NA_SE_IT_WOODSTICK_BROKEN`. True for any
    /// such stick.
    fn func_80842ac4(&mut self, env: &Env) -> bool {
        if self.held_item_ap == env.data.items.ap("DEKU_STICK") && self.unk_85c > 0.5 {
            if env.io.borrow().save.ammo(oot_game::item::ITEM_DEKU_STICK) != 0 {
                let (pos, yaw) = (self.body_parts_pos[BODYPART_R_HAND], self.actor.shape_rot.y.wrapping_add(0x8000u16 as i16));
                env.io.borrow_mut().ss().stick_spawn(pos, yaw, self.adult);
                self.unk_85c = 0.5;
                self.func_80842a88(env);
                self.play_sfx(NA_SE_IT_WOODSTICK_BROKEN);
            }
            return true;
        }
        false
    }

    /// `func_80842B7C`: the Biggoron's Sword wears down a point a hit (`swordHealth`) unless it's
    /// the unbreakable one (`bgsFlag`); at 0 the blade flies off (`EffectSsStick_Spawn`), the
    /// knife's broken in the save (`func_800849EC`) and `NA_SE_IT_MAJIN_SWORD_BROKEN` sounds. True
    /// for the Biggoron's Sword.
    fn func_80842b7c(&mut self, env: &Env) -> bool {
        if self.held_item_ap != env.data.items.ap("SWORD_BIGGORON") {
            return false;
        }
        let broke = {
            let mut io = env.io.borrow_mut();
            if !io.save.bgs_flag && io.save.sword_health > 0 {
                io.save.sword_health -= 1;
                io.save.sword_health == 0
            } else {
                false
            }
        };
        if broke {
            let (pos, yaw) = (self.body_parts_pos[BODYPART_R_HAND], self.actor.shape_rot.y.wrapping_add(0x8000u16 as i16));
            let mut io = env.io.borrow_mut();
            io.ss().stick_spawn(pos, yaw, self.adult);
            oot_game::item::func_800849ec(&mut io.save);
            drop(io);
            self.play_sfx(NA_SE_IT_MAJIN_SWORD_BROKEN);
        }
        true
    }

    /// `func_80842CF0`: a stick breaks or the Biggoron's Sword wears (a wall struck, a rebound).
    fn func_80842cf0(&mut self, env: &Env) {
        self.func_80842ac4(env);
        self.func_80842b7c(env);
    }

    /// `func_80842DF4`: the swing's contact. A swing bounced off something hard (`AT_BOUNCED`)
    /// rebounds (`func_80842D20`) with the hit's freeze; from frame 2, the sword's tip 10 beyond
    /// the blade meeting a wall (`BgCheck_EntityLineTest1`) strikes sparks (wood's, or the shield
    /// particles with a soft or hard wall's sound) and pushes Link back at 14 (unless he's going
    /// backwards). A hit on an actor freezes play a frame (not a sign's), and an electric
    /// backlash shocks Link (half a heart). True when the swing was cut short.
    fn func_80842df4(&mut self, env: &Env) -> bool {
        let data = env.data;
        if self.melee_weapon_state <= 0 {
            return false;
        }
        let spin = data.items.mwa("SPIN_ATTACK_1H");
        let q = &self.melee_weapon_quads;
        if self.melee_weapon_animation < spin {
            if q[0].base.at_flags & cc::AT_BOUNCED == 0 && q[1].base.at_flags & cc::AT_BOUNCED == 0 {
                if self.skel.cur_frame >= 2.0 {
                    let tip = self.melee_weapon_info[0].tip;
                    let base_to_tip = self.melee_weapon_info[0].base - tip;
                    let mut phi = base_to_tip.length();
                    if phi != 0.0 {
                        phi = (phi + 10.0) / phi;
                    }
                    let sp68 = tip + base_to_tip * phi;
                    // (SurfaceType_IsIgnoredByEntities: the entity line test skips those polys already.)
                    if let Some((sp5c, poly)) = env.col.entity_line_test(sp68, tip, true, false, false, true)
                        && env.col.floor_type(poly) != FLOOR_TYPE_6
                        && !self.func_8002f9ec(env, poly, sp5c)
                    {
                        // (The hammer's quake: not held.)
                        if self.linear_velocity >= 0.0 {
                            let material = env.col.sfx_type(poly) as u16;
                            if material == SURFACE_MATERIAL_WOOD {
                                self.play_requests.push(PlayRequest::ShieldParticles { pos: sp5c, metal: false });
                                self.sfx(PlayerSfx::Actor(NA_SE_IT_REFLECTION_WOOD));
                            } else {
                                self.play_requests.push(PlayRequest::ShieldParticles { pos: sp5c, metal: true });
                                self.play_sfx(if material == SURFACE_MATERIAL_DIRT_SOFT { NA_SE_IT_WALL_HIT_SOFT } else { NA_SE_IT_WALL_HIT_HARD });
                            }
                            self.func_80842cf0(env);
                            self.linear_velocity = -14.0;
                        }
                    }
                }
            } else {
                self.func_80842d20(env);
                self.play_requests.push(PlayRequest::FreezeFlash);
                return true;
            }
        }
        let q = &self.melee_weapon_quads;
        let temp1 = q[0].base.at_flags & cc::AT_HIT != 0 || q[1].base.at_flags & cc::AT_HIT != 0;
        if temp1 {
            if self.melee_weapon_animation < spin {
                // @bug (game): meleeWeaponQuads[temp1 ? 1 : 0] with temp1 a boolean is always the
                // second quad, whichever one hit.
                let at = self.melee_weapon_quads[1].base.at;
                if at.and_then(|h| env.target(h)).is_some_and(|a| a.id != crate::en_kanban::ACTOR_EN_KANBAN) {
                    self.play_requests.push(PlayRequest::FreezeFlash);
                }
            }
            if !self.func_80842ac4(env) && self.held_item_ap != data.items.ap("HAMMER") {
                self.func_80842b7c(env);
                if self.actor.col_chk_info.at_hit_backlash == cc::HIT_BACKLASH_ELECTRIC {
                    self.actor.col_chk_info.damage = 8;
                    let yaw = self.actor.shape_rot.y;
                    self.func_80837C0C(env, PLAYER_HIT_RESPONSE_ELECTRIFIED, 0.0, 0.0, yaw, 20);
                    return true;
                }
            }
        }
        false
    }

    /// `func_8002F9EC` (`z_actor.c`): the sword on a `FLOOR_TYPE_8` surface (Jabu-Jabu's
    /// flesh): blue blood and `NA_SE_IT_WALL_HIT_BUYO`. (`roomCtx.drawParams[0]` isn't modelled.)
    fn func_8002f9ec(&mut self, env: &Env, poly: eng_collision::bgcheck::PolyId, pos: Vec3) -> bool {
        if env.col.floor_type(poly) == FLOOR_TYPE_8 {
            self.play_requests.push(PlayRequest::Blood { kind: cc::BLOOD_BLUE, pos });
            self.play_sfx(NA_SE_IT_WALL_HIT_BUYO);
            return true;
        }
        false
    }

    /// `Player_Action_808505DC`: the rebound; from frame 6, standing.
    fn action_808505dc(&mut self, env: &Env) {
        let data = env.data;
        self.skel.update(data);
        self.decelerate_to_zero();
        if self.skel.cur_frame >= 6.0 {
            self.func_80839FFC(data);
        }
    }

    /// `Player_ZeroSpeedXZ`.
    fn zero_speed_xz(&mut self) {
        self.actor.speed_xz = 0.0;
        self.linear_velocity = 0.0;
    }

    /// `Player_ApplyYawFromAnim`: fold the root limb's yaw into the facing.
    fn apply_yaw_from_anim(&mut self) {
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(self.skel.joint[1][1]);
        self.skel.joint[1][1] = 0;
    }

    /// `Player_ResetAnimMovement`.
    fn reset_anim_movement(&mut self) {
        self.skel.prev_transl = self.skel.base_transl;
        self.skel.prev_rot = self.actor.shape_rot.y;
    }

    /// `Player_FinishAnimMovement`: end animation-driven movement.
    fn finish_anim_movement(&mut self) {
        if self.skel.move_flags != 0 {
            self.apply_yaw_from_anim();
            self.skel.joint[0][0] = self.skel.base_transl[0];
            self.skel.joint[0][2] = self.skel.base_transl[2];
            if self.skel.move_flags & 8 != 0 {
                if self.skel.move_flags & 2 != 0 {
                    self.skel.joint[0][1] = self.skel.prev_transl[1];
                }
            } else {
                self.skel.joint[0][1] = self.skel.base_transl[1];
            }
            self.reset_anim_movement();
            self.skel.move_flags = 0;
        }
    }

    /// `func_80832440`: drop interaction state when an action is interrupted.
    fn func_80832440(&mut self) {
        self.melee_weapon_state = 0;
        self.unk_6AD = 0;
        self.state1 &= !(STATE1_13 | STATE1_14 | STATE1_20 | STATE1_21);
        self.state2 &= !(STATE2_4 | (1 << 7) | STATE2_18);
        self.actor.shape_rot.x = 0;
    }

    /// `func_80833C3C`.
    fn func_80833C3C(&mut self) {
        self.unk_870 = 0.0;
        self.unk_874 = 0.0;
    }

    /// `Player_DecelerateToZero`: decelerate by `REG(43) / 100`.
    fn decelerate_to_zero(&mut self) -> bool {
        step_to_f(&mut self.linear_velocity, 0.0, self.regs.reg(43) as f32 / 100.0)
    }

    /// `Player_CalcSpeedAndYawFromControlStick`: target speed and yaw from the stick. `arg4` selects the curve used
    /// when starting from rest (0.018) versus the linear one (0).
    fn calc_speed_and_yaw_from_control_stick(&self, arg4: f32) -> (bool, f32, i16) {
        if self.unk_6AD != 0 || self.state1 & 1 != 0 {
            return (false, 0.0, self.actor.shape_rot.y);
        }
        let mut speed = self.s.stick_mag;
        let yaw = self.s.stick_angle;
        if arg4 != 0.0 {
            speed -= 20.0;
            if speed < 0.0 {
                speed = 0.0;
            } else {
                let t = 1.0 - cos_s(f2s(speed * 450.0));
                speed = (t * t) * 30.0 + 7.0;
            }
        } else {
            speed *= 0.8;
        }
        if self.s.stick_mag != 0.0 {
            let slope = sin_s(self.floor_pitch).clamp(0.0, 0.6);
            let mut limit = self.unk_880;
            if self.unk_6C4 != 0.0 {
                limit = (limit - self.unk_6C4 * 0.008).max(2.0);
            }
            speed = speed * 0.14 - 8.0 * slope * slope;
            speed = speed.clamp(0.0, limit);
            return (true, speed, yaw);
        }
        (false, speed, yaw)
    }

    /// `Player_GetMovementSpeedAndYaw`: target speed and world yaw. Returns false (and a yaw that keeps the
    /// current facing) when the stick is idle.
    fn get_movement_speed_and_yaw(&self, env: &Env, arg3: f32) -> (bool, f32, i16) {
        let (moving, speed, yaw) = self.calc_speed_and_yaw_from_control_stick(arg3);
        if !moving {
            let mut yaw = self.actor.shape_rot.y;
            if let Some(t) = self.focus_actor {
                if env.target.reticle_locked && self.state2 & STATE2_6 == 0 {
                    yaw = oot_game::target::yaw_to(self.actor.world_pos, env.target(t).expect("the locked-on actor").focus_pos);
                }
            } else if self.friendly_lock_on_or_parallel() {
                yaw = self.target_yaw;
            }
            (false, speed, yaw)
        } else {
            (true, speed, yaw.wrapping_add(env.cam_input_yaw))
        }
    }

    /// `Player_TryActionHandlerList`: the per-action list of interrupts (`sActionHandlerFuncs`). Ported: 0, 1,
    /// 2, 4 (talking), 5, 6 (`Player_ActionHandler_Roll`, roll / put the sword away on A), 7
    /// (`Player_ActionHandler_7`, B attacks), 10 (`Player_ActionHandler_10`, hops while targeting), 11
    /// (`Player_ActionHandler_11`, the guard on R), 12 (`Player_ActionHandler_12`, climbing onto ledges) and 13;
    /// the others need other items and report "not taken".
    fn try_action_handler_list(&mut self, env: &Env, list: &[i8], arg3: bool) -> bool {
        if self.state1 & (1 | (1 << 7) | STATE1_29) != 0 {
            return false;
        }
        if arg3 {
            // sUpperBodyIsBusy = Player_UpdateUpperBody(): the upper-body item action.
            self.s.item_action = self.update_upper_body(env) as i32;
        }
        // No interrupts while an item change is pending or playing.
        if self.state1 & STATE1_8 != 0 || self.upper == UpperAction::Change {
            return false;
        }
        let mut i = 0;
        loop {
            let e = list[i];
            let idx = e.unsigned_abs() as usize;
            if idx == 0 && self.action_handler_0(env) {
                return true;
            }
            if idx == 1 && self.action_handler_1(env) {
                return true;
            }
            if idx == 2 && self.action_handler_2(env) {
                return true;
            }
            if idx == 11 && self.action_handler_11(env) {
                return true;
            }
            if idx == 4 && self.action_handler_talk(env) {
                return true;
            }
            if idx == 5 && self.action_handler_5(env) {
                return true;
            }
            if idx == 6 && self.action_handler_roll(env) {
                return true;
            }
            if idx == 12 && self.action_handler_12(env) {
                return true;
            }
            if idx == 10 && self.action_handler_10(env) {
                return true;
            }
            if idx == 7 && self.action_handler_7(env) {
                return true;
            }
            if idx == 13 && self.action_handler_13(env) {
                return true;
            }
            if e < 0 {
                break;
            }
            i += 1;
        }
        false
    }

    // Interrupt lists (`sActionHandlerList1` ...) as indices into sActionHandlerFuncs.
    const S_ACTION_HANDLER_LIST_IDLE: &'static [i8] = &[0, 11, 1, 2, 3, 5, 4, 9, 8, 7, -6];
    const S_ACTION_HANDLER_LIST8: &'static [i8] = &[0, 11, 1, 2, 3, 12, 5, 4, 9, 8, 7, -6];
    const S_ACTION_HANDLER_LIST_TURN_IN_PLACE: &'static [i8] = &[-7];
    const S_ACTION_HANDLER_LIST1: &'static [i8] = &[13, 2, 4, 9, 10, 11, 8, -7];
    const S_ACTION_HANDLER_LIST2: &'static [i8] = &[13, 1, 2, 5, 3, 4, 9, 10, 11, 7, 8, -6];
    const S_ACTION_HANDLER_LIST3: &'static [i8] = &[13, 1, 2, 3, 4, 9, 10, 11, 8, 7, -6];
    const S_ACTION_HANDLER_LIST4: &'static [i8] = &[13, 2, 4, 9, 10, 11, 8, -7];
    const S_ACTION_HANDLER_LIST5: &'static [i8] = &[13, 2, 4, 9, 10, 11, 12, 8, -7];
    const S_ACTION_HANDLER_LIST9: &'static [i8] = &[13, 1, 2, 3, 12, 5, 4, 9, 10, 11, 8, 7, -6];
    const S_ACTION_HANDLER_LIST10: &'static [i8] = &[10, 8, -7];
    const S_ACTION_HANDLER_LIST11: &'static [i8] = &[0, 12, 5, -4];

    /// `Player_ActionHandler_Roll`: A pressed → roll if the stick points forward.
    fn action_handler_roll(&mut self, env: &Env) -> bool {
        if !self.player_update_hostile_lock_on_env(env) && self.s.item_action == 0 && self.state1 & (1 << 23) == 0 && self.input.press.held(BTN_A) {
            if self.try_roll(env) {
                return true;
            }
            if self.put_away_cooldown_timer == 0 && self.held_item_ap >= env.data.items.ap("SWORD_MASTER") {
                self.use_item(env.data, oot_game::item::ITEM_NONE);
            } else {
                self.state2 ^= 1 << 20;
            }
        }
        false
    }

    /// `Player_TryRoll`.
    fn try_roll(&mut self, env: &Env) -> bool {
        if self.stick_dir() == 0 && self.s.floor_type != FLOOR_TYPE_7 {
            self.setup_roll(env.data);
            return true;
        }
        false
    }

    /// `Player_SetupRoll`: start a roll.
    fn setup_roll(&mut self, data: &GameData) {
        self.setup_action(data, Action::Roll, 0);
        let a = self.anim(data, group::ROLL);
        self.skel.play_once_set_speed(data, a, 1.25 * self.s.speed_scale);
    }

    /// `Player_UpdateHostileLockOn`: locked on to a hostile target (sets `PLAYER_STATE1_HOSTILE_LOCK_ON`).
    fn player_update_hostile_lock_on_env(&mut self, env: &Env) -> bool {
        if let Some(t) = self.focus_actor
            && env.target(t).is_some_and(|a| a.is_hostile())
        {
            self.state1 |= STATE1_4;
            return true;
        }
        self.update_hostile_lock_on()
    }

    /// The "not locked on" half of `Player_UpdateHostileLockOn` (clears `PLAYER_STATE1_HOSTILE_LOCK_ON`).
    fn update_hostile_lock_on(&mut self) -> bool {
        if self.state1 & STATE1_4 != 0 {
            self.state1 &= !STATE1_4;
            if self.linear_velocity == 0.0 {
                self.current_yaw = self.actor.shape_rot.y;
            }
        }
        false
    }

    /// `func_8083DF68`: accelerate towards `speed` (`REG(19)`, 1.5) and turn (`REG(27)`).
    fn func_8083DF68(&mut self, speed: f32, yaw: i16) {
        asym_step_to_f(&mut self.linear_velocity, speed, self.regs.reg(19) as f32 / 100.0, 1.5);
        scaled_step_to_s(&mut self.current_yaw, yaw, self.regs.reg(27));
    }

    /// `func_8083DFE0`: air control.
    fn func_8083DFE0(&mut self, speed: f32, yaw: i16) {
        let yaw_diff = self.current_yaw.wrapping_sub(yaw);
        if self.melee_weapon_state == 0 {
            let rsl = self.regs.run_speed_limit();
            self.linear_velocity = self.linear_velocity.clamp(-rsl, rsl);
        }
        if abs16(yaw_diff) > 0x6000 {
            if step_to_f(&mut self.linear_velocity, 0.0, 1.0) {
                self.current_yaw = yaw;
            }
        } else {
            asym_step_to_f(&mut self.linear_velocity, speed, 0.05, 0.1);
            scaled_step_to_s(&mut self.current_yaw, yaw, 200);
        }
    }

    /// `func_8083C484`: sharp reversal while running → brake first.
    fn func_8083C484(&mut self, speed: &mut f32, yaw: &mut i16) -> bool {
        let d = self.current_yaw.wrapping_sub(*yaw);
        if abs16(d) > 0x6000 {
            if self.decelerate_to_zero() {
                *speed = 0.0;
                *yaw = self.current_yaw;
            } else {
                return true;
            }
        }
        false
    }

    // ================================================================================
    // Transitions

    /// `func_80853080`: stand still, playing the wait animation.
    /// Stands Player still where it is, out of any action (a test or sandbox placing it):
    /// `func_80832440`'s interruption, then `func_80853080`.
    pub fn stand_still(&mut self, data: &GameData) {
        self.func_80832440();
        self.zero_speed_xz();
        self.state1 &= !(STATE1_29 | STATE1_0);
        self.func_80853080(data);
    }

    fn func_80853080(&mut self, data: &GameData) {
        self.setup_action(data, Action::StandingStill, 1);
        let a = self.anim(data, group::WAIT);
        self.anim_change_once_morph(data, a);
        self.current_yaw = self.actor.shape_rot.y;
    }

    /// `Player_AnimChangeOnceMorph`: play once, blending in over 6 frames.
    fn anim_change_once_morph(&mut self, data: &GameData, a: AnimId) {
        let last = data.anims[a].last_frame();
        self.skel.change(data, a, 1.0, 0.0, last, ANIMMODE_ONCE, -6.0);
    }

    /// `func_80839FFC`: switch to the standing action (locked-on, parallel or plain) without
    /// touching the animation.
    fn func_80839FFC(&mut self, data: &GameData) {
        let action = if self.state1 & STATE1_4 != 0 {
            Action::TargetIdle
        } else if self.friendly_lock_on_or_parallel() {
            Action::ParallelIdle
        } else {
            Action::StandingStill
        };
        self.setup_action(data, action, 1);
    }

    /// `func_8083A060`.
    fn func_8083A060(&mut self, data: &GameData) {
        self.func_80839FFC(data);
        if self.state1 & STATE1_4 != 0 {
            self.action_var2 = 1;
        }
    }

    /// `func_8083A098`: stand, playing `anim` once (landing animations).
    fn func_8083A098(&mut self, data: &GameData, anim: AnimId) {
        self.func_8083A060(data);
        self.skel.play_once_set_speed(data, anim, self.s.speed_scale);
    }

    /// `func_8083C858`: start walking/running (the targeting run while targeting).
    fn func_8083C858(&mut self, data: &GameData) {
        let action = if self.is_z_targeting() { Action::TargetRun } else { Action::Run };
        self.setup_action(data, action, 1);
        let a = self.anim(data, group::RUN);
        self.skel.change(data, a, 1.0, 0.0, 0.0, ANIMMODE_LOOP, -6.0);
        self.unk_89C = 0;
        self.unk_864 = 0.0;
        self.unk_868 = 0.0;
    }

    /// `func_8083C8DC`: face `yaw` and start running.
    fn func_8083C8DC(&mut self, data: &GameData, yaw: i16) {
        self.actor.shape_rot.y = yaw;
        self.current_yaw = yaw;
        self.func_8083C858(data);
    }

    /// `Player_SetupTurnInPlace`: turn in place towards `yaw`.
    fn setup_turn_in_place(&mut self, data: &GameData, yaw: i16) {
        self.current_yaw = yaw;
        self.setup_action(data, Action::Turn, 1);
        self.turn_rate = (1200.0 * self.s.speed_scale) as i16;
        let a = self.anim(data, group::TURN);
        self.skel.change(data, a, 1.0, 0.0, 0.0, ANIMMODE_LOOP, -6.0);
    }

    /// `func_8083C0E8`: finish a turn.
    fn func_8083C0E8(&mut self, data: &GameData) {
        self.setup_action(data, Action::StandingStill, 1);
        let a = self.anim(data, group::WAIT);
        self.skel.play_once(data, a);
        self.current_yaw = self.actor.shape_rot.y;
    }

    /// `func_8083BF50`: the stop animation, picked by the walk phase so the feet match.
    fn func_8083BF50(&mut self, data: &GameData) {
        let mut sp30 = self.unk_868 - 3.0;
        if sp30 < 0.0 {
            sp30 += 29.0;
        }
        let anim;
        if sp30 < 14.0 {
            anim = self.anim(data, group::END_WALK_A);
            sp30 = 11.0 - sp30;
            if sp30 < 0.0 {
                sp30 = 1.375 * -sp30;
            }
            sp30 /= 11.0;
        } else {
            anim = self.anim(data, group::END_WALK_B);
            sp30 = 26.0 - sp30;
            if sp30 < 0.0 {
                sp30 = 2.0 * -sp30;
            }
            sp30 /= 12.0;
        }
        let last = data.anims[anim].last_frame();
        self.skel.change(data, anim, 1.0, 0.0, last, ANIMMODE_ONCE, 4.0 * sp30);
        self.current_yaw = self.actor.shape_rot.y;
    }

    /// `func_8083C0B8`: stop running.
    fn func_8083C0B8(&mut self, data: &GameData) {
        self.func_80839FFC(data);
        self.func_8083BF50(data);
    }

    /// `Player_ChooseNextIdleAnim`: the next idle animation: the plain wait every other time, otherwise
    /// a random fidget from `sFidgetAnimations`.
    fn choose_next_idle_anim(&mut self, data: &GameData) {
        self.idle_type = (self.idle_type + 1) & 1;
        let anim = if self.idle_type != 0 {
            self.state2 &= !STATE2_28;
            self.anim(data, group::WAIT)
        } else {
            self.state2 |= STATE2_28;
            // Room behaviour type 2 is 0 (normal); health not critical.
            let mut sp38 = 0usize;
            let sp34 = (self.rand_zero_one() * 5.0) as i32;
            // Right hand is open (nothing held), so only variants 1 and 2 qualify.
            if sp34 < 4 && sp34 != 0 && sp34 != 3 {
                sp38 = sp34 as usize + 9;
            }
            let pair = data.idle_variants[sp38];
            if self.model_anim_type != 1 { pair[1] } else { pair[0] }
        };
        let last = data.anims[anim].last_frame();
        self.skel.change(data, anim, (2.0 / 3.0) * self.s.speed_scale, 0.0, last, ANIMMODE_ONCE, -6.0);
    }

    // ================================================================================
    // Look and lean (head/upper body rotations applied at draw time)

    /// `Player_ScaledStepBinangClamped`.
    fn scaled_step_binang_clamped(v: &mut i16, arg1: i16, arg2: i16, arg3: i16, arg4: i16, arg5: i16) -> i16 {
        let t1 = arg4.wrapping_sub(*v);
        let t2 = t1.clamp(arg5.wrapping_neg().min(arg5), arg5.max(arg5.wrapping_neg()));
        *v = v.wrapping_add(t1.wrapping_sub(t2));
        scaled_step_to_s(v, arg1, arg2);
        let t3 = *v;
        if *v < -arg3 {
            *v = -arg3;
        } else if *v > arg3 {
            *v = arg3;
        }
        t3.wrapping_sub(*v)
    }

    /// `func_80836AB8(this, arg1)`: spread the focus rotation over head and torso (`arg1`:
    /// aiming, the whole upper body follows the focus).
    fn func_80836AB8_arg(&mut self, arg1: bool) -> i16 {
        if arg1 {
            self.upper_limb_rot_x = self.actor.focus_rot.x;
            self.unk_6AE_rot_flags |= 0x41;
            return self.actor.focus_rot.y;
        }
        self.func_80836AB8()
    }

    /// `func_80836AB8(this, 0)`.
    fn func_80836AB8(&mut self) -> i16 {
        let mut var = self.actor.shape_rot.y;
        let fx = self.actor.focus_rot.x;
        let inner = Self::scaled_step_binang_clamped(&mut self.head_limb_rot_x, fx, 600, 10000, fx, 0);
        let b6 = self.head_limb_rot_x;
        Self::scaled_step_binang_clamped(&mut self.upper_limb_rot_x, inner, 200, 4000, b6, 10000);
        let mut sp36 = self.actor.focus_rot.y.wrapping_sub(var);
        let be = self.upper_limb_rot_y;
        Self::scaled_step_binang_clamped(&mut sp36, 0, 200, 24000, be, 8000);
        var = self.actor.focus_rot.y.wrapping_sub(sp36);
        let be = self.upper_limb_rot_y;
        Self::scaled_step_binang_clamped(&mut self.head_limb_rot_y, sp36.wrapping_sub(be), 200, 8000, sp36, 8000);
        let b8 = self.head_limb_rot_y;
        Self::scaled_step_binang_clamped(&mut self.upper_limb_rot_y, sp36, 200, 8000, b8, 8000);
        self.unk_6AE_rot_flags |= 0xD9;
        var
    }

    /// `func_8083DC54`: look at the target (aiming, with the upper body), or at the ground ahead
    /// (steeply down on `FLOOR_TYPE_11`).
    fn func_8083DC54(&mut self, env: &Env) {
        let aiming = self.func_8002dd78() || self.func_808334b4();
        if self.focus_actor.is_some() {
            self.func_8083DB98(env, aiming);
            return;
        }
        if self.s.floor_type == FLOOR_TYPE_11 {
            smooth_step_to_s(&mut self.actor.focus_rot.x, -20000, 10, 4000, 800);
            self.func_80836AB8_arg(aiming);
            return;
        }
        let mut sp46 = 0i16;
        let p = self.get_relative_position(self.actor.world_pos, HEAD_PROBE);
        let (floor, _) = env.col.entity_raycast_down(p);
        if floor > BGCHECK_Y_MIN {
            let t = atan2_s(40.0, self.actor.world_pos.y - floor);
            sp46 = t.clamp(-4000, 4000);
        }
        self.actor.focus_rot.y = self.actor.shape_rot.y;
        smooth_step_to_s(&mut self.actor.focus_rot.x, sp46, 14, 4000, 30);
        self.func_80836AB8_arg(aiming);
    }

    /// `func_8083DDC8`: lean into turns when running fast (not aiming).
    fn func_8083DDC8(&mut self, env: &Env) {
        if !self.func_8002dd78() && !self.func_808334b4() && self.linear_velocity > 5.0 {
            let t1 = f2s(self.linear_velocity * 200.0).clamp(-4000, 4000);
            let t2 = f2s(self.current_yaw.wrapping_sub(self.actor.shape_rot.y) as f32 * self.linear_velocity * 0.1);
            let t2 = t2.wrapping_neg().clamp(-4000, 4000);
            scaled_step_to_s(&mut self.upper_limb_rot_x, t1, 900);
            self.head_limb_rot_x = f2s(-(self.upper_limb_rot_x as f32) * 0.5);
            scaled_step_to_s(&mut self.head_limb_rot_z, t2, 300);
            scaled_step_to_s(&mut self.upper_limb_rot_z, t2, 200);
            self.unk_6AE_rot_flags |= 0x168;
        } else {
            self.func_8083DC54(env);
        }
    }

    /// `Player_ApproachZeroBinang`.
    fn approach_zero_binang(v: &mut i16) {
        let step = f2s((abs16(*v) as f32 * 100.0) / 1000.0).clamp(400, 4000);
        scaled_step_to_s(v, 0, step);
    }

    /// `func_80847298`: relax whichever look/lean rotations were not driven last frame.
    fn func_80847298(&mut self) {
        let f = self.unk_6AE_rot_flags;
        if f & 2 == 0 {
            let mut d = self.actor.focus_rot.y.wrapping_sub(self.actor.shape_rot.y);
            Self::approach_zero_binang(&mut d);
            self.actor.focus_rot.y = self.actor.shape_rot.y.wrapping_add(d);
        }
        if f & 1 == 0 {
            Self::approach_zero_binang(&mut self.actor.focus_rot.x);
        }
        if f & 8 == 0 {
            Self::approach_zero_binang(&mut self.head_limb_rot_x);
        }
        if f & 0x40 == 0 {
            Self::approach_zero_binang(&mut self.upper_limb_rot_x);
        }
        if f & 4 == 0 {
            Self::approach_zero_binang(&mut self.actor.focus_rot.z);
        }
        if f & 0x10 == 0 {
            Self::approach_zero_binang(&mut self.head_limb_rot_y);
        }
        if f & 0x20 == 0 {
            Self::approach_zero_binang(&mut self.head_limb_rot_z);
        }
        if f & 0x80 == 0 {
            if self.upper_limb_yaw_secondary != 0 {
                Self::approach_zero_binang(&mut self.upper_limb_yaw_secondary);
            } else {
                Self::approach_zero_binang(&mut self.upper_limb_rot_y);
            }
        }
        if f & 0x100 == 0 {
            Self::approach_zero_binang(&mut self.upper_limb_rot_z);
        }
        self.unk_6AE_rot_flags = 0;
    }

    /// `Player_UpdateShapeYaw`: the facing. Standing, it turns towards a locked target (once the reticle
    /// has locked) or to `targetYaw` in parallel mode; running/rolling it follows `currentYaw`.
    fn update_shape_yaw(&mut self, env: &Env) {
        let prev = self.actor.shape_rot.y;
        if self.state2 & (STATE2_5 | STATE2_6) == 0 {
            if let Some(t) = self.focus_actor.and_then(|t| env.target(t))
                && env.target.reticle_locked
            {
                let y = oot_game::target::yaw_to(self.actor.world_pos, t.focus_pos);
                scaled_step_to_s(&mut self.actor.shape_rot.y, y, 4000);
            } else if self.state1 & STATE1_17 != 0 {
                let y = self.target_yaw;
                scaled_step_to_s(&mut self.actor.shape_rot.y, y, 4000);
            }
        } else if self.state2 & STATE2_6 == 0 {
            scaled_step_to_s(&mut self.actor.shape_rot.y, self.current_yaw, 2000);
        }
        self.unk_87C = self.actor.shape_rot.y.wrapping_sub(prev);
    }

    // ================================================================================
    // Walk / run animation

    /// A sound request (`PlayRequest::Sfx`).
    fn sfx(&mut self, s: PlayerSfx) {
        self.play_requests.push(PlayRequest::Sfx(s));
    }

    /// `Player_PlaySfx(&this->actor, sfxId)`.
    fn play_sfx(&mut self, sfx_id: u16) {
        self.sfx(PlayerSfx::Actor(sfx_id));
    }

    /// `Player_PlayVoiceSfx`: Link's voice (the age's voice bank, `ageProperties->unk_92`).
    fn play_voice_sfx(&mut self, sfx_id: u16) {
        if self.actor.category == ACTORCAT_PLAYER {
            self.play_sfx(sfx_id.wrapping_add(self.age.climb.unk_92));
        } else {
            self.sfx(PlayerSfx::F4190(sfx_id));
        }
    }

    /// `Player_ApplyFloorSfxOffset`: a sound for the floor (`floorSfxOffset`).
    fn apply_floor_sfx_offset(&self, sfx_id: u16) -> u16 {
        sfx_id.wrapping_add(self.floor_sfx_offset)
    }

    /// `Player_PlayFloorSfx`.
    fn play_floor_sfx(&mut self, sfx_id: u16) {
        let id = self.apply_floor_sfx_offset(sfx_id);
        self.play_sfx(id);
    }

    /// `Player_ApplyFloorAndAgeSfxOffsets`: a sound for the floor and the age (`ageProperties->unk_94`).
    fn apply_floor_and_age_sfx_offsets(&self, sfx_id: u16) -> u16 {
        sfx_id.wrapping_add(self.floor_sfx_offset).wrapping_add(self.age.climb.unk_94)
    }

    /// `Player_PlayFloorSfxByAge`.
    fn play_floor_sfx_by_age(&mut self, sfx_id: u16) {
        let id = self.apply_floor_and_age_sfx_offsets(sfx_id);
        self.play_sfx(id);
    }

    /// `Player_PlaySteppingSfx`: a footstep at speed `arg1`.
    fn play_stepping_sfx(&mut self, arg1: f32) {
        let sfx_id = if self.current_boots == PLAYER_BOOTS_IRON { NA_SE_PL_WALK_HEAVYBOOTS } else { self.apply_floor_and_age_sfx_offsets(NA_SE_PL_WALK_GROUND) };
        self.sfx(PlayerSfx::Footstep(sfx_id, arg1));
    }

    /// `Player_PlayJumpingSfx`: the jump.
    fn play_jumping_sfx(&mut self) {
        let sfx_id = if self.current_boots == PLAYER_BOOTS_IRON { NA_SE_PL_JUMP_HEAVYBOOTS } else { self.apply_floor_and_age_sfx_offsets(NA_SE_PL_JUMP) };
        self.play_sfx(sfx_id);
    }

    /// `Player_PlayLandingSfx`: the landing.
    fn play_landing_sfx(&mut self) {
        let sfx_id = if self.current_boots == PLAYER_BOOTS_IRON { NA_SE_PL_LAND_HEAVYBOOTS } else { self.apply_floor_and_age_sfx_offsets(NA_SE_PL_LAND) };
        self.play_sfx(sfx_id);
    }

    /// `Player_ProcessAnimSfxList`: the sounds of a `AnimSfxEntry` table on this frame of the animation:
    /// each row's frame is `field`'s low 11 bits, its kind the next four (0x800 at Player,
    /// 0x1000 for the floor, 0x1800 for the floor and the age, 0x2000 the voice, 0x2800 the
    /// landing, 0x3000 a running step, 0x3800 the jump, 0x4000 a step, 0x4800 the ladder); the
    /// last row's `field` is negative.
    fn process_anim_sfx_list(&mut self, entries: &[(u16, i16)]) {
        use oot_game::audio::sfx::NA_SE_PL_WALK_LADDER;
        for &(sfx_id, field) in entries {
            let data = (field as i32).abs();
            let flags = data & 0x7800;
            if self.skel.on_frame((data & 0x7FF) as f32) {
                match flags {
                    0x800 => self.play_sfx(sfx_id),
                    0x1000 => self.play_floor_sfx(sfx_id),
                    0x1800 => self.play_floor_sfx_by_age(sfx_id),
                    0x2000 => self.play_voice_sfx(sfx_id),
                    0x2800 => self.play_landing_sfx(),
                    0x3000 => self.play_stepping_sfx(6.0),
                    0x3800 => self.play_jumping_sfx(),
                    0x4000 => self.play_stepping_sfx(0.0),
                    0x4800 => self.sfx(PlayerSfx::Footstep(self.age.climb.unk_94.wrapping_add(NA_SE_PL_WALK_LADDER), 0.0)),
                    _ => {}
                }
            }
            if field < 0 {
                break;
            }
        }
    }

    /// `func_808328EC`: a sound that marks the frame (`PLAYER_STATE2_3`).
    fn func_808328EC(&mut self, sfx_id: u16) {
        self.play_sfx(sfx_id);
        self.state2 |= STATE2_3;
    }

    /// `func_8084021C`: did the phase cross `arg2`/`arg3` this step (footstep timing)?
    fn func_8084021C(arg0: f32, arg1: f32, arg2: f32, mut arg3: f32) -> bool {
        if arg3 == 0.0 && arg1 > 0.0 {
            arg3 = arg2;
        }
        let t = (arg0 + arg1) - arg3;
        t * arg1 >= 0.0 && (t - arg1) * arg1 < 0.0
    }

    /// `func_8084029C`: advance the walk phase `unk_868` (29-frame cycle).
    fn func_8084029C(&mut self, arg1: f32) {
        let arg1 = (arg1 * UPDATE_SCALE).clamp(-7.25, 7.25);
        // (The hover boots' NA_SE_PL_HOBBERBOOTS_LV: they aren't ported.)
        if Self::func_8084021C(self.unk_868, arg1, 29.0, 10.0) || Self::func_8084021C(self.unk_868, arg1, 29.0, 24.0) {
            let v = self.linear_velocity;
            self.play_stepping_sfx(v);
            if self.linear_velocity > 4.0 {
                self.state2 |= STATE2_3;
            }
        }
        self.unk_868 += arg1;
        if self.unk_868 < 0.0 {
            self.unk_868 += 29.0;
        } else if self.unk_868 >= 29.0 {
            self.unk_868 -= 29.0;
        }
    }

    /// `func_80833438`: which run animation (damage, iron boots, normal).
    fn func_80833438(&self, data: &GameData) -> AnimId {
        if self.unk_890 != 0 { self.anim(data, group::DAMAGE_RUN) } else { self.anim(data, group::RUN) }
    }

    /// `func_80841CC4`: walk animation, blended towards climbing up/down on slopes.
    fn func_80841CC4(&mut self, data: &GameData, to_morph: bool) {
        let target = if abs16(self.s.facing_slope) < 3640 { 0 } else { self.s.facing_slope.clamp(-10922, 10922) };
        scaled_step_to_s(&mut self.unk_89C, target, 400);
        let walk = self.anim(data, group::WALK);
        if self.model_anim_type == 3 || (self.unk_89C == 0 && self.unk_6C4 <= 0.0) {
            if to_morph {
                self.skel.load_to_morph(data, walk, self.unk_868);
            } else {
                self.skel.load_to_joint(data, walk, self.unk_868);
            }
            return;
        }
        let mut rate = if self.unk_89C != 0 { self.unk_89C as f32 / 10922.0 } else { self.unk_6C4 * 0.0006 };
        rate *= self.linear_velocity.abs() * 0.5;
        if rate > 1.0 {
            rate = 1.0;
        }
        let anim = if rate < 0.0 {
            rate = -rate;
            data.anim("link_normal_climb_down")
        } else {
            data.anim("link_normal_climb_up")
        };
        if to_morph {
            self.skel.blend_to_morph(data, walk, self.unk_868, anim, self.unk_868, rate);
        } else {
            self.skel.blend_to_joint(data, walk, self.unk_868, anim, self.unk_868, rate);
        }
    }

    /// `func_80841EE4`: the walk→run blend by speed (`REG(35..38)`, `REG(48)`).
    fn func_80841EE4(&mut self, data: &GameData) {
        let r = |n| self.regs.reg(n) as f32 / 1000.0;
        let temp1;
        if self.unk_864 < 1.0 {
            self.func_8084029C(r(35));
            let walk = self.anim(data, group::WALK);
            self.skel.load_to_joint(data, walk, self.unk_868);
            self.unk_864 = (self.unk_864 + UPDATE_SCALE).min(1.0);
            temp1 = self.unk_864;
        } else {
            let temp2 = self.linear_velocity - (self.regs.reg(48) as f32 / 100.0);
            if temp2 < 0.0 {
                temp1 = 1.0;
                self.func_8084029C(r(35) + r(36) * self.linear_velocity);
                self.func_80841CC4(data, false);
            } else {
                let mut t = r(37) * temp2;
                if t < 1.0 {
                    self.func_8084029C(r(35) + r(36) * self.linear_velocity);
                } else {
                    t = 1.0;
                    self.func_8084029C(1.2 + r(38) * temp2);
                }
                temp1 = t;
                self.func_80841CC4(data, true);
                let run = self.func_80833438(data);
                self.skel.load_to_joint(data, run, self.unk_868 * (20.0 / 29.0));
            }
        }
        if temp1 < 1.0 {
            self.skel.interp_joint_morph(1.0 - temp1);
        }
    }

    // ================================================================================
    // Actions

    /// `Player_Action_Idle`: standing still. Plays idle animations, turns in place towards the
    /// stick, and starts running once the stick gives a speed.
    fn action_idle(&mut self, env: &Env) {
        let data = env.data;
        let done = self.skel.update(data);
        if done {
            if self.action_var2 != 0 {
                // DECR(this->av2.actionVar2)
                if self.action_var2 != 0 {
                    self.action_var2 -= 1;
                    if self.action_var2 == 0 {
                        self.skel.end_frame = self.skel.anim_length - 1.0;
                    }
                }
                // Fall-damage stagger: the root bobs by ±0x28.
                self.skel.joint[0][1] = self.skel.joint[0][1].wrapping_add(((self.action_var2 & 1) * 0x50) - 0x28);
            } else {
                self.finish_anim_movement();
                self.choose_next_idle_anim(data);
            }
        }
        self.decelerate_to_zero();
        if self.action_var2 == 0 && !self.try_action_handler_list(env, Self::S_ACTION_HANDLER_LIST_IDLE, true) {
            if self.player_update_hostile_lock_on_env(env) {
                self.func_8083CEAC(data);
                return;
            }
            if self.friendly_lock_on_or_parallel() {
                self.func_80839F30(data);
                return;
            }
            let (_, speed, yaw) = self.get_movement_speed_and_yaw(env, 0.018);
            if speed != 0.0 {
                self.func_8083C8DC(data, yaw);
                return;
            }
            let d = yaw.wrapping_sub(self.actor.shape_rot.y);
            if abs16(d) > 800 {
                self.setup_turn_in_place(data, yaw);
                return;
            }
            scaled_step_to_s(&mut self.actor.shape_rot.y, yaw, 1200);
            self.current_yaw = self.actor.shape_rot.y;
            if self.anim(data, group::WAIT) == self.skel.animation {
                self.func_8083DC54(env);
            }
        }
    }

    /// `Player_Action_TurnInPlace`: turning in place.
    fn action_turn_in_place(&mut self, env: &Env) {
        let data = env.data;
        self.skel.update(data);
        let (_, speed, yaw) = self.get_movement_speed_and_yaw(env, 0.018);
        if !self.try_action_handler_list(env, Self::S_ACTION_HANDLER_LIST_TURN_IN_PLACE, true) {
            if speed != 0.0 {
                self.actor.shape_rot.y = yaw;
                self.func_8083C858(data);
            } else if scaled_step_to_s(&mut self.actor.shape_rot.y, yaw, self.turn_rate) {
                self.func_8083C0E8(data);
            }
            self.current_yaw = self.actor.shape_rot.y;
        }
    }

    /// `Player_Action_80842180`: walking and running.
    fn action_80842180(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5;
        self.func_80841EE4(data);
        if !self.try_action_handler_list(env, Self::S_ACTION_HANDLER_LIST8, true) {
            if self.is_z_targeting_with_hostile_update(env) {
                self.func_8083C858(data);
                return;
            }
            let (_, mut speed, mut yaw) = self.get_movement_speed_and_yaw(env, 0.018);
            if !self.func_8083C484(&mut speed, &mut yaw) {
                self.func_8083DF68(speed, yaw);
                self.func_8083DDC8(env);
                if self.linear_velocity == 0.0 && speed == 0.0 {
                    self.func_8083C0B8(data);
                }
            }
        }
    }

    /// `Player_Action_Roll`: rolling.
    fn action_roll(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5;
        let done = self.skel.update(data);
        if self.skel.on_frame(8.0) {
            self.set_invulnerability(-10);
        }
        // func_80842964 (first person, items, grabbing): not taken.
        if self.action_var2 != 0 {
            step_to_f(&mut self.linear_velocity, 0.0, 2.0);
            let temp = self.try_action_interrupt(env, 5.0);
            if temp != 0 && (temp > 0 || done) {
                self.func_8083A060(data);
            }
        } else {
            if self.linear_velocity >= 7.0
                && self.actor.bg_check_flags & BGCHECKFLAG_PLAYER_WALL_INTERACT != 0
                && self.s.wall_move_diff < 0x2000
            {
                // Bonk.
                let a = self.anim(data, group::ROLL_BONK);
                self.skel.play_once(data, a);
                self.linear_velocity = -self.linear_velocity;
                self.play_requests.push(PlayRequest::Quake { speed: 33267, y: 3, duration: 12 });
                // (Player_RequestRumble isn't ported.)
                self.play_sfx(NA_SE_PL_BODY_HIT);
                self.play_voice_sfx(NA_SE_VO_LI_CLIMB_END);
                self.action_var2 = 1;
                self.note("roll bonk");
                return;
            }
            // Past frame 15 Player_ActionHandler_7 could chain into a sword attack: no sword.
            if self.skel.cur_frame >= 20.0 {
                self.func_8083A060(data);
                return;
            }
            let (_, mut speed, _) = self.get_movement_speed_and_yaw(env, 0.018);
            speed *= 1.5;
            if speed < 3.0 || self.stick_dir() != 0 {
                speed = 3.0;
            }
            let yaw = self.actor.shape_rot.y;
            self.func_8083DF68(speed, yaw);
            if self.func_8084269c(env) {
                self.actor.play_sfx_flagged2(NA_SE_PL_ROLL_DUST - SFX_FLAG);
            }
            self.process_anim_sfx_list(env.audio.player_anim_sfx("sRollAnimSfxList"));
        }
    }

    /// `func_8084269C`: on a dirt or sand floor, a puff of dust at each foot (`func_800286CC`),
    /// each at a point scattered by `func_8084260C` from the floor under the foot. True when there
    /// was dust.
    ///
    /// @bug (game): the right foot's puff is at the foot itself; its scattered point is drawn
    /// and not used.
    fn func_8084269c(&mut self, env: &Env) -> bool {
        if self.floor_sfx_offset == 0 || self.floor_sfx_offset == SURFACE_MATERIAL_SAND {
            let mut io = env.io.borrow_mut();
            let [l, r] = self.feet_pos;
            let floor = self.actor.floor_height;
            let p = func_8084260c(&mut io.rand, l, floor - l.y, 7.0, 5.0);
            io.ss().func_800286cc(p, Vec3::ZERO, Vec3::ZERO, 50, 30);
            let _ = func_8084260c(&mut io.rand, r, floor - r.y, 7.0, 5.0);
            io.ss().func_800286cc(r, Vec3::ZERO, Vec3::ZERO, 50, 30);
            return true;
        }
        false
    }

    /// `Player_TryActionInterrupt`: near the end of an animation, allow interrupts or restarting to run.
    /// Returns 1 to move on, 0 if an interrupt took over, -1 to keep playing.
    fn try_action_interrupt(&mut self, env: &Env, arg3: f32) -> i32 {
        if (self.skel.end_frame - arg3) <= self.skel.cur_frame {
            if self.try_action_handler_list(env, Self::S_ACTION_HANDLER_LIST_IDLE, true) {
                return 0;
            }
            if self.get_movement_speed_and_yaw(env, 0.018).0 {
                return 1;
            }
        }
        -1
    }

    /// `func_80843E64`: landing. Returns 1/2 for a damaging fall (≥ 400 / 800: `D_80854600`'s
    /// half heart and heart, then 40 frames of invincibility, the body's and the voice's
    /// sounds), -1 when that was the last of Link's health, 0 otherwise with the landing's sound.
    /// (The rumble isn't ported.)
    fn func_80843E64(&mut self, env: &Env) -> i32 {
        let sp34 = if self.s.floor_type == FLOOR_TYPE_6 || self.s.floor_type == FLOOR_TYPE_9 { 0 } else { self.fall_distance as i32 };
        step_to_f(&mut self.linear_velocity, 0.0, 1.0);
        self.state1 &= !(STATE1_18 | STATE1_19);
        if sp34 >= 400 {
            let idx = if self.fall_distance < 800 { 0 } else { 1 };
            // D_80854600[impactIndex].damage.
            let damage = if idx == 0 { -8 } else { -16 };
            if self.player_inflict_damage(env, damage) {
                return -1;
            }
            self.set_intangibility(40);
            self.play_requests.push(PlayRequest::Quake { speed: 32967, y: 2, duration: 30 });
            self.play_sfx(NA_SE_PL_BODY_HIT);
            // D_80854600[impactIndex].sfxId (both rows).
            self.play_voice_sfx(NA_SE_VO_LI_LAND_DAMAGE_S);
            return idx + 1;
        }
        if sp34 > 200 && self.s.floor_type == FLOOR_TYPE_6 {
            // (The rumble, sp34 doubled, isn't ported.)
            self.play_voice_sfx(NA_SE_VO_LI_CLIMB_END);
        }
        self.play_landing_sfx();
        0
    }

    /// `func_80843E14`: a fall's voice (Ruto, held, would cry too: `En_Ru1` isn't ported, nor is
    /// holding).
    fn func_80843E14(&mut self, sfx_id: u16) {
        self.play_voice_sfx(sfx_id);
    }

    /// `Player_Action_8084411C`: in the air.
    fn action_8084411c(&mut self, env: &Env) {
        let data = env.data;
        // respawn[RESPAWN_MODE_TOP].data > 40 (void-out float): not modelled.
        if self.state1 & STATE1_4 != 0 {
            self.actor.gravity = -1.2;
        }
        let (_, speed, yaw) = self.get_movement_speed_and_yaw(env, 0.0);
        if !self.grounded() {
            self.skel.update(data);
            if self.state2 & STATE2_19 == 0 {
                self.func_8083DFE0(speed, yaw);
            }
            self.update_upper_body(env);
            // func_8083BBA0 (the jump slash in the air) is not ported.
            if self.actor.velocity.y < 0.0 {
                if self.action_var2 >= 0 {
                    if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 || self.action_var2 == 0 || self.fall_distance > 0 {
                        if self.s.floor_dist > 800.0 || self.state1 & STATE1_2 != 0 {
                            self.func_80843E14(NA_SE_VO_LI_FALL_S);
                            self.state1 &= !STATE1_2;
                        }
                        let a = data.anim("link_normal_landing");
                        self.skel.change(data, a, 1.0, 0.0, 0.0, ANIMMODE_ONCE, 8.0);
                        self.action_var2 = -1;
                    }
                } else {
                    if self.action_var2 == -1 && self.fall_distance > 120 && self.s.floor_dist > 280.0 {
                        self.action_var2 = -2;
                        self.func_80843E14(NA_SE_VO_LI_FALL_L);
                    }
                    if self.actor.bg_check_flags & BGCHECKFLAG_PLAYER_WALL_INTERACT != 0
                        && self.state2 & STATE2_19 == 0
                        && self.state1 & (STATE1_11 | STATE1_27) == 0
                        && self.linear_velocity > 0.0
                    {
                        if self.wall_height >= 150.0 && self.stick_dir() == 0 {
                            // func_8083EC18: grab a climbable wall from the air — not ported.
                            self.note("mid-air wall climb (func_8083EC18) not ported");
                        } else if self.ledge_climb_type >= 2
                            && self.wall_height < 150.0
                            && ((self.actor.world_pos.y - self.actor.floor_height) + self.wall_height) > 70.0 * self.age.translation_scale
                            && let Some(wp) = self.actor.wall_poly
                        {
                            self.skel.disable_queue();
                            self.play_voice_sfx(if self.state1 & STATE1_2 != 0 { NA_SE_VO_LI_HOOKSHOT_HANG } else { NA_SE_VO_LI_HANG });
                            self.actor.world_pos.y += self.wall_height;
                            let a = self.anim(data, group::HANG_GRAB);
                            let dist = self.wall_distance;
                            self.func_8083A5C4(data, env, wp, dist, a);
                            self.current_yaw = self.current_yaw.wrapping_add(i16::MIN);
                            self.actor.shape_rot.y = self.current_yaw;
                            self.state1 |= STATE1_13;
                            return;
                        }
                    }
                }
            }
        } else {
            let mut anim = self.anim(data, group::LANDING);
            if self.state2 & STATE2_19 != 0 {
                let row = data.side_hop_anims[self.action_var1.clamp(0, 3) as usize];
                anim = if self.state1 & STATE1_4 != 0 { row[2] } else { row[1] };
            } else if self.skel.animation == data.anim("link_normal_run_jump") {
                anim = data.anim("link_normal_run_jump_end");
            } else if self.state1 & STATE1_4 != 0 {
                anim = data.anim("link_anchor_landingR");
                self.func_80833C3C();
            } else if self.fall_distance <= 80 {
                anim = self.anim(data, group::SHORT_LANDING);
            } else if self.fall_distance < 800 && self.stick_dir() == 0 && self.state1 & STATE1_11 == 0 {
                self.setup_roll(data);
                return;
            }
            let sp3c = self.func_80843E64(env);
            if sp3c > 0 {
                let a = self.anim(data, group::LANDING);
                self.func_8083A098(data, a);
                self.skel.end_frame = 8.0;
                self.action_var2 = if sp3c == 1 { 10 } else { 20 };
            } else if sp3c == 0 {
                self.func_8083A098(data, anim);
            }
        }
    }

    // ================================================================================
    // Z-targeting (Player_UpdateZTargeting) and the targeting actions

    /// `Player_FriendlyLockOnOrParallel`: parallel mode or a non-hostile target.
    fn friendly_lock_on_or_parallel(&self) -> bool {
        self.state1 & (STATE1_16 | STATE1_17 | STATE1_30) != 0
    }

    /// `Player_IsZTargeting`: locked on or in parallel mode.
    fn is_z_targeting(&self) -> bool {
        self.state1 & STATE1_4 != 0 || self.friendly_lock_on_or_parallel()
    }

    /// `Player_IsZTargetingWithHostileUpdate`.
    fn is_z_targeting_with_hostile_update(&mut self, env: &Env) -> bool {
        self.player_update_hostile_lock_on_env(env) || self.friendly_lock_on_or_parallel()
    }

    /// `Player_ReleaseLockOn`: drop the target.
    fn release_lock_on(&mut self) {
        self.focus_actor = None;
        self.state2 &= !STATE2_13;
    }

    /// `Player_ClearZTargeting`: leave targeting (or, in the air, remember to after landing).
    fn clear_z_targeting(&mut self) {
        if self.grounded()
            || self.state1 & ((1 << 21) | STATE1_23 | STATE1_27) != 0
            || (self.state1 & (STATE1_18 | STATE1_19) == 0 && (self.actor.world_pos.y - self.actor.floor_height) < 100.0)
        {
            self.state1 &= !(STATE1_15 | STATE1_16 | STATE1_17 | STATE1_18 | STATE1_19 | STATE1_30);
        } else if self.state1 & (STATE1_18 | STATE1_19 | (1 << 21)) == 0 {
            self.state1 |= STATE1_19;
        }
        self.release_lock_on();
    }

    /// `Player_SetParallel`: enter parallel mode, squaring up to a wall in front.
    fn set_parallel(&mut self) {
        self.state1 |= STATE1_17;
        if self.skel.move_flags & 0x80 == 0 && self.actor.bg_check_flags & BGCHECKFLAG_PLAYER_WALL_INTERACT != 0 && self.s.wall_facing_diff < 0x2000 {
            self.current_yaw = self.actor.wall_yaw.wrapping_add(i16::MIN);
            self.actor.shape_rot.y = self.current_yaw;
        }
        self.target_yaw = self.actor.shape_rot.y;
    }

    /// `Player_UpdateZTargeting`: the Z button. With the default "Switch" setting (`zTargetSetting` 0) a
    /// press locks on to `arrowPointedActor` (or the next candidate if it's already locked) and
    /// keeps it (`PLAYER_STATE2_LOCK_ON_WITH_SWITCH`) until pressed again; with nothing to target, parallel mode.
    fn update_z_targeting(&mut self, env: &Env) {
        let z = self.input.cur.held(BTN_Z);
        if !z {
            self.state1 &= !STATE1_30;
        }
        if self.state1 & (STATE1_7 | STATE1_29) != 0 || self.state3 & STATE3_7 != 0 {
            self.z_target_active_timer = 0;
        } else if z || self.state2 & STATE2_13 != 0 {
            if self.z_target_active_timer <= 5 {
                self.z_target_active_timer = 5;
            } else {
                self.z_target_active_timer -= 1;
            }
        } else if self.state1 & STATE1_17 != 0 {
            self.z_target_active_timer = 0;
        } else if self.z_target_active_timer != 0 {
            self.z_target_active_timer -= 1;
        }
        let sp1c = self.z_target_active_timer >= 6;
        // Player_IsTalking: talking (ACTOR_FLAG_TALK) keeps the target as it is.
        let cond = self.actor.flags & ACTOR_FLAG_TALK == ACTOR_FLAG_TALK;
        if cond || self.z_target_active_timer != 0 || self.state1 & (STATE1_12 | STATE1_25) != 0 {
            if !cond {
                if self.state1 & STATE1_25 == 0 && self.input.press.held(BTN_Z) {
                    let hold = false;
                    self.state1 |= STATE1_15;
                    match env.target.arrow_pointed.filter(|&t| env.target(t).is_some_and(|a| a.flags & ACTOR_FLAG_LOCK_ON_DISABLED == 0)) {
                        Some(t) => {
                            let mut to = Some(t);
                            if to == self.focus_actor {
                                to = env.target.arrow_hover_actor;
                            }
                            if to != self.focus_actor {
                                if !hold {
                                    self.state2 |= STATE2_13;
                                }
                                self.focus_actor = to;
                                self.z_target_active_timer = 15;
                                self.state2 &= !(STATE2_1 | STATE2_21);
                            } else if !hold {
                                self.release_lock_on();
                            }
                            self.state1 &= !STATE1_30;
                        }
                        None => {
                            if self.state1 & (STATE1_17 | STATE1_30) == 0 {
                                self.set_parallel();
                            }
                        }
                    }
                }
                if let Some(t) = self.focus_actor {
                    if env.target(t).is_none_or(|a| oot_game::target::lost(&env.data.target_ranges, a, true, self.actor.shape_rot.y, sp1c)) {
                        self.release_lock_on();
                        self.state1 |= STATE1_30;
                    }
                }
            }
            if let Some(t) = self.focus_actor {
                self.state1 &= !(STATE1_16 | STATE1_17);
                if self.state1 & STATE1_11 != 0 || !env.target(t).is_some_and(|a| a.is_hostile()) {
                    self.state1 |= STATE1_16;
                }
            } else if self.state1 & STATE1_17 != 0 {
                self.state2 &= !STATE2_13;
            } else {
                self.clear_z_targeting();
            }
        } else {
            self.clear_z_targeting();
        }
    }

    /// `func_8083DB98`: turn the head (and torso) towards the target's focus.
    fn func_8083DB98(&mut self, env: &Env, arg1: bool) -> i16 {
        let Some(t) = self.focus_actor.and_then(|t| env.target(t)) else { return self.actor.shape_rot.y };
        let from = Vec3::new(self.actor.world_pos.x, self.head_pos.y + 3.0, self.actor.world_pos.z);
        let pitch = oot_game::target::pitch_to(from, t.focus_pos);
        let yaw = oot_game::target::yaw_to(from, t.focus_pos);
        smooth_step_to_s(&mut self.actor.focus_rot.y, yaw, 4, 10000, 0);
        smooth_step_to_s(&mut self.actor.focus_rot.x, pitch, 4, 10000, 0);
        self.unk_6AE_rot_flags |= 2;
        self.func_80836AB8_arg(arg1)
    }

    /// `func_808334E4` / `func_80833528`: the locked-on wait animations (right / left foot forward).
    fn func_808334E4(&self, data: &GameData) -> AnimId {
        self.anim(data, group::TARGET_WAIT_R)
    }
    fn func_80833528(&self, data: &GameData) -> AnimId {
        self.anim(data, group::TARGET_WAIT_L)
    }

    /// `func_808401B0`: blend the two target waits by `unk_870`.
    fn func_808401B0(&mut self, data: &GameData) {
        let (a, b) = (self.func_808334E4(data), self.func_80833528(data));
        self.skel.blend_to_joint(data, a, self.unk_868, b, self.unk_868, self.unk_870);
    }

    /// `func_80840138`: which foot leads (`unk_874`), eased into `unk_870`.
    fn func_80840138(&mut self, speed: f32, yaw: i16) {
        let d = yaw.wrapping_sub(self.actor.shape_rot.y);
        if speed > 0.0 {
            self.unk_874 = if d < 0 { 0.0 } else { 1.0 };
        }
        step_to_f(&mut self.unk_870, self.unk_874, 0.3);
    }

    /// `func_8083FC68`: locked on, classify the stick: 1 run forward, -1 step back, 0 sidestep/stay.
    fn func_8083FC68(&mut self, env: &Env, speed: f32, yaw: i16) -> i32 {
        let sp1c = yaw.wrapping_sub(self.actor.shape_rot.y) as f32;
        if self.focus_actor.is_some() {
            let aiming = self.func_8002dd78() || self.func_808334b4();
            self.func_8083DB98(env, aiming);
        }
        let t = sp1c.abs() / 32768.0;
        if speed > (t * t) * 50.0 + 6.0 {
            1
        } else if speed > (1.0 - t) * 10.0 + 6.8 {
            -1
        } else {
            0
        }
    }

    /// `func_8083FD78`: the same for parallel mode (relative to `targetYaw`).
    fn func_8083FD78(&mut self, env: &Env, speed: &mut f32, yaw: &mut i16) -> i32 {
        let sp2e = yaw.wrapping_sub(self.target_yaw);
        let sp2c = (sp2e as i32).unsigned_abs() as u16;
        if (self.func_8002dd78() || self.func_808334b4()) && self.focus_actor.is_none() {
            // Aiming in parallel: only sideways, the focus pitched by the stick.
            *speed *= sin_s(sp2c as i16);
            if *speed != 0.0 {
                *yaw = ((if sp2e >= 0 { 1i32 } else { -1 }) << 0xE).wrapping_add(self.actor.shape_rot.y as i32) as i16;
            } else {
                *yaw = self.actor.shape_rot.y;
            }
            if self.focus_actor.is_some() {
                self.func_8083DB98(env, true);
            } else {
                let t = (self.input.rel.stick_y as f32 * 240.0) as i32 as i16;
                smooth_step_to_s(&mut self.actor.focus_rot.x, t, 14, 4000, 30);
                self.func_80836AB8_arg(true);
            }
        } else if self.focus_actor.is_some() {
            return self.func_8083FC68(env, *speed, *yaw);
        } else {
            self.func_8083DC54(env);
            if *speed != 0.0 && sp2c < 6000 {
                return 1;
            } else if *speed > sin_s((0x4000u16.wrapping_sub(sp2c >> 1)) as i16) * 200.0 {
                return -1;
            }
        }
        0
    }

    /// `func_8083CEAC`: lock on (standing).
    fn func_8083CEAC(&mut self, data: &GameData) {
        self.setup_action(data, Action::TargetIdle, 1);
        let a = self.anim(data, group::TARGET_ENTER);
        self.anim_change_once_morph(data, a);
        self.action_var2 = 1;
    }

    /// `func_80839F30`: parallel mode (standing).
    fn func_80839F30(&mut self, data: &GameData) {
        self.setup_action(data, Action::ParallelIdle, 1);
        let a = self.anim(data, group::WAIT);
        self.anim_change_once_morph(data, a);
        self.current_yaw = self.actor.shape_rot.y;
    }

    /// `func_80839E88`: back to the locked-on wait, keeping the leading foot.
    fn func_80839E88(&mut self, data: &GameData) {
        self.setup_action(data, Action::TargetIdle, 1);
        let anim = if self.unk_870 < 0.5 {
            self.unk_870 = 0.0;
            self.func_808334E4(data)
        } else {
            self.unk_870 = 1.0;
            self.func_80833528(data)
        };
        self.unk_874 = self.unk_870;
        self.skel.play_loop(data, anim);
        self.current_yaw = self.actor.shape_rot.y;
    }

    /// `func_80839F90`.
    fn func_80839F90(&mut self, data: &GameData) {
        if self.state1 & STATE1_4 != 0 {
            self.func_80839E88(data);
        } else if self.friendly_lock_on_or_parallel() {
            self.func_80839F30(data);
        } else {
            self.func_80853080(data);
        }
    }

    /// `func_8083CF10`: target lost while standing.
    fn func_8083CF10(&mut self, data: &GameData) {
        if self.linear_velocity != 0.0 {
            self.func_8083C858(data);
        } else {
            self.func_8083CE0C(data);
        }
    }

    /// `func_8083CE0C`: end the lock-on stance.
    fn func_8083CE0C(&mut self, data: &GameData) {
        self.setup_action(data, Action::StandingStill, 1);
        let a = if self.unk_870 < 0.5 { self.anim(data, group::TARGET_EXIT_L) } else { self.anim(data, group::TARGET_EXIT_R) };
        self.skel.play_once(data, a);
        self.current_yaw = self.actor.shape_rot.y;
    }

    /// `func_8083CBF0`: step back from the target.
    fn func_8083CBF0(&mut self, data: &GameData, yaw: i16) {
        self.setup_action(data, Action::TargetBackwalk, 1);
        let a = data.anim("link_anchor_back_walk");
        let last = data.anims[a].last_frame();
        self.skel.change(data, a, 2.2, 0.0, last, ANIMMODE_ONCE, -6.0);
        self.linear_velocity = 8.0;
        self.current_yaw = yaw;
    }

    /// `func_8083CC9C`: start sidestepping.
    fn func_8083CC9C(&mut self, data: &GameData) {
        self.setup_action(data, Action::Sidestep, 1);
        let a = self.anim(data, group::SIDESTEP_L);
        self.skel.change(data, a, 1.0, 0.0, 0.0, ANIMMODE_LOOP, -6.0);
        self.unk_868 = 0.0;
    }

    /// `func_8083CB2C`: start walking backwards in parallel mode.
    fn func_8083CB2C(&mut self, data: &GameData, yaw: i16) {
        self.setup_action(data, Action::ParallelBackwalk, 1);
        self.skel.copy_joint_to_morph();
        self.unk_864 = 0.0;
        self.unk_868 = 0.0;
        self.current_yaw = yaw;
    }

    /// `func_8083CB94`: start walking sideways in parallel mode.
    fn func_8083CB94(&mut self, data: &GameData) {
        self.setup_action(data, Action::ParallelWalk, 1);
        let a = self.anim(data, group::WALK);
        self.skel.change(data, a, 1.0, 0.0, 0.0, ANIMMODE_LOOP, -6.0);
    }

    /// `Player_SetupActionPreserveAnimMovement`: switch action keeping the animation-driven movement flags.
    fn setup_action_preserve_anim_movement(&mut self, data: &GameData, action: Action, flags: i32) {
        let t = self.skel.move_flags;
        self.skel.move_flags = 0;
        self.setup_action(data, action, flags);
        self.skel.move_flags = t;
    }

    /// `Player_Action_80840450`: locked on, standing (and turning to face the target).
    fn action_80840450(&mut self, env: &Env) {
        let data = env.data;
        // PLAYER_STATE3_3 (sword swing recovery): no melee weapon in hand yet.
        self.state3 &= !STATE3_3;
        if self.action_var2 != 0 {
            if self.skel.update(data) {
                self.finish_anim_movement();
                let a = self.func_808334E4(data);
                self.skel.play_loop(data, a);
                self.action_var2 = 0;
                self.state3 &= !STATE3_3;
            }
            self.func_80833C3C();
        } else {
            self.func_808401B0(data);
        }
        self.decelerate_to_zero();
        if !self.try_action_handler_list(env, Self::S_ACTION_HANDLER_LIST1, true) {
            // func_80834B5C (shield up) is never the upper-body action here.
            if !self.player_update_hostile_lock_on_env(env) && !self.friendly_lock_on_or_parallel() {
                self.func_8083CF10(data);
                return;
            }
            let (_, sp44, sp42) = self.get_movement_speed_and_yaw(env, 0.0);
            let t1 = self.func_8083FC68(env, sp44, sp42);
            if t1 > 0 {
                self.func_8083C8DC(data, sp42);
                return;
            }
            if t1 < 0 {
                self.func_8083CBF0(data, sp42);
                return;
            }
            if sp44 > 4.0 {
                self.func_8083CC9C(data);
                return;
            }
            self.func_8084029C(self.linear_velocity * 0.3 + 1.0);
            self.func_80840138(sp44, sp42);
            let t2 = self.unk_868 as u32;
            if t2 < 6 || t2.wrapping_sub(0xE) < 6 {
                step_to_f(&mut self.linear_velocity, 0.0, 1.5);
                return;
            }
            let t3 = sp42.wrapping_sub(self.current_yaw);
            let t4 = abs16(t3);
            if t4 > 0x4000 {
                if step_to_f(&mut self.linear_velocity, 0.0, 1.5) {
                    self.current_yaw = sp42;
                }
                return;
            }
            asym_step_to_f(&mut self.linear_velocity, sp44 * 0.3, 2.0, 1.5);
            if self.state3 & STATE3_3 == 0 {
                scaled_step_to_s(&mut self.current_yaw, sp42, (t4 as f32 * 0.1) as i16);
            }
        }
    }

    /// `Player_Action_808407CC`: parallel mode, standing.
    fn action_808407cc(&mut self, env: &Env) {
        let data = env.data;
        if self.skel.update(data) {
            self.finish_anim_movement();
            let a = self.anim(data, group::WAIT);
            self.skel.play_once(data, a);
        }
        self.decelerate_to_zero();
        if !self.try_action_handler_list(env, Self::S_ACTION_HANDLER_LIST2, true) {
            if self.player_update_hostile_lock_on_env(env) {
                self.func_8083CEAC(data);
                return;
            }
            if !self.friendly_lock_on_or_parallel() {
                self.setup_action_preserve_anim_movement(data, Action::StandingStill, 1);
                self.current_yaw = self.actor.shape_rot.y;
                return;
            }
            let (_, mut sp3c, mut sp3a) = self.get_movement_speed_and_yaw(env, 0.0);
            let t1 = self.func_8083FD78(env, &mut sp3c, &mut sp3a);
            if t1 > 0 {
                self.func_8083C8DC(data, sp3a);
                return;
            }
            if t1 < 0 {
                self.func_8083CB2C(data, sp3a);
                return;
            }
            if sp3c > 4.9 {
                self.func_8083CC9C(data);
                self.func_80833C3C();
                return;
            }
            if sp3c != 0.0 {
                self.func_8083CB94(data);
                return;
            }
            let t2 = sp3a.wrapping_sub(self.actor.shape_rot.y);
            if abs16(t2) > 800 {
                self.setup_turn_in_place(data, sp3a);
            }
        }
    }

    /// `Player_Action_8084227C`: running forward while targeting.
    fn action_8084227c(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5;
        self.func_80841EE4(data);
        if !self.try_action_handler_list(env, Self::S_ACTION_HANDLER_LIST9, true) {
            if !self.is_z_targeting_with_hostile_update(env) {
                self.func_8083C858(data);
                return;
            }
            let (_, mut sp2c, mut sp2a) = self.get_movement_speed_and_yaw(env, 0.0);
            if !self.func_8083C484(&mut sp2c, &mut sp2a) {
                let stop = if self.friendly_lock_on_or_parallel() {
                    sp2c != 0.0 && self.func_8083FD78(env, &mut sp2c, &mut sp2a) <= 0
                } else {
                    self.func_8083FC68(env, sp2c, sp2a) <= 0
                };
                if stop {
                    self.func_80839F90(data);
                    return;
                }
                self.func_8083DF68(sp2c, sp2a);
                self.func_8083DDC8(env);
                if self.linear_velocity == 0.0 && sp2c == 0.0 {
                    self.func_80839F90(data);
                }
            }
        }
    }

    /// `func_80841860`: sidestep animation, the two sidestep cycles blended by `unk_870`.
    fn func_80841860(&mut self, data: &GameData) {
        let a = self.anim(data, group::SIDESTEP_R);
        let b = self.anim(data, group::SIDESTEP_L);
        self.skel.animation = a;
        let r = |n| self.regs.reg(n) as f32 / 1000.0;
        self.func_8084029C(r(30) + r(32) * self.linear_velocity);
        let frame = self.unk_868 * (16.0 / 29.0);
        self.skel.blend_to_joint(data, b, frame, a, frame, self.unk_870);
    }

    /// `Player_Action_8084193C`: sidestepping.
    fn action_8084193c(&mut self, env: &Env) {
        let data = env.data;
        self.func_80841860(data);
        if !self.try_action_handler_list(env, Self::S_ACTION_HANDLER_LIST5, true) {
            if !self.is_z_targeting_with_hostile_update(env) {
                self.func_8083C858(data);
                return;
            }
            let (_, mut sp3c, mut sp3a) = self.get_movement_speed_and_yaw(env, 0.0);
            let t1 = if self.friendly_lock_on_or_parallel() { self.func_8083FD78(env, &mut sp3c, &mut sp3a) } else { self.func_8083FC68(env, sp3c, sp3a) };
            if t1 > 0 {
                self.func_8083C858(data);
                return;
            }
            if t1 < 0 {
                if self.friendly_lock_on_or_parallel() {
                    self.func_8083CB2C(data, sp3a);
                } else {
                    self.func_8083CBF0(data, sp3a);
                }
                return;
            }
            if self.linear_velocity < 3.6 && sp3c < 4.0 {
                if self.state1 & STATE1_4 == 0 && self.friendly_lock_on_or_parallel() {
                    self.func_8083CB94(data);
                } else {
                    self.func_80839F90(data);
                }
                return;
            }
            self.func_80840138(sp3c, sp3a);
            let t2 = sp3a.wrapping_sub(self.current_yaw);
            let t3 = abs16(t2);
            if t3 > 0x4000 {
                if step_to_f(&mut self.linear_velocity, 0.0, 3.0) {
                    self.current_yaw = sp3a;
                }
                return;
            }
            asym_step_to_f(&mut self.linear_velocity, sp3c * 0.9, 2.0, 3.0);
            scaled_step_to_s(&mut self.current_yaw, sp3a, (t3 as f32 * 0.1) as i16);
        }
    }

    /// `Player_Action_808423EC`: the step back from a locked target.
    fn action_808423ec(&mut self, env: &Env) {
        let data = env.data;
        let sp34 = self.skel.update(data);
        if !self.try_action_handler_list(env, Self::S_ACTION_HANDLER_LIST5, true) {
            if !self.is_z_targeting_with_hostile_update(env) {
                self.func_8083C858(data);
                return;
            }
            let (_, sp30, sp2e) = self.get_movement_speed_and_yaw(env, 0.0);
            if self.skel.morph_weight == 0.0 && self.skel.cur_frame > 5.0 {
                self.decelerate_to_zero();
                if self.skel.cur_frame > 10.0 && self.func_8083FC68(env, sp30, sp2e) < 0 {
                    self.func_8083CBF0(data, sp2e);
                    return;
                }
                if sp34 {
                    // func_8083CD00.
                    self.setup_action(data, Action::TargetBackBrake, 1);
                    let a = data.anim("link_anchor_back_brake");
                    self.skel.play_once_set_speed(data, a, 2.0);
                }
            }
        }
    }

    /// `Player_Action_8084251C`: braking after the step back.
    fn action_8084251c(&mut self, env: &Env) {
        let data = env.data;
        let sp34 = self.skel.update(data);
        self.decelerate_to_zero();
        if !self.try_action_handler_list(env, Self::S_ACTION_HANDLER_LIST10, true) {
            let (_, sp30, sp2e) = self.get_movement_speed_and_yaw(env, 0.0);
            if self.linear_velocity == 0.0 {
                self.current_yaw = self.actor.shape_rot.y;
                if self.func_8083FC68(env, sp30, sp2e) > 0 {
                    self.func_8083C858(data);
                    return;
                }
                if sp30 != 0.0 || sp34 {
                    self.func_80839F90(data);
                }
            }
        }
    }

    /// `Player_Action_80840DE4`: walking sideways in parallel mode. The side-walk cycle plays at a speed set
    /// by `linearVelocity × MREG(95) / 100`, signed by which way Player moves.
    fn action_80840de4(&mut self, env: &Env) {
        let data = env.data;
        self.skel.mode = ANIMMODE_LOOP;
        self.skel.animation = self.func_8083356c(data);
        let (frames, coeff) = (29.0, self.regs.mreg(95) as f32 / 100.0);
        self.skel.anim_length = frames;
        self.skel.end_frame = frames - 1.0;
        let dir = if self.current_yaw.wrapping_sub(self.actor.shape_rot.y) >= 0 { 1.0 } else { -1.0 };
        self.skel.play_speed = dir * (self.linear_velocity * coeff);
        self.skel.update(data);
        if self.skel.on_frame(0.0) || self.skel.on_frame(frames * 0.5) {
            let v = self.linear_velocity;
            self.play_stepping_sfx(v);
        }
        if !self.try_action_handler_list(env, Self::S_ACTION_HANDLER_LIST3, true) {
            if self.player_update_hostile_lock_on_env(env) {
                self.func_8083CEAC(data);
                return;
            }
            if !self.friendly_lock_on_or_parallel() {
                self.func_80853080(data);
                return;
            }
            let (_, mut sp44, mut sp42) = self.get_movement_speed_and_yaw(env, 0.0);
            let t1 = self.func_8083FD78(env, &mut sp44, &mut sp42);
            if t1 > 0 {
                self.func_8083C8DC(data, sp42);
                return;
            }
            if t1 < 0 {
                self.func_8083CB2C(data, sp42);
                return;
            }
            if sp44 > 4.9 {
                self.func_8083CC9C(data);
                self.func_80833C3C();
                return;
            }
            if sp44 == 0.0 && self.linear_velocity == 0.0 {
                self.func_80839F30(data);
                return;
            }
            let t2 = sp42.wrapping_sub(self.current_yaw);
            let t3 = abs16(t2);
            if t3 > 0x4000 {
                if step_to_f(&mut self.linear_velocity, 0.0, 1.5) {
                    self.current_yaw = sp42;
                }
                return;
            }
            asym_step_to_f(&mut self.linear_velocity, sp44 * 0.4, 1.5, 1.5);
            scaled_step_to_s(&mut self.current_yaw, sp42, (t3 as f32 * 0.1) as i16);
        }
    }

    /// `func_80841138`: parallel back walk/run animation (like `func_80841EE4` with the back cycles).
    fn func_80841138(&mut self, data: &GameData) {
        let r = |n| self.regs.reg(n) as f32 / 1000.0;
        let back = self.anim(data, group::PARALLEL_BACKWALK);
        let temp1;
        if self.unk_864 < 1.0 {
            self.func_8084029C(r(35));
            self.skel.load_to_joint(data, back, self.unk_868);
            self.unk_864 = (self.unk_864 + UPDATE_SCALE).min(1.0);
            temp1 = self.unk_864;
        } else {
            let temp2 = self.linear_velocity - (self.regs.reg(48) as f32 / 100.0);
            if temp2 < 0.0 {
                temp1 = 1.0;
                self.func_8084029C(r(35) + r(36) * self.linear_velocity);
                self.skel.load_to_joint(data, back, self.unk_868);
            } else {
                let mut t = r(37) * temp2;
                if t < 1.0 {
                    self.func_8084029C(r(35) + r(36) * self.linear_velocity);
                } else {
                    t = 1.0;
                    self.func_8084029C(1.2 + r(38) * temp2);
                }
                temp1 = t;
                self.skel.load_to_morph(data, back, self.unk_868);
                let run = data.anim("link_normal_back_run");
                self.skel.load_to_joint(data, run, self.unk_868 * (16.0 / 29.0));
            }
        }
        if temp1 < 1.0 {
            self.skel.interp_joint_morph(1.0 - temp1);
        }
    }

    /// `Player_Action_808414F8`: walking/running backwards in parallel mode.
    fn action_808414f8(&mut self, env: &Env) {
        let data = env.data;
        self.func_80841138(data);
        if !self.try_action_handler_list(env, Self::S_ACTION_HANDLER_LIST4, true) {
            if !self.is_z_targeting_with_hostile_update(env) {
                let y = self.current_yaw;
                self.func_8083C8DC(data, y);
                return;
            }
            let (_, mut sp34, mut sp32) = self.get_movement_speed_and_yaw(env, 0.0);
            let sp2c = self.func_8083FD78(env, &mut sp34, &mut sp32);
            if sp2c >= 0 {
                // func_80841458.
                let handled = if self.linear_velocity > 6.0 {
                    // func_8084140C.
                    self.setup_action(data, Action::ParallelBackBrake, 1);
                    let a = data.anim("link_normal_back_brake");
                    self.anim_change_once_morph(data, a);
                    true
                } else if sp34 != 0.0 {
                    if self.decelerate_to_zero() {
                        // func_80841458 also resets *arg2 to currentYaw; it isn't read again.
                        sp34 = 0.0;
                        false
                    } else {
                        true
                    }
                } else {
                    false
                };
                if !handled {
                    if sp2c != 0 {
                        self.func_8083C858(data);
                    } else if sp34 > 4.9 {
                        self.func_8083CC9C(data);
                    } else {
                        self.func_8083CB94(data);
                    }
                }
            } else {
                let sp2a = sp32.wrapping_sub(self.current_yaw);
                asym_step_to_f(&mut self.linear_velocity, sp34 * 1.5, 1.5, 2.0);
                scaled_step_to_s(&mut self.current_yaw, sp32, (sp2a as f32 * 0.1) as i16);
                if sp34 == 0.0 && self.linear_velocity == 0.0 {
                    self.func_80839F30(data);
                }
            }
        }
    }

    /// `Player_Action_8084170C`: braking out of a parallel back run.
    fn action_8084170c(&mut self, env: &Env) {
        let data = env.data;
        let sp34 = self.skel.update(data);
        self.decelerate_to_zero();
        if !self.try_action_handler_list(env, Self::S_ACTION_HANDLER_LIST4, true) {
            let (_, mut sp30, mut sp2e) = self.get_movement_speed_and_yaw(env, 0.0);
            if self.linear_velocity == 0.0 {
                self.current_yaw = self.actor.shape_rot.y;
                if self.func_8083FD78(env, &mut sp30, &mut sp2e) > 0 {
                    self.func_8083C858(data);
                } else if sp30 != 0.0 || sp34 {
                    // func_808416C0.
                    self.setup_action(data, Action::ParallelBackBrakeEnd, 1);
                    let a = data.anim("link_normal_back_brake_end");
                    self.skel.play_once(data, a);
                }
            }
        }
    }

    /// `Player_Action_808417FC`: end of the parallel back brake.
    fn action_808417fc(&mut self, env: &Env) {
        let data = env.data;
        let sp1c = self.skel.update(data);
        if !self.try_action_handler_list(env, Self::S_ACTION_HANDLER_LIST4, true) && sp1c {
            self.func_80839F30(data);
        }
    }

    /// `Player_ActionHandler_10` (interrupt 10): A while targeting: a side hop or backflip away from the
    /// stick's forward, or a roll forward (a jump slash with a sword: not ported).
    fn action_handler_10(&mut self, env: &Env) -> bool {
        let floor_effect = self.actor.floor_poly.map(|p| env.col.floor_effect(p)).unwrap_or(0);
        // Room behaviour type 1 is 0 here (not ROOM_TYPE_INDOORS).
        if self.input.press.held(BTN_A) && self.s.floor_type != FLOOR_TYPE_7 && floor_effect != 1 {
            let sp2c = self.stick_dir();
            if sp2c <= 0 {
                if self.is_z_targeting() {
                    self.setup_roll(env.data);
                    return true;
                }
            } else {
                self.func_8083BCD0(env.data, sp2c);
                return true;
            }
        }
        false
    }

    /// `func_8083BCD0`: hop in direction `arg2` (1 left, 2 back = backflip, 3 right).
    fn func_8083BCD0(&mut self, data: &GameData, arg2: i8) {
        let side = arg2 & 1 != 0;
        let a = data.side_hop_anims[arg2 as usize][0];
        self.func_80838940(data, Some(a), if !side { 5.8 } else { 3.5 }, NA_SE_VO_LI_SWORD_N);
        self.action_var2 = 1;
        self.action_var1 = arg2;
        self.current_yaw = self.actor.shape_rot.y.wrapping_add(((arg2 as i32) << 14) as i16);
        self.linear_velocity = if !side { 6.0 } else { 8.5 };
        self.state2 |= STATE2_19;
        self.play_sfx(if ((arg2 as i32) << 0xE) == 0x8000 { NA_SE_PL_ROLL } else { NA_SE_PL_SKIP });
    }

    // ================================================================================
    // Items and the sword (Player_UpdateUpperBody, Player_UseItem, Player_Action_808502D0)

    /// `Player_CheckForIdleAnim`: -1 if playing the wait, 1 + index if an idle fidget (`sFidgetAnimations`), else 0.
    fn check_for_idle_anim(&self, data: &GameData) -> i32 {
        if self.anim(data, group::WAIT) == self.skel.animation {
            return -1;
        }
        for (i, a) in data.idle_variants.iter().flat_map(|p| p.iter()).enumerate() {
            if *a == self.skel.animation {
                return i as i32 + 1;
            }
        }
        0
    }

    /// `Player_UpdateUpperBody`: run the upper-body action; while it's active, its animation replaces the
    /// upper body (`sUpperBodyLimbCopyMap`), or the whole body when standing on the wait/fidget.
    fn update_upper_body(&mut self, env: &Env) -> bool {
        let data = env.data;
        // Hookshot flight: not held.
        if self.can_update_items(data) {
            self.update_items(env);
            if self.action == Action::ThrowNut {
                return true;
            }
        }
        if !self.run_upper(env) {
            return false;
        }
        // upperAnimInterpWeight (blend back out of skelAnime2) is only ever set to 0 in this decomp.
        if self.check_for_idle_anim(data) == 0 || self.linear_velocity != 0.0 {
            let mask = data.items.upper_body;
            let src = self.skel2.joint;
            self.skel.request_copy_external(&src, Some(mask));
        } else {
            let src = self.skel2.joint;
            self.skel.request_copy_external(&src, None);
        }
        true
    }

    /// `Player_CanUpdateItems`: the item system may run: not waiting for an item to be put away
    /// (unless a change to the cutscene's sword or to nothing is starting), and no change playing
    /// to an item not yet in hand.
    fn can_update_items(&self, data: &GameData) -> bool {
        use oot_game::item::ITEM_NONE;
        /// `ITEM_SWORD_CS` (`item.h`).
        const ITEM_SWORD_CS: u8 = 0xFC;
        (self.action != Action::ItemPutAway || (self.state1 & STATE1_8 != 0 && (self.held_item_id == ITEM_SWORD_CS || self.held_item_id == ITEM_NONE)))
            && (self.upper != UpperAction::Change || data.items.item_to_action_param(self.held_item_id) == self.held_item_ap)
    }

    fn run_upper(&mut self, env: &Env) -> bool {
        match self.upper {
            // func_8083485C.
            UpperAction::Default => self.func_80834758(env),
            // Player_UpperAction_Sword: func_80834758, then func_8083499C.
            UpperAction::Sword => {
                if self.func_80834758(env) {
                    true
                } else if self.state1 & STATE1_8 != 0 {
                    self.start_changing_held_item(env.data);
                    true
                } else {
                    false
                }
            }
            UpperAction::Change => self.upper_action_change_held_item(env),
            UpperAction::ShieldUp => self.func_80834b5c(env),
            UpperAction::ShieldHit => self.func_80834bd4(env),
            UpperAction::ShieldDown => self.func_80834c74(env),
            UpperAction::Bow => self.func_8083501c(env),
            UpperAction::BowDrawn => self.func_808351d4(env),
            UpperAction::BowShot => self.func_808353d8(env),
            UpperAction::BowLower => self.func_80835588(env),
        }
    }

    /// `sItemActionUpdateFuncs[actionParam]`: the upper-body action for a held item.
    /// The bombs' `Player_UpperAction_CarryActor` (carrying isn't ported: Player holds no bomb) and the
    /// boomerang's `func_80835800` (not ported) run `func_8083485C`'s default here.
    fn upper_for(&self, data: &GameData, ap: i32) -> UpperAction {
        let it = &data.items;
        if (it.ap("SWORD_MASTER")..=it.ap("SWORD_BIGGORON")).contains(&ap) {
            UpperAction::Sword
        } else if (it.ap("BOW")..=it.ap("LONGSHOT")).contains(&ap) {
            UpperAction::Bow
        } else {
            UpperAction::Default
        }
    }

    /// `Player_SetUpperActionFunc`.
    fn set_upper_action_func(&mut self, f: UpperAction) {
        self.upper = f;
        self.unk_836 = 0;
        self.upper_anim_interp_weight = 0.0;
        self.func_808326F0();
    }

    /// `func_808326F0`: the idle fidgets' voices stop (`D_8085361C`, the age's bank).
    fn func_808326F0(&mut self) {
        for id in [NA_SE_VO_LI_SWEAT, NA_SE_VO_LI_SNEEZE, NA_SE_VO_LI_RELAX, NA_SE_VO_LI_FALL_L] {
            self.sfx(PlayerSfx::StopById(id.wrapping_add(self.age.climb.unk_92)));
        }
    }

    /// `Player_UpdateItems`: the item buttons (`Player_ProcessItemButtons`) while Player may use
    /// them: Player's own category, no change starting, the held item the one in use (or the
    /// shield up), alive, no cutscene, no cutscene action, the main camera active, no exit
    /// starting; then a change pending starts. (`shootingGalleryStatus` is 0 and
    /// `timerState` never `TIMER_STATE_STOP`: no shooting gallery or timer is ported.)
    fn update_items(&mut self, env: &Env) {
        let (health, trigger) = {
            let io = env.io.borrow();
            (io.save.health, io.transition.trigger)
        };
        if self.actor.category == ACTORCAT_PLAYER
            && self.state1 & STATE1_8 == 0
            && (self.held_item_ap == self.item_ap || self.state1 & STATE1_22 != 0)
            && health != 0
            && env.cs_state == oot_game::cutscene::CS_STATE_IDLE
            && self.cs_mode == 0
            && env.active_cam_id == CAM_ID_MAIN
            && trigger != TRANS_TRIGGER_START
        {
            self.process_item_buttons(env);
        }
        if self.state1 & STATE1_8 != 0 {
            self.start_changing_held_item(env.data);
        }
    }

    /// `Player_ItemIsInUse`: `item` is the one in use (`itemAction`).
    fn item_is_in_use(&self, data: &GameData, item: u8) -> bool {
        item < oot_game::item::ITEM_NONE_FE && data.items.item_to_action_param(item) == self.item_ap
    }

    /// `Player_ItemIsItemAction`.
    fn item_is_item_action(data: &GameData, item1: u8, item_action: i32) -> bool {
        item1 < oot_game::item::ITEM_NONE_FE && data.items.item_to_action_param(item1) == item_action
    }

    /// `Player_GetItemOnButton`: B's item and the three C buttons' (`B_BTN_ITEM`, `C_BTN_ITEM`),
    /// `ITEM_NONE` past them (no button pressed). (`bombchuBowlingStatus` is 0: the bowling alley
    /// isn't ported.)
    fn get_item_on_button(buttons: &[u8; 4], index: usize) -> u8 {
        buttons.get(index).copied().unwrap_or(oot_game::item::ITEM_NONE)
    }

    /// `Player_ProcessItemButtons`: a mask no C button has any more comes off; an item in use
    /// (from the fishing rod on) that no button has is put away; then the first of B, C-Left,
    /// C-Down and C-Right pressed (`sItemButtons`) uses its item, or with none pressed, a button
    /// held down with the item in hand marks it held (`sHeldItemButtonIsHeldDown`).
    fn process_item_buttons(&mut self, env: &Env) {
        use oot_game::item::{ITEM_NONE, ITEM_NONE_FE};
        let data = env.data;
        let buttons = {
            let io = env.io.borrow();
            [io.save.b_btn_item(), io.save.c_btn_item(0), io.save.c_btn_item(1), io.save.c_btn_item(2)]
        };
        if self.current_mask != PLAYER_MASK_NONE {
            let mask_item_action = self.current_mask as i32 - 1 + data.items.ap("MASK_KEATON");
            if !buttons[1..].iter().any(|&b| Self::item_is_item_action(data, b, mask_item_action)) {
                self.current_mask = PLAYER_MASK_NONE;
            }
        }
        if self.state1 & (STATE1_11 | STATE1_29) != 0 || self.func_8008f128(data) {
            return;
        }
        if self.item_ap >= data.items.ap("FISHING_POLE") && !buttons.iter().any(|&b| self.item_is_in_use(data, b)) {
            self.use_item(data, ITEM_NONE);
            return;
        }
        let i = S_ITEM_BUTTONS.iter().position(|&b| self.input.press.held(b)).unwrap_or(S_ITEM_BUTTONS.len());
        let item = Self::get_item_on_button(&buttons, i);
        if item >= ITEM_NONE_FE {
            let i = S_ITEM_BUTTONS.iter().position(|&b| self.input.cur.held(b)).unwrap_or(S_ITEM_BUTTONS.len());
            let item = Self::get_item_on_button(&buttons, i);
            if item < ITEM_NONE_FE && data.items.item_to_action_param(item) == self.held_item_ap {
                self.s.held_item_button_is_held_down = true;
            }
        } else {
            self.held_item_button = i as i8;
            self.use_item(data, item);
        }
    }

    // ================================================================================
    // The bow and the slingshot (func_80834D2C to func_80835588), first person
    // (func_8083B8F4, func_8083AD4C, Player_Action_8084B1D8), the Deku nut (func_8083C61C,
    // Player_Action_8084E604)

    /// `func_8002DD6C` (`z_actor.c`): the bow, the slingshot or the hookshot in hand
    /// (`PLAYER_STATE1_3`, from `Player_InitBowOrSlingshotIA` or `Player_InitHookshotIA`).
    fn func_8002dd6c(&self) -> bool {
        self.state1 & STATE1_3 != 0
    }

    /// `func_8002DD78` (`z_actor.c`): and raised (`unk_834`).
    fn func_8002dd78(&self) -> bool {
        self.func_8002dd6c() && self.unk_834 != 0
    }

    /// `func_808332E4`: the boomerang in hand (`PLAYER_STATE1_USING_BOOMERANG`).
    fn func_808332e4(&self) -> bool {
        self.state1 & STATE1_24 != 0
    }

    /// `func_808334B4`: and raised.
    fn func_808334b4(&self) -> bool {
        self.func_808332e4() && self.unk_834 != 0
    }

    /// `func_8083356C`: walking sideways in parallel mode, the bow's side walk while aiming.
    fn func_8083356c(&self, data: &GameData) -> AnimId {
        if self.func_8002dd78() { data.anim("link_bow_side_walk") } else { self.anim(data, group::PARALLEL_SIDEWALK) }
    }

    /// `func_80832564`: `func_80832440`, and `Player_DetachHeldActor`.
    fn func_80832564(&mut self, data: &GameData) {
        self.func_80832440();
        self.detach_held_actor(data);
    }

    /// `func_8084FF7C` (`Player_UpdateCommon`, with the bow, slingshot or hookshot in hand): the
    /// string's spring after a shot: `unk_858` its stretch, `unk_85C` its speed, damped by 0.3.
    fn func_8084ff7c(&mut self) {
        self.unk_858 += self.unk_85c;
        self.unk_85c -= self.unk_858 * 5.0;
        self.unk_85c *= 0.3;
        if self.unk_85c.abs() < 0.00001 {
            self.unk_85c = 0.0;
            if self.unk_858.abs() < 0.00001 {
                self.unk_858 = 0.0;
            }
        }
    }

    /// `Camera_CheckValidMode(Play_GetCamera(play, CAM_ID_MAIN), mode)`: 0 if the main camera's
    /// setting has no such mode, -1 if it's already in it, else `mode | 0x80000000`. (The camera
    /// as this update began, with Player's own requests this update made.)
    fn camera_check_valid_mode(&self, env: &Env, mode: i16) -> i32 {
        let cur = self.s.cam_mode.unwrap_or(env.main_cam_mode);
        if env.main_cam_valid_modes & (1u32 << mode) == 0 {
            0
        } else if mode == cur {
            -1
        } else {
            (0x8000_0000u32 | mode as u32) as i32
        }
    }

    /// `Camera_RequestMode(Play_GetCamera(play, CAM_ID_MAIN), mode)`: what
    /// `Camera_RequestModeImpl` returns (-1 locked or unchanged; refused, `CAM_MODE_NORMAL` or,
    /// from another mode, `0xC0000000 | mode` with the camera forced to normal; else
    /// `0x80000000 | mode`). The request itself is made on the camera after Player's update, in
    /// its place among his requests (`PlayRequest::CamRequestMode`), with the camera's sounds.
    fn camera_request_mode(&mut self, env: &Env, mode: i16) -> i32 {
        use oot_game::camera::CAM_MODE_NORMAL;
        let cur = self.s.cam_mode.unwrap_or(env.main_cam_mode);
        self.play_requests.push(PlayRequest::CamRequestMode(mode));
        if env.cam_state_flags & CAM_STATE_LOCK_MODE != 0 {
            return -1;
        }
        if (env.main_cam_valid_modes & 0x3FFF_FFFF) & (1u32 << mode) == 0 {
            if cur != CAM_MODE_NORMAL {
                self.s.cam_mode = Some(CAM_MODE_NORMAL);
                return (0xC000_0000u32 | mode as u32) as i32;
            }
            return CAM_MODE_NORMAL as i32;
        }
        if mode == cur {
            return -1;
        }
        self.s.cam_mode = Some(mode);
        (0x8000_0000u32 | mode as u32) as i32
    }

    /// `func_80834D2C`: the item raised: the bow's (or hookshot's) ready on the upper body, if
    /// `func_8083442C` draws it; the boomerang's own wait (`func_80835884`: the boomerang's actions
    /// aren't ported, they log). Standing, the body waits (riding: `link_uma_anim_walk`).
    fn func_80834d2c(&mut self, env: &Env) -> bool {
        let data = env.data;
        if self.held_item_ap != data.items.ap("BOOMERANG") {
            if !self.func_8083442c(env) {
                return false;
            }
            let anim = data.anim(if !self.holds_hookshot(data) { "link_bow_bow_ready" } else { "link_hook_shot_ready" });
            self.skel2.play_once(data, anim);
        } else {
            self.note("func_80834D2C: the boomerang's wait (func_80835884) isn't ported");
            self.unk_834 = 10;
            let a = data.anim("link_boom_throw_wait2waitR");
            self.skel2.play_once(data, a);
        }
        if self.state1 & STATE1_23 != 0 {
            let a = data.anim("link_uma_anim_walk");
            self.skel.play_loop(data, a);
        } else if self.grounded() && !self.update_hostile_lock_on() {
            let a = self.anim(data, group::WAIT);
            self.skel.play_loop(data, a);
        }
        true
    }

    /// `func_80834E44`: the shooting gallery's B (`shootingGalleryStatus > 0`): no gallery is
    /// ported, so never.
    fn func_80834e44(&self) -> bool {
        false
    }

    /// `func_80834E7C`: the shooting gallery's buttons held (`shootingGalleryStatus != 0`): never.
    fn func_80834e7c(&self) -> bool {
        false
    }

    /// `func_80834EB8`: raised, stay in third person while Z-targeting (or where the main
    /// camera's setting has no aim, `CAM_MODE_AIM_ADULT` for either age); else first person
    /// (`unk_6AD` 2). Returns whether it stays in third person.
    fn func_80834eb8(&mut self, env: &Env) -> bool {
        if self.unk_6AD == 0 || self.unk_6AD == 2 {
            if self.is_z_targeting() || self.camera_check_valid_mode(env, oot_game::camera::CAM_MODE_AIM_ADULT) == 0 {
                return true;
            }
            self.unk_6AD = 2;
        }
        false
    }

    /// `func_80834F2C`: the item's button pressed again (`sUseHeldItem`), no door ahead and no
    /// boomerang in flight: raise it (`func_80834D2C`), then `func_80834EB8`.
    fn func_80834f2c(&mut self, env: &Env) -> bool {
        if self.door_type == PLAYER_DOORTYPE_NONE && self.state1 & STATE1_25 == 0 && (self.s.use_held_item || self.func_80834e44()) && self.func_80834d2c(env) {
            return self.func_80834eb8(env);
        }
        false
    }

    /// `func_80834FBC`: the hookshot's hook back on it (`actor.child`): held again, with its sound.
    fn func_80834fbc(&mut self) -> bool {
        if self.actor.child.is_some() {
            if self.held_actor.is_none() {
                self.held_actor = self.actor.child;
                // (Player_RequestRumble(this, 255, 10, 250, 0): the rumble isn't ported.)
                self.play_sfx(NA_SE_IT_HOOKSHOT_RECEIVE);
            }
            return true;
        }
        false
    }

    /// `func_8083501C` (`sItemActionUpdateFuncs` for the bow, the slingshot and the hookshot):
    /// lowered; the shield (`func_80834758`) or the item raised (`func_80834F2C`) takes the upper
    /// body. `unk_860` is made positive (a new shot may draw).
    fn func_8083501c(&mut self, env: &Env) -> bool {
        if self.unk_860 >= 0 {
            self.unk_860 = -self.unk_860;
        }
        if (!self.holds_hookshot(env.data) || self.func_80834fbc()) && !self.func_80834758(env) && !self.func_80834f2c(env) {
            return false;
        }
        true
    }

    /// `func_80834380`: the ammo's item and the arrow type for what's in hand (the adult's bow, by
    /// its arrows' kind, a plain arrow on a horse; the child's slingshot, a seed), and the ammo
    /// left (`minigameState` and `shootingGalleryStatus` are 0: no horseback archery or
    /// gallery is ported).
    fn func_80834380(&self, data: &GameData) -> (i32, u8, i16) {
        use oot_game::item::{ITEM_BOW, ITEM_SLINGSHOT};
        let (item, ty) = if self.adult {
            (ITEM_BOW, if self.state1 & STATE1_23 != 0 { ARROW_NORMAL_HORSE } else { ARROW_NORMAL + (self.held_item_ap - data.items.ap("BOW")) as i16 })
        } else {
            (ITEM_SLINGSHOT, ARROW_SEED)
        };
        (self.ammo(item) as i32, item, ty)
    }

    /// `func_8083442C`: draw the string (`func_808351D4`, `PLAYER_STATE1_9`, `unk_834` 14) with its
    /// sound (`D_80854398`), a seed or arrow in hand if there's ammo (`En_Arrow`, Player's child:
    /// `PlayRequest::SpawnHeldArrow`). A magic arrow with the magic busy gives the error instead
    /// (`gSaveContext.magicState` is idle: magic isn't ported); a magic arrow's cost
    /// (`Magic_RequestChange`, `sMagicArrowCosts`) logs and the arrow is a plain one.
    fn func_8083442c(&mut self, env: &Env) -> bool {
        let data = env.data;
        // (heldItemAction BOW_FIRE..BOW_0E with gSaveContext.magicState != MAGIC_STATE_IDLE: the
        // error. The magic meter isn't ported: always idle.)
        self.set_upper_action_func(UpperAction::BowDrawn);
        self.state1 |= STATE1_9;
        self.unk_834 = 14;
        if self.unk_860 >= 0 {
            self.play_sfx(D_80854398[(self.unk_860.unsigned_abs() as usize).saturating_sub(1).min(2)]);
            let (ammo, _item, mut arrow_type) = self.func_80834380(data);
            if !self.holds_hookshot(data) && ammo > 0 {
                let magic_arrow_type = arrow_type - ARROW_FIRE;
                if self.unk_860 >= 0 {
                    if (0..=2).contains(&magic_arrow_type) {
                        self.note("func_8083442C: a magic arrow's cost (Magic_RequestChange) isn't ported: a plain arrow");
                        arrow_type = ARROW_NORMAL;
                    }
                    self.play_requests.push(PlayRequest::SpawnHeldArrow { pos: self.actor.world_pos, yaw: self.actor.shape_rot.y, params: arrow_type });
                }
            }
        }
        true
    }

    /// `func_808350A4`: the shot: the seed or arrow in hand let go (`unk_A73` 4, its parent
    /// cleared), one less (`Inventory_ChangeAmmo`; the rumble isn't ported). False with nothing in
    /// hand.
    fn func_808350a4(&mut self, env: &Env) -> bool {
        let Some(h) = self.held_actor else { return false };
        if !self.holds_hookshot(env.data) {
            let (_, item, _) = self.func_80834380(env.data);
            // (minigameState 1: hbaAmmo; shootingGalleryStatus: the gallery's count. Neither is
            // ported.)
            self.change_ammo(env, item, -1);
            // (Player_RequestRumble(this, 150, 10, 150, 0).)
        }
        // (The hookshot's Player_RequestRumble(this, 255, 20, 150, 0).)
        self.unk_A73 = 4;
        self.play_requests.push(PlayRequest::ReleaseHeld(h));
        self.actor.child = None;
        self.held_actor = None;
        true
    }

    /// `func_808351D4`: raised and drawn: the upper body rolls to 1200; the raise from the side
    /// walk, then the wait loop (`unk_836` 1, then 2). Let go (the button up, or `unk_860`
    /// negative) once in the wait: the shot (`func_808353D8`; nothing in hand, the flick's sound).
    fn func_808351d4(&mut self, env: &Env) -> bool {
        let data = env.data;
        let sp2c = if !self.holds_hookshot(data) { 0 } else { 1 };
        scaled_step_to_s(&mut self.upper_limb_rot_z, 1200, 400);
        self.unk_6AE_rot_flags |= UNK6AE_ROT_UPPER_Z;
        if self.unk_836 == 0 && self.check_for_idle_anim(data) == 0 && self.skel.animation == data.anim("link_bow_side_walk") {
            let a = data.anim(D_808543CC[sp2c]);
            self.skel2.play_once(data, a);
            self.unk_836 = -1;
        } else if self.skel2.update(data) {
            let a = data.anim(D_808543D4[sp2c]);
            self.skel2.play_loop(data, a);
            self.unk_836 = 1;
        } else if self.unk_836 == 1 {
            self.unk_836 = 2;
        }
        if self.unk_834 > 10 {
            self.unk_834 -= 1;
        }
        self.func_80834eb8(env);
        if self.unk_836 > 0 && (self.unk_860 < 0 || (!self.s.held_item_button_is_held_down && !self.func_80834e7c())) {
            self.set_upper_action_func(UpperAction::BowShot);
            if self.unk_860 >= 0 {
                if sp2c == 0 {
                    if !self.func_808350a4(env) {
                        self.play_sfx(D_808543DC[(self.unk_860.unsigned_abs() as usize).saturating_sub(1).min(1)]);
                    }
                } else if self.grounded() {
                    self.func_808350a4(env);
                }
            }
            self.unk_834 = 10;
            self.zero_speed_xz();
        } else {
            self.state1 |= STATE1_9;
        }
        true
    }

    /// `func_808353D8`: after the shot: the button again draws the next (`func_8083442C`,
    /// `link_bow_bow_shoot_next`); else, after `unk_834`'s frames and out of Z-targeting and first
    /// person, lowered (`func_80835588`, `link_bow_bow_shoot_end`).
    fn func_808353d8(&mut self, env: &Env) -> bool {
        let data = env.data;
        self.skel2.update(data);
        if self.holds_hookshot(data) && !self.func_80834fbc() {
            return true;
        }
        if !self.func_80834758(env) && (self.s.use_held_item || (self.unk_860 < 0 && self.s.held_item_button_is_held_down) || self.func_80834e44()) {
            self.unk_860 = self.unk_860.abs();
            if self.func_8083442c(env) {
                if self.holds_hookshot(data) {
                    self.unk_836 = 1;
                } else {
                    let a = data.anim("link_bow_bow_shoot_next");
                    self.skel2.play_once(data, a);
                }
            }
        } else {
            if self.unk_834 != 0 {
                self.unk_834 -= 1;
            }
            if self.is_z_targeting() || self.unk_6AD != 0 || self.state1 & STATE1_20 != 0 {
                if self.unk_834 == 0 {
                    self.unk_834 += 1;
                }
                return true;
            }
            if self.holds_hookshot(data) {
                self.set_upper_action_func(UpperAction::Bow);
            } else {
                self.set_upper_action_func(UpperAction::BowLower);
                let a = data.anim("link_bow_bow_shoot_end");
                self.skel2.play_once(data, a);
            }
            self.unk_834 = 0;
        }
        true
    }

    /// `func_80835588`: lowering, back to `func_8083501C` when it ends (at once in the air).
    fn func_80835588(&mut self, env: &Env) -> bool {
        if !self.grounded() || self.skel2.update(env.data) {
            self.set_upper_action_func(UpperAction::Bow);
        }
        true
    }

    /// `func_8083B8F4`: C-Up's look (`unk_6AD` 1), not carrying or riding, where the main
    /// camera's setting has first person, on the ground or swimming shallow.
    fn func_8083b8f4(&mut self, env: &Env) -> bool {
        if self.state1 & (STATE1_11 | STATE1_23) == 0 && self.camera_check_valid_mode(env, oot_game::camera::CAM_MODE_FIRST_PERSON) != 0 && (self.grounded() || (self.func_808332B8() && self.actor.y_dist_to_water < self.age.unk_2C)) {
            self.unk_6AD = 1;
            return true;
        }
        false
    }

    /// `func_8083AD4C`: the main camera's first-person mode: aiming (`unk_6AD` 2) the bow's
    /// (`CAM_MODE_AIM_ADULT`), the slingshot's (`_AIM_CHILD`) or the boomerang's; else the look
    /// (`CAM_MODE_FIRST_PERSON`). Returns `Camera_RequestMode`'s result.
    fn func_8083ad4c(&mut self, env: &Env) -> i32 {
        use oot_game::camera::{CAM_MODE_AIM_ADULT, CAM_MODE_AIM_BOOMERANG, CAM_MODE_AIM_CHILD, CAM_MODE_FIRST_PERSON};
        let cam_mode = if self.unk_6AD == 2 {
            if self.func_8002dd6c() {
                if self.adult { CAM_MODE_AIM_ADULT } else { CAM_MODE_AIM_CHILD }
            } else {
                CAM_MODE_AIM_BOOMERANG
            }
        } else {
            CAM_MODE_FIRST_PERSON
        };
        self.camera_request_mode(env, cam_mode)
    }

    /// `Player_Action_8084B1D8`: first person. Swimming, Link floats (`func_8084B000`); else he
    /// stops. Aiming, the item's upper body runs. A cutscene, a lock-on, the camera refusing, or
    /// a button (aiming: A, B or R, Z-targeting, or the item lowered; the look: those or a C
    /// button) end it (`func_8083C148`, `NA_SE_SY_CAMERA_ZOOM_UP`); else after 13 frames (at once
    /// in the look) the stick turns the view (`func_8084ABD8`).
    fn action_8084b1d8(&mut self, env: &Env) {
        use eng_input::pad::{BTN_B, BTN_CDOWN, BTN_CLEFT, BTN_CRIGHT, BTN_CUP, BTN_R};
        let data = env.data;
        if self.state1 & STATE1_27 != 0 {
            self.func_8084B000();
            self.linear_velocity = self.func_8084AEEC(self.linear_velocity, 0.0, self.actor.shape_rot.y);
        } else {
            self.decelerate_to_zero();
        }
        if self.unk_6AD == 2 && (self.func_8002dd6c() || self.func_808332e4()) {
            self.update_upper_body(env);
        }
        let press = self.input.press.button;
        if self.cs_mode != 0
            || self.unk_6AD == 0
            || self.unk_6AD >= 4
            || self.update_hostile_lock_on()
            || self.focus_actor.is_some()
            || self.func_8083ad4c(env) == oot_game::camera::CAM_MODE_NORMAL as i32
            || (self.unk_6AD == 2 && (press & (BTN_A | BTN_B | BTN_R) != 0 || self.friendly_lock_on_or_parallel() || (!self.func_8002dd78() && !self.func_808334b4())))
            || (self.unk_6AD == 1 && press & (BTN_A | BTN_B | BTN_R | BTN_CUP | BTN_CDOWN | BTN_CLEFT | BTN_CRIGHT) != 0)
        {
            self.func_8083C148(data);
            self.sfx(PlayerSfx::NoPos(NA_SE_SY_CAMERA_ZOOM_UP));
        } else {
            // DECR(this->av2.actionVar2).
            if self.action_var2 != 0 {
                self.action_var2 -= 1;
            }
            if self.action_var2 == 0 || self.unk_6AD != 2 {
                if self.func_8008f128(data) {
                    self.unk_6AE_rot_flags |= UNK6AE_ROT_FOCUS_X | UNK6AE_ROT_FOCUS_Y | UNK6AE_ROT_UPPER_X;
                } else {
                    self.actor.shape_rot.y = self.func_8084abd8(false, 0);
                }
            }
        }
        self.current_yaw = self.actor.shape_rot.y;
    }

    /// `func_8084ABD8`: first person's turn. In the look the stick pitches the focus (to 240 per
    /// unit, eased) and turns it (16 per unit, at most 3000 a frame); aiming (or `arg2`) the
    /// focus moves by `1 - cos` of the stick (riding, pitched at most 3500, else 14000), turned
    /// at most 19114 off the body. Returns the body's yaw less `arg3` (`func_80836AB8`).
    fn func_8084abd8(&mut self, arg2: bool, arg3: i16) -> i16 {
        let (sx, sy) = (self.input.rel.stick_x as i32, self.input.rel.stick_y as i32);
        if !self.func_8002dd78() && !self.func_808334b4() && !arg2 {
            let temp2 = (sy as f32 * 240.0) as i32 as i16;
            smooth_step_to_s(&mut self.actor.focus_rot.x, temp2, 14, 4000, 30);
            let temp2 = ((sx as f32 * -16.0) as i32 as i16).clamp(-3000, 3000);
            self.actor.focus_rot.y = self.actor.focus_rot.y.wrapping_add(temp2);
        } else {
            let temp1: i32 = if self.state1 & STATE1_23 != 0 { 3500 } else { 14000 };
            let temp3 = ((if sy >= 0 { 1 } else { -1 }) * ((1.0 - cos_s((sy * 200) as i16)) * 1500.0) as i32) as i16;
            self.actor.focus_rot.x = self.actor.focus_rot.x.wrapping_add(temp3);
            self.actor.focus_rot.x = (self.actor.focus_rot.x as i32).clamp(-temp1, temp1) as i16;
            let temp1: i32 = 19114;
            let mut temp2 = self.actor.focus_rot.y.wrapping_sub(self.actor.shape_rot.y);
            let temp3 = ((if sx >= 0 { 1 } else { -1 }) * ((1.0 - cos_s((sx * 200) as i16)) * -1500.0) as i32) as i16;
            temp2 = temp2.wrapping_add(temp3);
            self.actor.focus_rot.y = ((temp2 as i32).clamp(-temp1, temp1) as i16).wrapping_add(self.actor.shape_rot.y);
        }
        self.unk_6AE_rot_flags |= UNK6AE_ROT_FOCUS_Y;
        // (play->shootingGalleryStatus != 0: no gallery.)
        let aiming = self.func_8002dd78() || self.func_808334b4();
        self.func_80836AB8_arg(aiming).wrapping_sub(arg3)
    }

    /// `func_8083C61C`: a Deku nut thrown (`Player_Action_8084E604`, `link_normal_light_bom`), not
    /// in an indoors room, on the ground, with nuts left.
    fn func_8083c61c(&mut self, data: &GameData) -> bool {
        if self.room_type_view != ROOM_TYPE_INDOORS && self.grounded() && self.ammo(oot_game::item::ITEM_DEKU_NUT) != 0 {
            self.setup_action(data, Action::ThrowNut, 0);
            let a = data.anim("link_normal_light_bom");
            self.skel.play_once(data, a);
            self.unk_6AD = 0;
            return true;
        }
        false
    }

    /// `Player_Action_8084E604`: the nut's throw: on frame 3 one nut less and `En_Arrow`
    /// (`ARROW_NUT`) from the right hand, pitched 4000 along Link's facing, with
    /// `NA_SE_VO_LI_SWORD_N`; at the end, `link_normal_light_bom_end` back to standing
    /// (`func_8083A098`). Link slows to a stop.
    fn action_8084e604(&mut self, env: &Env) {
        let data = env.data;
        if self.skel.update(data) {
            let a = data.anim("link_normal_light_bom_end");
            self.func_8083A098(data, a);
        } else if self.skel.on_frame(3.0) {
            self.change_ammo(env, oot_game::item::ITEM_DEKU_NUT, -1);
            let pos = self.body_parts_pos[BODYPART_R_HAND];
            self.play_requests.push(PlayRequest::SpawnArrow { pos, rot: [4000, self.actor.shape_rot.y, 0], params: ARROW_NUT });
            self.play_voice_sfx(NA_SE_VO_LI_SWORD_N);
        }
        self.decelerate_to_zero();
    }

    /// `Player_ActionToMeleeWeapon`.
    fn melee_weapon(ap: i32) -> i32 {
        let m = ap - 2;
        if m > 0 && m < 6 { m } else { 0 }
    }

    /// `Player_ActionToModelGroup`: the item's model group (`sActionModelGroups`), the child's
    /// sword alone when he wears the Hylian Shield.
    fn action_to_model_group(&self, data: &GameData, item_action: i32) -> usize {
        let it = &data.items;
        let model_group = it.action_model_group[item_action.max(0) as usize];
        if model_group == it.model_group("SWORD_AND_SHIELD") && self.is_child_with_hylian_shield() { it.model_group("CHILD_HYLIAN_SHIELD") } else { model_group }
    }

    /// `Player_ActionToMagicSpell`: 0 to 5 for the spells (`PLAYER_IA_MAGIC_SPELL_15` on), else -1.
    fn action_to_magic_spell(data: &GameData, item_action: i32) -> i32 {
        let magic_spell = item_action - data.items.ap("MAGIC_SPELL_15");
        if (0..6).contains(&magic_spell) { magic_spell } else { -1 }
    }

    /// `Player_ActionToExplosive`: 0 a bomb, 1 a bombchu, else -1.
    fn action_to_explosive(data: &GameData, item_action: i32) -> i32 {
        let explosive = item_action - data.items.ap("BOMB");
        if (0..2).contains(&explosive) { explosive } else { -1 }
    }

    /// `Player_HoldsHookshot`.
    fn holds_hookshot(&self, data: &GameData) -> bool {
        self.held_item_ap == data.items.ap("HOOKSHOT") || self.held_item_ap == data.items.ap("LONGSHOT")
    }

    /// `func_8008F128`: the hookshot in hand with no hook (`heldActor`) on it. Player holds no
    /// actor here (`Arms_Hook` isn't ported).
    fn func_8008f128(&self, data: &GameData) -> bool {
        self.holds_hookshot(data)
    }

    /// `func_8008F2BC`: 0 to 2 for the swords (the cutscene's sword is 0), else -1.
    fn func_8008f2bc(data: &GameData, item_action: i32) -> i32 {
        let it = &data.items;
        if item_action == it.ap("SWORD_CS") {
            return 0;
        }
        let sword = item_action - it.ap("SWORD_MASTER");
        if (0..3).contains(&sword) { sword } else { -1 }
    }

    /// `Player_DetachHeldActor`: what Player holds (the seed or arrow in hand; carrying isn't
    /// ported) is let go, unless it's the hookshot's hook: no child, no `heldActor`, its
    /// `parent` cleared (`PlayRequest::ReleaseHeld`), `PLAYER_STATE1_CARRYING_ACTOR` off. And an
    /// explosive in hand goes: no item action, `heldItemId` `ITEM_NONE_FE` (the NTSC 1.1 and
    /// later order, as the debug ROM has it).
    fn detach_held_actor(&mut self, data: &GameData) {
        if let Some(h) = self.held_actor
            && !self.holds_hookshot(data)
        {
            self.actor.child = None;
            self.held_actor = None;
            self.interact_range_actor = None;
            self.play_requests.push(PlayRequest::ReleaseHeld(h));
            self.state1 &= !STATE1_11;
        }
        if Self::action_to_explosive(data, self.held_item_ap) >= 0 {
            self.next_model_group = self.action_to_model_group(data, 0);
            self.init_item_action(data, 0);
            self.held_item_id = oot_game::item::ITEM_NONE_FE;
        }
    }

    /// `AMMO(item)` as Player sees it this frame: the save's, refreshed at the start of its update
    /// and after its own `Inventory_ChangeAmmo` (`Player_SetupAction` reaches `Player_UseItem`
    /// through `Player_FinishItemChange` without the save at hand).
    fn ammo(&self, item: u8) -> i8 {
        self.ammo_view.get(oot_game::item::slot(item)).copied().unwrap_or(0)
    }

    /// `Inventory_ChangeAmmo(item, change)`, with Player's view of the ammo brought up to date.
    fn change_ammo(&mut self, env: &Env, item: u8, change: i16) {
        let mut io = env.io.borrow_mut();
        oot_game::item::inventory_change_ammo(&mut io.save, item, change);
        self.ammo_view = io.save.inventory.ammo;
    }

    /// `Player_UseItem`: use `item` (`ITEM_NONE`: put away what's in hand), from a button or the
    /// change animation's swap. Only with the item in hand in use (or the shield up and a melee
    /// weapon or nothing asked for, or no item action at all), and not swimming (but nothing, or
    /// the hookshot on the ground):
    /// - no stick, bean or explosive left (or three explosives out): `NA_SE_SY_ERROR`;
    /// - nuts thrown (`func_8083C61C`); the lens, spells, masks, the ocarina and the bottles their
    ///   own ways (none can be on a button yet: what they'd start is logged);
    /// - another item: the change animation (`PLAYER_STATE1_START_CHANGING_HELD_ITEM`), or the
    ///   item at once when the two hold alike;
    /// - the item in hand: used (`sUseHeldItem`).
    fn use_item(&mut self, data: &GameData, item: u8) {
        use oot_game::item::{ITEM_BOMB, ITEM_BOMBCHU, ITEM_DEKU_NUT, ITEM_DEKU_STICK, ITEM_MAGIC_BEAN};
        let it = &data.items;
        let item_action = it.item_to_action_param(item);
        let none = it.ap("NONE");
        if !((self.held_item_ap == self.item_ap && (self.state1 & STATE1_22 == 0 || Self::melee_weapon(item_action) != 0 || item_action == none))
            || (self.item_ap < 0 && (Self::melee_weapon(item_action) != 0 || item_action == none)))
        {
            return;
        }
        if !(item_action == none || self.state1 & STATE1_27 == 0 || (self.grounded() && (item_action == it.ap("HOOKSHOT") || item_action == it.ap("LONGSHOT")))) {
            return;
        }
        // (play->bombchuBowlingStatus is 0: the bowling alley isn't ported.)
        let explosive = Self::action_to_explosive(data, item_action);
        const EXPLOSIVE_ITEMS: [u8; 2] = [ITEM_BOMB, ITEM_BOMBCHU];
        if (item_action == it.ap("DEKU_STICK") && self.ammo(ITEM_DEKU_STICK) == 0)
            || (item_action == it.ap("MAGIC_BEAN") && self.ammo(ITEM_MAGIC_BEAN) == 0)
            || (explosive >= 0 && (self.ammo(EXPLOSIVE_ITEMS[explosive as usize]) == 0 || self.explosive_count >= 3))
        {
            // Out of ammo, or three explosives out already.
            self.sfx(PlayerSfx::NoPos(NA_SE_SY_ERROR));
        } else if item_action == it.ap("LENS_OF_TRUTH") {
            self.note("Player_UseItem: the Lens of Truth (Magic_RequestChange, actorCtx.lensActive) isn't ported");
        } else if item_action == it.ap("DEKU_NUT") {
            if self.ammo(ITEM_DEKU_NUT) != 0 {
                self.func_8083c61c(data);
            } else {
                self.sfx(PlayerSfx::NoPos(NA_SE_SY_ERROR));
            }
        } else if Self::action_to_magic_spell(data, item_action) >= 0 {
            self.note("Player_UseItem: the spells (the magic meter, unk_6AD 4) aren't ported");
        } else if item_action >= it.ap("MASK_KEATON") {
            // The masks (not drawn).
            self.current_mask = if self.current_mask != PLAYER_MASK_NONE { PLAYER_MASK_NONE } else { (item_action - it.ap("MASK_KEATON") + 1) as u8 };
            self.func_808328EC(NA_SE_PL_CHANGE_ARMS);
        } else if (item_action >= it.ap("OCARINA_FAIRY") && item_action <= it.ap("OCARINA_OF_TIME")) || item_action >= it.ap("BOTTLE_FISH") {
            // The cutscene items: their action (Player_ActionHandler_13 with unk_6AD 4) isn't ported.
            if !self.check_hostile_lock_on() || (item_action >= it.ap("BOTTLE_POTION_RED") && item_action <= it.ap("BOTTLE_FAIRY")) {
                self.play_requests.push(PlayRequest::TitleCardClear);
                self.unk_6AD = 4;
                self.item_ap = item_action;
            }
        } else if item_action != self.held_item_ap || explosive >= 0 {
            // A new item in hand (an explosive with none held: Player holds no actor here).
            self.next_model_group = self.action_to_model_group(data, item_action);
            let next_anim_type = it.model_group_anim_type[self.next_model_group];
            if self.held_item_ap >= 0 && Self::action_to_magic_spell(data, item_action) < 0 && item != self.held_item_id && it.change_matrix[it.model_group_anim_type[self.model_group]][next_anim_type] != 0 {
                self.held_item_id = item;
                self.state1 |= STATE1_8;
            } else {
                // Player_DestroyHookshot: no hook is held.
                self.detach_held_actor(data);
                self.init_item_action_with_anim(data, item_action);
            }
        } else {
            self.s.use_held_item = true;
            self.s.held_item_button_is_held_down = true;
        }
    }

    /// `Player_InitItemActionWithAnim`: swap the item now, keeping the animation group.
    fn init_item_action_with_anim(&mut self, data: &GameData, ap: i32) {
        let cur = self.skel.animation;
        let groups = data.anim_group_names.len();
        let found = (0..groups).find(|&g| data.player_anim(g, self.model_anim_type) == cur);
        self.state1 &= !(STATE1_3 | STATE1_24);
        self.init_item_action(data, ap);
        if let Some(g) = found {
            self.skel.animation = data.player_anim(g, self.model_anim_type);
        }
    }

    /// `Player_InitItemAction`: the item is in hand: `unk_85C`, `unk_858` and `unk_860` zeroed,
    /// the item's init (`sItemActionInitFuncs`), the model group.
    fn init_item_action(&mut self, data: &GameData, ap: i32) {
        self.unk_85c = 0.0;
        self.unk_858 = 0.0;
        self.unk_860 = 0;
        self.held_item_ap = ap;
        self.item_ap = ap;
        self.model_group = self.next_model_group;
        self.state1 &= !(STATE1_3 | STATE1_24);
        let it = &data.items;
        if ap == it.ap("DEKU_STICK") {
            // Player_InitDekuStickIA: the stick whole.
            self.unk_85c = 1.0;
        } else if ap >= it.ap("BOW") && ap <= it.ap("SLINGSHOT") {
            // Player_InitBowOrSlingshotIA.
            self.state1 |= STATE1_3;
            self.unk_860 = if ap != it.ap("SLINGSHOT") { -1 } else { -2 };
        } else if ap == it.ap("HOOKSHOT") || ap == it.ap("LONGSHOT") {
            // Player_InitHookshotIA: the hook (Arms_Hook) isn't ported.
            self.state1 |= STATE1_3;
            self.unk_860 = -3;
            self.note("Player_InitHookshotIA: Arms_Hook isn't ported");
        } else if ap == it.ap("BOMB") || ap == it.ap("BOMBCHU") {
            // Player_InitExplosiveIA: carrying something, the item goes back; else the bomb
            // (En_Bom, En_Bom_Chu) would be spawned in hand and its ammo spent.
            if self.state1 & STATE1_11 != 0 {
                self.put_away_held_item(data);
            } else {
                self.note("Player_InitExplosiveIA: En_Bom and En_Bom_Chu aren't ported");
            }
        } else if ap == it.ap("BOOMERANG") {
            // Player_InitBoomerangIA.
            self.state1 |= STATE1_24;
        }
        // Player_InitDefaultIA and Player_InitHammerIA: nothing.
        self.player_set_model_group(data, self.model_group);
    }

    /// `Player_SetModelGroup`: the model group decides the animation type.
    fn player_set_model_group(&mut self, data: &GameData, mg: usize) {
        self.model_group = mg;
        let it = &data.items;
        let mut t = if mg == it.model_group("CHILD_HYLIAN_SHIELD") { 0 } else { it.model_group_anim_type[mg] };
        if t < 3 && self.current_shield == 0 {
            t = 0;
        }
        self.model_anim_type = t;
        // Player_SetModels: the group's hands and sheath, then Player_SetModelsForHoldingShield.
        self.holding_shield = false;
        self.set_models_for_holding_shield(data);
    }

    /// `Player_StartChangingHeldItem`: start the change animation on `skelAnime2` (the bottle's and
    /// the boomerang's own, `PLAYER_ITEM_CHG_13`, in either direction), twice as fast when an
    /// item comes out.
    fn start_changing_held_item(&mut self, data: &GameData) {
        let it = &data.items;
        let held_item_action = it.item_to_action_param(self.held_item_id);
        self.set_upper_action_func(UpperAction::Change);
        let next_type = it.model_group_anim_type[self.next_model_group];
        let mut item_change_type = it.change_matrix[it.model_group_anim_type[self.model_group]][next_type];
        let (bottle, boomerang) = (it.ap("BOTTLE"), it.ap("BOOMERANG"));
        if held_item_action == bottle || held_item_action == boomerang || (held_item_action == it.ap("NONE") && (self.held_item_ap == bottle || self.held_item_ap == boomerang)) {
            item_change_type = if held_item_action == it.ap("NONE") { -PLAYER_ITEM_CHG_13 } else { PLAYER_ITEM_CHG_13 };
        }
        self.item_change_type = item_change_type.unsigned_abs() as usize;
        let mut anim = it.change_anims[self.item_change_type].0;
        if anim == data.anim("link_normal_fighter2free") && self.current_shield == 0 {
            anim = data.anim("link_normal_free2fighter_free");
        }
        let last = data.anims[anim].last_frame();
        let (mut speed, start, end) = if item_change_type >= 0 { (1.2, 0.0, last) } else { (-1.2, last, 0.0) };
        if held_item_action != it.ap("NONE") {
            speed *= 2.0;
        }
        self.skel2.change(data, anim, speed, start, end, ANIMMODE_ONCE, 0.0);
        self.state1 &= !STATE1_8;
    }

    /// `Player_UpperAction_ChangeHeldItem`: the change playing. On its swap frame the item goes
    /// into the hand; once it's in hand, a press (any for an item not held like a sword,
    /// `PLAYER_ANIMTYPE_3`) ends the change.
    fn upper_action_change_held_item(&mut self, env: &Env) -> bool {
        let data = env.data;
        let done = self.skel2.update(data);
        // (shootingGalleryStatus is 0.)
        let in_hand = data.items.item_to_action_param(self.held_item_id) == self.held_item_ap && {
            self.s.use_held_item = self.s.use_held_item || self.model_anim_type != 3;
            self.s.use_held_item
        };
        if done || in_hand {
            let f = self.upper_for(data, self.held_item_ap);
            self.set_upper_action_func(f);
            self.unk_834 = 0;
            self.idle_type = 0;
            self.s.held_item_button_is_held_down = self.s.use_held_item;
            return self.run_upper(env);
        }
        if self.check_for_idle_anim(data) != 0 {
            self.wait_to_finish_item_change(env);
            let a = self.anim(data, group::WAIT);
            self.skel.play_once(data, a);
            self.idle_type = 0;
        } else {
            self.wait_to_finish_item_change(env);
        }
        true
    }

    /// `Player_WaitToFinishItemChange`: swap the item on the change animation's swap frame
    /// (`sItemChangeInfo`'s, a frame earlier played backwards).
    fn wait_to_finish_item_change(&mut self, env: &Env) {
        let mut t = env.data.items.change_anims[self.item_change_type].1;
        if self.skel2.play_speed < 0.0 {
            t -= 1.0;
        }
        if self.skel2.on_frame(t) {
            self.finish_item_change(env.data);
        }
        self.player_update_hostile_lock_on_env(env);
    }

    /// `Player_FinishItemChange`: the item in hand goes (a sword back in its sheath, anything else
    /// with `NA_SE_PL_CHANGE_ARMS`), `Player_UseItem(heldItemId)`, and the new one comes out the
    /// same way.
    fn finish_item_change(&mut self, data: &GameData) {
        let none = data.items.ap("NONE");
        if self.held_item_ap != none {
            self.func_808328EC(if Self::func_8008f2bc(data, self.held_item_ap) >= 0 { NA_SE_IT_SWORD_PUTAWAY } else { NA_SE_PL_CHANGE_ARMS });
        }
        let item = self.held_item_id;
        self.use_item(data, item);
        if Self::func_8008f2bc(data, self.held_item_ap) >= 0 {
            self.func_808328EC(NA_SE_IT_SWORD_PICKOUT);
        } else if self.held_item_ap != none {
            self.func_808328EC(NA_SE_PL_CHANGE_ARMS);
        }
    }

    /// `Player_ActionHandler_7` (interrupt 7): B with a melee weapon in hand attacks.
    fn action_handler_7(&mut self, env: &Env) -> bool {
        // func_8083C6B8 (bottles, the fishing rod): not held.
        if self.func_8083BB20() {
            let sp24 = self.func_80837818(env);
            self.func_80837948(env.data, sp24);
            if sp24 >= env.data.items.mwa("SPIN_ATTACK_1H") {
                self.state2 |= STATE2_17;
                // func_80837530 (spin attack effects): not ported.
            }
            return true;
        }
        false
    }

    /// `func_8083BB20`.
    fn func_8083BB20(&self) -> bool {
        self.state1 & STATE1_22 == 0 && Self::melee_weapon(self.held_item_ap) != 0 && self.s.use_held_item
    }

    /// `Player_CanSpinAttack`: the stick spun a quarter-turn each of the last 3 frames (a quick spin);
    /// never with a Deku Stick or the broken knife.
    fn can_spin_attack(&self, data: &GameData, sword_health: u16) -> bool {
        if self.held_item_ap == data.items.ap("DEKU_STICK") || self.holds_broken_knife(data, sword_health) {
            return false;
        }
        let mut sp = [0i32; 4];
        for (i, v) in self.control_stick_spin_angles.iter().enumerate() {
            if *v < 0 {
                return false;
            }
            sp[i] = (*v as i32 * 2) as i8 as i32;
        }
        let t1 = (sp[0] - sp[1]) as i8 as i32;
        if t1.abs() < 10 {
            return false;
        }
        for i in 1..3 {
            let t2 = (sp[i] - sp[i + 1]) as i8 as i32;
            if t2.abs() < 10 || t2 * t1 < 0 {
                return false;
            }
        }
        true
    }

    /// `Player_HoldsBrokenKnife`: the Biggoron's Sword with no health left.
    fn holds_broken_knife(&self, data: &GameData, sword_health: u16) -> bool {
        self.held_item_ap == data.items.ap("SWORD_BIGGORON") && sword_health == 0
    }

    /// `func_80837818`: which attack: the hammer's by the stick's direction (`D_80854484`); else a
    /// spin, or by the stick's direction (`D_80854480`: a stab only locked on), a Deku Stick
    /// always the forward slash; a two-handed weapon (the Biggoron's Sword, the stick, the
    /// hammer) its two-handed version, the next.
    fn func_80837818(&mut self, env: &Env) -> usize {
        let it = &env.data.items;
        let mut sp1c = self.stick_dir();
        let mut sp18;
        if self.held_item_ap == it.ap("HAMMER") {
            if sp1c <= -1 {
                sp1c = 0;
            }
            // D_80854484.
            sp18 = [it.mwa("HAMMER_FORWARD"), it.mwa("HAMMER_SIDE"), it.mwa("HAMMER_FORWARD"), it.mwa("HAMMER_SIDE")][sp1c as usize];
            self.unk_845 = 0;
        } else {
            let sword_health = env.io.borrow().save.sword_health;
            if self.can_spin_attack(env.data, sword_health) {
                sp18 = it.mwa("SPIN_ATTACK_1H");
            } else {
                if sp1c <= -1 {
                    sp18 = if self.is_z_targeting() { it.mwa("FORWARD_SLASH_1H") } else { it.mwa("RIGHT_SLASH_1H") };
                } else {
                    sp18 = it.attack_by_dir[sp1c as usize];
                    if sp18 == it.mwa("STAB_1H") {
                        self.state2 |= STATE2_30;
                        if !self.is_z_targeting() {
                            sp18 = it.mwa("FORWARD_SLASH_1H");
                        }
                    }
                }
                if self.held_item_ap == it.ap("DEKU_STICK") {
                    sp18 = it.mwa("FORWARD_SLASH_1H");
                }
            }
            if self.holds_two_handed_weapon(env.data) {
                sp18 += 1;
            }
        }
        sp18
    }

    /// `func_80837948`: start attack `arg2`; the third of the same attack in a row is its combo.
    fn func_80837948(&mut self, data: &GameData, mut arg2: usize) {
        let it = &data.items;
        let (flip0, jump1) = (it.mwa("FLIPSLASH_START"), it.mwa("JUMPSLASH_FINISH"));
        self.setup_action(data, Action::Attack, 0);
        self.unk_844 = 8;
        if !(arg2 >= it.mwa("FLIPSLASH_FINISH") && arg2 <= jump1) {
            self.func_80832318();
        }
        if arg2 != self.melee_weapon_animation || self.unk_845 >= 3 {
            self.unk_845 = 0;
        }
        self.unk_845 += 1;
        if self.unk_845 >= 3 {
            arg2 += 2;
        }
        self.melee_weapon_animation = arg2;
        let a = it.attacks[arg2].anim;
        self.skel.play_once_set_speed(data, a, 2.0 / 3.0);
        if arg2 != flip0 && arg2 != it.mwa("JUMPSLASH_START") {
            self.start_anim_movement(0x209);
        }
        self.current_yaw = self.actor.shape_rot.y;
        // Player_HoldsBrokenKnife: the Biggoron's Sword isn't held.
        let row = (Self::melee_weapon(self.held_item_ap) - 1).clamp(0, 4) as usize;
        let jump = arg2 >= flip0 && arg2 <= jump1;
        let dmg_flags = D_80854488[row][usize::from(jump)];
        self.func_80837918(0, dmg_flags);
        self.func_80837918(1, dmg_flags);
    }

    /// `func_80837918`: the sword quad's damage type for this attack.
    fn func_80837918(&mut self, quad: usize, dmg_flags: u32) {
        let q = &mut self.melee_weapon_quads[quad];
        q.info.at_dmg_info.dmg_flags = dmg_flags;
        q.info.at_elem_flags = if dmg_flags == cc::DMG_DEKU_STICK { cc::ATELEM_ON | cc::ATELEM_NEAREST | cc::ATELEM_SFX_WOOD } else { cc::ATELEM_ON | cc::ATELEM_NEAREST };
    }

    /// `func_80832318`: weapon inactive.
    fn func_80832318(&mut self) {
        self.state2 &= !STATE2_17;
        self.melee_weapon_state = 0;
        for w in &mut self.melee_weapon_info {
            w.active = false;
        }
    }

    /// `Player_StartAnimMovement`: animation-driven movement with `flags` (`ANIM_FLAG_*`, 0x200 scales
    /// the start translation by `ageProperties->unk_08`).
    fn start_anim_movement(&mut self, flags: u16) {
        if flags & 0x200 != 0 {
            // Player_ResetAnimMovementScaledByAge.
            self.reset_anim_movement();
            let k = self.age.translation_scale;
            self.skel.prev_transl = self.skel.prev_transl.map(|v| (v as f32 * k) as i16);
        } else if flags & 0x100 != 0 || self.skel.move_flags != 0 {
            self.reset_anim_movement();
        } else {
            self.skel.prev_transl = self.skel.joint[0];
            self.skel.prev_rot = self.actor.shape_rot.y;
        }
        self.skel.move_flags = flags;
        self.zero_speed_xz();
        self.skel.disable_queue();
    }

    /// `func_8084285C`: the weapon is active from `arg1` to `arg3` (`func_80833A20`).
    fn func_8084285C(&mut self, env: &Env, arg1: f32, arg2: f32, arg3: f32) -> bool {
        let f = self.skel.cur_frame;
        if arg1 <= f && f <= arg3 {
            self.func_80833A20(env, if arg2 <= f { 1 } else { -1 });
            return true;
        }
        self.func_80832318();
        false
    }

    /// `func_80833A20`: the weapon's state; as a swing starts, its sound (the swing, the hard
    /// swing from the fourth chained, the hammer, none for a spin) and Link's voice (not for
    /// the flip and jump slashes).
    fn func_80833A20(&mut self, env: &Env, new_melee_weapon_state: i8) {
        if self.melee_weapon_state == 0 {
            let items = &env.data.items;
            let ap = self.held_item_ap;
            let sword_health = env.io.borrow().save.sword_health;
            let mut item_sfx = if ap == items.ap("SWORD_BIGGORON") && sword_health > 0 { NA_SE_IT_HAMMER_SWING } else { NA_SE_IT_SWORD_SWING };
            let mut voice_sfx = NA_SE_VO_LI_SWORD_N;
            // PLAYER_MWA_SPIN_ATTACK_1H (24), PLAYER_MWA_FLIPSLASH_START (16) to
            // PLAYER_MWA_JUMPSLASH_FINISH (19) (player.h).
            if ap == items.ap("HAMMER") {
                item_sfx = NA_SE_IT_HAMMER_SWING;
            } else if self.melee_weapon_animation >= 24 {
                item_sfx = 0;
                voice_sfx = NA_SE_VO_LI_SWORD_L;
            } else if self.unk_845 >= 3 {
                item_sfx = NA_SE_IT_SWORD_SWING_HARD;
                voice_sfx = NA_SE_VO_LI_SWORD_L;
            }
            if item_sfx != 0 {
                self.func_808328EC(item_sfx);
            }
            if !(16..=19).contains(&self.melee_weapon_animation) {
                self.play_voice_sfx(voice_sfx);
            }
        }
        self.melee_weapon_state = new_melee_weapon_state;
    }

    /// `func_8083C50C`: B released during the attack.
    fn func_8083C50C(&mut self) {
        if self.unk_844 > 0 && !self.input.cur.held(eng_input::pad::BTN_B) {
            self.unk_844 = -self.unk_844;
        }
    }

    /// `Player_Action_808502D0`: attacking. At the end another B press chains (`Player_ActionHandler_7`);
    /// otherwise the end animation plays into the (targeting) stance.
    fn action_808502d0(&mut self, env: &Env) {
        let data = env.data;
        let a = data.items.attacks[self.melee_weapon_animation];
        self.state2 |= STATE2_5;
        if !self.func_80842df4(env) {
            self.func_8084285C(env, 0.0, a.active_start, a.active_end);
            if self.state2 & STATE2_30 != 0 && self.skel.on_frame(0.0) {
                self.linear_velocity = 15.0;
                self.state2 &= !STATE2_30;
            }
            if self.linear_velocity > 12.0 {
                self.func_8084269c(env);
            }
            step_to_f(&mut self.linear_velocity, 0.0, 5.0);
            self.func_8083C50C();
            if self.skel.update(data) && !self.action_handler_7(env) {
                let mf = self.skel.move_flags;
                let end = if self.state1 & STATE1_4 != 0 { a.end_locked } else { a.end };
                self.func_80832318();
                self.skel.move_flags = 0;
                self.func_8083A098(data, end);
                self.skel.move_flags = mf;
                self.state3 |= STATE3_3;
            }
        }
    }

    // ================================================================================
    // Water (func_8083D53C) and swimming

    /// `func_8083D53C`: the underwater filter, then entering and leaving water.
    fn func_8083D53C(&mut self, env: &Env) {
        if self.actor.y_dist_to_water < self.age.unk_2C {
            self.sfx(PlayerSfx::BaseFilter(0));
            self.underwater_timer = 0;
        } else {
            self.sfx(PlayerSfx::BaseFilter(0x20));
            if self.underwater_timer < 300 {
                self.underwater_timer += 1;
            }
        }
        let a = self.action;
        if a == Action::ClimbLedge || a == Action::ClimbUp {
            return;
        }
        let swim_actions = matches!(a, Action::Swim | Action::SwimMove | Action::SwimTarget | Action::Dive | Action::Surface);
        if self.age.unk_2C < self.actor.y_dist_to_water {
            // Kokiri boots; Player_Action_8084E30C / Player_Action_8084E368 / Player_Action_8084D7C4 are not ported.
            if self.state1 & STATE1_27 == 0 || !swim_actions {
                self.func_8083D36C(env);
            }
        } else if self.state1 & STATE1_27 != 0 && self.actor.y_dist_to_water < self.age.unk_24 {
            if self.skel.move_flags == 0 {
                let y = self.actor.shape_rot.y;
                self.setup_turn_in_place(env.data, y);
            }
            let vy = self.actor.velocity.y;
            self.func_8083D0A8(env, vy);
        }
    }

    /// `func_8083CFA8`: at more than 2 of vertical speed `arg2`, a splash at the waist where it
    /// meets a water surface less than 100 above Link (`BgCheck_GetWaterSurfaceAllHack`): true if there
    /// is one. (The splash, `EffectSsGSplash`, of kind 0 up to 10 of speed and 1 above, at
    /// `splash_scale`, isn't drawn: the effects aren't ported.)
    fn func_8083CFA8(&mut self, env: &Env, arg2: f32, _splash_scale: i32) -> bool {
        let sp3c = arg2.abs();
        if sp3c > 2.0 {
            let waist = self.body_parts_pos[PLAYER_BODYPART_WAIST];
            if let Some(sp34) = env.col.water_surface(waist.x, waist.z, env.col.water_room)
                && (sp34 - self.actor.world_pos.y) < 100.0
            {
                return true;
            }
        }
        false
    }

    /// `func_8083D36C`: start swimming.
    fn func_8083D36C(&mut self, env: &Env) {
        let data = env.data;
        self.func_80832564(data);
        if self.state2 & STATE2_10 != 0 {
            self.state2 &= !STATE2_10;
            self.func_8083D12C(env, false);
            self.action_var1 = 1;
        } else {
            // (From Player_Action_80844A44, the hookshot flight, it would dive: not ported.)
            self.setup_action(data, Action::Swim, 1);
            let a = if self.grounded() { data.anim("link_swimer_wait2swim_wait") } else { data.anim("link_swimer_land2swim_wait") };
            self.anim_change_once_morph(data, a);
        }
        if self.state1 & STATE1_27 == 0 || self.actor.y_dist_to_water < self.age.unk_2C {
            let vy = self.actor.velocity.y;
            if self.func_8083CFA8(env, vy, 500) {
                self.play_sfx(NA_SE_EV_DIVE_INTO_WATER);
                if self.fall_distance > 800 {
                    self.play_voice_sfx(NA_SE_VO_LI_CLIMB_END);
                }
            }
        }
        self.state1 |= STATE1_27;
        self.state2 |= STATE2_10;
        self.state1 &= !(STATE1_18 | STATE1_19);
        // Player_SetBootData: Kokiri boots keep their data in water.
    }

    /// `func_8083D0A8`: out of the water, with a splash's sound if there's one.
    fn func_8083D0A8(&mut self, env: &Env, vy: f32) {
        self.state1 |= STATE1_18;
        self.state1 &= !STATE1_27;
        self.func_80832340();
        if self.func_8083CFA8(env, vy, 500) {
            self.play_sfx(NA_SE_EV_JUMP_OUT_WATER);
        }
    }

    /// `func_80832340`.
    fn func_80832340(&mut self) {
        self.state2 &= !(STATE2_10 | STATE2_11);
    }

    /// `func_8083D12C`: dive on A from the surface (`input`), or surface when rising close to it.
    fn func_8083D12C(&mut self, env: &Env, input: bool) -> bool {
        let data = env.data;
        if self.state1 & STATE1_10 == 0 && self.state2 & STATE2_10 == 0 && (!input || (self.input.press.held(BTN_A) && abs16(self.unk_6C2) < 12000)) {
            self.setup_action(data, Action::Dive, 0);
            let a = data.anim("link_swimer_swim_deep_start");
            self.skel.play_once(data, a);
            self.unk_6C2 = 0;
            self.state2 |= STATE2_10;
            self.actor.velocity.y = 0.0;
            if input {
                self.state2 |= STATE2_11;
                self.play_sfx(NA_SE_PL_DIVE_BUBBLE);
            }
            return true;
        }
        if (self.state1 & STATE1_10 != 0 || self.state2 & STATE2_10 != 0) && self.actor.velocity.y > 0.0 && self.actor.y_dist_to_water < self.age.unk_30 {
            self.state2 &= !STATE2_10;
            if input {
                self.setup_action(data, Action::Surface, 1);
                self.action_var2 = 2;
            }
            self.func_80832340();
            let a = data.anim("link_swimer_swim_deep_end");
            self.anim_change_once_morph(data, a);
            let vy = self.actor.velocity.y;
            if self.func_8083CFA8(env, vy, 500) {
                self.play_sfx(NA_SE_PL_FACE_UP);
            }
            return true;
        }
        false
    }

    /// `func_8084B000`: buoyancy. Floats up to `unk_28` below the surface; sinks slowly when
    /// deeper than 100 (and flags `PLAYER_STATE2_10`).
    fn func_8084B000(&mut self) {
        let mut phi_f14 = -5.0f32;
        let mut phi_f16 = self.age.unk_28;
        if self.actor.velocity.y < 0.0 {
            phi_f16 += 1.0;
        }
        let phi_f18;
        if self.actor.y_dist_to_water < phi_f16 {
            phi_f16 = if self.actor.velocity.y <= 0.0 { 0.0 } else { self.actor.velocity.y * 0.5 };
            phi_f18 = -0.1 - phi_f16;
        } else {
            phi_f14 = 2.0;
            phi_f16 = if self.actor.velocity.y >= 0.0 { 0.0 } else { self.actor.velocity.y * -0.3 };
            phi_f18 = phi_f16 + 0.1;
            if self.actor.y_dist_to_water > 100.0 {
                self.state2 |= STATE2_10;
            }
        }
        self.actor.velocity.y += phi_f18;
        if (self.actor.velocity.y - phi_f14) * phi_f18 > 0.0 {
            self.actor.velocity.y = phi_f14;
        }
        self.actor.gravity = 0.0;
    }

    /// `func_8084AEEC`: swim acceleration, only in frames 10..20 of the stroke; turn at 1600.
    fn func_8084AEEC(&mut self, v: f32, mut arg2: f32, arg3: i16) -> f32 {
        let mut v = v;
        let mut t = self.skel.cur_frame - 10.0;
        let limit = self.regs.run_speed_limit() * 0.8;
        if v > limit {
            v = limit;
        }
        if 0.0 < t && t < 10.0 {
            t *= 6.0;
        } else {
            t = 0.0;
            arg2 = 0.0;
        }
        let dec = v.abs() * 0.02 + 0.05;
        asym_step_to_f(&mut v, arg2 * 0.8, t, dec);
        scaled_step_to_s(&mut self.current_yaw, arg3, 1600);
        v
    }

    /// `func_8084D530`: `func_8084AEEC`, and the strokes' sound (`D_808549D0`).
    fn func_8084D530(&mut self, env: &Env, v: f32, arg2: f32, arg3: i16) -> f32 {
        let v = self.func_8084AEEC(v, arg2, arg3);
        self.process_anim_sfx_list(env.audio.player_anim_sfx("D_808549D0"));
        v
    }

    /// `func_8084B158`: stroke animation speed from the swim speed (doubled on A/B).
    fn func_8084B158(&mut self, data: &GameData, input: bool, arg3: f32) {
        let k = if input && (self.input.press.held(BTN_A) || self.input.press.held(eng_input::pad::BTN_B)) { 1.0 } else { 0.5 };
        self.skel.play_speed = (k * arg3).max(1.0);
        self.skel.update(data);
    }

    /// `Player_AnimChangeLoopSlowMorph`: loop `anim`, blending over 16 frames.
    fn anim_change_loop_slow_morph(&mut self, data: &GameData, anim: AnimId) {
        self.skel.change(data, anim, 1.0, 0.0, 0.0, ANIMMODE_LOOP, -16.0);
    }

    /// `func_8084D574` / `func_8084D5CC` / `func_80838F18`: swim forward, swim targeting, tread.
    fn func_8084D574(&mut self, data: &GameData, yaw: i16) {
        self.setup_action(data, Action::SwimMove, 0);
        self.current_yaw = yaw;
        self.actor.shape_rot.y = yaw;
        let a = data.anim("link_swimer_swim");
        self.anim_change_loop_slow_morph(data, a);
    }
    fn func_8084D5CC(&mut self, data: &GameData) {
        self.setup_action(data, Action::SwimTarget, 0);
        let a = data.anim("link_swimer_swim");
        self.anim_change_loop_slow_morph(data, a);
    }
    fn func_80838F18(&mut self, data: &GameData) {
        self.setup_action(data, Action::Swim, 0);
        let a = data.anim("link_swimer_swim_wait");
        self.anim_change_loop_slow_morph(data, a);
    }

    /// `Player_Action_8084D610`: treading water.
    fn action_8084d610(&mut self, env: &Env) {
        let data = env.data;
        // func_80832CB0.
        if self.skel.update(data) {
            let a = data.anim("link_swimer_swim_wait");
            self.skel.play_loop(data, a);
        }
        self.func_8084B000();
        if !self.try_action_handler_list(env, Self::S_ACTION_HANDLER_LIST11, true) && !self.func_8083D12C(env, true) {
            if self.unk_6AD != 1 {
                self.unk_6AD = 0;
            }
            let (_, sp34, sp32) = self.get_movement_speed_and_yaw(env, 0.0);
            if sp34 != 0.0 {
                let t = self.actor.shape_rot.y.wrapping_sub(sp32);
                if abs16(t) > 0x6000 && !step_to_f(&mut self.linear_velocity, 0.0, 1.0) {
                    return;
                }
                if self.is_z_targeting_with_hostile_update(env) {
                    self.func_8084D5CC(data);
                } else {
                    self.func_8084D574(data, sp32);
                }
            }
            let v = self.linear_velocity;
            self.linear_velocity = self.func_8084AEEC(v, sp34, sp32);
        }
    }

    /// `Player_Action_8084D84C`: swimming forward.
    fn action_8084d84c(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5;
        let v = self.linear_velocity;
        self.func_8084B158(data, true, v);
        self.func_8084B000();
        if !self.try_action_handler_list(env, Self::S_ACTION_HANDLER_LIST11, true) && !self.func_8083D12C(env, true) {
            let (_, sp34, sp32) = self.get_movement_speed_and_yaw(env, 0.0);
            let t = self.actor.shape_rot.y.wrapping_sub(sp32);
            if sp34 == 0.0 || abs16(t) > 0x6000 {
                self.func_80838F18(data);
            } else if self.is_z_targeting_with_hostile_update(env) {
                self.func_8084D5CC(data);
            }
            let v = self.linear_velocity;
            self.linear_velocity = self.func_8084D530(env, v, sp34, sp32);
        }
    }

    /// `Player_Action_8084DAB4`: swimming while targeting (forward, back or sideways strokes).
    fn action_8084dab4(&mut self, env: &Env) {
        let data = env.data;
        let v = self.linear_velocity;
        self.func_8084B158(data, true, v);
        self.func_8084B000();
        if !self.try_action_handler_list(env, Self::S_ACTION_HANDLER_LIST11, true) && !self.func_8083D12C(env, true) {
            let (_, mut sp2c, mut sp2a) = self.get_movement_speed_and_yaw(env, 0.0);
            if sp2c == 0.0 {
                self.func_80838F18(data);
            } else if !self.is_z_targeting_with_hostile_update(env) {
                self.func_8084D574(data, sp2a);
            } else {
                self.func_8084D980(env, &mut sp2c, &mut sp2a);
            }
            let v = self.linear_velocity;
            self.linear_velocity = self.func_8084D530(env, v, sp2c, sp2a);
        }
    }

    /// `func_8084D980`: the stroke for the direction while targeting.
    fn func_8084D980(&mut self, env: &Env, speed: &mut f32, yaw: &mut i16) -> bool {
        let data = env.data;
        let t1 = self.current_yaw.wrapping_sub(*yaw);
        let anim = if abs16(t1) > 0x6000 {
            if step_to_f(&mut self.linear_velocity, 0.0, 1.0) {
                self.current_yaw = *yaw;
            } else {
                *speed = 0.0;
                *yaw = self.current_yaw;
            }
            data.anim("link_swimer_swim_wait")
        } else {
            let t2 = self.func_8083FD78(env, speed, yaw);
            if t2 > 0 {
                data.anim("link_swimer_swim")
            } else if t2 < 0 {
                data.anim("link_swimer_back_swim")
            } else if self.actor.shape_rot.y.wrapping_sub(*yaw) > 0 {
                data.anim("link_swimer_Rside_swim")
            } else {
                data.anim("link_swimer_Lside_swim")
            }
        };
        if anim != self.skel.animation {
            self.anim_change_loop_slow_morph(data, anim);
            return true;
        }
        false
    }

    /// `func_8083D330`.
    fn func_8083D330(&mut self, data: &GameData) {
        let a = data.anim("link_swimer_swim");
        self.skel.play_loop(data, a);
        self.unk_6C2 = 16000;
        self.action_var2 = 1;
    }

    /// `func_8084DBC4`: move under water (horizontal and vertical swim speeds).
    fn func_8084DBC4(&mut self, env: &Env, arg2: f32) {
        let (_, sp2c, sp2a) = self.get_movement_speed_and_yaw(env, 0.0);
        let v = self.linear_velocity;
        self.linear_velocity = self.func_8084AEEC(v, sp2c * 0.5, sp2a);
        let vy = self.actor.velocity.y;
        let cy = self.current_yaw;
        self.actor.velocity.y = self.func_8084AEEC(vy, arg2, cy);
    }

    /// `Player_Action_8084DC48`: diving: the dive start, holding A to go deeper (to D_80854784[no scale]
    /// = 120 below the surface), then pitching back up and rising.
    fn action_8084dc48(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5;
        self.actor.gravity = 0.0;
        self.update_upper_body(env);
        // Player_ActionHandler_13 (C-button items): none.
        if self.action_var1 == 0 {
            if self.action_var2 == 0 {
                if self.skel.update(data) || (self.skel.cur_frame >= 22.0 && !self.input.cur.held(BTN_A)) {
                    self.func_8083D330(data);
                } else if self.skel.on_frame(20.0) {
                    self.actor.velocity.y = -2.0;
                }
                self.decelerate_to_zero();
                return;
            }
            let vy = self.actor.velocity.y;
            self.func_8084B158(data, true, vy);
            self.unk_6C2 = 16000;
            // Player_ActionHandler_2 (picking up an item) is never taken; no scale upgrade.
            if self.input.cur.held(BTN_A) && !self.grounded() && self.actor.y_dist_to_water < 120.0 {
                self.func_8084DBC4(env, -2.0);
            } else {
                self.action_var1 += 1;
                let a = data.anim("link_swimer_swim_wait");
                self.anim_change_loop_slow_morph(data, a);
            }
        } else if self.action_var1 == 1 {
            self.skel.update(data);
            self.func_8084B000();
            if self.unk_6C2 < 10000 {
                self.action_var1 += 1;
                self.action_var2 = self.actor.y_dist_to_water as i16;
                let a = data.anim("link_swimer_swim");
                self.anim_change_loop_slow_morph(data, a);
            }
        } else if !self.func_8083D12C(env, true) {
            let sp2c = (self.action_var2 as f32 * 0.018 + 4.0).min(8.0);
            let vy = self.actor.velocity.y.abs();
            self.func_8084B158(data, true, vy);
            scaled_step_to_s(&mut self.unk_6C2, -10000, 800);
            self.func_8084DBC4(env, sp2c);
        }
    }

    /// `Player_Action_8084E1EC`: surfacing.
    fn action_8084e1ec(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5;
        if self.skel.update(data) {
            // Not holding an item from the bottom (PLAYER_STATE1_10).
            self.func_80838F18(data);
            self.func_80832340();
        } else if self.skel.on_frame(5.0) {
            // (PLAYER_STATE1_10's frame 10, the item from the bottom: not ported.)
            self.play_voice_sfx(NA_SE_VO_LI_BREATH_DRINK);
        }
        self.func_8084B000();
        let y = self.actor.shape_rot.y;
        let v = self.linear_velocity;
        self.linear_velocity = self.func_8084AEEC(v, 0.0, y);
    }

    // ================================================================================
    // Exits, entrances and voids

    /// `Player_HandleExitsAndVoids`: the floor's exit (or a void floor) starts the scene transition and the
    /// walk out; otherwise the fall-into-the-void check.
    fn handle_exits_and_voids(&mut self, env: &Env, poly: Option<PolyId>) -> bool {
        let col = env.col;
        let mut io = env.io.borrow_mut();
        let mut exit_index = 0;
        let exiting = self.state1 & STATE1_7 == 0
            && io.transition.trigger == TRANS_TRIGGER_OFF
            && self.cs_mode == 0
            && self.state1 & STATE1_0 == 0
            && ({
                exit_index = poly.map(|p| col.exit_index(p)).unwrap_or(0);
                exit_index != 0
            } || (func_8083816C(self.s.floor_type) && self.floor_property == FLOOR_PROPERTY_12));
        if exiting {
            let sp34 = self.unk_A84 as i32 - self.actor.world_pos.y as i32;
            if self.state1 & (STATE1_23 | STATE1_27 | STATE1_29) == 0 && !self.grounded() && sp34 < 100 && self.s.floor_dist > 100.0 {
                return false;
            }
            if exit_index == 0 {
                io.trigger_void_out();
                io.set_transition_for_next_entrance(env.entrances);
            } else {
                let Some(&next) = env.exits.get(exit_index as usize - 1) else {
                    drop(io);
                    self.note(format!("exit {exit_index} is past the exit list ({})", env.exits.len()));
                    return false;
                };
                io.transition.next_entrance_index = next;
                if next == ENTR_RETURN_GROTTO {
                    io.save.respawn_flag = 2;
                    io.transition.next_entrance_index = io.save.respawn[RESPAWN_MODE_RETURN].entrance_index;
                    io.transition.ty = TRANS_TYPE_FADE_WHITE;
                    io.save.next_transition_type = TRANS_TYPE_FADE_WHITE;
                } else if next >= ENTR_RETURN_GREAT_FAIRYS_FOUNTAIN_SPELLS {
                    // sReturnEntranceGroupData (the fountains' and shops' way back): not ported.
                    drop(io);
                    self.note(format!("exit to ENTR_RETURN group {next:#x} not ported"));
                    return false;
                } else {
                    if poly.is_some_and(|p| col.floor_effect(p) == FLOOR_EFFECT_2) {
                        io.save.respawn[RESPAWN_MODE_DOWN].entrance_index = next;
                        io.trigger_void_out();
                        io.save.respawn_flag = -2;
                    }
                    io.save.retain_weather_mode = true;
                    io.set_transition_for_next_entrance(env.entrances);
                }
                io.transition.trigger = TRANS_TRIGGER_START;
            }
            let temp = poly.map(|p| col.floor_type(p)).unwrap_or(0);
            if self.state1 & (STATE1_23 | STATE1_29) == 0
                && self.state2 & STATE2_18 == 0
                && !self.func_808332B8()
                && temp != FLOOR_TYPE_10
                && (sp34 < 100 || self.grounded())
            {
                if temp != FLOOR_TYPE_11 {
                    let mut v = self.linear_velocity;
                    if v < 0.0 {
                        self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(-0x8000i32 as i16);
                        v = -v;
                    }
                    let limit = self.regs.run_speed_limit();
                    io.save.entrance_speed = if v > limit { limit } else { v };
                    // (Conveyor floors would give sConveyorYaw: not modelled.)
                    let yaw = self.actor.world_rot.y;
                    drop(io);
                    self.func_80838E70(env.data, 400.0, yaw);
                } else {
                    // FLOOR_TYPE_11 (a secret hole): its sound, and the music fades.
                    self.sfx(PlayerSfx::NoPos(NA_SE_OC_SECRET_HOLE_OUT));
                    self.play_requests.push(PlayRequest::FadeOutAllSeq(5));
                    io.save.seq_id = oot_game::audio::NA_BGM_DISABLED as u8;
                    io.save.nature_ambience_id = oot_game::audio::NATURE_ID_DISABLED;
                }
            } else if !self.grounded() {
                self.zero_speed_xz();
            }
            self.state1 |= STATE1_0 | STATE1_29;
            self.play_requests.push(PlayRequest::CamSetting(oot_game::camera::CAM_SET_SCENE_TRANSITION));
            return true;
        }
        if io.transition.trigger == TRANS_TRIGGER_OFF {
            let fall = self.fall_distance;
            let scene = io.scene_id;
            if self.actor.world_pos.y < -4000.0
                || ((self.floor_property == FLOOR_PROPERTY_5 || self.floor_property == FLOOR_PROPERTY_12)
                    && (self.s.floor_dist < 100.0 || fall > 400 || (scene != SCENE_SHADOW_TEMPLE && fall > 200)))
                || (scene == SCENE_GANONS_TOWER_COLLAPSE_EXTERIOR && fall > 320)
            {
                if self.grounded() {
                    if self.floor_property == FLOOR_PROPERTY_5 {
                        let (pos, yaw) = (self.actor.world_pos, self.actor.shape_rot.y);
                        io.trigger_respawn(pos, yaw);
                    } else {
                        io.trigger_void_out();
                    }
                    io.transition.ty = TRANS_TYPE_FADE_BLACK_FAST;
                    self.sfx(PlayerSfx::NoPos(NA_SE_OC_ABYSS));
                } else {
                    drop(io);
                    self.func_80838F5C(env.data);
                    self.action_var2 = 9999;
                    self.action_var1 = if self.floor_property == FLOOR_PROPERTY_5 { -1 } else { 1 };
                    self.unk_A84 = self.actor.world_pos.y as i16;
                    return false;
                }
            }
            self.unk_A84 = self.actor.world_pos.y as i16;
        }
        false
    }

    /// `func_808332B8`: swimming without iron boots.
    fn func_808332B8(&self) -> bool {
        self.state1 & STATE1_27 != 0
    }

    /// `func_80838F5C`: start falling into the void (`Player_Action_8084F88C`).
    fn func_80838F5C(&mut self, data: &GameData) {
        self.setup_action(data, Action::VoidFall, 0);
        self.state1 |= STATE1_29 | STATE1_31;
        self.play_requests.push(PlayRequest::ChangeSetting(oot_game::camera::CAM_SET_FREE0));
    }

    /// `func_80838FB8`: falling off an edge while `func_80838F5C` is pending.
    fn func_80838FB8(&mut self, env: &Env) -> bool {
        if env.io.borrow().transition.trigger == TRANS_TRIGGER_OFF && self.state1 & STATE1_31 != 0 {
            self.func_80838F5C(env.data);
            // Player_AnimPlayLoop: LinkAnimation_PlayLoop.
            let a = env.data.anim("link_normal_landing_wait");
            self.skel.play_loop(env.data, a);
            self.play_voice_sfx(NA_SE_VO_LI_FALL_S);
            self.sfx(PlayerSfx::NoPos(NA_SE_OC_SECRET_WARP_IN));
            return true;
        }
        false
    }

    /// `Player_Action_8084F88C`: after 9 frames of falling, the void out (or respawn) starts.
    fn action_8084f88c(&mut self, env: &Env) {
        self.skel.update(env.data);
        let n = self.action_var2;
        self.action_var2 = self.action_var2.wrapping_add(1);
        let mut io = env.io.borrow_mut();
        if n > 8 && io.transition.trigger == TRANS_TRIGGER_OFF {
            if self.action_var1 != 0 {
                if self.action_var1 < 0 {
                    let (pos, yaw) = (self.actor.world_pos, self.actor.shape_rot.y);
                    io.trigger_respawn(pos, yaw);
                } else {
                    io.trigger_void_out();
                }
                io.transition.ty = TRANS_TYPE_FADE_BLACK_FAST;
                self.sfx(PlayerSfx::NoPos(NA_SE_OC_ABYSS));
            } else {
                io.transition.ty = TRANS_TYPE_FADE_BLACK;
                io.save.next_transition_type = TRANS_TYPE_FADE_BLACK;
            }
            io.transition.trigger = TRANS_TRIGGER_START;
        }
    }

    /// `Player_Action_80845CA4`: the walk through an exit (towards `unk_450` at the entrance speed) or
    /// in from an entrance (`av2.actionVar2` < 0 counts it down), then standing or running on.
    fn action_80845ca4(&mut self, env: &Env) {
        let data = env.data;
        if !self.action_handler_13(env) {
            if self.action_var2 == 0 {
                self.skel.update(data);
                self.door_timer -= 1;
                if self.door_timer <= 0 {
                    self.door_timer = 0;
                    self.linear_velocity = 0.1;
                    self.action_var2 = 1;
                }
            } else if self.action_var1 == 0 {
                let mut sp3c = 5.0 * self.s.speed_scale;
                if self.func_80845BA0(env, &mut sp3c, -1) < 30 {
                    self.action_var1 = 1;
                    self.state1 |= STATE1_29;
                    self.unk_450.x = self.unk_45C.x;
                    self.unk_450.z = self.unk_45C.z;
                }
            } else {
                let mut sp34 = 5.0;
                let mut sp30 = 20;
                if self.state1 & STATE1_0 != 0 {
                    sp34 = env.io.borrow().save.entrance_speed;
                } else if self.action_var2 < 0 {
                    self.action_var2 += 1;
                    sp34 = env.io.borrow().save.entrance_speed;
                    sp30 = -1;
                }
                let temp = self.func_80845BA0(env, &mut sp34, sp30);
                if self.action_var2 == 0 || (temp == 0 && self.linear_velocity == 0.0 && env.cam_state_flags & 0x10 != 0) {
                    self.play_requests.push(PlayRequest::CamDone);
                    // func_80845C68(play, respawn[DOWN].data):
                    let mut io = env.io.borrow_mut();
                    if io.save.respawn[RESPAWN_MODE_DOWN].data == 0 {
                        let (pos, yaw) = (self.actor.world_pos, self.actor.shape_rot.y);
                        io.setup_respawn_point(RESPAWN_MODE_DOWN, 0xDFF, pos, yaw);
                    }
                    io.save.respawn[RESPAWN_MODE_DOWN].data = 0;
                    drop(io);
                    if !self.action_handler_talk(env) {
                        self.func_8083CF5C(data);
                    }
                }
            }
        }
        if self.state1 & STATE1_11 != 0 {
            self.update_upper_body(env);
        }
    }

    /// `Player_ActionHandler_1`: A pressed with a door offering to open (`doorType`, set by `EnDoor_Idle`
    /// last frame): Link lines up at the door, 22 in front, and plays the opening animation
    /// for his side and age (`PLAYER_ANIMGROUP_doorA_free` .. `_12`), moved by it (`Player_StartAnimMovement`,
    /// 0x28F). A scene-exit door starts the exit under its far side (`Player_HandleExitsAndVoids`, entrance
    /// speed 2); any other gets the door camera and loads the room behind it.
    ///
    /// An ajar door (`PLAYER_DOORTYPE_AJAR`) shows text 0xD0 instead. A sliding door
    /// (`Door_Shutter`, `PLAYER_DOORTYPE_SLIDING`) is walked through instead
    /// (`Player_Action_80845CA4`), its camera index kept for the door (`cv.slidingDoorBgCamIndex`).
    ///
    /// Not ported: `Door_Killer`, holding Ruto (`ACTOR_EN_RU1`), and
    /// `Player_Action_TryOpeningDoor` (a cutscene's door walk).
    fn action_handler_1(&mut self, env: &Env) -> bool {
        if self.door_type == PLAYER_DOORTYPE_NONE || self.state1 & STATE1_11 != 0 {
            return false;
        }
        if !self.input.press.held(BTN_A) {
            return false;
        }
        let Some(dh) = self.door_actor else { return false };
        let Some(door) = env.actors.actor(dh) else { return false };
        if self.door_type <= PLAYER_DOORTYPE_AJAR {
            // doorActor->textId = 0xD0, then talking to it (and 0: no interrupt taken).
            self.play_requests.push(PlayRequest::SetTextId { actor: dh, text_id: 0xD0 });
            self.start_talking(env, dh, 0xD0);
            return false;
        }
        let mut door_direction = self.door_direction as i32;
        let (sp78, sp74) = (cos_s(door.shape_rot.y), sin_s(door.shape_rot.y));
        let (door_params, door_category) = (door.params, door.category);
        let entry = env.transi_actors.get((door_params as u16 >> oot_game::scene::TRANSITION_ACTOR_PARAMS_INDEX_SHIFT) as usize).copied();
        if self.door_type == PLAYER_DOORTYPE_SLIDING {
            // A Door_Shutter: Link walks up to it, 20 on, and through, to 120 past it
            // (Player_Action_80845CA4), facing the way it leads.
            let data = env.data;
            self.current_yaw = door.home_rot.y;
            if door_direction > 0 {
                self.current_yaw = self.current_yaw.wrapping_sub(-0x8000i32 as i16);
            }
            self.actor.shape_rot.y = self.current_yaw;
            if self.linear_velocity <= 0.0 {
                self.linear_velocity = 0.1;
            }
            let yaw = self.actor.shape_rot.y;
            self.func_80838E70(data, 50.0, yaw);
            self.action_var1 = 0;
            self.unk_447 = self.door_type;
            self.state1 |= STATE1_29;
            let pos = self.actor.world_pos;
            self.unk_450.x = pos.x + (door_direction as f32 * 20.0) * sp74;
            self.unk_450.z = pos.z + (door_direction as f32 * 20.0) * sp78;
            self.unk_45C.x = pos.x + (door_direction as f32 * -120.0) * sp74;
            self.unk_45C.z = pos.z + (door_direction as f32 * -120.0) * sp78;
            self.play_requests.push(PlayRequest::SlidingDoorActive(dh));
            self.func_80832224();
            if self.door_timer != 0 {
                // Unlocking first: Link stands until the door's timer runs out.
                self.action_var2 = 0;
                let a = self.anim(data, group::WAIT);
                self.anim_change_once_morph(data, a);
                self.skel.end_frame = 0.0;
            } else {
                self.linear_velocity = 0.1;
            }
            if door_category == oot_game::actor_ctx::ACTORCAT_DOOR
                && let Some(t) = entry
            {
                self.sliding_door_bg_cam_index = t.sides[if door_direction > 0 { 0 } else { 1 }].1 as i16;
                // (Actor_DisableLens: there's no lens.)
            }
        } else {
            self.open_door_with_handle(env, dh, &mut door_direction, sp78, sp74, entry);
        }
        let side = if door_direction > 0 { 0 } else { 1 };
        if door_category == oot_game::actor_ctx::ACTORCAT_DOOR
            && let Some(t) = entry
        {
            let front_room = t.sides[side].0;
            if front_room >= 0 && front_room != env.io.borrow().room {
                self.play_requests.push(PlayRequest::RoomLoad(front_room));
            }
        }
        self.play_requests.push(PlayRequest::DoorRoom { door: dh });
        true
    }

    /// `Player_ActionHandler_1`'s door with a handle (`En_Door`; `Door_Killer` isn't ported):
    /// Link lines up 22 in front and plays the opening for his side and age.
    fn open_door_with_handle(&mut self, env: &Env, dh: ActorHandle, door_direction_out: &mut i32, sp78: f32, sp74: f32, entry: Option<oot_game::scene::TransitionActorEntry>) {
        let Some(door) = env.actors.actor(dh) else { return };
        use crate::en_door::{DOOR_OPEN_ANIM_ADULT_L, DOOR_OPEN_ANIM_ADULT_R, DOOR_OPEN_ANIM_CHILD_L, DOOR_OPEN_ANIM_CHILD_R, DOOR_SCENEEXIT};
        let door_direction = *door_direction_out;
        let open_anim = match (door_direction < 0, self.adult) {
            (true, true) => DOOR_OPEN_ANIM_ADULT_L,
            (true, false) => DOOR_OPEN_ANIM_CHILD_L,
            (false, true) => DOOR_OPEN_ANIM_ADULT_R,
            (false, false) => DOOR_OPEN_ANIM_CHILD_R,
        };
        let group = match open_anim {
            DOOR_OPEN_ANIM_ADULT_L => 9,
            DOOR_OPEN_ANIM_CHILD_L => 10,
            DOOR_OPEN_ANIM_ADULT_R => 11,
            _ => 12,
        };
        let data = env.data;
        let anim = self.anim(data, group);
        let (door_pos, door_yaw, door_parent, door_params) = (door.world_pos, door.shape_rot.y, door.parent, door.params);
        self.setup_action(data, Action::DoorOpen, 0);
        self.put_away_held_item(data);
        self.actor.shape_rot.y = if door_direction < 0 { door_yaw } else { door_yaw.wrapping_add(-0x8000i32 as i16) };
        self.current_yaw = self.actor.shape_rot.y;
        let sp6c = door_direction as f32 * 22.0;
        self.actor.world_pos.x = door_pos.x + sp6c * sp74;
        self.actor.world_pos.z = door_pos.z + sp6c * sp78;
        // func_8083328C: LinkAnimation_PlayOnceSetSpeed at sWaterSpeedFactor.
        self.skel.play_once_set_speed(data, anim, self.s.speed_scale);
        if self.door_timer != 0 {
            self.skel.end_frame = 0.0;
        }
        self.func_80832224();
        self.start_anim_movement(0x28F);
        let mut door_direction = door_direction;
        // The second half of a double door (spawned as a child).
        if door_parent.is_some() {
            door_direction = -door_direction;
        }
        self.play_requests.push(PlayRequest::OpenDoor { door: dh, open_anim });
        // Not a Door_Killer: an EnDoor.
        self.state1 |= STATE1_29;
        // (Actor_DisableLens: there's no lens.)
        let side = if door_direction > 0 { 0 } else { 1 };
        if (door_params as u16 >> 7) & 7 == DOOR_SCENEEXIT {
            let check = Vec3::new(door_pos.x - sp6c * sp74, door_pos.y + 10.0, door_pos.z - sp6c * sp78);
            // BgCheck_EntityRaycastDown1. @bug (game): the poly's bgId is taken as BGCHECK_SCENE.
            let (_, ground_poly) = env.col.entity_raycast_down(check);
            if self.handle_exits_and_voids(env, ground_poly) {
                env.io.borrow_mut().save.entrance_speed = 2.0;
                env.io.borrow_mut().save.entrance_sound = NA_SE_OC_DOOR_OPEN;
            }
        } else if let Some(t) = entry {
            // 38, 26 and 10 frames times sInvWaterSpeedFactor (1).
            self.play_requests.push(PlayRequest::DoorCam { door: dh, bg_cam_index: t.sides[side].1 as i16, timers: [38, 26, 10] });
        }
        *door_direction_out = door_direction;
    }

    /// `Player_Action_80845EF8`: the door animation, then standing; the old room goes, the door camera
    /// is told Player is through, and the void-out point moves here.
    fn action_80845ef8(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5;
        let done = self.skel.update(data);
        self.update_upper_body(env);
        if done {
            if self.action_var2 == 0 {
                // DECR(doorTimer) == 0.
                if self.door_timer != 0 {
                    self.door_timer -= 1;
                }
                if self.door_timer == 0 {
                    self.action_var2 = 1;
                    self.skel.end_frame = self.skel.anim_length - 1.0;
                }
            } else {
                self.func_8083C0E8(data);
                if env.prev_room >= 0 {
                    self.play_requests.push(PlayRequest::RoomChangeDone);
                }
                self.play_requests.push(PlayRequest::CamDone);
                let (pos, yaw) = (self.actor.world_pos, self.actor.shape_rot.y);
                env.io.borrow_mut().setup_respawn_point(RESPAWN_MODE_DOWN, 0xDFF, pos, yaw);
            }
            return;
        }
        if self.state1 & STATE1_29 == 0 && self.skel.on_frame(15.0) {
            // play->func_11D54 (func_80853080): only a Door_Killer's opening gets here.
            self.func_80853080(data);
        }
    }

    /// `func_80845BA0`: walk towards `unk_450` at `speed`, stopping within `arg3`. Returns the
    /// distance left (0 while the animation holds Player in place).
    fn func_80845BA0(&mut self, env: &Env, speed: &mut f32, arg3: i32) -> i32 {
        let dx = self.unk_450.x - self.actor.world_pos.x;
        let dz = self.unk_450.z - self.actor.world_pos.z;
        let sp2c = (dx * dx + dz * dz).sqrt() as i32;
        let mut yaw = atan2_s(dz, dx);
        if sp2c < arg3 {
            *speed = 0.0;
            yaw = self.actor.shape_rot.y;
        }
        if self.func_80845964(env, *speed, yaw) {
            return 0;
        }
        sp2c
    }

    /// `func_80845964(play, this, NULL, speed, yaw, 2)`: the walk without a cutscene's
    /// positions. Standing still, it only plays the animation (true when it ends).
    fn func_80845964(&mut self, env: &Env, speed: f32, yaw: i16) -> bool {
        let data = env.data;
        if self.linear_velocity == 0.0 {
            return self.skel.update(data);
        }
        self.state2 |= STATE2_5;
        self.func_80841EE4(data);
        self.func_8083DF68(speed, yaw);
        if speed == 0.0 && self.linear_velocity == 0.0 {
            self.func_8083BF50(data);
        }
        false
    }

    /// `func_8083CF5C`: after an entrance, run on or stand.
    fn func_8083CF5C(&mut self, data: &GameData) {
        if self.linear_velocity != 0.0 {
            self.func_8083C858(data);
        } else {
            self.func_80839F90(data);
        }
    }

    /// `Player_StartCsAction`: with a cutscene mode pending (`unk_6AD` 3), Player's action becomes the
    /// cutscene's (`Player_Action_CsAction`), held (`PLAYER_STATE1_29`) when `Player_SetCsActionWithHaltedActors` set it.
    fn start_cs_action(&mut self, data: &GameData) -> bool {
        if self.unk_6AD == 3 {
            self.setup_action(data, Action::Cutscene, 0);
            if self.door_bg_cam_index != 0 {
                self.state1 |= STATE1_29;
            }
            self.func_80832318();
            return true;
        }
        false
    }

    /// `Player_ActionHandler_13` (interrupt 13, and the actions that check it first): with `unk_6AD` set
    /// and Link on the ground, swimming or riding, a cutscene mode takes over
    /// (`Player_StartCsAction`); else first person (`unk_6AD` 1 the look, 2 the aim) if the main
    /// camera takes its mode (`func_8083AD4C`), with `NA_SE_SY_CAMERA_ZOOM_UP`, or the error. The
    /// cutscene items and spells (`unk_6AD` 4: the trade items, bottles, the ocarina and magic)
    /// aren't ported: they log.
    fn action_handler_13(&mut self, env: &Env) -> bool {
        if self.unk_6AD != 0 && (self.func_808332B8() || self.grounded() || self.state1 & STATE1_23 != 0) {
            if !self.start_cs_action(env.data) {
                if self.unk_6AD == 4 {
                    self.note("Player_ActionHandler_13 with unk_6AD 4: the spells, trade items, bottles and the ocarina aren't ported");
                    return false;
                } else if self.func_8083ad4c(env) != oot_game::camera::CAM_MODE_NORMAL as i32 {
                    if self.state1 & STATE1_23 == 0 {
                        self.setup_action(env.data, Action::FirstPerson, 1);
                        self.action_var2 = 13;
                        self.func_8083B010();
                    }
                    self.state1 |= STATE1_20;
                    self.sfx(PlayerSfx::NoPos(NA_SE_SY_CAMERA_ZOOM_UP));
                    self.zero_speed_xz();
                    return true;
                } else {
                    self.unk_6AD = 0;
                    self.sfx(PlayerSfx::NoPos(NA_SE_SY_ERROR));
                    return false;
                }
            }
            self.func_80832224();
            return true;
        }
        false
    }

    /// `Player_ActionHandler_0` (interrupt 0): a cutscene mode (or an item) waiting (`unk_6AD`) takes
    /// over through `Player_ActionHandler_13`; a target Navi would talk about sets `PLAYER_STATE2_21`;
    /// else C-Up looks in first person (`func_8083B8F4`), or gives the error where it can't (not
    /// in the shops' and houses' fixed views).
    fn action_handler_0(&mut self, env: &Env) -> bool {
        if self.unk_6AD != 0 {
            self.action_handler_13(env);
            return true;
        }
        // (naviEnemyId is NAVI_ENEMY_NONE for every ported actor.)
        if self.focus_actor.and_then(|h| env.target(h)).is_some_and(|t| t.flags & (ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_TALK_WITH_C_UP) == (ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_TALK_WITH_C_UP)) {
            self.state2 |= STATE2_21;
        } else if self.navi_text_id == 0
            && !self.check_hostile_lock_on()
            && self.input.press.held(eng_input::pad::BTN_CUP)
            && env.scene_cam_type != oot_game::scene::SCENE_CAM_TYPE_FIXED_SHOP_VIEWPOINT
            && env.scene_cam_type != oot_game::scene::SCENE_CAM_TYPE_FIXED_TOGGLE_VIEWPOINT
            && !self.func_8083b8f4(env)
        {
            self.sfx(PlayerSfx::NoPos(NA_SE_SY_ERROR));
        }
        false
    }

    /// `Player_SetCsActionWithHaltedActors` on Player itself (`z_actor.c`): `csMode`, no actor, `doorBgCamIndex` 1.
    fn set_cs_action_with_halted_actors(&mut self, cs_mode: u8) {
        self.cs_mode = cs_mode;
        self.cs_actor = None;
        self.door_bg_cam_index = 1;
    }

    /// `Player_Action_CsAction`: the cutscene action. A new `csMode` runs its start (`D_80854B18`), and
    /// every frame its update (`D_80854E50`).
    fn action_cs_action(&mut self, env: &Env) {
        if self.cs_mode != self.prev_cs_mode {
            self.s.d_80858aa0 = self.skel.move_flags as i32;
            self.finish_anim_movement();
            self.prev_cs_mode = self.cs_mode;
            log::debug!("DEMO MODE={}", self.cs_mode);
            self.func_80852C0C(env.data, self.cs_mode);
            let m = self.cs_mode;
            self.func_80852B4C(env, None, m, true);
        }
        let m = self.cs_mode;
        self.func_80852B4C(env, None, m, false);
    }

    /// `func_80852B4C` with `D_80854B18[mode]` (`start`) or `D_80854E50[mode]` (the pack's
    /// `GameData::cs_mode_starts`, `cs_mode_updates`): a type's handler (`D_80854AA4`) with the
    /// entry's animation or sound table, or the entry's function. The functions that need what
    /// isn't ported (swimming, the ocarina, the items in hand, the effects) are logged.
    fn func_80852B4C(&mut self, env: &Env, cue: Option<CsCmdActorCue>, mode: u8, start: bool) {
        let tab = if start { &env.data.cs_mode_starts } else { &env.data.cs_mode_updates };
        match tab.get(mode as usize).cloned() {
            Some(e) if e.ty > 0 => self.cs_mode_type(env, &e),
            Some(e) if e.ty < 0 => self.cs_mode_func(env, cue, &e.name),
            Some(_) => {}
            None => self.note(format!("cutscene mode {mode} past the tables")),
        }
        if self.s.d_80858aa0 & 4 != 0 && self.skel.move_flags & 4 == 0 {
            self.skel.morph[0][1] = (self.skel.morph[0][1] as f32 / self.age.translation_scale) as i16;
            self.s.d_80858aa0 = 0;
        }
    }

    /// `D_80854AA4[type](play, this, ptr)`: the cutscene modes' typed handlers.
    fn cs_mode_type(&mut self, env: &Env, e: &oot_game::data::CsModeEntry) {
        let data = env.data;
        let a = match e.anim {
            Some(a) => a,
            // Types 1, 11 and 18 take no animation.
            None if matches!(e.ty, 1 | 11 | 18) => 0,
            None => {
                self.note(format!("cutscene mode type {} without an animation", e.ty));
                return;
            }
        };
        match e.ty {
            // func_80851008.
            1 => self.zero_speed_xz(),
            // func_80851030: Player_AnimChangeOnceMorphZeroRootYawSpeed.
            2 => self.anim_change_once_morph_zero_root_yaw_speed(data, a),
            // func_80851094: Player_AnimChangeOnceMorphAdjustedZeroRootYawSpeed.
            3 => self.anim_change_once_morph_adjusted_zero_root_yaw_speed(data, a),
            // func_808510B4: Player_AnimChangeLoopMorphAdjustedZeroRootYawSpeed.
            4 => self.anim_change_loop_morph_adjusted_zero_root_yaw_speed(data, a),
            // func_808510D4: Player_AnimReplaceNormalPlayOnceAdjusted (Player_AnimReplacePlayOnceAdjusted(play, this, anim, 0x1C)).
            5 => self.anim_replace_play_once_adjusted(data, a, 0x1C),
            // func_808510F4: Player_AnimReplacePlayOnce(play, this, anim, 0x9C).
            6 => self.anim_replace_play_once(data, a, 0x9C),
            // func_80851114: Player_AnimReplaceNormalPlayLoopAdjusted (Player_AnimReplacePlayLoopAdjusted(play, this, anim, 0x1C)).
            7 => self.anim_replace_play_loop_adjusted(data, a, 0x1C),
            // func_80851134: Player_AnimReplacePlayLoop(play, this, anim, 0x9C).
            8 => self.anim_replace_play_loop(data, a, 0x9C),
            // func_80851154: Player_AnimPlayOnce (LinkAnimation_PlayOnce).
            9 => self.skel.play_once(data, a),
            // func_80851174: Player_AnimPlayLoop (LinkAnimation_PlayLoop).
            10 => self.skel.play_loop(data, a),
            // func_808511D4: LinkAnimation_Update.
            11 => {
                self.skel.update(data);
            }
            // func_808511FC: at the animation's end, Player_AnimChangeLoopMorphAdjustedZeroRootYawSpeed, av2.actionVar2 1.
            12 => {
                if self.skel.update(data) {
                    self.anim_change_loop_morph_adjusted_zero_root_yaw_speed(data, a);
                    self.action_var2 = 1;
                }
            }
            // func_80851294: at the end, Player_AnimReplaceNormalPlayLoopAdjusted, av2.actionVar2 1.
            13 => self.func_80851294(data, a),
            // func_80851050: Player_ZeroRootLimbYaw, Player_AnimChangeFreeze, Player_ZeroSpeedXZ.
            14 => {
                self.zero_root_limb_yaw();
                self.skel.change(data, a, 1.0, 0.0, 0.0, ANIMMODE_ONCE, 0.0);
                self.zero_speed_xz();
            }
            // func_80851194: Player_AnimPlayOnceAdjusted (LinkAnimation_PlayOnceSetSpeed 2/3).
            15 => self.skel.play_once_set_speed(data, a, 2.0 / 3.0),
            // func_808511B4: Player_AnimPlayLoopAdjusted (LinkAnimation_PlayLoopSetSpeed 2/3).
            16 => self.skel.play_loop_set_speed(data, a, 2.0 / 3.0),
            // func_80851248: at the end, Player_FinishAnimMovement and Player_AnimPlayLoopAdjusted.
            17 => {
                if self.skel.update(data) {
                    self.finish_anim_movement();
                    self.skel.play_loop_set_speed(data, a, 2.0 / 3.0);
                }
            }
            // func_808512E0: LinkAnimation_Update and the sound table.
            18 => {
                self.skel.update(data);
                let t = env.audio.player_anim_sfx(&e.name).to_vec();
                self.process_anim_sfx_list(&t);
            }
            t => self.note(format!("cutscene mode type {t}")),
        }
    }

    /// `Player_AnimChangeOnceMorphAdjustedZeroRootYawSpeed`: the animation once at 2/3, from its last frame's morph, standing.
    fn anim_change_once_morph_adjusted_zero_root_yaw_speed(&mut self, data: &GameData, anim: AnimId) {
        self.zero_root_limb_yaw();
        let last = data.anims[anim].last_frame();
        self.skel.change(data, anim, 2.0 / 3.0, 0.0, last, ANIMMODE_ONCE, -8.0);
        self.zero_speed_xz();
    }

    /// `Player_AnimChangeLoopMorphAdjustedZeroRootYawSpeed`: the animation looped at 2/3, standing.
    fn anim_change_loop_morph_adjusted_zero_root_yaw_speed(&mut self, data: &GameData, anim: AnimId) {
        self.zero_root_limb_yaw();
        self.skel.change(data, anim, 2.0 / 3.0, 0.0, 0.0, ANIMMODE_LOOP, -8.0);
        self.zero_speed_xz();
    }

    /// `Player_AnimReplacePlayOnceAdjusted`: `Player_AnimReplacePlayOnceSetSpeed` at 2/3: the animation once with `moveFlags`.
    fn anim_replace_play_once_adjusted(&mut self, data: &GameData, anim: AnimId, flags: u16) {
        self.skel.play_once_set_speed(data, anim, 2.0 / 3.0);
        self.start_anim_movement(flags);
    }

    /// `Player_AnimReplacePlayLoopAdjusted`: `Player_AnimReplacePlayLoopSetSpeed` at 2/3: the animation looped with `moveFlags`.
    fn anim_replace_play_loop_adjusted(&mut self, data: &GameData, anim: AnimId, flags: u16) {
        self.skel.play_loop_set_speed(data, anim, 2.0 / 3.0);
        self.start_anim_movement(flags);
    }

    /// `func_80851294`: at the animation's end, `Player_AnimReplaceNormalPlayLoopAdjusted`, `av2.actionVar2` 1.
    fn func_80851294(&mut self, data: &GameData, anim: AnimId) {
        if self.skel.update(data) {
            self.anim_replace_play_loop_adjusted(data, anim, 0x1C);
            self.action_var2 = 1;
        }
    }

    /// `func_80851F14`: at the animation's end the next one looped at 2/3; before it, its
    /// sounds.
    fn func_80851F14(&mut self, env: &Env, anim: AnimId, table: &str) {
        let data = env.data;
        if self.skel.update(data) {
            self.skel.play_loop_set_speed(data, anim, 2.0 / 3.0);
            self.action_var2 = 1;
        } else if self.action_var2 == 0 {
            let t = env.audio.player_anim_sfx(table).to_vec();
            self.process_anim_sfx_list(&t);
        }
    }

    /// `func_80852414`: `func_80851294`, and until its end, the sounds.
    fn func_80852414(&mut self, env: &Env, anim: AnimId, table: &str) {
        self.func_80851294(env.data, anim);
        if self.action_var2 == 0 {
            let t = env.audio.player_anim_sfx(table).to_vec();
            self.process_anim_sfx_list(&t);
        }
    }

    /// `func_808520BC`: Link along the cue, from its start to its end by the script's frame.
    fn func_808520BC(&mut self, env: &Env, cue: &CsCmdActorCue) {
        let start = Vec3::new(cue.start_pos.x as f32, cue.start_pos.y as f32, cue.start_pos.z as f32);
        let dist = Vec3::new(cue.end_pos.x as f32, cue.end_pos.y as f32, cue.end_pos.z as f32) - start;
        let sp4 = (env.cs_frames as i32 - cue.start_frame as i32) as f32 / (cue.end_frame as i32 - cue.start_frame as i32) as f32;
        self.actor.world_pos = dist * sp4 + start;
    }

    /// The cutscene modes' functions (`struct_80854B18`'s `func`), by name.
    fn cs_mode_func(&mut self, env: &Env, cue: Option<CsCmdActorCue>, name: &str) {
        let data = env.data;
        match name {
            "func_808515A4" => self.func_808515A4(env),
            "func_808514C0" => self.func_808514C0(env),
            "func_8085157C" | "func_80852234" => {
                self.skel.update(data);
            }
            "func_80851998" => self.func_80851998(env, cue),
            "func_808519C0" => self.func_808519C0(env, cue),
            "func_80852C50" => self.func_80852C50(env),
            "func_80852944" => self.func_80852944(env),
            "func_80851688" => self.func_80851688(env),
            "func_80851750" => {
                self.skel.update(data);
                let t = env.audio.player_anim_sfx("D_80855188").to_vec();
                self.process_anim_sfx_list(&t);
            }
            "func_80851788" => {
                // Walking to unk_450 (func_80851828 after it).
                self.state1 &= !STATE1_25;
                // Math_Vec3f_Yaw.
                let yaw = vec3f_yaw(self.actor.world_pos, self.unk_450);
                self.current_yaw = yaw;
                self.actor.shape_rot.y = yaw;
                self.actor.world_rot.y = yaw;
                if self.linear_velocity <= 0.0 {
                    self.linear_velocity = 0.1;
                } else if self.linear_velocity > 2.5 {
                    self.linear_velocity = 2.5;
                }
            }
            "func_80851828" => {
                let mut sp1c = 2.5;
                self.func_80845BA0(env, &mut sp1c, 10);
                // SCENE_JABU_JABU_BOSS (Jabu-Jabu's boss room): wait on the message box.
                if env.scene_id == 0x12 {
                    let none = env.msg_state == oot_game::message::TEXT_STATE_NONE;
                    if (self.action_var2 == 0 && none) || (self.action_var2 != 0 && !none) {
                        return;
                    }
                }
                self.action_var2 += 1;
                if self.action_var2 > 20 {
                    self.cs_mode = 0xB;
                }
            }
            "func_808518DC" => self.func_8083CEAC(data),
            "func_8085190C" => {
                self.func_80851314(env);
                if self.action_var2 != 0 {
                    if self.skel.update(data) {
                        let a = self.func_808334E4(data);
                        self.skel.play_loop(data, a);
                        self.action_var2 = 0;
                    }
                    self.func_80833C3C();
                } else {
                    self.func_808401B0(data);
                }
            }
            "func_808519EC" => {
                // sPedestalPos, facing -z, the age's timeTravelStartAnim at 2/3, moveFlags 0x28F.
                self.actor.world_pos = Vec3::new(-1.0, 70.0, 20.0);
                self.actor.shape_rot.y = i16::MIN;
                let a = self.age.climb.time_travel_start_anim;
                self.skel.play_once_set_speed(data, a, 2.0 / 3.0);
                self.start_anim_movement(0x28F);
            }
            "func_80851B90" => {
                let a = data.anim("link_demo_warp");
                self.skel.change(data, a, -(2.0 / 3.0), 12.0, 12.0, ANIMMODE_ONCE, 0.0);
            }
            "func_80851BE8" => {
                self.skel.update(data);
                self.action_var2 += 1;
                if self.action_var2 >= 180 {
                    if self.action_var2 == 180 {
                        let a = data.anim("link_okarina_warp_goal");
                        let last = data.anims[a].last_frame();
                        self.skel.change(data, a, 2.0 / 3.0, 10.0, last, ANIMMODE_ONCE, -8.0);
                    }
                    let t = env.audio.player_anim_sfx("D_808551B4").to_vec();
                    self.process_anim_sfx_list(&t);
                }
            }
            "func_80851CA4" => {
                if self.skel.update(data) && self.action_var2 == 0 && self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
                    self.skel.play_once(data, data.anim("link_normal_back_downB"));
                    self.action_var2 = 1;
                }
                if self.action_var2 != 0 {
                    self.decelerate_to_zero();
                }
            }
            "func_80851DEC" | "func_80851E28" => {
                // Math_StepToS(&this->actor.shape.face, 0 or 2, 1).
                self.skel.update(data);
                let target = if name == "func_80851DEC" { 0 } else { 2 };
                if self.face < target {
                    self.face += 1;
                } else if self.face > target {
                    self.face -= 1;
                }
            }
            "func_80851E64" => self.anim_replace_play_once_adjusted(data, data.anim("link_swimer_swim_get"), 0x98),
            "func_80851E90" => self.func_80851E90(data),
            "func_80851ECC" => self.func_80851ECC(data),
            "func_80851F84" => self.func_80851F84(data),
            "func_80851FB0" => self.func_80851FB0(env),
            "func_80852048" => {
                self.skel.update(data);
                let t = env.audio.player_anim_sfx("D_808551C8").to_vec();
                self.process_anim_sfx_list(&t);
            }
            "func_80852080" => {
                self.anim_replace_play_once_adjusted(data, data.anim("clink_demo_futtobi"), 0x9D);
                self.play_voice_sfx(NA_SE_VO_LI_FALL_L);
            }
            "func_80852174" => {
                if let Some(c) = cue {
                    self.func_808520BC(env, &c);
                }
                self.skel.update(data);
                let t = env.audio.player_anim_sfx("D_808551D8").to_vec();
                self.process_anim_sfx_list(&t);
            }
            "func_808521B8" => {
                if let Some(c) = cue {
                    self.func_808520BC(env, &c);
                }
                self.skel.update(data);
            }
            "func_808521F4" => {
                let a = self.anim(data, 44);
                self.anim_change_once_morph(data, a);
                self.zero_speed_xz();
            }
            "func_8085225C" => self.start_anim_movement(0x98),
            // func_80852280: actor.draw = Player_Draw (Player is always drawn here).
            "func_80852280" | "func_80852544" | "func_80852554" => {}
            "func_80852328" => self.func_80851F14(env, data.anim("link_demo_furimuki2_wait"), "D_808551E0"),
            "func_80852358" => self.func_80851F14(env, data.anim("link_demo_nozokikomi_wait"), "D_808551E8"),
            "func_80852388" => {
                if self.skel.update(data) {
                    self.skel.play_loop_set_speed(data, data.anim("demo_link_twait"), 2.0 / 3.0);
                    self.action_var2 = 1;
                }
                // (rightHandType: the hand's model, PLAYER_MODELTYPE_LH_OPEN at 900 or RH_FF.)
            }
            "func_80852450" => self.func_80852414(env, data.anim("clink_demo_koutai_wait"), "D_808551F0"),
            "func_80852480" => self.func_80852414(env, data.anim("link_demo_kakeyori_wait"), "D_808551F8"),
            "func_80852564" => {
                self.state3 |= STATE3_1;
                self.linear_velocity = 2.0;
                self.actor.velocity.y = -1.0;
                self.skel.play_once(data, data.anim("link_normal_back_downA"));
                self.play_voice_sfx(NA_SE_VO_LI_FALL_L);
            }
            "func_808525C0" => match self.action_var2 {
                // D_808551FC: the knockdown's three stages.
                0 => self.action_8084377c(env),
                1 => self.action_80843954(env),
                2 => self.action_80843a38(env),
                _ => {}
            },
            n => self.note(format!("cutscene mode function {n} not ported (swimming, the ocarina, the items in hand or the effects)")),
        }
    }

    /// `Player_AnimChangeOnceMorphZeroRootYawSpeed` (type 2): the animation once (`Player_AnimChangeOnceMorph`), standing.
    fn anim_change_once_morph_zero_root_yaw_speed(&mut self, data: &GameData, anim: AnimId) {
        self.zero_root_limb_yaw();
        self.anim_change_once_morph(data, anim);
        self.zero_speed_xz();
    }

    /// `Player_AnimReplacePlayOnce`: `Player_AnimReplacePlayOnceSetSpeed` at speed 1: the animation once with `moveFlags`.
    fn anim_replace_play_once(&mut self, data: &GameData, anim: AnimId, flags: u16) {
        self.skel.play_once_set_speed(data, anim, 1.0);
        self.start_anim_movement(flags);
    }

    /// `Player_AnimReplacePlayLoop`: `Player_AnimReplacePlayLoopSetSpeed` at speed 1: the animation looped with `moveFlags`.
    fn anim_replace_play_loop(&mut self, data: &GameData, anim: AnimId, flags: u16) {
        self.skel.play_loop_set_speed(data, anim, 1.0);
        self.start_anim_movement(flags);
    }

    /// `func_80851F84` (mode 38's start): asleep in bed, no shadow, `clink_op3_wait1` looped
    /// (`func_80851134`: `Player_AnimReplacePlayLoop(play, this, anim, 0x9C)`).
    fn func_80851F84(&mut self, data: &GameData) {
        self.shadow_feet = false;
        self.anim_replace_play_loop(data, data.anim("clink_op3_wait1"), 0x9C);
    }

    /// `func_80851E90` (mode 39's start): tossing (`clink_op3_negaeri`), groaning.
    fn func_80851E90(&mut self, data: &GameData) {
        self.anim_replace_play_once(data, data.anim("clink_op3_negaeri"), 0x9C);
        self.play_voice_sfx(NA_SE_VO_LI_GROAN);
    }

    /// `func_80851ECC` (mode 39): after the toss, `clink_op3_wait2` looped.
    fn func_80851ECC(&mut self, data: &GameData) {
        if self.skel.update(data) {
            self.anim_replace_play_loop(data, data.anim("clink_op3_wait2"), 0x9C);
        }
    }

    /// `func_80851FB0` (mode 40): sitting up, with `D_808551BC`'s sounds (the sigh, the slips
    /// off the bed); then `clink_op3_wait3` looped. The shadow comes back on frame 240
    /// (`ActorShadow_DrawFeet`).
    fn func_80851FB0(&mut self, env: &Env) {
        let data = env.data;
        if self.skel.update(data) {
            self.anim_replace_play_loop(data, data.anim("clink_op3_wait3"), 0x9C);
            self.action_var2 = 1;
        } else if self.action_var2 == 0 {
            let t = env.audio.player_anim_sfx("D_808551BC").to_vec();
            self.process_anim_sfx_list(&t);
            if self.skel.on_frame(240.0) {
                self.shadow_feet = true;
            }
        }
    }

    /// `func_80852C0C`: modes other than 1, 8, 0x31 and 7 drop what Link holds
    /// (`Player_DetachHeldActor`).
    fn func_80852C0C(&mut self, data: &GameData, cs_mode: u8) {
        if cs_mode != 1 && cs_mode != 8 && cs_mode != 0x31 && cs_mode != 7 {
            self.detach_held_actor(data);
        }
    }

    /// `func_80852C50` (mode 6): Link follows the script's cues (`linkAction`), each mapped to
    /// a mode by `sCueToCsActionMap`. A cue's start puts him at its start (`func_808529D0`) or, for the
    /// walks (3, 4), only if he's far from it (`func_80852A54`). When the script ends
    /// (`CS_STATE_STOP`), mode 7.
    fn func_80852C50(&mut self, env: &Env) {
        let link = env.cs_link_action;
        if env.cs_state == oot_game::cutscene::CS_STATE_STOP {
            self.set_cs_action_with_halted_actors(7);
            self.cue_id = 0;
            self.zero_speed_xz();
            return;
        }
        let Some(link) = link else {
            self.actor.flags &= !ACTOR_FLAG_INSIDE_CULLING_VOLUME;
            return;
        };
        if self.cue_id != link.action {
            let sp24 = s_cue_to_cs_action_map(link.action);
            if sp24 >= 0 {
                if sp24 == 3 || sp24 == 4 {
                    self.func_80852A54(env, &link);
                } else {
                    self.func_808529D0(env, &link);
                }
            }
            self.s.d_80858aa0 = self.skel.move_flags as i32;
            self.finish_anim_movement();
            log::debug!("TOOL MODE={sp24}");
            let m = sp24.unsigned_abs();
            self.func_80852C0C(env.data, m);
            self.func_80852B4C(env, Some(link), m, true);
            self.action_var2 = 0;
            self.action_var1 = 0;
            self.cue_id = link.action;
        }
        let m = s_cue_to_cs_action_map(self.cue_id).unsigned_abs();
        self.func_80852B4C(env, Some(link), m, false);
    }

    /// `func_808529D0`: Link at the cue's start (a child in Kokiri Forest 1 lower), facing its
    /// rotation.
    fn func_808529D0(&mut self, env: &Env, cue: &CsCmdActorCue) {
        self.actor.world_pos.x = cue.start_pos.x as f32;
        self.actor.world_pos.y = cue.start_pos.y as f32;
        if env.scene_id == SCENE_KOKIRI_FOREST && !self.adult {
            self.actor.world_pos.y -= 1.0;
        }
        self.actor.world_pos.z = cue.start_pos.z as f32;
        self.actor.shape_rot.y = cue.rot[1];
        self.current_yaw = cue.rot[1];
        // Put there at once: the renderer doesn't blend Link across the jump.
        self.actor.teleported = true;
    }

    /// `func_80852A54`: a walk cue moves Link to its start only if he stands more than 50 from
    /// it or facing more than a quarter turn away.
    fn func_80852A54(&mut self, env: &Env, cue: &CsCmdActorCue) {
        let dx = cue.start_pos.x as f32 - self.actor.world_pos.x as i32 as f32;
        let dy = cue.start_pos.y as f32 - self.actor.world_pos.y as i32 as f32;
        let dz = cue.start_pos.z as f32 - self.actor.world_pos.z as i32 as f32;
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        let yaw_diff = cue.rot[1].wrapping_sub(self.actor.shape_rot.y);
        if self.linear_velocity == 0.0 && (dist > 50.0 || abs16(yaw_diff) > 0x4000) {
            self.func_808529D0(env, cue);
        }
        self.skel.move_flags = 0;
        self.zero_root_limb_yaw();
    }

    /// `Player_ZeroRootLimbYaw`.
    fn zero_root_limb_yaw(&mut self) {
        self.skel.joint[1][1] = 0;
    }

    /// `func_808515A4` (the start of modes 1, 8 and 0x31): Link stands in the cutscene's wait
    /// (`PLAYER_ANIMGROUP_nwait`), looping. (Swimming, `func_80851368`, isn't ported.)
    fn func_808515A4(&mut self, env: &Env) {
        let data = env.data;
        if self.func_808332B8() {
            self.note("cutscene hold while swimming (func_80851368) not ported");
            return;
        }
        let anim = self.anim(data, 44);
        if self.cue_id == 6 || self.cue_id == 0x2E {
            // Player_AnimPlayOnce: LinkAnimation_PlayOnce.
            self.skel.play_once(data, anim);
        } else {
            self.zero_root_limb_yaw();
            let last = data.anims[anim].last_frame();
            self.skel.change(data, anim, 2.0 / 3.0, 0.0, last, ANIMMODE_LOOP, -4.0);
        }
        self.zero_speed_xz();
    }

    /// `func_80851314`: Link faces the actor the mode is about (`csActor`), if it's still there.
    fn func_80851314(&mut self, env: &Env) {
        if self.cs_actor.is_some_and(|h| env.target(h).is_none_or(|a| a.killed)) {
            self.cs_actor = None;
        }
        self.focus_actor = self.cs_actor;
        if self.focus_actor.is_some() {
            self.actor.shape_rot.y = self.func_8083DB98(env, false);
        }
    }

    /// `func_808514C0` (mode 1, talking to an actor with no text): facing it, the animation,
    /// and a picked-up item offer (`textId` 0xFFFF).
    fn func_808514C0(&mut self, env: &Env) {
        self.func_80851314(env);
        if self.func_808332B8() {
            self.note("cutscene mode 1 while swimming (func_808513BC) not ported");
            return;
        }
        self.skel.update(env.data);
        // func_8008F128 (the hookshot or boomerang out) and PLAYER_STATE1_CARRYING_ACTOR: nothing held.
        if self.state1 & STATE1_11 != 0 {
            self.update_upper_body(env);
            return;
        }
        if self.interact_range_actor.and_then(|h| env.target(h)).is_some_and(|a| a.text_id == 0xFFFF) {
            self.action_handler_2(env);
        }
    }

    /// `func_80851688` (modes 8 and 0x31): held still, the wait playing; mode 0x31 ends by
    /// itself once no script runs (mode 7). (`func_8084B3CC`, the shooting gallery, never
    /// applies.)
    fn func_80851688(&mut self, env: &Env) {
        if self.cs_mode == 0x31 && env.cs_state == oot_game::cutscene::CS_STATE_IDLE {
            self.set_cs_action_with_halted_actors(7);
            return;
        }
        if self.func_808332B8() {
            self.note("cutscene hold while swimming (func_808513BC) not ported");
            return;
        }
        self.skel.update(env.data);
        if self.state1 & STATE1_11 != 0 {
            self.update_upper_body(env);
        }
    }

    /// `func_80851998` (mode 3): `func_80845964(play, this, arg2, 0.0f, 0, 0)`.
    fn func_80851998(&mut self, env: &Env, cue: Option<CsCmdActorCue>) {
        self.func_80845964_cs(env, cue, 0.0, 0, 0);
    }

    /// `func_808519C0` (mode 4): `func_80845964(play, this, arg2, 0.0f, 0, 1)`.
    fn func_808519C0(&mut self, env: &Env, cue: Option<CsCmdActorCue>) {
        self.func_80845964_cs(env, cue, 0.0, 0, 1);
    }

    /// `func_80845964` with a cue (`arg5` 0 or 1): the walk towards the cue's end, at the speed
    /// that gets there by its end frame (`R_UPDATE_RATE` × 0.5 a unit); with `arg5` 1 it stops
    /// short, leaving four times the cue's own pace for the slow-down, and stands once still.
    fn func_80845964_cs(&mut self, env: &Env, cue: Option<CsCmdActorCue>, mut speed: f32, mut yaw: i16, arg5: i32) -> bool {
        let data = env.data;
        if arg5 != 0 && self.linear_velocity == 0.0 {
            return self.skel.update(data);
        }
        if arg5 != 2
            && let Some(c) = cue
        {
            let sp34 = 3.0 * 0.5;
            let self_dist_x = c.end_pos.x as f32 - self.actor.world_pos.x;
            let self_dist_z = c.end_pos.z as f32 - self.actor.world_pos.z;
            let sp28 = (self_dist_x * self_dist_x + self_dist_z * self_dist_z).sqrt() / sp34;
            let sp24 = (c.end_frame as i32 - env.cs_frames as i32) + 1;
            yaw = atan2_s(self_dist_z, self_dist_x);
            if arg5 == 1 {
                let dist_x = (c.end_pos.x - c.start_pos.x) as f32;
                let dist_z = (c.end_pos.z - c.start_pos.z) as f32;
                let temp = ((((dist_x * dist_x + dist_z * dist_z).sqrt() / sp34) / (c.end_frame as i32 - c.start_frame as i32) as f32) / 1.5 * 4.0) as i32;
                if temp >= sp24 {
                    yaw = self.actor.shape_rot.y;
                    speed = 0.0;
                } else {
                    speed = sp28 / ((sp24 - temp) + 1) as f32;
                }
            } else {
                speed = sp28 / sp24 as f32;
            }
        }
        self.state2 |= STATE2_5;
        self.func_80841EE4(data);
        self.func_8083DF68(speed, yaw);
        if speed == 0.0 && self.linear_velocity == 0.0 {
            self.func_8083BF50(data);
        }
        false
    }

    /// `func_80852944` (mode 7): the cutscene is over: standing (or treading water), then the
    /// talk and pick-up interrupts, and `csMode` 0.
    fn func_80852944(&mut self, env: &Env) {
        let data = env.data;
        if self.func_808332B8() {
            self.func_80838F18(data);
            self.func_80832340();
        } else {
            self.func_8083C148(data);
            if !self.action_handler_talk(env) {
                self.action_handler_2(env);
            }
        }
        self.cs_mode = 0;
        self.unk_6AD = 0;
    }

    /// `func_8083C148`.
    fn func_8083C148(&mut self, data: &GameData) {
        if self.state3 & STATE3_7 == 0 {
            self.func_8083B010();
            if self.state1 & STATE1_27 != 0 {
                self.func_80838F18(data);
            } else {
                self.func_80839F90(data);
            }
            if self.unk_6AD < 4 {
                self.unk_6AD = 0;
            }
        }
        self.state1 &= !(STATE1_13 | STATE1_14 | STATE1_20);
    }

    /// `func_8083B010`: the head and upper body straight, the focus facing Link's way.
    fn func_8083B010(&mut self) {
        self.actor.focus_rot = Rot { x: 0, y: self.actor.shape_rot.y, z: 0 };
        self.head_limb_rot_x = 0;
        self.head_limb_rot_y = 0;
        self.head_limb_rot_z = 0;
        self.upper_limb_rot_x = 0;
        self.upper_limb_rot_y = 0;
        self.upper_limb_rot_z = 0;
    }

    /// `Player_ActionHandler_Talk` (interrupt 4): A talks to the actor that offered this frame
    /// (`targetActor`, from `Actor_OfferTalkExchange`), or C-Up talks to Navi: about the target, or her
    /// own text (`naviTextId`, at once when negative). On the ground (or swimming at the
    /// surface), when nothing else is targeted.
    ///
    /// No ported actor has a `naviEnemyId`, so a target Navi speaks about is an
    /// `ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_TALK_WITH_C_UP` one. Holding actors isn't ported (`PLAYER_STATE1_CARRYING_ACTOR` is
    /// never set).
    fn action_handler_talk(&mut self, env: &Env) -> bool {
        let mut sp34 = self.target_actor;
        let mut sp30 = self.focus_actor;
        let mut sp2c: Option<ActorHandle> = None;
        let navi_actor: Option<ActorHandle> = self.navi_actor;
        let checkable = |h: Option<ActorHandle>| h.and_then(|h| env.actors.actor(h)).is_some_and(|a| a.flags & (ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_TALK_WITH_C_UP) == (ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_TALK_WITH_C_UP));
        let sp24 = checkable(sp30);
        let mut sp28 = false;
        if sp24 || self.navi_text_id != 0 {
            sp28 = self.navi_text_id < 0 && ((self.navi_text_id as i32).abs() & 0xFF00) != 0x200;
            if sp28 || !sp24 {
                sp2c = navi_actor;
                if sp28 {
                    sp30 = None;
                    sp34 = None;
                }
            } else {
                sp2c = sp30;
            }
        }
        if sp34.is_none() && sp2c.is_none() {
            return false;
        }
        if !(sp30.is_none() || sp30 == sp34 || sp30 == sp2c) {
            return false;
        }
        if self.state1 & STATE1_11 != 0 {
            return false;
        }
        if !(self.grounded() || self.state1 & STATE1_23 != 0 || (self.func_808332B8() && self.state2 & STATE2_10 == 0)) {
            return false;
        }
        if let Some(t) = sp34 {
            self.state2 |= STATE2_1;
            let flag16 = env.actors.actor(t).is_some_and(|a| a.flags & ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED != 0);
            if self.input.press.held(BTN_A) || flag16 {
                sp2c = None;
            } else if sp2c.is_none() {
                return false;
            }
        }
        let mut text_override = None;
        if let Some(n) = sp2c {
            if !sp28 {
                self.state2 |= STATE2_21;
            }
            if !self.input.press.held(eng_input::pad::BTN_CUP) && !sp28 {
                return false;
            }
            sp34 = Some(n);
            self.target_actor = None;
            if sp28 || !sp24 {
                let id = if self.navi_text_id >= 0 { self.navi_text_id } else { -self.navi_text_id };
                text_override = Some(id as u16);
            }
            // (sp2c->naviEnemyId + 0x600: no actor has one.)
        }
        let Some(t) = sp34 else { return false };
        let Some(text_id) = text_override.or_else(|| env.actors.actor(t).map(|a| a.text_id)) else { return false };
        if let Some(id) = text_override {
            self.play_requests.push(PlayRequest::SetTextId { actor: t, text_id: id });
        }
        // this->currentMask = sSavedCurrentMask (masks aren't ported).
        self.start_talking(env, t, text_id);
        true
    }

    /// `Player_StartTalking`: start talking with `actor` (whose `textId` is `text_id`). The actor
    /// gets the talk request (`ACTOR_FLAG_TALK`) and Player its text; an NPC is faced after
    /// putting the item away, anything else at once. Link steps back first when closer than
    /// 40.
    fn start_talking(&mut self, env: &Env, h: ActorHandle, text_id: u16) {
        let data = env.data;
        let Some(actor) = env.actors.actor(h) else { return };
        let (category, xz_dist, flags) = (actor.category, actor.xz_dist_to_player, actor.flags);
        let is_navi = self.navi_actor == Some(h);
        if self.target_actor.is_some() || is_navi || flags & (ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_TALK_WITH_C_UP) == (ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_TALK_WITH_C_UP) {
            self.play_requests.push(PlayRequest::TalkRequest(h));
        }
        self.target_actor = Some(h);
        self.exchange_item_id = 0;
        if text_id == 0xFFFF {
            // Player_SetCsActionWithHaltedActors(play, actor, 1): a cutscene's talk, not ported.
            self.note("a talk with text 0xFFFF (a cutscene) isn't ported");
            self.play_requests.push(PlayRequest::TalkRequest(h));
            self.put_away_held_item(data);
            return;
        }
        if self.actor.flags & ACTOR_FLAG_TALK != 0 {
            self.actor.text_id = 0;
        } else {
            self.actor.flags |= ACTOR_FLAG_TALK;
            self.actor.text_id = text_id;
        }
        // (Riding isn't ported.)
        let backspace = data.anim("link_normal_backspace");
        if self.func_808332B8() {
            self.setup_wait_for_put_away(data, env, A74::Talk);
            self.anim_change_loop_slow_morph(data, data.anim("link_swimer_swim_wait"));
        } else if category != ACTORCAT_NPC {
            // (Or the fishing pole in hand, which isn't ported.)
            self.setup_talk(data);
            if self.state1 & STATE1_4 == 0 {
                if !is_navi && xz_dist < 40.0 {
                    self.skel.play_once_set_speed(data, backspace, 2.0 / 3.0);
                } else {
                    let a = self.anim(data, group::WAIT);
                    self.skel.play_loop(data, a);
                }
            }
        } else {
            self.setup_wait_for_put_away(data, env, A74::Talk);
            let a = if xz_dist < 40.0 { backspace } else { data.anim("link_normal_talk_free") };
            self.skel.play_once_set_speed(data, a, 2.0 / 3.0);
        }
        if self.skel.animation == backspace {
            self.start_anim_movement(0x19);
        }
        self.func_80832224();
        self.state1 |= STATE1_6 | STATE1_29;
        // Navi's own text (not a 0x2xx one): her talk request, and the camera turns round to
        // her (Player_SetTurnAroundCamera(play, 0xB)).
        if is_navi && self.target_actor == Some(h) && (text_id & 0xFF00) != 0x200 {
            self.play_requests.push(PlayRequest::TalkRequest(h));
            self.set_turn_around_camera(0xB);
        }
    }

    /// `Player_SetupTalk`: the talking action, and the message box for Player's `textId`.
    fn setup_talk(&mut self, data: &GameData) {
        self.setup_action_preserve_anim_movement(data, Action::Talk, 0);
        self.state1 |= STATE1_6 | STATE1_29;
        if self.actor.text_id != 0 {
            self.play_requests.push(PlayRequest::StartTextbox { text_id: self.actor.text_id, actor: self.target_actor });
            self.focus_actor = self.target_actor;
        }
    }

    /// `Player_Action_Talk`: talking, facing the target, until the box closes; then standing (or
    /// treading water), with A, B and C-Up ignored for 10 frames (`textboxBtnCooldownTimer`).
    fn action_talk(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5;
        self.update_upper_body(env);
        let target = self.target_actor.and_then(|h| env.actors.actor(h));
        if env.msg_state == oot_game::message::TEXT_STATE_CLOSING {
            self.actor.flags &= !ACTOR_FLAG_TALK;
            if !target.is_some_and(|t| t.flags & (ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE) == (ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE)) {
                self.state2 &= !STATE2_13;
            }
            self.play_requests.push(PlayRequest::CamDone);
            // func_8084B4D4 (the ocarina after a talk) and func_8084B3CC (the shooting gallery)
            // don't apply; Player_StartCsAction (a cutscene mode waiting) does; Player_ActionHandler_2 (an item
            // to pick up) doesn't apply.
            if self.start_cs_action(data) {
                self.textbox_btn_cooldown_timer = 10;
                return;
            }
            if self.func_808332B8() {
                self.func_80838F18(data);
            } else {
                self.func_80853080(data);
            }
            self.textbox_btn_cooldown_timer = 10;
            return;
        }
        if self.func_808332B8() {
            self.action_8084d610(env);
        } else if self.state1 & STATE1_4 == 0 && self.skel.update(data) {
            if self.skel.move_flags != 0 {
                self.finish_anim_movement();
                if target.is_some_and(|t| t.category == ACTORCAT_NPC) {
                    self.skel.play_once_set_speed(data, data.anim("link_normal_talk_free"), 2.0 / 3.0);
                } else {
                    let a = self.anim(data, group::WAIT);
                    self.skel.play_loop(data, a);
                }
            } else {
                self.skel.play_loop_set_speed(data, data.anim("link_normal_talk_free_wait"), 2.0 / 3.0);
            }
        }
        if self.focus_actor.is_some() {
            let y = self.func_8083DB98(env, false);
            self.actor.shape_rot.y = y;
            self.current_yaw = y;
        }
    }

    // ================================================================================
    // Getting items (Player_ActionHandler_2 and the get-item action Player_Action_8084E6D4)

    /// `Player_ActionHandler_2` (interrupt 2): take what `interactRangeActor` offers.
    /// - A positive get-item id (a collectible, an NPC's gift): if it would give something new
    ///   (`Item_CheckObtainability`), Link holds it up (`func_8083A434` after the item is put
    ///   away, `link_demo_get_itemB`, the turn-around camera); else it's given at once
    ///   (`func_8083E4C4`).
    /// - A negative one, on A: a chest. Link stands 29.4343 in front of it facing its way. A new
    ///   major item gets the long opening (`ageProperties->unk_98`, the chest's long animation
    ///   and `CAM_SET_SLOW_CHEST_CS`); anything else the kick open (`link_normal_box_kick`).
    ///   Items the table marks (0x40: a blue rupee if not obtainable, 0x20: if had) become a blue
    ///   rupee.
    /// - `GI_NONE` on A: picking up (bushes, rocks, the Master Sword): not ported.
    ///
    /// Title cards (`TitleCard_Clear`) aren't ported: there's always none to clear. `iREG(67)`'s
    /// debug item is 0.
    fn action_handler_2(&mut self, env: &Env) -> bool {
        use oot_game::item::*;
        let Some(ih) = self.interact_range_actor else { return false };
        if self.get_item_id > GI_NONE {
            if self.get_item_id < GI_MAX {
                let Some(gi) = env.items.get_item(self.get_item_id).copied() else { return false };
                if Some(ih) != env.me {
                    self.play_requests.push(PlayRequest::SetParent(ih));
                }
                /// `SCENE_BOMBCHU_BOWLING_ALLEY`.
                const SCENE_BOMBCHU_BOWLING_ALLEY: u16 = 0x4B;
                let (obtainable, scene) = {
                    let io = env.io.borrow();
                    (item_check_obtainability(&io.save, gi.item_id), io.scene_id)
                };
                if obtainable == ITEM_NONE || scene == SCENE_BOMBCHU_BOWLING_ALLEY {
                    // Player_DetachHeldActor (no held actor or explosive), func_8083AE40 (the item's
                    // object: every get-item model is baked).
                    // PLAYER_STATE2_10 is underwater (Kokiri boots, not iron).
                    if self.state2 & STATE2_10 == 0 {
                        self.setup_wait_for_put_away(env.data, env, A74::GetItem);
                        self.skel.play_once_set_speed(env.data, env.data.anim("link_demo_get_itemB"), 2.0 / 3.0);
                        self.set_turn_around_camera(9);
                    }
                    self.state1 |= STATE1_10 | STATE1_11 | STATE1_29;
                    self.func_80832224();
                    return true;
                }
                self.func_8083E4C4(gi);
                self.get_item_id = GI_NONE;
            }
        } else if self.input.press.held(BTN_A) && self.state1 & STATE1_11 == 0 && self.state2 & STATE2_10 == 0 {
            if self.get_item_id != GI_NONE {
                let Some(mut gi) = env.items.get_item(-self.get_item_id).copied() else { return false };
                let Some(chest) = env.target(ih) else { return false };
                let (chest_pos, chest_yaw) = (chest.world_pos, chest.shape_rot.y);
                let obtainable = |id: u8| item_check_obtainability(&env.io.borrow().save, id);
                if gi.item_id != ITEM_NONE && ((obtainable(gi.item_id) == ITEM_NONE && gi.field & 0x40 != 0) || (obtainable(gi.item_id) != ITEM_NONE && gi.field & 0x20 != 0)) {
                    self.get_item_id = -GI_RUPEE_BLUE;
                    gi = env.items.get_item(GI_RUPEE_BLUE).copied().unwrap_or(gi);
                }
                self.setup_wait_for_put_away(env.data, env, A74::GetItem);
                self.state1 |= STATE1_10 | STATE1_11 | STATE1_29;
                self.actor.world_pos.x = chest_pos.x - sin_s(chest_yaw) * 29.4343;
                self.actor.world_pos.z = chest_pos.z - cos_s(chest_yaw) * 29.4343;
                self.current_yaw = chest_yaw;
                self.actor.shape_rot.y = chest_yaw;
                self.func_80832224();
                if gi.item_id != ITEM_NONE && gi.gi >= 0 && obtainable(gi.item_id) == ITEM_NONE {
                    let a = self.age.climb.unk_98;
                    self.skel.play_once_set_speed(env.data, a, 2.0 / 3.0);
                    self.start_anim_movement(0x28F);
                    self.play_requests.push(PlayRequest::ChestOpen { chest: ih, unk_1f4: 1 });
                    self.play_requests.push(PlayRequest::ChangeSetting(oot_game::camera::CAM_SET_SLOW_CHEST_CS));
                } else {
                    self.skel.play_once(env.data, env.data.anim("link_normal_box_kick"));
                    self.play_requests.push(PlayRequest::ChestOpen { chest: ih, unk_1f4: -1 });
                }
                return true;
            }
            // Picking up (lifting, Bg_Toki_Swd): not ported.
            self.note("picking up (Player_ActionHandler_2 with GI_NONE) isn't ported");
        }
        false
    }

    /// `func_8083E4C4`: an item already had is given without holding it up: a drop over Link's
    /// head (unless the table says 0x80), and `Item_Give` unless the drop gives it itself.
    fn func_8083E4C4(&mut self, gi: oot_game::item::GetItemEntry) {
        use crate::en_item00::*;
        let drop_type = (gi.field & 0x1F) as i16;
        if gi.field & 0x80 == 0 {
            self.play_requests.push(PlayRequest::DropCollectible { pos: self.actor.world_pos, params: drop_type | i16::MIN });
            if !matches!(drop_type, ITEM00_BOMBS_A | ITEM00_ARROWS_SMALL | ITEM00_ARROWS_MEDIUM | ITEM00_ARROWS_LARGE | ITEM00_RUPEE_GREEN | ITEM00_RUPEE_BLUE | ITEM00_RUPEE_RED | ITEM00_RUPEE_PURPLE | ITEM00_RUPEE_ORANGE) {
                self.play_requests.push(PlayRequest::ItemGive(gi.item_id));
            }
        } else {
            self.play_requests.push(PlayRequest::ItemGive(gi.item_id));
        }
        let id = if self.get_item_id < 0 { oot_game::audio::sfx::NA_SE_SY_GET_BOXITEM } else { oot_game::audio::sfx::NA_SE_SY_GET_ITEM };
        self.sfx(PlayerSfx::NoPos(id));
    }

    /// `Player_SetTurnAroundCamera`: the turn-around camera (`Player_RequestCameraSetting(CAM_SET_TURN_AROUND)`), told what
    /// kind of item it's for (`Camera_SetCameraData(4, arg1)`: 9 for a held-up item, 8 when
    /// surfacing with one).
    fn set_turn_around_camera(&mut self, arg1: i16) {
        self.play_requests.push(PlayRequest::CamSetting(oot_game::camera::CAM_SET_TURN_AROUND));
        self.play_requests.push(PlayRequest::SetCameraData { data2: arg1 });
    }

    /// `func_8083A434`: the get-item action, once the item in hand is away. A heart container
    /// holds 20 frames longer; a chest's item (negative) first waits for the opening animation.
    fn func_8083A434(&mut self, data: &GameData) {
        self.setup_action_preserve_anim_movement(data, Action::GetItem, 0);
        self.state1 |= STATE1_10 | STATE1_29;
        if self.get_item_id == oot_game::item::GI_HEART_CONTAINER_2 {
            self.action_var2 = 20;
        } else if self.get_item_id >= 0 {
            self.action_var2 = 1;
        } else {
            self.get_item_id = -self.get_item_id;
        }
    }

    /// `func_808332F4`: the item appears over Link's head (`unk_862`, its draw id plus one).
    fn func_808332F4(&mut self, env: &Env) {
        if let Some(gi) = env.items.get_item(self.get_item_id) {
            self.unk_862 = (gi.gi as i16).abs();
        }
    }

    /// `func_8084DF6C`: the item is put away; the camera is told the get-item is over.
    fn func_8084DF6C(&mut self) {
        self.unk_862 = 0;
        self.state1 &= !(STATE1_10 | STATE1_11);
        self.get_item_id = oot_game::item::GI_NONE;
        self.play_requests.push(PlayRequest::CamDone);
    }

    /// `func_8084DFAC`: and Link stands again.
    fn func_8084DFAC(&mut self, data: &GameData) {
        self.func_8084DF6C();
        self.apply_yaw_from_anim();
        self.func_8083C0E8(data);
        self.current_yaw = self.actor.shape_rot.y;
    }

    /// `func_8084DFF4`: the first time, the item's text (`Message_StartTextbox` with Player as
    /// the talker), `Item_Give` and the item's fanfare; then waiting for the text to close.
    /// True once it has.
    fn func_8084DFF4(&mut self, env: &Env) -> bool {
        use oot_game::item::*;
        if self.get_item_id == GI_NONE {
            return true;
        }
        if self.action_var1 == 0 {
            let Some(gi) = env.items.get_item(self.get_item_id).copied() else { return true };
            self.action_var1 = 1;
            self.play_requests.push(PlayRequest::StartTextbox { text_id: gi.text_id as u16, actor: env.me });
            self.play_requests.push(PlayRequest::ItemGive(gi.item_id));
            self.play_requests.push(PlayRequest::GetItemFanfare(self.get_item_id));
        } else if env.msg_state == oot_game::message::TEXT_STATE_CLOSING {
            if self.get_item_id == GI_SILVER_GAUNTLETS {
                // The Silver Gauntlets' exit to the Desert Colossus (ENTR_DESERT_COLOSSUS_0 with the
                // sandstorm and cutscene 0xFFF1): not ported.
                self.note("the Silver Gauntlets' exit to the Desert Colossus isn't ported");
            }
            self.get_item_id = GI_NONE;
        }
        false
    }

    /// `Player_Action_8084E6D4`: getting an item.
    /// - From a chest (`av2.actionVar2` 0): the opening animation plays (with the child's sounds);
    ///   at its end Link turns to hold the item up (`link_demo_get_itemA`, or `_itemB` after
    ///   the kick) with the turn-around camera.
    /// - Holding up (`get_itemB` turns Link to face the camera): on frame 21 the item appears
    ///   over his head; each time the animation ends the text runs (`func_8084DFF4`), and when
    ///   it's closed and the hearts have counted in, Link stands (`func_8084DFAC`), or talks to
    ///   whoever gave the item.
    ///
    /// An ice trap (`GI_ICE_TRAP`: `En_Clear_Tag`, the freeze damage) isn't ported.
    fn action_8084e6d4(&mut self, env: &Env) {
        let data = env.data;
        if self.skel.update(data) {
            if self.action_var2 != 0 {
                if self.action_var2 >= 2 {
                    self.action_var2 -= 1;
                }
                if self.func_8084DFF4(env) && self.action_var2 == 1 {
                    // (PLAYER_STATE3_5 never comes up.)
                    let cond = self.target_actor.is_some() && (self.exchange_item_id as i8) < 0;
                    let health_accumulator = env.io.borrow().save.health_accumulator;
                    if cond || health_accumulator == 0 {
                        if cond {
                            self.func_8084DF6C();
                            self.exchange_item_id = 0;
                            // func_8084B4D4 (an ocarina after the talk) is 0.
                            if let Some(t) = self.target_actor
                                && let Some(text) = env.actors.actor(t).map(|a| a.text_id)
                            {
                                self.start_talking(env, t, text);
                            }
                        } else {
                            self.func_8084DFAC(data);
                        }
                    }
                }
            } else {
                self.finish_anim_movement();
                if self.get_item_id == oot_game::item::GI_ICE_TRAP {
                    self.note("an ice trap (GI_ICE_TRAP) isn't ported");
                }
                let a = if self.skel.animation == data.anim("link_normal_box_kick") { "link_demo_get_itemB" } else { "link_demo_get_itemA" };
                self.skel.play_once_set_speed(data, data.anim(a), 2.0 / 3.0);
                self.action_var2 = 2;
                self.set_turn_around_camera(9);
            }
        } else {
            if self.action_var2 == 0 {
                if !self.adult {
                    self.process_anim_sfx_list(env.audio.player_anim_sfx("D_808549E0"));
                }
                return;
            }
            if self.skel.animation == data.anim("link_demo_get_itemB") {
                scaled_step_to_s(&mut self.actor.shape_rot.y, env.cam_dir_yaw.wrapping_add(i16::MIN), 4000);
            }
            if self.skel.on_frame(21.0) {
                self.func_808332F4(env);
            }
        }
    }

    /// `Player_UpdateInterface`: what the A button would do (`Interface_SetDoAction`), when no message
    /// is open (the crawlspace's "Enter" when `Player_TryEnteringCrawlspace` lines Link up with one). Riding,
    /// the fishing pole, the ocarina and held actors aren't ported, so those actions never
    /// show. The ocarina's action (`Player_Action_8084E3C4`)
    /// never runs, so its exception doesn't apply.
    fn update_interface(&mut self, env: &Env) {
        use oot_game::interface::*;
        if env.msg_state != oot_game::message::TEXT_STATE_NONE {
            return;
        }
        let sp20 = self.control_stick_directions[self.control_stick_data_index as usize];
        let sp1c = self.func_808332B8();
        let mut do_action = DO_ACTION_NONE;
        if !self.in_cs_mode() {
            if self.state1 & STATE1_20 != 0 {
                do_action = DO_ACTION_RETURN;
            } else if self.state2 & STATE2_18 == 0 {
                let target_category = self.target_actor.and_then(|h| env.actors.actor(h)).map(|a| a.category);
                if self.door_type != PLAYER_DOORTYPE_NONE && self.state1 & STATE1_11 == 0 {
                    do_action = DO_ACTION_OPEN;
                } else if self.state1 & STATE1_11 == 0
                    && self.interact_range_actor.is_some()
                    && ((!sp1c && self.get_item_id == oot_game::item::GI_NONE) || (self.get_item_id < 0 && self.state1 & STATE1_27 == 0))
                {
                    // A chest (a negative get-item id), or something to pick up (Bg_Toki_Swd's
                    // DO_ACTION_DROP for the adult isn't ported).
                    do_action = if self.get_item_id < 0 { DO_ACTION_OPEN } else { DO_ACTION_GRAB };
                } else if !sp1c && self.state2 & STATE2_0 != 0 {
                    do_action = DO_ACTION_GRAB;
                } else if self.state2 & STATE2_2 != 0 {
                    do_action = DO_ACTION_CLIMB;
                } else if self.state2 & STATE2_1 != 0 && target_category.is_some() {
                    do_action = if target_category == Some(ACTORCAT_NPC) { DO_ACTION_SPEAK } else { DO_ACTION_CHECK };
                } else if self.state1 & (STATE1_13 | STATE1_21) != 0 {
                    do_action = DO_ACTION_DOWN;
                } else if self.state2 & STATE2_16 != 0 {
                    do_action = DO_ACTION_ENTER;
                } else if self.state2 & STATE2_11 != 0 {
                    // D_80854784[CUR_UPG_VALUE(UPG_SCALE)] (no scale: 120) less the depth, per 40.
                    let sp24 = ((120.0 - self.actor.y_dist_to_water) / 40.0) as i32;
                    do_action = DO_ACTION_1 + sp24.clamp(0, 7) as u16;
                } else if sp1c && self.state2 & STATE2_10 == 0 {
                    do_action = DO_ACTION_DIVE;
                } else if !sp1c && (self.state1 & STATE1_22 == 0 || self.is_z_targeting() || !(!self.adult && self.current_shield == 2)) {
                    let behavior_2 = env.room_behavior_type1 == 2;
                    if self.state1 & STATE1_14 == 0
                        && sp20 <= 0
                        && (self.state1 & STATE1_4 != 0 || (self.s.floor_type != FLOOR_TYPE_7 && (self.friendly_lock_on_or_parallel() || (!behavior_2 && self.state1 & STATE1_22 == 0 && sp20 == 0))))
                    {
                        do_action = DO_ACTION_ATTACK;
                    } else if !behavior_2 && self.is_z_targeting() && sp20 > 0 {
                        do_action = DO_ACTION_JUMP;
                    } else if self.held_item_ap >= env.data.items.ap("SWORD_MASTER") || (self.state2 & (1 << 20) != 0 && env.target.arrow_pointed.is_none()) {
                        do_action = DO_ACTION_PUTAWAY;
                    }
                }
            }
        }
        if do_action != DO_ACTION_PUTAWAY {
            self.put_away_cooldown_timer = 20;
        } else if self.put_away_cooldown_timer != 0 {
            do_action = DO_ACTION_NONE;
            self.put_away_cooldown_timer -= 1;
        }
        self.play_requests.push(PlayRequest::DoAction(do_action));
        // Interface_SetNaviCall: Navi calls while she has something to say.
        if self.state2 & STATE2_21 != 0 {
            if self.focus_actor.is_some() {
                self.play_requests.push(PlayRequest::NaviCall(0x1E));
            } else {
                self.play_requests.push(PlayRequest::NaviCall(0x1D));
            }
            self.play_requests.push(PlayRequest::NaviCall(0x1E));
        } else {
            self.play_requests.push(PlayRequest::NaviCall(0x1F));
        }
    }

    // ================================================================================
    // Colliders (the end of Player_UpdateCommon, and Player_PostLimbDrawGameplay)

    /// The end of `Player_UpdateCommon`: the body cylinder from the last draw's body parts,
    /// registered for OC and AC; the mass; then the per-frame resets of the AC and the sword's
    /// AT.
    fn update_colliders(&mut self, play: &mut PlayState) {
        use cc::ColliderShape;
        self.door_type = PLAYER_DOORTYPE_NONE;
        self.knockback_type = 0;
        // The talk offers of this frame end here (they're made again next frame), unless one
        // was accepted (ACTOR_FLAG_TALK).
        if self.actor.flags & ACTOR_FLAG_TALK == ACTOR_FLAG_TALK {
            self.target_actor_distance = 0.0;
        } else {
            self.target_actor = None;
            self.target_actor_distance = f32::MAX;
            self.exchange_item_id = 0;
        }
        // The get-item offers too (made again next frame), unless Player holds what offered.
        if self.state1 & STATE1_11 == 0 {
            self.interact_range_actor = None;
            self.get_item_direction = 0x6000;
        }
        // (rideActor: no riding.) Navi's text is set again by her next update.
        self.navi_text_id = 0;
        let mut temp_f0 = self.actor.world_pos.y - self.actor.prev_pos.y;
        let bp = &self.body_parts_pos;
        let mut phi_f12 = (bp[BODYPART_L_FOOT].y + bp[BODYPART_R_FOOT].y) * 0.5 + temp_f0;
        temp_f0 += bp[BODYPART_HEAD].y + 10.0;
        self.cylinder.dim.height = (temp_f0 - phi_f12) as i16;
        if self.cylinder.dim.height < 0 {
            phi_f12 = temp_f0;
            self.cylinder.dim.height = -self.cylinder.dim.height;
        }
        self.cylinder.dim.y_shift = (phi_f12 - self.actor.world_pos.y) as i16;
        if self.state1 & STATE1_22 != 0 {
            self.cylinder.dim.height = (self.cylinder.dim.height as f32 * 0.8) as i16;
        }
        self.cylinder.update(&self.actor);
        let invincibility_timer = self.invincibility_timer;
        if self.state2 & STATE2_14 == 0 {
            if self.state1 & (STATE1_7 | STATE1_13 | STATE1_14 | STATE1_23) == 0 {
                play.collision_check_set_oc(&self.actor, COLLIDER_CYLINDER, &mut self.cylinder);
            }
            if self.state1 & (STATE1_7 | STATE1_26) == 0 && invincibility_timer <= 0 {
                play.collision_check_set_ac(&self.actor, COLLIDER_CYLINDER, &mut self.cylinder);
                if invincibility_timer < 0 {
                    play.collision_check_set_at(&self.actor, COLLIDER_CYLINDER, &mut self.cylinder);
                }
            }
        }
        self.actor.col_chk_info.mass = if self.state1 & (STATE1_7 | STATE1_28 | STATE1_29) != 0 { cc::MASS_IMMOVABLE } else { 50 };
        self.state3 &= !(1 << 2);
        self.cylinder.reset_ac();
        self.melee_weapon_quads[0].reset_at();
        self.melee_weapon_quads[1].reset_at();
        self.shield_quad.reset_ac();
        self.shield_quad.reset_at();
    }

    /// `bodyPartsPos`: each body part's limb origin, as `Player_PostLimbDrawGameplay` records it
    /// from the matrix the limb drew with (after the foot IK and the look rotations). Returns
    /// every limb's world matrix.
    fn update_body_parts(&mut self, data: &GameData) -> Vec<glam::Mat4> {
        let rig = &data.rigs[if self.adult { 0 } else { 1 }];
        let bones = pose(rig, &self.draw_joints(), &self.look_rotations(), data.limb("HEAD"), data.limb("UPPER"));
        let r = self.actor.shape_rot;
        let root = oot_game::footik::actor_matrix(self.actor.world_pos, self.actor.shape_y_offset, [r.x, r.y, r.z]);
        let world: Vec<glam::Mat4> = bones.iter().map(|b| root * *b).collect();
        for (i, name) in BODYPART_LIMBS.iter().enumerate() {
            self.body_parts_pos[i] = world[data.limb(name)].transform_point3(Vec3::ZERO);
        }
        // Player_PostLimbDrawGameplay, PLAYER_LIMB_HEAD: actor.focus.pos is
        // sPlayerFocusOffsetFromHead (1100, -700, 0) in the head limb's space, as drawn (the look
        // rotations in): what Navi's point, the attention system and first person's camera
        // (Camera_Subj3, Actor_GetFocus) read.
        self.actor.focus_pos = world[data.limb("HEAD")].transform_point3(Vec3::new(1100.0, -700.0, 0.0));
        // Actor_SetFeetPos: sLeftRightFootLimbModelFootPos[linkAge] in each foot's space.
        let foot = if self.adult { Vec3::new(200.0, 300.0, 0.0) } else { Vec3::new(200.0, 200.0, 0.0) };
        self.feet_pos = [world[data.limb("L_FOOT")].transform_point3(foot), world[data.limb("R_FOOT")].transform_point3(foot)];
        world
    }

    /// `this->rightHandType == PLAYER_MODELTYPE_RH_SHIELD`: the model group's right hand
    /// (`gPlayerModelTypes`), or the shield held up (`Player_SetModelsForHoldingShield`).
    /// `rightHandType` is `PLAYER_MODELTYPE_RH_BOW_SLINGSHOT` or `_2` (not holding the shield).
    fn right_hand_is_bow_slingshot(&self, play: &PlayState) -> bool {
        if self.holding_shield {
            return false;
        }
        let rules = &play.rules;
        let name = play.data.items.model_group_names.get(self.model_group).map(String::as_str).unwrap_or("");
        rules.model_group(name).is_some_and(|g| {
            let r = rules.model_groups[g].right;
            r == rules.model_type("RH_BOW_SLINGSHOT") || r == rules.model_type("RH_BOW_SLINGSHOT_2")
        })
    }

    fn right_hand_is_shield(&self, play: &PlayState) -> bool {
        if self.holding_shield {
            return true;
        }
        let rules = &play.rules;
        let name = play.data.items.model_group_names.get(self.model_group).map(String::as_str).unwrap_or("");
        rules.model_group(name).is_some_and(|g| rules.model_groups[g].right == rules.model_type("RH_SHIELD"))
    }

    /// `Player_UpdateShieldCollider`: while shielding, the shield's quad from the limb's matrix,
    /// its material the shield's (`shieldColMaterials`: the Deku Shield wood, the others metal),
    /// registered for AC and AT.
    fn update_shield_collider(&mut self, play: &mut PlayState, mtx: glam::Mat4, quad_src: &[Vec3; 4]) {
        const SHIELD_COL_MATERIALS: [u8; 4] = [cc::COL_MATERIAL_METAL, cc::COL_MATERIAL_WOOD, cc::COL_MATERIAL_METAL, cc::COL_MATERIAL_METAL];
        if self.state1 & STATE1_22 != 0 {
            self.shield_quad.base.col_type = SHIELD_COL_MATERIALS[self.current_shield as usize];
            let d = quad_src.map(|v| mtx.transform_point3(v));
            self.shield_quad.set_vertices(d[0], d[1], d[2], d[3]);
            play.collision_check_set_ac(&self.actor, COLLIDER_SHIELD, &mut self.shield_quad);
            play.collision_check_set_at(&self.actor, COLLIDER_SHIELD, &mut self.shield_quad);
        }
    }

    /// `Player_PostLimbDrawGameplay`, `PLAYER_LIMB_R_HAND`: `sGetItemRefPos`, where the held-up
    /// item floats: the left hand while an exchange item is shown outside the get-item action,
    /// else between the hands.
    fn get_item_ref_pos(&self) -> Vec3 {
        if self.state1 & STATE1_10 == 0 && self.unk_862 != 0 && self.exchange_item_id != 0 {
            self.left_hand_pos
        } else {
            (self.body_parts_pos[BODYPART_R_HAND] + self.left_hand_pos) * 0.5
        }
    }

    /// `Player_PostLimbDrawGameplay` for `PLAYER_LIMB_L_HAND`, the weapon part. With a Deku Stick
    /// in use, its tip (`unk_85C × 5000` along the hand) is tracked every frame: the weapon info
    /// while it swings, else just the tip (what the torches and webs read); the stick's list is
    /// drawn by `draw`. Otherwise, with a weapon active, its tip and base from the hand's matrix
    /// and the quads.
    fn post_limb_draw_l_hand(&mut self, play: &mut PlayState, hand: glam::Mat4) {
        if self.item_ap == play.data.items.ap("DEKU_STICK") {
            if self.actor.scale.y >= 0.0 {
                let tips = self.calc_melee_weapon_tip_positions(hand, self.unk_85c * 5000.0);
                if self.melee_weapon_state != 0 {
                    self.update_melee_weapon_info(play, hand, tips);
                } else {
                    self.melee_weapon_info[0].tip = tips[0];
                }
            }
        } else if self.actor.scale.y >= 0.0 && self.melee_weapon_state != 0 {
            // Player_HoldsBrokenKnife: never (child Link). sMeleeWeaponLengths[Player_GetMeleeWeaponHeld].
            let len = MELEE_WEAPON_LENGTHS[Self::melee_weapon(self.held_item_ap) as usize];
            let tips = self.calc_melee_weapon_tip_positions(hand, len);
            self.update_melee_weapon_info(play, hand, tips);
        }
        // (A bottle in the left hand, PLAYER_MODELTYPE_LH_BOTTLE: the bottles aren't ported.)
        // The seed or arrow in hand, drawn back (PLAYER_STATE1_9): at D_80126128 from the hand,
        // turned by Matrix_RotateZYX(0x69E8, -0x5708, 0x458E). (Carrying, the carried actor's
        // turn, and mf_9E0 and unk_3BC with nothing held: carrying isn't ported.)
        if self.actor.scale.y >= 0.0
            && !self.holds_hookshot(&play.data)
            && let Some(h) = self.held_actor
            && self.state1 & STATE1_9 != 0
        {
            let mut mf = oot_game::sys_matrix::MtxF::from_mat4(hand);
            let pos = mf.mult_vec3f(Vec3::new(398.0, 1419.0, 244.0));
            mf.rotate_zyx(0x69E8, -0x5708, 0x458E);
            let r = mf.to_yxz_rot_s(false);
            if let Some(a) = play.actors.actor_mut(h) {
                a.world_pos = pos;
                a.world_rot = Rot { x: r[0], y: r[1], z: r[2] };
                a.shape_rot = a.world_rot;
            }
        }
    }

    /// `Player_PostLimbDrawGameplay`'s `PLAYER_LIMB_R_HAND` with the bow or slingshot in the right
    /// hand: drawn back (`PLAYER_STATE1_9`, `unk_860` positive, `unk_834` at most 10), the
    /// string's stretch (`unk_858`) is its point's distance from the hand less 3, by 1.6, at most
    /// 1, and its spring's speed (`unk_85C`) -0.5. Draw-time state.
    fn post_limb_draw_r_hand_string(&mut self, hand: glam::Mat4) {
        if self.state1 & STATE1_9 != 0 && self.unk_860 >= 0 && self.unk_834 <= 10 {
            let pos = BOW_SLINGSHOT_STRING[(!self.adult) as usize].2;
            let sp90 = hand.transform_point3(pos);
            let dist = self.body_parts_pos[BODYPART_R_HAND].distance(sp90);
            self.unk_858 = dist - 3.0;
            if dist < 3.0 {
                self.unk_858 = 0.0;
            } else {
                self.unk_858 *= 1.6;
                if self.unk_858 > 1.0 {
                    self.unk_858 = 1.0;
                }
            }
            self.unk_85c = -0.5;
        }
    }

    /// `Player_CalcMeleeWeaponTipPositions` with `sMeleeWeaponTipOffsetFromLeftHand0.x` = `len`: the
    /// far edge is 1200 longer, and longer still in a combo's third attack.
    fn calc_melee_weapon_tip_positions(&mut self, hand: glam::Mat4, len: f32) -> [Vec3; 3] {
        let melee_weapon_tip_offset_from_left_hand0 = Vec3::new(len, 400.0, 0.0);
        let mut x = len;
        if self.unk_845 >= 3 {
            // As written: drawing advances unk_845 while the combo attack is active.
            self.unk_845 += 1;
            x *= 1.0 + ((9 - self.unk_845) as f32 * 0.1);
        }
        x += 1200.0;
        let melee_weapon_tip_offset_from_left_hand1 = Vec3::new(x, -400.0, 1000.0);
        let melee_weapon_tip_offset_from_left_hand2 = Vec3::new(x, 1400.0, -1000.0);
        [hand.transform_point3(melee_weapon_tip_offset_from_left_hand0), hand.transform_point3(melee_weapon_tip_offset_from_left_hand1), hand.transform_point3(melee_weapon_tip_offset_from_left_hand2)]
    }

    /// `Player_UpdateMeleeWeaponInfo`: the first edge (the sword trail's: `EffectBlure` isn't
    /// ported), then while the weapon swings (not a spin, unless `PLAYER_STATE2_17`) the two quads.
    fn update_melee_weapon_info(&mut self, play: &mut PlayState, hand: glam::Mat4, tips: [Vec3; 3]) {
        let bases = S_MELEE_WEAPON_BASE_OFFSET_FROM_LEFT_HAND0.map(|v| hand.transform_point3(v));
        update_weapon_info(play, &self.actor, None, &mut self.melee_weapon_info[0], tips[0], bases[0]);
        let spin = play.data.items.mwa("SPIN_ATTACK_1H");
        if self.melee_weapon_state > 0 && (self.melee_weapon_animation < spin || self.state2 & STATE2_17 != 0) {
            let [q0, q1] = &mut self.melee_weapon_quads;
            update_weapon_info(play, &self.actor, Some((COLLIDER_SWORD_0, q0)), &mut self.melee_weapon_info[1], tips[1], bases[1]);
            update_weapon_info(play, &self.actor, Some((COLLIDER_SWORD_1, q1)), &mut self.melee_weapon_info[2], tips[2], bases[2]);
        }
    }

    // ================================================================================
    // Draw-time state

    /// The joint table the skeleton is drawn with, including
    /// `Player_OverrideLimbDrawGameplayCommon`'s child root scaling and the `unk_6C4` sink.
    pub fn draw_joints(&self) -> eng_anim::anim::JointTable {
        let mut rot: Vec<[i16; 3]> = self.skel.joint.to_vec();
        let mut root = [rot[0][0] as f32, rot[0][1] as f32, rot[0][2] as f32];
        if !self.adult {
            let mf = self.skel.move_flags;
            if mf & 4 == 0 || mf & 1 != 0 {
                root[0] *= 0.64;
                root[2] *= 0.64;
            }
            if mf & 4 == 0 || mf & 2 != 0 {
                root[1] *= 0.64;
            }
        }
        root[1] -= self.unk_6C4;
        rot[0] = [root[0] as i16, root[1] as i16, root[2] as i16];
        eng_anim::anim::JointTable { rot, face: self.skel.face }
    }

    /// Head (x, y, z additions) and upper-body (y, x, z pre-rotations) look/lean angles.
    pub fn look_rotations(&self) -> LookRotations {
        LookRotations {
            head: [self.head_limb_rot_z, self.head_limb_rot_y.wrapping_neg(), self.head_limb_rot_x],
            upper_y: self.upper_limb_rot_y,
            upper_x: self.upper_limb_rot_x,
            upper_z: self.upper_limb_rot_z,
            root_pitch: self.unk_6C2,
        }
    }

    pub fn wall_flag_3(&self) -> bool {
        self.s.wall_flags & WALL_FLAG_3 != 0
    }
}

/// Rotations `Player_OverrideLimbDrawGameplayCommon` applies to the head and upper body.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LookRotations {
    /// Added to the head limb's (x, y, z) rotation.
    pub head: [i16; 3],
    /// Applied before the upper body limb's own transform: RotY, then RotX, then RotZ.
    pub upper_y: i16,
    pub upper_x: i16,
    pub upper_z: i16,
    /// `unk_6C2`: the root limb's pitch while diving (`Player_OverrideLimbDrawGameplayCommon`).
    pub root_pitch: i16,
}

// ================================================================================
// The actor system

/// Player's collider ids (`ActorImpl::collider_mut`).
pub const COLLIDER_CYLINDER: u8 = 0;
pub const COLLIDER_SWORD_0: u8 = 1;
pub const COLLIDER_SWORD_1: u8 = 2;
pub const COLLIDER_SHIELD: u8 = 3;

/// `PLAYER_BODYPART_MAX`, and the limbs of `PLAYER_BODYPART_*` in order: every limb after the
/// root that has a display list (`Player_OverrideLimbDrawGameplayCommon`).
pub const BODYPART_MAX: usize = 18;
/// `func_8084260C`: `src` scattered by up to `arg3` across and `arg4` up, from `arg2` above it.
fn func_8084260c(rand: &mut oot_game::play::Rand, src: Vec3, arg2: f32, arg3: f32, arg4: f32) -> Vec3 {
    let x = (rand.zero_one() * arg3) + src.x;
    let y = (rand.zero_one() * arg4) + (src.y + arg2);
    let z = (rand.zero_one() * arg3) + src.z;
    Vec3::new(x, y, z)
}

const BODYPART_LIMBS: [&str; BODYPART_MAX] =
    ["WAIST", "R_THIGH", "R_SHIN", "R_FOOT", "L_THIGH", "L_SHIN", "L_FOOT", "HEAD", "HAT", "COLLAR", "L_SHOULDER", "L_FOREARM", "L_HAND", "R_SHOULDER", "R_FOREARM", "R_HAND", "SHEATH", "TORSO"];
pub const BODYPART_R_FOOT: usize = 3;
pub const BODYPART_L_FOOT: usize = 6;
pub const BODYPART_HEAD: usize = 7;
pub const BODYPART_L_HAND: usize = 12;
pub const BODYPART_R_HAND: usize = 15;

/// `WeaponInfo`: a weapon edge's tip and base as last drawn.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WeaponInfo {
    pub active: bool,
    pub tip: Vec3,
    pub base: Vec3,
}

/// `sMeleeWeaponLengths` (`z_player_lib.c`), by `Player_ActionToMeleeWeapon`.
const MELEE_WEAPON_LENGTHS: [f32; 6] = [0.0, 4000.0, 3000.0, 5500.0, 0.0, 2500.0];

/// `sMeleeWeaponBaseOffsetFromLeftHand0`: the sword's base points in the left hand's space.
const S_MELEE_WEAPON_BASE_OFFSET_FROM_LEFT_HAND0: [Vec3; 3] = [Vec3::new(0.0, 400.0, 0.0), Vec3::new(0.0, 1400.0, -1000.0), Vec3::new(0.0, -400.0, 1000.0)];

/// `D_80854488`: per melee weapon (`Player_GetMeleeWeaponHeld() - 1`), the damage type of a
/// slash and of a jump attack.
const D_80854488: [[u32; 2]; 5] = [
    [cc::DMG_SLASH_MASTER, cc::DMG_JUMP_MASTER],
    [cc::DMG_SLASH_KOKIRI, cc::DMG_JUMP_KOKIRI],
    [cc::DMG_SLASH_GIANT, cc::DMG_JUMP_GIANT],
    [cc::DMG_DEKU_STICK, cc::DMG_JUMP_MASTER],
    [cc::DMG_HAMMER_SWING, cc::DMG_HAMMER_JUMP],
];

const NO_TOUCH: ColliderElementDamageInfoAT = ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: 0, damage: 0 };

/// `D_80854624`: the body.
const D_80854624: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: cc::COL_MATERIAL_HIT5, at_flags: cc::AT_NONE, ac_flags: cc::AC_ON | cc::AC_TYPE_ENEMY, oc_flags1: cc::OC1_ON | cc::OC1_TYPE_ALL, oc_flags2: cc::OC2_TYPE_PLAYER, shape: cc::COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: cc::ELEM_MATERIAL_UNK1,
        at_dmg_info: NO_TOUCH,
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: 0, defense: 0 },
        at_elem_flags: cc::ATELEM_NONE,
        ac_elem_flags: cc::ACELEM_ON,
        oc_elem_flags: cc::OCELEM_ON,
    },
    dim: Cylinder16 { radius: 12, height: 60, y_shift: 0, pos: [0; 3] },
};

/// `D_80854650`: a sword quad.
const D_80854650: ColliderQuadInit = ColliderQuadInit {
    base: ColliderInit { col_type: cc::COL_MATERIAL_NONE, at_flags: cc::AT_ON | cc::AT_TYPE_PLAYER, ac_flags: cc::AC_NONE, oc_flags1: cc::OC1_NONE, oc_flags2: cc::OC2_TYPE_PLAYER, shape: cc::COLSHAPE_QUAD },
    info: ColliderElementInit {
        elem_material: cc::ELEM_MATERIAL_UNK2,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0100, hit_special_effect: 0, damage: 1 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: 0, defense: 0 },
        at_elem_flags: cc::ATELEM_ON | cc::ATELEM_SFX_NORMAL,
        ac_elem_flags: cc::ACELEM_NONE,
        oc_elem_flags: cc::OCELEM_NONE,
    },
    quad: [Vec3::ZERO; 4],
};

/// `D_808546A0`: the shield.
const D_808546A0: ColliderQuadInit = ColliderQuadInit {
    base: ColliderInit { col_type: cc::COL_MATERIAL_METAL, at_flags: cc::AT_ON | cc::AT_TYPE_PLAYER, ac_flags: cc::AC_ON | cc::AC_HARD | cc::AC_TYPE_ENEMY, oc_flags1: cc::OC1_NONE, oc_flags2: cc::OC2_TYPE_PLAYER, shape: cc::COLSHAPE_QUAD },
    info: ColliderElementInit {
        elem_material: cc::ELEM_MATERIAL_UNK2,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0010_0000, hit_special_effect: 0, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xDFCF_FFFF, hit_backlash: 0, defense: 0 },
        at_elem_flags: cc::ATELEM_ON | cc::ATELEM_SFX_NORMAL,
        ac_elem_flags: cc::ACELEM_ON,
        oc_elem_flags: cc::OCELEM_NONE,
    },
    quad: [Vec3::ZERO; 4],
};

/// `Player_UpdateWeaponInfo`: moves a weapon edge to its new tip and base. The first draw only records
/// it; after that, if it moved, the quad spans the old and the new edge and attacks
/// (`CollisionCheck_SetAT`). Returns whether the edge is new or moved.
fn update_weapon_info(play: &mut PlayState, owner: &Actor, collider: Option<(u8, &mut ColliderQuad)>, info: &mut WeaponInfo, new_tip: Vec3, new_base: Vec3) -> bool {
    use cc::ColliderShape;
    if !info.active {
        if let Some((_, c)) = collider {
            c.reset_at();
        }
        info.tip = new_tip;
        info.base = new_base;
        info.active = true;
        true
    } else if info.tip == new_tip && info.base == new_base {
        if let Some((_, c)) = collider {
            c.reset_at();
        }
        false
    } else {
        if let Some((id, c)) = collider {
            c.set_vertices(new_base, new_tip, info.base, info.tip);
            play.collision_check_set_at(owner, id, c);
        }
        info.base = new_base;
        info.tip = new_tip;
        info.active = true;
        true
    }
}

/// `Player_Profile` (`z_player_call.c`).
pub const PROFILE: oot_game::actor_ctx::ActorProfile = oot_game::actor_ctx::ActorProfile {
    id: ACTOR_PLAYER,
    name: "Player",
    category: ACTORCAT_PLAYER,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA | ACTOR_FLAG_CAN_PRESS_SWITCHES,
    object: "gameplay_keep",
};

/// Indices into Player's `RenderState` extras.
mod rs {
    /// `angles`: the head's x/y/z additions, the upper body's y/x/z, the root pitch.
    pub const HEAD: usize = 0;
    pub const UPPER_Y: usize = 3;
    pub const UPPER_X: usize = 4;
    pub const UPPER_Z: usize = 5;
    pub const ROOT_PITCH: usize = 6;
    /// `angles`: the held-up item's spin (`gameplayFrames * 1000`).
    pub const GET_ITEM_SPIN: usize = 7;
    /// `values`: `speedXZ`, `shape.yOffset`, `sGetItemRefPos` (x, y, z).
    pub const SPEED_XZ: usize = 0;
    pub const Y_OFFSET: usize = 1;
    pub const GET_ITEM_POS: usize = 2;
    /// `switches`: the blink face, `modelGroup`, `currentShield`, `unk_862`, and whether
    /// `exchangeItemId` is set.
    pub const FACE: usize = 0;
    pub const MODEL_GROUP: usize = 1;
    pub const SHIELD: usize = 2;
    pub const UNK_862: usize = 3;
    pub const EXCHANGE: usize = 4;
    /// `switches`: `PLAYER_STATE2_CRAWLING` (in a crawlspace).
    pub const CRAWLING: usize = 5;
    /// `switches`: `PLAYER_STATE2_29` (not drawn).
    pub const HIDDEN: usize = 6;
    /// `switches`: `shape.shadowDraw` set.
    pub const SHADOW: usize = 7;
    /// `switches`: the hit flash's fog far plane (`Player_Draw`'s `Gfx_SetFog2`), 0 for none.
    pub const DAMAGE_FLASH_FAR: usize = 8;
    /// `switches`: the frozen ice's scale (`PLAYER_STATE2_14`), 0 for none.
    pub const ICE_SCALE: usize = 9;
    /// `switches`: the shield in the right hand (`Player_SetModelsForHoldingShield`).
    pub const HOLDING_SHIELD: usize = 10;
    /// `switches`: a Deku Stick in use (`itemAction == PLAYER_IA_DEKU_STICK`), drawn in the left
    /// hand; `values`: its length (`unk_85C`).
    pub const DEKU_STICK: usize = 11;
    pub const STICK_LENGTH: usize = 5;
    /// `values`: the bow's or slingshot's string stretch (`unk_858`), and `actor.focus.pos`
    /// (x, y, z); `switches`: `unk_6AD` (first person).
    pub const STRING: usize = 6;
    pub const FOCUS: usize = 7;
    pub const UNK_6AD: usize = 12;
}

impl LookRotations {
    fn from_angles(a: &[i16]) -> LookRotations {
        LookRotations {
            head: [a[rs::HEAD], a[rs::HEAD + 1], a[rs::HEAD + 2]],
            upper_y: a[rs::UPPER_Y],
            upper_x: a[rs::UPPER_X],
            upper_z: a[rs::UPPER_Z],
            root_pitch: a[rs::ROOT_PITCH],
        }
    }
}

/// Poses Link's skeleton like `SkelAnime_DrawFlexLod` with
/// `Player_OverrideLimbDrawGameplayCommon`: the head limb's rotation gets the look offsets,
/// the upper body is pre-rotated (Y, then X, then Z) before its own transform, and while
/// diving the root pitches about a point 200 above it. Returns every limb's matrix in model
/// space.
pub fn pose(rig: &oot_game::footik::Rig, joints: &eng_anim::anim::JointTable, look: &LookRotations, head: usize, upper: usize) -> Vec<glam::Mat4> {
    use glam::Mat4;
    let r = |a: i16| eng_math::binang_to_rad(a);
    let n = rig.parents.len();
    let mut out = vec![Mat4::IDENTITY; n];
    let mut done = vec![false; n];
    // Parents before children (any such order gives the same matrices).
    while done.iter().any(|d| !d) {
        for l in 0..n {
            if done[l] || rig.parents[l].is_some_and(|p| !done[p as usize]) {
                continue;
            }
            let parent = rig.parents[l].map(|p| out[p as usize]).unwrap_or(Mat4::IDENTITY);
            let pos = if l == 0 {
                let t = joints.rot[0];
                Vec3::new(t[0] as f32, t[1] as f32, t[2] as f32)
            } else {
                Vec3::new(rig.joint_pos[l][0] as f32, rig.joint_pos[l][1] as f32, rig.joint_pos[l][2] as f32)
            };
            let mut rot = joints.rot.get(l + 1).copied().unwrap_or([0; 3]);
            let mut pre = Mat4::IDENTITY;
            if l == head {
                rot = [rot[0].wrapping_add(look.head[0]), rot[1].wrapping_add(look.head[1]), rot[2].wrapping_add(look.head[2])];
            } else if l == upper {
                pre = Mat4::from_rotation_y(r(look.upper_y)) * Mat4::from_rotation_x(r(look.upper_x)) * Mat4::from_rotation_z(r(look.upper_z));
            }
            out[l] = if l == 0 && look.root_pitch != 0 {
                // T(pos.x, (cos(unk_6C2) - 1) * 200 + pos.y, pos.z) * RotX(unk_6C2).
                let p = look.root_pitch;
                let t = Vec3::new(pos.x, (eng_math::cos_s(p) - 1.0) * 200.0 + pos.y, pos.z);
                parent * Mat4::from_translation(t) * Mat4::from_rotation_x(r(p)) * eng_anim::skeleton::local_transform(Vec3::ZERO, rot)
            } else {
                parent * pre * eng_anim::skeleton::local_transform(pos, rot)
            };
            done[l] = true;
        }
    }
    out
}

/// `play->damagePlayer(play, damage)` from another actor's update (`Player_Init` sets it to
/// `Player_InflictDamage`), Player in the actor arena: unless in a blocking cutscene mode
/// (`Player_InBlockingCsMode`) or invincible (`func_80837B18`), `Health_ChangeBy(damage)`; true
/// when it took the last of Link's health (`PLAYER_STATE2_7` cleared).
pub fn play_damage_player(play: &mut PlayState, damage: i32) -> bool {
    let trigger_start = play.transition.trigger == TRANS_TRIGGER_START;
    let Some(h) = play.player else { return false };
    let Some(p) = play.actors.downcast::<Player>(h) else { return false };
    // Player_InBlockingCsMode.
    if p.state1 & (STATE1_7 | STATE1_29) != 0 || p.cs_mode != 0 || trigger_start || p.state1 & STATE1_0 != 0 || p.state3 & STATE3_7 != 0 {
        return false;
    }
    // func_80837B18: nothing while invincible; Health_ChangeBy (a gain's sound centred).
    if p.invincibility_timer != 0 || p.actor.category != ACTORCAT_PLAYER {
        return false;
    }
    if oot_game::item::health_change_by(&mut play.save, Some(&mut play.audio), damage as i16) {
        return false;
    }
    if let Some(p) = play.actors.downcast_mut::<Player>(h) {
        p.state2 &= !STATE2_7;
    }
    true
}

/// `PLAYER_IA_DEKU_STICK` (`player.h`: 6), as `heldItemAction` (`held_item_ap`).
pub const PLAYER_IA_DEKU_STICK: i32 = 6;

impl Player {
    /// `Player_IsBurningStickInRange` (`z_player_lib.c`): a burning Deku Stick in hand
    /// (`unk_860` its timer) with its tip (`MELEE_WEAPON_INFO_TIP(&meleeWeaponInfo[0])`) within
    /// `xz_range` across of `pos` and 0 to `y_range` above it.
    pub fn is_burning_stick_in_range(&self, pos: Vec3, xz_range: f32, y_range: f32) -> bool {
        if self.held_item_ap == PLAYER_IA_DEKU_STICK && self.unk_860 != 0 {
            // Math_Vec3f_Diff(tip, pos, &diff).
            let diff = self.melee_weapon_info[0].tip - pos;
            ((diff.x * diff.x) + (diff.z * diff.z)) <= (xz_range * xz_range) && 0.0 <= diff.y && diff.y <= y_range
        } else {
            false
        }
    }
}

impl ActorImpl for Player {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `Player_Update` (`Player_DoNothing`, which does nothing, for start mode 0).
    fn update(&mut self, play: &mut PlayState) {
        if self.inert {
            return;
        }
        let io = RefCell::new(play.take_io());
        let assets = play.assets.clone();
        let audio_tables = play.audio.tables.clone();
        let env = Env {
            data: &play.data,
            col: &play.col,
            cam_input_yaw: play.input_dir_yaw(),
            gameplay_frames: play.gameplay_frames,
            actors: &play.actors,
            target: play.target_ctx.view(),
            io: &io,
            exits: play.exit_list(),
            entrances: assets.as_ref().map(|a| a.scenes.entrances.as_slice()).unwrap_or(&[]),
            transi_actors: &play.transi_actors,
            prev_room: play.room_ctx.prev.num,
            cam_state_flags: play.game_camera.state_flags,
            msg_state: play.message_state(),
            room_behavior_type1: play.room_ctx.cur.behavior_type1,
            me: play.player,
            cam_dir_yaw: play.cam_dir_yaw(),
            items: assets.as_ref().map(|a| &a.items).unwrap_or(&NO_ITEMS),
            audio: &audio_tables,
            cs_state: play.cs_ctx.state,
            cs_frames: play.cs_ctx.frames,
            cs_link_action: play.cs_ctx.link_action,
            scene_id: play.scene_id,
            game_over_state: play.game_over_ctx.state,
            room_behavior_type2: play.room_ctx.cur.behavior_type2,
            active_cam_id: play.active_cam_id,
            main_cam_valid_modes: play.data.camera.setting_flags(play.game_camera.setting),
            main_cam_mode: play.game_camera.mode,
            scene_cam_type: play.scene_cam_type,
        };
        // Player_Update: no input while talking or in a cutscene's hold (PLAYER_STATE1_5,
        // _29), and no A, B or C-Up for textboxBtnCooldownTimer frames after a talk.
        let mut input = play.input;
        if self.state1 & ((1 << 5) | STATE1_29) != 0 {
            input = Input::default();
        } else if self.textbox_btn_cooldown_timer != 0 {
            let mask = !(BTN_A | eng_input::pad::BTN_B | eng_input::pad::BTN_CUP);
            input.cur.button &= mask;
            input.press.button &= mask;
        }
        Player::update(self, &env, input);
        play.put_io(io.into_inner());
        // The requests run where the C's calls are, inside Player's update, with Player in place
        // for the camera (PlayState::player_out_of_arena).
        play.player_out_of_arena = play.player.and_then(|h| Some((play.player_view_of_impl(self)?, play.cam_actor_of(h, &self.actor))));
        for r in std::mem::take(&mut self.play_requests) {
            if let PlayRequest::SpawnHeldArrow { pos, yaw, params } = r {
                // Actor_SpawnAsChild: Player (the updating actor) its parent, it his child.
                self.held_actor = play.actor_spawn_as_child(&mut self.actor, ACTOR_EN_ARROW, pos, [0, yaw, 0], params).map_err(|e| log::debug!("En_Arrow: {e:?}")).ok();
                continue;
            }
            apply_play_request(play, r);
        }
        play.player_out_of_arena = None;
        // Player_UpdateCamAndSeqModes' requests, in its order: Camera_SetViewParam, then
        // Camera_RequestMode.
        if let Some((mode, target)) = self.cam_request.take() {
            if let Some(t) = target {
                play.game_camera.set_target(t);
            }
            play.game_camera.change_mode(&play.data.camera, mode);
            play.camera_sfx();
        }
        // Then its sequence mode (targetCtx.bgmEnemy is never set: no enemies yet), outside
        // the fishing pond.
        if self.actor.category == ACTORCAT_PLAYER && play.scene_id != oot_game::play_scene::SCENE_FISHING_POND {
            let m = self.seq_mode();
            play.audio.set_sequence_mode(m);
        }
        self.update_colliders(play);
    }

    fn animation_update(&mut self) {
        self.finish_frame();
    }

    /// What `Player_Draw` changes: the foot IK (`func_8008F87C`) writes into the joint table,
    /// then the limbs record the body parts, and the left hand places the sword's colliders.
    fn draw_update(&mut self, play: &mut PlayState) {
        if self.inert {
            return;
        }
        // Player_Draw (not under PLAYER_STATE2_29): while the invincibility is visible, the red
        // fog's far plane swings between 2000 and 6000 (Gfx_SetFog2(255, 0, 0, 0, 0, far)), by
        // damageFlickerAnimCounter stepped faster as the timer runs out.
        self.damage_flash_far = None;
        if self.state2 & STATE2_29 == 0 && self.invincibility_timer > 0 {
            let step = (50 - self.invincibility_timer as i32).clamp(8, 40);
            self.damage_flicker_anim_counter = self.damage_flicker_anim_counter.wrapping_add(step as u8);
            let far = 4000 - (cos_s((self.damage_flicker_anim_counter as i32 * 256) as i16) * 2000.0) as i32;
            self.damage_flash_far = Some(far);
        }
        if play.debug.foot_ik {
            self.legs = Some(self.apply_foot_ik(&play.data, &play.col));
        }
        let data = play.data.clone();
        let world = self.update_body_parts(&data);
        // Player_PostLimbDrawGameplay, PLAYER_LIMB_L_HAND: leftHandPos.
        self.left_hand_pos = world[data.limb("L_HAND")].transform_point3(Vec3::ZERO);
        self.post_limb_draw_l_hand(play, world[data.limb("L_HAND")]);
        // PLAYER_LIMB_R_HAND: the bow's or slingshot's string drawn back.
        if self.right_hand_is_bow_slingshot(play) {
            self.post_limb_draw_r_hand_string(world[data.limb("R_HAND")]);
        }
        // PLAYER_LIMB_R_HAND: the shield in hand (sRightHandLimbModelShieldQuadVertices).
        let right_hand_is_shield = self.right_hand_is_shield(play);
        if self.actor.scale.y >= 0.0 && right_hand_is_shield {
            const V: [Vec3; 4] = [Vec3::new(-4500.0, -3000.0, -600.0), Vec3::new(1500.0, -3000.0, -600.0), Vec3::new(-4500.0, 3000.0, -600.0), Vec3::new(1500.0, 3000.0, -600.0)];
            self.shield_mf = world[data.limb("R_HAND")];
            self.update_shield_collider(play, world[data.limb("R_HAND")], &V);
        }
        // PLAYER_LIMB_SHEATH, the right hand not holding the shield (RH_FF, Farore's Wind, isn't
        // ported): the child's Hylian Shield guards on his back (sSheathLimbModelShieldQuadVertices),
        // and the shield's matrix is on the back (sSheathLimbModelShieldOnBackPos, ...ZyxRot).
        if self.actor.scale.y >= 0.0 && !right_hand_is_shield {
            const V: [Vec3; 4] = [Vec3::new(-3000.0, -3000.0, -900.0), Vec3::new(3000.0, -3000.0, -900.0), Vec3::new(-3000.0, 3000.0, -900.0), Vec3::new(3000.0, 3000.0, -900.0)];
            let sheath = world[data.limb("SHEATH")];
            if self.is_child_with_hylian_shield() {
                self.update_shield_collider(play, sheath, &V);
            }
            let mut mf = oot_game::sys_matrix::MtxF::from_mat4(sheath);
            mf.translate_rotate_zyx(Vec3::new(630.0, 100.0, -30.0), [0, 0, 0x7FFF]);
            self.shield_mf = mf.to_mat4();
        }
    }


    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            COLLIDER_CYLINDER => Some(ColliderMut::Cylinder(&mut self.cylinder)),
            COLLIDER_SWORD_0 => Some(ColliderMut::Quad(&mut self.melee_weapon_quads[0])),
            COLLIDER_SWORD_1 => Some(ColliderMut::Quad(&mut self.melee_weapon_quads[1])),
            COLLIDER_SHIELD => Some(ColliderMut::Quad(&mut self.shield_quad)),
            _ => None,
        }
    }

    fn render_state(&self) -> RenderState {
        let look = self.look_rotations();
        let mut angles = vec![0i16; 8];
        angles[rs::HEAD..rs::HEAD + 3].copy_from_slice(&look.head);
        angles[rs::UPPER_Y] = look.upper_y;
        angles[rs::UPPER_X] = look.upper_x;
        angles[rs::UPPER_Z] = look.upper_z;
        angles[rs::ROOT_PITCH] = look.root_pitch;
        // Matrix_RotateZYX(0, play->gameplayFrames * 1000, 0) (Player_DrawGetItemImpl).
        angles[rs::GET_ITEM_SPIN] = (self.gameplay_frames as i32).wrapping_mul(1000) as i16;
        let mut values = vec![0.0f32; 10];
        values[rs::SPEED_XZ] = self.actor.speed_xz;
        values[rs::STICK_LENGTH] = self.unk_85c;
        values[rs::STRING] = self.unk_858;
        let f = self.actor.focus_pos;
        values[rs::FOCUS..rs::FOCUS + 3].copy_from_slice(&[f.x, f.y, f.z]);
        values[rs::Y_OFFSET] = self.actor.shape_y_offset;
        let r = self.get_item_ref_pos();
        values[rs::GET_ITEM_POS..rs::GET_ITEM_POS + 3].copy_from_slice(&[r.x, r.y, r.z]);
        let mut switches = vec![0u32; 13];
        switches[rs::DEKU_STICK] = (self.item_ap == PLAYER_IA_DEKU_STICK) as u32;
        switches[rs::UNK_6AD] = self.unk_6AD as u32;
        switches[rs::FACE] = self.face as u32;
        switches[rs::MODEL_GROUP] = self.model_group as u32;
        switches[rs::SHIELD] = self.current_shield as u32;
        switches[rs::UNK_862] = self.unk_862 as u16 as u32;
        switches[rs::EXCHANGE] = (self.exchange_item_id != 0) as u32;
        switches[rs::CRAWLING] = (self.state2 & STATE2_18 != 0) as u32;
        switches[rs::HIDDEN] = (self.state2 & STATE2_29 != 0) as u32;
        switches[rs::SHADOW] = self.shadow_feet as u32;
        switches[rs::DAMAGE_FLASH_FAR] = self.damage_flash_far.map(|f| f as u32).unwrap_or(0);
        // The ice's scale while frozen: (actionVar1 >> 1) * 22.
        switches[rs::ICE_SCALE] = if self.state2 & STATE2_14 != 0 { ((self.action_var1 >> 1) as i32 * 22) as u32 } else { 0 };
        switches[rs::HOLDING_SHIELD] = self.holding_shield as u32;
        RenderState {
            pos: self.actor.world_pos,
            rot: [0, self.actor.shape_rot.y, 0],
            scale: self.actor.scale,
            y_offset: self.actor.shape_y_offset,
            joints: Some(self.draw_joints()),
            angles,
            values,
            switches,
            teleported: self.actor.teleported,
            color_filter: (0, 0),
        }
    }

    /// `Player_Draw`: Link (`SkelAnime_DrawFlexLod` with the model group's hands and sheath for
    /// the shield worn and, for the child, whether the Kokiri Sword is on B; the face on
    /// segments 8 and 9, running fists), the item held up (`Player_DrawGetItem`), and the circle
    /// shadow.
    fn draw(&self, st: &RenderState, play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        use eng_gfx::{DrawCmd, MeshKey};
        use glam::Mat4;
        // Player_Draw draws nothing under PLAYER_STATE2_29.
        if self.inert || st.switches.get(rs::HIDDEN).copied().unwrap_or(0) != 0 {
            return;
        }
        let age = oot_game::player_lib::Age::from_adult(self.adult);
        let Some(joints) = &st.joints else { return };
        let look = LookRotations::from_angles(&st.angles);
        let bones = pose(&play.data.rigs[age as usize], joints, &look, play.data.limb("HEAD"), play.data.limb("UPPER"));
        let root = oot_game::play::actor_matrix(st.pos + Vec3::Y * (st.values[rs::Y_OFFSET] * 0.01), st.rot[1], 0.01);
        // Open hands close into fists above speed 2 (Player_OverrideLimbDrawGameplayDefault).
        let fists = st.values[rs::SPEED_XZ] > 2.0;
        let rules = &play.rules;
        let (eye, mouth) = rules.face_indices(joints.face, st.switches[rs::FACE] as usize);
        let group_name = play.data.items.model_group_names.get(st.switches[rs::MODEL_GROUP] as usize).cloned().unwrap_or_default();
        let default = rules.model_group("DEFAULT").unwrap_or(0);
        let loadout = oot_game::player_lib::Loadout {
            age,
            model_group: rules.model_group(&group_name).unwrap_or(default),
            shield: st.switches[rs::SHIELD] as usize,
            tunic: 0,
            // Player_OverrideLimbDrawGameplayDefault reads the save's B item as it draws.
            child_has_kokiri_sword: play.save.equips.button_items[0] == oot_game::item::ITEM_SWORD_KOKIRI,
            moving_fast: fists,
            holding_shield: st.switches.get(rs::HOLDING_SHIELD).copied().unwrap_or(0) != 0,
            first_person: false,
        };
        // In first person (unk_6AD), with the head (focus.pos) projected behind -4
        // (SkinMatrix_Vec3fMtxFMultXYZ with viewProjectionMtxF),
        // Player_OverrideLimbDrawGameplayFirstPerson: nothing in the look, only the arms aiming.
        // Else in a crawlspace, with Link behind the near plane (actor.projectedPos.z < 0),
        // Player_OverrideLimbDrawGameplayCrawling draws no limb: the crawl's camera is inside him.
        let unk_6ad = st.switches.get(rs::UNK_6AD).copied().unwrap_or(0);
        let v = &st.values;
        let focus = Vec3::new(v[rs::FOCUS], v[rs::FOCUS + 1], v[rs::FOCUS + 2]);
        let first_person = unk_6ad != 0 && (play.view_proj * focus.extend(1.0)).z < -4.0;
        let loadout = oot_game::player_lib::Loadout { first_person, ..loadout };
        let mesh = MeshKey { name: loadout.variant_key(rules), segment_textures: vec![(8, eye as u16), (9, mouth as u16)] };
        let projected_z = (play.view_proj * st.pos.extend(1.0)).z;
        let limbs_drawn = if unk_6ad != 0 { !first_person || unk_6ad == 2 } else { !(st.switches[rs::CRAWLING] != 0 && projected_z < 0.0) };
        // Player_PostLimbDrawGameplay, PLAYER_LIMB_R_HAND with the bow or slingshot in the right
        // hand (rightHandType BOW_SLINGSHOT): its string (sBowSlingshotStringData), at its point
        // from the hand, stretched along y by unk_858 and for the child turned by unk_858 * -0.2
        // about z, in the XLU list (whatever the limbs draw).
        if self.right_hand_is_bow_slingshot(play) {
            let (file, dl, p) = BOW_SLINGSHOT_STRING[(!self.adult) as usize];
            let hand = root * bones[play.data.limb("R_HAND")];
            let len = st.values[rs::STRING];
            let mut m = hand * Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(1.0, len, 1.0));
            if !self.adult {
                m *= Mat4::from_rotation_z(len * -0.2);
            }
            out.xlu.push(DrawCmd::new(MeshKey::named(oot_game::pack::keys::mesh(file, dl)), m));
        }
        if limbs_drawn {
            // The hit flash: Link's OPA lists in the red fog (Play_SetFog puts the scene's back).
            let flash = st.switches.get(rs::DAMAGE_FLASH_FAR).copied().unwrap_or(0);
            let fog = (flash != 0).then(|| oot_game::gbi::gfx_set_fog(255, 0, 0, 0, 0, flash as i32));
            // Player_PostLimbDrawGameplay, PLAYER_LIMB_L_HAND: a Deku Stick in use,
            // gLinkChildLinkDekuStickDL in the hand (translated, turned, and stretched along its
            // length by unk_85C), in Link's OPA list.
            if st.switches.get(rs::DEKU_STICK).copied().unwrap_or(0) != 0 {
                let hand = root * bones[play.data.limb("L_HAND")];
                let mut mf = oot_game::sys_matrix::MtxF::from_mat4(hand);
                mf.translate_rotate_zyx(Vec3::new(-428.26, 267.2, -33.82), [-0x8000, 0, 0x4000]);
                let m = mf.to_mat4() * Mat4::from_scale(Vec3::new(1.0, st.values[rs::STICK_LENGTH], 1.0));
                let stick = MeshKey::named(oot_game::pack::keys::mesh("object_link_child", "gLinkChildLinkDekuStickDL"));
                out.opa.push(DrawCmd { mesh: stick, transform: m, bones: Vec::new(), params: eng_gfx::DrawParams { fog, ..Default::default() } });
            }
            out.opa.push(DrawCmd { mesh, transform: root, bones, params: eng_gfx::DrawParams { fog, ..Default::default() } });
        }
        // Frozen (PLAYER_STATE2_14): the ice, at Actor_Draw's matrix scaled by
        // (actionVar1 >> 1) * 22, in the XLU list.
        let ice = st.switches.get(rs::ICE_SCALE).copied().unwrap_or(0);
        if ice != 0 {
            let mut sv = eng_gfx::SegmentValues::default();
            sv.read(SEG_ICE_SCROLL, &ice_scroll(play.gameplay_frames));
            let m = root * Mat4::from_scale(Vec3::splat(ice as f32));
            out.xlu.push(DrawCmd { mesh: MeshKey::named(oot_game::pack::keys::bake(ICE_BAKE)), transform: m, bones: Vec::new(), params: eng_gfx::DrawParams { segments: Some(sv), ..Default::default() } });
        }
        // Player_DrawGetItem (unk_862 > 0): GetItem_Draw at sGetItemRefPos, 3.3 in front and 14
        // up (6 for an exchange item; IREG(90) is 0), spinning, at 0.2.
        let unk_862 = st.switches[rs::UNK_862] as u16 as i16;
        if unk_862 > 0
            && let Some(a) = &play.assets
        {
            let yaw = st.rot[1];
            let height = if st.switches[rs::EXCHANGE] != 0 { 6.0 } else { 14.0 };
            let v = &st.values;
            let r = Vec3::new(v[rs::GET_ITEM_POS], v[rs::GET_ITEM_POS + 1], v[rs::GET_ITEM_POS + 2]);
            let t = Vec3::new(r.x + 3.3 * sin_s(yaw), r.y + height, r.z + 3.3 * cos_s(yaw));
            let m = Mat4::from_translation(t) * Mat4::from_rotation_y(eng_math::binang_to_rad(st.angles[rs::GET_ITEM_SPIN])) * Mat4::from_scale(Vec3::splat(0.2));
            oot_game::draw::get_item_draw(&a.items, unk_862.abs() - 1, m, play.gameplay_frames, view, out);
        }
        // ActorShadow_DrawFeet's stand-in: a soft disc on the floor below, shrinking with height
        // (none while shape.shadowDraw is NULL).
        if st.switches.get(rs::SHADOW).copied().unwrap_or(1) == 0 {
            return;
        }
        let (floor, _) = play.col.entity_raycast_down(st.pos + Vec3::Y * 20.0);
        let drop = (st.pos.y - floor).max(0.0);
        let size = if self.adult { 22.0 } else { 16.0 } * (1.0 - (drop / 400.0).min(0.7));
        let shadow = Mat4::from_translation(Vec3::new(st.pos.x, floor + 0.3, st.pos.z)) * Mat4::from_scale(Vec3::new(size, 1.0, size));
        out.xlu.push(DrawCmd::new(MeshKey::named(oot_game::play::builtin::SHADOW), shadow));
    }

    fn as_player(&self) -> Option<&dyn PlayerIface> {
        Some(self)
    }
    fn as_player_mut(&mut self) -> Option<&mut dyn PlayerIface> {
        Some(self)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl PlayerIface for Player {
    fn adult(&self) -> bool {
        self.adult
    }
    fn current_shield(&self) -> u8 {
        self.current_shield
    }
    fn melee_weapon_state(&self) -> i8 {
        self.melee_weapon_state as i8
    }
    fn shield_mf(&self) -> oot_game::sys_matrix::MtxF {
        oot_game::sys_matrix::MtxF::from_mat4(self.shield_mf)
    }
    fn set_cs_mode(&mut self, cs_mode: u8, actor: Option<ActorHandle>, door_bg_cam_index: i16) {
        self.cs_mode = cs_mode;
        self.cs_actor = actor;
        self.door_bg_cam_index = door_bg_cam_index;
    }
    fn cs_mode(&self) -> u8 {
        self.cs_mode
    }
    fn state_flags1(&self) -> u32 {
        self.state1
    }
    fn state_flags2(&self) -> u32 {
        self.state2
    }
    fn target(&self) -> Option<ActorHandle> {
        self.focus_actor
    }
    fn target_timer(&self) -> i16 {
        self.z_target_active_timer
    }
    fn stick_dir(&self) -> i8 {
        self.control_stick_directions[self.control_stick_data_index as usize]
    }
    fn focus(&self) -> Vec3 {
        self.actor.focus_pos
    }
    fn speed_xz(&self) -> f32 {
        self.actor.speed_xz
    }
    fn talk_target(&self) -> (Option<ActorHandle>, f32) {
        (self.target_actor, self.target_actor_distance)
    }
    fn set_talk_target(&mut self, actor: ActorHandle, distance: f32, exchange_item: u8) {
        self.target_actor = Some(actor);
        self.target_actor_distance = distance;
        self.exchange_item_id = exchange_item;
    }
    /// `Player_InBlockingCsMode` without `transitionTrigger` (magic isn't ported), or
    /// `unk_6AD == 4`.
    fn env_hazard_state(&self) -> (i16, u8, u8, bool) {
        (self.underwater_timer, self.current_boots, self.current_tunic, self.grounded())
    }
    fn in_cs_mode(&self) -> bool {
        self.state1 & (STATE1_7 | STATE1_29) != 0 || self.cs_mode != 0 || self.state1 & STATE1_0 != 0 || self.state3 & STATE3_7 != 0 || self.unk_6AD == 4
    }
    fn in_blocking_cs_mode(&self) -> bool {
        self.state1 & (STATE1_7 | STATE1_29) != 0 || self.cs_mode != 0 || self.state1 & STATE1_0 != 0 || self.state3 & STATE3_7 != 0
    }
    fn unk_a73(&self) -> u8 {
        self.unk_A73
    }
    /// Player holds no actors here (lifting isn't ported).
    fn holds_actor(&self) -> bool {
        false
    }
    fn get_item_direction(&self) -> i16 {
        self.get_item_direction
    }
    fn set_get_item(&mut self, actor: ActorHandle, get_item_id: i16, direction: i16) {
        self.get_item_id = get_item_id;
        self.interact_range_actor = Some(actor);
        self.get_item_direction = direction;
    }
    fn set_knockback(&mut self, damage: u8, kind: u8, yaw: i16, speed: f32, vy: f32) {
        self.knockback_damage = damage;
        self.knockback_type = kind;
        self.knockback_rot = yaw;
        self.knockback_speed = speed;
        self.knockback_y_velocity = vy;
    }
    fn invincibility_timer(&self) -> i8 {
        self.invincibility_timer
    }
    fn change_state_flags2(&mut self, set: u32, clear: u32) {
        self.state2 = (self.state2 | set) & !clear;
    }
    fn unk_89e(&self) -> u16 {
        self.floor_sfx_offset
    }
    fn current_boots(&self) -> u8 {
        self.current_boots
    }
    fn change_state_flags1(&mut self, set: u32, clear: u32) {
        self.state1 = (self.state1 | set) & !clear;
    }
    fn set_equipment_data(&mut self, data: &GameData, save: &oot_game::save::SaveContext) {
        Player::set_equipment_data(self, data, save);
    }
    fn body_part(&self, i: usize) -> Vec3 {
        self.body_parts_pos.get(i).copied().unwrap_or(self.actor.world_pos)
    }
    fn navi_text_id(&self) -> i16 {
        self.navi_text_id
    }
    fn set_navi_text_id(&mut self, id: i16) {
        self.navi_text_id = id;
    }
    fn navi_actor(&self) -> Option<ActorHandle> {
        self.navi_actor
    }
}

/// Applies a `PlayRequest` (see there), after Player's update.
fn apply_play_request(play: &mut PlayState, r: PlayRequest) {
    let d = play.data.clone();
    match r {
        PlayRequest::CamSetting(s) => {
            // !Play_CamIsNotFixed: Interface_ChangeHudVisibilityMode(2) for SCENE_TRANSITION (no interface).
            if play.cam_is_not_fixed() {
                play.game_camera.change_setting(&d.camera, s);
            }
        }
        PlayRequest::ChangeSetting(s) => {
            play.game_camera.change_setting(&d.camera, s);
        }
        PlayRequest::DoorCam { door, bg_cam_index, timers } => {
            play.game_camera.change_door_cam(&d.camera, &play.col, Some(door), bg_cam_index, timers[0], timers[1], timers[2]);
        }
        PlayRequest::CamDone => play.game_camera.set_finished_flag(),
        PlayRequest::RoomLoad(n) => {
            play.room_request(n);
        }
        PlayRequest::RoomChangeDone => play.room_change_done(),
        PlayRequest::OpenDoor { door, open_anim } => {
            if let Some(dr) = play.actors.downcast_mut::<crate::en_door::EnDoor>(door) {
                dr.open_anim = open_anim;
                dr.player_is_opening = true;
            }
        }
        PlayRequest::SlidingDoorActive(door) => {
            if let Some(dr) = play.actors.downcast_mut::<crate::door_shutter::DoorShutter>(door) {
                dr.is_active = 1;
            }
        }
        PlayRequest::DoorRoom { door } => {
            let room = play.room_ctx.cur.num;
            let attached = play.actors.actor_mut(door).and_then(|a| {
                a.room = room;
                a.child.or(a.parent)
            });
            if let Some(a) = attached.and_then(|h| play.actors.actor_mut(h)) {
                a.room = room;
            }
        }
        PlayRequest::TalkRequest(h) => {
            if let Some(a) = play.actors.actor_mut(h) {
                a.flags |= ACTOR_FLAG_TALK;
            }
        }
        PlayRequest::SetTextId { actor, text_id } => {
            if let Some(a) = play.actors.actor_mut(actor) {
                a.text_id = text_id;
            }
        }
        PlayRequest::StartTextbox { text_id, actor } => play.start_textbox(text_id, actor),
        PlayRequest::DoAction(action) => play.interface_ctx.set_do_action(action),
        PlayRequest::SetParent(h) => {
            let me = play.player;
            if let Some(a) = play.actors.actor_mut(h) {
                a.parent = me;
            }
        }
        PlayRequest::ChestOpen { chest, unk_1f4 } => {
            if let Some(c) = play.actors.downcast_mut::<crate::en_box::EnBox>(chest) {
                c.unk_1f4 = unk_1f4;
            }
        }
        PlayRequest::SetCameraData { data2 } => play.game_camera.set_camera_data(4, data2, 0),
        PlayRequest::OnePointCutscene { cs_id, timer, player, parent } => {
            let actor = if player { play.player.and_then(|h| play.cam_actor(h)) } else { None };
            play.onepoint_cutscene_init(cs_id, timer, actor, parent);
        }
        PlayRequest::DropCollectible { pos, params } => {
            crate::en_item00::item_drop_collectible(play, pos, params);
        }
        PlayRequest::ItemGive(item) => {
            oot_game::item::item_give(&mut play.save, Some(&mut play.audio), item);
        }
        PlayRequest::NaviCall(state) => {
            let cs_idle = play.cs_ctx.state == oot_game::cutscene::CS_STATE_IDLE;
            play.interface_ctx.set_navi_call(state, cs_idle, &mut play.audio);
        }
        PlayRequest::GetItemFanfare(gi) => {
            use oot_game::audio::*;
            use oot_game::item::*;
            if (GI_RUPEE_GREEN..=GI_RUPEE_RED).contains(&gi) || (GI_RUPEE_PURPLE..=GI_RUPEE_GOLD).contains(&gi) || (GI_RUPEE_GREEN_LOSE..=GI_RUPEE_PURPLE_LOSE).contains(&gi) || gi == GI_RECOVERY_HEART {
                play.audio.play_sfx_centered(oot_game::audio::sfx::NA_SE_SY_GET_BOXITEM);
            } else {
                let pieces = play.save.inventory.quest_items & 0xF000_0000;
                let temp1 = if gi == GI_HEART_CONTAINER_2 || gi == GI_HEART_CONTAINER || (gi == GI_HEART_PIECE && pieces == 4 << QUEST_HEART_PIECE_COUNT) {
                    NA_BGM_HEART_GET | 0x900
                } else if gi == GI_HEART_PIECE {
                    NA_BGM_SMALL_ITEM_GET
                } else {
                    NA_BGM_ITEM_GET | 0x900
                };
                play.audio.play_fanfare(temp1);
            }
        }
        PlayRequest::FadeOutAllSeq(n) => play.audio.func_800f6964(n),
        PlayRequest::GameOverState(s) => play.game_over_ctx.state = s,
        PlayRequest::LetterboxSizeTarget(t) => play.letterbox.set_size_target(t),
        PlayRequest::SpawnReviveFairy(pos) => {
            if let Err(e) = play.actor_spawn(crate::en_elf::ACTOR_EN_ELF, pos, [0; 3], crate::en_elf::FAIRY_REVIVE_DEATH) {
                log::debug!("the revival's fairy: {e:?}");
            }
        }
        PlayRequest::BurnDekuShield(pos) => {
            // Actor_Spawn(ACTOR_ITEM_SHIELD, params 1): the burning shield thrown off.
            if let Err(e) = play.actor_spawn(crate::item_shield::ACTOR_ITEM_SHIELD, pos, [0; 3], 1) {
                log::debug!("Item_Shield: {e:?}");
            }
            play.start_textbox(0x305F, None);
        }
        PlayRequest::ShieldParticles { pos, metal } => {
            let fx = if metal { cc::HitFx::ShieldParticlesMetal(pos) } else { cc::HitFx::ShieldParticlesWood(pos) };
            play.collision_check_hit_fx(vec![fx]);
        }
        PlayRequest::FreezeFlash => {
            if play.actors.freeze_flash_timer == 0 {
                play.actors.freeze_flash_timer = 1;
            }
        }
        PlayRequest::Blood { kind, pos } => play.collision_check_hit_fx(vec![cc::HitFx::Blood(kind, pos)]),
        PlayRequest::Quake { speed, y, duration } => {
            // Player_RequestQuake: the s32s passed on as the setters' s16s.
            let quake_index = play.quake_request(oot_game::camera::CAM_ID_MAIN, oot_game::quake::QUAKE_TYPE_3);
            play.quake_set_speed(quake_index, speed as i16);
            play.quake_set_perturbations(quake_index, y as i16, 0, 0, 0);
            play.quake_set_duration(quake_index, duration as i16);
        }
        PlayRequest::TitleCardClear => {
            play.title_ctx.clear();
        }
        // Handled by Player's update, which keeps the handle.
        PlayRequest::SpawnHeldArrow { .. } => {}
        PlayRequest::ReleaseHeld(h) => {
            if let Some(a) = play.actors.actor_mut(h) {
                a.parent = None;
            }
        }
        PlayRequest::SpawnArrow { pos, rot, params } => {
            if let Err(e) = play.actor_spawn(ACTOR_EN_ARROW, pos, rot, params) {
                log::debug!("En_Arrow: {e:?}");
            }
        }
        PlayRequest::CamRequestMode(mode) => {
            play.game_camera.change_mode(&play.data.camera, mode);
            play.camera_sfx();
        }
        PlayRequest::Audio(a) => match a {
            PlayerAudio::BgmVolumeOffDuringFanfare => play.audio.audio_set_bgm_volume_off_during_fanfare(),
            PlayerAudio::BgmVolumeOnDuringFanfare => play.audio.audio_set_bgm_volume_on_during_fanfare(),
            PlayerAudio::StopBgmAndFanfare(d) => play.audio.audio_stop_bgm_and_fanfare(d),
            PlayerAudio::PlayFanfare(id) => play.audio.play_fanfare(id),
        },
        PlayRequest::Sfx(s) => {
            use oot_game::audio::sfx::SfxPos;
            let Some(me) = play.player else { return };
            let pos = SfxPos::Actor(me);
            let a = &mut play.audio;
            match s {
                PlayerSfx::NoPos(id) => a.play_sfx_centered(id),
                PlayerSfx::Actor(id) => a.play_sfx_at_pos(pos, id),
                PlayerSfx::Footstep(id, v) => a.func_800f4010(pos, id, v),
                PlayerSfx::F4138(id, v) => a.func_800f4138(pos, id, v),
                PlayerSfx::F4190(id) => a.func_800f4190(pos, id),
                PlayerSfx::CodeReverb(r) => a.set_code_reverb(r),
                PlayerSfx::StopById(id) => a.stop_sfx_by_id(id),
                PlayerSfx::BaseFilter(f) => a.set_base_filter(f),
            }
        }
    }
}

/// `func_8083816C`: floor types that count as a void floor with `FLOOR_PROPERTY_12`.
fn func_8083816C(floor_type: u32) -> bool {
    floor_type == FLOOR_TYPE_4 || floor_type == FLOOR_TYPE_7 || floor_type == FLOOR_TYPE_12
}

/// No item tables (play without the pack).
static NO_ITEMS: oot_game::item::ItemTables = oot_game::item::ItemTables { get_items: Vec::new(), draw_items: Vec::new() };

