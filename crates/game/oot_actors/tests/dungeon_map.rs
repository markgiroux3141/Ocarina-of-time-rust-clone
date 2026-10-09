//! The pause menu's dungeon map page (GAME-05 milestone 5b-2) against the C
//! (`z_kaleido_map.c`, `z_lmap_mark.c`, `z_lmap_mark_data_mq.c`, `z_kaleido_scope.c`'s
//! `KaleidoScope_LoadDungeonMap`, `_UpdateDungeonMap`, `_OverridePalIndexCI4`, `z_map_exp.c`'s
//! `Map_SetFloorPalettesData`, `z_map_data.c`'s tables). Expected values are worked out from the C
//! in the comments.
//!
//! The Master Quest Deku Tree (`mapIndex` 0): `sFloorID[0]` = { 0, 0, 0, F_3F, F_2F, F_1F, F_B1,
//! F_B2 }, so floor 3 is 3F (cursor slot 6) .. floor 7 B2 (slot 10); `sFloorTexIndexOffset[0]` =
//! { 0, 0, 0, 0, 2, 4, 6, 8 } from `sDgnTexIndexBase[0]` 0; `sPaletteRoom[0]`: 3F rooms 0 and 10,
//! 2F 0, 1, 2, 1F 0, B1 3 to 8, B2 9; `sRoomPalette[0]` = { 10, 1, 2, ... }: room 0's palette is
//! 10. The spawn is on 1F (floor 5) in room 0. The `deku-tree-compass` preset has the compass,
//! floors 3 to 5 and rooms 0 to 2 visited.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_R, BTN_START, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::player::Action;
use oot_game::actor_ctx::ACTORCAT_ENEMY;
use oot_game::audio::sfx::{NA_SE_SY_CURSOR, NA_SE_SY_DECIDE};
use oot_game::camera::CAM_ID_MAIN;
use oot_game::kaleido::gfx::{Cc, KQuad, KTex};
use oot_game::kaleido::*;
use oot_game::map::{MAP_48X85_TEX_SIZE, MapSegment};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn frame(w: &mut PlayState, prev: PadState, cur: PadState) -> PadState {
    w.tick_with(scripted_input(prev, cur));
    cur
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        frame(w, PadState::default(), PadState::default());
    }
}

fn press(w: &mut PlayState, button: u16) {
    frame(w, PadState::default(), PadState { button, ..Default::default() });
}

/// A push of the stick for a frame (the repeat filter passes a first push), then a release.
fn nudge(w: &mut PlayState, x: i8, y: i8) {
    frame(w, PadState::default(), PadState { button: 0, stick_x: x, stick_y: y });
    idle(w, 1);
}

fn sfx_on(w: &PlayState, frame: u32, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f == frame && s == id)
}

fn menu_idle(w: &PlayState) -> bool {
    w.pause_ctx.state == PAUSE_STATE_MAIN && w.pause_ctx.main_state == PAUSE_MAIN_STATE_IDLE
}

/// Inside the Deku Tree's room 0 at its spawn (1F) on `save` (the sound log on), the room's
/// enemies gone, Link standing with the main camera and the transition over.
fn deku_tree_with(a: &Arc<GameAssets>, save: SaveContext) -> PlayState {
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 20);
    for h in w.actors.category(ACTORCAT_ENEMY).to_vec() {
        if let Some(a) = w.actors.actor_mut(h) {
            a.kill();
        }
    }
    for _ in 0..600 {
        let p = w.player();
        if w.active_cam_id == CAM_ID_MAIN && p.action == Action::StandingStill && p.grounded() && w.transition.mode == oot_game::transition::TRANS_MODE_OFF && w.letterbox.size == 0 {
            return w;
        }
        idle(&mut w, 1);
    }
    panic!("never settled");
}

fn preset_save(a: &Arc<GameAssets>, preset: &str) -> SaveContext {
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset(preset).unwrap();
    save
}

/// Start, then idle until the menu is open and idle.
fn open(w: &mut PlayState) {
    press(w, BTN_START);
    for _ in 0..100 {
        if menu_idle(w) {
            return;
        }
        idle(w, 1);
    }
    panic!("the menu never opened");
}

/// R from the item page, idle until the map page is turned to (the cursor on its left arrow,
/// `KaleidoScope_SetupPageSwitch`).
fn turn_to_map(w: &mut PlayState) {
    press(w, BTN_R);
    for _ in 0..40 {
        if menu_idle(w) && w.pause_ctx.page_index == PAUSE_MAP {
            break;
        }
        idle(w, 1);
    }
    assert_eq!((w.pause_ctx.page_index, w.pause_ctx.cursor_special_pos), (PAUSE_MAP, PAUSE_CURSOR_PAGE_LEFT));
}

/// `open` and `turn_to_map`, then the stick right onto the floors' column.
fn open_map(w: &mut PlayState) {
    open(w);
    turn_to_map(w);
    nudge(w, 80, 0);
    assert_eq!(w.pause_ctx.cursor_special_pos, 0);
}

const M: usize = PAUSE_MAP as usize;

/// `map_48x85_static`'s map `index`, with `target`'s texels moved to index 14 (written from
/// `KaleidoScope_OverridePalIndexCI4` for the test).
fn room_map(a: &GameAssets, index: usize, target: Option<u8>) -> Vec<u8> {
    let mut t = a.map.map_48x85_static[index * MAP_48X85_TEX_SIZE..(index + 1) * MAP_48X85_TEX_SIZE].to_vec();
    if let Some(target) = target {
        for b in t.iter_mut() {
            let (mut hi, mut lo) = (*b >> 4, *b & 0xF);
            if hi == target {
                hi = 14;
            }
            if lo == target {
                lo = 14;
            }
            *b = (hi << 4) | lo;
        }
    }
    t
}

fn quads_of(w: &PlayState, pred: impl Fn(&KTex) -> bool) -> Vec<KQuad> {
    w.pause_ctx.gfx.quads.iter().filter(|q| pred(&q.tex)).cloned().collect()
}

#[test]
fn the_menus_init_loads_links_floor_with_the_current_room_on_index_14() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_with(&a, preset_save(&a, "deku-tree-compass"));
    // Map_Update: Link above floorCoordY[0][5] (-40): floor 5, R_MAP_TEX_INDEX 0 + 4; room 0's
    // palette 10 (Map_SetPaletteData).
    assert_eq!((w.map.floor, w.map.r_map_tex_index, w.map.map_palette_index), (5, 4, 10));
    open(&mut w);
    // PAUSE_STATE_INIT: cursorPoint, cursorSlot, dungeonMapSlot VREG(30) + 3 = 8; in a dungeon,
    // mapPalette[28], [29] = 6, 99, then KaleidoScope_UpdateDungeonMap: map_48x85_static's maps
    // 4 and 5 into mapSegment (at 0 and ALIGN16(0x7F8) = 0x800), Map_SetFloorPalettesData(5)
    // (cleared, no map: [30] and [31] 0; 1F's room 0 visited: palette 10 = 2, 0xBF), and on Link's
    // floor (VREG(30) == cursorPoint - 3) room 0's palette index 10 moved to 14 in both.
    let p = &w.pause_ctx;
    assert_eq!((p.cursor_point[M], p.cursor_slot[M], p.dungeon_map_slot, p.cursor_x[M]), (8, 8, 8, 0));
    assert_eq!(w.map.map_segment, Some(MapSegment::PauseMap { index: 4 }));
    assert_eq!(&w.map.segment[..MAP_48X85_TEX_SIZE], room_map(&a, 4, Some(10)).as_slice());
    assert_eq!(&w.map.segment[0x800..0x800 + MAP_48X85_TEX_SIZE], room_map(&a, 5, Some(10)).as_slice());
    // 1F's left map is empty; its right one is room 0's.
    assert!(room_map(&a, 4, None).iter().all(|&b| b == 0));
    assert!(room_map(&a, 5, Some(10)) != room_map(&a, 5, None), "1F's right map has room 0's texels");
    // Since then the draws have pulsed [28], [29] (below); the rest is the floor's.
    let pal = w.map.map_palette;
    assert_eq!((pal[20], pal[21]), (2, 0xBF));
    assert!(pal.iter().enumerate().all(|(i, &b)| i == 20 || i == 21 || i == 28 || i == 29 || b == 0), "{pal:?}");
}

#[test]
fn the_current_rooms_colour_steps_between_two_colours_and_the_maps_draw_through_it() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_with(&a, preset_save(&a, "deku-tree-compass"));
    open(&mut w);
    // mapBgPulseColors { 0, 80 / 8, 255 / 8 }, { 0, 200 / 8, 140 / 8 } from (0, 25, 17), timer 20:
    // each draw steps a colour by (current - target) / timer (int division), writes it as RGBA16
    // into mapPalette[28], [29], and every 20 draws swaps the target. Every draw since the
    // opening has run it (the map page is drawn behind the others too), so the test follows the
    // statics: one draw (a frame) from each state, as the C computes it.
    const COLORS: [[i32; 3]; 2] = [[0, 10, 31], [0, 25, 17]];
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..60 {
        let s = w.pause_ctx.statics.clone();
        let (mut c, t) = ([s.map_bg_pulse_r as i32, s.map_bg_pulse_g as i32, s.map_bg_pulse_b as i32], s.map_bg_pulse_timer as i32);
        let target = COLORS[s.map_bg_pulse_stage as usize];
        for k in 0..3 {
            c[k] -= (c[k] - target[k]) / t;
        }
        let rgba16 = (((c[0] & 0x1F) << 11) | ((c[1] & 0x1F) << 6) | ((c[2] & 0x1F) << 1) | 1) as u16;
        idle(&mut w, 1);
        let s2 = &w.pause_ctx.statics;
        assert_eq!([s2.map_bg_pulse_r as i32, s2.map_bg_pulse_g as i32, s2.map_bg_pulse_b as i32], c);
        assert_eq!((w.map.map_palette[28], w.map.map_palette[29]), ((rgba16 >> 8) as u8, rgba16 as u8));
        let (timer, stage) = if t == 1 { (20, s.map_bg_pulse_stage ^ 1) } else { (t - 1, s.map_bg_pulse_stage) };
        assert_eq!((s2.map_bg_pulse_timer as i32, s2.map_bg_pulse_stage), (timer, stage));
        seen.insert(rgba16);
    }
    // It reaches both colours, each held as its stage ends.
    assert!(seen.contains(&((10 << 6) | (31 << 1) | 1)) && seen.contains(&((25 << 6) | (17 << 1) | 1)), "{seen:x?}");
    // On the map page, the two room maps are mapSegment's texels through mapPalette as
    // gDPLoadTLUT_pal16 loads it (RGBA16, the importer's decode): room 0 (index 14 on Link's
    // floor) in the pulse's colour, the unvisited rooms' indices (entries 0) transparent.
    turn_to_map(&mut w);
    let rooms = quads_of(&w, |t| *t == KTex::RoomMap);
    assert_eq!(rooms.len(), 2);
    let pal = w.map.map_palette;
    let colour = |i: usize| {
        let v = u16::from_be_bytes([pal[i * 2], pal[i * 2 + 1]]);
        let c5 = |x: u16| ((x as u32 * 255 + 15) / 31) as u8;
        [c5((v >> 11) & 0x1F), c5((v >> 6) & 0x1F), c5((v >> 1) & 0x1F), if v & 1 != 0 { 255 } else { 0 }]
    };
    for (q, texels) in rooms.iter().zip([room_map(&a, 4, Some(10)), room_map(&a, 5, Some(10))]) {
        let img = q.image.as_ref().expect("the room map's own texels");
        assert_eq!((img.width, img.height), (48, 85));
        for (px, rgba) in img.rgba.chunks_exact(4).enumerate() {
            let b = texels[px / 2];
            let index = if px % 2 == 0 { b >> 4 } else { b & 0xF };
            assert_eq!(rgba, colour(index as usize), "texel {px}");
        }
        assert_eq!((q.cc, q.prim), (Cc::ModulateIaPrim, [255, 255, 255, 255]));
    }
}

#[test]
fn the_floors_column_moves_over_the_visited_floors_and_reloads_the_maps() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_with(&a, preset_save(&a, "deku-tree-compass"));
    open_map(&mut w);
    // From the left arrow, the stick right: cursorSlot = cursorPoint = dungeonMapSlot (8),
    // cursorX 0; no item (cursorItem PAUSE_ITEM_NONE on a floor).
    let p = &w.pause_ctx;
    assert_eq!((p.cursor_point[M], p.cursor_slot[M], p.cursor_x[M], p.cursor_item[M]), (8, 8, 0, PAUSE_ITEM_NONE));
    // The stick up: from slot 8 the floors above (4, then 3) the first visited (floors bit 4):
    // slot 7 (2F), NA_SE_SY_CURSOR; R_MAP_TEX_INDEX 0 + sFloorTexIndexOffset[0][4] = 2, changed:
    // maps 2 and 3 loaded, 2F's palettes (rooms 0, 1, 2: palettes 10, 1, 2), no recolouring (not
    // Link's floor).
    let f = w.audio.frames + 1;
    nudge(&mut w, 0, 80);
    let p = &w.pause_ctx;
    assert_eq!((p.cursor_point[M], p.dungeon_map_slot, w.map.r_map_tex_index), (7, 7, 2));
    assert!(sfx_on(&w, f, NA_SE_SY_CURSOR));
    assert_eq!(w.map.map_segment, Some(MapSegment::PauseMap { index: 2 }));
    assert_eq!(&w.map.segment[..MAP_48X85_TEX_SIZE], room_map(&a, 2, None).as_slice());
    assert_eq!(&w.map.segment[0x800..0x800 + MAP_48X85_TEX_SIZE], room_map(&a, 3, None).as_slice());
    let pal = w.map.map_palette;
    for i in [10, 1, 2] {
        assert_eq!((pal[i * 2], pal[i * 2 + 1]), (2, 0xBF), "palette {i}");
    }
    // Up: 3F (slot 6, maps 0 and 1); up again: nothing visited above (floors 2 to 0), it stays.
    nudge(&mut w, 0, 80);
    assert_eq!((w.pause_ctx.cursor_point[M], w.map.r_map_tex_index), (6, 0));
    nudge(&mut w, 0, 80);
    assert_eq!(w.pause_ctx.cursor_point[M], 6);
    // Down twice: 2F, 1F (Link's floor: room 0 on index 14 again); down again: B1 and B2 aren't
    // visited (nor floors 8 to 10, the loop's @bug), no map: it stays.
    nudge(&mut w, 0, -80);
    nudge(&mut w, 0, -80);
    assert_eq!((w.pause_ctx.cursor_point[M], w.map.r_map_tex_index), (8, 4));
    assert_eq!(&w.map.segment[..MAP_48X85_TEX_SIZE], room_map(&a, 4, Some(10)).as_slice());
    nudge(&mut w, 0, -80);
    assert_eq!(w.pause_ctx.cursor_point[M], 8);
    // The buttons: 3F, 2F in (255, 255, 200), the viewed 1F in (150, 150, 255), its quad 2 bigger
    // left and up, 4 right and down (the cursor's: cursorX 0). The floors' quads are
    // sVtxPageMapDungeonQuads 6.. (x -88, 24 wide; y 50, 36, 22, .. 16 high): 1F's (floor 5) at
    // y -20.
    let buttons = quads_of(&w, |t| matches!(t, KTex::DungeonMap(s) if s.ends_with("ButtonTex")));
    let alpha = w.pause_ctx.alpha as u8;
    assert_eq!(
        buttons.iter().map(|q| (q.tex, q.prim)).collect::<Vec<_>>(),
        vec![
            (KTex::DungeonMap("gDungeonMap3FButtonTex"), [255, 255, 200, alpha]),
            (KTex::DungeonMap("gDungeonMap2FButtonTex"), [255, 255, 200, alpha]),
            (KTex::DungeonMap("gDungeonMap1FButtonTex"), [150, 150, 255, alpha])
        ]
    );
    let v = buttons[2].resolve(&w.pause_ctx.cursor_vtx);
    assert_eq!((v[0].ob[0], v[1].ob[0], v[0].ob[1], v[2].ob[1]), (-88 - 2, -88 + 24 + 4, -20 + 2, -20 - 16 - 4));
}

#[test]
fn the_items_column_and_the_arrows() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_with(&a, preset_save(&a, "deku-tree-compass"));
    open_map(&mut w);
    // The stick right from the floors: cursorX 1, cursorPoint 0, the boss key not owned: 1, the
    // compass owned: it stays; cursorItem ITEM_DUNGEON_BOSS_KEY + 1 (the compass, 0x75).
    nudge(&mut w, 80, 0);
    let p = &w.pause_ctx;
    assert_eq!((p.cursor_x[M], p.cursor_point[M], p.cursor_slot[M], p.cursor_item[M]), (1, 1, 1, 0x75));
    // Down: no map (DUNGEON_MAP), it stays; up: no boss key, it stays.
    nudge(&mut w, 0, -80);
    nudge(&mut w, 0, 80);
    assert_eq!(w.pause_ctx.cursor_point[M], 1);
    // Right again: cursorX isn't 0, onto the right arrow (NA_SE_SY_DECIDE).
    let f = w.audio.frames + 1;
    nudge(&mut w, 80, 0);
    assert_eq!(w.pause_ctx.cursor_special_pos, PAUSE_CURSOR_PAGE_RIGHT);
    assert!(sfx_on(&w, f, NA_SE_SY_DECIDE));
    // From the right arrow, left: the items' column, the compass.
    nudge(&mut w, -80, 0);
    let p = &w.pause_ctx;
    assert_eq!((p.cursor_special_pos, p.cursor_x[M], p.cursor_point[M], p.cursor_slot[M]), (0, 1, 1, 1));
    // Left: back to the floors, on dungeonMapSlot (8).
    nudge(&mut w, -80, 0);
    let p = &w.pause_ctx;
    assert_eq!((p.cursor_x[M], p.cursor_point[M], p.cursor_item[M]), (0, 8, PAUSE_ITEM_NONE));
    // Left again: the left arrow.
    nudge(&mut w, -80, 0);
    assert_eq!(w.pause_ctx.cursor_special_pos, PAUSE_CURSOR_PAGE_LEFT);
}

#[test]
fn the_title_the_items_the_head_the_skull_and_the_gold_skulltula() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_with(&a, preset_save(&a, "deku-tree-compass"));
    open_map(&mut w);
    let q = |w: &PlayState, t: KTex| quads_of(w, |x| *x == t);
    // dungeonTitleTexs[0] under G_CC_MODULATEIA; the compass alone of the dungeon items.
    assert_eq!(q(&w, KTex::Label("gPauseDekuTitleENGTex", 96))[0].cc, Cc::ModulateIa);
    assert_eq!(quads_of(&w, |t| matches!(t, KTex::Icon24(_))).iter().map(|q| q.tex).collect::<Vec<_>>(), vec![KTex::Icon24("gQuestIconDungeonCompassTex")]);
    // Link's head at pagesYOrigin1 (0) + 50 - VREG(30) * 14 - 1 = -21, 16 high; the skull (the
    // compass, sSkullFloorIconY[0] -47) at -47; no Gold Skulltula icon (GET_GS_FLAGS(0) 0, not
    // gAreaGsFlags[0] 0x0F).
    let head = q(&w, KTex::DungeonMap("gDungeonMapLinkHeadTex"))[0].resolve(&w.pause_ctx.cursor_vtx);
    assert_eq!((head[0].ob[0], head[0].ob[1], head[2].ob[1]), (-106, -21, -37));
    let skull = q(&w, KTex::DungeonMap("gDungeonMapSkullTex"))[0].resolve(&w.pause_ctx.cursor_vtx);
    assert_eq!((skull[0].ob[0], skull[0].ob[1], skull[2].ob[1]), (-62, -47, -63));
    assert!(q(&w, KTex::Icon24("gQuestIconGoldSkulltulaTex")).is_empty());
    // Without the compass: no skull, no marks.
    let mut save = preset_save(&a, "deku-tree-compass");
    save.inventory.dungeon_items[0] = 0;
    let mut w = deku_tree_with(&a, save);
    open_map(&mut w);
    assert!(q(&w, KTex::DungeonMap("gDungeonMapSkullTex")).is_empty());
    assert!(quads_of(&w, |t| matches!(t, KTex::MapMark(_))).is_empty());
    // All the Deku Tree's Gold Skulltulas (gsFlags 0x0F): the icon, 19 square 2 in from its
    // quad's corner (sVtxPageMapDungeonQuads 16: x -40, y 50).
    let mut save = preset_save(&a, "deku-tree-compass");
    save.set_gs_flags(0, 0x0F);
    let mut w = deku_tree_with(&a, save);
    open_map(&mut w);
    let gs = q(&w, KTex::Icon24("gQuestIconGoldSkulltulaTex"))[0].resolve(&w.pause_ctx.cursor_vtx);
    assert_eq!((gs[0].ob[0], gs[1].ob[0], gs[0].ob[1], gs[2].ob[1]), (-38, -19, 48, 29));
}

#[test]
fn the_compass_marks_the_viewed_floors_chests_until_opened() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_with(&a, preset_save(&a, "deku-tree-compass"));
    open_map(&mut w);
    // PauseMapMark_Draw: gPauseMapMarkDataTable[R_MAP_TEX_INDEX >> 1]: 1F's (map 2) chest 3 at
    // (84, -39), the list moved to (-36, 21) on the page's matrix (the room maps' quads'), scale
    // 1; sMarkChestVtx's quad (1, 3, 2, 0); on the page looked at under the cursor's combiner,
    // prim (255, 255, 255, 255), env (0, 0, 0, 255).
    let marks_at = |w: &PlayState| -> Vec<(KTex, Cc, Vec3)> {
        let page = quads_of(w, |t| *t == KTex::RoomMap)[0].mtx;
        quads_of(w, |t| matches!(t, KTex::MapMark(_))).iter().map(|q| (q.tex, q.cc, (page.inverse() * q.mtx).transform_point3(Vec3::ZERO))).collect()
    };
    let close = |a: Vec3, b: Vec3| (a - b).length() < 1e-3;
    let m = marks_at(&w);
    assert_eq!(m.len(), 1);
    assert_eq!((m[0].0, m[0].1), (KTex::MapMark(0), Cc::PrimEnvTexel));
    assert!(close(m[0].2, Vec3::new(-36.0 + 84.0, 21.0 - 39.0, 0.0)), "{:?}", m[0].2);
    let q = quads_of(&w, |t| matches!(t, KTex::MapMark(_)))[0].clone();
    assert_eq!((q.prim, q.env), ([255, 255, 255, 255], [0, 0, 0, 255]));
    let v = q.resolve(&w.pause_ctx.cursor_vtx);
    assert_eq!(v.map(|v| v.ob), [[-4, 4, 0], [4, 4, 0], [-4, -4, 0], [4, -4, 0]]);
    // 2F (map 1): chest 1 at (48, -63).
    nudge(&mut w, 0, 80);
    let m = marks_at(&w);
    assert!(m.len() == 1 && close(m[0].2, Vec3::new(12.0, -42.0, 0.0)), "{m:?}");
    // 3F (map 0): chests 2 at (40, -33) and 6 at (49, -42); with chest 6 opened, only 2's.
    nudge(&mut w, 0, 80);
    let m = marks_at(&w);
    assert!(m.len() == 2 && close(m[0].2, Vec3::new(4.0, -12.0, 0.0)) && close(m[1].2, Vec3::new(13.0, -21.0, 0.0)), "{m:?}");
    w.flags.set_treasure(6);
    idle(&mut w, 1);
    assert_eq!(marks_at(&w).len(), 1);
    // On the arrow (no cursor drawn), G_CC_MODULATEIA_PRIM: Gfx_SetupDL_42Opa's combiner.
    nudge(&mut w, -80, 0);
    assert_eq!(w.pause_ctx.cursor_special_pos, PAUSE_CURSOR_PAGE_LEFT);
    assert_eq!(marks_at(&w)[0].1, Cc::ModulateIaPrim);
    // The boss mark's pulse runs only in the boss scenes (where PauseMapMark_Draw draws
    // nothing): PauseMapMark_Init leaves its scale 1 each draw.
    assert_eq!(w.pause_ctx.boss_mark_scale, 1.0);
}

#[test]
fn every_quad_of_the_map_page_is_baked_and_covers_its_whole_texture() {
    let Some(a) = assets() else { return };
    let pack = oot_game::pack::GamePack::open_default().unwrap();
    // A session over the floors and the items, with and without the cursor on the page.
    let mut w = deku_tree_with(&a, preset_save(&a, "deku-tree-compass"));
    open_map(&mut w);
    let mut quads = Vec::new();
    for (x, y) in [(0, 80), (0, 80), (0, -80), (80, 0), (80, 0), (-80, 0), (-80, 0), (-80, 0)] {
        quads.extend(w.pause_ctx.gfx.quads.iter().map(|q| (q.clone(), w.pause_ctx.cursor_vtx.clone())));
        nudge(&mut w, x, y);
    }
    let baked = oot_game::kaleido::gfx::bake_list();
    for (q, cursor) in &quads {
        assert!(baked.contains(&(q.tex, q.cc)), "{:?} under {:?} isn't baked", q.tex, q.cc);
        let key = oot_game::pack::keys::bake(&oot_game::kaleido::gfx::bake_name(q.tex, q.cc));
        assert!(pack.assets.try_get::<eng_gfx::DrawList>(&key).ok().flatten().is_some(), "{key} isn't in the pack");
        let v = q.resolve(cursor);
        let (w, h) = q.tex.size();
        assert_eq!((v[1].tc[0] as u32, v[2].tc[1] as u32), (w * 32, h * 32), "{:?}", q.tex);
    }
}

#[test]
fn exit_the_dungeon_map_run() {
    use oot_actors::playthrough::{Playthrough, Route, Step};
    let Some(a) = assets() else { return };
    let route = Route::DungeonMap;
    let e = a.scenes.entrance_index(route.entrance()).unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), route.save(e)).expect("Play_Init");
    route.debug_start(&mut w);
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    for _ in 0..route.max_frames() {
        let Some(pad) = run.next(&w) else { break };
        prev = frame(&mut w, prev, pad);
    }
    assert!(run.failure.is_none() && run.finished(), "{:?} at {}", run.failure, run.at());
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::MenuOpened, Step::PageTurned, Step::FloorChanged, Step::MenuClosed]);
    // The floor changed to 2F; closed, the game resumed: Map_InitData's minimap back in
    // mapSegment (room 0's), 20 frames a second.
    assert_eq!(w.pause_ctx.dungeon_map_slot, 7);
    assert_eq!(w.map.map_segment, Some(MapSegment::Dungeon { index: 0 }));
    assert_eq!((w.pause_ctx.state, w.r_update_rate), (PAUSE_STATE_OFF, 3));
}
