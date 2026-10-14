//! `En_Ma1` (`ovl_En_Ma1/z_en_ma1.c`): child Malon. `shape.rot.z` is 3 for the ranch's Malon.
//!
//! She is there (`EnMa1_ShouldSpawn`) in the market until Talon is back from the castle or she has
//! moved to the castle (by day and by night), by the castle's vines from Link's second visit
//! there, in the ranch house at night and the ranch by day after Talon's return, and in the
//! ranch's cutscene layer 5.
//!
//! She sings (`gMalonChildSingAnim`, the Lon Lon song's voice through `Audio_ToggleMalonSinging`
//! and `Audio_UpdateMalonSinging` by her distance to the camera), stops to talk (her text by the
//! story: `EnMa1_GetTextId`, the flags her texts set as they close: `EnMa1_UpdateTalkState`) and
//! gives the Weird Egg (`EnMa1_GiveWeirdEgg`). At the ranch after Talon's return she teaches
//! Epona's Song (`EnMa1_IdleTeachSong` to `EnMa1_WaitForPlayback`).
//!
//! Drawn as `EnMa1_Draw` does: her eyes on segment 8 and her mouth on segment 9 (always the
//! neutral one: `mouthIndex` stays 0), her thighs not drawn, her head and chest turned to Link;
//! from meshes baked per eye (docs/adr/0012-actor-bakes.md).
//!
//! Not ported here: the mask reactions (`MaskReaction_GetTextId`: no masks yet, so none).
//!
//! Params: `params` unused (placed -1, read by nothing). `shape.rot.z`: 3 the ranch's Malon (by
//! day after Talon's return, and in the ranch's layer 5); else 0.
//! Flags: reads EVENTCHKINF_TALON_RETURNED_FROM_CASTLE (where she is, and the song taught);
//! reads and sets INFTABLE_MALON_SPAWNED_AT_HYRULE_CASTLE (her first spawn at the castle sets it
//! and goes); her texts read EVENTCHKINF_CAN_LEARN_EPONAS_SONG,
//! EVENTCHKINF_TALKED_TO_CHILD_MALON_AT_RANCH, INFTABLE_TOLD_EPONA_IS_SCARED,
//! EVENTCHKINF_RECEIVED_WEIRD_EGG and INFTABLE_TALKED_TO_MALON_FIRST_TIME, and their closing sets
//! them (0x2041 INFTABLE_TALKED_TO_MALON_FIRST_TIME and EVENTCHKINF_TALKED_TO_MALON_FIRST_TIME,
//! 0x2047 EVENTCHKINF_TALKED_TO_CHILD_MALON_AT_RANCH, 0x2048 INFTABLE_TOLD_EPONA_IS_SCARED, 0x2049
//! EVENTCHKINF_CAN_LEARN_EPONAS_SONG); the Weird Egg given sets EVENTCHKINF_RECEIVED_WEIRD_EGG.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::binang_to_rad;
use glam::{Mat4, Vec3};
use oot_game::actor::{
    ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_UPDATE_DURING_OCARINA,
    Actor, UPDBGCHECKINFO_FLAG_2,
};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind, Source};
use oot_game::{event_chk_inf, inf_table};
use oot_game::audio::NA_BGM_LONLON;
use oot_game::collision_check::*;
use oot_game::get_item::{actor_has_parent, offer_get_item_range};
use oot_game::message::*;
use oot_game::npc::{NpcTrack, talk_update, track_point};
use oot_game::pack::{BakeBody, BakeSegment, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::*;

/// `ACTOR_EN_MA1` (`actor_table.h`: 0x00E7).
pub const ACTOR_EN_MA1: i16 = 0x00E7;
pub const OBJECT: &str = "object_ma1";

/// `En_Ma1_Profile`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_MA1,
    name: "En_Ma1",
    category: ACTORCAT_NPC,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `this->actor.params`: placed -1 everywhere; nothing reads it.
pub const PARAMS: Field = Field::whole("params", Kind::Unused, "placed -1; read by nothing");
/// `this->actor.shape.rot.z == 3` (`EnMa1_ShouldSpawn`): the ranch's Malon.
pub const RANCH: Field = Field::rot(
    "shape.rot.z",
    Source::RotZ,
    0,
    16,
    Kind::Named(&[(0, "the market's, the castle's or the ranch house's Malon"), (3, "the ranch's Malon (by day after Talon's return, and in layer 5)")]),
    "3: the ranch's Malon",
);

/// En_Ma1's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[PARAMS, RANCH],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_TALKED_TO_MALON_FIRST_TIME)).sets().note("text 0x2041 closed"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_RECEIVED_WEIRD_EGG)).reads().sets().note("the Weird Egg given"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_TALON_RETURNED_FROM_CASTLE)).reads().note("where she is; the song taught at the ranch"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_TALKED_TO_CHILD_MALON_AT_RANCH)).reads().sets().note("text 0x2047 closed"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_CAN_LEARN_EPONAS_SONG)).reads().sets().note("text 0x2049 closed"),
        FlagUse::new(inf_table!(INFTABLE_TALKED_TO_MALON_FIRST_TIME)).reads().sets().note("text 0x2041 closed"),
        FlagUse::new(inf_table!(INFTABLE_TOLD_EPONA_IS_SCARED)).reads().sets().note("text 0x2048 closed"),
        FlagUse::new(inf_table!(INFTABLE_MALON_SPAWNED_AT_HYRULE_CASTLE)).reads().sets().note("set by her first spawn at the castle, which doesn't stay"),
    ],
    todo: false,
};

// `SCENE_*` (`scene_table.h`).
const SCENE_MARKET_DAY: u16 = 0x20;
const SCENE_MARKET_NIGHT: u16 = 0x21;
const SCENE_LON_LON_BUILDINGS: u16 = 0x4C;
const SCENE_HYRULE_CASTLE: u16 = 0x5F;
const SCENE_LON_LON_RANCH: u16 = 0x63;

// `EVENTCHKINF_*`, `INFTABLE_*` (`save.h`).
pub const EVENTCHKINF_TALKED_TO_MALON_FIRST_TIME: u16 = 0x10;
pub const EVENTCHKINF_RECEIVED_WEIRD_EGG: u16 = 0x12;
pub const EVENTCHKINF_TALON_RETURNED_FROM_CASTLE: u16 = 0x14;
pub const EVENTCHKINF_TALKED_TO_CHILD_MALON_AT_RANCH: u16 = 0x15;
pub const EVENTCHKINF_CAN_LEARN_EPONAS_SONG: u16 = 0x16;
pub const INFTABLE_TALKED_TO_MALON_FIRST_TIME: u16 = 0x84;
pub const INFTABLE_TOLD_EPONA_IS_SCARED: u16 = 0x85;
pub const INFTABLE_MALON_SPAWNED_AT_HYRULE_CASTLE: u16 = 0x8B;

/// `QUEST_SONG_EPONA` (`item.h`: 0x0D).
const QUEST_SONG_EPONA: u32 = 0x0D;
/// `GI_WEIRD_EGG` (`item.h`: 0x47).
pub const GI_WEIRD_EGG: i16 = 0x47;
/// `ENTR_LON_LON_RANCH_0` (`entrance_table.h`: 0x157).
const ENTR_LON_LON_RANCH_0: u16 = 0x157;
/// `CS_INDEX_1` (`z64cutscene.h`: 0xFFF1).
const CS_INDEX_1: u16 = 0xFFF1;
/// `TRANS_TYPE_CIRCLE(TCA_WAVE, TCC_WHITE, TCS_FAST)` (`transition.h`: `(1 << 5) | ((color & 3)
/// << 3) | ((appearance & 3) << 1) | (speed & 1)`, with `TCA_WAVE` 1, `TCC_WHITE` 1, `TCS_FAST` 0).
const TRANS_TYPE_CIRCLE_WAVE_WHITE_FAST: u8 = (1 << 5) | (1 << 3) | (1 << 1);
/// `OCARINA_MODE_03` (`ocarina.h`): the song played back right.
const OCARINA_MODE_03: u16 = 3;
/// `PLAYER_STATE2_23` (near an ocarina listener), `PLAYER_STATE2_24` (the ocarina out),
/// `PLAYER_STATE2_25` (playing to a listener) (`player.h`).
const PLAYER_STATE2_23: u32 = 1 << 23;
const PLAYER_STATE2_24: u32 = 1 << 24;
const PLAYER_STATE2_25: u32 = 1 << 25;

/// `ATTENTION_RANGE_6`.
const ATTENTION_RANGE_6: u8 = 6;
/// `NPC_TRACKING_PLAYER_AUTO_TURN`, `NPC_TRACKING_NONE` (`actor.h`).
const NPC_TRACKING_PLAYER_AUTO_TURN: i16 = 0;
const NPC_TRACKING_NONE: i16 = 1;
/// `NpcTalkState` (`actor.h`).
const NPC_TALK_STATE_IDLE: i16 = 0;
const NPC_TALK_STATE_TALKING: i16 = 1;
const NPC_TALK_STATE_ACTION: i16 = 2;
const NPC_TALK_STATE_ITEM_GIVEN: i16 = 3;

// `ChildMalonLimb` (`object_ma1.h`), 1-based as `EnMa1_OverrideLimbDraw` sees them.
const CHILD_MALON_LIMB_LEFT_THIGH: usize = 2;
const CHILD_MALON_LIMB_RIGHT_THIGH: usize = 5;
const CHILD_MALON_LIMB_CHEST: usize = 8;
const CHILD_MALON_LIMB_HEAD: usize = 15;

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
    dim: Cylinder16 { radius: 18, height: 46, y_shift: 0, pos: [0; 3] },
};

/// `sColChkInfoInit`: immovable.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit2 = CollisionCheckInfoInit2 { health: 0, cyl_radius: 0, cyl_height: 0, cyl_y_shift: 0, mass: MASS_IMMOVABLE };

/// `DamageTable_Get(22)` (`z_collision_btltbls.c`'s `sDamageTablePresets[22]`, "Used by En_Du,
/// En_Go, En_Ma1, En_Ma2, En_Ma3"): nothing hurts.
pub static S_DAMAGE_TABLE_PRESET_22: DamageTable = DamageTable { table: [dmg_entry(0, 0x0); 32] };

/// `EnMa1Animation`'s entries in `sAnimationInfo`: the animation, and the morph.
#[allow(dead_code)]
const MALON_ANIM_IDLE_NOMORPH: usize = 0;
const MALON_ANIM_IDLE: usize = 1;
const MALON_ANIM_SING_NOMORPH: usize = 2;
const MALON_ANIM_SING: usize = 3;
/// `sAnimationInfo`: (sing, morph frames); all play at 1, looped.
const ANIMATION_INFO: [(bool, f32); 4] = [(false, 0.0), (false, -10.0), (true, 0.0), (true, -10.0)];
const IDLE_ANIM: &str = "gMalonChildIdleAnim";
const SING_ANIM: &str = "gMalonChildSingAnim";

/// `sEyeTextures`.
const EYE_TEXTURES: [&str; 3] = ["gMalonChildEyeOpenTex", "gMalonChildEyeHalfTex", "gMalonChildEyeClosedTex"];
/// `sMouthTextures[0]`: the only mouth drawn (`mouthIndex` is never set).
const MOUTH_TEXTURE: &str = "gMalonChildNeutralMouthTex";

/// The bake drawn with eye `eye`.
pub fn bake_name(eye: usize) -> String {
    format!("En_Ma1/eye{eye}")
}

/// The meshes `EnMa1_Draw` draws: the skeleton with each eye, the neutral mouth, and no thighs.
pub fn bakes() -> Vec<MeshBake> {
    (0..EYE_TEXTURES.len())
        .map(|eye| MeshBake {
            name: bake_name(eye),
            object: OBJECT.into(),
            segments: vec![(0x08, BakeSegment::Texture { file: OBJECT.into(), symbol: EYE_TEXTURES[eye].into() }), (0x09, BakeSegment::Texture { file: OBJECT.into(), symbol: MOUTH_TEXTURE.into() })],
            prelude: Vec::new(),
            body: BakeBody::Skeleton {
                file: OBJECT.into(),
                symbol: "gMalonChildSkel".into(),
                // EnMa1_OverrideLimbDraw: *dList = NULL for the thighs.
                limbs: [CHILD_MALON_LIMB_LEFT_THIGH, CHILD_MALON_LIMB_RIGHT_THIGH].map(|l| LimbOverride { limb: (l - 1) as u8, file: OBJECT.into(), symbol: String::new() }).to_vec(),
            },
        })
        .collect()
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Idle,
    GiveWeirdEgg,
    FinishGivingWeirdEgg,
    IdleTeachSong,
    StartTeachSong,
    TeachSong,
    WaitForPlayback,
    DoNothing,
}

pub struct EnMa1 {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub action: Action,
    pub collider: ColliderCylinder,
    pub singing_disabled: bool,
    pub blink_timer: i16,
    pub eye_index: i16,
    pub mouth_index: i16,
    pub interact_info: NpcTrack,
    idle: Anim,
    sing: Anim,
}

impl EnMa1 {
    /// `EnMa1_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = (|| -> anyhow::Result<_> { Ok((assets.skeleton(OBJECT, "gMalonChildSkel")?, assets.animation(OBJECT, IDLE_ANIM)?, assets.animation(OBJECT, SING_ANIM)?)) })();
        let (skeleton, idle, sing) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Ma1: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let mut this = EnMa1 {
            // SkelAnime_InitFlex(.., gMalonChildSkel, NULL, ..).
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), None),
            skeleton,
            action: Action::Idle,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            singing_disabled: false,
            blink_timer: 0,
            eye_index: 0,
            mouth_index: 0,
            interact_info: NpcTrack::default(),
            idle,
            sing,
            actor,
        };
        // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 18).
        this.actor.shape_y_offset = 0.0;
        this.actor.col_chk_info.set_info2(Some(&S_DAMAGE_TABLE_PRESET_22), &COL_CHK_INFO_INIT);

        if !this.should_spawn(play) {
            this.actor.kill();
            return Box::new(this);
        }

        this.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
        this.actor.scale = Vec3::splat(0.01);
        this.actor.target_mode = ATTENTION_RANGE_6;
        this.interact_info.talk_state = NPC_TALK_STATE_IDLE;

        let s = &play.save;
        this.action = if !s.get_event_chk_inf(EVENTCHKINF_TALON_RETURNED_FROM_CASTLE) || s.check_quest_item(QUEST_SONG_EPONA) { Action::Idle } else { Action::IdleTeachSong };
        this.change_anim(MALON_ANIM_SING_NOMORPH);
        Box::new(this)
    }

    /// `EnMa1_GetTextId`.
    pub fn get_text_id(play: &PlayState) -> u16 {
        // MaskReaction_GetTextId(play, MASK_REACTION_SET_MALON): no masks, so 0.
        let s = &play.save;
        if s.check_quest_item(QUEST_SONG_EPONA) {
            return 0x204A;
        }
        if s.get_event_chk_inf(EVENTCHKINF_CAN_LEARN_EPONAS_SONG) {
            return 0x2049;
        }
        if s.get_event_chk_inf(EVENTCHKINF_TALKED_TO_CHILD_MALON_AT_RANCH) {
            return if s.get_inf_table(INFTABLE_TOLD_EPONA_IS_SCARED) { 0x2049 } else { 0x2048 };
        }
        if s.get_event_chk_inf(EVENTCHKINF_TALON_RETURNED_FROM_CASTLE) {
            return 0x2047;
        }
        if s.get_event_chk_inf(EVENTCHKINF_RECEIVED_WEIRD_EGG) {
            return 0x2044;
        }
        if s.get_inf_table(INFTABLE_TALKED_TO_MALON_FIRST_TIME) {
            return if s.get_inf_table(INFTABLE_MALON_SPAWNED_AT_HYRULE_CASTLE) { 0x2043 } else { 0x2042 };
        }
        0x2041
    }

    /// `EnMa1_UpdateTalkState`.
    fn update_talk_state(play: &mut PlayState, actor: &mut Actor) -> i16 {
        match play.message_state() {
            TEXT_STATE_CLOSING => {
                let s = &mut play.save;
                match actor.text_id {
                    0x2041 => {
                        s.set_inf_table(INFTABLE_TALKED_TO_MALON_FIRST_TIME);
                        s.set_event_chk_inf(EVENTCHKINF_TALKED_TO_MALON_FIRST_TIME);
                        NPC_TALK_STATE_IDLE
                    }
                    0x2043 => NPC_TALK_STATE_TALKING,
                    0x2047 => {
                        s.set_event_chk_inf(EVENTCHKINF_TALKED_TO_CHILD_MALON_AT_RANCH);
                        NPC_TALK_STATE_IDLE
                    }
                    0x2048 => {
                        s.set_inf_table(INFTABLE_TOLD_EPONA_IS_SCARED);
                        NPC_TALK_STATE_IDLE
                    }
                    0x2049 => {
                        s.set_event_chk_inf(EVENTCHKINF_CAN_LEARN_EPONAS_SONG);
                        NPC_TALK_STATE_IDLE
                    }
                    0x2061 => NPC_TALK_STATE_ACTION,
                    _ => NPC_TALK_STATE_IDLE,
                }
            }
            TEXT_STATE_CHOICE | TEXT_STATE_EVENT => {
                if play.message_should_advance() {
                    NPC_TALK_STATE_ACTION
                } else {
                    NPC_TALK_STATE_TALKING
                }
            }
            TEXT_STATE_DONE => {
                if play.message_should_advance() {
                    NPC_TALK_STATE_ITEM_GIVEN
                } else {
                    NPC_TALK_STATE_TALKING
                }
            }
            _ => NPC_TALK_STATE_TALKING,
        }
    }

    /// `EnMa1_ShouldSpawn`. At the castle the first time it sets
    /// `INFTABLE_MALON_SPAWNED_AT_HYRULE_CASTLE` and doesn't spawn: she's there from the second.
    fn should_spawn(&self, play: &mut PlayState) -> bool {
        if RANCH.get(self.actor.shape_rot.z) == 3 && play.save.scene_layer == 5 {
            return true;
        }
        if play.save.adult {
            return false;
        }
        let talon_back = play.save.get_event_chk_inf(EVENTCHKINF_TALON_RETURNED_FROM_CASTLE);
        let scene = play.scene_id;
        if (scene == SCENE_MARKET_NIGHT || scene == SCENE_MARKET_DAY) && !talon_back && !play.save.get_inf_table(INFTABLE_MALON_SPAWNED_AT_HYRULE_CASTLE) {
            return true;
        }
        if scene == SCENE_HYRULE_CASTLE && !talon_back {
            if play.save.get_inf_table(INFTABLE_MALON_SPAWNED_AT_HYRULE_CASTLE) {
                return true;
            }
            play.save.set_inf_table(INFTABLE_MALON_SPAWNED_AT_HYRULE_CASTLE);
            return false;
        }
        if scene == SCENE_LON_LON_BUILDINGS && !play.save.is_day() && talon_back {
            return true;
        }
        if scene != SCENE_LON_LON_RANCH {
            return false;
        }
        RANCH.get(self.actor.shape_rot.z) == 3 && play.save.is_day() && talon_back
    }

    /// `EnMa1_UpdateEyes`.
    fn update_eyes(&mut self, play: &mut PlayState) {
        // DECR(blinkTimer) == 0.
        if self.blink_timer != 0 {
            self.blink_timer -= 1;
            if self.blink_timer != 0 {
                return;
            }
        }
        self.eye_index += 1;
        if self.eye_index >= 3 {
            self.blink_timer = play.rand.s16_offset(30, 30);
            self.eye_index = 0;
        }
    }

    /// `EnMa1_ChangeAnim`.
    fn change_anim(&mut self, index: usize) {
        let (sing, morph) = ANIMATION_INFO[index];
        let anim = if sing { self.sing.clone() } else { self.idle.clone() };
        let frame_count = anim.last_frame();
        self.skel.change(anim, 1.0, 0.0, frame_count, ANIMMODE_LOOP, morph);
    }

    fn is_singing(&self) -> bool {
        self.skel.is(SING_ANIM)
    }

    /// `EnMa1_UpdateTracking`: Link, 10 above his feet; not while she sings and isn't talking.
    fn update_tracking(&mut self, play: &mut PlayState) {
        let tracking_mode = if self.interact_info.talk_state == NPC_TALK_STATE_IDLE && self.is_singing() { NPC_TRACKING_NONE } else { NPC_TRACKING_PLAYER_AUTO_TURN };
        self.interact_info.target = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(self.actor.world_pos);
        self.interact_info.target.y -= -10.0;
        let mut t = self.interact_info;
        track_point(play, &mut self.actor, &mut t, 0, tracking_mode);
        self.interact_info = t;
    }

    /// `EnMa1_UpdateSinging`: her voice off while she talks, on again after.
    fn update_singing(&mut self, play: &mut PlayState) {
        if self.is_singing() {
            if self.interact_info.talk_state == NPC_TALK_STATE_IDLE {
                if self.singing_disabled {
                    self.singing_disabled = false;
                    play.audio.toggle_malon_singing(false);
                }
            } else if !self.singing_disabled {
                self.singing_disabled = true;
                play.audio.toggle_malon_singing(true);
            }
        }
    }

    /// The idle and the song lesson's idle: the idle animation while talking, singing otherwise.
    fn update_anim(&mut self) {
        if self.interact_info.talk_state != NPC_TALK_STATE_IDLE {
            if !self.skel.is(IDLE_ANIM) {
                self.change_anim(MALON_ANIM_IDLE);
            }
        } else if !self.is_singing() {
            self.change_anim(MALON_ANIM_SING);
        }
    }

    /// `EnMa1_Idle`: gone from the castle once Talon is back; the Weird Egg when her text asks.
    fn idle(&mut self, play: &mut PlayState) {
        self.update_anim();
        let s = &play.save;
        let talon_back = s.get_event_chk_inf(EVENTCHKINF_TALON_RETURNED_FROM_CASTLE);
        if play.scene_id == SCENE_HYRULE_CASTLE && talon_back {
            self.actor.kill();
        } else if (!talon_back || s.check_quest_item(QUEST_SONG_EPONA)) && self.interact_info.talk_state == NPC_TALK_STATE_ACTION {
            self.action = Action::GiveWeirdEgg;
            play.msg_ctx.state_timer = 4;
            play.msg_ctx.msg_mode = MSGMODE_TEXT_CLOSING;
        }
    }

    /// `EnMa1_GiveWeirdEgg`.
    fn give_weird_egg(&mut self, play: &mut PlayState) {
        if actor_has_parent(&self.actor) {
            self.actor.parent = None;
            self.action = Action::FinishGivingWeirdEgg;
        } else {
            let a = self.actor.clone();
            offer_get_item_range(play, &a, GI_WEIRD_EGG, 120.0, 10.0);
        }
    }

    /// `EnMa1_FinishGivingWeirdEgg`.
    fn finish_giving_weird_egg(&mut self, play: &mut PlayState) {
        if self.interact_info.talk_state == NPC_TALK_STATE_ITEM_GIVEN {
            self.interact_info.talk_state = NPC_TALK_STATE_IDLE;
            self.action = Action::Idle;
            play.save.set_event_chk_inf(EVENTCHKINF_RECEIVED_WEIRD_EGG);
            play.msg_ctx.msg_mode = MSGMODE_TEXT_CLOSING;
        }
    }

    fn player_state2(play: &PlayState) -> u32 {
        play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map(|p| p.state_flags2()).unwrap_or(0)
    }

    fn set_player_state2(play: &mut PlayState, set: u32) {
        if let Some(p) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
            p.change_state_flags2(set, 0);
        }
    }

    /// `EnMa1_IdleTeachSong`: once she may teach the song, Link's ocarina out near her starts it.
    fn idle_teach_song(&mut self, play: &mut PlayState) {
        self.update_anim();
        if play.save.get_event_chk_inf(EVENTCHKINF_CAN_LEARN_EPONAS_SONG) {
            if Self::player_state2(play) & PLAYER_STATE2_24 != 0 {
                Self::set_player_state2(play, PLAYER_STATE2_25);
                if let Some(me) = play.cur_actor
                    && let Some(p) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut())
                {
                    p.set_unk_6a8(Some(me));
                }
                self.actor.text_id = 0x2061;
                play.start_textbox(self.actor.text_id, None);
                self.interact_info.talk_state = NPC_TALK_STATE_TALKING;
                self.actor.flags |= ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
                self.action = Action::StartTeachSong;
            } else if self.actor.xz_dist_to_player < 30.0 + self.collider.dim.radius as f32 {
                Self::set_player_state2(play, PLAYER_STATE2_23);
            }
        }
    }

    /// `EnMa1_StartTeachSong`.
    fn start_teach_song(&mut self, play: &mut PlayState) {
        Self::set_player_state2(play, PLAYER_STATE2_23);
        if self.interact_info.talk_state == NPC_TALK_STATE_ACTION {
            play.audio.audio_ocarina_set_instrument(oot_game::audio::ocarina::OCARINA_INSTRUMENT_MALON);
            play.message_start_ocarina(oot_game::message::OCARINA_ACTION_TEACH_EPONA);
            self.actor.flags &= !ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
            self.action = Action::TeachSong;
        }
    }

    /// `EnMa1_TeachSong`.
    fn teach_song(&mut self, play: &mut PlayState) {
        Self::set_player_state2(play, PLAYER_STATE2_23);
        if play.message_state() == TEXT_STATE_SONG_DEMO_DONE {
            play.message_start_ocarina(oot_game::message::OCARINA_ACTION_PLAYBACK_EPONA);
            self.action = Action::WaitForPlayback;
        }
    }

    /// `EnMa1_WaitForPlayback`: the song played back, to the ranch's cutscene 1.
    fn wait_for_playback(&mut self, play: &mut PlayState) {
        Self::set_player_state2(play, PLAYER_STATE2_23);
        if play.msg_ctx.ocarina_mode == OCARINA_MODE_03 {
            play.transition.next_entrance_index = ENTR_LON_LON_RANCH_0;
            play.save.next_cutscene_index = CS_INDEX_1;
            play.transition.ty = TRANS_TYPE_CIRCLE_WAVE_WHITE_FAST;
            play.transition.trigger = oot_game::transition::TRANS_TRIGGER_START;
            self.action = Action::DoNothing;
        }
    }

    /// The limb matrices `EnMa1_Draw` draws with: `EnMa1_OverrideLimbDraw`'s head and chest turns.
    fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], head: [i16; 2], torso: [i16; 2]) -> Vec<Mat4> {
        // BINANG_TO_RAD_ALT.
        let r = binang_to_rad;
        skeleton.pose_override(joints, |limb, _pos, _rot| {
            if limb == CHILD_MALON_LIMB_HEAD {
                // Translate(1400, 0, 0), RotateX(headRot.y), RotateZ(headRot.x), Translate(-1400, 0, 0).
                Mat4::from_translation(Vec3::new(1400.0, 0.0, 0.0)) * Mat4::from_rotation_x(r(head[1])) * Mat4::from_rotation_z(r(head[0])) * Mat4::from_translation(Vec3::new(-1400.0, 0.0, 0.0))
            } else if limb == CHILD_MALON_LIMB_CHEST {
                // RotateX(-torsoRot.y), RotateZ(-torsoRot.x).
                Mat4::from_rotation_x(r(torso[1].wrapping_neg())) * Mat4::from_rotation_z(r(torso[0].wrapping_neg()))
            } else {
                Mat4::IDENTITY
            }
        })
    }
}

impl ActorImpl for EnMa1 {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnMa1_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);

        self.skel.update();
        self.update_eyes(play);

        match self.action {
            Action::Idle => self.idle(play),
            Action::GiveWeirdEgg => self.give_weird_egg(play),
            Action::FinishGivingWeirdEgg => self.finish_giving_weird_egg(play),
            Action::IdleTeachSong => self.idle_teach_song(play),
            Action::StartTeachSong => self.start_teach_song(play),
            Action::TeachSong => self.teach_song(play),
            Action::WaitForPlayback => self.wait_for_playback(play),
            Action::DoNothing => {}
        }

        if self.action != Action::DoNothing {
            // Npc_UpdateTalking(.., collider.dim.radius + 30, EnMa1_GetTextId, EnMa1_UpdateTalkState).
            let mut talk_state = self.interact_info.talk_state;
            let range = self.collider.dim.radius as f32 + 30.0;
            talk_update(play, &mut self.actor, &mut talk_state, range, |play, _| Self::get_text_id(play), Self::update_talk_state);
            self.interact_info.talk_state = talk_state;
        }

        self.update_singing(play);
        self.update_tracking(play);
    }
    /// `EnMa1_Draw`'s `Audio_UpdateMalonSinging` (her voice by the camera's distance), and
    /// `EnMa1_PostLimbDraw`, the head: the focus 800 along it.
    fn draw_update(&mut self, play: &mut PlayState) {
        let eye = play.active_camera().eye;
        let dist_from_cam_eye = Vec3::new(self.actor.world_pos.x - eye.x, 0.0, self.actor.world_pos.z - eye.z).length();
        play.audio.update_malon_singing(dist_from_cam_eye, NA_BGM_LONLON);
        let t = &self.interact_info;
        let bones = Self::pose(&self.skeleton, &self.skel.joint_table, [t.head[0], t.head[1]], [t.torso[0], t.torso[1]]);
        if let Some(head) = bones.get(CHILD_MALON_LIMB_HEAD - 1) {
            let m = oot_game::play::actor_draw_matrix(&RenderState::of(&self.actor));
            self.actor.focus_pos = (m * *head).transform_point3(Vec3::new(800.0, 0.0, 0.0));
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        let t = &self.interact_info;
        rs.angles = vec![t.head[0], t.head[1], t.torso[0], t.torso[1]];
        rs.switches = vec![self.eye_index.clamp(0, 2) as u32];
        rs
    }
    /// `EnMa1_Draw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        if rs.angles.len() < 4 || rs.switches.is_empty() {
            return;
        }
        let a = &rs.angles;
        let bones = Self::pose(&self.skeleton, &joints.rot, [a[0], a[1]], [a[2], a[3]]);
        let eye = (rs.switches[0] as usize).min(EYE_TEXTURES.len() - 1);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(eye))), transform: oot_game::play::actor_draw_matrix(rs), bones, params: Default::default() });
        // ActorShadow_DrawCircle (shadowScale 18): the circle shadow's stand-in.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_damage_table_hurts_nothing() {
        assert!(S_DAMAGE_TABLE_PRESET_22.table.iter().all(|&e| e == 0));
    }

    #[test]
    fn the_circle_transition_is_wave_white_fast() {
        // TRANS_TYPE_CIRCLE(1, 1, 0) = 0x20 | 0x08 | 0x02.
        assert_eq!(TRANS_TYPE_CIRCLE_WAVE_WHITE_FAST, 0x2A);
    }
}
