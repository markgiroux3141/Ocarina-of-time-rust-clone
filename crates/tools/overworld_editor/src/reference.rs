//! The game's own dungeons, read from its pack for the Reference mode (`app::reference_ui`): every
//! room of a dungeon's scene as the game draws it, and its floors from the pause map's tables
//! (`oot_game::map::MapData`: each floor's height and the rooms drawn on its map); its collision,
//! each surface sorted by what the game does with it (`Surf`, read as `oot_game::surface` reads
//! the surface types), its water boxes, and its actors (each room's and the doors between them).
//! See docs/EDITOR-ROOFS-STOREYS-DUNGEONS.md, "Pivot: a real level viewer first".

use eng_collision::collision::CollisionHeader;
use eng_gfx::DrawList;
use glam::Vec3;
use oot_game::map::{MapData, MAP_FLOORS, MAP_FLOOR_PALETTES};
use oot_game::pack::GamePack;
use oot_game::play_scene::SceneState;
use oot_game::scene::layer_for;

/// A scene the Reference mode can open: its file, its name, and whether it's played as a child
/// (the layer it's loaded with).
pub struct Dungeon {
    pub scene: &'static str,
    pub name: &'static str,
    pub child: bool,
}

const fn d(scene: &'static str, name: &'static str, child: bool) -> Dungeon {
    Dungeon { scene, name, child }
}

/// The dungeons, then their bosses' rooms.
pub const DUNGEONS: &[Dungeon] = &[
    d("ydan", "Inside the Deku Tree", true),
    d("ddan", "Dodongo's Cavern", true),
    d("bdan", "Inside Jabu-Jabu's Belly", true),
    d("Bmori1", "Forest Temple", false),
    d("HIDAN", "Fire Temple", false),
    d("MIZUsin", "Water Temple", false),
    d("HAKAdan", "Shadow Temple", false),
    d("jyasinzou", "Spirit Temple", false),
    d("HAKAdanCH", "Bottom of the Well", true),
    d("ice_doukutu", "Ice Cavern", false),
    d("men", "Gerudo Training Ground", false),
    d("ganontika", "Inside Ganon's Castle", false),
    d("ganon", "Ganon's Tower", false),
    d("ydan_boss", "Gohma's lair", true),
    d("ddan_boss", "King Dodongo's lair", true),
    d("bdan_boss", "Barinade's lair", true),
    d("moribossroom", "Phantom Ganon's lair", false),
    d("FIRE_bs", "Volvagia's lair", false),
    d("MIZUsin_bs", "Morpha's lair", false),
    d("HAKAdan_bs", "Bongo Bongo's lair", false),
    d("jyasinboss", "Twinrova's lair", false),
    d("ganon_boss", "Ganondorf's tower top", false),
];

/// How many of `DUNGEONS` are dungeons (the rest are bosses' rooms).
pub const DUNGEON_COUNT: usize = 13;

/// `SCENE_ICE_CAVERN`: the scenes up to it are the ten dungeons with maps (their id is their
/// `mapIndex`); `SCENE_DEKU_TREE_BOSS` to `SCENE_SHADOW_TEMPLE_BOSS` their bosses' rooms.
const SCENE_ICE_CAVERN: u16 = 0x09;
const SCENE_DEKU_TREE_BOSS: u16 = 0x11;
const SCENE_SHADOW_TEMPLE_BOSS: u16 = 0x18;

/// A floor of a dungeon as its pause map has it.
#[derive(Clone, Debug, PartialEq)]
pub struct Floor {
    /// `3F`, `1F`, `B1`...: `floorID`'s.
    pub name: String,
    /// Link is on this floor when he's above `y` (`floorCoordY`) and not above the floor over it's.
    pub y: f32,
    /// The floor over it's `y`: what's over it is cut away when it's shown. `None` for the top floor.
    pub above: Option<f32>,
    /// The rooms its map draws (`paletteRoom`), and the rooms on no floor's map whose lowest point
    /// is on it (by height).
    pub rooms: Vec<usize>,
}

/// A room's meshes and what the room list says about it.
pub struct Room {
    pub index: usize,
    /// Its entries' OPA and XLU lists (the game draws every room's OPA before any XLU).
    pub opa: Vec<DrawList>,
    pub xlu: Vec<DrawList>,
    pub tris: usize,
    /// Its meshes' bounds, in the game's axes.
    pub lo: Vec3,
    pub hi: Vec3,
    /// The floors whose maps draw it (indices into `Loaded::floors`).
    pub floors: Vec<usize>,
}

/// What the game does with a collision surface: the overlay's colours, the most telling first.
/// Climbing wins over the hookshot: the Deku Tree's vines and ladders take it too (the status
/// bar's description says so).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Surf {
    /// An exit to another scene (`SurfaceType_GetExitIndex`).
    Exit,
    /// Falling in voids out (`FLOOR_TYPE_9`; `FLOOR_PROPERTY_5`, `_12`).
    Void,
    /// Hurts (surface word 1 bit 27, `SurfaceType_IsWallDamage`'s; the hot floors, `FLOOR_TYPE_2`, `_3`).
    Hurts,
    /// The hookshot (and longshot) holds (`SurfaceType_IsHookshotSurface`: word 1 bit 17).
    Hookshot,
    /// Ladders (`WALL_FLAG_1`) and their tops (`WALL_FLAG_2`).
    Ladder,
    /// Climbable walls, vines and grates (`WALL_FLAG_3`), and `FLOOR_PROPERTY_6`'s ledges.
    Climb,
    /// Crawlspaces (`WALL_FLAG_CRAWLSPACE`).
    Crawl,
    /// Walls to push or pull (`WALL_FLAG_6`).
    Grab,
    /// Sinking floors: sand and quicksand (`FLOOR_TYPE_4`, `_7`, `_12`).
    Sand,
    /// Slippery (`FLOOR_EFFECT_1`: ice).
    Slippery,
    Floor,
    Wall,
    Ceiling,
}

impl Surf {
    pub const ALL: [Surf; 13] = [
        Surf::Exit,
        Surf::Void,
        Surf::Hurts,
        Surf::Hookshot,
        Surf::Ladder,
        Surf::Climb,
        Surf::Crawl,
        Surf::Grab,
        Surf::Sand,
        Surf::Slippery,
        Surf::Floor,
        Surf::Wall,
        Surf::Ceiling,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Surf::Exit => "Exit",
            Surf::Void => "Void",
            Surf::Hurts => "Hurts",
            Surf::Hookshot => "Hookshot",
            Surf::Ladder => "Ladder",
            Surf::Climb => "Climb",
            Surf::Crawl => "Crawlspace",
            Surf::Grab => "Push / pull",
            Surf::Sand => "Sand",
            Surf::Slippery => "Slippery",
            Surf::Floor => "Floor",
            Surf::Wall => "Wall",
            Surf::Ceiling => "Ceiling",
        }
    }

    pub fn colour(self) -> [u8; 3] {
        match self {
            Surf::Exit => [236, 72, 220],
            Surf::Void => [40, 30, 60],
            Surf::Hurts => [240, 70, 40],
            Surf::Hookshot => [60, 210, 240],
            Surf::Ladder => [250, 210, 60],
            Surf::Climb => [150, 230, 70],
            Surf::Crawl => [190, 120, 60],
            Surf::Grab => [250, 150, 200],
            Surf::Sand => [220, 190, 130],
            Surf::Slippery => [190, 230, 255],
            Surf::Floor => [96, 140, 84],
            Surf::Wall => [150, 144, 132],
            Surf::Ceiling => [96, 90, 110],
        }
    }

    /// Anything but a plain floor, wall or ceiling.
    pub fn special(self) -> bool {
        !matches!(self, Surf::Floor | Surf::Wall | Surf::Ceiling)
    }
}

/// A surface type's fields (`z_bgcheck.c`'s `SurfaceType_Get*`; `oot_game::surface`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SurfaceBits {
    pub cam: u32,
    pub exit: u32,
    pub floor_type: u32,
    pub wall_type: u32,
    pub floor_property: u32,
    pub sfx: u32,
    pub floor_effect: u32,
    pub hookshot: bool,
    pub hurts: bool,
    pub conveyor_speed: u32,
}

impl SurfaceBits {
    pub fn new(w: [u32; 2]) -> SurfaceBits {
        SurfaceBits {
            cam: w[0] & 0xFF,
            exit: w[0] >> 8 & 0x1F,
            floor_type: w[0] >> 13 & 0x1F,
            wall_type: w[0] >> 21 & 0x1F,
            floor_property: w[0] >> 26 & 0xF,
            sfx: w[1] & 0xF,
            floor_effect: w[1] >> 4 & 3,
            hookshot: w[1] >> 17 & 1 != 0,
            hurts: w[1] & 0x0800_0000 != 0,
            conveyor_speed: w[1] >> 18 & 7,
        }
    }

    fn wall_flags(&self) -> u32 {
        oot_game::surface::WALL_FLAGS.get(self.wall_type as usize).copied().unwrap_or(0)
    }

    /// What it is to the overlay, on a poly facing `ny` up (`BgCheck`'s floors: over 0.5; ceilings
    /// under -0.8).
    pub fn classify(&self, ny: f32) -> Surf {
        use oot_game::surface::*;
        let (floor, flags) = (ny > 0.5, self.wall_flags());
        if self.exit != 0 {
            Surf::Exit
        } else if self.floor_type == 9 || (floor && matches!(self.floor_property, 5 | 12)) {
            Surf::Void
        } else if self.hurts || (floor && matches!(self.floor_type, 2 | 3)) {
            Surf::Hurts
        } else if !floor && flags & (WALL_FLAG_1 | WALL_FLAG_2) != 0 {
            Surf::Ladder
        } else if !floor && (flags & WALL_FLAG_3 != 0 || self.floor_property == 6) {
            Surf::Climb
        } else if !floor && flags & WALL_FLAG_CRAWLSPACE != 0 {
            Surf::Crawl
        } else if self.hookshot {
            Surf::Hookshot
        } else if !floor && flags & WALL_FLAG_6 != 0 {
            Surf::Grab
        } else if floor && matches!(self.floor_type, 4 | 7 | 12) {
            Surf::Sand
        } else if floor && self.floor_effect == 1 {
            Surf::Slippery
        } else if floor {
            Surf::Floor
        } else if ny < -0.8 {
            Surf::Ceiling
        } else {
            Surf::Wall
        }
    }

    /// Every field that isn't zero, as the status bar shows it, with what the game does with it
    /// where that's known.
    pub fn describe(&self, ny: f32) -> String {
        let mut out = vec![if ny > 0.5 { "floor" } else if ny < -0.8 { "ceiling" } else { "wall" }.to_string()];
        if self.exit != 0 {
            out.push(format!("exit {}", self.exit));
        }
        if self.floor_type != 0 {
            let what = match self.floor_type {
                2 | 3 => " (hot)",
                4 => " (sand)",
                6 => " (no fall damage)",
                7 => " (quicksand)",
                8 => " (Jabu-Jabu's flesh)",
                9 => " (void)",
                11 => " (secret hole)",
                12 => " (sinking)",
                _ => "",
            };
            out.push(format!("floor type {}{what}", self.floor_type));
        }
        if self.wall_type != 0 {
            let what = match self.wall_type {
                2 => " (ladder)",
                3 => " (ladder's top)",
                4 => " (climbable)",
                5 | 6 => " (crawlspace)",
                7 => " (push / pull)",
                _ => "",
            };
            out.push(format!("wall type {}{what}", self.wall_type));
        }
        if self.floor_property != 0 {
            let what = match self.floor_property {
                5 | 12 => " (void)",
                6 => " (climbable)",
                11 => " (diving board)",
                _ => "",
            };
            out.push(format!("floor property {}{what}", self.floor_property));
        }
        if self.hookshot {
            out.push("hookshot".into());
        }
        if self.hurts {
            out.push("hurts".into());
        }
        if self.floor_effect != 0 {
            out.push(format!("floor effect {}{}", self.floor_effect, if self.floor_effect == 1 { " (slippery)" } else { "" }));
        }
        if self.conveyor_speed != 0 {
            out.push(format!("conveyor {}", self.conveyor_speed));
        }
        out.push(format!("camera {}", self.cam));
        out.push(format!("sound {}", self.sfx));
        out.join(" · ")
    }
}

/// A collision poly: its corners and normal (the game's axes), what it is, the room it's in (the
/// smallest room whose meshes' bounds hold its middle; `None` for none), and its surface type.
pub struct Poly {
    pub tri: [Vec3; 3],
    pub normal: Vec3,
    pub surf: Surf,
    pub room: Option<usize>,
    pub bits: SurfaceBits,
}

/// A water box (`WaterBox`): its surface's rectangle at `lo.y`, and its room (`None`: every room).
pub struct Water {
    pub lo: Vec3,
    pub hi: Vec3,
    pub room: Option<usize>,
}

/// An actor the scene places: a room's (`room`), or a transition actor (a door) between the rooms
/// on its two `sides` (-1: none).
#[derive(Clone, Debug)]
pub struct Actor {
    pub name: String,
    pub id: i16,
    pub pos: Vec3,
    pub rot: [i16; 3],
    pub params: u16,
    pub room: Option<usize>,
    pub sides: Option<[i8; 2]>,
}

impl Actor {
    /// Whether it's in or beside a room `shown` says is drawn.
    pub fn shown(&self, shown: &[bool]) -> bool {
        let on = |r: i64| usize::try_from(r).ok().and_then(|r| shown.get(r).copied()).unwrap_or(false);
        match (self.room, self.sides) {
            (Some(r), _) => on(r as i64),
            (None, Some(s)) => on(s[0] as i64) || on(s[1] as i64),
            _ => true,
        }
    }
}

/// A dungeon read for the Reference mode.
pub struct Loaded {
    pub dungeon: usize,
    pub scene: SceneState,
    pub rooms: Vec<Room>,
    /// Top floor first, as the pause map lists them; empty when the scene has no map.
    pub floors: Vec<Floor>,
    pub lo: Vec3,
    pub hi: Vec3,
    pub polys: Vec<Poly>,
    pub water: Vec<Water>,
    pub actors: Vec<Actor>,
    /// The scene's exit list (`SCENE_CMD_EXIT_LIST`): an exit surface's entrance, 1-based.
    pub exits: Vec<u16>,
}

impl Loaded {
    /// The nearest point of the `shown` rooms' meshes along a ray, in the game's axes, leaving out
    /// what's over `cut` (a height): its distance along `d` (a unit vector).
    pub fn raycast(&self, shown: &[bool], o: Vec3, d: Vec3, cut: Option<f32>) -> Option<f32> {
        let mut best: Option<f32> = None;
        for r in self.rooms.iter().filter(|r| shown.get(r.index).copied().unwrap_or(false)) {
            if !ray_box(o, d, r.lo, r.hi) {
                continue;
            }
            for v in r.opa.iter().chain(&r.xlu).flat_map(|l| &l.batches).flat_map(|b| b.vertices.chunks_exact(3)) {
                if let Some(t) = ray_tri(o, d, v[0].pos, v[1].pos, v[2].pos) {
                    if best.is_none_or(|b| t < b) && cut.is_none_or(|c| (o + d * t).y <= c + 1.0) {
                        best = Some(t);
                    }
                }
            }
        }
        best
    }

    /// The nearest collision poly along a ray (as `raycast`): its distance and index. Polys in no
    /// room are always tested.
    pub fn raycast_collision(&self, shown: &[bool], o: Vec3, d: Vec3, cut: Option<f32>) -> Option<(f32, usize)> {
        let mut best: Option<(f32, usize)> = None;
        for (i, p) in self.polys.iter().enumerate() {
            if p.room.is_some_and(|r| !shown.get(r).copied().unwrap_or(false)) {
                continue;
            }
            if let Some(t) = ray_tri(o, d, p.tri[0], p.tri[1], p.tri[2]) {
                if best.is_none_or(|b| t < b.0) && cut.is_none_or(|c| (o + d * t).y <= c + 1.0) {
                    best = Some((t, i));
                }
            }
        }
        best
    }
}

/// The room whose bounds (grown by `margin`) hold `p`, the smallest such.
fn room_holding(rooms: &[Room], p: Vec3, margin: f32) -> Option<usize> {
    rooms
        .iter()
        .filter(|r| r.lo.x <= r.hi.x && (p.cmpge(r.lo - margin) & p.cmple(r.hi + margin)).all())
        .min_by(|a, b| {
            let v = |r: &Room| (r.hi - r.lo).element_product();
            v(a).total_cmp(&v(b))
        })
        .map(|r| r.index)
}

/// Whether a ray meets a box (slabs).
fn ray_box(o: Vec3, d: Vec3, lo: Vec3, hi: Vec3) -> bool {
    let inv = d.recip();
    let (a, b) = ((lo - o) * inv, (hi - o) * inv);
    let (near, far) = (a.min(b).max_element(), a.max(b).min_element());
    far >= near.max(0.0)
}

/// Where a ray meets a triangle, either side (Moller-Trumbore).
fn ray_tri(o: Vec3, d: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    let (e1, e2) = (b - a, c - a);
    let p = d.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-9 {
        return None;
    }
    let inv = 1.0 / det;
    let s = o - a;
    let u = s.dot(p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = d.dot(q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = e2.dot(q) * inv;
    (t > 0.0).then_some(t)
}

/// `FloorID`'s name: `F_8F` (1) to `F_1F` (8), then `F_B1` (9) to `F_B8` (16).
pub fn floor_name(id: u8) -> String {
    if id <= 8 { format!("{}F", 9 - id as i32) } else { format!("B{}", id as i32 - 8) }
}

/// The floors of map `map_index` (a dungeon's), top first. With `only` (a boss's room), just
/// that floor.
pub fn floors(d: &MapData, map_index: usize, only: Option<usize>) -> Vec<Floor> {
    let at = |v: &[u8], i: usize| v.get(map_index * MAP_FLOORS + i).copied().unwrap_or(0);
    let y = |i: usize| d.floor_coord_y.get(map_index * MAP_FLOORS + i).copied().unwrap_or(f32::MIN);
    let mut out = vec![];
    for i in 0..MAP_FLOORS {
        let id = at(&d.floor_id, i);
        if id == 0 || only.is_some_and(|o| o != i) {
            continue;
        }
        let mut rooms: Vec<usize> = (0..MAP_FLOOR_PALETTES)
            .filter_map(|j| d.palette_room.get((map_index * MAP_FLOORS + i) * MAP_FLOOR_PALETTES + j).copied())
            .filter(|&r| (0..0xFF).contains(&r))
            .map(|r| r as usize)
            .collect();
        rooms.sort();
        rooms.dedup();
        let above = (i > 0 && only.is_none()).then(|| y(i - 1));
        out.push(Floor { name: floor_name(id), y: y(i), above, rooms });
    }
    out
}

/// Which of `floors` (top first) Link standing at height `y` is on (`Map_Update`'s rule: the
/// first whose height he's above), or the lowest.
pub fn floor_at(floors: &[Floor], y: f32) -> Option<usize> {
    floors.iter().position(|f| y > f.y).or(floors.len().checked_sub(1))
}

/// Reads dungeon `i` of `DUNGEONS`: its rooms, its floors.
pub fn load(pack: &GamePack, i: usize) -> Result<Loaded, String> {
    let dg = &DUNGEONS[i];
    let env = pack.env_tables().map_err(|e| format!("the environment tables: {e:#}"))?;
    // noon, the layer the game picks for an entrance at that age
    let day = 0x8000;
    let mut scene = SceneState::load(pack, &env, dg.scene, layer_for(dg.child, false), dg.child, false, day).map_err(|e| format!("{}: {e:#}", dg.scene))?;
    scene.all_rooms = true;
    scene.run_draw_config(0);
    let mut rooms = vec![];
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for r in &scene.rooms {
        let mut room = Room { index: r.index, opa: vec![], xlu: vec![], tris: r.triangles(), lo: Vec3::splat(f32::MAX), hi: Vec3::splat(f32::MIN), floors: vec![] };
        for e in &r.entries {
            for (d, out) in [(&e.opa, &mut room.opa), (&e.xlu, &mut room.xlu)] {
                if let Some(d) = d.as_ref().filter(|d| !d.batches.is_empty()) {
                    for v in d.batches.iter().flat_map(|b| &b.vertices) {
                        room.lo = room.lo.min(v.pos);
                        room.hi = room.hi.max(v.pos);
                    }
                    out.push(d.clone());
                }
            }
        }
        if room.lo.x <= room.hi.x {
            lo = lo.min(room.lo);
            hi = hi.max(room.hi);
        }
        rooms.push(room);
    }
    if lo.x > hi.x {
        (lo, hi) = (Vec3::ZERO, Vec3::ZERO);
    }
    let id = scene.data.id;
    let mut floors = match pack.map_tables() {
        Ok(t) if id <= SCENE_ICE_CAVERN => self::floors(&t.data, id as usize, None),
        Ok(t) if (SCENE_DEKU_TREE_BOSS..=SCENE_SHADOW_TEMPLE_BOSS).contains(&id) => {
            let boss = (id - SCENE_DEKU_TREE_BOSS) as usize;
            let f = t.data.boss_floor.get(boss).copied().unwrap_or(-1);
            // the boss's room is one floor of its dungeon's map
            let mut fl = if f >= 0 { self::floors(&t.data, boss, Some(f as usize)) } else { vec![] };
            for f in &mut fl {
                f.rooms = (0..rooms.len()).collect();
                f.above = None;
            }
            fl
        }
        _ => vec![],
    };
    // rooms on no floor's map go on the floor their lowest point is on
    if !floors.is_empty() {
        for r in &rooms {
            if r.lo.x <= r.hi.x && !floors.iter().any(|f| f.rooms.contains(&r.index)) {
                if let Some(f) = floor_at(&floors, r.lo.y + 1.0) {
                    floors[f].rooms.push(r.index);
                    floors[f].rooms.sort();
                }
            }
        }
    }
    for r in &mut rooms {
        r.floors = floors.iter().enumerate().filter(|(_, f)| f.rooms.contains(&r.index)).map(|(i, _)| i).collect();
    }
    let ld = scene.layer_data().clone();
    let col: CollisionHeader = pack.collision(&ld.collision).map_err(|e| format!("{}'s collision: {e:#}", dg.scene))?;
    let polys = col
        .polys
        .iter()
        .map(|p| {
            let tri = col.triangle(p);
            let normal = Vec3::new(p.normal[0] as f32, p.normal[1] as f32, p.normal[2] as f32) / 32767.0;
            let bits = SurfaceBits::new(col.surface_types.get(p.ty as usize).map_or([0; 2], |s| s.data));
            let room = room_holding(&rooms, (tri[0] + tri[1] + tri[2]) / 3.0, 20.0);
            Poly { tri, normal, surf: bits.classify(normal.y), room, bits }
        })
        .collect();
    let water = col
        .water_boxes
        .iter()
        .map(|w| {
            let lo = Vec3::new(w.x_min as f32, w.y_surface as f32, w.z_min as f32);
            let room = w.properties >> 13 & 0x3F;
            Water { lo, hi: lo + Vec3::new(w.x_length as f32, 0.0, w.z_length as f32), room: (room != 0x3F).then_some(room as usize) }
        })
        .collect();
    let at = pack.actor_table().map_err(|e| format!("the actor table: {e:#}"))?;
    let v = |p: [i16; 3]| Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32);
    let mut actors: Vec<Actor> = scene
        .rooms
        .iter()
        .flat_map(|r| r.actors.iter().map(move |a| (r.index, a)))
        .map(|(room, a)| Actor { name: at.name(a.id), id: a.id, pos: v(a.pos), rot: a.rot, params: a.params as u16, room: Some(room), sides: None })
        .collect();
    actors.extend(ld.transition_actors.iter().map(|t| Actor {
        name: at.name(t.id),
        id: t.id,
        pos: v(t.pos),
        rot: [0, t.rot_y, 0],
        params: t.params as u16,
        room: None,
        sides: Some([t.sides[0].0, t.sides[1].0]),
    }));
    Ok(Loaded { dungeon: i, scene, rooms, floors, lo, hi, polys, water, actors, exits: ld.exits.clone() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floor_names_as_the_pause_map_has_them() {
        assert_eq!(floor_name(8), "1F");
        assert_eq!(floor_name(6), "3F");
        assert_eq!(floor_name(9), "B1");
        assert_eq!(floor_name(10), "B2");
    }

    #[test]
    fn rays_meet_triangles_from_either_side_and_boxes() {
        let (a, b, c) = (Vec3::new(-1.0, 0.0, -1.0), Vec3::new(1.0, 0.0, -1.0), Vec3::new(0.0, 0.0, 1.0));
        assert_eq!(ray_tri(Vec3::new(0.0, 5.0, 0.0), Vec3::NEG_Y, a, b, c), Some(5.0));
        assert_eq!(ray_tri(Vec3::new(0.0, -2.0, 0.0), Vec3::Y, a, b, c), Some(2.0));
        assert_eq!(ray_tri(Vec3::new(3.0, 5.0, 0.0), Vec3::NEG_Y, a, b, c), None);
        assert!(ray_box(Vec3::new(0.0, 5.0, 0.0), Vec3::NEG_Y, Vec3::splat(-1.0), Vec3::splat(1.0)));
        assert!(!ray_box(Vec3::new(3.0, 5.0, 0.0), Vec3::NEG_Y, Vec3::splat(-1.0), Vec3::splat(1.0)));
    }

    #[test]
    fn surfaces_sort_by_what_the_game_does_with_them() {
        let b = |w0: u32, w1: u32| SurfaceBits::new([w0, w1]);
        assert_eq!(b(2 << 8, 0).classify(1.0), Surf::Exit);
        assert_eq!(b(9 << 13, 0).classify(1.0), Surf::Void);
        assert_eq!(b(5 << 26, 0).classify(1.0), Surf::Void);
        assert_eq!(b(2 << 13, 0).classify(1.0), Surf::Hurts);
        assert_eq!(b(0, 1 << 27).classify(0.0), Surf::Hurts);
        assert_eq!(b(0, 1 << 17).classify(0.0), Surf::Hookshot);
        assert_eq!(b(4 << 21, 1 << 17).classify(0.0), Surf::Climb);
        assert_eq!(b(2 << 21, 0).classify(0.0), Surf::Ladder);
        assert_eq!(b(4 << 21, 0).classify(0.0), Surf::Climb);
        assert_eq!(b(5 << 21, 0).classify(0.0), Surf::Crawl);
        assert_eq!(b(7 << 21, 0).classify(0.0), Surf::Grab);
        assert_eq!(b(4 << 13, 0).classify(1.0), Surf::Sand);
        assert_eq!(b(0, 1 << 4).classify(1.0), Surf::Slippery);
        assert_eq!(b(0, 0).classify(1.0), Surf::Floor);
        assert_eq!(b(0, 0).classify(0.0), Surf::Wall);
        assert_eq!(b(0, 0).classify(-1.0), Surf::Ceiling);
        assert_eq!(b(2 << 21 | 3, 1 << 17).describe(0.0), "wall · wall type 2 (ladder) · hookshot · camera 3 · sound 0");
    }

    #[test]
    fn floor_at_takes_the_first_floor_it_is_above() {
        let f = |y| Floor { name: String::new(), y, above: None, rooms: vec![] };
        let fl = [f(400.0), f(0.0), f(-400.0)];
        assert_eq!(floor_at(&fl, 500.0), Some(0));
        assert_eq!(floor_at(&fl, 100.0), Some(1));
        assert_eq!(floor_at(&fl, -1000.0), Some(2));
    }

    /// Draws a few dungeons a floor at a time, headless, as the Reference mode draws them (the
    /// game's renderer, the draw config's textures, the cut over the floor), into PNGs in
    /// `$REF_RENDER_DIR` (else the temp folder): `cargo test -p overworld_editor --release
    /// render_reference -- --ignored`.
    #[test]
    #[ignore]
    fn render_reference_floors() {
        let Ok(pack) = GamePack::open_default() else { return };
        let dir = std::env::var_os("REF_RENDER_DIR").map_or_else(std::env::temp_dir, std::path::PathBuf::from);
        let (device, queue) = eng_render::headless_device().unwrap();
        let mut r = eng_render::Renderer::new(&device, &queue);
        let target = eng_render::Target::new(&device, 640, 480);
        for name in ["ydan", "HIDAN", "bdan"] {
            let i = DUNGEONS.iter().position(|d| d.scene == name).unwrap();
            let l = load(&pack, i).unwrap();
            let mut models = vec![];
            for room in &l.rooms {
                for (xlu, lists) in [(false, &room.opa), (true, &room.xlu)] {
                    for d in lists {
                        let mut m = r.upload(&device, &queue, d);
                        r.set_images(&device, &queue, &mut m, None, &l.scene.segment_textures[xlu as usize]);
                        m.set_draw_values(&queue, l.scene.segments.as_ref().map(|v| &v[xlu as usize]), &[], None, false);
                        let bones: Vec<glam::Mat4> = d.site_bases.iter().map(glam::Mat4::from_cols_array).collect();
                        m.pose(&bones, glam::Mat4::IDENTITY);
                        models.push((room.index, xlu, m));
                    }
                }
            }
            let mut views: Vec<(String, Option<usize>)> = vec![("all".into(), None)];
            views.extend(l.floors.iter().enumerate().map(|(f, fl)| (fl.name.clone(), Some(f))));
            for (label, f) in views {
                let rooms = f.map(|f| l.floors[f].rooms.clone());
                let cut = f.and_then(|f| l.floors[f].above);
                r.set_clip(cut.map(|c| (Vec3::Y, c + 1.0)));
                let on = |m: &(usize, bool, eng_render::GpuModel), xlu: bool| m.1 == xlu && rooms.as_ref().is_none_or(|rs| rs.contains(&m.0));
                let list: Vec<&eng_render::GpuModel> = models.iter().filter(|m| on(m, false)).chain(models.iter().filter(|m| on(m, true))).map(|m| &m.2).collect();
                let mut cam = eng_render::Camera::framing(l.lo, l.hi, 0.5, 0.75, 45f32.to_radians());
                cam.target.y = cut.map_or(cam.target.y, |c| c.min(l.hi.y) - 300.0);
                let light = eng_render::Lighting::default();
                let mut enc = device.create_command_encoder(&Default::default());
                r.render(&device, &queue, &mut enc, &target, &list, &cam, &light, &[], &[], [0.07, 0.08, 0.09, 1.0]);
                queue.submit([enc.finish()]);
                let px = target.read_rgba(&device, &queue).unwrap();
                let out = dir.join(format!("ref_{name}_{label}.png"));
                image::RgbaImage::from_raw(640, 480, px).unwrap().save(&out).unwrap();
                println!("{}", out.display());
            }
        }
    }

    /// Every dungeon loads from the pack (skipped without one), with its floors, and every room
    /// with a mesh is on a floor.
    #[test]
    fn the_dungeons_load_with_their_floors() {
        let Ok(pack) = GamePack::open_default() else { return };
        for (i, dg) in DUNGEONS.iter().enumerate() {
            let l = load(&pack, i).unwrap_or_else(|e| panic!("{}: {e}", dg.scene));
            assert!(!l.rooms.is_empty(), "{}", dg.scene);
            let names: Vec<String> = l.floors.iter().map(|f| format!("{} y>{} {:?}", f.name, f.y, f.rooms)).collect();
            let mut kinds: Vec<String> = Surf::ALL.iter().filter(|k| k.special()).map(|&k| (k, l.polys.iter().filter(|p| p.surf == k).count())).filter(|x| x.1 > 0).map(|(k, n)| format!("{} {n}", k.name())).collect();
            kinds.push(format!("in no room {}", l.polys.iter().filter(|p| p.room.is_none()).count()));
            println!("{} ({} rooms, {} tris): {}", dg.scene, l.rooms.len(), l.rooms.iter().map(|r| r.tris).sum::<usize>(), names.join(" | "));
            println!("    {} polys: {}; {} water boxes; {} actors ({} doors)", l.polys.len(), kinds.join(", "), l.water.len(), l.actors.len(), l.actors.iter().filter(|a| a.sides.is_some()).count());
            assert!(!l.polys.is_empty(), "{} has no collision", dg.scene);
            // the ten dungeons with maps have floors (Ganon's Castle has none)
            assert!(i >= 10 || !l.floors.is_empty(), "{} has no floors", dg.scene);
            if !l.floors.is_empty() {
                for r in l.rooms.iter().filter(|r| r.tris > 0) {
                    assert!(!r.floors.is_empty(), "{} room {} is on no floor", dg.scene, r.index);
                }
            }
        }
    }
}

