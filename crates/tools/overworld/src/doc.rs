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
/// - "bridge": a hanging bridge between its two anchors;
/// - "hedge": a walk-through hedge over the closed shape of its nodes (always closed);
/// - "tunnel": a tunnel Link walks through, from a wall to a wall (`tunnels.rs`): through a ridge
///   or under a plateau, or from one area's edge of the world to another's. A node's third value
///   sets the floor's height there.
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
    /// A tunnel's height, floor to roof (else the theme's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
    /// A tunnel's rough walls: they and its roof push in and out by up to `amplitude`, in features
    /// about `scale` across, smooth within `edge` of its mouths; the floor rolls a little.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub noise: Option<Noise>,
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
    /// Whether bumps level out under it (`props::pads`); else by its kind (houses, stumps and
    /// hedges do). Only on the ground: not with `z`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<bool>,
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
    /// Per edge (edge k runs from node k to the next): what lies beyond it, instead of the forest
    /// (null). See `beyond.rs`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub beyond: Vec<Option<Beyond>>,
}

/// What lies beyond an outline edge, instead of the forest: ground climbing from the edge to a
/// crest at `z`, as `profile` says (built outward from the edge: a cliff, a slope, terraces, a
/// stack...), and nothing past the crest. `beyond.rs`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Beyond {
    /// The crest's height (the ground's: a skyline wall rises above it).
    pub z: f64,
    pub profile: Profile,
    /// How rough the ground is above the walls at its foot: its height there varies by up to this
    /// fraction (0: smooth), in lumps about `ROUGH_SCALE` across, so its crest rises and falls as a
    /// mountain's does. A skyline wall's height varies by as much.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub rough: f64,
    /// A wall standing on the far edge, rising above the ground there (Kakariko's mossy wall: its
    /// cut-out top is the skyline).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skyline: Option<Skyline>,
}

impl Beyond {
    pub fn new(z: f64, profile: Profile) -> Beyond {
        Beyond { z, profile, rough: 0.0, skyline: None }
    }

    /// The highest it reaches, a skyline wall's top included (before roughness).
    pub fn top(&self) -> f64 {
        self.z + self.skyline.as_ref().map_or(0.0, |s| s.height)
    }
}

/// How far apart a rough band's lumps are (`Beyond::rough`), about: spot01's ridge rises and falls
/// about this often.
pub const ROUGH_SCALE: f64 = 900.0;

/// A wall on a band's far edge (`Beyond::skyline`): its style is stretched once over its height.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Skyline {
    pub style: String,
    /// How high it rises above the ground at its foot (less towards an end the forest is beside).
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Region {
    #[serde(default)]
    pub name: String,
    pub nodes: Vec<Vec<f64>>,
    #[serde(default)]
    pub z: f64,
    /// "floor", "water" (z is then the bed) or "pit" (a drop into the void: z is how far down
    /// its walls go, where Link voids out).
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
    /// How its edges come down to the floors beside it (or up to them, sunk): cliffs if none.
    /// See `profiles.rs`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<Profile>,
    /// Per edge (edge k runs from node k to the next), overriding `profile`; null keeps it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub profiles: Vec<Option<Profile>>,
}

impl Region {
    /// Edge k's profile: its own, else the region's (None: a cliff).
    pub fn edge_profile(&self, k: usize) -> Option<&Profile> {
        self.profiles.get(k).and_then(|p| p.as_ref()).or(self.profile.as_ref()).filter(|p| **p != Profile::Cliff)
    }
}

/// How a region's edge meets the floor beside it, built inward from the edge, so the region's
/// footprint stays as drawn (`profiles.rs`). A raised region comes down to the floor beside it; a
/// sunken one (a pond's bed too) goes up to it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Profile {
    /// A wall, as without a profile.
    Cliff,
    /// A slope from the floor beside it at the edge to the region's height, at most `angle`
    /// degrees steep. `round` (0 to 1) rounds its crest and its foot (1: an S curve), keeping the
    /// steepest part at `angle`, so the slope is up to half as long again.
    Slope {
        #[serde(default = "slope_angle")]
        angle: f64,
        #[serde(default, skip_serializing_if = "is_zero")]
        round: f64,
    },
    /// Steps from the floor beside it to the region's height: `steps` risers (the last up onto the
    /// region's own floor), `depth` deep each, `rise` tall (else the drop shared evenly). Steps
    /// that would go below the floor beside are left out; the height they don't cover is a cliff
    /// at the edge.
    Terraces {
        #[serde(default = "terrace_steps")]
        steps: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rise: Option<f64>,
        #[serde(default = "terrace_depth")]
        depth: f64,
    },
    /// A cliff whose top juts out `depth` over its foot: the wall is undercut into the higher side,
    /// its lip curving back out to the edge at the top, and the floor below runs in under it.
    Overhang {
        #[serde(default = "overhang_depth")]
        depth: f64,
    },
    /// A cliff whose face pushes in and out by up to `amplitude`, in lumps about `scale` across;
    /// its top and foot stay on the edge.
    Ragged {
        #[serde(default = "ragged_amplitude")]
        amplitude: f64,
        #[serde(default = "ragged_scale")]
        scale: f64,
        #[serde(default, skip_serializing_if = "is_zero_u32")]
        seed: u32,
    },
    /// Parts from the foot in, each a wall or a slope with its own height and look: a cliff with a
    /// grass slope above it, a brick wall under a rock face (Kakariko's edges). Walls past the edge
    /// are step lines cut into the floor, as terraces' risers are; slopes are the floor's height
    /// between them. See `Part`.
    Stack { parts: Vec<Part> },
}

/// One part of a stacked profile (`Profile::Stack`), listed from the foot in.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Part {
    /// "wall" (upright) or "slope".
    pub kind: String,
    /// How much of the drop it climbs. Blank: an even share of what the parts with one leave. If
    /// those climb more than the drop, the stack doesn't fit and that edge is a cliff (a short side
    /// of a stacked ridge); if every part has one and they climb less, the rest is a cliff at the
    /// edge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rise: Option<f64>,
    /// A slope's angle, in degrees.
    #[serde(default = "slope_angle")]
    pub angle: f64,
    /// A slope's rounding (0 to 1), as `Profile::Slope`'s.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub round: f64,
    /// A wall style, the theme's or another's (`kakariko:rock`): a wall's look, or a slope textured
    /// as a wall (along the edge and up the face). Blank: a wall by the theme's rules; a slope the
    /// floor, or the theme's cliff when it's steeper than `STEEP`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
}

/// Slopes steeper than this (degrees) with no style of their own are textured as the theme's
/// cliff, not the floor.
pub const STEEP: f64 = 60.0;

impl Part {
    pub fn wall(rise: Option<f64>, style: Option<&str>) -> Part {
        Part { kind: "wall".into(), rise, angle: slope_angle(), round: 0.0, style: style.map(String::from) }
    }

    pub fn slope(angle: f64, rise: Option<f64>, style: Option<&str>) -> Part {
        Part { kind: "slope".into(), rise, angle, round: 0.0, style: style.map(String::from) }
    }
}

/// One of Kakariko's edges, to start from: what the editor offers before a profile's parts.
pub struct StackLook {
    pub name: &'static str,
    /// Where it is in Kakariko, and what it's made of.
    pub tip: &'static str,
    /// How high it reaches above the village floor there (beyond the outline, its top).
    pub crest: f64,
    pub parts: Vec<Part>,
    /// Beyond the outline: its roughness (`Beyond::rough`).
    pub rough: f64,
    /// Beyond the outline: a wall standing on the far edge, this style, up to `crest`; the ground
    /// in front of it climbs only as its parts do.
    pub skyline: Option<&'static str>,
}

impl StackLook {
    pub fn profile(&self) -> Profile {
        Profile::Stack { parts: self.parts.clone() }
    }

    /// As a region's edge profile, which has no far edge for a skyline wall to stand on: the
    /// wall is the stack's last part.
    pub fn region_profile(&self) -> Profile {
        let mut parts = self.parts.clone();
        if let Some(style) = self.skyline {
            parts.push(Part::wall(None, Some(style)));
        }
        Profile::Stack { parts }
    }

    /// What lies beyond an outline edge in this look, with the level's ground at `ground`.
    pub fn beyond(&self, ground: f64) -> Beyond {
        let parts: f64 = self.parts.iter().filter_map(|p| p.rise).sum();
        match self.skyline {
            Some(style) => Beyond { z: ground + parts, profile: self.profile(), rough: self.rough, skyline: Some(Skyline { style: style.into(), height: (self.crest - parts).max(0.0) }) },
            None => Beyond { z: ground + self.crest, profile: self.profile(), rough: self.rough, skyline: None },
        }
    }

    /// Whether `b` is this look (at any height).
    pub fn is(&self, b: &Beyond) -> bool {
        b.profile == self.profile() && (b.rough - self.rough).abs() < 1e-9 && b.skyline.as_ref().map(|s| s.style.as_str()) == self.skyline
    }
}

/// Kakariko's edges, measured from spot01 (see docs/OVERWORLD-EDITOR-IDEAS.md, section 7), in
/// Kakariko's own styles whatever the level's theme. The first is the default.
pub fn stack_looks() -> Vec<StackLook> {
    let round = |mut p: Part, r: f64| {
        p.round = r;
        p
    };
    vec![
        StackLook {
            name: "Grass slope",
            tip: "Kakariko's west wing: a cliff 330 tall, then a grass slope at 36° up to about 940",
            crest: 940.0,
            parts: vec![Part::wall(Some(330.0), Some("kakariko:cliff")), round(Part::slope(36.0, None, None), 0.5)],
            rough: 0.12,
            skyline: None,
        },
        StackLook {
            name: "Rock face",
            tip: "Under Death Mountain Trail: a brick wall 320 tall, a rock wall 160 behind it, then a mountain side of rock up to a ridge about 1160 high that rises and falls",
            crest: 1160.0,
            parts: vec![
                Part::wall(Some(320.0), Some("kakariko:brick")),
                Part::wall(Some(160.0), Some("kakariko:rock")),
                Part::slope(65.0, Some(200.0), Some("kakariko:rock")),
                Part::slope(42.0, None, Some("kakariko:rock")),
            ],
            rough: 0.4,
            skyline: None,
        },
        StackLook {
            name: "Mossy wall",
            tip: "Kakariko's south edge: a cliff 330 tall, a strip of grass, then a mossy wall straight up to about 1080, its top rising and falling",
            crest: 1080.0,
            parts: vec![Part::wall(Some(330.0), Some("kakariko:cliff")), Part::slope(36.0, Some(75.0), None)],
            rough: 0.45,
            skyline: Some("kakariko:mountain"),
        },
    ]
}

pub fn overhang_depth() -> f64 {
    60.0
}

pub fn ragged_amplitude() -> f64 {
    18.0
}

pub fn ragged_scale() -> f64 {
    160.0
}

fn is_zero_u32(x: &u32) -> bool {
    *x == 0
}

pub fn slope_angle() -> f64 {
    30.0
}

pub fn terrace_steps() -> u32 {
    3
}

pub fn terrace_depth() -> f64 {
    120.0
}

fn is_zero(x: &f64) -> bool {
    *x == 0.0
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
    /// How its attached runs look: none, the ground; "steps", stairs as Kakariko's are (a ramp with
    /// steps drawn on it, its sides the stairs' profile: the theme's `steps`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look: Option<String>,
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
    /// How the outline's, regions' and paths' edges run between their nodes: "smooth" (curves,
    /// the default), "faceted" (the same curves in a few long straight pieces, low-poly like
    /// the game's own) or "hard" (straight from node to node, every node a corner).
    pub edges: String,
    /// How capped walls (the cliffs) are textured: "tiled" (the default: the caps keep their size
    /// and the middle repeats), "stretched_middle" (the caps keep their size, the middle is
    /// stretched once over the rest) or "stretched" (the texture once over the wall's height, as
    /// wide as that keeps its shape: Kokiri's own way, blurrier on tall walls). See `WallTexture`.
    pub wall_texture: String,
    /// The level's theme ("kokiri", the default, or "kakariko"): how every style the document
    /// doesn't pin to a theme looks. `Theme::for_doc`.
    #[serde(skip_serializing_if = "is_kokiri")]
    pub theme: String,
}

fn is_kokiri(t: &String) -> bool {
    t == "kokiri"
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            sample: 60.0,
            steiner: 250.0,
            weld: 1.0,
            seed: 0,
            detail: "high".into(),
            edges: "smooth".into(),
            wall_texture: "tiled".into(),
            theme: "kokiri".into(),
        }
    }
}

/// Faceted edges stay within this of the smooth curve: about 16 pieces round a circle 1000 across.
pub const FACET_TOL: f64 = 20.0;

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
    /// How capped walls are textured (`Settings::wall_texture`).
    pub walls: WallTexture,
}

/// How capped walls are textured (`Settings::wall_texture`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WallTexture {
    /// The caps at their size, the middle repeating (mirrored, folded to fit: `Caps`).
    Tiled,
    /// The caps at their size, the middle once, stretched over the rest.
    StretchedMiddle,
    /// The texture once over the wall's height, growing across with it.
    Stretched,
}

impl Settings {
    pub fn detail(&self) -> Result<Detail, String> {
        let s = self.sample.max(1.0);
        let mut d = match self.detail.as_str() {
            "high" | "" => Detail { curves: Sampling::Every(s), paths: Sampling::Every(s * 0.5), steiner: self.steiner, terrain: 1.0, bumps: 4.0, walls3: false, walls: WallTexture::Tiled },
            "medium" => Detail {
                curves: Sampling::Within { tol: 2.5, max: s * 4.0 },
                paths: Sampling::Within { tol: 2.5, max: s * 2.0 },
                steiner: self.steiner * 1.6,
                terrain: 1.5,
                bumps: 3.0,
                walls3: true,
                walls: WallTexture::Tiled,
            },
            "low" => Detail {
                curves: Sampling::Within { tol: 6.0, max: s * 8.0 },
                paths: Sampling::Within { tol: 5.0, max: s * 4.0 },
                steiner: self.steiner * 2.4,
                terrain: 2.0,
                bumps: 2.0,
                walls3: true,
                walls: WallTexture::Tiled,
            },
            d => return Err(format!("unknown detail {d:?} (high, medium or low)")),
        };
        // the detail's longest piece, so straight stretches keep their points for terrain and walls
        let longest = |c: Sampling| match c {
            Sampling::Every(m) | Sampling::Within { max: m, .. } | Sampling::Facets { max: m, .. } | Sampling::Straight { max: m } => m,
        };
        let edged = |c: Sampling| match self.edges.as_str() {
            "smooth" | "" => Ok(c),
            "faceted" => Ok(Sampling::Facets { tol: match c {
                Sampling::Within { tol, .. } => tol.max(FACET_TOL),
                _ => FACET_TOL,
            }, max: longest(c) }),
            "hard" => Ok(Sampling::Straight { max: longest(c) }),
            e => Err(format!("unknown edges {e:?} (smooth, faceted or hard)")),
        };
        d.curves = edged(d.curves)?;
        d.paths = edged(d.paths)?;
        d.walls = match self.wall_texture.as_str() {
            "tiled" | "" => WallTexture::Tiled,
            "stretched_middle" => WallTexture::StretchedMiddle,
            "stretched" => WallTexture::Stretched,
            w => return Err(format!("unknown wall_texture {w:?} (tiled, stretched_middle or stretched)")),
        };
        Ok(d)
    }
}

pub fn node_xy(n: &[f64]) -> [f64; 2] {
    [n[0], n[1]]
}

pub fn node_sharp(n: &[f64]) -> bool {
    n.len() > 2 && n[2] != 0.0
}
