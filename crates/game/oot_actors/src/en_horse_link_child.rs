//! `En_Horse_Link_Child` (`ovl_En_Horse_Link_Child/z_en_horse_link_child.c`): young Epona, the
//! foal in Lon Lon Ranch's paddock. A skin skeleton (`gChildEponaSkel`, `oot_game::skin`) playing
//! five animations (`sAnimations`: idle, whinny, walking, trotting, galloping) through the skin's
//! SkelAnime, at scale 0.005.
//!
//! Her init kills her on the ranch until Talon is back from the castle
//! (`EVENTCHKINF_TALON_RETURNED_FROM_CASTLE`; not on a cutscene layer) and starts her in
//! `func_80A6A068` (action 3). There, before Malon has talked to Link at the ranch
//! (`EVENTCHKINF_TALKED_TO_CHILD_MALON_AT_RANCH`) she stands, idling and whinnying in turn; after,
//! at each animation's end (or every frame while she stands) she picks her pace: with Link more
//! than 250 from her home she goes back home (galloping from 300, trotting from 150, walking from
//! 70, else standing), with Link nearer she runs from him (galloping within 200, trotting within
//! 300, walking within 400, else standing). While she moves she turns 0x12C a frame
//! (`func_80A69F5C`: home-wards, or by Link's facing when he is near her home: @bug (game) below).
//!
//! With `EVENTCHKINF_CAN_LEARN_EPONAS_SONG` and Epona's Song played (`R_EPONAS_SONG_PLAYED`, the
//! message box's `play.msg_ctx.r_eponas_song_played`), or on the ranch with `cutsceneIndex`
//! `CS_INDEX_1`, she stops to face Link (`func_80A6A4DC`, `func_80A6A5A4`, action 5): idling or
//! whinnying, and at an animation's end turning towards him on the spot (walking in place,
//! `Horse_RotateToPoint` 300 a frame) while he is behind her. Played again there, or when the
//! scene was entered by `ENTR_LON_LON_RANCH_1` (the end of Epona's Song's cutscene, destination 94
//! of the cutscene terminator), she neighs and comes to Link (`func_80A6A724`, `func_80A6A7D0`,
//! action 4) for 300 frames, then goes back home and faces Link again there.
//!
//! The other actions (`func_80A698F4`: every animation in turn; `func_80A69C18`: running to Link;
//! `func_80A699FC`: standing near him) are reached by nothing in the overlay; they're ported too.
//!
//! Every frame: `Actor_MoveXZGravity`, the floor and walls (20, 55, 100), on the ranch z kept at
//! -2400 or more, the focus 70 up, the blink (`sEyeIndexOrder`, a 1 in 40 chance from open), the
//! body's cylinder (OC) and the sounds by animation (`func_80A6948C`: `NA_SE_EV_KID_HORSE_WALK` at
//! frames 1 and 19 of the walk, `_RUN` at the start of the trot and gallop, `_GROAN` or `_NEIGH` at
//! the start of the whinny).
//!
//! Drawn as `EnHorseLinkChild_Draw` does: the skin (`func_800A6360`, `setTranslation` true), her
//! eyes (`EnHorseLinkChild_OverrideLimbDraw`: segment 8 for limb 13, `sEyeTextures`) swapped in per
//! draw (`DrawParams::texture_images`: the limb's texture is loaded from segment 8, which the skin's
//! animated limb 5 also uses for its vertices). The post-draw (`EnHorseLinkChild_PostDraw`) puts the
//! head's sphere on limb 13 (`Skin_GetLimbPos`) and registers it for OC (`@bug (game)`, as
//! `EnHorse_SkinCallback1`'s: from the draw, after the frame's collision check).
//!
//! The whole overlay is ported. Not drawn: her shadow (`ActorShadow_DrawHorse`, not ported for any
//! horse).
//!
//! Params: none read (her placements' are 0xFFFF). `world.rot.z`: placed 3 (the ranch), zeroed
//! by the init (`home`, `world` and `shape`), read by nothing.
//! Flags: reads EVENTCHKINF_TALON_RETURNED_FROM_CASTLE (the ranch: there only after it),
//! EVENTCHKINF_TALKED_TO_CHILD_MALON_AT_RANCH (she runs from Link after it) and
//! EVENTCHKINF_CAN_LEARN_EPONAS_SONG (with Epona's Song played: she faces Link); reads and clears
//! `R_EPONAS_SONG_PLAYED`; reads `gSaveContext.save.entranceIndex` (`ENTR_LON_LON_RANCH_1`: she
//! comes to Link) and `cutsceneIndex` (`CS_INDEX_1` on the ranch: she faces Link).

use std::sync::Arc;

use eng_anim::anim::JointTable;
use eng_collision::math3d::{Cylinder16, Sphere16};
use eng_gfx::{DrawCmd, DrawImage, DrawParams, MeshKey, SourceImage};
use eng_math::cos_s;
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_UPDATE_DURING_OCARINA, Actor, UPDBGCHECKINFO_FLAG_0, UPDBGCHECKINFO_FLAG_2, UPDBGCHECKINFO_FLAG_3, UPDBGCHECKINFO_FLAG_4};
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, Source};
use oot_game::collision_check::*;
use oot_game::event_chk_inf;
use oot_game::horse::horse_rotate_to_point;
use oot_game::pack::{BakeBody, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::{ANIMMODE_ONCE, Anim, SkelAnimeStd};
use oot_game::skin::{SkinPlace, SkinSkeleton, apply_anim_transformations, get_limb_pos, skin_draw};
use oot_game::target::yaw_to;

/// `ACTOR_EN_HORSE_LINK_CHILD` (`actor_table.h`: 0x009A).
pub const ACTOR_EN_HORSE_LINK_CHILD: i16 = 0x009A;
/// `OBJECT_HORSE_LINK_CHILD` (`object_table.h`: 0x00A0).
pub const OBJECT: &str = "object_horse_link_child";

/// `En_Horse_Link_Child_Profile`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_HORSE_LINK_CHILD,
    name: "En_Horse_Link_Child",
    category: ACTORCAT_BG,
    flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `SCENE_LON_LON_RANCH` (`scene_table.h`: 0x63).
pub const SCENE_LON_LON_RANCH: u16 = 0x63;
/// `ENTR_LON_LON_RANCH_1` (`entrance_table.h`: 0x2AE): the end of Epona's Song's cutscene.
pub const ENTR_LON_LON_RANCH_1: u16 = 0x2AE;
/// `CS_INDEX_1` (`save.h`: 0xFFF1).
pub const CS_INDEX_1: u16 = 0xFFF1;

// `EVENTCHKINF_*` (`save.h`).
pub const EVENTCHKINF_TALON_RETURNED_FROM_CASTLE: u16 = 0x14;
pub const EVENTCHKINF_TALKED_TO_CHILD_MALON_AT_RANCH: u16 = 0x15;
pub const EVENTCHKINF_CAN_LEARN_EPONAS_SONG: u16 = 0x16;

/// The params: nothing reads them.
pub const PARAMS: Field = Field::whole("params", Kind::Unused, "placed 0xFFFF; read by nothing");
/// `world.rot.z`: zeroed by the init.
pub const ROT_Z: Field = Field::rot("rot.z", Source::RotZ, 0, 16, Kind::Unused, "placed 3 on the ranch; zeroed by the init, read by nothing");

/// En_Horse_Link_Child's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[PARAMS, ROT_Z],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_TALON_RETURNED_FROM_CASTLE)).reads().note("the ranch: killed until Talon is back"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_TALKED_TO_CHILD_MALON_AT_RANCH)).reads().note("after it she runs from Link, or home"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_CAN_LEARN_EPONAS_SONG)).reads().note("with Epona's Song played she faces Link"),
        FlagUse::new(Flag::Other("R_EPONAS_SONG_PLAYED")).reads().clears().note("Epona's Song played: she faces Link, again: she comes to him"),
    ],
    todo: false,
};

// `NA_SE_EV_KID_HORSE_*` (`environmentbank_table.h`: 0x2842 to 0x2845).
pub const NA_SE_EV_KID_HORSE_WALK: u16 = 0x2842;
pub const NA_SE_EV_KID_HORSE_RUN: u16 = 0x2843;
pub const NA_SE_EV_KID_HORSE_NEIGH: u16 = 0x2844;
pub const NA_SE_EV_KID_HORSE_GROAN: u16 = 0x2845;

/// `sAnimations`.
pub const ANIMATIONS: [&str; 5] = ["gChildEponaIdleAnim", "gChildEponaWhinnyAnim", "gChildEponaWalkingAnim", "gChildEponaTrottingAnim", "gChildEponaGallopingAnim"];
const SKEL: &str = "gChildEponaSkel";

/// `sEyeTextures`.
pub const EYE_TEXTURES: [&str; 3] = ["gChildEponaEyeOpenTex", "gChildEponaEyeHalfTex", "gChildEponaEyeCloseTex"];
/// `sEyeIndexOrder`.
pub const EYE_INDEX_ORDER: [u8; 4] = [0, 1, 2, 1];
/// Segment 8: the eyes' texture (`gSPSegment(POLY_OPA_DISP++, 0x08, ..)` for limb 13).
const SEG_EYES: u32 = 0x0800_0000;

/// The actions (`sActionFuncs`' indices).
pub const ACTION_CYCLE: i32 = 0; // func_80A698F4
pub const ACTION_RUN_TO_LINK: i32 = 1; // func_80A69C18
pub const ACTION_STAND_NEAR_LINK: i32 = 2; // func_80A699FC
pub const ACTION_RANCH: i32 = 3; // func_80A6A068
pub const ACTION_COME_TO_LINK: i32 = 4; // func_80A6A7D0
pub const ACTION_FACE_LINK: i32 = 5; // func_80A6A5A4

/// `sCylinderInit` (`ColliderCylinderInitType1`: `Collider_SetCylinderType1` makes `ocFlags2`
/// `OC2_TYPE_1`).
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 20, height: 100, y_shift: 0, pos: [0; 3] },
};

/// `sJntSphElementsInit`: limb 13, centre 0, radius 10, scale 100.
const JNT_SPH_ELEMENTS: [ColliderJntSphElementInit; 1] = [ColliderJntSphElementInit {
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    limb: 13,
    model_sphere: Sphere16 { center: [0, 0, 0], radius: 10 },
    scale: 100,
}];

/// `sJntSphInit`.
const JNT_SPH_INIT: ColliderInit = ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1 | OC2_UNK1, shape: COLSHAPE_JNTSPH };

/// `sColCheckInfoInit`.
const COL_CHECK_INFO_INIT: CollisionCheckInfoInit = CollisionCheckInfoInit { health: 10, cyl_radius: 35, cyl_height: 100, mass: MASS_HEAVY };

/// `D_80A6AF64`: each animation's speed factor.
const D_80A6AF64: [f32; 5] = [1.0, 1.0, 1.5, 1.5, 1.5];
/// `D_80A6AF5C`: the walk's hoof frames.
const D_80A6AF5C: [i32; 2] = [1, 19];

/// The foal's mesh: the skin as `func_800A6360` draws it (her eyes swapped per draw).
pub const BAKE: &str = "En_Horse_Link_Child/skin";

/// The mesh `EnHorseLinkChild_Draw` draws.
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake { name: BAKE.into(), object: OBJECT.into(), segments: Vec::new(), prelude: Vec::new(), body: BakeBody::Skin { file: OBJECT.into(), symbol: SKEL.into() } }]
}

// The render state's extras.
mod rs {
    /// `switches`: the eye texture (`sEyeIndexOrder[eyeTexIndex]`).
    pub const EYES: usize = 0;
}

pub struct EnHorseLinkChild {
    pub actor: Actor,
    /// `action`: `sActionFuncs`' index (`ACTION_*`).
    pub action: i32,
    /// `animationIdx`: `sAnimations`' index.
    pub animation_idx: i32,
    /// `skin.skelAnime`.
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<SkinSkeleton>,
    pub timer: i32,
    /// `unk_1E8`: going home (`func_80A6A7D0` after 300 frames).
    pub unk_1e8: bool,
    /// `eyeTexIndex`: `sEyeIndexOrder`'s index.
    pub eye_tex_index: u8,
    /// `unk_1F0`: the walk's next hoof (`D_80A6AF5C`).
    pub unk_1f0: i32,
    pub body_collider: ColliderCylinder,
    pub head_collider: ColliderJntSph,
    /// `unk_2A0`: `EVENTCHKINF_CAN_LEARN_EPONAS_SONG` as last seen (set, never read).
    pub unk_2a0: bool,
    anims: Vec<Anim>,
    eyes: Vec<DrawImage>,
}

impl EnHorseLinkChild {
    /// `EnHorseLinkChild_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            actor.kill();
            return oot_game::spawn::Placeholder::init(actor, play);
        };
        let loaded = (|| -> anyhow::Result<_> {
            let skeleton = assets.pack.skin_skeleton(OBJECT, SKEL)?;
            let anims = ANIMATIONS.iter().map(|a| assets.animation(OBJECT, a)).collect::<anyhow::Result<Vec<_>>>()?;
            let mut eyes = Vec::new();
            for s in EYE_TEXTURES {
                let t = assets.pack.texture(OBJECT, s)?;
                eyes.push(DrawImage { width: t.width, height: t.height, rgba: t.rgba.clone().into() });
            }
            Ok((skeleton, anims, eyes))
        })();
        let (skeleton, anims, eyes) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Horse_Link_Child: {e:#}");
                actor.kill();
                return oot_game::spawn::Placeholder::init(actor, play);
            }
        };
        // sInitChain: cullingVolumeScale 1200.
        actor.culling_volume_scale = 1200.0;
        actor.scale = Vec3::splat(0.005);
        actor.gravity = -3.5;
        // ActorShape_Init(&shape, 0, ActorShadow_DrawHorse, 20): the shadow isn't ported.
        actor.shape_y_offset = 0.0;
        actor.speed_xz = 0.0;
        actor.focus_pos = actor.world_pos + Vec3::Y * 70.0;
        // Skin_Init(.., &gChildEponaSkel, &gChildEponaGallopingAnim): SkelAnime_InitSkin plays it
        // looped; then Animation_PlayOnce(sAnimations[0]).
        let mut skel = SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(anims[4].clone()));
        skel.play_once(anims[0].clone());
        let mut this = EnHorseLinkChild {
            actor,
            action: ACTION_RUN_TO_LINK,
            animation_idx: 0,
            skel,
            skeleton: Arc::new(skeleton),
            timer: 0,
            unk_1e8: false,
            eye_tex_index: 0,
            unk_1f0: 0,
            body_collider: ColliderCylinder::new(&CYLINDER_INIT),
            head_collider: ColliderJntSph::new(&JNT_SPH_INIT, &JNT_SPH_ELEMENTS),
            unk_2a0: false,
            anims,
            eyes,
        };
        this.actor.col_chk_info.set_info(None, &COL_CHECK_INFO_INIT);

        if play.is_cutscene_layer() {
            this.func_80a69ec0();
        } else if play.scene_id == SCENE_LON_LON_RANCH {
            if !play.save.get_event_chk_inf(EVENTCHKINF_TALON_RETURNED_FROM_CASTLE) {
                this.actor.kill();
                return Box::new(this);
            }
            this.unk_2a0 = play.save.get_event_chk_inf(EVENTCHKINF_CAN_LEARN_EPONAS_SONG);
            this.func_80a69ec0();
        } else {
            this.func_80a69ec0();
        }

        this.actor.home_rot.z = 0;
        this.actor.world_rot.z = 0;
        this.actor.shape_rot.z = 0;
        Box::new(this)
    }

    /// `Animation_Change(&skin.skelAnime, sAnimations[animationIdx], speed, start,
    /// Animation_GetLastFrame(..), mode, morph)`.
    fn change(&mut self, speed: f32, start: f32, mode: u8, morph: f32) {
        let a = self.anims[self.animation_idx as usize].clone();
        let last = a.last_frame();
        self.skel.change(a, speed, start, last, mode, morph);
    }

    fn player_actor(play: &PlayState) -> Option<&Actor> {
        play.player.and_then(|h| play.actors.actor(h))
    }

    /// `Actor_WorldDistXZToActor(&this->actor, &player->actor)`.
    fn dist_xz_to_player(&self, play: &PlayState) -> f32 {
        Self::player_actor(play).map_or(f32::MAX, |p| {
            let d = p.world_pos - self.actor.world_pos;
            (d.x * d.x + d.z * d.z).sqrt()
        })
    }

    /// `Actor_WorldYawTowardActor(&this->actor, &player->actor)`.
    fn yaw_to_player(&self, play: &PlayState) -> i16 {
        Self::player_actor(play).map_or(0, |p| yaw_to(self.actor.world_pos, p.world_pos))
    }

    fn player_pos(play: &PlayState) -> Vec3 {
        Self::player_actor(play).map_or(Vec3::ZERO, |p| p.world_pos)
    }

    /// `func_80A693D0`: the walk's hooves, at frames past 1 and 19.
    fn func_80a693d0(&mut self, play: &mut PlayState) {
        let cur = self.skel.cur_frame;
        if cur > D_80A6AF5C[self.unk_1f0 as usize] as f32 && !(self.unk_1f0 == 0 && cur > D_80A6AF5C[1] as f32) {
            // SFX_PLAY_AT_POS(&this->actor.projectedPos, NA_SE_EV_KID_HORSE_WALK).
            audio_play_actor_sfx2(play, NA_SE_EV_KID_HORSE_WALK);
            self.unk_1f0 += 1;
            if self.unk_1f0 >= D_80A6AF5C.len() as i32 {
                self.unk_1f0 = 0;
            }
        }
    }

    /// `func_80A6948C`: the sounds by animation.
    fn func_80a6948c(&mut self, play: &mut PlayState) {
        if self.animation_idx == 2 {
            self.func_80a693d0(play);
        } else if self.skel.cur_frame == 0.0 {
            if self.animation_idx == 3 || self.animation_idx == 4 {
                audio_play_actor_sfx2(play, NA_SE_EV_KID_HORSE_RUN);
            } else if self.animation_idx == 1 {
                if play.rand.zero_one() > 0.5 {
                    audio_play_actor_sfx2(play, NA_SE_EV_KID_HORSE_GROAN);
                } else {
                    audio_play_actor_sfx2(play, NA_SE_EV_KID_HORSE_NEIGH);
                }
            }
        }
    }

    /// `func_80A695A4`: the animation's speed: the walk, trot and gallop by `speed` (/2, /3, /5).
    pub fn func_80a695a4(&self) -> f32 {
        let i = self.animation_idx as usize;
        match self.animation_idx {
            2 => D_80A6AF64[i] * self.actor.speed_xz * (1.0 / 2.0),
            3 => D_80A6AF64[i] * self.actor.speed_xz * (1.0 / 3.0),
            4 => D_80A6AF64[i] * self.actor.speed_xz * (1.0 / 5.0),
            _ => D_80A6AF64[i],
        }
    }

    /// `func_80A6988C`: the next animation, once.
    fn func_80a6988c(&mut self) {
        self.action = ACTION_CYCLE;
        self.animation_idx += 1;
        if self.animation_idx >= ANIMATIONS.len() as i32 {
            self.animation_idx = 0;
        }
        let a = self.anims[self.animation_idx as usize].clone();
        self.skel.play_once(a);
        self.skel.play_speed = self.func_80a695a4();
    }

    /// `func_80A698F4` (action 0): every animation in turn.
    fn func_80a698f4(&mut self, _play: &mut PlayState) {
        self.actor.speed_xz = 0.0;
        if self.skel.update() {
            self.func_80a6988c();
        }
    }

    /// `func_80A6993C`: standing near Link (action 2), idling (0) or whinnying (1).
    fn func_80a6993c(&mut self, mut new_animation_idx: i32) {
        self.action = ACTION_STAND_NEAR_LINK;
        self.actor.speed_xz = 0.0;
        if !(new_animation_idx == 0 || new_animation_idx == 1) {
            new_animation_idx = 0;
        }
        if self.animation_idx != new_animation_idx {
            self.animation_idx = new_animation_idx;
            self.change(self.func_80a695a4(), 0.0, ANIMMODE_ONCE, -5.0);
        }
    }

    /// `func_80A699FC` (action 2): idling and whinnying in turn until Link is between 70 and 1000
    /// away, then running to him.
    fn func_80a699fc(&mut self, play: &mut PlayState) {
        let dist_from_link = self.dist_xz_to_player(play);
        if self.skel.update() {
            if dist_from_link < 1000.0 && dist_from_link > 70.0 {
                self.func_80a69b7c();
            } else {
                let new_animation_idx = if self.animation_idx == 1 { 0 } else { 1 };
                if self.animation_idx != new_animation_idx {
                    self.animation_idx = new_animation_idx;
                    self.change(self.func_80a695a4(), 0.0, ANIMMODE_ONCE, -5.0);
                } else {
                    self.change(self.func_80a695a4(), 0.0, ANIMMODE_ONCE, 0.0);
                }
            }
        }
    }

    /// `func_80A69B7C`: running to Link (action 1).
    fn func_80a69b7c(&mut self) {
        self.action = ACTION_RUN_TO_LINK;
        self.animation_idx = 0;
        self.actor.speed_xz = 0.0;
        self.change(self.func_80a695a4(), 0.0, ANIMMODE_ONCE, -5.0);
    }

    /// Turns `world.rot.y` by `yaw_diff`, at most 0x12C, and the shape with it.
    fn turn(&mut self, yaw_diff: i16) {
        if yaw_diff > 0x12C {
            self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(0x12C);
        } else if yaw_diff < -0x12C {
            self.actor.world_rot.y = self.actor.world_rot.y.wrapping_sub(0x12C);
        } else {
            self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(yaw_diff);
        }
        self.actor.shape_rot.y = self.actor.world_rot.y;
    }

    fn is_moving(&self) -> bool {
        self.animation_idx == 4 || self.animation_idx == 3 || self.animation_idx == 2
    }

    /// `func_80A69C18` (action 1): towards Link, galloping from 300, trotting from 150, walking
    /// from 70; standing beyond 1000 or within 70.
    fn func_80a69c18(&mut self, play: &mut PlayState) {
        if self.is_moving() {
            let yaw_diff = self.yaw_to_player(play).wrapping_sub(self.actor.world_rot.y);
            self.turn(yaw_diff);
        }
        if self.skel.update() {
            let dist_from_link = self.dist_xz_to_player(play);
            let new_animation_idx;
            if dist_from_link > 1000.0 {
                self.func_80a6993c(0);
                return;
            } else if dist_from_link < 1000.0 && dist_from_link >= 300.0 {
                new_animation_idx = 4;
                self.actor.speed_xz = 6.0;
            } else if dist_from_link < 300.0 && dist_from_link >= 150.0 {
                new_animation_idx = 3;
                self.actor.speed_xz = 4.0;
            } else if dist_from_link < 150.0 && dist_from_link >= 70.0 {
                new_animation_idx = 2;
                self.actor.speed_xz = 2.0;
                self.unk_1f0 = 0;
            } else {
                self.func_80a6993c(1);
                return;
            }
            if self.animation_idx != new_animation_idx {
                self.animation_idx = new_animation_idx;
                self.change(self.func_80a695a4(), 0.0, ANIMMODE_ONCE, -5.0);
            } else {
                self.change(self.func_80a695a4(), 0.0, ANIMMODE_ONCE, 0.0);
            }
        }
    }

    /// `func_80A69EC0`: the ranch's behaviour (action 3), idling.
    fn func_80a69ec0(&mut self) {
        self.action = ACTION_RANCH;
        self.animation_idx = 0;
        self.actor.speed_xz = 0.0;
        self.change(self.func_80a695a4(), 0.0, ANIMMODE_ONCE, -5.0);
    }

    /// `func_80A69F5C`: while she moves, turning 0x12C a frame: towards her home, or, with Link
    /// within 250 of her home, by `player->actor.shape.rot.y + (±1 << 0xE)` (the sign of
    /// `Actor_WorldYawTowardActor`). @bug (game): that yaw is used as the difference, without her
    /// own yaw taken from it (nor is the yaw towards Link made relative): she turns by the sign of
    /// Link's facing a quarter turn off, not away from him.
    fn func_80a69f5c(&mut self, play: &mut PlayState) {
        if !self.is_moving() {
            return;
        }
        let Some(player) = Self::player_actor(play) else { return };
        let yaw_diff = if player.world_pos.distance(self.actor.home_pos) < 250.0 {
            let mut yaw_diff = player.shape_rot.y;
            let yaw_sign: i32 = if yaw_to(self.actor.world_pos, player.world_pos) > 0 { 1 } else { -1 };
            let yaw_offset = yaw_sign << 0xE;
            yaw_diff = (yaw_diff as i32 + yaw_offset) as i16;
            yaw_diff
        } else {
            yaw_to(self.actor.world_pos, self.actor.home_pos).wrapping_sub(self.actor.world_rot.y)
        };
        self.turn(yaw_diff);
    }

    /// Standing: idling and whinnying in turn (a new one when one ends).
    fn standing_idx(&self, anim_finished: bool) -> i32 {
        if self.animation_idx == 0 {
            if anim_finished { 1 } else { 0 }
        } else if anim_finished {
            0
        } else {
            1
        }
    }

    /// `func_80A6A068` (action 3): the ranch. Entered by `ENTR_LON_LON_RANCH_1`: a neigh and to
    /// Link. Epona's Song played (with `EVENTCHKINF_CAN_LEARN_EPONAS_SONG`) or the ranch's
    /// `CS_INDEX_1`: facing Link (and this frame's pace still picked). After Malon's talk, her
    /// pace by her distance from home (Link more than 250 from it) or from Link; else standing.
    fn func_80a6a068(&mut self, play: &mut PlayState) {
        self.func_80a69f5c(play);
        let player_pos = Self::player_pos(play);
        let dist_from_link = self.dist_xz_to_player(play);

        if play.save.entrance_index == ENTR_LON_LON_RANCH_1 {
            audio_play_actor_sfx2(play, NA_SE_EV_KID_HORSE_NEIGH);
            self.func_80a6a724();
            return;
        }

        let can_learn = play.save.get_event_chk_inf(EVENTCHKINF_CAN_LEARN_EPONAS_SONG);
        if (can_learn && play.msg_ctx.r_eponas_song_played) || (play.scene_id == SCENE_LON_LON_RANCH && play.save.cutscene_index == CS_INDEX_1) {
            self.func_80a6a4dc(play);
        } else {
            self.unk_2a0 = can_learn;
        }

        let mut new_animation_idx = self.animation_idx;
        let anim_finished = self.skel.update();
        if anim_finished || self.animation_idx == 1 || self.animation_idx == 0 {
            if play.save.get_event_chk_inf(EVENTCHKINF_TALKED_TO_CHILD_MALON_AT_RANCH) {
                let dist_from_home = self.actor.world_pos.distance(self.actor.home_pos);
                let dist_link_from_home = player_pos.distance(self.actor.home_pos);
                if dist_link_from_home > 250.0 {
                    if dist_from_home >= 300.0 {
                        new_animation_idx = 4;
                        self.actor.speed_xz = 6.0;
                    } else if dist_from_home < 300.0 && dist_from_home >= 150.0 {
                        new_animation_idx = 3;
                        self.actor.speed_xz = 4.0;
                    } else if dist_from_home < 150.0 && dist_from_home >= 70.0 {
                        new_animation_idx = 2;
                        self.actor.speed_xz = 2.0;
                        self.unk_1f0 = 0;
                    } else {
                        self.actor.speed_xz = 0.0;
                        new_animation_idx = self.standing_idx(anim_finished);
                    }
                } else if dist_from_link < 200.0 {
                    new_animation_idx = 4;
                    self.actor.speed_xz = 6.0;
                } else if dist_from_link < 300.0 {
                    new_animation_idx = 3;
                    self.actor.speed_xz = 4.0;
                } else if dist_from_link < 400.0 {
                    new_animation_idx = 2;
                    self.actor.speed_xz = 2.0;
                    self.unk_1f0 = 0;
                } else {
                    self.actor.speed_xz = 0.0;
                    new_animation_idx = self.standing_idx(anim_finished);
                }
            } else {
                self.actor.speed_xz = 0.0;
                new_animation_idx = self.standing_idx(anim_finished);
            }
        }

        if self.animation_idx != new_animation_idx || anim_finished {
            self.animation_idx = new_animation_idx;
            self.change(self.func_80a695a4(), 0.0, ANIMMODE_ONCE, -5.0);
        } else {
            // Animation_Change(.., curFrame, .., 2 (ANIMMODE_ONCE), 0): on from where it is, at
            // this frame's speed.
            let cur = self.skel.cur_frame;
            self.change(self.func_80a695a4(), cur, ANIMMODE_ONCE, 0.0);
        }
    }

    /// `func_80A6A4DC`: facing Link (action 5), idling or whinnying at random; Epona's Song's
    /// played flag cleared.
    fn func_80a6a4dc(&mut self, play: &mut PlayState) {
        self.action = ACTION_FACE_LINK;
        self.animation_idx = if play.rand.zero_one() > 0.5 { 0 } else { 1 };
        play.msg_ctx.r_eponas_song_played = false;
        self.change(self.func_80a695a4(), 0.0, ANIMMODE_ONCE, 0.0);
    }

    /// `func_80A6A5A4` (action 5): Epona's Song played again: a neigh and to Link. Else standing;
    /// at an animation's end, with Link behind her (`Math_CosS` of the yaw difference below 0) the
    /// walk (at 1.5, in place), turning to him while he's more than 45 degrees off; else another
    /// idle or whinny.
    fn func_80a6a5a4(&mut self, play: &mut PlayState) {
        if play.msg_ctx.r_eponas_song_played {
            play.msg_ctx.r_eponas_song_played = false;
            audio_play_actor_sfx2(play, NA_SE_EV_KID_HORSE_NEIGH);
            self.func_80a6a724();
        } else {
            self.actor.speed_xz = 0.0;
            let yaw_diff = self.yaw_to_player(play).wrapping_sub(self.actor.world_rot.y);
            // 0.7071 = cos(pi/4).
            if cos_s(yaw_diff) < 0.7071 && self.animation_idx == 2 {
                horse_rotate_to_point(&mut self.actor, Self::player_pos(play), 300);
            }
            if self.skel.update() {
                if cos_s(yaw_diff) < 0.0 {
                    self.animation_idx = 2;
                    self.change(D_80A6AF64[self.animation_idx as usize], 0.0, ANIMMODE_ONCE, -5.0);
                } else {
                    self.func_80a6a4dc(play);
                }
            }
        }
    }

    /// `func_80A6A724`: coming to Link (action 4), walking.
    fn func_80a6a724(&mut self) {
        self.timer = 0;
        self.action = ACTION_COME_TO_LINK;
        self.animation_idx = 2;
        self.unk_1e8 = false;
        self.actor.speed_xz = 2.0;
        self.change(self.func_80a695a4(), 0.0, ANIMMODE_ONCE, -5.0);
    }

    /// `func_80A6A7D0` (action 4): towards Link (galloping from 300, trotting from 150, else
    /// walking) for 300 frames, then home (the same, and walking from 70); home, facing Link.
    fn func_80a6a7d0(&mut self, play: &mut PlayState) {
        self.timer += 1;
        if self.timer > 300 {
            self.unk_1e8 = true;
        }
        if self.is_moving() {
            if !self.unk_1e8 {
                horse_rotate_to_point(&mut self.actor, Self::player_pos(play), 300);
            } else {
                let home = self.actor.home_pos;
                horse_rotate_to_point(&mut self.actor, home, 300);
            }
        }
        if self.skel.update() {
            let dist = if !self.unk_1e8 { self.dist_xz_to_player(play) } else { self.actor.world_pos.distance(self.actor.home_pos) };
            let new_animation_idx;
            if !self.unk_1e8 {
                if dist >= 300.0 {
                    new_animation_idx = 4;
                    self.actor.speed_xz = 6.0;
                } else if dist >= 150.0 {
                    new_animation_idx = 3;
                    self.actor.speed_xz = 4.0;
                } else {
                    new_animation_idx = 2;
                    self.actor.speed_xz = 2.0;
                    self.unk_1f0 = 0;
                }
            } else if dist >= 300.0 {
                new_animation_idx = 4;
                self.actor.speed_xz = 6.0;
            } else if dist >= 150.0 {
                new_animation_idx = 3;
                self.actor.speed_xz = 4.0;
            } else if dist >= 70.0 {
                new_animation_idx = 2;
                self.actor.speed_xz = 2.0;
                self.unk_1f0 = 0;
            } else {
                self.func_80a6a4dc(play);
                return;
            }
            if self.animation_idx != new_animation_idx {
                self.animation_idx = new_animation_idx;
                self.change(self.func_80a695a4(), 0.0, ANIMMODE_ONCE, -5.0);
            } else {
                self.change(self.func_80a695a4(), 0.0, ANIMMODE_ONCE, 0.0);
            }
        }
    }

    /// What `Skin_ApplyAnimTransformations` reads of the actor now.
    fn place(&self) -> SkinPlace {
        SkinPlace { scale: self.actor.scale, rot: [self.actor.shape_rot.x, self.actor.shape_rot.y, self.actor.shape_rot.z], pos: self.actor.world_pos, y_offset: self.actor.shape_y_offset }
    }
}

impl ActorImpl for EnHorseLinkChild {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnHorseLinkChild_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            ACTION_CYCLE => self.func_80a698f4(play),
            ACTION_RUN_TO_LINK => self.func_80a69c18(play),
            ACTION_STAND_NEAR_LINK => self.func_80a699fc(play),
            ACTION_RANCH => self.func_80a6a068(play),
            ACTION_COME_TO_LINK => self.func_80a6a7d0(play),
            _ => self.func_80a6a5a4(play),
        }
        // Actor_MoveXZGravity.
        self.actor.move_forward();
        self.actor.update_bg_check_info(&play.col, 20.0, 55.0, 100.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
        if play.scene_id == SCENE_LON_LON_RANCH && self.actor.world_pos.z < -2400.0 {
            self.actor.world_pos.z = -2400.0;
        }
        self.actor.focus_pos = self.actor.world_pos + Vec3::Y * 70.0;

        if play.rand.zero_one() < 0.025 && self.eye_tex_index == 0 {
            self.eye_tex_index += 1;
        } else if self.eye_tex_index > 0 {
            self.eye_tex_index += 1;
            if self.eye_tex_index as usize >= EYE_INDEX_ORDER.len() {
                self.eye_tex_index = 0;
            }
        }

        self.body_collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.body_collider);
        self.func_80a6948c(play);
    }

    /// `EnHorseLinkChild_PostDraw` (from `func_800A6360`): the head's sphere on its limb
    /// (`Skin_GetLimbPos`, the skin's matrices as the draw leaves them), for OC.
    fn draw_update(&mut self, play: &mut PlayState) {
        let (mats, skin_mtx) = apply_anim_transformations(&self.skeleton, &self.skel.joint_table, &self.place(), true);
        for e in &mut self.head_collider.elements {
            let c = e.dim.model_sphere.center;
            let center = Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32);
            let new_center = get_limb_pos(&skin_mtx, &mats, e.dim.limb as usize, center);
            e.dim.world_sphere.center = [new_center.x as i16, new_center.y as i16, new_center.z as i16];
            e.dim.world_sphere.radius = (e.dim.model_sphere.radius as f32 * e.dim.scale) as i16;
        }
        // @bug (game) (see `EnHorse_SkinCallback1`): set from the draw, after this frame's check.
        play.collision_check_set_oc(&self.actor, 1, &mut self.head_collider);
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.switches = vec![EYE_INDEX_ORDER[self.eye_tex_index as usize] as u32];
        rs
    }

    /// `EnHorseLinkChild_Draw`: `Gfx_SetupDL_25Opa`, then the skin (`func_800A6360`, with
    /// `setTranslation`), the eyes by `EnHorseLinkChild_OverrideLimbDraw` (limb 13: every limb is
    /// drawn).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        let place = SkinPlace { scale: rs.scale, rot: rs.rot, pos: rs.pos, y_offset: rs.y_offset };
        let (transform, bones) = skin_draw(&self.skeleton, &joints.rot, &place, true);
        let eye = rs.switches.get(rs::EYES).copied().unwrap_or(0) as usize;
        let texture_images = vec![SourceImage { source: SEG_EYES, image: self.eyes[eye.min(self.eyes.len() - 1)].clone() }];
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE)), transform, bones, params: DrawParams { texture_images, ..Default::default() } });
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            0 => Some(ColliderMut::Cylinder(&mut self.body_collider)),
            1 => Some(ColliderMut::JntSph(&mut self.head_collider)),
            _ => None,
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
