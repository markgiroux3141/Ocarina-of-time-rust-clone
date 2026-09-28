//! `En_Ko` (`ovl_En_Ko/z_en_ko.c`): the Kokiri children, and Fado. `params & 0xFF` is the child
//! (`ENKO_TYPE_CHILD_0` .. `_11`, `_FADO` 12); `params >> 8` a path.
//!
//! A child waits for its objects (the skeleton of `object_km1` for a boy or `object_kw1` for a
//! girl, Fado's head in `object_fa`, and the animations in `object_os_anime`), then stands and
//! plays the animation its type and the story's progress pick (`sOsAnimeLookup`), turns its
//! head and torso to Link (`func_80034A14`), blinks, and offers to talk. In Kokiri Forest and
//! the Lost Woods it fades out beyond `appearDist` of Link (`func_80A98DB4`), and then can't be
//! targeted. Child 3 guards the way to the Lost Woods until Link has the Kokiri Emerald
//! (`func_80A995CC`).
//!
//! Drawn as `EnKo_Draw` does: the skeleton with the tunic and boots colours on segments 8 and 9,
//! the eyes on 0x0A, opaque at full alpha (`func_80034BA0`) and translucent while fading
//! (`func_80034CC4`), from meshes baked per head, eye and pass (docs/adr/0012-actor-bakes.md).
//!
//! Not ported: talking (the Player side and the message box), Fado's saw trade in the Lost Woods,
//! paths (child 3 with the emerald moves to its path's last point: paths aren't in the pack),
//! and the fairy each child has (`En_Elf` params 3: a placeholder).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey, SegmentValues};
use eng_math::{binang_to_rad, cos_s, sin_s};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_0, ACTOR_FLAG_3, ACTOR_FLAG_4, Actor, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile};
use oot_game::collision_check::*;
use oot_game::npc::{NpcTrack, func_80034a14, func_80034f54};
use oot_game::pack::{BakeBody, BakeSegment, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::save::{EVENTCHKINF_40, QUEST_KOKIRI_EMERALD, QUEST_MEDALLION_FOREST};
use oot_game::skelanime_std::*;

pub const ACTOR_EN_KO: i16 = 0x0163;
/// `ACTOR_EN_ELF`: the child's fairy (params 3).
const ACTOR_EN_ELF: i16 = 0x0018;

/// `En_Ko_InitVars`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_KO, name: "En_Ko", category: ACTORCAT_NPC, flags: ACTOR_FLAG_0 | ACTOR_FLAG_3 | ACTOR_FLAG_4, object: "gameplay_keep" };

// `OBJECT_*` (`object_table.h`).
const OBJECT_OS_ANIME: i16 = 0x00C5;
const OBJECT_KM1: i16 = 0x00FC;
const OBJECT_KW1: i16 = 0x00FD;
const OBJECT_FA: i16 = 0x013D;

// `SCENE_*`.
const SCENE_KOKIRI_HOME: u16 = 0x26;
const SCENE_KOKIRI_HOME3: u16 = 0x27;
const SCENE_KOKIRI_HOME4: u16 = 0x28;
const SCENE_KOKIRI_HOME5: u16 = 0x29;
const SCENE_KOKIRI_SHOP: u16 = 0x2D;
const SCENE_SPOT04: u16 = 0x55;
const SCENE_SPOT10: u16 = 0x5B;

// `KokiriChildren`.
pub const ENKO_TYPE_CHILD_0: u8 = 0;
pub const ENKO_TYPE_CHILD_1: u8 = 1;
pub const ENKO_TYPE_CHILD_2: u8 = 2;
pub const ENKO_TYPE_CHILD_3: u8 = 3;
pub const ENKO_TYPE_CHILD_4: u8 = 4;
pub const ENKO_TYPE_CHILD_5: u8 = 5;
pub const ENKO_TYPE_CHILD_6: u8 = 6;
pub const ENKO_TYPE_CHILD_7: u8 = 7;
pub const ENKO_TYPE_CHILD_8: u8 = 8;
pub const ENKO_TYPE_CHILD_9: u8 = 9;
pub const ENKO_TYPE_CHILD_10: u8 = 10;
pub const ENKO_TYPE_CHILD_11: u8 = 11;
pub const ENKO_TYPE_CHILD_FADO: u8 = 12;
const ENKO_TYPE_CHILD_MAX: u8 = 13;

// `KokiriForestQuestState`.
pub const ENKO_FQS_CHILD_START: usize = 0;
pub const ENKO_FQS_CHILD_STONE: usize = 1;
pub const ENKO_FQS_CHILD_SARIA: usize = 2;
pub const ENKO_FQS_ADULT_ENEMY: usize = 3;
pub const ENKO_FQS_ADULT_SAVED: usize = 4;

// `KokiriGender`: the index into `sHead` / `sSkeleton`.
const KO_BOY: usize = 0;
const KO_GIRL: usize = 1;
const KO_FADO: usize = 2;

// `INFTABLE_*`.
const INFTABLE_1E: u16 = 0x1E;
const INFTABLE_22: u16 = 0x22;
const INFTABLE_24: u16 = 0x24;
const INFTABLE_26: u16 = 0x26;
const INFTABLE_28: u16 = 0x28;
const INFTABLE_41: u16 = 0x41;
const INFTABLE_47: u16 = 0x47;
const INFTABLE_51: u16 = 0x51;
const INFTABLE_59: u16 = 0x59;
const INFTABLE_61: u16 = 0x61;
const INFTABLE_B7: u16 = 0xB7;

/// `sCylinderInit`.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COLTYPE_NONE, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderInfoInit {
        elem_type: ELEMTYPE_UNK0,
        toucher: ColliderTouch { dmg_flags: 0, effect: 0, damage: 0 },
        bumper: ColliderBumpInit { dmg_flags: 0, effect: 0, defense: 0 },
        toucher_flags: TOUCH_NONE,
        bumper_flags: BUMP_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 20, height: 46, y_shift: 0, pos: [0; 3] },
};

/// `sColChkInfoInit`.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit2 = CollisionCheckInfoInit2 { health: 0, cyl_radius: 0, cyl_height: 0, cyl_y_shift: 0, mass: MASS_IMMOVABLE };

/// `sHead`: the head's object, list and eye textures (`NULL` for the boys).
const HEAD_OBJECTS: [i16; 3] = [OBJECT_KM1, OBJECT_KW1, OBJECT_FA];
const HEAD_EYES: [Option<[&str; 3]>; 3] = [None, Some(["gKw1EyeOpenTex", "gKw1EyeHalfTex", "gKw1EyeClosedTex"]), Some(["gFaEyeOpenTex", "gFaEyeHalfTex", "gFaEyeClosedTex"])];
/// `sSkeleton`: the object and the skeleton, for a boy and a girl.
const SKELETONS: [(i16, &str, &str); 2] = [(OBJECT_KM1, "object_km1", "gKm1Skel"), (OBJECT_KW1, "object_kw1", "gKw1Skel")];

/// `EnKoModelInfo`: head, body, tunic colour, legs, boots colour.
struct ModelInfo {
    head: usize,
    body: usize,
    tunic: [u8; 4],
    legs: usize,
    boots: [u8; 4],
}

const BOY: ModelInfo = ModelInfo { head: KO_BOY, body: KO_BOY, tunic: [0, 130, 70, 255], legs: KO_BOY, boots: [110, 170, 20, 255] };
const GIRL: ModelInfo = ModelInfo { head: KO_GIRL, body: KO_GIRL, tunic: [70, 190, 60, 255], legs: KO_GIRL, boots: [100, 30, 0, 255] };
/// `sModelInfo` by type.
const MODEL_INFO: [ModelInfo; 13] = [
    BOY,
    GIRL,
    BOY,
    BOY,
    BOY,
    GIRL,
    GIRL,
    BOY,
    BOY,
    GIRL,
    GIRL,
    BOY,
    ModelInfo { head: KO_FADO, body: KO_GIRL, tunic: [70, 190, 60, 255], legs: KO_GIRL, boots: [100, 30, 0, 255] },
];

/// `sInteractInfo`: target mode, look distance (plus the collider's radius), appear distance.
const INTERACT_INFO: [(u8, f32, f32); 13] = [
    (6, 30.0, 180.0),
    (6, 30.0, 180.0),
    (6, 30.0, 180.0),
    (6, 30.0, 180.0),
    (6, 30.0, 180.0),
    (1, 30.0, 240.0),
    (6, 30.0, 180.0),
    (6, 30.0, 180.0),
    (6, 30.0, 180.0),
    (6, 30.0, 180.0),
    (6, 30.0, 180.0),
    (6, 30.0, 180.0),
    (6, 30.0, 180.0),
];

/// `sAnimationInfo`: animation (`object_os_anime`), speed, start, end, mode, morph frames.
const ANIMATION_INFO: [(&str, f32, f32, f32, u8, f32); 34] = [
    ("gObjOsAnim_8F6C", 1.0, 2.0, 14.0, ANIMMODE_LOOP_PARTIAL, 0.0),
    ("gObjOsAnim_8F6C", 0.0, 1.0, 1.0, ANIMMODE_LOOP_PARTIAL, 0.0),
    ("gObjOsAnim_9B64", 0.0, 0.0, 0.0, ANIMMODE_ONCE, 0.0),
    ("gObjOsAnim_9B64", 0.0, 1.0, 1.0, ANIMMODE_ONCE, 0.0),
    ("gObjOsAnim_9B64", 0.0, 2.0, 2.0, ANIMMODE_ONCE, 0.0),
    ("gObjOsAnim_62DC", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_62DC", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -10.0),
    ("gObjOsAnim_5808", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -10.0),
    ("gObjOsAnim_7830", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_8178", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_65E0", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_879C", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_7FFC", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_80B4", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_91AC", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_6F9C", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_7064", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_7120", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_7F38", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_7D94", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_6EE0", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_98EC", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_90EC", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_982C", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_9274", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_99A4", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_9028", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_7E64", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_7454", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_8F6C", 0.0, 1.0, 1.0, ANIMMODE_LOOP_PARTIAL, -8.0),
    ("gObjOsAnim_7D94", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -8.0),
    ("gObjOsAnim_879C", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -8.0),
    ("gObjOsAnim_6A60", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -8.0),
    ("gObjOsAnim_7830", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -8.0),
];

// `EnKoAnimation` indices used by name.
const ENKO_ANIM_29: usize = 29;
const ENKO_ANIM_30: usize = 30;
const ENKO_ANIM_31: usize = 31;
const ENKO_ANIM_32: usize = 32;
const ENKO_ANIM_33: usize = 33;

/// `sOsAnimeLookup[type][forestQuestState]`.
const OS_ANIME_LOOKUP: [[u8; 5]; 13] = [
    [8, 9, 9, 14, 11],
    [2, 12, 2, 13, 13],
    [11, 11, 11, 15, 9],
    [0, 16, 16, 17, 18],
    [19, 19, 20, 10, 9],
    [3, 3, 3, 3, 3],
    [4, 22, 22, 4, 23],
    [24, 16, 16, 25, 16],
    [26, 15, 15, 26, 15],
    [3, 3, 3, 27, 27],
    [2, 2, 2, 2, 22],
    [14, 14, 14, 14, 14],
    [5, 5, 5, 5, 5],
];

/// `D_80A9A62C` (`func_80A97BC0`): the height offset the head tracking looks from.
const EYE_HEIGHTS: [[f32; 5]; 13] = [
    [0.0, 0.0, 0.0, -30.0, -20.0],
    [0.0, 0.0, 0.0, -20.0, -10.0],
    [0.0, 0.0, 0.0, -30.0, -20.0],
    [-10.0, 10.0, 10.0, -10.0, -30.0],
    [0.0, 0.0, 0.0, -10.0, -20.0],
    [0.0, 0.0, 0.0, -20.0, -20.0],
    [0.0, 0.0, 0.0, -10.0, -20.0],
    [10.0, 10.0, 10.0, -60.0, -20.0],
    [-10.0, -10.0, -20.0, -30.0, -30.0],
    [-10.0, -10.0, -10.0, -40.0, -40.0],
    [0.0, 0.0, 0.0, -10.0, -20.0],
    [-10.0, -10.0, -20.0, -30.0, -30.0],
    [0.0, 0.0, 0.0, -20.0, -20.0],
];

/// `D_80A9A730` (`func_80A97C7C`): whether the child falls to the floor.
const FALLS: [[u8; 5]; 13] = [
    [1, 1, 1, 0, 1],
    [1, 1, 1, 1, 1],
    [1, 1, 1, 0, 1],
    [1, 1, 1, 0, 1],
    [1, 1, 1, 0, 1],
    [0, 0, 0, 0, 0],
    [1, 1, 1, 1, 1],
    [1, 1, 1, 0, 1],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [1, 1, 1, 1, 1],
    [0, 0, 0, 0, 0],
    [1, 1, 1, 1, 1],
];

/// The segments `EnKo_Draw` binds: 8 the tunic's env colour, 9 the boots', 0x0E the
/// `gDPSetEnvColor(0, 0, 0, alpha)` of `func_80034BA0` / `func_80034CC4`.
const SEG_TUNIC: u8 = 0x08;
const SEG_BOOTS: u8 = 0x09;
const SEG_ALPHA: u8 = 0x0E;
/// The eyes (`gSPSegment(0x0A, eyeTexture)`) and the render mode (`gSPSegment(0x0C, ...)`).
const SEG_EYES: u8 = 0x0A;
const SEG_RENDER_MODE: u8 = 0x0C;

/// `func_80034B54`: `gDPSetRenderMode(G_RM_FOG_SHADE_A, AA_EN | Z_CMP | Z_UPD | IM_RD |
/// CLR_ON_CVG | CVG_DST_WRAP | ZMODE_XLU | FORCE_BL | GBL_c2(G_BL_CLR_IN, G_BL_A_IN,
/// G_BL_CLR_MEM, G_BL_1MA))`: `G_SETOTHERMODE_L`, shift 3, length 29.
const XLU_RENDER_MODE: (u32, u32) = (0xE200_001C, 0xC810_49F8);

/// The bake of a head (`KO_*`), an eye (0..2) and a pass.
fn bake_name(head: usize, eye: usize, xlu: bool) -> String {
    let pass = if xlu { "xlu" } else { "opa" };
    match head {
        KO_BOY => format!("En_Ko/km1_{pass}"),
        KO_GIRL => format!("En_Ko/kw1_{pass}_eye{eye}"),
        _ => format!("En_Ko/fa_{pass}_eye{eye}"),
    }
}

/// The meshes `EnKo_Draw` draws: each head with each eye (the boys have none), opaque and
/// translucent.
pub fn bakes() -> Vec<MeshBake> {
    let mut v = Vec::new();
    for head in [KO_BOY, KO_GIRL, KO_FADO] {
        let eyes = if head == KO_BOY { 1 } else { 3 };
        for eye in 0..eyes {
            for xlu in [false, true] {
                // The legs' (and body's) skeleton: a boy's for the boys, a girl's for the girls and Fado.
                let (object, skel) = if head == KO_BOY { ("object_km1", "gKm1Skel") } else { ("object_kw1", "gKw1Skel") };
                let mut segments = vec![
                    (SEG_TUNIC, BakeSegment::DynamicColor { env: true, prim: false }),
                    (SEG_BOOTS, BakeSegment::DynamicColor { env: true, prim: false }),
                    (SEG_ALPHA, BakeSegment::DynamicColor { env: true, prim: false }),
                    (SEG_RENDER_MODE, BakeSegment::Commands(if xlu { vec![XLU_RENDER_MODE] } else { Vec::new() })),
                ];
                if let Some(e) = HEAD_EYES[head] {
                    let file = if head == KO_FADO { "object_fa" } else { "object_kw1" };
                    segments.push((SEG_EYES, BakeSegment::Texture { file: file.into(), symbol: e[eye].into() }));
                }
                // EnKo_OverrideLimbDraw, limb 15: the head's list (a boy's and a girl's are their
                // skeleton's own; Fado's is from object_fa, drawn with it on segment 6).
                let limbs = if head == KO_FADO { vec![LimbOverride { limb: 14, file: "object_fa".into(), symbol: "gFaDL".into() }] } else { Vec::new() };
                v.push(MeshBake {
                    name: bake_name(head, eye, xlu),
                    object: object.into(),
                    segments,
                    prelude: vec![SEG_ALPHA],
                    body: BakeBody::Skeleton { file: object.into(), symbol: skel.into(), limbs },
                });
            }
        }
    }
    v
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_80A99048`: waiting for the objects.
    WaitObjects,
    /// `func_80A99384`: standing (Fado's trade aside).
    Idle,
    /// `func_80A995CC`: child 3 guarding the Lost Woods.
    Guard,
}

pub struct EnKo {
    pub actor: Actor,
    pub skel: Option<SkelAnimeStd>,
    pub skeleton: Option<Arc<Skeleton>>,
    pub action: Action,
    pub head_bank: Option<usize>,
    pub body_bank: Option<usize>,
    pub legs_bank: Option<usize>,
    pub os_anime_bank: Option<usize>,
    pub collider: ColliderCylinder,
    /// `unk_1E8`.
    pub unk_1e8: NpcTrack,
    pub forest_quest_state: usize,
    pub blink_timer: i16,
    pub eye_texture_index: i16,
    pub appear_dist: f32,
    pub look_dist: f32,
    pub model_alpha: f32,
    pub unk_2e4: [i16; 16],
    pub unk_304: [i16; 16],
    animations: Vec<AnimationInfo>,
}

impl EnKo {
    fn ty(&self) -> u8 {
        (self.actor.params & 0xFF) as u8
    }

    fn model(&self) -> &'static ModelInfo {
        &MODEL_INFO[self.ty() as usize]
    }

    /// `EnKo_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut k = EnKo {
            actor,
            skel: None,
            skeleton: None,
            action: Action::WaitObjects,
            head_bank: None,
            body_bank: None,
            legs_bank: None,
            os_anime_bank: None,
            collider: ColliderCylinder::default(),
            unk_1e8: NpcTrack::default(),
            forest_quest_state: 0,
            blink_timer: 0,
            eye_texture_index: 0,
            appear_dist: 0.0,
            look_dist: 0.0,
            model_alpha: 0.0,
            unk_2e4: [0; 16],
            unk_304: [0; 16],
            animations: Vec::new(),
        };
        if k.ty() >= ENKO_TYPE_CHILD_MAX || !k.is_os_anime_available(play) || !k.are_objects_available(play) {
            k.actor.kill();
            return Box::new(k);
        }
        if !k.can_spawn(play) {
            k.actor.kill();
        }
        k.action = Action::WaitObjects;
        Box::new(k)
    }

    /// `EnKo_AreObjectsAvailable`.
    fn are_objects_available(&mut self, play: &PlayState) -> bool {
        let m = self.model();
        self.legs_bank = play.object_ctx.get_index(SKELETONS[m.legs].0);
        self.body_bank = play.object_ctx.get_index(SKELETONS[m.body].0);
        self.head_bank = play.object_ctx.get_index(HEAD_OBJECTS[m.head]);
        self.legs_bank.is_some() && self.body_bank.is_some() && self.head_bank.is_some()
    }

    /// `EnKo_AreObjectsLoaded`.
    fn are_objects_loaded(&self, play: &PlayState) -> bool {
        [self.legs_bank, self.body_bank, self.head_bank].iter().all(|b| b.is_some_and(|b| play.object_ctx.is_loaded(b)))
    }

    /// `EnKo_IsOsAnimeAvailable`, `EnKo_IsOsAnimeLoaded`.
    fn is_os_anime_available(&mut self, play: &PlayState) -> bool {
        self.os_anime_bank = play.object_ctx.get_index(OBJECT_OS_ANIME);
        self.os_anime_bank.is_some()
    }
    fn is_os_anime_loaded(&self, play: &PlayState) -> bool {
        self.os_anime_bank.is_some_and(|b| play.object_ctx.is_loaded(b))
    }

    /// `EnKo_CanSpawn`: whether this child is in this scene at this point of the story.
    fn can_spawn(&self, play: &PlayState) -> bool {
        let ty = self.ty();
        let s = &play.save;
        let adult = s.adult;
        let medallion = s.check_quest_item(QUEST_MEDALLION_FOREST);
        match play.scene_id {
            SCENE_SPOT04 => !(ty >= ENKO_TYPE_CHILD_7 && ty != ENKO_TYPE_CHILD_FADO) && !(!medallion && adult),
            SCENE_KOKIRI_HOME => matches!(ty, ENKO_TYPE_CHILD_7 | ENKO_TYPE_CHILD_8 | ENKO_TYPE_CHILD_11),
            SCENE_KOKIRI_HOME3 => {
                if adult && !medallion {
                    matches!(ty, ENKO_TYPE_CHILD_1 | ENKO_TYPE_CHILD_9)
                } else {
                    ty == ENKO_TYPE_CHILD_9
                }
            }
            SCENE_KOKIRI_HOME4 => adult && !medallion && matches!(ty, ENKO_TYPE_CHILD_0 | ENKO_TYPE_CHILD_4),
            SCENE_KOKIRI_HOME5 => adult && !medallion && ty == ENKO_TYPE_CHILD_6,
            SCENE_KOKIRI_SHOP => {
                if adult && !medallion {
                    matches!(ty, ENKO_TYPE_CHILD_5 | ENKO_TYPE_CHILD_10)
                } else {
                    ty == ENKO_TYPE_CHILD_10
                }
            }
            // INV_CONTENT(ITEM_TRADE_ADULT) == ITEM_ODD_POTION: no inventory yet.
            SCENE_SPOT10 => false,
            _ => false,
        }
    }

    /// `EnKo_GetForestQuestState`.
    fn forest_quest_state(play: &PlayState) -> usize {
        let s = &play.save;
        if !s.adult {
            if s.get_event_chk_inf(EVENTCHKINF_40) {
                return ENKO_FQS_CHILD_SARIA;
            }
            if s.check_quest_item(QUEST_KOKIRI_EMERALD) {
                return ENKO_FQS_CHILD_STONE;
            }
            return ENKO_FQS_CHILD_START;
        }
        if s.check_quest_item(QUEST_MEDALLION_FOREST) { ENKO_FQS_ADULT_SAVED } else { ENKO_FQS_ADULT_ENEMY }
    }

    /// `EnKo_GetForestQuestState2` (the stone and Saria's letter checked the other way round).
    fn forest_quest_state2(play: &PlayState) -> usize {
        let s = &play.save;
        if s.adult {
            return if s.check_quest_item(QUEST_MEDALLION_FOREST) { ENKO_FQS_ADULT_SAVED } else { ENKO_FQS_ADULT_ENEMY };
        }
        if s.check_quest_item(QUEST_KOKIRI_EMERALD) {
            return if s.get_event_chk_inf(EVENTCHKINF_40) { ENKO_FQS_CHILD_SARIA } else { ENKO_FQS_CHILD_STONE };
        }
        ENKO_FQS_CHILD_START
    }

    /// `Animation_ChangeByInfo(&skelAnime, sAnimationInfo, i)`.
    fn change_anim(&mut self, i: usize) {
        if let (Some(s), Some(info)) = (&mut self.skel, self.animations.get(i)) {
            s.change_by_info(info);
        }
    }

    fn anim_is(&self, name: &str) -> bool {
        self.skel.as_ref().is_some_and(|s| s.is(name))
    }

    /// `func_80A99048`: once the objects are in, the skeleton, the collider, the animation, the
    /// fairy, and the first action.
    fn wait_objects(&mut self, play: &mut PlayState) {
        if !(self.is_os_anime_loaded(play) && self.are_objects_loaded(play)) {
            return;
        }
        let Some(assets) = play.assets.clone() else { return };
        self.actor.flags &= !ACTOR_FLAG_4;
        self.actor.obj_bank_index = self.legs_bank;
        let (_, file, sym) = SKELETONS[self.model().legs];
        let skeleton = match assets.skeleton(file, sym) {
            Ok(s) => s,
            Err(e) => {
                log::error!("En_Ko: {e:#}");
                self.actor.kill();
                return;
            }
        };
        // SkelAnime_InitFlex(..., 16): the skeleton's 15 limbs and the root.
        self.skel = Some(SkelAnimeStd::init_flex(skeleton.limbs.len(), None));
        self.skeleton = Some(skeleton);
        self.animations = ANIMATION_INFO
            .iter()
            .filter_map(|&(name, play_speed, start_frame, frame_count, mode, morph_frames)| {
                let animation = assets.animation("object_os_anime", name).map_err(|e| log::error!("En_Ko: {e:#}")).ok()?;
                Some(AnimationInfo { animation, play_speed, start_frame, frame_count, mode, morph_frames })
            })
            .collect();
        if self.animations.len() != ANIMATION_INFO.len() {
            self.actor.kill();
            return;
        }
        // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 18).
        self.actor.shape_y_offset = 0.0;
        self.collider = ColliderCylinder::new(&CYLINDER_INIT);
        self.actor.col_chk_info.set_info2(None, &COL_CHK_INFO_INIT);
        let ty = self.ty();
        if ty == ENKO_TYPE_CHILD_7 {
            let medallion = play.save.check_quest_item(QUEST_MEDALLION_FOREST);
            let want = if play.save.adult && !medallion { 1 } else { 0 };
            if self.actor.shape_rot.z != want {
                self.actor.kill();
                return;
            }
        }
        if ty == ENKO_TYPE_CHILD_5 {
            self.collider.base.oc_flags1 |= 0x40;
        }
        self.forest_quest_state = Self::forest_quest_state2(play);
        self.change_anim(OS_ANIME_LOOKUP[ty as usize][self.forest_quest_state] as usize);
        self.actor.scale = Vec3::splat(0.01);
        // func_80A98CD8.
        let (target_mode, look, appear) = INTERACT_INFO[ty as usize];
        self.actor.target_mode = target_mode;
        self.look_dist = look + self.collider.dim.radius as f32;
        self.appear_dist = appear;
        self.model_alpha = 0.0;
        // Path_GetByIndex(play, ENKO_PATH, 0xFF): paths aren't in the pack.
        let pos = self.actor.world_pos;
        if let Err(e) = play.actor_spawn_as_child(&mut self.actor, ACTOR_EN_ELF, pos, [0; 3], 3) {
            log::debug!("En_Ko's fairy: {e:?}");
        }
        if ty == ENKO_TYPE_CHILD_3 {
            if !play.save.check_quest_item(QUEST_KOKIRI_EMERALD) {
                self.collider.dim.height += 200;
                self.action = Action::Guard;
                return;
            }
            // Path_CopyLastPoint(this->path, &world.pos): no paths yet.
        }
        self.action = Action::Idle;
    }

    /// `func_80A995CC`: child 3 stays between Link and the way it guards, 80 from home.
    fn guard(&mut self, play: &PlayState) {
        let Some(pp) = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos) else { return };
        let home_yaw = oot_game::target::yaw_to(self.actor.home_pos, pp);
        self.actor.world_pos.x = self.actor.home_pos.x + 80.0 * sin_s(home_yaw);
        self.actor.world_pos.z = self.actor.home_pos.z + 80.0 * cos_s(home_yaw);
        self.actor.world_rot.y = self.actor.yaw_towards_player;
        self.actor.shape_rot.y = self.actor.yaw_towards_player;
        let Some(skel) = &mut self.skel else { return };
        if self.unk_1e8.talk_state == 0 || !self.actor.is_targeted {
            let t = (self.actor.yaw_towards_player as f32 - home_yaw as f32).abs() * 0.001 * 3.0;
            skel.play_speed = if t < 1.0 { 1.0 } else { t.min(3.0) };
        } else {
            skel.play_speed = 1.0;
        }
    }

    /// `EnKo_IsWithinTalkAngle`.
    fn is_within_talk_angle(&self) -> bool {
        let d = (self.actor.yaw_towards_player as f32 - self.actor.shape_rot.y as f32) as i32 as i16;
        (d as i32).abs() < 0x3FFC
    }

    fn track(&mut self, play: &mut PlayState, preset: usize, forced: i16) {
        let mut t = self.unk_1e8;
        func_80034a14(play, &mut self.actor, &mut t, preset, forced);
        self.unk_1e8 = t;
    }

    fn sway(&mut self, play: &PlayState) {
        func_80034f54(play, &mut self.unk_2e4, &mut self.unk_304, 16);
    }

    /// `func_80A97D68`.
    fn func_80a97d68(&mut self, play: &mut PlayState) -> bool {
        let forced = if self.unk_1e8.talk_state != 0 {
            if !self.anim_is("gObjOsAnim_6A60") {
                self.change_anim(ENKO_ANIM_32);
            }
            2
        } else {
            if !self.anim_is("gObjOsAnim_7830") {
                self.change_anim(ENKO_ANIM_33);
            }
            1
        };
        self.track(play, 2, forced);
        self.is_within_talk_angle()
    }

    /// `func_80A97E18`.
    fn func_80a97e18(&mut self, play: &mut PlayState) -> bool {
        self.sway(play);
        let mut forced = if self.is_within_talk_angle() { 2 } else { 1 };
        if self.unk_1e8.talk_state != 0 {
            forced = 4;
        } else if self.look_dist < self.actor.xz_dist_to_player {
            forced = 1;
        }
        self.track(play, 2, forced);
        true
    }

    /// `func_80A97EB0`.
    fn func_80a97eb0(&mut self, play: &mut PlayState) -> bool {
        self.sway(play);
        let r = self.is_within_talk_angle();
        self.track(play, 2, if r { 2 } else { 1 });
        r
    }

    /// `func_80A97F20` (and its copy `func_80A98124`).
    fn func_80a97f20(&mut self, play: &mut PlayState) -> bool {
        self.sway(play);
        self.track(play, 2, 4);
        true
    }

    /// `func_80A97F70`.
    fn func_80a97f70(&mut self, play: &mut PlayState) -> bool {
        let forced = if self.unk_1e8.talk_state != 0 {
            if !self.anim_is("gObjOsAnim_8F6C") {
                self.change_anim(ENKO_ANIM_29);
            }
            self.sway(play);
            2
        } else {
            if !self.anim_is("gObjOsAnim_7D94") {
                self.change_anim(ENKO_ANIM_30);
            }
            1
        };
        self.track(play, 5, forced);
        self.is_within_talk_angle()
    }

    /// `func_80A98034`.
    fn func_80a98034(&mut self, play: &mut PlayState) -> bool {
        let (forced, r) = if self.unk_1e8.talk_state != 0 {
            if !self.anim_is("gObjOsAnim_8F6C") {
                self.change_anim(ENKO_ANIM_29);
            }
            self.sway(play);
            let r = self.is_within_talk_angle();
            (if r { 2 } else { 1 }, r)
        } else {
            if !self.anim_is("gObjOsAnim_879C") {
                self.change_anim(ENKO_ANIM_31);
            }
            (1, self.is_within_talk_angle())
        };
        self.track(play, 5, forced);
        r
    }

    /// `func_80A98174`.
    fn func_80a98174(&mut self, play: &mut PlayState) -> bool {
        if let Some(s) = &mut self.skel {
            if self.unk_1e8.talk_state != 0 {
                if s.on_frame(18.0) {
                    s.play_speed = 0.0;
                }
            } else if s.play_speed != 1.0 {
                s.play_speed = 1.0;
            }
        }
        let stopped = self.skel.as_ref().is_some_and(|s| s.play_speed == 0.0);
        if stopped {
            self.sway(play);
        }
        self.track(play, 2, if stopped { 2 } else { 1 });
        self.is_within_talk_angle()
    }

    /// `func_80A98ECC`: this frame's tracking by type and the story's progress
    /// (`EnKo_ChildStart` ... `EnKo_AdultSaved`).
    fn func_80a98ecc(&mut self, play: &mut PlayState) -> bool {
        if play.scene_id == SCENE_SPOT10 && self.ty() == ENKO_TYPE_CHILD_FADO {
            return self.func_80a97e18(play);
        }
        type F = fn(&mut EnKo, &mut PlayState) -> bool;
        let (d68, e18, eb0, f20, f70, k034, k174): (F, F, F, F, F, F, F) =
            (EnKo::func_80a97d68, EnKo::func_80a97e18, EnKo::func_80a97eb0, EnKo::func_80a97f20, EnKo::func_80a97f70, EnKo::func_80a98034, EnKo::func_80a98174);
        // By type (0..12), per quest state.
        let table: [[F; 13]; 5] = [
            // EnKo_ChildStart
            [d68, e18, k034, e18, f70, eb0, f20, eb0, eb0, eb0, e18, eb0, e18],
            // EnKo_ChildStone
            [f20, f20, k034, eb0, f70, eb0, f20, eb0, eb0, eb0, e18, eb0, e18],
            // EnKo_ChildSaria
            [f20, f20, k034, eb0, k174, eb0, f20, eb0, eb0, eb0, e18, eb0, e18],
            // EnKo_AdultEnemy
            [eb0, f20, eb0, eb0, eb0, eb0, f20, eb0, eb0, eb0, e18, eb0, e18],
            // EnKo_AdultSaved
            [k034, e18, e18, eb0, e18, eb0, f20, eb0, eb0, eb0, e18, eb0, e18],
        ];
        let f = table[Self::forest_quest_state(play)][self.ty() as usize];
        f(self, play)
    }

    /// `func_80A96FD0`: what a child says, as a child.
    fn child_text(&self, play: &PlayState) -> u16 {
        let s = &play.save;
        let letter = s.get_event_chk_inf(EVENTCHKINF_40);
        let stone = s.check_quest_item(QUEST_KOKIRI_EMERALD);
        match self.ty() {
            ENKO_TYPE_CHILD_FADO => {
                if letter {
                    0x10DA
                } else if stone {
                    0x10D9
                } else if s.get_inf_table(INFTABLE_B7) {
                    0x10D8
                } else {
                    0x10D7
                }
            }
            ENKO_TYPE_CHILD_0 => if letter { 0x1025 } else if stone { 0x1042 } else { 0x1004 },
            ENKO_TYPE_CHILD_1 => if letter { 0x1023 } else if stone { 0x1043 } else if s.get_inf_table(INFTABLE_1E) { 0x1006 } else { 0x1005 },
            ENKO_TYPE_CHILD_2 => if letter { 0x1022 } else { 0x1007 },
            ENKO_TYPE_CHILD_3 => if letter { 0x1021 } else if stone { 0x1044 } else if s.get_inf_table(INFTABLE_22) { 0x1009 } else { 0x1008 },
            ENKO_TYPE_CHILD_4 => if letter { 0x1097 } else if stone { 0x1042 } else if s.get_inf_table(INFTABLE_24) { 0x100B } else { 0x100A },
            ENKO_TYPE_CHILD_5 => if letter { 0x10B0 } else if stone { 0x1043 } else if s.get_inf_table(INFTABLE_26) { 0x100D } else { 0x100C },
            ENKO_TYPE_CHILD_6 => if letter { 0x10B5 } else if stone { 0x1043 } else if s.get_inf_table(INFTABLE_28) { 0x1019 } else { 0x100E },
            ENKO_TYPE_CHILD_7 => 0x1035,
            ENKO_TYPE_CHILD_8 => 0x1038,
            ENKO_TYPE_CHILD_9 => if stone { 0x104B } else { 0x103C },
            ENKO_TYPE_CHILD_10 => if stone { 0x104C } else { 0x103D },
            ENKO_TYPE_CHILD_11 => 0x103E,
            _ => 0,
        }
    }

    /// `func_80A97338`: what a child says to adult Link (Fado's exchange item aside).
    fn adult_text(&self, play: &PlayState) -> u16 {
        let s = &play.save;
        let medallion = s.check_quest_item(QUEST_MEDALLION_FOREST);
        match self.ty() {
            ENKO_TYPE_CHILD_FADO => 0x10B9,
            ENKO_TYPE_CHILD_0 => if medallion { 0x1072 } else if s.get_inf_table(INFTABLE_41) { 0x1056 } else { 0x1055 },
            ENKO_TYPE_CHILD_1 => if medallion { 0x1073 } else { 0x105A },
            ENKO_TYPE_CHILD_2 => if medallion { 0x1074 } else if s.get_inf_table(INFTABLE_47) { 0x105E } else { 0x105D },
            ENKO_TYPE_CHILD_3 => if medallion { 0x1075 } else { 0x105B },
            ENKO_TYPE_CHILD_4 => if medallion { 0x1076 } else { 0x105F },
            ENKO_TYPE_CHILD_5 => 0x1057,
            ENKO_TYPE_CHILD_6 => if medallion { 0x1077 } else if s.get_inf_table(INFTABLE_51) { 0x1059 } else { 0x1058 },
            ENKO_TYPE_CHILD_7 => if medallion { 0x1079 } else { 0x104E },
            ENKO_TYPE_CHILD_8 => if medallion { 0x107A } else if s.get_inf_table(INFTABLE_59) { 0x1050 } else { 0x104F },
            ENKO_TYPE_CHILD_9 => if medallion { 0x107B } else { 0x1051 },
            ENKO_TYPE_CHILD_10 => if medallion { 0x107C } else { 0x1052 },
            ENKO_TYPE_CHILD_11 => if medallion { 0x107C } else if s.get_inf_table(INFTABLE_61) { 0x1054 } else { 0x1053 },
            _ => 0,
        }
    }

    /// `func_80A97610`: the text (`Text_GetFaceReaction` is 0 without a mask).
    fn text(&self, play: &PlayState) -> u16 {
        if play.save.adult { self.adult_text(play) } else { self.child_text(play) }
    }

    /// `func_80A9877C`: track Link, and talk.
    fn func_80a9877c(&mut self, play: &mut PlayState) {
        // No cutscenes and no debug camera.
        self.unk_1e8.target = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(self.actor.world_pos);
        let fqs = Self::forest_quest_state(play);
        self.unk_1e8.eye_height = if play.save.adult && self.ty() == ENKO_TYPE_CHILD_FADO { -20.0 } else { EYE_HEIGHTS[self.ty() as usize][fqs] };
        if !self.func_80a98ecc(play) && self.unk_1e8.talk_state == 0 {
            return;
        }
        let text = self.text(play);
        let mut talk_state = self.unk_1e8.talk_state;
        // func_800343CC with func_80A97610 and func_80A97738 (the message states aren't ported:
        // a talk never starts, so the second callback isn't reached).
        oot_game::npc::talk_update(play, &mut self.actor, &mut talk_state, self.look_dist, |_, _| text, |_, _| 1);
        self.unk_1e8.talk_state = talk_state;
        // Fado's trade in the Lost Woods (SCENE_SPOT10): not ported.
    }

    /// `func_80A98DB4`: fading in and out by Link's distance in Kokiri Forest and the Lost Woods.
    fn func_80a98db4(&mut self, play: &PlayState) {
        if play.scene_id != SCENE_SPOT10 && play.scene_id != SCENE_SPOT04 {
            self.model_alpha = 255.0;
            return;
        }
        let dist = self.actor.xz_dist_to_player;
        let target = if self.appear_dist < dist { 0.0 } else { 255.0 };
        eng_math::smooth_step_to_f(&mut self.model_alpha, target, 0.3, 40.0, 1.0);
        if self.model_alpha < 10.0 {
            self.actor.flags &= !ACTOR_FLAG_0;
        } else {
            self.actor.flags |= ACTOR_FLAG_0;
        }
    }

    /// `EnKo_Blink`.
    fn blink(&mut self, play: &mut PlayState) {
        // DECR(blinkTimer) == 0.
        if self.blink_timer != 0 {
            self.blink_timer -= 1;
            if self.blink_timer != 0 {
                return;
            }
        }
        self.eye_texture_index = self.eye_texture_index.wrapping_add(1);
        if HEAD_EYES[self.model().head].is_some() && self.eye_texture_index >= 3 {
            self.blink_timer = play.rand.s16_offset(30, 30);
            self.eye_texture_index = 0;
        }
    }

    /// The limb matrices `EnKo_Draw` draws with: the pose, and `EnKo_OverrideLimbDraw`'s head
    /// and torso turns (limbs 15 and 8) and its sway of limbs 8, 9 and 12.
    fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], head: [i16; 2], torso: [i16; 2], sway: &[(usize, i16, i16)]) -> Vec<Mat4> {
        let r = binang_to_rad;
        skeleton.pose_override(joints, |limb, _pos, rot| {
            let mut pre = Mat4::IDENTITY;
            if limb == 8 {
                pre = Mat4::from_rotation_x(r(torso[1].wrapping_neg())) * Mat4::from_rotation_z(r(torso[0]));
            }
            if limb == 15 {
                pre = Mat4::from_translation(Vec3::new(1200.0, 0.0, 0.0))
                    * Mat4::from_rotation_x(r(head[1]))
                    * Mat4::from_rotation_z(r(head[0]))
                    * Mat4::from_translation(Vec3::new(-1200.0, 0.0, 0.0));
            }
            if let Some(&(_, a, b)) = sway.iter().find(|s| s.0 == limb) {
                rot[1] = (rot[1] as f32 + sin_s(a) * 200.0) as i32 as i16;
                rot[2] = (rot[2] as f32 + cos_s(b) * 200.0) as i32 as i16;
            }
            pre
        })
    }

    fn sway_of(a: &[i16; 16], b: &[i16; 16]) -> [(usize, i16, i16); 3] {
        [(8, a[8], b[8]), (9, a[9], b[9]), (12, a[12], b[12])]
    }
}

/// Indices into the render state's extras.
mod rs {
    /// `angles`: head x, y; torso x, y; then the sway (limbs 8, 9, 12: `unk_2E4`, `unk_304`).
    pub const HEAD: usize = 0;
    pub const TORSO: usize = 2;
    pub const SWAY: usize = 4;
    /// `values`: `modelAlpha`.
    pub const ALPHA: usize = 0;
    /// `switches`: the eye texture.
    pub const EYE: usize = 0;
}

impl ActorImpl for EnKo {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnKo_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if self.action != Action::WaitObjects {
            if self.model_alpha as i32 != 0 {
                if let Some(s) = &mut self.skel {
                    s.update();
                }
                self.func_80a98db4(play);
                self.blink(play);
            } else {
                self.func_80a98db4(play);
            }
        }
        if self.unk_1e8.talk_state == 0 {
            self.actor.move_forward();
        }
        let fqs = Self::forest_quest_state(play);
        if FALLS[(self.ty() as usize).min(12)][fqs] != 0 {
            self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
            self.actor.gravity = -1.0;
        } else {
            self.actor.gravity = 0.0;
        }
        match self.action {
            Action::WaitObjects => self.wait_objects(play),
            // func_80A99384: Fado's trade only.
            Action::Idle => {}
            Action::Guard => self.guard(play),
        }
        self.func_80a9877c(play);
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
    }
    /// `EnKo_PostLimbDraw`, limb 15: the focus at the head.
    fn draw_update(&mut self, _play: &mut PlayState) {
        let (Some(skel), Some(skeleton)) = (&self.skel, &self.skeleton) else { return };
        let t = &self.unk_1e8;
        let bones = Self::pose(skeleton, &skel.joint_table, [t.head[0], t.head[1]], [t.torso[0], t.torso[1]], &Self::sway_of(&self.unk_2e4, &self.unk_304));
        let rs = RenderState::of(&self.actor);
        let m = oot_game::play::actor_draw_matrix(&rs);
        if let Some(head) = bones.get(14) {
            self.actor.focus_pos = (m * *head).transform_point3(Vec3::ZERO);
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        if let Some(s) = &self.skel {
            rs.joints = Some(eng_anim::anim::JointTable { rot: s.joint_table.clone(), face: 0 });
        }
        let t = &self.unk_1e8;
        let sway = Self::sway_of(&self.unk_2e4, &self.unk_304);
        rs.angles = vec![t.head[0], t.head[1], t.torso[0], t.torso[1], sway[0].1, sway[0].2, sway[1].1, sway[1].2, sway[2].1, sway[2].2];
        rs.values = vec![self.model_alpha];
        rs.switches = vec![self.eye_texture_index.clamp(0, 2) as u32];
        rs
    }
    /// `EnKo_Draw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(skeleton), Some(joints)) = (&self.skeleton, &rs.joints) else { return };
        if rs.angles.len() < 10 || rs.values.is_empty() || rs.switches.is_empty() {
            return;
        }
        let alpha = rs.values[rs::ALPHA] as i32 as i16;
        if alpha == 0 {
            return;
        }
        let a = &rs.angles;
        let sway = [(8, a[rs::SWAY], a[rs::SWAY + 1]), (9, a[rs::SWAY + 2], a[rs::SWAY + 3]), (12, a[rs::SWAY + 4], a[rs::SWAY + 5])];
        let bones = Self::pose(skeleton, &joints.rot, [a[rs::HEAD], a[rs::HEAD + 1]], [a[rs::TORSO], a[rs::TORSO + 1]], &sway);
        let m = self.model();
        let xlu = alpha != 255;
        let a8 = if xlu { alpha as u8 } else { 255 };
        let mut sv = SegmentValues::default();
        sv.env[SEG_TUNIC as usize] = Some([m.tunic[0], m.tunic[1], m.tunic[2], a8]);
        sv.env[SEG_BOOTS as usize] = Some([m.boots[0], m.boots[1], m.boots[2], a8]);
        sv.env[SEG_ALPHA as usize] = Some([0, 0, 0, alpha.clamp(0, 255) as u8]);
        let key = MeshKey::named(keys::bake(&bake_name(m.head, rs.switches[rs::EYE] as usize, xlu)));
        let cmd = DrawCmd { mesh: key, transform: oot_game::play::actor_draw_matrix(rs), bones, params: eng_gfx::DrawParams { segments: Some(sv) } };
        if xlu {
            out.xlu.push(cmd);
        } else {
            out.opa.push(cmd);
        }
        // ActorShadow_DrawCircle (shadowScale 18, shadowAlpha = modelAlpha): the circle
        // shadow's stand-in, at full strength.
        let (floor, _) = play.col.entity_raycast_down(rs.pos + Vec3::Y * 20.0);
        let shadow = Mat4::from_translation(Vec3::new(rs.pos.x, floor + 0.3, rs.pos.z)) * Mat4::from_scale(Vec3::new(18.0, 1.0, 18.0));
        out.xlu.push(DrawCmd::new(MeshKey::named(oot_game::play::builtin::SHADOW), shadow));
    }
    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0).then_some(ColliderMut::Cylinder(&mut self.collider))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
