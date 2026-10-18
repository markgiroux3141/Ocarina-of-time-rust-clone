//! `En_Daiku_Kakariko` (`ovl_En_Daiku_Kakariko/z_en_daiku_kakariko.c`): Kakariko's carpenters,
//! Mutoh's four men building the village while Link is a child.
//!
//! `EnDaikuKakariko_Init` decides by age, scene and time: a child sees them in Kakariko by day
//! (`flags` 1, with the type's `initFlags`), in the guest house by night (2), and in the potion
//! shop always (4); anywhere else, or for an adult, they're killed. The type's `initFlags`:
//!
//! - `CARPENTER_ICHIRO` (0): 0x0080: stands (`EnDaikuKakariko_Wait`).
//! - `CARPENTER_SABOORO` (1): 0x00B0: runs his path (0x10), there and back (0x20); the bg check
//!   only on his first run frame (0x80). `Bg_Spot01_Objects2` (the shooting gallery's frame,
//!   type 3) spawns him by day as its child at its path's first point.
//! - `CARPENTER_JIRO` (2): 0x0070: runs there and back, falling (0x40: gravity -1, the bg check
//!   every frame).
//! - `CARPENTER_SHIRO` (3): 0x0470: as Jiro, but stopping at each end for two plays of
//!   `object_daiku_Anim_001AB0` (0x400, `EnDaikuKakariko_StopRunning`).
//!
//! By night (`flags` 8) the standing ones play another idle (Sabooro and Shiro
//! `object_daiku_Anim_008164`, keeping it while they talk: 0x800). Standing, the head and torso
//! follow Link (`Npc_TrackPoint`, 0x100 and `NPC_TRACKING_HEAD_AND_TORSO`); talking, the whole
//! body (0x200, `NPC_TRACKING_FULL_BODY`), the neck tipped 5900 more (`neckAngle`, 0x1000). Their
//! texts are by type and by `flags` 1 (Kakariko by day) or 8 (night): `EnDaikuKakariko_HandleTalking`.
//!
//! Drawn as `EnDaikuKakariko_Draw` does: the type's env colour (the trousers), the skeleton, and
//! the type's head (`carpenterHeadDLists`) after limb 15's own list, from baked meshes
//! (docs/adr/0012-actor-bakes.md); the focus 700 along and 1100 up limb 15.
//!
//! A leftover of `En_Daiku` (the Gerudo fortress's carpenters): `EnDaikuKakariko_GetTalkState`
//! sets `INFTABLE_176` after text 0x6061 and `INFTABLE_178` after 0x6064, texts these carpenters
//! never say (theirs are 0x50xx, a mask's 0x71xx): the flags are never set here.
//!
//! Not ported here: the mask reactions (`MaskReaction_GetTextId`: no masks yet, so none).
//!
//! Params: `type` bits 0 to 1 (`ENDAIKUKAKARIKO_GET_TYPE`: 0 Ichiro, 1 Sabooro, 2 Jiro, 3 Shiro:
//! the trousers' colour, the head, `initFlags`, the texts); `pathIndex` bits 8 to 15
//! (`ENDAIKUKAKARIKO_GET_PATH_INDEX`: the scene path a runner runs, types 1 to 3 in Kakariko by
//! day). `Bg_Spot01_Objects2` spawns Sabooro with `(path << 8) + 1`.
//! Flags: sets INFTABLE_176 (after text 0x6061) and INFTABLE_178 (after 0x6064), texts that
//! aren't his: never reached.

use std::sync::Arc;

use eng_anim::skeleton::{Skeleton, binang_to_rad};
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{smooth_step_to_f, smooth_step_to_s};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind};
use oot_game::collision_check::*;
use oot_game::inf_table;
use oot_game::message::TEXT_STATE_DONE;
use oot_game::npc::{NpcTrack, offer_talk, process_talk_request, track_point};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::path::rad_to_binang_trunc;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::*;

/// `ACTOR_EN_DAIKU_KAKARIKO` (`actor_table.h`: 0x01BC).
pub const ACTOR_EN_DAIKU_KAKARIKO: i16 = 0x01BC;
/// `OBJECT_DAIKU` (`object_table.h`: 0x0122).
pub const OBJECT: &str = "object_daiku";

/// `En_Daiku_Kakariko_Profile`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_DAIKU_KAKARIKO,
    name: "En_Daiku_Kakariko",
    category: ACTORCAT_NPC,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_CULLING_DISABLED,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `ENDAIKUKAKARIKO_GET_TYPE`: `PARAMS_GET_U(params, 0, 2)`.
pub const TYPE: Field = Field::params(
    "type",
    0,
    2,
    Kind::Named(&[(0, "CARPENTER_ICHIRO"), (1, "CARPENTER_SABOORO"), (2, "CARPENTER_JIRO"), (3, "CARPENTER_SHIRO")]),
    "which carpenter: his trousers, head, behaviour and texts",
);
/// `ENDAIKUKAKARIKO_GET_PATH_INDEX`: `PARAMS_GET_U(params, 8, 8)`.
pub const PATH_INDEX: Field = Field::params("pathIndex", 8, 8, Kind::PathIndex, "the path a runner runs (types 1 to 3, in Kakariko by day)");

/// En_Daiku_Kakariko's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, PATH_INDEX],
    flags: &[
        FlagUse::new(inf_table!(INFTABLE_176)).sets().note("after text 0x6061, which isn't his: never"),
        FlagUse::new(inf_table!(INFTABLE_178)).sets().note("after text 0x6064, which isn't his: never"),
    ],
    todo: false,
};

// `KakarikoCarpenterType`.
pub const CARPENTER_ICHIRO: i16 = 0;
pub const CARPENTER_SABOORO: i16 = 1;
pub const CARPENTER_JIRO: i16 = 2;
pub const CARPENTER_SHIRO: i16 = 3;

// `SCENE_*` (`scene_table.h`).
/// `SCENE_KAKARIKO_CENTER_GUEST_HOUSE` (0x2A, `kakariko_scene`).
pub const SCENE_KAKARIKO_CENTER_GUEST_HOUSE: u16 = 0x2A;
/// `SCENE_POTION_SHOP_KAKARIKO` (0x30, `drag_scene`).
pub const SCENE_POTION_SHOP_KAKARIKO: u16 = 0x30;
/// `SCENE_KAKARIKO_VILLAGE` (0x52, `spot01_scene`).
pub const SCENE_KAKARIKO_VILLAGE: u16 = 0x52;

// `INFTABLE_*` (`save.h`).
pub const INFTABLE_176: u16 = 0x176;
pub const INFTABLE_178: u16 = 0x178;

/// `NA_SE_EN_MORIBLIN_WALK` (`sfx/enemybank_table.h`: 0x38B8).
pub const NA_SE_EN_MORIBLIN_WALK: u16 = 0x38B8;

/// `ATTENTION_RANGE_6`.
const ATTENTION_RANGE_6: u8 = 6;
/// `NPC_TRACKING_HEAD_AND_TORSO`, `NPC_TRACKING_FULL_BODY` (`actor.h`).
const NPC_TRACKING_HEAD_AND_TORSO: i16 = 2;
const NPC_TRACKING_FULL_BODY: i16 = 4;
/// `SCREEN_WIDTH`, `SCREEN_HEIGHT`.
const SCREEN_WIDTH: i16 = 320;
const SCREEN_HEIGHT: i16 = 240;

/// `initFlags`: each type's first `flags` in Kakariko by day.
pub const INIT_FLAGS: [u16; 4] = [0x0080, 0x00B0, 0x0070, 0x0470];

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
    dim: Cylinder16 { radius: 18, height: 66, y_shift: 0, pos: [0; 3] },
};

/// `sColChkInit`: immovable. (`sDamageTable` is all zeroes: nothing hits them, `AC_NONE`.)
const COL_CHK_INFO_INIT: CollisionCheckInfoInit2 = CollisionCheckInfoInit2 { health: 0, cyl_radius: 0, cyl_height: 0, cyl_y_shift: 0, mass: MASS_IMMOVABLE };

/// `sAnimationInfo`: (animation, mode); all at speed 1, morphing over 7 frames.
const ANIMATION_INFO: [(&str, u8); 6] = [
    ("object_daiku_Anim_001AB0", ANIMMODE_ONCE),
    ("object_daiku_Anim_007DE0", ANIMMODE_LOOP),
    ("object_daiku_Anim_00885C", ANIMMODE_LOOP),
    ("object_daiku_Anim_000C44", ANIMMODE_LOOP),
    ("object_daiku_Anim_000600", ANIMMODE_LOOP),
    ("object_daiku_Anim_008164", ANIMMODE_LOOP),
];
/// `sAnimationInfo[].morphFrames`.
const MORPH_FRAMES: f32 = -7.0;

// `EnDaikuKakarikoAnimation`.
pub const ENDAIKUKAKARIKO_ANIM_0: i32 = 0;
pub const ENDAIKUKAKARIKO_ANIM_1: i32 = 1;
pub const ENDAIKUKAKARIKO_ANIM_3: i32 = 3;
pub const ENDAIKUKAKARIKO_ANIM_4: i32 = 4;
pub const ENDAIKUKAKARIKO_ANIM_5: i32 = 5;

/// `carpenterHeadDLists`: each type's head, drawn after limb 15.
const HEAD_DLISTS: [&str; 4] = ["object_daiku_DL_005BD0", "object_daiku_DL_005AC0", "object_daiku_DL_005990", "object_daiku_DL_005880"];
/// `EnDaikuKakariko_Draw`'s `gDPSetEnvColor` by type.
const ENV_COLORS: [[u8; 4]; 4] = [[170, 10, 70, 255], [170, 200, 255, 255], [0, 230, 70, 255], [200, 0, 150, 255]];
/// The limb (1-based) the torso turns (`EnDaikuKakariko_OverrideLimbDraw`'s 8).
const TORSO_LIMB: usize = 8;
/// The head's limb (1-based): its turn, the head list and the focus.
const HEAD_LIMB: usize = 15;
/// `unkVec`: the focus on limb 15.
const FOCUS_OFFSET: Vec3 = Vec3::new(700.0, 1100.0, 0.0);
/// The segment the bakes' env colour is in (run before the body, as the draw's own command).
const SEG_ENV: u8 = 0x08;

/// The type's skeleton mesh.
pub fn bake_name(ty: usize) -> String {
    format!("En_Daiku_Kakariko/skel{ty}")
}

/// The type's head mesh.
pub fn head_bake_name(ty: usize) -> String {
    format!("En_Daiku_Kakariko/head{ty}")
}

/// The meshes `EnDaikuKakariko_Draw` draws: per type, `object_daiku_Skel_007958` and the head
/// list, both after the type's env colour.
pub fn bakes() -> Vec<MeshBake> {
    let mut v = Vec::new();
    for ty in 0..4 {
        let env = BakeSegment::Commands(vec![(0xFB00_0000, u32::from_be_bytes(ENV_COLORS[ty]))]);
        v.push(MeshBake {
            name: bake_name(ty),
            object: OBJECT.into(),
            segments: vec![(SEG_ENV, env.clone())],
            prelude: vec![SEG_ENV],
            body: BakeBody::Skeleton { file: OBJECT.into(), symbol: "object_daiku_Skel_007958".into(), limbs: Vec::new() },
        });
        v.push(MeshBake {
            name: head_bake_name(ty),
            object: OBJECT.into(),
            segments: vec![(SEG_ENV, env)],
            prelude: vec![SEG_ENV],
            body: BakeBody::DLists(vec![(OBJECT.into(), HEAD_DLISTS[ty].into())]),
        });
    }
    v
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `EnDaikuKakariko_Wait`.
    Wait,
    /// `EnDaikuKakariko_Talk`.
    Talk,
    /// `EnDaikuKakariko_StopRunning`.
    StopRunning,
    /// `EnDaikuKakariko_Run`.
    Run,
}

pub struct EnDaikuKakariko {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub action: Action,
    pub collider: ColliderCylinder,
    /// `talkState`: 0 not talking, 2 talking.
    pub talk_state: i32,
    pub waypoint: i32,
    pub run_speed: f32,
    pub current_anim_index: i32,
    /// `pathContinue`: running the path backwards.
    pub path_continue: bool,
    /// `run`: running (else turning to the next point first).
    pub run: bool,
    pub flags: u16,
    pub run_flag: u16,
    pub interact_info: NpcTrack,
    pub timer: i32,
    pub neck_angle: [i16; 3],
    pub neck_angle_target: [i16; 3],
    anims: Vec<Anim>,
}

impl EnDaikuKakariko {
    /// `ENDAIKUKAKARIKO_GET_TYPE`.
    pub fn ty(&self) -> i16 {
        TYPE.get(self.actor.params)
    }

    /// `ENDAIKUKAKARIKO_GET_PATH_INDEX`.
    pub fn path_index(&self) -> usize {
        PATH_INDEX.get(self.actor.params) as usize
    }

    /// `EnDaikuKakariko_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = (|| -> anyhow::Result<_> {
            let skeleton = assets.skeleton(OBJECT, "object_daiku_Skel_007958")?;
            let anims = ANIMATION_INFO.iter().map(|(name, _)| assets.animation(OBJECT, name)).collect::<anyhow::Result<Vec<_>>>()?;
            Ok((skeleton, anims))
        })();
        let (skeleton, anims) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Daiku_Kakariko: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let mut this = EnDaikuKakariko {
            // SkelAnime_InitFlex(.., object_daiku_Skel_007958, NULL, .., 17).
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), None),
            skeleton,
            action: Action::Wait,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            talk_state: 0,
            waypoint: 0,
            run_speed: 0.0,
            current_anim_index: 0,
            path_continue: false,
            run: false,
            flags: 0,
            run_flag: 0,
            interact_info: NpcTrack::default(),
            timer: 0,
            neck_angle: [0; 3],
            neck_angle_target: [0; 3],
            anims,
            actor,
        };
        let ty = this.ty();
        let night = play.save.night_flag;
        // LINK_AGE_IN_YEARS == YEARS_CHILD.
        if !play.save.adult {
            match play.scene_id {
                SCENE_KAKARIKO_VILLAGE => {
                    if !night {
                        this.flags |= 1;
                        this.flags |= INIT_FLAGS[ty as usize];
                    }
                }
                SCENE_KAKARIKO_CENTER_GUEST_HOUSE => {
                    if night {
                        this.flags |= 2;
                    }
                }
                SCENE_POTION_SHOP_KAKARIKO => this.flags |= 4,
                _ => {}
            }
        }
        if this.flags & 7 == 0 {
            this.actor.kill();
        }
        if night {
            this.flags |= 8;
        }
        // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 40); Actor_Init's scale 0.01.
        this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 40.0);
        this.actor.scale = Vec3::splat(0.01);
        this.actor.col_chk_info.set_info2(None, &COL_CHK_INFO_INIT);
        // Animation_Change(sAnimationInfo[ENDAIKUKAKARIKO_ANIM_0]), morphing.
        let a0 = this.anims[ENDAIKUKAKARIKO_ANIM_0 as usize].clone();
        let last = a0.last_frame();
        this.skel.change(a0, 1.0, 0.0, last, ANIMATION_INFO[0].1, MORPH_FRAMES);
        this.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
        this.actor.gravity = 0.0;
        this.run_speed = 3.0;
        this.actor.culling_volume_distance = 1200.0;
        this.actor.target_mode = ATTENTION_RANGE_6;
        this.current_anim_index = -1;
        if this.flags & 0x40 != 0 {
            this.actor.gravity = -1.0;
        }
        if this.flags & 0x10 != 0 {
            this.change_anim(ENDAIKUKAKARIKO_ANIM_3);
            this.action = Action::Run;
        } else {
            if this.flags & 8 != 0 {
                if ty == CARPENTER_SABOORO || ty == CARPENTER_SHIRO {
                    this.change_anim(ENDAIKUKAKARIKO_ANIM_5);
                    this.flags |= 0x800;
                } else {
                    this.change_anim(ENDAIKUKAKARIKO_ANIM_1);
                }
            } else {
                this.change_anim(ENDAIKUKAKARIKO_ANIM_0);
            }
            this.skel.cur_frame = (play.rand.zero_one() * this.skel.end_frame) as i32 as f32;
            this.flags |= 0x100;
            this.action = Action::Wait;
        }
        Box::new(this)
    }

    /// `EnDaikuKakariko_ChangeAnim`: no morph from none or to the same.
    pub fn change_anim(&mut self, index: i32) {
        let morph_frames = if self.current_anim_index < 0 || index == self.current_anim_index { 0.0 } else { MORPH_FRAMES };
        let anim = self.anims[index as usize].clone();
        let last = anim.last_frame();
        self.skel.change(anim, 1.0, 0.0, last, ANIMATION_INFO[index as usize].1, morph_frames);
        self.current_anim_index = index;
    }

    /// `EnDaikuKakariko_GetTalkState`: 0 when the text ends (`INFTABLE_176` after 0x6061,
    /// `INFTABLE_178` after 0x6064), else 2.
    fn get_talk_state(&mut self, play: &mut PlayState) -> i32 {
        let mut talk_state = 2;
        if play.message_state() == TEXT_STATE_DONE && play.message_should_advance() {
            match self.actor.text_id {
                0x6061 => play.save.set_inf_table(INFTABLE_176),
                0x6064 => play.save.set_inf_table(INFTABLE_178),
                _ => {}
            }
            talk_state = 0;
        }
        talk_state
    }

    /// The text `EnDaikuKakariko_HandleTalking` picks.
    pub fn text_id(&self) -> u16 {
        // MaskReaction_GetTextId(play, sMaskReactionSets[type]) (MASK_REACTION_SET_CARPENTER_1
        // to _4): no masks, so 0.
        match self.ty() {
            0 => {
                if self.flags & 8 != 0 {
                    0x5076
                } else {
                    0x5075
                }
            }
            1 => {
                if self.flags & 1 != 0 {
                    0x502A
                } else {
                    0x5074
                }
            }
            2 => {
                if self.flags & 1 != 0 {
                    0x506A
                } else {
                    0x506B
                }
            }
            _ => {
                if self.flags & 1 != 0 {
                    0x5077
                } else {
                    0x5078
                }
            }
        }
    }

    /// `EnDaikuKakariko_HandleTalking`.
    fn handle_talking(&mut self, play: &mut PlayState) {
        if self.talk_state == 2 {
            self.talk_state = self.get_talk_state(play);
        } else if process_talk_request(&mut self.actor) {
            self.talk_state = 2;
        } else {
            let (screen_x, screen_y) = oot_game::target::actor_screen_pos(play.view_proj, &self.actor);
            if (0..=SCREEN_WIDTH).contains(&screen_x) && (0..=SCREEN_HEIGHT).contains(&screen_y) && self.talk_state == 0 {
                let a = self.actor.clone();
                if offer_talk(play, &a, 100.0) {
                    self.actor.text_id = self.text_id();
                }
            }
        }
    }

    /// `EnDaikuKakariko_Talk`: back to waiting (or running) when the talk ends.
    fn talk(&mut self, play: &mut PlayState) {
        if self.skel.update() {
            self.change_anim(ENDAIKUKAKARIKO_ANIM_3);
        }
        self.handle_talking(play);
        if self.talk_state == 0 {
            if self.flags & 0x10 != 0 {
                self.change_anim(ENDAIKUKAKARIKO_ANIM_3);
                self.flags &= !0x0300;
                self.action = Action::Run;
                return;
            }
            if self.flags & 8 == 0 {
                self.change_anim(ENDAIKUKAKARIKO_ANIM_0);
            }
            if self.flags & 0x800 == 0 {
                self.flags &= !0x0200;
                self.flags |= 0x100;
            }
            self.action = Action::Wait;
        }
    }

    /// `EnDaikuKakariko_Wait`.
    fn wait(&mut self, play: &mut PlayState) {
        self.handle_talking(play);
        if self.skel.update() {
            self.change_anim(ENDAIKUKAKARIKO_ANIM_0);
        }
        if self.talk_state != 0 {
            if self.flags & 8 == 0 {
                self.change_anim(ENDAIKUKAKARIKO_ANIM_4);
            }
            if self.flags & 0x800 == 0 {
                self.flags |= 0x200;
                self.flags &= !0x0100;
            }
            self.action = Action::Talk;
        }
    }

    /// `EnDaikuKakariko_StopRunning`: `timer` plays of the once-animation, then running again.
    fn stop_running(&mut self, play: &mut PlayState) {
        if self.skel.update() {
            self.timer -= 1;
            if self.timer <= 0 {
                self.change_anim(ENDAIKUKAKARIKO_ANIM_3);
                self.action = Action::Run;
            } else {
                self.skel.cur_frame = self.skel.start_frame;
            }
        }
        self.handle_talking(play);
        if self.talk_state != 0 {
            self.flags |= 0x200;
            self.change_anim(ENDAIKUKAKARIKO_ANIM_4);
            self.action = Action::Talk;
        }
    }

    /// `EnDaikuKakariko_Run`: along the path, turning on the spot at each point first; at an end,
    /// back (0x20) or round again.
    fn run_path(&mut self, play: &mut PlayState) {
        let Some(path) = play.setup_path_list().get(self.path_index()).cloned() else {
            log::error!("En_Daiku_Kakariko: no path {}", self.path_index());
            return;
        };
        let count = path.count() as i32;
        let (mut run_angle, mut run_dist);
        loop {
            let p = path.point(self.waypoint as usize);
            let x_dist = p.x - self.actor.world_pos.x;
            let z_dist = p.z - self.actor.world_pos.z;
            run_angle = rad_to_binang_trunc(oot_game::camera::f_atan2f(x_dist, z_dist));
            run_dist = (x_dist * x_dist + z_dist * z_dist).sqrt();
            let mut run = false;
            if run_dist <= 10.0 {
                if !self.path_continue {
                    self.waypoint += 1;
                    if self.waypoint >= count {
                        if self.flags & 0x20 != 0 {
                            self.waypoint = count - 2;
                            self.path_continue = true;
                            self.run = false;
                            run = false;
                            if self.flags & 0x400 != 0 {
                                self.timer = 2;
                                self.change_anim(ENDAIKUKAKARIKO_ANIM_0);
                                self.action = Action::StopRunning;
                                return;
                            }
                        } else {
                            self.waypoint = 0;
                            run = true;
                        }
                    } else {
                        self.run = true;
                        run = true;
                    }
                } else {
                    self.waypoint -= 1;
                    if self.waypoint < 0 {
                        self.waypoint = 1;
                        self.path_continue = false;
                        self.run = false;
                        run = false;
                        if self.flags & 0x400 != 0 {
                            self.timer = 2;
                            self.change_anim(ENDAIKUKAKARIKO_ANIM_0);
                            self.action = Action::StopRunning;
                            return;
                        }
                    } else {
                        run = true;
                    }
                }
            }
            if !run {
                break;
            }
        }
        let angle_step_diff = smooth_step_to_s(&mut self.actor.shape_rot.y, run_angle, 1, 5000, 0);
        self.actor.world_rot.y = self.actor.shape_rot.y;
        if !self.run {
            if angle_step_diff == 0 {
                self.run = true;
            } else {
                self.actor.speed_xz = 0.0;
            }
        }
        if self.run {
            smooth_step_to_f(&mut self.actor.speed_xz, self.run_speed, 0.8, run_dist, 0.0);
        }
        self.actor.move_forward();
        if self.flags & 0x40 != 0 {
            self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
        } else if self.flags & 0x80 != 0 {
            self.run_flag |= 1;
            self.flags &= !0x0080;
        } else if self.run_flag & 1 != 0 {
            self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
            self.run_flag &= !1;
        }
        self.skel.update();
        self.handle_talking(play);
        if self.talk_state != 0 {
            self.flags |= 0x200;
            self.change_anim(ENDAIKUKAKARIKO_ANIM_4);
            self.action = Action::Talk;
        }
    }

    /// The limb matrices with `EnDaikuKakariko_OverrideLimbDraw`'s turns: limb 8 by the torso's,
    /// limb 15 by the head's (and the neck's, 0x1000) about a point 1400 along it.
    /// `a`: the head's x and y, the torso's x and y, the neck's x and y, and 0x1000 set.
    pub fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], a: &[i16]) -> Vec<Mat4> {
        let (head, torso, neck, neck_on) = ([a[0], a[1]], [a[2], a[3]], [a[4], a[5]], a[6] != 0);
        let r = |v: i32| binang_to_rad_i32(v);
        skeleton.pose_override(joints, |limb, _pos, _rot| match limb {
            TORSO_LIMB => Mat4::from_rotation_x(-binang_to_rad(torso[1])) * Mat4::from_rotation_z(-binang_to_rad(torso[0])),
            HEAD_LIMB => {
                let (ry, rx) = if neck_on { (head[1] as i32 + neck[1] as i32, head[0] as i32 + neck[0] as i32) } else { (head[1] as i32, head[0] as i32) };
                Mat4::from_translation(Vec3::new(1400.0, 0.0, 0.0)) * Mat4::from_rotation_x(r(ry)) * Mat4::from_rotation_z(r(rx)) * Mat4::from_translation(Vec3::new(-1400.0, 0.0, 0.0))
            }
            _ => Mat4::IDENTITY,
        })
    }

    fn pose_angles(&self) -> Vec<i16> {
        let t = &self.interact_info;
        vec![t.head[0], t.head[1], t.torso[0], t.torso[1], self.neck_angle[0], self.neck_angle[1], (self.flags & 0x1000 != 0) as i16]
    }
}

/// `BINANG_TO_RAD` of an int sum (the C adds two s16 as ints).
fn binang_to_rad_i32(a: i32) -> f32 {
    a as f32 * (std::f32::consts::PI / 32768.0)
}

impl ActorImpl for EnDaikuKakariko {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnDaikuKakariko_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if self.current_anim_index == 3 {
            let f = self.skel.cur_frame as i32;
            if f == 6 || f == 15 {
                audio_play_actor_sfx2(play, NA_SE_EN_MORIBLIN_WALK);
            }
        }
        self.collider.update(&self.actor);
        if self.flags & 4 != 0 {
            self.collider.dim.pos[0] -= 27;
            self.collider.dim.pos[2] -= 27;
            self.collider.dim.radius = 63;
        }
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        match self.action {
            Action::Wait => self.wait(play),
            Action::Talk => self.talk(play),
            Action::StopRunning => self.stop_running(play),
            Action::Run => self.run_path(play),
        }
        let player_focus = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.focus_pos).unwrap_or(self.actor.focus_pos);
        self.interact_info.target = player_focus;
        if self.flags & 0x100 != 0 {
            self.neck_angle_target[0] = 5900;
            self.flags |= 0x1000;
            let mut t = self.interact_info;
            track_point(play, &mut self.actor, &mut t, 0, NPC_TRACKING_HEAD_AND_TORSO);
            self.interact_info = t;
        } else if self.flags & 0x200 != 0 {
            self.neck_angle_target[0] = 5900;
            self.flags |= 0x1000;
            let mut t = self.interact_info;
            track_point(play, &mut self.actor, &mut t, 0, NPC_TRACKING_FULL_BODY);
            self.interact_info = t;
        }
        smooth_step_to_s(&mut self.neck_angle[0], self.neck_angle_target[0], 1, 1820, 0);
    }
    /// `EnDaikuKakariko_PostLimbDraw`, limb 15: the focus.
    fn draw_update(&mut self, _play: &mut PlayState) {
        let bones = Self::pose(&self.skeleton, &self.skel.joint_table, &self.pose_angles());
        if let Some(head) = bones.get(HEAD_LIMB - 1) {
            let m = oot_game::play::actor_draw_matrix(&RenderState::of(&self.actor));
            self.actor.focus_pos = (m * *head).transform_point3(FOCUS_OFFSET);
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.angles = self.pose_angles();
        rs
    }
    /// `EnDaikuKakariko_Draw`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        if rs.angles.len() < 7 {
            return;
        }
        let ty = self.ty() as usize;
        let bones = Self::pose(&self.skeleton, &joints.rot, &rs.angles);
        let m = oot_game::play::actor_draw_matrix(rs);
        let head = bones.get(HEAD_LIMB - 1).copied();
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(ty))), transform: m, bones, params: Default::default() });
        // EnDaikuKakariko_PostLimbDraw, limb 15: the type's head on it.
        if let Some(head) = head {
            out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(&head_bake_name(ty))), m * head));
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
