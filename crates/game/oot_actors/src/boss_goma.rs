//! `Boss_Goma` (`ovl_Boss_Goma/z_boss_goma.c`): Queen Gohma, the Deku Tree's boss, with her
//! intro and her death, which are her own actions and drive her own sub camera.
//!
//! - **The intro** (`BossGoma_Encounter`): hanging from the ceiling until Link reaches the
//!   room's entrance (150, 350); the camera from the ceiling's centre zooming in on him (held,
//!   `PLAYER_CSACTION_8`), the slab falling behind him (`Door_Shutter` `SHUTTER_GOHMA_BLOCK`, her
//!   child, at frame 176), Link turning (`PLAYER_CSACTION_2`); then she wanders the ceiling until
//!   he has looked at her for 16 frames (`projectedPos`), rolls her eye, runs, drops to the floor,
//!   cries, and the boss's title card (the first time only), the boss music and
//!   `EVENTCHKINF_BEGAN_GOHMA_BATTLE`. With that flag set (a second try) the door and the look-at
//!   part are skipped.
//! - **The fight:** on the floor towards Link while patient (`patienceTimer`, 200), away from
//!   him after; within 150 she rears up, her eye red (a seed or a Deku Nut then stuns her: 90 or
//!   40 frames), and lunges; a wall she touches she climbs; on the ceiling she lays three eggs
//!   (`En_Goma` 0 to 2, her children), her eye red for the 70 frames before (a hit then knocks her
//!   down, stunned for 150); once they're dead she jumps down. Only her eye takes hits, only while
//!   open; stunned, a sword's damage (`CollisionCheck_GetSwordDamage`) comes off her 10 health.
//! - **The death** (`BossGoma_Defeated`): her sub camera circling her, Link pulled in front of
//!   her (`PLAYER_CSACTION_1`); bubbles, dust and fragments at her limbs, the room flashing blue,
//!   her textures erased pixel by pixel (`BossGoma_ClearPixels`, into her object's RAM:
//!   `ObjectContext::written`), her limbs breaking off as `En_Goma` pieces (params 100 and up),
//!   the boss-clear music, the heart container (`Item_B_Heart`) at her, the blue warp
//!   (`Door_Warp1`, `WARP_DUNGEON_CHILD`) somewhere free, the room cleared; she shrinks away.
//!
//! Her draw is one skeleton bake with her env colour (`mainEnvColor`) for every limb, her eye and
//! iris baked apart with their own (the eye's random while she's invincible: a draw-time `Rand`,
//! made in `draw_update`), her eyelids and iris turned, her iris and tail scaled, broken-off limbs
//! hidden; segment 8 culling back faces or not (two bakes of each). Her textures come from her
//! object's RAM once she's erased any (`DrawParams::texture_images`), and so do her pieces'.
//!
//! Not ported: the circle shadow (`ActorShadow_DrawCircle`, for no actor), the rumble
//! (`Rumble_Override`, logged).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Sphere16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues, SourceImage};
use eng_math::{approach_f, approach_s, approach_zero_f, atan2_s, cos_s, sin_s, smooth_step_to_f, vec3f_yaw};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_BOSS, ActorImpl, ActorProfile, actor_spawn_floor_dust_ring, audio_play_actor_sfx2, enemy_start_finishing_blow};
use oot_game::audio::bgm::{seq_cmd1, start_seq};
use oot_game::audio::sfx::*;
use oot_game::audio::{NA_BGM_BOSS, NA_BGM_BOSS_CLEAR, SEQ_PLAYER_BGM_MAIN};
use oot_game::camera::{CAM_ID_MAIN, CAM_STAT_ACTIVE, CAM_STAT_UNK3, CAM_STAT_WAIT, f_atan2f};
use oot_game::collision_check::*;
use oot_game::effect::hahen::HAHEN_OBJECT_DEFAULT;
use oot_game::env::LIGHT_BLENDRATE_OVERRIDE_NONE;
use oot_game::pack::{BakeBody, BakeSegment, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_ONCE, Anim, SkelAnimeStd};
use oot_game::sys_matrix::MtxF;

use crate::en_goma::{ACTOR_BOSS_GOMA, ACTOR_EN_GOMA, EnGoma};

/// `ACTOR_DOOR_SHUTTER`, `ACTOR_DOOR_WARP1`, `ACTOR_ITEM_B_HEART` (`actor_table.h`).
const ACTOR_DOOR_SHUTTER: i16 = crate::door_shutter::ACTOR_DOOR_SHUTTER;
pub const ACTOR_DOOR_WARP1: i16 = 0x005D;
pub const ACTOR_ITEM_B_HEART: i16 = 0x005F;
pub const OBJECT: &str = "object_goma";
const SKELETON: &str = "gGohmaSkel";

/// `Boss_Goma_Profile`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_BOSS_GOMA,
    name: "Boss_Goma",
    category: ACTORCAT_BOSS,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED,
    object: OBJECT,
};

/// `WARP_DUNGEON_CHILD` (`DoorWarp1Type`).
pub const WARP_DUNGEON_CHILD: i16 = 0;
/// `DOORSHUTTER_PARAMS(SHUTTER_GOHMA_BLOCK, 0)`.
const GOHMA_BLOCK_PARAMS: i16 = ((crate::door_shutter::SHUTTER_GOHMA_BLOCK as i16 & 0xF) << 6) | 0;
/// `EVENTCHKINF_BEGAN_GOHMA_BATTLE` (`save.h`).
pub const EVENTCHKINF_BEGAN_GOHMA_BATTLE: u16 = 0x70;
/// `NAVI_ENEMY_GOHMA` (`actor.h`).
const NAVI_ENEMY_GOHMA: u8 = 0x01;
/// `PLAYER_CSACTION_1`, `_2`, `_7`, `_8` (`player.h`): standing facing the cutscene's actor, the
/// turn to look at Gohma, the end of a cutscene, held still.
const PLAYER_CSACTION_1: u8 = 1;
const PLAYER_CSACTION_2: u8 = 2;
const PLAYER_CSACTION_7: u8 = 7;
const PLAYER_CSACTION_8: u8 = 8;
/// `SUB_CAM_ID_DONE` (`camera.h`).
pub const SUB_CAM_ID_DONE: i16 = 0;

/// `GohmaEyeState`.
pub const EYESTATE_IRIS_FOLLOW_BONUS_IFRAMES: i16 = 0;
pub const EYESTATE_IRIS_NO_FOLLOW_NO_IFRAMES: i16 = 1;
pub const EYESTATE_IRIS_FOLLOW_NO_IFRAMES: i16 = 2;

/// `GohmaVisualState`.
pub const VISUALSTATE_RED: i16 = 0;
pub const VISUALSTATE_DEFAULT: i16 = 1;
pub const VISUALSTATE_DEFEATED: i16 = 2;
pub const VISUALSTATE_STUNNED: i16 = 4;
pub const VISUALSTATE_HIT: i16 = 5;

/// `BossGomaLimb`: the limbs the C names.
pub const BOSSGOMA_LIMB_EYE: usize = 5;
pub const BOSSGOMA_LIMB_TAIL4: usize = 11;
pub const BOSSGOMA_LIMB_TAIL3: usize = 12;
pub const BOSSGOMA_LIMB_TAIL2: usize = 13;
pub const BOSSGOMA_LIMB_TAIL1: usize = 14;
pub const BOSSGOMA_LIMB_R_FEET_BACK: usize = 21;
pub const BOSSGOMA_LIMB_R_FEET: usize = 22;
pub const BOSSGOMA_LIMB_R_SHIN: usize = 23;
pub const BOSSGOMA_LIMB_R_THIGH_SHELL: usize = 29;
pub const BOSSGOMA_LIMB_EYE_LID_BOTTOM_ROOT2: usize = 32;
pub const BOSSGOMA_LIMB_EYE_LID_TOP_ROOT2: usize = 35;
pub const BOSSGOMA_LIMB_IRIS_ROOT2: usize = 38;
pub const BOSSGOMA_LIMB_IRIS: usize = 39;
pub const BOSSGOMA_LIMB_L_ANTENNA_CLAW: usize = 57;
pub const BOSSGOMA_LIMB_R_ANTENNA_CLAW: usize = 64;
pub const BOSSGOMA_LIMB_L_FEET_BACK: usize = 73;
pub const BOSSGOMA_LIMB_L_FEET: usize = 74;
pub const BOSSGOMA_LIMB_L_SHIN: usize = 75;
pub const BOSSGOMA_LIMB_L_THIGH_SHELL: usize = 81;
/// `BOSSGOMA_LIMB_MAX`.
pub const BOSSGOMA_LIMB_MAX: usize = 86;

/// The limbs with a display list in `gGohmaSkel` (`object_goma`'s `gGohmaLimbs`; limb
/// `gGohma<X>Limb` draws `gGohma<X>DL`), by `BossGomaLimb`.
pub const LIMB_DLISTS: [(usize, &str); 34] = [
    (3, "gGohmaBodyDL"),
    (4, "gGohmaBodyShellDL"),
    (5, "gGohmaEyeDL"),
    (11, "gGohmaTail4DL"),
    (12, "gGohmaTail3DL"),
    (13, "gGohmaTail2DL"),
    (14, "gGohmaTail1DL"),
    (21, "gGohmaRightFeetBackDL"),
    (22, "gGohmaRightFeetDL"),
    (23, "gGohmaRightShinDL"),
    (26, "gGohmaRightKneeDL"),
    (29, "gGohmaRightThighShellDL"),
    (30, "gGohmaRightThighDL"),
    (33, "gGohmaEyeLidBottomDL"),
    (36, "gGohmaEyeLidTopDL"),
    (39, "gGohmaIrisDL"),
    (42, "gGohmaMandiblesBodyDL"),
    (46, "gGohmaLeftMandibles2DL"),
    (47, "gGohmaLeftMandibles1DL"),
    (51, "gGohmaRightMandibles2DL"),
    (52, "gGohmaRightMandibles1DL"),
    (57, "gGohmaLeftAntennaClawDL"),
    (58, "gGohmaLeftAntennaShellDL"),
    (59, "gGohmaLeftAntennaBodyDL"),
    (64, "gGohmaRightAntennaClawDL"),
    (65, "gGohmaRightAntennaShellDL"),
    (66, "gGohmaRightAntennaBodyDL"),
    (73, "gGohmaLeftFeetBackDL"),
    (74, "gGohmaLeftFeetDL"),
    (75, "gGohmaLeftShinDL"),
    (78, "gGohmaLeftKneeDL"),
    (81, "gGohmaLeftThighShellDL"),
    (82, "gGohmaLeftThighDL"),
    (85, "gGohmaBodyShellBackDL"),
];

/// The display list of limb `limb` (1-based), if it has one.
pub fn limb_dlist(limb: usize) -> Option<&'static str> {
    LIMB_DLISTS.iter().find(|(l, _)| *l == limb).map(|(_, s)| *s)
}

/// `sDeadLimbLifetime`, indexed by limb (the root limb is 1): how long (in frames, plus 100 as
/// the `En_Goma` piece's params) a broken-off limb lasts; 0 for the limbs that don't break off.
pub const S_DEAD_LIMB_LIFETIME: [u8; 100] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //
    30, // tail end/last part
    40, // tail 2nd to last part
    0, 0, 0, 0, 0, 0, 0, 0, //
    10, // back of right claw/hand
    15, // front of right claw/hand
    21, // part of right arm (inner)
    0, 0, //
    25, // part of right arm (shell)
    0, 0, //
    31, // part of right arm (shell on shoulder)
    35, // part of right arm (shoulder)
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //
    43, // end of left antenna
    48, // middle of left antenna
    53, // start of left antenna
    0, 0, 0, 0, //
    42, // end of right antenna
    45, // middle of right antenna
    53, // start of right antenna
    0, 0, 0, 0, 0, 0, //
    11, // back of left claw/hand
    15, // front of left claw/hand
    21, // part of left arm (inner)
    0, 0, //
    25, // part of left arm (shell)
    0, 0, //
    30, // part of left arm (shell on shoulder)
    35, // part of left arm (shoulder)
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

/// `sClearPixelTableFirstPass`: the pixels (or 2x2 blocks) the first pass erases, in order.
#[rustfmt::skip]
pub const S_CLEAR_PIXEL_TABLE_FIRST_PASS: [u8; 256] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
    0x00, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x00, 0x00, 0x00, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x00, 0x00, 0x00, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00,
    0x00, 0x01, 0x01, 0x01, 0x00, 0x00, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00,
    0x01, 0x00, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01,
    0x01, 0x00, 0x01, 0x01, 0x00, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x01, 0x00,
    0x00, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x01, 0x01, 0x00, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x01, 0x01, 0x00, 0x00, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x00, 0x01, 0x00, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x01, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x00, 0x00, 0x01, 0x01, 0x00, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00,
];

/// `sClearPixelTableSecondPass`: every pixel but one (index 71).
pub const fn clear_pixel_table_second_pass(i: usize) -> u8 {
    if i == 71 { 0 } else { 1 }
}

/// `clearPixelTable[i]` for `i` up to 256: past a table's end the read goes on into what follows
/// it in the overlay's data (@bug (game): `decayingProgress` reaches 0x100, see
/// `BossGoma_Defeated`): after the first pass's table the second's (its first byte, 1), after the
/// second's `sDeadLimbLifetime` (its first, 0). Checked against the overlay's bytes.
pub fn clear_pixel_table(first_pass: bool, i: usize) -> u8 {
    match (first_pass, i) {
        (true, 0..=255) => S_CLEAR_PIXEL_TABLE_FIRST_PASS[i],
        (true, _) => clear_pixel_table_second_pass(i - 256),
        (false, 0..=255) => clear_pixel_table_second_pass(i),
        (false, _) => S_DEAD_LIMB_LIFETIME[i - 256],
    }
}

/// Her textures `BossGoma_ClearPixels` erases (`object_goma.xml`), one after another in the file:
/// the symbol, its offset and its size (RGBA16, square). The region they make starts at the first.
pub const DECAY_TEXTURES: [(&str, u32, u32); 6] = [
    ("gGohmaBodyTex", 0x183A8, 16),
    ("gGohmaShellUndersideTex", 0x185A8, 16),
    ("gGohmaDarkShellTex", 0x187A8, 16),
    ("gGohmaShellTex", 0x189A8, 32),
    ("gGohmaEyeTex", 0x191A8, 16),
    ("gGohmaIrisTex", 0x193A8, 32),
];
/// The decaying textures' region in `object_goma`'s RAM (`ObjectContext::written`).
pub const DECAY_REGION: u32 = DECAY_TEXTURES[0].1;
const DECAY_REGION_LEN: usize = 0x1800;

/// The object's segment (`SEGMENTED_TO_VIRTUAL`'s 0x06): where her lists load these textures from.
const SEGMENT_OBJECT: u32 = 0x0600_0000;

/// The images in place of her textures her object's RAM has (written once she starts to decay):
/// `DrawParams::texture_images` for her and her pieces, empty while it's as the file has it.
pub fn decay_images(play: &PlayState, bank: Option<usize>) -> Vec<SourceImage> {
    let Some(bytes) = bank.and_then(|b| play.object_ctx.written(b, DECAY_REGION)) else { return Vec::new() };
    DECAY_TEXTURES
        .iter()
        .map(|&(_, offset, size)| {
            let start = (offset - DECAY_REGION) as usize;
            let len = (size * size * 2) as usize;
            let texels = bytes.get(start..start + len).unwrap_or(&[]);
            SourceImage { source: SEGMENT_OBJECT | offset, image: oot_game::object_ctx::rgba16_image(texels, size, size) }
        })
        .collect()
}

/// `BossGoma_ClearPixels16x16Rgba16`: pixel `i` of the 16x16 texture at `start` (the region's
/// byte offset) to 0 if the table says so. At `i` 256 it writes the next texture's first pixel
/// (@bug (game)).
fn clear_pixels_16x16(region: &mut [u8], start: usize, first_pass: bool, i: i16) {
    if clear_pixel_table(first_pass, i as usize) != 0 {
        let o = start + i as usize * 2;
        if let Some(px) = region.get_mut(o..o + 2) {
            px.fill(0);
        }
    }
}

/// `BossGoma_ClearPixels32x32Rgba16`: the 2x2 block `i` (at `(i & 0xF, i >> 4)` of a 16x16 grid
/// of blocks) of the 32x32 texture at `start` to 0 if the table says so.
fn clear_pixels_32x32(region: &mut [u8], start: usize, first_pass: bool, i: i16) {
    if clear_pixel_table(first_pass, i as usize) != 0 {
        // targetPixel += (s16)((i & 0xF) * 2 + (i & 0xF0) * 4).
        let target = ((i & 0xF) * 2 + (i & 0xF0) * 4) as usize;
        for p in [target, target + 1, target + 32, target + 33] {
            let o = start + p * 2;
            if let Some(px) = region.get_mut(o..o + 2) {
                px.fill(0);
            }
        }
    }
}

/// `BossGoma_ClearPixels`: erases step `i` of a pass from her six textures, in her object's RAM
/// (the region copied from the pack the first time).
fn clear_pixels(play: &mut PlayState, bank: Option<usize>, first_pass: bool, i: i16) {
    let Some(bank) = bank else { return };
    let assets = play.assets.clone();
    let region = play.object_ctx.written_mut(bank, DECAY_REGION, || {
        let mut v = Vec::with_capacity(DECAY_REGION_LEN);
        for (symbol, _, _) in DECAY_TEXTURES {
            match assets.as_ref().map(|a| a.pack.texture(OBJECT, symbol)) {
                Some(Ok(t)) => v.extend_from_slice(&t.texels),
                _ => log::error!("Boss_Goma: {symbol} not in the pack"),
            }
        }
        v.resize(DECAY_REGION_LEN, 0);
        v
    });
    let at = |k: usize| (DECAY_TEXTURES[k].1 - DECAY_REGION) as usize;
    clear_pixels_16x16(region, at(0), first_pass, i);
    clear_pixels_16x16(region, at(1), first_pass, i);
    clear_pixels_16x16(region, at(2), first_pass, i);
    clear_pixels_16x16(region, at(4), first_pass, i);
    clear_pixels_32x32(region, at(3), first_pass, i);
    clear_pixels_32x32(region, at(5), first_pass, i);
}

/// A sphere of `sColliderJntSphElementsInit`: hits with `0xFFCFFFFF` for 8 and is hit by
/// everything (`0xFFCFFFFF`), on `limb`, `radius` around `center`, scale 100.
const fn sphere(elem_material: u8, limb: u8, center: [i16; 3], radius: i16) -> ColliderJntSphElementInit {
    ColliderJntSphElementInit {
        info: ColliderElementInit {
            elem_material,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x08 },
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

/// `sColliderJntSphElementsInit`: the eye (element 0, the only one her hit checks), then her tail,
/// legs and antennae.
const JNT_SPH_ELEMENTS: [ColliderJntSphElementInit; 13] = [
    sphere(ELEM_MATERIAL_UNK3, BOSSGOMA_LIMB_EYE as u8, [0, 0, 1200], 20),
    sphere(ELEM_MATERIAL_UNK2, BOSSGOMA_LIMB_TAIL4 as u8, [0, 0, 0], 20),
    sphere(ELEM_MATERIAL_UNK2, BOSSGOMA_LIMB_TAIL3 as u8, [0, 0, 0], 15),
    sphere(ELEM_MATERIAL_UNK2, BOSSGOMA_LIMB_TAIL2 as u8, [0, 0, 0], 12),
    sphere(ELEM_MATERIAL_UNK2, BOSSGOMA_LIMB_TAIL1 as u8, [0, 0, 0], 25),
    sphere(ELEM_MATERIAL_UNK2, BOSSGOMA_LIMB_R_FEET as u8, [0, 0, 0], 30),
    sphere(ELEM_MATERIAL_UNK2, BOSSGOMA_LIMB_R_SHIN as u8, [0, 0, 0], 15),
    sphere(ELEM_MATERIAL_UNK2, BOSSGOMA_LIMB_R_THIGH_SHELL as u8, [0, 0, 0], 15),
    sphere(ELEM_MATERIAL_UNK2, BOSSGOMA_LIMB_L_ANTENNA_CLAW as u8, [0, 0, 0], 20),
    sphere(ELEM_MATERIAL_UNK2, BOSSGOMA_LIMB_R_ANTENNA_CLAW as u8, [0, 0, 0], 20),
    sphere(ELEM_MATERIAL_UNK2, BOSSGOMA_LIMB_L_FEET as u8, [0, 0, 0], 30),
    sphere(ELEM_MATERIAL_UNK2, BOSSGOMA_LIMB_L_SHIN as u8, [0, 0, 0], 15),
    sphere(ELEM_MATERIAL_UNK2, BOSSGOMA_LIMB_L_THIGH_SHELL as u8, [0, 0, 0], 15),
];

/// `sColliderJntSphInit`.
const JNT_SPH_INIT: ColliderInit =
    ColliderInit { col_type: COL_MATERIAL_HIT3, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_PLAYER, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_JNTSPH };

/// `BossGoma_UpdateMainEnvColor`'s `colors1` and `colors2`, by `visualState`.
const COLORS1: [[f32; 3]; 6] = [[255.0, 17.0, 0.0], [0.0, 255.0, 170.0], [50.0, 50.0, 50.0], [0.0, 255.0, 170.0], [0.0, 255.0, 170.0], [0.0, 255.0, 170.0]];
const COLORS2: [[f32; 3]; 6] = [[255.0, 17.0, 0.0], [0.0, 255.0, 170.0], [50.0, 50.0, 50.0], [0.0, 255.0, 170.0], [0.0, 0.0, 255.0], [255.0, 17.0, 0.0]];
/// `BossGoma_UpdateEyeEnvColor`'s `targetEyeEnvColors`.
const TARGET_EYE_ENV_COLORS: [[f32; 3]; 6] =
    [[255.0, 17.0, 0.0], [255.0, 255.0, 255.0], [50.0, 50.0, 50.0], [0.0, 255.0, 170.0], [0.0, 255.0, 170.0], [0.0, 255.0, 170.0]];

/// The room's centre on the floor (`BossGoma_UpdateCeilingMovement`'s and `BossGoma_Defeated`'s
/// `roomCenter`).
const ROOM_CENTER: Vec3 = Vec3::new(-150.0, 0.0, -350.0);
/// `BossGoma_PostLimbDraw`'s `focusEyeLocalPos`: the centre of the lens's surface.
const FOCUS_EYE_LOCAL_POS: Vec3 = Vec3::new(0.0, 300.0, 2650.0);

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Encounter,
    Defeated,
    FloorAttackPosture,
    FloorPrepareAttack,
    FloorAttack,
    FloorDamaged,
    FloorLandStruckDown,
    FloorLand,
    FloorStunned,
    FallJump,
    FallStruckDown,
    CeilingSpawnGohmas,
    CeilingPrepareSpawnGohmas,
    FloorIdle,
    CeilingIdle,
    FloorMain,
    WallClimb,
    CeilingMoveToCenter,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::Encounter => "BossGoma_Encounter",
            Action::Defeated => "BossGoma_Defeated",
            Action::FloorAttackPosture => "BossGoma_FloorAttackPosture",
            Action::FloorPrepareAttack => "BossGoma_FloorPrepareAttack",
            Action::FloorAttack => "BossGoma_FloorAttack",
            Action::FloorDamaged => "BossGoma_FloorDamaged",
            Action::FloorLandStruckDown => "BossGoma_FloorLandStruckDown",
            Action::FloorLand => "BossGoma_FloorLand",
            Action::FloorStunned => "BossGoma_FloorStunned",
            Action::FallJump => "BossGoma_FallJump",
            Action::FallStruckDown => "BossGoma_FallStruckDown",
            Action::CeilingSpawnGohmas => "BossGoma_CeilingSpawnGohmas",
            Action::CeilingPrepareSpawnGohmas => "BossGoma_CeilingPrepareSpawnGohmas",
            Action::FloorIdle => "BossGoma_FloorIdle",
            Action::CeilingIdle => "BossGoma_CeilingIdle",
            Action::FloorMain => "BossGoma_FloorMain",
            Action::WallClimb => "BossGoma_WallClimb",
            Action::CeilingMoveToCenter => "BossGoma_CeilingMoveToCenter",
        }
    }
}

/// Her animations (`object_goma`).
#[derive(Default)]
struct Anims {
    stand: Option<Anim>,
    hang: Option<Anim>,
    walk: Option<Anim>,
    prepare_eggs: Option<Anim>,
    attack: Option<Anim>,
    rest_after_attack: Option<Anim>,
    recover_after_attack: Option<Anim>,
    crash: Option<Anim>,
    land: Option<Anim>,
    climb: Option<Anim>,
    damage: Option<Anim>,
    death: Option<Anim>,
    prepare_attack: Option<Anim>,
    stunned: Option<Anim>,
    initial_landing: Option<Anim>,
    eye_roll: Option<Anim>,
    lay_eggs: Option<Anim>,
    idle_crouched: Option<Anim>,
    walk_crouched: Option<Anim>,
}

/// `Animation_GetLastFrame`.
fn last(a: &Option<Anim>) -> f32 {
    a.as_ref().map_or(0.0, |a| a.last_frame())
}

/// `Math_Vec3f_Pitch` (`Actor_WorldPitchTowardActor`).
fn vec3f_pitch(a: Vec3, b: Vec3) -> i16 {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    atan2_s((dx * dx + dz * dz).sqrt(), a.y - b.y)
}

pub struct BossGoma {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: Anims,
    pub action: Action,
    pub frame_count: i16,
    pub patience_timer: i16,
    pub eye_lid_bottom_rot_x: i16,
    pub eye_lid_top_rot_x: i16,
    pub eye_closed_timer: i16,
    pub eye_iris_rot_x: i16,
    pub eye_iris_rot_y: i16,
    pub unused_timer: i16,
    /// `childrenGohmaState`: 0 not spawned, 1 spawned, -1 dead (set by the child, `En_Goma`).
    pub children_gohma_state: [i16; 3],
    pub tail_limbs_scale_timers: [i16; 4],
    pub spawn_gohmas_action_timer: i16,
    pub eye_state: i16,
    pub do_not_move_this_frame: bool,
    pub visual_state: i16,
    pub invincibility_frames: i16,
    pub sub_cam_id: i16,
    pub disable_gameplay_logic: bool,
    pub decaying_progress: i16,
    pub no_backface_culling: bool,
    pub blink_timer: i16,
    pub looked_at_frames: i16,
    pub action_state: i16,
    pub frames_until_next_action: i16,
    pub timer: i16,
    pub sfx_faint_timer: i16,
    pub tail_limbs_scale: [f32; 4],
    pub eye_iris_scale_x: f32,
    pub unused_init_x: f32,
    pub unused_init_z: f32,
    pub main_env_color: [f32; 3],
    pub eye_env_color: [f32; 3],
    pub current_anim_frame_count: f32,
    pub sub_cam_follow_speed: f32,
    pub eye_iris_scale_y: f32,
    pub defeated_camera_eye_dist: f32,
    pub defeated_camera_eye_angle: f32,
    pub last_tail_limb_world_pos: Vec3,
    pub first_tail_limb_world_pos: Vec3,
    pub right_hand_back_limb_world_pos: Vec3,
    pub left_hand_back_limb_world_pos: Vec3,
    pub sub_cam_eye: Vec3,
    pub sub_cam_at: Vec3,
    /// `defeatedLimbPositions[100]` (only the first 86 used; index 0 never written: the zeroed
    /// actor's origin).
    pub defeated_limb_positions: [Vec3; 100],
    pub dead_limbs_state: [u8; 100],
    pub collider: ColliderJntSph,
    /// The eye's env colour `BossGoma_OverrideLimbDraw` drew this game frame while invincible
    /// (its draw-time `Rand`, made in `draw_update`), else `None`.
    pub eye_rand_env: Option<[u8; 3]>,
}

impl BossGoma {
    /// `BossGoma_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: ATTENTION_RANGE_2, NAVI_ENEMY_GOHMA, gravity -2.
        actor.target_mode = 2;
        actor.navi_enemy_id = NAVI_ENEMY_GOHMA;
        actor.gravity = -2.0;
        // ActorShape_Init(&shape, 4000, ActorShadow_DrawCircle, 150): the shadow isn't ported.
        actor.shape_y_offset = 4000.0;
        let mut anims = Anims::default();
        let mut skeleton = None;
        if let Some(a) = play.assets.clone() {
            let anim = |name: &str| a.animation(OBJECT, name).map_err(|e| log::error!("Boss_Goma: {e:#}")).ok();
            skeleton = a.skeleton(OBJECT, SKELETON).map_err(|e| log::error!("Boss_Goma: {e:#}")).ok();
            anims = Anims {
                stand: anim("gGohmaStandAnim"),
                hang: anim("gGohmaHangAnim"),
                walk: anim("gGohmaWalkAnim"),
                prepare_eggs: anim("gGohmaPrepareEggsAnim"),
                attack: anim("gGohmaAttackAnim"),
                rest_after_attack: anim("gGohmaRestAfterAttackAnim"),
                recover_after_attack: anim("gGohmaRecoverAfterAttackAnim"),
                crash: anim("gGohmaCrashAnim"),
                land: anim("gGohmaLandAnim"),
                climb: anim("gGohmaClimbAnim"),
                damage: anim("gGohmaDamageAnim"),
                death: anim("gGohmaDeathAnim"),
                prepare_attack: anim("gGohmaPrepareAttackAnim"),
                stunned: anim("gGohmaStunnedAnim"),
                initial_landing: anim("gGohmaInitialLandingAnim"),
                eye_roll: anim("gGohmaEyeRollAnim"),
                lay_eggs: anim("gGohmaLayEggsAnim"),
                idle_crouched: anim("gGohmaIdleCrouchedAnim"),
                walk_crouched: anim("gGohmaWalkCrouchedAnim"),
            };
        }
        // SkelAnime_Init(&gGohmaSkel, &gGohmaIdleCrouchedAnim), Animation_PlayLoop.
        let limbs = skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(BOSSGOMA_LIMB_MAX - 1);
        let skel = SkelAnimeStd::init_flex(limbs, anims.idle_crouched.clone());
        let world = actor.world_pos;
        let mut g = BossGoma {
            actor,
            skel,
            skeleton,
            anims,
            action: Action::Encounter,
            frame_count: 0,
            patience_timer: 0,
            eye_lid_bottom_rot_x: 0,
            eye_lid_top_rot_x: 0,
            eye_closed_timer: 0,
            eye_iris_rot_x: 0,
            eye_iris_rot_y: 0,
            unused_timer: 0,
            children_gohma_state: [0; 3],
            tail_limbs_scale_timers: [0; 4],
            spawn_gohmas_action_timer: 0,
            eye_state: EYESTATE_IRIS_FOLLOW_BONUS_IFRAMES,
            do_not_move_this_frame: false,
            visual_state: VISUALSTATE_RED,
            invincibility_frames: 0,
            sub_cam_id: SUB_CAM_ID_DONE,
            disable_gameplay_logic: false,
            decaying_progress: 0,
            no_backface_culling: false,
            blink_timer: 0,
            looked_at_frames: 0,
            action_state: 0,
            frames_until_next_action: 0,
            timer: 0,
            sfx_faint_timer: 0,
            tail_limbs_scale: [0.0; 4],
            eye_iris_scale_x: 1.0,
            unused_init_x: world.x,
            unused_init_z: world.z,
            main_env_color: [0.0; 3],
            eye_env_color: [0.0; 3],
            current_anim_frame_count: 0.0,
            sub_cam_follow_speed: 0.0,
            eye_iris_scale_y: 1.0,
            defeated_camera_eye_dist: 0.0,
            defeated_camera_eye_angle: 0.0,
            last_tail_limb_world_pos: Vec3::ZERO,
            first_tail_limb_world_pos: Vec3::ZERO,
            right_hand_back_limb_world_pos: Vec3::ZERO,
            left_hand_back_limb_world_pos: Vec3::ZERO,
            sub_cam_eye: Vec3::ZERO,
            sub_cam_at: Vec3::ZERO,
            defeated_limb_positions: [Vec3::ZERO; 100],
            dead_limbs_state: [0; 100],
            collider: ColliderJntSph::new(&JNT_SPH_INIT, &JNT_SPH_ELEMENTS),
            eye_rand_env: None,
        };
        // Upside down on the ceiling.
        g.actor.shape_rot.x = i16::MIN;
        g.actor.world_pos.y = -300.0;
        g.actor.gravity = 0.0;
        g.setup_encounter(play);
        g.actor.col_chk_info.health = 10;
        g.actor.col_chk_info.mass = MASS_IMMOVABLE;
        if play.flags.get_clear(play.room_ctx.cur.num) {
            g.actor.kill();
            let pos = Vec3::new(0.0, -640.0, 0.0);
            if let Err(e) = play.actor_spawn_as_child(&mut g.actor, ACTOR_DOOR_WARP1, pos, [0; 3], WARP_DUNGEON_CHILD) {
                log::warn!("Boss_Goma: the blue warp not spawned: {e:?}");
            }
            if let Err(e) = play.actor_spawn(ACTOR_ITEM_B_HEART, Vec3::new(141.0, -640.0, -84.0), [0; 3], 0) {
                log::warn!("Boss_Goma: the heart container not spawned: {e:?}");
            }
        }
        Box::new(g)
    }

    /// `Animation_Change(&skelanime, anim, speed, start, end, mode, morph)`.
    fn change(&mut self, anim: Option<Anim>, speed: f32, start: f32, end: f32, mode: u8, morph: f32) {
        if let Some(a) = anim {
            self.skel.change(a, speed, start, end, mode, morph);
        }
    }

    fn player_pos(play: &PlayState) -> Vec3 {
        play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or_default()
    }

    /// `Actor_WorldYawTowardActor(&this->actor, &GET_PLAYER(play)->actor)`.
    fn yaw_to_player(&self, play: &PlayState) -> i16 {
        vec3f_yaw(self.actor.world_pos, Self::player_pos(play))
    }

    /// `GET_PLAYER(play)->actor`, to change.
    fn player_mut(play: &mut PlayState) -> Option<&mut Actor> {
        play.player.and_then(|h| play.actors.actor_mut(h))
    }

    /// `Player_SetCsActionWithHaltedActors(play, &this->actor, mode)`.
    fn player_cs(play: &mut PlayState, mode: u8) {
        let me = play.cur_actor;
        play.player_set_cs_action_with_halted_actors(me, mode);
    }

    /// `Rumble_Override`: not ported.
    fn rumble(strength: u8) {
        log::debug!("Boss_Goma: Rumble_Override({strength}) not ported");
    }

    /// `BossGoma_PlayEffectsAndSfx`: dust under her right (`arg2` 0, 1, 3) and left (0, 2, 3)
    /// claws, and her landing's or her step's sound.
    fn play_effects_and_sfx(&mut self, play: &mut PlayState, arg2: i16, amount_minus1: i16) {
        if arg2 == 0 || arg2 == 1 || arg2 == 3 {
            let (a, p) = (self.actor.clone(), self.right_hand_back_limb_world_pos);
            actor_spawn_floor_dust_ring(play, &a, p, 25.0, amount_minus1 as i32, 8.0, 500, 10, true);
        }
        if arg2 == 0 || arg2 == 2 || arg2 == 3 {
            let (a, p) = (self.actor.clone(), self.left_hand_back_limb_world_pos);
            actor_spawn_floor_dust_ring(play, &a, p, 25.0, amount_minus1 as i32, 8.0, 500, 10, true);
        }
        audio_play_actor_sfx2(play, if arg2 == 0 { NA_SE_EN_GOMA_DOWN } else { NA_SE_EN_GOMA_WALK });
    }

    /// `BossGoma_SetupDefeated`: her health gone.
    fn setup_defeated(&mut self, play: &mut PlayState) {
        self.change(self.anims.death.clone(), 1.0, 0.0, last(&self.anims.death), ANIMMODE_ONCE, -2.0);
        self.action = Action::Defeated;
        self.disable_gameplay_logic = true;
        self.decaying_progress = 0;
        self.no_backface_culling = false;
        self.frames_until_next_action = 1200;
        self.action_state = 0;
        self.actor.flags &= !(ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE);
        self.actor.speed_xz = 0.0;
        // this->actor.shape.shadowScale = 0: the shadow isn't ported.
        play.audio.queue_seq_cmd(seq_cmd1(SEQ_PLAYER_BGM_MAIN, 1));
        audio_play_actor_sfx2(play, NA_SE_EN_GOMA_DEAD);
    }

    /// `BossGoma_SetupEncounter`: waiting on the ceiling for the fight to start, the room's
    /// lights at setting 4 at once.
    fn setup_encounter(&mut self, play: &mut PlayState) {
        let last_frame = last(&self.anims.walk);
        self.change(self.anims.walk.clone(), 1.0, 0.0, last_frame, ANIMMODE_LOOP, -15.0);
        self.action = Action::Encounter;
        self.action_state = 0;
        self.disable_gameplay_logic = true;
        play.env_ctx.light_setting_override = 4;
        play.env_ctx.light_blend_rate_override = 255;
    }

    /// `BossGoma_SetupFloorIdle`: 20 to 49 frames.
    fn setup_floor_idle(&mut self, play: &mut PlayState) {
        let last_frame = last(&self.anims.idle_crouched);
        self.frames_until_next_action = play.rand.s16_offset(20, 30);
        self.change(self.anims.idle_crouched.clone(), 1.0, 0.0, last_frame, ANIMMODE_LOOP, -5.0);
        self.action = Action::FloorIdle;
    }

    /// `BossGoma_SetupCeilingIdle`: 20 to 49 frames.
    fn setup_ceiling_idle(&mut self, play: &mut PlayState) {
        self.frames_until_next_action = play.rand.s16_offset(20, 30);
        self.change(self.anims.hang.clone(), 1.0, 0.0, last(&self.anims.hang), ANIMMODE_LOOP, -5.0);
        self.action = Action::CeilingIdle;
    }

    /// `BossGoma_SetupFallJump`: her eggs' larvae dead, she jumps down (the landing's animation
    /// to frame 0).
    fn setup_fall_jump(&mut self) {
        self.change(self.anims.land.clone(), 1.0, 0.0, 0.0, ANIMMODE_ONCE, -5.0);
        self.action = Action::FallJump;
        self.actor.speed_xz = 0.0;
        self.actor.velocity.y = 0.0;
        self.actor.gravity = -2.0;
    }

    /// `BossGoma_SetupFallStruckDown`: hit on the ceiling.
    fn setup_fall_struck_down(&mut self) {
        self.change(self.anims.crash.clone(), 1.0, 0.0, 0.0, ANIMMODE_ONCE, -5.0);
        self.action = Action::FallStruckDown;
        self.actor.speed_xz = 0.0;
        self.actor.velocity.y = 0.0;
        self.actor.gravity = -2.0;
    }

    /// `BossGoma_SetupCeilingSpawnGohmas`.
    fn setup_ceiling_spawn_gohmas(&mut self) {
        self.change(self.anims.lay_eggs.clone(), 1.0, 0.0, last(&self.anims.lay_eggs), ANIMMODE_LOOP, -15.0);
        self.action = Action::CeilingSpawnGohmas;
        self.spawn_gohmas_action_timer = 0;
    }

    /// `BossGoma_SetupCeilingPrepareSpawnGohmas`: 70 frames.
    fn setup_ceiling_prepare_spawn_gohmas(&mut self) {
        self.change(self.anims.prepare_eggs.clone(), 1.0, 0.0, last(&self.anims.prepare_eggs), ANIMMODE_LOOP, -10.0);
        self.action = Action::CeilingPrepareSpawnGohmas;
        self.frames_until_next_action = 70;
    }

    /// `BossGoma_SetupWallClimb`.
    fn setup_wall_climb(&mut self) {
        self.change(self.anims.climb.clone(), 1.0, 0.0, last(&self.anims.climb), ANIMMODE_LOOP, -10.0);
        self.action = Action::WallClimb;
        self.actor.speed_xz = 0.0;
        self.actor.velocity.y = 0.0;
        self.actor.gravity = 0.0;
    }

    /// `BossGoma_SetupCeilingMoveToCenter`: 30 to 89 frames at least.
    fn setup_ceiling_move_to_center(&mut self, play: &mut PlayState) {
        self.change(self.anims.walk.clone(), 1.0, 0.0, last(&self.anims.walk), ANIMMODE_LOOP, -5.0);
        self.action = Action::CeilingMoveToCenter;
        self.actor.speed_xz = 0.0;
        self.actor.velocity.y = 0.0;
        self.actor.gravity = 0.0;
        self.frames_until_next_action = play.rand.s16_offset(30, 60);
    }

    /// `BossGoma_SetupFloorMain`: 70 to 179 frames.
    fn setup_floor_main(&mut self, play: &mut PlayState) {
        self.change(self.anims.walk_crouched.clone(), 1.0, 0.0, last(&self.anims.walk_crouched), ANIMMODE_LOOP, -5.0);
        self.action = Action::FloorMain;
        self.frames_until_next_action = play.rand.s16_offset(70, 110);
    }

    /// `BossGoma_SetupFloorLand`.
    fn setup_floor_land(&mut self) {
        self.change(self.anims.land.clone(), 1.0, 0.0, last(&self.anims.land), ANIMMODE_ONCE, -2.0);
        self.action = Action::FloorLand;
        self.current_anim_frame_count = last(&self.anims.land);
    }

    /// `BossGoma_SetupFloorLandStruckDown`.
    fn setup_floor_land_struck_down(&mut self) {
        self.change(self.anims.crash.clone(), 1.0, 0.0, last(&self.anims.crash), ANIMMODE_ONCE, -2.0);
        self.current_anim_frame_count = last(&self.anims.crash);
        self.action = Action::FloorLandStruckDown;
        self.current_anim_frame_count = last(&self.anims.crash);
    }

    /// `BossGoma_SetupFloorStunned`.
    fn setup_floor_stunned(&mut self) {
        self.change(self.anims.stunned.clone(), 1.0, 0.0, last(&self.anims.stunned), ANIMMODE_LOOP, -2.0);
        self.action = Action::FloorStunned;
    }

    /// `BossGoma_SetupFloorAttackPosture`.
    fn setup_floor_attack_posture(&mut self) {
        self.change(self.anims.prepare_attack.clone(), 1.0, 0.0, last(&self.anims.prepare_attack), ANIMMODE_ONCE, -10.0);
        self.action = Action::FloorAttackPosture;
    }

    /// `BossGoma_SetupFloorPrepareAttack`: a frame.
    fn setup_floor_prepare_attack(&mut self) {
        self.change(self.anims.stand.clone(), 1.0, 0.0, last(&self.anims.stand), ANIMMODE_LOOP, -10.0);
        self.action = Action::FloorPrepareAttack;
        self.frames_until_next_action = 0;
    }

    /// `BossGoma_SetupFloorAttack`.
    fn setup_floor_attack(&mut self) {
        self.change(self.anims.attack.clone(), 1.0, 0.0, last(&self.anims.attack), ANIMMODE_ONCE, -10.0);
        self.action = Action::FloorAttack;
        self.action_state = 0;
        self.frames_until_next_action = 0;
    }

    /// `BossGoma_SetupFloorDamaged`: the timers kept but her patience (`framesUntilNextAction`
    /// is the stun's).
    fn setup_floor_damaged(&mut self) {
        self.change(self.anims.damage.clone(), 1.0, 0.0, last(&self.anims.damage), ANIMMODE_ONCE, -2.0);
        self.action = Action::FloorDamaged;
    }

    /// `BossGoma_UpdateCeilingMovement`: towards `targetSpeedXZ`, turned away from the room's
    /// centre if asked; at her steps (frames 9 and 1) five fragments fall from her claw
    /// (`EffectSsHahen`) with `NA_SE_EN_GOMA_HIGH`. (`roomCenter.z += dz`: `dz` is always 0.)
    fn update_ceiling_movement(&mut self, play: &mut PlayState, _dz: f32, target_speed_xz: f32, rotate_towards_center: bool) {
        self.skel.update();
        approach_f(&mut self.actor.speed_xz, target_speed_xz, 0.5, 2.0);
        if rotate_towards_center {
            approach_s(&mut self.actor.world_rot.y, vec3f_yaw(self.actor.world_pos, ROOM_CENTER).wrapping_add(i16::MIN), 3, 0x3E8);
        }
        let base_pos = if self.skel.on_frame(9.0) {
            Some(self.right_hand_back_limb_world_pos)
        } else if self.skel.on_frame(1.0) {
            Some(self.left_hand_back_limb_world_pos)
        } else {
            None
        };
        if let Some(base) = base_pos {
            play.with_ss(|ss| {
                for _ in 0..5 {
                    // velInit, accelInit.
                    let vel = Vec3::ZERO;
                    let accel = Vec3::new(0.0, -0.5, 0.0);
                    let x = ss.rand.centered_float(70.0) + base.x;
                    let y = ss.rand.zero_float(30.0) + base.y;
                    let z = ss.rand.centered_float(70.0) + base.z;
                    let scale = (ss.rand.zero_one() * 5.0) as i16 + 10;
                    ss.hahen_spawn(Vec3::new(x, y, z), vel, accel, 0, scale, HAHEN_OBJECT_DEFAULT, 10, None);
                }
            });
            audio_play_actor_sfx2(play, NA_SE_EN_GOMA_HIGH);
        }
    }

    /// `BossGoma_SetupEncounterState4`: the focus on her at the ceiling's centre: a manual
    /// cutscene, Link facing her from the room's entrance (`PLAYER_CSACTION_1`), her sub camera
    /// from in front of him, her eye roll, the music stopped.
    fn setup_encounter_state4(&mut self, play: &mut PlayState) {
        let main_cam_eye_y = play.game_camera.eye.y;
        self.action_state = 4;
        self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
        play.start_manual();
        Self::player_cs(play, PLAYER_CSACTION_1);
        self.sub_cam_id = play.create_sub_camera();
        play.change_camera_status(CAM_ID_MAIN, CAM_STAT_UNK3);
        play.change_camera_status(self.sub_cam_id, CAM_STAT_ACTIVE);
        self.change(self.anims.eye_roll.clone(), 1.0, 0.0, last(&self.anims.eye_roll), ANIMMODE_ONCE, 0.0);
        self.current_anim_frame_count = last(&self.anims.eye_roll);
        // The room's centre.
        self.actor.world_pos.x = -150.0;
        self.actor.world_pos.z = -350.0;
        // The room's entrance, towards its centre.
        if let Some(p) = Self::player_mut(play) {
            p.world_pos.x = 150.0;
            p.world_pos.z = 300.0;
            p.world_rot.y = -0x705C;
            p.shape_rot.y = -0x705C;
        }
        self.actor.world_rot.y = self.yaw_to_player(play).wrapping_add(i16::MIN);
        // The room's entrance, closer to its centre.
        self.sub_cam_eye.x = 90.0;
        self.sub_cam_eye.z = 170.0;
        self.sub_cam_eye.y = main_cam_eye_y + 20.0;
        self.frames_until_next_action = 50;
        self.sub_cam_at = self.actor.world_pos;
        play.audio.queue_seq_cmd(seq_cmd1(SEQ_PLAYER_BGM_MAIN, 1));
    }

    /// The slab behind Link (`Actor_SpawnAsChild(ACTOR_DOOR_SHUTTER, 164.72, -480, 397.68, 0,
    /// -0x705C, 0, DOORSHUTTER_PARAMS(SHUTTER_GOHMA_BLOCK, 0))`).
    fn spawn_gohma_block(&mut self, play: &mut PlayState) {
        let pos = Vec3::new(164.72, -480.0, 397.68002);
        if let Err(e) = play.actor_spawn_as_child(&mut self.actor, ACTOR_DOOR_SHUTTER, pos, [0, -0x705C, 0], GOHMA_BLOCK_PARAMS) {
            log::warn!("Boss_Goma: the Gohma slab not spawned: {e:?}");
        }
    }

    /// The end of one of her cutscenes: the main camera taking her sub camera's view
    /// (`mainCam->eye`, `eyeNext`, `at`), `Play_ReturnToMainCam`, `SUB_CAM_ID_DONE`.
    fn return_to_main_cam(&mut self, play: &mut PlayState) {
        play.game_camera.eye = self.sub_cam_eye;
        play.game_camera.eye_next = self.sub_cam_eye;
        play.game_camera.at = self.sub_cam_at;
        play.play_return_to_main_cam(self.sub_cam_id, 0);
        self.sub_cam_id = SUB_CAM_ID_DONE;
    }

    /// `BossGoma_Encounter`: the intro (states 0 to 3: the door and the look at her, skipped once
    /// her battle has begun; 4 on: her eye roll, her run, the drop, the landing, the camera back).
    fn encounter(&mut self, play: &mut PlayState) {
        approach_zero_f(&mut self.actor.speed_xz, 0.5, 2.0);
        let player_pos = Self::player_pos(play);
        let mut state = self.action_state;
        // case 0: waiting for Link at the boss room's entrance.
        if state == 0 {
            if (player_pos.x - 150.0).abs() < 60.0 && (player_pos.z - 350.0).abs() < 60.0 {
                if play.save.get_event_chk_inf(EVENTCHKINF_BEGAN_GOHMA_BATTLE) {
                    self.setup_encounter_state4(play);
                    self.spawn_gohma_block(play);
                } else {
                    Self::player_cs(play, PLAYER_CSACTION_8);
                    self.action_state = 1;
                }
            }
        } else if state == 1 {
            // Link entered the room.
            play.start_manual();
            self.sub_cam_id = play.create_sub_camera();
            log::debug!("MAKE CAMERA !!!   1   !!!!!!!!!!!!!!!!!!!!!!!!!!");
            play.change_camera_status(CAM_ID_MAIN, CAM_STAT_WAIT);
            play.change_camera_status(self.sub_cam_id, CAM_STAT_ACTIVE);
            self.action_state = 2;
            // The ceiling's centre.
            self.actor.world_pos = Vec3::new(-150.0, -320.0, -350.0);
            // The room's entrance.
            if let Some(p) = Self::player_mut(play) {
                p.world_pos.x = 150.0;
                p.world_pos.z = 300.0;
            }
            let player_pos = Self::player_pos(play);
            // Near the ceiling's centre.
            self.sub_cam_eye = Vec3::new(-350.0, -310.0, -350.0);
            // Below the room's entrance.
            self.sub_cam_at = Vec3::new(player_pos.x, player_pos.y - 200.0 + 25.0, player_pos.z);
            self.frames_until_next_action = 50;
            self.timer = 80;
            self.frame_count = 0;
            self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
            // FALLTHROUGH
            state = 2;
        }
        match state {
            2 => {
                // The zoom on Link from the room's centre.
                if let Some(p) = Self::player_mut(play) {
                    p.shape_rot.y = -0x705C;
                    p.world_pos.x = 150.0;
                    p.world_pos.z = 300.0;
                    p.world_rot.y = p.shape_rot.y;
                    p.speed_xz = 0.0;
                }
                let player_pos = Self::player_pos(play);
                if self.frames_until_next_action == 0 {
                    // (-20, 25, -65) is towards the room's centre.
                    approach_f(&mut self.sub_cam_eye.x, player_pos.x - 20.0, 0.049999997, self.sub_cam_follow_speed * 50.0);
                    approach_f(&mut self.sub_cam_eye.y, player_pos.y + 25.0, 0.099999994, self.sub_cam_follow_speed * 130.0);
                    approach_f(&mut self.sub_cam_eye.z, player_pos.z - 65.0, 0.049999997, self.sub_cam_follow_speed * 30.0);
                    approach_f(&mut self.sub_cam_follow_speed, 0.29999998, 1.0, 0.0050000004);
                    if self.timer == 0 {
                        approach_f(&mut self.sub_cam_at.y, player_pos.y + 35.0, 0.099999994, self.sub_cam_follow_speed * 30.0);
                    }
                    self.sub_cam_at.x = player_pos.x;
                    self.sub_cam_at.z = player_pos.z;
                }
                play.camera_set_at_eye(CAM_ID_MAIN, self.sub_cam_at, self.sub_cam_eye);
                if self.frame_count == 176 {
                    self.spawn_gohma_block(play);
                }
                if self.frame_count == 176 {
                    play.env_ctx.light_setting_override = 3;
                    play.env_ctx.light_blend_rate_override = LIGHT_BLENDRATE_OVERRIDE_NONE;
                }
                if self.frame_count == 190 {
                    Self::player_cs(play, PLAYER_CSACTION_2);
                }
                if self.frame_count >= 228 {
                    self.return_to_main_cam(play);
                    play.stop_manual();
                    Self::player_cs(play, PLAYER_CSACTION_7);
                    self.action_state = 3;
                }
            }
            3 => {
                // Waiting for Link to look at her.
                let pp = self.actor.projected_pos;
                if pp.x.abs() < 150.0 && pp.y.abs() < 250.0 && pp.z < 800.0 && pp.z > 0.0 {
                    self.looked_at_frames += 1;
                    approach_zero_f(&mut self.actor.speed_xz, 0.5, 2.0);
                    let yaw = self.yaw_to_player(play).wrapping_add(i16::MIN);
                    approach_s(&mut self.actor.world_rot.y, yaw, 2, 0xBB8);
                    self.eye_lid_bottom_rot_x = 0;
                    self.eye_lid_top_rot_x = 0;
                    self.eye_iris_rot_x = 0;
                    self.eye_iris_rot_y = 0;
                } else {
                    self.looked_at_frames = 0;
                    self.update_ceiling_movement(play, 0.0, -5.0, true);
                }
                if self.looked_at_frames > 15 {
                    self.setup_encounter_state4(play);
                }
            }
            4 => {
                // The focus on her at the ceiling.
                if self.skel.on_frame(15.0) {
                    audio_play_actor_sfx2(play, NA_SE_EN_GOMA_DEMO_EYE);
                }
                if self.frames_until_next_action <= 40 {
                    // (22, -25, 45) is towards the room's entrance.
                    let w = self.actor.world_pos;
                    approach_f(&mut self.sub_cam_eye.x, w.x + 22.0, 0.2, 100.0);
                    approach_f(&mut self.sub_cam_eye.y, w.y - 25.0, 0.2, 100.0);
                    approach_f(&mut self.sub_cam_eye.z, w.z + 45.0, 0.2, 100.0);
                    approach_f(&mut self.sub_cam_at.x, w.x, 0.2, 100.0);
                    approach_f(&mut self.sub_cam_at.y, w.y + 5.0, 0.2, 100.0);
                    approach_f(&mut self.sub_cam_at.z, w.z, 0.2, 100.0);
                    if self.frames_until_next_action == 30 {
                        play.env_ctx.light_setting_override = 4;
                    }
                    if self.frames_until_next_action < 20 {
                        self.skel.update();
                        approach_f(&mut self.eye_iris_scale_x, 1.0, 0.8, 0.4);
                        approach_f(&mut self.eye_iris_scale_y, 1.0, 0.8, 0.4);
                        if self.skel.on_frame(36.0) {
                            self.eye_iris_scale_x = 1.8;
                            self.eye_iris_scale_y = 1.8;
                        }
                        if self.skel.on_frame(self.current_anim_frame_count) {
                            self.action_state = 5;
                            self.change(self.anims.walk.clone(), 2.0, 0.0, last(&self.anims.walk), ANIMMODE_LOOP, -5.0);
                            self.frames_until_next_action = 30;
                            self.sub_cam_follow_speed = 0.0;
                        }
                    }
                }
            }
            5 => {
                // Running on the ceiling. (98, 0, 85) is towards the room's entrance.
                let w = self.actor.world_pos;
                let player_y = Self::player_pos(play).y;
                approach_f(&mut self.sub_cam_eye.x, w.x + 8.0 + 90.0, 0.1, self.sub_cam_follow_speed * 30.0);
                approach_f(&mut self.sub_cam_eye.y, player_y, 0.1, self.sub_cam_follow_speed * 30.0);
                approach_f(&mut self.sub_cam_eye.z, w.z + 45.0 + 40.0, 0.1, self.sub_cam_follow_speed * 30.0);
                approach_f(&mut self.sub_cam_follow_speed, 1.0, 1.0, 0.05);
                self.sub_cam_at = self.actor.world_pos;
                if self.frames_until_next_action < 0 {
                    // @bug (game)? (the C's `//!`): unreachable, the timer is >= 0.
                    self.skel.update();
                    approach_zero_f(&mut self.actor.speed_xz, 1.0, 2.0);
                } else {
                    self.update_ceiling_movement(play, 0.0, -7.5, false);
                }
                if self.frames_until_next_action == 0 {
                    self.change(self.anims.hang.clone(), 1.0, 0.0, last(&self.anims.hang), ANIMMODE_LOOP, -5.0);
                }
                if self.frames_until_next_action == 0 {
                    self.action_state = 9;
                    self.actor.speed_xz = 0.0;
                    self.actor.velocity.y = 0.0;
                    self.actor.gravity = -2.0;
                    self.change(self.anims.initial_landing.clone(), 1.0, 0.0, last(&self.anims.initial_landing), ANIMMODE_ONCE, -5.0);
                    if let Some(p) = Self::player_mut(play) {
                        p.world_pos.x = 0.0;
                        p.world_pos.z = -30.0;
                    }
                }
            }
            9 => {
                // Falling from the ceiling.
                let w = self.actor.world_pos;
                let player_y = Self::player_pos(play).y;
                approach_f(&mut self.sub_cam_eye.x, w.x + 8.0 + 90.0, 0.1, self.sub_cam_follow_speed * 30.0);
                approach_f(&mut self.sub_cam_eye.y, player_y + 10.0, 0.1, self.sub_cam_follow_speed * 30.0);
                approach_f(&mut self.sub_cam_eye.z, w.z + 45.0 + 40.0, 0.1, self.sub_cam_follow_speed * 30.0);
                self.sub_cam_at = self.actor.world_pos;
                self.skel.update();
                approach_s(&mut self.actor.shape_rot.x, 0, 2, 0xBB8);
                let yaw = self.yaw_to_player(play);
                approach_s(&mut self.actor.world_rot.y, yaw, 2, 0x7D0);
                if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
                    self.action_state = 130;
                    self.actor.velocity.y = 0.0;
                    self.change(self.anims.initial_landing.clone(), 1.0, 0.0, last(&self.anims.initial_landing), ANIMMODE_ONCE, -2.0);
                    self.current_anim_frame_count = last(&self.anims.initial_landing);
                    self.play_effects_and_sfx(play, 0, 5);
                    self.frames_until_next_action = 15;
                    Self::rumble(200);
                }
            }
            130 => {
                // The focus on her on the floor.
                let w = self.actor.world_pos;
                let player_y = Self::player_pos(play).y;
                approach_f(&mut self.sub_cam_eye.x, w.x + 8.0 + 90.0, 0.1, self.sub_cam_follow_speed * 30.0);
                approach_f(&mut self.sub_cam_eye.y, player_y + 10.0, 0.1, self.sub_cam_follow_speed * 30.0);
                approach_f(&mut self.sub_cam_eye.z, w.z + 45.0 + 40.0, 0.1, self.sub_cam_follow_speed * 30.0);
                approach_s(&mut self.actor.shape_rot.x, 0, 2, 0xBB8);
                let yaw = self.yaw_to_player(play);
                approach_s(&mut self.actor.world_rot.y, yaw, 2, 0x7D0);
                self.skel.update();
                self.sub_cam_at.x = self.actor.world_pos.x;
                self.sub_cam_at.z = self.actor.world_pos.z;
                if self.frames_until_next_action != 0 {
                    let s = (self.frames_until_next_action as f32 * 3.1415 * 0.5).sin();
                    self.sub_cam_at.y = self.frames_until_next_action as f32 * s * 0.7 + self.actor.world_pos.y;
                } else {
                    approach_f(&mut self.sub_cam_at.y, self.actor.focus_pos.y, 0.1, 10.0);
                }
                if self.skel.on_frame(40.0) {
                    audio_play_actor_sfx2(play, NA_SE_EN_GOMA_CRY1);
                    if !play.save.get_event_chk_inf(EVENTCHKINF_BEGAN_GOHMA_BATTLE) {
                        // TitleCard_InitBossName(gGohmaTitleCardTex, 160, 180, 128, 40).
                        let (object, offset, w, h) = oot_game::title_card::BOSS_NAMES[0];
                        play.title_ctx.init_boss_name(object, offset, 160, 180, w, h);
                    }
                    play.audio.queue_seq_cmd(start_seq(SEQ_PLAYER_BGM_MAIN, 0, NA_BGM_BOSS));
                    play.save.set_event_chk_inf(EVENTCHKINF_BEGAN_GOHMA_BATTLE);
                }
                if self.skel.on_frame(self.current_anim_frame_count) {
                    self.action_state = 140;
                    self.change(self.anims.stand.clone(), 1.0, 0.0, last(&self.anims.stand), ANIMMODE_LOOP, -10.0);
                    self.frames_until_next_action = 20;
                }
            }
            140 => {
                self.skel.update();
                approach_f(&mut self.sub_cam_at.y, self.actor.focus_pos.y, 0.1, 10.0);
                if self.frames_until_next_action == 0 {
                    self.frames_until_next_action = 30;
                    self.action_state = 150;
                    play.change_camera_status(CAM_ID_MAIN, CAM_STAT_UNK3);
                }
            }
            150 => {
                self.skel.update();
                let w = self.actor.world_pos;
                smooth_step_to_f(&mut self.sub_cam_eye.x, w.x + 150.0, 0.2, 100.0, 0.1);
                smooth_step_to_f(&mut self.sub_cam_eye.y, w.y + 20.0, 0.2, 100.0, 0.1);
                smooth_step_to_f(&mut self.sub_cam_eye.z, w.z + 220.0, 0.2, 100.0, 0.1);
                if self.frames_until_next_action == 0 {
                    self.return_to_main_cam(play);
                    self.setup_floor_main(play);
                    self.disable_gameplay_logic = false;
                    self.patience_timer = 200;
                    play.stop_manual();
                    Self::player_cs(play, PLAYER_CSACTION_7);
                }
            }
            _ => {}
        }
        if self.sub_cam_id != SUB_CAM_ID_DONE {
            play.camera_set_at_eye(self.sub_cam_id, self.sub_cam_at, self.sub_cam_eye);
        }
    }

    /// `BossGoma_Defeated`: the death (see the module's doc).
    fn defeated(&mut self, play: &mut PlayState) {
        let vel1 = Vec3::ZERO;
        let accel1 = Vec3::new(0.0, 1.0, 0.0);
        let color1 = [255, 255, 255, 255];
        let color2 = [0, 100, 255, 255];
        let vel2 = Vec3::ZERO;
        let accel2 = Vec3::new(0.0, -0.5, 0.0);
        self.skel.update();
        approach_s(&mut self.actor.shape_rot.x, 0, 2, 0xBB8);
        if self.skel.on_frame(107.0) {
            self.play_effects_and_sfx(play, 0, 8);
            Self::rumble(150);
        }
        self.visual_state = VISUALSTATE_DEFEATED;
        self.eye_state = EYESTATE_IRIS_NO_FOLLOW_NO_IFRAMES;
        if self.frames_until_next_action == 1001 {
            for i in 0..90 {
                if S_DEAD_LIMB_LIFETIME[i] != 0 {
                    self.dead_limbs_state[i] = 1;
                }
            }
        }
        if self.frames_until_next_action < 1200 && self.frames_until_next_action > 1100 && self.frames_until_next_action % 8 == 0 {
            let focus = self.actor.focus_pos;
            play.with_ss(|ss| ss.sibuki_spawn_burst(focus));
        }
        if self.frames_until_next_action < 1080 && self.action_state < 3 {
            if self.frames_until_next_action < 1070 {
                audio_play_actor_sfx2(play, NA_SE_EN_GOMA_LAST - SFX_FLAG);
            }
            let positions = self.defeated_limb_positions;
            play.with_ss(|ss| {
                for _ in 0..4 {
                    // @bug (game) (the C's `//!`): limbs are 1-based, so index 0, never written
                    // (the zeroed actor's origin), can be picked, and the last limb can't.
                    let j = (ss.rand.zero_one() * (BOSSGOMA_LIMB_MAX - 1) as f32) as i16 as usize;
                    if positions[j].y < 10000.0 {
                        let x = ss.rand.centered_float(20.0) + positions[j].x;
                        let y = ss.rand.centered_float(10.0) + positions[j].y;
                        let z = ss.rand.centered_float(20.0) + positions[j].z;
                        ss.func_8002836c(Vec3::new(x, y, z), vel1, accel1, color1, color2, 500, 10, 10);
                    }
                }
                for _ in 0..15 {
                    // @bug (game): the same.
                    let j = (ss.rand.zero_one() * (BOSSGOMA_LIMB_MAX - 1) as f32) as i16 as usize;
                    if positions[j].y < 10000.0 {
                        let x = ss.rand.centered_float(20.0) + positions[j].x;
                        let y = ss.rand.centered_float(10.0) + positions[j].y;
                        let z = ss.rand.centered_float(20.0) + positions[j].z;
                        let scale = (ss.rand.zero_one() * 5.0) as i16 + 10;
                        ss.hahen_spawn(Vec3::new(x, y, z), vel2, accel2, 0, scale, HAHEN_OBJECT_DEFAULT, 10, None);
                    }
                }
            });
        }
        let player = play.player;
        match self.action_state {
            0 => {
                self.action_state = 1;
                play.start_manual();
                Self::player_cs(play, PLAYER_CSACTION_1);
                self.sub_cam_id = play.create_sub_camera();
                play.change_camera_status(CAM_ID_MAIN, CAM_STAT_UNK3);
                play.change_camera_status(self.sub_cam_id, CAM_STAT_ACTIVE);
                self.sub_cam_eye = play.game_camera.eye;
                self.sub_cam_at = play.game_camera.at;
                let dx = self.sub_cam_eye.x - self.actor.world_pos.x;
                let dz = self.sub_cam_eye.z - self.actor.world_pos.z;
                self.defeated_camera_eye_dist = (dx * dx + dz * dz).sqrt();
                self.defeated_camera_eye_angle = f_atan2f(dx, dz);
                self.timer = 270;
            }
            1 => {
                let dx = sin_s(self.actor.shape_rot.y) * 100.0;
                let dz = cos_s(self.actor.shape_rot.y) * 100.0;
                let (tx, tz) = (self.actor.world_pos.x + dx, self.actor.world_pos.z + dz);
                if let Some(p) = player.and_then(|h| play.actors.actor_mut(h)) {
                    approach_f(&mut p.world_pos.x, tx, 0.5, 5.0);
                    approach_f(&mut p.world_pos.z, tz, 0.5, 5.0);
                }
                if self.frames_until_next_action < 1080 {
                    self.no_backface_culling = true;
                    for _ in 0..4 {
                        clear_pixels(play, self.actor.obj_bank_index, true, self.decaying_progress);
                        // @bug (game) (the C's `//!`): this lets decayingProgress reach 0x100,
                        // past sClearPixelTableFirstPass's end (`clear_pixel_table`).
                        if self.decaying_progress < 0x100 {
                            self.decaying_progress += 1;
                        }
                    }
                }
                if self.frames_until_next_action < 1070 && self.frame_count % 4 == 0 && play.rand.zero_one() < 0.5 {
                    self.blink_timer = 3;
                }
                self.defeated_camera_eye_angle += 0.022;
                approach_f(&mut self.defeated_camera_eye_dist, 150.0, 0.1, 5.0);
                let dx = self.defeated_camera_eye_angle.sin() * self.defeated_camera_eye_dist;
                let dz = self.defeated_camera_eye_angle.cos() * self.defeated_camera_eye_dist;
                let w = self.actor.world_pos;
                smooth_step_to_f(&mut self.sub_cam_eye.x, w.x + dx, 0.2, 50.0, 0.1);
                smooth_step_to_f(&mut self.sub_cam_eye.y, w.y + 20.0, 0.2, 50.0, 0.1);
                smooth_step_to_f(&mut self.sub_cam_eye.z, w.z + dz, 0.2, 50.0, 0.1);
                smooth_step_to_f(&mut self.sub_cam_at.x, self.first_tail_limb_world_pos.x, 0.2, 50.0, 0.1);
                smooth_step_to_f(&mut self.sub_cam_at.y, self.actor.focus_pos.y, 0.5, 100.0, 0.1);
                smooth_step_to_f(&mut self.sub_cam_at.z, self.first_tail_limb_world_pos.z, 0.2, 50.0, 0.1);
                if self.timer == 80 {
                    play.audio.queue_seq_cmd(start_seq(SEQ_PLAYER_BGM_MAIN, 0, NA_BGM_BOSS_CLEAR));
                }
                if self.timer == 0 {
                    self.action_state = 2;
                    play.change_camera_status(CAM_ID_MAIN, CAM_STAT_UNK3);
                    self.timer = 70;
                    self.decaying_progress = 0;
                    self.sub_cam_follow_speed = 0.0;
                    if let Err(e) = play.actor_spawn(ACTOR_ITEM_B_HEART, self.actor.world_pos, [0; 3], 0) {
                        log::warn!("Boss_Goma: the heart container not spawned: {e:?}");
                    }
                }
            }
            2 => {
                let (eye, at) = (play.game_camera.eye, play.game_camera.at);
                let s = self.sub_cam_follow_speed * 50.0;
                smooth_step_to_f(&mut self.sub_cam_eye.x, eye.x, 0.2, s, 0.1);
                smooth_step_to_f(&mut self.sub_cam_eye.y, eye.y, 0.2, s, 0.1);
                smooth_step_to_f(&mut self.sub_cam_eye.z, eye.z, 0.2, s, 0.1);
                smooth_step_to_f(&mut self.sub_cam_at.x, at.x, 0.2, s, 0.1);
                smooth_step_to_f(&mut self.sub_cam_at.y, at.y, 0.2, s, 0.1);
                smooth_step_to_f(&mut self.sub_cam_at.z, at.z, 0.2, s, 0.1);
                smooth_step_to_f(&mut self.sub_cam_follow_speed, 1.0, 1.0, 0.02, 0.0);
                if self.timer == 0 {
                    let mut child_pos = ROOM_CENTER;
                    self.timer = 30;
                    self.action_state = 3;
                    let player_pos = Self::player_pos(play);
                    for _ in 0..10000 {
                        if ((child_pos.x - player_pos.x).abs() < 100.0 && (child_pos.z - player_pos.z).abs() < 100.0)
                            || ((child_pos.x - self.actor.world_pos.x).abs() < 150.0 && (child_pos.z - self.actor.world_pos.z).abs() < 150.0)
                        {
                            child_pos.x = play.rand.centered_float(400.0) + -150.0;
                            child_pos.z = play.rand.centered_float(400.0) + -350.0;
                        } else {
                            break;
                        }
                    }
                    let pos = Vec3::new(child_pos.x, self.actor.world_pos.y, child_pos.z);
                    if let Err(e) = play.actor_spawn_as_child(&mut self.actor, ACTOR_DOOR_WARP1, pos, [0; 3], WARP_DUNGEON_CHILD) {
                        log::warn!("Boss_Goma: the blue warp not spawned: {e:?}");
                    }
                    let room = play.room_ctx.cur.num;
                    play.flags.set_clear(room);
                }
                for _ in 0..4 {
                    clear_pixels(play, self.actor.obj_bank_index, false, self.decaying_progress);
                    // @bug (game): the same as the first pass's.
                    if self.decaying_progress < 0x100 {
                        self.decaying_progress += 1;
                    }
                }
            }
            3 => {
                for _ in 0..4 {
                    clear_pixels(play, self.actor.obj_bank_index, false, self.decaying_progress);
                    // @bug (game): the same as the first pass's.
                    if self.decaying_progress < 0x100 {
                        self.decaying_progress += 1;
                    }
                }
                if self.timer == 0 {
                    if smooth_step_to_f(&mut self.actor.scale.y, 0.0, 1.0, 0.00075, 0.0) <= 0.001 {
                        self.return_to_main_cam(play);
                        play.stop_manual();
                        Self::player_cs(play, PLAYER_CSACTION_7);
                        self.actor.kill();
                    }
                    self.actor.scale.x = self.actor.scale.y;
                    self.actor.scale.z = self.actor.scale.y;
                }
            }
            _ => {}
        }
        if self.sub_cam_id != SUB_CAM_ID_DONE {
            play.camera_set_at_eye(self.sub_cam_id, self.sub_cam_at, self.sub_cam_eye);
        }
        let env = &mut play.env_ctx;
        if self.blink_timer != 0 {
            self.blink_timer -= 1;
            env.adj_ambient_color[0] += 40;
            env.adj_ambient_color[1] += 40;
            env.adj_ambient_color[2] += 80;
            env.adj_fog_color[0] += 10;
            env.adj_fog_color[1] += 10;
            env.adj_fog_color[2] += 20;
        } else {
            env.adj_ambient_color[0] -= 20;
            env.adj_ambient_color[1] -= 20;
            env.adj_ambient_color[2] -= 40;
            env.adj_fog_color[0] -= 5;
            env.adj_fog_color[1] -= 5;
            env.adj_fog_color[2] -= 10;
        }
        for (c, max) in env.adj_ambient_color.iter_mut().zip([200, 200, 200]) {
            *c = (*c).min(max);
        }
        for (c, max) in env.adj_fog_color.iter_mut().zip([70, 70, 140]) {
            *c = (*c).min(max);
        }
        for c in env.adj_ambient_color.iter_mut().chain(env.adj_fog_color.iter_mut()) {
            *c = (*c).max(0);
        }
    }

    /// `BossGoma_FloorAttackPosture`: turning to Link through frames 19⅓ to 30; at its end, the
    /// attack if he's within 250, else back to her walk. Her eye red, open to a seed.
    fn floor_attack_posture(&mut self, play: &mut PlayState) {
        self.skel.update();
        approach_zero_f(&mut self.actor.speed_xz, 0.5, 2.0);
        if self.skel.cur_frame >= 19.0 + 1.0 / 3.0 && self.skel.cur_frame <= 30.0 {
            let yaw = self.yaw_to_player(play);
            approach_s(&mut self.actor.world_rot.y, yaw, 3, 0xBB8);
        }
        if self.skel.on_frame(last(&self.anims.prepare_attack)) {
            if self.actor.xz_dist_to_player < 250.0 {
                self.setup_floor_prepare_attack();
            } else {
                self.setup_floor_main(play);
            }
        }
        self.eye_state = EYESTATE_IRIS_FOLLOW_NO_IFRAMES;
        self.visual_state = VISUALSTATE_RED;
    }

    /// `BossGoma_FloorPrepareAttack`: a frame, then the attack with `NA_SE_EN_GOMA_CRY1`.
    fn floor_prepare_attack(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.frames_until_next_action == 0 {
            self.setup_floor_attack();
            audio_play_actor_sfx2(play, NA_SE_EN_GOMA_CRY1);
        }
        self.eye_state = EYESTATE_IRIS_FOLLOW_NO_IFRAMES;
        self.visual_state = VISUALSTATE_RED;
    }

    /// `BossGoma_FloorAttack`: the lunge (her body's hits sound as Player's; a hit landing keeps
    /// her resting 10 frames; frame 10 the dust and a quake), the rest (30 to 59 frames), the
    /// recovery, then idle.
    fn floor_attack(&mut self, play: &mut PlayState) {
        self.actor.flags |= ACTOR_FLAG_SFX_FOR_PLAYER_BODY_HIT;
        self.skel.update();
        match self.action_state {
            0 => {
                if self.collider.elements.iter().any(|e| e.info.at_elem_flags & ATELEM_HIT != 0) {
                    self.frames_until_next_action = 10;
                }
                if self.skel.on_frame(10.0) {
                    self.play_effects_and_sfx(play, 3, 5);
                    play.actor_request_quake_and_rumble(5, 15);
                }
                if self.skel.on_frame(last(&self.anims.attack)) {
                    self.action_state = 1;
                    self.change(self.anims.rest_after_attack.clone(), 1.0, 0.0, last(&self.anims.rest_after_attack), ANIMMODE_LOOP, -1.0);
                    if self.frames_until_next_action == 0 {
                        self.timer = (play.rand.zero_one() * 30.0) as i16 + 30;
                    }
                }
            }
            1 => {
                if self.skel.on_frame(3.0) {
                    audio_play_actor_sfx2(play, NA_SE_EN_GOMA_UNARI2);
                }
                if self.timer == 0 {
                    self.action_state = 2;
                    self.change(self.anims.recover_after_attack.clone(), 1.0, 0.0, last(&self.anims.recover_after_attack), ANIMMODE_ONCE, -5.0);
                }
            }
            2 => {
                if self.skel.on_frame(last(&self.anims.recover_after_attack)) {
                    self.setup_floor_idle(play);
                }
            }
            _ => {}
        }
        self.eye_state = EYESTATE_IRIS_FOLLOW_NO_IFRAMES;
        self.visual_state = VISUALSTATE_RED;
    }

    /// `BossGoma_FloorDamaged`: the animation to its end, then stunned again, her patience gone.
    fn floor_damaged(&mut self) {
        self.skel.update();
        if self.skel.on_frame(last(&self.anims.damage)) {
            self.setup_floor_stunned();
            self.patience_timer = 0;
        }
        self.eye_state = EYESTATE_IRIS_NO_FOLLOW_NO_IFRAMES;
        approach_f(&mut self.eye_iris_scale_x, 0.4, 0.5, 0.2);
        self.visual_state = VISUALSTATE_HIT;
    }

    /// `BossGoma_FloorLandStruckDown`: knocked down from the ceiling: stunned for 150 frames, her
    /// patience gone; dust every frame until then.
    fn floor_land_struck_down(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.skel.on_frame(self.current_anim_frame_count) {
            self.setup_floor_stunned();
            self.sfx_faint_timer = 92;
            self.patience_timer = 0;
            self.frames_until_next_action = 150;
        }
        let (a, p) = (self.actor.clone(), self.actor.world_pos);
        actor_spawn_floor_dust_ring(play, &a, p, 55.0, 4, 8.0, 500, 10, true);
    }

    /// `BossGoma_FloorLand`: down after her larvae died: idle, her patience refilled.
    fn floor_land(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.skel.on_frame(self.current_anim_frame_count) {
            self.setup_floor_idle(play);
            self.patience_timer = 200;
        }
    }

    /// `BossGoma_FloorStunned`: the only action she takes damage in; at its end back to her walk
    /// (backing off for 20 frames from Link within 130 when impatient).
    fn floor_stunned(&mut self, play: &mut PlayState) {
        if self.sfx_faint_timer <= 90 {
            audio_play_actor_sfx2(play, NA_SE_EN_GOMA_FAINT - 0x800);
        }
        self.skel.update();
        if self.timer == 1 {
            let (a, p) = (self.actor.clone(), self.actor.world_pos);
            actor_spawn_floor_dust_ring(play, &a, p, 55.0, 4, 8.0, 500, 10, true);
        }
        approach_zero_f(&mut self.actor.speed_xz, 0.5, 1.0);
        if self.frames_until_next_action == 0 {
            self.setup_floor_main(play);
            if self.patience_timer == 0 && self.actor.xz_dist_to_player < 130.0 {
                self.timer = 20;
            }
        }
        approach_s(&mut self.actor.shape_rot.x, 0, 2, 0xBB8);
        self.eye_state = EYESTATE_IRIS_NO_FOLLOW_NO_IFRAMES;
        approach_f(&mut self.eye_iris_scale_x, 0.4, 0.5, 0.2);
        self.visual_state = VISUALSTATE_STUNNED;
    }

    /// `BossGoma_FallJump`: down to the floor after her larvae died; landing, dust, a quake.
    fn fall_jump(&mut self, play: &mut PlayState) {
        self.skel.update();
        approach_s(&mut self.actor.shape_rot.x, 0, 2, 0xBB8);
        let yaw = self.yaw_to_player(play);
        approach_s(&mut self.actor.world_rot.y, yaw, 2, 0x7D0);
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.setup_floor_land();
            self.actor.velocity.y = 0.0;
            self.play_effects_and_sfx(play, 0, 8);
            play.actor_request_quake_and_rumble(5, 15);
        }
    }

    /// `BossGoma_FallStruckDown`: knocked off the ceiling; landing, dust, a bigger quake,
    /// `NA_SE_EN_GOMA_DAM1`.
    fn fall_struck_down(&mut self, play: &mut PlayState) {
        self.skel.update();
        approach_s(&mut self.actor.shape_rot.x, 0, 2, 0xBB8);
        let yaw = self.yaw_to_player(play);
        approach_s(&mut self.actor.world_rot.y, yaw, 3, 0x7D0);
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.setup_floor_land_struck_down();
            self.actor.velocity.y = 0.0;
            self.play_effects_and_sfx(play, 0, 8);
            play.actor_request_quake_and_rumble(10, 15);
            audio_play_actor_sfx2(play, NA_SE_EN_GOMA_DAM1);
        }
    }

    /// `BossGoma_CeilingSpawnGohmas`: her tail swells limb by limb (24, 32, 40, 48), and as it
    /// ends an egg drops (`BossGoma_SpawnChildGohma`), three in all; uninterruptible.
    fn ceiling_spawn_gohmas(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.frame_count % 16 == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_GOMA_UNARI);
        }
        approach_zero_f(&mut self.actor.speed_xz, 0.5, 2.0);
        self.spawn_gohmas_action_timer += 1;
        match self.spawn_gohmas_action_timer {
            // BOSSGOMA_LIMB_TAIL1, the tail limb closest to the body.
            24 => self.tail_limbs_scale_timers[3] = 10,
            // BOSSGOMA_LIMB_TAIL2.
            32 => self.tail_limbs_scale_timers[2] = 10,
            // BOSSGOMA_LIMB_TAIL3.
            40 => self.tail_limbs_scale_timers[1] = 10,
            // BOSSGOMA_LIMB_TAIL4, the furthest from the body.
            48 => self.tail_limbs_scale_timers[0] = 10,
            _ => {}
        }
        if self.tail_limbs_scale_timers[0] == 2 {
            for i in 0..3 {
                if self.children_gohma_state[i] == 0 {
                    self.spawn_child_gohma(play, i as i16);
                    break;
                }
            }
            if self.children_gohma_state.contains(&0) {
                self.spawn_gohmas_action_timer = 23;
            }
        }
        if self.spawn_gohmas_action_timer >= 64 {
            self.setup_ceiling_idle(play);
        }
        self.eye_state = EYESTATE_IRIS_NO_FOLLOW_NO_IFRAMES;
    }

    /// `BossGoma_CeilingPrepareSpawnGohmas`: 70 frames, her eye red (a hit knocks her down).
    fn ceiling_prepare_spawn_gohmas(&mut self) {
        self.skel.update();
        if self.frames_until_next_action == 0 {
            self.setup_ceiling_spawn_gohmas();
        }
        self.eye_state = EYESTATE_IRIS_NO_FOLLOW_NO_IFRAMES;
        self.visual_state = VISUALSTATE_RED;
    }

    /// `BossGoma_FloorIdle`.
    fn floor_idle(&mut self, play: &mut PlayState) {
        self.skel.update();
        approach_zero_f(&mut self.actor.speed_xz, 0.5, 2.0);
        approach_s(&mut self.actor.shape_rot.x, 0, 2, 0xBB8);
        if self.frames_until_next_action == 0 {
            self.setup_floor_main(play);
        }
    }

    /// `BossGoma_CeilingIdle`: no eggs yet, she lays them; all their larvae dead, she jumps down;
    /// else (all laid) back to the centre.
    fn ceiling_idle(&mut self, play: &mut PlayState) {
        self.skel.update();
        approach_zero_f(&mut self.actor.speed_xz, 0.5, 2.0);
        if self.frames_until_next_action == 0 {
            let s = self.children_gohma_state;
            if s[0] == 0 && s[1] == 0 && s[2] == 0 {
                // No child has been spawned.
                self.setup_ceiling_prepare_spawn_gohmas();
            } else if s[0] < 0 && s[1] < 0 && s[2] < 0 {
                // All the children are dead.
                self.setup_fall_jump();
            } else {
                if s.contains(&0) {
                    // A child not spawned: the C's comment calls this unreachable, since
                    // CeilingSpawnGohmas spawns all three and can't be interrupted.
                    self.setup_ceiling_spawn_gohmas();
                    return;
                }
                // All the children spawned.
                self.setup_ceiling_move_to_center(play);
            }
        }
    }

    /// `BossGoma_FloorMain`: patient, towards Link (her posture within 150); else away from him
    /// (backwards while `timer` runs); a wall, she climbs it; idle when her time's up while
    /// patient. She doesn't move on frames 1, 15, 16 and 30 of her walk (her steps: dust).
    fn floor_main(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.skel.on_frame(1.0) || self.skel.on_frame(30.0) || self.skel.on_frame(15.0) || self.skel.on_frame(16.0) {
            self.do_not_move_this_frame = true;
        }
        if self.skel.on_frame(15.0) {
            self.play_effects_and_sfx(play, 1, 3);
        } else if self.skel.on_frame(30.0) {
            self.play_effects_and_sfx(play, 2, 3);
        }
        if self.frame_count % 64 == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_GOMA_CRY2);
        }
        if !self.do_not_move_this_frame {
            let mut rot = self.yaw_to_player(play);
            if self.patience_timer != 0 {
                self.patience_timer -= 1;
                if self.actor.xz_dist_to_player < 150.0 {
                    self.setup_floor_attack_posture();
                }
                approach_f(&mut self.actor.speed_xz, 10.0 / 3.0, 0.5, 2.0);
                approach_s(&mut self.actor.world_rot.y, rot, 5, 0x3E8);
            } else {
                if self.timer != 0 {
                    // Away from Link, walking backwards.
                    approach_f(&mut self.actor.speed_xz, -10.0, 0.5, 2.0);
                    self.skel.play_speed = -3.0;
                    if self.timer == 1 {
                        self.actor.speed_xz = 0.0;
                    }
                } else {
                    // Away from Link, walking forwards.
                    approach_f(&mut self.actor.speed_xz, 20.0 / 3.0, 0.5, 2.0);
                    self.skel.play_speed = 2.0;
                    rot = rot.wrapping_add(i16::MIN);
                }
                approach_s(&mut self.actor.world_rot.y, rot, 3, 0x9C4);
            }
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.actor.velocity.y = 0.0;
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
            self.setup_wall_climb();
        }
        if self.frames_until_next_action == 0 && self.patience_timer != 0 {
            self.setup_floor_idle(play);
        }
    }

    /// `BossGoma_WallClimb`: up to just below the ceiling (-320), then to its centre, her eggs
    /// allowed again.
    fn wall_climb(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.frame_count % 8 == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_GOMA_CLIM);
        }
        approach_f(&mut self.actor.velocity.y, 5.0, 0.5, 2.0);
        approach_s(&mut self.actor.shape_rot.x, -0x4000, 2, 0x7D0);
        approach_s(&mut self.actor.world_rot.y, self.actor.wall_yaw.wrapping_add(i16::MIN), 2, 0x5DC);
        // -320 is a bit below the boss room's ceiling.
        if self.actor.world_pos.y > -320.0 {
            self.setup_ceiling_move_to_center(play);
            // Allow new spawns.
            self.children_gohma_state = [0; 3];
        }
    }

    /// `BossGoma_CeilingMoveToCenter`: across the ceiling to within 100 of its centre (pushed off
    /// a wall she walks into), then idle.
    fn ceiling_move_to_center(&mut self, play: &mut PlayState) {
        self.update_ceiling_movement(play, 0.0, -5.0, true);
        if self.frame_count % 64 == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_GOMA_CRY2);
        }
        approach_s(&mut self.actor.shape_rot.x, i16::MIN, 3, 0x3E8);
        // Avoid walking into a wall?
        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
            let mut angle = self.actor.shape_rot.y.wrapping_add(i16::MIN);
            if angle < self.actor.wall_yaw {
                let abs_diff = self.actor.wall_yaw.wrapping_sub(angle);
                angle = angle.wrapping_add(abs_diff / 2);
            } else {
                let abs_diff = angle.wrapping_sub(self.actor.wall_yaw);
                angle = self.actor.wall_yaw.wrapping_add(abs_diff / 2);
            }
            let dz = cos_s(angle) * (5.0 + play.rand.zero_one() * 5.0) + play.rand.centered_float(2.0);
            self.actor.world_pos.z += dz;
            let dx = sin_s(angle) * (5.0 + play.rand.zero_one() * 5.0) + play.rand.centered_float(2.0);
            self.actor.world_pos.x += dx;
        }
        if self.frames_until_next_action == 0 && (-150.0 - self.actor.world_pos.x).abs() < 100.0 && (-350.0 - self.actor.world_pos.z).abs() < 100.0 {
            self.setup_ceiling_idle(play);
        }
    }

    /// `BossGoma_UpdateEye`: her eyelids (closed while `eyeClosedTimer` runs: Link's shot, at
    /// random, or while a child lives), her iris following Link, its scale back to 1.
    fn update_eye(&mut self, play: &mut PlayState) {
        if self.disable_gameplay_logic {
            return;
        }
        if self.eye_state == EYESTATE_IRIS_FOLLOW_BONUS_IFRAMES {
            // player + 0xA73 seems to be related to "throwing something" (the C's comment).
            if let Some(p) = play.player.and_then(|h| play.actors.downcast_mut::<crate::player::Player>(h))
                && p.unk_A73 != 0
            {
                p.unk_A73 = 0;
                self.eye_closed_timer = 12;
            }
            if self.frame_count % 16 == 0 && play.rand.zero_one() < 0.3 {
                self.eye_closed_timer = 7;
            }
        }
        if self.children_gohma_state.iter().any(|&s| s > 0) {
            self.eye_closed_timer = 7;
        }
        if self.eye_closed_timer != 0 {
            self.eye_closed_timer -= 1;
            // Close the eye.
            approach_s(&mut self.eye_lid_bottom_rot_x, -0xA98, 1, 0x7D0);
            approach_s(&mut self.eye_lid_top_rot_x, 0x1600, 1, 0x7D0);
        } else {
            // Open the eye.
            approach_s(&mut self.eye_lid_bottom_rot_x, 0, 1, 0x7D0);
            approach_s(&mut self.eye_lid_top_rot_x, 0, 1, 0x7D0);
        }
        if self.eye_state != EYESTATE_IRIS_NO_FOLLOW_NO_IFRAMES {
            let player_pos = Self::player_pos(play);
            let mut target_y = vec3f_yaw(self.actor.world_pos, player_pos).wrapping_sub(self.actor.shape_rot.y);
            let mut target_x = vec3f_pitch(self.actor.world_pos, player_pos).wrapping_sub(self.actor.shape_rot.x);
            if self.actor.shape_rot.x > 0x4000 || self.actor.shape_rot.x < -0x4000 {
                target_y = target_y.wrapping_add(i16::MIN).wrapping_neg();
                target_x = -0xBB8;
            }
            target_y = target_y.clamp(-0x1770, 0x1770);
            approach_s(&mut self.eye_iris_rot_y, target_y, 3, 0x7D0);
            approach_s(&mut self.eye_iris_rot_x, target_x, 3, 0x7D0);
        } else {
            approach_s(&mut self.eye_iris_rot_y, 0, 3, 0x3E8);
            approach_s(&mut self.eye_iris_rot_x, 0, 3, 0x3E8);
        }
        approach_f(&mut self.eye_iris_scale_x, 1.0, 0.2, 0.07);
        approach_f(&mut self.eye_iris_scale_y, 1.0, 0.2, 0.07);
    }

    /// `BossGoma_UpdateTailLimbsScale`: each tail limb swelling to 1.5 while its timer runs, back
    /// to 1 after. (`unusedTimer` counts every 128 frames up to 3.)
    fn update_tail_limbs_scale(&mut self) {
        if self.frame_count % 128 == 0 {
            self.unused_timer += 1;
            if self.unused_timer >= 3 {
                self.unused_timer = 0;
            }
        }
        for i in 0..4 {
            if self.tail_limbs_scale_timers[i] != 0 {
                self.tail_limbs_scale_timers[i] -= 1;
                approach_f(&mut self.tail_limbs_scale[i], 1.5, 0.2, 0.1);
            } else {
                approach_f(&mut self.tail_limbs_scale[i], 1.0, 0.2, 0.1);
            }
        }
    }

    /// `BossGoma_UpdateHit`: a hit on her eye while it's open (not while she lays): on the ceiling
    /// she's knocked down; stunned, a sword's damage (at no health she's defeated, the finishing
    /// blow); on the floor while patient, a seed or a nut stuns her (nut 40 frames, seed 90).
    fn update_hit(&mut self, play: &mut PlayState) {
        if self.invincibility_frames != 0 {
            self.invincibility_frames -= 1;
            return;
        }
        let eye = &mut self.collider.elements[0].info;
        let ac_hit_elem = eye.ac_hit_elem;
        if self.eye_closed_timer == 0 && self.action != Action::CeilingSpawnGohmas && eye.ac_elem_flags & ACELEM_HIT != 0 {
            eye.ac_elem_flags &= !ACELEM_HIT;
            // acHitElem->atDmgInfo.dmgFlags (a hit always sets it).
            let dmg_flags = ac_hit_elem.map(|e| e.at_dmg_info.dmg_flags).unwrap_or(0);
            if matches!(self.action, Action::CeilingMoveToCenter | Action::CeilingIdle | Action::CeilingPrepareSpawnGohmas) {
                self.setup_fall_struck_down();
                audio_play_actor_sfx2(play, NA_SE_EN_GOMA_DAM2);
            } else if self.action == Action::FloorStunned && collision_check_get_sword_damage(dmg_flags) != 0 {
                let damage = collision_check_get_sword_damage(dmg_flags);
                self.actor.col_chk_info.health = self.actor.col_chk_info.health.wrapping_sub(damage);
                if self.actor.col_chk_info.health as i8 > 0 {
                    audio_play_actor_sfx2(play, NA_SE_EN_GOMA_DAM1);
                    self.setup_floor_damaged();
                    let focus = self.actor.focus_pos;
                    play.with_ss(|ss| ss.sibuki_spawn_burst(focus));
                } else {
                    self.setup_defeated(play);
                    enemy_start_finishing_blow(play, &self.actor);
                }
                self.invincibility_frames = 10;
            } else if self.action != Action::FloorStunned && self.patience_timer != 0 && dmg_flags & (DMG_SLINGSHOT | DMG_DEKU_NUT) != 0 {
                audio_play_actor_sfx2(play, NA_SE_EN_GOMA_DAM2);
                play.audio.stop_sfx_by_id(NA_SE_EN_GOMA_CRY1);
                self.invincibility_frames = 10;
                self.setup_floor_stunned();
                self.sfx_faint_timer = 100;
                self.frames_until_next_action = if dmg_flags & DMG_DEKU_NUT != 0 { 40 } else { 90 };
                self.timer = 4;
                play.actor_request_quake_and_rumble(4, 12);
            }
        }
    }

    /// `BossGoma_UpdateMainEnvColor`: grey every other 16 frames when well, flashing while
    /// invincible, else halfway to her state's colour.
    fn update_main_env_color(&mut self) {
        let v = self.visual_state as usize;
        if self.visual_state == VISUALSTATE_DEFAULT && self.frame_count & 0x10 != 0 {
            for c in &mut self.main_env_color {
                approach_f(c, 50.0, 0.5, 20.0);
            }
        } else if self.invincibility_frames != 0 {
            self.main_env_color = if self.invincibility_frames & 2 != 0 { COLORS2[v] } else { COLORS1[v] };
        } else {
            for (c, t) in self.main_env_color.iter_mut().zip(COLORS1[v]) {
                approach_f(c, t, 0.5, 20.0);
            }
        }
    }

    /// `BossGoma_UpdateEyeEnvColor`.
    fn update_eye_env_color(&mut self) {
        let v = self.visual_state as usize;
        for (c, t) in self.eye_env_color.iter_mut().zip(TARGET_EYE_ENV_COLORS[v]) {
            approach_f(c, t, 0.5, 20.0);
        }
    }

    /// `BossGoma_SpawnChildGohma`: egg `i` from her tail's end, 50 lower, a third of a turn apart.
    fn spawn_child_gohma(&mut self, play: &mut PlayState, i: i16) {
        let p = self.last_tail_limb_world_pos;
        let yaw = (i as i32 * (0x10000 / 3)) as i16;
        if let Err(e) = play.actor_spawn_as_child(&mut self.actor, ACTOR_EN_GOMA, Vec3::new(p.x, p.y - 50.0, p.z), [0, yaw, 0], i) {
            log::warn!("Boss_Goma: egg {i} not spawned: {e:?}");
        }
        self.children_gohma_state[i as usize] = 1;
    }

    /// Whether `BossGoma_OverrideLimbDraw` hides her eye and iris (`*dList = NULL`): her eye
    /// closed, in its default state.
    fn eye_hidden(&self) -> bool {
        self.eye_state == EYESTATE_IRIS_FOLLOW_BONUS_IFRAMES && self.eye_lid_bottom_rot_x < -0xA8C
    }

    /// `Actor_Draw`'s model matrix, then `BossGoma_Draw`'s `Matrix_Translate(0, -4000, 0)`.
    fn model_mtx(&self) -> MtxF {
        let a = &self.actor;
        let mut m = MtxF::set_translate_rotate_yxz(a.world_pos.x, a.world_pos.y + a.shape_y_offset * a.scale.y, a.world_pos.z, [a.shape_rot.x, a.shape_rot.y, a.shape_rot.z]);
        m.scale(a.scale.x, a.scale.y, a.scale.z);
        m.translate(0.0, -4000.0, 0.0);
        m
    }

    /// `BossGoma_OverrideLimbDraw`'s change to limb `limb`'s rotation: the eyelids' and the iris's.
    fn limb_rot(&self, limb: usize, rot: &mut [i16; 3]) {
        match limb {
            BOSSGOMA_LIMB_EYE_LID_BOTTOM_ROOT2 => rot[0] = rot[0].wrapping_add(self.eye_lid_bottom_rot_x),
            BOSSGOMA_LIMB_EYE_LID_TOP_ROOT2 => rot[0] = rot[0].wrapping_add(self.eye_lid_top_rot_x),
            BOSSGOMA_LIMB_IRIS_ROOT2 => {
                rot[0] = rot[0].wrapping_add(self.eye_iris_rot_x);
                rot[1] = rot[1].wrapping_add(self.eye_iris_rot_y);
            }
            _ => {}
        }
    }

    /// `BossGoma_PostLimbDraw` at limb `limb` (its matrix `mtx`, its list `dlist` as the override
    /// left it): her tail's, eye's and claws' points, the dead limbs' points, a limb breaking off
    /// (an `En_Goma` piece), the collider's spheres.
    fn post_limb_draw(&mut self, play: &mut PlayState, limb: usize, mtx: &MtxF, dlist: Option<&'static str>) {
        match limb {
            // The tail's end.
            BOSSGOMA_LIMB_TAIL4 => self.last_tail_limb_world_pos = mtx.mult_vec3f(Vec3::ZERO),
            // The tail's start.
            BOSSGOMA_LIMB_TAIL1 => self.first_tail_limb_world_pos = mtx.mult_vec3f(Vec3::ZERO),
            BOSSGOMA_LIMB_EYE => self.actor.focus_pos = mtx.mult_vec3f(FOCUS_EYE_LOCAL_POS),
            BOSSGOMA_LIMB_R_FEET_BACK => self.right_hand_back_limb_world_pos = mtx.mult_vec3f(Vec3::ZERO),
            BOSSGOMA_LIMB_L_FEET_BACK => self.left_hand_back_limb_world_pos = mtx.mult_vec3f(Vec3::ZERO),
            _ => {}
        }
        if self.visual_state == VISUALSTATE_DEFEATED {
            if dlist.is_some() {
                self.defeated_limb_positions[limb] = mtx.mult_vec3f(Vec3::ZERO);
            } else {
                self.defeated_limb_positions[limb].y = 10000.0;
            }
        }
        if self.dead_limbs_state[limb] == 1 {
            self.dead_limbs_state[limb] = 2;
            let child_pos = mtx.mult_vec3f(Vec3::ZERO);
            let child_rot = mtx.to_yxz_rot_s(false);
            // The pieces of Gohma as she falls apart: the same actor as her larvae.
            let params = S_DEAD_LIMB_LIFETIME[limb] as i16 + 100;
            match play.actor_spawn_as_child(&mut self.actor, ACTOR_EN_GOMA, child_pos, child_rot, params) {
                Ok(h) => {
                    let bank = self.actor.obj_bank_index;
                    if let Some(piece) = play.actors.downcast_mut::<EnGoma>(h) {
                        piece.boss_limb_dl = dlist.map(|s| (OBJECT, s));
                        piece.actor.obj_bank_index = bank;
                    }
                }
                Err(e) => log::debug!("Boss_Goma: limb {limb}'s piece not spawned: {e:?}"),
            }
        }
        self.collider.update_spheres(limb as u8, &mtx.to_mat4());
    }
}

/// The segment of her env colour in her bakes (`gDPSetEnvColor` before every limb).
const SEG_ENV: u8 = 0x0B;
/// Segment 8: `BossGoma_EmptyDlist` or `BossGoma_NoBackfaceCullingDlist`.
const SEG_08: u8 = 0x08;
pub const BAKE_SKEL: &str = "Boss_Goma/skel";
pub const BAKE_EYE: &str = "Boss_Goma/eye";
pub const BAKE_IRIS: &str = "Boss_Goma/iris";

/// A bake's name for segment 8's list: back faces culled or drawn.
pub fn bake_name(base: &str, no_backface_culling: bool) -> String {
    format!("{base}{}", if no_backface_culling { "/no_cull" } else { "" })
}

/// Segment 8's list: `BossGoma_EmptyDlist` (just its end), or `BossGoma_NoBackfaceCullingDlist`
/// (`G_RM_AA_ZB_TEX_EDGE2`'s cutout, back faces drawn).
fn segment_08(no_backface_culling: bool) -> BakeSegment {
    use oot_game::gbi::*;
    let mut d = Dl::default();
    if no_backface_culling {
        /// `G_RM_AA_ZB_TEX_EDGE2` (`gbi.h`).
        const G_RM_AA_ZB_TEX_EDGE2: u32 = 0x0044_3078;
        d.pipe_sync();
        d.render_mode(G_RM_PASS, G_RM_AA_ZB_TEX_EDGE2);
        d.clear_geometry_mode(G_CULL_BACK);
    }
    d.end();
    BakeSegment::Commands(d.0)
}

/// `BossGoma_Draw`'s meshes (after `Gfx_SetupDL_25Opa`), each with segment 8 culling back faces
/// or not:
/// - `gGohmaSkel` with her env colour (`mainEnvColor`, set before every limb) dynamic, her eye
///   and iris left out;
/// - her eye (`gGohmaEyeDL`) and iris (`gGohmaIrisDL`) alone, with their own env colours.
///
/// And her limb lists as `En_Goma`'s pieces (`en_goma::boss_limb_bake`), the ones that break off.
pub fn bakes() -> Vec<MeshBake> {
    let env = || (SEG_ENV, BakeSegment::DynamicColor { env: true, prim: false });
    let mut v = Vec::new();
    for no_cull in [false, true] {
        v.push(MeshBake {
            name: bake_name(BAKE_SKEL, no_cull),
            object: OBJECT.into(),
            segments: vec![env(), (SEG_08, segment_08(no_cull))],
            prelude: vec![SEG_ENV],
            body: BakeBody::Skeleton {
                file: OBJECT.into(),
                symbol: SKELETON.into(),
                limbs: [BOSSGOMA_LIMB_EYE, BOSSGOMA_LIMB_IRIS].iter().map(|&l| LimbOverride { limb: (l - 1) as u8, file: OBJECT.into(), symbol: String::new() }).collect(),
            },
        });
        for (base, limb) in [(BAKE_EYE, BOSSGOMA_LIMB_EYE), (BAKE_IRIS, BOSSGOMA_LIMB_IRIS)] {
            v.push(MeshBake {
                name: bake_name(base, no_cull),
                object: OBJECT.into(),
                segments: vec![env(), (SEG_08, segment_08(no_cull))],
                prelude: vec![SEG_ENV],
                body: BakeBody::DLists(vec![(OBJECT.into(), limb_dlist(limb).unwrap_or_default().into())]),
            });
        }
    }
    for (limb, symbol) in LIMB_DLISTS {
        if S_DEAD_LIMB_LIFETIME[limb] != 0 {
            v.push(crate::en_goma::boss_limb_bake(OBJECT, symbol));
        }
    }
    v
}

/// `render_state`'s layout.
/// switches: flags (`noBackfaceCulling` 1, the eye drawn 2), the main env colour, the eye's
/// (`0xRRGGBB`), the iris grey (1) or white (0), the broken-off limbs (three words of bits);
/// angles: the bottom eyelid, the top eyelid, the iris's x and y; values: the tail's four scales,
/// the iris's x and y scale.
mod rs {
    pub const FLAG_NO_CULL: u32 = 1;
    pub const FLAG_EYE_DRAWN: u32 = 2;
}

/// `0xRRGGBB` of a colour as the C casts it (`(s16)` of each float channel).
fn pack_rgb(c: [f32; 3]) -> u32 {
    let ch = |f: f32| f as i16 as u8 as u32;
    (ch(c[0]) << 16) | (ch(c[1]) << 8) | ch(c[2])
}

fn unpack_rgb(v: u32, a: u8) -> [u8; 4] {
    [(v >> 16) as u8, (v >> 8) as u8, v as u8, a]
}

impl ActorImpl for BossGoma {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BossGoma_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.visual_state = VISUALSTATE_DEFAULT;
        self.frame_count = self.frame_count.wrapping_add(1);
        if self.frames_until_next_action != 0 {
            self.frames_until_next_action -= 1;
        }
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.sfx_faint_timer != 0 {
            self.sfx_faint_timer -= 1;
        }
        self.eye_state = EYESTATE_IRIS_FOLLOW_BONUS_IFRAMES;
        match self.action {
            Action::Encounter => self.encounter(play),
            Action::Defeated => self.defeated(play),
            Action::FloorAttackPosture => self.floor_attack_posture(play),
            Action::FloorPrepareAttack => self.floor_prepare_attack(play),
            Action::FloorAttack => self.floor_attack(play),
            Action::FloorDamaged => self.floor_damaged(),
            Action::FloorLandStruckDown => self.floor_land_struck_down(play),
            Action::FloorLand => self.floor_land(play),
            Action::FloorStunned => self.floor_stunned(play),
            Action::FallJump => self.fall_jump(play),
            Action::FallStruckDown => self.fall_struck_down(play),
            Action::CeilingSpawnGohmas => self.ceiling_spawn_gohmas(play),
            Action::CeilingPrepareSpawnGohmas => self.ceiling_prepare_spawn_gohmas(),
            Action::FloorIdle => self.floor_idle(play),
            Action::CeilingIdle => self.ceiling_idle(play),
            Action::FloorMain => self.floor_main(play),
            Action::WallClimb => self.wall_climb(play),
            Action::CeilingMoveToCenter => self.ceiling_move_to_center(play),
        }
        self.actor.shape_rot.y = self.actor.world_rot.y;
        if !self.do_not_move_this_frame {
            self.actor.move_forward();
        } else {
            self.do_not_move_this_frame = false;
        }
        if self.actor.world_pos.y < -400.0 {
            self.actor.update_bg_check_info(&play.col, 30.0, 30.0, 80.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
        } else {
            self.actor.update_bg_check_info(&play.col, 0.0, 30.0, 80.0, UPDBGCHECKINFO_FLAG_0);
        }
        self.update_eye(play);
        self.update_main_env_color();
        self.update_eye_env_color();
        self.update_tail_limbs_scale();
        if self.disable_gameplay_logic {
            return;
        }
        self.update_hit(play);
        play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        if self.action != Action::FloorStunned && self.action != Action::FloorDamaged && (self.action != Action::FloorMain || self.timer == 0) {
            play.collision_check_set_at(&self.actor, 0, &mut self.collider);
        }
    }

    /// `BossGoma_Draw`'s effects on the actor and the game, once per game frame, in
    /// `SkelAnime_DrawOpa`'s limb order: `BossGoma_OverrideLimbDraw`'s random eye colour while she's
    /// invincible (three `Rand_ZeroOne() × 255`, red, green, blue), and `BossGoma_PostLimbDraw`'s
    /// points, dead limbs, broken-off pieces and spheres.
    fn draw_update(&mut self, play: &mut PlayState) {
        self.eye_rand_env = None;
        if self.actor.killed {
            return;
        }
        let Some(skeleton) = self.skeleton.clone() else { return };
        let model = self.model_mtx();
        let joints = self.skel.joint_table.clone();
        let mut limb_mtx = vec![MtxF::IDENTITY; skeleton.limbs.len()];
        for &l in &skeleton.draw_order {
            let l = l as usize;
            let limb = l + 1;
            let parent = skeleton.parents[l].map(|p| limb_mtx[p as usize]).unwrap_or(model);
            // OverrideLimbDraw: the list NULLed for a dead limb, the eye and iris when closed.
            let mut dlist = limb_dlist(limb);
            if self.dead_limbs_state[limb] >= 2 {
                dlist = None;
            }
            if limb == BOSSGOMA_LIMB_EYE {
                if self.eye_hidden() {
                    dlist = None;
                } else if self.invincibility_frames != 0 {
                    let r = (play.rand.zero_one() * 255.0) as i16 as u8;
                    let g = (play.rand.zero_one() * 255.0) as i16 as u8;
                    let b = (play.rand.zero_one() * 255.0) as i16 as u8;
                    self.eye_rand_env = Some([r, g, b]);
                }
            }
            if limb == BOSSGOMA_LIMB_IRIS && self.eye_hidden() {
                dlist = None;
            }
            let pos = if l == 0 {
                let r = joints.first().copied().unwrap_or([0; 3]);
                Vec3::new(r[0] as f32, r[1] as f32, r[2] as f32)
            } else {
                let p = skeleton.limbs[l].joint_pos;
                Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
            };
            let mut rot = joints.get(l + 1).copied().unwrap_or([0; 3]);
            self.limb_rot(limb, &mut rot);
            let mut m = parent;
            m.translate_rotate_zyx(pos, rot);
            limb_mtx[l] = m;
            self.post_limb_draw(play, limb, &m, dlist);
        }
    }

    fn render_state(&self) -> RenderState {
        let mut r = RenderState::of(&self.actor);
        r.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        let mut flags = 0;
        if self.no_backface_culling {
            flags |= rs::FLAG_NO_CULL;
        }
        if !self.eye_hidden() {
            flags |= rs::FLAG_EYE_DRAWN;
        }
        let eye_env = match self.eye_rand_env {
            Some(c) => ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | c[2] as u32,
            None => pack_rgb(self.eye_env_color),
        };
        let mut dead = [0u32; 3];
        for (limb, s) in self.dead_limbs_state.iter().enumerate().take(96) {
            if *s >= 2 {
                dead[limb / 32] |= 1 << (limb % 32);
            }
        }
        r.switches = vec![flags, pack_rgb(self.main_env_color), eye_env, (self.visual_state == VISUALSTATE_DEFEATED) as u32, dead[0], dead[1], dead[2]];
        r.angles = vec![self.eye_lid_bottom_rot_x, self.eye_lid_top_rot_x, self.eye_iris_rot_x, self.eye_iris_rot_y];
        r.values = vec![self.tail_limbs_scale[0], self.tail_limbs_scale[1], self.tail_limbs_scale[2], self.tail_limbs_scale[3], self.eye_iris_scale_x, self.eye_iris_scale_y];
        r
    }

    /// `BossGoma_Draw`: her skeleton at `Actor_Draw`'s matrix moved 4000 down, every limb in her
    /// env colour, her eyelids and iris turned, her tail limbs and iris scaled (after their own
    /// transform, so not their children), broken-off limbs hidden; her eye (random colours while
    /// invincible, alpha 63) and iris (white, grey when defeated) apart; her textures from her
    /// object's RAM once she's started to decay.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(joints), [flags, main_env, eye_env, iris_grey, dead @ ..], [lid_bottom, lid_top, iris_x, iris_y], v) =
            (&rs.joints, rs.switches.as_slice(), rs.angles.as_slice(), rs.values.as_slice())
        else {
            return;
        };
        let Some(skeleton) = &self.skeleton else { return };
        if v.len() < 6 || dead.len() < 3 {
            return;
        }
        let model = oot_game::play::actor_draw_matrix(rs) * Mat4::from_translation(Vec3::new(0.0, -4000.0, 0.0));
        let mut bones = skeleton.pose_override(&joints.rot, |limb, _pos, rot| {
            match limb {
                BOSSGOMA_LIMB_EYE_LID_BOTTOM_ROOT2 => rot[0] = rot[0].wrapping_add(*lid_bottom),
                BOSSGOMA_LIMB_EYE_LID_TOP_ROOT2 => rot[0] = rot[0].wrapping_add(*lid_top),
                BOSSGOMA_LIMB_IRIS_ROOT2 => {
                    rot[0] = rot[0].wrapping_add(*iris_x);
                    rot[1] = rot[1].wrapping_add(*iris_y);
                }
                _ => {}
            }
            Mat4::IDENTITY
        });
        // The tail's limbs scaled inside their Push/Pop: their own vertices only.
        for (i, limb) in [BOSSGOMA_LIMB_TAIL4, BOSSGOMA_LIMB_TAIL3, BOSSGOMA_LIMB_TAIL2, BOSSGOMA_LIMB_TAIL1].into_iter().enumerate() {
            if let Some(b) = bones.get_mut(limb - 1) {
                *b *= Mat4::from_scale(Vec3::splat(v[i]));
            }
        }
        let is_dead = |limb: usize| dead[limb / 32] & (1 << (limb % 32)) != 0;
        for (l, b) in bones.iter_mut().enumerate() {
            if is_dead(l + 1) {
                *b = Mat4::ZERO;
            }
        }
        let no_cull = *flags & rs::FLAG_NO_CULL != 0;
        let images = decay_images(play, self.actor.obj_bank_index);
        let params = |env: [u8; 4]| {
            let mut sv = SegmentValues::default();
            sv.env[SEG_ENV as usize] = Some(env);
            DrawParams { segments: Some(sv), texture_images: images.clone(), ..Default::default() }
        };
        let eye_m = model * bones[BOSSGOMA_LIMB_EYE - 1];
        let iris_m = model * bones[BOSSGOMA_LIMB_IRIS - 1] * Mat4::from_scale(Vec3::new(v[4], v[5], 1.0));
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(BAKE_SKEL, no_cull))), transform: model, bones, params: params(unpack_rgb(*main_env, 255)) });
        if *flags & rs::FLAG_EYE_DRAWN != 0 {
            if !is_dead(BOSSGOMA_LIMB_EYE) {
                out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(BAKE_EYE, no_cull))), transform: eye_m, bones: Vec::new(), params: params(unpack_rgb(*eye_env, 63)) });
            }
            if !is_dead(BOSSGOMA_LIMB_IRIS) {
                let iris_env = if *iris_grey != 0 { [50, 50, 50, 255] } else { [255, 255, 255, 255] };
                out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(BAKE_IRIS, no_cull))), transform: iris_m, bones: Vec::new(), params: params(iris_env) });
            }
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
