//! `En_Butte` (`ovl_En_Butte/z_en_butte.c`): a butterfly. `params & 1`: it turns into a fairy for
//! a Deku Stick (the first of an `Obj_Mure`'s butterflies).
//!
//! It flutters about its home (`EnButte_FlyAround`: in spells of flight picked at random
//! (`sFlyAroundParams`), bobbing (`func_809CD56C`), after another butterfly of its group
//! (`actor.child`, set by `Obj_Mure`) or away from Link when he's near). The fairy one follows a
//! Deku Stick held still (`EnButte_FollowLink`) and, led far enough from home, turns into a
//! fairy (`EnButte_TransformIntoFairy`: a flash, then an `En_Elf`).
//!
//! Drawn as `EnButte_Draw` does: the skeleton from `gameplay_field_keep`, its sphere moved to it
//! (`Collider_UpdateSpheres`), and the transformation's flash
//! (`EnButte_DrawTransformationEffect`); from baked meshes (docs/adr/0012-actor-bakes.md).
//!
//! Params: `PARAMS_GET_U(params, 0, 1)` bit 0 (1: it turns into a fairy for a Deku Stick); a
//! params of -1 is made 0 first (a plain butterfly). Only spawned, by `Obj_Mure` (-1, or 1 for a
//! group's first).
//! Flags: none.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Sphere16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{binang_to_rad, scaled_step_to_s, sin_s, smooth_step_to_f, step_to_f, vec3f_yaw};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::collision_check::*;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::*;

use crate::en_elf::{ACTOR_EN_ELF, FAIRY_HEAL_TIMED};

/// `ACTOR_EN_BUTTE` (`actor_table.h`: 0x001E).
pub const ACTOR_EN_BUTTE: i16 = 0x001E;
pub const OBJECT: &str = "gameplay_field_keep";

/// `En_Butte_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_BUTTE, name: "En_Butte", category: ACTORCAT_ITEMACTION, flags: 0, object: OBJECT, schema: &SCHEMA };

/// `PARAMS_GET_U(this->actor.params, 0, 1)`: the fairy one.
pub const FAIRY: Field =
    Field::params("PARAMS_GET_U(params, 0, 1)", 0, 1, Kind::Named(&[(0, "a butterfly"), (1, "turns into a fairy for a Deku Stick")]), "whether it turns into a fairy (a params of -1: made 0, a butterfly)");

/// En_Butte's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[FAIRY], flags: &[], todo: false };

/// `NA_SE_EV_BUTTERFRY_TO_FAIRY` (`sfx/environmentbank_table.h`: 0x28E7).
const NA_SE_EV_BUTTERFRY_TO_FAIRY: u16 = 0x28E7;
/// `PLAYER_IA_DEKU_STICK` (`player.h`: 6).
const PLAYER_IA_DEKU_STICK: i32 = 6;

/// `sJntSphElementsInit`: one sphere of 5 at the root.
const JNT_SPH_ELEMENTS: [ColliderJntSphElementInit; 1] = [ColliderJntSphElementInit {
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: 0, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: 0, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    limb: 0,
    model_sphere: Sphere16 { center: [0, 0, 0], radius: 5 },
    scale: 100,
}];

/// `sColliderJntSphInit`.
const JNT_SPH_INIT: ColliderInit =
    ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_PLAYER | OC1_TYPE_1, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_JNTSPH };

/// `EnButteFlightParams`: (`minTime`, `maxTime`, `speedXZTarget`, `speedXZScale`,
/// `speedXZStep`, `rotYStep`).
type FlightParams = (i16, i16, f32, f32, f32, i16);
/// `sFlyAroundParams`.
const FLY_AROUND_PARAMS: [FlightParams; 3] = [(5, 35, 0.0, 0.1, 0.5, 0), (10, 45, 1.1, 0.1, 0.25, 1000), (10, 40, 1.5, 0.1, 0.3, 2000)];
/// `sFollowLinkParams`.
const FOLLOW_LINK_PARAMS: [FlightParams; 3] = [(3, 3, 0.8, 0.1, 0.2, 0), (10, 20, 2.0, 0.3, 1.0, 0), (10, 20, 2.4, 0.3, 1.0, 0)];

/// The overlay's statics.
struct Statics {
    /// `sTransformationEffectScale`, `sTransformationEffectAlpha`.
    transformation_effect_scale: f32,
    transformation_effect_alpha: i16,
    /// `EnButte_FollowLink`'s `D_809CE410`: the swing about the stick's tip, flipped each spell.
    d_809ce410: i32,
}

impl Default for Statics {
    fn default() -> Statics {
        Statics { transformation_effect_scale: 0.0, transformation_effect_alpha: 0, d_809ce410: 1500 }
    }
}

fn statics(play: &mut PlayState) -> &mut Statics {
    play.overlay_static::<Statics>(ACTOR_EN_BUTTE)
}

/// The butterfly (`gButterflySkel`).
pub const BAKE: &str = "En_Butte/skel";
/// The transformation's flash: the colours (segment 10, set every frame), `gEffFlash1DL`.
pub const FLASH_BAKE: &str = "En_Butte/flash";
const SEG_COLOR: u8 = 0x0A;

/// The meshes `EnButte_Draw` draws.
pub fn bakes() -> Vec<MeshBake> {
    vec![
        MeshBake {
            name: BAKE.into(),
            object: OBJECT.into(),
            segments: Vec::new(),
            prelude: Vec::new(),
            body: BakeBody::Skeleton { file: OBJECT.into(), symbol: "gButterflySkel".into(), limbs: Vec::new() },
        },
        MeshBake {
            name: FLASH_BAKE.into(),
            object: "gameplay_keep".into(),
            // gDPSetPrimColor(0x80, 0x80, 200, 200, 180, alpha), gDPSetEnvColor(200, 200, 210, 255).
            segments: vec![(SEG_COLOR, BakeSegment::Dynamic(vec![(0xFA00_8080, 0xC8C8_B4FF), (0xFB00_0000, 0xC8C8_D2FF), (0xDF00_0000, 0)]))],
            prelude: vec![SEG_COLOR],
            body: BakeBody::DLists(vec![("gameplay_keep".into(), "gEffFlash1DL".into())]),
        },
    ]
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    FlyAround,
    FollowLink,
    TransformIntoFairy,
    WaitToDie,
}

pub struct EnButte {
    pub actor: Actor,
    pub collider: ColliderJntSph,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub action: Action,
    pub timer: i16,
    pub flight_params_idx: u8,
    /// `unk_257`: 0 free, 1 Link near (flee), 2 following the stick.
    pub unk_257: u8,
    pub draw_skel_anime: bool,
    pub sword_down_timer: i16,
    pub unk_25c: i16,
    pub unk_25e: i16,
    pub unk_260: i16,
    pub pos_y_target: f32,
    /// `actor.draw` isn't NULL (`EnButte_SetupWaitToDie` clears it).
    pub drawn: bool,
}

/// `Math3D_Dist2DSq`.
fn dist_2d_sq(x0: f32, y0: f32, x1: f32, y1: f32) -> f32 {
    let (dx, dy) = (x0 - x1, y0 - y1);
    dx * dx + dy * dy
}

impl EnButte {
    /// `EnButte_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = (|| -> anyhow::Result<_> { Ok((assets.skeleton(OBJECT, "gButterflySkel")?, assets.animation(OBJECT, "gButterflyAnim")?)) })();
        let (skeleton, anim) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Butte: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let mut this = EnButte {
            collider: ColliderJntSph::new(&JNT_SPH_INIT, &JNT_SPH_ELEMENTS),
            // SkelAnime_Init(.., gButterflySkel, gButterflyAnim, .., 8).
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(anim.clone())),
            skeleton,
            action: Action::FlyAround,
            timer: 0,
            flight_params_idx: 0,
            unk_257: 0,
            draw_skel_anime: false,
            sword_down_timer: 0,
            unk_25c: 0,
            unk_25e: 0,
            unk_260: 0,
            pos_y_target: 0.0,
            drawn: true,
            actor,
        };
        let a = &mut this.actor;
        // schema: the whole params, -1 (Obj_Mure's plain butterfly) made 0
        if a.params == -1 {
            a.params = 0;
        }
        // sInitChain.
        a.scale = Vec3::splat(0.01);
        a.culling_volume_distance = 700.0;
        a.culling_volume_scale = 20.0;
        a.culling_volume_downward = 600.0;
        if FAIRY.get(a.params) == 1 {
            a.culling_volume_scale = 200.0;
        }
        a.col_chk_info.mass = 0;
        this.unk_25c = (play.rand.zero_one() * 65535.0) as i32 as i16;
        this.unk_25e = (play.rand.zero_one() * 65535.0) as i32 as i16;
        this.unk_260 = (play.rand.zero_one() * 65535.0) as i32 as i16;
        this.skel.change(anim, 1.0, 0.0, 0.0, ANIMMODE_LOOP_INTERP, 0.0);
        this.setup_fly_around(play);
        this.actor.shape_rot.x = this.actor.shape_rot.x.wrapping_sub(0x2320);
        this.draw_skel_anime = true;
        // schema: printed
        log::debug!("En_Butte: (field keep butterfly)(arg_data {:#06x})", this.actor.params);
        Box::new(this)
    }

    fn fairy(&self) -> bool {
        FAIRY.get(self.actor.params) == 1
    }

    /// `EnButte_SelectFlightParams`: the next spell, still or flying, and its length.
    fn select_flight_params(&mut self, play: &mut PlayState, params: FlightParams) {
        if self.flight_params_idx == 0 {
            self.flight_params_idx = if play.rand.zero_one() < 0.6 { 1 } else { 2 };
        } else {
            self.flight_params_idx = 0;
        }
        self.timer = play.rand.s16_offset(params.0, params.1);
    }

    /// `func_809CD56C`: the flight's bob.
    fn func_809cd56c(&mut self) {
        const D_809CE3E0: [f32; 3] = [50.0, 80.0, 100.0];
        const D_809CE3EC: [f32; 3] = [30.0, 40.0, 50.0];
        let i = self.flight_params_idx as usize;
        self.actor.shape_y_offset += sin_s(self.unk_25c) * D_809CE3E0[i] + sin_s(self.unk_25e) * D_809CE3EC[i];
        self.actor.shape_y_offset = self.actor.shape_y_offset.clamp(-2000.0, 2000.0);
    }

    /// `func_809CD634`: the following's smaller bob.
    fn func_809cd634(&mut self) {
        const D_809CE3F8: [f32; 3] = [15.0, 20.0, 25.0];
        const D_809CE404: [f32; 3] = [7.5, 10.0, 12.5];
        let i = self.flight_params_idx as usize;
        self.actor.shape_y_offset += sin_s(self.unk_25c) * D_809CE3F8[i] + sin_s(self.unk_25e) * D_809CE404[i];
        self.actor.shape_y_offset = self.actor.shape_y_offset.clamp(-500.0, 500.0);
    }

    /// `EnButte_Turn`: the body turned to fly backwards along its heading, tilting.
    fn turn(&mut self) {
        let target = self.actor.world_rot.y.wrapping_add(i16::MIN);
        let diff = target.wrapping_sub(self.actor.shape_rot.y);
        scaled_step_to_s(&mut self.actor.shape_rot.y, target, (diff as i32).abs() as i16 >> 3);
        // sinf(unk_260): the binang as radians.
        self.actor.shape_rot.x = ((self.unk_260 as f32).sin() * 600.0) as i32 as i16 - 0x2320;
    }

    /// The wings' speed: `speed / 2 + rand * 0.2 + ... + minAnimSpeed`, clamped to 0.2 to 1.5.
    fn update_anim(&mut self, play: &mut PlayState, min_anim_speed: f32) {
        let anim_speed = self.actor.speed_xz / 2.0 + play.rand.zero_one() * 0.2 + (1.0 - sin_s(self.unk_260)) * 0.15 + (1.0 - sin_s(self.unk_25e)) * 0.3 + min_anim_speed;
        self.skel.play_speed = anim_speed.clamp(0.2, 1.5);
        self.skel.update();
    }

    fn player_pos(play: &PlayState) -> Vec3 {
        play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(Vec3::ZERO)
    }

    /// (`heldItemAction`, the sword's tip, `speed`, `shape.rot.y`, `meleeWeaponState`).
    fn player_info(play: &PlayState) -> (i32, Vec3, f32, i16, i8) {
        let rot_y = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.shape_rot.y).unwrap_or(0);
        play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map(|p| (p.held_item_action(), p.melee_weapon_tip(), p.speed_xz(), rot_y, p.melee_weapon_state())).unwrap_or((
            0,
            Vec3::ZERO,
            0.0,
            rot_y,
            0,
        ))
    }

    /// `EnButte_SetupFlyAround`.
    fn setup_fly_around(&mut self, play: &mut PlayState) {
        self.select_flight_params(play, FLY_AROUND_PARAMS[self.flight_params_idx as usize]);
        self.action = Action::FlyAround;
    }

    /// `EnButte_FlyAround`.
    fn fly_around(&mut self, play: &mut PlayState) {
        let flight_params = FLY_AROUND_PARAMS[self.flight_params_idx as usize];
        let (w, home) = (self.actor.world_pos, self.actor.home_pos);
        let dist_sq_from_home = dist_2d_sq(w.x, w.z, home.x, home.z);
        self.func_809cd56c();
        smooth_step_to_f(&mut self.actor.speed_xz, flight_params.2, flight_params.3, flight_params.4, 0.0);

        let (max_dist_sq_from_home, rot_step) = if self.unk_257 == 1 { (100.0f32 * 100.0, 1000) } else { (35.0 * 35.0, 600) };

        let mut min_anim_speed = 0.0;
        self.pos_y_target = self.actor.home_pos.y;

        let child_pos = self.actor.child.filter(|&c| Some(c) != play.cur_actor).and_then(|c| play.actors.actor(c)).map(|a| a.world_pos);
        if self.flight_params_idx != 0 && (dist_sq_from_home > max_dist_sq_from_home || self.timer < 4) {
            let yaw = vec3f_yaw(self.actor.world_pos, self.actor.home_pos);
            if !scaled_step_to_s(&mut self.actor.world_rot.y, yaw, flight_params.5) {
                min_anim_speed = 0.5;
            }
        } else if self.unk_257 == 0 && child_pos.is_some() {
            let yaw = vec3f_yaw(self.actor.world_pos, child_pos.unwrap_or_default());
            if !scaled_step_to_s(&mut self.actor.world_rot.y, yaw, rot_step) {
                min_anim_speed = 0.3;
            }
        } else if self.unk_257 == 1 {
            let yaw = self.actor.yaw_towards_player.wrapping_add(i16::MIN).wrapping_add(((play.rand.zero_one() - 0.5) * 24576.0) as i32 as i16);
            if !scaled_step_to_s(&mut self.actor.world_rot.y, yaw, rot_step) {
                min_anim_speed = 0.4;
            }
        } else {
            self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(((self.unk_25c as f32).sin() * 100.0) as i32 as i16);
        }

        self.turn();
        self.update_anim(play, min_anim_speed);

        if self.timer <= 0 {
            self.select_flight_params(play, FLY_AROUND_PARAMS[self.flight_params_idx as usize]);
        }

        let (held_item_action, _, _, _, _) = Self::player_info(play);
        let pp = Self::player_pos(play);
        if self.fairy() && held_item_action == PLAYER_IA_DEKU_STICK && self.sword_down_timer <= 0 && (dist_2d_sq(pp.x, pp.z, home.x, home.z) < 120.0 * 120.0 || self.actor.xz_dist_to_player < 60.0) {
            self.setup_follow_link(play);
            self.unk_257 = 2;
        } else if (self.actor.xz_dist_to_player as f64) < 120.0 {
            self.unk_257 = 1;
        } else {
            self.unk_257 = 0;
        }
    }

    /// `EnButte_SetupFollowLink`.
    fn setup_follow_link(&mut self, play: &mut PlayState) {
        self.select_flight_params(play, FOLLOW_LINK_PARAMS[self.flight_params_idx as usize]);
        self.action = Action::FollowLink;
    }

    /// `EnButte_FollowLink`: after the stick's tip; far from home near it, a fairy.
    fn follow_link(&mut self, play: &mut PlayState) {
        let flight_params = FOLLOW_LINK_PARAMS[self.flight_params_idx as usize];
        let (held_item_action, tip, player_speed, player_rot_y, _) = Self::player_info(play);
        let pp = Self::player_pos(play);

        self.func_809cd634();
        smooth_step_to_f(&mut self.actor.speed_xz, flight_params.2, flight_params.3, flight_params.4, 0.0);
        let mut min_anim_speed = 0.0;

        if self.flight_params_idx != 0 && self.timer < 12 {
            let sword_tip = Vec3::new(tip.x + sin_s(player_rot_y) * 10.0, tip.y, tip.z + eng_math::cos_s(player_rot_y) * 10.0);
            let swing = statics(play).d_809ce410;
            let yaw = vec3f_yaw(self.actor.world_pos, sword_tip).wrapping_add((play.rand.zero_one() * swing as f32) as i32 as i16);
            if scaled_step_to_s(&mut self.actor.world_rot.y, yaw, 2000) {
                if play.gameplay_frames % 2 != 0 {
                    self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(((self.unk_25c as f32).sin() * 60.0) as i32 as i16);
                }
            } else {
                min_anim_speed = 0.3;
            }
        }

        self.pos_y_target = (pp.y + 30.0).max(tip.y);

        self.turn();
        self.update_anim(play, min_anim_speed);

        if self.timer <= 0 {
            self.select_flight_params(play, FOLLOW_LINK_PARAMS[self.flight_params_idx as usize]);
            let s = statics(play);
            s.d_809ce410 = -s.d_809ce410;
        }

        let (w, home) = (self.actor.world_pos, self.actor.home_pos);
        let dist_sq_from_home = dist_2d_sq(w.x, w.z, home.x, home.z);
        if !(held_item_action == PLAYER_IA_DEKU_STICK && player_speed.abs() < 1.8 && self.sword_down_timer <= 0 && dist_sq_from_home < 320.0 * 320.0) {
            self.setup_fly_around(play);
        } else if dist_sq_from_home > 240.0 * 240.0 {
            let dist_sq_from_sword = dist_2d_sq(tip.x, tip.z, w.x, w.z);
            if dist_sq_from_sword < 60.0 * 60.0 {
                self.setup_transform_into_fairy(play);
            }
        }
    }

    /// `EnButte_SetupTransformIntoFairy`.
    fn setup_transform_into_fairy(&mut self, play: &mut PlayState) {
        self.timer = 9;
        self.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
        self.skel.play_speed = 1.0;
        // EnButte_ResetTransformationEffect.
        let s = statics(play);
        s.transformation_effect_scale = 0.0;
        s.transformation_effect_alpha = 0;
        self.action = Action::TransformIntoFairy;
    }

    /// `EnButte_TransformIntoFairy`.
    fn transform_into_fairy(&mut self, play: &mut PlayState) {
        self.skel.update();
        // EnButte_UpdateTransformationEffect.
        let s = statics(play);
        s.transformation_effect_scale += 0.003;
        s.transformation_effect_alpha = s.transformation_effect_alpha.wrapping_add(4000);

        if self.timer == 5 {
            play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.world_pos, 60, NA_SE_EV_BUTTERFRY_TO_FAIRY);
        } else if self.timer == 4 {
            let _ = play.actor_spawn(ACTOR_EN_ELF, self.actor.focus_pos, [0, self.actor.shape_rot.y, 0], FAIRY_HEAL_TIMED);
            self.draw_skel_anime = false;
        } else if self.timer <= 0 {
            // EnButte_SetupWaitToDie.
            self.timer = 64;
            self.action = Action::WaitToDie;
            self.drawn = false;
        }
    }

    /// `EnButte_WaitToDie`.
    fn wait_to_die(&mut self) {
        if self.timer <= 0 {
            self.actor.kill();
        }
    }
}

impl ActorImpl for EnButte {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnButte_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if let Some(c) = self.actor.child
            && Some(c) != play.cur_actor
            && play.actors.actor(c).is_none_or(|a| a.killed)
        {
            self.actor.child = None;
        }
        if self.timer > 0 {
            self.timer -= 1;
        }
        self.unk_25c = self.unk_25c.wrapping_add(0x222);
        self.unk_25e = self.unk_25e.wrapping_add(0x1000);
        self.unk_260 = self.unk_260.wrapping_add(0x600);

        if self.fairy() {
            let (_, _, _, _, melee_weapon_state) = Self::player_info(play);
            if melee_weapon_state == 0 {
                if self.sword_down_timer > 0 {
                    self.sword_down_timer -= 1;
                }
            } else {
                self.sword_down_timer = 80;
            }
        }

        match self.action {
            Action::FlyAround => self.fly_around(play),
            Action::FollowLink => self.follow_link(play),
            Action::TransformIntoFairy => self.transform_into_fairy(play),
            Action::WaitToDie => self.wait_to_die(),
        }

        if !self.actor.killed {
            self.actor.move_forward();
            step_to_f(&mut self.actor.world_pos.y, self.pos_y_target, 0.6);
            if self.actor.xyz_dist_to_player_sq < 5000.0 {
                play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
            }
            let y = self.actor.shape_y_offset * self.actor.scale.y;
            self.actor.set_focus(y);
        }
    }
    /// `EnButte_Draw`'s `Collider_UpdateSpheres(0, &collider)` after the skeleton: the sphere at
    /// the actor's model matrix.
    fn draw_update(&mut self, _play: &mut PlayState) {
        if self.drawn && self.draw_skel_anime {
            let m = oot_game::play::actor_draw_matrix(&RenderState::of(&self.actor));
            self.collider.update_spheres(0, &m);
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.switches = vec![self.drawn as u32, self.draw_skel_anime as u32, (self.fairy() && self.action == Action::TransformIntoFairy) as u32];
        rs
    }
    /// `EnButte_Draw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let [drawn, skel, flash] = rs.switches.as_slice() else { return };
        if *drawn == 0 {
            return;
        }
        if *skel != 0
            && let Some(joints) = &rs.joints
        {
            let bones = self.skeleton.pose(joints);
            out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE)), transform: oot_game::play::actor_draw_matrix(rs), bones, params: Default::default() });
        }
        if *flash != 0 {
            // EnButte_DrawTransformationEffect.
            let (scale, alpha_angle) =
                play.overlay_statics.get(&ACTOR_EN_BUTTE).and_then(|s| s.downcast_ref::<Statics>()).map(|s| (s.transformation_effect_scale, s.transformation_effect_alpha)).unwrap_or((0.0, 0));
            let alpha = ((sin_s(alpha_angle) * 250.0) as i32).clamp(0, 255);
            let cam_dir = play.active_camera().cam_dir;
            let rot = Mat4::from_rotation_y(binang_to_rad(cam_dir[1])) * Mat4::from_rotation_x(binang_to_rad(cam_dir[0])) * Mat4::from_rotation_z(binang_to_rad(cam_dir[2]));
            let sp5c = rot.transform_point3(Vec3::new(0.0, 0.0, -3.0));
            let m = Mat4::from_translation(self.actor.focus_pos + sp5c) * rot * Mat4::from_scale(Vec3::splat(scale));
            let mut sv = SegmentValues::default();
            sv.prim[SEG_COLOR as usize] = Some([200, 200, 180, alpha as u8]);
            sv.env[SEG_COLOR as usize] = Some([200, 200, 210, 255]);
            out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(FLASH_BAKE)), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
        }
    }
    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0).then_some(ColliderMut::JntSph(&mut self.collider))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
