//! `En_Kz` (`ovl_En_Kz/z_en_kz.c`): King Zora, on his throne at the top of Zora's Domain (room 0,
//! params 0x0100: path 1), blocking the way to Zora's Fountain.
//!
//! **His talk** (`EnKz_UpdateTalking`, the C's own copy of `Npc_UpdateTalking`): a talk accepted
//! starts it; while talking `EnKz_UpdateTalkState` runs; else he's targetable
//! (`ACTOR_FLAG_ATTENTION_ENABLED`) only with Link in front of his home (the yaw from his home to
//! Link within 1638 of his facing, in float: no wrap) and 265 or more from him (Link talks to him
//! from the floor below the throne); on screen (x -30 to 360, y -10 to 240) he offers to talk
//! within 340 measured from his home (`xzDistToPlayer` swapped for that distance around
//! `Actor_OfferTalk`) and picks his text (`EnKz_GetTextId`): a mask's reaction first
//! (`MASK_REACTION_SET_KING_ZORA`); a child's (`EnKz_GetTextIdChild`) 0x402B with the Zora's
//! Sapphire, 0x401C once he has the letter, else 0x401A, which also asks Player for Ruto's letter
//! (`player->exchangeItemId = EXCH_ITEM_BOTTLE_RUTOS_LETTER`).
//!
//! **Ruto's letter** (`func_80A9CB18`): a talk accepted on 0x401A before he has it becomes 0x401B
//! when Player showed the letter (`Actor_GetPlayerExchangeItemId`), Player's text with it. 0x401B
//! sounds the chime as it fades in (`NA_SE_SY_CORRECT_CHIME`) and, read (`TEXT_STATE_DONE` and
//! advanced), is his action: `EnKz_PreMweepWait` turns to the mweep (`gKzMweepAnim`) and
//! `EnKz_SetupMweep` makes his own sub camera (`Play_CreateSubCamera`, the main one waiting) at
//! his position 60 up, seen from his home 100 down and 260 south, holds Link
//! (`PLAYER_CSACTION_8`) and sets his speed to 0.1. `EnKz_Mweep` follows path 1 (`EnKz_FollowPath`:
//! turning to each point by `Math_SmoothStepToS`, on to the next within sqrt 10, back to 0 after
//! the last) with the camera on him, `NA_SE_VO_KZ_MOVE` on the mweep's frame 13; past the path's
//! last point (the waypoint back at 0) he stands (`gKzIdleAnim`), the letter's bottle is emptied
//! (`Inventory_ReplaceItem(ITEM_BOTTLE_RUTOS_LETTER, ITEM_BOTTLE_EMPTY)`), he's put on the path's
//! last point (`EnKz_SetMovedPos`), `EVENTCHKINF_GAVE_LETTER_TO_KING_ZORA` is set and he stops;
//! `EnKz_StopMweep` gives the main camera back, clears his and lets Link go
//! (`PLAYER_CSACTION_7`); then he waits (`EnKz_Wait`). With the flag set his init puts him on the
//! path's last point at once.
//!
//! **The adult's** (the Phase 16 part): his init spawns the ice he's frozen in (`Bg_Ice_Shelter`
//! 0x04FF as his child, before `INFTABLE_138`), which isn't ported: an adult's King Zora is a
//! placeholder from his init on (as `En_Go2`'s adult Gorons, ADR 0073). His adult functions are
//! ported all the same (`EnKz_GetTextIdAdult`: the prescription asked for, 0x4012 or 0x401F with
//! the Zora Tunic, `INFTABLE_139`, 0x4045 after the Serenade; the prescription's 0x4014 and its
//! choice, `EnKz_SetupGetItem`'s Eyeball Frog or Zora Tunic, `EnKz_StartTimer`'s 180-second sub
//! timer; the update's `INFTABLE_138`), for when the ice is: they're not reached now.
//!
//! Drawn as `EnKz_Draw` does: `Gfx_SetupDL_37Opa`, the eyes on segment 8 (`sEyeSegments`: open,
//! half, closed; he blinks every 30 to 60 frames, `EnKz_UpdateEyes`), `gKzSkel` with his cloak and
//! hands' sway (limbs 8 to 10, `Actor_UpdateFidgetTables` while he waits), from a bake per eye
//! (docs/adr/0012-actor-bakes.md); the focus 2600 along his head (`EnKz_PostLimbDraw`, limb 11). No
//! shadow (`ActorShape_Init`'s none).
//!
//! The whole overlay is ported but the adult's ice (`Bg_Ice_Shelter`, Phase 16): a placeholder.
//! The gc-eu-mq-dbg build's (`OOT_VERSION >= PAL_1_0`) branches throughout: the talk's yaw check
//! in `EnKz_UpdateTalking`, `TEXT_STATE_DONE`'s cases, `isTrading`, `EnKz_StartTimer`'s text check.
//!
//! Params: `path` bits 8 to 15 (the scene path he slides along: the Domain's 0x0100, path 1; 0xFF
//! none: no slide and no moved position); bits 0 to 7 unused (0 in the data).
//! Flags: reads and sets EVENTCHKINF_GAVE_LETTER_TO_KING_ZORA (0x33: the letter shown, moved aside);
//! the adult's (a placeholder now): reads and sets INFTABLE_138 (his ice spawned once) and
//! INFTABLE_139 (0x4012's prescription talk had), clears EVENTINF_MARATHON_ACTIVE (the frog's
//! timer).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{cos_s, sin_s, smooth_step_to_s, vec3f_yaw};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, Actor};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind, TextLine};
use oot_game::audio::sfx::{NA_SE_SY_CORRECT_CHIME, NA_SE_SY_TRE_BOX_APPEAR};
use oot_game::camera::{CAM_STAT_ACTIVE, CAM_STAT_WAIT};
use oot_game::collision_check::*;
use oot_game::get_item::{actor_has_parent, offer_get_item_range};
use oot_game::item::{EQUIP_INV_TUNIC_ZORA, EQUIP_TYPE_TUNIC, GI_TUNIC_ZORA, ITEM_BOTTLE_EMPTY, ITEM_BOTTLE_RUTOS_LETTER, ITEM_POCKET_EGG, QUEST_ZORA_SAPPHIRE, inventory_replace_item};
use oot_game::message::*;
use oot_game::npc::{actor_update_fidget_tables, get_player_exchange_item_id, offer_talk, process_talk_request};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::path::{path_get_by_index, rad_to_binang_trunc};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::scene::Path;
use oot_game::skelanime_std::*;
use oot_game::{event_chk_inf, event_inf, inf_table};

use crate::player::Player;

/// `ACTOR_EN_KZ` (`actor_table.h`: 0x0164).
pub const ACTOR_EN_KZ: i16 = 0x0164;
/// `OBJECT_KZ` (`object_table.h`: 0x00FF).
pub const OBJECT: &str = "object_kz";

/// `En_Kz_Profile`: `ACTORCAT_NPC`, `FLAGS` (`ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY`).
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_EN_KZ, name: "En_Kz", category: ACTORCAT_NPC, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY, object: OBJECT, schema: &SCHEMA };

/// `PARAMS_GET_S(params, 8, 8)`: the scene path he slides along (`PARAMS_GET_NOSHIFT(params, 8, 8)
/// == 0xFF00`: none).
pub const PATH: Field = Field::params("path", 8, 8, Kind::PathIndex, "the scene path he slides along as he moves aside (0xFF: none, he never moves)");
/// Bits 0 to 7: read by nothing.
pub const UNUSED: Field = Field::params("unused", 0, 8, Kind::Unused, "read by nothing (0 in the data)");

// `EVENTCHKINF_*`, `INFTABLE_*`, `EVENTINF_*` (`save.h`).
/// `EVENTCHKINF_GAVE_LETTER_TO_KING_ZORA` (`save.h`: 0x33).
pub const EVENTCHKINF_GAVE_LETTER_TO_KING_ZORA: u16 = 0x33;
/// `INFTABLE_138` (`save.h`: 0x138): an adult has met him (his ice spawned only before).
pub const INFTABLE_138: u16 = 0x138;
/// `INFTABLE_139` (`save.h`: 0x139): 0x4012 (the prescription asked for) read.
pub const INFTABLE_139: u16 = 0x139;
/// `EVENTINF_MARATHON_ACTIVE` (`save.h`: 0x10).
pub const EVENTINF_MARATHON_ACTIVE: u16 = 0x10;

/// En_Kz's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[PATH, UNUSED],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_GAVE_LETTER_TO_KING_ZORA)).reads().sets().note("Ruto's letter shown: he moves aside (and stays aside)"),
        FlagUse::new(inf_table!(INFTABLE_138)).reads().sets().note("an adult has seen him: his ice isn't spawned again (an adult's King Zora is a placeholder now)"),
        FlagUse::new(inf_table!(INFTABLE_139)).reads().sets().note("an adult has read 0x4012 (the prescription asked for)"),
        FlagUse::new(event_inf!(EVENTINF_MARATHON_ACTIVE)).clears().note("the Eyeball Frog given: the frog's timer isn't the marathon's"),
    ],
    todo: false,
    objects: &[],
    // `EnKz_GetTextId` (a mask's reaction first, then the child's or the adult's), then
    // `func_80A9CB18`'s swaps (the letter, the prescription) and 0x4014's choice.
    texts: &[
        TextLine::new(0x718C).cond("Link wearing the Keaton Mask, the Skull Mask, the Bunny Hood or the Mask of Truth").note("what's that face?"),
        TextLine::new(0x718D).cond("Link wearing the Spooky Mask or the Gerudo Mask").note("a mask"),
        TextLine::new(0x718E).cond("Link wearing the Goron Mask").note("a Goron's face"),
        TextLine::new(0x718F).cond("Link wearing the Zora Mask").note("a Zora's face"),
        TextLine::new(0x402B).cond("Link a child, with the Zora's Sapphire").note("thanks for saving Ruto"),
        TextLine::new(0x401C).cond("Link a child, after Ruto's letter").note("go on to Lord Jabu-Jabu"),
        TextLine::new(0x401A).cond("Link a child, before Ruto's letter (asks for it); an adult after 0x4012, before the Serenade").note("Ruto is missing"),
        TextLine::new(0x401B).cond("Link a child, Ruto's letter shown").note("a message in a bottle: he moves aside"),
        TextLine::new(0x401F).cond("only for an adult (a placeholder now), with the Zora Tunic").note("frozen, before 0x4012"),
        TextLine::new(0x4012).cond("only for an adult (a placeholder now)").note("frozen: the ice; asks for the prescription"),
        TextLine::new(0x4045).cond("only for an adult (a placeholder now), after 0x4012, with the Serenade").note("frozen"),
        TextLine::new(0x4014).cond("only for an adult (a placeholder now), the prescription shown").note("the Eyeball Frog offered: a choice"),
        TextLine::new(0x4016).cond("only for an adult (a placeholder now), 0x4014's \"no\"").note("the frog refused"),
    ],
};

/// `ITEM_TRADE_ADULT` (`item.h`: `ITEM_POCKET_EGG`), `ITEM_PRESCRIPTION` (0x34),
/// `ITEM_EYEBALL_FROG` (0x35).
const ITEM_TRADE_ADULT: u8 = ITEM_POCKET_EGG;
const ITEM_PRESCRIPTION: u8 = 0x34;
const ITEM_EYEBALL_FROG: u8 = 0x35;
/// `GI_EYEBALL_FROG` (`item.h`: 0x24).
const GI_EYEBALL_FROG: i16 = 0x24;
/// `EXCH_ITEM_PRESCRIPTION` (`item.h`: 0x0C), `EXCH_ITEM_BOTTLE_RUTOS_LETTER` (0x1D).
pub const EXCH_ITEM_PRESCRIPTION: u8 = 0x0C;
pub const EXCH_ITEM_BOTTLE_RUTOS_LETTER: u8 = 0x1D;
/// `QUEST_SONG_SERENADE` (`item.h`: 0x08).
const QUEST_SONG_SERENADE: u32 = 0x08;
/// `NA_SE_VO_KZ_MOVE` (`voicebank_table.h`: 0x687A).
pub const NA_SE_VO_KZ_MOVE: u16 = 0x687A;
/// `PLAYER_CSACTION_7`, `PLAYER_CSACTION_8` (`player.h`): let go, held.
pub const PLAYER_CSACTION_7: u8 = 7;
pub const PLAYER_CSACTION_8: u8 = 8;
/// `ATTENTION_RANGE_3` (`actor.h`).
const ATTENTION_RANGE_3: u8 = 3;
/// `FIDGET_AMPLITUDE` (`actor.h`: 200.0f).
const FIDGET_AMPLITUDE: f32 = 200.0;
/// `ACTOR_BG_ICE_SHELTER` (`actor_table.h`: 0x00EF), the adult's ice (not ported).
const ACTOR_BG_ICE_SHELTER: i16 = 0x00EF;

/// `NpcTalkState` (`actor.h`).
pub const NPC_TALK_STATE_IDLE: i16 = 0;
pub const NPC_TALK_STATE_TALKING: i16 = 1;
pub const NPC_TALK_STATE_ACTION: i16 = 2;

/// `MASK_REACTION_SET_KING_ZORA` (`face_reaction.h`: 0x1E).
const MASK_REACTION_SET_KING_ZORA: usize = 0x1E;
/// `sMaskReactionSetTextIds[MASK_REACTION_SET_KING_ZORA]` (`z_face_reaction.c`), by `PLAYER_MASK_*`.
const MASK_REACTION_KING_ZORA: [u16; 9] = [0x0000, 0x718C, 0x718C, 0x718D, 0x718C, 0x718E, 0x718F, 0x718D, 0x718C];

/// `sCylinderInit`.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: 0, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: 0, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 80, height: 120, y_shift: 0, pos: [0; 3] },
};

/// `sColChkInfoInit`: immovable.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit2 = CollisionCheckInfoInit2 { health: 0, cyl_radius: 0, cyl_height: 0, cyl_y_shift: 0, mass: MASS_IMMOVABLE };

/// `EnKzAnimation`: indices into `sAnimationInfo`.
pub const ENKZ_ANIM_0: usize = 0;
pub const ENKZ_ANIM_1: usize = 1;
pub const ENKZ_ANIM_2: usize = 2;

/// `sAnimationInfo`: animation, speed, start, end (-1: its last frame), mode, morph frames.
const ANIMATION_INFO: [(&str, f32, f32, f32, u8, f32); 3] = [
    ("gKzIdleAnim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gKzIdleAnim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -10.0),
    ("gKzMweepAnim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -10.0),
];

/// `EnKz_Draw`'s `sEyeSegments`, on segment 8.
const EYE_TEXTURES: [&str; 3] = ["gKzEyeOpenTex", "gKzEyeHalfTex", "gKzEyeClosedTex"];
const SEG_EYES: u8 = 0x08;
/// `Gfx_SetupDL_37Opa`'s commands, run before the skeleton.
const SEG_SETUP: u8 = 0x0C;
/// `jointTable[12]`: his 11 limbs and the root.
const KZ_LIMB_COUNT: usize = 12;
/// The limbs `EnKz_OverrideLimbDraw` sways (1-based, as `limbIndex`) and his head
/// (`EnKz_PostLimbDraw`).
const SWAY_LIMBS: [usize; 3] = [8, 9, 10];
const KZ_LIMB_HEAD: usize = 11;

/// The skeleton with eye `eye` on segment 8, after `Gfx_SetupDL_37Opa`.
pub fn bake_name(eye: usize) -> String {
    format!("En_Kz/eye{eye}")
}

/// The meshes `EnKz_Draw` draws: `gKzSkel` with each eye.
pub fn bakes() -> Vec<MeshBake> {
    (0..EYE_TEXTURES.len())
        .map(|eye| MeshBake {
            name: bake_name(eye),
            object: OBJECT.into(),
            segments: vec![(SEG_EYES, BakeSegment::Texture { file: OBJECT.into(), symbol: EYE_TEXTURES[eye].into() }), (SEG_SETUP, BakeSegment::Commands(crate::en_owl::setup_dl_37()))],
            prelude: vec![SEG_SETUP],
            body: BakeBody::Skeleton { file: OBJECT.into(), symbol: "gKzSkel".into(), limbs: Vec::new() },
        })
        .collect()
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `EnKz_PreMweepWait`: a child's: waiting for 0x401B to be read.
    PreMweepWait,
    /// `EnKz_SetupMweep`: his camera, Link held.
    SetupMweep,
    /// `EnKz_Mweep`: along the path.
    Mweep,
    /// `EnKz_StopMweep`: the camera given back, Link let go.
    StopMweep,
    /// `EnKz_Wait`: waiting (the adult's item on his action).
    Wait,
    /// `EnKz_SetupGetItem`: the adult's item offered until Link takes it.
    SetupGetItem,
    /// `EnKz_StartTimer`: the item's text read: the frog's timer.
    StartTimer,
}

pub struct EnKz {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub action: Action,
    pub collider: ColliderCylinder,
    /// `interactInfo.talkState`.
    pub talk_state: i16,
    /// `sfxPlayed`: the chime (0x401B) or the chest's sound (0x4014) played.
    pub sfx_played: bool,
    /// `isTrading`: the prescription shown (the frog given, not the tunic).
    pub is_trading: bool,
    /// `waypoint`: the path point he heads for.
    pub waypoint: i16,
    pub blink_timer: i16,
    pub eye_idx: i16,
    pub sub_cam_id: i16,
    pub return_to_cam_id: i16,
    pub fidget_table_y: [i16; KZ_LIMB_COUNT],
    pub fidget_table_z: [i16; KZ_LIMB_COUNT],
    animations: Vec<AnimationInfo>,
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

fn player_mut(play: &mut PlayState) -> Option<&mut Player> {
    let h = play.player?;
    play.actors.downcast_mut::<Player>(h)
}

// shared: MaskReaction_GetTextId
/// `MaskReaction_GetTextId(play, MASK_REACTION_SET_KING_ZORA)` (`z_face_reaction.c`):
/// `sMaskReactionSetTextIds[set][Player_GetMask]`, his row.
fn mask_reaction_get_text_id(play: &PlayState, set: usize) -> u16 {
    let mask = player(play).map_or(0, |p| p.current_mask) as usize;
    match set {
        MASK_REACTION_SET_KING_ZORA => MASK_REACTION_KING_ZORA.get(mask).copied().unwrap_or(0),
        _ => 0,
    }
}

/// `EnKz_GetTextIdChild`: 0x402B with the Sapphire, 0x401C once he has the letter, else 0x401A
/// and Player asked for the letter.
pub fn get_text_id_child(play: &mut PlayState) -> u16 {
    if play.save.check_quest_item(QUEST_ZORA_SAPPHIRE) {
        0x402B
    } else if play.save.get_event_chk_inf(EVENTCHKINF_GAVE_LETTER_TO_KING_ZORA) {
        0x401C
    } else {
        if let Some(p) = player_mut(play) {
            p.exchange_item_id = EXCH_ITEM_BOTTLE_RUTOS_LETTER;
        }
        0x401A
    }
}

/// `EnKz_GetTextIdAdult`: with the Eyeball Frog or later, 0x401F (the Zora Tunic) or 0x4012 before
/// `INFTABLE_139`, then 0x4045 (the Serenade) or 0x401A; before it, the prescription asked for and
/// 0x4012.
pub fn get_text_id_adult(play: &mut PlayState) -> u16 {
    let s = &play.save;
    if s.inv_content(ITEM_TRADE_ADULT) >= ITEM_EYEBALL_FROG {
        if !s.get_inf_table(INFTABLE_139) {
            if s.check_owned_equip(EQUIP_TYPE_TUNIC, EQUIP_INV_TUNIC_ZORA) { 0x401F } else { 0x4012 }
        } else if s.check_quest_item(QUEST_SONG_SERENADE) {
            0x4045
        } else {
            0x401A
        }
    } else {
        if let Some(p) = player_mut(play) {
            p.exchange_item_id = EXCH_ITEM_PRESCRIPTION;
        }
        0x4012
    }
}

/// `EnKz_GetTextId`: a mask's reaction, else the adult's or the child's.
pub fn get_text_id(play: &mut PlayState) -> u16 {
    let text_id = mask_reaction_get_text_id(play, MASK_REACTION_SET_KING_ZORA);
    if text_id != 0 {
        return text_id;
    }
    if play.save.adult { get_text_id_adult(play) } else { get_text_id_child(play) }
}

impl EnKz {
    /// `EnKz_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = assets.skeleton(OBJECT, "gKzSkel").and_then(|s| {
            let animations = ANIMATION_INFO
                .iter()
                .map(|&(name, play_speed, start_frame, frame_count, mode, morph_frames)| assets.animation(OBJECT, name).map(|animation| AnimationInfo { animation, play_speed, start_frame, frame_count, mode, morph_frames }))
                .collect::<anyhow::Result<Vec<_>>>()?;
            Ok((s, animations))
        });
        let (skeleton, animations) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Kz: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let mut this = EnKz {
            actor,
            // SkelAnime_InitFlex(.., &gKzSkel, NULL, jointTable, morphTable, 12).
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), None),
            skeleton,
            action: Action::PreMweepWait,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            talk_state: NPC_TALK_STATE_IDLE,
            sfx_played: false,
            is_trading: false,
            waypoint: 0,
            blink_timer: 0,
            eye_idx: 0,
            sub_cam_id: 0,
            return_to_cam_id: 0,
            fidget_table_y: [0; KZ_LIMB_COUNT],
            fidget_table_z: [0; KZ_LIMB_COUNT],
            animations,
        };
        this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::None, 0.0);
        // Collider_InitCylinder, Collider_SetCylinder: above.
        this.actor.col_chk_info.set_info2(None, &COL_CHK_INFO_INIT);
        this.actor.scale = Vec3::splat(0.01);
        this.actor.target_mode = ATTENTION_RANGE_3;
        this.talk_state = NPC_TALK_STATE_IDLE;
        this.change_anim(ENKZ_ANIM_0);

        if play.save.get_event_chk_inf(EVENTCHKINF_GAVE_LETTER_TO_KING_ZORA) {
            this.set_moved_pos(play);
        }

        if play.save.adult {
            if !play.save.get_inf_table(INFTABLE_138) {
                // Actor_SpawnAsChild(.., ACTOR_BG_ICE_SHELTER, world.pos, 0, 0, 0, 0x04FF).
                log::info!(
                    "En_Kz: an adult's King Zora (his ice, Bg_Ice_Shelter {ACTOR_BG_ICE_SHELTER:#06x} params 0x04FF, and the prescription's trade; Phase 16) isn't ported: a placeholder"
                );
            } else {
                log::info!("En_Kz: an adult's King Zora (the prescription's trade; Phase 16) isn't ported: a placeholder");
            }
            return oot_game::spawn::Placeholder::init(this.actor, play);
        }
        this.action = Action::PreMweepWait;
        Box::new(this)
    }

    /// `Animation_ChangeByInfo(&skelanime, sAnimationInfo, index)`.
    fn change_anim(&mut self, index: usize) {
        if let Some(info) = self.animations.get(index) {
            self.skel.change_by_info(info);
        }
    }

    /// `play->pathList[PARAMS_GET_S(params, 8, 8)]`, unless `PARAMS_GET_NOSHIFT(params, 8, 8) ==
    /// 0xFF00` (the C indexes the list unchecked; a path past the layer's list is none here).
    fn path<'a>(&self, play: &'a PlayState) -> Option<&'a Path> {
        let index = PATH.get(self.actor.params);
        path_get_by_index(play.setup_path_list(), index, 0xFF).map(|i| &play.setup_path_list()[i])
    }

    /// `EnKz_UpdateEyes`: open, half, closed, then 30 to 60 frames open.
    fn update_eyes(&mut self, play: &mut PlayState) {
        if decr(&mut self.blink_timer) == 0 {
            self.eye_idx += 1;
            if self.eye_idx >= 3 {
                self.blink_timer = play.rand.s16_offset(30, 30);
                self.eye_idx = 0;
            }
        }
    }

    /// `EnKz_UpdateTalkState`: 0x401B's read (done and advanced) is his action; 0x4012 (sets
    /// `INFTABLE_139`) too; 0x401F sets `INFTABLE_139`; 0x401B's chime and 0x4014's chest sound once
    /// as they fade in; 0x4014's choice: "yes" the frog offered and his action, "no" 0x4016; an
    /// event text advanced: his action.
    fn update_talk_state(&mut self, play: &mut PlayState) -> i16 {
        let mut talk_state = NPC_TALK_STATE_TALKING;
        match play.message_state() {
            TEXT_STATE_DONE => {
                talk_state = NPC_TALK_STATE_IDLE;
                match self.actor.text_id {
                    0x4012 => {
                        play.save.set_inf_table(INFTABLE_139);
                        talk_state = NPC_TALK_STATE_ACTION;
                    }
                    0x401B => talk_state = if !play.message_should_advance() { NPC_TALK_STATE_TALKING } else { NPC_TALK_STATE_ACTION },
                    0x401F => play.save.set_inf_table(INFTABLE_139),
                    _ => {}
                }
            }
            TEXT_STATE_DONE_FADING => {
                if self.actor.text_id != 0x4014 {
                    if self.actor.text_id == 0x401B && !self.sfx_played {
                        play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
                        self.sfx_played = true;
                    }
                } else if !self.sfx_played {
                    play.audio.play_sfx_centered(NA_SE_SY_TRE_BOX_APPEAR);
                    self.sfx_played = true;
                }
            }
            TEXT_STATE_CHOICE => {
                if play.message_should_advance() && self.actor.text_id == 0x4014 {
                    if play.msg_ctx.choice_index == 0 {
                        self.setup_get_item(play);
                        talk_state = NPC_TALK_STATE_ACTION;
                    } else {
                        self.actor.text_id = 0x4016;
                        play.continue_textbox(self.actor.text_id);
                    }
                }
            }
            TEXT_STATE_EVENT => {
                if play.message_should_advance() {
                    talk_state = NPC_TALK_STATE_ACTION;
                }
            }
            // TEXT_STATE_NONE, _DONE_HAS_NEXT, _CLOSING, _SONG_DEMO_DONE, _8, _9.
            _ => {}
        }
        talk_state
    }

    /// `EnKz_UpdateTalking` (the C's copy of `Npc_UpdateTalking`): true on a talk accepted.
    fn update_talking(&mut self, play: &mut PlayState, interact_range: f32) -> bool {
        if process_talk_request(&mut self.actor) {
            self.talk_state = NPC_TALK_STATE_TALKING;
            return true;
        }
        if self.talk_state != NPC_TALK_STATE_IDLE {
            self.talk_state = self.update_talk_state(play);
            return false;
        }
        let player_pos = player(play).map_or(self.actor.world_pos, |p| p.actor.world_pos);
        // f32 yaw = Math_Vec3f_Yaw(&home.pos, &player->actor.world.pos); yaw -= shape.rot.y: in
        // float, no wrap.
        let mut yaw = vec3f_yaw(self.actor.home_pos, player_pos) as f32;
        yaw -= self.actor.shape_rot.y as f32;
        if yaw.abs() > 1638.0 || self.actor.xz_dist_to_player < 265.0 {
            self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
            return false;
        }
        self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
        let (x, y) = oot_game::target::actor_screen_pos(play.view_proj, &self.actor);
        if !((-30..361).contains(&x) && (-10..241).contains(&y)) {
            return false;
        }
        // xzDistToPlayer from his home for the offer.
        let xz_dist_to_player = self.actor.xz_dist_to_player;
        let d = player_pos - self.actor.home_pos;
        self.actor.xz_dist_to_player = (d.x * d.x + d.z * d.z).sqrt();
        if !offer_talk(play, &self.actor, interact_range) {
            self.actor.xz_dist_to_player = xz_dist_to_player;
            return false;
        }
        self.actor.xz_dist_to_player = xz_dist_to_player;
        self.actor.text_id = get_text_id(play);
        false
    }

    /// `func_80A9CB18`: his talk within 340; accepted on 0x401A before the letter: 0x401B if
    /// Player showed the letter; an adult's prescription (0x4014) or his other texts.
    fn func_80a9cb18(&mut self, play: &mut PlayState) {
        if !self.update_talking(play, 340.0) {
            return;
        }
        if self.actor.text_id == 0x401A && !play.save.get_event_chk_inf(EVENTCHKINF_GAVE_LETTER_TO_KING_ZORA) {
            if get_player_exchange_item_id(play) == EXCH_ITEM_BOTTLE_RUTOS_LETTER {
                self.actor.text_id = 0x401B;
                self.sfx_played = false;
            } else {
                self.actor.text_id = 0x401A;
            }
            self.set_player_text_id(play);
            return;
        }
        if play.save.adult {
            if play.save.inv_content(ITEM_TRADE_ADULT) == ITEM_PRESCRIPTION && get_player_exchange_item_id(play) == EXCH_ITEM_PRESCRIPTION {
                self.actor.text_id = 0x4014;
                self.sfx_played = false;
                self.set_player_text_id(play);
                self.is_trading = true;
                return;
            }
            self.is_trading = false;
            if play.save.get_inf_table(INFTABLE_139) {
                self.actor.text_id = if play.save.check_quest_item(QUEST_SONG_SERENADE) { 0x4045 } else { 0x401A };
            } else {
                self.actor.text_id = if play.save.check_owned_equip(EQUIP_TYPE_TUNIC, EQUIP_INV_TUNIC_ZORA) { 0x401F } else { 0x4012 };
            }
            self.set_player_text_id(play);
        }
    }

    /// `player->actor.textId = this->actor.textId`.
    fn set_player_text_id(&self, play: &mut PlayState) {
        let text_id = self.actor.text_id;
        if let Some(p) = player_mut(play) {
            p.actor.text_id = text_id;
        }
    }

    /// `EnKz_FollowPath`: turned to the waypoint (`Math_SmoothStepToS`, 10, 1000, 1); within sqrt 10
    /// across the ground, on to the next (0 after the last): 1.
    fn follow_path(&mut self, play: &PlayState) -> bool {
        let Some(path) = self.path(play) else { return false };
        let count = path.count() as i16;
        let point = path.point(self.waypoint as usize);
        let path_diff_x = point.x - self.actor.world_pos.x;
        let path_diff_z = point.z - self.actor.world_pos.z;
        smooth_step_to_s(&mut self.actor.world_rot.y, rad_to_binang_trunc(oot_game::camera::f_atan2f(path_diff_x, path_diff_z)), 0xA, 0x3E8, 1);
        if path_diff_x * path_diff_x + path_diff_z * path_diff_z < 10.0 {
            self.waypoint += 1;
            if self.waypoint >= count {
                self.waypoint = 0;
            }
            return true;
        }
        false
    }

    /// `EnKz_SetMovedPos`: on the path's last point.
    fn set_moved_pos(&mut self, play: &PlayState) -> bool {
        let Some(path) = self.path(play) else { return false };
        self.actor.world_pos = path.point(path.count() - 1);
        true
    }

    /// `EnKz_PreMweepWait`: his action (0x401B read): the mweep; else his sway.
    fn pre_mweep_wait(&mut self, play: &mut PlayState) {
        if self.talk_state == NPC_TALK_STATE_ACTION {
            self.change_anim(ENKZ_ANIM_2);
            self.talk_state = NPC_TALK_STATE_IDLE;
            self.action = Action::SetupMweep;
        } else {
            actor_update_fidget_tables(play, &mut self.fidget_table_y, &mut self.fidget_table_z, KZ_LIMB_COUNT);
        }
    }

    /// The mweep's camera: at him 60 up, from his home 100 down and 260 south.
    fn mweep_at_eye(&self) -> (Vec3, Vec3) {
        let mut sub_cam_at = self.actor.world_pos;
        let mut sub_cam_eye = self.actor.home_pos;
        sub_cam_at.y += 60.0;
        sub_cam_eye.y += -100.0;
        sub_cam_eye.z += 260.0;
        (sub_cam_at, sub_cam_eye)
    }

    /// `EnKz_SetupMweep`: his sub camera active (the main one waiting), Link held
    /// (`PLAYER_CSACTION_8`), speed 0.1.
    fn setup_mweep(&mut self, play: &mut PlayState) {
        self.sub_cam_id = play.create_sub_camera();
        self.return_to_cam_id = play.active_cam_id;
        play.change_camera_status(self.return_to_cam_id, CAM_STAT_WAIT);
        play.change_camera_status(self.sub_cam_id, CAM_STAT_ACTIVE);
        let (at, eye) = self.mweep_at_eye();
        play.camera_set_at_eye(self.sub_cam_id, at, eye);
        play.player_set_cs_action_with_halted_actors(play.cur_actor, PLAYER_CSACTION_8);
        self.actor.speed_xz = 0.1;
        self.action = Action::Mweep;
    }

    /// `EnKz_Mweep`: the camera on him; past the path's end: standing, the letter's bottle empty,
    /// on the last point, the flag, stopped. `NA_SE_VO_KZ_MOVE` on frame 13.
    fn mweep(&mut self, play: &mut PlayState) {
        let (at, eye) = self.mweep_at_eye();
        play.camera_set_at_eye(self.sub_cam_id, at, eye);
        if self.follow_path(play) && self.waypoint == 0 {
            self.change_anim(ENKZ_ANIM_1);
            inventory_replace_item(&mut play.save, ITEM_BOTTLE_RUTOS_LETTER, ITEM_BOTTLE_EMPTY);
            self.set_moved_pos(play);
            play.save.set_event_chk_inf(EVENTCHKINF_GAVE_LETTER_TO_KING_ZORA);
            self.actor.speed_xz = 0.0;
            self.action = Action::StopMweep;
        }
        if self.skel.cur_frame == 13.0 {
            audio_play_actor_sfx2(play, NA_SE_VO_KZ_MOVE);
        }
    }

    /// `EnKz_StopMweep`: the camera back, his cleared, Link let go.
    fn stop_mweep(&mut self, play: &mut PlayState) {
        play.change_camera_status(self.return_to_cam_id, CAM_STAT_ACTIVE);
        play.clear_camera(self.sub_cam_id);
        play.player_set_cs_action_with_halted_actors(play.cur_actor, PLAYER_CSACTION_7);
        self.action = Action::Wait;
    }

    /// `EnKz_Wait`: his action: the adult's item; else his sway.
    fn wait(&mut self, play: &mut PlayState) {
        if self.talk_state == NPC_TALK_STATE_ACTION {
            self.action = Action::SetupGetItem;
            self.setup_get_item(play);
        } else {
            actor_update_fidget_tables(play, &mut self.fidget_table_y, &mut self.fidget_table_z, KZ_LIMB_COUNT);
        }
    }

    /// `EnKz_SetupGetItem`: taken: talking, the timer next; else the Eyeball Frog (traded) or the
    /// Zora Tunic offered at Link's distance + 1 both ways.
    fn setup_get_item(&mut self, play: &mut PlayState) {
        if actor_has_parent(&self.actor) {
            self.actor.parent = None;
            self.talk_state = NPC_TALK_STATE_TALKING;
            self.action = Action::StartTimer;
        } else {
            let get_item_id = if self.is_trading { GI_EYEBALL_FROG } else { GI_TUNIC_ZORA };
            let y_range = self.actor.y_dist_to_player.abs() + 1.0;
            let xz_range = self.actor.xz_dist_to_player + 1.0;
            offer_get_item_range(play, &self.actor, get_item_id, xz_range, y_range);
        }
    }

    /// `EnKz_StartTimer`: the item's text read: with the frog, the 180-second sub timer (not the
    /// marathon's); waiting again.
    fn start_timer(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_DONE && play.message_should_advance() {
            if play.save.inv_content(ITEM_TRADE_ADULT) == ITEM_EYEBALL_FROG {
                play.interface_set_sub_timer(180);
                play.save.clear_event_inf(EVENTINF_MARATHON_ACTIVE);
            }
            self.talk_state = NPC_TALK_STATE_IDLE;
            self.action = Action::Wait;
        }
    }

    /// His skeleton posed as `EnKz_OverrideLimbDraw` sways it: limbs 8, 9 and 10 by the fidget
    /// tables (`FIDGET_AMPLITUDE`).
    fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], sway: &[(usize, i16, i16)]) -> Vec<Mat4> {
        skeleton.pose_override(joints, |limb, _pos, rot| {
            if let Some(&(_, a, b)) = sway.iter().find(|s| s.0 == limb) {
                rot[1] = (rot[1] as f32 + sin_s(a) * FIDGET_AMPLITUDE) as i32 as i16;
                rot[2] = (rot[2] as f32 + cos_s(b) * FIDGET_AMPLITUDE) as i32 as i16;
            }
            Mat4::IDENTITY
        })
    }

    fn sway_of(y: &[i16; KZ_LIMB_COUNT], z: &[i16; KZ_LIMB_COUNT]) -> [(usize, i16, i16); 3] {
        SWAY_LIMBS.map(|l| (l, y[l], z[l]))
    }
}

/// Indices into the render state's extras.
mod rs {
    /// `angles`: the sway (limbs 8, 9, 10: y, z each).
    pub const SWAY: usize = 0;
    pub const ANGLES: usize = 6;
    /// `switches`: the eye.
    pub const EYE: usize = 0;
}

impl ActorImpl for EnKz {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnKz_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if play.save.adult && !play.save.get_inf_table(INFTABLE_138) {
            play.save.set_inf_table(INFTABLE_138);
        }
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        self.skel.update();
        self.update_eyes(play);
        self.actor.move_forward();
        if self.action != Action::StartTimer {
            self.func_80a9cb18(play);
        }
        match self.action {
            Action::PreMweepWait => self.pre_mweep_wait(play),
            Action::SetupMweep => self.setup_mweep(play),
            Action::Mweep => self.mweep(play),
            Action::StopMweep => self.stop_mweep(play),
            Action::Wait => self.wait(play),
            Action::SetupGetItem => self.setup_get_item(play),
            Action::StartTimer => self.start_timer(play),
        }
    }

    /// `EnKz_PostLimbDraw`: the focus 2600 along his head (limb 11).
    fn draw_update(&mut self, _play: &mut PlayState) {
        let bones = Self::pose(&self.skeleton, &self.skel.joint_table, &Self::sway_of(&self.fidget_table_y, &self.fidget_table_z));
        if let Some(head) = bones.get(KZ_LIMB_HEAD - 1) {
            let m = actor_draw_matrix(&RenderState::of(&self.actor));
            self.actor.focus_pos = (m * *head).transform_point3(Vec3::new(2600.0, 0.0, 0.0));
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        let sway = Self::sway_of(&self.fidget_table_y, &self.fidget_table_z);
        rs.angles = sway.iter().flat_map(|&(_, a, b)| [a, b]).collect();
        rs.switches = vec![self.eye_idx.clamp(0, 2) as u32];
        rs
    }

    /// `EnKz_Draw`: `Gfx_SetupDL_37Opa`, the eye on segment 8, the skeleton.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        if rs.angles.len() < rs::ANGLES || rs.switches.is_empty() {
            return;
        }
        let a = &rs.angles;
        let sway = [(SWAY_LIMBS[0], a[rs::SWAY], a[rs::SWAY + 1]), (SWAY_LIMBS[1], a[rs::SWAY + 2], a[rs::SWAY + 3]), (SWAY_LIMBS[2], a[rs::SWAY + 4], a[rs::SWAY + 5])];
        let bones = Self::pose(&self.skeleton, &joints.rot, &sway);
        let eye = (rs.switches[rs::EYE] as usize).min(EYE_TEXTURES.len() - 1);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(eye))), transform: actor_draw_matrix(rs), bones, params: Default::default() });
    }

    /// `EnKz_Destroy`: `Collider_DestroyCylinder`.
    fn destroy(&mut self, _play: &mut PlayState) {}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mask_reactions_are_king_zoras_row() {
        // PLAYER_MASK_ZORA (6): 0x718F; PLAYER_MASK_GORON (5): 0x718E.
        assert_eq!(MASK_REACTION_KING_ZORA[6], 0x718F);
        assert_eq!(MASK_REACTION_KING_ZORA[5], 0x718E);
        assert_eq!(bakes().len(), 3);
    }
}
