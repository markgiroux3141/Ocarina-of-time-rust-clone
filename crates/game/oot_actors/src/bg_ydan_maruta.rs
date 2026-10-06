//! `Bg_Ydan_Maruta` (`ovl_Bg_Ydan_Maruta/z_bg_ydan_maruta.c`): the Deku Tree's rolling spiked
//! log and falling ladder, of `object_ydan_objects`.
//!
//! Params: bits 0..7 the switch flag (`switchFlag`), bits 8..15 the kind, which the init leaves
//! in `params`.
//!
//! - **0, the spiked log** (Master Quest's room 5, `0x00FF`): spins in place about its length
//!   (`shape.rot.x` up 0x360 a frame) with `NA_SE_EV_TOGE_STICK_ROLLING`, and its two triangles
//!   (an upright 440 by 20 across its middle) hit what they touch (`AT_TYPE_ENEMY`, damage 4).
//!   When they've hit, Link is knocked down along the log's facing
//!   (`Actor_SetPlayerKnockbackLargeNoDamage(7, shape.rot.y, 6)`; `func_808BEFF4`). It has no
//!   bg collision.
//! - **1, the ladder** (room 2, `0x0121`): a DynaPoly (`gDTFallingLadderCol`, no carrying) whose
//!   home is 280 under where it's placed; already down if its switch flag is set. Its two
//!   triangles (32 by 135 up from where it's placed) take only a seed (`DMG_SLINGSHOT`): hit, it
//!   sets the flag with the chime and one-point cutscene 3010 for 50 frames (`func_808BF078`),
//!   shakes for 20 frames (`func_808BF108`, `NA_SE_EV_TRAP_OBJ_SLIDE`), then falls with its
//!   speed growing by 1 a frame to its home (`func_808BF1EC`, `NA_SE_EV_LADDER_DOUND`). Its
//!   triangles stay where it was placed.
//!
//! The whole overlay is ported. Link has no slingshot yet (GAME-05 milestone 5): the ladder's
//! seed is injected by the tests.

use std::sync::Arc;

use eng_collision::collision::CollisionHeader;
use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{cos_s, sin_s, step_to_f};
use glam::Vec3;
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile, actor_set_player_knockback_large_no_damage, audio_play_actor_sfx2};
use oot_game::audio::sfx::{NA_SE_SY_CORRECT_CHIME, SFX_FLAG};
use oot_game::camera::CAM_ID_MAIN;
use oot_game::collision_check::*;
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};

/// `ACTOR_BG_YDAN_MARUTA` (`actor_table.h`: 0x0051).
pub const ACTOR_BG_YDAN_MARUTA: i16 = 0x0051;
pub const OBJECT: &str = "object_ydan_objects";
const LADDER_COLLISION: &str = "gDTFallingLadderCol";
/// `BgYdanMaruta_Draw`'s lists.
pub const LOG_DISPLAY_LIST: &str = "gDTRollingSpikeTrapDL";
pub const LADDER_DISPLAY_LIST: &str = "gDTFallingLadderDL";

/// `Bg_Ydan_Maruta_Profile`: `FLAGS` 0.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_YDAN_MARUTA, name: "Bg_Ydan_Maruta", category: ACTORCAT_PROP, flags: 0, object: OBJECT };

/// The kinds (the params' high byte).
pub const MARUTA_LOG: i16 = 0;
pub const MARUTA_LADDER: i16 = 1;

/// `NA_SE_EV_TRAP_OBJ_SLIDE`, `NA_SE_EV_LADDER_DOUND`, `NA_SE_EV_TOGE_STICK_ROLLING`
/// (`environmentbank_table.h`: 0x2858, 0x2860, 0x28EC).
pub const NA_SE_EV_TRAP_OBJ_SLIDE: u16 = 0x2858;
pub const NA_SE_EV_LADDER_DOUND: u16 = 0x2860;
pub const NA_SE_EV_TOGE_STICK_ROLLING: u16 = 0x28EC;

/// The log's spin a frame (`shape.rot.x += 0x360`).
pub const SPIN_STEP: i16 = 0x360;
/// The ladder's drop: `home.pos.y += -280.0f`.
pub const LADDER_DROP: f32 = 280.0;
/// The shake's frames (`unk_16A = 20`).
pub const SHAKE_FRAMES: i16 = 20;

/// `sTrisElementsInit`'s element: hits with damage 4 (`0x20000000`), taken only from a seed
/// (`DMG_SLINGSHOT`, 4).
const TRIS_ELEMENT_INFO: ColliderElementInit = ColliderElementInit {
    elem_material: ELEM_MATERIAL_UNK0,
    at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x2000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x04 },
    ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0004, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
    at_elem_flags: ATELEM_ON | ATELEM_SFX_WOOD,
    ac_elem_flags: ACELEM_ON,
    oc_elem_flags: OCELEM_NONE,
};

/// `sTrisElementsInit`: the log's triangle (440 long, 20 high) and the ladder's (32 wide, 135
/// high). The init places one of them, by kind, as both of the collider's triangles.
pub fn tris_elements_init() -> [ColliderTrisElementInit; 2] {
    [
        ColliderTrisElementInit { info: TRIS_ELEMENT_INFO, vtx: [Vec3::new(220.0, -10.0, 0.0), Vec3::new(220.0, 10.0, 0.0), Vec3::new(-220.0, 10.0, 0.0)] },
        ColliderTrisElementInit { info: TRIS_ELEMENT_INFO, vtx: [Vec3::new(16.0, 0.0, 0.0), Vec3::new(16.0, 135.0, 0.0), Vec3::new(-16.0, 135.0, 0.0)] },
    ]
}

/// `sTrisInit`.
pub const TRIS_INIT: ColliderInit =
    ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_TRIS };

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_808BEFF4`: the log spinning.
    Spin,
    /// `func_808BF078`: the ladder up, waiting for a seed.
    WaitForHit,
    /// `func_808BF108`: shaking.
    Shake,
    /// `func_808BF1EC`: falling.
    Fall,
    DoNothing,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::Spin => "func_808BEFF4",
            Action::WaitForHit => "func_808BF078",
            Action::Shake => "func_808BF108",
            Action::Fall => "func_808BF1EC",
            Action::DoNothing => "BgYdanMaruta_DoNothing",
        }
    }
}

pub struct BgYdanMaruta {
    /// `dyna.actor`, `dyna.bgId` (`BG_ACTOR_MAX` for the log, which sets none).
    pub actor: Actor,
    pub bg: u16,
    pub action: Action,
    pub switch_flag: u8,
    /// `unk_16A`: the shake's countdown.
    pub unk_16a: i16,
    pub collider: ColliderTris,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

impl BgYdanMaruta {
    /// `BgYdanMaruta_Init` from `Actor_Spawn`: the ladder's `gDTFallingLadderCol` from the pack
    /// (`CollisionHeader_GetVirtual`), then [`BgYdanMaruta::init_with`].
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let header = if (actor.params as u16 >> 8) as i16 == MARUTA_LOG {
            None
        } else {
            play.assets.as_ref().and_then(|a| a.pack.collision(&keys::collision(OBJECT, LADDER_COLLISION)).map(Arc::new).map_err(|e| log::error!("Bg_Ydan_Maruta: {e:#}")).ok())
        };
        Box::new(Self::init_with(actor, play, header))
    }

    /// `BgYdanMaruta_Init`: scale 0.1 (`sInitChain`); the flag and the kind from the params. The
    /// log spins; the ladder sets its bg actor (`DynaPolyActor_Init(0)`), its home 280 down, and
    /// is down already if its flag is set. Then the collider's two triangles from the kind's
    /// element, turned by `shape.rot.y` about where it is: the element's own, and the second
    /// from its first vertex, its third, and a fourth corner (the third's x at the first's y),
    /// which makes the rectangle.
    pub fn init_with(mut actor: Actor, play: &mut PlayState, col_header: Option<Arc<CollisionHeader>>) -> BgYdanMaruta {
        // ICHAIN_VEC3F_DIV1000(scale, 100).
        actor.scale = Vec3::splat(0.1);
        let tri_inits = tris_elements_init();
        let mut collider = ColliderTris::new(&TRIS_INIT, &tri_inits);
        let params = actor.params as u16;
        // PARAMS_GET_U(params, 0, 8), PARAMS_GET_U(params, 8, 8).
        let switch_flag = (params & 0xFF) as u8;
        actor.params = ((params >> 8) & 0xFF) as i16;
        let mut bg = BG_ACTOR_MAX;
        let tri_init;
        let action;
        if actor.params == MARUTA_LOG {
            tri_init = &tri_inits[0];
            action = Action::Spin;
        } else {
            tri_init = &tri_inits[1];
            // DynaPolyActor_Init(&this->dyna, 0), DynaPoly_SetBgActor.
            if let Some(h) = col_header {
                bg = play.col.dyna.set_bg_actor(h, source(&actor), 0);
            }
            actor.home_pos.y += -LADDER_DROP;
            if play.flags.get_switch(switch_flag as i32) {
                actor.world_pos.y = actor.home_pos.y;
                action = Action::DoNothing;
            } else {
                action = Action::WaitForHit;
            }
        }
        let sin_rot_y = sin_s(actor.shape_rot.y);
        let cos_rot_y = cos_s(actor.shape_rot.y);
        let pos = actor.world_pos;
        let v = tri_init.vtx;
        let mut sp4c = [Vec3::ZERO; 3];
        for i in 0..3 {
            sp4c[i].x = (v[i].x * cos_rot_y) + pos.x;
            sp4c[i].y = v[i].y + pos.y;
            sp4c[i].z = pos.z - (v[i].x * sin_rot_y);
        }
        collider.set_vertices(0, sp4c[0], sp4c[1], sp4c[2]);
        sp4c[1].x = (v[2].x * cos_rot_y) + pos.x;
        sp4c[1].y = v[0].y + pos.y;
        sp4c[1].z = pos.z - (v[2].x * sin_rot_y);
        collider.set_vertices(1, sp4c[0], sp4c[2], sp4c[1]);
        BgYdanMaruta { actor, bg, action, switch_flag, unk_16a: 0, collider }
    }

    /// `func_808BEFF4`: last frame's hit knocks Link down along the log's facing; the spin, the
    /// AT, the rolling sound.
    fn spin(&mut self, play: &mut PlayState) {
        if self.collider.base.at_flags & AT_HIT != 0 {
            actor_set_player_knockback_large_no_damage(play, 7.0, self.actor.shape_rot.y, 6.0);
        }
        self.actor.shape_rot.x = self.actor.shape_rot.x.wrapping_add(SPIN_STEP);
        play.collision_check_set_at(&self.actor, 0, &mut self.collider);
        self.actor.play_sfx_flagged(NA_SE_EV_TOGE_STICK_ROLLING - SFX_FLAG);
    }

    /// `func_808BF078`: a seed's hit sets the flag, with the chime and one-point cutscene 3010
    /// for 50 frames, and starts the shake; else the AC again.
    fn wait_for_hit(&mut self, play: &mut PlayState) {
        if self.collider.base.ac_flags & AC_HIT != 0 {
            self.unk_16a = SHAKE_FRAMES;
            play.flags.set_switch(self.switch_flag as i32);
            play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
            self.action = Action::Shake;
            let me = play.cur_actor.map(|h| play.cam_actor_of(h, &self.actor));
            play.onepoint_cutscene_init(3010, 50, me, CAM_ID_MAIN);
        } else {
            play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
        }
    }

    /// `func_808BF108`: the countdown, then the fall; meanwhile across its face by
    /// [`shake_offset`] of the count, with the sliding sound.
    fn shake(&mut self) {
        if self.unk_16a != 0 {
            self.unk_16a -= 1;
        }
        if self.unk_16a == 0 {
            self.action = Action::Fall;
        }
        let temp = shake_offset(self.unk_16a) as f32;
        let yaw = self.actor.shape_rot.y;
        self.actor.world_pos.x = (cos_s(yaw) * temp) + self.actor.home_pos.x;
        self.actor.world_pos.z = (sin_s(yaw) * temp) + self.actor.home_pos.z;
        self.actor.play_sfx_flagged(NA_SE_EV_TRAP_OBJ_SLIDE - SFX_FLAG);
    }

    /// `func_808BF1EC`: 1 faster each frame, down to home, landing with `NA_SE_EV_LADDER_DOUND`.
    fn fall(&mut self, play: &mut PlayState) {
        self.actor.velocity.y += 1.0;
        let step = self.actor.velocity.y;
        if step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y, step) {
            audio_play_actor_sfx2(play, NA_SE_EV_LADDER_DOUND);
            self.action = Action::DoNothing;
        }
    }
}

/// `func_808BF108`'s offset for the count `unk_16A`: `(unk_16A % 4) - 2`, -2 made 0, the rest
/// doubled: 0, -2, 0, 2 for counts 0, 1, 2, 3 (mod 4).
pub fn shake_offset(unk_16a: i16) -> i16 {
    let temp = (unk_16a % 4) - 2;
    if temp == -2 { 0 } else { temp * 2 }
}

impl ActorImpl for BgYdanMaruta {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BgYdanMaruta_Update`, and the ladder's transform for `DynaPoly_UpdateContext`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Spin => self.spin(play),
            Action::WaitForHit => self.wait_for_hit(play),
            Action::Shake => self.shake(),
            Action::Fall => self.fall(play),
            Action::DoNothing => {}
        }
        if self.bg != BG_ACTOR_MAX {
            play.col.dyna.set_source(self.bg, source(&self.actor));
        }
    }

    /// `BgYdanMaruta_Destroy`: the ladder's bg actor (the collider is the actor's own).
    fn destroy(&mut self, play: &mut PlayState) {
        if self.actor.params == MARUTA_LADDER {
            play.col.dyna.delete_bg_actor(self.bg);
        }
    }

    /// The ladder's `dyna.bgId`: the interact flags `Actor_UpdateAll` clears after it.
    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }

    /// `BgYdanMaruta_Draw`: `Gfx_DrawDListOpa` of the log's list (kind 0) or the ladder's.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let dl = if self.actor.params == MARUTA_LOG { LOG_DISPLAY_LIST } else { LADDER_DISPLAY_LIST };
        out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh(OBJECT, dl)), actor_draw_matrix(rs)));
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0).then_some(ColliderMut::Tris(&mut self.collider))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
