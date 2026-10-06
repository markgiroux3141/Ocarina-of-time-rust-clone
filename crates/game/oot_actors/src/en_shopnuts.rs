//! `En_Shopnuts` (`ovl_En_Shopnuts/z_en_shopnuts.c`): the Business Scrub's attack phase, a
//! Deku Scrub in the ground that spits nuts (`En_Nutsball` 2) at Link like a Mad Scrub; hit by
//! anything (its own nut bounced back), it pops out and becomes the salesman (`En_Dns`) with its
//! params, the item he sells.
//!
//! Params: `SHOPNUTS_GET_TYPE`, an `EnDnsType` (the Deku Tree's one in room 3 is 4, the Deku
//! Shield). One selling the piece of heart or a capacity upgrade already bought isn't there.
//!
//! - **Idle** in the ground (`EnShopnuts_Idle`): its first animation held at speed 0 for 100 to
//!   150 frames, sped up to 1 with Link between 160 and 480 across and within 120 up or down;
//!   up, its collider grows from 5 (frame 9 on: `AC_ON`). Link within 120, it **burrows**
//!   (`EnShopnuts_Burrow`). Past 320 with its timer out, it **looks around** (two loops); else it
//!   **peeks** (`EnShopnuts_Peek`), turning to Link for a loop, then **spits**
//!   (`EnShopnuts_ThrowNut`: the nut at frame 6, 23 in front and 12 up, its nose swelling), peeks
//!   two loops and burrows.
//! - **Hit** (`EnShopnuts_ColliderCheck`, or the hammer's shock wave): `NA_SE_EN_NUTS_DAMAGE`,
//!   spinning up out of the ground turned to Link (`EnShopnuts_SpawnSalesman`), then gone, an
//!   `En_Dns` in its place.
//!
//! The whole overlay is ported. It never moves: its gravity is set but there's no
//! `Actor_MoveXZGravity`, and its collider stays where its init put it. Not ported: the circle
//! shadow (`ActorShadow_DrawCircle`, 35), not ported for any actor.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{approach_s, cos_s, sin_s};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::audio::sfx::*;
use oot_game::collision_check::*;
use oot_game::pack::{BakeBody, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_ONCE, Anim, SkelAnimeStd};

use crate::en_dekunuts::spit_nose_scale;
use crate::en_dns::{DNS_TYPE_DEKU_NUT_UPGRADE, DNS_TYPE_DEKU_STICK_UPGRADE, DNS_TYPE_HEART_PIECE, INFTABLE_HAS_DEKU_NUT_UPGRADE, INFTABLE_HAS_DEKU_STICK_UPGRADE, ITEMGETINF_DEKU_HEART_PIECE};

/// `ACTOR_EN_SHOPNUTS` (`actor_table.h`).
pub const ACTOR_EN_SHOPNUTS: i16 = 0x0195;
pub(crate) const OBJECT: &str = "object_shopnuts";

/// `En_Shopnuts_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_SHOPNUTS, name: "En_Shopnuts", category: ACTORCAT_ENEMY, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE, object: OBJECT };

/// `NAVI_ENEMY_BUSINESS_SCRUB` (`actor.h`).
pub(crate) const NAVI_ENEMY_BUSINESS_SCRUB: u8 = 0x4E;

/// `BUSINESS_SCRUB_LIMB_NOSE` (`object_shopnuts.xml`'s limbs, from 1).
const BUSINESS_SCRUB_LIMB_NOSE: usize = 9;

/// The skeleton without its nose (`EnShopnuts_OverrideLimbDraw`'s `*dList = NULL` while it spits).
const BAKE_NO_NOSE: &str = "En_Shopnuts/no_nose";

/// `sCylinderInit`: hit by everything but the shield and the mirror's ray; 20 by 40.
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HIT6, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 20, height: 40, y_shift: 0, pos: [0, 0, 0] },
};

/// `sColChkInfoInit`: 1 health, 20 by 40, heavy.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit = CollisionCheckInfoInit { health: 1, cyl_radius: 20, cyl_height: 40, mass: MASS_HEAVY };

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Idle,
    LookAround,
    Peek,
    ThrowNut,
    Burrow,
    SpawnSalesman,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::Idle => "EnShopnuts_Idle",
            Action::LookAround => "EnShopnuts_LookAround",
            Action::Peek => "EnShopnuts_Peek",
            Action::ThrowNut => "EnShopnuts_ThrowNut",
            Action::Burrow => "EnShopnuts_Burrow",
            Action::SpawnSalesman => "EnShopnuts_SpawnSalesman",
        }
    }
}

/// The animations of `object_shopnuts` it plays.
struct Anims {
    peek: Anim,
    initial: Anim,
    look_around: Anim,
    throw_nut: Anim,
    peek_burrow: Anim,
    rotate: Anim,
}

impl Anims {
    fn load(play: &PlayState) -> Option<Anims> {
        let a = play.assets.clone()?;
        let get = |s: &str| a.animation(OBJECT, s).map_err(|e| log::error!("En_Shopnuts: {e:#}")).ok();
        Some(Anims {
            peek: get("gBusinessScrubPeekAnim")?,
            initial: get("gBusinessScrubInitialAnim")?,
            look_around: get("gBusinessScrubLookAroundAnim")?,
            throw_nut: get("gBusinessScrubThrowNutAnim")?,
            peek_burrow: get("gBusinessScrubPeekBurrowAnim")?,
            rotate: get("gBusinessScrubRotateAnim")?,
        })
    }
}

/// The bake: `gBusinessScrubSkel` with the nose's list gone.
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: BAKE_NO_NOSE.into(),
        object: OBJECT.into(),
        segments: Vec::new(),
        prelude: Vec::new(),
        body: BakeBody::Skeleton {
            file: OBJECT.into(),
            symbol: "gBusinessScrubSkel".into(),
            limbs: vec![LimbOverride { limb: (BUSINESS_SCRUB_LIMB_NOSE - 1) as u8, file: OBJECT.into(), symbol: String::new() }],
        },
    }]
}

pub struct EnShopnuts {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: Option<Anims>,
    pub action: Action,
    /// `animFlagAndTimer`: "0x1000 bit denotes that projectile has been thrown".
    pub anim_flag_and_timer: i16,
    pub collider: ColliderCylinder,
}

impl EnShopnuts {
    /// `EnShopnuts_Init`: its skeleton, collider and info; gone if what it sells (the piece of
    /// heart, an upgrade) is already bought; else idle.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: NAVI_ENEMY_BUSINESS_SCRUB, gravity -1, lockOnArrowOffset 2600.
        actor.navi_enemy_id = NAVI_ENEMY_BUSINESS_SCRUB;
        actor.gravity = -1.0;
        actor.target_arrow_offset = 2600.0;
        // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 35): the circle shadow isn't ported.
        actor.shape_y_offset = 0.0;
        let skeleton = play.assets.clone().and_then(|a| a.skeleton(OBJECT, "gBusinessScrubSkel").map_err(|e| log::error!("En_Shopnuts: {e:#}")).ok());
        let anims = Anims::load(play);
        // SkelAnime_InitFlex(&gBusinessScrubSkel, &gBusinessScrubPeekAnim): BUSINESS_SCRUB_LIMB_MAX.
        let limbs = skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(17);
        let skel = SkelAnimeStd::init_flex(limbs, anims.as_ref().map(|a| a.peek.clone()));
        let mut collider = ColliderCylinder::new(&CYLINDER_INIT);
        actor.col_chk_info.set_info(None, &COL_CHK_INFO_INIT);
        collider.update(&actor);
        let mut n = EnShopnuts { actor, skel, skeleton, anims, action: Action::Idle, anim_flag_and_timer: 0, collider };
        let ty = n.actor.params;
        let s = &play.save;
        if (ty == DNS_TYPE_HEART_PIECE && s.get_item_get_inf(ITEMGETINF_DEKU_HEART_PIECE))
            || (ty == DNS_TYPE_DEKU_STICK_UPGRADE && s.get_inf_table(INFTABLE_HAS_DEKU_STICK_UPGRADE))
            || (ty == DNS_TYPE_DEKU_NUT_UPGRADE && s.get_inf_table(INFTABLE_HAS_DEKU_NUT_UPGRADE))
        {
            n.actor.kill();
        } else {
            n.setup_idle(play);
        }
        Box::new(n)
    }

    fn anim(&self, f: impl Fn(&Anims) -> &Anim) -> Option<Anim> {
        self.anims.as_ref().map(|a| f(a).clone())
    }

    fn change(&mut self, a: Option<Anim>, speed: f32, mode: u8, morph: f32, to_last: bool) {
        if let Some(a) = a {
            let end = if to_last { a.last_frame() } else { 0.0 };
            self.skel.change(a, speed, 0.0, end, mode, morph);
        }
    }

    /// `Animation_GetLastFrame(&gBusinessScrubPeekBurrowAnim)`.
    fn peek_burrow_last_frame(&self) -> f32 {
        self.anims.as_ref().map(|a| a.peek_burrow.last_frame()).unwrap_or(0.0)
    }

    /// `EnShopnuts_SetupIdle`: in the ground, its first animation held at speed 0 for 100 to
    /// 150 frames, the collider 5 high and off.
    fn setup_idle(&mut self, play: &mut PlayState) {
        self.change(self.anim(|a| &a.initial), 0.0, ANIMMODE_ONCE, 0.0, true);
        self.anim_flag_and_timer = play.rand.s16_offset(100, 50);
        self.collider.dim.height = 5;
        self.collider.base.ac_flags &= !AC_ON;
        self.action = Action::Idle;
    }

    /// `EnShopnuts_SetupLookAround`: two loops.
    fn setup_look_around(&mut self) {
        self.change(self.anim(|a| &a.look_around), 1.0, ANIMMODE_LOOP, 0.0, true);
        self.anim_flag_and_timer = 2;
        self.action = Action::LookAround;
    }

    /// `EnShopnuts_SetupThrowNut`.
    fn setup_throw_nut(&mut self) {
        self.change(self.anim(|a| &a.throw_nut), 1.0, ANIMMODE_ONCE, 0.0, true);
        self.action = Action::ThrowNut;
    }

    /// `EnShopnuts_SetupPeek`: after a spit two loops and the flag, else one.
    fn setup_peek(&mut self) {
        self.change(self.anim(|a| &a.peek), 1.0, ANIMMODE_LOOP, -3.0, false);
        self.anim_flag_and_timer = if self.action == Action::ThrowNut { 2 | 0x1000 } else { 1 };
        self.action = Action::Peek;
    }

    /// `EnShopnuts_SetupBurrow`.
    fn setup_burrow(&mut self, play: &mut PlayState) {
        self.change(self.anim(|a| &a.peek_burrow), 1.0, ANIMMODE_ONCE, -5.0, true);
        audio_play_actor_sfx2(play, NA_SE_EN_NUTS_DOWN);
        self.action = Action::Burrow;
    }

    /// `EnShopnuts_SetupSpawnSalesman`.
    fn setup_spawn_salesman(&mut self, play: &mut PlayState) {
        self.change(self.anim(|a| &a.rotate), 1.0, ANIMMODE_ONCE, -3.0, true);
        audio_play_actor_sfx2(play, NA_SE_EN_NUTS_DAMAGE);
        self.collider.base.ac_flags &= !AC_ON;
        self.action = Action::SpawnSalesman;
    }

    /// `EnShopnuts_Idle`: as `EnDekunuts_Wait`, its collider growing over frames 9 to 13.
    fn idle(&mut self, play: &mut PlayState) {
        let has_slow_playback_speed = self.skel.play_speed < 0.5;
        if has_slow_playback_speed && self.anim_flag_and_timer != 0 {
            self.anim_flag_and_timer -= 1;
        }
        if self.skel.on_frame(9.0) {
            self.collider.base.ac_flags |= AC_ON;
        } else if self.skel.on_frame(8.0) {
            audio_play_actor_sfx2(play, NA_SE_EN_NUTS_UP);
        }
        self.collider.dim.height = (((self.skel.cur_frame.clamp(9.0, 13.0) - 9.0) * 9.0) + 5.0) as i16;
        if !has_slow_playback_speed && self.actor.xz_dist_to_player < 120.0 {
            self.setup_burrow(play);
        } else if self.skel.update() {
            if self.actor.xz_dist_to_player < 120.0 {
                self.setup_burrow(play);
            } else if self.anim_flag_and_timer == 0 && self.actor.xz_dist_to_player > 320.0 {
                self.setup_look_around();
            } else {
                self.setup_peek();
            }
        }
        if has_slow_playback_speed && (self.actor.xz_dist_to_player > 160.0 && self.actor.y_dist_to_player.abs() < 120.0) && (self.anim_flag_and_timer == 0 || self.actor.xz_dist_to_player < 480.0) {
            self.skel.play_speed = 1.0;
        }
    }

    /// `EnShopnuts_LookAround`.
    fn look_around(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.skel.on_frame(0.0) && self.anim_flag_and_timer != 0 {
            self.anim_flag_and_timer -= 1;
        }
        if self.actor.xz_dist_to_player < 120.0 || self.anim_flag_and_timer == 0 {
            self.setup_burrow(play);
        }
    }

    /// `EnShopnuts_Peek`: turning to Link (unless just after a spit); Link within 120 or after
    /// a spit's two loops, down; its loop done, a spit.
    fn peek(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.skel.on_frame(0.0) && self.anim_flag_and_timer != 0 {
            self.anim_flag_and_timer -= 1;
        }
        if self.anim_flag_and_timer & 0x1000 == 0 {
            approach_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 2, 0xE38);
        }
        if self.actor.xz_dist_to_player < 120.0 || self.anim_flag_and_timer == 0x1000 {
            self.setup_burrow(play);
        } else if self.anim_flag_and_timer == 0 {
            self.setup_throw_nut();
        }
    }

    /// `EnShopnuts_ThrowNut`: turning to Link; Link within 120, down; the nut at frame 6
    /// (`ACTOR_EN_NUTSBALL` `EN_NUTSBALL_TYPE_SHOPNUTS`, `NA_SE_EN_NUTS_THROW` if it spawned).
    fn throw_nut(&mut self, play: &mut PlayState) {
        approach_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 2, 0xE38);
        if self.actor.xz_dist_to_player < 120.0 {
            self.setup_burrow(play);
        } else if self.skel.update() {
            self.setup_peek();
        } else if self.skel.on_frame(6.0) {
            let r = self.actor.shape_rot;
            let spawn_pos = Vec3::new(self.actor.world_pos.x + (sin_s(r.y) * 23.0), self.actor.world_pos.y + 12.0, self.actor.world_pos.z + (cos_s(r.y) * 23.0));
            if play.actor_spawn(crate::en_nutsball::ACTOR_EN_NUTSBALL, spawn_pos, [r.x, r.y, r.z], crate::en_nutsball::EN_NUTSBALL_TYPE_SHOPNUTS).is_ok() {
                audio_play_actor_sfx2(play, NA_SE_EN_NUTS_THROW);
            }
        }
    }

    /// `EnShopnuts_Burrow`: the collider shrinking from 45 to 5 over frames 0 to 4 (off at 4);
    /// done, idle.
    fn burrow(&mut self, play: &mut PlayState) {
        if self.skel.update() {
            self.setup_idle(play);
        } else {
            self.collider.dim.height = (((4.0 - self.skel.cur_frame.min(4.0)) * 10.0) + 5.0) as i16;
        }
        if self.skel.on_frame(4.0) {
            self.collider.base.ac_flags &= !AC_ON;
        }
    }

    /// `EnShopnuts_SpawnSalesman`: turning to Link while it spins up; then the salesman
    /// (`ACTOR_EN_DNS` with its type) in its place, and it's gone.
    fn spawn_salesman(&mut self, play: &mut PlayState) {
        if self.skel.update() {
            let r = self.actor.shape_rot;
            let pos = self.actor.world_pos;
            if let Err(e) = play.actor_spawn(crate::en_dns::ACTOR_EN_DNS, pos, [r.x, r.y, r.z], self.actor.params) {
                log::debug!("En_Shopnuts: the salesman: {e:?}");
            }
            self.actor.kill();
        } else {
            approach_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 2, 0xE38);
        }
    }

    /// `EnShopnuts_ColliderCheck`: hit (its drop flag set), or the hammer's shock wave: up it
    /// comes.
    fn collider_check(&mut self, play: &mut PlayState) {
        if self.collider.base.ac_flags & AC_HIT != 0 {
            self.collider.base.ac_flags &= !AC_HIT;
            let elem = self.collider.info;
            self.actor.set_drop_flag(&elem, true);
            self.setup_spawn_salesman(play);
        } else if play.actors.unk_02 != 0 {
            self.setup_spawn_salesman(play);
        }
    }
}

impl ActorImpl for EnShopnuts {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnShopnuts_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.collider_check(play);
        match self.action {
            Action::Idle => self.idle(play),
            Action::LookAround => self.look_around(play),
            Action::Peek => self.peek(play),
            Action::ThrowNut => self.throw_nut(play),
            Action::Burrow => self.burrow(play),
            Action::SpawnSalesman => self.spawn_salesman(play),
        }
        let (r, h) = (self.collider.dim.radius as f32, self.collider.dim.height as f32);
        self.actor.update_bg_check_info(&play.col, 20.0, r, h, UPDBGCHECKINFO_FLAG_2);
        if self.collider.base.ac_flags & AC_ON != 0 {
            play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
        }
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        if self.action == Action::Idle {
            self.actor.set_focus(self.skel.cur_frame);
        } else if self.action == Action::Burrow {
            self.actor.set_focus(20.0 - ((self.skel.cur_frame * 20.0) / self.peek_burrow_last_frame()));
        } else {
            self.actor.set_focus(20.0);
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.switches = vec![(self.action == Action::ThrowNut) as u32];
        rs.values = vec![self.skel.cur_frame];
        rs
    }

    /// `EnShopnuts_Draw`: the skeleton (`SkelAnime_DrawFlexOpa`); while it spits, without its
    /// nose (`EnShopnuts_OverrideLimbDraw`), which is drawn swelling at the nose's matrix
    /// (`EnShopnuts_PostLimbDraw`: `gBusinessScrubNoseDL`, along on y, across on x and z; as it
    /// is past frame 10).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(joints), Some(skeleton), [throwing], [cur_frame]) = (&rs.joints, &self.skeleton, rs.switches.as_slice(), rs.values.as_slice()) else { return };
        let bones = skeleton.pose(joints);
        let model = oot_game::play::actor_draw_matrix(rs);
        if *throwing != 0 {
            let (across, along) = spit_nose_scale(*cur_frame).unwrap_or((1.0, 1.0));
            let nose = model * bones[BUSINESS_SCRUB_LIMB_NOSE - 1] * Mat4::from_scale(Vec3::new(across, along, across));
            out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE_NO_NOSE)), transform: model, bones, params: Default::default() });
            out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh(OBJECT, "gBusinessScrubNoseDL")), nose));
        } else {
            out.opa.push(DrawCmd { mesh: MeshKey::named(keys::mesh(OBJECT, "gBusinessScrubSkel")), transform: model, bones, params: Default::default() });
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
