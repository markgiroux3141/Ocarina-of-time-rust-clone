//! `En_Nutsball` (`ovl_En_Nutsball/z_en_nutsball.c`): the Deku Scrubs' nut, fired straight
//! ahead at 10. `params` is the scrub's kind (`EnNutsballType`), which picks the object it waits
//! for and the nut's display list: 0 `En_Dekunuts`, 1 `En_Hintnuts`, 2 `En_Shopnuts`, 3 `En_Dns`,
//! 4 `En_Dnk`.
//!
//! - **Waiting** (`EnNutsball_WaitForObject`) for its object to load; then it shows and flies.
//! - **Flying** (`EnNutsball_Projectile`): straight for 30 frames, then falling (gravity -1),
//!   spinning on its face (`home.rot.z`, by 0x2AA8 a frame). Hitting a wall or the floor, or its
//!   collider touching anything (AT, AC or OC), it breaks: a burst of fragments
//!   (`EffectSsHahen_SpawnBurst`) and `NA_SE_EN_OCTAROCK_ROCK`. After 330 frames in the air it's
//!   gone.
//! - **Deflected**: with Link's Deku Shield (or the adult's Hylian Shield) up, a nut whose
//!   attack bounced off it (`AT_HIT`, `AT_TYPE_ENEMY` and `AT_BOUNCED`: the shield is `AC_HARD`)
//!   turns around, flying back the way the shield faces (`shieldMf`'s yaw + 0x8000) as Link's
//!   attack (`AT_TYPE_PLAYER`, `DMG_DEKU_STICK`), for another 30 frames before falling.
//!
//! The whole overlay is ported. The circle shadow (`ActorShadow_DrawCircle`, 13) isn't ported for
//! any actor.

use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile, PLAYER_STATE1_28, PLAYER_STATE1_29, PLAYER_STATE1_DEAD, PLAYER_STATE1_TALKING};
use oot_game::audio::sfx::NA_SE_EN_OCTAROCK_ROCK;
use oot_game::collision_check::*;
use oot_game::effect::hahen::HAHEN_OBJECT_DEFAULT;
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};

/// `ACTOR_EN_NUTSBALL` (`actor_table.h`).
pub const ACTOR_EN_NUTSBALL: i16 = 0x0193;

/// `En_Nutsball_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_NUTSBALL, name: "En_Nutsball", category: ACTORCAT_PROP, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: "gameplay_keep" };

/// `EnNutsballType`.
pub const EN_NUTSBALL_TYPE_DEKUNUTS: i16 = 0;
pub const EN_NUTSBALL_TYPE_HINTNUTS: i16 = 1;
pub const EN_NUTSBALL_TYPE_SHOPNUTS: i16 = 2;
pub const EN_NUTSBALL_TYPE_DNS: i16 = 3;
pub const EN_NUTSBALL_TYPE_DNK: i16 = 4;

/// `sObjectIds`: `OBJECT_DEKUNUTS`, `OBJECT_HINTNUTS`, `OBJECT_SHOPNUTS`, `OBJECT_DNS`,
/// `OBJECT_DNK` (`object_table.h`).
pub const OBJECT_IDS: [i16; 5] = [0x004A, 0x0164, 0x0168, 0x0171, 0x0172];
const OBJECT_FILES: [&str; 5] = ["object_dekunuts", "object_hintnuts", "object_shopnuts", "object_dns", "object_dnk"];
/// `sDLists`.
const DLISTS: [&str; 5] = ["gDekuNutsDekuNutDL", "gHintNutsNutDL", "gBusinessScrubDekuNutDL", "gDntJijiNutDL", "gDntStageNutDL"];

/// `PLAYER_SHIELD_DEKU`, `PLAYER_SHIELD_HYLIAN` (`player.h`).
const PLAYER_SHIELD_DEKU: u8 = 1;
const PLAYER_SHIELD_HYLIAN: u8 = 2;
/// `DMG_DEKU_STICK` (`collision_check.h`).
const DMG_DEKU_STICK: u32 = 1 << 1;

/// `sCylinderInit`.
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit {
        col_type: COL_MATERIAL_NONE,
        at_flags: AT_ON | AT_TYPE_ENEMY,
        ac_flags: AC_ON | AC_TYPE_PLAYER,
        oc_flags1: OC1_ON | OC1_TYPE_ALL,
        oc_flags2: OC2_TYPE_2,
        shape: COLSHAPE_CYLINDER,
    },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x08 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_WOOD,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 13, height: 13, y_shift: 0, pos: [0; 3] },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    WaitForObject,
    Projectile,
}

pub struct EnNutsball {
    pub actor: Actor,
    pub action: Action,
    /// `requiredObjectSlot`.
    pub required_object_slot: Option<usize>,
    pub timer: i16,
    pub collider: ColliderCylinder,
    /// Drawn (`actor.draw` set once the object is in).
    pub visible: bool,
}

impl EnNutsball {
    fn kind(&self) -> usize {
        (self.actor.params as usize).min(4)
    }

    /// `EnNutsball_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // ActorShape_Init(&shape, 400, ActorShadow_DrawCircle, 13): the circle shadow isn't ported.
        actor.shape_y_offset = 400.0;
        let collider = ColliderCylinder::new(&CYLINDER_INIT);
        let kind = (actor.params as usize).min(4);
        let required_object_slot = play.object_ctx.get_index(OBJECT_IDS[kind]);
        if required_object_slot.is_none() {
            actor.kill();
        }
        Box::new(EnNutsball { actor, action: Action::WaitForObject, required_object_slot, timer: 0, collider, visible: false })
    }

    /// `EnNutsball_WaitForObject`.
    fn wait_for_object(&mut self, play: &mut PlayState) {
        if let Some(slot) = self.required_object_slot
            && play.object_ctx.is_loaded(slot)
        {
            self.actor.obj_bank_index = Some(slot);
            self.visible = true;
            self.actor.shape_rot.y = 0;
            self.timer = 30;
            self.action = Action::Projectile;
            self.actor.speed_xz = 10.0;
        }
    }

    /// `EnNutsball_Projectile`.
    fn projectile(&mut self, play: &mut PlayState) {
        let player = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map(|p| (p.current_shield(), p.adult(), p.shield_mf()));
        self.timer = self.timer.wrapping_sub(1);
        if self.timer == 0 {
            self.actor.gravity = -1.0;
        }
        self.actor.home_rot.z = self.actor.home_rot.z.wrapping_add(0x2AA8);
        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0
            || self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0
            || self.collider.base.at_flags & AT_HIT != 0
            || self.collider.base.ac_flags & AC_HIT != 0
            || self.collider.base.oc_flags1 & OC1_HIT != 0
        {
            // Link's shield reflects it: the Deku Shield, or the Hylian Shield as an adult.
            if let Some((shield, adult, shield_mf)) = player
                && (shield == PLAYER_SHIELD_DEKU || (shield == PLAYER_SHIELD_HYLIAN && adult))
                && self.collider.base.at_flags & AT_HIT != 0
                && self.collider.base.at_flags & AT_TYPE_ENEMY != 0
                && self.collider.base.at_flags & AT_BOUNCED != 0
            {
                self.collider.base.at_flags &= !AT_TYPE_ENEMY & !AT_BOUNCED & !AT_HIT;
                self.collider.base.at_flags |= AT_TYPE_PLAYER;
                self.collider.info.at_dmg_info.dmg_flags = DMG_DEKU_STICK;
                let shield_rot = shield_mf.to_yxz_rot_s(false);
                self.actor.world_rot.y = shield_rot[1].wrapping_add(-0x8000i16);
                self.timer = 30;
                return;
            }
            let impact_pos = Vec3::new(self.actor.world_pos.x, self.actor.world_pos.y + 4.0, self.actor.world_pos.z);
            play.with_ss(|s| s.hahen_spawn_burst(impact_pos, 6.0, 0, 7, 3, 15, HAHEN_OBJECT_DEFAULT, 10, None));
            let pos = self.actor.world_pos;
            play.sfx_source_play_sfx_at_fixed_world_pos(pos, 20, NA_SE_EN_OCTAROCK_ROCK);
            self.actor.kill();
        } else if self.timer == -300 {
            self.actor.kill();
        }
    }
}

impl ActorImpl for EnNutsball {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnNutsball_Update`.
    fn update(&mut self, play: &mut PlayState) {
        let state1 = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map(|p| p.state_flags1()).unwrap_or(0);
        if state1 & (PLAYER_STATE1_TALKING | PLAYER_STATE1_DEAD | PLAYER_STATE1_28 | PLAYER_STATE1_29) == 0 || self.action == Action::WaitForObject {
            match self.action {
                Action::WaitForObject => self.wait_for_object(play),
                Action::Projectile => self.projectile(play),
            }
            self.actor.move_forward();
            let (r, h) = (CYLINDER_INIT.dim.radius as f32, CYLINDER_INIT.dim.height as f32);
            self.actor.update_bg_check_info(&play.col, 10.0, r, h, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
            self.collider.update(&self.actor);
            self.actor.flags |= ACTOR_FLAG_SFX_FOR_PLAYER_BODY_HIT;
            play.collision_check_set_at(&self.actor, 0, &mut self.collider);
            play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.angles = vec![self.actor.home_rot.z];
        rs
    }
    /// `EnNutsball_Draw`: the nut facing the camera (`billboardMtxF`), turned by `home.rot.z`
    /// (`* 9.58738e-05`: binary angle to radians).
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if !self.visible {
            return;
        }
        let rot_z = rs.angles.first().copied().unwrap_or(0) as f32 * 9.58738e-05;
        let m = actor_draw_matrix(rs) * play.billboard_mtx() * Mat4::from_rotation_z(rot_z);
        let k = self.kind();
        out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh(OBJECT_FILES[k], DLISTS[k])), m));
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
