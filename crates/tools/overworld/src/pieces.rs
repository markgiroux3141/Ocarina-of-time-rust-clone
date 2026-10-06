//! Kit pieces: Kokiri Forest's buildings, stumps, stones, hedge, exits and crawlspace, cut from the
//! clone's extract (`spot04.glb` for the meshes, `collision.json` for the collision) by a committed
//! manifest (`kit/kokiri.json`), into `pieces.json` (ROM data, so in out/). See ADR 0036.
//!
//! A piece is chosen by a box in the scene, in the builder's axes (x east, y north, z up: glTF's
//! (x, -z, y)). By default it takes every connected piece of a room mesh lying wholly inside the box
//! (pieces share no vertex with the terrain round them); with `materials` it takes the faces of those
//! roles inside the box instead, for pieces joined to the terrain (the log tunnels). Collision is every
//! poly wholly inside the box (or `collision.box`), with its spot04 surface type mapped to a role by the
//! manifest's `surfaces` table: an unmapped type fails, so nothing collides by accident.
//!
//! Each piece is moved into its own frame: the origin at its base (houses: the door's floor, stones:
//! their top), turned so +y is the way it faces (houses: out of the door). The manifest's other fields
//! (scale limits, doors, openings) pass through to the builder.

use crate::kit::{read_glb, KOKIRI_ROLES};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

/// The manifest (`kit/kokiri.json`).
#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub source: Source,
    /// spot04 surface type -> collision role (and an "about" entry).
    pub surfaces: BTreeMap<String, String>,
    pub pieces: Vec<PieceDef>,
    /// Measurements for the pieces the builder generates (fences, paths, planks), passed through.
    #[serde(default)]
    pub generated: Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Source {
    pub glb: String,
    pub collision: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PieceDef {
    pub name: String,
    #[serde(default)]
    pub label: String,
    pub kind: String,
    pub take: Take,
    #[serde(default)]
    pub collision: Option<BoxSel>,
    /// "base" (the bottom's centre), "top", "door" (the door's floor), or [x, y, z].
    #[serde(default = "base")]
    pub origin: Value,
    /// "door" (out of it), "normal" (the faces' mean normal), or [dx, dy]; north by default.
    #[serde(default)]
    pub facing: Option<Value>,
    #[serde(default)]
    pub door: Option<DoorDef>,
    #[serde(default)]
    pub opening: Option<OpeningDef>,
    /// Missing: locked at 1.
    #[serde(default)]
    pub scale: Option<Scale>,
    /// Sides to add where the piece's top has an open edge (spot04's hedge has skirts on two of
    /// its four sides): a band of `material` from the top down to the base, `tile` per repeat.
    #[serde(default)]
    pub close_sides: Option<CloseSides>,
    /// A wall piece whose texture repeats with its size instead of stretching (the vines): see
    /// `Piece::tiles`.
    #[serde(default)]
    pub tiles: bool,
    #[serde(default)]
    pub about: String,
}

fn base() -> Value {
    Value::String("base".into())
}

#[derive(Debug, Clone, Deserialize)]
pub struct Take {
    #[serde(rename = "box")]
    pub bbox: [[f64; 3]; 2],
    /// Only these room meshes (room_0_opa, ...).
    #[serde(default)]
    pub rooms: Vec<String>,
    /// Faces of these roles instead of whole connected pieces.
    #[serde(default)]
    pub materials: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CloseSides {
    pub material: String,
    pub tile: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BoxSel {
    #[serde(rename = "box")]
    pub bbox: [[f64; 3]; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoorDef {
    /// The scene exit its doorway's floor carries (checked).
    pub exit: u32,
    /// Where it led in the game.
    pub entrance: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpeningDef {
    #[serde(default)]
    pub exit: Option<u32>,
    /// The shortest the piece may be cut back to (the space behind a wall it needs).
    pub min_depth: f64,
}

/// Scale limits per axis (the piece's own x, y, z); `uniform`: one factor for all three.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scale {
    pub min: [f64; 3],
    pub max: [f64; 3],
    #[serde(default)]
    pub uniform: bool,
}

impl Default for Scale {
    fn default() -> Self {
        Scale { min: [1.0; 3], max: [1.0; 3], uniform: true }
    }
}

impl Scale {
    pub fn locked(&self) -> bool {
        self.min == [1.0; 3] && self.max == [1.0; 3]
    }

    /// `s` brought within the limits (made uniform first if it has to be).
    pub fn clamp(&self, s: [f64; 3]) -> [f64; 3] {
        let s = if self.uniform { [s[0]; 3] } else { s };
        [0, 1, 2].map(|k| s[k].clamp(self.min[k], self.max[k]))
    }
}

/// The cut kit (`pieces.json`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Kit {
    pub name: String,
    pub source: String,
    pub pieces: Vec<Piece>,
    #[serde(default)]
    pub generated: Value,
}

/// A piece in its own frame: origin at its base, facing +y.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Piece {
    pub name: String,
    pub label: String,
    pub kind: String,
    pub materials: Vec<PieceMaterial>,
    pub verts: Vec<[f64; 3]>,
    pub normals: Vec<[f64; 3]>,
    pub tris: Vec<[u32; 3]>,
    /// Per corner, v up (as the level export).
    pub uvs: Vec<[[f64; 2]; 3]>,
    pub mat: Vec<u32>,
    /// Collision roles (`oot_import::level::ROLES`).
    pub surfaces: Vec<String>,
    pub col_verts: Vec<[f64; 3]>,
    pub col_tris: Vec<[u32; 3]>,
    pub col_surf: Vec<u32>,
    pub bounds: [[f64; 3]; 2],
    /// The convex hull of everything, seen from above.
    pub footprint: Vec<[f64; 2]>,
    pub scale: Scale,
    pub door: Option<Door>,
    pub opening: Option<Opening>,
    /// What Link can do with it: door, exit, ladder, crawl, vines.
    pub functions: Vec<String>,
    /// Where it was in spot04: the origin and the turn (degrees) that faced it north.
    pub source_origin: [f64; 3],
    pub source_yaw: f64,
    /// Scaled, its texture repeats at the piece's own density instead of stretching; set on a
    /// wall, it reaches from the floor to the wall's top however tall that is.
    #[serde(default)]
    pub tiles: bool,
    pub about: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PieceMaterial {
    /// The texture library's name (kf_<role>).
    pub texture: String,
    /// The combiner's constant colour where the extractor couldn't bake it into the texture
    /// (translucent materials): multiply it into the shade.
    pub tint: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Door {
    /// The doorway's floor, in the piece's frame; out of the door is +y.
    pub pos: [f64; 3],
    pub exit: u32,
    pub entrance: String,
}

/// A piece set into a wall (a log tunnel, a crawlspace): the wall's face is y = 0, the opening's
/// foot the origin; it reaches `depth` behind the wall.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opening {
    pub width: f64,
    pub height: f64,
    pub depth: f64,
    pub min_depth: f64,
    pub exit: Option<u32>,
}

impl Kit {
    pub fn load(dir: &Path) -> Result<Kit, String> {
        let p = dir.join("pieces.json");
        let s = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        serde_json::from_str(&s).map_err(|e| format!("{}: {e}", p.display()))
    }

    pub fn get(&self, name: &str) -> Option<&Piece> {
        self.pieces.iter().find(|p| p.name == name)
    }
}

impl Piece {
    /// Unique collision vertices (what it adds to the level's 8192 at most).
    pub fn collision_vertices(&self) -> usize {
        self.col_verts.len()
    }

    /// Where it meets the ground, seen from above: the hull of what's within 10 of its base (a
    /// house's walls, not its eaves). The whole footprint if that's too little to have a hull.
    pub fn base_outline(&self) -> Vec<[f64; 2]> {
        let low = self.bounds[0][2] + 10.0;
        let h = hull(self.verts.iter().chain(&self.col_verts).filter(|q| q[2] <= low).map(|q| [q[0], q[1]]).collect());
        if h.len() >= 3 { h } else { self.footprint.clone() }
    }
}

/// One render triangle of the scene, in builder axes.
struct Tri {
    room: String,
    role: &'static str,
    mat: usize,
    p: [[f64; 3]; 3],
    n: [[f64; 3]; 3],
    uv: [[f64; 2]; 3],
}

fn to_builder(v: [f64; 3]) -> [f64; 3] {
    [v[0], -v[2], v[1]]
}

/// A glb accessor as f64s, `k` per element.
pub(crate) fn accessor(j: &Value, bin: &[u8], i: usize, k: usize) -> Result<Vec<f64>, String> {
    let a = &j["accessors"][i];
    let v = &j["bufferViews"][a["bufferView"].as_u64().ok_or("glb: sparse accessor")? as usize];
    let count = a["count"].as_u64().unwrap_or(0) as usize;
    let off = v["byteOffset"].as_u64().unwrap_or(0) as usize + a["byteOffset"].as_u64().unwrap_or(0) as usize;
    let comp = a["componentType"].as_u64().unwrap_or(0);
    let size = match comp {
        5126 | 5125 => 4,
        5123 => 2,
        5121 => 1,
        c => return Err(format!("glb: component type {c}")),
    };
    let stride = v["byteStride"].as_u64().map(|s| s as usize).unwrap_or(size * k);
    let mut out = Vec::with_capacity(count * k);
    for e in 0..count {
        for c in 0..k {
            let at = off + e * stride + c * size;
            let b = bin.get(at..at + size).ok_or("glb: accessor outside the binary chunk")?;
            out.push(match comp {
                5126 => f32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64,
                5125 => u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64,
                5123 => u16::from_le_bytes([b[0], b[1]]) as f64,
                _ => b[0] as f64,
            });
        }
    }
    Ok(out)
}

/// Every render triangle of the scene's room meshes, and each material's tint.
fn scene_tris(glb: &[u8]) -> Result<(Vec<Tri>, Vec<[f64; 3]>), String> {
    let (j, bin) = read_glb(glb)?;
    let mats = j["materials"].as_array().cloned().unwrap_or_default();
    let tints: Vec<[f64; 3]> = mats
        .iter()
        .map(|m| {
            let f = &m["pbrMetallicRoughness"]["baseColorFactor"];
            [0, 1, 2].map(|k| f[k].as_f64().unwrap_or(1.0))
        })
        .collect();
    let mut out = vec![];
    for node in j["nodes"].as_array().cloned().unwrap_or_default() {
        let name = node["name"].as_str().unwrap_or("");
        let Some(mi) = node["mesh"].as_u64() else { continue };
        if !name.starts_with("room_") {
            continue;
        }
        if node.get("matrix").is_some() || node.get("translation").is_some() || node.get("rotation").is_some() || node.get("scale").is_some() {
            return Err(format!("glb: node {name} has a transform; rooms were in world space"));
        }
        for prim in j["meshes"][mi as usize]["primitives"].as_array().cloned().unwrap_or_default() {
            let mat = prim["material"].as_u64().ok_or(format!("{name}: a primitive without a material"))? as usize;
            let mname = mats[mat]["name"].as_str().unwrap_or("");
            let n: usize = mname.rsplit("mat").next().and_then(|s| s.parse().ok()).ok_or(format!("{name}: material {mname:?}"))?;
            let role = *KOKIRI_ROLES.get(n).ok_or(format!("{mname}: no role (KOKIRI_ROLES)"))?;
            let at = &prim["attributes"];
            let get = |k: &str, w: usize| -> Result<Vec<f64>, String> {
                accessor(&j, bin, at[k].as_u64().ok_or(format!("{name}: no {k}"))? as usize, w)
            };
            // unlit materials (the mushrooms) have no normals: their faces' own are used
            let (pos, uv) = (get("POSITION", 3)?, get("TEXCOORD_0", 2)?);
            let nrm = if at.get("NORMAL").is_some() { Some(get("NORMAL", 3)?) } else { None };
            let idx: Vec<usize> = match prim["indices"].as_u64() {
                Some(i) => accessor(&j, bin, i as usize, 1)?.into_iter().map(|x| x as usize).collect(),
                None => (0..pos.len() / 3).collect(),
            };
            for t in idx.chunks_exact(3) {
                let v3 = |a: &[f64], i: usize| to_builder([a[3 * i], a[3 * i + 1], a[3 * i + 2]]);
                let p = [0, 1, 2].map(|c| v3(&pos, t[c]));
                let n = match &nrm {
                    Some(nrm) => [0, 1, 2].map(|c| v3(nrm, t[c])),
                    None => [face_normal(&p); 3],
                };
                out.push(Tri {
                    room: name.to_string(),
                    role,
                    mat,
                    p,
                    n,
                    // glTF's v runs down; the export's runs up
                    uv: [0, 1, 2].map(|c| [uv[2 * t[c]], 1.0 - uv[2 * t[c] + 1]]),
                });
            }
        }
    }
    Ok((out, tints))
}

fn face_normal(p: &[[f64; 3]; 3]) -> [f64; 3] {
    let (e, f) = ([p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]], [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]]);
    let n = [e[1] * f[2] - e[2] * f[1], e[2] * f[0] - e[0] * f[2], e[0] * f[1] - e[1] * f[0]];
    let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-12);
    n.map(|x| x / l)
}

fn inside(p: &[[f64; 3]], b: &[[f64; 3]; 2]) -> bool {
    p.iter().all(|q| (0..3).all(|k| q[k] >= b[0][k] - 0.5 && q[k] <= b[1][k] + 0.5))
}

/// Connected pieces of one room's triangles (sharing a vertex position, rounded to whole units).
fn components(tris: &[&Tri]) -> Vec<Vec<usize>> {
    let mut parent: Vec<usize> = (0..tris.len()).collect();
    fn find(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    let mut owner: HashMap<[i64; 3], usize> = HashMap::new();
    for (i, t) in tris.iter().enumerate() {
        for q in &t.p {
            let k = q.map(|x| x.round() as i64);
            match owner.get(&k) {
                Some(&o) => {
                    let (a, b) = (find(&mut parent, o), find(&mut parent, i));
                    parent[a] = b;
                }
                None => {
                    owner.insert(k, i);
                }
            }
        }
    }
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..tris.len() {
        let r = find(&mut parent, i);
        groups.entry(r).or_default().push(i);
    }
    groups.into_values().collect()
}

/// The convex hull (counter-clockwise), Andrew's monotone chain.
pub fn hull(mut pts: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    pts.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    pts.dedup();
    if pts.len() < 3 {
        return pts;
    }
    let cross = |o: [f64; 2], a: [f64; 2], b: [f64; 2]| (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
    let mut lower: Vec<[f64; 2]> = vec![];
    for &p in &pts {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0.0 {
            lower.pop();
        }
        lower.push(p);
    }
    let mut upper: Vec<[f64; 2]> = vec![];
    for &p in pts.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0.0 {
            upper.pop();
        }
        upper.push(p);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn r(x: f64, k: f64) -> f64 {
    let v = (x * k).round() / k;
    if v == 0.0 { 0.0 } else { v }
}

/// Cuts every piece of the manifest out of the scene's glb and collision.json.
pub fn cut(manifest: &Manifest, glb: &[u8], collision: &Value) -> Result<Kit, String> {
    let (tris, tints) = scene_tris(glb)?;
    let cverts: Vec<[f64; 3]> = collision["vertices"]
        .as_array()
        .ok_or("collision.json: no vertices")?
        .iter()
        .map(|v| to_builder([0, 1, 2].map(|k| v[k].as_f64().unwrap_or(0.0))))
        .collect();
    let stypes = collision["surface_types"].as_array().ok_or("collision.json: no surface types")?;
    let polys = collision["polys"].as_array().ok_or("collision.json: no polys")?;
    let mut rooms: BTreeMap<&str, Vec<&Tri>> = BTreeMap::new();
    for t in &tris {
        rooms.entry(t.room.as_str()).or_default().push(t);
    }
    let comps: BTreeMap<&str, Vec<Vec<usize>>> = rooms.iter().map(|(k, v)| (*k, components(v))).collect();
    let mut pieces = vec![];
    for def in &manifest.pieces {
        let fail = |m: String| format!("piece {}: {m}", def.name);
        let b = &def.take.bbox;
        // render triangles
        let mut sel: Vec<&Tri> = vec![];
        for (room, ts) in &rooms {
            if !def.take.rooms.is_empty() && !def.take.rooms.iter().any(|r| r == room) {
                continue;
            }
            if def.take.materials.is_empty() {
                for c in &comps[room] {
                    if c.iter().all(|&i| inside(&ts[i].p, b)) {
                        sel.extend(c.iter().map(|&i| ts[i]));
                    }
                }
            } else {
                sel.extend(ts.iter().filter(|t| def.take.materials.iter().any(|m| m == t.role) && inside(&t.p, b)));
            }
        }
        if sel.is_empty() {
            return Err(fail("nothing in its box: has the extraction changed?".into()));
        }
        // collision
        let cb = def.collision.as_ref().map(|c| &c.bbox).unwrap_or(b);
        let mut col: Vec<([[f64; 3]; 3], [usize; 3], String, u64)> = vec![];
        for p in polys {
            // polys only the camera collides with (the hedge's walls and top: Link wades through)
            if p["ignore_entities"].as_bool() == Some(true) {
                continue;
            }
            let vi: Vec<usize> = p["vertices"].as_array().map(|a| a.iter().filter_map(|x| x.as_u64().map(|x| x as usize)).collect()).unwrap_or_default();
            if vi.len() != 3 {
                continue;
            }
            let pts = [cverts[vi[0]], cverts[vi[1]], cverts[vi[2]]];
            if !inside(&pts, cb) {
                continue;
            }
            let st = p["surface_type"].as_u64().unwrap_or(0);
            let role = manifest.surfaces.get(&st.to_string()).ok_or_else(|| fail(format!("spot04 surface type {st} has no role in the manifest's surfaces")))?;
            let exit = stypes.get(st as usize).and_then(|s| s["exit_index"].as_u64()).unwrap_or(0);
            col.push((pts, [vi[0], vi[1], vi[2]], role.clone(), exit));
        }
        // the frame
        let all = sel.iter().flat_map(|t| t.p.iter());
        let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
        for q in all {
            for k in 0..3 {
                lo[k] = lo[k].min(q[k]);
                hi[k] = hi[k].max(q[k]);
            }
        }
        let doorway: Vec<[f64; 3]> = col.iter().filter(|c| c.2 == "door").flat_map(|c| c.0).collect();
        if let Some(d) = &def.door {
            if doorway.is_empty() {
                return Err(fail("no door floor (collision with role door) in its box".into()));
            }
            if let Some(c) = col.iter().find(|c| c.2 == "door" && c.3 != d.exit as u64) {
                return Err(fail(format!("its doorway carries exit {}, the manifest says {}", c.3, d.exit)));
            }
        }
        let door_c = (!doorway.is_empty()).then(|| {
            let n = doorway.len() as f64;
            let s = doorway.iter().fold([0.0; 3], |a, q| [a[0] + q[0], a[1] + q[1], a[2] + q[2]]);
            [s[0] / n, s[1] / n, doorway.iter().map(|q| q[2]).fold(f64::INFINITY, f64::min)]
        });
        let centre = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0];
        let origin = match &def.origin {
            Value::String(s) if s == "base" => [centre[0], centre[1], lo[2]],
            Value::String(s) if s == "top" => [centre[0], centre[1], hi[2]],
            Value::String(s) if s == "door" => [centre[0], centre[1], door_c.ok_or_else(|| fail("origin door, but it has no door".into()))?[2]],
            Value::Array(a) if a.len() == 3 => [0, 1, 2].map(|k| a[k].as_f64().unwrap_or(0.0)),
            o => return Err(fail(format!("origin {o}"))),
        };
        let facing = match &def.facing {
            None => [0.0, 1.0],
            Some(Value::String(s)) if s == "door" => {
                let d = door_c.ok_or_else(|| fail("facing door, but it has no door".into()))?;
                [d[0] - origin[0], d[1] - origin[1]]
            }
            Some(Value::String(s)) if s == "normal" => {
                let mut n = [0.0; 2];
                for t in &sel {
                    let (a, b2, c) = (t.p[0], t.p[1], t.p[2]);
                    let (e, f) = ([b2[0] - a[0], b2[1] - a[1], b2[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
                    n[0] += e[1] * f[2] - e[2] * f[1];
                    n[1] += e[2] * f[0] - e[0] * f[2];
                }
                n
            }
            Some(Value::Array(a)) if a.len() == 2 => [a[0].as_f64().unwrap_or(0.0), a[1].as_f64().unwrap_or(1.0)],
            Some(f) => return Err(fail(format!("facing {f}"))),
        };
        if facing[0].hypot(facing[1]) < 1e-9 {
            return Err(fail("no facing direction".into()));
        }
        let yaw = facing[1].atan2(facing[0]) - std::f64::consts::FRAC_PI_2;
        let (s, c) = (-yaw).sin_cos();
        let rot = |v: [f64; 3]| [v[0] * c - v[1] * s, v[0] * s + v[1] * c, v[2]];
        let local = |q: [f64; 3]| rot([q[0] - origin[0], q[1] - origin[1], q[2] - origin[2]]);

        let mut piece = Piece {
            name: def.name.clone(),
            label: if def.label.is_empty() { def.name.clone() } else { def.label.clone() },
            kind: def.kind.clone(),
            scale: def.scale.clone().unwrap_or_default(),
            source_origin: origin.map(|x| r(x, 1e3)),
            source_yaw: r(yaw.to_degrees(), 1e3),
            tiles: def.tiles,
            about: def.about.clone(),
            ..Default::default()
        };
        let mut vindex: HashMap<[i64; 6], u32> = HashMap::new();
        let mut mindex: HashMap<usize, u32> = HashMap::new();
        for t in &sel {
            let m = *mindex.entry(t.mat).or_insert_with(|| {
                piece.materials.push(PieceMaterial { texture: format!("kf_{}", t.role), tint: tints[t.mat].map(|x| r(x, 1e4)) });
                piece.materials.len() as u32 - 1
            });
            let mut ids = [0u32; 3];
            for k in 0..3 {
                let (p, n) = (local(t.p[k]).map(|x| r(x, 1e3)), rot(t.n[k]).map(|x| r(x, 1e4)));
                let key = [p[0], p[1], p[2], n[0], n[1], n[2]].map(|x| (x * 1e3).round() as i64);
                ids[k] = *vindex.entry(key).or_insert_with(|| {
                    piece.verts.push(p);
                    piece.normals.push(n);
                    piece.verts.len() as u32 - 1
                });
            }
            piece.tris.push(ids);
            piece.uvs.push(t.uv.map(|uv| uv.map(|x| r(x, 1e5))));
            piece.mat.push(m);
        }
        let mut cindex: HashMap<usize, u32> = HashMap::new();
        let mut sindex: HashMap<String, u32> = HashMap::new();
        for (pts, vi, role, _) in &col {
            let mut ids = [0u32; 3];
            for k in 0..3 {
                ids[k] = *cindex.entry(vi[k]).or_insert_with(|| {
                    piece.col_verts.push(local(pts[k]).map(|x| r(x, 1e3)));
                    piece.col_verts.len() as u32 - 1
                });
            }
            let si = *sindex.entry(role.clone()).or_insert_with(|| {
                piece.surfaces.push(role.clone());
                piece.surfaces.len() as u32 - 1
            });
            piece.col_tris.push(ids);
            piece.col_surf.push(si);
        }
        if let Some(cs) = &def.close_sides {
            close_sides(&mut piece, cs);
        }
        // a wall piece's collision lies where it's drawn (spot04's vines collide as the cliff
        // behind them, 4 back; set on another wall, that would be inside it)
        if def.kind == "wall" {
            for q in &mut piece.col_verts {
                q[1] = 0.0;
            }
        }
        let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
        for q in piece.verts.iter().chain(&piece.col_verts) {
            for k in 0..3 {
                lo[k] = lo[k].min(q[k]);
                hi[k] = hi[k].max(q[k]);
            }
        }
        piece.bounds = [lo, hi];
        piece.footprint = hull(piece.verts.iter().chain(&piece.col_verts).map(|q| [q[0], q[1]]).collect());
        piece.door = match (&def.door, door_c) {
            (Some(d), Some(c)) => Some(Door { pos: local(c).map(|x| r(x, 1e3)), exit: d.exit, entrance: d.entrance.clone() }),
            _ => None,
        };
        piece.opening = def.opening.as_ref().map(|o| {
            let (vlo, vhi) = piece.verts.iter().fold(([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]), |(lo, hi), q| {
                ([0, 1, 2].map(|k| lo[k].min(q[k])), [0, 1, 2].map(|k| hi[k].max(q[k])))
            });
            Opening { width: r(vhi[0] - vlo[0], 1e3), height: r(vhi[2], 1e3), depth: r(-vlo[1], 1e3), min_depth: o.min_depth, exit: o.exit }
        });
        let roles: BTreeSet<&str> = piece.surfaces.iter().map(String::as_str).collect();
        for (role, f) in [("door", "door"), ("exit", "exit"), ("ladder", "ladder"), ("crawl", "crawl"), ("vines", "vines")] {
            if roles.contains(role) {
                piece.functions.push(f.into());
            }
        }
        pieces.push(piece);
    }
    Ok(Kit { name: manifest.name.clone(), source: manifest.source.glb.clone(), pieces, generated: manifest.generated.clone() })
}

/// Adds a side band under each open edge of a piece's top (its faces facing up, above its base):
/// an edge no other face hangs from. Each faces out, `cs.material` from the top down to the base.
fn close_sides(piece: &mut Piece, cs: &CloseSides) {
    let base = piece.bounds[0][2];
    let up = |t: &[u32; 3]| {
        let [a, b, c] = t.map(|v| piece.verts[v as usize]);
        let n = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
        n > 1e-6 && a[2].min(b[2]).min(c[2]) > base + 5.0
    };
    let key = |v: u32| {
        let p = piece.verts[v as usize];
        [(p[0] * 10.0).round() as i64, (p[1] * 10.0).round() as i64, (p[2] * 10.0).round() as i64]
    };
    let tops: Vec<[u32; 3]> = piece.tris.iter().filter(|t| up(t)).copied().collect();
    let mut count: std::collections::HashMap<([i64; 3], [i64; 3]), usize> = Default::default();
    for t in &tops {
        for k in 0..3 {
            let (a, b) = (key(t[k]), key(t[(k + 1) % 3]));
            *count.entry(if a < b { (a, b) } else { (b, a) }).or_default() += 1;
        }
    }
    // the faces already hanging from an edge: any other face with both its ends (seen from above)
    let xy = |v: u32| [key(v)[0], key(v)[1]];
    let hangs = |a: u32, b: u32| {
        piece.tris.iter().filter(|t| !up(t)).any(|t| {
            let ps: Vec<[i64; 2]> = t.iter().map(|&v| xy(v)).collect();
            ps.contains(&xy(a)) && ps.contains(&xy(b))
        })
    };
    let m = match piece.materials.iter().position(|m| m.texture == format!("kf_{}", cs.material)) {
        Some(i) => i as u32,
        None => {
            piece.materials.push(PieceMaterial { texture: format!("kf_{}", cs.material), tint: [1.0; 3] });
            piece.materials.len() as u32 - 1
        }
    };
    let mut add = vec![];
    for t in &tops {
        for k in 0..3 {
            let (a, b) = (t[k], t[(k + 1) % 3]);
            let (ka, kb) = (key(a), key(b));
            if count[&if ka < kb { (ka, kb) } else { (kb, ka) }] == 1 && !hangs(a, b) {
                add.push((piece.verts[a as usize], piece.verts[b as usize]));
            }
        }
    }
    for (a, b) in add {
        // counter-clockwise top edge a -> b: outside is on its right, which this quad faces
        let len = (b[0] - a[0]).hypot(b[1] - a[1]) / cs.tile;
        let n = [b[1] - a[1], -(b[0] - a[0]), 0.0];
        let l = n[0].hypot(n[1]).max(1e-9);
        let n = [n[0] / l, n[1] / l, 0.0];
        let quad = [[a[0], a[1], base], [b[0], b[1], base], [b[0], b[1], b[2]], [a[0], a[1], a[2]]];
        let uv = [[0.0, 0.0], [len, 0.0], [len, 1.0], [0.0, 1.0]];
        let start = piece.verts.len() as u32;
        for q in quad {
            piece.verts.push(q);
            piece.normals.push(n);
        }
        for (tri, tuv) in [([0, 1, 2], [uv[0], uv[1], uv[2]]), ([0, 2, 3], [uv[0], uv[2], uv[3]])] {
            piece.tris.push(tri.map(|i| start + i));
            piece.uvs.push(tuv);
            piece.mat.push(m);
        }
    }
}

/// `overworld kit-pieces`: the manifest's pieces from the extract under `root`, into `out/pieces.json`.
pub fn export(manifest_path: &Path, root: &Path, out: &Path) -> Result<Kit, String> {
    let m: Manifest = serde_json::from_str(&std::fs::read_to_string(manifest_path).map_err(|e| format!("{}: {e}", manifest_path.display()))?)
        .map_err(|e| format!("{}: {e}", manifest_path.display()))?;
    let glb_path = root.join(&m.source.glb);
    let col_path = root.join(&m.source.collision);
    let glb = std::fs::read(&glb_path).map_err(|e| format!("{}: {e}", glb_path.display()))?;
    let col: Value = serde_json::from_str(&std::fs::read_to_string(&col_path).map_err(|e| format!("{}: {e}", col_path.display()))?)
        .map_err(|e| format!("{}: {e}", col_path.display()))?;
    let kit = cut(&m, &glb, &col)?;
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let p = out.join("pieces.json");
    std::fs::write(&p, serde_json::to_string(&kit).map_err(|e| e.to_string())?).map_err(|e| format!("{}: {e}", p.display()))?;
    Ok(kit)
}

/// The committed Kokiri manifest.
pub fn kokiri_manifest() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/kit/kokiri.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
    }

    #[test]
    fn hull_of_a_square_and_its_middle() {
        let h = hull(vec![[0.0, 0.0], [2.0, 0.0], [1.0, 1.0], [2.0, 2.0], [0.0, 2.0]]);
        assert_eq!(h, vec![[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0]]);
    }

    #[test]
    fn scale_limits() {
        let s = Scale { min: [1.0; 3], max: [1.5; 3], uniform: true };
        assert_eq!(s.clamp([2.0, 0.5, 1.0]), [1.5; 3]);
        let s = Scale { min: [0.5, 0.5, 1.0], max: [4.0, 4.0, 1.0], uniform: false };
        assert_eq!(s.clamp([2.0, 0.1, 3.0]), [2.0, 0.5, 1.0]);
        assert!(Scale::default().locked());
    }

    /// Needs the clone's extract (git-ignored ROM data): skipped without it.
    #[test]
    fn the_kokiri_pieces_come_out_of_the_extracted_scene() {
        let root = repo();
        if !root.join("extracted/scenes/overworld/spot04/spot04.glb").exists() {
            eprintln!("skipping: no extracted spot04");
            return;
        }
        let out = std::env::temp_dir().join(format!("ow_pieces_test_{}", std::process::id()));
        let kit = export(kokiri_manifest(), &root, &out).unwrap();
        assert_eq!(Kit::load(&out).unwrap().pieces.len(), kit.pieces.len());
        let get = |n: &str| kit.get(n).unwrap_or_else(|| panic!("no {n}"));
        // the Blender kit's counts (oot/kit/kokiri/kit.json in the Blender MCP repo), same boxes
        for (n, t, c) in [
            ("link_house", 129, 119),
            ("mido_house", 43, 41),
            ("saria_house", 67, 71),
            ("twins_house", 69, 67),
            ("knowitall_house", 55, 53),
            ("shop", 57, 61),
            ("stump_post", 10, 10),
            ("stone_large", 10, 10),
            ("hedge", 10, 2),
            ("vines", 2, 2),
        ] {
            assert_eq!((get(n).tris.len(), get(n).col_tris.len()), (t, c), "{n}");
        }
        // houses: origin on the door's floor, the door straight ahead (+y)
        for n in ["mido_house", "saria_house", "twins_house", "knowitall_house", "shop"] {
            let d = get(n).door.as_ref().unwrap();
            assert!(d.pos[0].abs() < 1.0 && d.pos[1] > 10.0 && d.pos[2].abs() < 0.5, "{n} door at {:?}", d.pos);
            assert!(get(n).functions.contains(&"door".to_string()));
        }
        // Link's house stands on its foot: the door's up on the porch, 181 above, and its ladder
        // climbs to it
        let link = get("link_house");
        assert!(link.functions.iter().any(|f| f == "ladder"));
        let d = link.door.as_ref().unwrap();
        assert!((d.pos[2] - 181.0).abs() < 2.0 && d.pos[1] > 0.0, "door at {:?}", d.pos);
        // the log tunnel: a round mouth about 220 across, deep in the wall, an exit floor inside
        let log = get("log_tunnel");
        let o = log.opening.as_ref().unwrap();
        assert!((200.0..240.0).contains(&o.width) && (190.0..240.0).contains(&o.height) && o.depth > 800.0, "{o:?}");
        assert!(log.functions.contains(&"exit".to_string()));
        assert!(log.materials.iter().all(|m| m.texture == "kf_log_bark" || m.texture == "kf_tunnel_ring"));
        // the crawlspace: 320 long, walls of wall type 5 at both ends
        let crawl = get("crawlspace");
        let o = crawl.opening.as_ref().unwrap();
        assert!((o.depth - 320.0).abs() < 1.0 && o.height < 45.0, "{o:?}");
        assert_eq!(crawl.functions, vec!["crawl".to_string()]);
        // stones stand on their top, which is flat
        let st = get("stone_small");
        assert!(st.bounds[1][2].abs() < 0.01 && st.bounds[0][2] < -30.0);
        // the hedge: skirts all round now, and only its inner floor collides (Link wades through)
        let hedge = get("hedge");
        assert!(hedge.materials.iter().any(|m| m.texture == "kf_grass_skirt"));
        assert_eq!(hedge.surfaces, vec!["tall_grass".to_string()]);
        // the door shadows come with the houses (decals: see the texture library)
        assert!(get("mido_house").materials.iter().any(|m| m.texture == "kf_shadow"));
        // translucent materials keep their tint; the vines face out of their wall
        assert!(get("waterfall").materials.iter().all(|m| m.tint == [1.0; 3]));
        let v = get("vines");
        assert!(v.verts.iter().all(|p| p[1].abs() < 1.0), "the vine patch lies in the wall's plane");
        let _ = std::fs::remove_dir_all(&out);
    }
}
