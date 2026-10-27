//! `En_Honotrap` (`ovl_En_Honotrap/z_en_honotrap.c`): the fake eye switches that shoot fire, and
//! fire. Three types by `params` (`EnHonotrapType`):
//!
//! - **`HONOTRAP_TYPE_EYE`** (0): a silver eye switch on a wall (`gEyeSwitch2DL`, scale 0.1),
//!   placed twice in the Shadow Temple's room 0 (every layer of `HAKAdan_scene`). Its two
//!   triangles (`sTrisInit`: 23 either side, 8.5 out, turned with it) are hurt by Link's weapons
//!   while it isn't shut: hit, it bursts (`EffectSsBomb2_SpawnLayered`) and is gone. Shut, it waits
//!   (`EnHonotrap_EyeIdle`) for Link within 750 across, 0 to 700 below it and in front of it
//!   (within 0x4000 of its facing); then it opens (`EnHonotrap_EyeOpen`: red for 40 frames,
//!   `NA_SE_EV_RED_EYE`, the eye opening a step a frame) and shoots a flame
//!   (`HONOTRAP_TYPE_FLAME_MOVE`, its child, 12 out and 10 down from it), stares 30 frames from its
//!   opening (`EnHonotrap_EyeAttack`), shuts (`EnHonotrap_EyeClose`) and waits 200 frames more
//!   after its flame is gone before it looks again.
//! - **`HONOTRAP_TYPE_FLAME_MOVE`** (1): the eye's flame. It grows (`EnHonotrap_FlameGrow`, 0.0006 a
//!   frame to 0.004) and flies (`EnHonotrap_FlameMove`, up to 13 a frame, speeding up 0.5 a frame)
//!   straight at where Link was as it was made (10 up), for 160 frames at most; there it chases
//!   him (`EnHonotrap_FlameChase`: turning 0x300 a frame, up to 3 a frame, rising or sinking to his
//!   height) for 100 frames, or until it meets a wall. Off Link's shield it goes back the way the
//!   shield faces (`Math3D_Vec3fReflect` on the move, the shield's yaw on the chase) and fades;
//!   hitting Link it stops and fades (`EnHonotrap_FlameVanish`: shrinking 0.00015 a frame).
//! - **`HONOTRAP_TYPE_FLAME_DROP`** (2): the flames Dampé's ghost (`En_Po_Relay`) drops in his race.
//!   No room's (`room` -1), a smaller cylinder (12 by 30). It grows to 0.0048, then is thrown
//!   (`EnHonotrap_FlameDrop`: 2 a frame the way it faces while it rises) and falls, faster each
//!   frame, to 1 above the floor; after 40 frames, or hitting Link (`Actor_SetPlayerKnockbackLarge
//!   NoDamage` unless his shield took it), it fades.
//!
//! A flame hurts Link (`sCylinderInit`: 4 damage, fire) and bobs (`bobPhase`, 0x640 a frame: 1000
//! either way about 600 up), burning (`NA_SE_EV_BURN_OUT`) every frame; lit with
//! `NA_SE_EV_FLAME_IGNITION`. Drawn as `EnHonotrap_DrawFlame` does: `gEffFire1DL` turned to the
//! camera under a scroll (`flameScroll`, 20 a drawn frame), yellow into red; its circle shadow (30,
//! alpha 128) is the shape's. The eye as `EnHonotrap_DrawEye`: its texture by `eyeState`
//! (`sSilverEyeTextures`). Bakes (docs/adr/0012-actor-bakes.md): the eye with each texture, the
//! flame.
//!
//! The whole overlay is ported. The C's collider is a union (`EnHonotrapCollider`): the eye's
//! triangles or a flame's cylinder; `EnHonotrap_FlameMove` reads `collider.tris.base.atFlags`, the
//! same bytes as the cylinder's base: here `cyl.base`.
//!
//! Params: `params` the whole params (`EnHonotrapType`: 0 `HONOTRAP_TYPE_EYE`, the placed ones; 1
//! `HONOTRAP_TYPE_FLAME_MOVE`, an eye's; 2 `HONOTRAP_TYPE_FLAME_DROP`, `En_Po_Relay`'s; any other a
//! flame drawn as none).
//! Flags: none.

use eng_collision::math3d::{Cylinder16, vec3f_dist_xyz, vec3f_magnitude};
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{atan2_s, binang_to_rad, cos_s, scaled_step_to_s, sin_s, step_to_f};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile, actor_set_player_knockback_large_no_damage, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::audio::sfx::{NA_SE_EV_FLAME_IGNITION, SFX_FLAG, SfxPos};
use oot_game::collision_check::*;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::scene_table::gfx_two_tex_scroll;

use crate::en_bubble::math3d_vec3f_reflect;

/// `ACTOR_EN_HONOTRAP` (`actor_table.h`: 0x011C).
pub const ACTOR_EN_HONOTRAP: i16 = 0x011C;
/// `OBJECT_GAMEPLAY_DANGEON_KEEP` (`object_table.h`: 0x0003).
pub const OBJECT: &str = "gameplay_dangeon_keep";

/// `En_Honotrap_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_HONOTRAP, name: "En_Honotrap", category: ACTORCAT_PROP, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `this->actor.params` (`EnHonotrapType`).
pub const TYPE: Field = Field::whole(
    "params",
    Kind::Named(&[(0, "HONOTRAP_TYPE_EYE: an eye that shoots fire"), (1, "HONOTRAP_TYPE_FLAME_MOVE: an eye's flame"), (2, "HONOTRAP_TYPE_FLAME_DROP: Dampé's flame (En_Po_Relay's)")]),
    "an eye switch that shoots fire, or a flame",
);

/// En_Honotrap's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[TYPE], flags: &[], todo: false, objects: &[], texts: &[] };

/// `EnHonotrapType`.
pub const HONOTRAP_TYPE_EYE: i16 = 0;
pub const HONOTRAP_TYPE_FLAME_MOVE: i16 = 1;
pub const HONOTRAP_TYPE_FLAME_DROP: i16 = 2;

/// `EnHonotrapEyeState`.
pub const HONOTRAP_EYE_OPEN: i16 = 0;
pub const HONOTRAP_EYE_HALF: i16 = 1;
pub const HONOTRAP_EYE_CLOSE: i16 = 2;
pub const HONOTRAP_EYE_SHUT: i16 = 3;
pub const HONOTRAP_EYE_MAX: i16 = 4;

/// `colChkFlags`.
pub const HONOTRAP_AT_ACTIVE: u8 = 1 << 0;
pub const HONOTRAP_AC_ACTIVE: u8 = 1 << 1;
pub const HONOTRAP_OC_ACTIVE: u8 = 1 << 2;

/// `NA_SE_EV_BURN_OUT` (`environmentbank_table.h`: 0x285B).
pub const NA_SE_EV_BURN_OUT: u16 = 0x285B;
/// `NA_SE_EV_RED_EYE` (`environmentbank_table.h`: 0x2881).
pub const NA_SE_EV_RED_EYE: u16 = 0x2881;

/// `sTrisElementsInit`: the eye's two halves.
pub const TRIS_ELEMENTS: [ColliderTrisElementInit; 2] = [
    ColliderTrisElementInit {
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK4,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0001_F824, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
            at_elem_flags: ATELEM_NONE,
            ac_elem_flags: ACELEM_ON,
            oc_elem_flags: OCELEM_NONE,
        },
        vtx: [Vec3::new(0.0, 23.0, 8.5), Vec3::new(-23.0, 0.0, 8.5), Vec3::new(0.0, -23.0, 8.5)],
    },
    ColliderTrisElementInit {
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0001_F824, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
            at_elem_flags: ATELEM_NONE,
            ac_elem_flags: ACELEM_ON,
            oc_elem_flags: OCELEM_NONE,
        },
        vtx: [Vec3::new(0.0, 23.0, 8.5), Vec3::new(0.0, -23.0, 8.5), Vec3::new(23.0, 0.0, 8.5)],
    },
];

/// `sTrisInit`.
pub const TRIS_INIT: ColliderInit = ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_NONE, shape: COLSHAPE_TRIS };

/// `sCylinderInit`: a flame, hurting Link for 4 with fire.
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_FIRE, damage: 0x04 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0010_0000, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 10, height: 25, y_shift: 0, pos: [0, 0, 0] },
};

/// `sColChkInfoInit`: 9 by 23, mass 1.
pub const COL_CHK_INFO_INIT: CollisionCheckInfoInit = CollisionCheckInfoInit { health: 0, cyl_radius: 9, cyl_height: 23, mass: 1 };

/// `EnHonotrap_Update`'s `sVelocity`, `sAccel`: the eye's burst.
const BURST_VELOCITY: Vec3 = Vec3::new(0.0, 0.0, 0.0);
const BURST_ACCEL: Vec3 = Vec3::new(0.0, 0.1, 0.0);

/// `sSilverEyeTextures`, by `eyeState`.
pub const SILVER_EYE_TEXTURES: [&str; HONOTRAP_EYE_MAX as usize] = ["gEyeSwitchSilverOpenTex", "gEyeSwitchSilverHalfTex", "gEyeSwitchSilverClosedTex", "gEyeSwitchSilverClosedTex"];

/// The eye's texture (segment 8) and the flame's scroll (segment 8) and colours.
const SEG_TEX: u8 = 0x08;
const SEG_SCROLL: u8 = 0x08;
const SEG_FLAME_COLORS: u8 = 0x0C;

/// The eye drawn with `tex`.
pub fn eye_bake(tex: &str) -> String {
    format!("En_Honotrap/eye/{tex}")
}
/// `gEffFire1DL` with its scroll and colours.
pub const FLAME_BAKE: &str = "En_Honotrap/flame";

/// The flame's scroll: `Gfx_TwoTexScroll(.., G_TX_RENDERTILE, 0, 0, 32, 64, 1, 0, flameScroll, 32,
/// 128)`.
pub fn flame_scroll(flame_scroll: i16) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, 0, 0, 32, 64, 1, 0, flame_scroll as i32 as u32, 32, 128)
}

/// The flame's colours: `gDPSetPrimColor(0x80, 0x80, 255, 200, 0, 255)`, `gDPSetEnvColor(255, 0, 0,
/// 0)`.
fn flame_colors() -> Vec<(u32, u32)> {
    vec![(0xFA00_8080, u32::from_be_bytes([255, 200, 0, 255])), (0xFB00_0000, u32::from_be_bytes([255, 0, 0, 0])), (0xDF00_0000, 0)]
}

/// The eye's list with each of its textures (`Gfx_SetupDL_25Opa`), and the flame
/// (`Gfx_SetupDL_25Xlu`, its colours, `gEffFire1DL` under the scroll).
pub fn bakes() -> Vec<MeshBake> {
    let mut v = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    for tex in SILVER_EYE_TEXTURES {
        if seen.contains(&tex) {
            continue;
        }
        seen.push(tex);
        v.push(MeshBake {
            name: eye_bake(tex),
            object: OBJECT.into(),
            segments: vec![(SEG_TEX, BakeSegment::Texture { file: OBJECT.into(), symbol: tex.into() })],
            prelude: vec![],
            body: BakeBody::DLists(vec![(OBJECT.into(), "gEyeSwitch2DL".into())]),
        });
    }
    v.push(MeshBake {
        name: FLAME_BAKE.into(),
        object: "gameplay_keep".into(),
        segments: vec![(SEG_FLAME_COLORS, BakeSegment::Commands(flame_colors())), (SEG_SCROLL, BakeSegment::Dynamic(flame_scroll(0)))],
        prelude: vec![SEG_FLAME_COLORS],
        body: BakeBody::DLists(vec![("gameplay_keep".into(), "gEffFire1DL".into())]),
    });
    v
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    EyeIdle,
    EyeOpen,
    EyeAttack,
    EyeClose,
    FlameGrow,
    FlameDrop,
    FlameMove,
    FlameChase,
    FlameVanish,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::EyeIdle => "EnHonotrap_EyeIdle",
            Action::EyeOpen => "EnHonotrap_EyeOpen",
            Action::EyeAttack => "EnHonotrap_EyeAttack",
            Action::EyeClose => "EnHonotrap_EyeClose",
            Action::FlameGrow => "EnHonotrap_FlameGrow",
            Action::FlameDrop => "EnHonotrap_FlameDrop",
            Action::FlameMove => "EnHonotrap_FlameMove",
            Action::FlameChase => "EnHonotrap_FlameChase",
            Action::FlameVanish => "EnHonotrap_FlameVanish",
        }
    }
}

pub struct EnHonotrap {
    pub actor: Actor,
    pub action: Action,
    /// `collider.tris`: the eye's.
    pub tris: ColliderTris,
    /// `collider.cyl`: a flame's.
    pub cyl: ColliderCylinder,
    pub timer: i16,
    pub eye_state: i16,
    pub target_pos: Vec3,
    pub speed_mod: f32,
    pub bob_phase: i16,
    pub flame_scroll: i16,
    pub col_chk_flags: u8,
}

/// `EnHonotrap_GetNormal`: `vec` made 1 long; (0, 0, 1) for one shorter than 0.001.
pub fn get_normal(vec: Vec3) -> Vec3 {
    let magnitude = vec3f_magnitude(vec);
    if magnitude < 0.001 {
        log::debug!("Warning : vector size zero (../z_en_honotrap.c 328)");
        Vec3::new(0.0, 0.0, 1.0)
    } else {
        Vec3::new(vec.x * (1.0 / magnitude), vec.y * (1.0 / magnitude), vec.z * (1.0 / magnitude))
    }
}

impl EnHonotrap {
    /// `EnHonotrap_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: cullingVolumeDistance 1000, cullingVolumeScale 500, cullingVolumeDownward 1000.
        actor.culling_volume_distance = 1000.0;
        actor.culling_volume_scale = 500.0;
        actor.culling_volume_downward = 1000.0;
        let mut this = EnHonotrap {
            actor,
            action: Action::EyeIdle,
            tris: ColliderTris::default(),
            cyl: ColliderCylinder::default(),
            timer: 0,
            eye_state: 0,
            target_pos: Vec3::ZERO,
            speed_mod: 0.0,
            bob_phase: 0,
            flame_scroll: 0,
            col_chk_flags: 0,
        };
        if TYPE.get(this.actor.params) == HONOTRAP_TYPE_EYE {
            this.init_eye();
        } else {
            this.init_flame(play);
        }
        Box::new(this)
    }

    /// `EnHonotrap_InitEye`: the two triangles turned with it, about it.
    fn init_eye(&mut self) {
        self.actor.scale = Vec3::splat(0.1);
        let sin = sin_s(self.actor.home_rot.y);
        let cos = cos_s(self.actor.home_rot.y);
        self.tris = ColliderTris::new(&TRIS_INIT, &TRIS_ELEMENTS);
        for (i, e) in TRIS_ELEMENTS.iter().enumerate() {
            let mut triangle = [Vec3::ZERO; 3];
            for (vtx, base) in triangle.iter_mut().zip(e.vtx.iter()) {
                vtx.x = base.z * sin + base.x * cos;
                vtx.y = base.y;
                vtx.z = base.z * cos - base.x * sin;
                *vtx += self.actor.world_pos;
            }
            self.tris.set_vertices(i, triangle[0], triangle[1], triangle[2]);
        }
        self.setup_eye_idle();
        self.actor.set_focus(0.0);
    }

    /// `EnHonotrap_InitFlame`: tiny, aimed at Link (10 up), lit.
    fn init_flame(&mut self, play: &mut PlayState) {
        self.actor.scale = Vec3::splat(0.0001);
        self.cyl = ColliderCylinder::new(&CYLINDER_INIT);
        self.cyl.update(&self.actor);
        self.actor.min_velocity_y = -1.0;
        self.actor.col_chk_info.set_info(None, &COL_CHK_INFO_INIT);
        self.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 30.0);
        self.actor.shape_shadow_alpha = 128;
        self.target_pos = play.player.and_then(|h| play.actors.actor(h)).map(|p| p.world_pos).unwrap_or_default();
        self.target_pos.y += 10.0;
        self.flame_scroll = (play.rand.zero_one() * 511.0) as i32 as i16;
        self.setup_flame_grow();
        // Actor_PlaySfx at its projectedPos, all zero at init (Actor_Spawn zeroes the actor):
        // the middle of the screen.
        play.audio.play_sfx_at_pos(SfxPos::Default, NA_SE_EV_FLAME_IGNITION);
        if TYPE.get(self.actor.params) == HONOTRAP_TYPE_FLAME_DROP {
            self.actor.room = -1;
            self.cyl.dim.radius = 12;
            self.cyl.dim.height = 30;
            self.actor.shape_y_offset = -1000.0;
        }
    }

    /// `EnHonotrap_FlameCollisionCheck`.
    fn flame_collision_check(&mut self, play: &mut PlayState) {
        self.cyl.update(&self.actor);
        play.collision_check_set_at(&self.actor, 1, &mut self.cyl);
        play.collision_check_set_ac(&self.actor, 1, &mut self.cyl);
        play.collision_check_set_oc(&self.actor, 1, &mut self.cyl);
        self.col_chk_flags |= HONOTRAP_AT_ACTIVE;
        self.col_chk_flags |= HONOTRAP_AC_ACTIVE;
        self.col_chk_flags |= HONOTRAP_OC_ACTIVE;
    }

    /// `EnHonotrap_SetupEyeIdle`.
    fn setup_eye_idle(&mut self) {
        self.action = Action::EyeIdle;
        self.eye_state = HONOTRAP_EYE_SHUT;
    }

    /// `EnHonotrap_EyeIdle`: while its flame lives, 200 frames' wait set; after them, Link within
    /// 750 across, below it (to 700) and in front of it opens it.
    fn eye_idle(&mut self, play: &mut PlayState) {
        if self.actor.child.is_some() {
            self.timer = 200;
        } else if self.timer <= 0 && self.actor.xz_dist_to_player < 750.0 && self.actor.y_dist_to_player < 0.0 && self.actor.y_dist_to_player > -700.0 {
            // @bug (game) the angle is an s32 difference of two s16s, not wrapped: for an eye
            // facing near 0x8000 one side of its front never counts (the Shadow Temple's face
            // -0x8000: Link at a yaw from it of 0x4000 to 0x7FFF isn't seen).
            let angle = self.actor.yaw_towards_player as i32 - self.actor.shape_rot.y as i32;
            if angle > -0x4000 && angle < 0x4000 {
                self.setup_eye_open(play);
            }
        }
    }

    /// `EnHonotrap_SetupEyeOpen`: red for 40 frames, 30 frames' stare.
    fn setup_eye_open(&mut self, play: &mut PlayState) {
        self.action = Action::EyeOpen;
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, 40);
        self.timer = 30;
        audio_play_actor_sfx2(play, NA_SE_EV_RED_EYE);
    }

    /// `EnHonotrap_EyeOpen`: a step a frame; open, its flame (12 out, 10 down, its child).
    fn eye_open(&mut self, play: &mut PlayState) {
        self.eye_state -= 1;
        if self.eye_state <= HONOTRAP_EYE_OPEN {
            self.setup_eye_attack();
            let h = self.actor.home_pos;
            let r = self.actor.home_rot;
            let pos = Vec3::new(sin_s(self.actor.shape_rot.y) * 12.0 + h.x, h.y - 10.0, cos_s(self.actor.shape_rot.y) * 12.0 + h.z);
            if let Err(e) = play.actor_spawn_as_child(&mut self.actor, ACTOR_EN_HONOTRAP, pos, [r.x, r.y, r.z], HONOTRAP_TYPE_FLAME_MOVE) {
                log::debug!("En_Honotrap: its flame: {e:?}");
            }
        }
    }

    /// `EnHonotrap_SetupEyeAttack`.
    fn setup_eye_attack(&mut self) {
        self.action = Action::EyeAttack;
        self.eye_state = HONOTRAP_EYE_OPEN;
    }

    /// `EnHonotrap_EyeAttack`.
    fn eye_attack(&mut self) {
        if self.timer <= 0 {
            self.setup_eye_close();
        }
    }

    /// `EnHonotrap_SetupEyeClose`.
    fn setup_eye_close(&mut self) {
        self.action = Action::EyeClose;
    }

    /// `EnHonotrap_EyeClose`: a step a frame; shut, 200 frames before it looks again.
    fn eye_close(&mut self) {
        self.eye_state += 1;
        if self.eye_state >= HONOTRAP_EYE_SHUT {
            self.setup_eye_idle();
            self.timer = 200;
        }
    }

    /// `EnHonotrap_SetupFlameGrow`.
    fn setup_flame_grow(&mut self) {
        self.action = Action::FlameGrow;
    }

    /// `EnHonotrap_FlameGrow`: to 0.004 (an eye's) or 0.0048 (Dampé's), 0.0006 a frame.
    fn flame_grow(&mut self) {
        let ty = TYPE.get(self.actor.params);
        let target_scale = if ty == HONOTRAP_TYPE_FLAME_MOVE { 0.004 } else { 0.0048 };
        let target_reached = step_to_f(&mut self.actor.scale.x, target_scale, 0.0006);
        self.actor.scale.y = self.actor.scale.x;
        self.actor.scale.z = self.actor.scale.x;
        if target_reached {
            if ty == HONOTRAP_TYPE_FLAME_MOVE {
                self.setup_flame_move();
            } else {
                // HONOTRAP_TYPE_FLAME_DROP
                self.setup_flame_drop();
            }
        }
    }

    /// `EnHonotrap_SetupFlameDrop`: thrown 2 a frame the way it faces, rising 1.
    fn setup_flame_drop(&mut self) {
        self.timer = 40;
        self.actor.velocity.y = 1.0;
        self.actor.velocity.x = 2.0 * sin_s(self.actor.world_rot.y);
        self.actor.velocity.z = 2.0 * cos_s(self.actor.world_rot.y);
        self.action = Action::FlameDrop;
    }

    /// `EnHonotrap_FlameDrop`: falling to 1 above the floor, faster each frame; out after 40
    /// frames or on Link (knocked down unless his shield took it).
    fn flame_drop(&mut self, play: &mut PlayState) {
        let at = self.cyl.base.at_flags;
        if at & AT_HIT != 0 || self.timer <= 0 {
            if at & AT_HIT != 0 && at & AT_BOUNCED == 0 {
                actor_set_player_knockback_large_no_damage(play, 5.0, self.actor.yaw_towards_player, 0.0);
            }
            self.actor.velocity = Vec3::ZERO;
            self.setup_flame_vanish();
            return;
        }
        if self.actor.velocity.y > 0.0 {
            self.actor.world_pos.x += self.actor.velocity.x;
            self.actor.world_pos.z += self.actor.velocity.z;
            self.actor.update_bg_check_info(&play.col, 7.0, 12.0, 0.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
        }
        if !step_to_f(&mut self.actor.world_pos.y, self.actor.floor_height + 1.0, self.actor.velocity.y) {
            self.actor.velocity.y += 1.0;
        } else {
            self.actor.velocity.y = 0.0;
        }
        self.flame_collision_check(play);
    }

    /// `EnHonotrap_SetupFlameMove`: towards `targetPos`, 160 frames.
    fn setup_flame_move(&mut self) {
        self.action = Action::FlameMove;
        let dist_inverse = 1.0 / (vec3f_dist_xyz(self.actor.world_pos, self.target_pos) + 1.0);
        self.actor.velocity.x = (self.target_pos.x - self.actor.world_pos.x) * dist_inverse;
        self.actor.velocity.y = (self.target_pos.y - self.actor.world_pos.y) * dist_inverse;
        self.actor.velocity.z = (self.target_pos.z - self.actor.world_pos.z) * dist_inverse;
        self.speed_mod = 0.0;
        self.timer = 160;
    }

    /// `EnHonotrap_FlameMove`.
    fn flame_move(&mut self, play: &mut PlayState) {
        step_to_f(&mut self.speed_mod, 13.0, 0.5);
        let v = self.actor.velocity;
        let speed = Vec3::new((self.speed_mod * v.x).abs(), (self.speed_mod * v.y).abs(), (self.speed_mod * v.z).abs());
        let mut target_reached = true;
        target_reached &= step_to_f(&mut self.actor.world_pos.x, self.target_pos.x, speed.x);
        target_reached &= step_to_f(&mut self.actor.world_pos.y, self.target_pos.y, speed.y);
        target_reached &= step_to_f(&mut self.actor.world_pos.z, self.target_pos.z, speed.z);
        self.actor.update_bg_check_info(&play.col, 7.0, 10.0, 0.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
        // `collider.tris.base.atFlags`: the union's base, the cylinder's.
        let at = self.cyl.base.at_flags;
        if at & AT_BOUNCED != 0 {
            let shield = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map(|p| p.shield_mf()).unwrap_or(oot_game::sys_matrix::MtxF::IDENTITY);
            let shield_vec = Vec3::new(-shield.xz, -shield.yz, -shield.zz);
            let shield_norm = get_normal(shield_vec);
            let temp_vel = self.actor.velocity;
            self.actor.velocity = math3d_vec3f_reflect(temp_vel, shield_norm);
            self.actor.speed_xz = self.speed_mod * 0.5;
            self.actor.world_rot.y = atan2_s(self.actor.velocity.z, self.actor.velocity.x);
            self.setup_flame_vanish();
        } else if at & AT_HIT != 0 {
            self.actor.speed_xz = 0.0;
            self.actor.velocity.y = 0.0;
            self.setup_flame_vanish();
        } else if self.timer <= 0 {
            self.setup_flame_vanish();
        } else {
            self.flame_collision_check(play);
            if target_reached {
                self.setup_flame_chase();
            }
        }
    }

    /// `EnHonotrap_SetupFlameChase`: still, 100 frames.
    fn setup_flame_chase(&mut self) {
        self.action = Action::FlameChase;
        self.actor.speed_xz = 0.0;
        self.actor.velocity = Vec3::ZERO;
        self.actor.world_rot.x = 0;
        self.actor.world_rot.y = 0;
        self.actor.world_rot.z = 0;
        self.timer = 100;
    }

    /// `EnHonotrap_FlameChase`: after Link, to his height.
    fn flame_chase(&mut self, play: &mut PlayState) {
        scaled_step_to_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player, 0x300);
        step_to_f(&mut self.actor.speed_xz, 3.0, 0.1);
        self.actor.gravity = if -self.actor.y_dist_to_player < 10.0 { 0.08 } else { -0.08 };
        self.actor.update_velocity();
        if self.actor.velocity.y > 1.0 {
            self.actor.velocity.y = 1.0;
        }
        self.actor.velocity.y *= 0.95;
        self.actor.update_pos();
        self.actor.update_bg_check_info(&play.col, 7.0, 10.0, 0.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
        let at = self.cyl.base.at_flags;
        if at & AT_BOUNCED != 0 {
            let shield = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map(|p| p.shield_mf()).unwrap_or(oot_game::sys_matrix::MtxF::IDENTITY);
            let shield_rot = shield.to_yxz_rot_s(false);
            self.actor.world_rot.y = shield_rot[1].wrapping_mul(2).wrapping_sub(self.actor.world_rot.y).wrapping_add(-0x8000);
            self.setup_flame_vanish();
        } else if at & AT_HIT != 0 {
            self.actor.speed_xz *= 0.1;
            self.actor.velocity.y *= 0.1;
            self.setup_flame_vanish();
        } else if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 || self.timer <= 0 {
            self.setup_flame_vanish();
        } else {
            self.flame_collision_check(play);
        }
    }

    /// `EnHonotrap_SetupFlameVanish`.
    fn setup_flame_vanish(&mut self) {
        self.action = Action::FlameVanish;
    }

    /// `EnHonotrap_FlameVanish`: shrinking (0.00015 a frame) as it drifts; gone at 0.0001.
    fn flame_vanish(&mut self, play: &mut PlayState) {
        let target_reached = step_to_f(&mut self.actor.scale.x, 0.0001, 0.00015);
        self.actor.scale.y = self.actor.scale.x;
        self.actor.scale.z = self.actor.scale.x;
        self.actor.move_forward();
        self.actor.update_bg_check_info(&play.col, 7.0, 10.0, 0.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
        if target_reached {
            self.actor.kill();
        }
    }
}

impl ActorImpl for EnHonotrap {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnHonotrap_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if self.timer > 0 {
            self.timer -= 1;
        }
        let eye = TYPE.get(self.actor.params) == HONOTRAP_TYPE_EYE;
        if eye {
            if let Some(c) = self.actor.child
                && play.actors.actor(c).is_none_or(|a| a.killed)
            {
                self.actor.child = None;
            }
        } else {
            self.col_chk_flags = 0;
            self.bob_phase = self.bob_phase.wrapping_add(0x640);
            self.actor.shape_y_offset = sin_s(self.bob_phase) * 1000.0 + 600.0;
            self.actor.set_focus(5.0);
            audio_play_actor_sfx2(play, NA_SE_EV_BURN_OUT - SFX_FLAG);
        }
        match self.action {
            Action::EyeIdle => self.eye_idle(play),
            Action::EyeOpen => self.eye_open(play),
            Action::EyeAttack => self.eye_attack(),
            Action::EyeClose => self.eye_close(),
            Action::FlameGrow => self.flame_grow(),
            Action::FlameDrop => self.flame_drop(play),
            Action::FlameMove => self.flame_move(play),
            Action::FlameChase => self.flame_chase(play),
            Action::FlameVanish => self.flame_vanish(play),
        }
        if eye {
            if self.tris.base.ac_flags & AC_HIT != 0 {
                let pos = self.actor.world_pos;
                play.with_ss(|s| s.bomb2_spawn_layered(pos, BURST_VELOCITY, BURST_ACCEL, 15, 8));
                self.actor.kill();
            } else if self.eye_state < HONOTRAP_EYE_SHUT {
                self.tris.base.ac_flags &= !AC_HIT;
                play.collision_check_set_ac(&self.actor, 0, &mut self.tris);
            }
        }
    }

    /// `EnHonotrap_DrawFlame`'s change: the scroll, 20 a drawn frame (`flameScroll %= 512U`).
    fn draw_update(&mut self, _play: &mut PlayState) {
        let ty = TYPE.get(self.actor.params);
        if ty == HONOTRAP_TYPE_FLAME_MOVE || ty == HONOTRAP_TYPE_FLAME_DROP {
            self.flame_scroll = self.flame_scroll.wrapping_sub(20);
            self.flame_scroll = ((self.flame_scroll as i32 as u32) % (128u32 << 2)) as i16;
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.switches = vec![self.eye_state.clamp(0, HONOTRAP_EYE_MAX - 1) as u32];
        rs
    }

    /// `EnHonotrap_Draw`: `EnHonotrap_DrawEye` or `EnHonotrap_DrawFlame` by type.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        match TYPE.get(self.actor.params) {
            HONOTRAP_TYPE_EYE => {
                let state = rs.switches.first().copied().unwrap_or(0) as usize;
                out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(&eye_bake(SILVER_EYE_TEXTURES[state.min(SILVER_EYE_TEXTURES.len() - 1)]))), actor_draw_matrix(rs)));
            }
            HONOTRAP_TYPE_FLAME_MOVE | HONOTRAP_TYPE_FLAME_DROP => {
                // Matrix_RotateY(BINANG_TO_RAD((s16)(camDirYaw - shape.rot.y + 0x8000)), MTXMODE_APPLY).
                let yaw = play.cam_dir_yaw().wrapping_sub(rs.rot[1]).wrapping_add(-0x8000);
                let m = actor_draw_matrix(rs) * Mat4::from_rotation_y(binang_to_rad(yaw));
                let mut sv = SegmentValues::default();
                sv.read(SEG_SCROLL, &flame_scroll(self.flame_scroll));
                out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(FLAME_BAKE)), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
            }
            _ => {}
        }
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            0 => Some(ColliderMut::Tris(&mut self.tris)),
            1 => Some(ColliderMut::Cylinder(&mut self.cyl)),
            _ => None,
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
