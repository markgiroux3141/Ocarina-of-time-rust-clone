//! The level document: what an editor saves. Everything else is derived from it.
//!
//! ```json
//! { "name": "sketch",
//!   "outline": { "nodes": [[x, y], ...], "z": 0 },
//!   "regions": [ { "name": "pond", "nodes": [[x, y], [x, y, 1], ...], "z": -100,
//!                  "kind": "water", "surface": -20 } ],
//!   "boundary": { "cliff_min": 280, "rise_slope": 0.25 } }
//! ```
//!
//! Nodes are control points: edges between them are smooth curves (centripetal Catmull-Rom),
//! except at a node marked sharp (a third value of 1). A region's node that lands on another
//! loop's node (within `settings.weld`) *is* that node, so a region drawn against the outline
//! shares its edge: its walls and the boundary meet without gaps.
//! Heights are absolute; x east, y north, z up.

use crate::geom::Sampling;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Doc {
    #[serde(default)]
    pub name: String,
    pub outline: Outline,
    #[serde(default)]
    pub regions: Vec<Region>,
    #[serde(default)]
    pub paths: Vec<Path>,
    #[serde(default)]
    pub boundary: BoundaryDesign,
    #[serde(default)]
    pub settings: Settings,
    /// Painted hills and hollows (`terrain.rs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terrain: Option<crate::terrain::Terrain>,
    /// Kit pieces placed in the level: houses, stumps, stones... (`props.rs`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub props: Vec<Prop>,
    /// Things drawn along lines of nodes: dirt paths, fences, hanging bridges (`lines.rs`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<Line>,
}

/// A line of nodes `[x, y]` (or `[x, y, z]` for a bridge's anchors) that the builder draws
/// something along, by `kind`:
/// - "dirt": a dirt path painted into the ground (smooth curve through the nodes, `width` across);
/// - "fence" and "lattice": fences, straight between nodes (`closed` joins the last to the first);
/// - "bridge": a hanging bridge between its two anchors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Line {
    #[serde(default)]
    pub name: String,
    pub kind: String,
    pub nodes: Vec<Vec<f64>>,
    /// Else the theme's for the kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub closed: bool,
}

/// A kit piece (`pieces.rs`) placed in the level. Its origin (a house's door floor, a stone's
/// top, a stump's base) stands at `at`, on the ground there unless `z` sets its height; it faces
/// `yaw` degrees counter-clockwise from north (0: as it faced in its source, +y).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Prop {
    pub piece: String,
    pub at: [f64; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub z: Option<f64>,
    #[serde(default)]
    pub yaw: f64,
    /// Along the piece's own x, y and z, within its limits (`Piece::scale`).
    #[serde(default = "one3")]
    pub scale: [f64; 3],
}

fn one3() -> [f64; 3] {
    [1.0; 3]
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Outline {
    pub nodes: Vec<Vec<f64>>,
    #[serde(default)]
    pub z: f64,
    /// Bumps on the ground.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub noise: Option<Noise>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Region {
    #[serde(default)]
    pub name: String,
    pub nodes: Vec<Vec<f64>>,
    #[serde(default)]
    pub z: f64,
    /// "floor" or "water" (z is then the bed).
    #[serde(default = "floor")]
    pub kind: String,
    /// Water: the surface height.
    #[serde(default)]
    pub surface: Option<f64>,
    /// Wall style for this region's own walls (where it's the higher side), overriding the
    /// theme's rules: e.g. "vines" for a climbable face.
    #[serde(default)]
    pub edge: Option<String>,
    /// Bumps on this region's floor (a pond's bed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub noise: Option<Noise>,
}

/// Bumps on a floor: smooth noise of up to `amplitude` up or down, with features about `scale`
/// across, fading to nothing within `edge` of the floor's edges. The edges stay at the floor's
/// height, so walls, the rim, path landings and bridge ends are exactly where they'd be
/// without it; under an attached path the path's surface wins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Noise {
    pub amplitude: f64,
    pub scale: f64,
    pub edge: f64,
    /// Varies the pattern (each region's differs anyway).
    pub seed: u32,
}

impl Default for Noise {
    fn default() -> Self {
        Noise { amplitude: 20.0, scale: 600.0, edge: 200.0, seed: 0 }
    }
}

/// A ramp, embankment or bridge along a line of nodes `[x, y]`, `[x, y, z]` or
/// `[x, y, z, width]` (z may be null). An end with no z takes the floor's height there; a
/// node between with none is interpolated. See `paths.rs` for landing and runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Path {
    #[serde(default)]
    pub name: String,
    pub nodes: Vec<Vec<Option<f64>>>,
    #[serde(default = "path_width")]
    pub width: f64,
    /// "attached" (an embankment, solid down to the ground) or "floating" (a bridge deck).
    #[serde(default = "attached")]
    pub mode: String,
    /// Per segment between nodes, overriding `mode`.
    #[serde(default)]
    pub modes: Vec<String>,
    /// Wall style for an embankment's sides (else the theme's).
    #[serde(default)]
    pub edge: Option<String>,
    /// Floating runs' shape: "rock" (a natural arch) or "slab" (else the theme's).
    #[serde(default)]
    pub shape: Option<String>,
}

fn path_width() -> f64 {
    160.0
}

fn attached() -> String {
    "attached".into()
}

fn floor() -> String {
    "floor".into()
}

/// How the edge of the world behaves. The rim (where the trees stand) sits `cliff_min` +
/// `bank_rise` above the outline's floor all round, and rises over any floor within `reach`
/// of the edge, no faster than `rise_slope` per unit along the edge: never a step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BoundaryDesign {
    pub cliff_min: f64,
    pub bank: f64,
    pub bank_rise: f64,
    pub rise_slope: f64,
    pub reach: f64,
    pub panel_tol: f64,
}

impl Default for BoundaryDesign {
    fn default() -> Self {
        // Kokiri: edge cliffs 200 to 460 tall, a bank about 220 deep rising about 80
        BoundaryDesign { cliff_min: 280.0, bank: 220.0, bank_rise: 80.0, rise_slope: 0.25, reach: 300.0, panel_tol: 90.0 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Curves are sampled about this often.
    pub sample: f64,
    /// Interior points in floors, this far apart (room for hills later).
    pub steiner: f64,
    /// Nodes closer than this are the same node.
    pub weld: f64,
    /// Varies the theme's noise (texture speed, shade) from level to level.
    pub seed: u32,
    /// Mesh resolution: "high" (curves every `sample`, the default), "medium" or "low" (curves
    /// sampled by how much they bend, coarser floors and bridges). See `Detail`.
    pub detail: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { sample: 60.0, steiner: 250.0, weld: 1.0, seed: 0, detail: "high".into() }
    }
}

/// What a detail level means for the builder.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Detail {
    /// The outline's and regions' curves.
    pub curves: Sampling,
    /// Paths' centre lines: their stations, so also embankment sides and bridge arches.
    pub paths: Sampling,
    /// Interior points in flat floors, this far apart.
    pub steiner: f64,
    /// Painted terrain's floor points: its `detail` times this.
    pub terrain: f64,
    /// Bumpy floors' points: the bumps' scale over this.
    pub bumps: f64,
    /// Capped walls in three bands (bottom cap, middle, top cap), the middle one band with its
    /// own texture (the middle rows, `textures::derived_name`) repeating as often as the height
    /// needs. Otherwise each middle repeat is a band of its own.
    pub walls3: bool,
}

impl Settings {
    pub fn detail(&self) -> Result<Detail, String> {
        let s = self.sample.max(1.0);
        Ok(match self.detail.as_str() {
            "high" | "" => Detail { curves: Sampling::Every(s), paths: Sampling::Every(s * 0.5), steiner: self.steiner, terrain: 1.0, bumps: 4.0, walls3: false },
            "medium" => Detail {
                curves: Sampling::Within { tol: 2.5, max: s * 4.0 },
                paths: Sampling::Within { tol: 2.5, max: s * 2.0 },
                steiner: self.steiner * 1.6,
                terrain: 1.5,
                bumps: 3.0,
                walls3: true,
            },
            "low" => Detail {
                curves: Sampling::Within { tol: 6.0, max: s * 8.0 },
                paths: Sampling::Within { tol: 5.0, max: s * 4.0 },
                steiner: self.steiner * 2.4,
                terrain: 2.0,
                bumps: 2.0,
                walls3: true,
            },
            d => return Err(format!("unknown detail {d:?} (high, medium or low)")),
        })
    }
}

pub fn node_xy(n: &[f64]) -> [f64; 2] {
    [n[0], n[1]]
}

pub fn node_sharp(n: &[f64]) -> bool {
    n.len() > 2 && n[2] != 0.0
}
