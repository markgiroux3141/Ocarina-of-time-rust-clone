//! A theme says how every surface is textured, so the level document never does: the builder
//! classifies each piece of geometry (floor, water, a wall and how tall it is and what's below
//! it, the edge of the world) and the theme maps that to a material, a tiling and overlays.
//! Material names are roles ("ground", "cliff"); `textures` maps them to what an engine or the
//! Blender kit calls them.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The built-in themes, one per region, in the order the editor lists them (`kit::REGIONS`'; Lon
/// Lon Ranch's buildings, interiors, have none); each draws from its region's texture library
/// (`kit::scenes`) and may borrow others' textures by name.
pub const BUILTIN: [&str; 19] = [
    "kokiri",
    "lost_woods",
    "sacred_meadow",
    "hyrule_field",
    "lon_lon",
    "hyrule_castle",
    "kakariko",
    "graveyard",
    "dm_trail",
    "dm_crater",
    "goron_city",
    "zora_river",
    "zora_domain",
    "zora_fountain",
    "lake_hylia",
    "gerudo_valley",
    "gerudo_fortress",
    "wasteland",
    "colossus",
];

const BUILTIN_JSON: [(&str, &str); 19] = [
    ("kokiri", include_str!("../themes/kokiri.json")),
    ("lost_woods", include_str!("../themes/lost_woods.json")),
    ("sacred_meadow", include_str!("../themes/sacred_meadow.json")),
    ("hyrule_field", include_str!("../themes/hyrule_field.json")),
    ("lon_lon", include_str!("../themes/lon_lon.json")),
    ("hyrule_castle", include_str!("../themes/hyrule_castle.json")),
    ("kakariko", include_str!("../themes/kakariko.json")),
    ("graveyard", include_str!("../themes/graveyard.json")),
    ("dm_trail", include_str!("../themes/dm_trail.json")),
    ("dm_crater", include_str!("../themes/dm_crater.json")),
    ("goron_city", include_str!("../themes/goron_city.json")),
    ("zora_river", include_str!("../themes/zora_river.json")),
    ("zora_domain", include_str!("../themes/zora_domain.json")),
    ("zora_fountain", include_str!("../themes/zora_fountain.json")),
    ("lake_hylia", include_str!("../themes/lake_hylia.json")),
    ("gerudo_valley", include_str!("../themes/gerudo_valley.json")),
    ("gerudo_fortress", include_str!("../themes/gerudo_fortress.json")),
    ("wasteland", include_str!("../themes/wasteland.json")),
    ("colossus", include_str!("../themes/colossus.json")),
];

/// Every built-in theme, parsed once.
fn builtins() -> &'static [Theme] {
    static THEMES: std::sync::OnceLock<Vec<Theme>> = std::sync::OnceLock::new();
    THEMES.get_or_init(|| BUILTIN_JSON.iter().map(|(n, j)| serde_json::from_str(j).unwrap_or_else(|e| panic!("built-in {n} theme: {e}"))).collect())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    #[serde(default)]
    pub textures: BTreeMap<String, String>,
    pub floor: Surface,
    pub water: Water,
    pub bank: Surface,
    pub wall_styles: BTreeMap<String, WallStyle>,
    /// First match wins; a rule with no conditions always matches.
    pub wall_rules: Vec<WallRule>,
    pub boundary_style: String,
    pub trees: Trees,
    /// Lighting baked into vertex colours, as the N64 lights geometry by its normals.
    #[serde(default)]
    pub light: Option<Light>,
    /// Breaking up texture repetition.
    #[serde(default)]
    pub variation: Option<Variation>,
    /// Embankment sides and bridge decks (needed by levels with paths).
    #[serde(default)]
    pub paths: Option<PathTheme>,
    /// Dirt paths painted into the ground (needed by levels with dirt lines).
    #[serde(default)]
    pub dirt: Option<Dirt>,
    /// Fence styles by line kind ("fence", "lattice").
    #[serde(default)]
    pub fences: std::collections::BTreeMap<String, FenceStyle>,
    /// Hanging bridges (needed by levels with bridge lines).
    #[serde(default)]
    pub hanging: Option<Hanging>,
    /// Walk-through hedges (needed by levels with hedge lines).
    #[serde(default)]
    pub hedge: Option<HedgeStyle>,
    /// Tunnels (needed by levels with tunnel lines).
    #[serde(default)]
    pub tunnel: Option<TunnelStyle>,
    /// Stairs (paths whose look is "steps"). A theme without its own borrows another's
    /// (`with_others`).
    #[serde(default)]
    pub steps: Option<Steps>,
    /// Freestanding rocks and arches (needed by levels with rock or arch lines).
    #[serde(default)]
    pub rocks: Option<Rocks>,
}

/// Freestanding rocks and arches (`rocks.rs`): their sides are the wall style `style` (a line's own
/// overrides it), their tops `top` (world-projected on a rock, along and across an arch). Their
/// faces push in and out by `lumps`, in lumps about `lump_scale` across, unless a line sets its own.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rocks {
    pub style: String,
    pub top: Surface,
    pub lumps: f64,
    pub lump_scale: f64,
    pub arch: ArchStyle,
}

/// A standalone arch's defaults (`rocks::arch`): `width` across its top, `height` from the ground
/// to the top of its crown, `depth` thick there; its cross-section is a bridge rock's (`lip` down
/// from the top's edge, sides bulging out by `bulge` of the half-width, a rounded underside).
/// Towards its feet it grows `foot` times as thick and half that again as wide.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchStyle {
    pub width: f64,
    pub height: f64,
    pub depth: f64,
    pub lip: f64,
    pub bulge: f64,
    pub foot: f64,
}

/// Stairs as Kakariko's are: a ramp with steps drawn on it. The ramp is `tread`, repeating
/// `across` times across the stair's width (Kakariko's twice, mirrored, so its middle is a seam
/// whatever its width) and once per step, every `step` along its surface, colliding as `surface`;
/// its sides are the wall style `side`, its texture stretched once over the whole stair (from its
/// low end to its high end, from its foot to its top), so the stairs' profile in it runs along the
/// slope. That only fits stairs of Kakariko's slope (1 in 2, within `PROFILE_FIT` degrees); others'
/// sides are the wall style `tiled`, a plain wall repeating along them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Steps {
    pub tread: String,
    pub across: f64,
    pub step: f64,
    pub side: String,
    #[serde(default)]
    pub tiled: Option<String>,
    /// The walls beside stairs cut into higher ground (above the stairs, up to the floor beside).
    #[serde(default)]
    pub cutting: Option<String>,
    pub surface: String,
}

/// How far (degrees) a stair's slope may be from 1 in 2 (26.6°) for the stairs' profile texture on
/// its sides; steeper or shallower stairs get `Steps::tiled`.
pub const PROFILE_FIT: f64 = 5.0;

/// A tunnel (`tunnels.rs`): `width` across its floor and `height` from its floor to its roof (a
/// line's own override them), walls rising to an arch. Its floor is `floor` (world-projected; a
/// `<floor>+dirt` blend is dirt all over), its walls and roof `wall`, `tile_u` along the tunnel
/// and `tile_v` round it per repeat, colliding as `wall_surface`. Inside, the light falls to `dark`
/// of what it is outside, `dark_depth` in from a mouth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelStyle {
    pub width: f64,
    pub height: f64,
    pub floor: Surface,
    pub wall: String,
    pub tile_u: f64,
    pub tile_v: f64,
    pub wall_surface: String,
    pub dark: f64,
    pub dark_depth: f64,
}

/// A walk-through hedge (Kokiri's tall grass): over a closed line of nodes, a top `height` above
/// the ground (`top`, world-projected, `top_tile` per repeat), with skirts (`side`, `side_tile`
/// per repeat along) down to the ground round it. Points every `spacing` inside and along its
/// edge, so it follows the ground. Only drawn: Link wades through it, on a floor of `surface`
/// footsteps a hair above the ground.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HedgeStyle {
    pub top: String,
    pub top_tile: f64,
    pub side: String,
    pub side_tile: f64,
    pub height: f64,
    pub spacing: f64,
    pub surface: String,
}

/// A hanging bridge between two anchors: a deck of planks `width` across (`deck` on top, `under`
/// beneath; the texture `across` times across, one plank every `plank` along), sagging on a
/// catenary by `sag` of its span; ropes (`rope`, a strip `rope_width` tall) `rail` above each edge
/// with uprights every `spacing`; a post (`post`, `post_size`: width, height) at each corner. The
/// deck collides as `surface`, with invisible walls (`side_surface`) `side` tall along both edges.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hanging {
    pub deck: String,
    pub under: String,
    pub width: f64,
    pub across: f64,
    pub plank: f64,
    pub sag: f64,
    pub rope: String,
    pub rope_width: f64,
    pub rail: f64,
    pub spacing: f64,
    pub post: String,
    pub post_size: [f64; 2],
    pub surface: String,
    pub side_surface: String,
    pub side: f64,
}

/// A fence: panels `height` tall standing on the ground along a line, the texture repeating every
/// `tile` (a post in each repeat), drawn double-sided, colliding as `surface` from both sides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FenceStyle {
    pub material: String,
    pub height: f64,
    pub tile: f64,
    pub surface: String,
}

/// A dirt path: the floor under it blends from its own material to `<floor>+dirt`, whose second
/// texture is the floor's with the dirt drawn over it (`textures::composite_name`): the decal's
/// `texture` (a strip, its `columns` middle mirrored across so it tiles), multiplied by `tint`, at
/// `opacity`, `repeats` times per floor tile. The weight is 1 within `width` / 2 - `soft` / 2 of
/// the centre line and falls to 0 at `width` / 2 + `soft` / 2; the edge wanders by up to
/// `wobble`. Floor that's mostly dirt collides as `surface` (dirt footsteps).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dirt {
    pub texture: String,
    pub tint: [f64; 3],
    pub opacity: f64,
    pub repeats: u32,
    pub columns: [u32; 2],
    pub width: f64,
    pub soft: f64,
    pub wobble: f64,
    pub surface: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathTheme {
    /// The wall style of an embankment's sides (a path's `edge` overrides it).
    pub side: String,
    /// The steepest walkable slope, in degrees: steeper paths are reported.
    pub max_slope: f64,
    pub bridge: Bridge,
}

/// A floating deck: `top` (world-projected), then either a `slab` (an underside `under`,
/// world-projected, and edge strips `side` over its `thickness`) or `rock`: a natural arch
/// whose sides bulge out from under the deck's edges and curve round to a rounded underside,
/// deep where it meets the ground and thinner mid-span.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bridge {
    /// "slab" or "rock" (a path's `shape` overrides it).
    #[serde(default = "slab")]
    pub shape: String,
    pub thickness: f64,
    pub top: Surface,
    pub side: Band,
    pub under: Surface,
    #[serde(default)]
    pub rock: Option<Rock>,
}

fn slab() -> String {
    "slab".into()
}

/// A rock arch's cross-section, below the deck: a `lip` down from the deck's edge, the sides
/// bulging out by `bulge` (a fraction of the half-width), then a rounded curve to the bottom,
/// `depth_end` below the deck where it meets the ground and `depth_mid` mid-span. `lumps`
/// pushes the surface in and out (smooth noise about `lump_scale` across). Textured by the wall
/// style `style` (top-anchored caps: the grass lip under the deck's edge, rock below).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rock {
    pub depth_mid: f64,
    pub depth_end: f64,
    pub bulge: f64,
    pub lip: f64,
    pub lumps: f64,
    pub lump_scale: f64,
    pub style: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Band {
    pub material: String,
    pub tile_u: f64,
    #[serde(default = "wall")]
    pub surface: String,
}

/// Breaking up repetition without seams. Along walls the texture's u advances at a speed that
/// drifts by up to `u_speed` (0.2 = 0.8x to 1.2x) over about `u_scale` units, so its streaks
/// stop landing at regular intervals. The baked shade is multiplied by 1 +- `shade` over about
/// `shade_scale` units. Both are smooth noise, so nothing jumps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variation {
    pub u_speed: f64,
    pub u_scale: f64,
    pub shade: f64,
    pub shade_scale: f64,
    #[serde(default)]
    pub seed: u32,
}

/// An ambient colour plus directional lights (0-255 colours; `dir` points towards the light,
/// in level axes: x east, y north, z up). Shade = ambient + sum of max(0, n . dir) * colour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Light {
    pub ambient: [f64; 3],
    pub lights: Vec<DirLight>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirLight {
    pub dir: [f64; 3],
    pub color: [f64; 3],
}

/// A wall texture with a fixed top and bottom (grass edges) and a middle that repeats: v from 0
/// to `bottom` is the bottom cap, from `1 - top` to 1 the top cap, and the rows between repeat
/// as many times as the wall needs. `tile_v` is the texture's full height in units (so caps
/// keep their size on every wall). Walls no taller than `tile_v` show the texture once,
/// stretched. A mirrored middle repeats a whole odd number of times; what's left is folded (a
/// part-repeat up and back down at its foot), so it keeps its size on every wall, or, where
/// triangles count more (three-band walls), the repeats stretch a little to fill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Caps {
    pub tile_v: f64,
    pub bottom: f64,
    pub top: f64,
    /// The texture's height in texels: the middle runs from texel centre to texel centre, so
    /// filtering at a repeat never blends in the rows either side of it.
    #[serde(default)]
    pub rows: u32,
    /// Every other repeat runs upside down, so repeats meet texel for texel (no seam at all).
    /// The repeat count is then odd, so the last one meets the top cap the right way up.
    #[serde(default)]
    pub mirror: bool,
    /// "top": the top cap sits at the wall's top and the middle repeats down from it, cut off
    /// wherever the wall's foot is (no bottom cap). For walls whose height runs to nothing, such as
    /// an embankment's sides: the texture keeps its size and its grass lip follows the top edge.
    #[serde(default)]
    pub anchor: String,
}

impl Caps {
    /// The middle's height in units.
    pub fn unit(&self) -> f64 {
        (1.0 - self.bottom - self.top) * self.tile_v
    }

    /// The middle's v range.
    pub fn middle(&self) -> (f64, f64) {
        let h = if self.rows > 0 { 0.5 / self.rows as f64 } else { 0.0 };
        (self.bottom + h, 1.0 - self.top - h)
    }

    /// How many times the middle repeats on a wall `h` tall (0: show the texture once,
    /// stretched). Mirrored, an odd count: with `fold`, the most that fit whole (the fold takes
    /// the rest); without, the one that stretches or squeezes them least. Not mirrored, the
    /// nearest count, stretched to fit.
    pub fn repeats(&self, h: f64, fold: bool) -> usize {
        if h <= self.tile_v {
            return 0;
        }
        let x = (h - (self.bottom + self.top) * self.tile_v) / self.unit();
        if !self.mirror {
            return (x.round() as usize).max(1);
        }
        let below = ((((x - 1.0) / 2.0).floor().max(0.0)) as usize) * 2 + 1;
        if fold || (x / below as f64).ln() <= ((below + 2) as f64 / x).ln() {
            below
        } else {
            below + 2
        }
    }
}

/// A horizontal-ish surface, textured by world projection (`tile` units per repeat), so it's
/// seamless across faces, regions and slopes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Surface {
    pub material: String,
    pub tile: f64,
    pub surface: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Water {
    pub material: String,
    pub tile: f64,
    pub surface: String,
    pub bed: Surface,
}

/// A wall: u runs along it (`tile_u` units per repeat, continuous round corners, snapped to
/// whole repeats round closed loops), v is stretched over its height as one band, or repeated
/// once per `band` units on walls taller than that.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallStyle {
    pub material: String,
    pub tile_u: f64,
    #[serde(default)]
    pub band: Option<f64>,
    #[serde(default)]
    pub caps: Option<Caps>,
    #[serde(default = "wall")]
    pub surface: String,
    /// Along the foot (not under water): Kokiri's grass tufts.
    #[serde(default)]
    pub skirt: Option<Overlay>,
    /// Hanging from the top, on walls over `fringe_min` tall: roots.
    #[serde(default)]
    pub fringe: Option<Overlay>,
    #[serde(default = "fringe_min")]
    pub fringe_min: f64,
}

fn wall() -> String {
    "wall".into()
}

fn fringe_min() -> f64 {
    150.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Overlay {
    pub material: String,
    pub tile: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallRule {
    #[serde(default)]
    pub max_height: Option<f64>,
    #[serde(default)]
    pub over_water: Option<bool>,
    pub style: String,
}

/// The boundary forest: trunks standing on the rim, foliage over them and a little in front.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trees {
    pub trunks: f64,
    pub trunk_material: String,
    pub trunk_tile: f64,
    pub foliage: f64,
    pub foliage_material: String,
    pub foliage_tile: f64,
    pub overlap: f64,
    pub foliage_in: f64,
    pub surface: String,
}

impl Theme {
    pub fn load(path: &str) -> Result<Theme, String> {
        let s = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        serde_json::from_str(&s).map_err(|e| format!("{path}: {e}"))
    }

    pub fn kokiri() -> Theme {
        Theme::builtin("kokiri").expect("built-in kokiri theme")
    }

    pub fn kakariko() -> Theme {
        Theme::builtin("kakariko").expect("built-in kakariko theme")
    }

    /// A built-in theme by name (`BUILTIN`).
    pub fn builtin(name: &str) -> Option<Theme> {
        BUILTIN.iter().position(|n| *n == name).map(|i| builtins()[i].clone())
    }

    /// The theme a document builds with: its `settings.theme` (or `main`, a theme file's, if
    /// given), with every built-in theme's styles reachable as `<theme>:<style>` (`with_others`).
    pub fn for_doc(doc: &crate::Doc, main: Option<Theme>) -> Result<Theme, String> {
        let main = match main {
            Some(t) => t,
            None => Theme::builtin(&doc.settings.theme)
                .ok_or_else(|| format!("no theme {:?} (themes: {})", doc.settings.theme, BUILTIN.join(", ")))?,
        };
        Ok(main.with_others(builtins()))
    }

    /// This theme, plus every theme's wall styles under `<theme>:<style>` (its own too, so a style
    /// pinned to the level's own theme still resolves). Their materials become `<theme>:<role>`,
    /// whose textures are that theme's, so a level can mix themes while each style keeps its look.
    pub fn with_others(mut self, others: &[Theme]) -> Theme {
        let own = self.clone();
        for o in std::iter::once(&own).chain(others.iter().filter(|o| o.name != own.name)) {
            let q = |role: &str| format!("{}:{role}", o.name);
            for (k, ws) in &o.wall_styles {
                let mut ws = ws.clone();
                for m in [Some(&mut ws.material), ws.skirt.as_mut().map(|x| &mut x.material), ws.fringe.as_mut().map(|x| &mut x.material)].into_iter().flatten() {
                    self.textures.insert(q(m), o.texture_name(m));
                    *m = q(m);
                }
                self.wall_styles.insert(q(k), ws);
            }
        }
        // stairs: a theme without its own borrows Kakariko's (else the first other theme's)
        if self.steps.is_none() {
            let with_steps = |o: &'_ Theme| o.steps.is_some();
            let from = others.iter().find(|o| o.name == "kakariko" && with_steps(o)).or_else(|| others.iter().find(|o| with_steps(o)));
            if let Some((o, st)) = from.and_then(|o| Some((o, o.steps.as_ref()?))) {
                let tread = format!("{}:{}", o.name, st.tread);
                self.textures.insert(tread.clone(), o.texture_name(&st.tread));
                let q = |t: &String| format!("{}:{t}", o.name);
                let (side, tiled, cutting) = (q(&st.side), st.tiled.as_ref().map(q), st.cutting.as_ref().map(q));
                self.steps = Some(Steps { tread, side, tiled, cutting, ..st.clone() });
            }
        }
        self
    }

    /// The wall styles a document can name, the theme's own first (plain), then the other themes'
    /// (`<theme>:<style>`), each group sorted.
    pub fn style_names(&self) -> Vec<String> {
        let (mut plain, mut pinned): (Vec<String>, Vec<String>) = self.wall_styles.keys().cloned().partition(|k| !k.contains(':'));
        pinned.retain(|k| !k.starts_with(&format!("{}:", self.name)));
        plain.sort();
        pinned.sort();
        plain.into_iter().chain(pinned).collect()
    }

    /// The style of a wall `height` tall, over water or not.
    /// The texture a material role uses. A role `<material>~mid` is the middle rows of the
    /// texture that capped walls of that material use (`Detail::walls3`).
    pub fn texture_name(&self, role: &str) -> String {
        // a blend (the ground under a dirt path) draws its base as texture 0
        if let Some((base, _)) = role.split_once('+') {
            return self.texture_name(base);
        }
        if let Some(base) = role.strip_suffix("~mid") {
            let name = self.texture_name(base);
            let caps = self.wall_styles.values().filter(|w| w.material == base).find_map(|w| w.caps.as_ref().filter(|c| c.rows > 0));
            return match caps {
                Some(c) => {
                    let rows = c.rows as f64;
                    let (r0, r1) = ((c.top * rows).round() as u32, c.rows - (c.bottom * rows).round() as u32);
                    crate::textures::derived_name(&name, r0, r1, c.mirror)
                }
                None => name,
            };
        }
        self.textures.get(role).cloned().unwrap_or_else(|| role.to_string())
    }

    /// A blend role's second texture (`<base>+dirt`: the base with the dirt drawn over it), blended
    /// in by each vertex's weight. None for every other role.
    pub fn overlay_texture(&self, role: &str) -> Option<String> {
        let (base, over) = role.split_once('+')?;
        match over {
            "dirt" => {
                let d = self.dirt.as_ref()?;
                Some(crate::textures::composite_name(&self.texture_name(base), &self.texture_name(&d.texture), d.tint, d.opacity, d.repeats, d.columns))
            }
            _ => None,
        }
    }

    pub fn wall_style(&self, height: f64, over_water: bool) -> &str {
        for r in &self.wall_rules {
            if r.max_height.is_some_and(|m| height > m) {
                continue;
            }
            if r.over_water.is_some_and(|w| w != over_water) {
                continue;
            }
            return &r.style;
        }
        "cliff"
    }
}
