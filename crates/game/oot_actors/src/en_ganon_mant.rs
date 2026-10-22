//! `En_Ganon_Mant` (`ovl_En_Ganon_Mant/z_en_ganon_mant.c`): Ganondorf's cape, a cloth of twelve
//! strands of twelve joints hanging from a collar between two points its parent writes every
//! frame. `Boss_Ganon` (params 1, not ported) and `En_Viewer`'s Ganondorfs (types 3, 5, 7, 8 and
//! 9, params 35) spawn it as their child; the parent writes its public fields (the forearms'
//! and shoulders' positions, `backPush`, the sway magnitudes, `minY`, `minDist`, `gravity`, the
//! attach timers, `tearTimer`) and the cape does the rest.
//!
//! - **Update** (`EnGanonMant_Update`): marks that it ran (`updateHasRun`), counts `frameTimer`,
//!   counts the three attach timers down to 0, takes its parent's shape yaw (the C casts the
//!   parent to `BossGanon` but reads only `actor.shape.rot.y`, the same for `En_Viewer`), and
//!   while `tearTimer` runs tears the cloth once a frame (`EnGanonMant_Tear`): a random line
//!   through the texture (`sTearShapes`: 10, 10, 15 or 7 steps of 1 texel, squares of 1 to 3
//!   texels) has its texels zeroed, and four random joints are marked `torn` (read by nothing).
//! - **Draw** (`EnGanonMant_Draw`), only when update has run since the last draw: the collar's
//!   endpoints by the timers (the right forearm and left shoulder while `attachRightArmTimer`
//!   runs, the right shoulder and left forearm while `attachLeftArmTimer` does, gravity -13;
//!   both shoulders while `attachShouldersTimer` does, gravity -3; else both forearms, gravity
//!   as the parent left it); the twelve roots on the half circle between them (strand 0 at the
//!   right one, 11 at the left), each strand's joints moved (`EnGanonMant_UpdateStrand`: gravity,
//!   the push back and the sways by `sBackSwayCoefficients` and `sSideSwayCoefficients`, joints
//!   9.5 apart, kept `minDist` × `sDistMultipliers` from the actor's position across and above
//!   `minY`, their speeds kept to 5 a frame each way); then the vertices written
//!   (`EnGanonMant_UpdateVertices`) into this frame's copy of the double-buffered `gMant1Vtx` /
//!   `gMant2Vtx` (`frameTimer` odd: 1), through `sVerticesMap` (strand `i`'s joint `j` is
//!   vertex `12 i + 11 - j`), the normals (0, 30, 0) turned by the joints' rotations. It draws
//!   `gMantMaterialDL` then `gMantDL` with segment 0x0C on that copy, on OPA, with the identity
//!   matrix (world coordinates).
//! - **Params 0x23** (`En_Viewer`'s): every joint pushed 30 back (away from the parent's facing)
//!   before each strand's update, joints 6.5 apart, and no speed limit.
//!
//! The whole overlay is ported. Its texture (`gMantTex`, in the overlay) is torn in a copy of
//! this instance's (the C writes the overlay's own, shared by every cape until the overlay is
//! unloaded); the draw passes the copy (`DrawParams::texture_images`) once torn. The vertices
//! the C writes are kept (`vertices`, as `Vtx`'s `ob` and `n`), and drawn through bakes whose
//! every vertex follows a bone of its own (`BakeSegment::BonedVertices`, from the file's
//! vertices): each bone moves its vertex from the file's place to the written one and turns its
//! normal (the file's (0, 120, 0)) to the written one. `gMant2Vtx` holds the same as
//! `gMant1Vtx` in the ROM; there's a bake for each. Before the first write a copy draws as the
//! file has it (a flat grid 11000 across at the origin).
//!
//! With `En_Viewer`'s type 3 (the nightmare, Zelda's escape) nothing is ever written: the
//! parent's `EnViewer_UpdateGanondorfCape` returns unless the type is 5. Every field stays at its
//! zeroed init: the endpoints at the world's origin, so every root is there; gravity 0 and the
//! floor (`minY`) at 0, the params-0x23 push lays every strand's joints in a line back from the
//! origin along y 0, and with no sway the twelve strands are one line: a cape of no width,
//! nothing to see. Ported as written.
//!
//! Params: `params` whole (`this->actor.params == 0x23`): 0x23 (35, `En_Viewer`'s) the push back
//! and 6.5 joints without a speed limit; anything else (`Boss_Ganon`'s 1) 9.5 joints, speeds
//! within 5.
//! Flags: none.

use std::sync::Arc;

use eng_gfx::{DrawCmd, DrawImage, DrawParams, MeshKey, SourceImage};
use glam::{Mat4, Quat, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_BOSS, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, Rand, RenderState, ViewInfo};
use oot_game::sys_matrix::MtxF;

/// `ACTOR_EN_GANON_MANT` (`actor_table.h`: 0x016F).
pub const ACTOR_EN_GANON_MANT: i16 = 0x016F;
/// The overlay, which holds the cape's texture, vertices and lists.
pub const OVERLAY: &str = "ovl_En_Ganon_Mant";

/// `En_Ganon_Mant_Profile`: `ACTORCAT_BOSS`, always updating and drawn, `OBJECT_GAMEPLAY_KEEP`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_GANON_MANT,
    name: "En_Ganon_Mant",
    category: ACTORCAT_BOSS,
    flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED,
    object: "gameplay_keep",
    schema: &SCHEMA,
};

/// `this->actor.params == 0x23`: the cape `En_Viewer` spawns.
pub const PARAMS_VIEWER: i16 = 0x23;
/// `Boss_Ganon`'s (`Actor_SpawnAsChild(.., ACTOR_EN_GANON_MANT, .., 1)`).
pub const PARAMS_BOSS: i16 = 1;

/// `params`, compared whole with 0x23.
pub const PARAMS: Field = Field::whole(
    "params",
    Kind::Named(&[(PARAMS_BOSS as i32, "Boss_Ganon's cape: joints 9.5 apart, speeds within 5 a frame"), (PARAMS_VIEWER as i32, "En_Viewer's cape: pushed 30 back every frame, joints 6.5 apart, speeds unlimited")]),
    "0x23 (En_Viewer's) or anything else (Boss_Ganon's 1); set by the parent that spawns it",
);

/// En_Ganon_Mant's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[PARAMS], flags: &[], todo: false, objects: &[], texts: &[] };

/// `GANON_MANT_NUM_JOINTS`, `GANON_MANT_NUM_STRANDS` (`z_en_ganon_mant.h`).
pub const NUM_JOINTS: usize = 12;
pub const NUM_STRANDS: usize = 12;
/// The vertices of each copy (`gMant1Vtx`, `gMant2Vtx`: 144).
pub const NUM_VERTICES: usize = NUM_STRANDS * NUM_JOINTS;

/// `gMantTex_WIDTH`, `gMantTex_HEIGHT` (RGBA16).
pub const TEX_WIDTH: usize = 32;
pub const TEX_HEIGHT: usize = 64;
/// `gMantTex` where `gMantMaterialDL` loads it from: the overlay's link address 0x80A23D60
/// (`segments.csv`) plus its assets' start 0x1218 (`config.yml`) plus the XML's offset 0.
pub const MANT_TEX_ADDR: u32 = 0x80A2_3D60 + 0x1218;

/// `sTearSizesMedium`, `sTearSizesLarge`, `sTearSizesSmall`.
const TEAR_SIZES_MEDIUM: [i16; 10] = [0, 0, 0, 1, 1, 1, 1, 1, 0, 0];
const TEAR_SIZES_LARGE: [i16; 15] = [0, 0, 0, 0, 1, 1, 2, 2, 2, 1, 1, 0, 0, 0, 0];
const TEAR_SIZES_SMALL: [i16; 7] = [0, 0, 0, 0, 0, 0, 0];
/// `sTearShapes`: how many texels each step of a tear zeroes (a square `size + 1` across).
pub const TEAR_SHAPES: [&[i16]; 4] = [&TEAR_SIZES_MEDIUM, &TEAR_SIZES_MEDIUM, &TEAR_SIZES_LARGE, &TEAR_SIZES_SMALL];

/// `sBackSwayCoefficients`: how much each joint takes of the push back and its sway.
pub const BACK_SWAY_COEFFICIENTS: [f32; NUM_JOINTS] = [0.0, 1.0, 0.5, 0.25, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
/// `sSideSwayCoefficients`: how much each joint takes of the sideways sway.
pub const SIDE_SWAY_COEFFICIENTS: [f32; NUM_JOINTS] = [0.0, 1.0, 0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1, 0.0];
/// `sDistMultipliers`: each joint's least distance across from the actor, times `minDist`.
pub const DIST_MULTIPLIERS: [f32; NUM_JOINTS] = [0.0, 1.0, 1.0, 1.0, 1.0, 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7];

/// `MAP_STRAND_TO_VTX(n)`: `n + 12 k` for `k` 0 to 11.
const fn map_strand_to_vtx(map: &mut [u16; NUM_VERTICES], at: usize, n: u16) {
    let mut k = 0;
    while k < NUM_JOINTS {
        map[at + k] = n + (NUM_JOINTS * k) as u16;
        k += 1;
    }
}

/// `sVerticesMap`: `MAP_STRAND_TO_VTX(11)` to `MAP_STRAND_TO_VTX(0)`.
pub const VERTICES_MAP: [u16; NUM_VERTICES] = {
    let mut map = [0u16; NUM_VERTICES];
    let mut a = 0;
    while a < NUM_STRANDS {
        map_strand_to_vtx(&mut map, a * NUM_JOINTS, (NUM_STRANDS - 1 - a) as u16);
        a += 1;
    }
    map
};

/// The bakes: `gMantMaterialDL` then `gMantDL` with segment 0x0C on `gMant1Vtx` or `gMant2Vtx`,
/// each vertex on its own bone.
pub const BAKES: [&str; 2] = ["En_Ganon_Mant/cape1", "En_Ganon_Mant/cape2"];
const VTX_SYMBOLS: [&str; 2] = ["gMant1Vtx", "gMant2Vtx"];
/// `gSPSegment(POLY_OPA_DISP++, 0x0C, gMant1Vtx / gMant2Vtx)`.
const SEG_VTX: u8 = 0x0C;

pub fn bakes() -> Vec<MeshBake> {
    BAKES
        .iter()
        .zip(VTX_SYMBOLS)
        .map(|(&name, vtx)| MeshBake {
            name: name.into(),
            object: "gameplay_keep".into(),
            segments: vec![(SEG_VTX, BakeSegment::BonedVertices { file: OVERLAY.into(), symbol: vtx.into(), bones: (0..NUM_VERTICES as u16).collect() })],
            prelude: Vec::new(),
            body: BakeBody::DLists(vec![(OVERLAY.into(), "gMantMaterialDL".into()), (OVERLAY.into(), "gMantDL".into())]),
        })
        .collect()
}

/// `MantStrand`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MantStrand {
    /// `root`: its place on the collar.
    pub root: Vec3,
    pub joints: [Vec3; NUM_JOINTS],
    /// `rotations`: x (pitch) and y (yaw) in radians, the normals' turn.
    pub rotations: [Vec3; NUM_JOINTS],
    pub velocities: [Vec3; NUM_JOINTS],
    /// `torn`: set by the tears, read by nothing.
    pub torn: [bool; NUM_JOINTS],
}

/// A vertex as `EnGanonMant_UpdateVertices` writes it: `n.ob` and `n.n`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MantVtx {
    pub ob: [i16; 3],
    pub n: [i8; 3],
}

pub struct EnGanonMant {
    pub actor: Actor,
    pub strands: [MantStrand; NUM_STRANDS],
    /// `minY`: the joints' floor.
    pub min_y: f32,
    /// `backPush`: the push back (negative: away from the actor).
    pub back_push: f32,
    /// `backSwayMagnitude`, `sideSwayMagnitude`.
    pub back_sway_magnitude: f32,
    pub side_sway_magnitude: f32,
    /// `attachRightArmTimer`, `attachLeftArmTimer`, `attachShouldersTimer`.
    pub attach_right_arm_timer: f32,
    pub attach_left_arm_timer: f32,
    pub attach_shoulders_timer: f32,
    pub gravity: f32,
    /// `baseYaw`: the collar's yaw less a quarter turn (radians).
    pub base_yaw: f32,
    /// `minDist`: the joints' least distance across from the actor (times `sDistMultipliers`).
    pub min_dist: f32,
    pub right_forearm_pos: Vec3,
    pub left_forearm_pos: Vec3,
    pub right_shoulder_pos: Vec3,
    pub left_shoulder_pos: Vec3,
    /// `tearTimer`: frames left to tear.
    pub tear_timer: u8,
    pub update_has_run: bool,
    pub frame_timer: u8,
    /// `gMant1Vtx`, `gMant2Vtx` as the draw wrote them (`None`: as the file has them).
    pub vertices: [Option<[MantVtx; NUM_VERTICES]>; 2],
    /// This cape's copy of `gMantTex` (RGBA16 texels, big-endian, as the overlay holds them).
    pub texture: Option<Vec<u8>>,
    /// The copy as an image, once torn.
    pub torn_image: Option<DrawImage>,
    /// Each bake's vertices' places in the file, by bone.
    file_vertices: [Option<Arc<[Vec3]>>; 2],
}

/// `Math_Atan2F(x, y)`: `Math_Atan2S` in radians (`BINANG_TO_RAD`).
fn atan2f(x: f32, y: f32) -> f32 {
    eng_math::binang_to_rad(eng_math::atan2_s(x, y))
}

/// `BINANG_TO_RAD_ALT`: `((f32)binang / (f32)0x8000) * M_PI`.
fn binang_to_rad_alt(b: i16) -> f32 {
    (b as f32 / 32768.0) * std::f32::consts::PI
}

/// `Matrix_RotateY(y, MTXMODE_NEW)`, then `Matrix_RotateX(x, MTXMODE_APPLY)` when given.
fn rot_yx(y: f32, x: Option<f32>) -> MtxF {
    let mut m = MtxF::IDENTITY;
    m.rotate_y(y);
    if let Some(x) = x {
        m.rotate_x(x);
    }
    m
}

/// The places of a bake's vertices in the file, by bone (the bake's own `pos`: no matrix).
fn file_vertices(play: &PlayState, bake: &str) -> Option<Arc<[Vec3]>> {
    let d = play.assets.as_ref()?.pack.bake(bake).map_err(|e| log::error!("En_Ganon_Mant: bake {bake}: {e:#}")).ok()?;
    let mut v = vec![Vec3::ZERO; NUM_VERTICES];
    for x in d.batches.iter().flat_map(|b| &b.vertices) {
        if let Some(p) = v.get_mut(x.bone as usize) {
            *p = x.pos;
        }
    }
    Some(v.into())
}

impl EnGanonMant {
    /// `EnGanonMant_Init`: not targetable. (The instance is zeroed by `Actor_Spawn`.) The
    /// texture's copy from the pack.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        let texture = play.assets.as_ref().and_then(|a| a.pack.texture(OVERLAY, "gMantTex").map_err(|e| log::error!("En_Ganon_Mant: gMantTex: {e:#}")).ok()).map(|t| t.texels);
        let file_vertices = BAKES.map(|b| file_vertices(play, b));
        Box::new(EnGanonMant::new(actor, texture, file_vertices))
    }

    fn new(actor: Actor, texture: Option<Vec<u8>>, file_vertices: [Option<Arc<[Vec3]>>; 2]) -> EnGanonMant {
        EnGanonMant {
            actor,
            strands: [MantStrand::default(); NUM_STRANDS],
            min_y: 0.0,
            back_push: 0.0,
            back_sway_magnitude: 0.0,
            side_sway_magnitude: 0.0,
            attach_right_arm_timer: 0.0,
            attach_left_arm_timer: 0.0,
            attach_shoulders_timer: 0.0,
            gravity: 0.0,
            base_yaw: 0.0,
            min_dist: 0.0,
            right_forearm_pos: Vec3::ZERO,
            left_forearm_pos: Vec3::ZERO,
            right_shoulder_pos: Vec3::ZERO,
            left_shoulder_pos: Vec3::ZERO,
            tear_timer: 0,
            update_has_run: false,
            frame_timer: 0,
            vertices: [None; 2],
            texture,
            torn_image: None,
            file_vertices,
        }
    }

    /// The copy of the vertices this frame's draw writes and draws (`frameTimer % 2 != 0`:
    /// `gMant1Vtx`, 0; else `gMant2Vtx`, 1).
    pub fn buffer(&self) -> usize {
        if self.frame_timer % 2 != 0 { 0 } else { 1 }
    }

    /// `EnGanonMant_Tear`: a line of `sTearShapes`' squares zeroed in the texture from a random
    /// texel in a random direction; four random joints marked torn.
    pub fn tear(&mut self, rand: &mut Rand) {
        let mut tx = rand.zero_float(TEX_WIDTH as f32);
        let mut ty = rand.zero_float(TEX_HEIGHT as f32);
        let tear_angle = rand.zero_float(2.0 * std::f32::consts::PI);
        let tear_dir_x = tear_angle.sin();
        let tear_dir_y = tear_angle.cos();
        let shape = TEAR_SHAPES[rand.zero_float(TEAR_SHAPES.len() as f32 - 0.01) as i16 as usize];
        for &size in shape {
            if (0.0 <= tx && tx < TEX_WIDTH as f32) && (0.0 <= ty && ty < TEX_HEIGHT as f32) {
                for area_x in 0..=size {
                    for area_y in 0..=size {
                        // (s16)((s16)tx + (s16)ty * 32) + (areaX + areaY * 32): a square past
                        // the row's end goes on into the next row.
                        let tex_idx = ((tx as i16).wrapping_add((ty as i16).wrapping_mul(TEX_WIDTH as i16))).wrapping_add(area_x.wrapping_add(area_y.wrapping_mul(TEX_WIDTH as i16)));
                        if tex_idx as i32 >= (TEX_WIDTH * TEX_HEIGHT) as i32 {
                            continue;
                        }
                        // ((u16*)gMantTex)[texIdx] = 0.
                        if let Some(px) = self.texture.as_mut().and_then(|t| t.get_mut(tex_idx as usize * 2..tex_idx as usize * 2 + 2)) {
                            px.fill(0);
                        }
                    }
                }
            }
            tx += tear_dir_x;
            ty += tear_dir_y;
        }
        for _ in 0..4 {
            // strands[(s16)Rand_ZeroFloat(11.9f)].torn[(s16)Rand_ZeroFloat(11.9f)]: the strand's
            // number drawn first.
            let s = rand.zero_float(NUM_STRANDS as f32 - 0.1) as i16 as usize;
            let j = rand.zero_float(NUM_JOINTS as f32 - 0.1) as i16 as usize;
            self.strands[s].torn[j] = true;
        }
        self.torn_image = self.texture.as_ref().map(|t| oot_game::object_ctx::rgba16_image(t, TEX_WIDTH as u32, TEX_HEIGHT as u32));
    }

    /// `EnGanonMant_UpdateStrand(play, this, root, pos, nextPos, rot, vel, strandNum)`: strand
    /// `strand_num`'s joints from its root, `next` the strand whose joints give the yaws.
    pub fn update_strand(&mut self, strand_num: usize, next: usize) {
        let next_joints = self.strands[next].joints;
        let params = PARAMS.get(self.actor.params);
        let (world, shape_yaw) = (self.actor.world_pos, self.actor.shape_rot.y);
        let (back_push, back_sway, side_sway, base_yaw, gravity, min_dist, min_y) = (self.back_push, self.back_sway_magnitude, self.side_sway_magnitude, self.base_yaw, self.gravity, self.min_dist, self.min_y);
        let s = &mut self.strands[strand_num];
        let joint_length;
        if params == PARAMS_VIEWER {
            // Pushes all the strands away from the actor.
            let pos_step = rot_yx(binang_to_rad_alt(shape_yaw), None).mult_vec3f(Vec3::new(0.0, 0.0, -30.0));
            for p in s.joints.iter_mut() {
                p.x += pos_step.x;
                p.z += pos_step.z;
            }
            joint_length = 6.5;
        } else {
            joint_length = 9.5;
        }
        let strand_f = strand_num as i16 as f32;
        for i in 0..NUM_JOINTS {
            if i == 0 {
                // Constraint: the first position is always the root.
                s.joints[0] = s.root;
                continue;
            }
            // Decelerate.
            let vel = &mut s.velocities[i];
            eng_math::approach_zero_f(&mut vel.x, 1.0, 0.1);
            eng_math::approach_zero_f(&mut vel.y, 1.0, 0.1);
            eng_math::approach_zero_f(&mut vel.z, 1.0, 0.1);
            let vel = *vel;

            // The push away from the actor, plus oscillations.
            let m = rot_yx(base_yaw, None);
            let dz = (back_push + ((strand_f * (2.0 * std::f32::consts::PI)) / 2.1).sin() * back_sway) * BACK_SWAY_COEFFICIENTS[i];
            let back_sway_offset = m.mult_vec3f(Vec3::new(0.0, 0.0, dz));
            // The push out to either side, swaying.
            let dx = ((strand_f * std::f32::consts::PI) / (NUM_STRANDS as f32 - 1.0)).cos() * side_sway * SIDE_SWAY_COEFFICIENTS[i];
            let side_sway_offset = m.mult_vec3f(Vec3::new(dx, 0.0, 0.0));

            // The position difference.
            let (pos, prev) = (s.joints[i], s.joints[i - 1]);
            let x = ((pos.x + vel.x) - prev.x) + (back_sway_offset.x + side_sway_offset.x);
            let y = ((pos.y + vel.y) - prev.y) + gravity;
            let z = ((pos.z + vel.z) - prev.z) + (back_sway_offset.z + side_sway_offset.z);

            // The rotations in its direction.
            let yaw = atan2f(z, x);
            let pitch = -atan2f((x * x + z * z).sqrt(), y);
            s.rotations[i - 1].x = pitch;

            // The step of the joint's length that way.
            let pos_step = rot_yx(yaw, Some(pitch)).mult_vec3f(Vec3::new(0.0, 0.0, joint_length));
            let old = pos;
            let mut p = prev + pos_step;

            // Pushed away from the actor if too close.
            let x_diff = p.x - world.x;
            let z_diff = p.z - world.z;
            if (x_diff * x_diff + z_diff * z_diff).sqrt() < DIST_MULTIPLIERS[i] * min_dist {
                let yaw = atan2f(z_diff, x_diff);
                let step = rot_yx(yaw, None).mult_vec3f(Vec3::new(0.0, 0.0, min_dist * DIST_MULTIPLIERS[i]));
                p.x = world.x + step.x;
                p.z = world.z + step.z;
            }
            // The floor.
            if p.y < min_y {
                p.y = min_y;
            }
            s.joints[i] = p;

            // The next velocity.
            let mut v = Vec3::new((p.x - old.x) * 80.0 / 100.0, (p.y - old.y) * 80.0 / 100.0, (p.z - old.z) * 80.0 / 100.0);
            if params != PARAMS_VIEWER {
                // Each kept within [-5, 5].
                v = v.clamp(Vec3::splat(-5.0), Vec3::splat(5.0));
            }
            s.velocities[i] = v;

            // The yaw towards the next strand's joint.
            let x_diff = p.x - next_joints[i].x;
            let z_diff = p.z - next_joints[i].z;
            s.rotations[i - 1].y = atan2f(z_diff, x_diff);
        }
        // `rot[11].y = rot[10].y; rot[11].x = rot[10].x;` with `rot` already past the end:
        // @bug (game) the last joint's rotation is never written (it stays 0, its normal straight
        // up); the writes land 23 and 22 `Vec3f`s on, in `velocities` (11 and 10).
        s.velocities[11].y = s.velocities[10].y;
        s.velocities[11].x = s.velocities[10].x;
    }

    /// `EnGanonMant_UpdateVertices`: this frame's copy of the vertices from the strands (strand
    /// `i`'s joint `j` to vertex `sVerticesMap[i + 12 j]`), the normals (0, 30, 0) turned by the
    /// joints' rotations.
    pub fn update_vertices(&mut self) {
        let b = self.buffer();
        let vertices = self.vertices[b].get_or_insert([MantVtx::default(); NUM_VERTICES]);
        let up = Vec3::new(0.0, 30.0, 0.0);
        for (i, strand) in self.strands.iter().enumerate() {
            for j in 0..NUM_JOINTS {
                let vtx = &mut vertices[VERTICES_MAP[i + j * NUM_JOINTS] as usize];
                let p = strand.joints[j];
                vtx.ob = [p.x as i32 as i16, p.y as i32 as i16, p.z as i32 as i16];
                let r = strand.rotations[j];
                let normal = rot_yx(r.y, Some(r.x)).mult_vec3f(up);
                vtx.n = [normal.x as i32 as i8, normal.y as i32 as i8, normal.z as i32 as i8];
            }
        }
    }

    /// `EnGanonMant_Draw`'s update, when update has run since the last draw: the endpoints by the
    /// timers, the roots, each strand, the vertices.
    pub fn draw_strands(&mut self) {
        if !self.update_has_run {
            return;
        }
        // Choose the endpoints.
        let (right_pos, left_pos) = if self.attach_right_arm_timer != 0.0 {
            self.gravity = -13.0;
            (self.right_forearm_pos, self.left_shoulder_pos)
        } else if self.attach_left_arm_timer != 0.0 {
            self.gravity = -13.0;
            (self.right_shoulder_pos, self.left_forearm_pos)
        } else if self.attach_shoulders_timer != 0.0 {
            self.gravity = -3.0;
            (self.right_shoulder_pos, self.left_shoulder_pos)
        } else {
            (self.right_forearm_pos, self.left_forearm_pos)
        };
        let x_diff = left_pos.x - right_pos.x;
        let y_diff = left_pos.y - right_pos.y;
        let z_diff = left_pos.z - right_pos.z;
        let midpoint = Vec3::new(right_pos.x + x_diff * 0.5, right_pos.y + y_diff * 0.5, right_pos.z + z_diff * 0.5);

        // The base orientation for the endpoints.
        let yaw = atan2f(z_diff, x_diff);
        let pitch = -atan2f((x_diff * x_diff + z_diff * z_diff).sqrt(), y_diff);
        let diff_half_dist = (x_diff * x_diff + y_diff * y_diff + z_diff * z_diff).sqrt() * 0.5;
        let m = rot_yx(yaw, Some(pitch));
        self.base_yaw = yaw - std::f32::consts::PI / 2.0;

        for strand_idx in 0..NUM_STRANDS {
            // The roots on the half circle between the endpoints (Matrix_Push: the strand's
            // matrices are its own).
            let a = (strand_idx as f32 * std::f32::consts::PI) / (NUM_STRANDS - 1) as f32;
            let strand_div_pos = m.mult_vec3f(Vec3::new(a.sin() * diff_half_dist, 0.0, -a.cos() * diff_half_dist));
            self.strands[strand_idx].root = midpoint + strand_div_pos;
            let next = if strand_idx + 1 >= NUM_STRANDS { strand_idx - 1 } else { strand_idx + 1 };
            self.update_strand(strand_idx, next);
        }
        self.update_vertices();
        self.update_has_run = false;
    }
}

impl ActorImpl for EnGanonMant {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnGanonMant_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.update_has_run = true;
        self.frame_timer = self.frame_timer.wrapping_add(1);
        if self.attach_left_arm_timer != 0.0 {
            self.attach_left_arm_timer -= 1.0;
        }
        if self.attach_right_arm_timer != 0.0 {
            self.attach_right_arm_timer -= 1.0;
        }
        if self.attach_shoulders_timer != 0.0 {
            self.attach_shoulders_timer -= 1.0;
        }
        // BossGanon* ganon = (BossGanon*)this->actor.parent: only its shape's yaw.
        match self.actor.parent.and_then(|h| play.actors.actor(h)) {
            Some(parent) => self.actor.shape_rot.y = parent.shape_rot.y,
            None => log::warn!("En_Ganon_Mant: no parent (the C reads through a null pointer)"),
        }
        if self.tear_timer != 0 {
            self.tear_timer -= 1;
            self.tear(&mut play.rand);
        }
    }

    /// `EnGanonMant_Destroy`: nothing.
    fn destroy(&mut self, _play: &mut PlayState) {}

    /// `EnGanonMant_Draw`'s writes: the strands and vertices when update has run.
    fn draw_update(&mut self, _play: &mut PlayState) {
        self.draw_strands();
    }

    /// The copy drawn (`switches`: it, and whether it was written) and its vertices' places and
    /// normals (`values`, six each).
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        let b = self.buffer();
        rs.switches = vec![b as u32, self.vertices[b].is_some() as u32];
        if let Some(v) = &self.vertices[b] {
            rs.values = v.iter().flat_map(|x| [x.ob[0] as f32, x.ob[1] as f32, x.ob[2] as f32, x.n[0] as f32, x.n[1] as f32, x.n[2] as f32]).collect();
        }
        rs
    }

    /// `EnGanonMant_DrawCloak`: the identity matrix, `gMantMaterialDL`, segment 0x0C on this
    /// frame's copy, `gMantDL`, on OPA; each vertex's bone from the file's place to the written one
    /// with its normal turned from the file's (straight up) to the written one.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let [b, written] = rs.switches.as_slice() else { return };
        let b = *b as usize & 1;
        let bones = match (&self.file_vertices[b], *written != 0) {
            (Some(file), true) if rs.values.len() == NUM_VERTICES * 6 => rs
                .values
                .chunks_exact(6)
                .zip(file.iter())
                .map(|(v, &p0)| {
                    let n = Vec3::new(v[3], v[4], v[5]).normalize_or_zero();
                    let r = if n == Vec3::ZERO { Quat::IDENTITY } else { Quat::from_rotation_arc(Vec3::Y, n) };
                    Mat4::from_translation(Vec3::new(v[0], v[1], v[2])) * Mat4::from_quat(r) * Mat4::from_translation(-p0)
                })
                .collect(),
            _ => vec![Mat4::IDENTITY; NUM_VERTICES],
        };
        let texture_images = self.torn_image.iter().map(|image| SourceImage { source: MANT_TEX_ADDR, image: image.clone() }).collect();
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKES[b])), transform: Mat4::IDENTITY, bones, params: DrawParams { texture_images, ..Default::default() } });
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
    fn strand_i_joint_j_is_vertex_12i_plus_11_minus_j() {
        for i in 0..NUM_STRANDS {
            for j in 0..NUM_JOINTS {
                assert_eq!(VERTICES_MAP[i + j * NUM_JOINTS] as usize, 12 * i + 11 - j);
            }
        }
    }
}
