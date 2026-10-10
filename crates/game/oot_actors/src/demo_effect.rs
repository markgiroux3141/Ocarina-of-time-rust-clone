//! `Demo_Effect` (`ovl_Demo_Effect/z_demo_effect.c`): the cutscenes' lights and set pieces. Its
//! params' low byte picks one of 26 types (`DemoEffectType`); bits 8 to 11 a light's size and
//! 12 to 15 its colour. Each type waits for its object (`DemoEffect_WaitForObject`), then runs its
//! own update and draw:
//! - the three goddesses' lights (`GOD_LGT_DIN`, `_NAYRU`, `_FARORE`), moved by their cue, Din's
//!   leaving fire balls (`FIRE_BALL`, which burst into a blue orb and two light rings), Nayru's
//!   light rings, Farore's light shower (`LGT_SHOWER`);
//! - the Triforce (`TRIFORCE_SPOT`) with its crystal light (`CRYSTAL_LIGHT`), light ring
//!   (`LIGHTRING_TRIFORCE`) and blue orb (`BLUE_ORB`);
//! - the light (`LIGHT`), the jewels (`JEWEL_KOKIRI`, `_GORON`, `_ZORA`: the Kokiri Emerald held
//!   up over Link), the medals and light arrows (`GetItem_Draw`), the Temple of Time's dust;
//! - the time warps (`TIMEWARP_*`): the Master Sword's, the Chamber of Sages' return, and the Song
//!   of Time blocks' (`Obj_Timeblock` spawns them), a curve skeleton (`oot_game::skel_curve`).
//!
//! Ported whole (GAME-06 milestone 1a, ADR 0053). The draws are bakes (ADR 0006): one per list,
//! their colours and scrolls dynamic. Two draws write their object's vertices
//! (`DemoEffect_TimewarpShrink`, the Triforce's light column): those writes are object RAM
//! (`ObjectContext::written`, ADR 0051), and the draws rebuild their vertex colours from it by
//! each baked vertex's source address (`eng_gfx::Batch::sources`).
//!
//! The draws' changes to the actor (the god lights' turn, the light's flicker, the blue orb's
//! spin, a medal's first frame, the vertex writes) are made once per game frame in `draw_update`;
//! their sounds in `draw_sfx`. Not drawn, as elsewhere: the sparkles
//! (`EffectSsKiraKira_SpawnDispersed`, whose and whose callers' `Rand` calls are made), the
//! look-at the jewels' and the Triforce's texgen read (`func_8002EBCC`, `func_8002ED80`: the
//! renderer's texgen reads the view).

use std::sync::Arc;

use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::smooth_step_to_f;
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_BG, ACTORCAT_BOSS, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::audio::sfx::*;
use oot_game::cutscene::{CS_STATE_IDLE, CsCmdActorCue};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::play_scene::SCENE_KOKIRI_FOREST;
use oot_game::scene_table::{gfx_tex_scroll, gfx_two_tex_scroll};
use oot_game::skel_curve::{CurveAnimation, CurveSkeleton, SkelCurve};
use oot_game::sys_matrix::{MtxF, rad_to_binang};

/// `ACTOR_DEMO_EFFECT` (`actor_table.h`: 0x008B).
pub const ACTOR_DEMO_EFFECT: i16 = 0x008B;

/// `Demo_Effect_Profile`: `ACTORCAT_BG`, `ACTOR_FLAG_UPDATE_CULLING_DISABLED |
/// ACTOR_FLAG_DRAW_CULLING_DISABLED`, `OBJECT_GAMEPLAY_KEEP` (each type waits for its own).
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_DEMO_EFFECT, name: "Demo_Effect", category: ACTORCAT_BG, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED, object: "gameplay_keep" };

/// `DemoEffectType`.
pub const DEMO_EFFECT_CRYSTAL_LIGHT: u8 = 0x00;
pub const DEMO_EFFECT_FIRE_BALL: u8 = 0x01;
pub const DEMO_EFFECT_BLUE_ORB: u8 = 0x02;
pub const DEMO_EFFECT_LGT_SHOWER: u8 = 0x03;
pub const DEMO_EFFECT_GOD_LGT_DIN: u8 = 0x04;
pub const DEMO_EFFECT_GOD_LGT_NAYRU: u8 = 0x05;
pub const DEMO_EFFECT_GOD_LGT_FARORE: u8 = 0x06;
pub const DEMO_EFFECT_LIGHTRING_EXPANDING: u8 = 0x07;
pub const DEMO_EFFECT_TRIFORCE_SPOT: u8 = 0x08;
pub const DEMO_EFFECT_MEDAL_FIRE: u8 = 0x09;
pub const DEMO_EFFECT_MEDAL_WATER: u8 = 0x0A;
pub const DEMO_EFFECT_MEDAL_FOREST: u8 = 0x0B;
pub const DEMO_EFFECT_MEDAL_SPIRIT: u8 = 0x0C;
pub const DEMO_EFFECT_MEDAL_SHADOW: u8 = 0x0D;
pub const DEMO_EFFECT_MEDAL_LIGHT: u8 = 0x0E;
pub const DEMO_EFFECT_TIMEWARP_MASTERSWORD: u8 = 0x0F;
pub const DEMO_EFFECT_LIGHTRING_SHRINKING: u8 = 0x10;
pub const DEMO_EFFECT_LIGHTRING_TRIFORCE: u8 = 0x11;
pub const DEMO_EFFECT_LIGHT: u8 = 0x12;
pub const DEMO_EFFECT_JEWEL_KOKIRI: u8 = 0x13;
pub const DEMO_EFFECT_JEWEL_GORON: u8 = 0x14;
pub const DEMO_EFFECT_JEWEL_ZORA: u8 = 0x15;
pub const DEMO_EFFECT_DUST: u8 = 0x16;
pub const DEMO_EFFECT_LIGHTARROW: u8 = 0x17;
pub const DEMO_EFFECT_TIMEWARP_TIMEBLOCK_LARGE: u8 = 0x18;
pub const DEMO_EFFECT_TIMEWARP_TIMEBLOCK_SMALL: u8 = 0x19;

/// `DemoEffectLightColor`.
pub const DEMO_EFFECT_LIGHT_RED: u8 = 0;
pub const DEMO_EFFECT_LIGHT_BLUE: u8 = 1;
pub const DEMO_EFFECT_LIGHT_GREEN: u8 = 2;
pub const DEMO_EFFECT_LIGHT_ORANGE: u8 = 3;
pub const DEMO_EFFECT_LIGHT_YELLOW: u8 = 4;
pub const DEMO_EFFECT_LIGHT_PURPLE: u8 = 5;
pub const DEMO_EFFECT_LIGHT_GREEN2: u8 = 6;

/// `DemoEffectGodLgtType`.
pub const GOD_LGT_DIN: u8 = 0;
pub const GOD_LGT_NAYRU: u8 = 1;
pub const GOD_LGT_FARORE: u8 = 2;

/// The objects (`object_table.h`).
const OBJECT_GAMEPLAY_KEEP: i16 = 0x0001;
const OBJECT_EFC_CRYSTAL_LIGHT: i16 = 0x008E;
const OBJECT_EFC_FIRE_BALL: i16 = 0x008F;
const OBJECT_EFC_LGT_SHOWER: i16 = 0x0091;
const OBJECT_GOD_LGT: i16 = 0x0093;
const OBJECT_LIGHT_RING: i16 = 0x0094;
const OBJECT_TRIFORCE_SPOT: i16 = 0x0095;
const OBJECT_EFC_TW: i16 = 0x00A8;
const OBJECT_GI_JEWEL: i16 = 0x00AD;
const OBJECT_GI_MEDAL: i16 = 0x00BA;
const OBJECT_GI_M_ARROW: i16 = 0x0158;

/// `sEffectTypeObjects`: the object each type draws with.
const S_EFFECT_TYPE_OBJECTS: [i16; 26] = [
    OBJECT_EFC_CRYSTAL_LIGHT,
    OBJECT_EFC_FIRE_BALL,
    OBJECT_GAMEPLAY_KEEP,
    OBJECT_EFC_LGT_SHOWER,
    OBJECT_GOD_LGT,
    OBJECT_GOD_LGT,
    OBJECT_GOD_LGT,
    OBJECT_LIGHT_RING,
    OBJECT_TRIFORCE_SPOT,
    OBJECT_GI_MEDAL,
    OBJECT_GI_MEDAL,
    OBJECT_GI_MEDAL,
    OBJECT_GI_MEDAL,
    OBJECT_GI_MEDAL,
    OBJECT_GI_MEDAL,
    OBJECT_EFC_TW,
    OBJECT_LIGHT_RING,
    OBJECT_LIGHT_RING,
    OBJECT_GAMEPLAY_KEEP,
    OBJECT_GI_JEWEL,
    OBJECT_GI_JEWEL,
    OBJECT_GI_JEWEL,
    OBJECT_GI_JEWEL,
    OBJECT_GI_M_ARROW,
    OBJECT_EFC_TW,
    OBJECT_EFC_TW,
];

/// `sTimewarpVertexSizeIndices`: which of `sizes` each of `gTimeWarpVtx`'s 21 vertices' alpha
/// takes (0: left as it is).
pub const S_TIMEWARP_VERTEX_SIZE_INDICES: [u8; 21] = [1, 1, 2, 0, 1, 1, 2, 0, 1, 2, 0, 2, 1, 0, 1, 0, 2, 0, 2, 2, 0];

/// `sJewelSparkleColors`: each jewel's sparkles' prim and env colours.
const S_JEWEL_SPARKLE_COLORS: [[[u8; 3]; 2]; 5] =
    [[[255, 255, 255], [100, 255, 0]], [[255, 255, 255], [200, 0, 150]], [[255, 255, 255], [0, 100, 255]], [[0, 0, 0], [0, 0, 0]], [[223, 0, 0], [0, 0, 0]]];

/// The entrances it checks (`entrance_table.h`).
const ENTR_TEMPLE_OF_TIME_0: u16 = 0x053;
const ENTR_CUTSCENE_MAP_0: u16 = 0x0A0;
const ENTR_KOKIRI_FOREST_0: u16 = 0x0EE;
const ENTR_DEATH_MOUNTAIN_TRAIL_0: u16 = 0x13D;
const ENTR_TEMPLE_OF_TIME_4: u16 = 0x324;
const ENTR_CASTLE_COURTYARD_ZELDA_0: u16 = 0x400;
/// The scenes it checks (`scene_table.h`).
const SCENE_JABU_JABU: u16 = 0x02;
const SCENE_GREAT_FAIRYS_FOUNTAIN_MAGIC: u16 = 0x3B;
const SCENE_GREAT_FAIRYS_FOUNTAIN_SPELLS: u16 = 0x3D;
const SCENE_TEMPLE_OF_TIME: u16 = 0x43;
const SCENE_ZORAS_FOUNTAIN: u16 = 0x59;
const SCENE_DEATH_MOUNTAIN_TRAIL: u16 = 0x60;
/// `save.h`.
const EVENTCHKINF_OPENED_DOOR_OF_TIME: u16 = 0x4B;
const EVENTCHKINF_C9: u16 = 0xC9;
const INFTABLE_RUTO_HAS_SAPPHIRE: u16 = 0x145;
/// `GID_MEDALLION_*`, `GID_ARROW_LIGHT` (`item.h`).
const GID_MEDALLION_FOREST: u8 = 0x0B;
const GID_MEDALLION_FIRE: u8 = 0x0C;
const GID_MEDALLION_WATER: u8 = 0x0D;
const GID_MEDALLION_SPIRIT: u8 = 0x0E;
const GID_MEDALLION_SHADOW: u8 = 0x0F;
const GID_MEDALLION_LIGHT: u8 = 0x10;
const GID_ARROW_LIGHT: u8 = 0x61;
/// `SEQ_CS_EFFECTS_*` (`sequence.h`).
const SEQ_CS_EFFECTS_SWORD_GLOW: u8 = 0x0;
const SEQ_CS_EFFECTS_FARORE_MAGIC: u8 = 0x3;
const SEQ_CS_EFFECTS_NAYRU_MAGIC: u8 = 0x4;
const SEQ_CS_EFFECTS_DIN_MAGIC: u8 = 0x5;

/// The vertex arrays the draws write (object RAM, `ObjectContext::written`), keyed by their
/// offset in the file: `gTimeWarpVtx` (21) in `object_efc_tw`, `gTriforceVtx` (96) in
/// `object_triforce_spot`. Each region holds the arrays' colour words (`cn`, or a lit vertex's
/// normal and alpha, bytes 12 to 15 of each `Vtx`), four bytes a vertex: what the draws read back.
pub const TIME_WARP_VTX: u32 = 0x0060;
pub const TIME_WARP_VTX_COUNT: usize = 21;
pub const TRIFORCE_VTX: u32 = 0x0000;
pub const TRIFORCE_VTX_COUNT: usize = 96;
/// `gTriforceLightColumnDL`'s vertices whose alpha follows the column's opacity.
const TRIFORCE_COLUMN_VERTICES: [usize; 8] = [86, 87, 88, 89, 92, 93, 94, 95];

/// The update functions (`updateFunc`, `initUpdateFunc`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateFunc {
    WaitForObject,
    CrystalLight,
    PositionToParent,
    BlueOrbGrow,
    BlueOrbShrink,
    LgtShower,
    GodLgtDin,
    GodLgtNayru,
    GodLgtFarore,
    LightRingExpanding,
    TriforceSpot,
    GetItem,
    LightRingShrinking,
    LightRingTriforce,
    LightEffect,
    JewelChild,
    JewelAdult,
    Dust,
    CreationFireball,
    InitCreationFireball,
    InitTimeWarp,
    InitTimeWarpTimeblock,
    TimeWarpReturnFromChamberOfSages,
    TimeWarpPullMasterSword,
    TimeWarpTimeblock,
    /// No update (an assert's default).
    None,
}

/// The draw functions (`initDrawFunc`, then `actor.draw`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawFunc {
    CrystalLight,
    FireBall,
    BlueOrb,
    LgtShower,
    GodLgt,
    LightRing,
    TriforceSpot,
    GetItem,
    LightEffect,
    TimeWarp,
    Jewel,
}

/// The struct's union (0x184): three bytes and an `s16` at 0x188, read through each type's
/// member names (`DemoEffectLightRing`'s `timerIncrement` is `godLgt.type`'s byte, ...).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Union {
    pub b0: u8,
    pub b1: u8,
    pub b2: u8,
    pub s4: i16,
}

macro_rules! members {
    ($($name:ident, $name_mut:ident: $field:ident $t:ty;)*) => {
        #[allow(dead_code)]
        impl Union {
            $(
                pub fn $name(&self) -> $t {
                    self.$field
                }
                pub fn $name_mut(&mut self) -> &mut $t {
                    &mut self.$field
                }
            )*
        }
    };
}

members! {
    fire_ball_timer, fire_ball_timer_mut: b0 u8;
    blue_orb_alpha, blue_orb_alpha_mut: b0 u8;
    blue_orb_scale, blue_orb_scale_mut: b1 u8;
    blue_orb_rotation, blue_orb_rotation_mut: s4 i16;
    light_alpha, light_alpha_mut: b0 u8;
    light_scale_flag, light_scale_flag_mut: b1 u8;
    light_flicker, light_flicker_mut: b2 u8;
    light_rotation, light_rotation_mut: s4 i16;
    lgt_shower_alpha, lgt_shower_alpha_mut: b0 u8;
    god_lgt_type, god_lgt_type_mut: b0 u8;
    god_lgt_light_ring_spawn_delay, god_lgt_light_ring_spawn_delay_mut: b1 u8;
    god_lgt_rotation, god_lgt_rotation_mut: b2 u8;
    god_lgt_light_ring_spawn_timer, god_lgt_light_ring_spawn_timer_mut: s4 i16;
    light_ring_timer_increment, light_ring_timer_increment_mut: b0 u8;
    light_ring_alpha, light_ring_alpha_mut: b1 u8;
    light_ring_timer, light_ring_timer_mut: s4 i16;
    triforce_spot_opacity, triforce_spot_opacity_mut: b0 u8;
    light_column_opacity, light_column_opacity_mut: b1 u8;
    crystal_light_opacity, crystal_light_opacity_mut: b2 u8;
    triforce_spot_rotation, triforce_spot_rotation_mut: s4 i16;
    get_item_is_position_init, get_item_is_position_init_mut: b0 u8;
    get_item_is_loaded, get_item_is_loaded_mut: b1 u8;
    get_item_draw_id, get_item_draw_id_mut: b2 u8;
    get_item_rotation, get_item_rotation_mut: s4 i16;
    time_warp_shrink_timer, time_warp_shrink_timer_mut: s4 i16;
    jewel_type, jewel_type_mut: b0 u8;
    jewel_is_position_init, jewel_is_position_init_mut: b1 u8;
    jewel_alpha, jewel_alpha_mut: b2 u8;
    jewel_timer, jewel_timer_mut: s4 i16;
    dust_timer, dust_timer_mut: b0 u8;
}

/// A bake's vertices: their colours and where they were loaded from, in the mesh's order
/// (what `DrawParams::vertex_colors` follows).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BakeVertices {
    pub colors: Vec<[u8; 4]>,
    pub sources: Vec<u32>,
}

impl BakeVertices {
    /// From the pack's bake `name`.
    fn load(play: &PlayState, name: &str) -> Option<Arc<BakeVertices>> {
        let d = play.assets.as_ref()?.pack.bake(name).map_err(|e| log::error!("Demo_Effect: bake {name}: {e:#}")).ok()?;
        let mut v = BakeVertices::default();
        for b in &d.batches {
            v.colors.extend(b.vertices.iter().map(|x| x.color));
            v.sources.extend(b.sources.iter().copied());
        }
        Some(Arc::new(v))
    }

    /// The colours with those of the vertices loaded from the array at `base` (in segment 6)
    /// replaced by `ram`'s colour words.
    pub fn with_ram(&self, base: u32, ram: &[u8]) -> Vec<[u8; 4]> {
        let base = 0x0600_0000 | base;
        self.colors
            .iter()
            .zip(self.sources.iter().copied().map(Some).chain(std::iter::repeat(None)))
            .map(|(&c, s)| match s.and_then(|s| s.checked_sub(base)).map(|o| o as usize / 16) {
                Some(i) if (i + 1) * 4 <= ram.len() => [ram[i * 4], ram[i * 4 + 1], ram[i * 4 + 2], ram[i * 4 + 3]],
                _ => c,
            })
            .collect()
    }

    /// The colour words of the array at `base`, `count` vertices, as this bake loads them (a
    /// vertex it doesn't load: zeros).
    pub fn ram_of(&self, base: u32, count: usize) -> Vec<u8> {
        let base = 0x0600_0000 | base;
        let mut ram = vec![0u8; count * 4];
        for (&c, &s) in self.colors.iter().zip(&self.sources) {
            if let Some(i) = s.checked_sub(base).map(|o| o as usize / 16)
                && i < count
            {
                ram[i * 4..i * 4 + 4].copy_from_slice(&c);
            }
        }
        ram
    }
}

/// The overlay's statics: `sSfxJewelId`, and what the time warps load once: the curve skeleton
/// and animation, the bakes' vertices.
#[derive(Debug, Default)]
struct Statics {
    sfx_jewel_id: i16,
    time_warp: Option<(Arc<CurveSkeleton>, Arc<CurveAnimation>)>,
    time_warp_vertices: Option<Arc<BakeVertices>>,
    triforce_column_vertices: Option<Arc<BakeVertices>>,
}

fn statics(play: &mut PlayState) -> &mut Statics {
    play.overlay_static::<Statics>(ACTOR_DEMO_EFFECT)
}

pub struct DemoEffect {
    pub actor: Actor,
    pub skel_curve: SkelCurve,
    /// `requiredObjectSlot`: the type's object's bank.
    pub required_object_slot: Option<usize>,
    /// `jewelDisplayList`, `jewelHolderDisplayList`: the jewel's bakes.
    pub jewel_bakes: Option<(&'static str, &'static str)>,
    pub prim_xlu_color: [u8; 3],
    pub env_xlu_color: [u8; 3],
    pub prim_opa_color: [u8; 3],
    pub env_opa_color: [u8; 3],
    pub u: Union,
    pub effect_flags: i16,
    pub cue_channel: i16,
    pub jewel_cs_rotation: [i16; 3],
    pub init_update_func: UpdateFunc,
    pub init_draw_func: Option<DrawFunc>,
    pub update_func: UpdateFunc,
    /// `actor.draw`: none until the object is loaded.
    pub draw: Option<DrawFunc>,
    /// The bakes' vertices the draws rebuild colours on.
    vertices: Option<Arc<BakeVertices>>,
    /// This frame's draw returned before drawing: the light's first (`light.flicker` set), a
    /// medal's first (`getItem.isLoaded` set). `draw_update` makes the change, `draw` skips.
    draw_skipped: bool,
}

/// The type: `PARAMS_GET_S(params, 0, 8)`.
fn effect_type(params: i16) -> u8 {
    (params & 0xFF) as u8
}

impl DemoEffect {
    pub fn effect_type(&self) -> u8 {
        effect_type(self.actor.params)
    }

    /// The cue on this actor's channel while a cutscene runs.
    fn cue(&self, play: &PlayState) -> Option<CsCmdActorCue> {
        cue_on(play, self.cue_channel as usize)
    }

    /// `DemoEffect_InitJewel`.
    fn init_jewel(&mut self, play: &mut PlayState) {
        self.init_draw_func = Some(DrawFunc::Jewel);
        self.init_update_func = if !play.save.adult { UpdateFunc::JewelChild } else { UpdateFunc::JewelAdult };
        if play.scene_id == SCENE_TEMPLE_OF_TIME {
            self.actor.scale = Vec3::splat(0.35);
        } else {
            self.actor.scale = Vec3::splat(0.10);
        }
        self.cue_channel = 1;
        self.actor.shape_rot.x = 16384;
        self.init_jewel_color();
        *self.u.jewel_alpha_mut() = 0;
        self.jewel_cs_rotation = [0; 3];
        statics(play).sfx_jewel_id = 0;
    }

    /// `DemoEffect_InitGetItem`.
    fn init_get_item(&mut self) {
        *self.u.get_item_is_position_init_mut() = 0;
        *self.u.get_item_is_loaded_mut() = 0;
        self.init_draw_func = Some(DrawFunc::GetItem);
        self.init_update_func = UpdateFunc::GetItem;
        self.actor.scale = Vec3::splat(0.25);
        self.cue_channel = 6;
    }

    /// `DemoEffect_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = DemoEffect {
            actor,
            skel_curve: SkelCurve::default(),
            required_object_slot: None,
            jewel_bakes: None,
            prim_xlu_color: [0; 3],
            env_xlu_color: [0; 3],
            prim_opa_color: [0; 3],
            env_opa_color: [0; 3],
            u: Union::default(),
            effect_flags: 0,
            cue_channel: 0,
            jewel_cs_rotation: [0; 3],
            init_update_func: UpdateFunc::None,
            init_draw_func: None,
            update_func: UpdateFunc::None,
            draw: None,
            vertices: None,
            draw_skipped: false,
        };
        let ty = this.effect_type();
        // PARAMS_GET_S(params, 12, 4).
        let light_effect = ((this.actor.params >> 12) & 0xF) as u8;
        log::debug!(" no = {ty}");
        let Some(&object) = S_EFFECT_TYPE_OBJECTS.get(ty as usize) else {
            log::error!("Demo_Effect: type {ty:#x} out of range (ASSERT in z_demo_effect.c)");
            return Box::new(this);
        };
        let slot = if object == OBJECT_GAMEPLAY_KEEP { play.object_ctx.get_index(OBJECT_GAMEPLAY_KEEP) } else { play.object_ctx.get_index(object) };
        log::debug!(" bank_ID = {slot:?}");
        match slot {
            None => log::error!("Demo_Effect: object {object:#x} isn't loaded (ASSERT in z_demo_effect.c)"),
            Some(s) => this.required_object_slot = Some(s),
        }
        this.effect_flags = 0;
        this.actor.scale = Vec3::splat(0.2);
        match ty {
            DEMO_EFFECT_CRYSTAL_LIGHT => {
                this.init_draw_func = Some(DrawFunc::CrystalLight);
                this.init_update_func = UpdateFunc::CrystalLight;
            }
            DEMO_EFFECT_FIRE_BALL => {
                this.init_draw_func = Some(DrawFunc::FireBall);
                this.init_update_func = UpdateFunc::PositionToParent;
                this.actor.scale = Vec3::splat(0.1);
            }
            DEMO_EFFECT_BLUE_ORB => {
                this.init_draw_func = Some(DrawFunc::BlueOrb);
                this.init_update_func = UpdateFunc::BlueOrbGrow;
                *this.u.blue_orb_alpha_mut() = 255;
                *this.u.blue_orb_scale_mut() = 5;
                *this.u.blue_orb_rotation_mut() = 0;
                this.actor.scale = Vec3::splat(0.05);
                this.prim_xlu_color = [188, 255, 255];
                this.env_xlu_color = [0, 100, 255];
            }
            DEMO_EFFECT_LIGHT => {
                this.init_draw_func = Some(DrawFunc::LightEffect);
                this.init_update_func = UpdateFunc::LightEffect;
                *this.u.light_alpha_mut() = 255;
                *this.u.light_scale_flag_mut() = 0;
                *this.u.light_flicker_mut() = 0;
                *this.u.light_rotation_mut() = 0;
                match light_effect {
                    DEMO_EFFECT_LIGHT_RED => {
                        this.prim_xlu_color = [255, 255, 255];
                        this.env_xlu_color = [255, 50, 0];
                    }
                    DEMO_EFFECT_LIGHT_BLUE => {
                        this.prim_xlu_color = [255, 255, 255];
                        this.env_xlu_color = [0, 150, 255];
                    }
                    DEMO_EFFECT_LIGHT_GREEN => {
                        this.prim_xlu_color = [255, 255, 255];
                        this.env_xlu_color = [0, 200, 0];
                    }
                    DEMO_EFFECT_LIGHT_ORANGE => {
                        this.prim_xlu_color = [255, 255, 255];
                        this.env_xlu_color = [255, 150, 0];
                    }
                    DEMO_EFFECT_LIGHT_YELLOW => {
                        this.prim_xlu_color = [255, 255, 255];
                        this.env_xlu_color = [200, 255, 0];
                    }
                    DEMO_EFFECT_LIGHT_PURPLE => {
                        this.prim_xlu_color = [255, 255, 255];
                        this.env_xlu_color = [200, 50, 255];
                    }
                    DEMO_EFFECT_LIGHT_GREEN2 => {
                        this.prim_xlu_color = [255, 255, 255];
                        this.env_xlu_color = [0, 200, 0];
                    }
                    _ => {}
                }
                this.cue_channel = 7;
                this.actor.scale = Vec3::ZERO;
            }
            DEMO_EFFECT_LGT_SHOWER => {
                *this.u.lgt_shower_alpha_mut() = 255;
                this.init_draw_func = Some(DrawFunc::LgtShower);
                this.init_update_func = UpdateFunc::LgtShower;
            }
            DEMO_EFFECT_GOD_LGT_DIN => {
                this.actor.scale = Vec3::splat(0.1);
                this.init_draw_func = Some(DrawFunc::GodLgt);
                this.prim_xlu_color = [255, 170, 255];
                this.env_xlu_color = [255, 0, 255];
                *this.u.god_lgt_type_mut() = GOD_LGT_DIN;
                *this.u.god_lgt_rotation_mut() = 0;
                this.init_update_func = UpdateFunc::GodLgtDin;
                this.cue_channel = 0;
            }
            DEMO_EFFECT_GOD_LGT_NAYRU => {
                this.actor.scale = Vec3::splat(if play.save.entrance_index == ENTR_DEATH_MOUNTAIN_TRAIL_0 { 1.0 } else { 0.1 });
                this.init_draw_func = Some(DrawFunc::GodLgt);
                this.prim_xlu_color = [170, 255, 255];
                this.env_xlu_color = [0, 40, 255];
                *this.u.god_lgt_type_mut() = GOD_LGT_NAYRU;
                *this.u.god_lgt_light_ring_spawn_delay_mut() = 4;
                *this.u.god_lgt_rotation_mut() = 0;
                *this.u.god_lgt_light_ring_spawn_timer_mut() = 0;
                this.init_update_func = UpdateFunc::GodLgtNayru;
                this.cue_channel = 1;
            }
            DEMO_EFFECT_GOD_LGT_FARORE => {
                this.actor.scale = Vec3::splat(if play.save.entrance_index == ENTR_KOKIRI_FOREST_0 { 2.4 } else { 0.1 });
                this.init_draw_func = Some(DrawFunc::GodLgt);
                this.prim_xlu_color = [170, 255, 170];
                this.env_xlu_color = [0, 200, 0];
                *this.u.god_lgt_type_mut() = GOD_LGT_FARORE;
                *this.u.god_lgt_rotation_mut() = 0;
                this.init_update_func = UpdateFunc::GodLgtFarore;
                this.cue_channel = 2;
            }
            DEMO_EFFECT_LIGHTRING_EXPANDING => {
                this.init_draw_func = Some(DrawFunc::LightRing);
                this.init_update_func = UpdateFunc::LightRingExpanding;
                // FRAMERATE_CONST(20, 6), FRAMERATE_CONST(4, 5): this ROM's 60 Hz values.
                *this.u.light_ring_timer_mut() = 20;
                *this.u.light_ring_timer_increment_mut() = 4;
                *this.u.light_ring_alpha_mut() = 255;
            }
            DEMO_EFFECT_LIGHTRING_TRIFORCE => {
                this.init_draw_func = Some(DrawFunc::LightRing);
                this.init_update_func = UpdateFunc::LightRingTriforce;
                *this.u.light_ring_timer_mut() = 20;
                *this.u.light_ring_timer_increment_mut() = 4;
                *this.u.light_ring_alpha_mut() = 0;
                this.cue_channel = 4;
            }
            DEMO_EFFECT_LIGHTRING_SHRINKING => {
                this.init_draw_func = Some(DrawFunc::LightRing);
                this.init_update_func = UpdateFunc::LightRingShrinking;
                // FRAMERATE_CONST(351, 405), FRAMERATE_CONST(2, 3).
                *this.u.light_ring_timer_mut() = 351;
                *this.u.light_ring_timer_increment_mut() = 2;
                *this.u.light_ring_alpha_mut() = 0;
            }
            DEMO_EFFECT_TRIFORCE_SPOT => {
                this.init_draw_func = Some(DrawFunc::TriforceSpot);
                this.init_update_func = UpdateFunc::TriforceSpot;
                *this.u.crystal_light_opacity_mut() = 0;
                *this.u.light_column_opacity_mut() = 0;
                *this.u.triforce_spot_opacity_mut() = 0;
                *this.u.triforce_spot_rotation_mut() = 0;
                this.prim_xlu_color[0] = 0;
                this.cue_channel = 3;
                this.actor.scale = Vec3::splat(0.020);
                this.vertices = triforce_column_vertices(play);
                let pos = this.actor.world_pos;
                // The crystal light, the init's child; the light ring, the crystal light's.
                let crystal = play.actor_spawn_as_child(&mut this.actor, ACTOR_DEMO_EFFECT, pos, [0, 0, 0], DEMO_EFFECT_CRYSTAL_LIGHT as i16).ok();
                if let Some(a) = crystal.and_then(|h| play.actors.actor_mut(h)) {
                    a.scale = Vec3::splat(0.6);
                }
                // Actor_SpawnAsChild(&crystalLight->actor, ...): its parent is the crystal light.
                let ring = play.actor_spawn(ACTOR_DEMO_EFFECT, pos, [0, 0, 0], DEMO_EFFECT_LIGHTRING_TRIFORCE as i16).ok();
                if let (Some(c), Some(r)) = (crystal, ring) {
                    if let Some(a) = play.actors.actor_mut(c) {
                        a.child = Some(r);
                    }
                    if let Some(a) = play.actors.actor_mut(r) {
                        a.parent = Some(c);
                        a.room = this.actor.room;
                    }
                }
                if let Some(a) = ring.and_then(|h| play.actors.actor_mut(h)) {
                    a.scale = Vec3::splat(0.4);
                }
            }
            DEMO_EFFECT_MEDAL_FIRE => {
                this.init_get_item();
                *this.u.get_item_draw_id_mut() = GID_MEDALLION_FIRE;
            }
            DEMO_EFFECT_MEDAL_WATER => {
                this.init_get_item();
                *this.u.get_item_draw_id_mut() = GID_MEDALLION_WATER;
            }
            DEMO_EFFECT_MEDAL_FOREST => {
                this.init_get_item();
                *this.u.get_item_draw_id_mut() = GID_MEDALLION_FOREST;
            }
            DEMO_EFFECT_MEDAL_SPIRIT => {
                this.init_get_item();
                *this.u.get_item_draw_id_mut() = GID_MEDALLION_SPIRIT;
            }
            DEMO_EFFECT_MEDAL_SHADOW => {
                this.init_get_item();
                *this.u.get_item_draw_id_mut() = GID_MEDALLION_SHADOW;
            }
            DEMO_EFFECT_MEDAL_LIGHT => {
                this.init_get_item();
                *this.u.get_item_draw_id_mut() = GID_MEDALLION_LIGHT;
            }
            DEMO_EFFECT_LIGHTARROW => {
                this.init_get_item();
                *this.u.get_item_draw_id_mut() = GID_ARROW_LIGHT;
            }
            DEMO_EFFECT_TIMEWARP_TIMEBLOCK_LARGE | DEMO_EFFECT_TIMEWARP_TIMEBLOCK_SMALL | DEMO_EFFECT_TIMEWARP_MASTERSWORD => {
                if ty != DEMO_EFFECT_TIMEWARP_MASTERSWORD {
                    this.actor.flags |= ACTOR_FLAG_UPDATE_DURING_OCARINA;
                }
                this.init_draw_func = Some(DrawFunc::TimeWarp);
                this.init_update_func = UpdateFunc::InitTimeWarp;
                this.env_xlu_color = [0, 100, 255];
                this.skel_curve.clear();
                *this.u.time_warp_shrink_timer_mut() = 0;
            }
            DEMO_EFFECT_JEWEL_KOKIRI => {
                this.jewel_bakes = Some((BAKE_JEWELS[0].0, BAKE_JEWELS[0].1));
                *this.u.jewel_type_mut() = DEMO_EFFECT_JEWEL_KOKIRI;
                *this.u.jewel_is_position_init_mut() = 0;
                this.init_jewel(play);
            }
            DEMO_EFFECT_JEWEL_GORON => {
                this.jewel_bakes = Some((BAKE_JEWELS[1].0, BAKE_JEWELS[1].1));
                *this.u.jewel_type_mut() = DEMO_EFFECT_JEWEL_GORON;
                *this.u.jewel_is_position_init_mut() = 0;
                this.init_jewel(play);
            }
            DEMO_EFFECT_JEWEL_ZORA => {
                this.jewel_bakes = Some((BAKE_JEWELS[2].0, BAKE_JEWELS[2].1));
                *this.u.jewel_type_mut() = DEMO_EFFECT_JEWEL_ZORA;
                *this.u.jewel_is_position_init_mut() = 0;
                this.init_jewel(play);
                // Actor_ChangeCategory(.., ACTORCAT_BOSS).
                this.actor.category = ACTORCAT_BOSS;
                if play.scene_id == SCENE_JABU_JABU && play.save.get_inf_table(INFTABLE_RUTO_HAS_SAPPHIRE) {
                    this.actor.kill();
                    return Box::new(this);
                }
            }
            DEMO_EFFECT_DUST => {
                this.init_draw_func = None;
                this.init_update_func = UpdateFunc::Dust;
                *this.u.dust_timer_mut() = 0;
                this.cue_channel = 2;
            }
            _ => log::error!("Demo_Effect: type {ty:#x} (ASSERT in z_demo_effect.c)"),
        }
        // ActorShape_Init(&shape, 0, NULL, 0).
        this.actor.shape_y_offset = 0.0;
        this.update_func = UpdateFunc::WaitForObject;
        Box::new(this)
    }

    /// `DemoEffect_WaitForObject`: once the object is in, the type's update and draw.
    fn wait_for_object(&mut self, play: &mut PlayState) {
        if self.required_object_slot.is_some_and(|s| play.object_ctx.is_loaded(s)) {
            self.actor.obj_bank_index = self.required_object_slot;
            self.draw = self.init_draw_func;
            self.update_func = self.init_update_func;
            log::debug!(" Transfer completed move_wait ");
        }
    }

    /// The parent's position (`actor.parent`), if it's still there.
    fn parent_pos(&self, play: &PlayState) -> Option<Vec3> {
        self.actor.parent.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos)
    }

    /// `DemoEffect_UpdatePositionToParent`.
    fn update_position_to_parent(&mut self, play: &PlayState) {
        if let Some(p) = self.parent_pos(play) {
            self.actor.world_pos = p;
        }
    }

    /// `DemoEffect_UpdateCrystalLight`: on the Triforce, rising 14 a frame (into the sky once
    /// it's gone).
    fn update_crystal_light(&mut self, play: &PlayState) {
        self.update_position_to_parent(play);
        self.actor.world_pos.y += 14.0;
    }

    /// `DemoEffect_MedalSparkle`.
    fn medal_sparkle(&self, play: &mut PlayState, is_small_spawner: bool) {
        if !is_small_spawner || play.gameplay_frames & 1 == 0 {
            let r = &mut play.rand;
            if is_small_spawner {
                let (_vx, _vz) = (r.zero_one() - 0.5, r.zero_one() - 0.5);
            } else {
                let (_vx, _vz) = ((r.zero_one() - 0.5) * 2.0, (r.zero_one() - 0.5) * 2.0);
            }
            let _pos = Vec3::new(r.centered_float(10.0), r.centered_float(10.0), r.centered_float(10.0)) + self.actor.world_pos;
            kira_kira_spawn_dispersed(play);
        }
    }

    /// `DemoEffect_UpdateGetItem`: the medals and the light arrows.
    fn update_get_item(&mut self, play: &mut PlayState) {
        let Some(cue) = self.cue(play) else { return };
        if self.u.get_item_is_position_init() != 0 {
            self.move_toward_cue_pos(&cue, 0.1);
        } else {
            self.set_start_pos_from_cue(&cue);
            *self.u.get_item_is_position_init_mut() = 1;
        }
        if self.u.get_item_draw_id() != GID_ARROW_LIGHT {
            self.actor.shape_rot.x = 0xE0C0u16 as i16;
        } else {
            self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x0400);
        }
        self.actor.scale = Vec3::splat(0.20);
        let tot = play.save.entrance_index == ENTR_TEMPLE_OF_TIME_0;
        if tot {
            match cue.action {
                2 => self.medal_sparkle(play, false),
                3 => self.medal_sparkle(play, true),
                _ => {}
            }
        }
        match cue.action {
            2 => {
                if tot {
                    audio_play_actor_sfx2(play, NA_SE_EV_MEDAL_APPEAR_L - SFX_FLAG);
                } else {
                    play.audio.play_sfx_centered2(NA_SE_EV_MEDAL_APPEAR_S - SFX_FLAG);
                }
                if self.u.get_item_draw_id() != GID_ARROW_LIGHT {
                    self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x3E80);
                }
                *self.u.get_item_rotation_mut() = 0x3E80;
            }
            3 => {
                let r = self.u.get_item_rotation();
                *self.u.get_item_rotation_mut() = r.wrapping_sub(((r as i32 - 0x03E8) as f32 * 0.10) as i16);
                if self.u.get_item_draw_id() != GID_ARROW_LIGHT {
                    self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(self.u.get_item_rotation());
                }
                if tot {
                    audio_play_actor_sfx2(play, NA_SE_EV_MEDAL_APPEAR_L - SFX_FLAG);
                } else {
                    play.audio.play_sfx_centered2(NA_SE_EV_MEDAL_APPEAR_S - SFX_FLAG);
                }
            }
            4 => audio_play_actor_sfx2(play, NA_SE_EV_MEDAL_APPEAR_S - SFX_FLAG),
            _ => {}
        }
    }

    /// `DemoEffect_InitTimeWarp`: the curve skeleton and its animation; the Song of Time blocks'
    /// grow once (frames 1 to 59 at 1.7), the cutscene layers' and the Chamber of Sages' return
    /// start at its end, the Master Sword's plays when its cutscene flag is set.
    fn init_time_warp(&mut self, play: &mut PlayState) {
        let ty = self.effect_type();
        let Some((skel, anim)) = time_warp_curves(play) else {
            log::error!("Demo_Effect: gTimeWarpSkel or gTimeWarpAnim missing from the pack (ASSERT in z_demo_effect.c)");
            return;
        };
        self.skel_curve.init(skel);
        self.vertices = time_warp_vertices(play);
        let rate = play.r_update_rate as i32;
        if ty == DEMO_EFFECT_TIMEWARP_TIMEBLOCK_LARGE || ty == DEMO_EFFECT_TIMEWARP_TIMEBLOCK_SMALL {
            self.skel_curve.set_anim(anim, 1.0, 59.0, 1.0, 1.7);
            self.skel_curve.update(rate);
            self.update_func = UpdateFunc::InitTimeWarpTimeblock;
            self.actor.scale = Vec3::splat(if ty == DEMO_EFFECT_TIMEWARP_TIMEBLOCK_LARGE { 0.14 } else { 84.0 * 0.001 });
        } else if play.save.scene_layer == 5 || play.save.scene_layer == 4 || (play.save.entrance_index == ENTR_TEMPLE_OF_TIME_4 && !play.save.get_event_chk_inf(EVENTCHKINF_C9)) {
            self.skel_curve.set_anim(anim, 1.0, 59.0, 59.0, 0.0);
            self.skel_curve.update(rate);
            self.update_func = UpdateFunc::TimeWarpReturnFromChamberOfSages;
            log::debug!(" Shrinking version ");
        } else {
            self.skel_curve.set_anim(anim, 1.0, 59.0, 1.0, 1.0);
            self.skel_curve.update(rate);
            self.update_func = UpdateFunc::TimeWarpPullMasterSword;
            log::debug!(" Normal version ");
        }
    }

    /// `DemoEffect_UpdateTimeWarpPullMasterSword`: with cutscene flag 1, the sword's glow
    /// (`SEQ_CS_EFFECTS_SWORD_GLOW`, once) and the warp's growth, then held at its end.
    fn update_time_warp_pull_master_sword(&mut self, play: &mut PlayState) {
        if play.flags_get_env(1) {
            if self.effect_flags & 0x2 == 0 {
                play.audio.play_cutscene_effects_sequence(SEQ_CS_EFFECTS_SWORD_GLOW);
                self.effect_flags |= 0x2;
            }
            if self.skel_curve.update(play.r_update_rate as i32)
                && let Some((_, anim)) = time_warp_curves(play)
            {
                self.skel_curve.set_anim(anim, 1.0, 60.0, 59.0, 0.0);
            }
        }
    }

    /// `DemoEffect_TimewarpShrink`: `gTimeWarpVtx`'s alphas by `size` (202 and 255 times it, by
    /// `sTimewarpVertexSizeIndices`), written into the object's RAM: every time warp reads them.
    fn timewarp_shrink(&self, play: &mut PlayState, size: f32) {
        let Some(bank) = self.required_object_slot else { return };
        let vertices = self.vertices.clone();
        let ram = play.object_ctx.written_mut(bank, TIME_WARP_VTX, || vertices.map(|v| v.ram_of(TIME_WARP_VTX, TIME_WARP_VTX_COUNT)).unwrap_or_else(|| vec![0; TIME_WARP_VTX_COUNT * 4]));
        let sizes = [0u8, (202.0 * size) as i32 as u8, (255.0 * size) as i32 as u8];
        for (i, &k) in S_TIMEWARP_VERTEX_SIZE_INDICES.iter().enumerate() {
            if k != 0 {
                ram[i * 4 + 3] = sizes[k as usize];
            }
        }
    }

    /// `DemoEffect_UpdateTimeWarpReturnFromChamberOfSages`: 250 frames; past 100 it narrows and
    /// fades (`DemoEffect_TimewarpShrink`); at its end `EVENTCHKINF_C9` (from the Temple of Time's
    /// entrance 4) and gone. `NA_SE_EV_TIMETRIP_LIGHT` every frame.
    fn update_time_warp_return_from_chamber_of_sages(&mut self, play: &mut PlayState) {
        *self.u.time_warp_shrink_timer_mut() += 1;
        if self.u.time_warp_shrink_timer() > 250 {
            if play.save.entrance_index == ENTR_TEMPLE_OF_TIME_4 {
                play.save.set_event_chk_inf(EVENTCHKINF_C9);
            }
            self.actor.kill();
            return;
        }
        if self.u.time_warp_shrink_timer() > 100 {
            let shrink_progress = (250 - self.u.time_warp_shrink_timer()) as f32 * (1.0 / 750.0);
            self.actor.scale.x = shrink_progress;
            self.actor.scale.z = shrink_progress;
            self.timewarp_shrink(play, shrink_progress * 5.0);
        }
        self.actor.play_sfx_flagged_centered2(NA_SE_EV_TIMETRIP_LIGHT - SFX_FLAG);
    }

    /// `DemoEffect_UpdateTimeWarpTimeblock`: 100 frames narrowing and fading, then the alphas put
    /// back and gone.
    fn update_time_warp_timeblock(&mut self, play: &mut PlayState) {
        *self.u.time_warp_shrink_timer_mut() += 1;
        if self.u.time_warp_shrink_timer() <= 100 {
            let shrink_progress = (100 - self.u.time_warp_shrink_timer()) as f32 * 0.010;
            let mut scale = shrink_progress * 0.14;
            if self.effect_type() == DEMO_EFFECT_TIMEWARP_TIMEBLOCK_SMALL {
                scale *= 0.6;
            }
            self.actor.scale.x = scale;
            self.actor.scale.z = scale;
            self.timewarp_shrink(play, shrink_progress);
            self.actor.play_sfx_flagged_centered2(NA_SE_EV_TIMETRIP_LIGHT - SFX_FLAG);
            return;
        }
        self.timewarp_shrink(play, 1.0);
        self.actor.kill();
    }

    /// `DemoEffect_InitTimeWarpTimeblock`: grows (its curve), then shrinks.
    fn init_time_warp_timeblock(&mut self, play: &mut PlayState) {
        self.actor.play_sfx_flagged_centered2(NA_SE_EV_TIMETRIP_LIGHT - SFX_FLAG);
        if self.skel_curve.update(play.r_update_rate as i32) {
            if let Some((_, anim)) = time_warp_curves(play) {
                self.skel_curve.set_anim(anim, 1.0, 60.0, 59.0, 0.0);
            }
            self.update_func = UpdateFunc::TimeWarpTimeblock;
            *self.u.time_warp_shrink_timer_mut() = 0;
        }
    }

    /// `DemoEffect_UpdateTriforceSpot`: turning; on its cue 2 `primXluColor[0]` counts to 140,
    /// fading in the Triforce (to 30), the column (to 60), then the crystal light; the ring's
    /// explosion sound at frame 143 of the cutscene map's layer 6.
    fn update_triforce_spot(&mut self, play: &mut PlayState) {
        let r = self.u.triforce_spot_rotation();
        *self.u.triforce_spot_rotation_mut() = r.wrapping_add(0x03E8);
        let Some(cue) = self.cue(play) else { return };
        self.set_pos_rot_from_cue(play, &cue, false);
        if cue.action == 2 {
            if self.prim_xlu_color[0] < 140 {
                self.prim_xlu_color[0] += 1;
            }
            let c = self.prim_xlu_color[0] as i32;
            if c < 30 {
                *self.u.triforce_spot_opacity_mut() = (c as f32 * 8.5) as i32 as u8;
            } else {
                *self.u.triforce_spot_opacity_mut() = 255;
                if c < 60 {
                    *self.u.light_column_opacity_mut() = ((c - 30) as f32 * 8.5) as i32 as u8;
                } else if c <= 140 {
                    *self.u.light_column_opacity_mut() = 255;
                    *self.u.crystal_light_opacity_mut() = ((c - 60) as f32 * 3.1875) as i32 as u8;
                }
            }
        }
        // FRAMERATE_CONST(143, 120).
        if play.save.entrance_index == ENTR_CUTSCENE_MAP_0 && play.save.scene_layer == 6 && play.cs_ctx.frames == 143 {
            audio_play_actor_sfx2(play, NA_SE_IT_DM_RING_EXPLOSION);
        }
    }

    /// `DemoEffect_UpdateLightRingShrinking`: Din's ring closing in, fading in over its last 30;
    /// `SEQ_CS_EFFECTS_DIN_MAGIC` at 255.
    fn update_light_ring_shrinking(&mut self, play: &mut PlayState) {
        let (timer, inc) = (self.u.light_ring_timer(), self.u.light_ring_timer_increment() as i16);
        if timer < inc {
            self.actor.kill();
            *self.u.light_ring_timer_mut() = 0;
        } else {
            *self.u.light_ring_timer_mut() = timer - inc;
        }
        let timer = self.u.light_ring_timer();
        if timer <= 255 {
            *self.u.light_ring_alpha_mut() = if timer >= 225 { ((-(timer as i32)) * 8 + 2048) as u8 } else { 255 };
        }
        if timer == 255 {
            play.audio.play_cutscene_effects_sequence(SEQ_CS_EFFECTS_DIN_MAGIC);
        }
    }

    /// `DemoEffect_UpdateLightRingExpanding`: on its parent, widening and fading out past 225,
    /// gone past 255.
    fn update_light_ring_expanding(&mut self, play: &PlayState) {
        self.update_position_to_parent(play);
        let timer = self.u.light_ring_timer().wrapping_add(self.u.light_ring_timer_increment() as i16);
        *self.u.light_ring_timer_mut() = timer;
        if timer >= 225 {
            *self.u.light_ring_alpha_mut() = ((-(timer as i32)) * 8 + 2048) as u8;
        }
        if timer > 255 {
            *self.u.light_ring_timer_mut() = 255;
            self.actor.kill();
            *self.u.light_ring_timer_mut() = 0;
        }
    }

    /// `DemoEffect_UpdateLightRingTriforce`: on its parent; on its cue 2, a blue orb, and on as an
    /// expanding ring.
    fn update_light_ring_triforce(&mut self, play: &mut PlayState) {
        self.update_position_to_parent(play);
        if play.cs_ctx.state != CS_STATE_IDLE
            && let Some(cue) = self.cue(play)
            && cue.action == 2
        {
            let pos = self.actor.world_pos;
            if let Some(a) = play.actor_spawn(ACTOR_DEMO_EFFECT, pos, [0, 0, 0], DEMO_EFFECT_BLUE_ORB as i16).ok().and_then(|h| play.actors.actor_mut(h)) {
                a.scale = Vec3::ZERO;
            }
            self.update_func = UpdateFunc::LightRingExpanding;
            *self.u.light_ring_alpha_mut() = 255;
        }
    }

    /// `DemoEffect_UpdateCreationFireball`: falling forward; when its timer runs out, a blue orb,
    /// an expanding and a shrinking ring, `NA_SE_IT_DM_RING_EXPLOSION`, and gone.
    fn update_creation_fireball(&mut self, play: &mut PlayState) {
        self.actor.move_forward();
        self.actor.speed_xz += self.actor.gravity * 0.5;
        if self.u.fire_ball_timer() != 0 {
            *self.u.fire_ball_timer_mut() -= 1;
            return;
        }
        let pos = self.actor.world_pos;
        for (ty, scale) in [(DEMO_EFFECT_BLUE_ORB, 0.0), (DEMO_EFFECT_LIGHTRING_EXPANDING, 0.1), (DEMO_EFFECT_LIGHTRING_SHRINKING, 0.2)] {
            if let Some(a) = play.actor_spawn(ACTOR_DEMO_EFFECT, pos, [0, 0, 0], ty as i16).ok().and_then(|h| play.actors.actor_mut(h)) {
                a.scale = Vec3::splat(scale);
            }
        }
        play.audio.play_sfx_centered2(NA_SE_IT_DM_RING_EXPLOSION);
        self.actor.kill();
    }

    /// `DemoEffect_InitCreationFireball`: Din's fire ball's start: her facing, 50 frames, speed
    /// 1.5, gravity -0.03 to -1.5 (this ROM's values).
    fn init_creation_fireball(&mut self, play: &PlayState) {
        if let Some(p) = self.actor.parent.and_then(|h| play.actors.actor(h)) {
            self.actor.world_rot.y = p.shape_rot.y;
        }
        // FRAMERATE_CONST(50, 42), FRAMERATE_CONST(1.5f, 1.8f); not OOT_VERSION < PAL_1_0:
        // FRAMERATE_CONST(-1.5f, -2.5f), FRAMERATE_CONST(-0.03f, -0.05f).
        *self.u.fire_ball_timer_mut() = 50;
        self.actor.speed_xz = 1.5;
        self.actor.min_velocity_y = -1.5;
        self.actor.gravity = -0.03;
        self.update_func = UpdateFunc::CreationFireball;
    }

    /// `DemoEffect_UpdateBlueOrbShrink`.
    fn update_blue_orb_shrink(&mut self) {
        *self.u.blue_orb_alpha_mut() = self.u.blue_orb_scale().wrapping_mul(16);
        *self.u.blue_orb_scale_mut() = self.u.blue_orb_scale().wrapping_sub(1);
        self.actor.scale = Vec3::splat(self.actor.scale.x * 0.9);
        if self.u.blue_orb_scale() == 0 {
            self.actor.kill();
        }
    }

    /// `DemoEffect_UpdateBlueOrbGrow`: 5 frames growing (by its parent's scale if it has one),
    /// then shrinking.
    fn update_blue_orb_grow(&mut self, play: &PlayState) {
        let s = self.u.blue_orb_scale() as i32;
        let parent_scale = self.actor.parent.and_then(|h| play.actors.actor(h)).map(|a| a.scale.x);
        let scale = match parent_scale {
            Some(ps) => ((5.0 - s as f32) * 0.01) * 10.0 * ps,
            None => (5.0 - s as f32) * 0.01,
        };
        self.actor.scale = Vec3::splat(scale);
        if s != 0 {
            *self.u.blue_orb_scale_mut() -= 1;
        } else {
            *self.u.blue_orb_scale_mut() = 15;
            self.update_func = UpdateFunc::BlueOrbShrink;
        }
    }

    /// `DemoEffect_UpdateLightEffect`: placed by its cue; cue 2 grows it (to 0.23, or 2.03 for a
    /// large one) and turns it, cue 3 shrinks it away; its scenes' sounds.
    fn update_light_effect(&mut self, play: &mut PlayState) {
        // PARAMS_GET_S(params, 8, 4).
        let is_large_size = (self.actor.params >> 8) & 0xF != 0;
        let Some(cue) = self.cue(play) else { return };
        self.set_pos_rot_from_cue(play, &cue, false);
        match cue.action {
            2 => {
                if self.u.light_rotation() < 240 {
                    if !is_large_size {
                        if self.actor.scale.x < 0.23 {
                            self.actor.scale.x += 0.001;
                            self.actor.scale = Vec3::splat(self.actor.scale.x);
                        }
                    } else if self.actor.scale.x < 2.03 {
                        self.actor.scale.x += 0.05;
                        self.actor.scale = Vec3::splat(self.actor.scale.x);
                    }
                }
                *self.u.light_rotation_mut() = self.u.light_rotation().wrapping_add(6);
                *self.u.light_scale_flag_mut() = self.u.light_scale_flag().wrapping_add(1);
            }
            3 => {
                smooth_step_to_f(&mut self.actor.scale.x, 0.0, 0.1, 0.1, 0.005);
                self.actor.scale = Vec3::splat(self.actor.scale.x);
            }
            _ => {}
        }
        let (scene, layer, frame) = (play.scene_id, play.save.scene_layer, play.cs_ctx.frames);
        if scene == SCENE_KOKIRI_FOREST && layer == 6 && frame == 197 {
            audio_play_actor_sfx2(play, NA_SE_EV_WHITE_OUT);
        }
        if scene == SCENE_DEATH_MOUNTAIN_TRAIL && layer == 5 {
            if !self.check_for_cue(play, 1) {
                audio_play_actor_sfx2(play, NA_SE_EV_LIGHT_GATHER - SFX_FLAG);
            }
            if frame == 640 {
                audio_play_actor_sfx2(play, NA_SE_EV_WHITE_OUT);
            }
        }
        if scene == SCENE_ZORAS_FOUNTAIN && layer == 4 {
            if !self.check_for_cue(play, 1) {
                audio_play_actor_sfx2(play, NA_SE_EV_LIGHT_GATHER - SFX_FLAG);
            }
            if frame == 648 {
                audio_play_actor_sfx2(play, NA_SE_EV_WHITE_OUT);
            }
        }
        if scene == SCENE_TEMPLE_OF_TIME && layer == 14 && cue.action == 2 {
            audio_play_actor_sfx2(play, NA_SE_EV_LIGHT_GATHER - SFX_FLAG);
        }
        if (scene == SCENE_GREAT_FAIRYS_FOUNTAIN_MAGIC || scene == SCENE_GREAT_FAIRYS_FOUNTAIN_SPELLS) && cue.action == 2 {
            audio_play_actor_sfx2(play, NA_SE_EV_LIGHT_GATHER - SFX_FLAG);
        }
    }

    /// `DemoEffect_UpdateLgtShower`: Farore's light spreading (×1.05) and fading (3 a frame).
    fn update_lgt_shower(&mut self) {
        if self.u.lgt_shower_alpha() > 3 {
            *self.u.lgt_shower_alpha_mut() -= 3;
            self.actor.scale *= 1.05;
        } else {
            self.actor.kill();
        }
    }

    /// The goddesses' flight sounds at frames of the cutscene map's layers 4, 6 and 11.
    fn god_flight_sfx(&self, play: &mut PlayState, frames: [(usize, u16, u16); 3]) {
        if play.save.entrance_index != ENTR_CUTSCENE_MAP_0 {
            return;
        }
        let (layer, frame) = (play.save.scene_layer, play.cs_ctx.frames);
        for (l, f, sfx) in frames {
            if layer == l && frame == f {
                audio_play_actor_sfx2(play, sfx);
            }
        }
    }

    /// `DemoEffect_UpdateGodLgtDin`: along her cue, facing it; on cue 3 a fire ball each frame
    /// (`DemoEffect_InitCreationFireball`, scale 0.02); her flight's sounds.
    fn update_god_lgt_din(&mut self, play: &mut PlayState) {
        let Some(cue) = self.cue(play) else { return };
        self.set_pos_rot_from_cue(play, &cue, true);
        if cue.action == 3 {
            let pos = self.actor.world_pos;
            if let Ok(h) = play.actor_spawn_as_child(&mut self.actor, ACTOR_DEMO_EFFECT, pos, [0, 0, 0], DEMO_EFFECT_FIRE_BALL as i16) {
                if let Some(f) = play.actors.downcast_mut::<DemoEffect>(h) {
                    f.init_update_func = UpdateFunc::InitCreationFireball;
                    f.actor.scale = Vec3::splat(0.020);
                }
            }
        }
        // FRAMERATE_CONST(288, 240), (635, 535), (55, 25), (350, 353).
        self.god_flight_sfx(play, [(4, 288, NA_SE_IT_DM_FLYING_GOD_PASS), (6, 55, NA_SE_IT_DM_FLYING_GOD_DASH), (11, 350, NA_SE_IT_DM_FLYING_GOD_DASH)]);
        if play.save.entrance_index == ENTR_CUTSCENE_MAP_0 && play.save.scene_layer == 4 && play.cs_ctx.frames == 635 {
            audio_play_actor_sfx2(play, NA_SE_IT_DM_FLYING_GOD_PASS);
        }
    }

    /// `DemoEffect_UpdateGodLgtNayru`: along her cue, facing it; on cue 3 an expanding ring every
    /// fifth frame (`lightRingSpawnDelay` 4), turned a quarter up; her sounds, and on Death
    /// Mountain Trail `SEQ_CS_EFFECTS_NAYRU_MAGIC`.
    fn update_god_lgt_nayru(&mut self, play: &mut PlayState) {
        let Some(cue) = self.cue(play) else { return };
        self.set_pos_rot_from_cue(play, &cue, true);
        if cue.action == 3 {
            if self.u.god_lgt_light_ring_spawn_timer() != 0 {
                *self.u.god_lgt_light_ring_spawn_timer_mut() -= 1;
            } else {
                *self.u.god_lgt_light_ring_spawn_timer_mut() = self.u.god_lgt_light_ring_spawn_delay() as i16;
                let (pos, rot) = (self.actor.world_pos, self.actor.world_rot);
                if let Some(a) =
                    play.actor_spawn(ACTOR_DEMO_EFFECT, pos, [rot.x.wrapping_add(0x4000), rot.y, rot.z], DEMO_EFFECT_LIGHTRING_EXPANDING as i16).ok().and_then(|h| play.actors.actor_mut(h))
                {
                    a.scale = Vec3::ONE;
                }
            }
        }
        // FRAMERATE_CONST(298, 248), (105, 88), (360, 362).
        self.god_flight_sfx(play, [(4, 298, NA_SE_IT_DM_FLYING_GOD_PASS), (6, 105, NA_SE_IT_DM_FLYING_GOD_DASH), (11, 360, NA_SE_IT_DM_FLYING_GOD_DASH)]);
        if play.save.entrance_index == ENTR_DEATH_MOUNTAIN_TRAIL_0 && play.save.scene_layer == 4 {
            // FRAMERATE_CONST(72, 57), FRAMERATE_CONST(80, 72).
            if play.cs_ctx.frames == 72 {
                audio_play_actor_sfx2(play, NA_SE_IT_DM_FLYING_GOD_DASH);
            }
            if play.cs_ctx.frames == 80 {
                play.audio.play_cutscene_effects_sequence(SEQ_CS_EFFECTS_NAYRU_MAGIC);
            }
        }
    }

    /// `DemoEffect_UpdateGodLgtFarore`: along her cue, facing it; on cue 3 a light shower 150
    /// below her, her dash and `SEQ_CS_EFFECTS_FARORE_MAGIC`; her sounds.
    fn update_god_lgt_farore(&mut self, play: &mut PlayState) {
        let Some(cue) = self.cue(play) else { return };
        self.set_pos_rot_from_cue(play, &cue, true);
        if cue.action == 3 {
            let pos = self.actor.world_pos - Vec3::Y * 150.0;
            if let Ok(h) = play.actor_spawn_as_child(&mut self.actor, ACTOR_DEMO_EFFECT, pos, [0, 0, 0], DEMO_EFFECT_LGT_SHOWER as i16)
                && let Some(a) = play.actors.actor_mut(h)
            {
                a.scale = Vec3::new(0.23, 0.15, 0.23);
            }
            audio_play_actor_sfx2(play, NA_SE_IT_DM_FLYING_GOD_DASH);
            play.audio.play_cutscene_effects_sequence(SEQ_CS_EFFECTS_FARORE_MAGIC);
        }
        // FRAMERATE_CONST(315, 265), (80, 60), (370, 371).
        self.god_flight_sfx(play, [(4, 315, NA_SE_IT_DM_FLYING_GOD_PASS), (6, 80, NA_SE_IT_DM_FLYING_GOD_DASH), (11, 370, NA_SE_IT_DM_FLYING_GOD_DASH)]);
    }

    /// `DemoEffect_MoveTowardTarget`.
    fn move_toward_target(&mut self, target: Vec3, speed: f32) {
        let p = &mut self.actor.world_pos;
        p.x += (target.x - p.x) * speed;
        p.y += (target.y - p.y) * speed;
        p.z += (target.z - p.z) * speed;
    }

    /// `DemoEffect_InitJewelColor`.
    fn init_jewel_color(&mut self) {
        match self.u.jewel_type() {
            DEMO_EFFECT_JEWEL_KOKIRI => {
                self.prim_xlu_color = [255, 255, 160];
                self.env_xlu_color = [0, 255, 0];
                self.prim_opa_color = [255, 255, 170];
                self.env_opa_color = [150, 120, 0];
            }
            DEMO_EFFECT_JEWEL_GORON => {
                self.prim_xlu_color = [255, 170, 255];
                self.env_xlu_color = [255, 0, 100];
                self.prim_opa_color = [255, 255, 170];
                self.env_opa_color = [150, 120, 0];
            }
            DEMO_EFFECT_JEWEL_ZORA => {
                self.prim_xlu_color = [50, 255, 255];
                self.env_xlu_color = [50, 0, 150];
                self.prim_opa_color = [255, 255, 170];
                self.env_opa_color = [150, 120, 0];
            }
            _ => {}
        }
    }

    /// `DemoEffect_SetJewelColor`: the colours drained towards white by `1 - alpha` (only ever 1).
    fn set_jewel_color(&mut self, alpha: f32) {
        self.init_jewel_color();
        let drain = |c: u8| ((c as i32) as f32 * alpha + 255.0 * (1.0 - alpha)) as i32 as u8;
        let scale = |c: u8| ((c as i32) as f32 * alpha) as i32 as u8;
        self.prim_xlu_color = self.prim_xlu_color.map(drain);
        self.prim_opa_color = self.prim_opa_color.map(drain);
        self.env_xlu_color = self.env_xlu_color.map(scale);
        self.env_opa_color = self.env_opa_color.map(scale);
    }

    /// `DemoEffect_MoveJewelSplit`: the jewels apart, to their places in the pedestal.
    fn move_jewel_split(&mut self) {
        match self.u.jewel_type() {
            DEMO_EFFECT_JEWEL_KOKIRI => self.actor.world_pos.x -= 40.0,
            DEMO_EFFECT_JEWEL_ZORA => self.actor.world_pos.x += 40.0,
            _ => {}
        }
    }

    /// `DemoEffect_MoveJewelSpherical`.
    fn move_jewel_spherical(&mut self, degrees: f32, frame_divisor: f32, start: Vec3, end: Vec3, radius: f32, rotation: [i16; 3]) {
        let d = end - start;
        let distance = frame_divisor * (d.x * d.x + d.y * d.y + d.z * d.z).sqrt();
        let deg_to_rad = |deg: f32| (deg as f64 * (std::f64::consts::PI / 180.0)) as f32;
        let b2r = oot_game::sys_matrix::binang_to_rad;
        let mut p = Vec3::new(radius * deg_to_rad(degrees).cos(), distance, radius * deg_to_rad(degrees).sin());
        let x_pos = p.x;
        let y_spherical = p.y * b2r(rotation[0]).cos() - b2r(rotation[0]).sin() * p.z;
        let xz_spherical = p.z * b2r(rotation[0]).cos() + b2r(rotation[0]).sin() * p.y;
        p.x = x_pos * b2r(rotation[1]).cos() - b2r(rotation[1]).sin() * xz_spherical;
        p.y = y_spherical;
        p.z = xz_spherical * b2r(rotation[1]).cos() + b2r(rotation[1]).sin() * x_pos;
        self.actor.world_pos = p + start;
    }

    /// `DemoEffect_MoveJewelActivateDoorOfTime`: the jewels circling Link up from his hands.
    fn move_jewel_activate_door_of_time(&mut self, play: &PlayState, cue: &CsCmdActorCue) {
        let start = ivec(cue.start_pos);
        let end = ivec(cue.end_pos);
        let frame_divisor = interpolate_cs_frames(play, cue);
        let mut degrees = match self.u.jewel_type() {
            DEMO_EFFECT_JEWEL_GORON => 120.0,
            DEMO_EFFECT_JEWEL_ZORA => 240.0,
            _ => 0.0,
        };
        let mut radius = 50.0 * frame_divisor;
        if radius > 30.0 {
            radius = 30.0;
        }
        if start != end {
            // RAD_TO_BINANG(Math_Atan2F(end.z - start.z, -(end.x - start.x))), Math_Vec3f_Yaw.
            self.jewel_cs_rotation[0] = rad_to_binang(math_atan2f(end.z - start.z, -(end.x - start.x)));
            self.jewel_cs_rotation[1] = oot_game::target::yaw_to(start, end);
        }
        self.jewel_cs_rotation[2] = self.jewel_cs_rotation[2].wrapping_add(0x0400);
        degrees += self.jewel_cs_rotation[2] as f32 * (360.0 / 65536.0);
        self.move_jewel_spherical(degrees, frame_divisor, start, end, radius, self.jewel_cs_rotation);
    }

    /// `DemoEffect_JewelSparkle`.
    fn jewel_sparkle(&self, play: &mut PlayState, spawner_count: i32) {
        let _colors = S_JEWEL_SPARKLE_COLORS[(self.u.jewel_type() - DEMO_EFFECT_JEWEL_KOKIRI) as usize];
        for _ in 0..spawner_count {
            let r = &mut play.rand;
            let (_vx, _vz) = ((r.zero_one() - 0.5) * 1.5, (r.zero_one() - 0.5) * 1.5);
            kira_kira_spawn_dispersed(play);
        }
    }

    /// `DemoEffect_PlayJewelSfx`: `NA_SE_EV_SPIRIT_STONE` from one jewel only (`sSfxJewelId`),
    /// unless on cue 1.
    fn play_jewel_sfx(&mut self, play: &mut PlayState) {
        if !self.check_for_cue(play, 1) {
            let params = self.actor.params;
            let st = statics(play);
            if params == st.sfx_jewel_id {
                self.actor.play_sfx_flagged(NA_SE_EV_SPIRIT_STONE - SFX_FLAG);
            } else if st.sfx_jewel_id == 0 {
                st.sfx_jewel_id = params;
                self.actor.play_sfx_flagged(NA_SE_EV_SPIRIT_STONE - SFX_FLAG);
            }
        }
    }

    /// `DemoEffect_UpdateJewelAdult`.
    fn update_jewel_adult(&mut self, play: &mut PlayState) {
        *self.u.jewel_timer_mut() = self.u.jewel_timer().wrapping_add(1);
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x0400);
        self.play_jewel_sfx(play);
        self.set_jewel_color(1.0);
    }

    /// `DemoEffect_UpdateJewelChild`: placed by its cue (the Door of Time's cues 3 and 4 circling
    /// and splitting with sparkles, 6 gone), turning.
    fn update_jewel_child(&mut self, play: &mut PlayState) {
        *self.u.jewel_timer_mut() = self.u.jewel_timer().wrapping_add(1);
        if let Some(cue) = self.cue(play) {
            match cue.action {
                3 => {
                    // @bug (game): sets the flag only when it's set already (a no-op).
                    if play.save.get_event_chk_inf(EVENTCHKINF_OPENED_DOOR_OF_TIME) {
                        play.save.set_event_chk_inf(EVENTCHKINF_OPENED_DOOR_OF_TIME);
                    }
                    self.move_jewel_activate_door_of_time(play, &cue);
                    if play.gameplay_frames & 1 == 0 {
                        self.jewel_sparkle(play, 1);
                    }
                }
                4 => {
                    if self.u.jewel_is_position_init() != 0 {
                        self.set_pos_rot_from_cue(play, &cue, false);
                        self.move_jewel_split();
                        if play.gameplay_frames & 1 == 0 {
                            self.jewel_sparkle(play, 1);
                        }
                    } else {
                        self.set_start_pos_from_cue(&cue);
                        self.move_jewel_split();
                        *self.u.jewel_is_position_init_mut() = 1;
                    }
                }
                6 => {
                    self.actor.kill();
                    return;
                }
                _ => {
                    self.set_pos_rot_from_cue(play, &cue, false);
                    if play.save.entrance_index == ENTR_TEMPLE_OF_TIME_0 {
                        self.move_jewel_split();
                    }
                }
            }
        }
        if play.save.entrance_index == ENTR_TEMPLE_OF_TIME_0 && !play.save.get_event_chk_inf(EVENTCHKINF_OPENED_DOOR_OF_TIME) && self.cue(play).is_none() {
            self.effect_flags |= 0x1;
            return;
        }
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x0400);
        self.play_jewel_sfx(play);
        self.effect_flags &= !1;
    }

    /// `DemoEffect_UpdateDust`: the Temple of Time's dust falling on cue 2.
    fn update_dust(&mut self, play: &mut PlayState) {
        if let Some(cue) = self.cue(play)
            && cue.action == 2
        {
            let mut pos = self.actor.world_pos;
            pos.y += 600.0;
            let r = &mut play.rand;
            pos.x += r.centered_float(300.0);
            pos.z += 200.0 + r.centered_float(300.0);
            let velocity = Vec3::new(0.0, -20.0, 0.0);
            let accel = Vec3::new(0.0, 0.2, 0.0);
            play.with_ss(|s| s.func_8002873c(pos, velocity, accel, 300, 0, 30));
            *self.u.dust_timer_mut() = self.u.dust_timer().wrapping_add(1);
        }
    }

    /// `DemoEffect_CheckForCue`.
    fn check_for_cue(&self, play: &PlayState, cue_id: u16) -> bool {
        self.cue(play).is_some_and(|c| c.action == cue_id)
    }

    /// `DemoEffect_FaceTowardPoint`.
    fn face_toward_point(&mut self, start: Vec3, end: Vec3) {
        let (x, z) = (end.x - start.x, end.z - start.z);
        let xz_distance = (x * x + z * z).sqrt();
        self.actor.shape_rot.y = rad_to_binang(oot_game::camera::f_atan2f(x, z));
        self.actor.shape_rot.x = rad_to_binang(oot_game::camera::f_atan2f(-(end.y - start.y), xz_distance));
    }

    /// `DemoEffect_SetPosRotFromCue`: along the cue by its frames, facing its end if asked.
    fn set_pos_rot_from_cue(&mut self, play: &PlayState, cue: &CsCmdActorCue, should_update_facing: bool) {
        let (start, end) = (ivec(cue.start_pos), ivec(cue.end_pos));
        let speed = interpolate_cs_frames(play, cue);
        self.actor.world_pos = Vec3::new((end.x - start.x) * speed + start.x, (end.y - start.y) * speed + start.y, (end.z - start.z) * speed + start.z);
        if should_update_facing {
            self.face_toward_point(start, end);
        }
    }

    /// `DemoEffect_MoveTowardCuePos`.
    fn move_toward_cue_pos(&mut self, cue: &CsCmdActorCue, speed: f32) {
        self.move_toward_target(ivec(cue.end_pos), speed);
    }

    /// `DemoEffect_SetStartPosFromCue`.
    fn set_start_pos_from_cue(&mut self, cue: &CsCmdActorCue) {
        self.actor.world_pos = ivec(cue.start_pos);
    }
}

/// `play->csCtx.actorCues[channel]` while a cutscene runs (`csCtx.state != CS_STATE_IDLE`).
fn cue_on(play: &PlayState, channel: usize) -> Option<CsCmdActorCue> {
    if play.cs_ctx.state != CS_STATE_IDLE { play.cs_ctx.npc_actions.get(channel).copied().flatten() } else { None }
}

fn ivec(v: glam::IVec3) -> Vec3 {
    Vec3::new(v.x as f32, v.y as f32, v.z as f32)
}

/// `Math_Atan2F(y, x)`: `Math_Atan2S` in radians.
fn math_atan2f(y: f32, x: f32) -> f32 {
    oot_game::sys_matrix::binang_to_rad(eng_math::atan2_s(y, x))
}

/// `DemoEffect_InterpolateCsFrames`: how far through its cue the cutscene is, at most 1.
fn interpolate_cs_frames(play: &PlayState, cue: &CsCmdActorCue) -> f32 {
    let w = oot_game::env::lerp_weight(cue.end_frame, cue.start_frame, play.cs_ctx.frames);
    if w > 1.0 { 1.0 } else { w }
}

/// `EffectSsKiraKira_SpawnDispersed`: not ported (`Effect_Ss_KiraKira`); its three `Rand` calls
/// (the velocity's and the acceleration's y, the yaw) are made.
fn kira_kira_spawn_dispersed(play: &mut PlayState) {
    let r = &mut play.rand;
    let (_vy, _ay, _yaw) = (r.zero_one(), r.zero_one(), r.zero_one());
}

/// The time warp's curve skeleton and animation (`gTimeWarpSkel`, `gTimeWarpAnim`), read once.
fn time_warp_curves(play: &mut PlayState) -> Option<(Arc<CurveSkeleton>, Arc<CurveAnimation>)> {
    if statics(play).time_warp.is_none() {
        let pack = &play.assets.as_ref()?.pack;
        let skel = pack.curve_skeleton("object_efc_tw", "gTimeWarpSkel").map_err(|e| log::error!("{e:#}")).ok()?;
        let anim = pack.curve_animation("object_efc_tw", "gTimeWarpAnim").map_err(|e| log::error!("{e:#}")).ok()?;
        statics(play).time_warp = Some((Arc::new(skel), Arc::new(anim)));
    }
    statics(play).time_warp.clone()
}

fn time_warp_vertices(play: &mut PlayState) -> Option<Arc<BakeVertices>> {
    if statics(play).time_warp_vertices.is_none() {
        let v = BakeVertices::load(play, BAKE_TIME_WARP)?;
        statics(play).time_warp_vertices = Some(v);
    }
    statics(play).time_warp_vertices.clone()
}

fn triforce_column_vertices(play: &mut PlayState) -> Option<Arc<BakeVertices>> {
    if statics(play).triforce_column_vertices.is_none() {
        let v = BakeVertices::load(play, BAKE_TRIFORCE_COLUMN)?;
        statics(play).triforce_column_vertices = Some(v);
    }
    statics(play).triforce_column_vertices.clone()
}

/// The bakes (docs/adr/0012-actor-bakes.md): each list as the draw calls it, after
/// `Gfx_SetupDL_25Opa`/`25Xlu` (the bakes' start), its colours and scrolls in segments.
const BAKE_CRYSTAL_LIGHT: &str = "Demo_Effect/crystal_light";
const BAKE_FIRE_BALL: &str = "Demo_Effect/fire_ball";
const BAKE_GOD_LGT_AURA: &str = "Demo_Effect/god_lgt_aura";
const BAKE_GOD_LGT_BODY: &str = "Demo_Effect/god_lgt_body";
const BAKE_LIGHT_RING: &str = "Demo_Effect/light_ring";
pub const BAKE_FLASH: &str = "Demo_Effect/flash";
const BAKE_LGT_SHOWER: &str = "Demo_Effect/lgt_shower";
pub const BAKE_TRIFORCE_COLUMN: &str = "Demo_Effect/triforce_column";
const BAKE_TRIFORCE_XLU: &str = "Demo_Effect/triforce_xlu";
const BAKE_TRIFORCE_OPA: &str = "Demo_Effect/triforce_opa";
pub const BAKE_TIME_WARP: &str = "Demo_Effect/time_warp";
/// The jewels' gems (translucent) and settings (opaque).
const BAKE_JEWELS: [(&str, &str); 3] =
    [("Demo_Effect/kokiri_gem", "Demo_Effect/kokiri_setting"), ("Demo_Effect/goron_gem", "Demo_Effect/goron_setting"), ("Demo_Effect/zora_gem", "Demo_Effect/zora_setting")];
const JEWEL_DLS: [(&str, &str); 3] = [("gGiKokiriEmeraldGemDL", "gGiKokiriEmeraldSettingDL"), ("gGiGoronRubyGemDL", "gGiGoronRubySettingDL"), ("gGiZoraSapphireGemDL", "gGiZoraSapphireSettingDL")];

/// The segments: the scrolls (8 and 9, as the lists call them), and the draw's colours (0x0A),
/// run before the list.
const SEG_SCROLL_8: u8 = 0x08;
const SEG_SCROLL_9: u8 = 0x09;
const SEG_COLOR: u8 = 0x0A;

/// `gDPSetPrimColor(m, l, r, g, b, a)` and `gDPSetEnvColor(r, g, b, a)` as commands.
fn prim(m: u8, l: u8, c: [u8; 4]) -> (u32, u32) {
    (0xFA00_0000 | (m as u32) << 8 | l as u32, u32::from_be_bytes(c))
}
fn env(c: [u8; 4]) -> (u32, u32) {
    (0xFB00_0000, u32::from_be_bytes(c))
}
/// `gDPSetRenderMode(G_RM_PASS, mode2)`: `G_SETOTHERMODE_L`'s render mode bits.
fn render_mode_pass(mode2: u32) -> (u32, u32) {
    // G_RM_PASS: GBL_c1(G_BL_CLR_IN, G_BL_0, G_BL_CLR_IN, G_BL_1).
    (0xE200_001C, 0x0C08_0000 | mode2)
}
/// `G_RM_AA_ZB_XLU_SURF2`, `G_RM_AA_ZB_OPA_SURF2` (`gbi.h`).
const G_RM_AA_ZB_XLU_SURF2: u32 = 0x0010_49D8;
const G_RM_AA_ZB_OPA_SURF2: u32 = 0x0011_2078;
const END: (u32, u32) = (0xDF00_0000, 0);

fn dl_bake(name: &str, object: &str, dl: &str, segments: Vec<(u8, BakeSegment)>, prelude: Vec<u8>) -> MeshBake {
    MeshBake { name: name.into(), object: object.into(), segments, prelude, body: BakeBody::DLists(vec![(object.into(), dl.into())]) }
}

/// The scrolls the draws compute every frame (`Gfx_TwoTexScroll`, `Gfx_TexScroll`), at frame
/// `frames` (or a type's own timer).
fn crystal_light_scroll(frames: u32) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, (frames.wrapping_mul(2)) % 512, 512 - (frames % 512) - 1, 128, 128, 1, 512 - ((frames.wrapping_mul(2)) % 512) - 1, 0, 64, 64)
}
fn fire_ball_scroll(frames: u32) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, 0, 0, 32, 32, 1, 0, 128 - ((frames.wrapping_mul(20)) % 128) - 1, 32, 32)
}
fn god_lgt_scrolls(frames: u32) -> (Vec<(u32, u32)>, Vec<(u32, u32)>) {
    (
        gfx_two_tex_scroll(0, (frames.wrapping_mul(4)) % 512, 0, 128, 64, 1, (frames.wrapping_mul(2)) % 256, 512 - ((frames.wrapping_mul(70)) % 512) - 1, 64, 32),
        gfx_two_tex_scroll(0, 0, 0, 16, 96, 1, (frames.wrapping_mul(10)) % 256, 256u32.wrapping_sub((frames.wrapping_mul(30)) % 512).wrapping_sub(1), 8, 32),
    )
}
fn lgt_shower_scroll(frames: u32) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, (frames.wrapping_mul(5)) % 1024, 0, 256, 64, 1, (frames.wrapping_mul(10)) % 128, 512 - ((frames.wrapping_mul(50)) % 512), 32, 16)
}
fn light_ring_scroll(timer: u32) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, (timer.wrapping_mul(5)) % 64, 512u32.wrapping_sub((timer.wrapping_mul(2)) % 512).wrapping_sub(1), 16, 128, 1, 0, 0, 8, 1024)
}
fn triforce_column_scroll(frames: u32) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, 0, 256 - ((frames.wrapping_mul(4)) % 256) - 1, 64, 64, 1, 0, 256 - ((frames.wrapping_mul(2)) % 256) - 1, 64, 32)
}
fn triforce_scroll() -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, 0, 0, 32, 16, 1, 0, 0, 16, 8)
}
fn time_warp_scroll(frames: u32) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(
        0,
        (frames.wrapping_mul(6)) % 1024,
        256 - ((frames.wrapping_mul(16)) % 256) - 1,
        256,
        64,
        1,
        (frames.wrapping_mul(4)) % 512,
        128 - ((frames.wrapping_mul(12)) % 128) - 1,
        128,
        32,
    )
}
/// The jewels' gem scroll by type, and the settings' (`Gfx_TexScroll((u8)frames, (u8)frames, 16, 16)`).
fn jewel_gem_scroll(ty: u8, frames: u32) -> Vec<(u32, u32)> {
    let (x1m, w1, h1, w2, h2) = match ty {
        DEMO_EFFECT_JEWEL_GORON => (128, 32, 64, 16, 8),
        DEMO_EFFECT_JEWEL_ZORA => (256, 32, 32, 16, 16),
        _ => (256, 64, 64, 16, 16),
    };
    gfx_two_tex_scroll(0, (frames.wrapping_mul(4)) % x1m, (256 - ((frames.wrapping_mul(2)) % 256)) - 1, w1, h1, 1, (frames.wrapping_mul(2)) % 256, (256 - (frames % 256)) - 1, w2, h2)
}
fn jewel_setting_scroll(frames: u32) -> Vec<(u32, u32)> {
    gfx_tex_scroll(frames & 0xFF, frames & 0xFF, 16, 16)
}

/// Its bakes.
pub fn bakes() -> Vec<MeshBake> {
    let dyn_color = |cmds: Vec<(u32, u32)>| (SEG_COLOR, BakeSegment::Dynamic(cmds));
    let fixed_color = |cmds: Vec<(u32, u32)>| (SEG_COLOR, BakeSegment::Commands(cmds));
    let mut v = vec![
        // DemoEffect_DrawCrystalLight: gDPSetPrimColor(128, 128, 255, 255, 170, opacity).
        dl_bake(
            BAKE_CRYSTAL_LIGHT,
            "object_efc_crystal_light",
            "gCrystalLightDL",
            vec![dyn_color(vec![prim(128, 128, [255, 255, 170, 255]), END]), (SEG_SCROLL_8, BakeSegment::Dynamic(crystal_light_scroll(0)))],
            vec![SEG_COLOR],
        ),
        // DemoEffect_DrawFireBall: (64, 64, 255, 200, 0, 255), env (255, 0, 0, 255).
        dl_bake(
            BAKE_FIRE_BALL,
            "object_efc_fire_ball",
            "gCreationFireBallDL",
            vec![fixed_color(vec![prim(64, 64, [255, 200, 0, 255]), env([255, 0, 0, 255]), END]), (SEG_SCROLL_8, BakeSegment::Dynamic(fire_ball_scroll(0)))],
            vec![SEG_COLOR],
        ),
        // DemoEffect_DrawGodLgt's aura: (128, 128, primXluColor, 255), env envXluColor.
        dl_bake(
            BAKE_GOD_LGT_AURA,
            "object_god_lgt",
            "gGoldenGoddessAuraDL",
            vec![
                dyn_color(vec![prim(128, 128, [255, 255, 255, 255]), env([255, 255, 255, 255]), END]),
                (SEG_SCROLL_8, BakeSegment::Dynamic(god_lgt_scrolls(0).0)),
                (SEG_SCROLL_9, BakeSegment::Dynamic(god_lgt_scrolls(0).1)),
            ],
            vec![SEG_COLOR],
        ),
        // Its body, in the opaque list: the list sets its own colours.
        dl_bake(BAKE_GOD_LGT_BODY, "object_god_lgt", "gGoldenGoddessBodyDL", vec![], vec![]),
        // DemoEffect_DrawLightRing: (128, 128, 170, 255, 255, alpha), env (0, 100, 255, 255).
        dl_bake(
            BAKE_LIGHT_RING,
            "object_light_ring",
            "gGoldenGoddessLightRingDL",
            vec![dyn_color(vec![prim(128, 128, [170, 255, 255, 255]), env([0, 100, 255, 255]), END]), (SEG_SCROLL_8, BakeSegment::Dynamic(light_ring_scroll(0)))],
            vec![SEG_COLOR],
        ),
        // DemoEffect_DrawLightEffect (0, 128, ...) and _DrawBlueOrb (128, 128, ...): gEffFlash1DL.
        dl_bake(BAKE_FLASH, "gameplay_keep", "gEffFlash1DL", vec![dyn_color(vec![prim(128, 128, [255, 255, 255, 255]), env([255, 255, 255, 255]), END])], vec![SEG_COLOR]),
        // DemoEffect_DrawLgtShower: (64, 64, 255, 255, 160, alpha), env (50, 200, 0, 255).
        dl_bake(
            BAKE_LGT_SHOWER,
            "object_efc_lgt_shower",
            "gEnliveningLightDL",
            vec![dyn_color(vec![prim(64, 64, [255, 255, 160, 255]), env([50, 200, 0, 255]), END]), (SEG_SCROLL_8, BakeSegment::Dynamic(lgt_shower_scroll(0)))],
            vec![SEG_COLOR],
        ),
        // DemoEffect_DrawTriforceSpot's column: (128, 128, 180, 255, 255, opacity), env (0, 255, 150).
        dl_bake(
            BAKE_TRIFORCE_COLUMN,
            "object_triforce_spot",
            "gTriforceLightColumnDL",
            vec![dyn_color(vec![prim(128, 128, [180, 255, 255, 255]), env([0, 255, 150, 255]), END]), (SEG_SCROLL_9, BakeSegment::Dynamic(triforce_column_scroll(0)))],
            vec![SEG_COLOR],
        ),
        // The Triforce, fading in (G_RM_PASS, G_RM_AA_ZB_XLU_SURF2; (128, 128, 255, 255, 160,
        // opacity), env (170, 140, 0)), then solid (G_RM_AA_ZB_OPA_SURF2, alpha 255).
        dl_bake(
            BAKE_TRIFORCE_XLU,
            "object_triforce_spot",
            "gTriforceDL",
            vec![dyn_color(vec![render_mode_pass(G_RM_AA_ZB_XLU_SURF2), prim(128, 128, [255, 255, 160, 255]), env([170, 140, 0, 255]), END]), (SEG_SCROLL_8, BakeSegment::Commands(triforce_scroll()))],
            vec![SEG_COLOR],
        ),
        dl_bake(
            BAKE_TRIFORCE_OPA,
            "object_triforce_spot",
            "gTriforceDL",
            vec![
                fixed_color(vec![render_mode_pass(G_RM_AA_ZB_OPA_SURF2), prim(128, 128, [255, 255, 160, 255]), env([170, 140, 0, 255]), END]),
                (SEG_SCROLL_8, BakeSegment::Commands(triforce_scroll())),
            ],
            vec![SEG_COLOR],
        ),
        // DemoEffect_OverrideLimbDrawTimeWarp's state before limb 1's list: (0, 128, 170, 255,
        // 255, 255), env envXluColor (0, 100, 255), segment 8's scroll.
        dl_bake(
            BAKE_TIME_WARP,
            "object_efc_tw",
            "gTimeWarpDL",
            vec![dyn_color(vec![prim(0, 128, [170, 255, 255, 255]), env([0, 100, 255, 255]), END]), (SEG_SCROLL_8, BakeSegment::Dynamic(time_warp_scroll(0)))],
            vec![SEG_COLOR],
        ),
    ];
    for (i, (gem, setting)) in BAKE_JEWELS.iter().enumerate() {
        let ty = DEMO_EFFECT_JEWEL_KOKIRI + i as u8;
        // DemoEffect_DrawJewel: the gem (0, 128, primXluColor, 255), env envXluColor, segment 9;
        // the setting (0, 128, primOpaColor, 255), env envOpaColor, segment 8.
        v.push(dl_bake(
            gem,
            "object_gi_jewel",
            JEWEL_DLS[i].0,
            vec![dyn_color(vec![prim(0, 128, [255, 255, 255, 255]), env([255, 255, 255, 255]), END]), (SEG_SCROLL_9, BakeSegment::Dynamic(jewel_gem_scroll(ty, 0)))],
            vec![SEG_COLOR],
        ));
        v.push(dl_bake(
            setting,
            "object_gi_jewel",
            JEWEL_DLS[i].1,
            vec![dyn_color(vec![prim(0, 128, [255, 255, 255, 255]), env([255, 255, 255, 255]), END]), (SEG_SCROLL_8, BakeSegment::Dynamic(jewel_setting_scroll(0)))],
            vec![SEG_COLOR],
        ));
    }
    v
}

/// A draw of bake `name` at `m`, with these colours (`None` keeps the bake's) and scrolls.
fn cmd(name: &str, m: Mat4, prim_c: Option<[u8; 4]>, env_c: Option<[u8; 4]>, scrolls: &[(u8, Vec<(u32, u32)>)]) -> DrawCmd {
    let mut sv = SegmentValues::default();
    for (seg, c) in scrolls {
        sv.read(*seg, c);
    }
    sv.prim[SEG_COLOR as usize] = prim_c;
    sv.env[SEG_COLOR as usize] = env_c;
    DrawCmd { mesh: MeshKey::named(keys::bake(name)), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } }
}

fn rgba(c: [u8; 3], a: u8) -> [u8; 4] {
    [c[0], c[1], c[2], a]
}

/// `DEG_TO_RAD`.
fn deg_to_rad(d: f32) -> f32 {
    (d as f64 * (std::f64::consts::PI / 180.0)) as f32
}

impl DemoEffect {
    /// `DemoEffect_DrawJewel`.
    fn draw_jewel(&self, m: Mat4, play: &PlayState, out: &mut DrawOut) {
        if self.check_for_cue(play, 1) || self.effect_flags & 0x1 != 0 {
            return;
        }
        let Some((gem, setting)) = self.jewel_bakes else { return };
        let frames = self.u.jewel_timer() as i32 as u32;
        out.xlu.push(cmd(gem, m, Some(rgba(self.prim_xlu_color, 255)), Some(rgba(self.env_xlu_color, 255)), &[(SEG_SCROLL_9, jewel_gem_scroll(self.u.jewel_type(), frames))]));
        out.opa.push(cmd(setting, m, Some(rgba(self.prim_opa_color, 255)), Some(rgba(self.env_opa_color, 255)), &[(SEG_SCROLL_8, jewel_setting_scroll(frames))]));
    }

    /// `DemoEffect_DrawCrystalLight`: three beams, a third of a turn apart, leaning 11° out, 150
    /// up; their alpha the Triforce's `crystalLightOpacity` (255 with no parent).
    fn draw_crystal_light(&self, m: Mat4, play: &PlayState, out: &mut DrawOut) {
        let frames = play.gameplay_frames & 0xFFFF;
        let alpha = match self.actor.parent.and_then(|h| play.actors.downcast::<DemoEffect>(h)) {
            Some(p) => p.u.crystal_light_opacity(),
            None => 255,
        };
        for y in [0.0, 120.0, 240.0] {
            let b = m * Mat4::from_rotation_y(deg_to_rad(y)) * Mat4::from_rotation_x(deg_to_rad(11.0)) * Mat4::from_translation(Vec3::new(0.0, 150.0, 0.0));
            out.xlu.push(cmd(BAKE_CRYSTAL_LIGHT, b, Some([255, 255, 170, alpha]), None, &[(SEG_SCROLL_8, crystal_light_scroll(frames))]));
        }
    }

    /// `DemoEffect_DrawFireBall`: facing the camera (`play->billboardMtx` multiplied in).
    fn draw_fire_ball(&self, m: Mat4, play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        out.xlu.push(cmd(BAKE_FIRE_BALL, m * view.billboard, None, None, &[(SEG_SCROLL_8, fire_ball_scroll(play.gameplay_frames))]));
    }

    /// `DemoEffect_DrawGodLgt`: but on cue 2, the aura in its colours, then the body, turned by
    /// `godLgt.rotation` (3° a step), on its side, 140 down, at 0.03. (Its sound: `draw_sfx`;
    /// the turn's step: `draw_update`.)
    fn draw_god_lgt(&self, m: Mat4, play: &PlayState, out: &mut DrawOut) {
        if self.check_for_cue(play, 2) {
            return;
        }
        let (s8, s9) = god_lgt_scrolls(play.gameplay_frames);
        out.xlu.push(cmd(BAKE_GOD_LGT_AURA, m, Some(rgba(self.prim_xlu_color, 255)), Some(rgba(self.env_xlu_color, 255)), &[(SEG_SCROLL_8, s8), (SEG_SCROLL_9, s9)]));
        // The rotation the draw stepped to before the body's matrix (draw_update's).
        let b = m
            * Mat4::from_rotation_z(deg_to_rad(self.u.god_lgt_rotation() as i32 as f32 * 3.0))
            * Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2)
            * Mat4::from_translation(Vec3::new(0.0, -140.0, 0.0))
            * Mat4::from_scale(Vec3::splat(0.03));
        out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(BAKE_GOD_LGT_BODY)), b));
    }

    /// `DemoEffect_DrawLightEffect`: but on cue 1, from its second draw on (`light.flicker`),
    /// two flashes turning opposite ways, pulsing by `scaleFlag`.
    fn draw_light_effect(&self, m: Mat4, play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        if self.check_for_cue(play, 1) || self.u.light_flicker() == 0 {
            return;
        }
        let pulse = (self.u.light_scale_flag() & 1) as f32 * 0.05 + 1.0;
        let m = m * Mat4::from_scale(Vec3::splat(pulse));
        let rot = self.u.light_rotation() as f32;
        let prim_c = Some(rgba(self.prim_xlu_color, self.u.light_alpha()));
        let env_c = Some(rgba(self.env_xlu_color, 255));
        out.xlu.push(cmd(BAKE_FLASH, m * view.billboard * Mat4::from_rotation_z(deg_to_rad(rot)), prim_c, env_c, &[]));
        out.xlu.push(cmd(BAKE_FLASH, m * view.billboard * Mat4::from_rotation_z(deg_to_rad(-rot)), prim_c, env_c, &[]));
    }

    /// `DemoEffect_DrawBlueOrb`: a flash facing the camera, spinning (its step: `draw_update`).
    fn draw_blue_orb(&self, m: Mat4, view: &ViewInfo, out: &mut DrawOut) {
        let b = m * view.billboard * Mat4::from_rotation_z(oot_game::sys_matrix::binang_to_rad(self.blue_orb_drawn_rotation()));
        out.xlu.push(cmd(BAKE_FLASH, b, Some([188, 255, 255, self.u.blue_orb_alpha()]), Some([0, 100, 255, 255]), &[]));
    }

    /// The blue orb's rotation its draw uses: the C steps it after the matrix, so the draw shows
    /// the value before `draw_update`'s step.
    fn blue_orb_drawn_rotation(&self) -> i16 {
        self.u.blue_orb_rotation().wrapping_sub(0x01F4)
    }

    /// `DemoEffect_DrawLgtShower`.
    fn draw_lgt_shower(&self, m: Mat4, play: &PlayState, out: &mut DrawOut) {
        out.xlu.push(cmd(BAKE_LGT_SHOWER, m, Some([255, 255, 160, self.u.lgt_shower_alpha()]), None, &[(SEG_SCROLL_8, lgt_shower_scroll(play.gameplay_frames))]));
    }

    /// `DemoEffect_DrawLightRing`: scrolled by its own timer.
    fn draw_light_ring(&self, m: Mat4, out: &mut DrawOut) {
        let timer = self.u.light_ring_timer() as i32 as u32;
        out.xlu.push(cmd(BAKE_LIGHT_RING, m, Some([170, 255, 255, self.u.light_ring_alpha()]), None, &[(SEG_SCROLL_8, light_ring_scroll(timer))]));
    }

    /// `DemoEffect_DrawTriforceSpot`: (not in Zelda's courtyard from frame 885) the column, 2.4
    /// tall, its vertices' alpha the opacity (object RAM, written by `draw_update`); the Triforce
    /// turning, translucent while it fades in, then solid.
    fn draw_triforce_spot(&self, m: Mat4, play: &PlayState, out: &mut DrawOut) {
        if play.save.entrance_index == ENTR_CASTLE_COURTYARD_ZELDA_0 && play.cs_ctx.frames >= 885 {
            return;
        }
        let frames = play.gameplay_frames;
        let column = self.u.light_column_opacity();
        if column > 0 {
            let mut c = cmd(BAKE_TRIFORCE_COLUMN, m * Mat4::from_scale(Vec3::new(1.0, 2.4, 1.0)), Some([180, 255, 255, column]), None, &[(SEG_SCROLL_9, triforce_column_scroll(frames))]);
            if let (Some(v), Some(ram)) = (&self.vertices, self.required_object_slot.and_then(|b| play.object_ctx.written(b, TRIFORCE_VTX))) {
                c.params.vertex_colors = Some(v.with_ram(TRIFORCE_VTX, ram));
            }
            out.xlu.push(c);
        }
        let spot = self.u.triforce_spot_opacity();
        if spot != 0 {
            let b = m * Mat4::from_rotation_y(oot_game::sys_matrix::binang_to_rad(self.u.triforce_spot_rotation()));
            if spot < 250 {
                out.xlu.push(cmd(BAKE_TRIFORCE_XLU, b, Some([255, 255, 160, spot]), None, &[]));
            } else {
                out.opa.push(cmd(BAKE_TRIFORCE_OPA, b, None, None, &[]));
            }
        }
    }

    /// `DemoEffect_DrawGetItem`: but on cues 1 and 4, from its second draw on, `GetItem_Draw`.
    fn draw_get_item(&self, m: Mat4, play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        if self.check_for_cue(play, 1) || self.check_for_cue(play, 4) || self.u.get_item_is_loaded() == 0 {
            return;
        }
        if let Some(a) = play.assets.as_ref() {
            oot_game::draw::get_item_draw(&a.items, self.u.get_item_draw_id() as i16, m, play.gameplay_frames, view, out);
        }
    }

    /// `DemoEffect_DrawTimeWarp`: (the blocks', or with cutscene flag 1, in a cutscene layer or
    /// from the Temple of Time's entrance 4) twice its size, its curve skeleton at level of
    /// detail 1 (`DemoEffect_OverrideLimbDrawTimeWarp`: limb 0's scale 1; the colours and scroll
    /// before each limb), its vertices' alphas from the object's RAM.
    fn draw_time_warp(&self, m: Mat4, play: &PlayState, out: &mut DrawOut) {
        let ty = self.effect_type();
        let shown = ty == DEMO_EFFECT_TIMEWARP_TIMEBLOCK_LARGE
            || ty == DEMO_EFFECT_TIMEWARP_TIMEBLOCK_SMALL
            || play.flags_get_env(1)
            || play.save.scene_layer >= 4
            || play.save.entrance_index == ENTR_TEMPLE_OF_TIME_4;
        if !shown {
            return;
        }
        let m = m * Mat4::from_scale(Vec3::splat(2.0));
        let mut sc = self.skel_curve.clone();
        let mut override_limb = |sc: &mut SkelCurve, limb: usize| {
            if limb == 0
                && let Some(t) = sc.joint_table.as_mut()
            {
                t[0][0] = 1024;
                t[0][2] = 1024;
                t[0][1] = 1024;
            }
            true
        };
        let draws = sc.draw(MtxF::from_mat4(m), 1, &mut override_limb, &mut |_, _| {});
        let frames = play.gameplay_frames;
        let vertex_colors = match (&self.vertices, self.required_object_slot.and_then(|b| play.object_ctx.written(b, TIME_WARP_VTX))) {
            (Some(v), Some(ram)) => Some(v.with_ram(TIME_WARP_VTX, ram)),
            _ => None,
        };
        for d in draws {
            // Only limb 1's translucent list, gTimeWarpDL, is in the skeleton.
            if d.dlist != 0x0600_01B0 || d.list != 1 {
                log::warn!("Demo_Effect: time warp limb {} list {:#010x} has no bake", d.limb, d.dlist);
                continue;
            }
            let mut c = cmd(BAKE_TIME_WARP, d.mtx.to_mat4(), None, Some(rgba(self.env_xlu_color, 255)), &[(SEG_SCROLL_8, time_warp_scroll(frames))]);
            c.params.vertex_colors = vertex_colors.clone();
            out.xlu.push(c);
        }
    }

    /// `DemoEffect_DrawTriforceSpot`'s write: the column's eight vertices' alpha, its opacity
    /// (`(s8)lightColumnOpacity`), into `gTriforceVtx` in the object's RAM.
    fn write_triforce_column_alpha(&self, play: &mut PlayState) {
        let Some(bank) = self.required_object_slot else { return };
        let vertices = self.vertices.clone();
        let ram = play.object_ctx.written_mut(bank, TRIFORCE_VTX, || vertices.map(|v| v.ram_of(TRIFORCE_VTX, TRIFORCE_VTX_COUNT)).unwrap_or_else(|| vec![0; TRIFORCE_VTX_COUNT * 4]));
        for i in TRIFORCE_COLUMN_VERTICES {
            ram[i * 4 + 3] = self.u.light_column_opacity();
        }
    }
}

impl ActorImpl for DemoEffect {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `DemoEffect_Update`: its update function.
    fn update(&mut self, play: &mut PlayState) {
        match self.update_func {
            UpdateFunc::WaitForObject => self.wait_for_object(play),
            UpdateFunc::CrystalLight => self.update_crystal_light(play),
            UpdateFunc::PositionToParent => self.update_position_to_parent(play),
            UpdateFunc::BlueOrbGrow => self.update_blue_orb_grow(play),
            UpdateFunc::BlueOrbShrink => self.update_blue_orb_shrink(),
            UpdateFunc::LgtShower => self.update_lgt_shower(),
            UpdateFunc::GodLgtDin => self.update_god_lgt_din(play),
            UpdateFunc::GodLgtNayru => self.update_god_lgt_nayru(play),
            UpdateFunc::GodLgtFarore => self.update_god_lgt_farore(play),
            UpdateFunc::LightRingExpanding => self.update_light_ring_expanding(play),
            UpdateFunc::TriforceSpot => self.update_triforce_spot(play),
            UpdateFunc::GetItem => self.update_get_item(play),
            UpdateFunc::LightRingShrinking => self.update_light_ring_shrinking(play),
            UpdateFunc::LightRingTriforce => self.update_light_ring_triforce(play),
            UpdateFunc::LightEffect => self.update_light_effect(play),
            UpdateFunc::JewelChild => self.update_jewel_child(play),
            UpdateFunc::JewelAdult => self.update_jewel_adult(play),
            UpdateFunc::Dust => self.update_dust(play),
            UpdateFunc::CreationFireball => self.update_creation_fireball(play),
            UpdateFunc::InitCreationFireball => self.init_creation_fireball(play),
            UpdateFunc::InitTimeWarp => self.init_time_warp(play),
            UpdateFunc::InitTimeWarpTimeblock => self.init_time_warp_timeblock(play),
            UpdateFunc::TimeWarpReturnFromChamberOfSages => self.update_time_warp_return_from_chamber_of_sages(play),
            UpdateFunc::TimeWarpPullMasterSword => self.update_time_warp_pull_master_sword(play),
            UpdateFunc::TimeWarpTimeblock => self.update_time_warp_timeblock(play),
            UpdateFunc::None => {}
        }
    }

    /// `DemoEffect_Destroy`: the time warps' joint table freed.
    fn destroy(&mut self, _play: &mut PlayState) {
        let ty = self.effect_type();
        if ty == DEMO_EFFECT_TIMEWARP_MASTERSWORD || ty == DEMO_EFFECT_TIMEWARP_TIMEBLOCK_LARGE || ty == DEMO_EFFECT_TIMEWARP_TIMEBLOCK_SMALL {
            self.skel_curve.destroy();
        }
    }

    /// The draws' changes, once per game frame: the god lights' turn (to 120, then back to 0),
    /// the light's flicker starting, the blue orb's spin, a medal's first frame, the Triforce
    /// column's vertices.
    fn draw_update(&mut self, play: &mut PlayState) {
        self.draw_skipped = false;
        if self.actor.killed {
            return;
        }
        match self.draw {
            Some(DrawFunc::GodLgt) if !self.check_for_cue(play, 2) => {
                let r = self.u.god_lgt_rotation().wrapping_add(1);
                *self.u.god_lgt_rotation_mut() = if r > 120 { 0 } else { r };
            }
            Some(DrawFunc::LightEffect) if !self.check_for_cue(play, 1) && self.u.light_flicker() == 0 => {
                *self.u.light_flicker_mut() = 1;
                self.draw_skipped = true;
            }
            Some(DrawFunc::BlueOrb) => *self.u.blue_orb_rotation_mut() = self.u.blue_orb_rotation().wrapping_add(0x01F4),
            Some(DrawFunc::GetItem) if !self.check_for_cue(play, 1) && !self.check_for_cue(play, 4) && self.u.get_item_is_loaded() == 0 => {
                *self.u.get_item_is_loaded_mut() = 1;
                self.draw_skipped = true;
            }
            Some(DrawFunc::TriforceSpot) => {
                let hidden = play.save.entrance_index == ENTR_CASTLE_COURTYARD_ZELDA_0 && play.cs_ctx.frames >= 885;
                if !hidden && self.u.light_column_opacity() > 0 {
                    self.write_triforce_column_alpha(play);
                }
            }
            _ => {}
        }
    }

    /// The draws' sounds: the god lights' flight (`Sfx_PlaySfxAtPos`, but on cue 2; in the
    /// cutscene map's layer 4 only to frame 680), the Triforce's column and spot.
    fn draw_sfx(&mut self, play: &mut PlayState) {
        if self.actor.killed {
            return;
        }
        match self.draw {
            Some(DrawFunc::GodLgt) if !self.check_for_cue(play, 2) => {
                let layer4 = play.save.entrance_index == ENTR_CUTSCENE_MAP_0 && play.save.scene_layer == 4;
                if !layer4 || play.cs_ctx.frames <= 680 {
                    audio_play_actor_sfx2(play, NA_SE_EV_GOD_FLYING - SFX_FLAG);
                }
            }
            Some(DrawFunc::TriforceSpot) => {
                let hidden = play.save.entrance_index == ENTR_CASTLE_COURTYARD_ZELDA_0 && play.cs_ctx.frames >= 885;
                if !hidden {
                    if self.u.light_column_opacity() > 0 {
                        audio_play_actor_sfx2(play, NA_SE_EV_AURORA - SFX_FLAG);
                    }
                    if self.u.triforce_spot_opacity() != 0 {
                        audio_play_actor_sfx2(play, NA_SE_EV_TRIFORCE - SFX_FLAG);
                    }
                }
            }
            _ => {}
        }
    }

    /// `actor.draw`, from `Actor_Draw`'s matrix.
    fn draw(&self, rs: &RenderState, play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        let Some(f) = self.draw else { return };
        if self.actor.killed || self.draw_skipped {
            return;
        }
        let m = actor_draw_matrix(rs);
        match f {
            DrawFunc::CrystalLight => self.draw_crystal_light(m, play, out),
            DrawFunc::FireBall => self.draw_fire_ball(m, play, view, out),
            DrawFunc::BlueOrb => self.draw_blue_orb(m, view, out),
            DrawFunc::LgtShower => self.draw_lgt_shower(m, play, out),
            DrawFunc::GodLgt => self.draw_god_lgt(m, play, out),
            DrawFunc::LightRing => self.draw_light_ring(m, out),
            DrawFunc::TriforceSpot => self.draw_triforce_spot(m, play, out),
            DrawFunc::GetItem => self.draw_get_item(m, play, view, out),
            DrawFunc::LightEffect => self.draw_light_effect(m, play, view, out),
            DrawFunc::TimeWarp => self.draw_time_warp(m, play, out),
            DrawFunc::Jewel => self.draw_jewel(m, play, out),
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
