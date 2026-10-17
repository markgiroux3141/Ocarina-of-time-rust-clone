//! Room drawing logic that runs every frame (`z_room.c`).

use eng_collision::bgcheck::CollisionContext;
use glam::Vec3;

use crate::camera::{CAM_SET_PREREND_FIXED, GameCamera, bg_cam_func_data};
use crate::scene::RoomData;

/// `Room_DrawImageSingle` / `Room_DrawImageMulti`'s background: drawn only while the active
/// camera is `CAM_SET_PREREND_FIXED`; a multi-background room's is the one for the camera's
/// bg camera, or for its `roomImageOverrideBgCamIndex` (`Room_GetImageMultiBgEntry`, which
/// hangs when there's none: nothing is drawn here). Not modelled: that function's write of the
/// index into Player's params, and the background's shift by the active camera's quake offset
/// (`Camera_GetQuakeOffset`: the renderer draws it unmoved).
pub fn image_background(cam: &GameCamera, col: &CollisionContext, room: &RoomData) -> Option<usize> {
    if cam.setting != CAM_SET_PREREND_FIXED || room.backgrounds.is_empty() {
        return None;
    }
    if room.backgrounds[0].bg_cam_index.is_none() {
        return Some(0);
    }
    let mut idx = cam.bg_cam_index as i32;
    if let Some(d) = bg_cam_func_data(col, idx)
        && d.flags >= 0
    {
        idx = d.flags as i32;
    }
    room.backgrounds.iter().position(|b| b.bg_cam_index.map(i32::from) == Some(idx))
}

/// `Room_DrawCullable`'s entry selection and order: project each bounding-sphere centre with
/// the view-projection matrix; keep entries with `-radius < z` and `z - radius < fogFar`;
/// draw them by ascending `z - radius` (stable for ties, as the insertion into the linked list
/// puts equal keys after existing ones). `clip_z` maps a world point to its clip-space z.
pub fn cullable_order(bounds: &[Option<(Vec3, f32)>], clip_z: impl Fn(Vec3) -> f32, fog_far: f32) -> Vec<usize> {
    let mut keep: Vec<(f32, usize)> = bounds
        .iter()
        .enumerate()
        .filter_map(|(i, b)| {
            let (c, r) = (*b)?;
            let z = clip_z(c);
            (-r < z && z - r < fog_far).then_some((z - r, i))
        })
        .collect();
    keep.sort_by(|a, b| a.0.total_cmp(&b.0));
    keep.into_iter().map(|k| k.1).collect()
}

/// `Room`: a room slot of the room context. `num` -1 is empty; `loaded` is `segment != NULL`
/// (the room file is in its buffer and its header has run).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Room {
    pub num: i8,
    pub loaded: bool,
    /// `SCENE_CMD_ID_ROOM_BEHAVIOR`: `behaviorType1`, `behaviorType2`, `lensMode`.
    pub behavior_type1: u8,
    pub behavior_type2: u8,
    pub lens_mode: u8,
    /// `SCENE_CMD_ID_ECHO_SETTINGS`.
    pub echo: u8,
}

impl Room {
    /// `Room_Init`.
    pub const EMPTY: Room = Room { num: -1, loaded: false, behavior_type1: 0, behavior_type2: 0, lens_mode: 0, echo: 0 };
}

/// `RoomContext`: the current room, the previous one (still loaded and drawn while Player is
/// between them), and the load in flight.
///
/// Rooms are in the asset pack, so a load is instant; its timing is kept. `Room_RequestNewRoom`
/// starts one (`status` 1), and the next `Room_ProcessRoomRequest` finishes it: the room's header runs
/// (its actors are spawned in the next `Actor_UpdateAll`) and the transition actors spawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomContext {
    pub cur: Room,
    pub prev: Room,
    /// 0: idle, 1: a load in flight.
    pub status: u8,
    /// `activeBufPage`: which of the two room buffers the next load uses.
    pub active_buf_page: u8,
}

impl Default for RoomContext {
    fn default() -> RoomContext {
        RoomContext { cur: Room::EMPTY, prev: Room::EMPTY, status: 0, active_buf_page: 0 }
    }
}

/// The room segment (`SEGMENTED_TO_VIRTUAL`'s 3): where a room's lists load its textures from.
pub const SEGMENT_ROOM: u32 = 0x0300_0000;

/// One region of a room's RAM the game has written: its bytes as they are now, and the RGBA16
/// texture they hold (width, height), for the room's draws.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomRegion {
    pub bytes: Vec<u8>,
    pub texture: (u32, u32),
}

/// What the game has written into the loaded rooms' RAM (King Dodongo's lava floor,
/// `gDodongosCavernBossLavaFloorTex`, through segment 3: the current room's buffer), by room and
/// the region's offset in the room's file. The region's bytes are copied from the pack where the
/// writes start, and lost when the room's buffer takes another room (a room load overwrites it).
/// The room's draws show the written textures in place of their own (`images`, as
/// `DrawParams::texture_images` by their segment-3 source; ADR 0051's pattern for objects).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoomRam {
    pub regions: std::collections::BTreeMap<(i8, u32), RoomRegion>,
}

impl RoomRam {
    /// The region of room `room`'s file at `offset` (an RGBA16 texture of `texture`'s size) to
    /// write into: `init` (the file's bytes there) the first time.
    pub fn region_mut(&mut self, room: i8, offset: u32, texture: (u32, u32), init: impl FnOnce() -> Vec<u8>) -> &mut Vec<u8> {
        &mut self.regions.entry((room, offset)).or_insert_with(|| RoomRegion { bytes: init(), texture }).bytes
    }

    /// The written region of `room` at `offset`, if any.
    pub fn region(&self, room: i8, offset: u32) -> Option<&[u8]> {
        self.regions.get(&(room, offset)).map(|r| r.bytes.as_slice())
    }

    /// Only `keep`'s regions stay (the rooms still in their buffers).
    pub fn keep_only(&mut self, keep: &[i8]) {
        self.regions.retain(|&(r, _), _| keep.contains(&r));
    }

    /// The images in place of `room`'s textures its RAM has written: `DrawParams::texture_images`
    /// for its draws (empty while the room is as its file has it).
    pub fn images(&self, room: i8) -> Vec<eng_gfx::SourceImage> {
        self.regions
            .range((room, 0)..=(room, u32::MAX))
            .map(|(&(_, offset), r)| eng_gfx::SourceImage { source: SEGMENT_ROOM | offset, image: crate::object_ctx::rgba16_image(&r.bytes, r.texture.0, r.texture.1) })
            .collect()
    }
}

impl RoomContext {
    /// `Room_RequestNewRoom`: starts loading `num` into the current slot (the old current room becomes
    /// the previous one). False if a load is already in flight.
    pub fn request(&mut self, num: i8) -> bool {
        if self.status != 0 {
            return false;
        }
        self.prev = self.cur;
        self.cur = Room { num, ..Room::EMPTY };
        self.status = 1;
        self.active_buf_page ^= 1;
        true
    }

    /// `EnHoll_SwapRooms`.
    pub fn swap(&mut self) {
        std::mem::swap(&mut self.cur, &mut self.prev);
        self.active_buf_page ^= 1;
    }

    /// The loaded rooms `Play_Draw` draws: the current one, then the previous one.
    pub fn drawn(&self) -> impl Iterator<Item = i8> + '_ {
        [self.cur, self.prev].into_iter().filter(|r| r.loaded && r.num >= 0).map(|r| r.num)
    }
}
