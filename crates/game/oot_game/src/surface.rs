//! What a collision poly's surface type means (`SurfaceType_Get*` in `z_bgcheck.c`). The
//! engine's collision returns the raw data words (`CollisionContext::surface_word`); the game
//! reads its floor types, wall flags, floor properties and so on out of them.
//!
//! Word 0: bg camera bits 0..7, exit index 8..12, floor type 13..17, wall type 21..25, floor property 26..29, soft bit 30.
//! Word 1: sfx type 0..3, floor effect 4..5, conveyor speed 18..20, bit 27.

use eng_collision::bgcheck::{CollisionContext, PolyId};

/// `D_80119D90`: wall type → wall flags.
pub const WALL_FLAGS: [u32; 8] = [0, 1, 1 | 2, 1 | 4, 8, 16, 32, 64];
pub const WALL_FLAG_0: u32 = 1;
pub const WALL_FLAG_1: u32 = 2;
pub const WALL_FLAG_2: u32 = 4;
pub const WALL_FLAG_3: u32 = 8;
pub const WALL_FLAG_CRAWLSPACE_1: u32 = 16;
pub const WALL_FLAG_CRAWLSPACE_2: u32 = 32;
pub const WALL_FLAG_6: u32 = 64;
/// `WALL_FLAG_CRAWLSPACE` (`bgcheck.h`): either crawlspace flag.
pub const WALL_FLAG_CRAWLSPACE: u32 = WALL_FLAG_CRAWLSPACE_1 | WALL_FLAG_CRAWLSPACE_2;
/// `COLPOLY_IGNORE_PROJECTILES` (`bgcheck.h`): a poly flag, in the top 3 bits of its first
/// vertex index (`COLPOLY_VTX_CHECK_FLAGS_ANY`).
pub const COLPOLY_IGNORE_PROJECTILES: u16 = 1 << 2;

/// The `SurfaceType_Get*` accessors, on the collision context that owns the poly.
pub trait SurfaceType {
    /// `SurfaceType_GetBgCamIndex`.
    fn bg_cam_index(&self, id: PolyId) -> u32;
    /// `SurfaceType_GetExitIndex`: 1-based into the scene's exit list, 0 for none.
    fn exit_index(&self, id: PolyId) -> u32;
    /// `SurfaceType_GetFloorType`.
    fn floor_type(&self, id: PolyId) -> u32;
    /// `SurfaceType_GetWallType`.
    fn wall_type(&self, id: PolyId) -> u32;
    /// `SurfaceType_GetWallFlags`.
    fn wall_flags(&self, id: PolyId) -> u32;
    /// `SurfaceType_GetFloorProperty`.
    fn floor_property(&self, id: PolyId) -> u32;
    /// `SurfaceType_IsSoft`.
    fn is_soft(&self, id: PolyId) -> bool;
    /// `SurfaceType_GetMaterial`.
    fn sfx_type(&self, id: PolyId) -> u32;
    /// `SurfaceType_GetFloorEffect`.
    fn floor_effect(&self, id: PolyId) -> u32;
    /// `SurfaceType_GetConveyorSpeed`.
    fn conveyor_speed(&self, id: PolyId) -> u32;
    /// `func_80042108`: surface data[1] bit 27.
    fn flag27(&self, id: PolyId) -> bool;
    /// `SurfaceType_GetEcho`: the floor's reverb (`Audio_SetCodeReverb`).
    fn echo(&self, id: PolyId) -> u32;
    /// `SurfaceType_IsIgnoredByProjectiles`: the poly's `COLPOLY_IGNORE_PROJECTILES` flag (a bg
    /// actor's poly always has its header here, which the C checks first).
    fn is_ignored_by_projectiles(&self, id: PolyId) -> bool;
}

impl SurfaceType for CollisionContext {
    fn bg_cam_index(&self, id: PolyId) -> u32 {
        self.surface_word(id, 0) & 0xFF
    }
    fn exit_index(&self, id: PolyId) -> u32 {
        self.surface_word(id, 0) >> 8 & 0x1F
    }
    fn floor_type(&self, id: PolyId) -> u32 {
        self.surface_word(id, 0) >> 13 & 0x1F
    }
    fn wall_type(&self, id: PolyId) -> u32 {
        self.surface_word(id, 0) >> 21 & 0x1F
    }
    fn wall_flags(&self, id: PolyId) -> u32 {
        WALL_FLAGS.get(self.wall_type(id) as usize).copied().unwrap_or(0)
    }
    fn floor_property(&self, id: PolyId) -> u32 {
        self.surface_word(id, 0) >> 26 & 0xF
    }
    fn is_soft(&self, id: PolyId) -> bool {
        self.surface_word(id, 0) >> 30 & 1 != 0
    }
    fn sfx_type(&self, id: PolyId) -> u32 {
        self.surface_word(id, 1) & 0xF
    }
    fn floor_effect(&self, id: PolyId) -> u32 {
        self.surface_word(id, 1) >> 4 & 3
    }
    fn conveyor_speed(&self, id: PolyId) -> u32 {
        self.surface_word(id, 1) >> 18 & 7
    }
    fn flag27(&self, id: PolyId) -> bool {
        self.surface_word(id, 1) & 0x0800_0000 != 0
    }
    fn echo(&self, id: PolyId) -> u32 {
        self.surface_word(id, 1) >> 11 & 0x3F
    }
    fn is_ignored_by_projectiles(&self, id: PolyId) -> bool {
        self.poly(id).vtx[0] & ((COLPOLY_IGNORE_PROJECTILES & 7) << 13) != 0
    }
}
