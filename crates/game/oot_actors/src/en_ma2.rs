//! `En_Ma2` (`ovl_En_Ma2/z_en_ma2.c`): adult Malon, in the stable and the ranch.
//!
//! Where she is (`EnMa2_GetSpawnIndex`, by Link's age, the scene, the time, `shape.rot.z` and
//! Epona): never for a child (`MALON_ADULT_SPAWN_NONE`: killed); for an adult before Epona, in the
//! stable by day (rot.z 5, `MALON_ADULT_SPAWN_DAY_STABLES_NO_EPONA`: standing with her basket) and
//! in the ranch by night (rot.z 6, `_NIGHT_RANCH_NO_EPONA`: singing, waiting for Epona's Song on
//! the ocarina); after Epona, in the ranch by day (rot.z 7) or by night (rot.z 8)
//! (`MALON_ADULT_SPAWN_ALWAYS`: singing until `INFTABLE_8D`, then idle); anything else is killed.
//! No placement uses rot.z 7: the ranch by day after Epona is `En_Ma3`'s.
//!
//! She talks (`EnMa2_GetTextId`: a mask's text, `MASK_REACTION_SET_MALON`; 0x2056 after Epona; by
//! night 0x2050, 0x2051 once Epona's Song is played to her, 0x2052 after; by day 0x204C). By night
//! before Epona (`EnMa2_WaitForOcarina`), Link within 48 of her (`30 + collider.dim.radius`) may
//! take out his ocarina (`PLAYER_STATE2_23`); out (`PLAYER_STATE2_24`) she listens for Epona's Song
//! (`OCARINA_ACTION_CHECK_EPONA`, `EnMa2_WaitForEponasSong`); played (`OCARINA_MODE_03`): the
//! chime, `INFTABLE_PLAYED_SONG_FOR_ADULT_MALON`, and 30 frames on she talks at once
//! (`EnMa2_ForceTalkAfterSong`: `ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED`), 0x2051; its closing sets
//! `INFTABLE_TALKED_TO_ADULT_MALON_AFTER_SONG` (`EnMa2_WaitToEndTalk` takes the auto-accept off).
//! Another song (`OCARINA_MODE_04` and up): back to waiting.
//!
//! She sings (`gMalonAdultSingAnim`, the Lon Lon song's voice through `Audio_ToggleMalonSinging`,
//! off while she talks, and `Audio_UpdateMalonSinging` by her distance to the camera), her eyes
//! closing and her mouth turning happy as she does (`EnMa2_IsSinging`); her head and chest turn to
//! Link while she isn't singing (`EnMa2_UpdateTracking`).
//!
//! Drawn as `EnMa2_Draw` does: her eyes on segment 8, her mouth on segment 9, her thighs not drawn,
//! her head and chest turned to Link, the chest and shoulders' `upperBodyRot` sway (never written:
//! a constant 200 on each one's z), and her basket in her left hand with `gMalonAdultStandStillAnim`
//! (`EnMa2_PostLimbDraw`); from meshes baked per eye and mouth (docs/adr/0012-actor-bakes.md).
//! `En_Ma3` draws the same skeleton the same way: its model (the bakes, the pose and the draw) is
//! this module's ([`MalonAdultModel`], [`bakes`], [`draw_malon_adult`]).
//!
//! Params: `params` unused (placed -1, read by nothing). `shape.rot.z`: 5 the stable's Malon by day
//! before Epona; 6 the ranch's by night before Epona; 7 and 8 the ranch's by day and by night after.
//! Flags: reads EVENTCHKINF_EPONA_OBTAINED (where she is, her texts); reads INFTABLE_8D
//! (`MALON_ADULT_SPAWN_ALWAYS` sings until it's set) and sets it for text 0x2053, which she never
//! shows (a Gossip Stone's text: unreachable); reads and sets INFTABLE_PLAYED_SONG_FOR_ADULT_MALON
//! (Epona's Song played to her) and INFTABLE_TALKED_TO_ADULT_MALON_AFTER_SONG (text 0x2051
//! closed).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{binang_to_rad, cos_s, sin_s};
use glam::{Mat4, Vec3};
use oot_game::actor::{
    ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_UPDATE_DURING_OCARINA,
    Actor, UPDBGCHECKINFO_FLAG_2,
};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind, Source, TextLine};
use oot_game::audio::NA_BGM_LONLON;
use oot_game::audio::sfx::NA_SE_SY_CORRECT_CHIME;
use oot_game::collision_check::*;
use oot_game::message::{OCARINA_ACTION_CHECK_EPONA, OCARINA_MODE_03, OCARINA_MODE_04, TEXT_STATE_CLOSING};
use oot_game::npc::{NpcTrack, talk_update, track_point};
use oot_game::pack::{BakeBody, BakeSegment, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::*;
use oot_game::{event_chk_inf, inf_table};

use crate::player::{STATE2_23, STATE2_24, STATE2_25};

/// `ACTOR_EN_MA2` (`actor_table.h`: 0x00D9).
pub const ACTOR_EN_MA2: i16 = 0x00D9;
pub const OBJECT: &str = "object_ma2";

/// `En_Ma2_Profile`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_MA2,
    name: "En_Ma2",
    category: ACTORCAT_NPC,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `this->actor.params`: placed -1; nothing reads it.
pub const PARAMS: Field = Field::whole("params", Kind::Unused, "placed -1; read by nothing");
/// `this->actor.shape.rot.z` (`EnMa2_GetSpawnIndex`): which Malon.
pub const ROT_Z: Field = Field::rot(
    "shape.rot.z",
    Source::RotZ,
    0,
    16,
    Kind::Named(&[
        (5, "the stable by day, before Epona"),
        (6, "the ranch by night, before Epona (Epona's Song)"),
        (7, "the ranch by day, after Epona"),
        (8, "the ranch by night, after Epona"),
    ]),
    "which adult Malon, by the scene, the time and Epona",
);

// `EVENTCHKINF_*`, `INFTABLE_*` (`save.h`).
pub const EVENTCHKINF_EPONA_OBTAINED: u16 = 0x18;
pub const INFTABLE_TALKED_TO_ADULT_MALON_AFTER_SONG: u16 = 0x8C;
/// "unreachable flag in EnMa2" (`save.h`).
pub const INFTABLE_8D: u16 = 0x8D;
pub const INFTABLE_PLAYED_SONG_FOR_ADULT_MALON: u16 = 0x8E;

/// En_Ma2's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[PARAMS, ROT_Z],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_EPONA_OBTAINED)).reads().note("where she is; her texts"),
        FlagUse::new(inf_table!(INFTABLE_8D)).reads().sets().note("MALON_ADULT_SPAWN_ALWAYS sings until set; set by text 0x2053, which she never shows (unreachable)"),
        FlagUse::new(inf_table!(INFTABLE_PLAYED_SONG_FOR_ADULT_MALON)).reads().sets().note("Epona's Song played to her"),
        FlagUse::new(inf_table!(INFTABLE_TALKED_TO_ADULT_MALON_AFTER_SONG)).reads().sets().note("text 0x2051 closed"),
    ],
    todo: false,
    objects: &[],
    texts: &[
        TextLine::new(0x204C).when("shape.rot.z", &[5]).cond("an adult before Epona, by day (the stable)").note("a visitor; since Ganondorf came Mr. Ingo kicked Dad out"),
        TextLine::new(0x2050).when("shape.rot.z", &[6]).cond("an adult before Epona, by night, before Epona's Song").note("keep it secret from Mr. Ingo that I sing this song"),
        TextLine::new(0x2051).when("shape.rot.z", &[6]).cond("Epona's Song played to her (she talks at once); runs on into 0x2052").note("do you know my mother's song?"),
        TextLine::new(0x2052).when("shape.rot.z", &[6]).cond("after 0x2051, and on from it").note("Epona really liked that song"),
        TextLine::new(0x2056).when("shape.rot.z", &[7, 8]).cond("after Epona").note("Mr. Ingo must have been tempted by evil powers"),
        TextLine::new(0x7174).cond("Link wearing the Spooky Mask, Bunny Hood, Zora Mask or Gerudo Mask").note("hi, fairy boy! you think you're in disguise?"),
        TextLine::new(0x7175).cond("Link wearing the Goron Mask").note("what big eyes! Gorons are so cute"),
        TextLine::new(0x7176).cond("Link wearing the Keaton Mask").note("it's Keaton!"),
        TextLine::new(0x7177).cond("Link wearing the Skull Mask or Mask of Truth").note("oh, brother! boys will be boys"),
    ],
};

// `SCENE_*` (`scene_table.h`).
const SCENE_STABLE: u16 = 0x36;
const SCENE_LON_LON_RANCH: u16 = 0x63;

// `EnMa2SpawnIndex`.
pub const MALON_ADULT_SPAWN_NONE: u16 = 0;
pub const MALON_ADULT_SPAWN_DAY_STABLES_NO_EPONA: u16 = 1;
pub const MALON_ADULT_SPAWN_NIGHT_RANCH_NO_EPONA: u16 = 2;
pub const MALON_ADULT_SPAWN_ALWAYS: u16 = 3;

/// `MASK_REACTION_SET_MALON` (`face_reaction.h`: 0x17).
const MASK_REACTION_SET_MALON: usize = 0x17;
/// `sMaskReactionSetTextIds[MASK_REACTION_SET_MALON]` (`z_face_reaction.c`), by `PLAYER_MASK_*`
/// (none, Keaton, Skull, Spooky, Bunny, Goron, Zora, Gerudo, Truth).
const MASK_REACTION_MALON: [u16; 9] = [0x0000, 0x7176, 0x7177, 0x7174, 0x7174, 0x7175, 0x7174, 0x7174, 0x7177];

/// `ATTENTION_RANGE_6` (`actor.h`).
const ATTENTION_RANGE_6: u8 = 6;
/// `NpcTrackingMode` (`actor.h`).
pub const NPC_TRACKING_PLAYER_AUTO_TURN: i16 = 0;
pub const NPC_TRACKING_NONE: i16 = 1;
/// `NpcTalkState` (`actor.h`).
pub const NPC_TALK_STATE_IDLE: i16 = 0;
pub const NPC_TALK_STATE_TALKING: i16 = 1;
pub const NPC_TALK_STATE_ACTION: i16 = 2;

// The adult Malon's model, `En_Ma2`'s and `En_Ma3`'s (`object_ma2`, `gMalonAdultSkel`).

// `MalonAdultLimb` (`object_ma2.xml`), 1-based as the limb draws see them.
pub const MALON_ADULT_LIMB_LEFT_THIGH: usize = 3;
pub const MALON_ADULT_LIMB_RIGHT_THIGH: usize = 6;
pub const MALON_ADULT_LIMB_CHEST_AND_NECK: usize = 11;
pub const MALON_ADULT_LIMB_LEFT_SHOULDER: usize = 12;
pub const MALON_ADULT_LIMB_LEFT_HAND: usize = 14;
pub const MALON_ADULT_LIMB_RIGHT_SHOULDER: usize = 15;
pub const MALON_ADULT_LIMB_HEAD: usize = 18;
pub const MALON_ADULT_LIMB_MAX: usize = 19;

/// `sCylinderInit` (the same in `z_en_ma2.c` and `z_en_ma3.c`).
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
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

/// `sColChkInfoInit`: immovable (the same in both).
pub const COL_CHK_INFO_INIT: CollisionCheckInfoInit2 = CollisionCheckInfoInit2 { health: 0, cyl_radius: 0, cyl_height: 0, cyl_y_shift: 0, mass: MASS_IMMOVABLE };

// `EnMa2Animation` (`EnMa3Animation`'s `ENMA3_ANIM_0` to `_4` are the same): indices into
// `sAnimationInfo`.
pub const MALON_ADULT_ANIM_IDLE: usize = 0;
pub const MALON_ADULT_ANIM_IDLE_NOMORPH: usize = 1;
pub const MALON_ADULT_ANIM_STANDING: usize = 2;
pub const MALON_ADULT_ANIM_SING: usize = 3;
pub const MALON_ADULT_ANIM_SING_NOMORPH: usize = 4;

pub const IDLE_ANIM: &str = "gMalonAdultIdleAnim";
pub const STAND_STILL_ANIM: &str = "gMalonAdultStandStillAnim";
pub const SING_ANIM: &str = "gMalonAdultSingAnim";
/// `sAnimationInfo` (the same in both): the animation and the morph; all play at 1, looped. (The
/// enum's names have the morphs backwards: `_NOMORPH` is the one with -10.)
const ANIMATION_INFO: [(&str, f32); 5] = [(IDLE_ANIM, 0.0), (IDLE_ANIM, -10.0), (STAND_STILL_ANIM, 0.0), (SING_ANIM, 0.0), (SING_ANIM, -10.0)];

/// `sEyeTextures`.
pub const EYE_TEXTURES: [&str; 3] = ["gMalonAdultEyeOpenTex", "gMalonAdultEyeHalfTex", "gMalonAdultEyeClosedTex"];
/// `sMouthTextures`. `mouthIndex` is only ever 0 or 2 (`EnMa2_IsSinging`, `func_80AA2F28`): the sad
/// mouth (1) is never drawn, and isn't baked.
pub const MOUTH_TEXTURES: [&str; 3] = ["gMalonAdultMouthNeutralTex", "gMalonAdultMouthSadTex", "gMalonAdultMouthHappyTex"];
/// The mouths baked: neutral and happy.
const MOUTHS_BAKED: [usize; 2] = [0, 2];
/// The basket `EnMa2_PostLimbDraw` puts in her left hand.
pub const BASKET_BAKE: &str = "En_Ma2/basket";

/// The bake drawn with eye `eye` and mouth `mouth` (0 or 2).
pub fn bake_name(eye: usize, mouth: usize) -> String {
    format!("En_Ma2/eye{eye}_mouth{mouth}")
}

/// The meshes `EnMa2_Draw` and `EnMa3_Draw` draw: the skeleton with each eye and mouth, no thighs;
/// and the basket.
pub fn bakes() -> Vec<MeshBake> {
    let mut v = Vec::new();
    for eye in 0..EYE_TEXTURES.len() {
        for mouth in MOUTHS_BAKED {
            v.push(MeshBake {
                name: bake_name(eye, mouth),
                object: OBJECT.into(),
                segments: vec![(0x08, BakeSegment::Texture { file: OBJECT.into(), symbol: EYE_TEXTURES[eye].into() }), (0x09, BakeSegment::Texture { file: OBJECT.into(), symbol: MOUTH_TEXTURES[mouth].into() })],
                prelude: Vec::new(),
                body: BakeBody::Skeleton {
                    file: OBJECT.into(),
                    symbol: "gMalonAdultSkel".into(),
                    // EnMa2_OverrideLimbDraw: *dList = NULL for the thighs.
                    limbs: [MALON_ADULT_LIMB_LEFT_THIGH, MALON_ADULT_LIMB_RIGHT_THIGH].map(|l| LimbOverride { limb: (l - 1) as u8, file: OBJECT.into(), symbol: String::new() }).to_vec(),
                },
            });
        }
    }
    v.push(MeshBake { name: BASKET_BAKE.into(), object: OBJECT.into(), segments: Vec::new(), prelude: Vec::new(), body: BakeBody::DLists(vec![(OBJECT.into(), "gMalonAdultBasketDL".into())]) });
    v
}

/// `gMalonAdultSkel` and its three animations, loaded once per actor.
pub struct MalonAdultModel {
    pub skeleton: Arc<Skeleton>,
    idle: Anim,
    stand_still: Anim,
    sing: Anim,
}

impl MalonAdultModel {
    /// `SkelAnime_InitFlex(.., &gMalonAdultSkel, NULL, ..)`'s skeleton and the animations;
    /// None (logged) without them.
    pub fn load(play: &PlayState, who: &str) -> Option<MalonAdultModel> {
        let assets = play.assets.clone()?;
        let loaded = (|| -> anyhow::Result<_> {
            Ok(MalonAdultModel { skeleton: assets.skeleton(OBJECT, "gMalonAdultSkel")?, idle: assets.animation(OBJECT, IDLE_ANIM)?, stand_still: assets.animation(OBJECT, STAND_STILL_ANIM)?, sing: assets.animation(OBJECT, SING_ANIM)? })
        })();
        match loaded {
            Ok(m) => Some(m),
            Err(e) => {
                log::error!("{who}: {e:#}");
                None
            }
        }
    }

    /// `EnMa2_ChangeAnim` / `EnMa3_ChangeAnim`: `sAnimationInfo[index]` from 0 to its last frame.
    pub fn change_anim(&self, skel: &mut SkelAnimeStd, index: usize) {
        let (name, morph) = ANIMATION_INFO[index];
        let anim = match name {
            IDLE_ANIM => self.idle.clone(),
            STAND_STILL_ANIM => self.stand_still.clone(),
            _ => self.sing.clone(),
        };
        let frame_count = anim.last_frame();
        skel.change(anim, 1.0, 0.0, frame_count, ANIMMODE_LOOP, morph);
    }

    /// The limb matrices the draw draws with: `EnMa2_OverrideLimbDraw` / `EnMa3_OverrideLimbDraw`.
    /// `sway` is `upperBodyRot` / `unk_212`'s (y, z) for the chest, the left and right shoulders.
    pub fn pose(&self, joints: &[[i16; 3]], head: [i16; 2], torso: [i16; 2], sway: &[[i16; 2]; 3]) -> Vec<Mat4> {
        // BINANG_TO_RAD_ALT.
        let r = binang_to_rad;
        self.skeleton.pose_override(joints, |limb, _pos, rot| {
            let mut pre = Mat4::IDENTITY;
            if limb == MALON_ADULT_LIMB_HEAD {
                // Translate(1400, 0, 0), RotateX(headRot.y), RotateZ(headRot.x), Translate(-1400, 0, 0).
                pre = Mat4::from_translation(Vec3::new(1400.0, 0.0, 0.0)) * Mat4::from_rotation_x(r(head[1])) * Mat4::from_rotation_z(r(head[0])) * Mat4::from_translation(Vec3::new(-1400.0, 0.0, 0.0));
            }
            if limb == MALON_ADULT_LIMB_CHEST_AND_NECK {
                // RotateY(-torsoRot.y), RotateX(-torsoRot.x).
                pre = Mat4::from_rotation_y(r(torso[1].wrapping_neg())) * Mat4::from_rotation_x(r(torso[0].wrapping_neg()));
            }
            let i = [MALON_ADULT_LIMB_CHEST_AND_NECK, MALON_ADULT_LIMB_LEFT_SHOULDER, MALON_ADULT_LIMB_RIGHT_SHOULDER].iter().position(|&l| l == limb);
            if let Some(i) = i {
                // rot->y += Math_SinS(rot[limb].y) * 200; rot->z += Math_CosS(rot[limb].z) * 200.
                rot[1] = (rot[1] as f32 + sin_s(sway[i][0]) * 200.0) as i32 as i16;
                rot[2] = (rot[2] as f32 + cos_s(sway[i][1]) * 200.0) as i32 as i16;
            }
            pre
        })
    }
}

/// The chest's and shoulders' (y, z) from an `upperBodyRot` / `unk_212` table.
pub fn sway_of(rot: &[[i16; 3]; MALON_ADULT_LIMB_MAX]) -> [[i16; 2]; 3] {
    [MALON_ADULT_LIMB_CHEST_AND_NECK, MALON_ADULT_LIMB_LEFT_SHOULDER, MALON_ADULT_LIMB_RIGHT_SHOULDER].map(|l| [rot[l][1], rot[l][2]])
}

/// Indices into the render state's extras.
mod rs {
    /// `angles`: head x, y; torso x, y; the sway (chest, left and right shoulders: y, z each).
    pub const HEAD: usize = 0;
    pub const TORSO: usize = 2;
    pub const SWAY: usize = 4;
    pub const ANGLES: usize = 10;
    /// `switches`: the eye, the mouth, the basket.
    pub const EYE: usize = 0;
    pub const MOUTH: usize = 1;
    pub const BASKET: usize = 2;
    pub const SWITCHES: usize = 3;
}

/// What the draw needs of one adult Malon.
pub struct MalonAdultLook<'a> {
    pub skel: &'a SkelAnimeStd,
    pub interact_info: &'a NpcTrack,
    pub upper_body_rot: &'a [[i16; 3]; MALON_ADULT_LIMB_MAX],
    pub eye_index: i16,
    pub mouth_index: i16,
}

/// `EnMa2_Draw`'s / `EnMa3_Draw`'s `Audio_UpdateMalonSinging` (her voice by the active camera's
/// eye, on x and z, `NA_BGM_LONLON`), and the post-limb draw's head: the focus 900 along it.
pub fn draw_update_malon_adult(play: &mut PlayState, actor: &mut Actor, model: &MalonAdultModel, look: &MalonAdultLook) {
    let eye = play.active_camera().eye;
    let dist_from_cam_eye = Vec3::new(actor.world_pos.x - eye.x, 0.0, actor.world_pos.z - eye.z).length();
    play.audio.update_malon_singing(dist_from_cam_eye, NA_BGM_LONLON);
    let t = look.interact_info;
    let bones = model.pose(&look.skel.joint_table, [t.head[0], t.head[1]], [t.torso[0], t.torso[1]], &sway_of(look.upper_body_rot));
    if let Some(head) = bones.get(MALON_ADULT_LIMB_HEAD - 1) {
        let m = oot_game::play::actor_draw_matrix(&RenderState::of(actor));
        actor.focus_pos = (m * *head).transform_point3(Vec3::new(900.0, 0.0, 0.0));
    }
}

/// The render state `draw_malon_adult` draws from.
pub fn render_state_malon_adult(actor: &Actor, look: &MalonAdultLook) -> RenderState {
    let mut s = RenderState::of(actor);
    s.joints = Some(eng_anim::anim::JointTable { rot: look.skel.joint_table.clone(), face: 0 });
    let t = look.interact_info;
    let sway = sway_of(look.upper_body_rot);
    s.angles = vec![t.head[0], t.head[1], t.torso[0], t.torso[1], sway[0][0], sway[0][1], sway[1][0], sway[1][1], sway[2][0], sway[2][1]];
    // EnMa2_PostLimbDraw: the basket with gMalonAdultStandStillAnim.
    let basket = look.skel.is(STAND_STILL_ANIM);
    s.switches = vec![look.eye_index.clamp(0, 2) as u32, look.mouth_index.clamp(0, 2) as u32, basket as u32];
    s
}

/// `EnMa2_Draw` / `EnMa3_Draw`: the skeleton (eyes on segment 8, the mouth on 9) and the basket in
/// the left hand.
pub fn draw_malon_adult(model: &MalonAdultModel, s: &RenderState, out: &mut DrawOut) {
    let Some(joints) = &s.joints else { return };
    if s.angles.len() < rs::ANGLES || s.switches.len() < rs::SWITCHES {
        return;
    }
    let a = &s.angles;
    let sway = [[a[rs::SWAY], a[rs::SWAY + 1]], [a[rs::SWAY + 2], a[rs::SWAY + 3]], [a[rs::SWAY + 4], a[rs::SWAY + 5]]];
    let bones = model.pose(&joints.rot, [a[rs::HEAD], a[rs::HEAD + 1]], [a[rs::TORSO], a[rs::TORSO + 1]], &sway);
    let m = oot_game::play::actor_draw_matrix(s);
    if s.switches[rs::BASKET] != 0
        && let Some(b) = bones.get(MALON_ADULT_LIMB_LEFT_HAND - 1)
    {
        out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(BASKET_BAKE)), m * *b));
    }
    let eye = (s.switches[rs::EYE] as usize).min(EYE_TEXTURES.len() - 1);
    // The sad mouth (1) is never chosen; drawn as the neutral one if it were.
    let mouth = if s.switches[rs::MOUTH] == 2 { 2 } else { 0 };
    out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(eye, mouth))), transform: m, bones, params: Default::default() });
}

/// `DECR(x)`: `x` less one unless it's 0; the new value.
pub fn decr(x: &mut i16) -> i16 {
    if *x != 0 {
        *x -= 1;
    }
    *x
}

pub(crate) fn player_state1(play: &PlayState) -> u32 {
    play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map_or(0, |p| p.state_flags1())
}

fn player_state2(play: &PlayState) -> u32 {
    play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map_or(0, |p| p.state_flags2())
}

/// `player->stateFlags2 |= set`.
fn set_player_state2(play: &mut PlayState, set: u32) {
    if let Some(p) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
        p.change_state_flags2(set, 0);
    }
}

// shared: MaskReaction_GetTextId
/// `MaskReaction_GetTextId` (`z_face_reaction.c`): `sMaskReactionSetTextIds[set][Player_GetMask]`,
/// here only Malon's row (`MASK_REACTION_SET_MALON`); 0 without a mask.
fn mask_reaction_get_text_id(play: &PlayState, set: usize) -> u16 {
    let mask = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map_or(0, |p| p.current_mask()) as usize;
    match set {
        MASK_REACTION_SET_MALON => MASK_REACTION_MALON.get(mask).copied().unwrap_or(0),
        _ => 0,
    }
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    WaitToEndTalk,
    WaitForOcarina,
    WaitForEponasSong,
    ForceTalkAfterSong,
}

pub struct EnMa2 {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub model: Option<MalonAdultModel>,
    pub action: Action,
    pub collider: ColliderCylinder,
    pub interact_info: NpcTrack,
    pub timer: i16,
    pub singing_disabled: bool,
    pub blink_timer: i16,
    pub eye_index: i16,
    pub mouth_index: i16,
    /// `upperBodyRot`: never written (zeros).
    pub upper_body_rot: [[i16; 3]; MALON_ADULT_LIMB_MAX],
}

impl EnMa2 {
    /// `EnMa2_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(model) = MalonAdultModel::load(play, PROFILE.name) else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let mut this = EnMa2 {
            // SkelAnime_InitFlex(.., &gMalonAdultSkel, NULL, ..).
            skel: SkelAnimeStd::init_flex(model.skeleton.limbs.len(), None),
            model: Some(model),
            action: Action::WaitToEndTalk,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            interact_info: NpcTrack::default(),
            timer: 0,
            singing_disabled: false,
            blink_timer: 0,
            eye_index: 0,
            mouth_index: 0,
            upper_body_rot: [[0; 3]; MALON_ADULT_LIMB_MAX],
            actor,
        };
        this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 18.0);
        // DamageTable_Get(22).
        this.actor.col_chk_info.set_info2(Some(&crate::en_ma1::S_DAMAGE_TABLE_PRESET_22), &COL_CHK_INFO_INIT);

        match Self::get_spawn_index(&this.actor, play) {
            MALON_ADULT_SPAWN_DAY_STABLES_NO_EPONA => {
                this.change_anim(MALON_ADULT_ANIM_STANDING);
                this.action = Action::WaitToEndTalk;
            }
            MALON_ADULT_SPAWN_NIGHT_RANCH_NO_EPONA => {
                this.change_anim(MALON_ADULT_ANIM_SING);
                this.action = Action::WaitForOcarina;
            }
            MALON_ADULT_SPAWN_ALWAYS => {
                if play.save.get_inf_table(INFTABLE_8D) {
                    this.change_anim(MALON_ADULT_ANIM_IDLE);
                } else {
                    this.change_anim(MALON_ADULT_ANIM_SING);
                }
                this.action = Action::WaitToEndTalk;
            }
            _ => {
                this.actor.kill();
                return Box::new(this);
            }
        }

        this.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
        this.actor.scale = Vec3::splat(0.01);
        this.actor.target_mode = ATTENTION_RANGE_6;
        this.interact_info.talk_state = NPC_TALK_STATE_IDLE;
        Box::new(this)
    }

    /// `EnMa2_GetTextId`.
    pub fn get_text_id(play: &PlayState) -> u16 {
        let text_id = mask_reaction_get_text_id(play, MASK_REACTION_SET_MALON);
        if text_id != 0 {
            return text_id;
        }
        let s = &play.save;
        if s.get_event_chk_inf(EVENTCHKINF_EPONA_OBTAINED) {
            return 0x2056;
        }
        // IS_NIGHT.
        if !s.is_day() {
            if s.get_inf_table(INFTABLE_TALKED_TO_ADULT_MALON_AFTER_SONG) {
                return 0x2052;
            } else if s.get_inf_table(INFTABLE_PLAYED_SONG_FOR_ADULT_MALON) {
                return 0x2051;
            } else {
                return 0x2050;
            }
        }
        0x204C
    }

    /// `EnMa2_UpdateTalkState`.
    fn update_talk_state(play: &mut PlayState, actor: &mut Actor) -> i16 {
        if play.message_state() != TEXT_STATE_CLOSING {
            return NPC_TALK_STATE_TALKING;
        }
        match actor.text_id {
            0x2051 => {
                play.save.set_inf_table(INFTABLE_TALKED_TO_ADULT_MALON_AFTER_SONG);
                NPC_TALK_STATE_ACTION
            }
            // Unreachable, 0x2053 is a text id for Gossip Stones, which EnMa2 never sets.
            // schema: compared, never shown (the C's unreachable case).
            0x2053 => {
                play.save.set_inf_table(INFTABLE_8D);
                NPC_TALK_STATE_IDLE
            }
            _ => NPC_TALK_STATE_IDLE,
        }
    }

    /// `EnMa2_GetSpawnIndex`.
    pub fn get_spawn_index(actor: &Actor, play: &PlayState) -> u16 {
        let s = &play.save;
        if !s.adult {
            return MALON_ADULT_SPAWN_NONE;
        }
        let epona = s.get_event_chk_inf(EVENTCHKINF_EPONA_OBTAINED);
        let rot_z = ROT_Z.get(actor.shape_rot.z);
        if !epona && play.scene_id == SCENE_STABLE && s.is_day() && rot_z == 5 {
            return MALON_ADULT_SPAWN_DAY_STABLES_NO_EPONA;
        }
        if !epona && play.scene_id == SCENE_LON_LON_RANCH && !s.is_day() && rot_z == 6 {
            return MALON_ADULT_SPAWN_NIGHT_RANCH_NO_EPONA;
        }
        if !epona || play.scene_id != SCENE_LON_LON_RANCH {
            return MALON_ADULT_SPAWN_NONE;
        }
        if rot_z == 7 && s.is_day() {
            return MALON_ADULT_SPAWN_ALWAYS;
        }
        if rot_z == 8 && !s.is_day() {
            return MALON_ADULT_SPAWN_ALWAYS;
        }
        MALON_ADULT_SPAWN_NONE
    }

    fn is_sing_anim(&self) -> bool {
        self.skel.is(SING_ANIM)
    }

    /// `EnMa2_UpdateTracking`: Link's position; none while she sings and isn't talking.
    fn update_tracking(&mut self, play: &mut PlayState) {
        let tracking_mode = if self.interact_info.talk_state == NPC_TALK_STATE_IDLE && self.is_sing_anim() { NPC_TRACKING_NONE } else { NPC_TRACKING_PLAYER_AUTO_TURN };
        self.interact_info.target = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(self.actor.world_pos);
        self.interact_info.eye_height = 0.0;
        let mut t = self.interact_info;
        track_point(play, &mut self.actor, &mut t, 0, tracking_mode);
        self.interact_info = t;
    }

    /// `EnMa2_IsSinging`: singing and not talking, her blink held and, once her eyes are closed,
    /// her mouth happy.
    fn is_singing(&mut self) -> bool {
        if !self.is_sing_anim() {
            return false;
        }
        if self.interact_info.talk_state != NPC_TALK_STATE_IDLE {
            return false;
        }
        self.blink_timer = 0;
        if self.eye_index != 2 {
            return false;
        }
        self.mouth_index = 2;
        true
    }

    /// `EnMa2_UpdateEyes`.
    fn update_eyes(&mut self, play: &mut PlayState) {
        if !self.is_singing() && decr(&mut self.blink_timer) == 0 {
            self.eye_index += 1;
            if self.eye_index >= 3 {
                self.blink_timer = play.rand.s16_offset(30, 30);
                self.eye_index = 0;
            }
        }
    }

    /// `EnMa2_ChangeAnim`.
    fn change_anim(&mut self, index: usize) {
        if let Some(m) = &self.model {
            m.change_anim(&mut self.skel, index);
        }
    }

    /// `EnMa2_UpdateSinging`: her voice off while she talks, on again after.
    fn update_singing(&mut self, play: &mut PlayState) {
        if self.is_sing_anim() {
            if self.interact_info.talk_state == NPC_TALK_STATE_IDLE {
                if self.singing_disabled {
                    // Turn on singing.
                    play.audio.toggle_malon_singing(false);
                    self.singing_disabled = false;
                }
            } else if !self.singing_disabled {
                // Turn off singing.
                play.audio.toggle_malon_singing(true);
                self.singing_disabled = true;
            }
        }
    }

    /// `EnMa2_WaitToEndTalk`.
    fn wait_to_end_talk(&mut self, _play: &mut PlayState) {
        if self.interact_info.talk_state == NPC_TALK_STATE_ACTION {
            self.actor.flags &= !ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
            self.interact_info.talk_state = NPC_TALK_STATE_IDLE;
        }
    }

    /// `EnMa2_WaitForOcarina`: the ocarina out, she listens for Epona's Song; Link near her may take
    /// it out.
    fn wait_for_ocarina(&mut self, play: &mut PlayState) {
        if player_state2(play) & STATE2_24 != 0 {
            set_player_state2(play, STATE2_25);
            if let Some(me) = play.cur_actor
                && let Some(p) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut())
            {
                p.set_unk_6a8(Some(me));
            }
            play.message_start_ocarina(OCARINA_ACTION_CHECK_EPONA);
            self.action = Action::WaitForEponasSong;
        } else if self.actor.xz_dist_to_player < 30.0 + self.collider.dim.radius as f32 {
            set_player_state2(play, STATE2_23);
        }
    }

    /// `EnMa2_WaitForEponasSong`.
    fn wait_for_eponas_song(&mut self, play: &mut PlayState) {
        if play.msg_ctx.ocarina_mode >= OCARINA_MODE_04 {
            self.action = Action::WaitForOcarina;
            play.msg_ctx.ocarina_mode = OCARINA_MODE_04;
        } else if play.msg_ctx.ocarina_mode == OCARINA_MODE_03 {
            // SFX_PLAY_CENTERED.
            play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
            self.timer = 0x1E;
            play.save.set_inf_table(INFTABLE_PLAYED_SONG_FOR_ADULT_MALON);
            self.action = Action::ForceTalkAfterSong;
            play.msg_ctx.ocarina_mode = OCARINA_MODE_04;
        } else {
            set_player_state2(play, STATE2_23);
        }
    }

    /// `EnMa2_ForceTalkAfterSong`: 30 frames, then she talks at once.
    fn force_talk_after_song(&mut self, play: &mut PlayState) {
        if decr(&mut self.timer) != 0 {
            set_player_state2(play, STATE2_23);
        } else if self.interact_info.talk_state == NPC_TALK_STATE_IDLE {
            self.actor.flags |= ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
            // Message_CloseTextbox.
            play.with_msg(|m, f| m.close_textbox(f.audio));
        } else {
            self.actor.flags &= !ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
            self.action = Action::WaitToEndTalk;
        }
    }

    fn look(&self) -> MalonAdultLook<'_> {
        MalonAdultLook { skel: &self.skel, interact_info: &self.interact_info, upper_body_rot: &self.upper_body_rot, eye_index: self.eye_index, mouth_index: self.mouth_index }
    }
}

impl ActorImpl for EnMa2 {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnMa2_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        self.skel.update();
        self.update_eyes(play);
        match self.action {
            Action::WaitToEndTalk => self.wait_to_end_talk(play),
            Action::WaitForOcarina => self.wait_for_ocarina(play),
            Action::WaitForEponasSong => self.wait_for_eponas_song(play),
            Action::ForceTalkAfterSong => self.force_talk_after_song(play),
        }
        self.update_singing(play);
        self.update_tracking(play);
        if self.action != Action::WaitForEponasSong {
            // Npc_UpdateTalking(.., collider.dim.radius + 30, EnMa2_GetTextId, EnMa2_UpdateTalkState).
            let mut talk_state = self.interact_info.talk_state;
            let range = self.collider.dim.radius as f32 + 30.0;
            talk_update(play, &mut self.actor, &mut talk_state, range, |play, _| Self::get_text_id(play), Self::update_talk_state);
            self.interact_info.talk_state = talk_state;
        }
    }
    /// `EnMa2_Draw`'s `Audio_UpdateMalonSinging`, and `EnMa2_PostLimbDraw`'s focus.
    fn draw_update(&mut self, play: &mut PlayState) {
        let Some(model) = self.model.take() else { return };
        let look = MalonAdultLook { skel: &self.skel, interact_info: &self.interact_info, upper_body_rot: &self.upper_body_rot, eye_index: self.eye_index, mouth_index: self.mouth_index };
        draw_update_malon_adult(play, &mut self.actor, &model, &look);
        self.model = Some(model);
    }
    fn render_state(&self) -> RenderState {
        render_state_malon_adult(&self.actor, &self.look())
    }
    /// `EnMa2_Draw`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if let Some(model) = &self.model {
            draw_malon_adult(model, rs, out);
        }
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
    fn the_bakes_drop_the_thighs_and_add_the_basket() {
        let b = bakes();
        assert_eq!(b.len(), 3 * 2 + 1);
        let BakeBody::Skeleton { limbs, .. } = &b[0].body else { panic!() };
        // MALON_ADULT_LIMB_LEFT_THIGH 3 and _RIGHT_THIGH 6, 0-based.
        assert_eq!(limbs.iter().map(|l| l.limb).collect::<Vec<_>>(), vec![2, 5]);
        assert_eq!(b[6].name, BASKET_BAKE);
    }

    #[test]
    fn the_mask_reactions_are_malons_row() {
        assert_eq!(MASK_REACTION_MALON[1], 0x7176);
        assert_eq!(MASK_REACTION_MALON[5], 0x7175);
    }
}
