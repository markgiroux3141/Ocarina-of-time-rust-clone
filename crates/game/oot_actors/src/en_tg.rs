//! `En_Tg` (`ovl_En_Tg/z_en_tg.c`): the dancing couple, in the market and in Kakariko.
//!
//! They spin (`EnTg_SpinIfNotTalking`: 0x800 a frame) until Link talks to them; their two texts
//! alternate from a start by the frame they were spawned on (`nextDialogue`).
//!
//! Drawn as `EnTg_Draw` does: the skeleton shifted by (0, 0, -560), his shoes and shirt royal blue
//! (segment 8) and her shirt white (segment 9); the focus 800 up his head (limb 9,
//! `EnTg_PostLimbDraw`). From a baked mesh (docs/adr/0012-actor-bakes.md).
//!
//! Not ported here: the mask reactions (`MaskReaction_GetTextId`: no masks yet, so none).
//!
//! Params: none (the data places 0; the market's texts flip bit 0, `params ^= 1`, and nothing
//! reads it).
//! Flags: none.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, Actor, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile};
use oot_game::collision_check::*;
use oot_game::message::TEXT_STATE_CLOSING;
use oot_game::npc::talk_update;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::*;

/// `ACTOR_EN_TG` (`actor_table.h`: 0x01AC).
pub const ACTOR_EN_TG: i16 = 0x01AC;
pub const OBJECT: &str = "object_mu";

/// `En_Tg_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_TG, name: "En_Tg", category: ACTORCAT_NPC, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY, object: OBJECT, schema: &oot_game::actor_schema::ActorSchema::NONE };

/// `SCENE_KAKARIKO_VILLAGE` (`scene_table.h`: 0x52).
const SCENE_KAKARIKO_VILLAGE: u16 = 0x52;
/// `ATTENTION_RANGE_6`.
const ATTENTION_RANGE_6: u8 = 6;
/// `NpcTalkState` (`actor.h`).
const NPC_TALK_STATE_IDLE: i16 = 0;
const NPC_TALK_STATE_TALKING: i16 = 1;
/// The man's head (1-based), where `EnTg_PostLimbDraw` puts the focus.
const HEAD_LIMB: usize = 9;

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
    dim: Cylinder16 { radius: 20, height: 64, y_shift: 0, pos: [0; 3] },
};

/// `sColChkInfoInit`: immovable.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit2 = CollisionCheckInfoInit2 { health: 0, cyl_radius: 0, cyl_height: 0, cyl_y_shift: 0, mass: MASS_IMMOVABLE };

/// `EnTg_Draw`'s `Matrix_Translate(0, 0, -560)`.
const MODEL_OFFSET: Vec3 = Vec3::new(0.0, 0.0, -560.0);

/// The couple's mesh.
pub const BAKE: &str = "En_Tg/skel";

/// The mesh `EnTg_Draw` draws: `gDancingCoupleSkel` with his blue (segment 8) and her white
/// (segment 9).
pub fn bakes() -> Vec<MeshBake> {
    let env = |c: [u8; 4]| BakeSegment::Commands(vec![(0xFB00_0000, u32::from_be_bytes(c))]);
    vec![MeshBake {
        name: BAKE.into(),
        object: OBJECT.into(),
        segments: vec![(0x08, env([0, 50, 160, 0])), (0x09, env([255, 255, 255, 0]))],
        prelude: Vec::new(),
        body: BakeBody::Skeleton { file: OBJECT.into(), symbol: "gDancingCoupleSkel".into(), limbs: Vec::new() },
    }]
}

pub struct EnTg {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub collider: ColliderCylinder,
    /// `interactInfo.talkState`.
    pub talk_state: i16,
    pub next_dialogue: u8,
}

impl EnTg {
    /// `EnTg_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = (|| -> anyhow::Result<_> { Ok((assets.skeleton(OBJECT, "gDancingCoupleSkel")?, assets.animation(OBJECT, "gDancingCoupleAnim")?)) })();
        let (skeleton, anim) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Tg: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let mut this = EnTg {
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(anim)),
            skeleton,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            talk_state: 0,
            next_dialogue: (play.state_frames % 2) as u8,
            actor,
        };
        // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 28).
        this.actor.shape_y_offset = 0.0;
        this.actor.col_chk_info.set_info2(None, &COL_CHK_INFO_INIT);
        this.actor.target_mode = ATTENTION_RANGE_6;
        this.actor.scale = Vec3::splat(0.01);
        Box::new(this)
    }

    /// `EnTg_GetTextId`.
    pub fn get_text_id(play: &PlayState, next_dialogue: u8) -> u16 {
        // MaskReaction_GetTextId(play, MASK_REACTION_SET_DANCING_COUPLE): no masks, so 0.
        let odd = next_dialogue % 2 != 0;
        if play.scene_id == SCENE_KAKARIKO_VILLAGE {
            if odd { 0x5089 } else { 0x508A }
        } else if odd {
            0x7025
        } else {
            0x7026
        }
    }

    /// The limb matrices: `EnTg_OverrideLimbDraw` changes nothing.
    fn pose(&self, joints: &[[i16; 3]]) -> Vec<Mat4> {
        self.skeleton.pose_override(joints, |_, _, _| Mat4::IDENTITY)
    }

    fn model_matrix(rs: &RenderState) -> Mat4 {
        oot_game::play::actor_draw_matrix(rs) * Mat4::from_translation(MODEL_OFFSET)
    }
}

impl ActorImpl for EnTg {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnTg_Update`.
    fn update(&mut self, play: &mut PlayState) {
        let p = self.actor.world_pos;
        // sp2C.z = (s16)world.pos.z + 3.
        self.collider.set_position([p.x as i16, p.y as i16, (p.z as i16).wrapping_add(3)]);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        self.skel.update();
        self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
        // EnTg_SpinIfNotTalking.
        if self.talk_state == 0 {
            self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x800);
        }
        // Npc_UpdateTalking(.., collider.dim.radius + 30, EnTg_GetTextId, EnTg_UpdateTalkState).
        let temp = self.collider.dim.radius as f32 + 30.0;
        let mut talk_state = self.talk_state;
        let next_dialogue = self.next_dialogue;
        let mut advance = false;
        talk_update(
            play,
            &mut self.actor,
            &mut talk_state,
            temp,
            |play, _| Self::get_text_id(play, next_dialogue),
            |play, actor| {
                // EnTg_UpdateTalkState.
                if play.message_state() != TEXT_STATE_CLOSING {
                    return NPC_TALK_STATE_TALKING;
                }
                match actor.text_id {
                    0x5089 | 0x508A => advance = true,
                    0x7025 | 0x7026 => {
                        actor.params ^= 1;
                        advance = true;
                    }
                    _ => {}
                }
                NPC_TALK_STATE_IDLE
            },
        );
        self.talk_state = talk_state;
        if advance {
            self.next_dialogue = self.next_dialogue.wrapping_add(1);
        }
    }
    /// `EnTg_PostLimbDraw`, the man's head: the focus 800 up it.
    fn draw_update(&mut self, _play: &mut PlayState) {
        let bones = self.pose(&self.skel.joint_table);
        if let Some(head) = bones.get(HEAD_LIMB - 1) {
            let m = Self::model_matrix(&RenderState::of(&self.actor));
            self.actor.focus_pos = (m * *head).transform_point3(Vec3::new(0.0, 800.0, 0.0));
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs
    }
    /// `EnTg_Draw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        let bones = self.pose(&joints.rot);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE)), transform: Self::model_matrix(rs), bones, params: Default::default() });
        // ActorShadow_DrawCircle (shadowScale 28): the circle shadow's stand-in.
        let (floor, _) = play.col.entity_raycast_down(rs.pos + Vec3::Y * 20.0);
        let shadow = Mat4::from_translation(Vec3::new(rs.pos.x, floor + 0.3, rs.pos.z)) * Mat4::from_scale(Vec3::new(28.0, 1.0, 28.0));
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
