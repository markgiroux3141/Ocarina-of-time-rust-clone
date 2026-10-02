//! `En_Md` (`ovl_En_Md/z_en_md.c`): Mido. `params >> 8` is his path (0xFF for none).
//!
//! **In Kokiri Forest** (`SCENE_KOKIRI_FOREST`, room 0's 0x0100 at (1522, 0, 105)) he blocks the way to
//! the Deku Tree until Link wears the Kokiri Sword and the Deku Shield:
//! - he stands 60 from his home towards Link, facing him (`EnMd_BlockPath`), and his collider is
//!   immovable, so Link can't get round him;
//! - he talks (`Npc_UpdateTalking`): 0x102F the first time (setting `EVENTCHKINF_MIDO_DENIED_DEKU_TREE_ACCESS` and
//!   `INFTABLE_0C` as it closes), then 0x1030; with both worn 0x1033, and when it closes he
//!   sets `EVENTCHKINF_04` and walks along path 1 to its last point (`EnMd_Walk`), where
//!   he stays (`EnMd_Watch`);
//! - with `EVENTCHKINF_04` set he starts at the path's end (`EnMd_SetMovedPos`, `EnMd_Idle`)
//!   and says 0x1034; with the Kokiri Emerald 0x1045, and he goes once he's walked away
//!   (`EVENTCHKINF_1C`);
//! - he fades out beyond 400 of Link (`EnMd_UpdateAlphaByDistance`, `Actor_UpdateAlphaByDistance`).
//!
//! **In his house** (`SCENE_MIDOS_HOUSE`) he's there once Link has Zelda's letter (or has
//! said goodbye to Saria: `EVENTCHKINF_1C`), and says 0x1028 or 0x1046. **In the Lost Woods**
//! (`SCENE_LOST_WOODS`) he blocks the way to the Sacred Forest Meadow until Link plays Saria's Song
//! (`Message_StartOcarina(OCARINA_ACTION_CHECK_SARIA)`, `EnMd_ListenToOcarina`): there's no ocarina, so
//! `PLAYER_STATE2_24` never comes up, and the check is logged when it would start.
//!
//! Drawn as `EnMd_Draw` does: the skeleton with his eyes on segment 8, opaque at full alpha
//! (`func_80034BA0`) and translucent while fading (`func_80034CC4`), with `EnMd_OverrideLimbDraw`'s
//! head and torso turns and sway, from meshes baked per eye and pass
//! (docs/adr/0012-actor-bakes.md).
//!
//! Not ported: his fairy (`En_Elf`, `FAIRY_KOKIRI`: a placeholder), the circle shadow's alpha.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey, SegmentValues};
use eng_math::{binang_to_rad, cos_s, sin_s, smooth_step_to_s};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_UPDATE_DURING_OCARINA, Actor, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile};
use oot_game::collision_check::*;
use oot_game::item::{EQUIP_TYPE_SHIELD, EQUIP_TYPE_SWORD, EQUIP_VALUE_SHIELD_DEKU, EQUIP_VALUE_SWORD_KOKIRI};
use oot_game::message::*;
use oot_game::npc::{NpcTrack, track_point, actor_update_alpha_by_distance, actor_update_fidget_tables, get_tracking_preset_max_player_yaw};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::save::{EVENTCHKINF_04, EVENTCHKINF_OBTAINED_ZELDAS_LETTER, QUEST_KOKIRI_EMERALD};
use oot_game::skelanime_std::*;

/// `ACTOR_EN_MD` (`actor_table.h`).
pub const ACTOR_EN_MD: i16 = 0x016D;
/// `ACTOR_EN_ELF`: his fairy (`FAIRY_KOKIRI`, 3).
const ACTOR_EN_ELF: i16 = 0x0018;
const FAIRY_KOKIRI: i16 = 3;

/// `En_Md_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_MD, name: "En_Md", category: ACTORCAT_NPC, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA, object: "object_md" };

// `SCENE_*` (`scene_table.h`).
pub const SCENE_MIDOS_HOUSE: u16 = 0x28;
pub const SCENE_KOKIRI_FOREST: u16 = 0x55;
pub const SCENE_LOST_WOODS: u16 = 0x5B;

// `EVENTCHKINF_*` (`save.h`), besides `EVENTCHKINF_04` and `EVENTCHKINF_OBTAINED_ZELDAS_LETTER` (`oot_game::save`).
/// Mido's first talk read (0x102F).
pub const EVENTCHKINF_MIDO_DENIED_DEKU_TREE_ACCESS: u16 = 0x02;
/// Mido has let Link into the Sacred Forest Meadow (the Lost Woods).
pub const EVENTCHKINF_0A: u16 = 0x0A;
/// Mido's talk in his house read (0x1028).
pub const EVENTCHKINF_0F: u16 = 0x0F;
/// Link said goodbye to Saria and Mido at the bridge (`EnMd_Walk`).
pub const EVENTCHKINF_1C: u16 = 0x1C;
/// The Forest Medallion's cutscene.
pub const EVENTCHKINF_48: u16 = 0x48;

// `INFTABLE_*` (`save.h`): the bit numbers.
/// Mido's first talk read (0x102F): from then on 0x1030.
pub const INFTABLE_0C: u16 = 0x0C;
const INFTABLE_15: u16 = 0x15;
const INFTABLE_19: u16 = 0x19;

/// `MidoLimb`: the limbs `EnMd_OverrideLimbDraw` and `EnMd_PostLimbDraw` name (1-based, as
/// `limbIndex`).
pub const MIDO_LIMB_TORSO: usize = 9;
pub const MIDO_LIMB_LEFT_UPPER_ARM: usize = 10;
pub const MIDO_LIMB_RIGHT_UPPER_ARM: usize = 13;
pub const MIDO_LIMB_HEAD: usize = 16;
/// `MIDO_LIMB_MAX`: the joint table's size (the skeleton's 16 limbs and the root).
pub const MIDO_LIMB_MAX: usize = 17;

/// `sCylinderInit`.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_type: ELEM_MATERIAL_UNK0,
        toucher: ColliderElementDamageInfoAT { dmg_flags: 0, effect: 0, damage: 0 },
        bumper: ColliderElementDamageInfoACInit { dmg_flags: 0, effect: 0, defense: 0 },
        toucher_flags: ATELEM_NONE,
        bumper_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 36, height: 46, y_shift: 0, pos: [0; 3] },
};

/// `sColChkInfoInit`.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit2 = CollisionCheckInfoInit2 { health: 0, cyl_radius: 0, cyl_height: 0, cyl_y_shift: 0, mass: MASS_IMMOVABLE };

/// `gMidoIdleAnim`: his standing animation, which the actions compare against.
const IDLE_ANIM: &str = "gMidoIdleAnim";

/// `sAnimationInfo` (`EnMdAnimIndex` 0..13): animation, speed, start, end, mode, morph frames.
const ANIMATION_INFO: [(&str, f32, f32, f32, u8, f32); 14] = [
    (IDLE_ANIM, 0.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    (IDLE_ANIM, 0.0, 0.0, -1.0, ANIMMODE_LOOP, -10.0),
    ("gMidoIdleToHaltAnim", 1.0, 0.0, -1.0, ANIMMODE_ONCE, -1.0),
    ("gMidoHaltAnim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -1.0),
    ("gMidoHaltToCuriousAnim", 1.0, 0.0, -1.0, ANIMMODE_ONCE, -1.0),
    ("gMidoCuriousAnim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -1.0),
    ("gMidoAnnoyedAnim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -1.0),
    ("gMidoIdleToWalkAnim", 1.0, 0.0, -1.0, ANIMMODE_ONCE, -1.0),
    ("gMidoWalkAnim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -1.0),
    ("gMidoIdleToSurpriseAnim", 1.0, 0.0, -1.0, ANIMMODE_ONCE, -1.0),
    (IDLE_ANIM, 0.0, 0.0, -1.0, ANIMMODE_LOOP, -8.0),
    ("gMidoCuriousToAnnoyedAnim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -1.0),
    ("gMidoAnnoyedToHaltAnim", 1.0, 0.0, -1.0, ANIMMODE_ONCE, -1.0),
    ("gMidoIdleToAnnoyedAnim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -1.0),
];

// `EnMdAnimIndex` indices used by name.
pub const ENMD_ANIM_INDEX_IDLE_DEFAULT: usize = 0;
pub const ENMD_ANIM_INDEX_IDLE_TO_HALT: usize = 2;
pub const ENMD_ANIM_INDEX_HALT: usize = 3;
pub const ENMD_ANIM_INDEX_HALT_TO_CURIOUS: usize = 4;
pub const ENMD_ANIM_INDEX_CURIOUS: usize = 5;
pub const ENMD_ANIM_INDEX_ANNOYED: usize = 6;
pub const ENMD_ANIM_INDEX_IDLE_TO_WALK: usize = 7;
pub const ENMD_ANIM_INDEX_WALK: usize = 8;
pub const ENMD_ANIM_INDEX_IDLE_TO_SURPISE: usize = 9;
pub const ENMD_ANIM_INDEX_IDLE: usize = 10;
pub const ENMD_ANIM_INDEX_CURIOUS_TO_ANNOYED: usize = 11;
pub const ENMD_ANIM_INDEX_ANNOYED_TO_HALT: usize = 12;
pub const ENMD_ANIM_INDEX_IDLE_TO_ANNOYED: usize = 13;

/// `sEyeTextures` (`EnMd_Draw`), on segment 8.
const EYES: [&str; 3] = ["gMidoEyeOpenTex", "gMidoEyeHalfTex", "gMidoEyeClosedTex"];
const SEG_EYES: u8 = 0x08;
/// `gSPSegment(0x0C, ...)`: `func_80034B28`'s empty list (opaque) or `func_80034B54`'s render
/// mode (translucent).
const SEG_RENDER_MODE: u8 = 0x0C;
/// The `gDPSetEnvColor(0, 0, 0, alpha)` of `func_80034BA0` / `func_80034CC4`, as a dynamic
/// colour run before the skeleton.
const SEG_ALPHA: u8 = 0x0E;
/// `func_80034B54`'s `gDPSetRenderMode(G_RM_FOG_SHADE_A, AA_EN | Z_CMP | Z_UPD | IM_RD |
/// CLR_ON_CVG | CVG_DST_WRAP | ZMODE_XLU | FORCE_BL | GBL_c2(G_BL_CLR_IN, G_BL_A_IN,
/// G_BL_CLR_MEM, G_BL_1MA))`.
const XLU_RENDER_MODE: (u32, u32) = (0xE200_001C, 0xC810_49F8);

/// The bake of an eye (0..2) in a pass.
pub fn bake_name(eye: usize, xlu: bool) -> String {
    format!("En_Md/{}_eye{eye}", if xlu { "xlu" } else { "opa" })
}

/// The meshes `EnMd_Draw` draws: each eye, opaque and translucent.
pub fn bakes() -> Vec<MeshBake> {
    let mut v = Vec::new();
    for (eye, tex) in EYES.iter().enumerate() {
        for xlu in [false, true] {
            v.push(MeshBake {
                name: bake_name(eye, xlu),
                object: "object_md".into(),
                segments: vec![
                    (SEG_EYES, BakeSegment::Texture { file: "object_md".into(), symbol: (*tex).into() }),
                    (SEG_RENDER_MODE, BakeSegment::Commands(if xlu { vec![XLU_RENDER_MODE] } else { Vec::new() })),
                    (SEG_ALPHA, BakeSegment::DynamicColor { env: true, prim: false }),
                ],
                prelude: vec![SEG_ALPHA],
                body: BakeBody::Skeleton { file: "object_md".into(), symbol: "gMidoSkel".into(), limbs: Vec::new() },
            });
        }
    }
    v
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `EnMd_Idle`: standing where he moved to (or in his house).
    Moved,
    /// `EnMd_Watch`: standing at the end of his path, after walking there.
    Arrived,
    /// `EnMd_BlockPath`: blocking the way.
    Blocking,
    /// `EnMd_ListenToOcarina`: waiting for Link's ocarina (the Lost Woods).
    Ocarina,
    /// `EnMd_Walk`: walking along his path.
    Walking,
}

pub struct EnMd {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub action: Action,
    pub collider: ColliderCylinder,
    /// `interactInfo`: the talk state and the head and torso tracking.
    pub interact_info: NpcTrack,
    /// `messageEntry`: how many of the conversation's boxes have gone by; `messageState` the last
    /// `Message_GetState`.
    pub message_entry: u8,
    pub message_state: u8,
    /// `animSequenceEntry`: the animation sequence's step; `animSequence` the sequence (`EnMd_UpdateAnimSequence`).
    pub anim_sequence_entry: u8,
    pub anim_sequence: u8,
    pub blink_timer: i16,
    pub eye_idx: i16,
    pub alpha: i16,
    pub waypoint: i16,
    /// `fidgetTableY`, `fidgetTableZ`: the idle sway (`Actor_UpdateFidgetTables`), by limb.
    pub fidget_table_y: [i16; MIDO_LIMB_MAX],
    pub fidget_table_z: [i16; MIDO_LIMB_MAX],
    animations: Vec<AnimationInfo>,
}

impl EnMd {
    /// `EnMd_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = assets.skeleton("object_md", "gMidoSkel").and_then(|s| {
            let animations = ANIMATION_INFO
                .iter()
                .map(|&(name, play_speed, start_frame, frame_count, mode, morph_frames)| {
                    assets.animation("object_md", name).map(|animation| AnimationInfo { animation, play_speed, start_frame, frame_count, mode, morph_frames })
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            Ok((s, animations))
        });
        let (skeleton, animations) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Md: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let mut m = EnMd {
            actor,
            // SkelAnime_InitFlex(..., MIDO_LIMB_MAX): the 16 limbs and the root.
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), None),
            skeleton,
            action: Action::Moved,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            interact_info: NpcTrack::default(),
            message_entry: 0,
            message_state: 0,
            anim_sequence_entry: 0,
            anim_sequence: 0,
            blink_timer: 0,
            eye_idx: 0,
            alpha: 0,
            waypoint: 0,
            fidget_table_y: [0; MIDO_LIMB_MAX],
            fidget_table_z: [0; MIDO_LIMB_MAX],
            animations,
        };
        // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 24).
        m.actor.shape_y_offset = 0.0;
        m.actor.col_chk_info.set_info2(None, &COL_CHK_INFO_INIT);
        if !m.should_spawn(play) {
            m.actor.kill();
            return Box::new(m);
        }
        m.change_anim(ENMD_ANIM_INDEX_IDLE_DEFAULT);
        m.actor.scale = Vec3::splat(0.01);
        m.actor.target_mode = 6;
        m.alpha = 255;
        let pos = m.actor.world_pos;
        if let Err(e) = play.actor_spawn_as_child(&mut m.actor, ACTOR_EN_ELF, pos, [0; 3], FAIRY_KOKIRI) {
            log::debug!("En_Md's fairy: {e:?}");
        }
        let s = &play.save;
        let spot04 = play.scene_id == SCENE_KOKIRI_FOREST;
        if (spot04 && !s.get_event_chk_inf(EVENTCHKINF_04))
            || (spot04 && s.get_event_chk_inf(EVENTCHKINF_04) && s.check_quest_item(QUEST_KOKIRI_EMERALD))
            || (play.scene_id == SCENE_LOST_WOODS && !s.get_event_chk_inf(EVENTCHKINF_0A))
        {
            m.actor.home_pos = m.actor.world_pos;
            m.action = Action::Blocking;
            return Box::new(m);
        }
        if play.scene_id != SCENE_MIDOS_HOUSE {
            m.set_moved_pos(play);
        }
        m.action = Action::Moved;
        Box::new(m)
    }

    /// `EnMd_ShouldSpawn`.
    fn should_spawn(&self, play: &PlayState) -> bool {
        let s = &play.save;
        let (goodbye, letter) = (s.get_event_chk_inf(EVENTCHKINF_1C), s.get_event_chk_inf(EVENTCHKINF_OBTAINED_ZELDAS_LETTER));
        if play.scene_id == SCENE_KOKIRI_FOREST && !goodbye && !letter {
            return true;
        }
        if play.scene_id == SCENE_MIDOS_HOUSE && (goodbye || letter) && !s.adult {
            return true;
        }
        play.scene_id == SCENE_LOST_WOODS
    }

    /// `Animation_ChangeByInfo(&skelAnime, sAnimationInfo, i)`.
    fn change_anim(&mut self, i: usize) {
        if let Some(info) = self.animations.get(i) {
            self.skel.change_by_info(info);
        }
    }

    fn anim_is_idle(&self) -> bool {
        self.skel.is(IDLE_ANIM)
    }

    /// `Animation_OnFrame(&skelAnime, skelAnime.endFrame)`.
    fn on_end_frame(&self) -> bool {
        self.skel.on_frame(self.skel.end_frame)
    }

    /// `EnMd_ReverseAnimation`: the animation played backwards, from its end.
    fn reverse_animation(&mut self) {
        let s = &mut self.skel;
        let start_frame = s.start_frame;
        s.start_frame = s.end_frame;
        s.cur_frame = s.end_frame;
        s.end_frame = start_frame;
        s.play_speed = -1.0;
    }

    /// One of `EnMd_UpdateAnimSequence_IdleToHalt` .. `EnMd_UpdateAnimSequence_StopWalking`: at step 0 the first animation (backwards if
    /// `reversed`), then at each end the next of `then` (the C's cases fall through only where
    /// `falls` says).
    fn sequence(&mut self, first: usize, reversed: bool, then: &[usize], falls: bool) {
        if self.anim_sequence_entry == 0 {
            self.change_anim(first);
            if reversed {
                self.reverse_animation();
            }
            self.anim_sequence_entry += 1;
        }
        // Case 1 (fallen into from case 0).
        if self.anim_sequence_entry == 1 {
            if self.on_end_frame() {
                self.change_anim(then[0]);
                self.anim_sequence_entry += 1;
            } else {
                return;
            }
            if !falls {
                return;
            }
        } else if !(falls && self.anim_sequence_entry == 2) {
            return;
        }
        // Case 2 of EnMd_UpdateAnimSequence_WalkAway.
        if then.len() > 1 && self.anim_sequence_entry == 2 && self.on_end_frame() {
            self.change_anim(then[1]);
            self.anim_sequence_entry += 1;
        }
    }

    /// `EnMd_SetAnimSequence`.
    fn set_anim_sequence(&mut self, arg1: u8) {
        self.anim_sequence = arg1;
        self.anim_sequence_entry = 0;
    }

    /// `EnMd_UpdateAnimSequence`: this frame's step of the current animation sequence.
    fn update_anim_sequence(&mut self) {
        match self.anim_sequence {
            // EnMd_UpdateAnimSequence_IdleToHalt
            1 => self.sequence(ENMD_ANIM_INDEX_IDLE_TO_HALT, false, &[ENMD_ANIM_INDEX_HALT], false),
            // EnMd_UpdateAnimSequence_HaltToCurious
            2 => self.sequence(ENMD_ANIM_INDEX_HALT_TO_CURIOUS, false, &[ENMD_ANIM_INDEX_CURIOUS], false),
            // EnMd_UpdateAnimSequence_WalkAway: case 1 falls into case 2 when it changes.
            3 => self.sequence(ENMD_ANIM_INDEX_IDLE_TO_HALT, true, &[ENMD_ANIM_INDEX_IDLE_TO_WALK, ENMD_ANIM_INDEX_WALK], true),
            // EnMd_UpdateAnimSequence_TwitchIdle_Unused
            4 => self.sequence(ENMD_ANIM_INDEX_IDLE_TO_WALK, false, &[ENMD_ANIM_INDEX_IDLE], false),
            // EnMd_UpdateAnimSequence_HaltToIdle
            5 => self.sequence(ENMD_ANIM_INDEX_IDLE_TO_HALT, true, &[ENMD_ANIM_INDEX_IDLE], false),
            // EnMd_UpdateAnimSequence_SurpriseToAnnoyed
            6 => self.sequence(ENMD_ANIM_INDEX_IDLE_TO_SURPISE, false, &[ENMD_ANIM_INDEX_ANNOYED], false),
            // EnMd_UpdateAnimSequence_SurpriseToIdle
            7 => self.sequence(ENMD_ANIM_INDEX_IDLE_TO_SURPISE, true, &[ENMD_ANIM_INDEX_IDLE], false),
            // EnMd_UpdateAnimSequence_CuriousToAnnoyed
            8 => self.sequence(ENMD_ANIM_INDEX_CURIOUS_TO_ANNOYED, false, &[ENMD_ANIM_INDEX_ANNOYED], false),
            // EnMd_UpdateAnimSequence_AnnoyedToHalt
            9 => self.sequence(ENMD_ANIM_INDEX_ANNOYED_TO_HALT, false, &[ENMD_ANIM_INDEX_HALT], false),
            // EnMd_UpdateAnimSequence_IdleToAnnoyed
            10 => self.sequence(ENMD_ANIM_INDEX_IDLE_TO_ANNOYED, false, &[ENMD_ANIM_INDEX_ANNOYED], false),
            // EnMd_UpdateAnimSequence_StopWalking
            11 => self.sequence(ENMD_ANIM_INDEX_IDLE_TO_WALK, true, &[ENMD_ANIM_INDEX_IDLE], false),
            _ => {}
        }
    }

    /// `EnMd_UpdateAnimSequence_WithTalking`: while talking, the gestures for the text's boxes (`messageEntry`); not
    /// talking, back to standing.
    fn update_anim_sequence_with_talking(&mut self) {
        if self.interact_info.talk_state != 0 {
            let b = self.message_entry;
            let set = |m: &mut Self, at: u8, seq: u8| {
                if b == at && m.anim_sequence != seq {
                    m.set_anim_sequence(seq);
                }
            };
            match self.actor.text_id {
                0x102F => {
                    set(self, 0, 1);
                    set(self, 2, 2);
                    set(self, 5, 8);
                    set(self, 11, 9);
                }
                0x1033 => {
                    set(self, 0, 1);
                    set(self, 1, 2);
                    set(self, 5, 10);
                    set(self, 7, 9);
                }
                0x1030 | 0x1034 | 0x1045 => set(self, 0, 1),
                0x1046 => set(self, 0, 6),
                _ => {}
            }
        } else if !self.anim_is_idle() {
            self.change_anim(ENMD_ANIM_INDEX_IDLE);
            self.set_anim_sequence(0);
        }
        self.update_anim_sequence();
    }

    /// `EnMd_UpdateEyes`.
    fn update_eyes(&mut self, play: &mut PlayState) {
        // DECR(blinkTimer) == 0.
        if self.blink_timer != 0 {
            self.blink_timer -= 1;
            if self.blink_timer != 0 {
                return;
            }
        }
        self.eye_idx += 1;
        if self.eye_idx > 2 {
            self.blink_timer = play.rand.s16_offset(30, 30);
            self.eye_idx = 0;
        }
    }

    /// `EnMd_UpdateAlphaByDistance`: fading in within 400 of Link (100 in Kokiri Forest with the Kokiri
    /// Emerald before the goodbye), always seen in his house.
    fn update_alpha_by_distance(&mut self, play: &PlayState) {
        if play.scene_id != SCENE_MIDOS_HOUSE {
            let s = &play.save;
            let temp = if s.check_quest_item(QUEST_KOKIRI_EMERALD) && !s.get_event_chk_inf(EVENTCHKINF_1C) && play.scene_id == SCENE_KOKIRI_FOREST { 100.0 } else { 400.0 };
            let pp = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(self.actor.world_pos);
            self.alpha = actor_update_alpha_by_distance(&mut self.actor, pp, self.alpha, temp);
        } else {
            self.alpha = 255;
        }
        // shape.shadowAlpha = alpha.
    }

    /// `EnMd_UpdateTalking`: the head and torso tracking, and talking (`Npc_UpdateTalking`).
    fn update_talking(&mut self, play: &mut PlayState) {
        let (mut temp, mut temp2);
        if self.actor.xz_dist_to_player < 170.0 {
            let yaw_diff = (self.actor.yaw_towards_player as f32 - self.actor.shape_rot.y as f32) as i32 as i16;
            let abs_yaw_diff = (yaw_diff as i32).abs() as i16;
            temp = if abs_yaw_diff <= get_tracking_preset_max_player_yaw(2) { 2 } else { 1 };
            temp2 = true;
        } else {
            temp = 1;
            temp2 = false;
        }
        if self.interact_info.talk_state != 0 {
            temp = 4;
        }
        if self.action == Action::Walking {
            temp = 1;
            temp2 = false;
        }
        if self.action == Action::Arrived {
            temp = 4;
            temp2 = true;
        }
        if play.cs_ctx.state != oot_game::cutscene::CS_STATE_IDLE {
            // In a cutscene: the view's eye, 40 up, the whole head (the debug camera isn't
            // ported).
            self.interact_info.target = play.view.eye;
            self.interact_info.eye_height = 40.0;
            temp = 2;
        } else {
            // Link's position, and a child's eyes.
            self.interact_info.target = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(self.actor.world_pos);
            self.interact_info.eye_height = if !play.save.adult { 0.0 } else { -18.0 };
        }
        let mut t = self.interact_info;
        track_point(play, &mut self.actor, &mut t, 2, temp);
        self.interact_info = t;
        if self.action != Action::Ocarina && temp2 {
            let range = self.collider.dim.radius as f32 + 30.0;
            let mut talk_state = self.interact_info.talk_state;
            // Both callbacks read and write Mido's box count (messageEntry, messageState).
            let counts = std::cell::Cell::new((self.message_entry, self.message_state));
            oot_game::npc::talk_update(
                play,
                &mut self.actor,
                &mut talk_state,
                range,
                |play, _| {
                    let (mut a, mut b) = counts.get();
                    let text = en_md_get_text(play, &mut a, &mut b);
                    counts.set((a, b));
                    text
                },
                |play, actor| {
                    let (mut a, mut b) = counts.get();
                    let r = update_talk_state(play, actor, &mut a, &mut b);
                    counts.set((a, b));
                    r
                },
            );
            (self.message_entry, self.message_state) = counts.get();
            self.interact_info.talk_state = talk_state;
        }
    }

    /// `EnMd_FollowPath`: turns towards the next point of his path; true when he's within 10 of
    /// it (the next point is then the one after, back to 0 after the last).
    fn follow_path(&mut self, play: &PlayState) -> bool {
        if (self.actor.params as u16 & 0xFF00) == 0xFF00 {
            return false;
        }
        let Some(path) = play.setup_path_list().get(((self.actor.params as u16 & 0xFF00) >> 8) as usize) else { return false };
        let p = path.point(self.waypoint as usize);
        let dx = p.x - self.actor.world_pos.x;
        let dz = p.z - self.actor.world_pos.z;
        // Math_FAtan2F(pathDiffX, pathDiffZ) * (65536.0f / (2 * M_PI)), a double product
        // truncated to s16.
        let yaw = (oot_game::camera::f_atan2f(dx, dz) as f64 * (65536.0 / (2.0 * std::f64::consts::PI))) as i32 as i16;
        smooth_step_to_s(&mut self.actor.world_rot.y, yaw, 4, 4000, 1);
        if dx * dx + dz * dz < 100.0 {
            self.waypoint += 1;
            if self.waypoint as usize >= path.points.len() {
                self.waypoint = 0;
            }
            return true;
        }
        false
    }

    /// `EnMd_SetMovedPos`: at his path's last point.
    fn set_moved_pos(&mut self, play: &PlayState) -> bool {
        if (self.actor.params as u16 & 0xFF00) == 0xFF00 {
            return false;
        }
        let Some(path) = play.setup_path_list().get(((self.actor.params as u16 & 0xFF00) >> 8) as usize) else { return false };
        self.actor.world_pos = path.point(path.points.len() - 1);
        true
    }

    fn sway(&mut self, play: &PlayState) {
        actor_update_fidget_tables(play, &mut self.fidget_table_y, &mut self.fidget_table_z, MIDO_LIMB_MAX);
    }

    /// `EnMd_Idle`.
    fn idle(&mut self, play: &PlayState) {
        if self.anim_is_idle() {
            self.sway(play);
        } else if self.interact_info.talk_state == 0 && self.anim_sequence != 7 {
            self.set_anim_sequence(7);
        }
        self.update_anim_sequence_with_talking();
    }

    /// `EnMd_Watch`.
    fn watch(&mut self, play: &PlayState) {
        if self.anim_is_idle() {
            self.sway(play);
        }
        self.update_anim_sequence();
    }

    /// `EnMd_BlockPath`: between Link and the way, 60 from home; when his text says so, out of
    /// the way along his path.
    fn block_path(&mut self, play: &mut PlayState) {
        self.update_anim_sequence_with_talking();
        let pp = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(self.actor.world_pos);
        if self.interact_info.talk_state == 0 {
            self.actor.world_rot.y = self.actor.yaw_towards_player;
            self.actor.shape_rot.y = self.actor.yaw_towards_player;
            let yaw = oot_game::target::yaw_to(self.actor.home_pos, pp);
            self.actor.world_pos.x = self.actor.home_pos.x + 60.0 * sin_s(yaw);
            self.actor.world_pos.z = self.actor.home_pos.z + 60.0 * cos_s(yaw);
            let temp = (self.actor.yaw_towards_player as f32 - yaw as f32).abs() * 0.001 * 3.0;
            self.skel.play_speed = if temp < 1.0 {
                1.0
            } else if temp > 3.0 {
                3.0
            } else {
                temp
            };
        }
        if self.interact_info.talk_state == 2 {
            let s = &play.save;
            if s.check_quest_item(QUEST_KOKIRI_EMERALD) && !s.get_event_chk_inf(EVENTCHKINF_1C) && play.scene_id == SCENE_KOKIRI_FOREST {
                play.msg_ctx.msg_mode = MSGMODE_PAUSED;
            }
            if play.scene_id == SCENE_KOKIRI_FOREST {
                play.save.set_event_chk_inf(EVENTCHKINF_04);
            }
            if play.scene_id == SCENE_LOST_WOODS {
                play.save.set_event_chk_inf(EVENTCHKINF_0A);
            }
            self.set_anim_sequence(3);
            self.update_anim_sequence();
            self.waypoint = 1;
            self.interact_info.talk_state = 0;
            self.action = Action::Walking;
            self.actor.speed_xz = 1.5;
            return;
        }
        if self.anim_is_idle() {
            self.sway(play);
        }
        if self.interact_info.talk_state == 0 && play.scene_id == SCENE_LOST_WOODS {
            let state2 = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map(|p| p.state_flags2()).unwrap_or(0);
            if state2 & PLAYER_STATE2_24 != 0 {
                // player->stateFlags2 |= PLAYER_STATE2_25; player->unk_6A8 = this;
                // Message_StartOcarina(play, OCARINA_ACTION_CHECK_SARIA): the ocarina isn't ported.
                log::info!("En_Md: Saria's Song's check (Message_StartOcarina) isn't ported");
                self.change_player_state2(play, PLAYER_STATE2_25);
                self.action = Action::Ocarina;
                return;
            }
            if self.actor.xz_dist_to_player < 30.0 + self.collider.dim.radius as f32 {
                self.change_player_state2(play, PLAYER_STATE2_23);
            }
        }
    }

    fn change_player_state2(&self, play: &mut PlayState, set: u32) {
        if let Some(p) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
            p.change_state_flags2(set, 0);
        }
    }

    /// `EnMd_ListenToOcarina`: waiting for the ocarina's result (`msgCtx.ocarinaMode`).
    fn listen_to_ocarina(&mut self, play: &mut PlayState) {
        /// `OCARINA_MODE_03` (Saria's Song played), `OCARINA_MODE_04` (done).
        const OCARINA_MODE_03: u16 = 3;
        const OCARINA_MODE_04: u16 = 4;
        if play.msg_ctx.ocarina_mode >= OCARINA_MODE_04 {
            self.action = Action::Blocking;
            play.msg_ctx.ocarina_mode = OCARINA_MODE_04;
        } else if play.msg_ctx.ocarina_mode == OCARINA_MODE_03 {
            play.audio.play_sfx_centered(oot_game::audio::sfx::NA_SE_SY_CORRECT_CHIME);
            self.actor.text_id = 0x1067;
            let range = self.collider.dim.radius as f32 + 30.0;
            let a = self.actor.clone();
            oot_game::npc::offer_talk(play, &a, range);
            self.action = Action::Blocking;
            play.msg_ctx.ocarina_mode = OCARINA_MODE_04;
        } else {
            self.change_player_state2(play, PLAYER_STATE2_23);
        }
    }

    /// `EnMd_Walk`: along his path to its end; there he stops and stands (in Kokiri Forest
    /// with the Kokiri Emerald, he goes instead: the goodbye).
    fn walk(&mut self, play: &mut PlayState) {
        self.sway(play);
        self.update_anim_sequence();
        if !self.follow_path(play) || self.waypoint != 0 {
            self.actor.shape_rot = self.actor.world_rot;
            return;
        }
        let s = &play.save;
        if s.check_quest_item(QUEST_KOKIRI_EMERALD) && !s.get_event_chk_inf(EVENTCHKINF_1C) && play.scene_id == SCENE_KOKIRI_FOREST {
            play.with_msg(|m, f| m.close_textbox(f.audio));
            play.save.set_event_chk_inf(EVENTCHKINF_1C);
            self.actor.kill();
            return;
        }
        self.set_anim_sequence(11);
        self.skel.play_speed = 0.0;
        self.actor.speed_xz = 0.0;
        self.actor.home_pos = self.actor.world_pos;
        self.action = Action::Arrived;
    }

    /// The limb matrices `EnMd_Draw` draws with: the pose, and `EnMd_OverrideLimbDraw`'s head
    /// and torso turns and the sway of the torso and upper arms.
    fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], head: [i16; 2], torso: [i16; 2], sway: &[(usize, i16, i16)]) -> Vec<Mat4> {
        let r = binang_to_rad;
        skeleton.pose_override(joints, |limb, _pos, rot| {
            let mut pre = Mat4::IDENTITY;
            if limb == MIDO_LIMB_HEAD {
                // vec = interactInfo.headRot: RotateX(vec.y), RotateZ(vec.x) about (1200, 0, 0).
                pre =
                    Mat4::from_translation(Vec3::new(1200.0, 0.0, 0.0)) * Mat4::from_rotation_x(r(head[1])) * Mat4::from_rotation_z(r(head[0])) * Mat4::from_translation(Vec3::new(-1200.0, 0.0, 0.0));
            }
            if limb == MIDO_LIMB_TORSO {
                // vec = interactInfo.unk_0E: RotateX(vec.x), RotateY(vec.y).
                pre = Mat4::from_rotation_x(r(torso[0])) * Mat4::from_rotation_y(r(torso[1]));
            }
            if let Some(&(_, a, b)) = sway.iter().find(|s| s.0 == limb) {
                rot[1] = (rot[1] as f32 + sin_s(a) * 200.0) as i32 as i16;
                rot[2] = (rot[2] as f32 + cos_s(b) * 200.0) as i32 as i16;
            }
            pre
        })
    }

    fn sway_of(a: &[i16; MIDO_LIMB_MAX], b: &[i16; MIDO_LIMB_MAX]) -> [(usize, i16, i16); 3] {
        [MIDO_LIMB_TORSO, MIDO_LIMB_LEFT_UPPER_ARM, MIDO_LIMB_RIGHT_UPPER_ARM].map(|l| (l, a[l], b[l]))
    }
}

/// `PLAYER_STATE2_23` (the ocarina prompt nearby), `_24` (the ocarina out), `_25`.
const PLAYER_STATE2_23: u32 = 1 << 23;
const PLAYER_STATE2_24: u32 = 1 << 24;
const PLAYER_STATE2_25: u32 = 1 << 25;

/// `EnMd_GetTextId`: his text for the scene and the story (`EnMd_GetTextIdKokiriForest`,
/// `EnMd_GetTextIdMidosHouse`, `EnMd_GetTextIdLostWoods`), resetting the box count.
fn en_md_get_text(play: &PlayState, message_entry: &mut u8, message_state: &mut u8) -> u16 {
    let s = &play.save;
    match play.scene_id {
        SCENE_KOKIRI_FOREST => {
            // MaskReaction_GetTextId(play, 0x11): 0 without a mask.
            *message_entry = 0;
            *message_state = TEXT_STATE_NONE;
            if s.check_quest_item(QUEST_KOKIRI_EMERALD) {
                return 0x1045;
            }
            if s.get_event_chk_inf(EVENTCHKINF_04) {
                return 0x1034;
            }
            if s.cur_equip_value(EQUIP_TYPE_SHIELD) == EQUIP_VALUE_SHIELD_DEKU && s.cur_equip_value(EQUIP_TYPE_SWORD) == EQUIP_VALUE_SWORD_KOKIRI {
                return 0x1033;
            }
            if s.get_inf_table(INFTABLE_0C) {
                return 0x1030;
            }
            0x102F
        }
        SCENE_MIDOS_HOUSE => {
            *message_entry = 0;
            *message_state = TEXT_STATE_NONE;
            if s.get_event_chk_inf(EVENTCHKINF_OBTAINED_ZELDAS_LETTER) { 0x1028 } else { 0x1046 }
        }
        SCENE_LOST_WOODS => {
            *message_entry = 0;
            *message_state = TEXT_STATE_NONE;
            if s.get_event_chk_inf(EVENTCHKINF_48) {
                return if s.get_inf_table(INFTABLE_19) { 0x1071 } else { 0x1070 };
            }
            if s.get_event_chk_inf(EVENTCHKINF_0A) {
                return 0x1068;
            }
            if s.get_inf_table(INFTABLE_15) { 0x1061 } else { 0x1060 }
        }
        _ => 0,
    }
}

/// `EnMd_TrackMessageState`: counts the conversation's boxes (`messageEntry`): each time the state
/// (`messageState`) leaves waiting for the next box, an event end, the closing, or a box with a next
/// text.
fn track_message_state(play: &PlayState, message_entry: &mut u8, message_state: &mut u8) -> u8 {
    let dialog_state = play.message_state();
    if matches!(*message_state, TEXT_STATE_AWAITING_NEXT | TEXT_STATE_EVENT | TEXT_STATE_CLOSING | TEXT_STATE_DONE_HAS_NEXT) && *message_state != dialog_state {
        *message_entry = message_entry.wrapping_add(1);
    }
    *message_state = dialog_state;
    dialog_state
}

/// `EnMd_UpdateTalkState`: the conversation's state each frame (`Npc_UpdateTalking`'s second callback). 1
/// while it goes on; as it closes, the flag its text sets, then 0, or 2 for the texts that end
/// in him moving (0x1033, and 0x1067 in the Lost Woods); 2 at an event end's A.
fn update_talk_state(play: &mut PlayState, actor: &mut Actor, message_entry: &mut u8, message_state: &mut u8) -> i16 {
    match track_message_state(play, message_entry, message_state) {
        TEXT_STATE_NONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_DONE_FADING | TEXT_STATE_CHOICE | TEXT_STATE_DONE | TEXT_STATE_SONG_DEMO_DONE | TEXT_STATE_8 | TEXT_STATE_9 => 1,
        TEXT_STATE_CLOSING => {
            match actor.text_id {
                0x1028 => play.save.set_event_chk_inf(EVENTCHKINF_0F),
                0x102F => {
                    play.save.set_event_chk_inf(EVENTCHKINF_MIDO_DENIED_DEKU_TREE_ACCESS);
                    play.save.set_inf_table(INFTABLE_0C);
                }
                0x1060 => play.save.set_inf_table(INFTABLE_15),
                0x1070 => play.save.set_inf_table(INFTABLE_19),
                0x1033 | 0x1067 => return 2,
                _ => {}
            }
            0
        }
        TEXT_STATE_EVENT => {
            if play.message_should_advance() {
                2
            } else {
                1
            }
        }
        _ => 1,
    }
}

/// Indices into the render state's extras.
mod rs {
    /// `angles`: head x, y; torso x, y; then the sway (torso, left and right upper arms).
    pub const HEAD: usize = 0;
    pub const TORSO: usize = 2;
    pub const SWAY: usize = 4;
    /// `values`: the alpha.
    pub const ALPHA: usize = 0;
    /// `switches`: the eye.
    pub const EYE: usize = 0;
}

impl ActorImpl for EnMd {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnMd_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        self.skel.update();
        self.update_eyes(play);
        self.update_alpha_by_distance(play);
        self.actor.move_forward();
        self.update_talking(play);
        self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
        match self.action {
            Action::Moved => self.idle(play),
            Action::Arrived => self.watch(play),
            Action::Blocking => self.block_path(play),
            Action::Ocarina => self.listen_to_ocarina(play),
            Action::Walking => self.walk(play),
        }
    }
    /// `EnMd_PostLimbDraw`, the head: the focus 400 along it.
    fn draw_update(&mut self, _play: &mut PlayState) {
        let t = &self.interact_info;
        let bones = Self::pose(&self.skeleton, &self.skel.joint_table, [t.head[0], t.head[1]], [t.torso[0], t.torso[1]], &Self::sway_of(&self.fidget_table_y, &self.fidget_table_z));
        let rs = RenderState::of(&self.actor);
        let m = oot_game::play::actor_draw_matrix(&rs);
        if let Some(head) = bones.get(MIDO_LIMB_HEAD - 1) {
            self.actor.focus_pos = (m * *head).transform_point3(Vec3::new(400.0, 0.0, 0.0));
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        let t = &self.interact_info;
        let sway = Self::sway_of(&self.fidget_table_y, &self.fidget_table_z);
        rs.angles = vec![t.head[0], t.head[1], t.torso[0], t.torso[1], sway[0].1, sway[0].2, sway[1].1, sway[1].2, sway[2].1, sway[2].2];
        rs.values = vec![self.alpha as f32];
        rs.switches = vec![self.eye_idx.clamp(0, 2) as u32];
        rs
    }
    /// `EnMd_Draw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        if rs.angles.len() < 10 || rs.values.is_empty() || rs.switches.is_empty() {
            return;
        }
        let alpha = rs.values[rs::ALPHA] as i32 as i16;
        if alpha == 0 {
            return;
        }
        let a = &rs.angles;
        let sway = [(MIDO_LIMB_TORSO, a[rs::SWAY], a[rs::SWAY + 1]), (MIDO_LIMB_LEFT_UPPER_ARM, a[rs::SWAY + 2], a[rs::SWAY + 3]), (MIDO_LIMB_RIGHT_UPPER_ARM, a[rs::SWAY + 4], a[rs::SWAY + 5])];
        let bones = Self::pose(&self.skeleton, &joints.rot, [a[rs::HEAD], a[rs::HEAD + 1]], [a[rs::TORSO], a[rs::TORSO + 1]], &sway);
        let xlu = alpha != 255;
        let mut sv = SegmentValues::default();
        sv.env[SEG_ALPHA as usize] = Some([0, 0, 0, alpha.clamp(0, 255) as u8]);
        let key = MeshKey::named(keys::bake(&bake_name(rs.switches[rs::EYE] as usize, xlu)));
        let cmd = DrawCmd { mesh: key, transform: oot_game::play::actor_draw_matrix(rs), bones, params: eng_gfx::DrawParams { segments: Some(sv), ..Default::default() } };
        if xlu {
            out.xlu.push(cmd);
        } else {
            out.opa.push(cmd);
        }
        // ActorShadow_DrawCircle (shadowScale 24, shadowAlpha = alpha): the circle shadow's
        // stand-in, at full strength.
        let (floor, _) = play.col.entity_raycast_down(rs.pos + Vec3::Y * 20.0);
        let shadow = Mat4::from_translation(Vec3::new(rs.pos.x, floor + 0.3, rs.pos.z)) * Mat4::from_scale(Vec3::new(24.0, 1.0, 24.0));
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
