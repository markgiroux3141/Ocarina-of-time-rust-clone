# 0015: Camera settings and the scene's bg cameras

- **Status:** accepted, built in GAME-02 milestone 2; extends ADR 0013
- **Date:** 2026-09-28

## Context

ADR 0013 gave the camera its modes, but only for `CAM_SET_NORMAL0`. The game's camera also has
a *setting*: `sCameraSettings`, 65 of them, each with its own modes, valid-mode mask and
priority. Scenes change the setting through their *bg cameras*: `BgCamInfo` entries in the
collision header, each a setting plus data (usually a position, a rotation and a fov). The
entries are named from several places:

- **floors**: `Camera_Update` changes to the one under Player (`Camera_RequestBgCam`);
- **spawns**: `Play_Init` starts on the one in Player's params (`params & 0xFF`);
- **the viewpoint**: a fixed-camera scene's `Play_RequestViewpointBgCam` asks for
  `viewpoint - 1` every frame, and C-Up toggles it;
- **doors**: `Camera_ChangeDoorCam` with the transition actor's side, or `CAM_SET_DOORC`.

Player also changes the setting outright: `CAM_SET_SCENE_TRANSITION` on exits, `CAM_SET_FREE0`
on void-outs. The prerendered rooms, doors and exits of milestone 2 all hang on this.

Three things had to be decided:
1. how the data gets into the pack;
2. what the unported mode functions do now that there are settings;
3. how much of `stateFlags` / `paramData` to model.

## Decision

- **All of `sCameraSettings` is data.**
  - The importer reads every entry: its `unk_00` (the valid modes, the priority in bits
    24..27, and flags 0x40000000 / 0x80000000), and its `sCamSet*Modes` array, with
    `{ CAM_FUNC_NONE, 0, NULL }` holes.
  - Setting names come from `camera.h`'s enum (`CameraData::settings`).
  - The mode functions read their data by `(setting, mode)`.
- **The bg cameras are part of the collision header** (`CollisionHeader::bg_cams`).
  - The list has no count. The importer reads at least every index the collision names
    (surface types, water boxes), plus the scene's spawns, transition actors and viewpoints.
  - It then goes on while entries look like entries: a setting below `CAM_SET_MAX`, a count
    up to 0x400, a data pointer in the file or NULL. It stops before any list the header or
    an earlier entry points at, as the extractor does.
  - A test checks that every scene's list covers everything the scene names.
- **The setting changes are ported as they are**: `Camera_RequestSettingImpl` (priorities,
  `prevSetting`, the bg camera index flags 4 and 8), `Camera_RequestBgCam` (once a frame
  through `behaviorFlags & 0x40`), `Camera_ChangeDoorCam`, `func_80057FC4` (a room's starting
  setting), `Camera_SetFinishedFlag`, and the floor check in `Camera_Update`. The play state has the
  viewpoint and `R_SCENE_CAM_TYPE`.
- **Mode functions ported this milestone**, besides Normal1, Parallel1 and KeepOn1:
  - `Camera_Fixed2` (`PIVOT_CRAWLSPACE`);
  - `Camera_Fixed3` (`PREREND_FIXED`);
  - `Camera_Fixed4` (`PIVOT_IN_FRONT`, Link's porch);
  - `Camera_Data4` (`PIVOT_SHOP_BROWSING`);
  - `Camera_Unique0` (`START1`, at some Kokiri Forest entrances);
  - `Camera_Unique2` (`SCENE_TRANSITION`);
  - `Camera_Unique3` (`DOOR0`);
  - `Camera_Unique6` (`FREE0`);
  - `Camera_Unique7` (`PREREND_PIVOT`);
  - `Camera_Special9` (`DOORC`).
- **The fallback for an unported function.** Run the setting's NORMAL function if that one is
  ported (on the NORMAL mode's data), else Normal1 on NORMAL0's NORMAL data.
  - For NORMAL0 this is ADR 0013's rule.
  - For the prerendered settings, TALK (`Camera_KeepOn0`) keeps the fixed or pivot camera
    instead of flying off with Normal1.
- **`Play_Init`'s `Camera_OverwriteStateFlags(&mainCamera, 0xFF)` is `GameCamera::play_init_settings`.**
  - Among other bits it sets `stateFlags` bit 1, which enables the floor's bg cameras.
  - The spikes' view of a scene (`PlayState::new`, no `Play_Init`) keeps `Camera_Init`'s
    0x4000 | 4, so it runs without floor cameras, as before. That's why the goldens are
    unchanged.
- **`paramData.doorParams` is its own field.** In the C it shares a union with the functions'
  data, so a function's reload overwrites it. Here it lasts until the next
  `Camera_ChangeDoorCam`. The difference only shows for a `START1` camera with timer -1
  reached some other way than a spawn or a door.

## Consequences

- **Entering by `Play_Init` changes the camera wherever a floor names a bg camera.**
  - Link's porch is `PIVOT_IN_FRONT`.
  - Some Kokiri entrances start on `START1`.
  - `ROOM_TYPE_DUNGEON` rooms start on `DUNGEON0`.
  - Exits switch to `SCENE_TRANSITION`.
  - One climbing test steers at the ladder instead of holding the stick up, since the porch
    camera looks from in front of the house.
- **Not modelled:**
  - the underwater and hot-room settings (`Camera_UpdateWater`, `Camera_UpdateHotRoom`);
  - `-5` for the Sacred Forest Meadow's bird's-eye settings as adult;
  - DynaPoly floors' own bg cameras (only scene floors change the index, as the C's check
    `bgId == BGCHECK_SCENE` leaves it);
  - `camera->bgId`;
  - the sounds.
- **Still on the fallback:** the other 50-odd functions, among them Battle1, KeepOn0, KeepOn3,
  Jump1/2, Uniq1, Subj3/4 and Normal2/3. A setting whose NORMAL function isn't ported
  (`TOWER_CLIMB`'s Normal2, `CRAWLSPACE`'s Subj4) runs Normal1 on NORMAL0's data.
- `Camera_RequestBgCam` falls off the end of the C without a return value when refused;
  here it returns 0 (`@bug (game)`, no caller reads it).
