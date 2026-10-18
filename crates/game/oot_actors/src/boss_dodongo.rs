//! `Boss_Dodongo` (`ovl_Boss_Dodongo/z_boss_dodongo.c`): King Dodongo, Dodongo's Cavern's boss,
//! with his intro and his death, which are his own actions and drive his own sub camera (Queen
//! Gohma's pattern, docs/adr/0050-queen-gohma.md).
//!
//! - **The intro** (`BossDodongo_IntroCutscene`): he waits until Link falls below y -1223.76 (the
//!   floor bombed open above); then the manual cutscene, Link held facing east at the arena's
//!   north side (`PLAYER_CSACTION_1`, `9` at 110 frames left, `12` at 5), the camera on his own sub
//!   camera following Link down, then on King Dodongo walking in from the west corner (the camera
//!   shaking with his steps, its up vector tilted by `unk_20C`: `Play_SetCameraAtEyeUp`), his roar
//!   (`NA_SE_EN_DODO_K_OTAKEBI`), the boss's title card the first time, `NA_BGM_FIRE_BOSS`, the
//!   camera back to the main one and `EVENTCHKINF_BEGAN_KING_DODONGO_BATTLE`. With that flag set (a
//!   second try) he starts close to Link and the walk is skipped.
//! - **The fight:** he walks the arena's four corners round the lava (`sCornerPositions`) and rolls
//!   from corner to corner (`BossDodongo_Roll`, his roll's sphere hitting Link) when Link isn't
//!   ahead of him; with Link ahead within 500 he stops and inhales (`BossDodongo_Inhale`), then
//!   breathes fire (`En_Bdfire`, his child: a flame at his mouth and a stream of fire balls rolling
//!   along the floor) and the room's light red (`envCtx.adjLight1Color`, `adjAmbientColor`). The
//!   red screen fill his update sets from `unk_244` (`MREG(64)` to `MREG(68)`) never shows: nothing
//!   sets `unk_244` above 0 (the breath only steps it towards 0); it's ported as written. While he inhales (frames 21 to 81) any
//!   explosive (`ACTORCAT_EXPLOSIVE`) within 40 of his mouth on each axis is eaten (killed, no
//!   blast): he swallows it, and 35 frames on it blows inside him (`BossDodongo_Explode`: dust,
//!   `NA_SE_IT_BOMB_EXPLOSION`, a quake, 2 off his 12 health but never the last), he lies down
//!   (`BossDodongo_LayDown`, `_Vulnerable` for 100 frames) and a sword hit on his head (element 0)
//!   takes `CollisionCheck_GetSwordDamage` off; then he gets up and rolls. A seed or the boomerang
//!   on him while he inhales stops it. His body is hit by everything but takes damage only that way.
//! - **The death** (`BossDodongo_DeathCutscene`): his sub camera from the main one's view, Link
//!   pulled beside him, his death throes, the lava's cover (`Bg_Breakwall` 0x6000), a last roll
//!   round the corners, then he rolls into the lava: his textures erased pixel by pixel into his
//!   object's RAM (`func_808C12C4`, as Queen Gohma's: `ObjectContext::written`), the floor's lava
//!   cooling to rock (`unk_1C6`), the boss-clear music and the heart container, the camera back,
//!   the blue warp (`Door_Warp1`, `WARP_DUNGEON_CHILD`, his child) and the room cleared; he sinks.
//!
//! **The lava floor** is a room texture he writes (`gDodongosCavernBossLavaFloorTex`, room 1's,
//! through segment 3): while Link is below y -1000 every update copies his overlay's
//! `sLavaFloorLavaTex` into it wobbling (`func_808C1554`) by `unk_224`; as he dies `unk_1C6` copies
//! `sLavaFloorRockTex` into the overlay's lava texture 20 texels a frame and the wobble dies down;
//! a room already cleared gets the rock at once (`BossDodongo_Init`). The room's RAM is play state
//! (`oot_game::room::RoomRam`), shown in the room's draws by its source address (ADR 0075). The
//! two overlay textures are his: an overlay's data is loaded with its first instance.
//!
//! **His effects:** his own 80 particles (`BossDodongoEffect`: sparks rising from the lava, his
//! `play->specialEffects`), `Effect_Ss_G_Magma` (the lava's bubbles) and `Effect_Ss_G_Magma2` (its
//! splashes, and dark smoke as it cools), both spawned round the arena's centre (-890, -3304)
//! while Link is below y -1000, their rates by `unk_224`.
//!
//! **His draw:** `object_kingdodongo_Skel_01B310` (48 limbs) in his own fog (`colorFilter*`: the
//! scene's, white while hit, red while down), turned by `unk_23C` (z) and `unk_1C4` (his roll, x);
//! every limb but 6 and 7 wobbled by its own `unk_25C` (a slow random drift, `unk_324`) and
//! squashed by `unk_208`, limbs 33 and 48 (his tail's base) scaled by `unk_1F8`, each inside its own
//! push and pop (the limb's own vertices, not its children's). Its effects on the game (his
//! points: `vec`, the focus, `firePos`, `mouthPos`, his feet; the collider's spheres) run in
//! `draw_update`.
//!
//! His circle shadow is the shape's (`ActorShape_Init`'s, drawn by `Actor_Draw`; the death shrinks
//! `shape.shadowScale`).
//!
//! Not ported: the rumble (`Rumble_Override`, logged).
//!
//! **@bug (game)**, kept: his profile's id is `ACTOR_EN_DODONGO` (0x12), so his actor's `id` is the
//! Dodongo's; the decay reads `D_808C7000[256]` past the table's 254 entries (the padding's two
//! zeros at 254 and 255, then the collider data's first byte, 3), so its last step writes again
//! (`func_808C1230` and `func_808C1278` at index 0, `func_808C11D0` the next texture's first pixel,
//! `func_808C1190` `Tex_015990`'s first pixel, `func_808C1200` the title card's first texel pair,
//! which isn't in the port's RAM: logged).
//!
//! Params: unused (the data's 0xFFFF).
//! Flags: reads `EVENTCHKINF_BEGAN_KING_DODONGO_BATTLE` (set: a second try, the intro cut short)
//! and sets it as the fight starts; reads the room's clear flag (set: beaten, the warp and the
//! heart container at once, the floor rock) and sets it at his death.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Sphere16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues, SourceImage};
use eng_math::{cos_s, sin_s, smooth_step_to_f, smooth_step_to_s, vec3f_yaw};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_BOSS, ACTORCAT_EXPLOSIVE, ActorImpl, ActorProfile, actor_spawn_floor_dust_ring, audio_play_actor_sfx2, enemy_start_finishing_blow, func_80033480};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::bgm::{seq_cmd1, start_seq};
use oot_game::audio::sfx::*;
use oot_game::audio::{NA_BGM_BOSS_CLEAR, NA_BGM_FIRE_BOSS, SEQ_PLAYER_BGM_MAIN};
use oot_game::camera::{CAM_ID_MAIN, CAM_STAT_ACTIVE, CAM_STAT_UNK3, CAM_STAT_WAIT, f_atan2f};
use oot_game::collision_check::*;
use oot_game::event_chk_inf;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, SEGMENT_CALL, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_ONCE, Anim, SkelAnimeStd};
use oot_game::sys_matrix::MtxF;

use crate::boss_goma::{ACTOR_DOOR_WARP1, ACTOR_ITEM_B_HEART, SUB_CAM_ID_DONE, WARP_DUNGEON_CHILD};

/// `ACTOR_BOSS_DODONGO`, `ACTOR_EN_DODONGO`, `ACTOR_EN_BDFIRE`, `ACTOR_BG_BREAKWALL`
/// (`actor_table.h`).
pub const ACTOR_BOSS_DODONGO: i16 = 0x0027;
pub const ACTOR_EN_DODONGO: i16 = 0x0012;
pub const ACTOR_EN_BDFIRE: i16 = crate::en_bdfire::ACTOR_EN_BDFIRE;
const ACTOR_BG_BREAKWALL: i16 = crate::bg_breakwall::ACTOR_BG_BREAKWALL;
pub const OBJECT: &str = "object_kingdodongo";
const SKELETON: &str = "object_kingdodongo_Skel_01B310";

/// `Boss_Dodongo_Profile`: its id is `ACTOR_EN_DODONGO` (@bug (game): the actor's `id` is the
/// Dodongo's).
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_DODONGO,
    name: "Boss_Dodongo",
    category: ACTORCAT_BOSS,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `params`: 0xFFFF in the data, read by nothing.
pub const PARAMS: Field = Field::whole("params", Kind::Unused, "0xFFFF in the data; read by nothing");

/// `EVENTCHKINF_BEGAN_KING_DODONGO_BATTLE` (`save.h`).
pub const EVENTCHKINF_BEGAN_KING_DODONGO_BATTLE: u16 = 0x71;

/// Boss_Dodongo's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[PARAMS],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_BEGAN_KING_DODONGO_BATTLE)).reads().sets().note("the fight begun: a second try starts him beside Link"),
        FlagUse::new(Flag::Clear).reads().sets().note("beaten: the warp, the heart container and the cooled floor at once; set at his death"),
    ],
    todo: false,
};

/// `NAVI_ENEMY_KING_DODONGO` (`actor.h`).
const NAVI_ENEMY_KING_DODONGO: u8 = 0x0C;
/// `PLAYER_CSACTION_*` (`player.h`): 1 standing still, 7 the end of a cutscene, 9 a fall's
/// landing turned to look, 12 a step back in surprise; 10 and up the warps' and the scripts'.
const PLAYER_CSACTION_1: u8 = 1;
const PLAYER_CSACTION_7: u8 = 7;
const PLAYER_CSACTION_9: u8 = 9;
const PLAYER_CSACTION_10: u8 = 10;
const PLAYER_CSACTION_12: u8 = 12;
/// `BOSS_DODONGO_EFFECT_COUNT`.
pub const BOSS_DODONGO_EFFECT_COUNT: usize = 80;
/// The skeleton's limbs.
pub const LIMB_COUNT: usize = 48;

/// The arena's centre on the lava, and its floor (`-890, -1523.76, -3304`).
pub const ARENA_CENTER: Vec3 = Vec3::new(-890.0, -1523.76, -3304.0);

/// `sCornerPositions`: the arena's four corners, walked round in this order (or back).
pub const S_CORNER_POSITIONS: [Vec3; 4] = [Vec3::new(-1390.0, 0.0, -3804.0), Vec3::new(-1390.0, 0.0, -2804.0), Vec3::new(-390.0, 0.0, -2804.0), Vec3::new(-390.0, 0.0, -3804.0)];

/// `gDodongosCavernBossLavaFloorTex`: its room, offset (`ddan_boss_v2.xml`, this ROM's: 0x21D8;
/// the other versions' `ddan_boss.xml` has 0x21C8) and size (RGBA16).
pub const FLOOR_ROOM: i8 = 1;
pub const FLOOR_ROOM_FILE: &str = "ddan_boss_room_1";
pub const FLOOR_TEX: &str = "gDodongosCavernBossLavaFloorTex";
pub const FLOOR_TEX_OFFSET: u32 = 0x21D8;
/// The floor's texels (32 by 64).
const FLOOR_TEXELS: usize = 2048;

/// `D_808C7000`: which steps of the decay erase (254 entries; the console reads on into the
/// padding and the collider data: `decay_table`).
pub const D_808C7000: [u8; 254] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 0, 1, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 0, 0, 1, 0, 0, 1, 1, //
    1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 0, 0, 1, 1, 1, 1, 1, 0, 0, 0, //
    1, 0, 0, 1, 1, 1, 0, 0, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 0, 0, 1, 0, 0, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1, 1, 0, 0, //
    0, 0, 1, 1, 0, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1, //
    1, 0, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 0, 0, 1, 1, 1, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 0, 1, 1, 0, 0, 0, 1, //
    1, 1, 1, 1, 0, 0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, //
    1, 0, 1, 1, 1, 1, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0,
];

/// `D_808C7000[i]` as the console reads it: past the table's 254 entries, the overlay's next
/// bytes (checked in the ROM: two of padding, then `sJntSphElementsInit[0]`'s `ELEM_MATERIAL_UNK3`).
/// @bug (game): step 256 reads 3.
pub fn decay_table(i: usize) -> u8 {
    match i {
        0..=253 => D_808C7000[i],
        254 | 255 => 0,
        256 => 3,
        _ => 0,
    }
}

/// The textures `func_808C12C4` erases (`object_kingdodongo.xml`), one after another in the file
/// from `Tex_015890` to `Tex_017210`'s end: the symbol, its offset, width and height (RGBA16).
pub const DECAY_TEXTURES: [(&str, u32, u32, u32); 10] = [
    ("object_kingdodongo_Tex_015890", 0x15890, 8, 16),
    ("object_kingdodongo_Tex_015990", 0x15990, 16, 32),
    ("object_kingdodongo_Tex_015D90", 0x15D90, 16, 16),
    ("object_kingdodongo_Tex_015F90", 0x15F90, 16, 32),
    ("object_kingdodongo_Tex_016390", 0x16390, 16, 16),
    ("object_kingdodongo_Tex_016590", 0x16590, 16, 16),
    ("object_kingdodongo_Tex_016790", 0x16790, 16, 16),
    ("object_kingdodongo_Tex_016990", 0x16990, 32, 16),
    ("object_kingdodongo_Tex_016E10", 0x16E10, 32, 16),
    ("object_kingdodongo_Tex_017210", 0x17210, 8, 32),
];
/// The decaying textures' region in `object_kingdodongo`'s RAM (`ObjectContext::written`):
/// `Tex_015890` to `Tex_017210`'s end (the bytes between `Tex_016990` and `Tex_016E10` in it).
pub const DECAY_REGION: u32 = 0x15890;
const DECAY_REGION_LEN: usize = 0x17410 - 0x15890;
/// The object's segment: where his lists load these textures from.
const SEGMENT_OBJECT: u32 = 0x0600_0000;

/// The images in place of his textures his object's RAM has (written once he decays):
/// `DrawParams::texture_images`, empty while it's as the file has it.
pub fn decay_images(play: &PlayState, bank: Option<usize>) -> Vec<SourceImage> {
    let Some(bytes) = bank.and_then(|b| play.object_ctx.written(b, DECAY_REGION)) else { return Vec::new() };
    DECAY_TEXTURES
        .iter()
        .map(|&(_, offset, w, h)| {
            let start = (offset - DECAY_REGION) as usize;
            let texels = bytes.get(start..start + (w * h * 2) as usize).unwrap_or(&[]);
            SourceImage { source: SEGMENT_OBJECT | offset, image: oot_game::object_ctx::rgba16_image(texels, w, h) }
        })
        .collect()
}

/// A texel of the region at byte `o` to 0 (a u16 store), if inside it.
fn clear_texel(region: &mut [u8], o: usize) {
    if let Some(px) = region.get_mut(o..o + 2) {
        px.fill(0);
    }
}

/// `func_808C12C4`: step `arg2` of his decay (`D_808C7000`) erases from his ten textures, in his
/// object's RAM (the region copied from the pack the first time).
fn func_808c12c4(play: &mut PlayState, bank: Option<usize>, arg2: i16) {
    let Some(bank) = bank else { return };
    if decay_table(arg2 as usize) == 0 {
        return;
    }
    let assets = play.assets.clone();
    let region = play.object_ctx.written_mut(bank, DECAY_REGION, || {
        let mut v = vec![0u8; DECAY_REGION_LEN];
        for (symbol, offset, _, _) in DECAY_TEXTURES {
            match assets.as_ref().map(|a| a.pack.texture(OBJECT, symbol)) {
                Some(Ok(t)) => {
                    let at = (offset - DECAY_REGION) as usize;
                    let n = t.texels.len().min(v.len() - at);
                    v[at..at + n].copy_from_slice(&t.texels[..n]);
                }
                _ => log::error!("Boss_Dodongo: {symbol} not in the pack"),
            }
        }
        v
    });
    let at = |k: usize| (DECAY_TEXTURES[k].1 - DECAY_REGION) as usize;
    let i = arg2 as usize;
    // func_808C1190(Tex_015890): arg0[arg2 / 2] = 0 (at 256: Tex_015990's first pixel).
    clear_texel(region, at(0) + (i / 2) * 2);
    // func_808C1200(Tex_017210): arg0[arg2] = 0; at 256 past the region (the title card's first
    // texels, which the port draws from the pack).
    if i < 256 {
        clear_texel(region, at(9) + i * 2);
    } else {
        log::debug!("Boss_Dodongo: func_808C1200 writes past Tex_017210 (gKingDodongoTitleCardTex's first texels): not modelled");
    }
    // func_808C11D0: arg0[arg2] = 0 (at 256: the next texture's first pixel).
    for k in [2, 4, 5, 6] {
        clear_texel(region, at(k) + i * 2);
    }
    // func_808C1230: two rows of a 16-wide texture.
    let index = (arg2 & 0xF) as usize + ((arg2 & 0xF0) as usize * 2);
    for k in [1, 3] {
        clear_texel(region, at(k) + (index + 16) * 2);
        clear_texel(region, at(k) + index * 2);
    }
    // func_808C1278: two pixels of a 32-wide row.
    let index = ((arg2 & 0xF) as usize * 2) + ((arg2 & 0xF0) as usize * 2);
    for k in [7, 8] {
        clear_texel(region, at(k) + (index + 1) * 2);
        clear_texel(region, at(k) + index * 2);
    }
}

/// `func_808C1554`: `floor_tex` (the overlay's lava, 32 by 64) into `dst` (the room's floor)
/// wobbling: each row shifted across by `sin(row + frame / 2) × amount`, then each column down by
/// `sin(column + frame × 0.8) × amount` rows, both wrapping.
pub fn func_808c1554(dst: &mut [u16], floor_tex: &[u16], arg2: i32, arg3: f32) {
    let mut sp54 = [0u16; FLOOR_TEXELS];
    let pi16 = std::f32::consts::PI / 16.0;
    let mut i = 0;
    while i < FLOOR_TEXELS {
        let k = ((i / 32) as i32 + ((arg2 as f32 * 50.0) / 100.0) as i16 as i32) & 0x1F;
        let temp = ((k as f32 * pi16).sin() * arg3) as i16 as i32;
        for i2 in 0..32 {
            sp54[i + ((temp + i2 as i32) & 0x1F) as usize] = floor_tex.get(i + i2).copied().unwrap_or(0);
        }
        i += 32;
    }
    for i in 0..32 {
        let k = (i as i32 + ((arg2 as f32 * 80.0) / 100.0) as i16 as i32) & 0x1F;
        let temp = (((k as f32 * pi16).sin() * arg3) as i16).wrapping_mul(32) as i32;
        let mut i2 = 0;
        while i2 < FLOOR_TEXELS {
            let temp2 = ((temp + i2 as i32) & 0x7FF) as usize;
            if let Some(d) = dst.get_mut(i + temp2) {
                *d = sp54[i + i2];
            }
            i2 += 32;
        }
    }
}

fn to_u16s(b: &[u8]) -> Vec<u16> {
    b.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect()
}

fn write_u16s(dst: &mut [u8], v: &[u16]) {
    for (c, x) in dst.chunks_exact_mut(2).zip(v) {
        c.copy_from_slice(&x.to_be_bytes());
    }
}

/// A sphere of `sJntSphElementsInit`: hits for 4 with `0xFFCFFFFF` and is hit by `0xFFCFFFFF`, on
/// `limb`, `radius` around `center`, scale 100.
const fn sphere(elem_material: u8, limb: u8, center: [i16; 3], radius: i16) -> ColliderJntSphElementInit {
    ColliderJntSphElementInit {
        info: ColliderElementInit {
            elem_material,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x04 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
            at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
            ac_elem_flags: ACELEM_ON,
            oc_elem_flags: OCELEM_ON,
        },
        limb,
        model_sphere: Sphere16 { center, radius },
        scale: 100,
    }
}

/// `sJntSphElementsInit`: his head (element 0, the one a sword's damage counts on), his tail's base,
/// his body, his legs and tail.
const JNT_SPH_ELEMENTS: [ColliderJntSphElementInit; 19] = [
    sphere(ELEM_MATERIAL_UNK3, 7, [8500, 1200, 0], 40),
    sphere(ELEM_MATERIAL_UNK3, 33, [2000, -2000, 0], 60),
    sphere(ELEM_MATERIAL_UNK2, 1, [0, 0, 0], 60),
    sphere(ELEM_MATERIAL_UNK2, 15, [0, 0, 0], 20),
    sphere(ELEM_MATERIAL_UNK2, 16, [0, 0, 0], 30),
    sphere(ELEM_MATERIAL_UNK2, 17, [0, 0, 0], 40),
    sphere(ELEM_MATERIAL_UNK2, 22, [0, 0, 0], 30),
    sphere(ELEM_MATERIAL_UNK2, 23, [0, 0, 0], 20),
    sphere(ELEM_MATERIAL_UNK2, 24, [0, 0, 0], 30),
    sphere(ELEM_MATERIAL_UNK2, 29, [0, 0, 0], 30),
    sphere(ELEM_MATERIAL_UNK2, 30, [0, 0, 0], 30),
    sphere(ELEM_MATERIAL_UNK2, 31, [0, 0, 0], 30),
    sphere(ELEM_MATERIAL_UNK2, 32, [0, 0, 0], 50),
    sphere(ELEM_MATERIAL_UNK2, 38, [0, 0, 0], 30),
    sphere(ELEM_MATERIAL_UNK2, 39, [0, 0, 0], 30),
    sphere(ELEM_MATERIAL_UNK2, 40, [0, 0, 0], 40),
    sphere(ELEM_MATERIAL_UNK2, 45, [0, 0, 0], 30),
    sphere(ELEM_MATERIAL_UNK2, 46, [0, 0, 0], 30),
    sphere(ELEM_MATERIAL_UNK2, 47, [0, 0, 0], 40),
];

/// `sJntSphInit`.
const JNT_SPH_INIT: ColliderInit =
    ColliderInit { col_type: COL_MATERIAL_HIT3, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_PLAYER, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_JNTSPH };

/// `BossDodongo_PostLimbDraw`'s points: on limb 6 his `vec` (`D_808CA45C`), focus (`D_808CA450`),
/// `firePos` (`D_808CA468`) and `mouthPos` (`D_808CA474`); on limbs 39 and 46 his feet.
const D_808CA450: Vec3 = Vec3::new(5000.0, -2500.0, 0.0);
const D_808CA45C: Vec3 = Vec3::ZERO;
const D_808CA468: Vec3 = Vec3::new(11500.0, -3000.0, 0.0);
const D_808CA474: Vec3 = Vec3::new(5000.0, -2000.0, 0.0);
const D_808CA480: Vec3 = Vec3::new(8000.0, 0.0, 0.0);
const D_808CA48C: Vec3 = Vec3::new(8000.0, 0.0, 0.0);

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    IntroCutscene,
    Walk,
    Inhale,
    BlowFire,
    Roll,
    Explode,
    LayDown,
    Vulnerable,
    GetUp,
    Damaged,
    DeathCutscene,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::IntroCutscene => "BossDodongo_IntroCutscene",
            Action::Walk => "BossDodongo_Walk",
            Action::Inhale => "BossDodongo_Inhale",
            Action::BlowFire => "BossDodongo_BlowFire",
            Action::Roll => "BossDodongo_Roll",
            Action::Explode => "BossDodongo_Explode",
            Action::LayDown => "BossDodongo_LayDown",
            Action::Vulnerable => "BossDodongo_Vulnerable",
            Action::GetUp => "BossDodongo_GetUp",
            Action::Damaged => "BossDodongo_Damaged",
            Action::DeathCutscene => "BossDodongo_DeathCutscene",
        }
    }
}

/// `BossDodongoEffect`: one of his particles.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BossDodongoEffect {
    /// `unk_00` (position), `unk_0C` (velocity), `unk_18` (acceleration).
    pub unk_00: Vec3,
    pub unk_0c: Vec3,
    pub unk_18: Vec3,
    /// `unk_24`: 0 free, 1 in use.
    pub unk_24: u8,
    /// `unk_25`: its age, from a random start (the colour cycles by it).
    pub unk_25: u8,
    pub color: [u8; 3],
    pub alpha: i16,
    /// `unk_2C`: its scale.
    pub unk_2c: f32,
}

/// His animations (`object_kingdodongo_Anim_*`).
#[derive(Default)]
struct Anims {
    a_001074: Option<Anim>,
    a_002d0c: Option<Anim>,
    a_003cf8: Option<Anim>,
    a_0042a8: Option<Anim>,
    a_004e0c: Option<Anim>,
    a_0061d4: Option<Anim>,
    a_008eec: Option<Anim>,
    a_009d10: Option<Anim>,
    a_00df38: Option<Anim>,
    a_00e848: Option<Anim>,
    a_00f0d8: Option<Anim>,
    a_01cae0: Option<Anim>,
    a_01d934: Option<Anim>,
}

fn last(a: &Option<Anim>) -> f32 {
    a.as_ref().map(|a| a.last_frame()).unwrap_or(0.0)
}

/// `RAD_TO_BINANG` (`(s16)(radians * (0x8000 / M_PI))`, truncated).
fn rad_to_binang(r: f32) -> i16 {
    (r * (32768.0 / std::f32::consts::PI)) as i32 as i16
}

pub struct BossDodongo {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: Anims,
    pub action: Action,
    pub health: i16,
    pub unk_196: i16,
    pub unk_198: i16,
    pub unk_19a: i16,
    pub cs_state: i16,
    /// `unk_19E`: his frame counter.
    pub unk_19e: i16,
    /// `unk_1A0`: the corner he's going to (`sCornerPositions`).
    pub unk_1a0: i16,
    /// `unk_1A2`: the way round the corners (0 on, 1 back).
    pub unk_1a2: i16,
    /// `unk_1A4`: Link's distance ahead of him while he's within 150 across (else 0).
    pub unk_1a4: i16,
    /// `unk_1A6`: the same behind him.
    pub unk_1a6: i16,
    pub num_wall_collisions: i16,
    /// `unk_1AA`: the walk's start played.
    pub unk_1aa: i16,
    /// `unk_1AC`: frames inhaling.
    pub unk_1ac: i16,
    /// `unk_1AE`: fire balls breathed.
    pub unk_1ae: i16,
    pub unk_1b0: i16,
    pub sub_cam_id: i16,
    /// `unk_1B6`: the intro camera's shake.
    pub unk_1b6: i16,
    pub player_yaw_in_range: bool,
    pub player_pos_in_range: bool,
    /// `unk_1BC`: 0 fighting, 1 in a cutscene, 2 the intro's walk without step sounds at him.
    pub unk_1bc: i16,
    /// `unk_1BE`: the red fog while down (1000 and up: dark red).
    pub unk_1be: i16,
    /// `unk_1C0`: the hit flash's frames (white on odd ones from 2).
    pub unk_1c0: i16,
    /// `unk_1C2`: where the lava's cooling copies next.
    pub unk_1c2: i16,
    /// `unk_1C4`: his roll (x).
    pub unk_1c4: i16,
    /// `unk_1C6`: the lava cooling.
    pub unk_1c6: i16,
    /// `unk_1C8`: the fire breath's light.
    pub unk_1c8: i16,
    /// `unk_1CC`: the decay's step.
    pub unk_1cc: i16,
    /// `unk_1DA`, `unk_1DC`, `unk_1DE`: timers.
    pub unk_1da: i16,
    pub unk_1dc: i16,
    pub unk_1de: i16,
    pub unk_1e0: i16,
    /// `unk_1E2`: inhaling this frame (`En_Bdfire`'s flame reads it).
    pub unk_1e2: u8,
    pub unk_1e3: i8,
    /// `unk_1E4`: his speed to his corner.
    pub unk_1e4: f32,
    /// `unk_1E8`: his turn's speed.
    pub unk_1e8: f32,
    pub unk_1ec: f32,
    /// `unk_1F8`: his tail's base's scale (limbs 33 and 48).
    pub unk_1f8: f32,
    pub unk_1fc: f32,
    pub unk_200: f32,
    /// `unk_204`: the sub camera's follow speed.
    pub unk_204: f32,
    /// `unk_208`: the limbs' squash.
    pub unk_208: f32,
    /// `unk_20C`: the intro camera's tilt (its up vector's x and z).
    pub unk_20c: f32,
    pub color_filter_r: f32,
    pub color_filter_g: f32,
    pub color_filter_b: f32,
    pub color_filter_min: f32,
    pub color_filter_max: f32,
    /// `unk_224`: the lava's wobble (2 hot, 0 cooled).
    pub unk_224: f32,
    /// `unk_228`: `shape.yOffset`'s target.
    pub unk_228: f32,
    pub unk_22c: f32,
    pub unk_230: f32,
    pub unk_234: f32,
    pub unk_238: f32,
    /// `unk_23C`: his tilt (z).
    pub unk_23c: f32,
    /// `unk_240`: the fire breath's red light.
    pub unk_240: f32,
    /// `unk_244`: the fire breath's screen flush (`MREG(68)`).
    pub unk_244: f32,
    /// `unk_25C`: each limb's wobble; `unk_324` its drift a frame.
    pub unk_25c: [f32; 50],
    pub unk_324: [f32; 50],
    pub vec: Vec3,
    pub fire_pos: Vec3,
    pub unk_404: Vec3,
    pub unk_410: Vec3,
    pub mouth_pos: Vec3,
    pub sub_cam_eye: Vec3,
    pub sub_cam_at: Vec3,
    pub collider: ColliderJntSph,
    pub effects: [BossDodongoEffect; BOSS_DODONGO_EFFECT_COUNT],
    /// The overlay's `sLavaFloorLavaTex` (its first 2048 texels, the ones read) and
    /// `sLavaFloorRockTex`: loaded with the overlay.
    pub lava_tex: Vec<u16>,
    pub rock_tex: Vec<u16>,
}

impl BossDodongo {
    /// `BossDodongo_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: ATTENTION_RANGE_5, NAVI_ENEMY_KING_DODONGO, gravity -3, lockOnArrowOffset 8200.
        actor.target_mode = 5;
        actor.navi_enemy_id = NAVI_ENEMY_KING_DODONGO;
        actor.gravity = -3.0;
        actor.target_arrow_offset = 8200.0;
        actor.shape_init(9200.0, oot_game::shadow::ShadowDraw::Circle, 250.0);
        actor.scale = Vec3::splat(0.01);
        let mut anims = Anims::default();
        let mut skeleton = None;
        let (mut lava_tex, mut rock_tex) = (vec![0u16; 4096], vec![0u16; FLOOR_TEXELS]);
        if let Some(a) = play.assets.clone() {
            let anim = |name: &str| a.animation(OBJECT, &format!("object_kingdodongo_Anim_{name}")).map_err(|e| log::error!("Boss_Dodongo: {e:#}")).ok();
            skeleton = a.skeleton(OBJECT, SKELETON).map_err(|e| log::error!("Boss_Dodongo: {e:#}")).ok();
            anims = Anims {
                a_001074: anim("001074"),
                a_002d0c: anim("002D0C"),
                a_003cf8: anim("003CF8"),
                a_0042a8: anim("0042A8"),
                a_004e0c: anim("004E0C"),
                a_0061d4: anim("0061D4"),
                a_008eec: anim("008EEC"),
                a_009d10: anim("009D10"),
                a_00df38: anim("00DF38"),
                a_00e848: anim("00E848"),
                a_00f0d8: anim("00F0D8"),
                a_01cae0: anim("01CAE0"),
                a_01d934: anim("01D934"),
            };
            match (a.pack.texture("ovl_Boss_Dodongo", "sLavaFloorLavaTex"), a.pack.texture("ovl_Boss_Dodongo", "sLavaFloorRockTex")) {
                (Ok(l), Ok(r)) => {
                    lava_tex = to_u16s(&l.texels);
                    rock_tex = to_u16s(&r.texels);
                }
                _ => log::error!("Boss_Dodongo: his overlay's lava textures aren't in the pack"),
            }
        }
        // SkelAnime_Init(&object_kingdodongo_Skel_01B310, &object_kingdodongo_Anim_00F0D8), PlayLoop.
        let limbs = skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(LIMB_COUNT);
        let skel = SkelAnimeStd::init_flex(limbs, anims.a_00f0d8.clone());
        let mut b = BossDodongo {
            actor,
            skel,
            skeleton,
            anims,
            action: Action::IntroCutscene,
            health: 0,
            unk_196: 0,
            unk_198: 0,
            unk_19a: 0,
            cs_state: 0,
            unk_19e: 0,
            unk_1a0: 0,
            unk_1a2: 0,
            unk_1a4: 0,
            unk_1a6: 0,
            num_wall_collisions: 0,
            unk_1aa: 0,
            unk_1ac: 0,
            unk_1ae: 0,
            unk_1b0: 0,
            sub_cam_id: SUB_CAM_ID_DONE,
            unk_1b6: 0,
            player_yaw_in_range: false,
            player_pos_in_range: false,
            unk_1bc: 0,
            unk_1be: 0,
            unk_1c0: 0,
            unk_1c2: 0,
            unk_1c4: 0,
            unk_1c6: 0,
            unk_1c8: 0,
            unk_1cc: 0,
            unk_1da: 0,
            unk_1dc: 0,
            unk_1de: 0,
            unk_1e0: 0,
            unk_1e2: 0,
            unk_1e3: 0,
            unk_1e4: 0.0,
            unk_1e8: 0.0,
            unk_1ec: 0.0,
            unk_1f8: 0.0,
            unk_1fc: 0.0,
            unk_200: 0.0,
            unk_204: 0.0,
            unk_208: 0.0,
            unk_20c: 0.0,
            color_filter_r: 0.0,
            color_filter_g: 0.0,
            color_filter_b: 0.0,
            color_filter_min: 0.0,
            color_filter_max: 0.0,
            unk_224: 0.0,
            unk_228: 0.0,
            unk_22c: 0.0,
            unk_230: 0.0,
            unk_234: 0.0,
            unk_238: 0.0,
            unk_23c: 0.0,
            unk_240: 0.0,
            unk_244: 0.0,
            unk_25c: [0.0; 50],
            unk_324: [0.0; 50],
            vec: Vec3::ZERO,
            fire_pos: Vec3::ZERO,
            unk_404: Vec3::ZERO,
            unk_410: Vec3::ZERO,
            mouth_pos: Vec3::ZERO,
            sub_cam_eye: Vec3::ZERO,
            sub_cam_at: Vec3::ZERO,
            collider: ColliderJntSph::new(&JNT_SPH_INIT, &JNT_SPH_ELEMENTS),
            effects: [BossDodongoEffect::default(); BOSS_DODONGO_EFFECT_COUNT],
            lava_tex,
            rock_tex,
        };
        b.unk_1f8 = 1.0;
        b.setup_intro_cutscene();
        b.health = 12;
        b.color_filter_min = 995.0;
        b.actor.col_chk_info.mass = MASS_IMMOVABLE;
        b.color_filter_max = 1000.0;
        b.unk_224 = 2.0;
        b.unk_228 = 9200.0;
        if play.flags.get_clear(play.room_ctx.cur.num) {
            // KD is dead.
            b.actor.kill();
            let me = &mut b.actor;
            if let Err(e) = play.actor_spawn_as_child(me, ACTOR_DOOR_WARP1, ARENA_CENTER, [0; 3], WARP_DUNGEON_CHILD) {
                log::warn!("Boss_Dodongo: the blue warp not spawned: {e:?}");
            }
            if let Err(e) = play.actor_spawn(ACTOR_BG_BREAKWALL, ARENA_CENTER, [0; 3], 0x6000) {
                log::warn!("Boss_Dodongo: the lava's cover not spawned: {e:?}");
            }
            if let Err(e) = play.actor_spawn(ACTOR_ITEM_B_HEART, Vec3::new(-690.0, -1523.76, -3304.0), [0; 3], 0) {
                log::warn!("Boss_Dodongo: the heart container not spawned: {e:?}");
            }
            let rock = b.rock_tex.clone();
            Self::write_floor(play, |floor| floor[..FLOOR_TEXELS].copy_from_slice(&rock[..FLOOR_TEXELS]));
        }
        b.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        Box::new(b)
    }

    /// The floor's texture in the current room's RAM (`SEGMENTED_TO_VIRTUAL` of room 1's
    /// `gDodongosCavernBossLavaFloorTex`: segment 3, whichever room is current), as texels to change.
    fn write_floor(play: &mut PlayState, f: impl FnOnce(&mut [u16])) {
        let room = play.room_ctx.cur.num;
        if room != FLOOR_ROOM {
            log::debug!("Boss_Dodongo: the lava floor written into room {room}'s RAM (the C's segment 3): not modelled");
            return;
        }
        let assets = play.assets.clone();
        let region = play.room_ram.region_mut(room, FLOOR_TEX_OFFSET, (32, 64), || match assets.as_ref().map(|a| a.pack.texture(FLOOR_ROOM_FILE, FLOOR_TEX)) {
            Some(Ok(t)) => t.texels.clone(),
            _ => {
                log::error!("Boss_Dodongo: {FLOOR_TEX} not in the pack");
                vec![0; FLOOR_TEXELS * 2]
            }
        });
        let mut texels = to_u16s(region);
        texels.resize(FLOOR_TEXELS, 0);
        f(&mut texels);
        write_u16s(region, &texels);
    }

    /// `Animation_Change(&skelAnime, anim, speed, start, end, mode, morph)`.
    fn change(&mut self, anim: Option<Anim>, speed: f32, start: f32, end: f32, mode: u8, morph: f32) {
        if let Some(a) = anim {
            self.skel.change(a, speed, start, end, mode, morph);
        }
    }

    fn player_pos(play: &PlayState) -> Vec3 {
        play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or_default()
    }

    fn player_mut(play: &mut PlayState) -> Option<&mut Actor> {
        play.player.and_then(|h| play.actors.actor_mut(h))
    }

    /// `Player_SetCsActionWithHaltedActors(play, &this->actor, mode)`.
    fn player_cs(play: &mut PlayState, mode: u8) {
        let me = play.cur_actor;
        play.player_set_cs_action_with_halted_actors(me, mode);
    }

    /// `func_808C17C8`: a particle in the first free of `count_limit` slots, its age random.
    fn func_808c17c8(&mut self, play: &mut PlayState, arg1: Vec3, arg2: Vec3, arg3: Vec3, arg4: f32, count_limit: usize) {
        for eff in self.effects.iter_mut().take(count_limit) {
            if eff.unk_24 == 0 {
                eff.unk_24 = 1;
                eff.unk_00 = arg1;
                eff.unk_0c = arg2;
                eff.unk_18 = arg3;
                eff.unk_2c = arg4 / 1000.0;
                eff.alpha = 255;
                eff.unk_25 = play.rand.zero_float(10.0) as i16 as u8;
                break;
            }
        }
    }

    /// `BossDodongo_AteExplosive`: the first explosive (not himself) within 40 of his mouth on every
    /// axis is killed (eaten).
    fn ate_explosive(&mut self, play: &mut PlayState) -> bool {
        let me = play.cur_actor;
        let list: Vec<_> = play.actors.category(ACTORCAT_EXPLOSIVE).to_vec();
        for h in list {
            if Some(h) == me {
                continue;
            }
            let Some(a) = play.actors.actor_mut(h) else { continue };
            let d = a.world_pos - self.mouth_pos;
            if d.x.abs() < 40.0 && d.y.abs() < 40.0 && d.z.abs() < 40.0 {
                a.kill();
                return true;
            }
        }
        false
    }

    /// `BossDodongo_SetupIntroCutscene`: `Anim_00F0D8` looped (from a morph of 10).
    fn setup_intro_cutscene(&mut self) {
        let frames = last(&self.anims.a_00f0d8) as i16 as f32;
        self.change(self.anims.a_00f0d8.clone(), 1.0, 0.0, frames, ANIMMODE_LOOP, -10.0);
        self.action = Action::IntroCutscene;
        self.cs_state = 0;
        self.unk_1bc = 1;
    }

    /// `BossDodongo_IntroCutscene`.
    fn intro_cutscene(&mut self, play: &mut PlayState) {
        if self.unk_196 != 0 {
            self.unk_196 -= 1;
        }
        if self.unk_198 != 0 {
            self.unk_198 -= 1;
        }
        if self.unk_19a != 0 {
            self.unk_19a -= 1;
        }
        let began = play.save.get_event_chk_inf(EVENTCHKINF_BEGAN_KING_DODONGO_BATTLE);
        let mut state = self.cs_state;
        if state == 0 {
            if Self::player_pos(play).y < -1223.76 {
                self.cs_state = 1;
                self.actor.world_pos.x = -1390.0;
                self.actor.world_pos.z = -3374.0;
                self.unk_1a0 = 1;
            }
        } else if state == 1 {
            play.start_manual();
            Self::player_cs(play, PLAYER_CSACTION_1);
            play.clear_all_sub_cameras();
            self.sub_cam_id = play.create_sub_camera();
            play.change_camera_status(CAM_ID_MAIN, CAM_STAT_WAIT);
            play.change_camera_status(self.sub_cam_id, CAM_STAT_ACTIVE);
            self.cs_state = 2;
            self.unk_196 = 0x3C;
            self.unk_198 = 160;
            if let Some(p) = Self::player_mut(play) {
                p.world_pos.y = -1023.76;
            }
            self.sub_cam_eye.y = Self::player_pos(play).y - 480.0 + 50.0;
            // FALLTHROUGH
            state = 2;
        }
        match state {
            2 => {
                if self.unk_198 >= 131 {
                    if let Some(p) = Self::player_mut(play) {
                        p.world_pos.x = -890.0;
                        p.world_pos.z = -2804.0;
                        p.speed_xz = 0.0;
                        p.world_rot.y = 0x3FFF;
                        p.shape_rot.y = 0x3FFF;
                    }
                    let pp = Self::player_pos(play);
                    self.sub_cam_eye.x = -890.0;
                    self.sub_cam_eye.z = pp.z - 100.0;
                    self.sub_cam_at = Vec3::new(pp.x, pp.y + 20.0, pp.z);
                }
                if self.unk_198 == 110 {
                    Self::player_cs(play, PLAYER_CSACTION_9);
                }
                if self.unk_198 == 5 {
                    Self::player_cs(play, PLAYER_CSACTION_12);
                }
                let yaw = if self.unk_198 < 6 { -0x4001 } else { 0x3FFF };
                if let Some(p) = Self::player_mut(play) {
                    p.shape_rot.y = yaw;
                }
                self.unk_1bc = if self.unk_198 < 60 { 1 } else { 2 };
                self.walk(play);
                if self.unk_196 == 1 {
                    play.audio.queue_seq_cmd(seq_cmd1(SEQ_PLAYER_BGM_MAIN, 1));
                }
                if self.unk_196 == 0 {
                    let s = self.unk_204 * 20.0;
                    smooth_step_to_f(&mut self.sub_cam_eye.x, self.vec.x + 30.0, 0.2, s, 0.0);
                    smooth_step_to_f(&mut self.sub_cam_eye.y, self.vec.y, 0.2, s, 0.0);
                    smooth_step_to_f(&mut self.sub_cam_eye.z, self.vec.z + 10.0, 0.2, s, 0.0);
                    smooth_step_to_f(&mut self.unk_204, 1.0, 1.0, 0.02, 0.0);
                } else {
                    let pp = Self::player_pos(play);
                    self.sub_cam_at = Vec3::new(pp.x, pp.y + 20.0, pp.z);
                }
                if began {
                    if self.unk_198 == 100 {
                        self.actor.world_pos.x = -1114.0;
                        self.actor.world_pos.z = -2804.0;
                        self.actor.world_rot.y = 0x3FFF;
                        self.unk_1a2 = 0;
                        self.unk_1a0 = 2;
                        self.cs_state = 4;
                        self.unk_196 = 30;
                        self.unk_198 = 150;
                        self.unk_204 = 0.0;
                        self.change(self.anims.a_008eec.clone(), 1.0, 0.0, last(&self.anims.a_008eec), ANIMMODE_ONCE, 0.0);
                        self.skel.update();
                    }
                } else if self.unk_198 == 0 {
                    self.cs_state = 3;
                    self.unk_19e = 0x14;
                    self.unk_204 = 0.0;
                }
            }
            3 => {
                self.walk(play);
                let s = self.unk_204 * 20.0;
                smooth_step_to_f(&mut self.unk_20c, (self.unk_19e as f32 * 0.05).sin() * 0.1, 1.0, 0.01, 0.0);
                smooth_step_to_f(&mut self.sub_cam_eye.x, self.vec.x + 90.0, 0.2, s, 0.0);
                smooth_step_to_f(&mut self.sub_cam_eye.y, self.vec.y + 50.0, 0.2, s, 0.0);
                smooth_step_to_f(&mut self.sub_cam_eye.z, self.vec.z, 0.2, s, 0.0);
                smooth_step_to_f(&mut self.sub_cam_at.y, self.vec.y - 10.0, 0.2, s, 0.0);
                smooth_step_to_f(&mut self.unk_204, 1.0, 1.0, 0.02, 0.0);
                if (Self::player_pos(play).x - self.actor.world_pos.x).abs() < 200.0 {
                    self.cs_state = 4;
                    self.unk_196 = 0x1E;
                    self.unk_198 = 0x96;
                    self.unk_204 = 0.0;
                    self.change(self.anims.a_008eec.clone(), 1.0, 0.0, last(&self.anims.a_008eec), ANIMMODE_ONCE, -5.0);
                }
            }
            4 => {
                smooth_step_to_f(&mut self.unk_20c, 0.0, 1.0, 0.01, 0.0);
                let phi_f0 = if began { -50.0 } else { 0.0 };
                let pp = Self::player_pos(play);
                let s = self.unk_204 * 20.0;
                smooth_step_to_f(&mut self.sub_cam_eye.x, pp.x + phi_f0 + 70.0, 0.2, s, 0.0);
                smooth_step_to_f(&mut self.sub_cam_eye.y, pp.y + 10.0, 0.2, s, 0.0);
                smooth_step_to_f(&mut self.sub_cam_eye.z, pp.z - 60.0, 0.2, s, 0.0);
                smooth_step_to_f(&mut self.sub_cam_at.x, self.vec.x, 0.2, s, 0.0);
                smooth_step_to_f(&mut self.sub_cam_at.y, self.vec.y, 0.2, s, 0.0);
                smooth_step_to_f(&mut self.sub_cam_at.z, self.vec.z, 0.2, s, 0.0);
                smooth_step_to_f(&mut self.unk_204, 1.0, 1.0, 0.02, 0.0);
                if self.unk_196 == 0 {
                    self.skel.update();
                    smooth_step_to_f(&mut self.unk_208, 0.05, 1.0, 0.005, 0.0);
                }
                if self.unk_198 == 0x64 {
                    audio_play_actor_sfx2(play, NA_SE_EN_DODO_K_OTAKEBI);
                }
                if self.unk_198 == 0x5A {
                    if !began {
                        let &(object, offset, w, h) = &oot_game::title_card::BOSS_NAMES[1];
                        play.title_ctx.init_boss_name(object, offset, 160, 180, w, h);
                    }
                    play.audio.queue_seq_cmd(start_seq(SEQ_PLAYER_BGM_MAIN, 0, NA_BGM_FIRE_BOSS));
                }
                if self.unk_198 == 0 {
                    play.game_camera.eye = self.sub_cam_eye;
                    play.game_camera.eye_next = self.sub_cam_eye;
                    play.game_camera.at = self.sub_cam_at;
                    play.play_return_to_main_cam(self.sub_cam_id, 0);
                    self.sub_cam_id = SUB_CAM_ID_DONE;
                    play.stop_manual();
                    Self::player_cs(play, PLAYER_CSACTION_7);
                    self.setup_walk();
                    self.unk_1da = 50;
                    self.unk_1bc = 0;
                    if let Some(p) = Self::player_mut(play) {
                        p.shape_rot.y = -0x4002;
                    }
                    play.save.set_event_chk_inf(EVENTCHKINF_BEGAN_KING_DODONGO_BATTLE);
                }
            }
            _ => {}
        }
        if self.sub_cam_id != SUB_CAM_ID_DONE {
            if self.unk_1b6 != 0 {
                self.unk_1b6 -= 1;
            }
            let shake = |b: i16| {
                let phi_f0 = ((b as f32 * 3.1415 * 90.0) / 180.0).sin();
                b as f32 * phi_f0 * 0.7
            };
            let eye = Vec3::new(self.sub_cam_eye.x, shake(self.unk_1b6) + self.sub_cam_eye.y, self.sub_cam_eye.z);
            let at = Vec3::new(self.sub_cam_at.x, shake(self.unk_1b6) + self.sub_cam_at.y, self.sub_cam_at.z);
            let up = Vec3::new(self.unk_20c, 1.0, self.unk_20c);
            play.camera_set_at_eye_up(self.sub_cam_id, at, eye, up);
        }
    }

    /// `BossDodongo_SetupDamaged`.
    fn setup_damaged(&mut self) {
        if self.action != Action::Damaged {
            self.change(self.anims.a_001074.clone(), 1.0, 0.0, last(&self.anims.a_001074), ANIMMODE_ONCE, -5.0);
            self.action = Action::Damaged;
        }
        self.unk_1da = 100;
    }

    /// `BossDodongo_SetupExplode`: the bomb swallowed blows 35 frames on.
    fn setup_explode(&mut self) {
        self.change(self.anims.a_00e848.clone(), 1.0, 0.0, last(&self.anims.a_00e848), ANIMMODE_ONCE, -5.0);
        self.action = Action::Explode;
        self.unk_1b0 = 10;
        self.unk_1c0 = 2;
        self.unk_1da = 35;
        self.unk_1fc = 50.0;
        self.unk_200 = 300.0;
    }

    /// `BossDodongo_SetupWalk`.
    fn setup_walk(&mut self) {
        self.change(self.anims.a_01d934.clone(), 1.0, 0.0, last(&self.anims.a_01d934), ANIMMODE_ONCE, -10.0);
        self.unk_1aa = 0;
        self.action = Action::Walk;
        self.unk_1da = 0;
        self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
        self.unk_1e4 = 0.0;
    }

    /// `BossDodongo_SetupRoll`.
    fn setup_roll(&mut self) {
        self.change(self.anims.a_00df38.clone(), 1.0, 0.0, 59.0, ANIMMODE_ONCE, -5.0);
        self.action = Action::Roll;
        self.num_wall_collisions = 0;
        self.unk_1da = 27;
    }

    /// `BossDodongo_SetupBlowFire`.
    fn setup_blow_fire(&mut self) {
        self.unk_1e4 = 0.0;
        self.actor.speed_xz = 0.0;
        self.change(self.anims.a_0061d4.clone(), 1.0, 0.0, last(&self.anims.a_0061d4), ANIMMODE_ONCE, 0.0);
        self.action = Action::BlowFire;
        self.unk_1da = 50;
        self.unk_1ae = 0;
    }

    /// `BossDodongo_SetupInhale`.
    fn setup_inhale(&mut self) {
        self.actor.speed_xz = 0.0;
        self.change(self.anims.a_008eec.clone(), 1.0, 0.0, last(&self.anims.a_008eec), ANIMMODE_ONCE, -5.0);
        self.action = Action::Inhale;
        self.unk_1da = 100;
        self.unk_1ac = 0;
        self.unk_1e2 = 1;
    }

    /// `BossDodongo_Damaged`.
    fn damaged(&mut self) {
        self.skel.update();
        smooth_step_to_f(&mut self.unk_1f8, 1.0, 0.5, 0.02, 0.001);
        smooth_step_to_f(&mut self.unk_208, 0.05, 1.0, 0.005, 0.0);
        if self.skel.on_frame(last(&self.anims.a_001074)) {
            self.setup_roll();
        }
    }

    /// `BossDodongo_Explode`: at 0, the blast inside him (30 dust puffs from his middle), lying
    /// down, 2 off his health (never the last).
    fn explode(&mut self, play: &mut PlayState) {
        const DUST_PRIM: [u8; 4] = [255, 255, 0, 255];
        const DUST_ENV: [u8; 4] = [255, 10, 0, 255];
        smooth_step_to_f(&mut self.unk_208, 0.05, 1.0, 0.005, 0.0);
        self.skel.update();
        if self.unk_1da == 0 {
            for _ in 0..30 {
                let vel = Vec3::new(play.rand.centered_float(20.0), play.rand.centered_float(20.0), play.rand.centered_float(20.0));
                let accel = vel * -0.1;
                let w = self.actor.world_pos;
                let pos = Vec3::new(w.x + vel.x * 3.0, w.y + 90.0 + vel.y * 3.0, w.z + vel.z * 3.0);
                play.with_ss(|s| s.func_8002836c(pos, vel, accel, DUST_PRIM, DUST_ENV, 500, 10, 10));
            }
            self.change(self.anims.a_004e0c.clone(), 1.0, 0.0, last(&self.anims.a_004e0c), ANIMMODE_ONCE, -5.0);
            self.action = Action::LayDown;
            audio_play_actor_sfx2(play, NA_SE_IT_BOMB_EXPLOSION);
            audio_play_actor_sfx2(play, NA_SE_EN_DODO_K_DAMAGE);
            play.actor_request_quake_and_rumble(4, 10);
            self.health -= 2;
            // Make sure not to die from the bomb explosion.
            if self.health <= 0 {
                self.health = 1;
            }
        }
    }

    /// `BossDodongo_LayDown`.
    fn lay_down(&mut self) {
        self.unk_1be = 10;
        smooth_step_to_f(&mut self.unk_1f8, 1.3, 1.0, 0.1, 0.001);
        self.skel.update();
        if self.skel.on_frame(last(&self.anims.a_004e0c)) {
            self.change(self.anims.a_0042a8.clone(), 1.0, 0.0, last(&self.anims.a_0042a8), ANIMMODE_LOOP, -5.0);
            self.action = Action::Vulnerable;
            self.unk_1da = 100;
        }
    }

    /// `BossDodongo_Vulnerable`.
    fn vulnerable(&mut self, play: &mut PlayState) {
        audio_play_actor_sfx2(play, NA_SE_EN_DODO_K_DOWN - SFX_FLAG);
        self.unk_1be = 10;
        smooth_step_to_f(&mut self.unk_1f8, 1.0, 0.5, 0.02, 0.001);
        smooth_step_to_f(&mut self.unk_208, 0.05, 1.0, 0.005, 0.0);
        self.skel.update();
        if self.unk_1da == 0 {
            self.change(self.anims.a_009d10.clone(), 1.0, 0.0, last(&self.anims.a_009d10), ANIMMODE_ONCE, -5.0);
            self.action = Action::GetUp;
        }
    }

    /// `BossDodongo_GetUp`.
    fn get_up(&mut self) {
        self.skel.update();
        if self.skel.on_frame(last(&self.anims.a_009d10)) {
            self.setup_roll();
        }
    }

    /// `BossDodongo_BlowFire`: the cry at frame 12, the breath's light from 17, a fire ball each
    /// frame from 18 to 34.
    fn blow_fire(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.skel.on_frame(12.0) {
            audio_play_actor_sfx2(play, NA_SE_EN_DODO_K_CRY);
        }
        if self.skel.on_frame(17.0) {
            self.unk_1c8 = 28;
        }
        if self.skel.cur_frame > 17.0 && self.skel.cur_frame < 35.0 {
            self.spawn_fire(play, self.unk_1ae);
            self.unk_1ae += 1;
            smooth_step_to_f(&mut self.unk_244, 0.0, 1.0, 8.0, 0.0);
        }
        if self.unk_1da == 0 {
            self.setup_roll();
        }
    }

    /// `BossDodongo_Inhale`.
    fn inhale(&mut self, play: &mut PlayState) {
        self.unk_1e2 = 1;
        if self.unk_1ac > 20 {
            audio_play_actor_sfx2(play, NA_SE_EN_DODO_K_BREATH - SFX_FLAG);
        }
        smooth_step_to_f(&mut self.unk_208, 0.05, 1.0, 0.005, 0.0);
        self.skel.update();
        if self.unk_1da == 0 {
            self.setup_blow_fire();
        } else {
            self.unk_1ac += 1;
            if self.unk_1ac > 20 && self.unk_1ac < 82 && self.ate_explosive(play) {
                audio_play_actor_sfx2(play, NA_SE_EN_DODO_K_DRINK);
                self.setup_explode();
            }
        }
    }

    /// `BossDodongo_Walk`: towards his corner (stepping on frames 1 and 31, dust and a quake or the
    /// sub camera's shake); at it, the next. Fighting: Link ahead within 500 off a corner, the inhale
    /// and the flame; Link neither ahead nor near a corner, a roll.
    fn walk(&mut self, play: &mut PlayState) {
        if self.unk_1aa == 0 {
            if self.skel.on_frame(14.0) {
                if let Some(a) = self.anims.a_01cae0.clone() {
                    let l = a.last_frame();
                    self.skel.change(a, 1.0, 0.0, l, ANIMMODE_LOOP, 0.0);
                }
                self.unk_1aa = 1;
            }
        } else if self.unk_1bc != 2 {
            let f = self.skel.cur_frame as i32;
            if f == 1 || f == 31 {
                let pos = if f == 1 { self.unk_410 } else { self.unk_404 };
                let a = self.actor.clone();
                actor_spawn_floor_dust_ring(play, &a, pos, 25.0, 10, 8.0, 500, 10, false);
                if self.unk_1bc != 0 {
                    play.audio.play_sfx_centered(NA_SE_EN_DODO_K_WALK);
                } else {
                    audio_play_actor_sfx2(play, NA_SE_EN_DODO_K_WALK);
                }
                if self.sub_cam_id == SUB_CAM_ID_DONE {
                    play.actor_request_quake_and_rumble(4, 10);
                } else {
                    self.unk_1b6 = 10;
                    log::debug!("Boss_Dodongo: Rumble_Override(0, 180, 20, 100) not ported");
                }
            }
        }
        self.skel.update();
        let corner = S_CORNER_POSITIONS[self.unk_1a0 as usize & 3];
        self.unk_1ec = 0.7;
        smooth_step_to_f(&mut self.unk_1e4, self.unk_1ec * 4.0, 1.0, self.unk_1ec * 0.25, 0.0);
        smooth_step_to_f(&mut self.actor.world_pos.x, corner.x, 0.3, self.unk_1e4, 0.0);
        smooth_step_to_f(&mut self.actor.world_pos.z, corner.z, 0.3, self.unk_1e4, 0.0);
        let sp48 = corner.x - self.actor.world_pos.x;
        let sp44 = corner.z - self.actor.world_pos.z;
        smooth_step_to_f(&mut self.unk_1e8, 2000.0, 1.0, self.unk_1ec * 80.0, 0.0);
        smooth_step_to_s(&mut self.actor.world_rot.y, rad_to_binang(f_atan2f(sp48, sp44)), 5, (self.unk_1ec * self.unk_1e8) as i32 as i16, 5);
        smooth_step_to_s(&mut self.unk_1c4, 0, 2, 2000, 0);
        if sp48.abs() <= 5.0 && sp44.abs() <= 5.0 {
            self.unk_1e8 = 0.0;
            self.unk_1e4 = 0.0;
            self.next_corner();
        }
        if self.unk_1da == 0 && self.unk_1bc == 0 {
            if self.actor.xz_dist_to_player < 500.0 && self.unk_1a4 != 0 && !self.player_pos_in_range {
                self.setup_inhale();
                self.spawn_fire(play, -1);
            }
            if !self.player_pos_in_range && !self.player_yaw_in_range {
                self.setup_roll();
            }
        }
    }

    /// The next corner round (`unk_1A2` 0) or back.
    fn next_corner(&mut self) {
        if self.unk_1a2 == 0 {
            self.unk_1a0 += 1;
            if self.unk_1a0 >= 4 {
                self.unk_1a0 = 0;
            }
        } else {
            self.unk_1a0 -= 1;
            if self.unk_1a0 < 0 {
                self.unk_1a0 = 3;
            }
        }
    }

    /// `BossDodongo_Roll`: a hop at 10 frames left of the curl, then rolling corner to corner (dust,
    /// a quake every 8 frames); at the second corner he lands from a hop, a quake and dust, and
    /// walks (turning back first if Link is behind him).
    fn roll(&mut self, play: &mut PlayState) {
        self.actor.flags |= ACTOR_FLAG_SFX_FOR_PLAYER_BODY_HIT;
        self.skel.update();
        if self.unk_1da == 10 {
            self.actor.velocity.y = 15.0;
            audio_play_actor_sfx2(play, NA_SE_EN_DODO_K_CRY);
        }
        if self.unk_1da == 1 {
            audio_play_actor_sfx2(play, NA_SE_EN_DODO_K_COLI2);
        }
        let corner = S_CORNER_POSITIONS[self.unk_1a0 as usize & 3];
        self.unk_1ec = 3.0;
        if self.unk_1da == 0 {
            smooth_step_to_f(&mut self.unk_1e4, self.unk_1ec * 5.0, 1.0, self.unk_1ec * 0.25, 0.0);
            smooth_step_to_f(&mut self.actor.world_pos.x, corner.x, 1.0, self.unk_1e4, 0.0);
            smooth_step_to_f(&mut self.actor.world_pos.z, corner.z, 1.0, self.unk_1e4, 0.0);
            self.unk_1c4 = self.unk_1c4.wrapping_add(2000);
            if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
                self.unk_228 = 7700.0;
                audio_play_actor_sfx2(play, NA_SE_EN_DODO_K_ROLL - SFX_FLAG);
                if self.unk_19e & 7 == 0 {
                    play.camera_request_quake(CAM_ID_MAIN, 1, 8);
                }
                if self.unk_19e & 1 == 0 {
                    let (a, p) = (self.actor.clone(), self.actor.world_pos);
                    actor_spawn_floor_dust_ring(play, &a, p, 40.0, 3, 8.0, 500, 10, false);
                }
            }
        }
        let sp4c = corner.x - self.actor.world_pos.x;
        let sp48 = corner.z - self.actor.world_pos.z;
        smooth_step_to_f(&mut self.unk_1e8, 2000.0, 1.0, self.unk_1ec * 100.0, 0.0);
        smooth_step_to_s(&mut self.actor.world_rot.y, rad_to_binang(f_atan2f(sp4c, sp48)), 5, (self.unk_1ec * self.unk_1e8) as i32 as i16, 0);
        if sp4c.abs() <= 15.0 && sp48.abs() <= 15.0 {
            self.num_wall_collisions += 1;
            if self.num_wall_collisions >= 2 {
                if self.unk_1a6 != 0 {
                    self.unk_1a2 = 1 - self.unk_1a2;
                }
                self.unk_1e8 = 0.0;
                self.unk_1e4 = 0.0;
                self.setup_walk();
                self.unk_228 = 9200.0;
                self.actor.velocity.y = 20.0;
                audio_play_actor_sfx2(play, NA_SE_EN_DODO_K_COLI);
                play.camera_request_quake(CAM_ID_MAIN, 6, 8);
                let w = self.actor.world_pos;
                func_80033480(play, Vec3::new(w.x, w.y + 60.0, w.z), 250.0, 40, 800, 10, 0);
                play.actor_request_quake_and_rumble(6, 15);
            } else {
                self.actor.velocity.y = 15.0;
                audio_play_actor_sfx2(play, NA_SE_EN_DODO_K_COLI2);
            }
            self.next_corner();
        }
    }

    /// `func_808C4F6C`: how far ahead of him Link is, within 150 across and 100 to 2000 ahead
    /// (else -1).
    fn func_808c4f6c(&self, play: &PlayState) -> f32 {
        Self::ahead(self.actor.world_pos, Self::player_pos(play), self.actor.world_rot.y.wrapping_neg())
    }

    /// `func_808C50A8`: the same behind him.
    fn func_808c50a8(&self, play: &PlayState) -> f32 {
        Self::ahead(self.actor.world_pos, Self::player_pos(play), (-0x8000i32 - self.actor.world_rot.y as i32) as i16)
    }

    fn ahead(me: Vec3, player: Vec3, angle: i16) -> f32 {
        let x_diff = player.x - me.x;
        let z_diff = player.z - me.z;
        let rotation = cos_s(angle);
        let sp2c = (sin_s(angle) * z_diff) + (rotation * x_diff);
        let rotation = sin_s(angle);
        let temp_f2 = (cos_s(angle) * z_diff) + (-rotation * x_diff);
        if sp2c.abs() < 150.0 && temp_f2 >= 100.0 && temp_f2 <= 2000.0 { temp_f2 } else { -1.0 }
    }

    /// `BossDodongo_PlayerYawCheck`: Link within 80° of his facing.
    fn player_yaw_check(&mut self, play: &PlayState) {
        let yaw_diff = vec3f_yaw(self.actor.world_pos, Self::player_pos(play)).wrapping_sub(self.actor.world_rot.y);
        self.player_yaw_in_range = yaw_diff < 0x38E3 && -0x38E3 < yaw_diff;
    }

    /// `BossDodongo_PlayerPosCheck`: whether *he* is within 200 of a corner (the name is the
    /// decomp's).
    fn player_pos_check(&mut self) {
        self.player_pos_in_range = S_CORNER_POSITIONS.iter().any(|c| (self.actor.world_pos.x - c.x).abs() < 200.0 && (self.actor.world_pos.z - c.z).abs() < 200.0);
    }

    /// `BossDodongo_SpawnFire`: an `En_Bdfire` (his child) at his `vec`, 20 lower, turned as he is:
    /// params -1 the flame at his mouth, 0 and up the fire balls.
    fn spawn_fire(&mut self, play: &mut PlayState, params: i16) {
        let pos = Vec3::new(self.vec.x, self.vec.y - 20.0, self.vec.z);
        let rot = [0, self.actor.shape_rot.y, 0];
        if let Err(e) = play.actor_spawn_as_child(&mut self.actor, ACTOR_EN_BDFIRE, pos, rot, params) {
            log::debug!("Boss_Dodongo: En_Bdfire {params} not spawned: {e:?}");
        }
    }

    /// `BossDodongo_UpdateDamage`: at no health, the death; else (not flashing) a seed or the
    /// boomerang on him while he inhales stops it, and a sword on his head while he's down takes
    /// its damage.
    fn update_damage(&mut self, play: &mut PlayState) {
        if self.health <= 0 && self.action != Action::DeathCutscene {
            self.setup_death_cutscene(play);
            enemy_start_finishing_blow(play, &self.actor);
            return;
        }
        if self.unk_1c0 == 0 {
            if self.action == Action::Inhale {
                for i in 0..self.collider.elements.len() {
                    let e = &mut self.collider.elements[i].info;
                    if e.ac_elem_flags & ACELEM_HIT != 0 {
                        let dmg_flags = e.ac_hit_elem.map(|h| h.at_dmg_info.dmg_flags).unwrap_or(0);
                        if dmg_flags & DMG_BOOMERANG != 0 || dmg_flags & DMG_SLINGSHOT != 0 {
                            e.ac_elem_flags &= !ACELEM_HIT;
                            self.unk_1c0 = 2;
                            self.setup_walk();
                            self.unk_1da = 0x32;
                            return;
                        }
                    }
                }
            }
            let head = &mut self.collider.elements[0].info;
            if head.ac_elem_flags & ACELEM_HIT != 0 {
                head.ac_elem_flags &= !ACELEM_HIT;
                let dmg_flags = head.ac_hit_elem.map(|h| h.at_dmg_info.dmg_flags).unwrap_or(0);
                if self.action == Action::Vulnerable || self.action == Action::LayDown {
                    let damage = collision_check_get_sword_damage(dmg_flags);
                    if damage != 0 {
                        audio_play_actor_sfx2(play, NA_SE_EN_DODO_K_DAMAGE);
                        self.setup_damaged();
                        self.unk_1c0 = 5;
                        self.health -= damage as i16;
                    }
                }
            }
        }
    }

    /// `BossDodongo_SetupDeathCutscene`.
    fn setup_death_cutscene(&mut self, play: &mut PlayState) {
        self.unk_1e4 = 0.0;
        self.actor.speed_xz = 0.0;
        self.change(self.anims.a_002d0c.clone(), 1.0, 0.0, last(&self.anims.a_002d0c), ANIMMODE_ONCE, -5.0);
        self.action = Action::DeathCutscene;
        audio_play_actor_sfx2(play, NA_SE_EN_DODO_K_DEAD);
        self.unk_1da = 0;
        self.cs_state = 0;
        self.actor.flags &= !(ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE);
        self.unk_1bc = 1;
        play.audio.queue_seq_cmd(seq_cmd1(SEQ_PLAYER_BGM_MAIN, 1));
    }

    /// `BossDodongo_DeathCutscene`.
    fn death_cutscene(&mut self, play: &mut PlayState) {
        self.skel.update();
        let mut state = self.cs_state;
        match state {
            0 => {
                self.cs_state = 5;
                play.start_manual();
                Self::player_cs(play, PLAYER_CSACTION_1);
                self.sub_cam_id = play.create_sub_camera();
                play.change_camera_status(CAM_ID_MAIN, CAM_STAT_UNK3);
                play.change_camera_status(self.sub_cam_id, CAM_STAT_ACTIVE);
                self.sub_cam_eye = play.game_camera.eye;
                self.sub_cam_at = play.game_camera.at;
            }
            5 => {
                let ts = sin_s(self.actor.shape_rot.y.wrapping_sub(0x1388)) * 150.0;
                let tc = cos_s(self.actor.shape_rot.y.wrapping_sub(0x1388)) * 150.0;
                let w = self.actor.world_pos;
                if let Some(p) = Self::player_mut(play) {
                    smooth_step_to_f(&mut p.world_pos.x, w.x + ts, 0.5, 5.0, 0.0);
                    smooth_step_to_f(&mut p.world_pos.z, w.z + tc, 0.5, 5.0, 0.0);
                }
                smooth_step_to_f(&mut self.unk_208, 0.07, 1.0, 0.005, 0.0);
                let ts = sin_s(self.actor.world_rot.y) * 230.0;
                let tc = cos_s(self.actor.world_rot.y) * 230.0;
                smooth_step_to_f(&mut self.sub_cam_eye.x, w.x + ts, 0.2, 50.0, 0.1);
                smooth_step_to_f(&mut self.sub_cam_eye.y, w.y + 20.0, 0.2, 50.0, 0.1);
                smooth_step_to_f(&mut self.sub_cam_eye.z, w.z + tc, 0.2, 50.0, 0.1);
                smooth_step_to_f(&mut self.sub_cam_at.x, w.x, 0.2, 30.0, 0.1);
                smooth_step_to_f(&mut self.sub_cam_at.y, self.actor.focus_pos.y - 70.0, 0.2, 30.0, 0.1);
                smooth_step_to_f(&mut self.sub_cam_at.z, w.z, 0.2, 30.0, 0.1);
                if self.skel.on_frame(last(&self.anims.a_002d0c)) {
                    self.change(self.anims.a_003cf8.clone(), 1.0, 0.0, last(&self.anims.a_003cf8), ANIMMODE_ONCE, -1.0);
                    self.cs_state = 6;
                    if let Err(e) = play.actor_spawn(ACTOR_BG_BREAKWALL, ARENA_CENTER, [0; 3], 0x6000) {
                        log::warn!("Boss_Dodongo: the lava's cover not spawned: {e:?}");
                    }
                }
            }
            6 => {
                let w = self.actor.world_pos;
                smooth_step_to_f(&mut self.sub_cam_at.x, w.x, 0.2, 30.0, 0.1);
                smooth_step_to_f(&mut self.sub_cam_at.y, (w.y - 70.0) + 130.0, 0.2, 20.0, 0.1);
                smooth_step_to_f(&mut self.sub_cam_at.z, w.z, 0.2, 30.0, 0.1);
                if self.skel.on_frame(last(&self.anims.a_003cf8)) {
                    self.change(self.anims.a_00df38.clone(), 1.0, 30.0, 59.0, ANIMMODE_ONCE, -1.0);
                    self.cs_state = 7;
                    self.unk_228 = 7700.0;
                    self.unk_204 = 0.0;
                    self.unk_1e4 = 0.0;
                    self.num_wall_collisions = 0;
                    self.unk_19e = 0;
                }
            }
            7 => self.death_roll(play),
            _ => {}
        }
        if state == 8 || state == 9 {
            self.death_sink(play);
            // FALLTHROUGH into case 100.
            state = 100;
        }
        if state == 100 && self.unk_1da < 0x2C6 && play.rand.zero_one() < 0.5 {
            let f = self.actor.focus_pos;
            let pos = Vec3::new(play.rand.centered_float(60.0) + f.x, play.rand.zero_one() * 50.0 + -1498.76, play.rand.centered_float(60.0) + f.z);
            let scale = (play.rand.zero_one() * 50.0) as i16 + 50;
            play.with_ss(|s| s.g_magma2_spawn(pos, [0, 0, 0, 100], [0, 0, 0, 0], 5, 1, scale));
        }
        if self.sub_cam_id != SUB_CAM_ID_DONE {
            play.camera_set_at_eye(self.sub_cam_id, self.sub_cam_at, self.sub_cam_eye);
        }
    }

    /// `BossDodongo_DeathCutscene`'s case 7: the last roll round the corners (bouncing at each),
    /// the camera to the arena's centre; at `unk_1DA` 1 the sink begins.
    fn death_roll(&mut self, play: &mut PlayState) {
        self.unk_1c4 = self.unk_1c4.wrapping_add(0x7D0);
        let w = self.actor.world_pos;
        smooth_step_to_f(&mut self.sub_cam_at.x, w.x, 0.2, 30.0, 0.0);
        smooth_step_to_f(&mut self.sub_cam_at.y, (w.y - 70.0) + 130.0, 0.2, 20.0, 0.0);
        smooth_step_to_f(&mut self.sub_cam_at.z, w.z, 0.2, 30.0, 0.0);
        smooth_step_to_f(&mut self.sub_cam_eye.x, -890.0, 0.1, self.unk_204 * 5.0, 0.1);
        smooth_step_to_f(&mut self.sub_cam_eye.z, -3304.0, 0.1, self.unk_204 * 5.0, 0.1);
        smooth_step_to_f(&mut self.unk_204, 1.0, 1.0, 0.1, 0.0);
        if self.unk_1da == 1 {
            self.cs_state = 8;
            self.actor.speed_xz = self.unk_1e4 / 1.5;
            self.unk_238 = if self.unk_1a2 == 0 { 250.0 } else { -250.0 };
            self.unk_1da = 1000;
            self.unk_234 = 2000.0;
            return;
        }
        let corner = S_CORNER_POSITIONS[self.unk_1a0 as usize & 3];
        self.unk_1ec = 3.0;
        smooth_step_to_f(&mut self.unk_1e4, self.unk_1ec * 5.0, 1.0, self.unk_1ec * 0.25, 0.0);
        let ts = corner.x - w.x;
        let tc = corner.z - w.z;
        let mut sp178 = (ts * ts + tc * tc).sqrt() - 200.0;
        if (ts * ts + tc * tc).sqrt() < 200.0 || self.unk_1da != 0 {
            sp178 = 0.0;
        }
        sp178 = sp178.min(70.0);
        self.unk_23c = (sin_s(self.unk_19e.wrapping_mul(1000)) * -50.0) / 100.0;
        let sp198 = Vec3::new(sin_s(self.unk_19e.wrapping_mul(1000)) * sp178, 0.0, 0.0);
        // Matrix_RotateY(BINANG_TO_RAD(shape.rot.y), MTXMODE_NEW), Matrix_MultVec3f.
        let mut m = MtxF::IDENTITY;
        m.rotate_y(oot_game::sys_matrix::binang_to_rad(self.actor.shape_rot.y));
        let sp184 = m.mult_vec3f(sp198);
        smooth_step_to_f(&mut self.actor.world_pos.x, corner.x + sp184.x, 1.0, self.unk_1e4, 0.0);
        smooth_step_to_f(&mut self.actor.world_pos.z, corner.z + sp184.z, 1.0, self.unk_1e4, 0.0);
        audio_play_actor_sfx2(play, NA_SE_EN_DODO_K_ROLL - SFX_FLAG);
        if self.unk_19e & 7 == 0 {
            play.camera_request_quake(CAM_ID_MAIN, 1, 8);
        }
        if self.unk_19e & 1 == 0 {
            let (a, p) = (self.actor.clone(), self.actor.world_pos);
            actor_spawn_floor_dust_ring(play, &a, p, 40.0, 3, 8.0, 500, 10, false);
        }
        let ts = corner.x - self.actor.world_pos.x;
        let tc = corner.z - self.actor.world_pos.z;
        smooth_step_to_f(&mut self.unk_1e8, 1500.0, 1.0, self.unk_1ec * 100.0, 0.0);
        smooth_step_to_s(&mut self.actor.world_rot.y, rad_to_binang(f_atan2f(ts, tc)), 5, (self.unk_1ec * self.unk_1e8) as i32 as i16, 0);
        if ts.abs() <= 15.0 && tc.abs() <= 15.0 {
            self.actor.velocity.y = 15.0;
            audio_play_actor_sfx2(play, NA_SE_EN_DODO_K_COLI2);
            self.next_corner();
            self.unk_1da = 0xA;
            let w = self.actor.world_pos;
            func_80033480(play, Vec3::new(w.x, w.y + 60.0, w.z), 250.0, 0x28, 0x320, 0xA, 0);
        }
    }

    /// `BossDodongo_DeathCutscene`'s cases 8 and 9: rolling to a stop in the lava, sinking (dust and
    /// magma), the decay, the floor cooling (`unk_1C6`), Link put at a corner's middle, the music and
    /// the heart container (820), the camera back with the warp and the room's clear flag (600).
    fn death_sink(&mut self, play: &mut PlayState) {
        if self.unk_1da == 884 {
            self.change(self.anims.a_0042a8.clone(), 1.0, 0.0, last(&self.anims.a_0042a8) as i16 as f32, ANIMMODE_LOOP, -20.0);
            let ts = self.sub_cam_eye.x - self.actor.world_pos.x;
            let tc = self.sub_cam_eye.z - self.actor.world_pos.z;
            self.unk_22c = (ts * ts + tc * tc).sqrt();
            self.unk_230 = f_atan2f(ts, tc);
            self.unk_1dc = 350;
            self.cs_state = 9;
        }
        if self.unk_1da < 854 {
            for _ in 0..2 {
                func_808c12c4(play, self.actor.obj_bank_index, self.unk_1cc);
                if self.unk_1cc < 256 {
                    self.unk_1cc += 1;
                }
            }
        }
        if self.unk_1da < 984 {
            smooth_step_to_s(&mut self.unk_1c4, -0x4000, 0xA, 0x12C, 0);
        }
        if self.unk_1da == 904 {
            audio_play_actor_sfx2(play, NA_SE_EN_DODO_K_END);
        }
        if self.unk_1da < 854 {
            audio_play_actor_sfx2(play, NA_SE_EN_DODO_K_LAST - SFX_FLAG);
        }
        if self.unk_1da == 960 {
            audio_play_actor_sfx2(play, NA_SE_EN_DODO_K_LAVA);
        }
        if self.unk_1da < 960 {
            smooth_step_to_f(&mut self.actor.shape_shadow_scale, 0.0, 1.0, 10.0, 0.0);
            if self.unk_1da >= 710 {
                if self.unk_1da == 710 {
                    const SP124: [Vec3; 4] = [Vec3::new(-440.0, 0.0, -3304.0), Vec3::new(-890.0, 0.0, -3754.0), Vec3::new(-1340.0, 0.0, -3304.0), Vec3::new(-890.0, 0.0, -2854.0)];
                    const SPF4: [Vec3; 4] = [Vec3::new(-890.0, 0.0, -2854.0), Vec3::new(-440.0, 0.0, -3304.0), Vec3::new(-890.0, 0.0, -3754.0), Vec3::new(-1340.0, 0.0, -3304.0)];
                    self.unk_1c6 = 1;
                    let p = if self.unk_1a2 == 0 { SP124[self.unk_1a0 as usize & 3] } else { SPF4[self.unk_1a0 as usize & 3] };
                    if let Some(pl) = Self::player_mut(play) {
                        pl.world_pos.x = p.x;
                        pl.world_pos.z = p.z;
                    }
                    self.unk_204 = 0.0;
                }
                if self.unk_1da >= 885 {
                    smooth_step_to_f(&mut self.unk_228, 200.0, 0.2, 100.0, 0.0);
                } else {
                    smooth_step_to_f(&mut self.unk_228, -6600.0, 0.2, 30.0, 0.0);
                }
                const DUST_VEL: Vec3 = Vec3::ZERO;
                const DUST_ACCEL: Vec3 = Vec3::new(0.0, 1.0, 0.0);
                let f = self.actor.focus_pos;
                let w = self.actor.world_pos;
                let pos = Vec3::new(play.rand.centered_float(120.0) + f.x, play.rand.zero_float(50.0) + w.y, play.rand.centered_float(120.0) + f.z);
                play.with_ss(|s| s.func_8002836c(pos, DUST_VEL, DUST_ACCEL, [255, 255, 100, 255], [255, 100, 0, 255], 0x1F4, 0xA, 0xA));
                let pos = Vec3::new(play.rand.centered_float(120.0) + f.x, -1498.76, play.rand.centered_float(120.0) + f.z);
                let color_index = (play.rand.zero_one() * 1.9) as i16;
                let prim = [[255, 255, 0, 255], [0, 0, 0, 100]][color_index as usize & 1];
                let env = [[255, 0, 0, 255], [0, 0, 0, 0]][color_index as usize & 1];
                let scale = (play.rand.zero_one() * 100.0) as i16 + 100;
                play.with_ss(|s| s.g_magma2_spawn(pos, prim, env, 10 - color_index * 5, color_index, scale));
            }
        } else {
            audio_play_actor_sfx2(play, NA_SE_EN_DODO_K_ROLL - SFX_FLAG);
            if self.unk_19e & 1 == 0 {
                let (a, p) = (self.actor.clone(), self.actor.world_pos);
                actor_spawn_floor_dust_ring(play, &a, p, 40.0, 3, 8.0, 500, 10, false);
            }
        }
        smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 0.2, 0.1, 0.0);
        self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(self.unk_238 as i16);
        self.unk_1c4 = self.unk_1c4.wrapping_add(self.unk_234 as i16);
        if self.unk_1da >= 0x367 {
            if self.unk_1a2 == 0 {
                if self.unk_238 < 450.0 {
                    self.unk_238 += 10.0;
                }
            } else if -450.0 < self.unk_238 {
                self.unk_238 -= 10.0;
            }
        } else {
            smooth_step_to_f(&mut self.unk_238, 0.0, 0.05, 40.0, 0.0);
        }
        smooth_step_to_f(&mut self.unk_234, 0.0, 0.2, 17.0, 0.0);
        let w = self.actor.world_pos;
        smooth_step_to_f(&mut self.sub_cam_at.x, w.x, 0.2, 30.0, 0.0);
        smooth_step_to_f(&mut self.sub_cam_at.y, (w.y - 70.0) + 130.0, 0.2, 20.0, 0.0);
        smooth_step_to_f(&mut self.sub_cam_at.z, w.z, 0.2, 30.0, 0.0);
        if self.cs_state == 9 {
            if self.unk_1da < 0x2C6 {
                const SPAC: [Vec3; 4] = [Vec3::new(-390.0, 0.0, -3304.0), Vec3::new(-890.0, 0.0, -3804.0), Vec3::new(-1390.0, 0.0, -3304.0), Vec3::new(-890.0, 0.0, -2804.0)];
                const SP7C: [Vec3; 4] = [Vec3::new(-890.0, 0.0, -2804.0), Vec3::new(-390.0, 0.0, -3304.0), Vec3::new(-890.0, 0.0, -3804.0), Vec3::new(-1390.0, 0.0, -3304.0)];
                let sp78 = if self.unk_1a2 == 0 { SPAC[self.unk_1a0 as usize & 3] } else { SP7C[self.unk_1a0 as usize & 3] };
                let s = self.unk_204 * 20.0;
                let py = Self::player_pos(play).y;
                smooth_step_to_f(&mut self.sub_cam_eye.x, sp78.x, 0.2, s, 0.0);
                smooth_step_to_f(&mut self.sub_cam_eye.y, py + 30.0, 0.1, s, 0.0);
                smooth_step_to_f(&mut self.sub_cam_eye.z, sp78.z, 0.1, s, 0.0);
                smooth_step_to_f(&mut self.unk_204, 1.0, 1.0, 0.02, 0.0);
            } else {
                if self.unk_1a2 == 0 {
                    self.unk_230 += 0.01;
                } else {
                    self.unk_230 -= 0.01;
                }
                smooth_step_to_f(&mut self.unk_22c, 220.0, 0.1, 5.0, 0.1);
                let ts = self.unk_230.sin() * self.unk_22c;
                let tc = self.unk_230.cos() * self.unk_22c;
                smooth_step_to_f(&mut self.sub_cam_eye.x, w.x + ts, 0.2, 50.0, 0.0);
                smooth_step_to_f(&mut self.sub_cam_eye.y, w.y + 20.0, 0.2, 50.0, 0.0);
                smooth_step_to_f(&mut self.sub_cam_eye.z, w.z + tc, 0.2, 50.0, 0.0);
                smooth_step_to_f(&mut self.unk_23c, 0.0, 0.2, 0.01, 0.0);
            }
        } else {
            if self.unk_1a2 == 0 {
                smooth_step_to_f(&mut self.unk_23c, -0.5, 0.2, 0.05, 0.0);
            } else {
                smooth_step_to_f(&mut self.unk_23c, 0.5, 0.2, 0.05, 0.0);
            }
            smooth_step_to_f(&mut self.sub_cam_eye.x, -890.0, 0.1, self.unk_204 * 5.0, 0.1);
            smooth_step_to_f(&mut self.sub_cam_eye.z, -3304.0, 0.1, self.unk_204 * 5.0, 0.1);
            smooth_step_to_f(&mut self.unk_204, 1.0, 1.0, 0.05, 0.0);
        }
        if self.unk_1da == 820 {
            play.audio.queue_seq_cmd(start_seq(SEQ_PLAYER_BGM_MAIN, 0, NA_BGM_BOSS_CLEAR));
            let w = self.actor.world_pos;
            let r = self.actor.shape_rot.y;
            let pos = Vec3::new(sin_s(r) * -50.0 + w.x, w.y, cos_s(r) * -50.0 + w.z);
            if let Err(e) = play.actor_spawn(ACTOR_ITEM_B_HEART, pos, [0; 3], 0) {
                log::warn!("Boss_Dodongo: the heart container not spawned: {e:?}");
            }
        }
        if self.unk_1da == 600 {
            play.game_camera.eye = self.sub_cam_eye;
            play.game_camera.eye_next = self.sub_cam_eye;
            play.game_camera.at = self.sub_cam_at;
            play.play_return_to_main_cam(self.sub_cam_id, 0);
            self.unk_1bc = 0;
            self.sub_cam_id = SUB_CAM_ID_DONE;
            self.cs_state = 100;
            play.change_camera_status(CAM_ID_MAIN, CAM_STAT_ACTIVE);
            play.stop_manual();
            Self::player_cs(play, PLAYER_CSACTION_7);
            if let Err(e) = play.actor_spawn_as_child(&mut self.actor, ACTOR_DOOR_WARP1, ARENA_CENTER, [0; 3], WARP_DUNGEON_CHILD) {
                log::warn!("Boss_Dodongo: the blue warp not spawned: {e:?}");
            }
            self.skel.play_speed = 0.0;
            let room = play.room_ctx.cur.num;
            play.flags.set_clear(room);
        }
    }

    /// `BossDodongo_UpdateEffects`: each particle moves, ages and (in use) cycles its colour and
    /// fades 20 a frame.
    fn update_effects(&mut self) {
        const EFFECT_COLORS: [[u8; 3]; 4] = [[255, 128, 0], [255, 0, 0], [255, 255, 0], [255, 0, 0]];
        for eff in self.effects.iter_mut() {
            if eff.unk_24 != 0 {
                eff.unk_00 += eff.unk_0c;
                eff.unk_25 = eff.unk_25.wrapping_add(1);
                eff.unk_0c += eff.unk_18;
                if eff.unk_24 == 1 {
                    eff.color = EFFECT_COLORS[(eff.unk_25 % 4) as usize];
                    eff.alpha -= 20;
                    if eff.alpha <= 0 {
                        eff.alpha = 0;
                        eff.unk_24 = 0;
                    }
                }
            }
        }
    }

    /// `Actor_Draw`'s model matrix, then `BossDodongo_Draw`'s turns: `Matrix_RotateZ(unk_23C)`,
    /// `Matrix_RotateX(unk_1C4 / 32768 × 3.14159)`.
    fn model_mtx(&self) -> MtxF {
        let a = &self.actor;
        let mut m = MtxF::set_translate_rotate_yxz(a.world_pos.x, a.world_pos.y + a.shape_y_offset * a.scale.y, a.world_pos.z, [a.shape_rot.x, a.shape_rot.y, a.shape_rot.z]);
        m.scale(a.scale.x, a.scale.y, a.scale.z);
        m.rotate_z(self.unk_23c);
        m.rotate_x((self.unk_1c4 as f32 / 32768.0) * 3.14159);
        m
    }

    /// `BossDodongo_PostLimbDraw` at limb `limb` (its matrix as `OverrideLimbDraw` left it, popped):
    /// his points, the collider's spheres.
    fn post_limb_draw(&mut self, limb: usize, mtx: &MtxF) {
        match limb {
            6 => {
                self.vec = mtx.mult_vec3f(D_808CA45C);
                self.actor.focus_pos = mtx.mult_vec3f(D_808CA450);
                self.fire_pos = mtx.mult_vec3f(D_808CA468);
                self.mouth_pos = mtx.mult_vec3f(D_808CA474);
            }
            39 => self.unk_410 = mtx.mult_vec3f(D_808CA480),
            46 => self.unk_404 = mtx.mult_vec3f(D_808CA48C),
            _ => {}
        }
        self.collider.update_spheres(limb as u8, &mtx.to_mat4());
    }
}

/// `BossDodongo_OverrideLimbDraw`'s matrix for limb `limb`'s own list (inside its push and pop):
/// limbs 33 and 48 scaled `(1, s, s)` by `unk_1F8`; every limb but 6 and 7 turned by its wobble
/// `w`, squashed by `k` (`unk_208`) and turned back.
pub fn limb_local(limb: usize, w: f32, s: f32, k: f32) -> Mat4 {
    let mut m = if limb == 33 || limb == 48 { Mat4::from_scale(Vec3::new(1.0, s, s)) } else { Mat4::IDENTITY };
    if limb != 6 && limb != 7 {
        m = m * Mat4::from_rotation_x(w * 0.115)
            * Mat4::from_rotation_y(w * 0.13)
            * Mat4::from_rotation_z(w * 0.1)
            * Mat4::from_scale(Vec3::new(1.0 - k, k + 1.0, 1.0 - k))
            * Mat4::from_rotation_z(-(w * 0.1))
            * Mat4::from_rotation_y(-(w * 0.13))
            * Mat4::from_rotation_x(-(w * 0.115));
    }
    m
}

pub const BAKE_SKEL: &str = "Boss_Dodongo/skel";
pub const BAKE_PARTICLE: &str = "Boss_Dodongo/particle";
/// The particles' prim colour (`gDPSetPrimColor` between the material and the quad).
const SEG_PRIM: u8 = 0x0E;

/// `BossDodongo_Draw`'s meshes: his skeleton (after `Gfx_SetupDL_25Opa`), and a particle
/// (`Gfx_SetupDL_25Xlu`, `DL_009D50`'s material, its colour, `DL_009DD0`'s quad).
pub fn bakes() -> Vec<MeshBake> {
    vec![
        MeshBake { name: BAKE_SKEL.into(), object: OBJECT.into(), segments: Vec::new(), prelude: Vec::new(), body: BakeBody::Skeleton { file: OBJECT.into(), symbol: SKELETON.into(), limbs: Vec::new() } },
        MeshBake {
            name: BAKE_PARTICLE.into(),
            object: OBJECT.into(),
            segments: vec![(SEG_PRIM, BakeSegment::DynamicColor { env: false, prim: true })],
            prelude: Vec::new(),
            body: BakeBody::DLists(vec![
                (OBJECT.into(), "object_kingdodongo_DL_009D50".into()),
                (SEGMENT_CALL.into(), format!("{SEG_PRIM:x}")),
                (OBJECT.into(), "object_kingdodongo_DL_009DD0".into()),
            ]),
        },
    ]
}

/// `render_state`'s layout. values: `unk_25C[1..=48]` (each limb's wobble), `unk_208`, `unk_1F8`,
/// `unk_23C`, the fog's colour (r, g, b), near and far; angles: `unk_1C4`; switches: the hit flash
/// (`unk_1C0 >= 2 && unk_1C0 & 1`), then each particle in use as five words (the colour and alpha
/// `0xRRGGBBAA`, and its x, y, z and scale as float bits: not blended).
mod rs {
    pub const V_WOBBLE: usize = 0;
    pub const V_SQUASH: usize = 48;
    pub const V_TAIL: usize = 49;
    pub const V_TILT: usize = 50;
    pub const V_FOG: usize = 51;
    pub const V_LEN: usize = 56;
}

impl ActorImpl for BossDodongo {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BossDodongo_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.unk_1e2 = 0;
        self.unk_19e = self.unk_19e.wrapping_add(1);
        for t in [&mut self.unk_1da, &mut self.unk_1dc, &mut self.unk_1de, &mut self.unk_1c0, &mut self.unk_1c8] {
            if *t != 0 {
                *t -= 1;
            }
        }
        let f = self.func_808c4f6c(play);
        self.unk_1a4 = if f > 0.0 { f as i32 as i16 } else { 0 };
        let f = self.func_808c50a8(play);
        self.unk_1a6 = if f > 0.0 { f as i32 as i16 } else { 0 };
        self.player_yaw_check(play);
        self.player_pos_check();
        match self.action {
            Action::IntroCutscene => self.intro_cutscene(play),
            Action::Walk => self.walk(play),
            Action::Inhale => self.inhale(play),
            Action::BlowFire => self.blow_fire(play),
            Action::Roll => self.roll(play),
            Action::Explode => self.explode(play),
            Action::LayDown => self.lay_down(),
            Action::Vulnerable => self.vulnerable(play),
            Action::GetUp => self.get_up(),
            Action::Damaged => self.damaged(),
            Action::DeathCutscene => self.death_cutscene(play),
        }
        self.actor.shape_rot.y = self.actor.world_rot.y;
        smooth_step_to_f(&mut self.actor.shape_y_offset, self.unk_228, 1.0, 100.0, 0.0);
        self.actor.move_forward();
        self.update_damage(play);
        self.actor.update_bg_check_info(&play.col, 10.0, 10.0, 20.0, UPDBGCHECKINFO_FLAG_2);
        smooth_step_to_f(&mut self.unk_208, 0.0, 1.0, 0.001, 0.0);
        smooth_step_to_f(&mut self.unk_20c, 0.0, 1.0, 0.001, 0.0);
        if self.unk_19e % 128 == 0 {
            for v in self.unk_324.iter_mut() {
                *v = (play.rand.zero_one() * 0.25) + 0.5;
            }
        }
        for i in 0..50 {
            self.unk_25c[i] += self.unk_324[i];
        }
        if self.unk_1c8 != 0 {
            if self.unk_1c8 >= 11 {
                smooth_step_to_f(&mut self.unk_240, if self.unk_1c8 & 1 != 0 { 40.0 } else { 60.0 }, 1.0, 50.0, 0.0);
            } else {
                smooth_step_to_f(&mut self.unk_240, 0.0, 1.0, 10.0, 0.0);
            }
            let env = &mut play.env_ctx;
            if env.adj_light1_color[2] == 0 && env.adj_ambient_color[2] == 0 {
                env.adj_light1_color[0] = self.unk_240 as i32 as u8 as i16;
                env.adj_light1_color[1] = (self.unk_240 * 0.1) as i32 as u8 as i16;
                env.adj_ambient_color[0] = self.unk_240 as i32 as u8 as i16;
                env.adj_ambient_color[1] = (self.unk_240 * 0.1) as i32 as u8 as i16;
            }
        }
        if self.unk_1be != 0 {
            if self.unk_1be >= 1000 {
                smooth_step_to_f(&mut self.color_filter_r, 30.0, 1.0, 20.0, 0.0);
                smooth_step_to_f(&mut self.color_filter_g, 10.0, 1.0, 20.0, 0.0);
            } else {
                self.unk_1be -= 1;
                smooth_step_to_f(&mut self.color_filter_r, 255.0, 1.0, 20.0, 0.0);
                smooth_step_to_f(&mut self.color_filter_g, 0.0, 1.0, 20.0, 0.0);
            }
            smooth_step_to_f(&mut self.color_filter_b, 0.0, 1.0, 20.0, 0.0);
            smooth_step_to_f(&mut self.color_filter_min, 900.0, 1.0, 10.0, 0.0);
            smooth_step_to_f(&mut self.color_filter_max, 1099.0, 1.0, 10.0, 0.0);
        } else {
            let l = play.scene.as_ref().map(|s| s.lights);
            let (fog, near) = l.map(|l| (l.fog_color, l.fog_near)).unwrap_or(([0; 3], 0));
            smooth_step_to_f(&mut self.color_filter_r, fog[0] as f32, 1.0, 5.0, 0.0);
            smooth_step_to_f(&mut self.color_filter_g, fog[1] as f32, 1.0, 5.0, 0.0);
            smooth_step_to_f(&mut self.color_filter_b, fog[2] as f32, 1.0, 5.0, 0.0);
            smooth_step_to_f(&mut self.color_filter_min, near as f32, 1.0, 5.0, 0.0);
            smooth_step_to_f(&mut self.color_filter_max, 1000.0, 1.0, 5.0, 0.0);
        }
        let player_y = Self::player_pos(play).y;
        if player_y < -1000.0 {
            self.lava(play);
        }
        if self.unk_1c6 != 0 {
            for _ in 0..20 {
                let i = (self.unk_1c2 & 0x7FF) as usize;
                if let (Some(d), Some(&s)) = (self.lava_tex.get_mut(i), self.rock_tex.get(i)) {
                    *d = s;
                }
                self.unk_1c2 = self.unk_1c2.wrapping_add(37);
            }
            smooth_step_to_f(&mut self.unk_224, 0.0, 1.0, 0.01, 0.0);
        }
        if self.unk_1bc == 0 {
            if self.action != Action::DeathCutscene {
                play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
            }
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
            if self.action == Action::Roll {
                play.collision_check_set_at(&self.actor, 0, &mut self.collider);
            }
        }
        self.collider.elements[0].dim.scale = if self.action == Action::Inhale { 0.0 } else { 1.0 };
        for i in 6..19 {
            if i == 12 {
                continue;
            }
            self.collider.elements[i].dim.scale = if self.action == Action::Roll { 0.0 } else { 1.0 };
        }
        if self.unk_244 != 0.0 {
            play.mreg_fill = [1, 255, 80, 0, self.unk_244 as i32 as u8 as i16];
        } else {
            play.mreg_fill[0] = 0;
        }
        smooth_step_to_f(&mut self.unk_244, 0.0, 1.0, 2.0, 0.0);
        self.update_effects();
    }

    /// `BossDodongo_Draw`'s effects on the actor and the game, once per game frame, in
    /// `SkelAnime_DrawOpa`'s limb order: `BossDodongo_PostLimbDraw`'s points and spheres.
    fn draw_update(&mut self, _play: &mut PlayState) {
        if self.actor.killed {
            return;
        }
        let Some(skeleton) = self.skeleton.clone() else { return };
        let model = self.model_mtx();
        let joints = self.skel.joint_table.clone();
        let mut limb_mtx = vec![MtxF::IDENTITY; skeleton.limbs.len()];
        for &l in &skeleton.draw_order {
            let l = l as usize;
            let parent = skeleton.parents[l].map(|p| limb_mtx[p as usize]).unwrap_or(model);
            let pos = if l == 0 {
                let r = joints.first().copied().unwrap_or([0; 3]);
                Vec3::new(r[0] as f32, r[1] as f32, r[2] as f32)
            } else {
                let p = skeleton.limbs[l].joint_pos;
                Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
            };
            let rot = joints.get(l + 1).copied().unwrap_or([0; 3]);
            let mut m = parent;
            m.translate_rotate_zyx(pos, rot);
            limb_mtx[l] = m;
            self.post_limb_draw(l + 1, &m);
        }
    }

    fn render_state(&self) -> RenderState {
        let mut r = RenderState::of(&self.actor);
        r.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        let mut v = vec![0.0; rs::V_LEN];
        v[rs::V_WOBBLE..rs::V_WOBBLE + 48].copy_from_slice(&self.unk_25c[1..=48]);
        v[rs::V_SQUASH] = self.unk_208;
        v[rs::V_TAIL] = self.unk_1f8;
        v[rs::V_TILT] = self.unk_23c;
        v[rs::V_FOG..rs::V_FOG + 5].copy_from_slice(&[self.color_filter_r, self.color_filter_g, self.color_filter_b, self.color_filter_min, self.color_filter_max]);
        r.values = v;
        r.angles = vec![self.unk_1c4];
        let mut sw = vec![(self.unk_1c0 >= 2 && self.unk_1c0 & 1 != 0) as u32];
        for e in self.effects.iter().filter(|e| e.unk_24 == 1) {
            sw.push(u32::from_be_bytes([e.color[0], e.color[1], e.color[2], e.alpha as u8]));
            sw.extend([e.unk_00.x.to_bits(), e.unk_00.y.to_bits(), e.unk_00.z.to_bits(), e.unk_2c.to_bits()]);
        }
        r.switches = sw;
        r
    }

    /// `BossDodongo_Draw`: his skeleton in his own fog (white while the hit flashes), turned by his
    /// tilt and roll, each limb's own list wobbled; his textures from his object's RAM once he's
    /// started to decay; then his particles (`BossDodongo_DrawEffects`), each facing the camera,
    /// `unk_2C` across.
    fn draw(&self, rs: &RenderState, play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        let (Some(joints), [roll], v) = (&rs.joints, rs.angles.as_slice(), rs.values.as_slice()) else { return };
        let Some(skeleton) = &self.skeleton else { return };
        if v.len() < rs::V_LEN || rs.switches.is_empty() {
            return;
        }
        let model = oot_game::play::actor_draw_matrix(rs) * Mat4::from_rotation_z(v[rs::V_TILT]) * Mat4::from_rotation_x((*roll as f32 / 32768.0) * 3.14159);
        let mut bones = skeleton.pose_override(&joints.rot, |_, _, _| Mat4::IDENTITY);
        for (l, b) in bones.iter_mut().enumerate() {
            let limb = l + 1;
            let w = if limb <= 48 { v[rs::V_WOBBLE + limb - 1] } else { 0.0 };
            *b *= limb_local(limb, w, v[rs::V_TAIL], v[rs::V_SQUASH]);
        }
        let fog = if rs.switches[0] != 0 {
            oot_game::gbi::gfx_set_fog(255, 255, 255, 0, 900, 1099)
        } else {
            let f = &v[rs::V_FOG..rs::V_FOG + 5];
            oot_game::gbi::gfx_set_fog(f[0] as u32 as u8, f[1] as u32 as u8, f[2] as u32 as u8, 0, f[3] as i32, f[4] as i32)
        };
        let texture_images = decay_images(play, self.actor.obj_bank_index);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE_SKEL)), transform: model, bones, params: DrawParams { fog: Some(fog), texture_images, ..Default::default() } });
        // BossDodongo_DrawEffects.
        for p in rs.switches[1..].chunks_exact(5) {
            let [r, g, b, a] = p[0].to_be_bytes();
            let pos = Vec3::new(f32::from_bits(p[1]), f32::from_bits(p[2]), f32::from_bits(p[3]));
            let scale = f32::from_bits(p[4]);
            let m = oot_game::draw::replace_rotation(Mat4::from_translation(pos), view.billboard) * Mat4::from_scale(Vec3::new(scale, scale, 1.0));
            let mut sv = SegmentValues::default();
            sv.prim[SEG_PRIM as usize] = Some([r, g, b, a]);
            out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE_PARTICLE)), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
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

impl BossDodongo {
    /// `BossDodongo_Update`'s lava (Link below y -1000): `Effect_Ss_G_Magma2` splashes and
    /// `G_Magma` bubbles with particles round the arena's centre, their rates by `unk_224` (the
    /// splashes dark smoke once it's cooled below 1.1; none while Link is warping out), and the floor's
    /// texture wobbled from the overlay's lava (`func_808C1554`).
    fn lava(&mut self, play: &mut PlayState) {
        let mut magma_scale: i16 = 0;
        let (mut phi_s0_3, sp90, magma2_draw_mode): (i32, i32, i16) = if self.unk_224 > 1.9 {
            (1, 0, 0)
        } else if self.unk_224 > 1.7 {
            (3, 1, 0)
        } else if self.unk_224 > 1.4 {
            (7, 3, (play.rand.zero_one() * 1.9) as i16)
        } else if self.unk_224 > 1.1 {
            (7, 4095, (play.rand.zero_one() * 1.9) as i16)
        } else {
            magma_scale = (play.rand.zero_one() * 50.0) as i16 - 50;
            (1, -1, 1)
        };
        let cs_action = play.player.and_then(|h| play.actors.downcast::<crate::player::Player>(h)).map(|p| p.cs_mode).unwrap_or(0);
        if cs_action >= PLAYER_CSACTION_10 {
            phi_s0_3 = -1;
        }
        const MAGMA_PRIM: [[u8; 4]; 2] = [[255, 255, 0, 255], [0, 0, 0, 150]];
        const MAGMA_ENV: [[u8; 4]; 2] = [[255, 0, 0, 255], [0, 0, 0, 0]];
        let frame = self.unk_19e as i32;
        if frame & phi_s0_3 == 0 {
            let r = play.rand.zero_one() * 330.0;
            let a = play.rand.zero_one() * 6.28;
            let pos = Vec3::new(a.sin() * r + -890.0, -1523.76, a.cos() * r + -3304.0);
            let m = magma2_draw_mode as usize & 1;
            play.with_ss(|s| s.g_magma2_spawn(pos, MAGMA_PRIM[m], MAGMA_ENV[m], 10 - magma2_draw_mode * 5, magma2_draw_mode, magma_scale + 100));
        }
        if frame & sp90 == 0 {
            let sp6c = Vec3::ZERO;
            let mut sp60 = Vec3::ZERO;
            let r = play.rand.zero_one() * 330.0;
            let a = play.rand.zero_one() * 6.28;
            let pos = Vec3::new(a.sin() * r + -890.0, -1523.76, a.cos() * r + -3304.0);
            play.with_ss(|s| s.g_magma_spawn(pos));
            for _ in 0..4 {
                sp60.y = 0.4;
                sp60.x = play.rand.centered_float(0.5);
                sp60.z = play.rand.centered_float(0.5);
                let r = play.rand.zero_one() * 330.0;
                let a = play.rand.zero_one() * 6.28;
                let pos = Vec3::new(a.sin() * r + -890.0, -1513.76, a.cos() * r + -3304.0);
                let scale = play.rand.zero_float(2.0) as i16 as f32 + 6.0;
                self.func_808c17c8(play, pos, sp6c, sp60, scale, BOSS_DODONGO_EFFECT_COUNT);
            }
        }
        let (lava, frame, amount) = (self.lava_tex.clone(), self.unk_19e as i32, self.unk_224);
        Self::write_floor(play, |floor| func_808c1554(floor, &lava, frame, amount));
    }
}
