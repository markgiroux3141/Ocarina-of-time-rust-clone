//! The effects: `z_effect_soft_sprite.c` (the soft sprites, `EffectSs`: a table of 85 small
//! effects each run by its overlay's init, update and draw) and `z_effect.c` (the three other
//! kinds, `Effect`: sparks, the sword's trails and the shield's particles), with the spawn
//! helpers of `z_effect_soft_sprite_old_init.c` (docs/adr/0033-effects.md).
//!
//! ## The soft sprites
//!
//! `EffectSsInfo` is `sEffectSsInfo`: the table `Play_Init` makes (`EffectSs_InitInfo(this,
//! 0x55)`), where `EffectSs_Spawn` finds a slot (`EffectSs_FindSlot`: a free one from
//! `searchStartIndex` round, else one of lower priority) and runs the type's init.
//! `EffectSs_UpdateAll` (in `Play_Update`, after the actors and the cutscenes) counts each
//! one's life down, deletes it at -1, else adds its acceleration and velocity and runs its
//! update; `EffectSs_DrawAll` (at the end of `Actor_DrawAll`) draws each live one, deleting one
//! that wandered past ±32000.
//!
//! An effect's `update` and `draw` are the overlay's functions, here `SsUpdate` and `SsDraw`;
//! its `regs` are as the overlay names them. The ported overlays are the ones the Deku Tree's
//! first enemies, the Deku Baba and Player's burning and shock call: `Effect_Ss_Dust`,
//! `Effect_Ss_Hahen`, `Effect_Ss_HitMark`, `Effect_Ss_En_Fire`, `Effect_Ss_En_Ice`,
//! `Effect_Ss_Dead_Db`, `Effect_Ss_Fire_Tail` and `Effect_Ss_Fhg_Flash` (its shock; the light
//! ball needs `object_fhg`); and for milestone 3b's enemies, `Effect_Ss_Fcircle` (a Mad Scrub
//! set alight), `Effect_Ss_Blast` (the Skulltula's landing), and the Gohma larvae's
//! `Effect_Ss_K_Fire` and `Effect_Ss_Sibuki`; for milestone 4b's props, `Effect_Ss_Kakera` (the
//! fragments of crates, bushes, rocks and the falling platform) and the Deku Stick's
//! `Effect_Ss_Stick` (its broken half). Spawning another type logs it
//! and does nothing, as the C does for a type with no init.
//!
//! ## `z_effect.c`
//!
//! `EffectContext`'s sparks (`EffectSpark`: the hit's blood) and shield particles
//! (`EffectShieldParticle`: a blocked hit's streaks of light, with their point light) are
//! ported; the sword's trails (`EffectBlure`) aren't: `Effect_Add` of one finds no slot.
//!
//! ## Drawing
//!
//! The draws run where `Actor_DrawAll` runs them, once per game frame (`PlayState::tick_with`,
//! after the actors' draw-time state), since some make `Rand` calls (a spark's sizes) or
//! change the effect (`Effect_Ss_Fire_Tail` follows Link's body parts). Their `DrawCmd`s are
//! kept (`PlayState::effect_draws`) and appended after the actors' by `PlayState::draw`. Each
//! texture and setup an overlay draws with is a bake (`bakes()`); the colours are dynamic
//! segment values.

pub mod blast;
pub mod dead_db;
pub mod dust;
pub mod en_fire;
pub mod en_ice;
pub mod fcircle;
pub mod fhg_flash;
pub mod fire_tail;
pub mod hahen;
pub mod hitmark;
pub mod k_fire;
pub mod kakera;
pub mod shield_particle;
pub mod sibuki;
pub mod spark;
pub mod stick;

use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use glam::{Mat4, Vec3};

use crate::actor_ctx::ActorHandle;
use crate::audio::sfx::SfxPos;
use crate::object_ctx::ObjectContext;
use crate::pack::{MeshBake, keys};
use crate::play::{PlayState, Rand};

// EffectSsType (`tables/effect_ss_table.h`): the ported ones, and the table's end.
pub const EFFECT_SS_DUST: u8 = 0x00;
pub const EFFECT_SS_BLAST: u8 = 0x04;
pub const EFFECT_SS_HAHEN: u8 = 0x0F;
pub const EFFECT_SS_STICK: u8 = 0x10;
pub const EFFECT_SS_SIBUKI: u8 = 0x11;
pub const EFFECT_SS_HITMARK: u8 = 0x15;
pub const EFFECT_SS_FHG_FLASH: u8 = 0x16;
pub const EFFECT_SS_K_FIRE: u8 = 0x17;
pub const EFFECT_SS_KAKERA: u8 = 0x19;
pub const EFFECT_SS_EN_ICE: u8 = 0x1B;
pub const EFFECT_SS_FIRE_TAIL: u8 = 0x1C;
pub const EFFECT_SS_EN_FIRE: u8 = 0x1D;
pub const EFFECT_SS_FCIRCLE: u8 = 0x1F;
pub const EFFECT_SS_DEAD_DB: u8 = 0x20;
pub const EFFECT_SS_TYPE_MAX: u8 = 0x25;

/// `EffectSs_InitInfo(this, 0x55)` (`Play_Init`): the table's size.
pub const EFFECT_SS_TABLE_SIZE: usize = 0x55;

/// An effect's update (`EffectSs.update`): the overlay's function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SsUpdate {
    /// `EffectSsDust_Update`.
    Dust,
    /// `EffectSsDust_UpdateFire` (unused in the game).
    DustFire,
    /// `EffectSsBlast_Update`.
    Blast,
    /// `EffectSsHahen_Update`.
    Hahen,
    /// `EffectSsHitMark_Update`.
    HitMark,
    /// `EffectSsEnFire_Update`.
    EnFire,
    /// `EffectSsEnIce_Update`.
    EnIce,
    /// `EffectSsEnIce_UpdateFlying`.
    EnIceFlying,
    /// `EffectSsDeadDb_Update`.
    DeadDb,
    /// `EffectSsFireTail_Update`.
    FireTail,
    /// `EffectSsFhgFlash_UpdateShock`.
    FhgFlashShock,
    /// `EffectSsFhgFlash_UpdateLightBall`.
    FhgFlashLightBall,
    /// `EffectSsFcircle_Update`.
    Fcircle,
    /// `EffectSsKFire_Update`.
    KFire,
    /// `EffectSsSibuki_Update`.
    Sibuki,
    /// `EffectSsStick_Update`.
    Stick,
    /// `EffectSsKakera_Update`.
    Kakera,
}

/// An effect's draw (`EffectSs.draw`): the overlay's function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SsDraw {
    /// `EffectSsDust_Draw`.
    Dust,
    /// `EffectSsBlast_Draw`.
    Blast,
    /// `EffectSsHahen_Draw`.
    Hahen,
    /// `EffectSsHahen_DrawGray` (the Shadow Temple's skull pots: drawn as `Hahen`'s, logged).
    HahenGray,
    /// `EffectSsHitMark_Draw`.
    HitMark,
    /// `EffectSsEnFire_Draw`.
    EnFire,
    /// `EffectSsEnIce_Draw`.
    EnIce,
    /// `EffectSsDeadDb_Draw`.
    DeadDb,
    /// `EffectSsFireTail_Draw`.
    FireTail,
    /// `EffectSsFhgFlash_DrawShock`.
    FhgFlashShock,
    /// `EffectSsFcircle_Draw`.
    Fcircle,
    /// `EffectSsKFire_Draw`.
    KFire,
    /// `EffectSsSibuki_Draw`.
    Sibuki,
    /// `EffectSsStick_Draw`.
    Stick,
    /// `EffectSsKakera_Draw`.
    Kakera,
}

/// `EffectSs.gfx` where it's a display list from an object (`EffectSsHahen_Spawn`'s `dList`):
/// the file and the symbol.
pub type SsGfx = Option<(&'static str, &'static str)>;

/// `EffectSs`: one slot of the table.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectSs {
    pub pos: Vec3,
    pub velocity: Vec3,
    pub accel: Vec3,
    pub update: Option<SsUpdate>,
    pub draw: Option<SsDraw>,
    /// `vec`: as each overlay uses it.
    pub vec: Vec3,
    pub gfx: SsGfx,
    /// `actor`: the actor the effect follows, if any.
    pub actor: Option<ActorHandle>,
    /// `regs[13]`: as each overlay names them.
    pub regs: [i16; 13],
    /// `flags`: 1 equal priority counts as lower, 2 and 4 the sounds at `pos` and `vec` stop
    /// with it.
    pub flags: u16,
    /// `life`: -1 for a free slot.
    pub life: i16,
    /// `priority`: lower is more important.
    pub priority: u8,
    /// `type` (`EFFECT_SS_*`).
    pub ty: u8,
}

impl Default for EffectSs {
    /// `EffectSs_Reset`.
    fn default() -> EffectSs {
        EffectSs {
            pos: Vec3::ZERO,
            velocity: Vec3::ZERO,
            accel: Vec3::ZERO,
            update: None,
            draw: None,
            vec: Vec3::ZERO,
            gfx: None,
            actor: None,
            regs: [0; 13],
            flags: 0,
            life: -1,
            priority: 128,
            ty: EFFECT_SS_TYPE_MAX,
        }
    }
}

/// `sEffectSsInfo`.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectSsInfo {
    /// `table` (`data_table`).
    pub table: Vec<EffectSs>,
    pub search_start_index: i32,
}

impl Default for EffectSsInfo {
    /// `EffectSs_InitInfo(play, 0x55)`.
    fn default() -> EffectSsInfo {
        EffectSsInfo { table: vec![EffectSs::default(); EFFECT_SS_TABLE_SIZE], search_start_index: 0 }
    }
}

/// What an effect's init reads besides its parameters: the game's random numbers, the objects
/// loaded (`Object_GetSlot`), and where to stop sounds (`EffectSs_Delete`'s, run by whoever
/// holds the audio).
pub struct SsSpawn<'a> {
    pub info: &'a mut EffectSsInfo,
    pub rand: &'a mut Rand,
    pub objects: Option<&'a ObjectContext>,
    /// `Audio_StopSfxByPos` calls an `EffectSs_Delete` of a slot being reused asked for.
    pub stops: &'a mut Vec<SfxPos>,
}

/// An effect type's init parameters (`EffectSs*InitParams`).
#[derive(Debug, Clone, PartialEq)]
pub enum SsInit {
    Dust(dust::DustInit),
    Blast(blast::BlastInit),
    Hahen(hahen::HahenInit),
    HitMark(hitmark::HitMarkInit),
    EnFire(en_fire::EnFireInit),
    EnIce(en_ice::EnIceInit),
    DeadDb(dead_db::DeadDbInit),
    FireTail(fire_tail::FireTailInit),
    FhgFlash(fhg_flash::FhgFlashInit),
    Fcircle(fcircle::FcircleInit),
    KFire(k_fire::KFireInit),
    Sibuki(sibuki::SibukiInit),
    Stick(stick::StickInit),
    Kakera(kakera::KakeraInit),
}

/// An actor an effect is spawned for (`initParams->actor`), as its init reads it: its handle,
/// `world.pos` and `shape.rot`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SsActor {
    pub handle: ActorHandle,
    pub world_pos: Vec3,
    pub shape_rot: [i16; 3],
}

/// `Audio_StopSfxByPos` for a deleted effect's sounds (`flags` 2 at `pos`, 4 at `vec`).
fn delete_stops(e: &EffectSs, index: usize, stops: &mut Vec<SfxPos>) {
    if e.flags & 2 != 0 {
        stops.push(SfxPos::EffectSsPos(index as u8));
    }
    if e.flags & 4 != 0 {
        stops.push(SfxPos::EffectSsVec(index as u8));
    }
}

impl EffectSsInfo {
    /// `EffectSs_FindSlot`: a free slot from `searchStartIndex` round the table; else the first
    /// from there whose priority is lower than `priority` (a higher number; an equal one with
    /// flag 1). `None` when there's none.
    pub fn find_slot(&mut self, priority: i32) -> Option<usize> {
        let size = self.table.len() as i32;
        if self.search_start_index >= size {
            self.search_start_index = 0;
        }
        let start = self.search_start_index;
        let mut i = start;
        loop {
            if self.table[i as usize].life == -1 {
                return Some(i as usize);
            }
            i += 1;
            if i >= size {
                i = 0;
            }
            if i == start {
                break;
            }
        }
        i = start;
        loop {
            let e = &self.table[i as usize];
            if priority <= e.priority as i32 && !(priority == e.priority as i32 && e.flags & 1 != 0) {
                return Some(i as usize);
            }
            i += 1;
            if i >= size {
                i = 0;
            }
            if i == start {
                return None;
            }
        }
    }
}

impl SsSpawn<'_> {
    /// `EffectSs_Spawn`: a slot for `ty` at `priority`, the slot's last effect deleted, and the
    /// type's init; a failed init resets the slot.
    pub fn spawn(&mut self, ty: u8, priority: i32, init: SsInit) {
        let Some(index) = self.info.find_slot(priority) else {
            // Abort: no suitable slot.
            return;
        };
        self.info.search_start_index = index as i32 + 1;
        // EffectSs_Delete(&sEffectSsInfo.table[index]).
        let old = std::mem::take(&mut self.info.table[index]);
        delete_stops(&old, index, self.stops);
        let mut e = EffectSs { ty, priority: priority as u8, ..EffectSs::default() };
        let ok = match init {
            SsInit::Dust(p) => dust::init(self, &mut e, &p),
            SsInit::Blast(p) => blast::init(&mut e, &p),
            SsInit::Hahen(p) => hahen::init(self, &mut e, &p),
            SsInit::HitMark(p) => hitmark::init(&mut e, &p),
            SsInit::EnFire(p) => en_fire::init(self, &mut e, &p),
            SsInit::EnIce(p) => en_ice::init(self, &mut e, &p),
            SsInit::DeadDb(p) => dead_db::init(&mut e, &p),
            SsInit::FireTail(p) => fire_tail::init(&mut e, &p),
            SsInit::FhgFlash(p) => fhg_flash::init(self, &mut e, &p),
            SsInit::Fcircle(p) => fcircle::init(&mut e, &p),
            SsInit::KFire(p) => k_fire::init(self, &mut e, &p),
            SsInit::Sibuki(p) => sibuki::init(self, &mut e, &p),
            SsInit::Stick(p) => stick::init(self, &mut e, &p),
            SsInit::Kakera(p) => kakera::init(self, &mut e, &p),
        };
        // "Construction failed for some reason": EffectSs_Reset.
        self.info.table[index] = if ok { e } else { EffectSs::default() };
    }

    /// `EffectSsDust_Spawn`.
    #[allow(clippy::too_many_arguments)]
    pub fn dust_spawn(&mut self, draw_flags: u16, pos: Vec3, velocity: Vec3, accel: Vec3, prim: [u8; 4], env: [u8; 4], scale: i16, scale_step: i16, life: i16, update_mode: u8) {
        let p = dust::DustInit { pos, velocity, accel, prim_color: prim, env_color: env, scale, scale_step, life, draw_flags, update_mode };
        self.spawn(EFFECT_SS_DUST, 128, SsInit::Dust(p));
    }

    /// `func_8002829C`: dust, draw flags 0, 10 frames.
    #[allow(clippy::too_many_arguments)]
    pub fn func_8002829c(&mut self, pos: Vec3, velocity: Vec3, accel: Vec3, prim: [u8; 4], env: [u8; 4], scale: i16, scale_step: i16) {
        self.dust_spawn(0, pos, velocity, accel, prim, env, scale, scale_step, 10, 0);
    }

    /// `func_80028304`: dust, draw flags 1, 10 frames.
    #[allow(clippy::too_many_arguments)]
    pub fn func_80028304(&mut self, pos: Vec3, velocity: Vec3, accel: Vec3, prim: [u8; 4], env: [u8; 4], scale: i16, scale_step: i16) {
        self.dust_spawn(1, pos, velocity, accel, prim, env, scale, scale_step, 10, 0);
    }

    /// `func_8002836C`: dust, draw flags 0.
    #[allow(clippy::too_many_arguments)]
    pub fn func_8002836c(&mut self, pos: Vec3, velocity: Vec3, accel: Vec3, prim: [u8; 4], env: [u8; 4], scale: i16, scale_step: i16, life: i16) {
        self.dust_spawn(0, pos, velocity, accel, prim, env, scale, scale_step, life, 0);
    }

    /// `func_8002843C`: dust, draw flags 2.
    #[allow(clippy::too_many_arguments)]
    pub fn func_8002843c(&mut self, pos: Vec3, velocity: Vec3, accel: Vec3, prim: [u8; 4], env: [u8; 4], scale: i16, scale_step: i16, life: i16) {
        self.dust_spawn(2, pos, velocity, accel, prim, env, scale, scale_step, life, 0);
    }

    /// `func_8002857C`: brown dust, draw flags 4, scale 100 growing 5, 10 frames.
    pub fn func_8002857c(&mut self, pos: Vec3, velocity: Vec3, accel: Vec3) {
        self.dust_spawn(4, pos, velocity, accel, S_DUST_BROWN_PRIM, S_DUST_BROWN_ENV, 100, 5, 10, 0);
    }

    /// `func_8002865C`: brown dust (`sDustBrownPrim`, `sDustBrownEnv`), draw flags 4.
    pub fn func_8002865c(&mut self, pos: Vec3, velocity: Vec3, accel: Vec3, scale: i16, scale_step: i16) {
        self.dust_spawn(4, pos, velocity, accel, S_DUST_BROWN_PRIM, S_DUST_BROWN_ENV, scale, scale_step, 10, 0);
    }

    /// `func_800286CC`: brown dust, draw flags 5.
    pub fn func_800286cc(&mut self, pos: Vec3, velocity: Vec3, accel: Vec3, scale: i16, scale_step: i16) {
        self.dust_spawn(5, pos, velocity, accel, S_DUST_BROWN_PRIM, S_DUST_BROWN_ENV, scale, scale_step, 10, 0);
    }

    /// `func_8002873C`: brown dust, draw flags 4.
    pub fn func_8002873c(&mut self, pos: Vec3, velocity: Vec3, accel: Vec3, scale: i16, scale_step: i16, life: i16) {
        self.dust_spawn(4, pos, velocity, accel, S_DUST_BROWN_PRIM, S_DUST_BROWN_ENV, scale, scale_step, life, 0);
    }

    /// `func_800287AC`: brown dust, draw flags 5.
    pub fn func_800287ac(&mut self, pos: Vec3, velocity: Vec3, accel: Vec3, scale: i16, scale_step: i16, life: i16) {
        self.dust_spawn(5, pos, velocity, accel, S_DUST_BROWN_PRIM, S_DUST_BROWN_ENV, scale, scale_step, life, 0);
    }

    /// `func_80028894`: a point up to `rand_scale` from `src_pos` the way `randAngle` points, with
    /// a velocity of 1 up and outwards that way.
    pub fn func_80028894(&mut self, src_pos: Vec3, rand_scale: f32) -> (Vec3, Vec3, Vec3) {
        let rand = self.rand.zero_one() * rand_scale;
        let rand_angle = (self.rand.zero_one() * 65536.0) as i32 as i16;
        let (s, c) = (eng_math::sin_s(rand_angle), eng_math::cos_s(rand_angle));
        let new_pos = Vec3::new(src_pos.x + s * rand, src_pos.y, src_pos.z + c * rand);
        let velocity = Vec3::new(s, 1.0, c);
        (new_pos, velocity, Vec3::ZERO)
    }

    /// `func_80028990`: 20 puffs of brown dust round `src_pos` (`func_8002873C`, 100 big
    /// growing by 30, 7 frames).
    pub fn func_80028990(&mut self, rand_scale: f32, src_pos: Vec3) {
        for _ in 0..20 {
            let (pos, velocity, accel) = self.func_80028894(src_pos, rand_scale);
            self.func_8002873c(pos, velocity, accel, 100, 30, 7);
        }
    }

    /// `EffectSsBlast_Spawn`: a ring-shaped shockwave on the floor under `pos`, `scale × 64 / 400`
    /// wide, growing by `scale_step`, which shrinks by `scale_step_decay` each frame.
    #[allow(clippy::too_many_arguments)]
    pub fn blast_spawn(&mut self, pos: Vec3, velocity: Vec3, accel: Vec3, inner_color: [u8; 4], outer_color: [u8; 4], scale: i16, scale_step: i16, scale_step_decay: i16, life: i16) {
        let p = blast::BlastInit { pos, velocity, accel, inner_color, outer_color, scale, scale_step, scale_step_decay, life };
        self.spawn(EFFECT_SS_BLAST, 128, SsInit::Blast(p));
    }

    /// `EffectSsBlast_SpawnWhiteShockwaveSetScale`: white inside (`{255, 255, 255, 255}`), grey
    /// out (`{200, 200, 200, 0}`), the step's decay 35.
    pub fn blast_spawn_white_shockwave_set_scale(&mut self, pos: Vec3, velocity: Vec3, accel: Vec3, scale: i16, scale_step: i16, life: i16) {
        self.blast_spawn(pos, velocity, accel, [255, 255, 255, 255], [200, 200, 200, 0], scale, scale_step, 35, life);
    }

    /// `EffectSsBlast_SpawnShockwaveSetColor`: quickly spreading (scale 100, step 375, decay 35).
    pub fn blast_spawn_shockwave_set_color(&mut self, pos: Vec3, velocity: Vec3, accel: Vec3, inner_color: [u8; 4], outer_color: [u8; 4], life: i16) {
        self.blast_spawn(pos, velocity, accel, inner_color, outer_color, 100, 375, 35, life);
    }

    /// `EffectSsBlast_SpawnWhiteShockwave`: white, quickly spreading, for 10 frames.
    pub fn blast_spawn_white_shockwave(&mut self, pos: Vec3, velocity: Vec3, accel: Vec3) {
        self.blast_spawn_shockwave_set_color(pos, velocity, accel, [255, 255, 255, 255], [200, 200, 200, 0], 10);
    }

    /// `EffectSsStick_Spawn`: a broken stick's (or blade's) half flying off from `pos` along
    /// `yaw`, the age's piece (`adult`: `gSaveContext.save.linkAge`).
    pub fn stick_spawn(&mut self, pos: Vec3, yaw: i16, adult: bool) {
        self.spawn(EFFECT_SS_STICK, 128, SsInit::Stick(stick::StickInit { pos, yaw, adult }));
    }

    /// `EffectSsHahen_Spawn`: one fragment, `gEffFragments1DL` (the withered Deku fragment)
    /// without `dlist`. Its life is capped at 200.
    #[allow(clippy::too_many_arguments)]
    pub fn hahen_spawn(&mut self, pos: Vec3, velocity: Vec3, accel: Vec3, unused: i16, scale: i16, obj_id: i16, life: i16, dlist: SsGfx) {
        let p = hahen::HahenInit { pos, velocity, accel, dlist, unused, scale, obj_id, life };
        self.spawn(EFFECT_SS_HAHEN, 128, SsInit::Hahen(p));
    }

    /// `EffectSsHahen_SpawnBurst`: `count` fragments flying out at `burst_scale`, each scaled
    /// `Rand_S16Offset(scale, rand_scale_range)`.
    #[allow(clippy::too_many_arguments)]
    pub fn hahen_spawn_burst(&mut self, pos: Vec3, burst_scale: f32, unused: i16, scale: i16, rand_scale_range: i16, count: i16, obj_id: i16, life: i16, dlist: SsGfx) {
        let accel = Vec3::new(0.0, -0.07 * burst_scale, 0.0);
        for _ in 0..count {
            let vx = (self.rand.zero_one() - 0.5) * burst_scale;
            let vz = (self.rand.zero_one() - 0.5) * burst_scale;
            let vy = ((self.rand.zero_one() * 0.5) + 0.5) * burst_scale;
            let s = self.rand.s16_offset(scale, rand_scale_range);
            self.hahen_spawn(pos, Vec3::new(vx, vy, vz), accel, unused, s, obj_id, life, dlist);
        }
    }

    /// `EffectSsKakera_Spawn`: one fragment drawn with `dlist` (in object `obj_id`), at
    /// priority 101; `arg3` is its `vec` (where its forces pull from), `arg5` to `arg11` its regs
    /// as `Effect_Ss_Kakera` names them (`rReg4`, `rReg5`, `rReg6`, `rReg0`, `rScale`, `rReg8`,
    /// `rReg9`).
    #[allow(clippy::too_many_arguments)]
    pub fn kakera_spawn(
        &mut self,
        pos: Vec3,
        velocity: Vec3,
        arg3: Vec3,
        gravity: i16,
        arg5: i16,
        arg6: i16,
        arg7: i16,
        arg8: i16,
        scale: i16,
        arg10: i16,
        arg11: i16,
        life: i32,
        color_idx: i16,
        obj_id: i16,
        dlist: SsGfx,
    ) {
        let p = kakera::KakeraInit {
            pos,
            velocity,
            unk_18: arg3,
            gravity,
            unk_26: arg5,
            unk_28: arg6,
            unk_2a: arg7,
            unk_2c: arg8,
            scale,
            unk_30: arg10,
            unk_32: arg11,
            life,
            color_idx,
            obj_id,
            dlist,
        };
        self.spawn(EFFECT_SS_KAKERA, 101, SsInit::Kakera(p));
    }

    /// `EffectSsHitMark_Spawn`.
    pub fn hit_mark_spawn(&mut self, ty: i32, scale: i16, pos: Vec3) {
        self.spawn(EFFECT_SS_HITMARK, 128, SsInit::HitMark(hitmark::HitMarkInit { ty, scale, pos }));
    }

    /// `EffectSsHitMark_SpawnFixedScale`: scale 300.
    pub fn hit_mark_spawn_fixed_scale(&mut self, ty: i32, pos: Vec3) {
        self.hit_mark_spawn(ty, 300, pos);
    }

    /// `EffectSsHitMark_SpawnCustomScale`.
    pub fn hit_mark_spawn_custom_scale(&mut self, ty: i32, scale: i16, pos: Vec3) {
        self.hit_mark_spawn(ty, scale, pos);
    }

    /// `EffectSsEnFire_SpawnVec3f`'s spawn (the caller plays `NA_SE_EV_FLAME_IGNITION` at the
    /// actor first).
    pub fn en_fire_spawn_vec3f(&mut self, actor: Option<SsActor>, pos: Vec3, scale: i16, arg4: i16, flags: i16, body_part: i16) {
        let p = en_fire::EnFireInit { actor, pos, scale, unk_12: arg4, flags, body_part };
        self.spawn(EFFECT_SS_EN_FIRE, 128, SsInit::EnFire(p));
    }

    /// `EffectSsEnIce_SpawnFlyingVec3f`'s spawn (the caller plays `NA_SE_PL_FREEZE_S` at the
    /// actor first).
    #[allow(clippy::too_many_arguments)]
    pub fn en_ice_spawn_flying_vec3f(&mut self, actor: Option<SsActor>, pos: Vec3, prim: [i16; 4], env: [i16; 3], scale: f32) {
        let p = en_ice::EnIceInit {
            actor,
            pos,
            scale,
            velocity: Vec3::ZERO,
            accel: Vec3::ZERO,
            prim_color: [prim[0] as u8, prim[1] as u8, prim[2] as u8, prim[3] as u8],
            env_color: [env[0] as u8, env[1] as u8, env[2] as u8, 0],
            life: 0,
            ty: 0,
        };
        self.spawn(EFFECT_SS_EN_ICE, 80, SsInit::EnIce(p));
    }

    /// `EffectSsEnIce_Spawn`.
    #[allow(clippy::too_many_arguments)]
    pub fn en_ice_spawn(&mut self, pos: Vec3, scale: f32, velocity: Vec3, accel: Vec3, prim: [u8; 4], env: [u8; 4], life: i32) {
        let p = en_ice::EnIceInit { actor: None, pos, scale, velocity, accel, prim_color: prim, env_color: env, life, ty: 1 };
        self.spawn(EFFECT_SS_EN_ICE, 128, SsInit::EnIce(p));
    }

    /// `EffectSsDeadDb_Spawn`.
    #[allow(clippy::too_many_arguments)]
    pub fn dead_db_spawn(&mut self, pos: Vec3, velocity: Vec3, accel: Vec3, scale: i16, scale_step: i16, prim: [i16; 4], env: [i16; 3], unused: i16, arg14: i32, play_sfx: i16) {
        let p = dead_db::DeadDbInit {
            pos,
            velocity,
            accel,
            scale,
            scale_step,
            prim_color: [prim[0] as u8, prim[1] as u8, prim[2] as u8, prim[3] as u8],
            env_color: [env[0] as u8, env[1] as u8, env[2] as u8, 0],
            unused,
            unk_34: arg14,
            play_sfx,
        };
        self.spawn(EFFECT_SS_DEAD_DB, 120, SsInit::DeadDb(p));
    }

    /// `EffectSsFireTail_Spawn`.
    #[allow(clippy::too_many_arguments)]
    pub fn fire_tail_spawn(&mut self, actor: Option<ActorHandle>, pos: Vec3, scale: f32, arg4: Vec3, arg5: i16, prim: [u8; 4], env: [u8; 4], ty: i16, body_part: i16, life: i32) {
        let p = fire_tail::FireTailInit { actor, pos, scale, unk_14: arg4, unk_20: arg5, prim_color: prim, env_color: env, ty, body_part, life };
        self.spawn(EFFECT_SS_FIRE_TAIL, 128, SsInit::FireTail(p));
    }

    /// `EffectSsFireTail_SpawnFlame`: a flame `colorIntensity` bright (the statics' colours,
    /// rewritten every call), following the actor's velocity.
    pub fn fire_tail_spawn_flame(&mut self, actor: ActorHandle, actor_velocity: Vec3, pos: Vec3, arg3: f32, body_part: i16, color_intensity: f32) {
        let c = (255.0 * color_intensity) as i32 as u8;
        let prim = [c, c, 0, 255];
        let env = [c, 0, 0, 255];
        self.fire_tail_spawn(Some(actor), pos, arg3, actor_velocity, 15, prim, env, if color_intensity == 1.0 { 0 } else { 1 }, body_part, 1);
    }

    /// `EffectSsFhgFlash_SpawnShock`.
    pub fn fhg_flash_spawn_shock(&mut self, actor: Option<ActorHandle>, pos: Vec3, scale: i16, param: u8) {
        let p = fhg_flash::FhgFlashInit { pos, velocity: Vec3::ZERO, accel: Vec3::ZERO, scale, param, actor, ty: fhg_flash::FHGFLASH_SHOCK };
        self.spawn(EFFECT_SS_FHG_FLASH, 128, SsInit::FhgFlash(p));
    }

    /// `EffectSsFCircle_Spawn`.
    pub fn fcircle_spawn(&mut self, actor: SsActor, pos: Vec3, radius: i16, height: i16) {
        self.spawn(EFFECT_SS_FCIRCLE, 128, SsInit::Fcircle(fcircle::FcircleInit { actor, pos, radius, height }));
    }

    /// `EffectSsKFire_Spawn`.
    pub fn k_fire_spawn(&mut self, pos: Vec3, velocity: Vec3, accel: Vec3, scale_max: i16, ty: u8) {
        let p = k_fire::KFireInit { pos, velocity, accel, scale_max, ty };
        self.spawn(EFFECT_SS_K_FIRE, 128, SsInit::KFire(p));
    }

    /// `EffectSsSibuki_Spawn`.
    pub fn sibuki_spawn(&mut self, pos: Vec3, velocity: Vec3, accel: Vec3, move_delay: i16, direction: i16, scale: i16) {
        let p = sibuki::SibukiInit { pos, velocity, accel, move_delay, direction, scale };
        self.spawn(EFFECT_SS_SIBUKI, 128, SsInit::Sibuki(p));
    }

    /// `EffectSsSibuki_SpawnBurst`: 30 bubbles (`KREG(19) + 30`, the debug register 0) at `pos`,
    /// all to one side (`Rand_ZeroOne() × 1.99`, truncated), six more leaving each frame
    /// (`moveDelay` `i / (KREG(27) + 6)`), 40 big (`KREG(18) + 40`).
    pub fn sibuki_spawn_burst(&mut self, pos: Vec3) {
        let rand_direction = (self.rand.zero_one() * 1.99) as i16;
        for i in 0..30i16 {
            self.sibuki_spawn(pos, Vec3::ZERO, Vec3::ZERO, i / 6, rand_direction, 40);
        }
    }
}

/// `sDustBrownPrim`, `sDustBrownEnv` (`z_effect_soft_sprite_old_init.c`).
pub const S_DUST_BROWN_PRIM: [u8; 4] = [170, 130, 90, 255];
pub const S_DUST_BROWN_ENV: [u8; 4] = [100, 60, 20, 255];

/// `EffectSs_LerpInv`: from `a` to `b` by `1 / weight_inv` (`b` at 0).
pub fn lerp_inv(a: i16, b: i16, weight_inv: i32) -> i16 {
    if weight_inv == 0 { b } else { (a as i32 + ((b as i32 - a as i32) as f32 / weight_inv as f32) as i32) as i16 }
}

/// `EffectSs_LerpS16`.
pub fn lerp_s16(a: i16, b: i16, weight: f32) -> i16 {
    ((b as i32 - a as i32) as f32 * weight + a as f32) as i32 as i16
}

/// `EffectSs_LerpU8`.
pub fn lerp_u8(a: u8, b: u8, weight: f32) -> u8 {
    (weight * (b as f32 - a as f32) + a as f32) as i32 as u8
}

/// The draws of one game frame's effects: `Effect_DrawAll`'s and `EffectSs_DrawAll`'s, into
/// the OPA and XLU lists after the actors'.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EffectDraws {
    pub opa: Vec<DrawCmd>,
    pub xlu: Vec<DrawCmd>,
}

/// What an effect's draw reads of the frame.
pub struct DrawCtx<'a> {
    /// `play->billboardMtxF`.
    pub billboard: Mat4,
    /// `play->view.eye`.
    pub eye: Vec3,
    /// `Camera_GetCamDirYaw(GET_ACTIVE_CAM(play))`.
    pub cam_dir_yaw: i16,
    /// `play->gameplayFrames`, `play->state.frames`.
    pub gameplay_frames: u32,
    pub state_frames: u32,
    /// `play->colCtx` (`Effect_Ss_Blast`'s floor, `func_800BFCB8`).
    pub col: &'a eng_collision::bgcheck::CollisionContext,
    /// `GET_PLAYER(play)->bodyPartsPos`.
    pub player_body_parts: Option<Vec<Vec3>>,
    pub actors: &'a crate::actor_ctx::ActorContext,
    pub rand: &'a mut Rand,
}

/// A draw of `bake` at `m` with the colours `prim` and `env` on the bake's colour segment.
pub(crate) fn colored(bake: &str, m: Mat4, seg: u8, prim: [u8; 4], env: [u8; 4]) -> DrawCmd {
    let mut sv = SegmentValues::default();
    sv.prim[seg as usize] = Some(prim);
    sv.env[seg as usize] = Some(env);
    DrawCmd { mesh: MeshKey::named(keys::bake(bake)), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } }
}

/// The segments the effects' bakes use: the texture (`gSPSegment(0x08, ...)`), the setup, the
/// draw's colours, and data the bake builds (vertices).
pub(crate) const SEG_TEX: u8 = 0x08;
pub(crate) const SEG_DATA: u8 = 0x09;
pub(crate) const SEG_SETUP: u8 = 0x0D;
pub(crate) const SEG_COLOR: u8 = 0x0E;

/// `SkinMatrix_SetTranslate` × `billboardMtxF` × `SkinMatrix_SetScale(sx, sy, sz)`: the soft
/// sprites' facing quad.
pub(crate) fn billboard_mtx(pos: Vec3, billboard: Mat4, scale: Vec3) -> Mat4 {
    Mat4::from_translation(pos) * billboard * Mat4::from_scale(scale)
}

/// An identity `Mtx` (s15.16, the integer halves first), for a list's `gSPMatrix` of a segment
/// the draw multiplies in itself (the billboard on segment 1).
pub(crate) fn identity_mtx() -> Vec<u8> {
    let mut b = vec![0u8; 64];
    for i in 0..4 {
        b[(i * 4 + i) * 2 + 1] = 1;
    }
    b
}

/// Every bake the effects draw with.
pub fn bakes() -> Vec<MeshBake> {
    let mut v = dust::bakes();
    v.extend(blast::bakes());
    v.extend(hitmark::bakes());
    v.extend(en_fire::bakes());
    v.extend(en_ice::bakes());
    v.extend(dead_db::bakes());
    v.extend(fhg_flash::bakes());
    v.extend(fcircle::bakes());
    v.extend(sibuki::bakes());
    v.extend(spark::bakes());
    v.extend(shield_particle::bakes());
    v.extend(kakera::bakes());
    v
}

impl PlayState {
    /// The effects' spawner, on this play state's table and generator.
    pub fn ss(&mut self) -> (SsSpawn<'_>, &mut crate::audio::GameAudio) {
        let s = SsSpawn { info: &mut self.effect_ss, rand: &mut self.rand, objects: Some(&self.object_ctx), stops: &mut self.effect_ss_stops };
        (s, &mut self.audio)
    }

    /// Runs `f` with the spawner, then stops the sounds of the slots it reused.
    pub fn with_ss<R>(&mut self, f: impl FnOnce(&mut SsSpawn<'_>) -> R) -> R {
        let r = {
            let (mut s, _) = self.ss();
            f(&mut s)
        };
        self.flush_effect_ss_stops();
        r
    }

    /// The `Audio_StopSfxByPos` calls the spawns' deletions asked for.
    pub fn flush_effect_ss_stops(&mut self) {
        for p in std::mem::take(&mut self.effect_ss_stops) {
            self.audio.stop_sfx_by_pos(p);
        }
    }

    /// `EffectSsEnFire_SpawnVec3f`: `Actor_PlaySfx(actor, NA_SE_EV_FLAME_IGNITION)`, then the
    /// flame (`body_part` ≥ 0 follows the actor's fire positions).
    pub fn effect_ss_en_fire_spawn_vec3f(&mut self, actor: Option<SsActor>, pos: Vec3, scale: i16, arg4: i16, flags: i16, body_part: i16) {
        if let Some(a) = actor {
            crate::actor_ctx::audio_play_actor_sfx_at(self, a.handle, crate::audio::sfx::NA_SE_EV_FLAME_IGNITION);
        }
        self.with_ss(|s| s.en_fire_spawn_vec3f(actor, pos, scale, arg4, flags, body_part));
    }

    /// `EffectSsEnIce_SpawnFlyingVec3f`: `Actor_PlaySfx(actor, NA_SE_PL_FREEZE_S)`, then the
    /// ice.
    pub fn effect_ss_en_ice_spawn_flying_vec3f(&mut self, actor: Option<SsActor>, pos: Vec3, prim: [i16; 4], env: [i16; 3], scale: f32) {
        if let Some(a) = actor {
            crate::actor_ctx::audio_play_actor_sfx_at(self, a.handle, crate::audio::sfx::NA_SE_PL_FREEZE_S);
        }
        self.with_ss(|s| s.en_ice_spawn_flying_vec3f(actor, pos, prim, env, scale));
    }

    /// `EffectSs_Delete` of slot `i`, its sounds stopped.
    fn effect_ss_delete(&mut self, i: usize) {
        let old = std::mem::take(&mut self.effect_ss.table[i]);
        let mut stops = Vec::new();
        delete_stops(&old, i, &mut stops);
        for p in stops {
            self.audio.stop_sfx_by_pos(p);
        }
    }

    /// `EffectSs_UpdateAll`: each live effect's life counted down (deleted below 0), then its
    /// velocity and position stepped and its update run (`EffectSs_Update`).
    pub fn effect_ss_update_all(&mut self) {
        for i in 0..self.effect_ss.table.len() {
            if self.effect_ss.table[i].life > -1 {
                self.effect_ss.table[i].life -= 1;
                if self.effect_ss.table[i].life < 0 {
                    self.effect_ss_delete(i);
                }
            }
            if self.effect_ss.table[i].life > -1 {
                self.effect_ss_update(i);
            }
        }
    }

    /// `EffectSs_Update`.
    fn effect_ss_update(&mut self, i: usize) {
        let mut e = self.effect_ss.table[i].clone();
        let Some(u) = e.update else { return };
        e.velocity += e.accel;
        e.pos += e.velocity;
        match u {
            SsUpdate::Dust => dust::update(self, &mut e),
            SsUpdate::DustFire => dust::update_fire(self, &mut e),
            SsUpdate::Blast => blast::update(&mut e),
            SsUpdate::Hahen => hahen::update(self, &mut e),
            SsUpdate::HitMark => hitmark::update(&mut e),
            SsUpdate::EnFire => en_fire::update(self, &mut e),
            SsUpdate::EnIce => en_ice::update(&mut e),
            SsUpdate::EnIceFlying => en_ice::update_flying(self, &mut e),
            SsUpdate::DeadDb => dead_db::update(self, i, &mut e),
            SsUpdate::FireTail => fire_tail::update(&mut e),
            SsUpdate::FhgFlashShock => fhg_flash::update_shock(self, &mut e),
            SsUpdate::FhgFlashLightBall => fhg_flash::update_light_ball(self, &mut e),
            SsUpdate::Fcircle => fcircle::update(self, &mut e),
            SsUpdate::KFire => k_fire::update(&mut e),
            SsUpdate::Sibuki => sibuki::update(self, &mut e),
            SsUpdate::Stick => stick::update(&mut e),
            SsUpdate::Kakera => kakera::update(self, &mut e),
        }
        self.effect_ss.table[i] = e;
    }

    /// `play->billboardMtxF` for this frame's view (`View_Apply`'s viewing matrix, rotation
    /// only, inverted).
    pub fn billboard_mtx(&self) -> Mat4 {
        let view = glam::camera::rh::view::look_at_mat4(self.view.eye, self.view.at, Vec3::Y);
        Mat4::from_mat3(glam::Mat3::from_mat4(view).transpose())
    }

    /// `Effect_DrawAll`, then `EffectSs_DrawAll` (the end of `Actor_DrawAll`): this frame's
    /// effect draws into `effect_draws`.
    pub fn effect_draw_all(&mut self) {
        let mut out = EffectDraws::default();
        let player_body_parts = self.player.and_then(|h| self.actors.get(h)).and_then(|p| p.as_player()).map(|p| (0..crate::actor_ctx::PLAYER_BODYPART_MAX).map(|i| p.body_part(i)).collect());
        let mut ctx = DrawCtx {
            billboard: self.billboard_mtx(),
            eye: self.view.eye,
            cam_dir_yaw: self.cam_dir_yaw(),
            gameplay_frames: self.gameplay_frames,
            state_frames: self.state_frames,
            col: &self.col,
            player_body_parts,
            actors: &self.actors,
            rand: &mut self.rand,
        };
        // Effect_DrawAll: the sparks, the trails (not ported), the shield particles.
        for s in self.effect_ctx.sparks.iter().flatten() {
            spark::draw(s, &mut ctx, &mut out);
        }
        for s in self.effect_ctx.shield_particles.iter().flatten() {
            shield_particle::draw(s, &mut out);
        }
        // EffectSs_DrawAll.
        let mut deleted = Vec::new();
        for i in 0..self.effect_ss.table.len() {
            let e = &mut self.effect_ss.table[i];
            if e.life <= -1 {
                continue;
            }
            if e.pos.x > 32000.0 || e.pos.x < -32000.0 || e.pos.y > 32000.0 || e.pos.y < -32000.0 || e.pos.z > 32000.0 || e.pos.z < -32000.0 {
                // "Since the position is outside the area, delete it."
                log::debug!("EffectSoftSprite2_disp(): effect type {} at {:?} is outside the area: deleted", e.ty, e.pos);
                deleted.push(i);
                continue;
            }
            let Some(d) = e.draw else { continue };
            match d {
                SsDraw::Dust => dust::draw(e, &ctx, &mut out),
                SsDraw::Blast => blast::draw(e, &ctx, &mut out),
                SsDraw::Hahen | SsDraw::HahenGray => hahen::draw(e, &mut out),
                SsDraw::HitMark => hitmark::draw(e, &ctx, &mut out),
                SsDraw::EnFire => en_fire::draw(e, &ctx, &mut out),
                SsDraw::EnIce => en_ice::draw(e, &ctx, &mut out),
                SsDraw::DeadDb => dead_db::draw(e, ctx.billboard, &mut out),
                SsDraw::FireTail => fire_tail::draw(e, &ctx, &mut out),
                SsDraw::FhgFlashShock => fhg_flash::draw_shock(e, ctx.billboard, &mut out),
                SsDraw::Fcircle => fcircle::draw(e, ctx.gameplay_frames, &mut out),
                SsDraw::KFire => k_fire::draw(e, i, &ctx, &mut out),
                SsDraw::Sibuki => sibuki::draw(e, ctx.billboard, &mut out),
                SsDraw::Stick => stick::draw(e, ctx.state_frames, &mut out),
                SsDraw::Kakera => kakera::draw(e, &mut out),
            }
        }
        for i in deleted {
            self.effect_ss_delete(i);
        }
        self.effect_draws = out;
    }
}

/// `z_effect.c`'s `EffectContext`: the sparks and the shield particles (the trails aren't
/// ported).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EffectContext {
    /// `sparks[SPARK_COUNT]`: `None` while inactive.
    pub sparks: [Option<spark::EffectSpark>; SPARK_COUNT],
    /// `shieldParticles[SHIELD_PARTICLE_COUNT]`.
    pub shield_particles: [Option<shield_particle::EffectShieldParticle>; SHIELD_PARTICLE_COUNT],
}

/// `SPARK_COUNT`, `BLURE_COUNT`, `SHIELD_PARTICLE_COUNT` (`effect.h`).
pub const SPARK_COUNT: usize = 3;
pub const BLURE_COUNT: usize = 25;
pub const SHIELD_PARTICLE_COUNT: usize = 3;

/// `EffectType` and its init parameters (`Effect_Add`).
#[derive(Debug, Clone, PartialEq)]
pub enum EffectInit {
    /// `EFFECT_SPARK`.
    Spark(spark::EffectSparkInit),
    /// `EFFECT_BLURE1`, `EFFECT_BLURE2`: not ported.
    Blure,
    /// `EFFECT_SHIELD_PARTICLE`.
    ShieldParticle(shield_particle::EffectShieldParticleInit),
}

impl PlayState {
    /// `Effect_Add`: the first inactive slot of the kind, initialised. Returns its index
    /// (`TOTAL_EFFECT_COUNT` when there's none).
    pub fn effect_add(&mut self, init: EffectInit) -> usize {
        const TOTAL: usize = SPARK_COUNT + BLURE_COUNT + SHIELD_PARTICLE_COUNT;
        match init {
            EffectInit::Spark(p) => {
                let Some(i) = self.effect_ctx.sparks.iter().position(|s| s.is_none()) else {
                    log::debug!("EffectAdd(): I cannot secure it. Type 0");
                    return TOTAL;
                };
                match spark::init(&p, &mut self.rand) {
                    Some(s) => self.effect_ctx.sparks[i] = Some(s),
                    // The C marks the slot active either way, with what the failed init left.
                    None => self.effect_ctx.sparks[i] = Some(spark::EffectSpark::failed(&p)),
                }
                i
            }
            EffectInit::Blure => {
                log::debug!("Effect_Add: EffectBlure is not ported");
                TOTAL
            }
            EffectInit::ShieldParticle(p) => {
                let Some(i) = self.effect_ctx.shield_particles.iter().position(|s| s.is_none()) else {
                    log::debug!("EffectAdd(): I cannot secure it. Type 3");
                    return TOTAL;
                };
                let s = shield_particle::init(self, &p);
                self.effect_ctx.shield_particles[i] = Some(s);
                i + SPARK_COUNT + BLURE_COUNT
            }
        }
    }

    /// `Effect_Delete`: the effect at `index` (as `Effect_Add` gave it) made inactive and
    /// destroyed. `TOTAL_EFFECT_COUNT` (what an `Effect_Add` that found no slot gives, as for
    /// every `EffectBlure`, which isn't ported) does nothing.
    pub fn effect_delete(&mut self, index: usize) {
        const TOTAL: usize = SPARK_COUNT + BLURE_COUNT + SHIELD_PARTICLE_COUNT;
        if index == TOTAL {
            return;
        }
        if index < SPARK_COUNT {
            // EffectSpark_Destroy does nothing.
            self.effect_ctx.sparks[index] = None;
            return;
        }
        let index = index - SPARK_COUNT;
        if index < BLURE_COUNT {
            // No trail is ever added (EffectBlure isn't ported).
            return;
        }
        let index = index - BLURE_COUNT;
        if index < SHIELD_PARTICLE_COUNT
            && let Some(s) = self.effect_ctx.shield_particles[index].take()
        {
            shield_particle::destroy(self, &s);
        }
    }

    /// `Effect_UpdateAll`: each active one's update, deleted when it says it's done.
    pub fn effect_update_all(&mut self) {
        for i in 0..SPARK_COUNT {
            if let Some(s) = &mut self.effect_ctx.sparks[i]
                && spark::update(s)
            {
                self.effect_ctx.sparks[i] = None;
            }
        }
        for i in 0..SHIELD_PARTICLE_COUNT {
            if let Some(mut s) = self.effect_ctx.shield_particles[i].take() {
                if shield_particle::update(self, &mut s) {
                    shield_particle::destroy(self, &s);
                } else {
                    self.effect_ctx.shield_particles[i] = Some(s);
                }
            }
        }
    }

    /// `Effect_DeleteAll` (`Play_Destroy`): the shield particles' lights removed.
    pub fn effect_delete_all(&mut self) {
        self.effect_ctx.sparks = Default::default();
        for i in 0..SHIELD_PARTICLE_COUNT {
            if let Some(s) = self.effect_ctx.shield_particles[i].take() {
                shield_particle::destroy(self, &s);
            }
        }
    }
}

/// A spark of `CollisionCheck_*Blood`'s and `CollisionCheck_SpawnWaterDroplets`' kind at `v`:
/// 27 drops (`uDiv`, `vDiv` 5) at speed 8 falling by 1, for 16 frames.
fn blood_spark(v: Vec3, start: [[u8; 4]; 4], end: [[u8; 4]; 4]) -> spark::EffectSparkInit {
    spark::EffectSparkInit {
        position: [v.x as i32 as i16, v.y as i32 as i16, v.z as i32 as i16],
        speed: 8.0,
        gravity: -1.0,
        u_div: 5,
        v_div: 5,
        color_start: start,
        color_end: end,
        timer: 0,
        duration: 16,
    }
}

/// `CollisionCheck_SpawnShieldParticles`' (`metalInit`, with the light) and
/// `CollisionCheck_SpawnShieldParticlesWood`'s (`woodInit`, without) parameters at `v`.
fn shield_particles_init(v: Vec3, light_decay: bool) -> shield_particle::EffectShieldParticleInit {
    let p = [v.x as i32 as i16, v.y as i32 as i16, v.z as i32 as i16];
    shield_particle::EffectShieldParticleInit {
        num_elements: 16,
        position: p,
        prim_color_start: [0, 200, 255, 255],
        env_color_start: [255, 255, 255, 255],
        prim_color_mid: [255, 255, 128, 255],
        env_color_mid: [255, 255, 0, 255],
        prim_color_end: [255, 64, 0, 200],
        env_color_end: [255, 0, 0, 255],
        deceleration: 2.1,
        max_initial_speed: 35.0,
        length_cutoff: 30.0,
        duration: 8,
        light_point: crate::lights::LightInfo::point_no_glow(p[0], p[1], p[2], [0, 128, 255], 300),
        light_decay,
    }
}

impl PlayState {
    /// The collision check's hit effects (`CollisionCheck_HitEffects`), in its order.
    pub fn collision_check_hit_fx(&mut self, fx: Vec<crate::collision_check::HitFx>) {
        use crate::audio::sfx::{SfxF32, SfxS8};
        use crate::collision_check::{BLOOD_BLUE, BLOOD_GREEN, BLOOD_RED, BLOOD_RED2, BLOOD_WATER, HitFx};
        for f in fx {
            match f {
                HitFx::Sfx(id, pos) => self.audio.play_sfx_general(id, pos, 4, SfxF32::One, SfxF32::One, SfxS8::Zero),
                HitFx::HitMark(ty, pos) => self.with_ss(|s| s.hit_mark_spawn_fixed_scale(ty, pos)),
                HitFx::Blood(BLOOD_BLUE, v) => {
                    // CollisionCheck_BlueBlood.
                    let start = [[10, 10, 200, 255], [0, 0, 128, 255], [0, 0, 128, 255], [0, 0, 128, 255]];
                    let end = [[0, 0, 32, 0], [0, 0, 32, 0], [0, 0, 64, 0], [0, 0, 64, 0]];
                    self.effect_add(EffectInit::Spark(blood_spark(v, start, end)));
                }
                HitFx::Blood(BLOOD_GREEN, v) => {
                    // CollisionCheck_GreenBlood.
                    let start = [[10, 200, 10, 255], [0, 128, 0, 255], [0, 128, 0, 255], [0, 128, 0, 255]];
                    let end = [[0, 32, 0, 0], [0, 32, 0, 0], [0, 64, 0, 0], [0, 64, 0, 0]];
                    self.effect_add(EffectInit::Spark(blood_spark(v, start, end)));
                }
                HitFx::Blood(BLOOD_WATER, v) => {
                    // CollisionCheck_WaterBurst: EffectSsSibuki_SpawnBurst (no actor has HIT4),
                    // then CollisionCheck_SpawnWaterDroplets.
                    self.with_ss(|s| s.sibuki_spawn_burst(v));
                    let start = [[255, 255, 255, 255], [100, 100, 100, 100], [100, 100, 100, 100], [100, 100, 100, 100]];
                    let end = [[50, 50, 50, 50], [50, 50, 50, 50], [50, 50, 50, 50], [0, 0, 0, 0]];
                    self.effect_add(EffectInit::Spark(blood_spark(v, start, end)));
                }
                HitFx::Blood(BLOOD_RED | BLOOD_RED2, v) => {
                    // CollisionCheck_SpawnRedBlood.
                    let start = [[128, 0, 64, 255], [128, 0, 64, 255], [255, 128, 0, 255], [255, 128, 0, 255]];
                    let end = [[64, 0, 32, 0], [64, 0, 32, 0], [128, 0, 64, 0], [128, 0, 64, 0]];
                    self.effect_add(EffectInit::Spark(blood_spark(v, start, end)));
                }
                HitFx::Blood(..) => {}
                HitFx::ShieldParticlesMetal(v) => {
                    self.effect_add(EffectInit::ShieldParticle(shield_particles_init(v, true)));
                }
                HitFx::ShieldParticlesWood(v) => {
                    self.effect_add(EffectInit::ShieldParticle(shield_particles_init(v, false)));
                }
            }
        }
    }
}
