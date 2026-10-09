//! The map's and the compass's tables (`oot_game::map`, `table/map`), read from the C:
//! `gMapDataTable` (`src/code/z_map_data.c`), `gMapMarkDataTable` (the file `spec` builds
//! `ovl_map_mark_data` from for this version: `z_map_mark_data_mq.c`) and the pause map's
//! `gPauseMapMarkDataTable` (the one `ovl_kaleido_scope` includes: `z_lmap_mark_data_mq.c`); and
//! `map_48x85_static` from the ROM as it is (the pause map's room maps, which the menu copies and
//! recolours at run time). The importer's tests check the tables against the ROM's bytes.
//!
//! Each array is read at the size it's declared with and filled as C fills an initializer: rows
//! and elements past the ones written are zero.

use std::path::Path;

use anyhow::{Context, Result};
use oot_game::kaleido::gfx::Vtx;
use oot_game::map::{MAP_COMPASS_ROOMS, MAP_FLOOR_PALETTES, MAP_FLOORS, MAP_ROOM_PALETTES, MAP_SWITCHES, MapData, MapMarkIconData, MapMarkPoint, MapTables, PauseMapMarkData, PauseMapMarkPoint};

use crate::csrc::{Init, Macros, find_initializer, read_c};

/// `[N][M]...`: the dimensions `name` is declared with in `src`.
fn declared_dims(src: &str, name: &str) -> Result<Vec<usize>> {
    let b = src.as_bytes();
    let ident = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let mut from = 0;
    while let Some(rel) = src[from..].find(name) {
        let at = from + rel;
        from = at + name.len();
        if (at > 0 && ident(b[at - 1])) || b.get(from) != Some(&b'[') {
            continue;
        }
        let mut dims = Vec::new();
        let mut i = from;
        while b.get(i) == Some(&b'[') {
            let close = src[i..].find(']').context("an unclosed [")? + i;
            dims.push(src[i + 1..close].trim().parse::<usize>().with_context(|| format!("{name}'s dimension {}", &src[i + 1..close]))?);
            i = close + 1;
        }
        return Ok(dims);
    }
    anyhow::bail!("no declaration of {name}[...]")
}

/// `init` as an array of `dims`, flattened in row order, zero-filled (`"0"`).
fn fill(init: &Init, dims: &[usize], out: &mut Vec<String>) -> Result<()> {
    let Some((&n, rest)) = dims.split_first() else {
        let atom = match init {
            Init::Atom(a) => a.clone(),
            // A scalar in braces.
            Init::List(l) => l.first().and_then(|i| i.atom()).unwrap_or("0").to_string(),
        };
        out.push(atom);
        return Ok(());
    };
    let items = init.list();
    anyhow::ensure!(items.len() <= n, "{} initializers for {n} elements", items.len());
    for item in items {
        fill(item, rest, out)?;
    }
    let row: usize = rest.iter().product();
    out.extend(std::iter::repeat_n("0".to_string(), (n - items.len()) * row));
    Ok(())
}

/// `name`'s values (integers, through `m`), flattened and zero-filled; `dims` must be what it's
/// declared with.
fn ints(src: &str, m: &Macros, name: &str, dims: &[usize]) -> Result<Vec<i64>> {
    let declared = declared_dims(src, name)?;
    anyhow::ensure!(declared == dims, "{name} is declared {declared:?}, not {dims:?}");
    let mut atoms = Vec::new();
    fill(&find_initializer(src, name)?, dims, &mut atoms).with_context(|| name.to_string())?;
    atoms.iter().map(|a| m.eval(a).with_context(|| format!("{name}: {a}"))).collect()
}

fn i16s(src: &str, m: &Macros, name: &str, dims: &[usize]) -> Result<Vec<i16>> {
    Ok(ints(src, m, name, dims)?.into_iter().map(|v| v as i16).collect())
}

fn u16s(src: &str, m: &Macros, name: &str, dims: &[usize]) -> Result<Vec<u16>> {
    Ok(ints(src, m, name, dims)?.into_iter().map(|v| v as u16).collect())
}

fn u8s(src: &str, m: &Macros, name: &str, dims: &[usize]) -> Result<Vec<u8>> {
    Ok(ints(src, m, name, dims)?.into_iter().map(|v| v as u8).collect())
}

/// `z_map_data.c`'s arrays, the ones `gMapDataTable` points at in `MapData`'s order.
pub fn load_map_data(decomp: &Path) -> Result<MapData> {
    let src = read_c(decomp, "src/code/z_map_data.c")?;
    // F_* (map.h), INFTABLE_*_SHIFT (save.h).
    let m = Macros::read(decomp, &["include/map.h", "include/save.h"])?;
    let (f, rp, fp, cr, sw) = (MAP_FLOORS, MAP_ROOM_PALETTES, MAP_FLOOR_PALETTES, MAP_COMPASS_ROOMS, MAP_SWITCHES);
    let floor_coord_y = {
        let name = "sFloorCoordY";
        anyhow::ensure!(declared_dims(&src, name)? == [10, f], "{name}'s size");
        let mut atoms = Vec::new();
        fill(&find_initializer(&src, name)?, &[10, f], &mut atoms)?;
        atoms.iter().map(|a| crate::csrc::eval_expr(a).with_context(|| format!("{name}: {a}"))).collect::<Result<Vec<f32>>>()?
    };
    Ok(MapData {
        floor_tex_index_offset: i16s(&src, &m, "sFloorTexIndexOffset", &[10, f])?,
        boss_floor: i16s(&src, &m, "sBossFloor", &[8])?,
        room_palette: i16s(&src, &m, "sRoomPalette", &[10, rp])?,
        max_palette_count: i16s(&src, &m, "sMaxPaletteCount", &[10])?,
        palette_room: i16s(&src, &m, "sPaletteRoom", &[10, f, fp])?,
        room_compass_offset_x: i16s(&src, &m, "sRoomCompassOffsetX", &[10, cr])?,
        room_compass_offset_y: i16s(&src, &m, "sRoomCompassOffsetY", &[10, cr])?,
        dgn_minimap_count: u8s(&src, &m, "sDgnMinimapCount", &[12])?,
        dgn_minimap_tex_index_offset: u16s(&src, &m, "sDgnMinimapTexIndexOffset", &[10])?,
        ow_minimap_tex_size: u16s(&src, &m, "sOwMinimapTexSize", &[24])?,
        ow_minimap_tex_offset: u16s(&src, &m, "sOwMinimapTexOffset", &[24])?,
        ow_minimap_pos_x: i16s(&src, &m, "sOwMinimapPosX", &[24])?,
        ow_minimap_pos_y: i16s(&src, &m, "sOwMinimapPosY", &[24])?,
        ow_compass_info: i16s(&src, &m, "sOwCompassInfo", &[24, 4])?,
        dgn_tex_index_base: i16s(&src, &m, "sDgnTexIndexBase", &[10])?,
        dgn_compass_info: i16s(&src, &m, "sDgnCompassInfo", &[10, 4])?,
        ow_minimap_width: i16s(&src, &m, "sOwMinimapWidth", &[24])?,
        ow_minimap_height: i16s(&src, &m, "sOwMinimapHeight", &[24])?,
        ow_entrance_icon_pos_x: i16s(&src, &m, "sOwEntranceIconPosX", &[24])?,
        ow_entrance_icon_pos_y: i16s(&src, &m, "sOwEntranceIconPosY", &[24])?,
        ow_entrance_flag: u16s(&src, &m, "sOwEntranceFlag", &[20])?,
        floor_coord_y,
        switch_entry_count: u16s(&src, &m, "sSwitchEntryCount", &[10])?,
        switch_from_room: u8s(&src, &m, "sSwitchFromRoom", &[10, sw])?,
        switch_from_floor: u8s(&src, &m, "sSwitchFromFloor", &[10, sw])?,
        switch_to_room: u8s(&src, &m, "sSwitchToRoom", &[10, sw])?,
        floor_id: u8s(&src, &m, "sFloorID", &[10, f])?,
        skull_floor_icon_y: i16s(&src, &m, "sSkullFloorIconY", &[10])?,
    })
}

/// The C file `spec` builds `ovl_map_mark_data` from for this version (`z_map_mark_data.c`, or
/// `z_map_mark_data_mq.c` for Master Quest), from its segment's `include` line.
pub fn map_mark_data_file(decomp: &Path) -> Result<String> {
    segment_source(decomp, "ovl_map_mark_data", "")
}

/// The C file `spec` builds `ovl_kaleido_scope`'s pause map marks from for this version
/// (`z_lmap_mark_data.c`, or `z_lmap_mark_data_mq.c` for Master Quest).
pub fn pause_map_mark_data_file(decomp: &Path) -> Result<String> {
    segment_source(decomp, "ovl_kaleido_scope", "/z_lmap_mark_data")
}

/// The first C file `segment`'s `include` lines name (once the version's `#if`s are applied)
/// whose path contains `part`.
fn segment_source(decomp: &Path, segment: &str, part: &str) -> Result<String> {
    let spec = read_c(decomp, "spec/overlays.inc")?;
    let name = format!("name \"{segment}\"");
    let seg = spec.split("beginseg").find(|s| s.contains(&name)).with_context(|| format!("spec: no {segment} segment"))?;
    let seg = seg.split("endseg").next().unwrap_or(seg);
    let obj = seg
        .lines()
        .filter_map(|l| l.trim().strip_prefix("include \"$(BUILD_DIR)/").and_then(|r| r.strip_suffix(".o\"")))
        .find(|o| o.contains(part))
        .with_context(|| format!("spec: {segment} includes no {part}"))?;
    Ok(format!("{obj}.c"))
}

/// `MapMarkIconData`: `{ markType, count, { { chestFlag, x, y }, ... } }`, its 12 points
/// zero-filled (fully braced, or `{ 0 }`).
fn icon(init: &Init, m: &Macros) -> Result<MapMarkIconData> {
    let l = init.list();
    let int = |i: Option<&Init>| -> Result<i64> {
        match i {
            None => Ok(0),
            Some(i) => {
                let a = i.atom().with_context(|| format!("MapMarkIconData: {i:?}"))?;
                m.eval(a).with_context(|| format!("MapMarkIconData: {a}"))
            }
        }
    };
    let mut points = Vec::new();
    if let Some(p) = l.get(2) {
        let items = p.list();
        if items.iter().all(|i| matches!(i, Init::List(_))) {
            for q in items {
                let q = q.list();
                points.push(MapMarkPoint { chest_flag: int(q.first())? as i8, x: int(q.get(1))? as u8, y: int(q.get(2))? as u8 });
            }
        } else {
            // Brace elision: the scalars fill the points in order.
            let atoms = p.flatten();
            for c in atoms.chunks(3) {
                let v = |k: usize| c.get(k).map(|a| m.eval(a).with_context(|| format!("MapMarkPoint: {a}"))).unwrap_or(Ok(0));
                points.push(MapMarkPoint { chest_flag: v(0)? as i8, x: v(1)? as u8, y: v(2)? as u8 });
            }
        }
    }
    anyhow::ensure!(points.len() <= 12, "{} points", points.len());
    points.resize(12, MapMarkPoint::default());
    Ok(MapMarkIconData { mark_type: int(l.first())? as i8, count: int(l.get(1))? as u8, points })
}

/// `gMapMarkDataTable`: by dungeon, its `MapMarkData` per room's minimap (three
/// `MapMarkIconData` each, zero-filled).
pub fn load_map_marks(decomp: &Path) -> Result<Vec<Vec<[MapMarkIconData; 3]>>> {
    let file = map_mark_data_file(decomp)?;
    let src = read_c(decomp, &file)?;
    let m = Macros::read(decomp, &["include/map_mark.h"])?;
    let mut out = Vec::new();
    for name in find_initializer(&src, "gMapMarkDataTable")?.flatten() {
        let name = name.trim();
        let mut rooms = Vec::new();
        for room in find_initializer(&src, name)?.list() {
            let icons = room.list();
            anyhow::ensure!(icons.len() <= 3, "{name}: {} MapMarkIconData", icons.len());
            let mut data: [MapMarkIconData; 3] = Default::default();
            for (k, i) in icons.iter().enumerate() {
                data[k] = icon(i, &m).with_context(|| name.to_string())?;
            }
            for d in data.iter_mut().skip(icons.len()) {
                d.points = vec![MapMarkPoint::default(); 12];
            }
            rooms.push(data);
        }
        out.push(rooms);
    }
    Ok(out)
}

/// A `Vtx` array (`static const Vtx name[] = { VTX(x, y, z, s, t, r, g, b, a), ... }`).
fn vtx_array(src: &str, m: &Macros, name: &str) -> Result<Vec<Vtx>> {
    let mut out = Vec::new();
    for v in find_initializer(src, name)?.list() {
        let a = v.atom().with_context(|| format!("{name}: {v:?}"))?;
        let args = a.trim().strip_prefix("VTX(").and_then(|r| r.strip_suffix(')')).with_context(|| format!("{name}: {a}"))?;
        let n: Vec<i64> = args.split(',').map(|x| m.eval(x.trim()).with_context(|| format!("{name}: {x}"))).collect::<Result<_>>()?;
        anyhow::ensure!(n.len() == 9, "{name}: VTX takes 9 values, not {}", n.len());
        out.push(Vtx { ob: [n[0] as i16, n[1] as i16, n[2] as i16], tc: [n[3] as i16, n[4] as i16], cn: [n[5] as u8, n[6] as u8, n[7] as u8, n[8] as u8] });
    }
    Ok(out)
}

/// `gPauseMapMarkDataTable`: by floor's map, its three `PauseMapMarkData` (`{ markType, unk_04,
/// vtx, vtxCount, count, { { chestFlag, x, y }, ... } }`), zero-filled, each with the `Vtx` array
/// it points at.
pub fn load_pause_map_marks(decomp: &Path) -> Result<Vec<[PauseMapMarkData; 3]>> {
    let file = pause_map_mark_data_file(decomp)?;
    let src = read_c(decomp, &file)?;
    let m = Macros::read(decomp, &["include/pause.h"])?;
    let int = |i: Option<&Init>| -> Result<i64> {
        match i.and_then(|i| i.atom()) {
            None => Ok(0),
            Some(a) => m.eval(a).with_context(|| format!("PauseMapMarkData: {a}")),
        }
    };
    let mut out = Vec::new();
    for maps in find_initializer(&src, "gPauseMapMarkDataTable")?.list() {
        let datas = maps.list();
        anyhow::ensure!(datas.len() <= 3, "{} PauseMapMarkData", datas.len());
        let mut data: [PauseMapMarkData; 3] = Default::default();
        for (k, d) in datas.iter().enumerate() {
            let l = d.list();
            let vtx = match l.get(2).and_then(|i| i.atom()).map(str::trim) {
                None | Some("NULL") | Some("0") => Vec::new(),
                Some(name) => vtx_array(&src, &m, name)?,
            };
            let mut points = Vec::new();
            if let Some(p) = l.get(5) {
                for q in p.list().iter().filter(|q| matches!(q, Init::List(_))) {
                    let q = q.list();
                    let f = |k: usize| q.get(k).and_then(|i| i.atom()).map(|a| crate::csrc::eval_expr(a).with_context(|| format!("PauseMapMarkPoint: {a}"))).unwrap_or(Ok(0.0));
                    points.push(PauseMapMarkPoint { chest_flag: int(q.first())? as i16, x: f(1)?, y: f(2)? });
                }
            }
            anyhow::ensure!(points.len() <= 12, "{} points", points.len());
            points.resize(12, PauseMapMarkPoint::default());
            data[k] = PauseMapMarkData { mark_type: int(l.first())? as i16, unk_04: int(l.get(1))? as i32, vtx, vtx_count: int(l.get(3))? as i32, count: int(l.get(4))? as i32, points };
        }
        for d in data.iter_mut().skip(datas.len()) {
            d.points = vec![PauseMapMarkPoint::default(); 12];
        }
        out.push(data);
    }
    Ok(out)
}

/// `table/map`: the tables from the C, `map_48x85_static` from the ROM.
pub fn load(decomp: &Path, rom: &crate::rom::Rom) -> Result<MapTables> {
    Ok(MapTables {
        data: load_map_data(decomp).context("z_map_data.c")?,
        marks: load_map_marks(decomp).context("gMapMarkDataTable")?,
        pause_marks: load_pause_map_marks(decomp).context("gPauseMapMarkDataTable")?,
        map_48x85_static: rom.file_by_name("map_48x85_static").context("map_48x85_static")?.to_vec(),
    })
}
