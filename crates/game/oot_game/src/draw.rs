//! `GetItem_Draw` (`z_draw.c`): the get-item models, drawn over Link's head when he holds an
//! item up (`Player_DrawGetItem`), and by `En_Item00` for the placed recovery hearts, the
//! shields and the tunics.
//!
//! `sDrawItemTable` is data from the C (`table/items`: each draw id's function name and display
//! lists). Each `GetItem_Draw*` function is ported as its *pieces*: the display lists it draws
//! under one matrix into one buffer, after one setup list, with the scroll it binds to segment
//! 8 (`Gfx_TwoTexScroll`) and whether the matrix's rotation is the billboard
//! (`Matrix_ReplaceRotation`). The importer bakes every piece of every draw id
//! (docs/adr/0012-actor-bakes.md); the scroll is a dynamic segment, baked at frame 0 and given
//! each frame's tile sizes when drawn (docs/adr/0019-inventory-and-saves.md).
//!
//! The scrolls count `play->state.frames`, the game state's frame counter; here that's
//! `gameplayFrames`, which counts the same game frames outside the pause menu.

use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use glam::{Mat4, Vec3, Vec4};

use crate::gbi::setup_dl;
use crate::item::{DrawItemEntry, ItemTables};
use crate::pack::{BakeBody, BakeSegment, MeshBake, keys};
use crate::play::{DrawOut, ViewInfo};
use crate::scene_table::gfx_two_tex_scroll;

/// The segment `Gfx_TwoTexScroll` is bound to, and the one a bake's setup list is on.
const SEG_SCROLL: u8 = 0x08;
const SEG_SETUP: u8 = 0x0D;

/// Which `sSetupDL` entry a piece is drawn after.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setup {
    /// `Gfx_SetupDL_25Opa` / `_25Xlu`.
    Dl25,
    /// `Gfx_SetupDL_26Opa`.
    Dl26,
    /// `Gfx_SetupDL(POLY_XLU_DISP, SETUPDL_5)`.
    Dl5,
}

/// A `Gfx_TwoTexScroll(gfxCtx, G_TX_RENDERTILE, x1, y1, w1, h1, 1, x2, y2, w2, h2)` as the
/// draw functions write it, each coordinate `a * frames` (the C's `n * (frames * m)` and
/// `n * -(frames * m)`, in u32), optionally `% modulo` first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scroll {
    pub x1: i32,
    pub y1: i32,
    pub w1: i32,
    pub h1: i32,
    pub x2: i32,
    pub y2: i32,
    pub w2: i32,
    pub h2: i32,
    /// `% 256` on tile 0's and `% 128` on tile 1's coordinates (`GetItem_DrawMirrorShield`).
    pub mirror_modulo: bool,
}

impl Scroll {
    const fn new(x1: i32, y1: i32, w1: i32, h1: i32, x2: i32, y2: i32, w2: i32, h2: i32) -> Scroll {
        Scroll { x1, y1, w1, h1, x2, y2, w2, h2, mirror_modulo: false }
    }

    /// The list `Gfx_TwoTexScroll` builds at `frames`.
    pub fn dl(&self, frames: u32) -> Vec<(u32, u32)> {
        let at = |a: i32, m: u32| {
            let v = (a as u32).wrapping_mul(frames);
            if self.mirror_modulo { v % m } else { v }
        };
        gfx_two_tex_scroll(0, at(self.x1, 256), at(self.y1, 256), self.w1, self.h1, 1, at(self.x2, 128), at(self.y2, 128), self.w2, self.h2)
    }
}

/// One `gSPMatrix` + display lists of a draw function, into one buffer.
#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    pub xlu: bool,
    pub setup: Setup,
    /// Indices into the entry's `dlists`, in the order drawn.
    pub lists: Vec<usize>,
    /// `gSPSegment(0x08, Gfx_TwoTexScroll(..))` for this buffer before the lists.
    pub scroll: Option<Scroll>,
    /// `Matrix_Push`, `Matrix_Translate(t)`, `Matrix_ReplaceRotation(&play->billboardMtxF)`.
    pub billboard: Option<Vec3>,
    /// `Matrix_Scale(s, s, s)` before the matrix (`GetItem_DrawSmallRupee`'s 0.7).
    pub scale: Option<f32>,
}

fn piece(xlu: bool, setup: Setup, lists: &[usize]) -> Piece {
    Piece { xlu, setup, lists: lists.to_vec(), scroll: None, billboard: None, scale: None }
}

impl Piece {
    fn scroll(mut self, s: Scroll) -> Piece {
        self.scroll = Some(s);
        self
    }
    fn billboard(mut self, t: Vec3) -> Piece {
        self.billboard = Some(t);
        self
    }
}

/// A draw function's pieces (`GetItem_Draw*` in `z_draw.c`), by its name.
///
/// Where a function binds segment 8 in one buffer and a list in the other buffer reads it, the
/// game reads whatever that buffer last bound; no such list reads segment 8 here (the bakes
/// check it: a list reading an unbound segment fails the import).
pub fn pieces(func: &str) -> Option<Vec<Piece>> {
    use Setup::*;
    let p = match func {
        "GetItem_DrawMaskOrBombchu" => vec![piece(false, Dl26, &[0])],
        "GetItem_DrawSoldOut" => vec![piece(true, Dl5, &[0])],
        // The flame: Matrix_Translate(-8, -2, 0), then the billboard.
        "GetItem_DrawBlueFire" => vec![piece(false, Dl25, &[0]), piece(true, Dl25, &[1]).scroll(Scroll::new(0, 0, 16, 32, 1, -8, 16, 32)).billboard(Vec3::new(-8.0, -2.0, 0.0))],
        // dlists[1] is drawn before the scroll is bound; the billboarded contents after.
        "GetItem_DrawPoes" => vec![piece(false, Dl25, &[0]), piece(true, Dl25, &[1]), piece(true, Dl25, &[3, 2]).scroll(Scroll::new(0, 0, 16, 32, 1, -6, 16, 32)).billboard(Vec3::ZERO)],
        "GetItem_DrawFairy" => vec![piece(false, Dl25, &[0]), piece(true, Dl25, &[1]), piece(true, Dl25, &[2]).scroll(Scroll::new(0, 0, 32, 32, 1, -6, 32, 32)).billboard(Vec3::ZERO)],
        "GetItem_DrawMirrorShield" => {
            let s = Scroll { mirror_modulo: true, ..Scroll::new(0, 2, 64, 64, 0, 1, 32, 32) };
            vec![piece(false, Dl25, &[0]).scroll(s), piece(true, Dl25, &[1])]
        }
        "GetItem_DrawSkullToken" => vec![piece(false, Dl25, &[0]), piece(true, Dl25, &[1]).scroll(Scroll::new(0, -5, 32, 32, 0, 0, 32, 64))],
        "GetItem_DrawEggOrMedallion" => vec![piece(false, Dl26, &[0, 1])],
        "GetItem_DrawCompass" => vec![piece(false, Dl25, &[0]), piece(true, Dl5, &[1])],
        "GetItem_DrawPotion" => vec![piece(false, Dl25, &[1, 0, 2, 3]).scroll(Scroll::new(-1, 1, 32, 32, -1, 1, 32, 32)), piece(true, Dl25, &[4, 5])],
        "GetItem_DrawGoronSword" => vec![piece(false, Dl25, &[0]).scroll(Scroll::new(1, 0, 32, 32, 0, 0, 32, 32))],
        "GetItem_DrawDekuNuts" => vec![piece(false, Dl25, &[0]).scroll(Scroll::new(6, 6, 32, 32, 6, 6, 32, 32))],
        "GetItem_DrawRecoveryHeart" => vec![piece(true, Dl25, &[0]).scroll(Scroll::new(0, -3, 32, 32, 0, -2, 32, 32))],
        "GetItem_DrawFish" => vec![piece(true, Dl25, &[0]).scroll(Scroll::new(0, 1, 32, 32, 0, 1, 32, 32))],
        "GetItem_DrawOpa0" => vec![piece(false, Dl25, &[0])],
        "GetItem_DrawOpa0Xlu1" => vec![piece(false, Dl25, &[0]), piece(true, Dl25, &[1])],
        "GetItem_DrawXlu01" => vec![piece(true, Dl25, &[0, 1])],
        "GetItem_DrawOpa10Xlu2" => vec![piece(false, Dl25, &[1, 0]), piece(true, Dl25, &[2])],
        "GetItem_DrawMagicArrow" => vec![piece(false, Dl25, &[0]), piece(true, Dl25, &[1, 2])],
        "GetItem_DrawMagicSpell" => vec![piece(true, Dl25, &[0, 1, 2]).scroll(Scroll::new(2, -6, 32, 32, 1, -2, 32, 32))],
        "GetItem_DrawOpa1023" => vec![piece(false, Dl25, &[1, 0, 2, 3])],
        "GetItem_DrawOpa10Xlu32" => vec![piece(false, Dl25, &[1, 0]), piece(true, Dl25, &[3, 2])],
        "GetItem_DrawSmallRupee" => {
            let s = |p: Piece| Piece { scale: Some(0.7), ..p };
            vec![s(piece(false, Dl25, &[1, 0])), s(piece(true, Dl25, &[3, 2]))]
        }
        "GetItem_DrawScale" => vec![piece(true, Dl25, &[2, 3, 1, 0]).scroll(Scroll::new(2, -2, 64, 64, 4, -4, 32, 32))],
        "GetItem_DrawBulletBag" => vec![piece(false, Dl25, &[1, 0]), piece(true, Dl25, &[2, 3, 4])],
        "GetItem_DrawWallet" => vec![piece(false, Dl25, &[1, 0, 2, 3, 4, 5, 6, 7])],
        _ => return None,
    };
    Some(p)
}

/// A draw id's piece `k`'s bake.
pub fn bake_name(draw_id: usize, k: usize) -> String {
    format!("GetItem/{draw_id:02X}/{k}")
}

/// Every draw id's pieces as bakes. `file_of` finds the object file a display list symbol is
/// in (the importer's symbol index); that file is on segment 6.
pub fn bakes(table: &ItemTables, file_of: &dyn Fn(&str) -> Option<String>) -> anyhow::Result<Vec<MeshBake>> {
    let mut out = Vec::new();
    for (id, e) in table.draw_items.iter().enumerate() {
        let ps = pieces(&e.func).ok_or_else(|| anyhow::anyhow!("sDrawItemTable[{id:#x}]: {} isn't ported", e.func))?;
        for (k, p) in ps.iter().enumerate() {
            out.push(piece_bake(id, k, e, p, file_of)?);
        }
    }
    Ok(out)
}

fn piece_bake(id: usize, k: usize, e: &DrawItemEntry, p: &Piece, file_of: &dyn Fn(&str) -> Option<String>) -> anyhow::Result<MeshBake> {
    let mut lists = Vec::new();
    for &i in &p.lists {
        let sym = e.dlists.get(i).ok_or_else(|| anyhow::anyhow!("sDrawItemTable[{id:#x}] has no list {i}"))?;
        let file = file_of(sym).ok_or_else(|| anyhow::anyhow!("{sym}: in no object's XML"))?;
        lists.push((file, sym.clone()));
    }
    let object = lists[0].0.clone();
    let mut segments = Vec::new();
    let mut prelude = Vec::new();
    // Gfx_SetupDL_25* is the bake's own start; the others are written out after it.
    match p.setup {
        Setup::Dl25 => {}
        Setup::Dl26 | Setup::Dl5 => {
            let mut d = if p.setup == Setup::Dl26 { setup_dl::setup_dl_26() } else { setup_dl::setup_dl_5() };
            d.end();
            segments.push((SEG_SETUP, BakeSegment::Commands(d.0)));
            prelude.push(SEG_SETUP);
        }
    }
    // The scroll at frame 0, marked dynamic; without one, segment 8 is an empty list.
    match p.scroll {
        Some(s) => segments.push((SEG_SCROLL, BakeSegment::Dynamic(s.dl(0)))),
        None => segments.push((SEG_SCROLL, BakeSegment::Commands(vec![(0xDF00_0000, 0)]))),
    }
    Ok(MeshBake { name: bake_name(id, k), object, segments, prelude, body: BakeBody::DLists(lists) })
}

/// `Matrix_ReplaceRotation(mf)`: `m`'s rotation columns become `mf`'s, each scaled by the
/// length of `m`'s column it replaces; the translation stays.
pub fn replace_rotation(m: Mat4, mf: Mat4) -> Mat4 {
    let norm = |c: Vec4| c.truncate().length();
    Mat4::from_cols(mf.x_axis * norm(m.x_axis), mf.y_axis * norm(m.y_axis), mf.z_axis * norm(m.z_axis), m.w_axis)
}

/// `GetItem_Draw(play, drawId)` under the current matrix `m`: each piece's bake into its
/// buffer, with this frame's scroll.
pub fn get_item_draw(table: &ItemTables, draw_id: i16, m: Mat4, frames: u32, view: &ViewInfo, out: &mut DrawOut) {
    let Some(e) = usize::try_from(draw_id).ok().and_then(|i| table.draw_items.get(i)) else { return };
    let Some(ps) = pieces(&e.func) else { return };
    for (k, p) in ps.iter().enumerate() {
        let mut t = m;
        if let Some(s) = p.scale {
            t *= Mat4::from_scale(Vec3::splat(s));
        }
        if let Some(tr) = p.billboard {
            t = replace_rotation(t * Mat4::from_translation(tr), view.billboard);
        }
        let segments = p.scroll.map(|s| {
            let mut sv = SegmentValues::default();
            sv.read(SEG_SCROLL, &s.dl(frames));
            sv
        });
        let cmd = DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(draw_id as usize, k))), transform: t, bones: Vec::new(), params: DrawParams { segments, ..Default::default() } };
        if p.xlu {
            out.xlu.push(cmd);
        } else {
            out.opa.push(cmd);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_recovery_hearts_scroll() {
        // GetItem_DrawRecoveryHeart: tile 0 (0, -(frames * 3)), tile 1 (0, -(frames * 2)), 32x32,
        // in u32, % 2048 (Gfx_TwoTexScroll).
        let s = pieces("GetItem_DrawRecoveryHeart").unwrap()[0].scroll.unwrap();
        let mut sv = SegmentValues::default();
        sv.read(8, &s.dl(10));
        assert_eq!(sv.tiles[8][0], Some((0, ((0u32.wrapping_sub(30)) % 2048) as u16 & 0xFFF)));
        assert_eq!(sv.tiles[8][1], Some((0, ((0u32.wrapping_sub(20)) % 2048) as u16)));
    }

    #[test]
    fn replace_rotation_keeps_scale_and_place() {
        let m = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0)) * Mat4::from_rotation_y(0.7) * Mat4::from_scale(Vec3::splat(0.2));
        let bb = Mat4::from_rotation_x(0.3);
        let r = replace_rotation(m, bb);
        assert!((r.x_axis.truncate() - bb.x_axis.truncate() * 0.2).length() < 1e-5);
        assert_eq!(r.w_axis, m.w_axis);
    }
}
