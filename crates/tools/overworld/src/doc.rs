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
    /// How the level plays in the game: the region it starts from, and changes to it.
    #[serde(default, skip_serializing_if = "Game::is_default")]
    pub game: Game,
    /// The game's actors placed in it: pots, bushes, enemies, characters, switches...
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actors: Vec<Actor>,
    /// The storey the document's own regions, paths, lines, props, actors and terrain (and the
    /// outline's height, bumps and ceiling) are, when it has others (`storeys`): its name...
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub storey: String,
    /// ...and the slab under its floor (`SLAB` if none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slab: Option<f64>,
    /// The level's other storeys (docs/EDITOR-ROOFS-STOREYS-DUNGEONS.md, slice 1), each a plan of
    /// its own at its own height inside the same outline. Stacked by height, not by where they're
    /// kept: any of them can be swapped into the document's own fields (`swap_storey`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub storeys: Vec<Storey>,
    /// Its own texts: what the game's text ids say while the level is played (the actors' lines
    /// edited in the editor's Dialogue section), and new ids of its own. By id, one each.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub texts: Vec<Text>,
}

/// A text of the level's (docs/EDITOR-ACTORS.md, "Texts"; ADR 0079): the game's message of that
/// id while the level is played (the real scenes keep theirs), or a new one from 0x8000. The text
/// is in the editor's readable form (`oot_game::message_text`: `{COLOR:RED}`, `{NAME}`, `[A]`, a
/// newline a line), the box the game's `TEXTBOX_TYPE_*` and `TEXTBOX_POS_*`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Text {
    pub id: u16,
    #[serde(default)]
    pub box_type: u8,
    #[serde(default)]
    pub box_pos: u8,
    pub text: String,
}

/// One of a level's storeys (`Doc::storeys`): what the document's own fields hold for the storey
/// that's in them. A storey with one above it is enclosed, its ceiling the underside of the slab
/// under the next storey; the top storey's ceiling is its own (`ceiling`: none, open to the sky).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Storey {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// The slab under its floor (`SLAB` if none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slab: Option<f64>,
    /// Its floor's height: the outline's `z` while it's in the document's own fields.
    #[serde(default)]
    pub z: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub noise: Option<Noise>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ceiling: Option<Ceiling>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regions: Vec<Region>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<Path>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<Line>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub props: Vec<Prop>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actors: Vec<Actor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terrain: Option<crate::terrain::Terrain>,
}

/// How thick a storey's floor is (the slab between it and the ceiling of the storey below), unless
/// it says.
pub const SLAB: f64 = 60.0;

/// One storey of a level's stack (`Doc::stack`): where it's kept (None: the document's own fields,
/// else `storeys[i]`), its floor's height and its slab.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stacked {
    pub at: Option<usize>,
    pub z: f64,
    pub slab: f64,
}

impl Doc {
    /// Swaps storey `i` (`storeys[i]`) with the one in the document's own fields: the editor edits
    /// a storey this way, with every tool as it is. Swapping again swaps it back.
    pub fn swap_storey(&mut self, i: usize) {
        let s = &mut self.storeys[i];
        std::mem::swap(&mut self.storey, &mut s.name);
        std::mem::swap(&mut self.slab, &mut s.slab);
        std::mem::swap(&mut self.outline.z, &mut s.z);
        std::mem::swap(&mut self.outline.noise, &mut s.noise);
        std::mem::swap(&mut self.outline.ceiling, &mut s.ceiling);
        std::mem::swap(&mut self.regions, &mut s.regions);
        std::mem::swap(&mut self.paths, &mut s.paths);
        std::mem::swap(&mut self.lines, &mut s.lines);
        std::mem::swap(&mut self.props, &mut s.props);
        std::mem::swap(&mut self.actors, &mut s.actors);
        std::mem::swap(&mut self.terrain, &mut s.terrain);
    }

    /// The storeys bottom to top (one, the document's own, if it has no others).
    pub fn stack(&self) -> Vec<Stacked> {
        let mut out = vec![Stacked { at: None, z: self.outline.z, slab: self.slab.unwrap_or(SLAB) }];
        out.extend(self.storeys.iter().enumerate().map(|(i, s)| Stacked { at: Some(i), z: s.z, slab: s.slab.unwrap_or(SLAB) }));
        out.sort_by(|a, b| a.z.total_cmp(&b.z));
        out
    }

    /// A storey's name: its own, else its place in the stack ("storey 2", from the bottom).
    pub fn storey_name(&self, at: Option<usize>) -> String {
        let own = match at {
            None => &self.storey,
            Some(i) => &self.storeys[i].name,
        };
        if !own.is_empty() {
            return own.clone();
        }
        let k = self.stack().iter().position(|s| s.at == at).unwrap_or(0);
        format!("storey {}", k + 1)
    }

    /// The level as if storey `at` were all there is: in the document's own fields, no others.
    pub fn storey_doc(&self, at: Option<usize>) -> Doc {
        let mut d = self.clone();
        if let Some(i) = at {
            d.swap_storey(i);
        }
        d.storeys.clear();
        d
    }

    /// The level's text for `id`, if it has one.
    pub fn text(&self, id: u16) -> Option<&Text> {
        self.texts.iter().find(|t| t.id == id)
    }

    /// Sets the level's text for `id` (kept in id order), or with `None` gives it back to the game.
    pub fn set_text(&mut self, id: u16, text: Option<Text>) {
        self.texts.retain(|t| t.id != id);
        if let Some(t) = text {
            let at = self.texts.partition_point(|x| x.id < id);
            self.texts.insert(at, Text { id, ..t });
        }
    }
}

/// How the level plays in the game (docs/EDITOR-PLAY-IN-THE-GAME.md). The theme chooses the look
/// (and with it the lights, fog and sky); this chooses the game's dynamics: the region whose
/// scene's header and first room the level plays with (the music and nature ambience, the HUD's
/// rules, Navi's hints, the keep object, the camera type, the room's hazards and clock), and
/// changes to it. Empty or `None` takes the start's.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Game {
    /// The region it starts from (a theme's name: "dm_crater"); empty for the theme's own.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub start_from: String,
    /// Another region's music and nature ambience, or "none" for silence; empty for the start's.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub music: String,
    /// A hot room: the heat timer runs, as in Death Mountain Crater (`ROOM_ENV_HOT`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hot: Option<bool>,
    /// The sky drawn (the room's skybox on or off).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sky: Option<bool>,
    /// The clock running (the room's time speed), or stopped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clock: Option<bool>,
    /// Objects the room loads besides its actors' own (`object_*` files): for an actor that spawns
    /// others needing theirs, as the game's own rooms list them.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub objects: Vec<String>,
}

impl Game {
    pub fn is_default(&self) -> bool {
        *self == Game::default()
    }
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
/// - "rock": a freestanding rock over the closed shape of its nodes (its footprint), lofted up
///   through its `contours` (`rocks.rs`): boulders, spires, mesas, mushrooms. A node's third value
///   of 1 makes it a sharp corner, as a region's.
/// - "arch": a natural rock arch from the ground at its first node to the ground at its last,
///   `height` tall, `width` across its top, `depth` thick at its crown (`rocks.rs`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
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
    /// about `scale` across, smooth within `edge` of its mouths; the floor rolls a little. A rock's
    /// or an arch's lumps (`edge` unused): its faces push in and out by up to `amplitude`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub noise: Option<Noise>,
    /// A rock's shape above its footprint, lowest first: each a copy of the footprint, scaled and
    /// shifted, at a height over the ground. The last is its flat top.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contours: Vec<Contour>,
    /// A rock's layers: grooves round it every so often, as in banded sandstone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layers: Option<Layers>,
    /// A rock's or an arch's wall style, the theme's or another's (`kakariko:rock`). Else the
    /// theme's `rocks.style`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// An arch's thickness at its crown, top to underside (else the theme's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth: Option<f64>,
    /// How many pieces an arch is built of along it: few for a faceted arch, many for a smooth one
    /// (`rocks::ARCH_SEGMENTS`). Else about one every so often, by the level's detail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub segments: Option<u32>,
}

/// One of a rock's contours (`Line::contours`): its footprint scaled by `scale` about the
/// footprint's centre and moved by `shift`, `z` above the ground at its foot. A contour wider than
/// the one below it overhangs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contour {
    pub z: f64,
    #[serde(default = "one")]
    pub scale: f64,
    #[serde(default, skip_serializing_if = "is_zero2")]
    pub shift: [f64; 2],
}

impl Contour {
    pub fn new(z: f64, scale: f64) -> Contour {
        Contour { z, scale, shift: [0.0; 2] }
    }
}

/// Grooves round a rock (`Line::layers`): one every `height`, cut up to `depth` in.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layers {
    pub height: f64,
    pub depth: f64,
}

fn one() -> f64 {
    1.0
}

fn is_zero2(x: &[f64; 2]) -> bool {
    *x == [0.0; 2]
}

/// A rock shape to start from: what the editor offers for a new rock, its contours as heights
/// and scales for a footprint about `size` across.
pub struct RockLook {
    pub name: &'static str,
    pub tip: &'static str,
    /// (height, scale) for each contour, lowest first.
    pub contours: &'static [(f64, f64)],
    /// Lumps: (how much, how big).
    pub lumps: Option<(f64, f64)>,
    pub layers: Option<(f64, f64)>,
}

impl RockLook {
    /// This look's contours for a footprint about `size` across (the heights are for one about
    /// 400 across, and grow with it).
    pub fn contours(&self, size: f64) -> Vec<Contour> {
        let k = (size / 400.0).clamp(0.5, 4.0);
        self.contours.iter().map(|&(z, s)| Contour::new((z * k).round(), s)).collect()
    }

    /// Whether `line`'s contours have this look's shape (at any size).
    pub fn is(&self, line: &Line) -> bool {
        let c = &line.contours;
        if c.len() != self.contours.len() || c.iter().any(|x| x.shift != [0.0; 2]) {
            return false;
        }
        let k = c.last().map_or(1.0, |t| t.z) / self.contours.last().map_or(1.0, |t| t.0);
        c.iter().zip(self.contours).all(|(x, &(z, s))| (x.scale - s).abs() < 1e-6 && (x.z - z * k).abs() < 1.5)
    }

    pub fn noise(&self) -> Option<Noise> {
        self.lumps.map(|(amplitude, scale)| Noise { amplitude, scale, edge: 0.0, seed: 0 })
    }
}

/// Rock shapes to start from. The first is the default.
pub fn rock_looks() -> Vec<RockLook> {
    vec![
        RockLook {
            name: "Boulder",
            tip: "A rounded rock: wider a little way up, then curving in to a small top",
            contours: &[(90.0, 1.08), (190.0, 0.82), (240.0, 0.35)],
            lumps: Some((16.0, 140.0)),
            layers: None,
        },
        RockLook {
            name: "Mesa",
            tip: "Steep sides up to a broad flat top you can stand on (Gerudo Valley, Desert Colossus)",
            contours: &[(380.0, 0.86)],
            lumps: Some((12.0, 160.0)),
            layers: Some((120.0, 14.0)),
        },
        RockLook {
            name: "Spire",
            tip: "A tall needle of rock narrowing to a point",
            contours: &[(260.0, 0.62), (620.0, 0.3), (820.0, 0.08)],
            lumps: Some((10.0, 120.0)),
            layers: None,
        },
        RockLook {
            name: "Mushroom",
            tip: "A narrow stem under a wide overhanging cap",
            contours: &[(160.0, 0.48), (260.0, 0.62), (330.0, 1.2), (380.0, 1.1)],
            lumps: Some((10.0, 120.0)),
            layers: None,
        },
        RockLook {
            name: "Pillar",
            tip: "Straight up to a flat top (Lake Hylia's pillar)",
            contours: &[(700.0, 0.94)],
            lumps: Some((8.0, 180.0)),
            layers: None,
        },
    ]
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

/// One of the game's actors placed in the level (docs/EDITOR-ACTORS.md): the actor by its overlay
/// name, with the game's own placement record, its params and rotation, as a room's actor list
/// holds them. The editor shows them as named fields from the actor's schema (ADR 0066); what's
/// stored is what the game reads, so nothing the schema doesn't name is lost.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Actor {
    /// The overlay's name: "En_Kusa".
    pub actor: String,
    pub at: [f64; 2],
    /// Its height; on the ground there (the highest floor, as built) if `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub z: Option<f64>,
    /// `params`, its 16 bits.
    #[serde(default)]
    pub params: u16,
    /// The rotation as the game has it: binary angles (0x10000 a turn) about x, y and z. Some
    /// actors read a component as a setting. y 0 faces the game's +z, which is south here
    /// (`Actor::yaw`).
    #[serde(default)]
    pub rot: [i16; 3],
    /// The game layers it's in: child by day, child by night, adult by day, adult by night (a
    /// scene's first four setups). All of them if `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layers: Option<[bool; 4]>,
    /// A name of its own, for the outliner.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
}

impl Actor {
    pub fn new(actor: &str, at: [f64; 2]) -> Actor {
        Actor { actor: actor.to_string(), at, z: None, params: 0, rot: [0; 3], layers: None, name: String::new() }
    }

    /// Which way it faces, in degrees counter-clockwise from north (as a prop's `yaw`): the game's
    /// y rotation less half a turn.
    pub fn yaw(&self) -> f64 {
        let a = self.rot[1] as f64 / 65536.0 * 360.0 - 180.0;
        let a = ((a % 360.0) + 540.0) % 360.0 - 180.0;
        (a * 100.0).round() / 100.0
    }

    /// Faces `yaw` degrees counter-clockwise from north.
    pub fn set_yaw(&mut self, yaw: f64) {
        self.rot[1] = yaw_to_rot(yaw);
    }

    /// Whether it's in game layer `l` (0 child day, 1 child night, 2 adult day, 3 adult night).
    pub fn in_layer(&self, l: usize) -> bool {
        self.layers.is_none_or(|ls| ls.get(l).copied().unwrap_or(false))
    }
}

/// A yaw (degrees counter-clockwise from north) as the game's y rotation.
pub fn yaw_to_rot(yaw: f64) -> i16 {
    (((yaw + 180.0) / 360.0 * 65536.0).round() as i64).rem_euclid(65536) as u16 as i16
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
    /// A ceiling over the whole level: an enclosed level (a cave, a cavern, an interior), walled
    /// up to its ceiling instead of the edge of the world. Its `roof` is where a skylight's shaft
    /// ends, open to the sky. See `ceilings.rs`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ceiling: Option<Ceiling>,
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
    /// What's over it: none inherits its parent's (a ledge in a cave is under the cave's ceiling),
    /// `sky` opens it (a skylight in a covered parent), `closed` covers it. See `ceilings.rs`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ceiling: Option<Ceiling>,
}

/// What's over a region or the whole level (`ceilings.rs`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Ceiling {
    /// Open to the sky, though its parent is covered: a skylight.
    Sky,
    /// Covered.
    Closed(Cover),
}

/// A ceiling: `height` over the floor (the region's z, so it moves with the floor), rising by up
/// to `rise` away from its edges (a dome, or over `reach` from the edge and level beyond it: a
/// vault), rough by `noise` (fading out at its edges, as floor bumps do). The solid over it goes
/// up to `roof`: by default the floor round it (a cave dug into a hill keeps the hill), never less
/// than `ROOF_MIN` over the ceiling.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cover {
    #[serde(default = "cover_height")]
    pub height: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub rise: f64,
    /// How far from its edges a ceiling takes to rise (0: all the way to its middle).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub reach: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub noise: Option<Noise>,
    /// The solid's top (absolute), instead of the floor round it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roof: Option<f64>,
    /// The wall style of the walls the cover adds (the theme's rock by default). The ceiling itself
    /// is always the theme's ceiling rock (`theme::CeilingLook`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
}

impl Default for Cover {
    fn default() -> Self {
        Cover { height: cover_height(), rise: 0.0, reach: 0.0, noise: None, roof: None, style: None }
    }
}

fn cover_height() -> f64 {
    300.0
}

/// The least solid between a ceiling and its roof.
pub const ROOF_MIN: f64 = 40.0;

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
    /// steps drawn on it, its sides the stairs' profile: the theme's `steps`). Stairs are one
    /// straight flight between two nodes, as all of Kakariko's are (`Path::is_stairs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look: Option<String>,
    /// Its cross-section (`Section`): none, the ground as its line says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<Section>,
    /// Per segment between nodes, overriding `section` (null: none), as `modes` does `mode`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sections: Vec<Option<Section>>,
    /// Railings along its sides wherever they drop away (`RAIL_DROP` or more): a fence kind of the
    /// theme's (`fence`, `lattice`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub railings: Option<String>,
    /// Climb between its two nodes in switchbacks, solved so the slope stays walkable
    /// (`paths::meander`): its two nodes are the ends, and the builder lays the zig-zag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub switchbacks: Option<Switchbacks>,
}

/// "Make it walkable": a path between two nodes that zig-zags across a corridor so it climbs no
/// steeper than `grade` (`paths::meander`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Switchbacks {
    /// How wide the corridor is that its legs cross, side to side, centred on the line between
    /// its nodes. Else four path widths.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    /// The slope to climb at, in degrees.
    #[serde(default = "switchback_grade")]
    pub grade: f64,
    /// "round": the turns are half circles climbing on with the legs; "flat": they're level
    /// landings, the legs climbing between them.
    #[serde(default = "round")]
    pub turns: String,
    /// Which side of the line between its nodes the first leg heads off to: "left" or "right".
    #[serde(default = "left")]
    pub first: String,
}

impl Default for Switchbacks {
    fn default() -> Self {
        Switchbacks { width: None, grade: switchback_grade(), turns: round(), first: left() }
    }
}

fn switchback_grade() -> f64 {
    20.0
}

fn round() -> String {
    "round".into()
}

fn left() -> String {
    "left".into()
}

impl Default for Path {
    fn default() -> Self {
        Path {
            name: String::new(),
            nodes: vec![],
            width: path_width(),
            mode: attached(),
            modes: vec![],
            edge: None,
            shape: None,
            look: None,
            section: None,
            sections: vec![],
            railings: None,
            switchbacks: None,
        }
    }
}

impl Path {
    /// Whether it's stairs (`look` "steps"): one straight flight between its two nodes.
    pub fn is_stairs(&self) -> bool {
        self.look.as_deref() == Some("steps")
    }

    /// Segment k's section (`sections`, else `section`).
    pub fn section_of(&self, k: usize) -> Option<&Section> {
        if self.sections.is_empty() { self.section.as_ref() } else { self.sections.get(k).and_then(Option::as_ref) }
    }
}

/// A side drops away where the ground beside it is this far below the path, or more: railings
/// stand there (`Path::railings`).
pub const RAIL_DROP: f64 = 40.0;

/// A path's cross-section, for the whole path or a segment (`Path::section`, `sections`). The
/// height its line gives (`paths.rs`) is still where it is; a section raises or sinks the surface
/// from there, ramping back to the line at the path's ends and where sections change, so the ends
/// still meet the ground (`paths::RAMP`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Section {
    /// An embankment with a flat top, `height` above its line.
    Causeway {
        #[serde(default = "causeway_height")]
        height: f64,
    },
    /// A lane in a cutting, `depth` below its line: walls up both sides (Kakariko's, Hyrule
    /// Field's).
    Sunken {
        #[serde(default = "sunken_depth")]
        depth: f64,
    },
    /// Cut along a cliff (draw it along the cliff's edge): the walls above it and the drop below
    /// are the cliff's own, not an embankment's (Death Mountain Trail, Gerudo Valley).
    Ledge,
    /// A deck of planks on posts, `height` above its line, a pair of posts every `spacing`, open
    /// underneath (Kokiri Forest's walkways). Its segments float whatever their mode.
    Boardwalk {
        #[serde(default = "boardwalk_height")]
        height: f64,
        #[serde(default = "boardwalk_spacing")]
        spacing: f64,
    },
}

impl Section {
    pub fn name(&self) -> &'static str {
        match self {
            Section::Causeway { .. } => "causeway",
            Section::Sunken { .. } => "sunken",
            Section::Ledge => "ledge",
            Section::Boardwalk { .. } => "boardwalk",
        }
    }

    /// How far it raises (or, negative, sinks) the surface from its line.
    pub fn offset(&self) -> f64 {
        match self {
            Section::Causeway { height } | Section::Boardwalk { height, .. } => *height,
            Section::Sunken { depth } => -depth,
            Section::Ledge => 0.0,
        }
    }

    /// A new section of this kind, with its defaults.
    pub fn of_kind(kind: &str) -> Option<Section> {
        Some(match kind {
            "causeway" => Section::Causeway { height: causeway_height() },
            "sunken" => Section::Sunken { depth: sunken_depth() },
            "ledge" => Section::Ledge,
            "boardwalk" => Section::Boardwalk { height: boardwalk_height(), spacing: boardwalk_spacing() },
            _ => return None,
        })
    }
}

fn causeway_height() -> f64 {
    60.0
}

fn sunken_depth() -> f64 {
    120.0
}

fn boardwalk_height() -> f64 {
    80.0
}

fn boardwalk_spacing() -> f64 {
    200.0
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
