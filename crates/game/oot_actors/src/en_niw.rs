//! `En_Niw` (`ovl_En_Niw/z_en_niw.c`): the cucco. Its params are its kind:
//! - 0: the usual cucco, wandering about its home (`func_80AB6570`), carried and thrown;
//! - 1, 3: the ranch's before and after Talon's return; 5, 7: before and after Epona; 2: at night;
//! - 4: a cucco to pick up (`Actor_OfferGetItem` with `GI_NONE`), floating until then;
//! - 0xA: the market girl's cucco, which runs her path while she is near (`func_80AB6A38`, the
//!   path set by `En_Niw_Girl`), never carried;
//! - 0xB, 0xC: Zora's River's two (one each at a time); 0xD: flies off when let go; 0xE: the
//!   ranch's cucco that comes back.
//!
//! Struck enough times (`unk_2A4`, 5 to 8) a cucco calls the others (`func_80AB70A0`: the
//! one-point camera 2290, then `EnNiw_SpawnAttackCucco`, up to 7 `En_Attack_Niw`), and knocks
//! Link down when it reaches him. Feathers fly when it's hit or carried (`EnNiw_SpawnFeather`).
//!
//! Drawn as `EnNiw_Draw` does: the skeleton with its head, tail and wings turned
//! (`EnNiw_OverrideLimbDraw`), a second circle shadow while it waits to be picked up
//! (`func_80033C30`, 0.15), and the feathers (`EnNiw_DrawEffects`); from baked meshes
//! (docs/adr/0012-actor-bakes.md). Its circle shadow (25, then 15 from its first update) is the
//! shape's (`ActorShape_Init`'s, drawn by `Actor_Draw`).
//!
//! Carried (`Actor_OfferCarry`, Player's lift), it flaps over Link's head and slows his fall
//! (Player's glide, `Player_UpperAction_CarryActor`); thrown, it flutters down.
//!
//! Not ported here: the splash and ripples in water (`EffectSsGSplash`, `EffectSsGRipple`: the
//! effects aren't ported).
//!
//! Params: `params` whole, the kind (negative: 0 at init; 0, 6 and 9 the usual cucco, 8 one that
//! wanders nearer home; 1, 3 the ranch's before and after Talon's return; 5, 7 before and after
//! Epona; 2 by night only; 4 to pick up, 0 once picked; 0xA the market girl's (`En_Niw_Girl`'s,
//! never carried); 0xB, 0xC Zora's River's two; 0xD Talon's (`En_Ta`'s), flying off when let go; 0xE the ranch's that
//! comes back; 0xA, 0xD and 0xE gone from Link's house until the cow is unlocked). In Kakariko a
//! cucco placed at one of `sKakarikoPosList`'s seven places, brought back, starts by the pen as
//! kind 0. The init's last params are printed.
//! Flags: reads EVENTCHKINF_TALON_RETURNED_FROM_CASTLE (kinds 1, 3), EVENTCHKINF_EPONA_OBTAINED
//! (5, 7), EVENTCHKINF_HORSE_RACE_COW_UNLOCK (0xA, 0xD, 0xE in Link's house) and INFTABLE_199 to
//! INFTABLE_19F (Kakariko's cuccos brought back, `sKakarikoFlagList`).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::bgcheck::BGCHECK_Y_MIN;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{approach_f, approach_zero_f, sin_s, smooth_step_to_s};
use glam::{Mat4, Vec3};
use oot_game::actor::{
    ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_THROW_ONLY, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, BGCHECKFLAG_GROUND, BGCHECKFLAG_WALL, BGCHECKFLAG_WATER, UPDBGCHECKINFO_FLAG_0,
    UPDBGCHECKINFO_FLAG_1, UPDBGCHECKINFO_FLAG_2, UPDBGCHECKINFO_FLAG_3, UPDBGCHECKINFO_FLAG_4,
};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile, actor_set_player_knockback_large, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind};
use oot_game::{event_chk_inf, inf_table};
use oot_game::camera::{CAM_ID_MAIN, f_atan2f};
use oot_game::collision_check::*;
use oot_game::get_item::{actor_has_parent, offer_get_item_range, offer_pick_up};
use oot_game::item::GI_NONE;
use oot_game::pack::{BakeBody, MeshBake, keys};
use oot_game::path::rad_to_binang_trunc;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::*;

/// `ACTOR_EN_NIW` (`actor_table.h`: 0x0019).
pub const ACTOR_EN_NIW: i16 = 0x0019;
/// `ACTOR_EN_ATTACK_NIW` (`actor_table.h`: 0x0144).
pub const ACTOR_EN_ATTACK_NIW: i16 = 0x0144;
pub const OBJECT: &str = "object_niw";

/// `En_Niw_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_NIW, name: "En_Niw", category: ACTORCAT_PROP, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_THROW_ONLY, object: OBJECT, schema: &SCHEMA };

/// `this->actor.params`: the kind (negative: made 0 at init).
pub const TYPE: Field = Field::whole(
    "params",
    Kind::Named(&[
        (-1, "the usual cucco (made 0 at init)"),
        (0x0, "the usual cucco"),
        (0x1, "the ranch's, before Talon's return"),
        (0x2, "by night only"),
        (0x3, "the ranch's, after Talon's return"),
        (0x4, "to pick up, floating till then"),
        (0x5, "before Epona"),
        (0x6, "the usual cucco"),
        (0x7, "after Epona"),
        (0x8, "wanders nearer home"),
        (0x9, "the usual cucco"),
        (0xA, "the market girl's, running her path"),
        (0xB, "Zora's River's lower one"),
        (0xC, "Zora's River's upper one"),
        (0xD, "Talon's super cucco (En_Ta's): flies off when let go"),
        (0xE, "the ranch's that comes back"),
    ]),
    "which cucco: where and when it's there, how it moves",
);

/// `INFTABLE_19A` to `INFTABLE_19F` (`save.h`).
const INFTABLE_19A: u16 = 0x19A;
const INFTABLE_19B: u16 = 0x19B;
const INFTABLE_19C: u16 = 0x19C;
const INFTABLE_19D: u16 = 0x19D;
const INFTABLE_19E: u16 = 0x19E;
const INFTABLE_19F: u16 = 0x19F;
const KAKARIKO_FLAGS: &str = "sKakarikoFlagList[i], INFTABLE_199 + i";

/// En_Niw's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_TALON_RETURNED_FROM_CASTLE)).reads().note("kind 1 gone after it, kind 3 before"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_EPONA_OBTAINED)).reads().note("kind 5 gone after it, kind 7 before"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_HORSE_RACE_COW_UNLOCK)).reads().note("kinds 0xA, 0xD, 0xE in Link's house: gone before it"),
        FlagUse::new(inf_table!(INFTABLE_199)).reads().note("Kakariko's cucco 0 brought back").indirect(KAKARIKO_FLAGS),
        FlagUse::new(inf_table!(INFTABLE_19A)).reads().note("Kakariko's cucco 1 brought back").indirect(KAKARIKO_FLAGS),
        FlagUse::new(inf_table!(INFTABLE_19B)).reads().note("Kakariko's cucco 2 brought back").indirect(KAKARIKO_FLAGS),
        FlagUse::new(inf_table!(INFTABLE_19C)).reads().note("Kakariko's cucco 3 brought back").indirect(KAKARIKO_FLAGS),
        FlagUse::new(inf_table!(INFTABLE_19D)).reads().note("Kakariko's cucco 4 brought back").indirect(KAKARIKO_FLAGS),
        FlagUse::new(inf_table!(INFTABLE_19E)).reads().note("Kakariko's cucco 5 brought back").indirect(KAKARIKO_FLAGS),
        FlagUse::new(inf_table!(INFTABLE_19F)).reads().note("Kakariko's cucco 6 brought back").indirect(KAKARIKO_FLAGS),
    ],
    todo: false,
};


// `SCENE_*` (`scene_table.h`).
const SCENE_LINKS_HOUSE: u16 = 0x34;
const SCENE_KAKARIKO_VILLAGE: u16 = 0x52;
const SCENE_ZORAS_RIVER: u16 = 0x54;

// `EVENTCHKINF_*`, `INFTABLE_*` (`save.h`).
const EVENTCHKINF_TALON_RETURNED_FROM_CASTLE: u16 = 0x14;
const EVENTCHKINF_EPONA_OBTAINED: u16 = 0x18;
const EVENTCHKINF_HORSE_RACE_COW_UNLOCK: u16 = 0x1E;
/// `INFTABLE_199` to `INFTABLE_19F`: Kakariko's cuccos brought back (`sKakarikoFlagList`).
const INFTABLE_199: u16 = 0x199;

// Sound effects (`sfx/environmentbank_table.h`, `enemybank_table.h`).
const NA_SE_EV_CHICKEN_CRY_N: u16 = 0x2811;
const NA_SE_EV_CHICKEN_CRY_A: u16 = 0x2812;
const NA_SE_EV_CHICKEN_CRY_M: u16 = 0x2813;
const NA_SE_EN_DEKU_WAKEUP: u16 = 0x38FF;

/// `ATTENTION_RANGE_6`.
const ATTENTION_RANGE_6: u8 = 6;
/// `EN_NIW_EFFECT_COUNT`.
pub const EN_NIW_EFFECT_COUNT: usize = 20;

/// `D_80AB8604`.
const D_80AB8604: [f32; 2] = [5000.0, -5000.0];
/// `D_80AB860C`.
const D_80AB860C: [f32; 3] = [5000.0, 3000.0, 4000.0];
/// `sKakarikoPosList`: where Kakariko's seven cuccos start.
const KAKARIKO_POS_LIST: [Vec3; 7] = [
    Vec3::new(-1697.0, 80.0, 870.0),
    Vec3::new(57.0, 320.0, -673.0),
    Vec3::new(796.0, 80.0, 1639.0),
    Vec3::new(1417.0, 465.0, 169.0),
    Vec3::new(-60.0, 0.0, -46.0),
    Vec3::new(-247.0, 80.0, 854.0),
    Vec3::new(1079.0, 80.0, -47.0),
];

/// `sCylinderInit1`: struck by Player.
const CYLINDER_INIT1: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HIT5, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: 0, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: 0, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 15, height: 25, y_shift: 4, pos: [0; 3] },
};

/// `sCylinderInit2`: only pushed.
const CYLINDER_INIT2: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: 0, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: 0, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 15, height: 25, y_shift: 4, pos: [0; 3] },
};

/// The overlay's statics.
#[derive(Default)]
struct Statics {
    /// `D_80AB85E0`: a cucco has called the others.
    d_80ab85e0: i16,
    /// `sLowerRiverSpawned`, `sUpperRiverSpawned`.
    lower_river_spawned: bool,
    upper_river_spawned: bool,
}

/// The cucco's skeleton.
pub const BAKE: &str = "En_Niw/skel";
/// A feather (`gCuccoEffectFeatherMaterialDL`, then `gCuccoEffectFeatherModelDL`).
pub const FEATHER_BAKE: &str = "En_Niw/feather";

/// The meshes `EnNiw_Draw` draws.
pub fn bakes() -> Vec<MeshBake> {
    vec![
        MeshBake {
            name: BAKE.into(),
            object: OBJECT.into(),
            segments: Vec::new(),
            prelude: Vec::new(),
            body: BakeBody::Skeleton { file: OBJECT.into(), symbol: "gCuccoSkel".into(), limbs: Vec::new() },
        },
        MeshBake {
            name: FEATHER_BAKE.into(),
            object: OBJECT.into(),
            segments: Vec::new(),
            prelude: Vec::new(),
            body: BakeBody::DLists(vec![(OBJECT.into(), "gCuccoEffectFeatherMaterialDL".into()), (OBJECT.into(), "gCuccoEffectFeatherModelDL".into())]),
        },
    ]
}

/// `EnNiwEffect`: a feather.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EnNiwEffect {
    pub ty: u8,
    pub pos: Vec3,
    pub vel: Vec3,
    pub accel: Vec3,
    pub life: i16,
    pub unk_2a: i16,
    pub scale: f32,
    pub unk_30: f32,
    pub timer: u8,
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    ResetAction,
    /// Waiting to be let go (0xD).
    Func80AB6324,
    /// Let go, falling (0xD).
    Func80AB63A8,
    /// Waiting to be picked up (4).
    Func80AB6450,
    /// Wandering about home.
    Func80AB6570,
    /// Running the girl's path (0xA).
    Func80AB6A38,
    /// Carried.
    Func80AB6BF8,
    /// Thrown.
    Func80AB6D08,
    /// Flying off (0xD).
    Func80AB6EB4,
    /// Swimming.
    Func80AB6F04,
    /// Calling the others: the camera.
    Func80AB70A0,
    Func80AB70F8,
    Func80AB714C,
    /// Calling the others: the attack.
    Func80AB7204,
    /// Running from Link.
    Func80AB7290,
    Func80AB7328,
    /// Fallen out of the world, coming down at home.
    Func80AB7420,
}

pub struct EnNiw {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub action: Action,
    pub timer1: i16,
    pub timer2: i16,
    pub timer3: i16,
    pub timer4: i16,
    pub timer5: i16,
    pub timer6: i16,
    pub sfx_timer1: i16,
    pub sfx_timer2: i16,
    pub sfx_timer3: i16,
    pub timer7: i16,
    pub timer8: i16,
    pub timer9: i16,
    pub unk_26c: [f32; 10],
    pub unk_294: i16,
    pub unk_296: i16,
    pub unk_298: i16,
    pub unk_29a: i16,
    pub unk_29c: i16,
    pub unk_29e: i16,
    pub unk_2a0: i16,
    pub unk_2a2: i16,
    pub unk_2a4: i16,
    pub unk_2a6: i16,
    pub unk_2a8: i16,
    pub unk_2aa: i16,
    pub unk_2ac: Vec3,
    pub unk_2b8: Vec3,
    pub unk_2c4: f32,
    pub unk_2c8: f32,
    pub unk_2cc: f32,
    pub unk_2d0: f32,
    pub unk_2d4: f32,
    pub unk_2d8: f32,
    pub unk_2dc: f32,
    pub unk_2e0: f32,
    pub unk_2e4: i16,
    pub unk_2e6: i16,
    /// `path`: the girl's path + 1 while she's near (0 for none); for a thrown cucco, set.
    pub path: i16,
    pub waypoint: i16,
    /// `unk_2EC`: the girl's path's point count.
    pub unk_2ec: i16,
    pub unk_2ee: i16,
    pub unk_2f0: Vec3,
    pub unk_2fc: f32,
    pub unk_300: f32,
    pub unk_304: f32,
    /// `unk_308`: let go (0xD).
    pub unk_308: u8,
    pub collider: ColliderCylinder,
    pub effects: [EnNiwEffect; EN_NIW_EFFECT_COUNT],
    anim: Anim,
}

/// `(s16)f`: a float to `s16` as the MIPS does it (truncated to `s32`, the low 16 bits).
fn f_to_s16(f: f32) -> i16 {
    f as i32 as i16
}

impl EnNiw {
    /// `EnNiw_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = (|| -> anyhow::Result<_> { Ok((assets.skeleton(OBJECT, "gCuccoSkel")?, assets.animation(OBJECT, "gCuccoAnim")?)) })();
        let (skeleton, anim) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Niw: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let mut this = EnNiw {
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(anim.clone())),
            skeleton,
            action: Action::ResetAction,
            timer1: 0,
            timer2: 0,
            timer3: 0,
            timer4: 0,
            timer5: 0,
            timer6: 0,
            sfx_timer1: 0,
            sfx_timer2: 0,
            sfx_timer3: 0,
            timer7: 0,
            timer8: 0,
            timer9: 0,
            unk_26c: [0.0; 10],
            unk_294: 0,
            unk_296: 0,
            unk_298: 0,
            unk_29a: 0,
            unk_29c: 0,
            unk_29e: 0,
            unk_2a0: 0,
            unk_2a2: 0,
            unk_2a4: 0,
            unk_2a6: 0,
            unk_2a8: 0,
            unk_2aa: 0,
            unk_2ac: Vec3::ZERO,
            unk_2b8: Vec3::ZERO,
            unk_2c4: 0.0,
            unk_2c8: 0.0,
            unk_2cc: 0.0,
            unk_2d0: 0.0,
            unk_2d4: 0.0,
            unk_2d8: 0.0,
            unk_2dc: 0.0,
            unk_2e0: 0.0,
            unk_2e4: 0,
            unk_2e6: 0,
            path: 0,
            waypoint: 0,
            unk_2ec: 0,
            unk_2ee: 0,
            unk_2f0: Vec3::ZERO,
            unk_2fc: 0.0,
            unk_300: 0.0,
            unk_304: 0.0,
            unk_308: 0,
            collider: ColliderCylinder::default(),
            effects: [EnNiwEffect::default(); EN_NIW_EFFECT_COUNT],
            anim,
            actor,
        };
        let a = &mut this.actor;
        if TYPE.get(a.params) < 0 {
            a.params = 0;
        }

        // The cucco at the very beginning of Zora's River.
        if TYPE.get(a.params) == 0xB {
            let s = play.overlay_static::<Statics>(ACTOR_EN_NIW);
            if s.lower_river_spawned {
                a.kill();
                log::debug!("En_Niw: I'm already here -Hara Ver.1");
                return Box::new(this);
            }
            s.lower_river_spawned = true;
            a.room = -1;
        }

        // The cucco on the tall platform in Zora's River's first room.
        if TYPE.get(a.params) == 0xC {
            let s = play.overlay_static::<Statics>(ACTOR_EN_NIW);
            if s.upper_river_spawned {
                a.kill();
                log::debug!("En_Niw: I'm already here -Hara Ver.2");
                return Box::new(this);
            }
            s.upper_river_spawned = true;
            a.room = -1;
        }

        // sInitChain.
        a.target_mode = ATTENTION_RANGE_6;
        a.gravity = -2.0;
        a.target_arrow_offset = 0.0;
        a.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
        a.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 25.0);

        if play.scene_id == SCENE_KAKARIKO_VILLAGE {
            for (i, p) in KAKARIKO_POS_LIST.iter().enumerate() {
                if (a.world_pos.x - p.x).abs() < 40.0 && (a.world_pos.z - p.z).abs() < 40.0 {
                    this.unk_2aa = i as i16;
                    log::debug!("En_Niw: normal chicken index {i}");
                    if play.save.get_inf_table(INFTABLE_199 + i as u16) { // schema: sKakarikoFlagList[i]
                        a.world_pos = Vec3::new(300.0, 100.0, 1530.0);
                        a.params = 0;
                    }
                }
            }
        }

        this.unk_2ac = a.world_pos;
        this.unk_2b8 = a.world_pos;
        this.unk_304 = 10.0;
        a.scale = Vec3::splat(0.01);
        this.unk_2a4 = (play.rand.zero_float(3.99) as i16).wrapping_add(5);
        if this.unk_2a4 < 0 {
            this.unk_2a4 = 1;
        }

        let s = &play.save;
        match TYPE.get(a.params) {
            2 => {
                if s.is_day() {
                    a.kill();
                }
            }
            1 => {
                if s.get_event_chk_inf(EVENTCHKINF_TALON_RETURNED_FROM_CASTLE) {
                    a.kill();
                }
            }
            3 => {
                if !s.get_event_chk_inf(EVENTCHKINF_TALON_RETURNED_FROM_CASTLE) {
                    a.kill();
                }
            }
            5 => {
                if s.get_event_chk_inf(EVENTCHKINF_EPONA_OBTAINED) {
                    a.kill();
                }
            }
            7 => {
                if !s.get_event_chk_inf(EVENTCHKINF_EPONA_OBTAINED) {
                    a.kill();
                }
            }
            0xD | 0xE => {
                if TYPE.get(a.params) == 0xD {
                    a.gravity = 0.0;
                }
                a.col_chk_info.mass = 0;
                a.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
            }
            4 => a.gravity = 0.0,
            _ => {}
        }

        match TYPE.get(a.params) {
            0xA | 0xD | 0xE => {
                if TYPE.get(a.params) == 0xA {
                    a.col_chk_info.mass = MASS_IMMOVABLE;
                }
                this.collider = ColliderCylinder::new(&CYLINDER_INIT2);
                if play.scene_id == SCENE_LINKS_HOUSE && !s.get_event_chk_inf(EVENTCHKINF_HORSE_RACE_COW_UNLOCK) {
                    a.kill();
                }
            }
            _ => this.collider = ColliderCylinder::new(&CYLINDER_INIT1),
        }

        log::debug!("En_Niw: what kind of guy? {}", a.params); // schema: printed
        this.action = Action::ResetAction;
        Box::new(this)
    }

    /// The cucco's statics.
    fn statics(play: &mut PlayState) -> &mut Statics {
        play.overlay_static::<Statics>(ACTOR_EN_NIW)
    }

    /// `func_80AB5BF8`: the head, wings and tail's flapping, by `arg2` (0 still, 1 pecking, 2
    /// flapping, 3 to 5 the calling's).
    fn func_80ab5bf8(&mut self, play: &mut PlayState, arg2: i16) {
        let factor = if TYPE.get(self.actor.params) == 0xD { 2.0 } else { 1.0 };
        if self.timer1 == 0 {
            self.unk_26c[0] = if arg2 == 0 { 0.0 } else { -10000.0 * factor };
            self.unk_298 = self.unk_298.wrapping_add(1);
            self.timer1 = 3;
            if self.unk_298 & 1 == 0 {
                self.unk_26c[0] = 0.0;
                if arg2 == 0 {
                    self.timer1 = play.rand.zero_float(30.0) as i16;
                }
            }
        }
        if self.timer2 == 0 {
            self.unk_29c = self.unk_29c.wrapping_add(1);
            self.unk_29c &= 1;
            let u = &mut self.unk_26c;
            match arg2 {
                0 => {
                    u[1] = 0.0;
                    u[2] = 0.0;
                }
                1 => {
                    self.timer2 = 3;
                    u[1] = 7000.0 * factor;
                    u[2] = 7000.0 * factor;
                    if self.unk_29c == 0 {
                        u[1] = 0.0;
                        u[2] = 0.0;
                    }
                }
                2 => {
                    self.timer2 = 2;
                    u[1] = -10000.0;
                    u[2] = -10000.0;
                    u[7] = 25000.0;
                    u[5] = 25000.0;
                    u[8] = 6000.0;
                    u[6] = 6000.0;
                    if self.unk_29c == 0 {
                        u[5] = 8000.0;
                        u[7] = 8000.0;
                    }
                }
                3 => {
                    self.timer2 = 2;
                    u[7] = 10000.0;
                    u[5] = 10000.0;
                    if self.unk_29c == 0 {
                        u[7] = 3000.0;
                        u[5] = 3000.0;
                    }
                }
                4 => self.timer1 = 5,
                5 => {
                    self.timer2 = 5;
                    u[7] = 14000.0;
                    u[5] = 14000.0;
                    if self.unk_29c == 0 {
                        u[7] = 10000.0;
                        u[5] = 10000.0;
                    }
                }
                _ => {}
            }
        }
        let u = self.unk_26c;
        if self.unk_2e0 != u[9] {
            approach_f(&mut self.unk_2e0, u[9], 0.5, 4000.0);
        }
        if self.unk_2dc != u[0] {
            approach_f(&mut self.unk_2dc, u[0], 0.5, 4000.0);
        }
        if self.unk_2c4 != u[2] {
            approach_f(&mut self.unk_2c4, u[2], 0.8, 7000.0);
        }
        if self.unk_2c8 != u[7] {
            approach_f(&mut self.unk_2c8, u[7], 0.8, 7000.0);
        }
        if self.unk_2cc != u[8] {
            approach_f(&mut self.unk_2cc, u[8], 0.8, 7000.0);
        }
        if self.unk_2d0 != u[1] {
            approach_f(&mut self.unk_2d0, u[1], 0.8, 7000.0);
        }
        if self.unk_2d4 != u[5] {
            approach_f(&mut self.unk_2d4, u[5], 0.8, 7000.0);
        }
        if self.unk_2d8 != u[6] {
            approach_f(&mut self.unk_2d8, u[6], 0.8, 7000.0);
        }
    }

    /// `EnNiw_SpawnAttackCucco`: an `En_Attack_Niw` somewhere in view, every 10 frames, up to 7.
    fn spawn_attack_cucco(&mut self, play: &mut PlayState) {
        if self.timer5 == 0 && self.unk_296 < 7 {
            let (eye, at) = (play.view.eye, play.view.at);
            let view = at - eye;
            let x = (play.rand.zero_one() - 0.5) * view.x + eye.x;
            let y = play.rand.centered_float(0.3) + ((eye.y + 50.0) + (view.y * 0.5));
            let z = (play.rand.zero_one() - 0.5) * view.z + eye.z;
            match play.actor_spawn_as_child(&mut self.actor, ACTOR_EN_ATTACK_NIW, Vec3::new(x, y, z), [0; 3], 0) {
                Ok(_) => {
                    self.unk_296 += 1;
                    self.timer5 = 10;
                }
                Err(_) => log::debug!("En_Niw: cannot spawn"),
            }
        }
    }

    /// `func_80AB6100`: running, hopping, its yaw swinging about `unk_2E4`.
    fn func_80ab6100(&mut self, play: &mut PlayState, arg2: usize) {
        if self.timer4 == 0 {
            self.timer4 = 3;
            if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
                self.actor.velocity.y = 3.5;
            }
        }
        if self.timer3 == 0 {
            self.unk_2a0 = self.unk_2a0.wrapping_add(1);
            self.unk_2a0 &= 1;
            self.timer3 = 5;
        }
        let factor = if self.unk_2a0 == 0 { D_80AB860C[arg2] } else { -D_80AB860C[arg2] };
        if arg2 == 1 && (self.timer6 == 0 || self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0) {
            self.timer6 = 150;
            if self.timer8 == 0 {
                self.timer8 = 70;
                self.unk_2e4 = self.actor.yaw_towards_player;
            }
        }
        let target_rot_y = self.unk_2e4 as f32 + factor;
        smooth_step_to_s(&mut self.actor.world_rot.y, f_to_s16(target_rot_y), 3, f_to_s16(self.unk_2fc), 0);
        approach_f(&mut self.unk_2fc, 3000.0, 1.0, 500.0);
        self.func_80ab5bf8(play, 5);
    }

    /// `Animation_Change(gCuccoAnim, 1, 0, lastFrame, ANIMMODE_LOOP, -10)`.
    fn change_anim(&mut self) {
        let frames = self.anim.last_frame();
        self.skel.change(self.anim.clone(), 1.0, 0.0, frames, ANIMMODE_LOOP, -10.0);
    }

    /// Home where it stands, still (the reset the actions share).
    fn reset_home(&mut self) {
        self.unk_2ac = self.actor.world_pos;
        self.unk_2b8 = self.actor.world_pos;
        self.timer5 = 0;
        self.timer4 = 0;
        self.unk_29e = 0;
        self.unk_26c[7] = 0.0;
        self.unk_26c[5] = 0.0;
        self.unk_26c[6] = 0.0;
        self.unk_26c[8] = 0.0;
        self.actor.speed_xz = 0.0;
        self.unk_2fc = 0.0;
        self.unk_300 = 0.0;
    }

    /// Taken by Player (the hold's start the actions share).
    fn grabbed(&mut self, play: &mut PlayState) {
        audio_play_actor_sfx2(play, NA_SE_EV_CHICKEN_CRY_M);
        self.sfx_timer1 = 30;
        self.path = 0;
        self.timer4 = 30;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.actor.speed_xz = 0.0;
        self.action = Action::Func80AB6BF8;
    }

    /// `EnNiw_ResetAction`.
    fn reset_action(&mut self) {
        self.change_anim();
        self.action = match TYPE.get(self.actor.params) {
            4 => Action::Func80AB6450,
            0xD => Action::Func80AB6324,
            _ => Action::Func80AB6570,
        };
    }

    /// `func_80AB6324`.
    fn func_80ab6324(&mut self, play: &mut PlayState) {
        if self.unk_308 != 0 {
            self.actor.velocity.y = play.rand.zero_float(2.0) + 4.0;
            self.actor.speed_xz = play.rand.zero_float(2.0) + 3.0;
            self.action = Action::Func80AB63A8;
        }
        self.func_80ab5bf8(play, 1);
    }

    /// `func_80AB63A8`.
    fn func_80ab63a8(&mut self, play: &mut PlayState) {
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 && self.actor.velocity.y < 0.0 {
            self.reset_home();
            self.action = Action::Func80AB6570;
        } else {
            self.func_80ab5bf8(play, 2);
        }
    }

    /// `func_80AB6450`: waiting to be picked up; it runs when Link stands by it.
    fn func_80ab6450(&mut self, play: &mut PlayState) {
        let player_y = play.player.and_then(|h| play.actors.actor(h)).map(|p| p.world_pos.y).unwrap_or(0.0);
        if self.actor.xz_dist_to_player < 30.0 && (self.actor.world_pos.y - player_y).abs() < 5.0 {
            self.timer6 = 100;
            self.actor.gravity = -2.0;
            self.action = Action::Func80AB7290;
        } else if actor_has_parent(&self.actor) {
            self.actor.gravity = -2.0;
            self.grabbed(play);
        } else {
            // GI_NONE in this case allows the player to lift the actor.
            let a = self.actor.clone();
            offer_get_item_range(play, &a, GI_NONE, 25.0, 10.0);
            self.func_80ab5bf8(play, 1);
        }
    }

    /// `func_80AB6570`: wandering about home, in hops; the girl's cucco runs once she's near.
    fn func_80ab6570(&mut self, play: &mut PlayState) {
        let mut pos_y = play.rand.centered_float(100.0);
        let mut pos_z = play.rand.centered_float(100.0);

        if TYPE.get(self.actor.params) != 0xA {
            if actor_has_parent(&self.actor) {
                self.grabbed(play);
                return;
            }
            let a = self.actor.clone();
            offer_pick_up(play, &a);
        } else if self.path != 0 {
            self.unk_2a6 = 1;
            if self.sfx_timer3 == 0 {
                audio_play_actor_sfx2(play, NA_SE_EV_CHICKEN_CRY_M);
                self.sfx_timer3 = 100;
            }
            self.unk_2a0 = play.rand.zero_float(1.99) as i16;
            self.actor.speed_xz = 4.0;
            self.unk_300 = 0.0;
            self.unk_2fc = 0.0;
            self.action = Action::Func80AB6A38;
            return;
        }

        let mut tmp = 0;

        if self.timer5 != 0 {
            if play.rand.zero_float(3.99) < 1.0 {
                self.unk_2e6 = self.unk_2e6.wrapping_add(1);
                self.unk_2e6 &= 1;
            }
            approach_f(&mut self.unk_26c[9], D_80AB8604[self.unk_2e6 as usize], 0.5, 4000.0);
        }

        if self.timer5 == 0 && self.timer4 == 0 {
            self.unk_29e = self.unk_29e.wrapping_add(1);
            if self.unk_29e >= 8 {
                self.timer5 = play.rand.zero_float(30.0) as i16;
                self.unk_29e = play.rand.zero_float(3.99) as i16;
                if TYPE.get(self.actor.params) != 0xA && TYPE.get(self.actor.params) != 8 {
                    pos_y += if pos_y < 0.0 { -100.0 } else { 100.0 };
                    pos_z += if pos_z < 0.0 { -100.0 } else { 100.0 };
                } else {
                    pos_y = play.rand.centered_float(30.0);
                    pos_z = play.rand.centered_float(30.0);
                    pos_y += if pos_y < 0.0 { -20.0 } else { 20.0 };
                    pos_z += if pos_z < 0.0 { -20.0 } else { 20.0 };
                }
                self.unk_2b8.x = self.unk_2ac.x + pos_y;
                self.unk_2b8.z = self.unk_2ac.z + pos_z;
            } else {
                self.timer4 = 4;
                if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
                    self.actor.speed_xz = 0.0;
                    self.actor.velocity.y = 3.5;
                }
            }
        }

        if self.timer4 != 0 {
            approach_zero_f(&mut self.unk_26c[9], 0.5, 4000.0);
            tmp = 1;
            approach_f(&mut self.actor.world_pos.x, self.unk_2b8.x, 1.0, self.unk_2fc);
            approach_f(&mut self.actor.world_pos.z, self.unk_2b8.z, 1.0, self.unk_2fc);
            approach_f(&mut self.unk_2fc, 3.0, 1.0, 0.3);
            pos_y = self.unk_2b8.x - self.actor.world_pos.x;
            pos_z = self.unk_2b8.z - self.actor.world_pos.z;
            if pos_y.abs() < 10.0 {
                pos_y = 0.0;
            }
            if pos_z.abs() < 10.0 {
                pos_z = 0.0;
            }
            if pos_y == 0.0 && pos_z == 0.0 {
                self.timer4 = 0;
                self.unk_29e = 7;
            }
            smooth_step_to_s(&mut self.actor.world_rot.y, rad_to_binang_trunc(f_atan2f(pos_y, pos_z)), 3, f_to_s16(self.unk_300), 0);
            approach_f(&mut self.unk_300, 10000.0, 1.0, 1000.0);
        }

        self.func_80ab5bf8(play, tmp);
    }

    /// `func_80AB6A38`: running the girl's path while she's near.
    fn func_80ab6a38(&mut self, play: &mut PlayState) {
        let path_index = self.path - 1;
        if self.path == 0 {
            self.reset_home();
            self.action = Action::ResetAction;
            return;
        }
        // play->pathList[pathIndex].points[waypoint].
        let point = play.setup_path_list().get(path_index as usize).and_then(|p| p.points.get(self.waypoint as usize)).copied();
        let Some(point) = point else {
            log::warn!("En_Niw: path {path_index} has no point {}", self.waypoint);
            return;
        };
        let path_diff_x = point[0] as f32 - self.actor.world_pos.x;
        let path_diff_z = point[2] as f32 - self.actor.world_pos.z;
        self.unk_2e4 = rad_to_binang_trunc(f_atan2f(path_diff_x, path_diff_z));
        self.func_80ab6100(play, 2);
        if path_diff_x.abs() < 30.0 && path_diff_z.abs() < 30.0 {
            self.waypoint += 1;
            if self.waypoint >= self.unk_2ec {
                self.waypoint = 0;
            }
        }
        self.func_80ab5bf8(play, 2);
    }

    /// `func_80AB6BF8`: carried, wriggling.
    fn func_80ab6bf8(&mut self, play: &mut PlayState) {
        if self.timer4 == 0 {
            self.unk_2a6 = 2;
            self.timer4 = 10;
        }
        self.actor.shape_rot.x = play.rand.centered_float(5000.0) as i16;
        self.actor.shape_rot.y = play.rand.centered_float(5000.0) as i16;
        self.actor.shape_rot.z = play.rand.centered_float(5000.0) as i16;
        // Actor_HasNoParent.
        if !actor_has_parent(&self.actor) {
            if TYPE.get(self.actor.params) == 0xD {
                self.sfx_timer1 = 0;
                self.unk_2a6 = 1;
                self.action = Action::Func80AB6EB4;
                self.actor.velocity.y = 4.0;
                return;
            }
            self.actor.shape_rot.z = 0;
            self.actor.shape_rot.y = 0;
            self.actor.shape_rot.x = 0;
            self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
            self.action = Action::Func80AB6D08;
        }
        self.func_80ab5bf8(play, 2);
    }

    /// `func_80AB6D08`: thrown; hops about for a while, then runs from Link.
    fn func_80ab6d08(&mut self, play: &mut PlayState) {
        if self.path == 0 {
            if self.actor.bg_check_flags & BGCHECKFLAG_GROUND == 0 {
                return;
            }
            if TYPE.get(self.actor.params) == 0xE {
                self.reset_home();
                self.action = Action::ResetAction;
                return;
            }
            self.path = 1;
            self.timer5 = 80;
            self.actor.speed_xz = 0.0;
            self.actor.velocity.y = 4.0;
        } else {
            if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
                self.sfx_timer1 = 0;
                self.actor.velocity.y = 4.0;
                self.unk_2a6 = 1;
            }
            if self.timer5 == 0 {
                self.timer6 = 100;
                self.timer4 = 0;
                self.path = 0;
                self.action = Action::Func80AB7290;
                return;
            }
        }

        if actor_has_parent(&self.actor) {
            self.grabbed(play);
        } else {
            if self.timer5 >= 6 {
                let a = self.actor.clone();
                offer_pick_up(play, &a);
            }
            self.func_80ab5bf8(play, 2);
        }
    }

    /// `func_80AB6EB4`: flying off until 400 up.
    fn func_80ab6eb4(&mut self, play: &mut PlayState) {
        if self.actor.world_pos.y > 400.0 {
            self.actor.kill();
        }
        self.func_80ab5bf8(play, 2);
    }

    /// `func_80AB6F04`: swimming to the shore.
    fn func_80ab6f04(&mut self, play: &mut PlayState) {
        if self.unk_2a8 != 0 {
            self.spawn_attack_cucco(play);
        }
        self.actor.speed_xz = 2.0;
        if self.actor.bg_check_flags & BGCHECKFLAG_WATER != 0 {
            self.actor.gravity = 0.0;
            if self.actor.y_dist_to_water > 15.0 {
                self.actor.world_pos.y += 2.0;
            }
            if self.timer4 == 0 {
                self.timer4 = 30;
                // EffectSsGRipple_Spawn(play, &pos + depthInWater, 100, 500, 30): the ripples
                // aren't ported.
            }
            if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
                self.actor.velocity.y = 10.0;
                self.actor.speed_xz = 1.0;
            }
        } else {
            self.actor.gravity = -2.0;
            if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
                self.actor.velocity.y = 10.0;
                self.actor.speed_xz = 1.0;
                self.actor.gravity = 0.0;
            } else {
                self.actor.speed_xz = 4.0;
            }
            if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
                self.actor.gravity = -2.0;
                self.timer6 = 100;
                self.timer4 = 0;
                self.actor.velocity.y = 0.0;
                self.action = if self.unk_2a8 == 0 { Action::Func80AB7290 } else { Action::Func80AB7204 };
            }
        }
        self.func_80ab5bf8(play, 2);
    }

    /// `func_80AB70A0`: calling the others, the camera on it.
    fn func_80ab70a0(&mut self, play: &mut PlayState) {
        let me = play.cur_actor.map(|h| play.cam_actor_of(h, &self.actor));
        play.onepoint_cutscene_init(2290, -99, me, CAM_ID_MAIN);
        self.timer5 = 100;
        self.unk_2a2 = 1;
        self.action = Action::Func80AB70F8;
    }

    /// `func_80AB70F8`.
    fn func_80ab70f8(&mut self, play: &mut PlayState) {
        self.sfx_timer1 = 100;
        if self.timer5 == 0 {
            self.timer5 = 60;
            self.timer1 = 10;
            self.unk_2a2 = 4;
            self.action = Action::Func80AB714C;
        }
        let a = self.unk_2a2;
        self.func_80ab5bf8(play, a);
    }

    /// `func_80AB714C`.
    fn func_80ab714c(&mut self, play: &mut PlayState) {
        self.sfx_timer1 = 100;
        if self.timer5 == 40 {
            self.unk_26c[0] = 10000.0;
            self.unk_26c[7] = 14000.0;
            self.unk_26c[5] = 14000.0;
            self.unk_26c[6] = 0.0;
            self.unk_26c[8] = 0.0;
            self.unk_26c[1] = 0.0;
            self.unk_26c[2] = 0.0;
            self.timer1 = 10;
            audio_play_actor_sfx2(play, NA_SE_EV_CHICKEN_CRY_M);
        }
        if self.timer5 == 0 {
            self.timer7 = 10;
            self.unk_2e4 = self.actor.yaw_towards_player;
            self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
            self.action = Action::Func80AB7204;
        }
        let a = self.unk_2a2;
        self.func_80ab5bf8(play, a);
    }

    /// `func_80AB7204`: the attack, the cucco running at Link.
    fn func_80ab7204(&mut self, play: &mut PlayState) {
        self.spawn_attack_cucco(play);
        if self.timer7 < 2 {
            if self.timer7 == 1 {
                self.actor.speed_xz = 3.0;
                self.unk_2a0 = play.rand.zero_float(1.99) as i16;
                self.timer1 = 0;
                self.timer2 = 0;
                self.timer3 = 0;
                self.timer4 = 0;
            } else {
                self.func_80ab6100(play, 1);
            }
        }
    }

    /// `func_80AB7290`.
    fn func_80ab7290(&mut self, play: &mut PlayState) {
        self.change_anim();
        self.unk_2a0 = play.rand.zero_float(1.99) as i16;
        self.actor.speed_xz = 4.0;
        self.action = Action::Func80AB7328;
    }

    /// `func_80AB7328`: running from Link for `timer6` frames.
    fn func_80ab7328(&mut self, play: &mut PlayState) {
        if self.timer6 == 0 {
            self.reset_home();
            if TYPE.get(self.actor.params) == 4 {
                self.actor.params = 0;
            }
            self.action = Action::ResetAction;
        } else {
            let pp = play.player.and_then(|h| play.actors.actor(h)).map(|p| p.world_pos).unwrap_or(self.actor.world_pos);
            self.unk_2e4 = rad_to_binang_trunc(f_atan2f(self.actor.world_pos.x - pp.x, self.actor.world_pos.z - pp.z));
            self.func_80ab6100(play, 0);
            self.func_80ab5bf8(play, 2);
        }
    }

    /// `func_80AB7420`.
    fn func_80ab7420(&mut self, play: &mut PlayState) {
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.unk_2a4 = (play.rand.zero_float(3.99) as i16).wrapping_add(5);
            self.action = Action::ResetAction;
        }
    }

    /// `func_80AB747C`: struck: a cry, feathers, a strike counted, and away from Link.
    fn func_80ab747c(&mut self, play: &mut PlayState) {
        if self.unk_2a8 == 0 && TYPE.get(self.actor.params) != 0xA && self.action != Action::Func80AB6450 && self.collider.base.ac_flags & AC_HIT != 0 {
            self.collider.base.ac_flags &= !AC_HIT;
            self.sfx_timer1 = 30;
            if self.unk_2a4 > 0 && Self::statics(play).d_80ab85e0 == 0 {
                self.unk_2a4 -= 1;
            }
            self.unk_2a6 = 1;
            audio_play_actor_sfx2(play, NA_SE_EV_CHICKEN_CRY_M);
            self.timer6 = 100;
            self.path = 0;
            self.action = Action::Func80AB7290;
        }
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            Action::ResetAction => self.reset_action(),
            Action::Func80AB6324 => self.func_80ab6324(play),
            Action::Func80AB63A8 => self.func_80ab63a8(play),
            Action::Func80AB6450 => self.func_80ab6450(play),
            Action::Func80AB6570 => self.func_80ab6570(play),
            Action::Func80AB6A38 => self.func_80ab6a38(play),
            Action::Func80AB6BF8 => self.func_80ab6bf8(play),
            Action::Func80AB6D08 => self.func_80ab6d08(play),
            Action::Func80AB6EB4 => self.func_80ab6eb4(play),
            Action::Func80AB6F04 => self.func_80ab6f04(play),
            Action::Func80AB70A0 => self.func_80ab70a0(play),
            Action::Func80AB70F8 => self.func_80ab70f8(play),
            Action::Func80AB714C => self.func_80ab714c(play),
            Action::Func80AB7204 => self.func_80ab7204(play),
            Action::Func80AB7290 => self.func_80ab7290(play),
            Action::Func80AB7328 => self.func_80ab7328(play),
            Action::Func80AB7420 => self.func_80ab7420(play),
        }
    }

    /// `EnNiw_SpawnFeather`: in the first free slot.
    fn spawn_feather(&mut self, play: &mut PlayState, pos: Vec3, vel: Vec3, accel: Vec3, scale: f32) {
        if let Some(effect) = self.effects.iter_mut().find(|e| e.ty == 0) {
            effect.ty = 1;
            effect.pos = pos;
            effect.vel = vel;
            effect.accel = accel;
            effect.timer = 0;
            effect.scale = scale / 1000.0;
            effect.life = (play.rand.zero_float(20.0) as i16).wrapping_add(40);
            effect.unk_2a = play.rand.zero_float(1000.0) as i16;
        }
    }

    /// `EnNiw_UpdateEffects`: the feathers drift down, swaying.
    fn update_effects(&mut self) {
        for effect in self.effects.iter_mut().filter(|e| e.ty != 0) {
            effect.timer = effect.timer.wrapping_add(1);
            effect.pos += effect.vel;
            effect.vel += effect.accel;
            if effect.ty == 1 {
                effect.unk_2a = effect.unk_2a.wrapping_add(1);
                approach_f(&mut effect.vel.x, 0.0, 1.0, 0.05);
                approach_f(&mut effect.vel.z, 0.0, 1.0, 0.05);
                if effect.vel.y < -0.5 {
                    effect.vel.y = -0.5;
                }
                effect.unk_30 = sin_s(effect.unk_2a.wrapping_mul(0xBB8)) * std::f32::consts::PI * 0.2;
                if (effect.life as i32) < effect.timer as i32 {
                    effect.ty = 0;
                }
            }
        }
    }

    /// The limb matrices `EnNiw_Draw` draws with: `EnNiw_OverrideLimbDraw`'s turns. `t` is
    /// `unk_2C4` to `unk_2E0`.
    fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], t: [i16; 8]) -> Vec<Mat4> {
        let [c4, c8, cc, d0, d4, d8, dc, e0] = t;
        skeleton.pose_override(joints, |limb, _pos, rot| {
            match limb {
                13 => rot[1] = rot[1].wrapping_add(dc),
                15 => rot[1] = rot[1].wrapping_add(e0),
                11 => {
                    rot[0] = rot[0].wrapping_add(d8);
                    rot[1] = rot[1].wrapping_add(d4);
                    rot[2] = rot[2].wrapping_add(d0);
                }
                7 => {
                    rot[0] = rot[0].wrapping_add(cc);
                    rot[1] = rot[1].wrapping_add(c8);
                    rot[2] = rot[2].wrapping_add(c4);
                }
                _ => {}
            }
            Mat4::IDENTITY
        })
    }

    fn turns(&self) -> [i16; 8] {
        [self.unk_2c4, self.unk_2c8, self.unk_2cc, self.unk_2d0, self.unk_2d4, self.unk_2d8, self.unk_2dc, self.unk_2e0].map(f_to_s16)
    }
}

impl ActorImpl for EnNiw {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnNiw_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.unk_294 = self.unk_294.wrapping_add(1);

        if self.action != Action::Func80AB6570 {
            self.unk_26c[9] = 0.0;
        }
        if self.unk_2a6 != 0 {
            let feather_count = if self.unk_2a6 == 2 { 4 } else { 20 };
            for _ in 0..feather_count {
                let w = self.actor.world_pos;
                let mut pos = Vec3::new(play.rand.centered_float(10.0) + w.x, 0.0, 0.0);
                pos.y = play.rand.centered_float(10.0) + (w.y + self.unk_304);
                pos.z = play.rand.centered_float(10.0) + w.z;
                let mut scale = play.rand.zero_float(6.0) + 6.0;
                if self.unk_2a6 == 2 && self.unk_304 != 0.0 {
                    pos.y += 10.0;
                }
                if self.unk_304 == 0.0 {
                    scale = play.rand.zero_float(2.0) + 2.0;
                }
                let vx = play.rand.centered_float(3.0);
                let vy = (play.rand.zero_float(2.0) * 0.5) + 2.0;
                let vz = play.rand.centered_float(3.0);
                self.spawn_feather(play, pos, Vec3::new(vx, vy, vz), Vec3::new(0.0, -0.15, 0.0), scale);
            }
            self.unk_2a6 = 0;
        }

        self.update_effects();
        for t in [
            &mut self.timer1,
            &mut self.timer2,
            &mut self.timer3,
            &mut self.timer4,
            &mut self.timer5,
            &mut self.timer7,
            &mut self.timer6,
            &mut self.sfx_timer1,
            &mut self.sfx_timer2,
            &mut self.sfx_timer3,
            &mut self.timer8,
            &mut self.timer9,
        ] {
            // DECR.
            if *t != 0 {
                *t -= 1;
            }
        }

        self.actor.shape_rot = self.actor.world_rot;
        self.actor.shape_shadow_scale = 15.0;
        self.run_action(play);
        if self.actor.killed {
            return;
        }
        self.actor.set_focus(self.unk_304);
        self.actor.move_forward();

        if self.action != Action::Func80AB6EB4 && self.action != Action::Func80AB6450 && play.scene_id != SCENE_ZORAS_RIVER {
            self.actor.update_bg_check_info(&play.col, 20.0, 20.0, 60.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_1 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
        }
        if play.scene_id == SCENE_ZORAS_RIVER {
            self.actor.update_bg_check_info(&play.col, 20.0, 20.0, 60.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
        }
        if self.actor.floor_height <= BGCHECK_Y_MIN || self.actor.floor_height >= 32000.0 {
            log::debug!("En_Niw: up and down? {}", self.actor.floor_height);
            let cam = play.view.at - play.view.eye;
            let cam_result = cam.y / (cam.x * cam.x + cam.y * cam.y + cam.z * cam.z).sqrt();
            let a = &mut self.actor;
            a.world_pos.x = a.home_pos.x;
            a.world_pos.z = a.home_pos.z;
            a.world_pos.y = (a.home_pos.y + play.view.eye.y) + (cam_result * 160.0);
            if a.world_pos.y < a.home_pos.y {
                a.world_pos.y = a.home_pos.y + 300.0;
            }
            log::debug!("En_Niw: after correction {:?}", a.world_pos);
            a.speed_xz = 0.0;
            a.gravity = -2.0;
            self.unk_2ac = a.home_pos;
            self.unk_2b8 = a.home_pos;
            self.unk_300 = 0.0;
            self.unk_2fc = 0.0;
            self.unk_2f0 = Vec3::ZERO;
            self.unk_2d8 = 0.0;
            self.unk_2d4 = 0.0;
            self.unk_2d0 = 0.0;
            self.unk_2cc = 0.0;
            self.unk_2c8 = 0.0;
            self.unk_2c4 = 0.0;
            self.unk_2dc = 0.0;
            self.unk_2e0 = 0.0;
            self.unk_2a8 = 0;
            self.unk_294 = 0;
            self.unk_298 = 0;
            self.unk_2a6 = 0;
            self.unk_29e = 0;
            self.unk_2a0 = 0;
            self.unk_2a2 = 0;
            self.unk_26c = [0.0; 10];
            self.unk_2a8 = 0;
            self.action = Action::Func80AB7420;
            return;
        }

        let params = TYPE.get(self.actor.params);
        if self.actor.bg_check_flags & BGCHECKFLAG_WATER != 0 && self.actor.y_dist_to_water > 15.0 && self.action != Action::Func80AB6F04 && params != 0xD && params != 0xE && params != 0xA {
            self.actor.velocity.y = 0.0;
            self.actor.gravity = 0.0;
            self.timer4 = 30;
            // EffectSsGSplash_Spawn(play, &pos + depthInWater, NULL, NULL, 0, 400): the splash
            // isn't ported.
            self.timer5 = 0;
            log::debug!("En_Niw: bubbling");
            self.action = Action::Func80AB6F04;
            return;
        }

        if Self::statics(play).d_80ab85e0 == 0 && self.unk_2a4 <= 0 && params != 0xD && params != 0xE && params != 0xA {
            self.timer6 = 100;
            if self.actor.xz_dist_to_player > 10.0 {
                Self::statics(play).d_80ab85e0 = 1;
                self.timer5 = 0;
                self.timer4 = 0;
                self.unk_29e = 0;
                self.actor.speed_xz = 0.0;
                self.unk_2fc = 0.0;
                self.unk_300 = 0.0;
                self.unk_26c[7] = 0.0;
                self.unk_26c[5] = 0.0;
                self.unk_26c[6] = 0.0;
                self.unk_26c[8] = 0.0;
                self.sfx_timer1 = 10000;
                self.unk_2a8 = 1;
                self.unk_2ac = self.actor.world_pos;
                self.unk_2b8 = self.actor.world_pos;
                self.action = Action::Func80AB70A0;
                return;
            }
        }

        let dist = 20.0;
        // OOT_VERSION >= NTSC_1_1 (this ROM): Player's invincibilityTimer.
        let invincibility_timer = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map(|p| p.invincibility_timer()).unwrap_or(0);
        if self.unk_2a8 != 0 && self.actor.xyz_dist_to_player_sq < dist * dist && invincibility_timer == 0 {
            actor_set_player_knockback_large(play, 2.0, self.actor.world_rot.y, 0.0, 0x10);
        }

        self.func_80ab747c(play);

        if self.sfx_timer2 == 0 && self.action == Action::Func80AB6BF8 {
            self.sfx_timer2 = 7;
            audio_play_actor_sfx2(play, NA_SE_EN_DEKU_WAKEUP);
        }
        if self.sfx_timer1 == 0 {
            if self.action != Action::Func80AB6570 {
                self.sfx_timer1 = 30;
                audio_play_actor_sfx2(play, NA_SE_EV_CHICKEN_CRY_A);
            } else {
                self.sfx_timer1 = 300;
                audio_play_actor_sfx2(play, NA_SE_EV_CHICKEN_CRY_N);
            }
        }
        if self.unk_2a8 == 0 {
            self.collider.update(&self.actor);
            let params = TYPE.get(self.actor.params);
            if params != 0xA && params != 0xD && params != 0xE && params != 4 {
                play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
            }
            if !matches!(self.action, Action::Func80AB6BF8 | Action::Func80AB6D08 | Action::Func80AB6324 | Action::Func80AB63A8 | Action::Func80AB6450) {
                play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
            }
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.angles = self.turns().to_vec();
        rs.switches = vec![(self.action == Action::Func80AB6450) as u32];
        rs
    }
    /// `EnNiw_Draw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        if rs.angles.len() < 8 || rs.switches.is_empty() {
            return;
        }
        let a = &rs.angles;
        let bones = Self::pose(&self.skeleton, &joints.rot, [a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7]]);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE)), transform: oot_game::play::actor_draw_matrix(rs), bones, params: Default::default() });
        // func_80033C30 (0.15) while it waits to be picked up (func_80AB6450).
        if rs.switches[0] != 0 {
            out.opa.push(oot_game::shadow::func_80033c30(&play.col, rs.pos, Vec3::splat(0.15), 255));
        }
        // EnNiw_DrawEffects: Translate(pos), ReplaceRotation(billboard), Scale(scale, scale, 1),
        // RotateZ(unk_30), Translate(0, -1000, 0).
        for e in self.effects.iter().filter(|e| e.ty == 1) {
            let m = Mat4::from_translation(e.pos)
                * view.billboard
                * Mat4::from_scale(Vec3::new(e.scale, e.scale, 1.0))
                * Mat4::from_rotation_z(e.unk_30)
                * Mat4::from_translation(Vec3::new(0.0, -1000.0, 0.0));
            out.xlu.push(DrawCmd::new(MeshKey::named(keys::bake(FEATHER_BAKE)), m));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats_become_s16_through_s32() {
        // (s16)35000.0f: trunc.w.s to 35000, the low 16 bits.
        assert_eq!(f_to_s16(35000.0), 35000i32 as i16);
        assert_eq!(f_to_s16(-1.5), -1);
    }
}
