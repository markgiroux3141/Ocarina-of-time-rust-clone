//! `Arms_Hook` (`ovl_Arms_Hook/z_arms_hook.c`): the hookshot's (and the longshot's) hook and chain.
//!
//! Player spawns it as his child when the hookshot is put in hand (`Player_InitHookshotIA`) and
//! holds it (`heldActor`); his draw sets it at the hookshot's tip each frame. The shot
//! (`func_808350A4`) lets go of it (its `parent` cleared): it flies out at 20 for 13 frames (26 for
//! the longshot) along the hand's aim, drawing its chain from the hand (`unk_3C8`):
//! - into an actor that takes the hookshot (`ACTOR_FLAG_HOOKSHOT_PULLS_ACTOR` or
//!   `_PULLS_PLAYER`) through a hookable element (`ACELEM_HOOKABLE`): attached; one that pulls
//!   Player (`_PULLS_PLAYER`) starts the pull, else it's dragged back with the hook;
//! - into a wall that takes it (`SurfaceType_CanHookshot`): attached (to a moving wall's actor
//!   too), and Player is pulled (`ArmsHook_PullPlayer`: his `parent` the hook, which
//!   `Player_UpdateUpperBody` turns into `Player_Action_HookshotFly`);
//! - into another wall: the metal spark and the reflect sound; back;
//! - a button pressed, or its time out: back.
//!
//! Back, it comes to the hand at 200 a frame (50 dragging an actor; Player flies at 30 a frame
//! to it) and is held again (`ArmsHook_AttachToPlayer`); a pull ends with Player's velocity set to
//! the last step, 20 down. While it flies it's Player's attack (the quad, `Player_UpdateWeaponInfo`
//! at draw time: damage 1, `DMG_HOOKSHOT`).
//!
//! Params: none (spawned with 0). Flags: none of the save's. It sets and clears
//! `ACTOR_FLAG_HOOKSHOT_ATTACHED` on what it holds.

use eng_gfx::{DrawCmd, DrawParams, MeshKey};
use glam::Vec3;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorHandle, ActorImpl, ActorProfile, func_8002f9ec};
use oot_game::actor_schema::ActorSchema;
use oot_game::audio::sfx::{SFX_FLAG, SfxPos};
use oot_game::camera::f_atan2f;
use oot_game::collision_check::*;
use oot_game::pack::{BakeBody, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::surface::SurfaceType;
use oot_game::sys_matrix::MtxF;

use crate::player::Player;

/// `ACTOR_ARMS_HOOK` (`actor_table.h`: 0x0066).
pub const ACTOR_ARMS_HOOK: i16 = 0x0066;
/// `ACTOR_BG_SPOT06_OBJECTS` (`actor_table.h`: 0x00D5): Lake Hylia's water temple lid, which
/// moves the hook with it.
const ACTOR_BG_SPOT06_OBJECTS: i16 = 0x00D5;

/// `ACTOR_FLAG_HOOKSHOT_PULLS_PLAYER` (`actor.h`: 1 << 10), `ACTOR_FLAG_HOOKSHOT_ATTACHED` (1 << 13).
pub const ACTOR_FLAG_HOOKSHOT_PULLS_PLAYER: u32 = 1 << 10;
pub const ACTOR_FLAG_HOOKSHOT_ATTACHED: u32 = 1 << 13;

/// `PLAYER_STATE1_DEAD` (`player.h`: 1 << 7), `PLAYER_STATE1_26` (1 << 26).
const PLAYER_STATE1_DEAD: u32 = 1 << 7;
const PLAYER_STATE1_26: u32 = 1 << 26;

// `itembank_table.h`.
/// `NA_SE_IT_HOOKSHOT_CHAIN` (0x180B).
pub const NA_SE_IT_HOOKSHOT_CHAIN: u16 = 0x180B;
/// `NA_SE_IT_HOOKSHOT_REFLECT` (0x1813).
pub const NA_SE_IT_HOOKSHOT_REFLECT: u16 = 0x1813;
/// `NA_SE_IT_ARROW_STICK_CRE` (0x1814).
pub const NA_SE_IT_ARROW_STICK_CRE: u16 = 0x1814;
/// `NA_SE_IT_HOOKSHOT_STICK_OBJ` (0x1829).
pub const NA_SE_IT_HOOKSHOT_STICK_OBJ: u16 = 0x1829;

/// `Arms_Hook_Profile`: `ACTORCAT_ITEMACTION`, `OBJECT_LINK_BOY`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_ARMS_HOOK,
    name: "Arms_Hook",
    category: ACTORCAT_ITEMACTION,
    flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED,
    object: "object_link_boy",
    schema: &SCHEMA,
};

/// Arms_Hook's configuration (docs/adr/0066-the-actor-schema.md): no params, no flags.
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[], flags: &[], todo: false, objects: &[], texts: &[] };

/// `sQuadInit`: Player's attack, the hookshot's damage (0x80, `DMG_HOOKSHOT`), 1.
pub const QUAD_INIT: ColliderQuadInit = ColliderQuadInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_ON | AT_TYPE_PLAYER, ac_flags: AC_NONE, oc_flags1: OC1_NONE, oc_flags2: OC2_TYPE_PLAYER, shape: COLSHAPE_QUAD },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK2,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0080, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x01 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_NEAREST | ATELEM_SFX_NORMAL,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_NONE,
    },
    quad: [Vec3::ZERO; 4],
};

// The draw's points, in the hook's model space.
/// `D_80865B70`: the hook's point while held or coming back.
const D_80865B70: Vec3 = Vec3::new(0.0, 0.0, 0.0);
/// `D_80865B7C`: its point while it flies out, 900 ahead.
const D_80865B7C: Vec3 = Vec3::new(0.0, 0.0, 900.0);
/// `D_80865B88`, `D_80865B94`: the quad's edge while held (behind it).
const D_80865B88: Vec3 = Vec3::new(0.0, 500.0, -3000.0);
const D_80865B94: Vec3 = Vec3::new(0.0, -500.0, -3000.0);
/// `D_80865BA0`, `D_80865BAC`: the quad's edge while it flies out.
const D_80865BA0: Vec3 = Vec3::new(0.0, 500.0, 1200.0);
const D_80865BAC: Vec3 = Vec3::new(0.0, -500.0, 1200.0);

const TIP_BAKE: &str = "ArmsHook/tip";
const CHAIN_BAKE: &str = "ArmsHook/chain";

/// The hook's meshes (docs/adr/0012-actor-bakes.md): `gLinkAdultHookshotTipDL` and
/// `gLinkAdultHookshotChainDL` from `object_link_boy`.
pub fn bakes() -> Vec<MeshBake> {
    [(TIP_BAKE, "gLinkAdultHookshotTipDL"), (CHAIN_BAKE, "gLinkAdultHookshotChainDL")]
        .into_iter()
        .map(|(name, dl)| MeshBake { name: name.into(), object: "object_link_boy".into(), segments: Vec::new(), prelude: Vec::new(), body: BakeBody::DLists(vec![("object_link_boy".into(), dl.into())]) })
        .collect()
}

/// `ArmsHookActionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `ArmsHook_Wait`: on the hookshot, until Player lets go of it.
    Wait,
    /// `ArmsHook_Shoot`: out, and back.
    Shoot,
}

/// `ArmsHook`.
pub struct ArmsHook {
    pub actor: Actor,
    pub collider: ColliderQuad,
    pub weapon_info: WeaponInfo,
    /// `unk_1E8`: the hook's point (900 ahead while it flies out), set at draw time and moved by
    /// the update; the wall test runs to it.
    pub unk_1e8: Vec3,
    /// `unk_1F4`: `unk_1E8` as the last update left it.
    pub unk_1f4: Vec3,
    /// `attachedActor`.
    pub attached_actor: Option<ActorHandle>,
    /// `attachPointOffset`: from the hook to the attached actor's position.
    pub attach_point_offset: Vec3,
    /// `timer`: the frames it still flies out.
    pub timer: i16,
    pub action: Action,
    /// What the draw last saw: Player drawn holding the hookshot (`rightHandType ==
    /// PLAYER_MODELTYPE_RH_HOOKSHOT`), and the hand's point the chain runs to (`unk_3C8`).
    pub drawn: bool,
    pub chain_to: Vec3,
}

/// `DECR(x)`.
fn decr(t: &mut i16) -> i16 {
    if *t != 0 {
        *t -= 1;
    }
    *t
}

fn player_mut(play: &mut PlayState) -> Option<&mut Player> {
    play.player.and_then(|h| play.actors.downcast_mut::<Player>(h))
}

impl ArmsHook {
    /// `ArmsHook_Init`: the quad; waiting; `unk_1E8` where it is.
    pub fn init(actor: Actor, _play: &mut PlayState) -> Box<dyn ActorImpl> {
        let unk_1e8 = actor.world_pos;
        Box::new(ArmsHook {
            actor,
            collider: ColliderQuad::new(&QUAD_INIT),
            weapon_info: WeaponInfo::default(),
            unk_1e8,
            unk_1f4: Vec3::ZERO,
            attached_actor: None,
            attach_point_offset: Vec3::ZERO,
            timer: 0,
            action: Action::Wait,
            drawn: false,
            chain_to: Vec3::ZERO,
        })
    }

    /// `Actor_SetProjectileSpeed`: `speed` along the pitch (`world.rot.x`, positive down).
    fn set_projectile_speed(&mut self, speed: f32) {
        self.actor.speed_xz = speed * eng_math::cos_s(self.actor.world_rot.x);
        self.actor.velocity.y = speed * -eng_math::sin_s(self.actor.world_rot.x);
    }

    /// `ArmsHook_Wait`: once Player lets go (`parent` cleared by the shot), out: 13 frames at 20
    /// (26 for the longshot), Player its parent again.
    fn wait(&mut self, play: &mut PlayState) {
        if self.actor.parent.is_none() {
            let Some(p) = player_mut(play) else { return };
            // The timer's length for the hookshot or the longshot.
            let length = if p.held_item_ap == p.ia_hookshot() { 13 } else { 26 };
            self.action = Action::Shoot;
            self.set_projectile_speed(20.0);
            self.actor.parent = play.player;
            self.timer = length;
        }
    }

    /// `ArmsHook_PullPlayer`: Player flies to it (his `parent` the hook, its `child` Player).
    fn pull_player(&mut self, play: &mut PlayState) {
        self.actor.child = self.actor.parent;
        let me = play.cur_actor;
        if let Some(p) = player_mut(play) {
            p.actor.parent = me;
        }
    }

    /// `ArmsHook_AttachToPlayer`: held again (Player's `child` and `heldActor`); true if it was
    /// pulling him (his `parent` cleared).
    fn attach_to_player(&mut self, play: &mut PlayState) -> bool {
        let me = play.cur_actor;
        let pulling = self.actor.child.is_some();
        let Some(p) = player_mut(play) else { return false };
        p.actor.child = me;
        p.held_actor = me;
        if pulling {
            p.actor.parent = None;
            self.actor.child = None;
            return true;
        }
        false
    }

    /// `ArmsHook_DetachFromActor`.
    fn detach_from_actor(&mut self, play: &mut PlayState) {
        if let Some(h) = self.attached_actor.take()
            && let Some(a) = play.actors.actor_mut(h)
        {
            a.flags &= !ACTOR_FLAG_HOOKSHOT_ATTACHED;
        }
    }

    /// `ArmsHook_CheckForCancel`: with the hookshot in hand, another item in use, a talk, or Player
    /// dead or in `PLAYER_STATE1_26`: no more flying out, let go, the hook at the hand.
    fn check_for_cancel(&mut self, play: &mut PlayState) -> bool {
        let Some(p) = play.player.and_then(|h| play.actors.downcast::<Player>(h)) else { return false };
        if p.holds_hookshot_ap()
            && (p.item_ap != p.held_item_ap || p.actor.flags & ACTOR_FLAG_TALK != 0 || p.state1 & (PLAYER_STATE1_DEAD | PLAYER_STATE1_26) != 0)
        {
            let hand = p.unk_3c8;
            self.timer = 0;
            self.detach_from_actor(play);
            self.actor.world_pos = hand;
            return true;
        }
        false
    }

    /// `ArmsHook_AttachToActor`: `ACTOR_FLAG_HOOKSHOT_ATTACHED` on it, the offset from the hook kept.
    fn attach_to_actor(&mut self, play: &mut PlayState, h: ActorHandle) {
        if let Some(a) = play.actors.actor_mut(h) {
            a.flags |= ACTOR_FLAG_HOOKSHOT_ATTACHED;
            self.attach_point_offset = a.world_pos - self.actor.world_pos;
        }
        self.attached_actor = Some(h);
    }

    /// The hit element's `acElemFlags` (`this->collider.elem.atHitElem->acElemFlags`).
    fn at_hit_ac_elem_flags(play: &mut PlayState, hit: &HitElem) -> u8 {
        let r = hit.elem;
        let i = r.elem as usize;
        match play.actors.get_mut(r.col.actor).and_then(|a| a.collider_mut(r.col.id)) {
            Some(ColliderMut::JntSph(c)) => c.elements.get(i).map(|e| e.info.ac_elem_flags),
            Some(ColliderMut::Tris(c)) => c.elements.get(i).map(|e| e.info.ac_elem_flags),
            Some(ColliderMut::Cylinder(c)) => (i == 0).then_some(c.info.ac_elem_flags),
            Some(ColliderMut::Quad(c)) => (i == 0).then_some(c.info.ac_elem_flags),
            None => None,
        }
        .unwrap_or(0)
    }

    /// `ArmsHook_Shoot`.
    fn shoot(&mut self, play: &mut PlayState) {
        let holds = player_mut(play).is_some_and(|p| p.holds_hookshot_ap());
        if self.actor.parent.is_none() || !holds {
            self.detach_from_actor(play);
            self.actor.kill();
            return;
        }
        if let Some(p) = player_mut(play) {
            p.actor.play_sfx_flagged2(NA_SE_IT_HOOKSHOT_CHAIN - SFX_FLAG);
        }
        self.check_for_cancel(play);

        if self.timer != 0
            && self.collider.base.at_flags & AT_HIT != 0
            && let Some(hit) = self.collider.info.at_hit_elem
            && hit.elem_material != ELEM_MATERIAL_UNK4
        {
            if let Some(touched) = self.collider.base.at
                && play.actors.actor(touched).is_some_and(|a| !a.killed && a.flags & (ACTOR_FLAG_HOOKSHOT_PULLS_ACTOR | ACTOR_FLAG_HOOKSHOT_PULLS_PLAYER) != 0)
                && Self::at_hit_ac_elem_flags(play, &hit) & ACELEM_HOOKABLE != 0
            {
                self.attach_to_actor(play, touched);
                if play.actors.actor(touched).is_some_and(|a| a.flags & ACTOR_FLAG_HOOKSHOT_PULLS_PLAYER == ACTOR_FLAG_HOOKSHOT_PULLS_PLAYER) {
                    self.pull_player(play);
                }
            }
            self.timer = 0;
            if let Some(me) = play.cur_actor {
                play.audio.play_sfx_at_pos(SfxPos::Actor(me), NA_SE_IT_ARROW_STICK_CRE);
            }
            return;
        }

        if decr(&mut self.timer) == 0 {
            let mut attached = self.attached_actor;
            if let Some(h) = attached {
                let a = play.actors.actor(h).filter(|a| !a.killed && a.flags & ACTOR_FLAG_HOOKSHOT_ATTACHED == ACTOR_FLAG_HOOKSHOT_ATTACHED).map(|a| a.world_pos);
                match a {
                    None => {
                        attached = None;
                        self.attached_actor = None;
                    }
                    Some(apos) if self.actor.child.is_some() => {
                        let cur_actor_offset_xyz = self.actor.world_pos.distance(apos);
                        let attach_point_offset_xyz = self.attach_point_offset.length();
                        // Keep the hook at the attachment's offset even if the actor moves.
                        self.actor.world_pos = apos - self.attach_point_offset;
                        // Moved more than 50 from where it was caught: let go.
                        if cur_actor_offset_xyz - attach_point_offset_xyz > 50.0 {
                            self.detach_from_actor(play);
                            attached = None;
                        }
                    }
                    Some(_) => {}
                }
            }

            let hand = player_mut(play).map(|p| p.unk_3c8).unwrap_or_default();
            let body_dist_diff_vec = self.actor.world_pos - hand;
            let body_dist_diff = body_dist_diff_vec.length();
            let (mut velocity, mut phi_f16);
            if body_dist_diff < 30.0 {
                velocity = 0.0;
                phi_f16 = 0.0;
            } else {
                velocity = if self.actor.child.is_some() {
                    30.0
                } else if attached.is_some() {
                    50.0
                } else {
                    200.0
                };
                phi_f16 = body_dist_diff - velocity;
                if body_dist_diff <= velocity {
                    phi_f16 = 0.0;
                }
                velocity = phi_f16 / body_dist_diff;
            }
            let new_pos = body_dist_diff_vec * velocity;

            if self.actor.child.is_none() {
                // Not pulling Player.
                let spot06 = attached.and_then(|h| play.actors.actor(h)).filter(|a| a.id == ACTOR_BG_SPOT06_OBJECTS).map(|a| a.world_pos);
                if let Some(apos) = spot06 {
                    self.actor.world_pos = apos - self.attach_point_offset;
                    phi_f16 = 1.0;
                } else {
                    self.actor.world_pos = hand + new_pos;
                    if let Some(a) = attached.and_then(|h| play.actors.actor_mut(h)) {
                        a.world_pos = self.actor.world_pos + self.attach_point_offset;
                    }
                }
            } else if let Some(p) = player_mut(play) {
                // Pulling Player.
                p.actor.velocity = body_dist_diff_vec - new_pos;
                let v = body_dist_diff_vec;
                p.actor.world_rot.x = eng_math::atan2_s((v.x * v.x + v.z * v.z).sqrt(), -v.y);
            }

            if phi_f16 < 50.0 {
                self.detach_from_actor(play);
                if phi_f16 == 0.0 {
                    self.action = Action::Wait;
                    if self.attach_to_player(play) {
                        let pos = self.actor.world_pos;
                        if let Some(p) = player_mut(play) {
                            p.actor.velocity = pos - p.actor.world_pos;
                            p.actor.velocity.y -= 20.0;
                        }
                    }
                }
            }
        } else {
            self.actor.move_forward();
            let prev_frame_diff = self.actor.world_pos - self.actor.prev_pos;
            self.unk_1e8 += prev_frame_diff;
            self.actor.shape_rot.x = eng_math::atan2_s(self.actor.speed_xz, -self.actor.velocity.y);
            let sp60 = self.unk_1f4 - (self.unk_1e8 - self.unk_1f4);
            let hit = play.col.entity_line_test(sp60, self.unk_1e8, true, true, true, true).filter(|&(point, poly)| !func_8002f9ec(play, poly, point));
            if let Some((intersect_pos, poly)) = hit {
                let n = play.col.poly_normal(poly);
                self.actor.world_pos = intersect_pos;
                self.actor.world_pos.x += 10.0 * n.x;
                self.actor.world_pos.z += 10.0 * n.z;
                self.timer = 0;
                if play.col.can_hookshot(poly) {
                    if poly.bg != eng_collision::bgcheck::BGCHECK_SCENE {
                        // DynaPoly_GetActor.
                        let owner = play.actors.all().into_iter().find(|&h| play.actors.get(h).is_some_and(|a| a.dyna_bg_id() == Some(poly.bg)));
                        if let Some(h) = owner {
                            self.attach_to_actor(play, h);
                        }
                    }
                    self.pull_player(play);
                    if let Some(me) = play.cur_actor {
                        play.audio.play_sfx_at_pos(SfxPos::Actor(me), NA_SE_IT_HOOKSHOT_STICK_OBJ);
                    }
                } else {
                    play.collision_check_hit_fx(vec![HitFx::ShieldParticlesMetal(self.actor.world_pos)]);
                    if let Some(me) = play.cur_actor {
                        play.audio.play_sfx_at_pos(SfxPos::Actor(me), NA_SE_IT_HOOKSHOT_REFLECT);
                    }
                }
            } else {
                use eng_input::pad::*;
                if play.input.press.button & (BTN_A | BTN_B | BTN_R | BTN_CUP | BTN_CDOWN | BTN_CLEFT | BTN_CRIGHT) != 0 {
                    self.timer = 0;
                }
            }
        }
    }

    /// `Actor_Draw`'s matrix: the hook's position, shape rotation and scale.
    fn draw_mtx(a: &Actor) -> MtxF {
        let mut m = MtxF::set_translate_rotate_yxz(a.world_pos.x, a.world_pos.y + a.shape_y_offset * a.scale.y, a.world_pos.z, [a.shape_rot.x, a.shape_rot.y, a.shape_rot.z]);
        m.scale(a.scale.x, a.scale.y, a.scale.z);
        m
    }
}

impl ActorImpl for ArmsHook {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `ArmsHook_Update`: the action, then `unk_1F4` from `unk_1E8`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Wait => self.wait(play),
            Action::Shoot => self.shoot(play),
        }
        self.unk_1f4 = self.unk_1e8;
    }

    /// `ArmsHook_Destroy`: what it held let go.
    fn destroy(&mut self, play: &mut PlayState) {
        if let Some(h) = self.attached_actor
            && let Some(a) = play.actors.actor_mut(h)
        {
            a.flags &= !ACTOR_FLAG_HOOKSHOT_ATTACHED;
        }
    }

    /// `ArmsHook_Draw`'s part on the game: drawn only with Player drawn holding the hookshot. Its
    /// point (`unk_1E8`) and the quad's edge from its matrix: out ahead while it flies, behind it
    /// otherwise (the weapon off); `Player_UpdateWeaponInfo`. The chain's end, Player's hand.
    fn draw_update(&mut self, play: &mut PlayState) {
        let (drawn, hand) = match play.player.and_then(|h| play.actors.downcast::<Player>(h)) {
            Some(p) => (p.has_draw() && p.right_hand_is_hookshot(play), p.unk_3c8),
            None => (false, Vec3::ZERO),
        };
        self.drawn = drawn && !self.actor.killed;
        self.chain_to = hand;
        if !self.drawn {
            return;
        }
        let m = Self::draw_mtx(&self.actor);
        let (pos_a, pos_b);
        if self.action != Action::Shoot || self.timer <= 0 {
            self.unk_1e8 = m.mult_vec3f(D_80865B70);
            pos_a = m.mult_vec3f(D_80865B88);
            pos_b = m.mult_vec3f(D_80865B94);
            self.weapon_info.active = false;
        } else {
            self.unk_1e8 = m.mult_vec3f(D_80865B7C);
            pos_a = m.mult_vec3f(D_80865BA0);
            pos_b = m.mult_vec3f(D_80865BAC);
        }
        player_update_weapon_info(play, &self.actor, Some((0, &mut self.collider)), &mut self.weapon_info, pos_a, pos_b);
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.switches = vec![self.drawn as u32];
        rs.values = vec![self.chain_to.x, self.chain_to.y, self.chain_to.z];
        rs
    }

    /// `ArmsHook_Draw`: the tip on the hook's matrix; the chain from the hook to the hand, turned
    /// to it and stretched along z (0.015 wide, its length × 0.01).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if rs.switches.first().copied().unwrap_or(0) == 0 {
            return;
        }
        let mut m = MtxF::set_translate_rotate_yxz(rs.pos.x, rs.pos.y + rs.y_offset * rs.scale.y, rs.pos.z, rs.rot);
        m.scale(rs.scale.x, rs.scale.y, rs.scale.z);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(TIP_BAKE)), transform: m.to_mat4(), bones: Vec::new(), params: DrawParams::default() });
        let hand = Vec3::new(rs.values[0], rs.values[1], rs.values[2]);
        let sp78 = hand - rs.pos;
        let sp58 = sp78.x * sp78.x + sp78.z * sp78.z;
        let sp5c = sp58.sqrt();
        let mut c = MtxF::set_translate(rs.pos.x, rs.pos.y, rs.pos.z);
        c.rotate_y(f_atan2f(sp78.x, sp78.z));
        c.rotate_x(f_atan2f(-sp78.y, sp5c));
        c.scale(0.015, 0.015, (sp78.y * sp78.y + sp58).sqrt() * 0.01);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(CHAIN_BAKE)), transform: c.to_mat4(), bones: Vec::new(), params: DrawParams::default() });
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
