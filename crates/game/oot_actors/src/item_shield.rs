//! `Item_Shield` (`ovl_Item_Shield/z_item_shield.c`): the Deku Shield as an actor (the child's
//! model, `gLinkChildDekuShieldDL`).
//!
//! - **params 0**, a shield lying about (`func_80B86BC8`): offered to Link (`GI_SHIELD_DEKU`,
//!   within 30 across and 50 up or down); struck (its cylinder's `AC_HIT`), it hops up (4) and
//!   falls (`func_80B86AC8`), still offered, blinking out its last 60 of 160 frames on the
//!   ground.
//! - **params 1**, the burnt shield (`func_8083819C`: Player's Deku Shield caught fire): on its
//!   first update it takes the shield's place (`func_80B86F68`: `shieldMf`'s position and facing,
//!   hidden until then), then hops up (4) and falls, burning (`func_80B86CA8`): eight flames
//!   (`EffectSsFireTail_SpawnFlame`) rising and fading through 16 frames each, at random places
//!   about it, until its timer runs low; on the ground it rocks flat (a spring on `shape.rot.x`)
//!   and shrinks away over frames 24 to 8 of its 70, then it's gone.
//!
//! The whole overlay is ported.

use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{atan2_s, sin_s};
use glam::Vec3;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorImpl, ActorProfile};
use oot_game::collision_check::*;
use oot_game::get_item::{actor_has_parent, offer_get_item_range};
use oot_game::item::GI_SHIELD_DEKU;
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};

/// `ACTOR_ITEM_SHIELD` (`actor_table.h`).
pub const ACTOR_ITEM_SHIELD: i16 = 0x00EE;

/// `Item_Shield_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_ITEM_SHIELD, name: "Item_Shield", category: ACTORCAT_ITEMACTION, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT };

const OBJECT: &str = "object_link_child";

/// `sCylinderInit`.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0004, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 15, height: 15, y_shift: 0, pos: [0; 3] },
};

/// `D_80B87200`: each flame's size through its 16 frames.
const D_80B87200: [f32; 16] = [0.3, 0.6, 0.9, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.85, 0.7, 0.55, 0.4, 0.25, 0.1, 0.0];
/// `D_80B87240`: each flame's colour intensity through its 16 frames.
const D_80B87240: [f32; 16] = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.8, 0.6, 0.4, 0.2, 0.0, 0.0, 0.0, 0.0, 0.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_80B86BC8`: lying, offered.
    Lying,
    /// `func_80B86AC8`: knocked up, falling, offered.
    Knocked,
    /// `func_80B86F68`: the burnt shield's first frame.
    TakePlace,
    /// `func_80B86CA8`: burning.
    Burning,
}

pub struct ItemShield {
    pub actor: Actor,
    pub collider: ColliderCylinder,
    /// `unk_198`: the rocking's speed.
    pub unk_198: i16,
    pub timer: i16,
    /// `unk_19C`: 1 upside down (never read), 2 hidden.
    pub unk_19c: i16,
    /// `unk_19E`: each flame's frames left.
    pub unk_19e: [u8; 8],
    /// `unk_1A8`: each flame's place about the shield.
    pub unk_1a8: [Vec3; 8],
    pub action: Action,
}

impl ItemShield {
    /// `ItemShield_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut s = ItemShield {
            actor: Actor::new(Vec3::ZERO, 0),
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            unk_198: 0,
            timer: 0,
            unk_19c: 0,
            unk_19e: [0; 8],
            unk_1a8: [Vec3::ZERO; 8],
            action: Action::Lying,
        };
        match actor.params {
            0 => {
                // ActorShape_Init(&shape, 1400, NULL, 0).
                actor.shape_y_offset = 1400.0;
                actor.shape_rot.x = 0x4000;
                s.action = Action::Lying;
            }
            1 => {
                actor.shape_y_offset = 0.0;
                s.action = Action::TakePlace;
                s.unk_19c |= 2;
                for i in 0..8 {
                    s.unk_19e[i] = 1 + 2 * i as u8;
                    s.unk_1a8[i].x = play.rand.centered_float(10.0);
                    s.unk_1a8[i].y = play.rand.centered_float(10.0);
                    s.unk_1a8[i].z = play.rand.centered_float(10.0);
                }
            }
            _ => {}
        }
        // Actor_SetScale(0.01).
        actor.scale = Vec3::splat(0.01);
        s.actor = actor;
        Box::new(s)
    }

    /// `func_80B86AC8`.
    fn knocked(&mut self, play: &mut PlayState) {
        self.actor.move_forward();
        if actor_has_parent(&self.actor) {
            self.actor.kill();
            return;
        }
        offer_get_item_range(play, &self.actor, GI_SHIELD_DEKU, 30.0, 50.0);
        self.actor.update_bg_check_info(&play.col, 10.0, 10.0, 0.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.timer -= 1;
            if self.timer < 60 {
                if self.timer & 1 != 0 {
                    self.unk_19c |= 2;
                } else {
                    self.unk_19c &= !2;
                }
            }
            if self.timer == 0 {
                self.actor.kill();
            }
        }
    }

    /// `func_80B86BC8`.
    fn lying(&mut self, play: &mut PlayState) {
        if actor_has_parent(&self.actor) {
            self.actor.kill();
            return;
        }
        offer_get_item_range(play, &self.actor, GI_SHIELD_DEKU, 30.0, 50.0);
        if self.collider.base.ac_flags & AC_HIT != 0 {
            self.action = Action::Knocked;
            self.actor.velocity.y = 4.0;
            self.actor.min_velocity_y = -4.0;
            self.actor.gravity = -0.8;
            self.actor.speed_xz = 0.0;
            self.timer = 160;
        } else {
            self.collider.update(&self.actor);
            play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
        }
    }

    /// `func_80B86CA8`.
    fn burning(&mut self, play: &mut PlayState) {
        self.actor.move_forward();
        self.actor.update_bg_check_info(&play.col, 10.0, 10.0, 0.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
        self.actor.shape_y_offset = sin_s(self.actor.shape_rot.x).abs() * 1500.0;
        let me = play.cur_actor;
        for i in 0..8 {
            let temp = 15 - self.unk_19e[i] as usize;
            // D_80B871F4 (a static rewritten each time).
            let pos = Vec3::new(self.unk_1a8[i].x, self.unk_1a8[i].y + (self.actor.shape_y_offset * 0.01) + (D_80B87200[temp] * -10.0 * 0.2), self.unk_1a8[i].z);
            if let Some(me) = me {
                let v = self.actor.velocity;
                play.with_ss(|s| s.fire_tail_spawn_flame(me, v, pos, D_80B87200[temp] * 0.2, -1, D_80B87240[temp]));
            }
            if self.unk_19e[i] != 0 {
                self.unk_19e[i] -= 1;
            } else if self.timer > 16 {
                self.unk_19e[i] = 15;
                self.unk_1a8[i].x = play.rand.centered_float(15.0);
                self.unk_1a8[i].y = play.rand.centered_float(10.0);
                self.unk_1a8[i].z = play.rand.centered_float(15.0);
            }
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.unk_198 = self.unk_198.wrapping_sub(self.actor.shape_rot.x >> 1);
            self.unk_198 = self.unk_198.wrapping_sub(self.unk_198 >> 2);
            self.actor.shape_rot.x = self.actor.shape_rot.x.wrapping_add(self.unk_198);
            if self.timer >= 8 && self.timer < 24 {
                // Actor_SetScale.
                self.actor.scale = Vec3::splat((self.timer - 8) as f32 * 0.000625);
            }
            if self.timer != 0 {
                self.timer -= 1;
            } else {
                self.actor.kill();
            }
        }
    }

    /// `func_80B86F68`: where Player's shield is (`shieldMf`'s translation), facing as it does.
    fn take_place(&mut self, play: &mut PlayState) {
        let Some(shield) = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map(|p| p.shield_mf()) else { return };
        self.actor.world_pos = Vec3::new(shield.xw, shield.yw, shield.zw);
        self.unk_19c &= !2;
        self.actor.shape_rot.y = atan2_s(-shield.zz, -shield.xz);
        self.actor.shape_rot.x = atan2_s(-shield.yz, (shield.zz * shield.zz + shield.xz * shield.xz).sqrt());
        if (self.actor.shape_rot.x as i32).abs() > 0x4000 {
            self.unk_19c |= 1;
        }
        self.action = Action::Burning;
        self.actor.velocity.y = 4.0;
        self.actor.min_velocity_y = -4.0;
        self.actor.gravity = -0.8;
        self.unk_198 = 0;
        self.timer = 70;
        self.actor.speed_xz = 0.0;
    }
}

impl ActorImpl for ItemShield {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `ItemShield_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Lying => self.lying(play),
            Action::Knocked => self.knocked(play),
            Action::TakePlace => self.take_place(play),
            Action::Burning => self.burning(play),
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.switches = vec![(self.unk_19c & 2 != 0) as u32];
        rs
    }
    /// `ItemShield_Draw`: the child's Deku Shield, unless hidden.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if rs.switches.first().copied().unwrap_or(0) != 0 {
            return;
        }
        out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh(OBJECT, "gLinkChildDekuShieldDL")), actor_draw_matrix(rs)));
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
