//! `Demo_Du` (`ovl_Demo_Du/z_demo_du.c`): Darunia in four cutscenes, by params: the Chamber of
//! Sages' Fire Medallion (0, the default), the Goron's Ruby (1), the Chamber of Sages after
//! Ganon's last blow (2), the credits (3).
//!
//! **The Goron's Ruby** (`DEMO_DU_CS_GORONS_RUBY`, `gDMTGoronRubyCs`, Death Mountain Trail's layer
//! 5, King Dodongo's blue warp's arrival), on his cue on channel 2 (`actorCues[2]`):
//! - Hidden (`drawIndex` 0) through cue 1; at frame 160 the fall's whistle
//!   (`NA_SE_EV_OBJECT_FALL`, centred), at 174 Link's surprise (`NA_SE_VO_LI_SURPRISE_KID`).
//! - Cue 2 (`DemoDu_CsGoronsRuby_AdvanceTo02`): drawn, `gDaruniaStandUpAfterFallingAnim` set
//!   (held on its first frame), dropping along the cue from its start to its end
//!   (`DemoDu_CsGoronsRuby_DaruniaFalling`); at its end frame he lands (`func_8096A630`): 11 puffs of
//!   dust 100 across (`func_80033480`), `NA_SE_EN_GOLON_LAND_BIG`.
//! - Cue 3: he stands up, then idles (`gDaruniaIdleAnim`).
//! - Cue 4: he beats his chest (`gDaruniaHitBreastAnim`, a 4-frame morph,
//!   `NA_SE_EN_DARUNIA_HIT_BREAST` held every frame of it), then idles.
//! - Cue 5: he slaps Link's back (`gDaruniaHitLinkAnim`): on its frames 31 and 41 five puffs of dust
//!   (`func_8002829C`, brown, each shade moved by -10 to 9) about Link's left forearm, and
//!   `NA_SE_EN_DARUNIA_HIT_LINK` and Link's `NA_SE_VO_LI_DAMAGE_S_KID`; then his chest again, then
//!   idle.
//! - Cue 6: he gives the Ruby (`gDaruniaItemGiveAnim`, then `gDaruniaItemGiveIdleAnim` looped).
//! - Cue 7: idle to the end; at frame 1400 Link's cry as the Gorons chase him
//!   (`NA_SE_VO_LI_FALL_L_KID`).
//!
//! From his stand on, his face follows the script's frame
//! (`DemoDu_CsGoronsRuby_UpdateFaceTextures`): blinking (`DemoDu_UpdateEyes`), the serious mouth
//! to frame 260, happy to 335, eyes closing and the grin to 365, eyes open and happy to 395, blinking
//! and serious to 410, then blinking and happy. Every state from the fall on keeps him on the floor
//! (`Actor_UpdateBgCheckInfo`, floor and walls).
//!
//! Drawn as `DemoDu_Draw` does by `drawIndex`: nothing, `DemoDu_Draw_01` (the skeleton
//! `gDaruniaSkel` opaque, the eyes on segment 8, the mouth on 9, `gDaruniaNoseSeriousTex` on 0xA,
//! `ACTOR_SETUP_OPA_DL` on 0xC, env alpha 255: `En_Du`'s bake, the textures swapped per draw) or
//! `DemoDu_Draw_02` (the same translucent, `gActorSetupXluDL` on 0xC, env alpha `shadowAlpha`: this
//! module's bake). His circle shadow (30) is the shape's (`ActorShape_Init`'s, drawn by
//! `Actor_Draw` once he has a floor).
//!
//! Ported: the Goron's Ruby whole (`DemoDu_InitCs_GoronsRuby` .. `DemoDu_UpdateCs_GR_13`) and the
//! shared functions (`DemoDu_UpdateEyes`, `_SetEyeTexIndex`, `_SetMouthTexIndex`,
//! `_UpdateSkelAnime`, `_UpdateBgCheckInfo`, `_GetCue`, `_CheckForCue`, `_CheckForNoCue`,
//! `_SetStartPosRotFromCue`, `func_80969DDC`, the update and draw tables, `DemoDu_Draw_01` and
//! `_02`, the destroy). **Not ported** (the user's call): the other three cutscenes, chosen at
//! init (`DemoDu_Init`'s switch) and spawned as placeholders with a `log::info!`: the Fire
//! Medallion (Phase 15: `DemoDu_InitCs_FireMedallion` .. `DemoDu_UpdateCs_FM_06`, `gFireMedallionCs`,
//! `Door_Warp1`'s `WARP_SAGES`, `Demo_Effect`'s `DEMO_EFFECT_MEDAL_FIRE`, `Item_Give`), the
//! Chamber of Sages after Ganon (Phase 19: `DemoDu_InitCs_AfterGanon` .. `_AG_02`, `Demo_6K`, the
//! fade) and the credits (Phase 19: `DemoDu_InitCs_Credits` .. `_CR_04`).
//!
//! Placed: Death Mountain Trail layers 5 (the Ruby's cutscene) and 6 (`gDMTUnusedCs`, no cues: he
//! stays hidden), params 1; layer 8 (`gDMTCreditsSixSagesCs`) params 3; the Chamber of Sages'
//! layers 0 to 3 and 5 params 0, layer 6 params 2; Inside Ganon's Castle's layers 0 to 3 params 2
//! (tilted, `rot.x` 0x2000).
//!
//! Params: `params` whole, the cutscene (0, and any other not listed: the Chamber of Sages' Fire
//! Medallion, cue channels 2 and 6; 1: the Goron's Ruby, cue channel 2; 2: the Chamber of Sages
//! after Ganon, channel 2; 3: the credits, channel 2); only 1 is ported. `shape.rot.x`: a tilt,
//! drawn as placed (Inside Ganon's Castle's, 0x2000).
//! Flags: none for the Ruby; the Fire Medallion (not ported) reads
//! `gSaveContext.chamberCutsceneNum` (CHAMBER_CS_FIRE outside a cutscene layer starts it).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_gfx::{DrawCmd, DrawImage, DrawParams, MeshKey, SegmentValues, SourceImage};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, UPDBGCHECKINFO_FLAG_0, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile, func_80033480};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, Source};
use oot_game::audio::sfx::{SFX_FLAG, SfxPos};
use oot_game::cutscene::{CS_STATE_IDLE, CsCmdActorCue};
use oot_game::gbi::Dl;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::*;

use crate::player::Player;

/// `ACTOR_DEMO_DU` (`actor_table.h`: 0x00A8).
pub const ACTOR_DEMO_DU: i16 = 0x00A8;
/// `OBJECT_DU` (`object_table.h`: 0x009D).
pub const OBJECT: &str = "object_du";

/// `Demo_Du_Profile`: `ACTORCAT_NPC`, `FLAGS` (`ACTOR_FLAG_UPDATE_CULLING_DISABLED`).
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_DEMO_DU, name: "Demo_Du", category: ACTORCAT_NPC, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `DemoDu_Cutscene` (`z_demo_du.h`).
pub const DEMO_DU_CS_FIREMEDALLION: i16 = 0;
pub const DEMO_DU_CS_GORONS_RUBY: i16 = 1;
pub const DEMO_DU_CS_CHAMBER_AFTER_GANON: i16 = 2;
pub const DEMO_DU_CS_CREDITS: i16 = 3;

/// `this->actor.params` (`DemoDu_Init`'s switch): the cutscene.
pub const TYPE: Field = Field::whole(
    "params",
    Kind::Named(&[
        (0, "the Chamber of Sages: the Fire Medallion (any other not listed too; not ported, Phase 15: cue channels 2 and 6)"),
        (1, "the Goron's Ruby (Death Mountain Trail's layer 5): cue channel 2"),
        (2, "the Chamber of Sages after Ganon's last blow (not ported, Phase 19: cue channel 2)"),
        (3, "the credits (not ported, Phase 19: cue channel 2)"),
    ]),
    "which cutscene Darunia is in",
);
/// `shape.rot.x`: Inside Ganon's Castle's Darunia (params 2) is placed tilted (0x2000), drawn so.
pub const TILT: Field = Field::rot("shape.rot.x", Source::RotX, 0, 16, Kind::Rotation, "his tilt, drawn as placed");

/// Demo_Du's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, TILT],
    flags: &[FlagUse::new(Flag::Other("gSaveContext.chamberCutsceneNum"))
        .reads()
        .note("params 0: CHAMBER_CS_FIRE outside a cutscene layer starts the Fire Medallion's cutscene (gFireMedallionCs) and gives the medallion")
        .indirect("not ported: DemoDu_CsFireMedallion_AdvanceTo01 (the Fire Medallion, Phase 15)")],
    todo: false,
    objects: &[],
    texts: &[],
};

/// `CS_GORONSRUBY_SUBSCENE(x)`: `sUpdateFuncs`' index of `DemoDu_UpdateCs_GR_xx`.
pub const fn cs_goronsruby_subscene(x: i32) -> i32 {
    7 + x
}
/// `CS_FIREMEDALLION_SUBSCENE(0)`, `CS_CHAMBERAFTERGANON_SUBSCENE(0)`, `CS_CREDITS_SUBSCENE(0)`.
pub const CS_FIREMEDALLION_SUBSCENE_0: i32 = 0;
pub const CS_CHAMBERAFTERGANON_SUBSCENE_0: i32 = 21;
pub const CS_CREDITS_SUBSCENE_0: i32 = 24;
/// `sUpdateFuncs`' length.
const UPDATE_FUNCS: i32 = 29;
/// `sDrawFuncs`' length: `DemoDu_Draw_NoDraw`, `DemoDu_Draw_01`, `DemoDu_Draw_02`.
const DRAW_FUNCS: i32 = 3;

// The sounds (`include/tables/sfx/*.h`).
/// `NA_SE_EN_GOLON_LAND_BIG` (`enemybank_table.h`: 0x3879).
pub const NA_SE_EN_GOLON_LAND_BIG: u16 = 0x3879;
/// `NA_SE_EN_DARUNIA_HIT_BREAST` (`enemybank_table.h`: 0x3924).
pub const NA_SE_EN_DARUNIA_HIT_BREAST: u16 = 0x3924;
/// `NA_SE_EN_DARUNIA_HIT_LINK` (`enemybank_table.h`: 0x3925).
pub const NA_SE_EN_DARUNIA_HIT_LINK: u16 = 0x3925;
/// `NA_SE_EV_OBJECT_FALL` (`environmentbank_table.h`: 0x28A0).
pub const NA_SE_EV_OBJECT_FALL: u16 = crate::demo_go::NA_SE_EV_OBJECT_FALL;
/// `NA_SE_VO_LI_DAMAGE_S_KID` (`voicebank_table.h`: 0x6825).
pub const NA_SE_VO_LI_DAMAGE_S_KID: u16 = 0x6825;
/// `NA_SE_VO_LI_FALL_L_KID` (`voicebank_table.h`: 0x6828).
pub const NA_SE_VO_LI_FALL_L_KID: u16 = 0x6828;
/// `NA_SE_VO_LI_SURPRISE_KID` (`voicebank_table.h`: 0x6836).
pub const NA_SE_VO_LI_SURPRISE_KID: u16 = 0x6836;

/// `PLAYER_BODYPART_L_FOREARM` (`player.h`: 0x0B).
const PLAYER_BODYPART_L_FOREARM: usize = 0x0B;

/// The debug registers `kREG(1)` to `kREG(5)` (0: they start zeroed).
const KREG_1: f32 = 0.0;
const KREG_2: i32 = 0;
const KREG_3: i32 = 0;
const KREG_4: i32 = 0;
const KREG_5: f32 = 0.0;

/// `sEyeTextures`, `sMouthTextures`, and the nose both draws bind.
pub const EYE_TEXTURES: [&str; 4] = ["gDaruniaEyeOpenTex", "gDaruniaEyeOpeningTex", "gDaruniaEyeShutTex", "gDaruniaEyeClosingTex"];
pub const MOUTH_TEXTURES: [&str; 4] = ["gDaruniaMouthSeriousTex", "gDaruniaMouthGrinningTex", "gDaruniaMouthOpenTex", "gDaruniaMouthHappyTex"];
pub const NOSE_TEXTURE: &str = "gDaruniaNoseSeriousTex";
/// The segments the draws bind: the eyes (8), the mouth (9), the nose (0xA); a draw swaps its
/// textures in by these addresses (`En_Du`'s bake binds each to a different texture, as this
/// module's does).
const SEG_EYES: u32 = 0x0800_0000;
const SEG_MOUTH: u32 = 0x0900_0000;
const SEG_NOSE: u32 = 0x0A00_0000;
/// Segment 0xC: `ACTOR_SETUP_OPA_DL` (`&gActorSetupXluDL[2]`, an end) or `gActorSetupXluDL`.
const SEG_SETUP: u8 = 0x0C;
/// `gDPSetEnvColor(0, 0, 0, alpha)`, as a dynamic colour run before the skeleton (`En_Du`'s too).
const SEG_ALPHA: u8 = 0x0E;

/// `DemoDu_Draw_01`'s mesh: `En_Du`'s (the same skeleton, segments and setup).
pub const BAKE_OPA: &str = crate::en_du::BAKE;
/// `DemoDu_Draw_02`'s: the skeleton translucent.
pub const BAKE_XLU: &str = "Demo_Du/xlu";

// shared: gActorSetupXluDL (z_actor.c), as demo_sa.rs builds it.
/// `gActorSetupXluDL`: `gsDPSetRenderMode(G_RM_FOG_SHADE_A, G_RM_AA_ZB_XLU_SURF2 | Z_UPD)`,
/// `gsDPSetAlphaCompare(G_AC_THRESHOLD)`, the end.
fn actor_setup_xlu_dl() -> Vec<(u32, u32)> {
    let mut d = Dl::default();
    d.0.push((0xE200_001C, 0xC810_49F8));
    d.othermode_l(0, 2, 1);
    d.end();
    d.0
}

/// The mesh `DemoDu_Draw_02` draws (`Gfx_SetupDL_25Xlu`; the eyes, mouth and nose bound to segments
/// 8, 9, 0xA, the draw swapping them; `gActorSetupXluDL` on 0xC; the env alpha). `DemoDu_Draw_01`'s
/// is `En_Du`'s.
pub fn bakes() -> Vec<MeshBake> {
    let tex = |s: &str| BakeSegment::Texture { file: OBJECT.into(), symbol: s.into() };
    vec![MeshBake {
        name: BAKE_XLU.into(),
        object: OBJECT.into(),
        segments: vec![
            (0x08, tex(EYE_TEXTURES[0])),
            (0x09, tex(MOUTH_TEXTURES[0])),
            (0x0A, tex(NOSE_TEXTURE)),
            (SEG_SETUP, BakeSegment::Commands(actor_setup_xlu_dl())),
            (SEG_ALPHA, BakeSegment::DynamicColor { env: true, prim: false }),
        ],
        prelude: vec![SEG_ALPHA],
        body: BakeBody::Skeleton { file: OBJECT.into(), symbol: "gDaruniaSkel".into(), limbs: Vec::new() },
    }]
}

/// The Ruby's animations.
pub const ANIM_STAND_UP_AFTER_FALLING: &str = "gDaruniaStandUpAfterFallingAnim";
pub const ANIM_IDLE: &str = "gDaruniaIdleAnim";
pub const ANIM_HIT_BREAST: &str = "gDaruniaHitBreastAnim";
pub const ANIM_HIT_LINK: &str = "gDaruniaHitLinkAnim";
pub const ANIM_ITEM_GIVE: &str = "gDaruniaItemGiveAnim";
pub const ANIM_ITEM_GIVE_IDLE: &str = "gDaruniaItemGiveIdleAnim";

/// `dustPosOffsets` (`DemoDu_CsGoronsRuby_SpawnDustWhenHittingLink`).
const DUST_POS_OFFSETS: [Vec3; 10] = [
    Vec3::new(11.0, -11.0, -6.0),
    Vec3::new(0.0, 14.0, -13.0),
    Vec3::new(14.0, -2.0, -10.0),
    Vec3::new(10.0, -6.0, -8.0),
    Vec3::new(8.0, 6.0, 8.0),
    Vec3::new(13.0, 8.0, -10.0),
    Vec3::new(-14.0, 1.0, -14.0),
    Vec3::new(5.0, 12.0, -9.0),
    Vec3::new(11.0, 6.0, -7.0),
    Vec3::new(14.0, 14.0, -14.0),
];

struct Anims {
    stand_up_after_falling: Anim,
    idle: Anim,
    hit_breast: Anim,
    hit_link: Anim,
    item_give: Anim,
    item_give_idle: Anim,
}

pub struct DemoDu {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub eye_tex_index: i16,
    pub blink_timer: i16,
    pub mouth_tex_index: i16,
    /// `updateIndex`: the index into `sUpdateFuncs`.
    pub update_index: i32,
    /// `drawIndex`: the index into `sDrawFuncs`.
    pub draw_index: i32,
    pub unused: i32,
    pub unk_1a4: f32,
    /// `shadowAlpha`: `DemoDu_Draw_02`'s env alpha.
    pub shadow_alpha: i32,
    pub demo6k_spawned: i32,
    pub cue_id: i32,
    anims: Anims,
    /// The face textures as images: `EYE_TEXTURES`, `MOUTH_TEXTURES`, `NOSE_TEXTURE`.
    face: Vec<DrawImage>,
    not_ported_logged: bool,
}

/// `DECR`: 0 when it was 0, else the decremented value.
fn decr(x: &mut i16) -> i16 {
    if *x == 0 {
        0
    } else {
        *x -= 1;
        *x
    }
}

fn player(play: &PlayState) -> Option<&Player> {
    play.player.and_then(|h| play.actors.downcast::<Player>(h))
}

/// `&player->actor.projectedPos`.
fn player_sfx_pos(play: &PlayState) -> SfxPos {
    play.player.map(SfxPos::Actor).unwrap_or(SfxPos::Default)
}

impl DemoDu {
    /// `DemoDu_Init`: the circle shadow (30), then the cutscene's init; the three not ported are
    /// placeholders.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut actor = actor;
        actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 30.0);
        let what = match TYPE.get(actor.params) {
            DEMO_DU_CS_GORONS_RUBY => None,
            DEMO_DU_CS_CHAMBER_AFTER_GANON => Some("DemoDu_InitCs_AfterGanon: the Chamber of Sages after Ganon's last blow (Phase 19)"),
            DEMO_DU_CS_CREDITS => Some("DemoDu_InitCs_Credits: the credits (Phase 19)"),
            _ => Some("DemoDu_InitCs_FireMedallion: the Chamber of Sages' Fire Medallion (Phase 15)"),
        };
        if let Some(what) = what {
            log::info!("Demo_Du: {what} isn't ported: a placeholder");
            // DemoDu_CsFireMedallion_AdvanceTo01 (the Fire Medallion's type).
            if !matches!(TYPE.get(actor.params), DEMO_DU_CS_CHAMBER_AFTER_GANON | DEMO_DU_CS_CREDITS) && play.save.chamber_cutscene_num == oot_game::save::CHAMBER_CS_FIRE && !play.is_cutscene_layer() {
                log::warn!("Demo_Du: chamberCutsceneNum CHAMBER_CS_FIRE: the Fire Medallion's cutscene (gFireMedallionCs) would start (Phase 15)");
            }
            return oot_game::spawn::Placeholder::init(actor, play);
        }
        let Some(assets) = play.assets.clone() else {
            actor.kill();
            return oot_game::spawn::Placeholder::init(actor, play);
        };
        let loaded = (|| -> anyhow::Result<_> {
            let skeleton = assets.skeleton(OBJECT, "gDaruniaSkel")?;
            let anim = |s: &str| assets.animation(OBJECT, s);
            let anims = Anims {
                stand_up_after_falling: anim(ANIM_STAND_UP_AFTER_FALLING)?,
                idle: anim(ANIM_IDLE)?,
                hit_breast: anim(ANIM_HIT_BREAST)?,
                hit_link: anim(ANIM_HIT_LINK)?,
                item_give: anim(ANIM_ITEM_GIVE)?,
                item_give_idle: anim(ANIM_ITEM_GIVE_IDLE)?,
            };
            let mut face = Vec::new();
            for s in EYE_TEXTURES.iter().chain(MOUTH_TEXTURES.iter()).chain([NOSE_TEXTURE].iter()) {
                let t = assets.pack.texture(OBJECT, s)?;
                face.push(DrawImage { width: t.width, height: t.height, rgba: t.rgba.clone().into() });
            }
            Ok((skeleton, anims, face))
        })();
        let (skeleton, anims, face) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("Demo_Du: {e:#}");
                actor.kill();
                return oot_game::spawn::Placeholder::init(actor, play);
            }
        };
        let mut this = DemoDu {
            actor,
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), None),
            skeleton,
            eye_tex_index: 0,
            blink_timer: 0,
            mouth_tex_index: 0,
            update_index: 0,
            draw_index: 0,
            unused: 0,
            unk_1a4: 0.0,
            shadow_alpha: 0,
            demo6k_spawned: 0,
            cue_id: 0,
            anims,
            face,
            not_ported_logged: false,
        };
        this.init_cs_gorons_ruby();
        Box::new(this)
    }

    /// `DemoDu_UpdateEyes`: the blink, every 60 to 120 frames.
    fn update_eyes(&mut self, play: &mut PlayState) {
        if decr(&mut self.blink_timer) == 0 {
            self.blink_timer = play.rand.s16_offset(60, 60);
        }
        self.eye_tex_index = self.blink_timer;
        if self.eye_tex_index >= 3 {
            self.eye_tex_index = 0;
        }
    }

    /// `DemoDu_SetEyeTexIndex`.
    fn set_eye_tex_index(&mut self, eye_tex_index: i16) {
        self.eye_tex_index = eye_tex_index;
    }

    /// `DemoDu_SetMouthTexIndex`.
    fn set_mouth_tex_index(&mut self, mouth_tex_index: i16) {
        self.mouth_tex_index = mouth_tex_index;
    }

    /// `DemoDu_UpdateSkelAnime`.
    fn update_skel_anime(&mut self) -> bool {
        self.skel.update()
    }

    /// `DemoDu_UpdateBgCheckInfo`: `Actor_UpdateBgCheckInfo(75, 30, 30, floor and walls)`.
    fn update_bg_check_info(&mut self, play: &PlayState) {
        self.actor.update_bg_check_info(&play.col, 75.0, 30.0, 30.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
    }

    /// `DemoDu_GetCue`: the cue on `cue_channel` while a script runs.
    pub fn get_cue(play: &PlayState, cue_channel: usize) -> Option<CsCmdActorCue> {
        if play.cs_ctx.state != CS_STATE_IDLE { play.cs_ctx.npc_actions[cue_channel] } else { None }
    }

    /// `DemoDu_CheckForCue`: the AG part's (not ported).
    pub fn check_for_cue(play: &PlayState, cue_id: u16, cue_channel: usize) -> bool {
        Self::get_cue(play, cue_channel).is_some_and(|c| c.action == cue_id)
    }

    /// `DemoDu_CheckForNoCue`: the AG part's (not ported).
    pub fn check_for_no_cue(play: &PlayState, cue_id: u16, cue_channel: usize) -> bool {
        Self::get_cue(play, cue_channel).is_some_and(|c| c.action != cue_id)
    }

    /// `DemoDu_SetStartPosRotFromCue`: the credits' (not ported).
    pub fn set_start_pos_rot_from_cue(&mut self, play: &PlayState, cue_channel: usize) {
        if let Some(cue) = Self::get_cue(play, cue_channel) {
            self.actor.world_pos = Vec3::new(cue.start_pos.x as f32, cue.start_pos.y as f32, cue.start_pos.z as f32);
            self.actor.shape_rot.y = cue.rot[1];
            self.actor.world_rot.y = cue.rot[1];
        }
    }

    /// `func_80969DDC`: `animation` from its start (or, reversed, from its end) at speed 1: the
    /// credits' (not ported).
    pub fn func_80969ddc(&mut self, animation: Anim, mode: u8, morph_frames: f32, arg4: bool) {
        let last_frame = animation.last_frame() as i16 as f32;
        let (start_frame, end_frame, play_speed) = if !arg4 { (0.0, last_frame, 1.0) } else { (last_frame, 0.0, -1.0) };
        self.skel.change(animation, play_speed, start_frame, end_frame, mode, morph_frames);
    }

    /// `Animation_Change(&skelAnime, anim, 1, 0, Animation_GetLastFrame(anim), mode, morph)`.
    fn change(&mut self, anim: Anim, mode: u8, morph_frames: f32) {
        let end = anim.last_frame();
        self.skel.change(anim, 1.0, 0.0, end, mode, morph_frames);
    }

    // The Goron's Ruby.

    /// `DemoDu_InitCs_GoronsRuby`: `gDaruniaSkel` without an animation.
    fn init_cs_gorons_ruby(&mut self) {
        // SkelAnime_InitFlex(.., &gDaruniaSkel, NULL, NULL, NULL, 0): in init.
        self.update_index = cs_goronsruby_subscene(0);
    }

    /// `DemoDu_CsPlaySfx_GoronLanding`.
    fn cs_play_sfx_goron_landing(play: &mut PlayState) {
        let pos = play.cur_actor.map(SfxPos::Actor).unwrap_or(SfxPos::Default);
        play.audio.play_sfx_at_pos(pos, NA_SE_EN_GOLON_LAND_BIG);
    }

    /// `DemoDu_CsPlaySfx_DaruniaFalling`: frame 160.
    fn cs_play_sfx_darunia_falling(play: &mut PlayState) {
        if play.cs_ctx.frames == 160 {
            play.audio.play_sfx_centered2(NA_SE_EV_OBJECT_FALL);
        }
    }

    /// `DemoDu_CsPlaySfx_DaruniaHitsLink`: at Link.
    fn cs_play_sfx_darunia_hits_link(play: &mut PlayState) {
        let pos = player_sfx_pos(play);
        play.audio.play_sfx_at_pos(pos, NA_SE_EN_DARUNIA_HIT_LINK);
        play.audio.play_sfx_at_pos(pos, NA_SE_VO_LI_DAMAGE_S_KID);
    }

    /// `DemoDu_CsPlaySfx_HitBreast`: held while it's asked for (`- SFX_FLAG`).
    fn cs_play_sfx_hit_breast(play: &mut PlayState) {
        let pos = play.cur_actor.map(SfxPos::Actor).unwrap_or(SfxPos::Default);
        play.audio.play_sfx_at_pos(pos, NA_SE_EN_DARUNIA_HIT_BREAST - SFX_FLAG);
    }

    /// `DemoDu_CsPlaySfx_LinkEscapeFromGorons`: frame 1400, at Link.
    fn cs_play_sfx_link_escape_from_gorons(play: &mut PlayState) {
        if play.cs_ctx.frames == 1400 {
            let pos = player_sfx_pos(play);
            play.audio.play_sfx_at_pos(pos, NA_SE_VO_LI_FALL_L_KID);
        }
    }

    /// `DemoDu_CsPlaySfx_LinkSurprised`: frame 174, at Link.
    fn cs_play_sfx_link_surprised(play: &mut PlayState) {
        if play.cs_ctx.frames == 174 {
            let pos = player_sfx_pos(play);
            play.audio.play_sfx_at_pos(pos, NA_SE_VO_LI_SURPRISE_KID);
        }
    }

    /// `DemoDu_CsGoronsRuby_UpdateFaceTextures`: by the script's frame.
    fn cs_gorons_ruby_update_face_textures(&mut self, play: &mut PlayState) {
        let frames = play.cs_ctx.frames;
        if frames < 260 {
            self.update_eyes(play);
            self.set_mouth_tex_index(0);
        } else if frames < 335 {
            self.update_eyes(play);
            self.set_mouth_tex_index(3);
        } else if frames < 365 {
            self.set_eye_tex_index(3);
            self.set_mouth_tex_index(1);
        } else if frames < 395 {
            self.set_eye_tex_index(0);
            self.set_mouth_tex_index(3);
        } else if frames < 410 {
            self.update_eyes(play);
            self.set_mouth_tex_index(0);
        } else {
            self.update_eyes(play);
            self.set_mouth_tex_index(3);
        }
    }

    /// `func_8096A630`: the landing: 11 puffs of dust 100 across (`func_80033480`), the sound.
    fn func_8096a630(&self, play: &mut PlayState) {
        let mut pos = self.actor.world_pos;
        pos.y += KREG_5;
        func_80033480(play, pos, KREG_1 + 100.0, KREG_2 + 10, (KREG_3 + 300) as i16, KREG_4 as i16, 0);
        Self::cs_play_sfx_goron_landing(play);
    }

    /// `DemoDu_CsGoronsRuby_SpawnDustWhenHittingLink`: on the slap's frames 31 and 41, five puffs
    /// about Link's left forearm (`func_8002829C`, each shade moved by `Rand_ZeroOne() × 20 - 10`,
    /// scale 200 to 239), and the sounds.
    fn cs_gorons_ruby_spawn_dust_when_hitting_link(&self, play: &mut PlayState) {
        let on_31 = self.skel.on_frame(31.0);
        if !(on_31 || self.skel.on_frame(41.0)) {
            return;
        }
        let pos = player(play).map_or(Vec3::ZERO, |p| p.body_parts_pos[PLAYER_BODYPART_L_FOREARM]);
        let velocity = Vec3::ZERO;
        let accel = Vec3::new(0.0, 0.3, 0.0);
        play.with_ss(|ss| {
            for i in (0..=4).rev() {
                let mut prim_color = [190u8, 150, 110, 255];
                let mut env_color = [120u8, 80, 40, 255];
                let position = if on_31 { DUST_POS_OFFSETS[i + 5] + pos } else { DUST_POS_OFFSETS[i] + pos };
                let color_delta = (ss.rand.zero_one() * 20.0 - 10.0) as i32;
                for c in 0..3 {
                    prim_color[c] = (prim_color[c] as i32 + color_delta) as u8;
                    env_color[c] = (env_color[c] as i32 + color_delta) as u8;
                }
                let scale = (ss.rand.zero_one() * 40.0 + 200.0) as i16;
                ss.func_8002829c(position, velocity, accel, prim_color, env_color, scale, 0);
            }
        });
        Self::cs_play_sfx_darunia_hits_link(play);
    }

    /// `DemoDu_CsGoronsRuby_DaruniaFalling`: from the cue's start to its end by
    /// `Environment_LerpWeight` of the frame.
    fn cs_gorons_ruby_darunia_falling(&mut self, play: &PlayState) {
        if play.cs_ctx.state == CS_STATE_IDLE {
            return;
        }
        let Some(cue) = play.cs_ctx.npc_actions[2] else { return };
        let traveled_percent = oot_game::env::lerp_weight(cue.end_frame, cue.start_frame, play.cs_ctx.frames);
        let start = Vec3::new(cue.start_pos.x as f32, cue.start_pos.y as f32, cue.start_pos.z as f32);
        let end = Vec3::new(cue.end_pos.x as f32, cue.end_pos.y as f32, cue.end_pos.z as f32);
        // LERP(start, end, t): start + (end - start) * t.
        self.actor.world_pos.x = start.x + (end.x - start.x) * traveled_percent;
        self.actor.world_pos.y = start.y + (end.y - start.y) * traveled_percent;
        self.actor.world_pos.z = start.z + (end.z - start.z) * traveled_percent;
    }

    /// `DemoDu_CsGoronsRuby_AdvanceTo01`.
    fn cs_gorons_ruby_advance_to_01(&mut self) {
        self.update_index = cs_goronsruby_subscene(1);
    }

    /// The cue on channel 2 isn't `id` (the `AdvanceTo` checks).
    fn cue_2_is_not(play: &PlayState, id: u16) -> bool {
        Self::get_cue(play, 2).is_some_and(|c| c.action != id)
    }

    /// `DemoDu_CsGoronsRuby_AdvanceTo02`: past cue 1: shown, the stand-up set, falling.
    fn cs_gorons_ruby_advance_to_02(&mut self, play: &PlayState) {
        if Self::cue_2_is_not(play, 1) {
            self.change(self.anims.stand_up_after_falling.clone(), ANIMMODE_ONCE, 0.0);
            self.update_index = cs_goronsruby_subscene(2);
            self.draw_index = 1;
            self.cs_gorons_ruby_darunia_falling(play);
        }
    }

    /// `DemoDu_CsGoronsRuby_AdvanceTo03`: the cue's end frame: landed.
    fn cs_gorons_ruby_advance_to_03(&mut self, play: &mut PlayState) {
        if let Some(cue) = Self::get_cue(play, 2)
            && play.cs_ctx.frames >= cue.end_frame
        {
            self.update_index = cs_goronsruby_subscene(3);
            self.func_8096a630(play);
        }
    }

    /// `DemoDu_CsGoronsRuby_AdvanceTo04`: past cue 2: standing up.
    fn cs_gorons_ruby_advance_to_04(&mut self, play: &PlayState) {
        if Self::cue_2_is_not(play, 2) {
            self.update_index = cs_goronsruby_subscene(4);
        }
    }

    /// `DemoDu_CsGoronsRuby_AdvanceTo05`: stood up: idle.
    fn cs_gorons_ruby_advance_to_05(&mut self, anim_finished: bool) {
        if anim_finished {
            self.change(self.anims.idle.clone(), ANIMMODE_LOOP, 0.0);
            self.update_index = cs_goronsruby_subscene(5);
        }
    }

    /// `DemoDu_CsGoronsRuby_AdvanceTo06`: past cue 3: his chest, a 4-frame morph.
    fn cs_gorons_ruby_advance_to_06(&mut self, play: &PlayState) {
        if Self::cue_2_is_not(play, 3) {
            self.change(self.anims.hit_breast.clone(), ANIMMODE_ONCE, -4.0);
            self.update_index = cs_goronsruby_subscene(6);
        }
    }

    /// `DemoDu_CsGoronsRuby_AdvanceTo07`: idle again.
    fn cs_gorons_ruby_advance_to_07(&mut self, anim_finished: bool) {
        if anim_finished {
            self.change(self.anims.idle.clone(), ANIMMODE_LOOP, 0.0);
            self.update_index = cs_goronsruby_subscene(7);
        }
    }

    /// `DemoDu_CsGoronsRuby_AdvanceTo08`: past cue 4: the slap.
    fn cs_gorons_ruby_advance_to_08(&mut self, play: &PlayState) {
        if Self::cue_2_is_not(play, 4) {
            self.change(self.anims.hit_link.clone(), ANIMMODE_ONCE, 0.0);
            self.update_index = cs_goronsruby_subscene(8);
        }
    }

    /// `DemoDu_CsGoronsRuby_AdvanceTo09`: his chest again.
    fn cs_gorons_ruby_advance_to_09(&mut self, anim_finished: bool) {
        if anim_finished {
            self.change(self.anims.hit_breast.clone(), ANIMMODE_ONCE, 0.0);
            self.update_index = cs_goronsruby_subscene(9);
        }
    }

    /// `DemoDu_CsGoronsRuby_AdvanceTo10`: idle.
    fn cs_gorons_ruby_advance_to_10(&mut self, anim_finished: bool) {
        if anim_finished {
            self.change(self.anims.idle.clone(), ANIMMODE_LOOP, 0.0);
            self.update_index = cs_goronsruby_subscene(10);
        }
    }

    /// `DemoDu_CsGoronsRuby_AdvanceTo11`: past cue 5: the Ruby given.
    fn cs_gorons_ruby_advance_to_11(&mut self, play: &PlayState) {
        if Self::cue_2_is_not(play, 5) {
            self.change(self.anims.item_give.clone(), ANIMMODE_ONCE, 0.0);
            self.update_index = cs_goronsruby_subscene(11);
        }
    }

    /// `DemoDu_CsGoronsRuby_AdvanceTo12`: holding it out (mode 0, looped).
    fn cs_gorons_ruby_advance_to_12(&mut self, anim_finished: bool) {
        if anim_finished {
            self.change(self.anims.item_give_idle.clone(), ANIMMODE_LOOP, 0.0);
            self.update_index = cs_goronsruby_subscene(12);
        }
    }

    /// `DemoDu_CsGoronsRuby_AdvanceTo13`: past cue 6: idle to the end.
    fn cs_gorons_ruby_advance_to_13(&mut self, play: &PlayState) {
        if Self::cue_2_is_not(play, 6) {
            self.change(self.anims.idle.clone(), ANIMMODE_LOOP, 0.0);
            self.update_index = cs_goronsruby_subscene(13);
        }
    }

    /// `sUpdateFuncs[updateIndex]`: the Ruby's `DemoDu_UpdateCs_GR_00` .. `_13`.
    fn run_update(&mut self, play: &mut PlayState) {
        match self.update_index - cs_goronsruby_subscene(0) {
            // DemoDu_UpdateCs_GR_00.
            0 => {
                Self::cs_play_sfx_darunia_falling(play);
                self.cs_gorons_ruby_advance_to_01();
            }
            // DemoDu_UpdateCs_GR_01.
            1 => {
                Self::cs_play_sfx_darunia_falling(play);
                Self::cs_play_sfx_link_surprised(play);
                self.cs_gorons_ruby_advance_to_02(play);
            }
            // DemoDu_UpdateCs_GR_02.
            2 => {
                self.cs_gorons_ruby_darunia_falling(play);
                self.update_bg_check_info(play);
                Self::cs_play_sfx_darunia_falling(play);
                Self::cs_play_sfx_link_surprised(play);
                self.cs_gorons_ruby_advance_to_03(play);
            }
            // DemoDu_UpdateCs_GR_03.
            3 => {
                self.update_bg_check_info(play);
                Self::cs_play_sfx_link_surprised(play);
                self.cs_gorons_ruby_advance_to_04(play);
            }
            // DemoDu_UpdateCs_GR_04.
            4 => {
                self.update_bg_check_info(play);
                let anim_finished = self.update_skel_anime();
                self.cs_gorons_ruby_update_face_textures(play);
                self.cs_gorons_ruby_advance_to_05(anim_finished);
            }
            // DemoDu_UpdateCs_GR_05.
            5 => {
                self.update_bg_check_info(play);
                self.update_skel_anime();
                self.cs_gorons_ruby_update_face_textures(play);
                self.cs_gorons_ruby_advance_to_06(play);
            }
            // DemoDu_UpdateCs_GR_06.
            6 => {
                self.update_bg_check_info(play);
                let anim_finished = self.update_skel_anime();
                Self::cs_play_sfx_hit_breast(play);
                self.cs_gorons_ruby_update_face_textures(play);
                self.cs_gorons_ruby_advance_to_07(anim_finished);
            }
            // DemoDu_UpdateCs_GR_07.
            7 => {
                self.update_bg_check_info(play);
                self.update_skel_anime();
                self.cs_gorons_ruby_update_face_textures(play);
                self.cs_gorons_ruby_advance_to_08(play);
            }
            // DemoDu_UpdateCs_GR_08.
            8 => {
                self.update_bg_check_info(play);
                let anim_finished = self.update_skel_anime();
                self.cs_gorons_ruby_update_face_textures(play);
                self.cs_gorons_ruby_spawn_dust_when_hitting_link(play);
                self.cs_gorons_ruby_advance_to_09(anim_finished);
            }
            // DemoDu_UpdateCs_GR_09.
            9 => {
                self.update_bg_check_info(play);
                let anim_finished = self.update_skel_anime();
                Self::cs_play_sfx_hit_breast(play);
                self.cs_gorons_ruby_update_face_textures(play);
                self.cs_gorons_ruby_advance_to_10(anim_finished);
            }
            // DemoDu_UpdateCs_GR_10.
            10 => {
                self.update_bg_check_info(play);
                self.update_skel_anime();
                self.cs_gorons_ruby_update_face_textures(play);
                self.cs_gorons_ruby_advance_to_11(play);
            }
            // DemoDu_UpdateCs_GR_11.
            11 => {
                self.update_bg_check_info(play);
                let anim_finished = self.update_skel_anime();
                self.cs_gorons_ruby_update_face_textures(play);
                self.cs_gorons_ruby_advance_to_12(anim_finished);
            }
            // DemoDu_UpdateCs_GR_12.
            12 => {
                self.update_bg_check_info(play);
                self.update_skel_anime();
                self.cs_gorons_ruby_update_face_textures(play);
                self.cs_gorons_ruby_advance_to_13(play);
            }
            // DemoDu_UpdateCs_GR_13.
            13 => {
                self.update_bg_check_info(play);
                self.update_skel_anime();
                self.cs_gorons_ruby_update_face_textures(play);
                Self::cs_play_sfx_link_escape_from_gorons(play);
            }
            // DemoDu_UpdateCs_FM_00 .. _06, _AG_00 .. _02, _CR_00 .. _04: their cutscenes are
            // placeholders from init, so never reached.
            _ => {
                if !self.not_ported_logged {
                    log::debug!("Demo_Du: sUpdateFuncs[{}] (the Fire Medallion, after Ganon, the credits) isn't ported", self.update_index);
                    self.not_ported_logged = true;
                }
            }
        }
    }
}

/// Indices into the render state's switches.
mod rs {
    pub const DRAW_INDEX: usize = 0;
    pub const EYE: usize = 1;
    pub const MOUTH: usize = 2;
    /// `shadowAlpha` (`DemoDu_Draw_02`'s env alpha).
    pub const ALPHA: usize = 3;
    pub const SWITCHES: usize = 4;
}

impl ActorImpl for DemoDu {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `DemoDu_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if !(0..UPDATE_FUNCS).contains(&self.update_index) {
            if !self.not_ported_logged {
                log::debug!("Demo_Du: The main mode is wrong!!!!!!!!!!!!!!!!!!!!!!!!!");
                self.not_ported_logged = true;
            }
            return;
        }
        self.run_update(play);
    }

    fn render_state(&self) -> RenderState {
        let mut r = RenderState::of(&self.actor);
        r.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        r.switches = vec![self.draw_index as u32, self.eye_tex_index as u32, self.mouth_tex_index as u32, (self.shadow_alpha as u32) & 0xFF];
        r
    }

    /// `DemoDu_Draw`: by `drawIndex`, `DemoDu_Draw_NoDraw`, `DemoDu_Draw_01` (opaque, env alpha 255)
    /// or `DemoDu_Draw_02` (translucent, env alpha `shadowAlpha`): the eyes on segment 8, the mouth
    /// on 9, the serious nose on 0xA.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if rs.switches.len() < rs::SWITCHES {
            return;
        }
        let draw_index = rs.switches[rs::DRAW_INDEX] as i32;
        if !(0..DRAW_FUNCS).contains(&draw_index) {
            log::debug!("Demo_Du: The drawing mode is wrong!!!!!!!!!!!!!!!!!!!!!!!!!");
            return;
        }
        let Some(joints) = &rs.joints else { return };
        let (bake, xlu, alpha) = match draw_index {
            1 => (BAKE_OPA, false, 255u8),
            2 => (BAKE_XLU, true, rs.switches[rs::ALPHA] as u8),
            _ => return,
        };
        let eyes = EYE_TEXTURES.len();
        let mouths = MOUTH_TEXTURES.len();
        let eye = self.face[(rs.switches[rs::EYE] as usize).min(eyes - 1)].clone();
        let mouth = self.face[eyes + (rs.switches[rs::MOUTH] as usize).min(mouths - 1)].clone();
        let nose = self.face[eyes + mouths].clone();
        let texture_images = vec![SourceImage { source: SEG_EYES, image: eye }, SourceImage { source: SEG_MOUTH, image: mouth }, SourceImage { source: SEG_NOSE, image: nose }];
        let mut sv = SegmentValues::default();
        sv.env[SEG_ALPHA as usize] = Some([0, 0, 0, alpha]);
        let cmd = DrawCmd { mesh: MeshKey::named(keys::bake(bake)), transform: actor_draw_matrix(rs), bones: self.skeleton.pose(joints), params: DrawParams { segments: Some(sv), texture_images, ..Default::default() } };
        if xlu {
            out.xlu.push(cmd);
        } else {
            out.opa.push(cmd);
        }
    }

    /// `DemoDu_Destroy`: `SkelAnime_Free`.
    fn destroy(&mut self, _play: &mut PlayState) {}

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
