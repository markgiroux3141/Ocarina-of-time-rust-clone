//! `En_A_Obj` (`code/z_en_a_keep.c`): the signposts, and a handful of plain props and blocks
//! from `gameplay_keep` (GAME-06 milestone 4).
//!
//! Params: the low byte is the type (`AObjType`), the high byte a signpost's text (`0x300 |
//! it`). Only the signposts (`A_OBJ_SIGNPOST_OBLONG`, `_ARROW`) are placed in rooms.
//!
//! - **Signposts:** read from the front (within 0x2800 of their facing; an arrow from behind
//!   too), within 50 of their cylinder (`Actor_OfferTalkNearColChkInfoCylinder`); solid to bump
//!   into (their OC cylinder, 25 by 60).
//! - **Blocks** (`DynaPoly` bg actors): the large ones slide when pushed (`EnAObj_Block`), the
//!   rotating ones tip over a frame's push (`EnAObj_BlockRot`).
//! - **A boulder's fragment:** bounces off the floor until slow, then goes.
//! - The rest stand (gravity -2) and can be read if they have a text.
//!
//! The whole file is ported. `A_OBJ_UNKNOWN_6` reads `object_d_hsblock`'s hookshot post, which
//! the actor doesn't load (@bug (game), never placed): logged and dropped. The circle shadow
//! (`ActorShadow_DrawCircle`) isn't drawn, as for the other ported actors.

use std::sync::Arc;

use eng_collision::collision::CollisionHeader;
use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource, DYNA_TRANSFORM_POS};
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::smooth_step_to_f;
use glam::Vec3;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_BG, ACTORCAT_PROP, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::audio::sfx::{NA_SE_EV_ROCK_SLIDE, SFX_FLAG};
use oot_game::collision_check::*;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};

/// `ACTOR_EN_A_OBJ` (`actor_table.h`: 0x0039).
pub const ACTOR_EN_A_OBJ: i16 = 0x0039;
const KEEP: &str = "gameplay_keep";

/// `En_A_Obj_Profile`: `ACTORCAT_PROP`, `ACTOR_FLAG_UPDATE_CULLING_DISABLED`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_A_OBJ, name: "En_A_Obj", category: ACTORCAT_PROP, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: KEEP };

/// `AObjType`.
pub const A_OBJ_BLOCK_SMALL: i16 = 0x00;
pub const A_OBJ_BLOCK_LARGE: i16 = 0x01;
pub const A_OBJ_BLOCK_HUGE: i16 = 0x02;
pub const A_OBJ_BLOCK_SMALL_ROT: i16 = 0x03;
pub const A_OBJ_BLOCK_LARGE_ROT: i16 = 0x04;
pub const A_OBJ_CUBE_SMALL: i16 = 0x05;
pub const A_OBJ_UNKNOWN_6: i16 = 0x06;
pub const A_OBJ_GRASS_CLUMP: i16 = 0x07;
pub const A_OBJ_TREE_STUMP: i16 = 0x08;
pub const A_OBJ_SIGNPOST_OBLONG: i16 = 0x09;
pub const A_OBJ_SIGNPOST_ARROW: i16 = 0x0A;
pub const A_OBJ_BOULDER_FRAGMENT: i16 = 0x0B;
pub const A_OBJ_MAX: i16 = 0x0C;

/// `sCylinderInit`: the signposts' OC cylinder, 25 by 60.
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_ALL, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK2,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 25, height: 60, y_shift: 0, pos: [0, 0, 0] },
};

/// `sColHeaders`: by `dyna.bgId` as the init sets it (5 is `object_d_hsblock`'s, not loaded).
const COL_HEADERS: [&str; 5] = ["gLargerCubeCol", "gLargerCubeCol", "gSmallerFlatBlockCol", "gLargerFlatBlockCol", "gSmallerCubeCol"];

/// `sDLists`, by type (6 is `object_d_hsblock`'s `gHookshotPostDL`).
const DLISTS: [&str; 12] = [
    "gFlatBlockDL",
    "gFlatBlockDL",
    "gFlatBlockDL",
    "gFlatRotBlockDL",
    "gFlatRotBlockDL",
    "gSmallCubeDL",
    "",
    "gGrassBladesDL",
    "gTreeStumpDL",
    "gSignRectangularDL",
    "gSignDirectionalDL",
    "gBoulderFragmentsDL",
];

/// The boulder fragment after `gDPSetPrimColor(0, 1, 60, 60, 60, 50)`.
const FRAGMENT_BAKE: &str = "En_A_Obj/boulder_fragment";
const SEG_COLOR: u8 = 0x0B;

pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: FRAGMENT_BAKE.into(),
        object: KEEP.into(),
        segments: vec![(SEG_COLOR, BakeSegment::Commands(vec![(0xFA00_0001, 0x3C3C_3C32), (0xDF00_0000, 0)]))],
        prelude: vec![SEG_COLOR],
        body: BakeBody::DLists(vec![(KEEP.into(), "gBoulderFragmentsDL".into())]),
    }]
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `EnAObj_WaitTalk`.
    WaitTalk,
    /// `EnAObj_WaitFinishedTalking`.
    WaitFinishedTalking,
    /// `EnAObj_BlockRot`.
    BlockRot,
    /// `EnAObj_BoulderFragment`.
    BoulderFragment,
    /// `EnAObj_Block`.
    Block,
}

pub struct EnAObj {
    /// `dyna.actor`.
    pub actor: Actor,
    /// `dyna.bgId`.
    pub bg: u16,
    pub action: Action,
    pub rotate_wait_timer: i32,
    pub text_id: i16,
    pub rotate_state: i16,
    pub rotate_for_timer: i16,
    pub rot_speed_y: i16,
    pub rot_speed_x: i16,
    pub focus_y_offset: f32,
    pub collider: ColliderCylinder,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: 0.0 }
}

fn load_collision(play: &PlayState, symbol: &str) -> anyhow::Result<Arc<CollisionHeader>> {
    let assets = play.assets.as_ref().ok_or_else(|| anyhow::anyhow!("no asset pack"))?;
    Ok(Arc::new(assets.pack.collision(&keys::collision(KEEP, symbol))?))
}

impl EnAObj {
    fn is_signpost(&self) -> bool {
        matches!(self.actor.params, A_OBJ_SIGNPOST_OBLONG | A_OBJ_SIGNPOST_ARROW)
    }

    /// `EnAObj_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let text_id = (actor.params >> 8) & 0xFF;
        actor.params &= 0xFF;
        let scale = match actor.params {
            A_OBJ_BLOCK_SMALL => 0.025,
            A_OBJ_BLOCK_LARGE => 0.05,
            A_OBJ_BLOCK_HUGE | A_OBJ_CUBE_SMALL | A_OBJ_UNKNOWN_6 => 0.1,
            A_OBJ_BLOCK_SMALL_ROT => 0.005,
            _ => 0.01,
        };
        actor.scale = Vec3::splat(scale);
        // ActorShape_Init(0, ActorShadow_DrawCircle, 6, or 12 for the signposts and up): the circle
        // shadow isn't drawn.
        actor.shape_y_offset = 0.0;
        actor.focus_pos = actor.world_pos;
        actor.culling_volume_downward = 1200.0;
        actor.culling_volume_scale = 200.0;
        let mut this = EnAObj {
            actor,
            bg: BG_ACTOR_MAX,
            action: Action::WaitTalk,
            rotate_wait_timer: 0,
            text_id,
            rotate_state: 0,
            rotate_for_timer: 0,
            rot_speed_y: 0,
            rot_speed_x: 0,
            focus_y_offset: 0.0,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
        };
        // BGACTOR_NEG_ONE: none.
        let mut bg_index: Option<usize> = None;
        match this.actor.params {
            A_OBJ_BLOCK_LARGE | A_OBJ_BLOCK_HUGE => {
                bg_index = Some(1);
                this.change_category(play, ACTORCAT_BG);
                this.setup_block();
            }
            A_OBJ_BLOCK_SMALL_ROT | A_OBJ_BLOCK_LARGE_ROT => {
                bg_index = Some(3);
                this.change_category(play, ACTORCAT_BG);
                this.setup_block_rot();
            }
            A_OBJ_UNKNOWN_6 => {
                this.focus_y_offset = 10.0;
                this.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
                // bgId 5: object_d_hsblock's gHookshotPostCol, not this actor's object.
                log::error!("En_A_Obj: A_OBJ_UNKNOWN_6 reads object_d_hsblock's hookshot post, which it doesn't load (never placed)");
                this.actor.gravity = -2.0;
                this.action = Action::WaitTalk;
            }
            A_OBJ_GRASS_CLUMP | A_OBJ_TREE_STUMP => {
                bg_index = Some(0);
                this.action = Action::WaitTalk;
            }
            A_OBJ_SIGNPOST_OBLONG | A_OBJ_SIGNPOST_ARROW => {
                this.actor.text_id = ((this.text_id & 0xFF) | 0x300) as u16;
                this.actor.target_arrow_offset = 500.0;
                this.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY;
                this.focus_y_offset = 45.0;
                this.action = Action::WaitTalk;
                this.collider.update(&this.actor);
                this.actor.col_chk_info.mass = MASS_IMMOVABLE;
                // ATTENTION_RANGE_0.
                this.actor.target_mode = 0;
            }
            A_OBJ_BOULDER_FRAGMENT => {
                this.actor.gravity = -1.5;
                this.action = Action::BoulderFragment;
            }
            _ => {
                this.actor.gravity = -2.0;
                this.action = Action::WaitTalk;
            }
        }
        if this.actor.params <= A_OBJ_BLOCK_LARGE_ROT {
            this.actor.col_chk_info.mass = MASS_IMMOVABLE;
        }
        if let Some(i) = bg_index {
            match load_collision(play, COL_HEADERS[i]) {
                Ok(h) => this.bg = play.col.dyna.set_bg_actor(h, source(&this.actor), DYNA_TRANSFORM_POS),
                Err(e) => log::error!("En_A_Obj: {e:#}"),
            }
        }
        Box::new(this)
    }

    /// `Actor_ChangeCategory`: the actor's own (its init: `spawn` files it under the new one).
    fn change_category(&mut self, _play: &mut PlayState, category: usize) {
        self.actor.category = category;
    }

    /// `EnAObj_WaitFinishedTalking`.
    fn wait_finished_talking(&mut self, play: &PlayState) {
        if oot_game::npc::textbox_is_closing(play) {
            self.action = Action::WaitTalk;
        }
    }

    /// `EnAObj_WaitTalk`: with a text, read from within 0x2800 of its facing (an arrow sign from
    /// behind too, past 0x5800).
    fn wait_talk(&mut self, play: &mut PlayState) {
        if self.actor.text_id != 0 {
            let rel = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
            if (rel as i32).abs() < 0x2800 || (self.actor.params == A_OBJ_SIGNPOST_ARROW && (rel as i32).abs() > 0x5800) {
                if oot_game::npc::process_talk_request(&mut self.actor) {
                    self.action = Action::WaitFinishedTalking;
                } else {
                    let a = self.actor.clone();
                    oot_game::npc::offer_talk_default(play, &a);
                }
            }
        }
    }

    /// `EnAObj_SetupBlockRot`.
    fn setup_block_rot(&mut self) {
        self.rotate_state = 0;
        self.rotate_wait_timer = 10;
        self.actor.world_rot.y = 0;
        self.actor.shape_rot = self.actor.world_rot;
        self.action = Action::BlockRot;
    }

    /// `EnAObj_BlockRot`: something on it (`interactFlags`) starts a tip: 10 frames' wait, then
    /// 20 of turning 0x3E8 a frame on x and y (signs by Link's side), falling; then back home.
    fn block_rot(&mut self, play: &PlayState) {
        if self.rotate_state == 0 {
            if play.col.dyna.interact_flag(self.bg, 0xFF) {
                self.rotate_state += 1;
                self.rotate_for_timer = 20;
                self.rot_speed_x = if self.actor.yaw_towards_player.wrapping_add(0x4000) < 0 { -0x3E8 } else { 0x3E8 };
                self.rot_speed_y = if self.actor.yaw_towards_player < 0 { -self.rot_speed_x } else { self.rot_speed_x };
            }
        } else if self.rotate_wait_timer != 0 {
            self.rotate_wait_timer -= 1;
        } else {
            self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(self.rot_speed_y);
            self.actor.shape_rot.x = self.actor.shape_rot.x.wrapping_add(self.rot_speed_x);
            self.rotate_for_timer -= 1;
            self.actor.gravity = -1.0;
            if self.rotate_for_timer == 0 {
                self.actor.world_pos = self.actor.home_pos;
                self.rotate_state = 0;
                self.rotate_wait_timer = 10;
                self.actor.velocity.y = 0.0;
                self.actor.gravity = 0.0;
                self.actor.shape_rot = self.actor.world_rot;
            }
        }
    }

    /// `EnAObj_BoulderFragment`: speeding up to 1, tumbling by half its x and z turn; off a wall
    /// it reflects; on the ground it bounces back (×0.6) while falling faster than 8, else goes.
    fn boulder_fragment(&mut self) {
        let a = &mut self.actor;
        smooth_step_to_f(&mut a.speed_xz, 1.0, 1.0, 0.5, 0.0);
        a.shape_rot.x = a.shape_rot.x.wrapping_add(a.world_rot.x >> 1);
        a.shape_rot.z = a.shape_rot.z.wrapping_add(a.world_rot.z >> 1);
        if a.speed_xz != 0.0 && a.bg_check_flags & BGCHECKFLAG_WALL != 0 {
            a.world_rot.y = a.wall_yaw.wrapping_sub(a.world_rot.y).wrapping_add(a.wall_yaw).wrapping_sub(i16::MIN);
            a.bg_check_flags &= !BGCHECKFLAG_WALL;
        }
        if a.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 {
            if a.velocity.y < -8.0 {
                a.velocity.y *= -0.6;
                a.speed_xz *= 0.6;
                a.bg_check_flags &= !(BGCHECKFLAG_GROUND | BGCHECKFLAG_GROUND_TOUCH);
            } else {
                a.kill();
            }
        }
    }

    /// `EnAObj_SetupBlock`.
    fn setup_block(&mut self) {
        self.actor.culling_volume_downward = 1200.0;
        self.actor.culling_volume_scale = 720.0;
        self.action = Action::Block;
    }

    /// `EnAObj_Block`: pushed (`dyna.unk_150` along `unk_158`), up to 2.5, easing to a stop, with
    /// its sliding sound while it moves.
    fn block(&mut self, play: &mut PlayState) {
        let unk_150 = play.col.dyna.unk_150(self.bg);
        self.actor.speed_xz += unk_150;
        self.actor.world_rot.y = play.col.dyna.unk_158(self.bg);
        self.actor.speed_xz = self.actor.speed_xz.clamp(-2.5, 2.5);
        smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 1.0, 1.0, 0.0);
        if self.actor.speed_xz != 0.0 {
            audio_play_actor_sfx2(play, NA_SE_EV_ROCK_SLIDE - SFX_FLAG);
        }
        play.col.dyna.set_unk_154(self.bg, 0.0);
        play.col.dyna.set_unk_150(self.bg, 0.0);
    }
}

impl ActorImpl for EnAObj {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnAObj_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::WaitTalk => self.wait_talk(play),
            Action::WaitFinishedTalking => self.wait_finished_talking(play),
            Action::BlockRot => self.block_rot(play),
            Action::BoulderFragment => self.boulder_fragment(),
            Action::Block => self.block(play),
        }
        self.actor.move_forward();
        if self.actor.gravity != 0.0 {
            let flags = UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4;
            let radius = if self.actor.params != A_OBJ_BOULDER_FRAGMENT { 40.0 } else { 20.0 };
            self.actor.update_bg_check_info_of(&play.col, 5.0, radius, 0.0, flags, (self.bg != BG_ACTOR_MAX).then_some(self.bg));
        }
        self.actor.focus_pos = self.actor.world_pos;
        self.actor.focus_pos.y += self.focus_y_offset;
        if self.is_signpost() {
            self.collider.update(&self.actor);
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        }
        if self.bg != BG_ACTOR_MAX {
            play.col.dyna.set_source(self.bg, source(&self.actor));
        }
    }

    /// `EnAObj_Draw`: its type's list after `Gfx_SetupDL_25Opa` (a boulder fragment's prim
    /// colour first).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let mut ty = self.actor.params;
        if ty >= A_OBJ_MAX {
            ty = A_OBJ_BOULDER_FRAGMENT;
        }
        let m = actor_draw_matrix(rs);
        if self.actor.params == A_OBJ_BOULDER_FRAGMENT {
            out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(FRAGMENT_BAKE)), m));
            return;
        }
        let dl = DLISTS[ty as usize];
        if !dl.is_empty() {
            out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh(KEEP, dl)), m));
        }
    }

    /// `EnAObj_Destroy`: its bg actor out.
    fn destroy(&mut self, play: &mut PlayState) {
        if self.bg != BG_ACTOR_MAX {
            play.col.dyna.delete_bg_actor(self.bg);
        }
    }

    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }
    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0 && self.is_signpost()).then_some(ColliderMut::Cylinder(&mut self.collider))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
