//! Player's draw rules from `z_player_lib.c` and `player.h`: which display lists replace
//! the hand, sheath and waist limbs for each model group and shield
//! (`Player_OverrideLimbDrawGameplayDefault`), the eye/mouth textures bound to segments
//! 0x08/0x09, the tunic colours, and the blink timer.
//!
//! The tables (`PlayerRules`) come from the asset pack; `oot_import` reads them from the
//! decomp's C at import time. The rules that use them are Rust.
//!
//! Link's meshes are baked at import time, one per age and set of hand, sheath and waist lists
//! (`LinkVariant`, keyed by `PlayerRules::limb_dlists`: every model group, hand state, shield and
//! the child's sword on B or not give one of them), with the default eyes and mouth. The face is a texture swap: segments 8
//! and 9 only ever provide the eye and mouth textures, so `LinkVariant::with_face` replaces
//! the textures that came from them with the ones for this frame's face (`LinkFaces`). The
//! importer checks the result equals interpreting the display lists with that face bound.

use eng_gfx::{DrawList, TextureImage};

/// Skeleton limb indices (`PLAYER_LIMB_*` minus one, since the enum starts at NONE).
pub const LIMB_WAIST: u8 = 1;
pub const LIMB_L_HAND: u8 = 15;
pub const LIMB_R_HAND: u8 = 18;
pub const LIMB_SHEATH: u8 = 19;

/// Child Link plays adult animations with the root translation scaled by this
/// (`Player_OverrideLimbDrawGameplayCommon`).
pub const CHILD_ROOT_SCALE: f32 = 0.64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
    /// `adult` / `child`, as asset names use it.
    pub fn name(self) -> &'static str {
        match self {
            Age::Adult => "adult",
            Age::Child => "child",
        }
    }
    pub fn from_adult(adult: bool) -> Age {
        if adult { Age::Adult } else { Age::Child }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ModelGroup {
    /// `PLAYER_MODELGROUP_*` without the prefix.
    pub name: String,
    /// Indices into `PlayerRules::model_types`.
    pub left: usize,
    pub right: usize,
    pub sheath: usize,
    pub waist: usize,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
    /// `sPlayerFaces`: default eye and mouth per `actor.shape.face`.
    pub eye_mouth_indices: Vec<[u8; 2]>,
}

impl PlayerRules {
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
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Loadout {
    pub age: Age,
    pub model_group: usize,
    /// Index into `PlayerRules::shields` (`currentShield`).
    pub shield: usize,
    pub tunic: usize,
    /// Child only: `gSaveContext.equips.buttonItems[0] == ITEM_SWORD_KOKIRI` (otherwise the
    /// sheath is drawn without the sword).
    pub child_has_kokiri_sword: bool,
    /// `actor.speed > 2`: open hands are drawn as fists while running.
    pub moving_fast: bool,
}

impl Loadout {
    /// The mesh record this loadout is drawn with.
    pub fn variant_key(&self, rules: &PlayerRules) -> String {
        crate::pack::keys::link_variant(self.age, &rules.limb_dlists(self, 0))
    }

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

/// Link's blink timer (`FaceChange_UpdateBlinking(faceChange, 20, 80, 6)`), stepped once per game frame.
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

/// Link's mesh for one age, model group and hand state, as `SkelAnime_DrawFlexLod` with
/// `Player_OverrideLimbDrawGameplayDefault` draws it with the default face.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LinkVariant {
    pub draw: DrawList,
    /// Indices into `draw.textures` of the textures loaded from segment 8 (eyes) and 9 (mouth),
    /// in order.
    pub eye_slots: Vec<usize>,
    pub mouth_slots: Vec<usize>,
    /// This variant's own face textures, when a slot also holds texels its hand or sheath
    /// lists left in TMEM (the shared `LinkFaces` are the default loadout's).
    pub faces: Option<LinkFaces>,
}

/// The textures segments 8 and 9 give for each eye and mouth index (`sEyeTextures`,
/// `sMouthTextures`), decoded the way the variants' materials sample them: `eyes[e][k]` goes
/// into a variant's `eye_slots[k]`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LinkFaces {
    pub eyes: Vec<Vec<TextureImage>>,
    pub mouths: Vec<Vec<TextureImage>>,
}

impl LinkVariant {
    /// The mesh with `eye` and `mouth` bound (`Player_DrawImpl`'s `gSPSegment(0x08, ..)` and
    /// `gSPSegment(0x09, ..)`).
    pub fn with_face(&self, faces: &LinkFaces, eye: usize, mouth: usize) -> DrawList {
        let faces = self.faces.as_ref().unwrap_or(faces);
        let mut d = self.draw.clone();
        let pick = |set: &[Vec<TextureImage>], i: usize| set.get(i.min(set.len().saturating_sub(1))).cloned().unwrap_or_default();
        for (k, &slot) in self.eye_slots.iter().enumerate() {
            if let Some(t) = pick(&faces.eyes, eye).get(k) {
                d.textures[slot] = t.clone();
            }
        }
        for (k, &slot) in self.mouth_slots.iter().enumerate() {
            if let Some(t) = pick(&faces.mouths, mouth).get(k) {
                d.textures[slot] = t.clone();
            }
        }
        d
    }

    /// The indices of `draw`'s textures with texels loaded from `segment`.
    pub fn slots_from(draw: &DrawList, segment: u8) -> Vec<usize> {
        draw.textures.iter().enumerate().filter(|(_, t)| t.source_segments & (1 << segment) != 0).map(|(i, _)| i).collect()
    }
}
