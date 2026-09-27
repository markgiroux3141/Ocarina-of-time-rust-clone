//! Player (Link) draw rules, read from the decomp's `z_player_lib.c` and `z64player.h`
//! rather than copied: which display lists replace the hand, sheath and waist limbs for each
//! model group and shield, the eye/mouth textures bound to segments 0x08/0x09, and the tunic
//! colours. `PlayerModel` applies them the way `Player_DrawImpl` and
//! `Player_OverrideLimbDrawGameplayDefault` do.

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result, bail};

use crate::anim::LinkAnimation;
use crate::csrc::{Init, enum_members, find_initializer, strip_comments};
use crate::gbi::DrawList;
use crate::model::{Binding, BuildOptions, build_draw_list};
use crate::project::Project;
use crate::skeleton::{LimbType, Skeleton};
use crate::symbols::AssetFile;

/// Skeleton limb indices (`PLAYER_LIMB_*` minus one, since the enum starts at NONE).
pub const LIMB_WAIST: u8 = 1;
pub const LIMB_L_HAND: u8 = 15;
pub const LIMB_R_HAND: u8 = 18;
pub const LIMB_SHEATH: u8 = 19;

/// Child Link plays adult animations with the root translation scaled by this
/// (`Player_OverrideLimbDrawGameplayCommon`).
pub const CHILD_ROOT_SCALE: f32 = 0.64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Age {
    Adult = 0,
    Child = 1,
}

impl Age {
    pub fn object(self) -> &'static str {
        match self {
            Age::Adult => "object_link_boy",
            Age::Child => "object_link_child",
        }
    }
    pub fn root_scale(self) -> f32 {
        match self {
            Age::Adult => 1.0,
            Age::Child => CHILD_ROOT_SCALE,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModelGroup {
    /// `PLAYER_MODELGROUP_*` without the prefix.
    pub name: String,
    /// Indices into `PlayerRules::model_types`.
    pub left: usize,
    pub right: usize,
    pub sheath: usize,
    pub waist: usize,
}

#[derive(Debug, Clone)]
pub struct PlayerRules {
    /// `PLAYER_MODELTYPE_*` without the prefix, in enum order.
    pub model_types: Vec<String>,
    /// Display-list names for each model type (`sPlayerDListGroups`), laid out as
    /// `variant * 4 + lod * 2 + age`. `None` is a NULL entry.
    pub dl_groups: Vec<Vec<Option<String>>>,
    pub model_groups: Vec<ModelGroup>,
    /// `PLAYER_SHIELD_*` without the prefix.
    pub shields: Vec<String>,
    pub tunics: Vec<(String, [u8; 3])>,
    /// `sEyeTextures` / `sMouthTextures` as shipped: adult symbols, used for both ages.
    pub eye_textures: Vec<String>,
    pub mouth_textures: Vec<String>,
    /// `sEyeMouthIndices`: default eye and mouth per `actor.shape.face`.
    pub eye_mouth_indices: Vec<[u8; 2]>,
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

fn bytes(init: &Init) -> Result<Vec<u8>> {
    init.flatten().iter().map(|s| Init::Atom(s.clone()).as_int().map(|v| v as u8).context("non-numeric entry")).collect()
}

impl PlayerRules {
    pub fn load(decomp: &Path) -> Result<PlayerRules> {
        let lib_path = decomp.join("src/code/z_player_lib.c");
        let lib = strip_comments(&std::fs::read_to_string(&lib_path).with_context(|| lib_path.display().to_string())?);
        let header = std::fs::read_to_string(decomp.join("include/z64player.h")).context("z64player.h")?;

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
                let b = bytes(c)?;
                Ok((tunic_names.get(i).cloned().unwrap_or_default(), [b[0], b[1], b[2]]))
            })
            .collect::<Result<Vec<_>>>()?;
        let eye_mouth_indices = find_initializer(&lib, "sEyeMouthIndices")?
            .list()
            .iter()
            .map(|p| bytes(p).map(|b| [b[0], b[1]]))
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

    pub fn model_type(&self, name: &str) -> usize {
        self.model_types.iter().position(|m| m == name).unwrap_or(usize::MAX)
    }

    pub fn model_group(&self, name: &str) -> Option<usize> {
        self.model_groups.iter().position(|g| g.name == name)
    }

    pub fn shield(&self, name: &str) -> usize {
        self.shields.iter().position(|s| s == name).unwrap_or(0)
    }

    fn pick(&self, ty: usize, variant: usize, age: Age, lod: usize) -> Option<String> {
        self.dl_groups.get(ty)?.get(variant * 4 + lod * 2 + age as usize).cloned().flatten()
    }

    /// Display lists for the hand, sheath and waist limbs, following
    /// `Player_OverrideLimbDrawGameplayDefault`. `None` means the limb draws nothing.
    pub fn limb_dlists(&self, lo: &Loadout, lod: usize) -> Vec<(u8, Option<String>)> {
        let g = &self.model_groups[lo.model_group.min(self.model_groups.len() - 1)];
        let t = |n: &str| self.model_type(n);
        let shield_max = self.shields.len();

        let mut left = g.left;
        if left == t("LH_OPEN") && lo.moving_fast {
            left = t("LH_CLOSED");
        }

        let (mut right, mut right_variant) = (g.right, 0);
        if right == t("RH_SHIELD") {
            right_variant = lo.shield;
        } else if right == t("RH_OPEN") && lo.moving_fast {
            right = t("RH_CLOSED");
        }

        let (mut sheath, mut sheath_variant) = (g.sheath, 0);
        let no_kokiri_sword = lo.age == Age::Child && !lo.child_has_kokiri_sword;
        if sheath == t("SHEATH_18") || sheath == t("SHEATH_19") {
            sheath_variant = lo.shield;
            if no_kokiri_sword && lo.shield < self.shield("HYLIAN") {
                sheath_variant += shield_max;
            }
        } else if no_kokiri_sword && (sheath == t("SHEATH_16") || sheath == t("SHEATH_17")) {
            sheath = t("SHEATH_18");
            sheath_variant = shield_max;
        }

        vec![
            (LIMB_L_HAND, self.pick(left, 0, lo.age, lod)),
            (LIMB_R_HAND, self.pick(right, right_variant, lo.age, lod)),
            (LIMB_SHEATH, self.pick(sheath, sheath_variant, lo.age, lod)),
            (LIMB_WAIST, self.pick(g.waist, 0, lo.age, lod)),
        ]
    }

    /// Eye and mouth texture indices for a frame, as in `Player_DrawImpl`: the animation's
    /// face field wins, otherwise the blink state (`actor.shape.face`) picks the default.
    pub fn face_indices(&self, anim_face: u16, blink_face: usize) -> (usize, usize) {
        let d = self.eye_mouth_indices.get(blink_face).copied().unwrap_or([0, 0]);
        let eye = (anim_face & 0xF) as i32 - 1;
        let mouth = (anim_face >> 4) as i32 - 1;
        (
            if eye < 0 { d[0] as usize } else { eye as usize },
            if mouth < 0 { d[1] as usize } else { mouth as usize },
        )
    }
}

/// The equipment state that decides Player's limb display lists.
#[derive(Debug, Clone, PartialEq)]
pub struct Loadout {
    pub age: Age,
    pub model_group: usize,
    /// Index into `PlayerRules::shields` (`currentShield`).
    pub shield: usize,
    pub tunic: usize,
    /// Child only: Kokiri Sword on the B button (otherwise the sheath is drawn without it).
    pub child_has_kokiri_sword: bool,
    /// `actor.speedXZ > 2`: open hands are drawn as fists while running.
    pub moving_fast: bool,
}

impl Loadout {
    /// Standing with nothing in hand, sword and shield on the back.
    pub fn default_for(rules: &PlayerRules, age: Age) -> Loadout {
        Loadout {
            age,
            model_group: rules.model_group("DEFAULT").unwrap_or(0),
            shield: rules.shield(if age == Age::Adult { "HYLIAN" } else { "DEKU" }),
            tunic: 0,
            child_has_kokiri_sword: true,
            moving_fast: false,
        }
    }
}

/// Link's blink timer (`func_80032CB4(unk_3A8, 20, 80, 6)`), stepped once per game frame.
#[derive(Debug, Clone)]
pub struct Blinker {
    timer: i16,
    rng: u32,
}

impl Default for Blinker {
    fn default() -> Self {
        Blinker { timer: 40, rng: 0x1234_5678 }
    }
}

impl Blinker {
    /// Advances one frame and returns the face: 0 open, 1 half, 2 closed.
    pub fn step(&mut self) -> usize {
        self.timer -= 1;
        if self.timer <= 0 {
            self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            self.timer = 20 + ((self.rng >> 8) % 80) as i16;
        }
        let t = self.timer - 6;
        if t > 0 {
            0
        } else if t > -2 || self.timer < 2 {
            1
        } else {
            2
        }
    }
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
