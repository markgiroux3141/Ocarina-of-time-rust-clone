//! `En_Niw_Girl` (`ovl_En_Niw_Girl/z_en_niw_girl.c`): the girl who chases a cucco round the
//! market (and Kakariko). `(params >> 8) & 0xFF` is her path.
//!
//! She spawns her cucco (`En_Niw` type 0xA) 50 ahead of her, then by turns jumps on the spot,
//! where Link can talk to her (`EnNiwGirl_Talk`, `func_80AB94D0`, her whole body turned to him),
//! and chases it (`func_80AB9210`, 250 to 350 frames): near it (within 70) she gives it her path
//! to run (`EnNiw`'s `path` is hers + 1), and takes it back when it's far (beyond 150) or a
//! message is up.
//!
//! Drawn as `EnNiwGirl_Draw` does: her eyes on segment 8, her torso and head turned
//! (`EnNiwGirlOverrideLimbDraw`), and a circle shadow (`func_80033C30`, 0.2); from meshes baked
//! per eye (docs/adr/0012-actor-bakes.md).
//!
//! Params: `path` bits 8 to 15 (her path, given to her cucco as `path + 1`; a negative params,
//! a path of 0x80 and up, is made 0 first).
//! Flags: reads EVENTCHKINF_ZELDA_FLED_CASTLE (her text: 0x70EA).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{approach_f, approach_zero_f, binang_to_rad, smooth_step_to_s};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, UPDBGCHECKINFO_FLAG_2, UPDBGCHECKINFO_FLAG_3, UPDBGCHECKINFO_FLAG_4};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorHandle, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind};
use oot_game::event_chk_inf;
use oot_game::camera::f_atan2f;
use oot_game::collision_check::*;
use oot_game::message::TEXT_STATE_NONE;
use oot_game::npc::{NpcTrack, offer_talk, process_talk_request, track_point};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::path::rad_to_binang_trunc;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::*;

use crate::en_niw::{ACTOR_EN_NIW, EnNiw};

/// `ACTOR_EN_NIW_GIRL` (`actor_table.h`: 0x019A).
pub const ACTOR_EN_NIW_GIRL: i16 = 0x019A;
pub const OBJECT: &str = "object_gr";

/// `En_Niw_Girl_Profile`.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_EN_NIW_GIRL, name: "En_Niw_Girl", category: ACTORCAT_NPC, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `EVENTCHKINF_ZELDA_FLED_CASTLE` (`save.h`: 0x80).
const EVENTCHKINF_ZELDA_FLED_CASTLE: u16 = 0x80;

/// `PARAMS_GET_U(this->actor.params, 8, 8)`: her path.
pub const PATH: Field = Field::params("path", 8, 8, Kind::PathIndex, "the path her cucco runs (0x80 and up: the params made 0, path 0)");

/// En_Niw_Girl's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[PATH],
    flags: &[FlagUse::new(event_chk_inf!(EVENTCHKINF_ZELDA_FLED_CASTLE)).reads().note("her text")],
    todo: false,
};
/// `ATTENTION_RANGE_6`.
const ATTENTION_RANGE_6: u8 = 6;
/// `NPC_TRACKING_FULL_BODY`.
const NPC_TRACKING_FULL_BODY: i16 = 4;

// `PlayerMask` (`player.h`).
const PLAYER_MASK_NONE: u8 = 0;
const PLAYER_MASK_KEATON: u8 = 1;
const PLAYER_MASK_SKULL: u8 = 2;
const PLAYER_MASK_SPOOKY: u8 = 3;
const PLAYER_MASK_BUNNY: u8 = 4;
const PLAYER_MASK_GORON: u8 = 5;
const PLAYER_MASK_ZORA: u8 = 6;
const PLAYER_MASK_GERUDO: u8 = 7;
const PLAYER_MASK_TRUTH: u8 = 8;

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
    dim: Cylinder16 { radius: 10, height: 30, y_shift: 0, pos: [0; 3] },
};

/// `eyeTextures`.
const EYE_TEXTURES: [&str; 3] = ["gNiwGirlEyeOpenTex", "gNiwGirlEyeHalfTex", "gNiwGirlEyeClosedTex"];

/// The bake drawn with eye `eye`.
pub fn bake_name(eye: usize) -> String {
    format!("En_Niw_Girl/eye{eye}")
}

/// The meshes `EnNiwGirl_Draw` draws: the skeleton with each eye.
pub fn bakes() -> Vec<MeshBake> {
    (0..EYE_TEXTURES.len())
        .map(|eye| MeshBake {
            name: bake_name(eye),
            object: OBJECT.into(),
            segments: vec![(0x08, BakeSegment::Texture { file: OBJECT.into(), symbol: EYE_TEXTURES[eye].into() })],
            prelude: Vec::new(),
            body: BakeBody::Skeleton { file: OBJECT.into(), symbol: "gNiwGirlSkel".into(), limbs: Vec::new() },
        })
        .collect()
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `EnNiwGirl_Jump`: off to chase.
    Jump,
    /// `func_80AB9210`: chasing the cucco.
    Chase,
    /// `EnNiwGirl_Talk`: stopping to jump and talk.
    Talk,
    /// `func_80AB94D0`: jumping, talking.
    Func80AB94D0,
}

pub struct EnNiwGirl {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub action: Action,
    pub head_rot: [i16; 3],
    pub torso_rot: [i16; 3],
    /// `jumpTimer`: how many frames she jumps for and how long until she jumps again.
    pub jump_timer: i16,
    pub unk_up_timer: i16,
    pub unk_270: i16,
    pub eye_index: i16,
    pub blink_timer: i16,
    pub path: i16,
    pub unk_278: i16,
    /// `unk_27A`: the "Zelda fled" text heard.
    pub unk_27a: i16,
    pub unk_27c: f32,
    pub unk_280: f32,
    /// `chasedEnNiw`.
    pub chased_en_niw: Option<ActorHandle>,
    pub collider: ColliderCylinder,
    pub interact_info: NpcTrack,
    run: Anim,
    jump: Anim,
}

impl EnNiwGirl {
    /// `EnNiwGirl_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = (|| -> anyhow::Result<_> { Ok((assets.skeleton(OBJECT, "gNiwGirlSkel")?, assets.animation(OBJECT, "gNiwGirlRunAnim")?, assets.animation(OBJECT, "gNiwGirlJumpAnim")?)) })();
        let (skeleton, run, jump) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Niw_Girl: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let mut this = EnNiwGirl {
            // SkelAnime_InitFlex(.., gNiwGirlSkel, gNiwGirlRunAnim, .., 17).
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(run.clone())),
            skeleton,
            action: Action::Talk,
            head_rot: [0; 3],
            torso_rot: [0; 3],
            jump_timer: 0,
            unk_up_timer: 0,
            unk_270: 0,
            eye_index: 0,
            blink_timer: 0,
            path: 0,
            unk_278: 0,
            unk_27a: 0,
            unk_27c: 0.0,
            unk_280: 0.0,
            chased_en_niw: None,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            interact_info: NpcTrack::default(),
            run,
            jump,
            actor,
        };
        this.actor.target_mode = ATTENTION_RANGE_6;
        // schema: the whole params' sign (`path` 0x80 and up)
        if this.actor.params < 0 {
            this.actor.params = 0;
        }
        this.path = PATH.get(this.actor.params);
        this.actor.gravity = -3.0;
        // Matrix_RotateY(shape.rot.y), Matrix_MultVec3f((0, 0, 50)).
        let r = binang_to_rad(this.actor.shape_rot.y);
        let vec2 = Vec3::new(50.0 * r.sin(), 0.0, 50.0 * r.cos());
        let pos = this.actor.world_pos + vec2;
        let rot = [0, this.actor.world_rot.y, 0];
        match play.actor_spawn_as_child(&mut this.actor, ACTOR_EN_NIW, pos, rot, 0xA) {
            Ok(h) => {
                // schema: printed
                log::debug!("En_Niw_Girl: That's so mean! Punpun {}; Kyahahahaha, wait {}", this.actor.params, this.path);
                this.chased_en_niw = Some(h);
                this.actor.col_chk_info.mass = MASS_IMMOVABLE;
                this.action = Action::Talk;
            }
            Err(_) => {
                // schema: printed
                log::debug!("En_Niw_Girl: for some reason, I can't set it up {}", this.actor.params);
                this.actor.kill();
            }
        }
        Box::new(this)
    }

    /// `this->chasedEnNiw->path = path` (with `unk_2EC` when given).
    fn set_niw_path(&self, play: &mut PlayState, path: i16, count: Option<i16>) {
        if let Some(niw) = self.chased_en_niw.and_then(|h| play.actors.get_mut(h)).and_then(|a| a.as_any_mut().downcast_mut::<EnNiw>()) {
            niw.path = path;
            if let Some(c) = count {
                niw.unk_2ec = c;
            }
        }
    }

    fn current_mask(play: &PlayState) -> u8 {
        play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map(|p| p.current_mask()).unwrap_or(PLAYER_MASK_NONE)
    }

    /// `EnNiwGirl_Jump`.
    fn jump(&mut self) {
        let frame_count = self.run.last_frame();
        self.skel.change(self.run.clone(), 1.0, 0.0, frame_count, ANIMMODE_LOOP, -10.0);
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.action = Action::Chase;
    }

    /// `func_80AB9210`: after her cucco, giving it her path when she's close.
    fn chase(&mut self, play: &mut PlayState) {
        // Path* path = &play->pathList[this->path].
        let count = play.setup_path_list().get(self.path as usize).map(|p| p.count() as i16);
        self.skel.update();
        approach_f(&mut self.actor.speed_xz, 3.0, 0.2, 0.4);

        // The X and Z distance between the girl and the cucco she is chasing.
        let niw_pos = self.chased_en_niw.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(self.actor.world_pos);
        let x_dist_between = niw_pos.x - self.actor.world_pos.x;
        let z_dist_between = niw_pos.z - self.actor.world_pos.z;
        if play.message_state() != TEXT_STATE_NONE {
            self.set_niw_path(play, 0, None);
        }
        let dist = (x_dist_between * x_dist_between + z_dist_between * z_dist_between).sqrt();
        if dist < 70.0 {
            match count {
                Some(c) => self.set_niw_path(play, self.path + 1, Some(c)),
                None => log::warn!("En_Niw_Girl: no path {}", self.path),
            }
        } else if dist > 150.0 {
            self.set_niw_path(play, 0, None);
        }

        // Always facing the cucco.
        smooth_step_to_s(&mut self.actor.shape_rot.y, rad_to_binang_trunc(f_atan2f(x_dist_between, z_dist_between)), 3, self.unk_27c as i32 as i16, 0);
        approach_f(&mut self.unk_27c, 5000.0, 30.0, 150.0);
        self.actor.world_rot.y = self.actor.shape_rot.y;

        // Link may only talk to her while she jumps.
        if self.jump_timer == 0 || Self::current_mask(play) != PLAYER_MASK_NONE {
            self.jump_timer = 60;
            self.action = Action::Talk;
        }
    }

    /// `EnNiwGirl_Talk`: the jump, and her text.
    fn talk(&mut self, play: &PlayState) {
        let frame_count = self.jump.last_frame();
        self.skel.change(self.jump.clone(), 1.0, 0.0, frame_count, ANIMMODE_LOOP, -10.0);
        self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
        self.actor.text_id = 0x7000;
        if play.save.get_event_chk_inf(EVENTCHKINF_ZELDA_FLED_CASTLE) && self.unk_27a == 0 {
            self.actor.text_id = 0x70EA;
        }
        match Self::current_mask(play) {
            PLAYER_MASK_KEATON => self.actor.text_id = 0x7118,
            PLAYER_MASK_SPOOKY => self.actor.text_id = 0x7119,
            PLAYER_MASK_BUNNY | PLAYER_MASK_ZORA | PLAYER_MASK_GERUDO => self.actor.text_id = 0x711A,
            PLAYER_MASK_SKULL | PLAYER_MASK_GORON | PLAYER_MASK_TRUTH => self.actor.text_id = 0x711B,
            _ => {}
        }
        self.unk_270 = 6;
        self.action = Action::Func80AB94D0;
    }

    /// `func_80AB94D0`: jumping on the spot, offering to talk, until it's time to chase.
    fn func_80ab94d0(&mut self, play: &mut PlayState) {
        self.skel.update();
        if play.message_state() != TEXT_STATE_NONE {
            self.set_niw_path(play, 0, None);
        }
        approach_zero_f(&mut self.actor.speed_xz, 0.8, 0.2);
        if process_talk_request(&mut self.actor) {
            if self.actor.text_id == 0x70EA {
                self.unk_27a = 1;
            }
        } else if self.jump_timer == 0 && play.message_state() == TEXT_STATE_NONE {
            self.jump_timer = (play.rand.zero_float(100.0) + 250.0) as i16;
            self.action = Action::Jump;
        } else {
            let a = self.actor.clone();
            offer_talk(play, &a, 100.0);
        }
    }

    /// The limb matrices `EnNiwGirl_Draw` draws with: `EnNiwGirlOverrideLimbDraw`'s turns.
    fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], head: [i16; 3], torso: [i16; 3]) -> Vec<Mat4> {
        skeleton.pose_override(joints, |limb, _pos, rot| {
            if limb == 3 {
                rot[0] = rot[0].wrapping_add(torso[1]);
            }
            if limb == 4 {
                rot[0] = rot[0].wrapping_add(head[1]);
                rot[2] = rot[2].wrapping_add(head[2]);
            }
            Mat4::IDENTITY
        })
    }
}

impl ActorImpl for EnNiwGirl {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnNiwGirl_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.actor.scale = Vec3::splat(0.013);
        self.unk_up_timer = self.unk_up_timer.wrapping_add(1);
        if self.blink_timer == 0 {
            self.eye_index += 1;
            if self.eye_index >= 3 {
                self.eye_index = 0;
                self.blink_timer = (play.rand.zero_float(60.0) as i16).wrapping_add(20);
            }
        }
        self.unk_280 = 30.0;
        self.actor.set_focus(30.0);
        if self.action == Action::Func80AB94D0 {
            let pp = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(self.actor.world_pos);
            self.interact_info.target = pp;
            if !play.save.adult {
                self.interact_info.target.y = pp.y - 10.0;
            }
            let mut t = self.interact_info;
            track_point(play, &mut self.actor, &mut t, 2, NPC_TRACKING_FULL_BODY);
            self.interact_info = t;
            self.head_rot = self.interact_info.head;
            self.torso_rot = self.interact_info.torso;
        } else {
            smooth_step_to_s(&mut self.torso_rot[1], 0, 5, 3000, 0);
            smooth_step_to_s(&mut self.head_rot[1], 0, 5, 3000, 0);
            smooth_step_to_s(&mut self.head_rot[2], 0, 5, 3000, 0);
        }
        if self.blink_timer != 0 {
            self.blink_timer -= 1;
        }
        if self.jump_timer != 0 {
            self.jump_timer -= 1;
        }
        match self.action {
            Action::Jump => self.jump(),
            Action::Chase => self.chase(play),
            Action::Talk => self.talk(play),
            Action::Func80AB94D0 => self.func_80ab94d0(play),
        }
        self.actor.move_forward();
        self.actor.update_bg_check_info(&play.col, 100.0, 100.0, 200.0, UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.angles = vec![self.head_rot[0], self.head_rot[1], self.head_rot[2], self.torso_rot[0], self.torso_rot[1], self.torso_rot[2]];
        rs.switches = vec![self.eye_index.clamp(0, 2) as u32];
        rs
    }
    /// `EnNiwGirl_Draw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        if rs.angles.len() < 6 || rs.switches.is_empty() {
            return;
        }
        let a = &rs.angles;
        let bones = Self::pose(&self.skeleton, &joints.rot, [a[0], a[1], a[2]], [a[3], a[4], a[5]]);
        let eye = (rs.switches[0] as usize).min(EYE_TEXTURES.len() - 1);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(eye))), transform: oot_game::play::actor_draw_matrix(rs), bones, params: Default::default() });
        // func_80033C30(&world.pos, (0.2, 0.2, 0.2), 255): the circle shadow's stand-in.
        let (floor, _) = play.col.entity_raycast_down(rs.pos + Vec3::Y * 20.0);
        let shadow = Mat4::from_translation(Vec3::new(rs.pos.x, floor + 0.3, rs.pos.z)) * Mat4::from_scale(Vec3::new(20.0, 1.0, 20.0));
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
