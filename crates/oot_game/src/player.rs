//! Player movement, ported from `ovl_player_actor/z_player.c` (the decomp commit in use still
//! calls most of these `func_808xxxxx`; each port names its original).
//!
//! Scope: standing, turning in place, walking/running with the walk/run blend, rolling on A
//! (including bonking into walls), running off ledges (auto-jump or fall), air control,
//! landing (including fall-damage stagger and landing rolls), and the head/torso look and
//! lean rotations. Everything that needs items, targeting, water, cutscenes, other actors,
//! ledge hanging or climbing is left out. The interrupt checks for those are stubbed to
//! "not taken", and the list is in the spike 03 findings.
//!
//! Naming: fields keep the decomp's `unk_XXX` names where the decomp has no name yet, so the
//! port can be diffed against the C. File-scope statics (`D_808535D4` ...) live in
//! `PlayerStatics`.

#![allow(non_snake_case)] // fields keep the decomp's unk_XXX names

use glam::Vec3;
use oot_core::player::Blinker;

use crate::actor::*;
use crate::bgcheck::{BGCHECK_Y_MIN, StaticCollision, WALL_FLAG_0, WALL_FLAG_1, WALL_FLAG_3, udist_plane_to_pos};
use crate::data::{AgeProperties, AnimId, GameData, Regs};
use crate::input::{BTN_A, Input, stick_to_mag_angle};
use crate::math::*;
use crate::skelanime::{ANIMMODE_LOOP, ANIMMODE_ONCE, SkelAnime};

// stateFlags1
pub const STATE1_2: u32 = 1 << 2;
pub const STATE1_4: u32 = 1 << 4; // targeting (func_8008E9C4)
pub const STATE1_6: u32 = 1 << 6;
pub const STATE1_11: u32 = 1 << 11; // holding an actor
pub const STATE1_12: u32 = 1 << 12;
pub const STATE1_13: u32 = 1 << 13;
pub const STATE1_14: u32 = 1 << 14;
pub const STATE1_16: u32 = 1 << 16;
pub const STATE1_17: u32 = 1 << 17;
pub const STATE1_18: u32 = 1 << 18; // jumping
pub const STATE1_19: u32 = 1 << 19; // freefall
pub const STATE1_20: u32 = 1 << 20;
pub const STATE1_21: u32 = 1 << 21;
pub const STATE1_22: u32 = 1 << 22; // shielding
pub const STATE1_26: u32 = 1 << 26;
pub const STATE1_27: u32 = 1 << 27; // swimming
pub const STATE1_28: u32 = 1 << 28;
pub const STATE1_29: u32 = 1 << 29;
pub const STATE1_30: u32 = 1 << 30;
pub const STATE1_31: u32 = 1 << 31;
// stateFlags2
pub const STATE2_0: u32 = 1 << 0;
pub const STATE2_2: u32 = 1 << 2;
pub const STATE2_3: u32 = 1 << 3;
pub const STATE2_5: u32 = 1 << 5; // facing follows currentYaw
pub const STATE2_6: u32 = 1 << 6;
pub const STATE2_8: u32 = 1 << 8;
pub const STATE2_9: u32 = 1 << 9;
pub const STATE2_12: u32 = 1 << 12;
pub const STATE2_14: u32 = 1 << 14;
pub const STATE2_16: u32 = 1 << 16;
pub const STATE2_18: u32 = 1 << 18;
pub const STATE2_19: u32 = 1 << 19; // side hop / backflip
pub const STATE2_22: u32 = 1 << 22;
pub const STATE2_26: u32 = 1 << 26;
pub const STATE2_27: u32 = 1 << 27;
pub const STATE2_28: u32 = 1 << 28; // idle fidget
// stateFlags3
pub const STATE3_1: u32 = 1 << 1;
pub const STATE3_3: u32 = 1 << 3;
pub const STATE3_4: u32 = 1 << 4;
pub const STATE3_7: u32 = 1 << 7;

// floor properties (FLOOR_PROPERTY_*)
const FLOOR_PROPERTY_6: u32 = 6;
const FLOOR_PROPERTY_7: u32 = 7;
const FLOOR_PROPERTY_8: u32 = 8;
const FLOOR_PROPERTY_9: u32 = 9;
const FLOOR_TYPE_6: u32 = 6;
const FLOOR_TYPE_7: u32 = 7;
const FLOOR_TYPE_9: u32 = 9;

/// `PLAYER_ANIMGROUP_*` indices used here (group 0 = wait, 1 = walk, 2 = run, ...).
pub mod group {
    pub const WAIT: usize = 0;
    pub const WALK: usize = 1;
    pub const RUN: usize = 2;
    pub const DAMAGE_RUN: usize = 3;
    pub const HEAVY_RUN: usize = 4;
    pub const LANDING: usize = 14;
    pub const SHORT_LANDING: usize = 15;
    pub const ROLL: usize = 16;
    pub const ROLL_BONK: usize = 17;
    pub const END_WALK_A: usize = 18;
    pub const END_WALK_B: usize = 19;
    pub const TURN: usize = 26;
}

/// The action function in `this->func_674`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_80840BC8`: standing (idle, landing, end of roll).
    StandingStill,
    /// `func_80842180`: walking and running.
    Run,
    /// `func_80841BA8`: turning in place.
    Turn,
    /// `func_80844708`: rolling.
    Roll,
    /// `func_8084411C`: in the air (jumping or falling).
    Midair,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::StandingStill => "func_80840BC8",
            Action::Run => "func_80842180",
            Action::Turn => "func_80841BA8",
            Action::Roll => "func_80844708",
            Action::Midair => "func_8084411C",
        }
    }
}

/// File-scope statics of `z_player.c` that Player's update shares between functions.
#[derive(Debug, Clone, Default)]
pub struct PlayerStatics {
    /// `D_808535D4`: stick magnitude 0..60 (`func_80077D10`).
    pub stick_mag: f32,
    /// `D_808535D8`: stick angle relative to up.
    pub stick_angle: i16,
    /// `D_808535DC`: stick direction in world space (camera input yaw + stick angle).
    pub stick_world_yaw: i16,
    /// `D_808535E0`: upper-body item action result (always 0 without items).
    pub item_action: i32,
    /// `D_808535E4`: floor type under Player.
    pub floor_type: u32,
    /// `D_808535E8`: animation/speed scale, 0.5 underwater and 1 otherwise.
    pub speed_scale: f32,
    /// `D_808535F0`: wall flags of the touched wall.
    pub wall_flags: u32,
    /// `D_80853600`: height above the floor.
    pub floor_dist: f32,
    /// `D_80853604`: floor property.
    pub floor_property: u32,
    /// `D_80853608`: |facing - wall normal|.
    pub wall_facing_diff: i32,
    /// `D_8085360C`: |movement yaw - wall normal|.
    pub wall_move_diff: i32,
    /// `D_80853610`: floor slope along the facing direction.
    pub facing_slope: i16,
}

/// Per-frame environment the update needs.
pub struct Env<'a> {
    pub data: &'a GameData,
    pub col: &'a StaticCollision,
    /// `Camera_GetInputDirYaw(GET_ACTIVE_CAM(play))`.
    pub cam_input_yaw: i16,
    /// `play->gameplayFrames` (face alternation).
    pub gameplay_frames: u32,
}

#[derive(Debug, Clone)]
pub struct Player {
    pub actor: Actor,
    pub skel: SkelAnime,
    pub action: Action,
    pub regs: Regs,
    pub age: AgeProperties,
    pub adult: bool,
    /// `modelAnimType` (0: nothing in hand).
    pub model_anim_type: usize,
    pub state1: u32,
    pub state2: u32,
    pub state3: u32,
    pub linear_velocity: f32,
    pub current_yaw: i16,
    pub target_yaw: i16,
    pub unk_84F: i8,
    /// Multipurpose timer.
    pub unk_850: i16,
    /// Rolling index into the stick history below.
    pub unk_846: u8,
    /// Stick direction history (8 sectors), -1 when the stick is below 55.
    pub unk_847: [i8; 4],
    /// Stick direction relative to facing (0 forward, 1 left, 2 back, 3 right), -1 below 55.
    pub unk_84B: [i8; 4],
    pub unk_A7C: f32,
    pub unk_A80: i16,
    /// Walk start blend (0 → 1).
    pub unk_864: f32,
    /// Walk/run cycle phase in walk-animation frames (0..29).
    pub unk_868: f32,
    pub unk_870: f32,
    pub unk_874: f32,
    /// Current run speed limit (`R_RUN_SPEED_LIMIT`, reduced when running into walls).
    pub unk_880: f32,
    /// Floor slope along `currentYaw` and across it.
    pub unk_898: i16,
    pub unk_89A: i16,
    /// Walk climb/descend blend.
    pub unk_89C: i16,
    pub unk_6C2: i16,
    pub unk_6C4: f32,
    /// Which of the look/lean rotations were driven this frame (`func_80847298` decays the rest).
    pub unk_6AE: u16,
    pub unk_6AC: i8,
    pub unk_6AD: u8,
    /// Head: z (unk_6B6), y (unk_6B8), x (unk_6BA). Upper body: x (unk_6BC), y (unk_6BE), z (unk_6C0).
    pub unk_6B0: i16,
    pub unk_6B6: i16,
    pub unk_6B8: i16,
    pub unk_6BA: i16,
    pub unk_6BC: i16,
    pub unk_6BE: i16,
    pub unk_6C0: i16,
    pub unk_87C: i16,
    pub unk_87E: i16,
    pub unk_890: u8,
    pub unk_A7A: u32,
    pub unk_A79: u8,
    pub unk_A7B: u32,
    pub fall_start_height: i16,
    pub fall_distance: i16,
    pub wall_height: f32,
    pub wall_distance: f32,
    pub unk_88C: u8,
    pub unk_88D: u8,
    pub hover_boots_timer: u8,
    pub pushed_speed: f32,
    pub pushed_yaw: i16,
    pub melee_weapon_state: i8,
    pub s: PlayerStatics,
    pub input: Input,
    /// `unk_3A8` blink timer; `actor.shape.face`.
    pub blinker: Blinker,
    pub face: usize,
    rng: u32,
    /// Diagnostics: things the port saw but does not implement (ledge grab, climbing...).
    pub notes: Vec<String>,
    pub frame: u32,
}

/// `D_80854730`: the root translation Link's animations are authored around.
const BASE_TRANSL: [i16; 3] = [-57, 3377, 0];
/// `D_8085456C`: head-look floor probe offset.
const HEAD_PROBE: Vec3 = Vec3::new(0.0, 100.0, 40.0);

fn abs16(v: i16) -> i32 {
    (v as i32).abs()
}

impl Player {
    /// `Player_Init` / `Player_InitCommon` for a plain spawn standing at `pos`.
    pub fn new(data: &GameData, adult: bool, pos: Vec3, yaw: i16) -> Player {
        let age = data.ages[if adult { 0 } else { 1 }];
        let regs = data.regs[if adult { 0 } else { 1 }].clone();
        let model_anim_type = 0;
        let skel = SkelAnime::new_link(data, data.player_anim(group::WAIT, model_anim_type), BASE_TRANSL);
        let mut actor = Actor::new(pos, yaw);
        actor.gravity = regs.reg(68) as f32 / 100.0;
        let mut p = Player {
            actor,
            skel,
            action: Action::StandingStill,
            unk_880: regs.run_speed_limit(),
            regs,
            age,
            adult,
            model_anim_type,
            state1: 0,
            state2: 0,
            state3: 0,
            linear_velocity: 0.0,
            current_yaw: yaw,
            target_yaw: 0,
            unk_84F: 0,
            unk_850: 0,
            unk_846: 0,
            unk_847: [-1; 4],
            unk_84B: [-1; 4],
            unk_A7C: 0.0,
            unk_A80: 0,
            unk_864: 0.0,
            unk_868: 0.0,
            unk_870: 0.0,
            unk_874: 0.0,
            unk_898: 0,
            unk_89A: 0,
            unk_89C: 0,
            unk_6C2: 0,
            unk_6C4: 0.0,
            unk_6AE: 0,
            unk_6AC: 0,
            unk_6AD: 0,
            unk_6B0: 0,
            unk_6B6: 0,
            unk_6B8: 0,
            unk_6BA: 0,
            unk_6BC: 0,
            unk_6BE: 0,
            unk_6C0: 0,
            unk_87C: 0,
            unk_87E: 0,
            unk_890: 0,
            unk_A7A: 0,
            unk_A79: 0,
            unk_A7B: 0,
            fall_start_height: pos.y as i16,
            fall_distance: 0,
            wall_height: 0.0,
            wall_distance: 0.0,
            unk_88C: 0,
            unk_88D: 0,
            hover_boots_timer: 0,
            pushed_speed: 0.0,
            pushed_yaw: 0,
            melee_weapon_state: 0,
            s: PlayerStatics { speed_scale: 1.0, ..Default::default() },
            input: Input::default(),
            blinker: Blinker::default(),
            face: 0,
            rng: 0x1234_5678,
            notes: Vec::new(),
            frame: 0,
        };
        // `func_80846648`-style plain start: standing still (`func_80853080`).
        p.func_80853080(data);
        p
    }

    fn note(&mut self, s: impl Into<String>) {
        let s = s.into();
        if self.notes.last() != Some(&s) {
            log::debug!("player: {s}");
            self.notes.push(s);
        }
    }

    /// `Rand_ZeroOne` (`code_800FD970.c`).
    fn rand_zero_one(&mut self) -> f32 {
        self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        f32::from_bits((self.rng >> 9) | 0x3F80_0000) - 1.0
    }

    fn anim(&self, data: &GameData, g: usize) -> AnimId {
        data.player_anim(g, self.model_anim_type)
    }

    pub fn grounded(&self) -> bool {
        self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0
    }

    // ================================================================================
    // Per-frame update

    /// `Player_Update` → `Player_UpdateCommon`, movement path. Call once per game frame
    /// with the frame's input; then `finish_frame` (the AnimationContext update).
    pub fn update(&mut self, env: &Env, input: Input) {
        self.frame += 1;
        self.input = input;
        let data = env.data;
        self.actor.prev_pos = self.actor.home_pos;

        // unk_A73/88E/A87 timers, invincibility: not modelled.
        // func_808473D4 (do-action HUD), func_80836BEC (Z targeting): not modelled.

        scaled_step_to_s(&mut self.unk_6C2, 0, 400);
        // func_80032CB4(this->unk_3A8, 20, 80, 6) and the face alternation.
        let blink = self.blinker.step();
        self.face = blink + if env.gameplay_frames & 32 != 0 { 0 } else { 3 };

        if self.skel.move_flags & 0x80 == 0 {
            // Not on ice and no hover boots: speed and direction come straight from Player.
            self.actor.speed_xz = self.linear_velocity;
            self.actor.world_rot.y = self.current_yaw;
            self.actor.update_velocity();
            if self.pushed_speed != 0.0 {
                self.actor.velocity.x += self.pushed_speed * sin_s(self.pushed_yaw);
                self.actor.velocity.z += self.pushed_speed * cos_s(self.pushed_yaw);
            }
            self.actor.update_pos();
            self.func_80847BA0(env);
        }
        if self.pushed_speed != 0.0 {
            step_to_f(&mut self.pushed_speed, 0.0, 1.0);
        }

        // func_8083D53C (water), damage (func_808382DC): not modelled.
        self.func_8083AA10(env);

        self.func_8083D6EC();

        self.state1 &= !(STATE1_12 | STATE1_22 | (1 << 1) | (1 << 9));
        self.state2 &= !(STATE2_0 | STATE2_2 | STATE2_3 | STATE2_5 | STATE2_6 | STATE2_8 | (1 << 9) | STATE2_12 | STATE2_14 | STATE2_16 | STATE2_22 | STATE2_26);
        self.state3 &= !STATE3_4;

        self.func_80847298();
        self.func_8083315C(env);

        // Not swimming.
        self.s.speed_scale = 1.0;

        if self.state3 & (1 << 2) == 0 {
            self.run_action(env);
        }

        // Player_UpdateCamAndSeqModes: camera is not ported.
        if self.skel.move_flags & 8 != 0 {
            let s = if self.skel.move_flags & 4 != 0 { 1.0 } else { self.age.translation_scale };
            self.skel.request_move_actor(s);
        }
        self.func_808368EC();
        let _ = data;
        self.actor.home_pos = self.actor.world_pos;
    }

    /// `AnimationContext_Update`, run after all actors (only Player here) have updated.
    pub fn finish_frame(&mut self) {
        self.skel.run_queue(&mut self.actor);
    }

    fn run_action(&mut self, env: &Env) {
        match self.action {
            Action::StandingStill => self.func_80840BC8(env),
            Action::Run => self.func_80842180(env),
            Action::Turn => self.func_80841BA8(env),
            Action::Roll => self.func_80844708(env),
            Action::Midair => self.func_8084411C(env),
        }
    }

    /// `func_8083D6EC`: gravity and terminal velocity (water/sand parts not modelled).
    fn func_8083D6EC(&mut self) {
        self.actor.min_velocity_y = -20.0;
        self.actor.gravity = self.regs.reg(68) as f32 / 100.0;
    }

    /// `func_8083315C`: stick processing, once per frame.
    fn func_8083315C(&mut self, env: &Env) {
        self.unk_A7C = self.s.stick_mag;
        self.unk_A80 = self.s.stick_angle;
        let (mag, ang) = stick_to_mag_angle(&self.input);
        self.s.stick_mag = mag;
        self.s.stick_angle = ang;
        self.s.stick_world_yaw = env.cam_input_yaw.wrapping_add(ang);
        self.unk_846 = (self.unk_846 + 1) % 4;
        let (v1, v0) = if self.s.stick_mag < 55.0 {
            (-1i8, -1i8)
        } else {
            (
                ((self.s.stick_angle.wrapping_add(0x2000) as u16) >> 9) as i8,
                (((self.s.stick_world_yaw.wrapping_sub(self.actor.shape_rot.y)).wrapping_add(0x2000) as u16) >> 14) as i8,
            )
        };
        self.unk_847[self.unk_846 as usize] = v1;
        self.unk_84B[self.unk_846 as usize] = v0;
    }

    fn stick_dir(&self) -> i8 {
        self.unk_84B[self.unk_846 as usize]
    }

    // ================================================================================
    // Collision (func_80847BA0)

    /// `func_80839768`: line from Player's position (raised by `off.y`) to a point `off`
    /// ahead in facing space; walls only, one-sided.
    fn func_80839768(&self, env: &Env, off: Vec3) -> Option<(Vec3, u16)> {
        let a = Vec3::new(self.actor.world_pos.x, self.actor.world_pos.y + off.y, self.actor.world_pos.z);
        let b = self.func_808395DC(self.actor.world_pos, off);
        env.col.entity_line_test(a, b, true, false, false, true)
    }

    /// `func_808395DC`: `base` + `off` rotated by the facing yaw.
    fn func_808395DC(&self, base: Vec3, off: Vec3) -> Vec3 {
        let (c, s) = (cos_s(self.actor.shape_rot.y), sin_s(self.actor.shape_rot.y));
        Vec3::new(base.x + (off.x * c + off.z * s), base.y + off.y, base.z + (off.z * c - off.x * s))
    }

    /// `func_80847BA0`: Player's bg check and everything derived from the floor and wall.
    fn func_80847BA0(&mut self, env: &Env) {
        let col = env.col;
        let mut spc7 = 0u8;
        self.s.floor_property = self.unk_A7A;
        let (radius, wall_h, ceil_h) = if self.state2 & STATE2_18 != 0 {
            (10.0, 15.0, 30.0)
        } else {
            (self.age.wall_radius, 26.0, self.age.ceiling_check_height)
        };
        let flags = UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_1 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4 | UPDBGCHECKINFO_FLAG_5;
        self.actor.update_bg_check_info(col, wall_h, radius, ceil_h, flags);
        if self.actor.bg_check_flags & BGCHECKFLAG_CEILING != 0 {
            self.actor.velocity.y = 0.0;
        }
        self.s.floor_dist = self.actor.world_pos.y - self.actor.floor_height;
        if let Some(fp) = self.actor.floor_poly {
            self.unk_A7A = col.floor_property(fp);
        }
        // func_80839034 (exits, voids): not modelled.

        self.actor.bg_check_flags &= !BGCHECKFLAG_PLAYER_WALL_INTERACT;
        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
            let probe = Vec3::new(0.0, 18.0, self.age.wall_radius + 10.0);
            if self.state2 & STATE2_18 == 0 {
                if let Some((_, poly)) = self.func_80839768(env, probe) {
                    self.actor.bg_check_flags |= BGCHECKFLAG_PLAYER_WALL_INTERACT;
                    if self.actor.wall_poly != Some(poly) {
                        self.actor.wall_poly = Some(poly);
                        let n = col.poly(poly).normal;
                        self.actor.wall_yaw = atan2_s(n[2] as f32, n[0] as f32);
                    }
                }
            }
            let wall_back = self.actor.wall_yaw.wrapping_add(-0x8000i32 as i16);
            let sp9a = self.actor.shape_rot.y.wrapping_sub(wall_back);
            self.s.wall_flags = self.actor.wall_poly.map(|w| col.wall_flags(w)).unwrap_or(0);
            self.s.wall_facing_diff = abs16(sp9a);
            let sp9a = self.current_yaw.wrapping_sub(wall_back);
            self.s.wall_move_diff = abs16(sp9a);
            let k = self.s.wall_move_diff as f32 * 0.00008;
            let rsl = self.regs.run_speed_limit();
            if !self.grounded() || k >= 1.0 {
                self.unk_880 = rsl;
            } else {
                self.unk_880 = (rsl * k).max(0.1);
            }
            if self.actor.bg_check_flags & BGCHECKFLAG_PLAYER_WALL_INTERACT != 0 && self.s.wall_facing_diff < 0x3000 {
                if let Some(wp) = self.actor.wall_poly {
                    spc7 = self.wall_height_class(env, wp, probe);
                }
            }
        } else {
            self.unk_880 = self.regs.run_speed_limit();
            self.unk_88D = 0;
            self.wall_height = 0.0;
        }
        if spc7 == self.unk_88C {
            if self.linear_velocity != 0.0 && self.unk_88D < 100 {
                self.unk_88D += 1;
            }
        } else {
            self.unk_88C = spc7;
            self.unk_88D = 0;
        }

        if self.grounded() {
            self.s.floor_type = self.actor.floor_poly.map(|p| col.floor_type(p)).unwrap_or(0);
            if !self.func_80847A78() {
                if let Some(fp) = self.actor.floor_poly {
                    let n = col.poly_normal(fp);
                    let inv_ny = 1.0 / n.y;
                    let (s, c) = (sin_s(self.current_yaw), cos_s(self.current_yaw));
                    self.unk_898 = atan2_s(1.0, (-(n.x * s) - (n.z * c)) * inv_ny);
                    self.unk_89A = atan2_s(1.0, (-(n.x * c) - (n.z * s)) * inv_ny);
                    let (s, c) = (sin_s(self.actor.shape_rot.y), cos_s(self.actor.shape_rot.y));
                    self.s.facing_slope = atan2_s(1.0, (-(n.x * s) - (n.z * c)) * inv_ny);
                    // func_8083E318: slippery slopes (FLOOR_EFFECT_1) are not in the course.
                    if col.floor_effect(fp) == 1 {
                        self.note("slippery slope (func_8083E318) not modelled");
                    }
                }
            }
        } else {
            self.func_80847A78();
        }
        if self.unk_A7B == self.s.floor_type {
            self.unk_A79 = self.unk_A79.saturating_add(1);
        } else {
            self.unk_A7B = self.s.floor_type;
            self.unk_A79 = 0;
        }
    }

    /// The wall-top probe in `func_80847BA0`: sets `wallHeight` and returns the climb class
    /// (`spC7`: 1 step, 2..4 increasingly tall ledges).
    fn wall_height_class(&mut self, env: &Env, wp: u16, probe: Vec3) -> u8 {
        let col = env.col;
        let wall = col.poly(wp);
        if abs16(wall.normal[1]) >= 600 {
            return 0;
        }
        let n = col.poly_normal(wp);
        self.wall_distance = udist_plane_to_pos(n, wall.dist as f32, self.actor.world_pos);
        let k = self.wall_distance + 10.0;
        let top_probe = Vec3::new(self.actor.world_pos.x - k * n.x, self.actor.world_pos.y + self.age.wall_probe_height, self.actor.world_pos.z - k * n.z);
        let (top, ground) = col.entity_raycast_down(top_probe);
        self.wall_height = top - self.actor.world_pos.y;
        let ceiling = col
            .check_ceiling(crate::bgcheck::IGNORE_ENTITY, self.actor.world_pos, (top - self.actor.world_pos.y) + 20.0)
            .is_some();
        if self.wall_height < 18.0 || ceiling {
            self.wall_height = 399.96002;
            return 0;
        }
        let probe2 = Vec3::new(probe.x, (top + 5.0) - self.actor.world_pos.y, probe.z);
        if let Some((_, p2)) = self.func_80839768(env, probe2) {
            let n2 = col.poly(p2).normal;
            let d = self.actor.wall_yaw.wrapping_sub(atan2_s(n2[2] as f32, n2[0] as f32));
            if abs16(d) < 0x4000 && col.wall_flags(p2) & WALL_FLAG_1 == 0 {
                self.wall_height = 399.96002;
                return 0;
            }
        }
        if col.wall_flags(wp) & WALL_FLAG_0 != 0 {
            return 0;
        }
        if self.age.unk_1C <= self.wall_height {
            let steep = ground.map(|g| abs16(col.poly(g).normal[1]) > 28000).unwrap_or(false);
            if steep {
                if self.age.unk_14 <= self.wall_height {
                    4
                } else if self.age.unk_18 <= self.wall_height {
                    3
                } else {
                    2
                }
            } else {
                0
            }
        } else {
            1
        }
    }

    /// `func_80847A78`: hover boots bookkeeping; returns true when airborne.
    fn func_80847A78(&mut self) -> bool {
        // Kokiri boots: the timer never counts.
        self.hover_boots_timer = 0;
        if self.grounded() {
            self.hover_boots_timer = 19;
            return false;
        }
        self.s.floor_type = 0;
        self.unk_898 = 0;
        self.unk_89A = 0;
        self.s.facing_slope = 0;
        true
    }

    /// `func_8083AA10`: leaving the ground. Starts an auto-jump off a ledge when running
    /// forward fast, otherwise a fall (or a ledge grab, which is not modelled).
    fn func_8083AA10(&mut self, env: &Env) {
        let data = env.data;
        self.fall_distance = self.fall_start_height.wrapping_sub(self.actor.world_pos.y as i32 as i16);
        if self.state1 & (STATE1_27 | STATE1_29) == 0 && !self.grounded() {
            // func_80838FB8 (void out): not modelled.
            if self.s.floor_property == FLOOR_PROPERTY_8 {
                self.actor.world_pos.x = self.actor.prev_pos.x;
                self.actor.world_pos.z = self.actor.prev_pos.z;
                return;
            }
            if self.state3 & STATE3_1 == 0 && self.skel.move_flags & 0x80 == 0 && self.action != Action::Midair {
                if self.s.floor_property == FLOOR_PROPERTY_7 || self.melee_weapon_state != 0 {
                    self.actor.world_pos = self.actor.prev_pos;
                    self.func_80832210();
                    return;
                }
                if self.hover_boots_timer != 0 {
                    self.actor.velocity.y = 1.0;
                    self.s.floor_property = FLOOR_PROPERTY_9;
                    return;
                }
                let sp5c = self.current_yaw.wrapping_sub(self.actor.shape_rot.y);
                self.setup_action(data, Action::Midair, 1);
                self.func_80832440();
                if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_LEAVE != 0
                    && self.state1 & STATE1_27 == 0
                    && self.s.floor_property != FLOOR_PROPERTY_6
                    && self.s.floor_property != FLOOR_PROPERTY_9
                    && self.s.floor_dist > 20.0
                    && self.melee_weapon_state == 0
                    && abs16(sp5c) < 0x2000
                    && self.linear_velocity > 3.0
                {
                    // FLOOR_PROPERTY_11 jump into water: no water.
                    self.func_8083A4A8(data);
                    return;
                }
                if self.s.floor_property == FLOOR_PROPERTY_9 || self.s.floor_dist <= self.age.ledge_grab_min_drop || !self.func_8083A6AC(env) {
                    let a = data.anim("link_normal_landing_wait");
                    self.skel.play_loop(data, a);
                }
            }
        } else {
            self.fall_start_height = self.actor.world_pos.y as i32 as i16;
        }
    }

    /// `func_8083A6AC`: would Player grab the ledge it just walked off? The grab itself
    /// (`func_8083A5C4` → `func_8084BBE4`) is not ported, so this only reports it and returns
    /// false (Player falls instead).
    fn func_8083A6AC(&mut self, env: &Env) -> bool {
        if self.actor.y_dist_to_water < -80.0 && abs16(self.unk_898) < 2730 && abs16(self.unk_89A) < 2730 {
            let d = Vec3::new(self.actor.prev_pos.x - self.actor.world_pos.x, 0.0, self.actor.prev_pos.z - self.actor.world_pos.z);
            let len = (d.x * d.x + d.z * d.z).sqrt();
            let k = if len != 0.0 { 5.0 / len } else { 0.0 };
            let b = Vec3::new(self.actor.prev_pos.x + d.x * k, self.actor.world_pos.y, self.actor.prev_pos.z + d.z * k);
            if let Some((_, poly)) = env.col.entity_line_test(self.actor.world_pos, b, true, false, false, true) {
                if abs16(env.col.poly(poly).normal[1]) < 600 {
                    self.note("ledge grab (func_8083A6AC) would start here; falling instead");
                }
            }
        }
        false
    }

    /// `func_8083A4A8`: the automatic jump off a ledge.
    fn func_8083A4A8(&mut self, data: &GameData) {
        let yaw_diff = self.current_yaw.wrapping_sub(self.actor.shape_rot.y);
        let anim = if abs16(yaw_diff) < 0x1000 && self.linear_velocity > 4.0 {
            data.anim("link_normal_run_jump")
        } else {
            data.anim("link_normal_jump")
        };
        let r = &self.regs;
        let vy = if self.linear_velocity > r.ireg(66) as f32 / 100.0 {
            r.ireg(67) as f32 / 100.0
        } else {
            (r.ireg(68) as f32 / 100.0) + ((r.ireg(69) as f32 * self.linear_velocity) / 1000.0)
        };
        self.func_80838940(data, Some(anim), vy);
        self.unk_850 = 1;
    }

    /// `func_80838940`: start a jump with vertical speed `vy`.
    fn func_80838940(&mut self, data: &GameData, anim: Option<AnimId>, vy: f32) {
        self.setup_action(data, Action::Midair, 1);
        if let Some(a) = anim {
            self.skel.play_once_set_speed(data, a, 2.0 / 3.0);
        }
        self.actor.velocity.y = vy * self.s.speed_scale;
        self.hover_boots_timer = 0;
        self.actor.bg_check_flags &= !BGCHECKFLAG_GROUND;
        self.state1 |= STATE1_18;
    }

    // ================================================================================
    // Action setup and shared helpers

    /// `func_80835C58`: switch action function.
    fn setup_action(&mut self, _data: &GameData, action: Action, flags: i32) -> bool {
        if self.action == action {
            return false;
        }
        self.action = action;
        if flags & 1 == 0 && self.state1 & STATE1_11 == 0 {
            // func_80834644: nothing held.
            self.state1 &= !STATE1_22;
        }
        self.func_80832DBC();
        self.state1 &= !(STATE1_2 | STATE1_6 | STATE1_26 | STATE1_28 | STATE1_29 | STATE1_31);
        self.state2 &= !(STATE2_19 | STATE2_27 | STATE2_28);
        self.state3 &= !(STATE3_1 | STATE3_3 | STATE3_7);
        self.unk_84F = 0;
        self.unk_850 = 0;
        self.unk_6AC = 0;
        true
    }

    /// `func_80832210`.
    fn func_80832210(&mut self) {
        self.actor.speed_xz = 0.0;
        self.linear_velocity = 0.0;
    }

    /// `func_808322FC`: fold the root limb's yaw into the facing.
    fn func_808322FC(&mut self) {
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(self.skel.joint[1][1]);
        self.skel.joint[1][1] = 0;
    }

    /// `func_80832CFC`.
    fn func_80832CFC(&mut self) {
        self.skel.prev_transl = self.skel.base_transl;
        self.skel.prev_rot = self.actor.shape_rot.y;
    }

    /// `func_80832DBC`: end animation-driven movement.
    fn func_80832DBC(&mut self) {
        if self.skel.move_flags != 0 {
            self.func_808322FC();
            self.skel.joint[0][0] = self.skel.base_transl[0];
            self.skel.joint[0][2] = self.skel.base_transl[2];
            if self.skel.move_flags & 8 != 0 {
                if self.skel.move_flags & 2 != 0 {
                    self.skel.joint[0][1] = self.skel.prev_transl[1];
                }
            } else {
                self.skel.joint[0][1] = self.skel.base_transl[1];
            }
            self.func_80832CFC();
            self.skel.move_flags = 0;
        }
    }

    /// `func_80832440`: drop interaction state when an action is interrupted.
    fn func_80832440(&mut self) {
        self.melee_weapon_state = 0;
        self.unk_6AD = 0;
        self.state1 &= !(STATE1_13 | STATE1_14 | STATE1_20 | STATE1_21);
        self.state2 &= !((1 << 4) | (1 << 7) | STATE2_18);
        self.actor.shape_rot.x = 0;
    }

    /// `func_80833C3C`.
    #[allow(dead_code)]
    fn func_80833C3C(&mut self) {
        self.unk_870 = 0.0;
        self.unk_874 = 0.0;
    }

    /// `func_8083721C`: decelerate by `REG(43) / 100`.
    fn func_8083721C(&mut self) -> bool {
        step_to_f(&mut self.linear_velocity, 0.0, self.regs.reg(43) as f32 / 100.0)
    }

    /// `func_80836FAC`: target speed and yaw from the stick. `arg4` selects the curve used
    /// when starting from rest (0.018) versus the linear one (0).
    fn func_80836FAC(&self, arg4: f32) -> (bool, f32, i16) {
        if self.unk_6AD != 0 || self.state1 & 1 != 0 {
            return (false, 0.0, self.actor.shape_rot.y);
        }
        let mut speed = self.s.stick_mag;
        let yaw = self.s.stick_angle;
        if arg4 != 0.0 {
            speed -= 20.0;
            if speed < 0.0 {
                speed = 0.0;
            } else {
                let t = 1.0 - cos_s(f2s(speed * 450.0));
                speed = (t * t) * 30.0 + 7.0;
            }
        } else {
            speed *= 0.8;
        }
        if self.s.stick_mag != 0.0 {
            let slope = sin_s(self.unk_898).clamp(0.0, 0.6);
            let mut limit = self.unk_880;
            if self.unk_6C4 != 0.0 {
                limit = (limit - self.unk_6C4 * 0.008).max(2.0);
            }
            speed = speed * 0.14 - 8.0 * slope * slope;
            speed = speed.clamp(0.0, limit);
            return (true, speed, yaw);
        }
        (false, speed, yaw)
    }

    /// `func_80837268`: target speed and world yaw. Returns false (and a yaw that keeps the
    /// current facing) when the stick is idle.
    fn func_80837268(&self, env: &Env, arg3: f32) -> (bool, f32, i16) {
        let (moving, speed, yaw) = self.func_80836FAC(arg3);
        if !moving {
            // No target and not in parallel (Z) mode.
            (false, speed, self.actor.shape_rot.y)
        } else {
            (true, speed, yaw.wrapping_add(env.cam_input_yaw))
        }
    }

    /// `func_80837348`: the per-action list of interrupts (`D_80854448`). Only index 6
    /// (`func_8083C1DC`, roll on A) is in scope; the others need items, targeting, doors,
    /// NPCs or climbing and report "not taken".
    fn func_80837348(&mut self, env: &Env, list: &[i8], arg3: bool) -> bool {
        if self.state1 & (1 | (1 << 7) | STATE1_29) != 0 {
            return false;
        }
        if arg3 {
            // D_808535E0 = func_80836670(): upper-body item action; nothing held → 0.
            self.s.item_action = 0;
        }
        let mut i = 0;
        loop {
            let e = list[i];
            let idx = e.unsigned_abs() as usize;
            if idx == 6 && self.func_8083C1DC(env) {
                return true;
            }
            if e < 0 {
                break;
            }
            i += 1;
        }
        false
    }

    // Interrupt lists (`D_808543E0` ...) as indices into D_80854448.
    const D_80854418: &'static [i8] = &[0, 11, 1, 2, 3, 5, 4, 9, 8, 7, -6];
    const D_80854424: &'static [i8] = &[0, 11, 1, 2, 3, 12, 5, 4, 9, 8, 7, -6];
    const D_80854414: &'static [i8] = &[-7];

    /// `func_8083C1DC`: A pressed → roll if the stick points forward.
    fn func_8083C1DC(&mut self, env: &Env) -> bool {
        if !self.func_80833B54() && self.s.item_action == 0 && self.state1 & (1 << 23) == 0 && self.input.press.held(BTN_A) {
            if self.func_8083BC7C(env) {
                return true;
            }
            // Otherwise A toggles "sword out" (PLAYER_STATE2_20) / puts the sword away.
            self.state2 ^= 1 << 20;
        }
        false
    }

    /// `func_8083BC7C`.
    fn func_8083BC7C(&mut self, env: &Env) -> bool {
        if self.stick_dir() == 0 && self.s.floor_type != FLOOR_TYPE_7 {
            self.func_8083BC04(env.data);
            return true;
        }
        false
    }

    /// `func_8083BC04`: start a roll.
    fn func_8083BC04(&mut self, data: &GameData) {
        self.setup_action(data, Action::Roll, 0);
        let a = self.anim(data, group::ROLL);
        self.skel.play_once_set_speed(data, a, 1.25 * self.s.speed_scale);
    }

    /// `func_80833B54`: locked-on target (none here).
    fn func_80833B54(&mut self) -> bool {
        if self.state1 & STATE1_4 != 0 {
            self.state1 &= !STATE1_4;
            if self.linear_velocity == 0.0 {
                self.current_yaw = self.actor.shape_rot.y;
            }
        }
        false
    }

    /// `func_8083DF68`: accelerate towards `speed` (`REG(19)`, 1.5) and turn (`REG(27)`).
    fn func_8083DF68(&mut self, speed: f32, yaw: i16) {
        asym_step_to_f(&mut self.linear_velocity, speed, self.regs.reg(19) as f32 / 100.0, 1.5);
        scaled_step_to_s(&mut self.current_yaw, yaw, self.regs.reg(27));
    }

    /// `func_8083DFE0`: air control.
    fn func_8083DFE0(&mut self, speed: f32, yaw: i16) {
        let yaw_diff = self.current_yaw.wrapping_sub(yaw);
        if self.melee_weapon_state == 0 {
            let rsl = self.regs.run_speed_limit();
            self.linear_velocity = self.linear_velocity.clamp(-rsl, rsl);
        }
        if abs16(yaw_diff) > 0x6000 {
            if step_to_f(&mut self.linear_velocity, 0.0, 1.0) {
                self.current_yaw = yaw;
            }
        } else {
            asym_step_to_f(&mut self.linear_velocity, speed, 0.05, 0.1);
            scaled_step_to_s(&mut self.current_yaw, yaw, 200);
        }
    }

    /// `func_8083C484`: sharp reversal while running → brake first.
    fn func_8083C484(&mut self, speed: &mut f32, yaw: &mut i16) -> bool {
        let d = self.current_yaw.wrapping_sub(*yaw);
        if abs16(d) > 0x6000 {
            if self.func_8083721C() {
                *speed = 0.0;
                *yaw = self.current_yaw;
            } else {
                return true;
            }
        }
        false
    }

    // ================================================================================
    // Transitions

    /// `func_80853080`: stand still, playing the wait animation.
    fn func_80853080(&mut self, data: &GameData) {
        self.setup_action(data, Action::StandingStill, 1);
        let a = self.anim(data, group::WAIT);
        self.func_80832B0C(data, a);
        self.current_yaw = self.actor.shape_rot.y;
    }

    /// `func_80832B0C`: play once, blending in over 6 frames.
    fn func_80832B0C(&mut self, data: &GameData, a: AnimId) {
        let last = data.anims[a].last_frame();
        self.skel.change(data, a, 1.0, 0.0, last, ANIMMODE_ONCE, -6.0);
    }

    /// `func_80839FFC`: switch to the standing action without touching the animation.
    fn func_80839FFC(&mut self, data: &GameData) {
        // Not targeting, not in parallel mode.
        self.setup_action(data, Action::StandingStill, 1);
    }

    /// `func_8083A060`.
    fn func_8083A060(&mut self, data: &GameData) {
        self.func_80839FFC(data);
    }

    /// `func_8083A098`: stand, playing `anim` once (landing animations).
    fn func_8083A098(&mut self, data: &GameData, anim: AnimId) {
        self.func_8083A060(data);
        self.skel.play_once_set_speed(data, anim, self.s.speed_scale);
    }

    /// `func_8083C858`: start walking/running.
    fn func_8083C858(&mut self, data: &GameData) {
        self.setup_action(data, Action::Run, 1);
        let a = self.anim(data, group::RUN);
        self.skel.change(data, a, 1.0, 0.0, 0.0, ANIMMODE_LOOP, -6.0);
        self.unk_89C = 0;
        self.unk_864 = 0.0;
        self.unk_868 = 0.0;
    }

    /// `func_8083C8DC`: face `yaw` and start running.
    fn func_8083C8DC(&mut self, data: &GameData, yaw: i16) {
        self.actor.shape_rot.y = yaw;
        self.current_yaw = yaw;
        self.func_8083C858(data);
    }

    /// `func_8083CD54`: turn in place towards `yaw`.
    fn func_8083CD54(&mut self, data: &GameData, yaw: i16) {
        self.current_yaw = yaw;
        self.setup_action(data, Action::Turn, 1);
        self.unk_87E = (1200.0 * self.s.speed_scale) as i16;
        let a = self.anim(data, group::TURN);
        self.skel.change(data, a, 1.0, 0.0, 0.0, ANIMMODE_LOOP, -6.0);
    }

    /// `func_8083C0E8`: finish a turn.
    fn func_8083C0E8(&mut self, data: &GameData) {
        self.setup_action(data, Action::StandingStill, 1);
        let a = self.anim(data, group::WAIT);
        self.skel.play_once(data, a);
        self.current_yaw = self.actor.shape_rot.y;
    }

    /// `func_8083BF50`: the stop animation, picked by the walk phase so the feet match.
    fn func_8083BF50(&mut self, data: &GameData) {
        let mut sp30 = self.unk_868 - 3.0;
        if sp30 < 0.0 {
            sp30 += 29.0;
        }
        let anim;
        if sp30 < 14.0 {
            anim = self.anim(data, group::END_WALK_A);
            sp30 = 11.0 - sp30;
            if sp30 < 0.0 {
                sp30 = 1.375 * -sp30;
            }
            sp30 /= 11.0;
        } else {
            anim = self.anim(data, group::END_WALK_B);
            sp30 = 26.0 - sp30;
            if sp30 < 0.0 {
                sp30 = 2.0 * -sp30;
            }
            sp30 /= 12.0;
        }
        let last = data.anims[anim].last_frame();
        self.skel.change(data, anim, 1.0, 0.0, last, ANIMMODE_ONCE, 4.0 * sp30);
        self.current_yaw = self.actor.shape_rot.y;
    }

    /// `func_8083C0B8`: stop running.
    fn func_8083C0B8(&mut self, data: &GameData) {
        self.func_80839FFC(data);
        self.func_8083BF50(data);
    }

    /// `func_808409CC`: the next idle animation: the plain wait every other time, otherwise
    /// a random fidget from `D_80853D7C`.
    fn func_808409CC(&mut self, data: &GameData) {
        self.unk_6AC = (self.unk_6AC + 1) & 1;
        let anim = if self.unk_6AC != 0 {
            self.state2 &= !STATE2_28;
            self.anim(data, group::WAIT)
        } else {
            self.state2 |= STATE2_28;
            // Room behaviour type 2 is 0 (normal); health not critical.
            let mut sp38 = 0usize;
            let sp34 = (self.rand_zero_one() * 5.0) as i32;
            // Right hand is open (nothing held), so only variants 1 and 2 qualify.
            if sp34 < 4 && sp34 != 0 && sp34 != 3 {
                sp38 = sp34 as usize + 9;
            }
            let pair = data.idle_variants[sp38];
            if self.model_anim_type != 1 { pair[1] } else { pair[0] }
        };
        let last = data.anims[anim].last_frame();
        self.skel.change(data, anim, (2.0 / 3.0) * self.s.speed_scale, 0.0, last, ANIMMODE_ONCE, -6.0);
    }

    // ================================================================================
    // Look and lean (head/upper body rotations applied at draw time)

    /// `func_808369C8`.
    fn func_808369C8(v: &mut i16, arg1: i16, arg2: i16, arg3: i16, arg4: i16, arg5: i16) -> i16 {
        let t1 = arg4.wrapping_sub(*v);
        let t2 = t1.clamp(arg5.wrapping_neg().min(arg5), arg5.max(arg5.wrapping_neg()));
        *v = v.wrapping_add(t1.wrapping_sub(t2));
        scaled_step_to_s(v, arg1, arg2);
        let t3 = *v;
        if *v < -arg3 {
            *v = -arg3;
        } else if *v > arg3 {
            *v = arg3;
        }
        t3.wrapping_sub(*v)
    }

    /// `func_80836AB8(this, 0)`: spread the focus rotation over head and torso.
    fn func_80836AB8(&mut self) -> i16 {
        let mut var = self.actor.shape_rot.y;
        let fx = self.actor.focus_rot.x;
        let inner = Self::func_808369C8(&mut self.unk_6B6, fx, 600, 10000, fx, 0);
        let b6 = self.unk_6B6;
        Self::func_808369C8(&mut self.unk_6BC, inner, 200, 4000, b6, 10000);
        let mut sp36 = self.actor.focus_rot.y.wrapping_sub(var);
        let be = self.unk_6BE;
        Self::func_808369C8(&mut sp36, 0, 200, 24000, be, 8000);
        var = self.actor.focus_rot.y.wrapping_sub(sp36);
        let be = self.unk_6BE;
        Self::func_808369C8(&mut self.unk_6B8, sp36.wrapping_sub(be), 200, 8000, sp36, 8000);
        let b8 = self.unk_6B8;
        Self::func_808369C8(&mut self.unk_6BE, sp36, 200, 8000, b8, 8000);
        self.unk_6AE |= 0xD9;
        var
    }

    /// `func_8083DC54`: look at the ground ahead.
    fn func_8083DC54(&mut self, env: &Env) {
        let mut sp46 = 0i16;
        let p = self.func_808395DC(self.actor.world_pos, HEAD_PROBE);
        let (floor, _) = env.col.entity_raycast_down(p);
        if floor > BGCHECK_Y_MIN {
            let t = atan2_s(40.0, self.actor.world_pos.y - floor);
            sp46 = t.clamp(-4000, 4000);
        }
        self.actor.focus_rot.y = self.actor.shape_rot.y;
        smooth_step_to_s(&mut self.actor.focus_rot.x, sp46, 14, 4000, 30);
        self.func_80836AB8();
    }

    /// `func_8083DDC8`: lean into turns when running fast.
    fn func_8083DDC8(&mut self, env: &Env) {
        if self.linear_velocity > 5.0 {
            let t1 = f2s(self.linear_velocity * 200.0).clamp(-4000, 4000);
            let t2 = f2s(self.current_yaw.wrapping_sub(self.actor.shape_rot.y) as f32 * self.linear_velocity * 0.1);
            let t2 = t2.wrapping_neg().clamp(-4000, 4000);
            scaled_step_to_s(&mut self.unk_6BC, t1, 900);
            self.unk_6B6 = f2s(-(self.unk_6BC as f32) * 0.5);
            scaled_step_to_s(&mut self.unk_6BA, t2, 300);
            scaled_step_to_s(&mut self.unk_6C0, t2, 200);
            self.unk_6AE |= 0x168;
        } else {
            self.func_8083DC54(env);
        }
    }

    /// `func_808471F4`.
    fn func_808471F4(v: &mut i16) {
        let step = f2s((abs16(*v) as f32 * 100.0) / 1000.0).clamp(400, 4000);
        scaled_step_to_s(v, 0, step);
    }

    /// `func_80847298`: relax whichever look/lean rotations were not driven last frame.
    fn func_80847298(&mut self) {
        let f = self.unk_6AE;
        if f & 2 == 0 {
            let mut d = self.actor.focus_rot.y.wrapping_sub(self.actor.shape_rot.y);
            Self::func_808471F4(&mut d);
            self.actor.focus_rot.y = self.actor.shape_rot.y.wrapping_add(d);
        }
        if f & 1 == 0 {
            Self::func_808471F4(&mut self.actor.focus_rot.x);
        }
        if f & 8 == 0 {
            Self::func_808471F4(&mut self.unk_6B6);
        }
        if f & 0x40 == 0 {
            Self::func_808471F4(&mut self.unk_6BC);
        }
        if f & 4 == 0 {
            Self::func_808471F4(&mut self.actor.focus_rot.z);
        }
        if f & 0x10 == 0 {
            Self::func_808471F4(&mut self.unk_6B8);
        }
        if f & 0x20 == 0 {
            Self::func_808471F4(&mut self.unk_6BA);
        }
        if f & 0x80 == 0 {
            if self.unk_6B0 != 0 {
                Self::func_808471F4(&mut self.unk_6B0);
            } else {
                Self::func_808471F4(&mut self.unk_6BE);
            }
        }
        if f & 0x100 == 0 {
            Self::func_808471F4(&mut self.unk_6C0);
        }
        self.unk_6AE = 0;
    }

    /// `func_808368EC`: facing follows the movement direction while running/rolling.
    fn func_808368EC(&mut self) {
        let prev = self.actor.shape_rot.y;
        if self.state2 & (STATE2_5 | STATE2_6) != 0 && self.state2 & STATE2_6 == 0 {
            scaled_step_to_s(&mut self.actor.shape_rot.y, self.current_yaw, 2000);
        }
        self.unk_87C = self.actor.shape_rot.y.wrapping_sub(prev);
    }

    // ================================================================================
    // Walk / run animation

    /// `func_8084021C`: did the phase cross `arg2`/`arg3` this step (footstep timing)?
    fn func_8084021C(arg0: f32, arg1: f32, arg2: f32, mut arg3: f32) -> bool {
        if arg3 == 0.0 && arg1 > 0.0 {
            arg3 = arg2;
        }
        let t = (arg0 + arg1) - arg3;
        t * arg1 >= 0.0 && (t - arg1) * arg1 < 0.0
    }

    /// `func_8084029C`: advance the walk phase `unk_868` (29-frame cycle).
    fn func_8084029C(&mut self, arg1: f32) {
        let arg1 = (arg1 * UPDATE_SCALE).clamp(-7.25, 7.25);
        if Self::func_8084021C(self.unk_868, arg1, 29.0, 10.0) || Self::func_8084021C(self.unk_868, arg1, 29.0, 24.0) {
            // Footstep sound; running steps set PLAYER_STATE2_3.
            if self.linear_velocity > 4.0 {
                self.state2 |= STATE2_3;
            }
        }
        self.unk_868 += arg1;
        if self.unk_868 < 0.0 {
            self.unk_868 += 29.0;
        } else if self.unk_868 >= 29.0 {
            self.unk_868 -= 29.0;
        }
    }

    /// `func_80833438`: which run animation (damage, iron boots, normal).
    fn func_80833438(&self, data: &GameData) -> AnimId {
        if self.unk_890 != 0 { self.anim(data, group::DAMAGE_RUN) } else { self.anim(data, group::RUN) }
    }

    /// `func_80841CC4`: walk animation, blended towards climbing up/down on slopes.
    fn func_80841CC4(&mut self, data: &GameData, to_morph: bool) {
        let target = if abs16(self.s.facing_slope) < 3640 { 0 } else { self.s.facing_slope.clamp(-10922, 10922) };
        scaled_step_to_s(&mut self.unk_89C, target, 400);
        let walk = self.anim(data, group::WALK);
        if self.model_anim_type == 3 || (self.unk_89C == 0 && self.unk_6C4 <= 0.0) {
            if to_morph {
                self.skel.load_to_morph(data, walk, self.unk_868);
            } else {
                self.skel.load_to_joint(data, walk, self.unk_868);
            }
            return;
        }
        let mut rate = if self.unk_89C != 0 { self.unk_89C as f32 / 10922.0 } else { self.unk_6C4 * 0.0006 };
        rate *= self.linear_velocity.abs() * 0.5;
        if rate > 1.0 {
            rate = 1.0;
        }
        let anim = if rate < 0.0 {
            rate = -rate;
            data.anim("link_normal_climb_down")
        } else {
            data.anim("link_normal_climb_up")
        };
        if to_morph {
            self.skel.blend_to_morph(data, walk, self.unk_868, anim, self.unk_868, rate);
        } else {
            self.skel.blend_to_joint(data, walk, self.unk_868, anim, self.unk_868, rate);
        }
    }

    /// `func_80841EE4`: the walk→run blend by speed (`REG(35..38)`, `REG(48)`).
    fn func_80841EE4(&mut self, data: &GameData) {
        let r = |n| self.regs.reg(n) as f32 / 1000.0;
        let temp1;
        if self.unk_864 < 1.0 {
            self.func_8084029C(r(35));
            let walk = self.anim(data, group::WALK);
            self.skel.load_to_joint(data, walk, self.unk_868);
            self.unk_864 = (self.unk_864 + UPDATE_SCALE).min(1.0);
            temp1 = self.unk_864;
        } else {
            let temp2 = self.linear_velocity - (self.regs.reg(48) as f32 / 100.0);
            if temp2 < 0.0 {
                temp1 = 1.0;
                self.func_8084029C(r(35) + r(36) * self.linear_velocity);
                self.func_80841CC4(data, false);
            } else {
                let mut t = r(37) * temp2;
                if t < 1.0 {
                    self.func_8084029C(r(35) + r(36) * self.linear_velocity);
                } else {
                    t = 1.0;
                    self.func_8084029C(1.2 + r(38) * temp2);
                }
                temp1 = t;
                self.func_80841CC4(data, true);
                let run = self.func_80833438(data);
                self.skel.load_to_joint(data, run, self.unk_868 * (20.0 / 29.0));
            }
        }
        if temp1 < 1.0 {
            self.skel.interp_joint_morph(1.0 - temp1);
        }
    }

    // ================================================================================
    // Actions

    /// `func_80840BC8`: standing still. Plays idle animations, turns in place towards the
    /// stick, and starts running once the stick gives a speed.
    fn func_80840BC8(&mut self, env: &Env) {
        let data = env.data;
        let done = self.skel.update(data);
        if done {
            if self.unk_850 != 0 {
                // DECR(this->unk_850)
                if self.unk_850 != 0 {
                    self.unk_850 -= 1;
                    if self.unk_850 == 0 {
                        self.skel.end_frame = self.skel.anim_length - 1.0;
                    }
                }
                // Fall-damage stagger: the root bobs by ±0x28.
                self.skel.joint[0][1] = self.skel.joint[0][1].wrapping_add(((self.unk_850 & 1) * 0x50) - 0x28);
            } else {
                self.func_80832DBC();
                self.func_808409CC(data);
            }
        }
        self.func_8083721C();
        if self.unk_850 == 0 && !self.func_80837348(env, Self::D_80854418, true) {
            if self.func_80833B54() {
                return;
            }
            let (_, speed, yaw) = self.func_80837268(env, 0.018);
            if speed != 0.0 {
                self.func_8083C8DC(data, yaw);
                return;
            }
            let d = yaw.wrapping_sub(self.actor.shape_rot.y);
            if abs16(d) > 800 {
                self.func_8083CD54(data, yaw);
                return;
            }
            scaled_step_to_s(&mut self.actor.shape_rot.y, yaw, 1200);
            self.current_yaw = self.actor.shape_rot.y;
            if self.anim(data, group::WAIT) == self.skel.animation {
                self.func_8083DC54(env);
            }
        }
    }

    /// `func_80841BA8`: turning in place.
    fn func_80841BA8(&mut self, env: &Env) {
        let data = env.data;
        self.skel.update(data);
        let (_, speed, yaw) = self.func_80837268(env, 0.018);
        if !self.func_80837348(env, Self::D_80854414, true) {
            if speed != 0.0 {
                self.actor.shape_rot.y = yaw;
                self.func_8083C858(data);
            } else if scaled_step_to_s(&mut self.actor.shape_rot.y, yaw, self.unk_87E) {
                self.func_8083C0E8(data);
            }
            self.current_yaw = self.actor.shape_rot.y;
        }
    }

    /// `func_80842180`: walking and running.
    fn func_80842180(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5;
        self.func_80841EE4(data);
        if !self.func_80837348(env, Self::D_80854424, true) {
            // func_80833C04: targeting/parallel → never here.
            let (_, mut speed, mut yaw) = self.func_80837268(env, 0.018);
            if !self.func_8083C484(&mut speed, &mut yaw) {
                self.func_8083DF68(speed, yaw);
                self.func_8083DDC8(env);
                if self.linear_velocity == 0.0 && speed == 0.0 {
                    self.func_8083C0B8(data);
                }
            }
        }
    }

    /// `func_80844708`: rolling.
    fn func_80844708(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5;
        let done = self.skel.update(data);
        // Frame 8: brief invincibility (func_80837AFC) — no damage system here.
        // func_80842964 (first person, items, grabbing): not taken.
        if self.unk_850 != 0 {
            step_to_f(&mut self.linear_velocity, 0.0, 2.0);
            let temp = self.func_808374A0(env, 5.0);
            if temp != 0 && (temp > 0 || done) {
                self.func_8083A060(data);
            }
        } else {
            if self.linear_velocity >= 7.0
                && self.actor.bg_check_flags & BGCHECKFLAG_PLAYER_WALL_INTERACT != 0
                && self.s.wall_move_diff < 0x2000
            {
                // Bonk.
                let a = self.anim(data, group::ROLL_BONK);
                self.skel.play_once(data, a);
                self.linear_velocity = -self.linear_velocity;
                self.unk_850 = 1;
                self.note("roll bonk");
                return;
            }
            // Past frame 15 func_80850224 could chain into a sword attack: no sword.
            if self.skel.cur_frame >= 20.0 {
                self.func_8083A060(data);
                return;
            }
            let (_, mut speed, _) = self.func_80837268(env, 0.018);
            speed *= 1.5;
            if speed < 3.0 || self.stick_dir() != 0 {
                speed = 3.0;
            }
            let yaw = self.actor.shape_rot.y;
            self.func_8083DF68(speed, yaw);
        }
    }

    /// `func_808374A0`: near the end of an animation, allow interrupts or restarting to run.
    /// Returns 1 to move on, 0 if an interrupt took over, -1 to keep playing.
    fn func_808374A0(&mut self, env: &Env, arg3: f32) -> i32 {
        if (self.skel.end_frame - arg3) <= self.skel.cur_frame {
            if self.func_80837348(env, Self::D_80854418, true) {
                return 0;
            }
            if self.func_80837268(env, 0.018).0 {
                return 1;
            }
        }
        -1
    }

    /// `func_80843E64`: landing. Returns 1/2 for a damaging fall (≥ 400 / 800), 0 otherwise.
    fn func_80843E64(&mut self) -> i32 {
        let sp34 = if self.s.floor_type == FLOOR_TYPE_6 || self.s.floor_type == FLOOR_TYPE_9 { 0 } else { self.fall_distance as i32 };
        step_to_f(&mut self.linear_velocity, 0.0, 1.0);
        self.state1 &= !(STATE1_18 | STATE1_19);
        if sp34 >= 400 {
            let idx = if self.fall_distance < 800 { 0 } else { 1 };
            self.note(format!("fall damage {}", if idx == 0 { "-8 (half heart)" } else { "-16 (1 heart)" }));
            return idx + 1;
        }
        0
    }

    /// `func_8084411C`: in the air.
    fn func_8084411C(&mut self, env: &Env) {
        let data = env.data;
        let (_, speed, yaw) = self.func_80837268(env, 0.0);
        if !self.grounded() {
            self.skel.update(data);
            if self.state2 & STATE2_19 == 0 {
                self.func_8083DFE0(speed, yaw);
            }
            // func_80836670 (items) → 0; func_8083BBA0 (jump slash) needs a sword.
            if self.actor.velocity.y < 0.0 {
                if self.unk_850 >= 0 {
                    if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 || self.unk_850 == 0 || self.fall_distance > 0 {
                        let a = data.anim("link_normal_landing");
                        self.skel.change(data, a, 1.0, 0.0, 0.0, ANIMMODE_ONCE, 8.0);
                        self.unk_850 = -1;
                    }
                } else {
                    if self.unk_850 == -1 && self.fall_distance > 120 && self.s.floor_dist > 280.0 {
                        self.unk_850 = -2;
                    }
                    if self.actor.bg_check_flags & BGCHECKFLAG_PLAYER_WALL_INTERACT != 0
                        && self.state2 & STATE2_19 == 0
                        && self.state1 & (STATE1_11 | STATE1_27) == 0
                        && self.linear_velocity > 0.0
                        && self.unk_88C >= 2
                        && self.wall_height < 150.0
                        && ((self.actor.world_pos.y - self.actor.floor_height) + self.wall_height) > 70.0 * self.age.translation_scale
                    {
                        self.note("mid-air ledge grab (func_8083A5C4) would start here");
                    }
                }
            }
        } else {
            let mut anim = self.anim(data, group::LANDING);
            if self.skel.animation == data.anim("link_normal_run_jump") {
                anim = data.anim("link_normal_run_jump_end");
            } else if self.fall_distance <= 80 {
                anim = self.anim(data, group::SHORT_LANDING);
            } else if self.fall_distance < 800 && self.stick_dir() == 0 && self.state1 & STATE1_11 == 0 {
                self.func_8083BC04(data);
                return;
            }
            let sp3c = self.func_80843E64();
            if sp3c > 0 {
                let a = self.anim(data, group::LANDING);
                self.func_8083A098(data, a);
                self.skel.end_frame = 8.0;
                self.unk_850 = if sp3c == 1 { 10 } else { 20 };
            } else if sp3c == 0 {
                self.func_8083A098(data, anim);
            }
        }
    }

    // ================================================================================
    // Draw-time state

    /// The joint table the skeleton is drawn with, including
    /// `Player_OverrideLimbDrawGameplayCommon`'s child root scaling and the `unk_6C4` sink.
    pub fn draw_joints(&self) -> oot_core::anim::JointTable {
        let mut rot: Vec<[i16; 3]> = self.skel.joint.to_vec();
        let mut root = [rot[0][0] as f32, rot[0][1] as f32, rot[0][2] as f32];
        if !self.adult {
            let mf = self.skel.move_flags;
            if mf & 4 == 0 || mf & 1 != 0 {
                root[0] *= 0.64;
                root[2] *= 0.64;
            }
            if mf & 4 == 0 || mf & 2 != 0 {
                root[1] *= 0.64;
            }
        }
        root[1] -= self.unk_6C4;
        rot[0] = [root[0] as i16, root[1] as i16, root[2] as i16];
        oot_core::anim::JointTable { rot, face: self.skel.face }
    }

    /// Head (x, y, z additions) and upper-body (y, x, z pre-rotations) look/lean angles.
    pub fn look_rotations(&self) -> LookRotations {
        LookRotations {
            head: [self.unk_6BA, self.unk_6B8.wrapping_neg(), self.unk_6B6],
            upper_y: self.unk_6BE,
            upper_x: self.unk_6BC,
            upper_z: self.unk_6C0,
        }
    }

    pub fn wall_flag_3(&self) -> bool {
        self.s.wall_flags & WALL_FLAG_3 != 0
    }
}

/// Rotations `Player_OverrideLimbDrawGameplayCommon` applies to the head and upper body.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LookRotations {
    /// Added to the head limb's (x, y, z) rotation.
    pub head: [i16; 3],
    /// Applied before the upper body limb's own transform: RotY, then RotX, then RotZ.
    pub upper_y: i16,
    pub upper_x: i16,
    pub upper_z: i16,
}
