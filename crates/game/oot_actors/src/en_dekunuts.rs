//! `En_Dekunuts` (`ovl_En_Dekunuts/z_en_dekunuts.c`): the Mad Scrub, a Deku Scrub hiding in its
//! flower that pops up to spit Deku nuts (`En_Nutsball`) at Link; its own nut bounced back off his
//! shield knocks it out of the ground, and it runs; caught and hit, it dies.
//!
//! Params: the low byte 10 is the flower (`DEKUNUTS_FLOWER`, spawned by the scrub as its child,
//! drawn at its home); else the high byte is how many nuts it spits a round (`shotsPerRound`,
//! 0 and 0xFF count as 1). The Deku Tree's two are 0xFF00.
//!
//! - **Waiting** in the flower (`EnDekunuts_Wait`): the "up" animation held at speed 0 for
//!   100 to 150 frames (`Rand_S16Offset`), sped up to 1 with Link between 160 and 480 across and
//!   within 120 up or down; up, its collider grows from 5 (frame 9 on: `AC_ON`). Link within 120,
//!   it **burrows** (`EnDekunuts_Burrow`). Past 320 with its timer out, it **looks around**
//!   (`EnDekunuts_LookAround`, two loops); else it **stands** (`EnDekunuts_Stand`), turning to
//!   Link for a loop, then **spits** (`EnDekunuts_ThrowNut`): the nut at frame 6, 23 in front and
//!   12 up (`ACTOR_EN_NUTSBALL` 0), its nose swelling (`EnDekunuts_OverrideLimbDraw`); after a
//!   round it stands two loops and spits again, or burrows with Link past 480 or within 120.
//! - **Hit** (`EnDekunuts_ColliderCheck`) while in its flower (immovable): it pops out and runs
//!   (`EnDekunuts_BeginRun`, `EnDekunuts_Run`, mass 50): away from Link three times, gasping
//!   between (`EnDekunuts_Gasp`), then home to burrow. Out of the ground: a Deku Nut stuns it
//!   (`EnDekunuts_BeStunned`, 5 loops, blue); anything else that does damage (its table) knocks
//!   it back (`EnDekunuts_BeDamaged`, red) and it **dies** (`EnDekunuts_Die`): a white puff
//!   (`EffectSsDeadDb`), 15 fragments (`EffectSsHahen_SpawnBurst`), a drop from table 3, and its
//!   flower becomes a prop. Fire sets it in a ring of fire (`EffectSsFCircle_Spawn`).
//!
//! The whole overlay is ported. Not ported: the circle shadow (`ActorShadow_DrawCircle`, 35),
//! not ported for any actor.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{approach_f, approach_s, cos_s, sin_s, smooth_step_to_s, step_to_f, vec3f_yaw};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ACTORCAT_PROP, ActorHandle, ActorImpl, ActorProfile, audio_play_actor_sfx2, enemy_start_finishing_blow};
use oot_game::audio::sfx::*;
use oot_game::collision_check::*;
use oot_game::effect::hahen::HAHEN_OBJECT_DEFAULT;
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_ONCE, Anim, SkelAnimeStd};

/// `ACTOR_EN_DEKUNUTS` (`actor_table.h`).
pub const ACTOR_EN_DEKUNUTS: i16 = 0x0060;
const OBJECT: &str = "object_dekunuts";

/// `En_Dekunuts_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_DEKUNUTS, name: "En_Dekunuts", category: ACTORCAT_ENEMY, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE, object: OBJECT };

/// `DEKUNUTS_FLOWER`: the flower's params.
pub const DEKUNUTS_FLOWER: i16 = 10;

/// `NAVI_ENEMY_MAD_SCRUB` (`actor.h`).
const NAVI_ENEMY_MAD_SCRUB: u8 = 0x4D;
/// `COLLECTIBLE_DROP_TABLE_3` (`z_en_item00.h`).
const COLLECTIBLE_DROP_TABLE_3: i16 = 3;

/// `sCylinderInit`: hit by everything but the shield and the mirror's ray; 18 by 32.
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
    dim: Cylinder16 { radius: 18, height: 32, y_shift: 0, pos: [0, 0, 0] },
};

/// `sColChkInfoInit`: 1 health, 18 by 32, immovable.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit = CollisionCheckInfoInit { health: 1, cyl_radius: 18, cyl_height: 32, mass: MASS_IMMOVABLE };

/// `sDamageTable`.
pub static S_DAMAGE_TABLE: DamageTable = DamageTable {
    table: [
        dmg_entry(0, 0x1), // Deku nut
        dmg_entry(2, 0x0), // Deku stick
        dmg_entry(1, 0x0), // Slingshot
        dmg_entry(2, 0x0), // Explosive
        dmg_entry(1, 0x0), // Boomerang
        dmg_entry(2, 0x0), // Normal arrow
        dmg_entry(2, 0x0), // Hammer swing
        dmg_entry(2, 0x0), // Hookshot
        dmg_entry(1, 0x0), // Kokiri sword
        dmg_entry(2, 0x0), // Master sword
        dmg_entry(4, 0x0), // Giant's Knife
        dmg_entry(4, 0x2), // Fire arrow
        dmg_entry(2, 0x0), // Ice arrow
        dmg_entry(2, 0x0), // Light arrow
        dmg_entry(2, 0x0), // Unk arrow 1
        dmg_entry(2, 0x0), // Unk arrow 2
        dmg_entry(2, 0x0), // Unk arrow 3
        dmg_entry(4, 0x2), // Fire magic
        dmg_entry(0, 0x0), // Ice magic
        dmg_entry(0, 0x0), // Light magic
        dmg_entry(0, 0x0), // Shield
        dmg_entry(0, 0x0), // Mirror Ray
        dmg_entry(1, 0x0), // Kokiri spin
        dmg_entry(4, 0x0), // Giant spin
        dmg_entry(2, 0x0), // Master spin
        dmg_entry(2, 0x0), // Kokiri jump
        dmg_entry(8, 0x0), // Giant jump
        dmg_entry(4, 0x0), // Master jump
        dmg_entry(0, 0x0), // Unknown 1
        dmg_entry(0, 0x0), // Unblockable
        dmg_entry(4, 0x0), // Hammer jump
        dmg_entry(0, 0x0), // Unknown 2
    ],
};

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Wait,
    LookAround,
    Stand,
    ThrowNut,
    Burrow,
    BeginRun,
    Run,
    Gasp,
    BeDamaged,
    BeStunned,
    Die,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::Wait => "EnDekunuts_Wait",
            Action::LookAround => "EnDekunuts_LookAround",
            Action::Stand => "EnDekunuts_Stand",
            Action::ThrowNut => "EnDekunuts_ThrowNut",
            Action::Burrow => "EnDekunuts_Burrow",
            Action::BeginRun => "EnDekunuts_BeginRun",
            Action::Run => "EnDekunuts_Run",
            Action::Gasp => "EnDekunuts_Gasp",
            Action::BeDamaged => "EnDekunuts_BeDamaged",
            Action::BeStunned => "EnDekunuts_BeStunned",
            Action::Die => "EnDekunuts_Die",
        }
    }
}

/// The animations of `object_dekunuts` it plays.
struct Anims {
    up: Anim,
    look_around: Anim,
    spit: Anim,
    stand: Anim,
    burrow: Anim,
    unburrow: Anim,
    run: Anim,
    gasp: Anim,
    damage: Anim,
    die: Anim,
}

impl Anims {
    fn load(play: &PlayState) -> Option<Anims> {
        let a = play.assets.clone()?;
        let get = |s: &str| a.animation(OBJECT, s).map_err(|e| log::error!("En_Dekunuts: {e:#}")).ok();
        Some(Anims {
            up: get("gDekuNutsUpAnim")?,
            look_around: get("gDekuNutsLookAroundAnim")?,
            spit: get("gDekuNutsSpitAnim")?,
            stand: get("gDekuNutsStandAnim")?,
            burrow: get("gDekuNutsBurrowAnim")?,
            unburrow: get("gDekuNutsUnburrowAnim")?,
            run: get("gDekuNutsRunAnim")?,
            gasp: get("gDekuNutsGaspAnim")?,
            damage: get("gDekuNutsDamageAnim")?,
            die: get("gDekuNutsDieAnim")?,
        })
    }
}

/// `Math_Vec3f_DistXZ`.
fn dist_xz(a: Vec3, b: Vec3) -> f32 {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    (dx * dx + dz * dz).sqrt()
}

/// `Actor_ChangeCategory` of another actor (`this->actor.child`).
pub(crate) fn change_category(play: &mut PlayState, h: ActorHandle, category: usize) {
    if let Some(a) = play.actors.actor_mut(h) {
        a.category = category;
        play.actors.change_category(h, category);
    }
}

/// `EnDekunuts_OverrideLimbDraw`'s, `EnHintnuts_OverrideLimbDraw`'s and
/// `EnShopnuts_PostLimbDraw`'s swelling nose at `cur_frame` of the spit, as (across, along,
/// across): the nose shrinks along and swells across to frame 6, springs out at 7 and settles by
/// 10. `None` past frame 10.
pub(crate) fn spit_nose_scale(cur_frame: f32) -> Option<(f32, f32)> {
    let mut f = cur_frame;
    if f <= 6.0 {
        Some((1.0 + (f * 0.1167), 1.0 - (f * 0.0833)))
    } else if f <= 7.0 {
        f -= 6.0;
        Some((1.7 - (f * 0.7), 0.5 + f))
    } else if f <= 10.0 {
        Some((1.0, 1.5 - ((f - 7.0) * 0.1667)))
    } else {
        None
    }
}

pub struct EnDekunuts {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: Option<Anims>,
    pub action: Action,
    /// `playWalkSfx`.
    pub play_walk_sfx: bool,
    /// `runAwayCount`.
    pub run_away_count: u8,
    /// `animFlagAndTimer`: a timer counted in frames or loops, 0x1000 after a spit.
    pub anim_flag_and_timer: i16,
    /// `runDirection`.
    pub run_direction: i16,
    /// `shotsPerRound`.
    pub shots_per_round: i16,
    pub collider: ColliderCylinder,
}

impl EnDekunuts {
    /// `EnDekunuts_Init`: the flower untargetable; the scrub with its skeleton, collider and
    /// table, its shots from the high byte, waiting, and its flower spawned as its child.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: NAVI_ENEMY_MAD_SCRUB, gravity -1, lockOnArrowOffset 2600.
        actor.navi_enemy_id = NAVI_ENEMY_MAD_SCRUB;
        actor.gravity = -1.0;
        actor.target_arrow_offset = 2600.0;
        let collider = ColliderCylinder::new(&CYLINDER_INIT);
        if actor.params == DEKUNUTS_FLOWER {
            actor.flags &= !(ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE);
            return Box::new(EnDekunuts {
                actor,
                skel: SkelAnimeStd::init_flex(0, None),
                skeleton: None,
                anims: None,
                action: Action::Wait,
                play_walk_sfx: false,
                run_away_count: 0,
                anim_flag_and_timer: 0,
                run_direction: 0,
                shots_per_round: 0,
                collider,
            });
        }
        // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 35): the circle shadow isn't ported.
        actor.shape_y_offset = 0.0;
        let skeleton = play.assets.clone().and_then(|a| a.skeleton(OBJECT, "gDekuNutsSkel").map_err(|e| log::error!("En_Dekunuts: {e:#}")).ok());
        let anims = Anims::load(play);
        // SkelAnime_Init(&gDekuNutsSkel, &gDekuNutsStandAnim): 25 entries.
        let limbs = skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(24);
        let skel = SkelAnimeStd::init_flex(limbs, anims.as_ref().map(|a| a.stand.clone()));
        actor.col_chk_info.set_info(Some(&S_DAMAGE_TABLE), &COL_CHK_INFO_INIT);
        let mut shots_per_round = ((actor.params as u16 >> 8) & 0xFF) as i16;
        actor.params &= 0xFF;
        if shots_per_round == 0xFF || shots_per_round == 0 {
            shots_per_round = 1;
        }
        let mut n = EnDekunuts { actor, skel, skeleton, anims, action: Action::Wait, play_walk_sfx: false, run_away_count: 0, anim_flag_and_timer: 0, run_direction: 0, shots_per_round, collider };
        n.setup_wait(play);
        let (pos, rot_y) = (n.actor.world_pos, n.actor.world_rot.y);
        if let Err(e) = play.actor_spawn_as_child(&mut n.actor, ACTOR_EN_DEKUNUTS, pos, [0, rot_y, 0], DEKUNUTS_FLOWER) {
            log::debug!("En_Dekunuts: its flower: {e:?}");
        }
        Box::new(n)
    }

    fn is_flower(&self) -> bool {
        self.actor.params == DEKUNUTS_FLOWER
    }

    fn anim(&self, f: impl Fn(&Anims) -> &Anim) -> Option<Anim> {
        self.anims.as_ref().map(|a| f(a).clone())
    }

    /// `Animation_PlayOnceSetSpeed`, `Animation_PlayOnce`, `Animation_PlayLoop`,
    /// `Animation_MorphToLoop`, `Animation_MorphToPlayOnce`.
    fn play_once_set_speed(&mut self, a: Option<Anim>, speed: f32) {
        if let Some(a) = a {
            let last = a.last_frame();
            self.skel.change(a, speed, 0.0, last, ANIMMODE_ONCE, 0.0);
        }
    }
    fn play_loop(&mut self, a: Option<Anim>) {
        if let Some(a) = a {
            let last = a.last_frame();
            self.skel.change(a, 1.0, 0.0, last, ANIMMODE_LOOP, 0.0);
        }
    }
    fn morph_to_loop(&mut self, a: Option<Anim>, morph: f32) {
        if let Some(a) = a {
            self.skel.change(a, 1.0, 0.0, 0.0, ANIMMODE_LOOP, morph);
        }
    }
    fn morph_to_play_once(&mut self, a: Option<Anim>, morph: f32) {
        if let Some(a) = a {
            let last = a.last_frame();
            self.skel.change(a, 1.0, 0.0, last, ANIMMODE_ONCE, morph);
        }
    }

    /// `Animation_GetLastFrame(anim)`.
    fn last_frame(&self, f: impl Fn(&Anims) -> &Anim) -> f32 {
        self.anims.as_ref().map(|a| f(a).last_frame()).unwrap_or(0.0)
    }

    /// `EnDekunuts_SetupWait`: in the flower at home, "up" held at speed 0 for 100 to 150 frames,
    /// the collider 5 high and off.
    fn setup_wait(&mut self, play: &mut PlayState) {
        self.play_once_set_speed(self.anim(|a| &a.up), 0.0);
        self.anim_flag_and_timer = play.rand.s16_offset(100, 50);
        self.collider.dim.height = 5;
        self.actor.world_pos = self.actor.home_pos;
        self.collider.base.ac_flags &= !AC_ON;
        self.action = Action::Wait;
    }

    /// `EnDekunuts_SetupLookAround`: two loops.
    fn setup_look_around(&mut self) {
        self.play_loop(self.anim(|a| &a.look_around));
        self.anim_flag_and_timer = 2;
        self.action = Action::LookAround;
    }

    /// `EnDekunuts_SetupThrowNut`: a round of `shotsPerRound`.
    fn setup_throw_nut(&mut self) {
        let a = self.anim(|a| &a.spit);
        self.play_once_set_speed(a, 1.0);
        self.anim_flag_and_timer = self.shots_per_round;
        self.action = Action::ThrowNut;
    }

    /// `EnDekunuts_SetupStand`: after a spit two loops and the flag (no turning), else one.
    fn setup_stand(&mut self) {
        self.morph_to_loop(self.anim(|a| &a.stand), -3.0);
        self.anim_flag_and_timer = if self.action == Action::ThrowNut { 2 | 0x1000 } else { 1 };
        self.action = Action::Stand;
    }

    /// `EnDekunuts_SetupBurrow`.
    fn setup_burrow(&mut self, play: &mut PlayState) {
        self.morph_to_play_once(self.anim(|a| &a.burrow), -5.0);
        audio_play_actor_sfx2(play, NA_SE_EN_NUTS_DOWN);
        self.action = Action::Burrow;
    }

    /// `EnDekunuts_SetupBeginRun`: popping out, 37 high, mass 50, the collider off.
    fn setup_begin_run(&mut self, play: &mut PlayState) {
        self.morph_to_play_once(self.anim(|a| &a.unburrow), -3.0);
        self.collider.dim.height = 37;
        self.actor.col_chk_info.mass = 50;
        audio_play_actor_sfx2(play, NA_SE_EN_NUTS_DAMAGE);
        self.collider.base.ac_flags &= !AC_ON;
        self.action = Action::BeginRun;
    }

    /// `EnDekunuts_SetupRun`: two loops, the collider on.
    fn setup_run(&mut self) {
        self.play_loop(self.anim(|a| &a.run));
        self.anim_flag_and_timer = 2;
        self.play_walk_sfx = false;
        self.collider.base.ac_flags |= AC_ON;
        self.action = Action::Run;
    }

    /// `EnDekunuts_SetupGasp`: three loops, one run fewer to go.
    fn setup_gasp(&mut self) {
        self.play_loop(self.anim(|a| &a.gasp));
        self.anim_flag_and_timer = 3;
        self.actor.speed_xz = 0.0;
        if self.run_away_count != 0 {
            self.run_away_count -= 1;
        }
        self.action = Action::Gasp;
    }

    /// `EnDekunuts_SetupBeDamaged`: knocked back at 10 the way an arrow or a seed flew, else away
    /// from what hit it; red for the damage animation.
    fn setup_be_damaged(&mut self, play: &mut PlayState) {
        self.morph_to_play_once(self.anim(|a| &a.damage), -3.0);
        let dmg_flags = self.collider.info.ac_hit_elem.map(|h| h.at_dmg_info.dmg_flags).unwrap_or(0);
        let ac = self.collider.base.ac.and_then(|h| play.actors.actor(h));
        if let Some(ac) = ac {
            if dmg_flags & (DMG_ARROW | DMG_SLINGSHOT) != 0 {
                self.actor.world_rot.y = ac.world_rot.y;
            } else {
                // Actor_WorldYawTowardActor(&this->actor, this->collider.base.ac) + 0x8000.
                self.actor.world_rot.y = vec3f_yaw(self.actor.world_pos, ac.world_pos).wrapping_add(i16::MIN);
            }
        }
        self.collider.base.ac_flags &= !AC_ON;
        self.action = Action::BeDamaged;
        self.actor.speed_xz = 10.0;
        audio_play_actor_sfx2(play, NA_SE_EN_NUTS_DAMAGE);
        audio_play_actor_sfx2(play, NA_SE_EN_NUTS_CUTBODY);
        let d = self.last_frame(|a| &a.damage) as i16 as u16;
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, d);
    }

    /// `EnDekunuts_SetupBeStunned`: five loops of the damage animation, blue.
    fn setup_be_stunned(&mut self, play: &mut PlayState) {
        self.morph_to_loop(self.anim(|a| &a.damage), -3.0);
        self.anim_flag_and_timer = 5;
        self.action = Action::BeStunned;
        self.actor.speed_xz = 0.0;
        audio_play_actor_sfx2(play, NA_SE_EN_GOMA_JR_FREEZE);
        let d = (self.last_frame(|a| &a.damage) as i16 as i32 * self.anim_flag_and_timer as i32) as u16;
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 255, COLORFILTER_BUFFLAG_OPA, d);
    }

    /// `EnDekunuts_SetupDie`.
    fn setup_die(&mut self, play: &mut PlayState) {
        let a = self.anim(|a| &a.die);
        self.play_once_set_speed(a, 1.0);
        self.action = Action::Die;
        self.actor.speed_xz = 0.0;
        audio_play_actor_sfx2(play, NA_SE_EN_NUTS_DEAD);
    }

    /// `EnDekunuts_Wait`.
    fn wait(&mut self, play: &mut PlayState) {
        let has_slow_playback_speed = self.skel.play_speed < 0.5;
        if has_slow_playback_speed && self.anim_flag_and_timer != 0 {
            self.anim_flag_and_timer -= 1;
        }
        if self.skel.on_frame(9.0) {
            self.collider.base.ac_flags |= AC_ON;
        } else if self.skel.on_frame(8.0) {
            audio_play_actor_sfx2(play, NA_SE_EN_NUTS_UP);
        }
        self.collider.dim.height = (((self.skel.cur_frame.clamp(9.0, 12.0) - 9.0) * 9.0) + 5.0) as i16;
        if !has_slow_playback_speed && self.actor.xz_dist_to_player < 120.0 {
            self.setup_burrow(play);
        } else if self.skel.update() {
            if self.actor.xz_dist_to_player < 120.0 {
                self.setup_burrow(play);
            } else if self.anim_flag_and_timer == 0 && self.actor.xz_dist_to_player > 320.0 {
                self.setup_look_around();
            } else {
                self.setup_stand();
            }
        }
        if has_slow_playback_speed && (self.actor.xz_dist_to_player > 160.0 && self.actor.y_dist_to_player.abs() < 120.0) && (self.anim_flag_and_timer == 0 || self.actor.xz_dist_to_player < 480.0) {
            self.skel.play_speed = 1.0;
        }
    }

    /// `EnDekunuts_LookAround`.
    fn look_around(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.skel.on_frame(0.0) && self.anim_flag_and_timer != 0 {
            self.anim_flag_and_timer -= 1;
        }
        if self.actor.xz_dist_to_player < 120.0 || self.anim_flag_and_timer == 0 {
            self.setup_burrow(play);
        }
    }

    /// `EnDekunuts_Stand`: turning to Link (unless just after a spit); its loops done, it
    /// spits, or after a round burrows with Link past 480 or within 120.
    fn stand(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.skel.on_frame(0.0) && self.anim_flag_and_timer != 0 {
            self.anim_flag_and_timer -= 1;
        }
        if self.anim_flag_and_timer & 0x1000 == 0 {
            approach_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 2, 0xE38);
        }
        if self.anim_flag_and_timer == 0x1000 {
            if self.actor.xz_dist_to_player > 480.0 || self.actor.xz_dist_to_player < 120.0 {
                self.setup_burrow(play);
            } else {
                self.setup_throw_nut();
            }
        } else if self.anim_flag_and_timer == 0 {
            self.setup_throw_nut();
        }
    }

    /// `EnDekunuts_ThrowNut`: turning to Link; the nut at frame 6 (`NA_SE_EN_NUTS_THROW` if it
    /// spawned); with more of the round to go, the spit again from frame 12.
    fn throw_nut(&mut self, play: &mut PlayState) {
        approach_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 2, 0xE38);
        if self.skel.update() {
            self.setup_stand();
        } else if self.skel.on_frame(6.0) {
            let r = self.actor.shape_rot;
            let spawn_pos = Vec3::new(self.actor.world_pos.x + (sin_s(r.y) * 23.0), self.actor.world_pos.y + 12.0, self.actor.world_pos.z + (cos_s(r.y) * 23.0));
            if play.actor_spawn(crate::en_nutsball::ACTOR_EN_NUTSBALL, spawn_pos, [r.x, r.y, r.z], crate::en_nutsball::EN_NUTSBALL_TYPE_DEKUNUTS).is_ok() {
                audio_play_actor_sfx2(play, NA_SE_EN_NUTS_THROW);
            }
        } else if self.anim_flag_and_timer > 1 && self.skel.on_frame(12.0) {
            self.morph_to_play_once(self.anim(|a| &a.spit), -3.0);
            if self.anim_flag_and_timer != 0 {
                self.anim_flag_and_timer -= 1;
            }
        }
    }

    /// `EnDekunuts_Burrow`: the collider shrinking from 29 to 5 over frames 1 to 3 (off at 4),
    /// back home; done, waiting.
    fn burrow(&mut self, play: &mut PlayState) {
        if self.skel.update() {
            self.setup_wait(play);
        } else {
            self.collider.dim.height = (((3.0 - self.skel.cur_frame.clamp(1.0, 3.0)) * 12.0) + 5.0) as i16;
        }
        if self.skel.on_frame(4.0) {
            self.collider.base.ac_flags &= !AC_ON;
        }
        approach_f(&mut self.actor.world_pos.x, self.actor.home_pos.x, 0.5, 3.0);
        approach_f(&mut self.actor.world_pos.z, self.actor.home_pos.z, 0.5, 3.0);
    }

    /// `EnDekunuts_BeginRun`: out of the ground, turning to Link; then off away from him, three
    /// runs to go.
    fn begin_run(&mut self) {
        if self.skel.update() {
            self.run_direction = self.actor.yaw_towards_player.wrapping_add(i16::MIN);
            self.run_away_count = 3;
            self.setup_run();
        }
        approach_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 2, 0xE38);
    }

    /// `EnDekunuts_Run`: up to 7.5, a step every other frame; once turned its way, a new way:
    /// home out of water, along a wall it met, away from Link while it has runs to go, else home
    /// (or 45° off Link's way when home is towards him); home and level, it burrows; its two
    /// loops done, it gasps. It faces back the way it runs.
    fn run(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.skel.on_frame(0.0) && self.anim_flag_and_timer != 0 {
            self.anim_flag_and_timer -= 1;
        }
        if self.play_walk_sfx {
            audio_play_actor_sfx2(play, NA_SE_EN_NUTS_WALK);
            self.play_walk_sfx = false;
        } else {
            self.play_walk_sfx = true;
        }
        step_to_f(&mut self.actor.speed_xz, 7.5, 1.0);
        if smooth_step_to_s(&mut self.actor.world_rot.y, self.run_direction, 1, 0xE38, 0xB6) == 0 {
            if self.actor.bg_check_flags & BGCHECKFLAG_WATER != 0 {
                self.run_direction = vec3f_yaw(self.actor.world_pos, self.actor.home_pos);
            } else if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
                self.run_direction = self.actor.wall_yaw;
            } else if self.run_away_count == 0 {
                let diff_rot_init = vec3f_yaw(self.actor.world_pos, self.actor.home_pos);
                let diff_rot = diff_rot_init.wrapping_sub(self.actor.yaw_towards_player);
                if (diff_rot as i32).abs() > 0x2000 {
                    self.run_direction = diff_rot_init;
                } else {
                    let phi_f0 = if diff_rot as f32 >= 0.0 { 1.0 } else { -1.0 };
                    // (phi_f0 * -0x2000) + yawTowardsPlayer, as an f32 truncated to s16.
                    self.run_direction = ((phi_f0 * -8192.0) + self.actor.yaw_towards_player as f32) as i32 as i16;
                }
            } else {
                self.run_direction = self.actor.yaw_towards_player.wrapping_add(i16::MIN);
            }
        }
        self.actor.shape_rot.y = self.actor.world_rot.y.wrapping_add(i16::MIN);
        if self.run_away_count == 0 && dist_xz(self.actor.world_pos, self.actor.home_pos) < 20.0 && (self.actor.world_pos.y - self.actor.home_pos.y).abs() < 2.0 {
            self.actor.col_chk_info.mass = MASS_IMMOVABLE;
            self.actor.speed_xz = 0.0;
            self.setup_burrow(play);
        } else if self.anim_flag_and_timer == 0 {
            self.setup_gasp();
        }
    }

    /// `EnDekunuts_Gasp`: three loops, then running again.
    fn gasp(&mut self) {
        self.skel.update();
        if self.skel.on_frame(0.0) && self.anim_flag_and_timer != 0 {
            self.anim_flag_and_timer -= 1;
        }
        if self.anim_flag_and_timer == 0 {
            self.setup_run();
        }
    }

    /// `EnDekunuts_BeDamaged`: slowing by 1 a frame; then it dies.
    fn be_damaged(&mut self, play: &mut PlayState) {
        step_to_f(&mut self.actor.speed_xz, 0.0, 1.0);
        if self.skel.update() {
            self.setup_die(play);
        }
    }

    /// `EnDekunuts_BeStunned`: `NA_SE_EN_NUTS_FAINT` each loop; its loops done, it runs.
    fn be_stunned(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.skel.on_frame(0.0) {
            if self.anim_flag_and_timer != 0 {
                self.anim_flag_and_timer -= 1;
            }
            if self.anim_flag_and_timer == 0 {
                self.setup_run();
            } else {
                audio_play_actor_sfx2(play, NA_SE_EN_NUTS_FAINT);
            }
        }
    }

    /// `EnDekunuts_Die`: at the animation's end, a white puff 18 up (`EffectSsDeadDb`, 13
    /// frames, its sound), 15 fragments 10 up, a drop from table 3; its flower a prop; gone.
    fn die(&mut self, play: &mut PlayState) {
        // effectVelAndAccel.
        let effect_vel_and_accel = Vec3::ZERO;
        if self.skel.update() {
            let mut effect_pos = Vec3::new(self.actor.world_pos.x, self.actor.world_pos.y + 18.0, self.actor.world_pos.z);
            play.with_ss(|s| s.dead_db_spawn(effect_pos, effect_vel_and_accel, effect_vel_and_accel, 200, 0, [255, 255, 255, 255], [150, 150, 150], 1, 13, 1));
            effect_pos.y = self.actor.world_pos.y + 10.0;
            play.with_ss(|s| s.hahen_spawn_burst(effect_pos, 3.0, 0, 12, 3, 15, HAHEN_OBJECT_DEFAULT, 10, None));
            let pos = self.actor.world_pos;
            crate::en_item00::item_drop_collectible_random(play, Some(&self.actor), pos, COLLECTIBLE_DROP_TABLE_3 * 16);
            if let Some(c) = self.actor.child {
                change_category(play, c, ACTORCAT_PROP);
            }
            self.actor.kill();
        }
    }

    /// `EnDekunuts_ColliderCheck`: hit (its drop flag set): out of the ground (mass 50), a hit
    /// with a reaction or damage stuns it (a Deku Nut) or damages it (fire: the ring of fire),
    /// the finishing blow at no health; in the ground, it pops out and runs. The hammer's shock
    /// wave (`actorCtx.unk_02`) pops it out too.
    fn collider_check(&mut self, play: &mut PlayState) {
        if self.collider.base.ac_flags & AC_HIT != 0 {
            self.collider.base.ac_flags &= !AC_HIT;
            let elem = self.collider.info;
            self.actor.set_drop_flag(&elem, true);
            if self.actor.col_chk_info.mass == 50 {
                let reaction = self.actor.col_chk_info.damage_reaction;
                if reaction != 0 || self.actor.col_chk_info.damage != 0 {
                    if reaction != 1 {
                        if reaction == 2 {
                            let r = self.actor.shape_rot;
                            if let Some(me) = play.cur_actor {
                                let a = oot_game::effect::SsActor { handle: me, world_pos: self.actor.world_pos, shape_rot: [r.x, r.y, r.z] };
                                let pos = self.actor.world_pos;
                                play.with_ss(|s| s.fcircle_spawn(a, pos, 40, 50));
                            }
                        }
                        self.setup_be_damaged(play);
                        if self.actor.apply_damage() == 0 {
                            enemy_start_finishing_blow(play, &self.actor);
                        }
                    } else if self.action != Action::BeStunned {
                        self.setup_be_stunned(play);
                    }
                }
            } else {
                self.setup_begin_run(play);
            }
        } else if self.actor.col_chk_info.mass == MASS_IMMOVABLE && play.actors.unk_02 != 0 {
            self.setup_begin_run(play);
        }
    }
}

impl ActorImpl for EnDekunuts {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnDekunuts_Update` (nothing for the flower).
    fn update(&mut self, play: &mut PlayState) {
        if self.is_flower() {
            return;
        }
        self.collider_check(play);
        match self.action {
            Action::Wait => self.wait(play),
            Action::LookAround => self.look_around(play),
            Action::Stand => self.stand(play),
            Action::ThrowNut => self.throw_nut(play),
            Action::Burrow => self.burrow(play),
            Action::BeginRun => self.begin_run(),
            Action::Run => self.run(play),
            Action::Gasp => self.gasp(),
            Action::BeDamaged => self.be_damaged(play),
            Action::BeStunned => self.be_stunned(play),
            Action::Die => self.die(play),
        }
        self.actor.move_forward();
        let (r, h) = (self.collider.dim.radius as f32, self.collider.dim.height as f32);
        self.actor.update_bg_check_info(&play.col, 20.0, r, h, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
        self.collider.update(&self.actor);
        if self.collider.base.ac_flags & AC_ON != 0 {
            play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
        }
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        if self.action == Action::Wait {
            self.actor.set_focus(self.skel.cur_frame);
        } else if self.action == Action::Burrow {
            self.actor.set_focus(20.0 - ((self.skel.cur_frame * 20.0) / self.last_frame(|a| &a.burrow)));
        } else {
            self.actor.set_focus(20.0);
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.switches = vec![self.is_flower() as u32, (self.action == Action::ThrowNut) as u32];
        rs.values = vec![self.skel.cur_frame];
        rs
    }

    /// `EnDekunuts_Draw`: the flower (`gDekuNutsFlowerDL`), or the skeleton
    /// (`SkelAnime_DrawOpa`), its nose (limb 7) swelling while it spits
    /// (`EnDekunuts_OverrideLimbDraw`: across on x, along on y and z).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let ([flower, throwing], [cur_frame]) = (rs.switches.as_slice(), rs.values.as_slice()) else { return };
        if *flower != 0 {
            crate::gfx_draw_dlist_opa(out, OBJECT, "gDekuNutsFlowerDL", rs);
            return;
        }
        let (Some(joints), Some(skeleton)) = (&rs.joints, &self.skeleton) else { return };
        let nose = if *throwing != 0 { spit_nose_scale(*cur_frame) } else { None };
        let bones = skeleton.pose_override(&joints.rot, |limb, _pos, _rot| match nose {
            // EnDekunuts: x = 1 - f × 0.0833, y = z = 1 + f × 0.1167 (to frame 6).
            Some((across, along)) if limb == 7 => Mat4::from_scale(Vec3::new(along, across, across)),
            _ => Mat4::IDENTITY,
        });
        let model = oot_game::play::actor_draw_matrix(rs);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::mesh(OBJECT, "gDekuNutsSkel")), transform: model, bones, params: Default::default() });
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
