//! `En_Viewer` (`ovl_En_Viewer/z_en_viewer.c`): the opening's and other cutscenes' riders. Its
//! type (`params >> 8`) is one of ten: Zelda's horse, Impa, Zelda, Ganondorf on his horse
//! (several poses), Ganondorf's horse, and the adult Ganondorf of the ending. The nightmare
//! (Hyrule Field's scene layer 4) places types 0 to 4.
//!
//! Each waits until its skeleton's and its animation's objects are loaded
//! (`EnViewer_InitImpl`), then plays its animation and follows the script's actor cue: Zelda,
//! Impa and their horse cue 0 (`npcActions[0]`), Ganondorf and his horse cue 1. They go from
//! the cue's start to its end position over its frames (`EnViewer_UpdatePosition`), change
//! their animations on the cue's action (Ganondorf's rear, look, charge; Zelda's and Impa's
//! looks back), make their sounds (the horses' gallop and neigh, the cuts' `NA_SE_SY_DEMO_CUT`),
//! and are drawn only while their cue is there.
//!
//! The horses are skin skeletons (`oot_game::skin`, `func_800A6330`); the others flex
//! skeletons, baked with what their draws bind: Zelda's eyes and mouth by the frame, Impa's
//! masked head, Ganondorf's eyes and his open hand (from the nightmare's frame 400).
//!
//! Not ported: the horses' shadow (`ActorShadow_DrawHorse`), the adult Ganondorf's draw (type 9:
//! the crazed eyes on XLU and the jewel's `gGanondorfEyesDL`), and the cape's settings
//! `EnViewer_UpdateGanondorfCape` writes for type 5 into `En_Ganon_Mant` (its neck position is
//! kept), and the fire of type 5 (`EnViewer_DrawFireEffects`, run from its update: its state and
//! its `Rand_ZeroOne` calls are, its flames aren't drawn).

use std::sync::Arc;

use eng_anim::anim::JointTable;
use eng_anim::skeleton::Skeleton;
use eng_gfx::{DrawCmd, DrawParams, MeshKey};
use eng_math::{smooth_step_to_f, smooth_step_to_s, vec3f_yaw};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_4, Actor};
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorImpl, ActorProfile, audio_play_actor_sfx2, cur_sfx_pos};
use oot_game::audio::SEQ_PLAYER_FANFARE;
use oot_game::audio::sfx::*;
use oot_game::cutscene::{CS_STATE_IDLE, CsCmdActorAction};
use oot_game::env::lerp_weight;
use oot_game::pack::{BakeBody, BakeSegment, ForeignAnim, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_ONCE, Anim, SkelAnimeStd};
use oot_game::skin::{SkinPlace, SkinSkeleton, skin_draw};

/// `ACTOR_EN_VIEWER` (`actor_table.h`: 0x002A).
pub const ACTOR_EN_VIEWER: i16 = 0x002A;
/// `ACTOR_EN_GANON_MANT`, `ACTOR_ITEM_OCARINA`, `ACTOR_DEMO_6K` (`actor_table.h`).
const ACTOR_EN_GANON_MANT: i16 = 0x016F;
const ACTOR_ITEM_OCARINA: i16 = 0x00F1;
const ACTOR_DEMO_6K: i16 = 0x00F5;

/// `En_Viewer_InitVars`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_VIEWER, name: "En_Viewer", category: ACTORCAT_ITEMACTION, flags: ACTOR_FLAG_4, object: "gameplay_keep" };

/// `EnViewerType`.
pub const ENVIEWER_TYPE_0_HORSE_ZELDA: u8 = 0;
pub const ENVIEWER_TYPE_1_IMPA: u8 = 1;
pub const ENVIEWER_TYPE_2_ZELDA: u8 = 2;
pub const ENVIEWER_TYPE_3_GANONDORF: u8 = 3;
pub const ENVIEWER_TYPE_4_HORSE_GANONDORF: u8 = 4;
pub const ENVIEWER_TYPE_5_GANONDORF: u8 = 5;
pub const ENVIEWER_TYPE_6_HORSE_GANONDORF: u8 = 6;
pub const ENVIEWER_TYPE_7_GANONDORF: u8 = 7;
pub const ENVIEWER_TYPE_8_GANONDORF: u8 = 8;
pub const ENVIEWER_TYPE_9_GANONDORF: u8 = 9;

/// `EnViewerDrawType`.
const ENVIEWER_DRAW_GANONDORF: u8 = 0;
const ENVIEWER_DRAW_HORSE: u8 = 1;
const ENVIEWER_DRAW_ZELDA: u8 = 2;
const ENVIEWER_DRAW_IMPA: u8 = 3;

/// `SCENE_SPOT00` (Hyrule Field), `SCENE_TOKINOMA` (the Temple of Time's inside).
const SCENE_SPOT00: u16 = 0x51;

/// `NA_BGM_OPENING_GANON` (`sequence.h`).
const NA_BGM_OPENING_GANON: u16 = 0x23;

/// `YOUNG_GANONDORF_LIMB_LEFT_HAND`, `_HEAD` (`z_en_viewer.h`, 1-based).
const YOUNG_GANONDORF_LIMB_LEFT_HAND: u8 = 5;
const YOUNG_GANONDORF_LIMB_HEAD: usize = 15;

// `OBJECT_*` (`object_table.h`).
const OBJECT_HORSE_GANON: i16 = 0x002D;
const OBJECT_HORSE_ZELDA: i16 = 0x0046;
const OBJECT_OPENING_DEMO1: i16 = 0x0047;
const OBJECT_IM: i16 = 0x0087;
const OBJECT_GNDD: i16 = 0x009B;
const OBJECT_GANON: i16 = 0x00E1;
const OBJECT_ZL4: i16 = 0x0191;

/// An object's id and file.
type Object = (i16, &'static str);
const HORSE_GANON: Object = (OBJECT_HORSE_GANON, "object_horse_ganon");
const HORSE_ZELDA: Object = (OBJECT_HORSE_ZELDA, "object_horse_zelda");
const OPENING_DEMO1: Object = (OBJECT_OPENING_DEMO1, "object_opening_demo1");
const IM: Object = (OBJECT_IM, "object_im");
const GNDD: Object = (OBJECT_GNDD, "object_gndd");
const GANON: Object = (OBJECT_GANON, "object_ganon");
const ZL4: Object = (OBJECT_ZL4, "object_zl4");

/// `EnViewerInitData`: skeleton object, animation object, scale (/100), yOffset (*100), shadow
/// type, shadow scale, draw type, skeleton, animation.
struct InitData {
    skeleton_object: Object,
    anim_object: Object,
    scale: u8,
    y_offset: i8,
    shadow_type: u8,
    shadow_scale: u8,
    draw_type: u8,
    skeleton: &'static str,
    anim: &'static str,
}

/// `ENVIEWER_SHADOW_*`.
const ENVIEWER_SHADOW_NONE: u8 = 0;
const ENVIEWER_SHADOW_HORSE: u8 = 2;

/// `sInitData`.
const INIT_DATA: [InitData; 10] = [
    InitData {
        skeleton_object: HORSE_ZELDA,
        anim_object: HORSE_ZELDA,
        scale: 1,
        y_offset: 0,
        shadow_type: ENVIEWER_SHADOW_HORSE,
        shadow_scale: 20,
        draw_type: ENVIEWER_DRAW_HORSE,
        skeleton: "gHorseZeldaSkel",
        anim: "gHorseZeldaGallopingAnim",
    },
    InitData {
        skeleton_object: IM,
        anim_object: OPENING_DEMO1,
        scale: 1,
        y_offset: 0,
        shadow_type: ENVIEWER_SHADOW_NONE,
        shadow_scale: 10,
        draw_type: ENVIEWER_DRAW_IMPA,
        skeleton: "gImpaSkel",
        anim: ANIM_IMPA_RIDE,
    },
    InitData {
        skeleton_object: ZL4,
        anim_object: OPENING_DEMO1,
        scale: 1,
        y_offset: 0,
        shadow_type: ENVIEWER_SHADOW_NONE,
        shadow_scale: 10,
        draw_type: ENVIEWER_DRAW_ZELDA,
        skeleton: "gChildZeldaSkel",
        anim: ANIM_ZELDA_RIDE,
    },
    InitData {
        skeleton_object: GNDD,
        anim_object: GNDD,
        scale: 1,
        y_offset: -6,
        shadow_type: ENVIEWER_SHADOW_NONE,
        shadow_scale: 10,
        draw_type: ENVIEWER_DRAW_GANONDORF,
        skeleton: "gYoungGanondorfSkel",
        anim: "gYoungGanondorfHorsebackIdleAnim",
    },
    InitData {
        skeleton_object: HORSE_GANON,
        anim_object: HORSE_GANON,
        scale: 1,
        y_offset: 0,
        shadow_type: ENVIEWER_SHADOW_HORSE,
        shadow_scale: 20,
        draw_type: ENVIEWER_DRAW_HORSE,
        skeleton: "gHorseGanonSkel",
        anim: "gHorseGanonRearingAnim",
    },
    InitData {
        skeleton_object: GNDD,
        anim_object: GNDD,
        scale: 1,
        y_offset: -6,
        shadow_type: ENVIEWER_SHADOW_NONE,
        shadow_scale: 10,
        draw_type: ENVIEWER_DRAW_GANONDORF,
        skeleton: "gYoungGanondorfSkel",
        anim: "gYoungGanondorfHorsebackRideAnim",
    },
    InitData {
        skeleton_object: HORSE_GANON,
        anim_object: HORSE_GANON,
        scale: 1,
        y_offset: 0,
        shadow_type: ENVIEWER_SHADOW_HORSE,
        shadow_scale: 20,
        draw_type: ENVIEWER_DRAW_HORSE,
        skeleton: "gHorseGanonSkel",
        anim: "gHorseGanonGallopingAnim",
    },
    InitData {
        skeleton_object: GNDD,
        anim_object: GNDD,
        scale: 1,
        y_offset: -6,
        shadow_type: ENVIEWER_SHADOW_NONE,
        shadow_scale: 10,
        draw_type: ENVIEWER_DRAW_GANONDORF,
        skeleton: "gYoungGanondorfSkel",
        anim: "gYoungGanondorfArmsCrossedAnim",
    },
    InitData {
        skeleton_object: GNDD,
        anim_object: GNDD,
        scale: 1,
        y_offset: -6,
        shadow_type: ENVIEWER_SHADOW_NONE,
        shadow_scale: 10,
        draw_type: ENVIEWER_DRAW_GANONDORF,
        skeleton: "gYoungGanondorfSkel",
        anim: "gYoungGanondorfWalkAnim",
    },
    InitData {
        skeleton_object: GANON,
        anim_object: GANON,
        scale: 1,
        y_offset: -6,
        shadow_type: ENVIEWER_SHADOW_NONE,
        shadow_scale: 10,
        draw_type: ENVIEWER_DRAW_GANONDORF,
        skeleton: "gGanondorfSkel",
        anim: "gGanondorfEndingFloatAnim",
    },
];

// `object_opening_demo1`'s animations by their roles.
const ANIM_IMPA_RIDE: &str = "object_opening_demo1_Anim_0029CC";
const ANIM_IMPA_LOOK_BACK: &str = "object_opening_demo1_Anim_002574";
const ANIM_ZELDA_RIDE: &str = "object_opening_demo1_Anim_000450";
const ANIM_ZELDA_LOOK_BACK: &str = "object_opening_demo1_Anim_001410";
const ANIM_ZELDA_00504C: &str = "object_opening_demo1_Anim_00504C";
const ANIM_ZELDA_00420C: &str = "object_opening_demo1_Anim_00420C";
const ANIM_ZELDA_0048FC: &str = "object_opening_demo1_Anim_0048FC";

/// Impa's animations live in an object without a skeleton: decoded for `gImpaSkel`.
pub fn foreign_anims() -> Vec<ForeignAnim> {
    [ANIM_IMPA_RIDE, ANIM_IMPA_LOOK_BACK].iter().map(|a| ForeignAnim { anim_file: OPENING_DEMO1.1.into(), anim: (*a).into(), skel_file: IM.1.into(), skel: "gImpaSkel".into() }).collect()
}

// The draws' segments.
const SEG_EYE_L: u8 = 0x08;
const SEG_EYE_R: u8 = 0x09;
const SEG_MOUTH: u8 = 0x0A;
const SEG_ENV: u8 = 0x0E;

/// `EnViewer_DrawZelda`'s eyes (segments 8 and 9) by the frame in Hyrule Field.
const ZELDA_EYES: [(&str, &str); 5] = [
    ("gChildZeldaEyeInTex", "gChildZeldaEyeOutTex"),
    ("gChildZeldaEyeBlinkTex", "gChildZeldaEyeBlinkTex"),
    ("gChildZeldaEyeShutTex", "gChildZeldaEyeShutTex"),
    ("gChildZeldaEyeWideTex", "gChildZeldaEyeWideTex"),
    ("gChildZeldaEyeShutTex", "gChildZeldaEyeShutTex"),
];
/// Segment 0x0A.
const ZELDA_MOUTHS: [&str; 2] = ["gChildZeldaMouthWorriedTex", "gChildZeldaMouthSurprisedTex"];
/// Young Ganondorf's eyes by the frame.
const GANONDORF_EYES: [&str; 4] = ["gYoungGanondorfEyeOpenTex", "gYoungGanondorfEyeHalfTex", "gYoungGanondorfEyeClosedTex", "gYoungGanondorfEyeLookingDownTex"];

fn zelda_bake(field: bool, eyes: usize, mouth: usize) -> String {
    format!("En_Viewer/zelda_{}_{eyes}_{mouth}", if field { "field" } else { "other" })
}
fn ganondorf_bake(eyes: usize, open_hand: bool) -> String {
    format!("En_Viewer/ganondorf_{eyes}_{}", if open_hand { "open" } else { "closed" })
}
fn horse_bake(skeleton: &str) -> String {
    format!("En_Viewer/{skeleton}")
}
const IMPA_BAKE: &str = "En_Viewer/impa";

fn tex(file: &str, symbol: &str) -> BakeSegment {
    BakeSegment::Texture { file: file.into(), symbol: symbol.into() }
}

/// The draws' meshes: Zelda's eyes and mouths (and her Hyrule Field limbs:
/// `EnViewer_ZeldaOverrideLimbDraw`), Impa (`gImpaHeadMaskedDL` on limb 16, the env colour, segment
/// 0x0C's empty list), young Ganondorf's eyes with his hand closed or open, and the horses.
pub fn bakes() -> Vec<MeshBake> {
    let mut v = Vec::new();
    // Zelda's limbs 2 (the cutscene dress), 3, 5, 7, 8, 9 (none) in Hyrule Field (0-based here).
    let zelda_field_limbs = || {
        let none = |l: u8| LimbOverride { limb: l - 1, file: ZL4.1.into(), symbol: String::new() };
        let mut l = vec![LimbOverride { limb: 1, file: ZL4.1.into(), symbol: "gChildZeldaCutsceneDressDL".into() }];
        l.extend([3, 5, 7, 8, 9].map(none));
        l
    };
    for (eyes, (l, r)) in ZELDA_EYES.iter().enumerate() {
        for (mouth, m) in ZELDA_MOUTHS.iter().enumerate() {
            for field in [true, false] {
                // Outside Hyrule Field only the shut eyes (4) and the worried mouth.
                if !field && (eyes != 4 || mouth != 0) {
                    continue;
                }
                v.push(MeshBake {
                    name: zelda_bake(field, eyes, mouth),
                    object: ZL4.1.into(),
                    segments: vec![(SEG_EYE_L, tex(ZL4.1, l)), (SEG_EYE_R, tex(ZL4.1, r)), (SEG_MOUTH, tex(ZL4.1, m))],
                    prelude: Vec::new(),
                    body: BakeBody::Skeleton { file: ZL4.1.into(), symbol: "gChildZeldaSkel".into(), limbs: if field { zelda_field_limbs() } else { Vec::new() } },
                });
            }
        }
    }
    v.push(MeshBake {
        name: IMPA_BAKE.into(),
        object: IM.1.into(),
        segments: vec![
            (SEG_EYE_L, tex(IM.1, "gImpaEyeOpenTex")),
            (SEG_EYE_R, tex(IM.1, "gImpaEyeOpenTex")),
            (SEG_ENV, BakeSegment::Commands(vec![(0xFB00_0000, 0x0000_00FF)])),
            // &D_80116280[2]: its gsSPEndDisplayList.
            (0x0C, BakeSegment::Commands(Vec::new())),
        ],
        prelude: vec![SEG_ENV],
        body: BakeBody::Skeleton { file: IM.1.into(), symbol: "gImpaSkel".into(), limbs: vec![LimbOverride { limb: 15, file: IM.1.into(), symbol: "gImpaHeadMaskedDL".into() }] },
    });
    for (eyes, e) in GANONDORF_EYES.iter().enumerate() {
        for open in [false, true] {
            let limbs = if open { vec![LimbOverride { limb: YOUNG_GANONDORF_LIMB_LEFT_HAND - 1, file: GNDD.1.into(), symbol: "gYoungGanondorfOpenLeftHandDL".into() }] } else { Vec::new() };
            v.push(MeshBake {
                name: ganondorf_bake(eyes, open),
                object: GNDD.1.into(),
                segments: vec![(SEG_EYE_L, tex(GNDD.1, e)), (SEG_EYE_R, tex(GNDD.1, e))],
                prelude: Vec::new(),
                body: BakeBody::Skeleton { file: GNDD.1.into(), symbol: "gYoungGanondorfSkel".into(), limbs },
            });
        }
    }
    for (object, skel) in [(HORSE_ZELDA.1, "gHorseZeldaSkel"), (HORSE_GANON.1, "gHorseGanonSkel")] {
        v.push(MeshBake { name: horse_bake(skel), object: object.into(), segments: Vec::new(), prelude: Vec::new(), body: BakeBody::Skin { file: object.into(), symbol: skel.into() } });
    }
    v
}

/// The overlay's statics: `sHorseSfxPlayed`, `sTimer`, `sGanondorfNeckWorldPos`.
#[derive(Debug, Default)]
struct Statics {
    horse_sfx_played: bool,
    timer: i16,
    ganondorf_neck_world_pos: Vec3,
}

/// `EnViewerFireEffect`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FireEffect {
    pub start_pos: Vec3,
    pub end_pos: Vec3,
    pub pos: Vec3,
    pub lerp_factor_speed: f32,
    pub scale: f32,
    pub lerp_factor: f32,
    pub state: u8,
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `EnViewer_InitImpl`: waiting for the objects.
    InitImpl,
    /// `EnViewer_UpdateImpl`.
    UpdateImpl,
}

/// The model: a flex skeleton or a skin one.
#[derive(Clone)]
enum Model {
    None,
    Flex(Arc<Skeleton>),
    Skin(Arc<SkinSkeleton>),
}

pub struct EnViewer {
    pub actor: Actor,
    pub action: Action,
    pub ty: u8,
    /// `skin.skelAnime`.
    pub skel: Option<SkelAnimeStd>,
    model: Model,
    /// `drawFuncIndex`.
    pub draw_func_index: u8,
    pub state: u8,
    /// `isVisible`.
    pub is_visible: bool,
    /// `fireEffects`.
    pub fire_effects: [FireEffect; 20],
}

fn anim(play: &PlayState, file: &str, symbol: &str) -> Option<Anim> {
    match play.assets.as_ref()?.animation(file, symbol) {
        Ok(a) => Some(a),
        Err(e) => {
            log::warn!("En_Viewer: {file}/{symbol}: {e:#}");
            None
        }
    }
}

impl EnViewer {
    /// `EnViewer_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: the cull zone (not ported).
        play.overlay_static::<Statics>(ACTOR_EN_VIEWER).horse_sfx_played = false;
        let ty = (actor.params >> 8) as u8;
        if matches!(ty, ENVIEWER_TYPE_3_GANONDORF | ENVIEWER_TYPE_5_GANONDORF | ENVIEWER_TYPE_7_GANONDORF | ENVIEWER_TYPE_8_GANONDORF | ENVIEWER_TYPE_9_GANONDORF) {
            // sGanondorfCape = Actor_SpawnAsChild(.., ACTOR_EN_GANON_MANT, 0, 0, 0, 0, 0, 0, 35).
            if let Err(e) = play.actor_spawn_as_child(&mut actor, ACTOR_EN_GANON_MANT, Vec3::ZERO, [0; 3], 35) {
                log::debug!("En_Viewer: En_Ganon_Mant: {e:?}");
            }
        }
        Box::new(EnViewer { actor, action: Action::InitImpl, ty, skel: None, model: Model::None, draw_func_index: 0, state: 0, is_visible: false, fire_effects: [FireEffect::default(); 20] })
    }

    fn data(&self) -> &'static InitData {
        &INIT_DATA[self.ty.min(9) as usize]
    }

    fn anim_file(&self) -> &'static str {
        self.data().anim_object.1
    }

    fn anim(&self, play: &PlayState, symbol: &str) -> Option<Anim> {
        anim(play, self.anim_file(), symbol)
    }

    /// `Animation_PlayLoopSetSpeed`.
    fn play_loop_set_speed(&mut self, play: &PlayState, symbol: &str, speed: f32) {
        if let (Some(a), Some(s)) = (self.anim(play, symbol), self.skel.as_mut()) {
            let last = a.last_frame();
            s.change(a, speed, 0.0, last, ANIMMODE_LOOP, 0.0);
        }
    }

    /// `Animation_PlayOnceSetSpeed`.
    fn play_once_set_speed(&mut self, play: &PlayState, symbol: &str, speed: f32) {
        if let (Some(a), Some(s)) = (self.anim(play, symbol), self.skel.as_mut()) {
            let last = a.last_frame();
            s.change(a, speed, 0.0, last, ANIMMODE_ONCE, 0.0);
        }
    }

    /// `Animation_MorphToPlayOnce`.
    fn morph_to_play_once(&mut self, play: &PlayState, symbol: &str, morph: f32) {
        if let (Some(a), Some(s)) = (self.anim(play, symbol), self.skel.as_mut()) {
            let last = a.last_frame();
            s.change(a, 1.0, 0.0, last, ANIMMODE_ONCE, morph);
        }
    }

    /// `Animation_MorphToLoop`.
    fn morph_to_loop(&mut self, play: &PlayState, symbol: &str, morph: f32) {
        if let (Some(a), Some(s)) = (self.anim(play, symbol), self.skel.as_mut()) {
            s.change(a, 1.0, 0.0, 0.0, ANIMMODE_LOOP, morph);
        }
    }

    fn is(&self, symbol: &str) -> bool {
        self.skel.as_ref().is_some_and(|s| s.is(symbol))
    }

    /// `EnViewer_InitImpl`: once both objects are loaded, the scale, the shape, the draw type and
    /// the skeleton with its animation (`sInitAnimFuncs`).
    fn init_impl(&mut self, play: &mut PlayState) {
        let d = self.data();
        let skel_bank = play.object_ctx.get_index(d.skeleton_object.0);
        let anim_bank = play.object_ctx.get_index(d.anim_object.0);
        let (Some(skel_bank), Some(anim_bank)) = (skel_bank, anim_bank) else {
            log::error!("En_Viewer type {}: {} or {} not in the object list", self.ty, d.skeleton_object.1, d.anim_object.1);
            return;
        };
        if !play.object_ctx.is_loaded(skel_bank) || !play.object_ctx.is_loaded(anim_bank) {
            return;
        }
        let Some(assets) = play.assets.clone() else { return };
        self.is_visible = true;
        self.actor.scale = Vec3::splat(d.scale as f32 / 100.0);
        // ActorShape_Init(yOffset * 100, sShadowDrawFuncs[shadowType], shadowScale).
        self.actor.shape_y_offset = (d.y_offset as i32 * 100) as f32;
        // The horses' shadow (ActorShadow_DrawHorse) isn't ported.
        let _ = (d.shadow_type, d.shadow_scale);
        self.draw_func_index = d.draw_type;
        let first = anim(play, d.anim_object.1, d.anim);
        match d.draw_type {
            ENVIEWER_DRAW_HORSE => {
                // EnViewer_InitAnimHorse: Skin_Init (SkelAnime_InitSkin plays it looped).
                match assets.pack.skin_skeleton(d.skeleton_object.1, d.skeleton) {
                    Ok(s) => {
                        self.skel = Some(SkelAnimeStd::init_flex(s.limbs.len(), first.clone()));
                        self.model = Model::Skin(Arc::new(s));
                    }
                    Err(e) => log::error!("En_Viewer: {}: {e:#}", d.skeleton),
                }
                if !matches!(self.ty, ENVIEWER_TYPE_3_GANONDORF | ENVIEWER_TYPE_4_HORSE_GANONDORF | ENVIEWER_TYPE_7_GANONDORF | ENVIEWER_TYPE_8_GANONDORF | ENVIEWER_TYPE_9_GANONDORF) {
                    self.play_loop_set_speed(play, d.anim, 3.0);
                } else {
                    self.play_once_set_speed(play, d.anim, 1.0);
                }
            }
            _ => {
                // EnViewer_InitAnimGanondorfOrZelda / _InitAnimImpa: SkelAnime_InitFlex (types
                // 0, 1, 4 and 6 would take SkelAnime_Init: none of them draws this way).
                match assets.skeleton(d.skeleton_object.1, d.skeleton) {
                    Ok(s) => {
                        self.skel = Some(SkelAnimeStd::init_flex(s.limbs.len(), None));
                        self.model = Model::Flex(s);
                    }
                    Err(e) => log::error!("En_Viewer: {}: {e:#}", d.skeleton),
                }
                let speed = if matches!(self.ty, ENVIEWER_TYPE_3_GANONDORF | ENVIEWER_TYPE_7_GANONDORF | ENVIEWER_TYPE_8_GANONDORF | ENVIEWER_TYPE_9_GANONDORF) { 1.0 } else { 3.0 };
                self.play_loop_set_speed(play, d.anim, speed);
            }
        }
        self.action = Action::UpdateImpl;
    }

    /// The cue this type follows: 0 for Zelda, Impa and their horse, 1 for the Ganondorfs.
    fn cue_slot(&self) -> usize {
        if self.ty <= ENVIEWER_TYPE_2_ZELDA { 0 } else { 1 }
    }

    fn cue(play: &PlayState, slot: usize) -> Option<CsCmdActorAction> {
        if play.cs_ctx.state != CS_STATE_IDLE { play.cs_ctx.npc_actions[slot] } else { None }
    }

    /// `Audio_PlaySfxGeneral(sfx, &gSfxDefaultPos, 4, ..)`.
    fn sfx_default(play: &mut PlayState, sfx: u16) {
        play.audio.play_sfx_general(sfx, SfxPos::Default, 4, SfxF32::One, SfxF32::One, SfxS8::Zero);
    }

    /// `EnViewer_UpdateImpl`.
    fn update_impl(&mut self, play: &mut PlayState) {
        let ty = self.ty;
        let layer = play.save.scene_layer;
        let frames = play.cs_ctx.frames;
        if ty == ENVIEWER_TYPE_2_ZELDA {
            if layer == 5 {
                if frames == 792 {
                    audio_play_actor_sfx2(play, NA_SE_VO_Z0_SURPRISE);
                } else if frames == 845 {
                    audio_play_actor_sfx2(play, NA_SE_VO_Z0_THROW);
                }
            }
        } else if ty == ENVIEWER_TYPE_7_GANONDORF {
            // Actor_SetScale(0.3), the cull zones 10000.
            self.actor.scale = Vec3::splat(0.3);
        } else if ty == ENVIEWER_TYPE_3_GANONDORF {
            if layer == 4 && matches!(frames, 20 | 59 | 71 | 129 | 140 | 219 | 280 | 320 | 380 | 409 | 438) {
                Self::sfx_default(play, NA_SE_SY_DEMO_CUT);
            }
            if layer == 5 {
                if frames == 1508 {
                    audio_play_actor_sfx2(play, NA_SE_EN_FANTOM_ST_LAUGH);
                }
                if frames == 1545 {
                    let _ = play.actor_spawn_as_child(&mut self.actor, ACTOR_DEMO_6K, Vec3::new(32.0, 101.0, 1226.0), [0; 3], 0xC);
                }
            }
            if frames == 1020 {
                // Audio_QueueSeqCmd(SEQ_PLAYER_FANFARE << 24 | NA_BGM_OPENING_GANON).
                play.audio.queue_seq_cmd(((SEQ_PLAYER_FANFARE as u32) << 24) | NA_BGM_OPENING_GANON as u32);
            }
            if frames == 960 {
                let pos = cur_sfx_pos(play);
                play.audio.play_sfx_general(NA_SE_EV_HORSE_GROAN, pos, 4, SfxF32::One, SfxF32::One, SfxS8::Zero);
            }
        } else if ty == ENVIEWER_TYPE_6_HORSE_GANONDORF {
            if layer == 5 || layer == 10 {
                audio_play_actor_sfx2(play, NA_SE_EV_HORSE_RUN_LEVEL - SFX_FLAG);
            }
        } else if ty == ENVIEWER_TYPE_4_HORSE_GANONDORF {
            let cur_frame = self.skel.as_ref().map_or(0, |s| s.cur_frame as i32 as i16);
            if self.is("gHorseGanonRearingAnim") {
                if cur_frame == 8 {
                    audio_play_actor_sfx2(play, NA_SE_EV_GANON_HORSE_NEIGH);
                }
                if cur_frame == 30 {
                    audio_play_actor_sfx2(play, NA_SE_EV_HORSE_LAND2);
                }
            } else if self.is("gHorseGanonIdleAnim") {
                if cur_frame == 25 {
                    audio_play_actor_sfx2(play, NA_SE_EV_HORSE_SANDDUST);
                }
            } else if self.is("gHorseGanonGallopingAnim") {
                audio_play_actor_sfx2(play, NA_SE_EV_HORSE_RUN_LEVEL - SFX_FLAG);
            }
        }

        {
            let st = play.overlay_static::<Statics>(ACTOR_EN_VIEWER);
            if st.timer != 0 {
                st.timer -= 1;
            }
        }

        self.update_position(play);
        // Actor_MoveForward: no effect (speed, velocity and gravity are 0).

        let animation_ended = self.skel.as_mut().is_some_and(|s| s.update());
        let timer = play.overlay_static::<Statics>(ACTOR_EN_VIEWER).timer;
        if ty == ENVIEWER_TYPE_3_GANONDORF || ty == ENVIEWER_TYPE_4_HORSE_GANONDORF {
            if let Some(cue) = Self::cue(play, 1) {
                if cue.action == 2 && timer == 0 {
                    if ty == ENVIEWER_TYPE_3_GANONDORF {
                        if !self.is("gYoungGanondorfHorsebackIdleAnim") {
                            self.play_loop_set_speed(play, "gYoungGanondorfHorsebackIdleAnim", 1.0);
                        }
                    } else if !self.is("gHorseGanonIdleAnim") {
                        self.play_loop_set_speed(play, "gHorseGanonIdleAnim", 1.0);
                    }
                } else if cue.action == 1 {
                    play.overlay_static::<Statics>(ACTOR_EN_VIEWER).timer = 100;
                    if ty == ENVIEWER_TYPE_3_GANONDORF {
                        if !self.is("gYoungGanondorfHorsebackRearAnim") {
                            self.play_loop_set_speed(play, "gYoungGanondorfHorsebackRearAnim", 1.0);
                        }
                    } else if !self.is("gHorseGanonRearingAnim") {
                        self.play_loop_set_speed(play, "gHorseGanonRearingAnim", 1.0);
                    }
                } else if ty == ENVIEWER_TYPE_3_GANONDORF {
                    match self.state {
                        0 => {
                            if cue.action == 4 {
                                self.morph_to_play_once(play, "gYoungGanondorfHorsebackLookSidewaysStartAnim", -5.0);
                                self.state += 1;
                            }
                        }
                        1 => {
                            if animation_ended {
                                self.morph_to_loop(play, "gYoungGanondorfHorsebackLookSidewaysLoopAnim", -5.0);
                                self.state += 1;
                            }
                        }
                        2 => {
                            if cue.action == 5 {
                                self.morph_to_play_once(play, "gYoungGanondorfHorsebackMagicChargeUpStartAnim", -5.0);
                                self.state += 1;
                            }
                        }
                        3 => {
                            if animation_ended {
                                self.morph_to_loop(play, "gYoungGanondorfHorsebackMagicChargeUpLoopAnim", -5.0);
                                self.state += 1;
                            }
                        }
                        4 => {
                            if cue.action == 11 {
                                self.morph_to_loop(play, "gYoungGanondorfHorsebackLookSidewaysLoopAnim", -20.0);
                                self.state += 1;
                            }
                        }
                        5 => {
                            if cue.action == 8 {
                                self.morph_to_loop(play, "gYoungGanondorfHorsebackIdleAnim", -15.0);
                                self.state += 1;
                            }
                        }
                        6 => {
                            if cue.action == 12 {
                                audio_play_actor_sfx2(play, NA_SE_EN_GANON_VOICE_DEMO);
                                self.play_loop_set_speed(play, "gYoungGanondorfHorsebackRideAnim", 3.0);
                                self.state += 1;
                            }
                        }
                        7 => self.state = 0,
                        _ => {}
                    }
                } else if !self.is("gHorseGanonGallopingAnim") && cue.action == 12 {
                    self.play_loop_set_speed(play, "gHorseGanonGallopingAnim", 3.0);
                }
            }
        } else if ty == ENVIEWER_TYPE_1_IMPA {
            if layer == 5 {
                if frames == 845 {
                    let _ = play.actor_spawn_as_child(&mut self.actor, ACTOR_ITEM_OCARINA, Vec3::new(4.0, 81.0, 2600.0), [0; 3], 0);
                }
            } else if frames == 195 {
                let _ = play.actor_spawn_as_child(&mut self.actor, ACTOR_ITEM_OCARINA, Vec3::new(4.0, 81.0, 2035.0), [0; 3], 1);
            }
            let cue = Self::cue(play, 0);
            match self.state {
                0 => {
                    if cue.is_some_and(|c| c.action == 6) && !self.is(ANIM_IMPA_LOOK_BACK) {
                        self.play_loop_set_speed(play, ANIM_IMPA_LOOK_BACK, 1.5);
                        self.state += 1;
                    }
                }
                1 => {
                    if cue.is_some_and(|c| c.action == 2) && !self.is(ANIM_IMPA_RIDE) {
                        self.play_loop_set_speed(play, ANIM_IMPA_RIDE, 3.0);
                        self.state += 1;
                    }
                }
                _ => {}
            }
        } else if ty == ENVIEWER_TYPE_2_ZELDA {
            if play.scene_id == SCENE_SPOT00 {
                let cue = Self::cue(play, 0);
                match self.state {
                    0 => {
                        if cue.is_some_and(|c| c.action == 6) && !self.is(ANIM_ZELDA_LOOK_BACK) {
                            self.play_loop_set_speed(play, ANIM_ZELDA_LOOK_BACK, 1.5);
                            self.state += 1;
                        }
                    }
                    1 => {
                        if cue.is_some_and(|c| c.action == 2) && !self.is(ANIM_ZELDA_RIDE) {
                            self.play_loop_set_speed(play, ANIM_ZELDA_RIDE, 3.0);
                            self.state += 1;
                        }
                    }
                    _ => {}
                }
            } else {
                play.audio.set_base_filter(0);
                match self.state {
                    0 => {
                        self.play_loop_set_speed(play, ANIM_ZELDA_00504C, 1.0);
                        self.state += 1;
                    }
                    1 => {
                        // The C reads npcActions[0]->action without checking it.
                        if play.cs_ctx.npc_actions[0].is_some_and(|c| c.action == 11) {
                            self.morph_to_play_once(play, ANIM_ZELDA_00420C, -5.0);
                            self.state += 1;
                        }
                    }
                    2 => {
                        if animation_ended {
                            self.morph_to_loop(play, ANIM_ZELDA_0048FC, -5.0);
                            self.state += 1;
                        }
                    }
                    _ => {}
                }
            }
        } else if ty == ENVIEWER_TYPE_7_GANONDORF {
            match self.state {
                0 => {
                    if Self::cue(play, 1).is_some_and(|c| c.action == 7) {
                        Self::sfx_default(play, NA_SE_EN_GANON_LAUGH);
                        self.morph_to_play_once(play, "gYoungGanondorfLaughStartAnim", -5.0);
                        self.state += 1;
                    }
                }
                1 => {
                    if animation_ended {
                        self.morph_to_loop(play, "gYoungGanondorfLaughLoopAnim", -5.0);
                        self.state += 1;
                    }
                }
                _ => {}
            }
        } else if ty == ENVIEWER_TYPE_8_GANONDORF {
            match self.state {
                0 => {
                    if Self::cue(play, 1).is_some_and(|c| c.action == 9) {
                        self.play_loop_set_speed(play, "gYoungGanondorfWalkAnim", 1.0);
                        self.state += 1;
                    }
                }
                1 => {
                    if play.cs_ctx.npc_actions[1].is_some_and(|c| c.action == 10) {
                        self.morph_to_play_once(play, "gYoungGanondorfKneelStartAnim", -10.0);
                        self.state += 1;
                    }
                }
                2 => {
                    if animation_ended {
                        self.morph_to_loop(play, "gYoungGanondorfKneelLoopAnim", -5.0);
                        self.state += 1;
                    }
                }
                3 => {
                    if play.cs_ctx.npc_actions[1].is_some_and(|c| c.action == 4) {
                        self.morph_to_play_once(play, "gYoungGanondorfKneelLookSidewaysAnim", -5.0);
                        self.state += 1;
                    }
                }
                _ => self.state = 0,
            }
        }
    }

    /// `EnViewer_UpdatePosition`: along the cue from its start to its end position by the frame
    /// (`Environment_LerpWeight`); Ganondorf turns to his way on action 12; the adult Ganondorf
    /// takes the cue's rotation; type 5 burns.
    fn update_position(&mut self, play: &mut PlayState) {
        let slot = self.cue_slot();
        if let Some(cue) = Self::cue(play, slot)
            && play.cs_ctx.frames < cue.end_frame
        {
            if self.ty == ENVIEWER_TYPE_0_HORSE_ZELDA {
                let st = play.overlay_static::<Statics>(ACTOR_EN_VIEWER);
                if !st.horse_sfx_played {
                    st.horse_sfx_played = true;
                    let pos = cur_sfx_pos(play);
                    play.audio.play_sfx_general(NA_SE_EV_HORSE_NEIGH, pos, 4, SfxF32::One, SfxF32::One, SfxS8::Zero);
                }
                audio_play_actor_sfx2(play, NA_SE_EV_HORSE_RUN_LEVEL - SFX_FLAG);
            }
            let start = cue.start_pos.as_vec3();
            let end = cue.end_pos.as_vec3();
            let lerp = lerp_weight(cue.end_frame, cue.start_frame, play.cs_ctx.frames);
            self.actor.world_pos = Vec3::new((end.x - start.x) * lerp + start.x, (end.y - start.y) * lerp + start.y, (end.z - start.z) * lerp + start.z);
            if slot == 1 {
                if cue.action == 12 {
                    let yaw = vec3f_yaw(start, end);
                    smooth_step_to_s(&mut self.actor.world_rot.y, yaw, 0xA, 0x3E8, 1);
                    smooth_step_to_s(&mut self.actor.shape_rot.y, yaw, 0xA, 0x3E8, 1);
                }
                if self.ty == ENVIEWER_TYPE_9_GANONDORF {
                    let r = cue.rot;
                    self.actor.world_rot = oot_game::actor::Rot { x: r[0], y: r[1], z: r[2] };
                    self.actor.shape_rot = oot_game::actor::Rot { x: r[0], y: r[1], z: r[2] };
                }
            }
        }
        if self.ty == ENVIEWER_TYPE_5_GANONDORF {
            Self::sfx_default(play, NA_SE_EV_BURNING - SFX_FLAG);
            self.update_fire_effects(play);
        }
    }

    /// `EnViewer_InitFireEffect`: from x ±100 (even, odd), 400 to -400 along z, at -420.
    fn init_fire_effect(&mut self, play: &mut PlayState, i: usize) {
        let x = if i % 2 == 0 { 100.0 } else { -100.0 };
        let e = &mut self.fire_effects[i];
        e.start_pos = Vec3::new(x, -420.0, 400.0);
        e.end_pos = Vec3::new(x, -420.0, -400.0);
        e.scale = (play.rand.zero_one() * 5.0 + 12.0) * 0.001;
    }

    /// `EnViewer_DrawFireEffects`' state (it runs in the update): each flame along its way by
    /// `lerpFactor`, starting over at the end.
    fn update_fire_effects(&mut self, play: &mut PlayState) {
        for i in 0..self.fire_effects.len() {
            match self.fire_effects[i].state {
                0 => {
                    self.init_fire_effect(play, i);
                    let e = &mut self.fire_effects[i];
                    e.lerp_factor = (i >> 1) as f32 * 0.1;
                    e.lerp_factor_speed = 0.01;
                    e.state += 1;
                }
                1 => {
                    let e = &mut self.fire_effects[i];
                    smooth_step_to_f(&mut e.lerp_factor, 1.0, 1.0, e.lerp_factor_speed, e.lerp_factor_speed);
                    e.pos = e.start_pos + (e.end_pos - e.start_pos) * e.lerp_factor;
                    if e.lerp_factor >= 1.0 {
                        e.state += 1;
                    }
                }
                2 => {
                    self.init_fire_effect(play, i);
                    let e = &mut self.fire_effects[i];
                    e.lerp_factor = 0.0;
                    e.lerp_factor_speed = 0.01;
                    e.state -= 1;
                }
                _ => {}
            }
        }
    }

    /// Whether `EnViewer_Draw` draws this frame: Zelda, Impa and their horse with cue 0, the
    /// others with cue 1 (the adult Ganondorf always).
    fn drawn(&self, play: &PlayState) -> bool {
        if !self.is_visible {
            return false;
        }
        if self.ty <= ENVIEWER_TYPE_2_ZELDA { Self::cue(play, 0).is_some() } else { Self::cue(play, 1).is_some() || self.ty == ENVIEWER_TYPE_9_GANONDORF }
    }

    /// `EnViewer_DrawZelda`'s eyes (index into `ZELDA_EYES`) and mouth by the frame.
    fn zelda_face(play: &PlayState) -> (bool, usize, usize) {
        if play.scene_id != SCENE_SPOT00 {
            return (false, 4, 0);
        }
        let f = play.cs_ctx.frames;
        let eyes = if f < 771 {
            0
        } else if f < 772 {
            1
        } else if f < 773 {
            2
        } else if f < 791 {
            3
        } else if f < 792 {
            1
        } else if f < 793 {
            2
        } else {
            0
        };
        let mouth = if play.save.scene_layer == 6 || (758..848).contains(&f) { 1 } else { 0 };
        (true, eyes, mouth)
    }

    /// `EnViewer_DrawGanondorf`'s eyes for the young Ganondorf, and its open hand
    /// (`EnViewer_Ganondorf3OverrideLimbDraw`, type 3 only).
    fn ganondorf_face(&self, play: &PlayState) -> (usize, bool) {
        let frames = if play.save.scene_layer != 4 { 149 } else { 0 };
        let f = play.cs_ctx.frames as i32;
        let eyes = if frames + 1127 >= f {
            0
        } else if frames + 1128 >= f {
            1
        } else if frames + 1129 >= f {
            2
        } else {
            3
        };
        let open = self.ty == ENVIEWER_TYPE_3_GANONDORF && if play.save.scene_layer == 4 { play.cs_ctx.frames >= 400 } else { (1510..=1650).contains(&play.cs_ctx.frames) };
        (eyes, open)
    }
}

impl ActorImpl for EnViewer {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnViewer_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::InitImpl => self.init_impl(play),
            Action::UpdateImpl => self.update_impl(play),
        }
    }

    /// `EnViewer_GanondorfPostLimbDrawUpdateCapeVec`: the head limb's origin
    /// (`sGanondorfNeckWorldPos`) as the young Ganondorf's draw finds it.
    fn draw_update(&mut self, play: &mut PlayState) {
        if self.draw_func_index != ENVIEWER_DRAW_GANONDORF || self.ty == ENVIEWER_TYPE_9_GANONDORF || !self.drawn(play) {
            return;
        }
        let (Model::Flex(s), Some(sk)) = (&self.model, &self.skel) else { return };
        let bones = s.pose(&JointTable { rot: sk.joint_table.clone(), face: 0 });
        if let Some(head) = bones.get(YOUNG_GANONDORF_LIMB_HEAD - 1) {
            let m = actor_draw_matrix(&RenderState::of(&self.actor)) * *head;
            play.overlay_static::<Statics>(ACTOR_EN_VIEWER).ganondorf_neck_world_pos = m.transform_point3(Vec3::ZERO);
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        if let Some(s) = &self.skel {
            rs.joints = Some(JointTable { rot: s.joint_table.clone(), face: 0 });
        }
        rs
    }

    /// `EnViewer_Draw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if !self.drawn(play) {
            return;
        }
        let Some(joints) = &rs.joints else { return };
        match (&self.model, self.draw_func_index) {
            (Model::Skin(s), ENVIEWER_DRAW_HORSE) => {
                // EnViewer_DrawHorse: func_800A6330(.., setTranslation true).
                let place = SkinPlace { scale: rs.scale, rot: rs.rot, pos: rs.pos, y_offset: rs.y_offset };
                let (t, bones) = skin_draw(s, &joints.rot, &place, true);
                out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&horse_bake(self.data().skeleton))), transform: t, bones, params: DrawParams::default() });
            }
            (Model::Flex(s), ty) => {
                let name = match ty {
                    ENVIEWER_DRAW_ZELDA => {
                        let (field, eyes, mouth) = Self::zelda_face(play);
                        zelda_bake(field, eyes, mouth)
                    }
                    ENVIEWER_DRAW_IMPA => IMPA_BAKE.to_string(),
                    _ if self.ty == ENVIEWER_TYPE_9_GANONDORF => {
                        log::trace!("En_Viewer: the adult Ganondorf's draw isn't ported");
                        return;
                    }
                    _ => {
                        let (eyes, open) = self.ganondorf_face(play);
                        ganondorf_bake(eyes, open)
                    }
                };
                let bones = s.pose(joints);
                out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&name)), transform: actor_draw_matrix(rs), bones, params: DrawParams::default() });
            }
            _ => {}
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
