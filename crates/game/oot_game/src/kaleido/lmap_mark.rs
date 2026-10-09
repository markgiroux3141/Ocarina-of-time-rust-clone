//! `z_lmap_mark.c`: the pause map's marks (`PauseMapMark_Draw`), drawn on the dungeon map page
//! with the compass: the viewed floor's chests (until opened) and the boss's skull, from
//! `gPauseMapMarkDataTable` (`z_lmap_mark_data_mq.c`, the pack's `table/map`).

use glam::{Mat4, Vec3};

use super::gfx::KTex;
use super::*;
use crate::map::{PAUSE_MAP_MARK_BOSS, PAUSE_MAP_MARK_CHEST, PAUSE_MAP_MARK_NONE};

/// `SCENE_DEKU_TREE_BOSS` (0x11) .. `SCENE_GANONS_TOWER_COLLAPSE_EXTERIOR` (0x1A): the boss mark
/// pulses there; `SCENE_DEKU_TREE_BOSS` .. `SCENE_SHADOW_TEMPLE_BOSS` (0x18): no chest marks.
const SCENE_DEKU_TREE_BOSS: u16 = 0x11;
const SCENE_SHADOW_TEMPLE_BOSS: u16 = 0x18;
const SCENE_GANONS_TOWER_COLLAPSE_EXTERIOR: u16 = 0x1A;
/// `SCENE_ICE_CAVERN` (9): `PauseMapMark_Draw` draws in `SCENE_DEKU_TREE` (0) .. it.
const SCENE_ICE_CAVERN: u16 = 0x09;

impl PlayState {
    /// `PauseMapMark_Init`: the boss mark's pulse reset, the table loaded
    /// (`gLoadedPauseMarkDataTable`; the 64DD's hook is `PLATFORM_N64`'s).
    fn pause_map_mark_init(&mut self) {
        self.pause_ctx.boss_mark_state = 0;
        self.pause_ctx.boss_mark_scale = 1.0;
    }

    /// `PauseMapMark_Clear` (`gLoadedPauseMarkDataTable = NULL`).
    fn pause_map_mark_clear(&mut self) {}

    /// `PauseMapMark_DrawForDungeon`: the viewed floor's marks (`R_MAP_TEX_INDEX >> 1`), each list
    /// under the page's matrix moved to the map (-36, 21; 101 while the pages turn up or down),
    /// each mark at its point.
    fn pause_map_mark_draw_for_dungeon(&mut self) {
        let Some(a) = self.assets.clone() else { return };
        let Some(marks) = a.map.pause_marks.get((self.map.r_map_tex_index >> 1).max(0) as usize) else { return };
        let page = self.pause_ctx.gfx.model();
        for mark_data in marks.iter() {
            if mark_data.mark_type == PAUSE_MAP_MARK_NONE {
                break;
            }
            let p = &mut self.pause_ctx;
            let scale = if mark_data.mark_type == PAUSE_MAP_MARK_BOSS && (SCENE_DEKU_TREE_BOSS..=SCENE_GANONS_TOWER_COLLAPSE_EXTERIOR).contains(&self.scene_id) {
                if p.boss_mark_state == 0 {
                    eng_math::approach_f(&mut p.boss_mark_scale, 1.5, 1.0, 0.041);
                    if p.boss_mark_scale == 1.5 {
                        p.boss_mark_state = 1;
                    }
                } else {
                    eng_math::approach_f(&mut p.boss_mark_scale, 1.0, 1.0, 0.041);
                    if p.boss_mark_scale == 1.0 {
                        p.boss_mark_state = 0;
                    }
                }
                p.boss_mark_scale
            } else {
                1.0
            };
            let y = if p.state == PAUSE_STATE_OPENING_1 || p.state >= PAUSE_STATE_CLOSING { 101.0 } else { 21.0 };
            let list = page * Mat4::from_translation(Vec3::new(-36.0, y, 0.0));
            p.gfx.prim_color(255, 255, 255, 255);
            p.gfx.env_color(0, 0, 0, 255);
            for point in mark_data.points.iter().take(mark_data.count.max(0) as usize) {
                let display = if mark_data.mark_type == PAUSE_MAP_MARK_CHEST {
                    // Not once opened, nor in the boss rooms.
                    !self.flags.get_treasure(point.chest_flag as i32) && !(SCENE_DEKU_TREE_BOSS..=SCENE_SHADOW_TEMPLE_BOSS).contains(&self.scene_id)
                } else {
                    true
                };
                if display {
                    let p = &mut self.pause_ctx;
                    // (DEBUG_FEATURES: GREG(92), GREG(93) added to the point.)
                    let at = Vec3::new(point.x + p.regs.greg92 as f32, point.y + p.regs.greg93 as f32, 0.0);
                    p.gfx.matrix(list * Mat4::from_translation(at) * Mat4::from_scale(Vec3::splat(scale)));
                    p.gfx.vertex(&mark_data.vtx, mark_data.vtx_count.max(0) as usize, 0);
                    // gSP1Quadrangle(1, 3, 2, 0): its top left is vertex 0, top right 2, bottom
                    // left 1, bottom right 3 (sMarkChestVtx, sMarkBossVtx).
                    p.gfx.quad_at(KTex::MapMark(mark_data.mark_type as u8), [0, 2, 1, 3]);
                }
            }
        }
        // (Matrix_Pop restores the CPU's stack; the RSP keeps the last mark's matrix.)
    }

    /// `PauseMapMark_Draw`: in the ten dungeons, the viewed floor's marks.
    pub(super) fn pause_map_mark_draw(&mut self) {
        self.pause_map_mark_init();
        if self.scene_id <= SCENE_ICE_CAVERN {
            self.pause_map_mark_draw_for_dungeon();
        }
        self.pause_map_mark_clear();
    }
}
