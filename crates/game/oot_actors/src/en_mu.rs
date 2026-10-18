//! `En_Mu` (`ovl_En_Mu/z_en_mu.c`): the haggling townsfolk, the two by the market's stall arguing
//! over a price. `params` (0 or 1) picks their clothes' colours.
//!
//! They stand and fidget (`EnMu_Pose`) and talk (`Npc_UpdateTalking`): each talk picks the next of
//! five texts not yet heard (`EnMu_Interact`, from a random start), remembered in
//! `eventInf[EVENTINF_INDEX_HAGGLING_TOWNSFOLK]` and shared by both.
//!
//! Drawn as `EnMu_Draw` does: the skeleton shifted by (-1200, 0, -1400), the five env colours on
//! segments 8 to 12 by `params`, and the fidget's sway; from meshes baked per colour set
//! (docs/adr/0012-actor-bakes.md).
//!
//! Not ported here: the mask reactions (`MaskReaction_GetTextId`: no masks yet, so none).
//!
//! Params: `params` whole (0 or 1: the colour set, `colors[params]`; also the mask reactions' set,
//! `MASK_REACTION_SET_HAGGLING_TOWNSPEOPLE_1 + params`).
//! Flags: reads, clears and sets EVENTINF_HAGGLING_TOWNSFOLK_MESG_0 to _4 (the five texts heard,
//! shared by both; all five heard, they start again).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{cos_s, sin_s};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, Actor, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind};
use oot_game::event_inf;
use oot_game::collision_check::*;
use oot_game::message::TEXT_STATE_CLOSING;
use oot_game::npc::{actor_update_fidget_tables, talk_update};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::*;

/// `ACTOR_EN_MU` (`actor_table.h`: 0x01AD).
pub const ACTOR_EN_MU: i16 = 0x01AD;
pub const OBJECT: &str = "object_mu";

/// `En_Mu_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_MU, name: "En_Mu", category: ACTORCAT_NPC, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY, object: OBJECT, schema: &SCHEMA };

/// `this->actor.params`: the colour set (`colors[params]`).
pub const COLORS_INDEX: Field = Field::whole("params", Kind::Named(&[(0, "the first colour set"), (1, "the second colour set")]), "which of the two colour sets they wear");

/// `EVENTINF_HAGGLING_TOWNSFOLK_MESG_0` to `_4` (`save.h`: 0x20 to 0x24): the five texts heard.
const EVENTINF_HAGGLING_TOWNSFOLK_MESG_0: u16 = 0x20;
const EVENTINF_HAGGLING_TOWNSFOLK_MESG_1: u16 = 0x21;
const EVENTINF_HAGGLING_TOWNSFOLK_MESG_2: u16 = 0x22;
const EVENTINF_HAGGLING_TOWNSFOLK_MESG_3: u16 = 0x23;
const EVENTINF_HAGGLING_TOWNSFOLK_MESG_4: u16 = 0x24;
const TALK_FLAGS: &str = "the talk flags, read, cleared and set together through the eventInf word";

/// En_Mu's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[COLORS_INDEX],
    flags: &[
        FlagUse::new(event_inf!(EVENTINF_HAGGLING_TOWNSFOLK_MESG_0)).reads().sets().clears().note("text 0x7042 heard").indirect(TALK_FLAGS),
        FlagUse::new(event_inf!(EVENTINF_HAGGLING_TOWNSFOLK_MESG_1)).reads().sets().clears().note("text 0x7043 heard").indirect(TALK_FLAGS),
        FlagUse::new(event_inf!(EVENTINF_HAGGLING_TOWNSFOLK_MESG_2)).reads().sets().clears().note("text 0x703F heard").indirect(TALK_FLAGS),
        FlagUse::new(event_inf!(EVENTINF_HAGGLING_TOWNSFOLK_MESG_3)).reads().sets().clears().note("text 0x7041 heard").indirect(TALK_FLAGS),
        FlagUse::new(event_inf!(EVENTINF_HAGGLING_TOWNSFOLK_MESG_4)).reads().sets().clears().note("text 0x703E heard").indirect(TALK_FLAGS),
    ],
    todo: false,
    objects: &[],
};

/// `EVENTINF_INDEX_HAGGLING_TOWNSFOLK` (`save.h`: 0x2).
const EVENTINF_INDEX_HAGGLING_TOWNSFOLK: usize = 0x2;
/// `EVENTINF_HAGGLING_TOWNSFOLK_MASK`: `EVENTINF_MASK` of `EVENTINF_HAGGLING_TOWNSFOLK_MESG_0`
/// to `_4` (0x20 to 0x24).
const EVENTINF_HAGGLING_TOWNSFOLK_MASK: u16 = 0x1F;
/// `ATTENTION_RANGE_6`.
const ATTENTION_RANGE_6: u8 = 6;
/// `NpcTalkState` (`actor.h`).
const NPC_TALK_STATE_IDLE: i16 = 0;
const NPC_TALK_STATE_TALKING: i16 = 1;
/// `FIDGET_AMPLITUDE` (`actor.h`).
const FIDGET_AMPLITUDE: f32 = 200.0;
/// The limbs `EnMu_OverrideLimbDraw` sways (1-based).
const FIDGET_LIMBS: [usize; 7] = [5, 6, 7, 11, 12, 13, 14];

/// `D_80AB0BD0`.
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
    dim: Cylinder16 { radius: 100, height: 70, y_shift: 0, pos: [0; 3] },
};

/// `D_80AB0BFC`: immovable.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit2 = CollisionCheckInfoInit2 { health: 0, cyl_radius: 0, cyl_height: 0, cyl_y_shift: 0, mass: MASS_IMMOVABLE };

/// `EnMu_Interact`'s `textIdOffset`: the five texts' low bytes (0x70xx).
const TEXT_ID_OFFSET: [u8; 5] = [0x42, 0x43, 0x3F, 0x41, 0x3E];
/// `EnMu_Interact`'s `bitmask`: `EVENTINF_MASK(EVENTINF_HAGGLING_TOWNSFOLK_MESG_0..4)`.
const BITMASK: [u8; 5] = [1 << 0, 1 << 1, 1 << 2, 1 << 3, 1 << 4];

/// `EnMu_Draw`'s `colors[params]`, on `segmentId` 8 to 12.
const COLORS: [[[u8; 4]; 5]; 2] =
    [[[100, 130, 235, 0], [160, 250, 60, 0], [90, 60, 20, 0], [30, 240, 200, 0], [140, 70, 20, 0]], [[140, 70, 20, 0], [30, 240, 200, 0], [90, 60, 20, 0], [160, 250, 60, 0], [100, 130, 235, 0]]];
const SEGMENT_ID: [u8; 5] = [0x08, 0x09, 0x0A, 0x0B, 0x0C];
/// `EnMu_Draw`'s `Matrix_Translate(-1200, 0, -1400)`.
const MODEL_OFFSET: Vec3 = Vec3::new(-1200.0, 0.0, -1400.0);

/// `gDPSetEnvColor(r, g, b, a)` (G_SETENVCOLOR 0xFB).
fn set_env_color(c: [u8; 4]) -> (u32, u32) {
    (0xFB00_0000, u32::from_be_bytes(c))
}

/// The bake drawn for `params`.
pub fn bake_name(params: usize) -> String {
    format!("En_Mu/colors{params}")
}

/// The meshes `EnMu_Draw` draws: the skeleton in each colour set.
pub fn bakes() -> Vec<MeshBake> {
    (0..COLORS.len())
        .map(|p| MeshBake {
            name: bake_name(p),
            object: OBJECT.into(),
            segments: (0..5).map(|i| (SEGMENT_ID[i], BakeSegment::Commands(vec![set_env_color(COLORS[p][i])]))).collect(),
            prelude: Vec::new(),
            body: BakeBody::Skeleton { file: OBJECT.into(), symbol: "object_mu_Skel_004F70".into(), limbs: Vec::new() },
        })
        .collect()
}

pub struct EnMu {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub collider: ColliderCylinder,
    /// `npcInfo.talkState`.
    pub talk_state: i16,
    pub default_text_id: u16,
    pub fidget_table_y: [i16; 16],
    pub fidget_table_z: [i16; 16],
}

impl EnMu {
    /// `EnMu_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = (|| -> anyhow::Result<_> { Ok((assets.skeleton(OBJECT, "object_mu_Skel_004F70")?, assets.animation(OBJECT, "object_mu_Anim_0003F4")?)) })();
        let (skeleton, anim) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Mu: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let mut this = EnMu {
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(anim)),
            skeleton,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            talk_state: 0,
            default_text_id: 0,
            fidget_table_y: [0; 16],
            fidget_table_z: [0; 16],
            actor,
        };
        this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 160.0);
        this.actor.col_chk_info.set_info2(None, &COL_CHK_INFO_INIT);
        this.actor.target_mode = ATTENTION_RANGE_6;
        this.actor.scale = Vec3::splat(0.01);
        this.interact(play);
        Box::new(this)
    }

    /// `EnMu_Interact`: the next text, the first not yet heard from a random start; when all five
    /// have been, they start again, though never with the one just heard.
    pub fn interact(&mut self, play: &mut PlayState) {
        // schema: GET_EVENTINF_ENMU_TALK_FLAGS, RESET_EVENTINF_ENMU_TALK_FLAGS
        let inf = &mut play.save.event_inf[EVENTINF_INDEX_HAGGLING_TOWNSFOLK];
        let mut talk_flags = (*inf & EVENTINF_HAGGLING_TOWNSFOLK_MASK) as u8;
        *inf &= !EVENTINF_HAGGLING_TOWNSFOLK_MASK;
        // (play->state.frames + (s32)(Rand_ZeroOne() * 5)) % 5, in u32.
        let mut random_index = ((play.state_frames as u32).wrapping_add((play.rand.zero_one() * 5.0) as i32 as u32) % 5) as i32;

        let mut i = 0;
        while i < 5 {
            if talk_flags & BITMASK[random_index as usize] == 0 {
                break;
            }
            random_index += 1;
            if random_index >= 5 {
                random_index = 0;
            }
            i += 1;
        }

        if i == 5 {
            if self.default_text_id == (TEXT_ID_OFFSET[random_index as usize] as u16 | 0x7000) {
                random_index += 1;
                if random_index >= 5 {
                    random_index = 0;
                }
            }
            talk_flags = 0;
        }

        talk_flags |= BITMASK[random_index as usize];
        self.default_text_id = TEXT_ID_OFFSET[random_index as usize] as u16 | 0x7000;
        play.save.event_inf[EVENTINF_INDEX_HAGGLING_TOWNSFOLK] |= talk_flags as u16; // schema: SET_EVENTINF_ENMU_TALK_FLAGS
    }

    /// The limb matrices `EnMu_Draw` draws with: `EnMu_OverrideLimbDraw`'s sway.
    fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], table_y: &[i16; 16], table_z: &[i16; 16]) -> Vec<Mat4> {
        skeleton.pose_override(joints, |limb, _pos, rot| {
            if FIDGET_LIMBS.contains(&limb) {
                rot[1] = (rot[1] as f32 + sin_s(table_y[limb]) * FIDGET_AMPLITUDE) as i32 as i16;
                rot[2] = (rot[2] as f32 + cos_s(table_z[limb]) * FIDGET_AMPLITUDE) as i32 as i16;
            }
            Mat4::IDENTITY
        })
    }
}

impl ActorImpl for EnMu {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnMu_Update`.
    fn update(&mut self, play: &mut PlayState) {
        let p = self.actor.world_pos;
        self.collider.set_position([p.x as i16, p.y as i16, p.z as i16]);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        self.skel.update();
        self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
        // EnMu_Pose.
        actor_update_fidget_tables(play, &mut self.fidget_table_y, &mut self.fidget_table_z, 16);
        // Npc_UpdateTalking(.., collider.dim.radius + 30, EnMu_GetTextId, EnMu_UpdateTalkState).
        let talk_dist = self.collider.dim.radius as f32 + 30.0;
        let mut talk_state = self.talk_state;
        // MaskReaction_GetTextId(play, MASK_REACTION_SET_HAGGLING_TOWNSPEOPLE_1 + params): no
        // masks, so the default text.
        let text = self.default_text_id;
        let mut closing = false;
        talk_update(
            play,
            &mut self.actor,
            &mut talk_state,
            talk_dist,
            |_, _| text,
            |play, _| {
                // EnMu_UpdateTalkState.
                if play.message_state() == TEXT_STATE_CLOSING {
                    closing = true;
                    NPC_TALK_STATE_IDLE
                } else {
                    NPC_TALK_STATE_TALKING
                }
            },
        );
        self.talk_state = talk_state;
        if closing {
            self.interact(play);
        }
        self.actor.focus_pos = self.actor.world_pos + Vec3::Y * 60.0;
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.angles = FIDGET_LIMBS.iter().flat_map(|&l| [self.fidget_table_y[l], self.fidget_table_z[l]]).collect();
        rs
    }
    /// `EnMu_Draw`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        if rs.angles.len() < FIDGET_LIMBS.len() * 2 {
            return;
        }
        let (mut ty, mut tz) = ([0i16; 16], [0i16; 16]);
        for (i, &l) in FIDGET_LIMBS.iter().enumerate() {
            ty[l] = rs.angles[i * 2];
            tz[l] = rs.angles[i * 2 + 1];
        }
        let bones = Self::pose(&self.skeleton, &joints.rot, &ty, &tz);
        let m = oot_game::play::actor_draw_matrix(rs) * Mat4::from_translation(MODEL_OFFSET);
        let colors = (COLORS_INDEX.get(self.actor.params) as usize).min(COLORS.len() - 1);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(colors))), transform: m, bones, params: Default::default() });
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
