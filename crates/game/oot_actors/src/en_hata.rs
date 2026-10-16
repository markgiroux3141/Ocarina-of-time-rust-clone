//! `En_Hata` (`ovl_En_Hata/z_en_hata.c`): a wooden flagpole with a red cloth, on Death Mountain
//! Trail, Gerudo Valley, the Gerudo's Fortress and Lake Hylia. A DynaPoly actor (`gFlagpoleCol`,
//! the pole Link bumps into) at scale 1/75 with a skeleton (`gFlagpoleSkel`) looping
//! `gFlagpoleFlapAnim`.
//!
//! The scene's wind moves it (`envCtx.windDirection`, `envCtx.windSpeed`: the room's wind
//! command). Each frame both cloths hang down (their base limbs turned -0x4000 on y), and their
//! hoist ends lift towards the wind's pitch: from 15000 at no wind to the pitch itself at full
//! (255), with gusts (`unk_278` stepping 3000 or 6000 a frame at random, its sine × 80 off the
//! full speed), smoothly (`Math_SmoothStepToS`, a sixth, 1000 at most), and turn to face along it
//! (minus its yaw). The flapping's speed is 2.75 to 4 times the wind's speed out of 255. The
//! speed is clamped to 0 to 255 in the environment itself.
//!
//! The whole overlay is ported. Its cylinder (`sCylinderInit`) and `sColChkInfoInit` are unused
//! data in the C (the struct's `collider` is never set up): not ported.
//!
//! Params: none (every placement 0xFFFF; read by nothing).
//! Flags: none.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::smooth_step_to_s;
use glam::{Mat4, Vec3};
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::gbi::setup_dl;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::{ANIMMODE_LOOP, SkelAnimeStd};

/// `ACTOR_EN_HATA` (`actor_table.h`: 0x0026).
pub const ACTOR_EN_HATA: i16 = 0x0026;
/// `OBJECT_HATA` (`object_table.h`: 0x0076).
pub const OBJECT: &str = "object_hata";
pub const SKELETON: &str = "gFlagpoleSkel";
pub const ANIMATION: &str = "gFlagpoleFlapAnim";
pub const COLLISION: &str = "gFlagpoleCol";

/// `En_Hata_Profile`: `ACTORCAT_PROP`, `FLAGS` 0, `OBJECT_HATA`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_HATA, name: "En_Hata", category: ACTORCAT_PROP, flags: 0, object: OBJECT, schema: &SCHEMA };

/// `params`: never read (every placement 0xFFFF).
pub const PARAMS: Field = Field::whole("params", Kind::Unused, "0xFFFF in every placement; read by nothing");

/// En_Hata's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[PARAMS], flags: &[], todo: false };

/// `FlagpoleLimb` (`object_hata.h`, the skeleton's limb order): the cloths' bases and hoist ends.
pub const FLAGPOLE_LIMB_FLAG_1_BASE: usize = 3;
pub const FLAGPOLE_LIMB_FLAG_1_HOIST_END_BASE: usize = 4;
pub const FLAGPOLE_LIMB_FLAG_2_BASE: usize = 12;
pub const FLAGPOLE_LIMB_FLAG_2_HOIST_END_BASE: usize = 13;
pub const FLAGPOLE_LIMB_MAX: usize = 21;

/// The limbs `EnHata_OverrideLimbDraw` turns by `limbs`.
const TURNED_LIMBS: [usize; 4] = [FLAGPOLE_LIMB_FLAG_2_BASE, FLAGPOLE_LIMB_FLAG_1_BASE, FLAGPOLE_LIMB_FLAG_2_HOIST_END_BASE, FLAGPOLE_LIMB_FLAG_1_HOIST_END_BASE];

pub struct EnHata {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    pub skel: Option<SkelAnimeStd>,
    skeleton: Option<Arc<Skeleton>>,
    /// `limbs`: what the draw adds to each limb's rotation.
    pub limbs: [[i16; 3]; FLAGPOLE_LIMB_MAX],
    /// `invScale`, `maxStep`, `minStep`: the hoist ends' smooth step.
    pub inv_scale: i16,
    pub max_step: i16,
    pub min_step: i16,
    /// `unk_278`: the gusts' phase.
    pub unk_278: i16,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

/// `Math_Vec3f_Pitch(a, b)`: `Math_Atan2S(Math_Vec3f_DistXZ(a, b), a.y - b.y)`.
// shared: Math_Vec3f_Pitch (z_lib.c), private in several modules.
fn vec3f_pitch(a: Vec3, b: Vec3) -> i16 {
    let dist_xz = ((b.x - a.x) * (b.x - a.x) + (b.z - a.z) * (b.z - a.z)).sqrt();
    eng_math::atan2_s(dist_xz, a.y - b.y)
}

impl EnHata {
    /// `EnHata_Init`: scale 1/75; the skeleton looping the flapping from its first frame to its
    /// last at speed 1 (`SkelAnime_Init`, `Animation_Change`); `gFlagpoleCol`; the culling volume
    /// (500, 550 down, 2200 away); the step 1/6, at most 1000, at least 1; the gusts' phase at
    /// random.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = EnHata { actor, bg: BG_ACTOR_MAX, skel: None, skeleton: None, limbs: [[0; 3]; FLAGPOLE_LIMB_MAX], inv_scale: 0, max_step: 0, min_step: 0, unk_278: 0 };
        let loaded = play.assets.clone().map(|a| -> anyhow::Result<_> { Ok((a.skeleton(OBJECT, SKELETON)?, a.animation(OBJECT, ANIMATION)?)) });
        this.actor.scale = Vec3::splat(1.0 / 75.0);
        match loaded {
            Some(Ok((skeleton, anim))) => {
                let frame_count = anim.last_frame();
                // SkelAnime_Init(.., &gFlagpoleSkel, &gFlagpoleFlapAnim, NULL, NULL, 0):
                // Animation_PlayLoop.
                let mut sk = SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(anim.clone()));
                sk.change(anim, 1.0, 0.0, frame_count, ANIMMODE_LOOP, 0.0);
                this.skel = Some(sk);
                this.skeleton = Some(skeleton);
            }
            Some(Err(e)) => log::error!("En_Hata: {e:#}"),
            None => {}
        }
        // DynaPolyActor_Init(&this->dyna, 0), CollisionHeader_GetVirtual(&gFlagpoleCol).
        if let Some(h) = crate::obj_kibako2::load_collision(play, OBJECT, COLLISION) {
            this.bg = play.col.dyna.set_bg_actor(h, source(&this.actor), 0);
        }
        this.actor.culling_volume_scale = 500.0;
        this.actor.culling_volume_downward = 550.0;
        this.actor.culling_volume_distance = 2200.0;
        this.inv_scale = 6;
        this.max_step = 1000;
        this.min_step = 1;
        this.unk_278 = (play.rand.zero_one() * 65535.0) as i32 as i16;
        Box::new(this)
    }

    /// The pose `EnHata_OverrideLimbDraw` draws: `limbs` added to the four turned limbs'
    /// rotations.
    pub fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], limbs: &[[i16; 3]; FLAGPOLE_LIMB_MAX]) -> Vec<Mat4> {
        skeleton.pose_override(joints, |limb, _pos, rot| {
            if TURNED_LIMBS.contains(&limb) {
                let l = limbs[limb];
                rot[0] = rot[0].wrapping_add(l[0]);
                rot[1] = rot[1].wrapping_add(l[1]);
                rot[2] = rot[2].wrapping_add(l[2]);
            }
            Mat4::IDENTITY
        })
    }
}

/// The bake: `gFlagpoleSkel` after `Gfx_SetupDL_37Opa` (each limb's list sets its own modes).
const BAKE: &str = "En_Hata/flagpole";
const SEG_SETUP: u8 = 0x08;

pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: BAKE.into(),
        object: OBJECT.into(),
        segments: vec![(SEG_SETUP, BakeSegment::Commands(setup_dl::setup_dl_37().0))],
        prelude: vec![SEG_SETUP],
        body: BakeBody::Skeleton { file: OBJECT.into(), symbol: SKELETON.into(), limbs: Vec::new() },
    }]
}

impl ActorImpl for EnHata {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnHata_Update`.
    fn update(&mut self, play: &mut PlayState) {
        let zero_vec = Vec3::ZERO;
        if let Some(sk) = self.skel.as_mut() {
            sk.update();
        }
        // Rotate to hang down by default.
        self.limbs[FLAGPOLE_LIMB_FLAG_2_BASE][1] = -0x4000;
        self.limbs[FLAGPOLE_LIMB_FLAG_1_BASE][1] = -0x4000;
        let d = play.env_ctx.wind_direction;
        let wind_vec = Vec3::new(d[0] as f32, d[1] as f32, d[2] as f32);
        if play.env_ctx.wind_speed > 255.0 {
            play.env_ctx.wind_speed = 255.0;
        }
        if play.env_ctx.wind_speed < 0.0 {
            play.env_ctx.wind_speed = 0.0;
        }
        if play.rand.zero_one() > 0.5 {
            self.unk_278 = self.unk_278.wrapping_add(6000);
        } else {
            self.unk_278 = self.unk_278.wrapping_add(3000);
        }
        // Mimic varying wind gusts.
        let sin = eng_math::sin_s(self.unk_278) * 80.0;
        let mut pitch: i32 = -(vec3f_pitch(zero_vec, wind_vec) as i32);
        let wind_speed = play.env_ctx.wind_speed;
        pitch = (((15000 - pitch) as f32 * (1.0 - (wind_speed / (255.0 - sin)))) as i32) + pitch;
        let l = FLAGPOLE_LIMB_FLAG_1_HOIST_END_BASE;
        smooth_step_to_s(&mut self.limbs[l][1], pitch as i16, self.inv_scale, self.max_step, self.min_step);
        self.limbs[FLAGPOLE_LIMB_FLAG_2_HOIST_END_BASE][1] = self.limbs[l][1];
        self.limbs[l][2] = eng_math::vec3f_yaw(zero_vec, wind_vec).wrapping_neg();
        self.limbs[FLAGPOLE_LIMB_FLAG_2_HOIST_END_BASE][2] = self.limbs[l][2];
        let r = play.rand.zero_float(1.25);
        if let Some(sk) = self.skel.as_mut() {
            sk.play_speed = (r + 2.75) * (play.env_ctx.wind_speed / 255.0);
        }
    }

    /// `EnHata_Destroy`: `SkelAnime_Free`, `DynaPoly_DeleteBgActor`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }

    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        if let Some(s) = &self.skel {
            rs.joints = Some(eng_anim::anim::JointTable { rot: s.joint_table.clone(), face: 0 });
        }
        rs.angles = self.limbs.iter().flatten().copied().collect();
        rs
    }

    /// `EnHata_Draw`: `Gfx_SetupDL_37Opa`, the model 1.1 times taller (`Matrix_Scale(1, 1.1, 1)`),
    /// the skeleton with `EnHata_OverrideLimbDraw`'s turns (`EnHata_PostLimbDraw` does nothing).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(skeleton), Some(joints)) = (&self.skeleton, &rs.joints) else { return };
        let mut limbs = [[0i16; 3]; FLAGPOLE_LIMB_MAX];
        for (i, v) in rs.angles.chunks_exact(3).take(FLAGPOLE_LIMB_MAX).enumerate() {
            limbs[i] = [v[0], v[1], v[2]];
        }
        let bones = Self::pose(skeleton, &joints.rot, &limbs);
        let m = oot_game::play::actor_draw_matrix(rs) * Mat4::from_scale(Vec3::new(1.0, 1.1, 1.0));
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE)), transform: m, bones, params: Default::default() });
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pitch_of_a_level_wind_is_level() {
        // Math_Atan2S(xz distance, 0): 0x4000 (straight along x of the atan2's (y, x) = (0, d)).
        assert_eq!(vec3f_pitch(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)), eng_math::atan2_s(10.0, 0.0));
    }
}
