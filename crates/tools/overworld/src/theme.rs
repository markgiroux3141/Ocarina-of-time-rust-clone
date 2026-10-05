//! A theme says how every surface is textured, so the level document never does: the builder
//! classifies each piece of geometry (floor, water, a wall and how tall it is and what's below
//! it, the edge of the world) and the theme maps that to a material, a tiling and overlays.
//! Material names are roles ("ground", "cliff"); `textures` maps them to what an engine or the
//! Blender kit calls them.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
/// stretched.
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

    /// How many times the middle repeats on a wall `h` tall (0: show the texture once, stretched).
    pub fn repeats(&self, h: f64) -> usize {
        if h <= self.tile_v {
            0
        } else {
            let x = (h - (self.bottom + self.top) * self.tile_v) / self.unit();
            if self.mirror {
                ((((x - 1.0) / 2.0).round().max(0.0)) as usize) * 2 + 1
            } else {
                (x.round() as usize).max(1)
            }
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
        serde_json::from_str(include_str!("../themes/kokiri.json")).expect("built-in kokiri theme")
    }

    /// The style of a wall `height` tall, over water or not.
    /// The texture a material role uses. A role `<material>~mid` is the middle rows of the
    /// texture that capped walls of that material use (`Detail::walls3`).
    pub fn texture_name(&self, role: &str) -> String {
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
