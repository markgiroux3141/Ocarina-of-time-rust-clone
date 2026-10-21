//! `En_Boom` (`ovl_En_Boom/z_en_boom.c`): the boomerang in flight, spawned by Player's throw
//! (`func_808359FC`, the upper body's throw on its frame 6) and gone when he catches it
//! (GAME-10 milestone 4, done with Player's boomerang ahead of it).
//!
//! - **Flying** (`EnBoom_Fly`): with an actor to fly to (`moveTo`: Player's focus actor when thrown,
//!   then Player once it turns back) it steers at its focus, the yaw and the pitch each turned by
//!   their difference times `(200 - distance) × 0.005` (at least 0.12) a frame; an actor that isn't
//!   Player and has gone (`update == NULL`) or lies more than a quarter turn off its yaw is
//!   dropped (`moveTo` `NULL`: on it flies straight, `@bug (game)`: the "random" fly-off). 12 a frame
//!   along its pitch (`Actor_SetProjectileSpeed`, `Actor_MoveXZGravity`), its whirr
//!   (`NA_SE_IT_BOOMERANG_FLY`). Its quad hitting an `En_Item00` or a Skulltula token (`En_Si`,
//!   marked `ACTOR_FLAG_HOOKSHOT_ATTACHED`) grabs it.
//! - **`returnTimer`** (20 from the throw) counted down; at 0 it flies back to Player, and within 40
//!   of his focus it's caught: a grabbed drop put at his feet falling (gravity -0.9), a token let go
//!   to touch him, his `PLAYER_STATE1_BOOMERANG_THROWN` cleared, and it's gone. Before that, a hit
//!   (its quad's `AT_HIT`) puts it back where it was; otherwise a wall, floor or ceiling crossed this
//!   frame (`BgCheck_EntityLineTest1`) stops it, but a poly `func_8002F9EC` takes (Jabu-Jabu's
//!   flesh: the blood) or Jabu-Jabu's octo platform (`Bg_Bdan_Objects` params 0) lets it through;
//!   anything else clanks (`CollisionCheck_SpawnShieldParticlesMetal`). Stopped, it turns back
//!   (the pitch negated, the yaw half a turn), Player its target, the timer 0.
//! - What it grabbed rides with it, until it goes (`update == NULL`).
//! - **The update** (`EnBoom_Update`) does nothing while Player is held (`PLAYER_STATE1_29`); else
//!   the action, the focus at its position, `activeTimer` up.
//!
//! The whole overlay is ported. `draw_update` is `EnBoom_Draw`'s part on the actor: through
//! `Actor_Draw`'s matrix turned by its yaw, 0x1F40 about z and its pitch, the quad's edge
//! (`sPosAOffset`, `sPosBOffset`: ±960 along x) given to `Player_UpdateWeaponInfo`, and when that
//! takes it, the trail's vertex (`EffectBlure_AddVertex`: `EFFECT_BLURE1`, yellow 255 to 0 alpha, 8
//! frames). The draw: `gBoomerangRefDL` (`Gfx_SetupDL_25Opa`) spun by `activeTimer × 12000` about
//! y. No shadow (no `ActorShape_Init`).
//!
//! Params: none (0). `moveTo` and `returnTimer` are written by Player after the spawn.
//! Flags: none.

use eng_gfx::{DrawCmd, DrawParams, MeshKey};
use glam::Vec3;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_MISC, ActorHandle, ActorImpl, ActorProfile, func_8002f9ec};
use oot_game::actor_schema::ActorSchema;
use oot_game::audio::sfx::SFX_FLAG;
use oot_game::collision_check::*;
use oot_game::effect::EffectInit;
use oot_game::effect::blure::{EffectBlureInit, EffectBlureInit1};
use oot_game::pack::{BakeBody, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::sys_matrix::{MtxF, binang_to_rad};

/// `ACTOR_EN_BOOM` (`actor_table.h`: 0x0032).
pub const ACTOR_EN_BOOM: i16 = 0x0032;
/// `ACTOR_EN_ITEM00` (0x0015), `ACTOR_EN_SI` (0x019C), `ACTOR_BG_BDAN_OBJECTS` (0x00C8)
/// (`actor_table.h`).
const ACTOR_EN_ITEM00: i16 = 0x0015;
const ACTOR_EN_SI: i16 = 0x019C;
const ACTOR_BG_BDAN_OBJECTS: i16 = 0x00C8;

/// `ACTOR_FLAG_HOOKSHOT_ATTACHED` (`actor.h`: 1 << 13).
const ACTOR_FLAG_HOOKSHOT_ATTACHED: u32 = 1 << 13;

/// `NA_SE_IT_BOOMERANG_FLY` (`itembank_table.h`: 0x1810).
pub const NA_SE_IT_BOOMERANG_FLY: u16 = 0x1810;

/// `PLAYER_STATE1_BOOMERANG_THROWN` (`player.h`: 1 << 25), `PLAYER_STATE1_29`.
const PLAYER_STATE1_BOOMERANG_THROWN: u32 = 1 << 25;
const PLAYER_STATE1_29: u32 = 1 << 29;

/// `En_Boom_Profile`.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_EN_BOOM, name: "En_Boom", category: ACTORCAT_MISC, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED, object: "gameplay_keep", schema: &SCHEMA };

/// En_Boom's configuration (docs/adr/0066-the-actor-schema.md): no params.
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[], flags: &[], todo: false, objects: &[], texts: &[] };

/// `sQuadInit`: Player's attack, the boomerang's damage (0x10, `DMG_BOOMERANG`), 1.
pub const QUAD_INIT: ColliderQuadInit = ColliderQuadInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_ON | AT_TYPE_PLAYER, ac_flags: AC_NONE, oc_flags1: OC1_NONE, oc_flags2: OC2_TYPE_PLAYER, shape: COLSHAPE_QUAD },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK2,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0010, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x01 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_NEAREST | ATELEM_SFX_NORMAL,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_NONE,
    },
    quad: [Vec3::ZERO; 4],
};

/// `EnBoom_Draw`'s statics: the quad's edge, ±960 along the boomerang's model x.
const S_POS_A_OFFSET: Vec3 = Vec3::new(-960.0, 0.0, 0.0);
const S_POS_B_OFFSET: Vec3 = Vec3::new(960.0, 0.0, 0.0);

const BAKE: &str = "EnBoom/boomerang";

/// The boomerang's mesh: `gBoomerangRefDL` from `gameplay_keep` (`Gfx_SetupDL_25Opa`, the bake's
/// start).
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake { name: BAKE.into(), object: "gameplay_keep".into(), segments: Vec::new(), prelude: Vec::new(), body: BakeBody::DLists(vec![("gameplay_keep".into(), "gBoomerangRefDL".into())]) }]
}

pub struct EnBoom {
    pub actor: Actor,
    pub collider: ColliderQuad,
    /// `moveTo`: the actor it flies to (`None`: straight on).
    pub move_to: Option<ActorHandle>,
    /// `grabbed`: the drop or token it carries.
    pub grabbed: Option<ActorHandle>,
    /// `returnTimer`: it turns back to Player at 0.
    pub return_timer: u8,
    /// `activeTimer`: up once each update it runs (its spin).
    pub active_timer: u8,
    pub effect_index: usize,
    pub weapon_info: WeaponInfo,
}

/// `DECR(x)`.
fn decr(t: &mut u8) -> u8 {
    if *t != 0 {
        *t -= 1;
    }
    *t
}

impl EnBoom {
    /// `EnBoom_Init`: no room (`room` -1), no turn (`shape.rot` 0), attention range 5; the trail
    /// (`EFFECT_BLURE1`: from (255, 255, 100) at 255 and 64 to 0, 8 frames, calc mode 0), the quad.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        actor.room = -1;
        // Actor_ProcessInitChain: ICHAIN_S8(attentionRangeType, ATTENTION_RANGE_5), ICHAIN_VEC3S(shape.rot, 0).
        actor.target_mode = 5;
        actor.shape_rot = Rot { x: 0, y: 0, z: 0 };
        let blure = EffectBlureInit1 {
            p1_start_color: [255, 255, 100, 255],
            p2_start_color: [255, 255, 100, 64],
            p1_end_color: [255, 255, 100, 0],
            p2_end_color: [255, 255, 100, 0],
            elem_duration: 8,
            unk_flag: 0,
            calc_mode: 0,
        };
        let effect_index = play.effect_add(EffectInit::Blure(EffectBlureInit::One(blure)));
        Box::new(EnBoom { actor, collider: ColliderQuad::new(&QUAD_INIT), move_to: None, grabbed: None, return_timer: 0, active_timer: 0, effect_index, weapon_info: WeaponInfo::default() })
    }

    /// `Actor_SetProjectileSpeed`: `speed` along the pitch (`world.rot.x`, positive down).
    fn set_projectile_speed(&mut self, speed: f32) {
        self.actor.speed_xz = speed * eng_math::cos_s(self.actor.world_rot.x);
        self.actor.velocity.y = speed * -eng_math::sin_s(self.actor.world_rot.x);
    }

    /// `EnBoom_Fly`.
    fn fly(&mut self, play: &mut PlayState) {
        let player = play.player;
        let (player_focus, player_pos) = player.and_then(|h| play.actors.actor(h)).map(|p| (p.focus_pos, p.world_pos)).unwrap_or((Vec3::ZERO, Vec3::ZERO));
        if let Some(t) = self.move_to {
            let target = play.actors.actor(t).filter(|a| !a.killed).map(|a| a.focus_pos);
            let is_player = Some(t) == player;
            match target {
                Some(focus) => {
                    // Actor_WorldYawTowardPoint, Actor_WorldPitchTowardPoint.
                    let yaw_target = eng_math::vec3f_yaw(self.actor.world_pos, focus);
                    let yaw_diff = self.actor.world_rot.y.wrapping_sub(yaw_target);
                    let pitch_target = eng_math::vec3f_pitch(self.actor.world_pos, focus);
                    let pitch_diff = self.actor.world_rot.x.wrapping_sub(pitch_target);
                    let dist_xyz_scale = ((200.0 - self.actor.world_pos.distance(focus)) * 0.005).max(0.12);
                    if !is_player && (yaw_diff as i32).abs() > 0x4000 {
                        self.move_to = None;
                    } else {
                        let step_y = ((yaw_diff as i32).abs() as f32 * dist_xyz_scale) as i16;
                        let step_x = ((pitch_diff as i32).abs() as f32 * dist_xyz_scale) as i16;
                        eng_math::scaled_step_to_s(&mut self.actor.world_rot.y, yaw_target, step_y);
                        eng_math::scaled_step_to_s(&mut self.actor.world_rot.x, pitch_target, step_x);
                    }
                }
                // The target gone (update == NULL): dropped, unless it's Player.
                None => {
                    if !is_player {
                        self.move_to = None;
                    }
                }
            }
        }
        self.set_projectile_speed(12.0);
        self.actor.move_forward();
        self.actor.play_sfx_flagged(NA_SE_IT_BOOMERANG_FLY.wrapping_sub(SFX_FLAG));

        // A drop or a Skulltula token hit is grabbed.
        if self.collider.base.at_flags & AT_HIT != 0
            && let Some(at) = self.collider.base.at
            && let Some(a) = play.actors.actor_mut(at)
            && (a.id == ACTOR_EN_ITEM00 || a.id == ACTOR_EN_SI)
        {
            self.grabbed = Some(at);
            if a.id == ACTOR_EN_SI {
                a.flags |= ACTOR_FLAG_HOOKSHOT_ATTACHED;
            }
        }

        if decr(&mut self.return_timer) == 0 {
            let dist_from_link = self.actor.world_pos.distance(player_focus);
            self.move_to = player;
            if dist_from_link < 40.0 {
                if let Some(g) = self.grabbed
                    && let Some(a) = play.actors.actor_mut(g)
                {
                    a.world_pos = player_pos;
                    if a.id == ACTOR_EN_ITEM00 {
                        a.gravity = -0.9;
                        a.bg_check_flags &= !(BGCHECKFLAG_GROUND | BGCHECKFLAG_GROUND_TOUCH);
                    } else {
                        a.flags &= !ACTOR_FLAG_HOOKSHOT_ATTACHED;
                    }
                }
                if let Some(p) = player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
                    p.change_state_flags1(0, PLAYER_STATE1_BOOMERANG_THROWN);
                }
                self.actor.kill();
            }
        } else {
            let mut collided = self.collider.base.at_flags & AT_HIT != 0;
            if collided {
                // Back where it was: the bounce starts there.
                self.actor.world_pos = self.actor.prev_pos;
            } else if let Some((hit_point, poly)) = play.col.entity_line_test(self.actor.prev_pos, self.actor.world_pos, true, true, true, true) {
                self.actor.wall_poly = Some(poly);
                collided = true;
                let bdan_platform = poly.bg != eng_collision::bgcheck::BGCHECK_SCENE
                    // schema: another actor's params read whole, as the C does (Bg_Bdan_Objects' type 0).
                    && play.actors.all().into_iter().filter_map(|h| play.actors.get(h)).any(|a| a.dyna_bg_id() == Some(poly.bg) && a.base().id == ACTOR_BG_BDAN_OBJECTS && a.base().params == 0);
                if func_8002f9ec(play, poly, hit_point) || bdan_platform {
                    collided = false;
                } else {
                    play.collision_check_hit_fx(vec![HitFx::ShieldParticlesMetal(hit_point), HitFx::Sfx(oot_game::audio::sfx::NA_SE_IT_SHIELD_REFLECT_SW, oot_game::audio::sfx::SfxPos::Default)]);
                }
            }
            if collided {
                self.actor.world_rot.x = self.actor.world_rot.x.wrapping_neg();
                self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(i16::MIN);
                self.move_to = player;
                self.return_timer = 0;
            }
        }

        // What it carries rides with it, until it's gone.
        if let Some(g) = self.grabbed {
            match play.actors.actor_mut(g).filter(|a| !a.killed) {
                Some(a) => a.world_pos = self.actor.world_pos,
                None => self.grabbed = None,
            }
        }
    }

    /// `EnBoom_Draw`'s matrix: `Actor_Draw`'s (no shape turn, scale 0.01), turned by its yaw, 0x1F40
    /// about z and its pitch.
    fn draw_mtx(&self) -> MtxF {
        let a = &self.actor;
        let mut m = MtxF::set_translate_rotate_yxz(a.world_pos.x, a.world_pos.y + a.shape_y_offset * a.scale.y, a.world_pos.z, [a.shape_rot.x, a.shape_rot.y, a.shape_rot.z]);
        m.scale(a.scale.x, a.scale.y, a.scale.z);
        m.rotate_y(binang_to_rad(a.world_rot.y));
        m.rotate_z(binang_to_rad(0x1F40));
        m.rotate_x(binang_to_rad(a.world_rot.x));
        m
    }
}

impl ActorImpl for EnBoom {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnBoom_Update`: nothing while Player is held (`PLAYER_STATE1_29`).
    fn update(&mut self, play: &mut PlayState) {
        let held = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).is_some_and(|p| p.state_flags1() & PLAYER_STATE1_29 != 0);
        if !held {
            self.fly(play);
            self.actor.set_focus(0.0);
            self.active_timer = self.active_timer.wrapping_add(1);
        }
    }

    /// `EnBoom_Destroy`: the trail deleted.
    fn destroy(&mut self, play: &mut PlayState) {
        play.effect_delete(self.effect_index);
    }

    /// `EnBoom_Draw`'s part on the actor: the quad's edge to `Player_UpdateWeaponInfo` and the
    /// trail's vertex.
    fn draw_update(&mut self, play: &mut PlayState) {
        if self.actor.killed {
            return;
        }
        let m = self.draw_mtx();
        let pos_a = m.mult_vec3f(S_POS_A_OFFSET);
        let pos_b = m.mult_vec3f(S_POS_B_OFFSET);
        if player_update_weapon_info(play, &self.actor, Some((0, &mut self.collider)), &mut self.weapon_info, pos_a, pos_b)
            && let Some(b) = play.effect_blure_mut(self.effect_index)
        {
            b.add_vertex(pos_a, pos_b);
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.switches = vec![self.active_timer as u32];
        rs
    }

    /// `EnBoom_Draw`: `gBoomerangRefDL`, spun by `activeTimer × 12000` about y.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let timer = rs.switches.first().copied().unwrap_or(0);
        let mut m = self.draw_mtx();
        // BINANG_TO_RAD of the product, not wrapped to 16 bits.
        m.rotate_y(((timer as i32 * 12000) as f32 as f64 * (std::f64::consts::PI / 32768.0)) as f32);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE)), transform: m.to_mat4(), bones: Vec::new(), params: DrawParams::default() });
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0).then_some(ColliderMut::Quad(&mut self.collider))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
