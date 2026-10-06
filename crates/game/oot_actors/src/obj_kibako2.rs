//! `Obj_Kibako2` (`ovl_Obj_Kibako2/z_obj_kibako2.c`): the large crate, a DynaPoly actor of
//! `object_kibako2` (`gLargeCrateCol`, flags 0) drawn with `gLargeCrateDL`.
//!
//! Params: bit 15 clear spawns a Gold Skulltula (`En_Sw`, `params | 0x8000`) where it breaks.
//! `home.rot.x` is the collectible it drops (`ITEM00_*`, none outside `0..ITEM00_MAX`) and
//! `home.rot.z & 0x3F` its collectible flag; the init zeroes `rot.x` and `rot.z` after.
//!
//! It breaks (`ObjKibako2_Idle`) on a hit its cylinder takes (`0x40000040`: the hammer's swing
//! and jump only, not a sword), or an exploding explosive within reach (`func_80033684`: a
//! bomb's blast, params 1): 16 fragments (`EffectSsKakera`, `gLargeCrateFragmentDL`), seven
//! puffs of dust (`func_80033480`), `NA_SE_EV_WOODBOX_BREAK`, its collision gone and not drawn;
//! the next frame (`ObjKibako2_Kill`) the Gold Skulltula, the collectible, and it's gone.
//!
//! The Master Quest Deku Tree's: room 0's middle floor (279, 360, 333), params 0xFFFF, with its
//! Gold Skulltula placed inside it on its own (`En_Sw` 0x8102 at (278, 360, 332)); room 10's
//! two at (-805, 720, -2) and (-805, 720, -62), 0xFFFF. Each drops a green rupee (`rot.x` 0,
//! flag 0). Link has no hammer or bombs yet (GAME-05 milestone 4b): the tests inject the hit
//! and the explosion. The whole overlay is ported; the cull zone (`cullingVolume*`) isn't, for
//! any actor.

use std::sync::Arc;

use eng_collision::collision::CollisionHeader;
use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use eng_collision::math3d::Cylinder16;
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile, func_80033480, func_80033684};
use oot_game::collision_check::*;
use oot_game::effect::kakera::KAKERA_COLOR_NONE;
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

/// `ACTOR_OBJ_KIBAKO2` (`actor_table.h`: 0x01A0).
pub const ACTOR_OBJ_KIBAKO2: i16 = 0x01A0;
pub const OBJECT: &str = "object_kibako2";
/// `OBJECT_KIBAKO2` (`object_table.h`: 0x0170).
pub const OBJECT_KIBAKO2: i16 = 0x0170;
const COLLISION: &str = "gLargeCrateCol";

/// `NA_SE_EV_WOODBOX_BREAK` (`environmentbank_table.h`: 0x28AA).
pub const NA_SE_EV_WOODBOX_BREAK: u16 = 0x28AA;
/// `ITEM00_MAX` (`z_en_item00.h`: 0x1A).
const ITEM00_MAX: i16 = 0x1A;

/// `Obj_Kibako2_Profile`: `FLAGS` 0.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_OBJ_KIBAKO2, name: "Obj_Kibako2", category: ACTORCAT_BG, flags: 0, object: OBJECT };

/// `sCylinderInit`: 31 by 48, hit only by `0x40000040` (`DMG_HAMMER_JUMP | DMG_HAMMER_SWING`).
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x4000_0040, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_NONE,
    },
    dim: Cylinder16 { radius: 31, height: 48, y_shift: 0, pos: [0, 0, 0] },
};

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `ObjKibako2_Idle`.
    Idle,
    /// `ObjKibako2_Kill`.
    Kill,
}

pub struct ObjKibako2 {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    pub collider: ColliderCylinder,
    pub action: Action,
    pub collectible_flag: i16,
    /// `actor.draw` (NULL once broken).
    pub drawn: bool,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

/// `CollisionHeader_GetVirtual(&col, ..)`: one of `object`'s headers from the pack.
pub(crate) fn load_collision(play: &PlayState, object: &str, symbol: &str) -> Option<Arc<CollisionHeader>> {
    let r = play.assets.as_ref()?.pack.collision(&keys::collision(object, symbol));
    r.map(Arc::new).map_err(|e| log::error!("{object} {symbol}: {e:#}")).ok()
}

impl ObjKibako2 {
    /// `ObjKibako2_Init`: `DynaPolyActor_Init(0)`, scale 0.1 (`sInitChain`), the cylinder
    /// (`ObjKibako2_InitCollider`), `gLargeCrateCol`; the collectible flag from `home.rot.z`,
    /// then `rot.x` and `rot.z` zeroed (`home.rot.x`, the collectible, kept).
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: ICHAIN_VEC3F_DIV1000(scale, 100); cullingVolumeDistance 3000,
        // cullingVolumeScale 500, cullingVolumeDownward 1000 (the cull zone isn't ported).
        actor.scale = Vec3::splat(0.1);
        // ObjKibako2_InitCollider.
        let mut collider = ColliderCylinder::new(&CYLINDER_INIT);
        collider.update(&actor);
        let bg = match load_collision(play, OBJECT, COLLISION) {
            Some(h) => play.col.dyna.set_bg_actor(h, source(&actor), 0),
            None => BG_ACTOR_MAX,
        };
        let collectible_flag = actor.home_rot.z & 0x3F;
        actor.home_rot.z = 0;
        actor.world_rot.z = 0;
        actor.shape_rot.z = 0;
        actor.world_rot.x = 0;
        actor.shape_rot.x = 0;
        log::debug!("Wooden box (stationary)(arg {:04x}H)(item {:04x}H {})", actor.params, collectible_flag, actor.home_rot.x);
        Box::new(ObjKibako2 { actor, bg, collider, action: Action::Idle, collectible_flag, drawn: true })
    }

    /// `ObjKibako2_Break`: 16 fragments round it, a turn of 0x4E20 apart (up to 30 out, 2 to 12
    /// up, flying out at a fifth of that and up at 2 to 12; tumbling slowly 70% of the time,
    /// fast 5%, else medium; 5 to 35 big), then seven puffs of dust 90 across
    /// (`func_80033480`, 100 big, lit).
    pub fn break_(&mut self, play: &mut PlayState) {
        let this_pos = self.actor.world_pos;
        play.with_ss(|ss| {
            let mut angle: i16 = 0;
            for _ in 0..0x10 {
                let sn = eng_math::sin_s(angle);
                let cs = eng_math::cos_s(angle);
                let temp_rand = ss.rand.zero_one() * 30.0;
                let mut pos = Vec3::new(sn * temp_rand, (ss.rand.zero_one() * 10.0) + 2.0, cs * temp_rand);
                let velocity = Vec3::new(pos.x * 0.2, (ss.rand.zero_one() * 10.0) + 2.0, pos.z * 0.2);
                pos += this_pos;
                let temp_rand = ss.rand.zero_one();
                let phi_s0 = if temp_rand < 0.05 {
                    0x60
                } else if temp_rand < 0.7 {
                    0x40
                } else {
                    0x20
                };
                let scale = ((ss.rand.zero_one() * 30.0) + 5.0) as i16;
                ss.kakera_spawn(pos, velocity, pos, -200, phi_s0, 28, 2, 0, scale, 0, 0, 70, KAKERA_COLOR_NONE, OBJECT_KIBAKO2, Some((OBJECT, "gLargeCrateFragmentDL")));
                angle = angle.wrapping_add(0x4E20);
            }
        });
        func_80033480(play, this_pos, 90.0, 6, 100, 160, 1);
    }

    /// `ObjKibako2_SpawnCollectible`: `home.rot.x` with the collectible flag, if it's an item.
    fn spawn_collectible(&self, play: &mut PlayState) {
        let collectible_flag_temp = self.collectible_flag;
        let item_dropped = self.actor.home_rot.x;
        if (0..ITEM00_MAX).contains(&item_dropped) {
            crate::en_item00::item_drop_collectible(play, self.actor.world_pos, item_dropped | (collectible_flag_temp << 8));
        }
    }

    /// `ObjKibako2_Idle`: a hit, `home.rot.z` (zeroed at init) or an explosion within reach
    /// breaks it, with `NA_SE_EV_WOODBOX_BREAK` (20 frames at its position), its collision and
    /// draw gone; else within 600 of Link, its cylinder is set for hits.
    fn idle(&mut self, play: &mut PlayState) {
        if self.collider.base.ac_flags & AC_HIT != 0 || self.actor.home_rot.z != 0 || func_80033684(play, play.cur_actor, &self.actor).is_some() {
            self.break_(play);
            play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.world_pos, 20, NA_SE_EV_WOODBOX_BREAK);
            self.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
            play.col.dyna.set_collision_disabled(self.bg, true);
            self.drawn = false;
            self.action = Action::Kill;
        } else if self.actor.xz_dist_to_player < 600.0 {
            play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
        }
    }

    /// `ObjKibako2_Kill`: the Gold Skulltula (`En_Sw`, `params | 0x8000`, turned as the crate)
    /// unless params bit 15 is set, the collectible, and it's gone.
    fn kill(&mut self, play: &mut PlayState) {
        let params = self.actor.params;
        if params as u16 & 0x8000 == 0 {
            let p = self.actor.world_pos;
            let _ = play.actor_spawn(crate::en_sw::ACTOR_EN_SW, p, [0, self.actor.shape_rot.y, 0], (params as u16 | 0x8000) as i16);
        }
        self.spawn_collectible(play);
        self.actor.kill();
    }
}

impl ActorImpl for ObjKibako2 {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `ObjKibako2_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Idle => self.idle(play),
            Action::Kill => self.kill(play),
        }
        play.col.dyna.set_source(self.bg, source(&self.actor));
    }

    /// `ObjKibako2_Destroy`: the cylinder (nothing to free) and the bg actor.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }

    fn dyna_bg_id(&self) -> Option<u16> {
        Some(self.bg)
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.switches = vec![self.drawn as u32];
        rs
    }

    /// `ObjKibako2_Draw`: `Gfx_DrawDListOpa(gLargeCrateDL)`, while it's whole.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if rs.switches.first() == Some(&1) {
            crate::gfx_draw_dlist_opa(out, OBJECT, "gLargeCrateDL", rs);
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
