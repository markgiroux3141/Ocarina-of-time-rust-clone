//! `En_Hs2` (`ovl_En_Hs2/z_en_hs2.c`): the carpenter's son as a child sees him, sitting under
//! a tree in Kakariko at night ("Chick's Shop (Children's Time)", the C's print says).
//!
//! He sits (`gCarpenterSonSittingAnim`, from its first frame again at its end) and offers to talk
//! (0x5069) when Link is within 100 and within 0x2151 of his front (`EnHs2_Interact`); then his
//! head and torso follow Link (`Actor_TrackPlayer`), else they ease back to rest (the head 12800
//! down). Placed only on Kakariko's night layer: his init keeps him anywhere.
//!
//! Drawn as `EnHs2_Draw` does: the skeleton without limbs 10 to 13 (`EnHs2_OverrideLimbDraw`
//! empties their lists), the head (limb 9) turned; the focus 300 along and 1000 up it
//! (`EnHs2_PostLimbDraw`). From a baked mesh (docs/adr/0012-actor-bakes.md), which is set up with
//! `Gfx_SetupDL_25Opa` where the draw uses `Gfx_SetupDL_37Opa`.
//!
//! Not ported here: the mask reactions (`MaskReaction_GetTextId`: no masks yet, so none).
//!
//! Params: none read (placed as 0xFFFF, unused).
//! Flags: none.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::smooth_step_to_s;
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, Actor, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::collision_check::*;
use oot_game::npc::{actor_track_player, offer_talk, process_talk_request, textbox_is_closing};
use oot_game::pack::{BakeBody, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::*;

/// `ACTOR_EN_HS2` (`actor_table.h`: 0x01A6).
pub const ACTOR_EN_HS2: i16 = 0x01A6;
/// `OBJECT_HS` (`object_table.h`: 0x0128).
pub const OBJECT: &str = "object_hs";

/// `En_Hs2_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_HS2, name: "En_Hs2", category: ACTORCAT_NPC, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY, object: OBJECT, schema: &SCHEMA };

/// The placed params, read by nothing.
pub const PARAMS: Field = Field::whole("params", Kind::Unused, "placed as 0xFFFF; read by nothing");

/// En_Hs2's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[PARAMS], flags: &[], todo: false };

/// `ENHS2_TRACKING_PLAYER`.
pub const ENHS2_TRACKING_PLAYER: u16 = 1 << 0;
/// `ATTENTION_RANGE_6`.
const ATTENTION_RANGE_6: u8 = 6;
/// The head's limb (1-based).
const HEAD_LIMB: usize = 9;
/// `sFocusOffset`.
const FOCUS_OFFSET: Vec3 = Vec3::new(300.0, 1000.0, 0.0);

/// `sCylinderInit`.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_ENEMY, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: 0, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: 0, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 40, height: 40, y_shift: 0, pos: [0; 3] },
};

/// The son's mesh.
pub const BAKE: &str = "En_Hs2/skel";

/// The mesh `EnHs2_Draw` draws: `gCarpenterSonSkel` without limbs 10 to 13.
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: BAKE.into(),
        object: OBJECT.into(),
        segments: Vec::new(),
        prelude: Vec::new(),
        body: BakeBody::Skeleton {
            file: OBJECT.into(),
            symbol: "gCarpenterSonSkel".into(),
            // EnHs2_OverrideLimbDraw: *dList = NULL for limbs 10, 11, 12 and 13.
            limbs: (10..=13).map(|l: u8| LimbOverride { limb: l - 1, file: OBJECT.into(), symbol: String::new() }).collect(),
        },
    }]
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `EnHs2_Idle`.
    Idle,
    /// `EnHs2_Talking`.
    Talking,
}

pub struct EnHs2 {
    pub actor: Actor,
    pub collider: ColliderCylinder,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub head_rot: [i16; 3],
    pub torso_rot: [i16; 3],
    pub tracking_flags: u16,
    pub action: Action,
}

impl EnHs2 {
    /// `EnHs2_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = (|| -> anyhow::Result<_> { Ok((assets.skeleton(OBJECT, "gCarpenterSonSkel")?, assets.animation(OBJECT, "gCarpenterSonSittingAnim")?)) })();
        let (skeleton, anim) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Hs2: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        // SkelAnime_InitFlex(.., gCarpenterSonSkel, gCarpenterSonSittingAnim, .., 16);
        // Animation_PlayLoop(gCarpenterSonSittingAnim).
        let mut skel = SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(anim.clone()));
        let last = anim.last_frame();
        skel.change(anim, 1.0, 0.0, last, ANIMMODE_LOOP, 0.0);
        let mut this = EnHs2 { collider: ColliderCylinder::new(&CYLINDER_INIT), skel, skeleton, head_rot: [0; 3], torso_rot: [0; 3], tracking_flags: 0, action: Action::Idle, actor };
        // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 36).
        this.actor.shape_y_offset = 0.0;
        this.actor.col_chk_info.mass = MASS_IMMOVABLE;
        this.actor.scale = Vec3::splat(0.01);
        log::debug!("En_Hs2: Chick's Shop (Children's Time)");
        this.actor.target_mode = ATTENTION_RANGE_6;
        Box::new(this)
    }

    /// `EnHs2_Interact`: true when a talk starts (then `action`); else `text_id` offered within
    /// 100 when Link is in front.
    fn interact(&mut self, play: &mut PlayState, text_id: u16, action: Action) -> bool {
        if process_talk_request(&mut self.actor) {
            self.action = action;
            return true;
        }
        self.actor.text_id = text_id;
        let yaw = (self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y) as i32).abs();
        if yaw < 0x2151 && self.actor.xz_dist_to_player < 100.0 {
            self.tracking_flags |= ENHS2_TRACKING_PLAYER;
            let a = self.actor.clone();
            offer_talk(play, &a, 100.0);
        }
        false
    }

    /// `EnHs2_Talking`.
    fn talking(&mut self, play: &mut PlayState) {
        if textbox_is_closing(play) {
            self.action = Action::Idle;
        }
        self.tracking_flags |= ENHS2_TRACKING_PLAYER;
    }

    /// `EnHs2_Idle`.
    fn idle(&mut self, play: &mut PlayState) {
        // MaskReaction_GetTextId(play, MASK_REACTION_SET_CARPENTERS_SON): no masks, so 0.
        let text_id = 0x5069;
        self.interact(play, text_id, Action::Talking);
    }

    /// The pose with `EnHs2_OverrideLimbDraw`'s head turn: limb 9's x by the head's y, its z by
    /// the head's x.
    pub fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], head: [i16; 3]) -> Vec<Mat4> {
        skeleton.pose_override(joints, |limb, _pos, rot| {
            if limb == HEAD_LIMB {
                rot[0] = rot[0].wrapping_add(head[1]);
                rot[2] = rot[2].wrapping_add(head[0]);
            }
            Mat4::IDENTITY
        })
    }
}

impl ActorImpl for EnHs2 {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnHs2_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        self.actor.move_forward();
        self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
        if self.skel.update() {
            self.skel.cur_frame = 0.0;
        }
        match self.action {
            Action::Idle => self.idle(play),
            Action::Talking => self.talking(play),
        }
        if self.tracking_flags & ENHS2_TRACKING_PLAYER != 0 {
            let focus = self.actor.focus_pos;
            actor_track_player(play, &mut self.actor, &mut self.head_rot, &mut self.torso_rot, focus);
            self.tracking_flags &= !ENHS2_TRACKING_PLAYER;
        } else {
            smooth_step_to_s(&mut self.head_rot[0], 12800, 6, 6200, 100);
            smooth_step_to_s(&mut self.head_rot[1], 0, 6, 6200, 100);
            smooth_step_to_s(&mut self.torso_rot[0], 0, 6, 6200, 100);
            smooth_step_to_s(&mut self.torso_rot[1], 0, 6, 6200, 100);
        }
    }
    /// `EnHs2_PostLimbDraw`, limb 9: the focus.
    fn draw_update(&mut self, _play: &mut PlayState) {
        let bones = Self::pose(&self.skeleton, &self.skel.joint_table, self.head_rot);
        if let Some(head) = bones.get(HEAD_LIMB - 1) {
            let m = oot_game::play::actor_draw_matrix(&RenderState::of(&self.actor));
            self.actor.focus_pos = (m * *head).transform_point3(FOCUS_OFFSET);
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.angles = self.head_rot.to_vec();
        rs
    }
    /// `EnHs2_Draw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        if rs.angles.len() < 3 {
            return;
        }
        let bones = Self::pose(&self.skeleton, &joints.rot, [rs.angles[0], rs.angles[1], rs.angles[2]]);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE)), transform: oot_game::play::actor_draw_matrix(rs), bones, params: Default::default() });
        // ActorShadow_DrawCircle (shadowScale 36): the circle shadow's stand-in.
        let (floor, _) = play.col.entity_raycast_down(rs.pos + Vec3::Y * 20.0);
        let shadow = Mat4::from_translation(Vec3::new(rs.pos.x, floor + 0.3, rs.pos.z)) * Mat4::from_scale(Vec3::new(36.0, 1.0, 36.0));
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
