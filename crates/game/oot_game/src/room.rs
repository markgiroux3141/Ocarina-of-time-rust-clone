//! Room drawing logic that runs every frame (`z_room.c`).

use glam::Vec3;

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
    /// `func_80096FD4`.
    pub const EMPTY: Room = Room { num: -1, loaded: false, behavior_type1: 0, behavior_type2: 0, lens_mode: 0, echo: 0 };
}

/// `RoomContext`: the current room, the previous one (still loaded and drawn while Player is
/// between them), and the load in flight.
///
/// Rooms are in the asset pack, so a load is instant; its timing is kept. `func_8009728C`
/// starts one (`status` 1), and the next `func_800973FC` finishes it: the room's header runs
/// (its actors are spawned in the next `Actor_UpdateAll`) and the transition actors spawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomContext {
    pub cur: Room,
    pub prev: Room,
    /// 0: idle, 1: a load in flight.
    pub status: u8,
    /// `unk_30`: which of the two room buffers the next load uses.
    pub unk_30: u8,
}

impl Default for RoomContext {
    fn default() -> RoomContext {
        RoomContext { cur: Room::EMPTY, prev: Room::EMPTY, status: 0, unk_30: 0 }
    }
}

impl RoomContext {
    /// `func_8009728C`: starts loading `num` into the current slot (the old current room becomes
    /// the previous one). False if a load is already in flight.
    pub fn request(&mut self, num: i8) -> bool {
        if self.status != 0 {
            return false;
        }
        self.prev = self.cur;
        self.cur = Room { num, ..Room::EMPTY };
        self.status = 1;
        self.unk_30 ^= 1;
        true
    }

    /// `EnHoll_SwapRooms`.
    pub fn swap(&mut self) {
        std::mem::swap(&mut self.cur, &mut self.prev);
        self.unk_30 ^= 1;
    }

    /// The loaded rooms `Play_Draw` draws: the current one, then the previous one.
    pub fn drawn(&self) -> impl Iterator<Item = i8> + '_ {
        [self.cur, self.prev].into_iter().filter(|r| r.loaded && r.num >= 0).map(|r| r.num)
    }
}
