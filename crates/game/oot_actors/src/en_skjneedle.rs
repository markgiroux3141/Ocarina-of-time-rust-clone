//! `En_Skjneedle` (`ovl_En_Skjneedle/z_en_skjneedle.c`): the Skull Kid's needle (`En_Skj`'s
//! `EnSkj_ShootNeedle`, three a volley). It flies straight on at the speed its spawner gives it
//! (24, along its yaw: `Actor_MoveXZGravity`, no gravity), hurting Link on touch (8 damage), and
//! goes when it hits (`EnSkjNeedle_CollisionCheck`) or when `killTimer` runs out (the spawner sets
//! it to 100; 0 at init, so a needle nothing set goes at its first update).
//!
//! The whole overlay is ported. Drawn as `EnSkjneedle_Draw` does: `gSkullKidNeedleDL` after
//! `Gfx_SetupDL_25Opa` (the pack's mesh of it); the circle shadow (`ActorShadow_DrawCircle`, 20)
//! is the usual stand-in.
//!
//! Params: none (spawned with 0).
//! Flags: none.

use eng_gfx::{DrawCmd, MeshKey};
use eng_collision::math3d::Cylinder16;
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_HOSTILE, Actor, UPDBGCHECKINFO_FLAG_0, UPDBGCHECKINFO_FLAG_1, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ActorImpl, ActorProfile};
use oot_game::actor_schema::ActorSchema;
use oot_game::collision_check::*;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

/// `ACTOR_EN_SKJNEEDLE` (`actor_table.h`: 0x0116).
pub const ACTOR_EN_SKJNEEDLE: i16 = 0x0116;
/// `OBJECT_SKJ` (`object_table.h`: 0x010A).
pub const OBJECT: &str = "object_skj";

/// `ACTOR_FLAG_HOOKSHOT_PULLS_ACTOR` (`actor.h`: 1 << 9).
pub const ACTOR_FLAG_HOOKSHOT_PULLS_ACTOR: u32 = 1 << 9;
/// `ATTENTION_RANGE_2` (`actor.h`).
const ATTENTION_RANGE_2: u8 = 2;

/// `En_Skjneedle_Profile`: `ACTORCAT_ENEMY`, `FLAGS` (`ACTOR_FLAG_ATTENTION_ENABLED |
/// ACTOR_FLAG_HOSTILE | ACTOR_FLAG_HOOKSHOT_PULLS_ACTOR`), `OBJECT_SKJ`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_SKJNEEDLE,
    name: "En_Skjneedle",
    category: ACTORCAT_ENEMY,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_HOOKSHOT_PULLS_ACTOR,
    object: OBJECT,
    schema: &SCHEMA,
};

/// En_Skjneedle's configuration (docs/adr/0066-the-actor-schema.md): none.
pub const SCHEMA: ActorSchema = ActorSchema::NONE;

/// `sCylinderInit` (`ColliderCylinderInitType1`: `Collider_SetCylinderType1` makes `ocFlags2`
/// `OC2_TYPE_1`): an enemy's attack on Link (0xFFCFFFFF, 8 damage), hit by Player, 10 by 4, 2
/// down.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HIT1, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x08 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 10, height: 4, y_shift: -2, pos: [0, 0, 0] },
};

pub struct EnSkjneedle {
    pub actor: Actor,
    pub collider: ColliderCylinder,
    /// `unusedTimer1`: counted up each update, read by nothing.
    pub unused_timer1: i16,
    /// `killTimer`: counted down; at 0 the needle goes.
    pub kill_timer: i16,
}

impl EnSkjneedle {
    /// `EnSkjneedle_Init`.
    pub fn init(mut actor: Actor, _play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: attentionRangeType ATTENTION_RANGE_2, lockOnArrowOffset 30.
        actor.target_mode = ATTENTION_RANGE_2;
        actor.target_arrow_offset = 30.0;
        // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 20).
        actor.shape_y_offset = 0.0;
        actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        actor.scale = Vec3::splat(0.01);
        Box::new(EnSkjneedle { actor, collider: ColliderCylinder::new(&CYLINDER_INIT), unused_timer1: 0, kill_timer: 0 })
    }

    /// `EnSkjNeedle_CollisionCheck`: it touched something. @bug (game): it clears `AC_HIT`
    /// rather than the `AT_HIT` it read (harmless: the needle's killed then).
    fn collision_check(&mut self) -> bool {
        if self.collider.base.at_flags & AT_HIT != 0 {
            self.collider.base.ac_flags &= !AC_HIT;
            return true;
        }
        false
    }
}

impl ActorImpl for EnSkjneedle {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnSkjneedle_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.unused_timer1 = self.unused_timer1.wrapping_add(1);
        if self.kill_timer != 0 {
            self.kill_timer -= 1;
        }
        if self.collision_check() || self.kill_timer == 0 {
            self.actor.kill();
        } else {
            self.actor.scale = Vec3::splat(0.01);
            self.collider.update(&self.actor);
            play.collision_check_set_at(&self.actor, 0, &mut self.collider);
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
            self.actor.move_forward();
            self.actor.update_bg_check_info(&play.col, 20.0, 20.0, 20.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_1 | UPDBGCHECKINFO_FLAG_2);
        }
    }

    /// `EnSkjneedle_Draw`: `gSkullKidNeedleDL`; the circle shadow's stand-in (20).
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        crate::gfx_draw_dlist_opa(out, OBJECT, "gSkullKidNeedleDL", rs);
        let (floor, _) = play.col.entity_raycast_down(rs.pos + Vec3::Y * 20.0);
        let m = Mat4::from_translation(Vec3::new(rs.pos.x, floor + 0.3, rs.pos.z)) * Mat4::from_scale(Vec3::new(20.0, 1.0, 20.0));
        out.xlu.push(DrawCmd::new(MeshKey::named(oot_game::play::builtin::SHADOW), m));
    }

    /// `EnSkjneedle_Destroy`: `Collider_DestroyCylinder` (nothing to free here).
    fn destroy(&mut self, _play: &mut PlayState) {}

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
