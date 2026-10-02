//! Player (Link) from the ROM and the decomp: the draw-rule tables read from `z_player_lib.c`
//! and `player.h` (`oot_game::player_lib`, re-exported here), Link's skeleton and object
//! data (`PlayerModel`, which applies the rules the way `Player_DrawImpl` and
//! `Player_OverrideLimbDrawGameplayDefault` do), and Player's animations.

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use eng_anim::anim::LinkAnimation;
use eng_anim::skeleton::{LimbType, Skeleton};
use eng_gbi::gbi::DrawList;
use eng_gbi::model::{Binding, BuildOptions, build_draw_list};

use crate::csrc::{Init, Macros, enum_members, find_initializer};
use crate::project::Project;
use crate::symbols::AssetFile;
use crate::z64::{ParseLinkAnimation, ParseSkeleton};
pub use oot_game::player_lib::*;

/// Reads `PlayerRules` from the decomp: `PlayerRules::load(decomp)` with this trait in scope.
pub trait LoadPlayerRules: Sized {
    fn load(decomp: &Path) -> Result<Self>;
}

impl LoadPlayerRules for PlayerRules {
    fn load(decomp: &Path) -> Result<PlayerRules> {
        let lib_path = decomp.join("src/code/z_player_lib.c");
        let lib = crate::csrc::prepare(&std::fs::read_to_string(&lib_path).with_context(|| lib_path.display().to_string())?);
        let header = std::fs::read_to_string(decomp.join("include/player.h")).context("player.h")?;
        let macros = Macros::read(decomp, &["include/player.h"])?;

        let model_types = members(&header, "PLAYER_MODELTYPE_");
        let group_names = members(&header, "PLAYER_MODELGROUP_")
            .into_iter()
            .filter(|n| !n.starts_with("ENTRY_"))
            .collect::<Vec<_>>();
        let shields = members(&header, "PLAYER_SHIELD_");
        let tunic_names = members(&header, "PLAYER_TUNIC_");

        let group_tables = find_initializer(&lib, "sPlayerDListGroups")?.flatten();
        if group_tables.len() != model_types.len() {
            bail!("sPlayerDListGroups has {} entries, expected {}", group_tables.len(), model_types.len());
        }
        let dl_groups = group_tables
            .iter()
            .map(|t| find_initializer(&lib, t).map(|i| names(&i)))
            .collect::<Result<Vec<_>>>()?;

        let ty = |s: &str| -> Result<usize> {
            let n = s.strip_prefix("PLAYER_MODELTYPE_").unwrap_or(s);
            model_types.iter().position(|m| m == n).with_context(|| format!("unknown model type {s}"))
        };
        let rows = find_initializer(&lib, "gPlayerModelTypes")?;
        let model_groups = rows
            .list()
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let e = r.flatten();
                if e.len() != 5 {
                    bail!("gPlayerModelTypes row {i} has {} entries", e.len());
                }
                Ok(ModelGroup {
                    name: group_names.get(i).cloned().unwrap_or_else(|| i.to_string()),
                    left: ty(&e[1])?,
                    right: ty(&e[2])?,
                    sheath: ty(&e[3])?,
                    waist: ty(&e[4])?,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let tunics = find_initializer(&lib, "sTunicColors")?
            .list()
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let b = bytes(c, &macros)?;
                Ok((tunic_names.get(i).cloned().unwrap_or_default(), [b[0], b[1], b[2]]))
            })
            .collect::<Result<Vec<_>>>()?;
        let eye_mouth_indices = find_initializer(&lib, "sPlayerFaces")?
            .list()
            .iter()
            .map(|p| bytes(p, &macros).map(|b| [b[0], b[1]]))
            .collect::<Result<Vec<_>>>()?;

        Ok(PlayerRules {
            dl_groups,
            model_groups,
            shields,
            tunics,
            eye_textures: find_initializer(&lib, "sEyeTextures")?.flatten(),
            mouth_textures: find_initializer(&lib, "sMouthTextures")?.flatten(),
            eye_mouth_indices,
            model_types,
        })
    }
}

fn members(header: &str, prefix: &str) -> Vec<String> {
    enum_members(header, prefix)
        .into_iter()
        .filter(|m| !m.ends_with("_MAX"))
        .map(|m| m[prefix.len()..].to_string())
        .collect()
}

fn names(init: &Init) -> Vec<Option<String>> {
    init.flatten().into_iter().map(|s| (s != "NULL").then_some(s)).collect()
}

/// Numbers or names (`{ PLAYER_EYES_OPEN, PLAYER_MOUTH_CLOSED }`), as bytes.
fn bytes(init: &Init, m: &Macros) -> Result<Vec<u8>> {
    init.flatten().iter().map(|s| m.eval(s).map(|v| v as u8).with_context(|| format!("can't evaluate {s}"))).collect()
}

/// Link's skeleton and object data for one age, ready to build draw lists.
pub struct PlayerModel {
    pub age: Age,
    pub object: Arc<[u8]>,
    pub gameplay_keep: Arc<[u8]>,
    pub skeleton: Skeleton,
    pub symbols: AssetFile,
    /// Offsets of `sEyeTextures` / `sMouthTextures` in this age's object. The table holds
    /// adult symbols; the child object has its textures at the same offsets.
    pub eye_offsets: Vec<usize>,
    pub mouth_offsets: Vec<usize>,
}

impl PlayerModel {
    pub fn load(p: &Project, rules: &PlayerRules, age: Age) -> Result<PlayerModel> {
        let symbols = p.symbols.file(age.object()).with_context(|| format!("{}.xml", age.object()))?.clone();
        let adult = p.symbols.file(Age::Adult.object()).context("object_link_boy.xml")?;
        let object = p.rom.file_by_name(age.object())?;
        let gameplay_keep = p.rom.file_by_name("gameplay_keep")?;
        let skel = symbols.of_kind("Skeleton").next().context("no skeleton in Link object")?;
        let skeleton = Skeleton::parse(&object, 6, skel.offset as usize, LimbType::Lod, true)?;
        let offsets = |list: &[String]| -> Result<Vec<usize>> {
            list.iter()
                .map(|n| adult.find(n).map(|s| s.offset as usize).with_context(|| format!("texture {n}")))
                .collect()
        };
        Ok(PlayerModel {
            age,
            eye_offsets: offsets(&rules.eye_textures)?,
            mouth_offsets: offsets(&rules.mouth_textures)?,
            object,
            gameplay_keep,
            skeleton,
            symbols,
        })
    }

    /// Segmented address of a display list in this age's object.
    pub fn dl_address(&self, name: &str) -> Option<u32> {
        self.symbols.find(name).map(|s| 0x0600_0000 | s.offset)
    }

    /// Builds the draw list for a loadout and face. Returns the list and any DL names that
    /// could not be found in the object's XML.
    pub fn draw_list(&self, rules: &PlayerRules, lo: &Loadout, eye: usize, mouth: usize, lod: usize) -> Result<(DrawList, Vec<String>)> {
        let mut missing = Vec::new();
        let limb_dlists = rules
            .limb_dlists(lo, lod)
            .into_iter()
            .map(|(limb, name)| {
                let addr = match name {
                    None => 0,
                    Some(n) => self.dl_address(&n).unwrap_or_else(|| {
                        missing.push(n);
                        0
                    }),
                };
                (limb, addr)
            })
            .collect();
        let tunic = rules.tunics.get(lo.tunic).map(|t| t.1).unwrap_or([30, 105, 27]);
        let opts = BuildOptions {
            lod,
            env_color: Some([tunic[0], tunic[1], tunic[2], 0]),
            bindings: vec![
                Binding { segment: 4, buf: self.gameplay_keep.clone(), base: 0 },
                Binding { segment: 6, buf: self.object.clone(), base: 0 },
                Binding { segment: 8, buf: self.object.clone(), base: self.eye_offsets[eye.min(self.eye_offsets.len() - 1)] },
                Binding { segment: 9, buf: self.object.clone(), base: self.mouth_offsets[mouth.min(self.mouth_offsets.len() - 1)] },
            ],
            limb_dlists,
            ..Default::default()
        };
        Ok((build_draw_list(&self.skeleton, &opts)?, missing))
    }
}

/// Every Player animation named in gameplay_keep's XML, decoded from link_animetion.
pub fn player_animations(p: &Project) -> Result<Vec<(String, LinkAnimation)>> {
    let keep_sym = p.symbols.file("gameplay_keep").context("gameplay_keep.xml")?;
    let keep = p.rom.file_by_name("gameplay_keep")?;
    let data = p.rom.file_by_name("link_animetion")?;
    let mut out = Vec::new();
    for s in keep_sym.of_kind("PlayerAnimation") {
        let o = s.offset as usize;
        if o + 8 > keep.len() {
            continue;
        }
        if let Ok(a) = LinkAnimation::parse(&keep[o..o + 8], &data) {
            out.push((s.name.clone(), a));
        }
    }
    Ok(out)
}
