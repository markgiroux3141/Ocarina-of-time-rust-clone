//! `Door_Ana` (`ovl_Door_Ana/z_door_ana.c`): a grotto's hole (GAME-06 milestone 4).
//!
//! Params: bits 12..14 the destination (`sGrottoEntrances[n - 1]`: the fairy fountain or one of
//! the fourteen grottos; 0 takes `home.rot.z + 1`), bits 8 and 9 hidden (8 opened by the Song of
//! Storms, 9 by an explosion or the hammer, its cylinder 50 by 10 taking only those), the low
//! byte the grotto's own data (the return point's `data`).
//!
//! Open, the hole grows to its size (0.01) and, with Link standing within 15 of its middle (and
//! between 50 below and 15 above), marks him to fall in (`PLAYER_STATE1_31`): falling off its
//! edge he drops through (`func_80838FB8`), and once he's falling the hole sets the return point
//! (`RESPAWN_MODE_RETURN`: here, facing `home.rot.y`, Link launched up on return,
//! `PLAYER_START_MODE_GROTTO`) and the next entrance; it holds him over itself while he falls.
//! It always faces the camera.
//!
//! The whole overlay is ported (`Actor_SetClosestSecretDistance`, the Stone of Agony's rumble,
//! isn't: no rumble).

use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{cos_s, sin_s, step_to_f};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_UPDATE_DURING_OCARINA, Actor};
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorImpl, ActorProfile};
use oot_game::audio::sfx::NA_SE_SY_CORRECT_CHIME;
use oot_game::collision_check::*;
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};

/// `ACTOR_DOOR_ANA` (`actor_table.h`: 0x009B).
pub const ACTOR_DOOR_ANA: i16 = 0x009B;
const FIELD_KEEP: &str = "gameplay_field_keep";

/// `Door_Ana_Profile`: `ACTORCAT_ITEMACTION`, `ACTOR_FLAG_UPDATE_DURING_OCARINA`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_DOOR_ANA, name: "Door_Ana", category: ACTORCAT_ITEMACTION, flags: ACTOR_FLAG_UPDATE_DURING_OCARINA, object: FIELD_KEEP };

/// `sCylinderInit`: a bombable hole's, 50 by 10, hit by explosions and the hammer (0x48).
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_NONE, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK2,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0048, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_NONE,
    },
    dim: Cylinder16 { radius: 50, height: 10, y_shift: 0, pos: [0, 0, 0] },
};

/// `sGrottoEntrances`.
pub const GROTTO_ENTRANCES: [&str; 15] = [
    "ENTR_FAIRYS_FOUNTAIN_0",
    "ENTR_GROTTOS_0",
    "ENTR_GROTTOS_1",
    "ENTR_GROTTOS_2",
    "ENTR_GROTTOS_3",
    "ENTR_GROTTOS_4",
    "ENTR_GROTTOS_5",
    "ENTR_GROTTOS_6",
    "ENTR_GROTTOS_7",
    "ENTR_GROTTOS_8",
    "ENTR_GROTTOS_9",
    "ENTR_GROTTOS_10",
    "ENTR_GROTTOS_11",
    "ENTR_GROTTOS_12",
    "ENTR_GROTTOS_13",
];

/// `RESPAWN_MODE_RETURN` (`save.h`).
const RESPAWN_MODE_RETURN: usize = 1;
/// `PLAYER_PARAMS(PLAYER_START_MODE_GROTTO, PLAYER_START_BG_CAM_DEFAULT)`.
const GROTTO_PLAYER_PARAMS: i16 = (4 << 8) | 0xFF;
/// `PLAYER_STATE1_23`, `_27`, `_31` (`player.h`).
const PLAYER_STATE1_23: u32 = 1 << 23;
const PLAYER_STATE1_27: u32 = 1 << 27;
const PLAYER_STATE1_31: u32 = 1 << 31;

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `DoorAna_WaitClosed`.
    WaitClosed,
    /// `DoorAna_WaitOpen`.
    WaitOpen,
    /// `DoorAna_GrabPlayer`.
    GrabPlayer,
}

pub struct DoorAna {
    pub actor: Actor,
    pub collider: ColliderCylinder,
    pub action: Action,
}

impl DoorAna {
    fn bombable(&self) -> bool {
        self.actor.params & 0x0200 != 0
    }

    /// `DoorAna_Init`: facing nowhere; hidden ones at scale 0 (a bombable one with its cylinder,
    /// a song's always updating), waiting closed; else open. `ATTENTION_RANGE_0`.
    pub fn init(mut actor: Actor, _play: &mut PlayState) -> Box<dyn ActorImpl> {
        actor.shape_rot.z = 0;
        actor.shape_rot.y = 0;
        let collider = ColliderCylinder::new(&CYLINDER_INIT);
        let mut action = Action::WaitOpen;
        if actor.params & 0x0300 != 0 {
            if actor.params & 0x0200 == 0 {
                actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
            }
            actor.scale = Vec3::ZERO;
            action = Action::WaitClosed;
        }
        actor.target_mode = 0;
        Box::new(DoorAna { actor, collider, action })
    }

    /// `DoorAna_WaitClosed`: opened by the Song of Storms within 200 (`CutsceneFlags_Get(5)`)
    /// or by a hit on its cylinder; open, the chime.
    fn wait_closed(&mut self, play: &mut PlayState) {
        let mut open = false;
        if !self.bombable() {
            if self.actor.xyz_dist_to_player_sq < 200.0 * 200.0 && play.flags_get_env(5) {
                open = true;
                self.actor.flags &= !ACTOR_FLAG_UPDATE_CULLING_DISABLED;
            }
        } else if self.collider.base.ac_flags & AC_HIT != 0 {
            open = true;
        } else {
            self.collider.update(&self.actor);
            play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
        }
        if open {
            self.actor.params &= !0x0300;
            self.action = Action::WaitOpen;
            play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
        }
        // (Actor_SetClosestSecretDistance: the Stone of Agony isn't ported.)
    }

    /// `DoorAna_WaitOpen`: grown in, with Link falling in (`PLAYER_STATE1_31`, `av1.actionVar1`
    /// 0, no transition): the return point and the next entrance, then holding him; else Link
    /// standing over its middle (not in a cutscene, swimming or on the ground's edge) is marked to
    /// fall (`ATTENTION_RANGE_1`).
    fn wait_open(&mut self, play: &mut PlayState) {
        let Some(ph) = play.player else { return };
        let (state1, action_var1) = play.actors.get(ph).and_then(|p| p.as_player()).map(|p| (p.state_flags1(), p.action_var1())).unwrap_or((0, 0));
        if step_to_f(&mut self.actor.scale.x, 0.01, 0.001) {
            if self.actor.target_mode != 0 && play.transition.trigger == oot_game::transition::TRANS_TRIGGER_OFF && state1 & PLAYER_STATE1_31 != 0 && action_var1 == 0 {
                let mut destination = ((self.actor.params >> 12) & 7) - 1;
                play.setup_respawn_point(RESPAWN_MODE_RETURN, GROTTO_PLAYER_PARAMS);
                let r = &mut play.save.respawn[RESPAWN_MODE_RETURN];
                r.pos.y = self.actor.world_pos.y;
                r.yaw = self.actor.home_rot.y;
                r.data = self.actor.params as i8;
                if destination < 0 {
                    destination = self.actor.home_rot.z + 1;
                }
                let name = GROTTO_ENTRANCES.get(destination as usize).copied().unwrap_or(GROTTO_ENTRANCES[0]);
                match play.assets.as_ref().and_then(|a| a.scenes.entrance_index(name)) {
                    Some(e) => play.transition.next_entrance_index = e,
                    None => log::error!("Door_Ana: no entrance {name}"),
                }
                self.action = Action::GrabPlayer;
            } else if !play.player_in_cs_mode()
                && state1 & (PLAYER_STATE1_23 | PLAYER_STATE1_27) == 0
                && self.actor.xz_dist_to_player <= 15.0
                && -50.0 <= self.actor.y_dist_to_player
                && self.actor.y_dist_to_player <= 15.0
            {
                if let Some(p) = play.actors.get_mut(ph).and_then(|p| p.as_player_mut()) {
                    p.change_state_flags1(PLAYER_STATE1_31, 0);
                }
                self.actor.target_mode = 1;
            } else {
                self.actor.target_mode = 0;
            }
        }
        let s = self.actor.scale.x;
        self.actor.scale = Vec3::splat(s);
    }

    /// `DoorAna_GrabPlayer`: Link below it and past 15 across is put back 15 from its middle,
    /// towards where he is.
    fn grab_player(&mut self, play: &mut PlayState) {
        if self.actor.y_dist_to_player <= 0.0 && 15.0 < self.actor.xz_dist_to_player {
            let (yaw, pos) = (self.actor.yaw_towards_player, self.actor.world_pos);
            if let Some(p) = play.player.and_then(|h| play.actors.actor_mut(h)) {
                p.world_pos.x = sin_s(yaw) * 15.0 + pos.x;
                p.world_pos.z = cos_s(yaw) * 15.0 + pos.z;
            }
        }
    }
}

impl ActorImpl for DoorAna {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `DoorAna_Update`: the action, then facing the camera (`Camera_GetCamDirYaw + 0x8000`).
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::WaitClosed => self.wait_closed(play),
            Action::WaitOpen => self.wait_open(play),
            Action::GrabPlayer => self.grab_player(play),
        }
        self.actor.shape_rot.y = play.cam_dir_yaw().wrapping_add(i16::MIN);
    }

    /// `DoorAna_Draw`: `gGrottoDL` after `Gfx_SetupDL_25Xlu`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        out.xlu.push(DrawCmd::new(MeshKey::named(keys::mesh(FIELD_KEEP, "gGrottoDL")), actor_draw_matrix(rs)));
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0 && self.bombable()).then_some(ColliderMut::Cylinder(&mut self.collider))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
