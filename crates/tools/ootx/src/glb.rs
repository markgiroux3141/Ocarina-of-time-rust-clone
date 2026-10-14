//! `ootx export-glb`: the pack's characters as .glb files for Blender or the game editor
//! (docs/adr/0068-glb-export.md).
//!
//! By default, `characters\<Name>.glb`: one file per character of `characters::CHARACTERS` (the
//! ported actors' people and creatures, Link, the horses), by its English name, with the colours
//! its draw code sets, standing in a T-pose: its idle animation's first frame with the arms
//! straightened out sideways. With `--all`, into `all\`, every skeleton (`skel/`), actor bake
//! of a skeleton's mesh (`BakeBody::Skeleton`), Link variant (`player/`) and skin skeleton
//! (`skin/`) as the pack has it, in the skeleton's zero pose.
//!
//! Each file: a node per limb in the rest pose, a skin whose joints are the limbs, the mesh in
//! model space with each vertex on its bone (a skin's vertex groups on their limbs, weighted), the
//! textures as PNG and the materials approximated (`oot_extract::gltf`). With `--anims`, the
//! object's standard animations as channels on the limbs at 20 frames a second.
//!
//! The geometry comes from the pack alone. The names the pack keys by address (the limbs, the
//! textures and so the materials) come from the decomp's XMLs when `oot.toml`'s project opens;
//! without it they're generic. The output is derived from the ROM: it goes under `out\`, which
//! git ignores.

mod characters;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::{Context, Result, ensure};
use eng_anim::anim::{JointTable, LinkAnimation, StandardAnimation};
use eng_anim::skeleton::{Limb, LimbType, Skeleton};
use eng_gfx::{DrawList, NO_BONE, Vertex};
use glam::{Mat4, Quat, Vec3};
use oot_extract::models::{NamedAnim, SkinnedModel, limb_quat, rest_pose};
use oot_game::pack::{BakeBody, BakeSegment, GamePack, MeshBake};
use oot_game::player_lib::{Age, Loadout};
use oot_game::skin::{LIMB_DONE, SkinPlace, SkinSkeleton};
use oot_import::project::Project;
use serde_json::json;

use characters::{CHARACTERS, Cast, Character, Color};

/// `export-glb`'s options.
pub struct Options {
    pub out: PathBuf,
    /// Only the files whose path under `out` contains this.
    pub filter: Option<String>,
    pub anims: bool,
    /// Everything the pack has, as it has it, instead of the characters.
    pub all: bool,
}

/// Exports everything `opts` selects and prints what it wrote.
pub fn run(opts: &Options) -> Result<()> {
    let pack = GamePack::open_default()?;
    let names = match Project::open_default() {
        Ok(p) => Some(Names::new(p)),
        Err(e) => {
            eprintln!("no decomp project ({e:#}): limbs and materials get generic names");
            None
        }
    };
    let mut ex = Exporter::new(&pack, names, opts.anims);
    let items = if opts.all { ex.items()? } else { characters_items() };
    // The pack's records go in a folder of their own, beside the characters.
    let items: Vec<Item> = items.into_iter().map(|i| Item { rel: if opts.all { format!("all/{}", i.rel) } else { i.rel }, ..i }).collect();
    let (mut files, mut triangles, mut anims) = (0usize, 0usize, 0usize);
    let mut errors = Vec::new();
    for item in items.iter().filter(|i| opts.filter.as_deref().is_none_or(|f| i.rel.contains(f))) {
        match ex.export(item, &opts.out) {
            Ok(w) => {
                files += 1;
                triangles += w.triangles;
                anims += w.anims;
            }
            Err(e) => errors.push(format!("{}: {e:#}", item.rel)),
        }
    }
    println!("{files} .glb files ({triangles} triangles, {anims} animations) in {}", opts.out.display());
    if !errors.is_empty() {
        println!("{} failed:", errors.len());
        for e in &errors {
            println!("  {e}");
        }
    }
    Ok(())
}

/// One .glb to write: its path under the output folder (without `.glb`) and what it's made of.
struct Item {
    rel: String,
    source: Source,
    /// A character's T-pose and colours; else the skeleton's zero pose.
    character: Option<&'static Character>,
}

enum Source {
    /// `skel/<file>/<symbol>` with its mesh, `mesh/<file>/<symbol>`.
    Skeleton { file: String, symbol: String },
    /// An actor's bake of a skeleton's mesh (`BakeBody::Skeleton`).
    Bake(MeshBake),
    /// A Link variant (`player/<age>/…`) with the default face; `anims` gives it Player's
    /// animations (one variant per age: the default loadout's).
    Link { age: Age, key: Option<String>, anims: bool },
    /// `skin/<file>/<symbol>` with its bake (`BakeBody::Skin`); baked here from the ROM through
    /// the importer when no actor bakes it.
    Skin { file: String, symbol: String, bake: Option<String> },
    /// A character whose bake isn't there (the table out of step with the actors).
    Missing(String),
}

/// The characters' files, `characters/<Name>`.
fn characters_items() -> Vec<Item> {
    let bakes = oot_actors::bakes();
    CHARACTERS
        .iter()
        .map(|c| {
            let source = match c.cast {
                Cast::Bake(name) => match bakes.iter().find(|b| b.name == name) {
                    Some(b) => Source::Bake(b.clone()),
                    None => Source::Missing(format!("no bake named {name} in oot_actors::bakes()")),
                },
                Cast::Skel(file, symbol) => Source::Skeleton { file: file.into(), symbol: symbol.into() },
                Cast::Link(age) => Source::Link { age, key: None, anims: true },
                Cast::Skin(file, symbol) => {
                    let bake = bakes.iter().find(|b| matches!(&b.body, BakeBody::Skin { file: f, symbol: s } if f == file && s == symbol));
                    Source::Skin { file: file.into(), symbol: symbol.into(), bake: bake.map(|b| b.name.clone()) }
                }
            };
            Item { rel: format!("characters/{}", c.name), source, character: Some(c) }
        })
        .collect()
}

/// What one .glb holds.
struct Written {
    triangles: usize,
    anims: usize,
}

/// A mesh and its skeleton, loaded for writing.
struct Loaded {
    skel: Skeleton,
    draw: DrawList,
    textures: HashMap<u64, String>,
    /// A skin skeleton's limbs and groups (`Source::Skin`).
    skin: Option<SkinSkeleton>,
    /// The skeleton's object and symbol: its limbs' names and its animations.
    file: String,
    symbol: String,
    /// Scales the root's translation (child Link's 0.64).
    root_scale: f32,
    /// Player's animations, with `--anims`.
    link_anims: bool,
    extras: serde_json::Value,
}

struct Exporter<'a> {
    pack: &'a GamePack,
    names: Option<Names>,
    anims: bool,
    /// An object's standard animations for a joint count.
    anim_cache: HashMap<(String, usize), Rc<[NamedAnim]>>,
    link_anims: Option<Rc<[NamedAnim]>>,
}

impl<'a> Exporter<'a> {
    fn new(pack: &'a GamePack, names: Option<Names>, anims: bool) -> Exporter<'a> {
        Exporter { pack, names, anims, anim_cache: HashMap::new(), link_anims: None }
    }

    /// Everything there is to export, in the pack's order (`--all`).
    fn items(&self) -> Result<Vec<Item>> {
        let mut v = Vec::new();
        for key in self.pack.assets.names("skel/") {
            if let Some((file, symbol)) = key.strip_prefix("skel/").and_then(|k| k.split_once('/')) {
                v.push(Item { rel: format!("{file}/{symbol}"), source: Source::Skeleton { file: file.into(), symbol: symbol.into() }, character: None });
            }
        }
        let bakes = oot_actors::bakes();
        for b in &bakes {
            if matches!(b.body, BakeBody::Skeleton { .. }) {
                v.push(Item { rel: format!("bake/{}", b.name), source: Source::Bake(b.clone()), character: None });
            }
        }
        let rules = self.pack.player_rules()?;
        for age in [Age::Adult, Age::Child] {
            let default = Loadout::default_for(&rules, age).variant_key(&rules);
            let mut used = HashSet::new();
            for key in self.pack.link_variant_names(age) {
                let mut rel = format!("player/{}/{}", age.name(), link_variant_short_name(age, &key));
                while !used.insert(rel.clone()) {
                    rel.push('_');
                }
                v.push(Item { rel, source: Source::Link { age, anims: key == default, key: Some(key) }, character: None });
            }
        }
        for key in self.pack.assets.names("skin/") {
            if let Some((file, symbol)) = key.strip_prefix("skin/").and_then(|k| k.split_once('/')) {
                let bake = bakes.iter().find(|b| matches!(&b.body, BakeBody::Skin { file: f, symbol: s } if f == file && s == symbol));
                v.push(Item {
                    rel: format!("{file}/{symbol}"),
                    source: Source::Skin { file: file.into(), symbol: symbol.into(), bake: bake.map(|b| b.name.clone()) },
                    character: None,
                });
            }
        }
        Ok(v)
    }

    fn export(&mut self, item: &Item, out: &Path) -> Result<Written> {
        let path = item.rel.split('/').fold(out.to_path_buf(), |p, c| p.join(file_name(c))).with_extension("glb");
        let mut l = self.load(&item.source)?;
        let name = match item.character {
            Some(c) => c.name.to_string(),
            None => match &item.source {
                Source::Skeleton { symbol, .. } | Source::Skin { symbol, .. } => symbol.clone(),
                _ => item.rel.rsplit('/').next().unwrap_or(&item.rel).to_string(),
            },
        };
        let (rest, bones) = match item.character {
            Some(c) => {
                set_colors(&mut l.draw, c.colors);
                for p in c.parts {
                    self.add_part(&mut l, p)?;
                }
                l.extras["character"] = json!({ "pose": "T-pose: the idle animation's first frame, the arms straightened sideways", "idle": c.idle.map(|(f, s)| format!("anim/{f}/{s}")) });
                let (rest, bones) = self.character_pose(c, &l)?;
                (Some(rest), bones)
            }
            None => (None, l.skin.as_ref().map(|s| skin_bones_for(s, &skin_zero_joints(s)))),
        };
        self.write(&path, &name, &l, rest.as_deref(), bones.as_deref())
    }

    /// The mesh, skeleton and names for a source.
    fn load(&mut self, source: &Source) -> Result<Loaded> {
        match source {
            Source::Missing(why) => anyhow::bail!("{why}"),
            Source::Skeleton { file, symbol } => {
                let skel = self.pack.skeleton(file, symbol)?;
                let draw = self.pack.mesh(file, symbol)?;
                let textures = self.texture_names(&draw, &[file.as_str()], &[]);
                let extras = json!({ "source": format!("skel/{file}/{symbol}") });
                Ok(Loaded { skel, draw, textures, skin: None, file: file.clone(), symbol: symbol.clone(), root_scale: 1.0, link_anims: false, extras })
            }
            Source::Bake(bake) => {
                let BakeBody::Skeleton { file, symbol, limbs } = &bake.body else { anyhow::bail!("{} isn't a skeleton's bake", bake.name) };
                let skel = self.pack.skeleton(file, symbol)?;
                let draw = self.pack.bake(&bake.name)?;
                // Segment 6 is the bake's object, or a limb override's file while that limb draws.
                let mut files = vec![bake.object.as_str(), file.as_str()];
                files.extend(limbs.iter().map(|l| l.file.as_str()));
                let bound: Vec<(u8, &str)> = bake
                    .segments
                    .iter()
                    .filter_map(|(seg, s)| match s {
                        BakeSegment::Texture { symbol, .. } => Some((*seg, symbol.as_str())),
                        _ => None,
                    })
                    .collect();
                let textures = self.texture_names(&draw, &files, &bound);
                let extras = json!({ "source": format!("bake/{}", bake.name), "skeleton": format!("skel/{file}/{symbol}"), "object": bake.object });
                Ok(Loaded { skel, draw, textures, skin: None, file: file.clone(), symbol: symbol.clone(), root_scale: 1.0, link_anims: false, extras })
            }
            Source::Link { age, key, anims } => {
                let rules = self.pack.player_rules()?;
                let key = key.clone().unwrap_or_else(|| Loadout::default_for(&rules, *age).variant_key(&rules));
                let skel = self.pack.link_skeleton(*age)?;
                let variant: oot_game::player_lib::LinkVariant = self.pack.assets.get(&key)?;
                let draw = variant.with_face(&self.pack.link_faces(*age)?, 0, 0);
                let textures = self.texture_names(&draw, &[age.object()], &[]);
                let symbol = if *age == Age::Adult { "gLinkAdultSkel" } else { "gLinkChildSkel" };
                let extras = json!({ "source": key, "skeleton": format!("skel/{}/{symbol}", age.object()), "face": "eyes 0 (open), mouth 0" });
                Ok(Loaded { skel, draw, textures, skin: None, file: age.object().into(), symbol: symbol.into(), root_scale: age.root_scale(), link_anims: *anims, extras })
            }
            Source::Skin { file, symbol, bake } => {
                let skin = self.pack.skin_skeleton(file, symbol)?;
                let draw = match bake {
                    Some(b) => self.pack.bake(b)?,
                    None => self.bake_skin(file, symbol)?,
                };
                let textures = self.texture_names(&draw, &[file.as_str()], &[]);
                let source = match bake {
                    Some(b) => format!("bake/{b}"),
                    None => "baked by ootx from the ROM (no actor bakes it)".into(),
                };
                let extras = json!({ "source": source, "skeleton": format!("skin/{file}/{symbol}"), "skin": "each vertex group on its SkinTransformations' limbs, weighted by scale / 100 (the four heaviest)" });
                Ok(Loaded { skel: skin_as_skeleton(&skin), draw, textures, skin: Some(skin), file: file.clone(), symbol: symbol.clone(), root_scale: 1.0, link_anims: false, extras })
            }
        }
    }

    /// Adds a part to a character's mesh: its vertices on its limb (the draw puts it at the limb's
    /// matrix), its materials and textures after the mesh's.
    fn add_part(&mut self, l: &mut Loaded, p: &characters::Part) -> Result<()> {
        let bake = oot_actors::bakes().into_iter().find(|b| b.name == p.bake).with_context(|| format!("no bake named {}", p.bake))?;
        let mut draw = self.pack.bake(p.bake)?;
        set_colors(&mut draw, p.colors);
        let textures = self.texture_names(&draw, &[bake.object.as_str()], &[]);
        l.textures.extend(textures);
        let (materials, images) = (l.draw.materials.len(), l.draw.textures.len());
        for mut m in draw.materials {
            for slot in m.textures.iter_mut().flatten() {
                slot.image += images;
            }
            l.draw.materials.push(m);
        }
        l.draw.textures.extend(draw.textures);
        for mut b in draw.batches {
            b.material += materials;
            for v in &mut b.vertices {
                v.bone = p.limb as u16;
            }
            l.draw.batches.push(b);
        }
        Ok(())
    }

    /// A character's rest pose: its idle animation's first frame (else the zero pose) at the
    /// origin, its arms straightened sideways; a skin's bones for it.
    fn character_pose(&self, c: &Character, l: &Loaded) -> Result<(Vec<(Vec3, Quat)>, Option<Vec<Mat4>>)> {
        let limbs = l.skel.limbs.len();
        let mut joints = match c.idle {
            Some((file, symbol)) => {
                let frame: JointTable = if symbol.starts_with("gPlayerAnim_") {
                    let a: LinkAnimation = self.pack.assets.get(&oot_game::pack::keys::anim(file, symbol))?;
                    a.frames.into_iter().next().context("an empty animation")?
                } else {
                    self.pack.standard_animation(file, symbol)?.frames.into_iter().next().context("an empty animation")?
                };
                ensure!(frame.rot.len() == limbs + 1, "{symbol} has {} joints, the skeleton {}", frame.rot.len(), limbs + 1);
                frame.rot
            }
            None => {
                let mut z = vec![[0i16; 3]; limbs + 1];
                z[0] = l.skel.limbs[0].joint_pos;
                z
            }
        };
        // At the origin, at the animation's height.
        joints[0] = [0, (joints[0][1] as f32 * l.root_scale) as i16, 0];
        let mut local: Vec<(Vec3, Quat)> = (0..limbs)
            .map(|i| {
                let t = if i == 0 { joints[0] } else { l.skel.limbs[i].joint_pos };
                (Vec3::from(t.map(|c| c as f32)), limb_quat(joints[i + 1]))
            })
            .collect();
        match &l.skin {
            Some(s) => Ok((local, Some(skin_bones_for(s, &joints)))),
            None => {
                for arm in [c.left, c.right] {
                    straighten_arm(&l.skel, &mut local, arm);
                }
                Ok((local, None))
            }
        }
    }

    /// A skin skeleton no actor bakes, baked as `ObjectSegments::bake_mesh` bakes one.
    fn bake_skin(&self, file: &str, symbol: &str) -> Result<DrawList> {
        let p = &self.names.as_ref().context("no actor bakes this skin skeleton, and baking it needs the decomp project")?.project;
        let bake = MeshBake { name: String::new(), object: file.into(), segments: Vec::new(), prelude: Vec::new(), body: BakeBody::Skin { file: file.into(), symbol: symbol.into() } };
        oot_import::objects::ObjectSegments::load(p).bake_mesh(p, &oot_import::objects::Files::new(p), &bake)
    }

    fn write(&mut self, path: &Path, name: &str, l: &Loaded, rest: Option<&[(Vec3, Quat)]>, bones: Option<&[Mat4]>) -> Result<Written> {
        let limbs = l.skel.limbs.len();
        let limb_names = match &self.names {
            Some(n) => n.limbs(&l.file, &l.symbol, limbs),
            None => Vec::new(),
        };
        let anims: Rc<[NamedAnim]> = match self.anims {
            true if l.file == "object_link_boy" || l.file == "object_link_child" => match l.link_anims {
                true => self.link_anims()?,
                false => Rc::from(Vec::new()),
            },
            true => self.object_anims(&l.file, limbs + 1),
            false => Rc::from(Vec::new()),
        };
        let groups = l.skin.as_ref().map(skin_weights).unwrap_or_default();
        let weights = move |v: &Vertex| -> ([u16; 4], [f32; 4]) {
            match v.bone as usize {
                _ if v.bone == NO_BONE => ([0; 4], [1.0, 0.0, 0.0, 0.0]),
                b if b < limbs => ([v.bone, 0, 0, 0], [1.0, 0.0, 0.0, 0.0]),
                b => groups.get(b - limbs).copied().unwrap_or(([0; 4], [1.0, 0.0, 0.0, 0.0])),
            }
        };
        let mut extras = l.extras.clone();
        extras["unresolved_segments"] = json!(l.draw.stats.unresolved_addresses.keys().collect::<Vec<_>>());
        extras["note"] = json!("Derived from the user's ROM: local only, never shared or committed.");
        let triangles = SkinnedModel {
            name,
            skel: &l.skel,
            limb_names: &limb_names,
            draw: &l.draw,
            rest,
            bones,
            weights: l.skin.is_some().then_some(&weights as oot_extract::models::VertexWeights),
            anims: &anims,
            root_scale: l.root_scale,
            texture_names: l.textures.clone(),
            extras,
        }
        .write(path)?;
        Ok(Written { triangles, anims: anims.len() })
    }

    /// The textures' symbols by `TextureImage::hash`, from where each was loaded: a segment the
    /// bake binds a texture to (`bound`), else the keeps on 4 and 5 or one of `files` on 6
    /// (tried in order) through the XMLs.
    fn texture_names(&mut self, draw: &DrawList, files: &[&str], bound: &[(u8, &str)]) -> HashMap<u64, String> {
        let mut out = HashMap::new();
        for t in &draw.textures {
            let Some(addr) = t.source_addr else { continue };
            let (seg, offset) = ((addr >> 24) as u8, addr & 0xFF_FFFF);
            let name = match bound.iter().find(|(s, _)| *s == seg) {
                Some((_, sym)) if offset == 0 => Some(sym.to_string()),
                Some(_) => None,
                None => {
                    let candidates: &[&str] = match seg {
                        4 => &["gameplay_keep"],
                        5 => &["gameplay_field_keep"],
                        6 => files,
                        _ => &[],
                    };
                    let names = &mut self.names;
                    candidates.iter().find_map(|f| names.as_mut().and_then(|n| n.texture(f, offset)))
                }
            };
            if let Some(n) = name {
                out.insert(t.hash, n);
            }
        }
        out
    }

    /// The object's standard animations for a skeleton of `joints` entries (root and limbs).
    fn object_anims(&mut self, file: &str, joints: usize) -> Rc<[NamedAnim]> {
        let pack = self.pack;
        self.anim_cache
            .entry((file.to_string(), joints))
            .or_insert_with(|| {
                let prefix = format!("anim/{file}/");
                pack.assets
                    .names(&prefix)
                    .iter()
                    .filter_map(|k| k.strip_prefix(&prefix))
                    // gameplay_keep's Player animations are `LinkAnimation`s.
                    .filter(|s| !s.starts_with("gPlayerAnim_"))
                    .filter_map(|s| {
                        let a: StandardAnimation = pack.standard_animation(file, s).ok()?;
                        (a.frames.first()?.rot.len() == joints).then(|| NamedAnim { name: s.to_string(), frames: a.frames })
                    })
                    .collect::<Vec<_>>()
                    .into()
            })
            .clone()
    }

    /// Player's animations (`gameplay_keep`'s `gPlayerAnim_*`).
    fn link_anims(&mut self) -> Result<Rc<[NamedAnim]>> {
        if self.link_anims.is_none() {
            let prefix = "anim/gameplay_keep/gPlayerAnim_";
            let v = self
                .pack
                .assets
                .names(prefix)
                .into_iter()
                .map(|k| -> Result<NamedAnim> {
                    let a: LinkAnimation = self.pack.assets.get(&k)?;
                    Ok(NamedAnim { name: k["anim/gameplay_keep/".len()..].to_string(), frames: a.frames })
                })
                .collect::<Result<Vec<_>>>()?;
            self.link_anims = Some(v.into());
        }
        Ok(self.link_anims.clone().unwrap_or_else(|| Rc::from(Vec::new())))
    }
}

/// A file name as the user reads it: the name with only the characters Windows refuses replaced.
fn file_name(name: &str) -> String {
    name.chars().map(|c| if c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') { '_' } else { c }).collect()
}

/// The colours a character's draw code sets on the bake's dynamic segments (which the bake
/// stores as black): each material that reads the segment's env or prim takes it.
fn set_colors(draw: &mut DrawList, colors: &[(u8, Color, [u8; 4])]) {
    for m in &mut draw.materials {
        for &(seg, kind, rgba) in colors {
            match kind {
                Color::Env if m.env_dyn == Some(seg) => m.env = rgba,
                Color::Prim if m.prim_dyn == Some(seg) => m.prim = rgba,
                _ => {}
            }
        }
    }
}

/// Turns an arm's limbs (shoulder to hand, each a descendant of the one before) so each points
/// straight out to its side along X, keeping the last limb's rotation relative to its parent.
/// The side is the first of the arm's joints off the body's centre line (the shoulder's, as a
/// pose's hands may be clasped across the body).
fn straighten_arm(skel: &Skeleton, local: &mut [(Vec3, Quat)], arm: &[u8]) {
    let Some(&first) = arm.first() else { return };
    let pose = rest_pose(skel, local);
    let at = |m: &Mat4| m.w_axis.truncate();
    let centre = skel.parents[first as usize].map(|p| at(&pose[p as usize])).unwrap_or(Vec3::ZERO);
    let Some(side) = arm.iter().map(|&l| at(&pose[l as usize]).x - centre.x).find(|x| x.abs() > 1.0) else { return };
    let target = Vec3::new(side.signum(), 0.0, 0.0);
    for w in arm.windows(2) {
        let (l, next) = (w[0] as usize, w[1] as usize);
        let pose = rest_pose(skel, local);
        let dir = at(&pose[next]) - at(&pose[l]);
        if dir.length() < 1e-3 {
            continue;
        }
        let world = Quat::from_mat4(&pose[l]);
        let parent = skel.parents[l].map(|p| Quat::from_mat4(&pose[p as usize])).unwrap_or(Quat::IDENTITY);
        let turned = Quat::from_rotation_arc(dir.normalize(), target) * world;
        local[l].1 = (parent.inverse() * turned).normalize();
    }
}

/// A Link variant's file name: its lists without the age's prefix and `NearDL`, `-` (none) left
/// out (`LeftHandClosed_RightHandClosed_HylianShieldSwordAndSheath_Waist`).
fn link_variant_short_name(age: Age, key: &str) -> String {
    let prefix = if age == Age::Adult { "gLinkAdult" } else { "gLinkChild" };
    let lists = key.rsplit('/').next().unwrap_or(key);
    let parts: Vec<&str> = lists
        .split('+')
        .filter(|s| *s != "-")
        .map(|s| s.strip_prefix(prefix).unwrap_or(s))
        .map(|s| s.strip_suffix("NearDL").or_else(|| s.strip_suffix("DL")).unwrap_or(s))
        .collect();
    if parts.is_empty() { "none".into() } else { parts.join("_") }
}

/// A skin skeleton's limbs as a `Skeleton` (its hierarchy and joint positions, no lists).
fn skin_as_skeleton(s: &SkinSkeleton) -> Skeleton {
    let link = |i: u8| (i != LIMB_DONE).then_some(i);
    let limbs: Vec<Limb> = s.limbs.iter().map(|l| Limb { joint_pos: l.joint_pos, child: link(l.child), sibling: link(l.sibling), dlists: Vec::new() }).collect();
    let mut parents = vec![None; limbs.len()];
    let mut draw_order = Vec::with_capacity(limbs.len());
    let mut stack = vec![(0u8, None)];
    while let Some((l, parent)) = stack.pop() {
        if draw_order.contains(&l) || l as usize >= limbs.len() {
            continue;
        }
        parents[l as usize] = parent;
        draw_order.push(l);
        if let Some(s) = limbs[l as usize].sibling {
            stack.push((s, parent));
        }
        if let Some(c) = limbs[l as usize].child {
            stack.push((c, Some(l)));
        }
    }
    Skeleton { limbs, limb_type: LimbType::Lod, flex: false, dlist_count: 0, parents, draw_order }
}

/// The zero pose's joint table of a skin (the root at its joint position, as `Skeleton::bind_pose`).
fn skin_zero_joints(s: &SkinSkeleton) -> Vec<[i16; 3]> {
    let mut joints = vec![[0i16; 3]; s.limbs.len() + 1];
    joints[0] = s.limbs.first().map(|l| l.joint_pos).unwrap_or([0; 3]);
    joints
}

/// A skin's bones for a joint table (`oot_game::skin::skin_bones`, the root translated).
fn skin_bones_for(s: &SkinSkeleton, joints: &[[i16; 3]]) -> Vec<Mat4> {
    let place = SkinPlace { scale: Vec3::ONE, rot: [0; 3], pos: Vec3::ZERO, y_offset: 0.0 };
    let (mats, _) = oot_game::skin::apply_anim_transformations(s, joints, &place, true);
    oot_game::skin::skin_bones(s, &mats, false)
}

/// Each vertex group's joints and weights: its `SkinTransformation`s' limbs by `scale / 100`, the
/// four heaviest, normalised (one transformation, weight 1, as `Skin_ApplyLimbModifications`
/// ignores its scale).
fn skin_weights(s: &SkinSkeleton) -> Vec<([u16; 4], [f32; 4])> {
    s.limbs
        .iter()
        .flat_map(|l| &l.modifs)
        .map(|m| {
            let mut w: Vec<(u16, f32)> = match m.transformations.as_slice() {
                [t] => vec![(t.limb_index as u16, 1.0)],
                ts => ts.iter().map(|t| (t.limb_index as u16, t.scale as f32 * 0.01)).collect(),
            };
            w.sort_by(|a, b| b.1.total_cmp(&a.1));
            w.truncate(4);
            let sum = w.iter().map(|x| x.1).sum::<f32>().max(1e-6);
            let (mut j, mut ww) = ([0u16; 4], [0f32; 4]);
            for (k, (l, x)) in w.iter().enumerate() {
                j[k] = *l;
                ww[k] = x / sum;
            }
            (j, ww)
        })
        .collect()
}

/// The decomp's names for what the pack keys by address: the limbs (through the skeleton's limb
/// table in the ROM) and the textures (by offset in their file).
struct Names {
    project: Project,
    /// File → offset → texture symbol.
    textures: HashMap<String, HashMap<u32, String>>,
}

impl Names {
    fn new(project: Project) -> Names {
        Names { project, textures: HashMap::new() }
    }

    fn limbs(&self, file: &str, symbol: &str, count: usize) -> Vec<String> {
        let found = self.project.symbols.file(file).and_then(|f| Some((f, f.find(symbol)?, self.project.rom.file_by_name(file).ok()?)));
        match found {
            Some((f, s, data)) => oot_extract::models::limb_names(f, &data, s.offset as usize, count),
            None => Vec::new(),
        }
    }

    fn texture(&mut self, file: &str, offset: u32) -> Option<String> {
        let symbols = &self.project.symbols;
        self.textures
            .entry(file.to_string())
            .or_insert_with(|| symbols.file(file).map(|f| f.of_kind("Texture").map(|s| (s.offset, s.name.clone())).collect()).unwrap_or_default())
            .get(&offset)
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eng_gfx::posed_bounds;
    use oot_extract::models::ACTOR_SCALE;
    use serde_json::Value;

    /// The JSON and BIN chunks of a .glb, checking its header.
    fn read_glb(bytes: &[u8]) -> (Value, &[u8]) {
        let u32_at = |o: usize| u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap()) as usize;
        assert_eq!(&bytes[0..4], b"glTF");
        assert_eq!(u32_at(4), 2, "version");
        assert_eq!(u32_at(8), bytes.len(), "length");
        let json_len = u32_at(12);
        assert_eq!(&bytes[16..20], b"JSON");
        let json: Value = serde_json::from_slice(&bytes[20..20 + json_len]).unwrap();
        let bin_at = 20 + json_len;
        let bin_len = u32_at(bin_at);
        assert_eq!(&bytes[bin_at + 4..bin_at + 8], b"BIN\0");
        assert_eq!(bin_at + 8 + bin_len, bytes.len());
        assert_eq!(json["buffers"][0]["byteLength"].as_u64().unwrap() as usize, bin_len);
        (json, &bytes[bin_at + 8..])
    }

    /// An accessor's floats (tightly packed, as `oot_extract::gltf` writes them).
    fn floats(json: &Value, bin: &[u8], accessor: usize) -> Vec<f32> {
        let a = &json["accessors"][accessor];
        let v = &json["bufferViews"][a["bufferView"].as_u64().unwrap() as usize];
        let (o, n) = (v["byteOffset"].as_u64().unwrap() as usize, v["byteLength"].as_u64().unwrap() as usize);
        bin[o..o + n].chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect()
    }

    /// The union of the meshes' POSITION bounds.
    fn position_bounds(json: &Value) -> (Vec3, Vec3) {
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        let v3 = |v: &Value| Vec3::new(v[0].as_f64().unwrap() as f32, v[1].as_f64().unwrap() as f32, v[2].as_f64().unwrap() as f32);
        for p in json["meshes"][0]["primitives"].as_array().unwrap() {
            let a = &json["accessors"][p["attributes"]["POSITION"].as_u64().unwrap() as usize];
            lo = lo.min(v3(&a["min"]));
            hi = hi.max(v3(&a["max"]));
        }
        (lo, hi)
    }

    fn export_one(pack: &GamePack, item: &Item, anims: bool) -> (Value, Vec<u8>) {
        // A folder per test: they run in parallel.
        let dir = std::env::temp_dir().join(format!("ootx-export-glb-{}-{}", std::process::id(), oot_extract::sanitize(&item.rel)));
        let mut ex = Exporter::new(pack, None, anims);
        ex.export(item, &dir).unwrap();
        let path = dir.join(item.rel.replace('/', std::path::MAIN_SEPARATOR_STR)).with_extension("glb");
        let bytes = std::fs::read(&path).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        let (json, bin) = read_glb(&bytes);
        (json, bin.to_vec())
    }

    fn skeleton_item(file: &str, symbol: &str) -> Item {
        Item { rel: format!("{file}/{symbol}"), source: Source::Skeleton { file: file.into(), symbol: symbol.into() }, character: None }
    }

    #[test]
    fn the_arrow_reads_back_in_its_bind_pose() {
        let Ok(pack) = GamePack::open_default() else {
            eprintln!("no asset pack: skipped");
            return;
        };
        let (file, symbol) = ("gameplay_keep", "gArrowSkel");
        let (json, bin) = export_one(&pack, &skeleton_item(file, symbol), true);
        let skel = pack.skeleton(file, symbol).unwrap();
        let draw = pack.mesh(file, symbol).unwrap();
        let limbs = skel.limbs.len();
        // The armature, a node per limb, the mesh's node; nothing scaled by a node.
        assert_eq!(json["nodes"].as_array().unwrap().len(), 1 + limbs + 1);
        assert!(json["nodes"].as_array().unwrap().iter().all(|n| n.get("scale").is_none()));
        assert_eq!(json["meshes"].as_array().unwrap().len(), 1);
        let skin = &json["skins"][0];
        assert_eq!(skin["joints"].as_array().unwrap().len(), limbs);
        assert_eq!(json["accessors"][skin["inverseBindMatrices"].as_u64().unwrap() as usize]["count"], limbs);
        // Each limb's node at its joint position (in the file's scale), its parent's child.
        for (i, l) in skel.limbs.iter().enumerate() {
            let n = &json["nodes"][1 + i];
            assert_eq!(n["translation"], json!(l.joint_pos.map(|c| c as f32 * ACTOR_SCALE)), "limb {i}");
            if let Some(p) = skel.parents[i] {
                assert!(json["nodes"][1 + p as usize]["children"].as_array().unwrap().contains(&json!(1 + i)));
            }
        }
        // The inverse bind matrices undo the bind pose.
        let bind = skel.bind_pose();
        let scale = Mat4::from_scale(Vec3::splat(ACTOR_SCALE));
        let ibm = floats(&json, &bin, skin["inverseBindMatrices"].as_u64().unwrap() as usize);
        for (i, m) in bind.iter().enumerate() {
            let inv = Mat4::from_cols_slice(&ibm[i * 16..i * 16 + 16]);
            assert!((scale * *m * scale.inverse() * inv).abs_diff_eq(Mat4::IDENTITY, 1e-4), "limb {i}");
        }
        // The vertices in model space for the bind pose.
        let (lo, hi) = position_bounds(&json);
        let (blo, bhi) = posed_bounds(&draw, &bind);
        assert!(lo.abs_diff_eq(blo * ACTOR_SCALE, 1e-5) && hi.abs_diff_eq(bhi * ACTOR_SCALE, 1e-5), "{lo} {hi} against {blo} {bhi}");
        // Its object's animations for its joint count, each a rotation per limb and the root's
        // translation.
        let anims = json["animations"].as_array().map(|a| a.len()).unwrap_or(0);
        let expected = Exporter::new(&pack, None, true).object_anims(file, limbs + 1).len();
        assert_eq!(anims, expected);
        for a in json["animations"].as_array().into_iter().flatten() {
            assert_eq!(a["channels"].as_array().unwrap().len(), limbs + 1);
        }
        eprintln!("gArrowSkel: {limbs} limbs, {} triangles, {anims} animations", draw.triangle_count());
    }

    #[test]
    fn animations_turn_the_limbs_as_the_joint_tables_do() {
        let Ok(pack) = GamePack::open_default() else {
            eprintln!("no asset pack: skipped");
            return;
        };
        let (file, symbol) = ("object_sa", "gSariaSkel");
        let (json, bin) = export_one(&pack, &skeleton_item(file, symbol), true);
        let limbs = pack.skeleton(file, symbol).unwrap().limbs.len();
        let expected = Exporter::new(&pack, None, true).object_anims(file, limbs + 1);
        let anims = json["animations"].as_array().unwrap();
        assert!(!expected.is_empty());
        assert_eq!(anims.len(), expected.len());
        for (a, e) in anims.iter().zip(expected.iter()) {
            assert_eq!(a["name"], json!(e.name));
            let frames = e.frames.len();
            // Frame f at f / 20 seconds.
            let input = &json["accessors"][a["samplers"][0]["input"].as_u64().unwrap() as usize];
            assert_eq!(input["count"], frames);
            assert!((input["max"][0].as_f64().unwrap() as f32 - (frames - 1) as f32 / 20.0).abs() < 1e-5);
            for c in a["channels"].as_array().unwrap() {
                let node = c["target"]["node"].as_u64().unwrap() as usize;
                let out = floats(&json, &bin, a["samplers"][c["sampler"].as_u64().unwrap() as usize]["output"].as_u64().unwrap() as usize);
                let limb = node - 1;
                for (f, jt) in e.frames.iter().enumerate() {
                    if c["target"]["path"] == "translation" {
                        assert_eq!(limb, 0, "only the root moves");
                        let want = Vec3::from(jt.rot[0].map(|v| v as f32)) * ACTOR_SCALE;
                        assert!(Vec3::from_slice(&out[f * 3..f * 3 + 3]).abs_diff_eq(want, 1e-5));
                    } else {
                        // `Matrix_TranslateRotateZYX`'s order, the s16s as binary angles.
                        let r = jt.rot[limb + 1].map(eng_anim::skeleton::binang_to_rad);
                        let q = Quat::from_rotation_z(r[2]) * Quat::from_rotation_y(r[1]) * Quat::from_rotation_x(r[0]);
                        let got = Quat::from_slice(&out[f * 4..f * 4 + 4]);
                        assert!(got.dot(q).abs() > 0.9999, "{} limb {limb} frame {f}: {got} against {q}", e.name);
                    }
                }
            }
        }
        eprintln!("gSariaSkel: {} animations", anims.len());
    }

    #[test]
    fn a_skin_weighs_its_groups_on_their_limbs() {
        let Ok(pack) = GamePack::open_default() else {
            eprintln!("no asset pack: skipped");
            return;
        };
        let (file, symbol) = ("object_horse_zelda", "gHorseZeldaSkel");
        let bake = oot_actors::bakes().into_iter().find(|b| matches!(&b.body, BakeBody::Skin { symbol: s, .. } if s == symbol)).unwrap();
        let item = Item { rel: format!("{file}/{symbol}"), source: Source::Skin { file: file.into(), symbol: symbol.into(), bake: Some(bake.name.clone()) }, character: None };
        let (json, bin) = export_one(&pack, &item, false);
        let skin = pack.skin_skeleton(file, symbol).unwrap();
        let limbs = skin.limbs.len();
        assert_eq!(json["skins"][0]["joints"].as_array().unwrap().len(), limbs);
        // Every vertex on limbs of the skeleton, its weights summing to 1.
        let mut weighted = 0;
        for p in json["meshes"][0]["primitives"].as_array().unwrap() {
            let w = floats(&json, &bin, p["attributes"]["WEIGHTS_0"].as_u64().unwrap() as usize);
            let a = &json["accessors"][p["attributes"]["JOINTS_0"].as_u64().unwrap() as usize];
            let v = &json["bufferViews"][a["bufferView"].as_u64().unwrap() as usize];
            let o = v["byteOffset"].as_u64().unwrap() as usize;
            let joints: Vec<u16> = bin[o..o + v["byteLength"].as_u64().unwrap() as usize].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            assert!(joints.iter().all(|&j| (j as usize) < limbs));
            for w in w.chunks_exact(4) {
                assert!((w.iter().sum::<f32>() - 1.0).abs() < 1e-4, "{w:?}");
                weighted += (w[1] > 0.0) as usize;
            }
        }
        assert!(weighted > 0, "no vertex blends two limbs");
        // The vertices where the game's bones put them in the zero pose.
        let draw = pack.bake(&bake.name).unwrap();
        let bones = skin_bones_for(&skin, &skin_zero_joints(&skin));
        let (lo, hi) = position_bounds(&json);
        let (blo, bhi) = posed_bounds(&draw, &bones);
        assert!(lo.abs_diff_eq(blo * ACTOR_SCALE, 1e-4) && hi.abs_diff_eq(bhi * ACTOR_SCALE, 1e-4), "{lo} {hi} against {blo} {bhi}");
        eprintln!("gHorseZeldaSkel: {limbs} limbs, {} triangles, {weighted} blended vertices", draw.triangle_count());
    }

    #[test]
    fn link_stands_in_a_t_pose() {
        let Ok(pack) = GamePack::open_default() else {
            eprintln!("no asset pack: skipped");
            return;
        };
        let c = CHARACTERS.iter().find(|c| c.name == "Link (adult)").unwrap();
        let mut ex = Exporter::new(&pack, None, false);
        let l = ex.load(&Source::Link { age: Age::Adult, key: None, anims: false }).unwrap();
        let (local, bones) = ex.character_pose(c, &l).unwrap();
        assert!(bones.is_none());
        let pose = rest_pose(&l.skel, &local);
        let at = |i: u8| pose[i as usize].w_axis.truncate();
        // `PLAYER_LIMB_HEAD` above `PLAYER_LIMB_R_FOOT` and `_L_FOOT`: standing.
        assert!(at(10).y > at(5).y + 1000.0 && at(10).y > at(8).y + 1000.0, "head {} feet {} {}", at(10), at(5), at(8));
        // Each arm level, out to its own side, the hands farther out than the shoulders.
        for (shoulder, hand) in [(13u8, 15u8), (16, 18)] {
            let (s, h) = (at(shoulder), at(hand));
            assert!((h.y - s.y).abs() < 1.0 && (h.z - s.z).abs() < 1.0, "shoulder {s} hand {h}");
            assert!(h.x.abs() > s.x.abs() + 300.0 && h.x.signum() == s.x.signum(), "shoulder {s} hand {h}");
        }
        assert!(at(15).x.signum() != at(18).x.signum());
        eprintln!("Link (adult): head {}, hands {} {}", at(10), at(15), at(18));
    }
}
