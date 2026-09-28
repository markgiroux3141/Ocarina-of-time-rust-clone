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
use crate::bgcheck::{BGCHECK_Y_MIN, PolyId, StaticCollision, WALL_FLAG_0, WALL_FLAG_1, WALL_FLAG_3, udist_plane_to_pos};
use crate::data::{AgeProperties, AnimId, GameData, Regs};
use crate::input::{BTN_A, BTN_Z, Input, stick_to_mag_angle};
use crate::target::{ACTOR_FLAG_27, TargetActor, TargetView};
use crate::math::*;
use crate::skelanime::{ANIMMODE_LOOP, ANIMMODE_ONCE, SkelAnime};

// stateFlags1
pub const STATE1_2: u32 = 1 << 2;
pub const STATE1_3: u32 = 1 << 3;
pub const STATE1_8: u32 = 1 << 8; // item change pending
pub const STATE1_10: u32 = 1 << 10; // getting an item
pub const STATE1_24: u32 = 1 << 24;
pub const STATE1_4: u32 = 1 << 4; // locked on (func_8008E9C4)
pub const STATE1_7: u32 = 1 << 7; // dead
pub const STATE1_15: u32 = 1 << 15; // Z pressed this lock-on
pub const STATE1_23: u32 = 1 << 23; // riding
pub const STATE1_25: u32 = 1 << 25; // boomerang in flight
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
pub const STATE2_10: u32 = 1 << 10; // under the water surface
pub const STATE2_11: u32 = 1 << 11; // diving from the surface
pub const STATE2_1: u32 = 1 << 1;
pub const STATE2_13: u32 = 1 << 13; // keep the target (Switch Z-targeting)
pub const STATE2_21: u32 = 1 << 21;
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
pub const STATE2_17: u32 = 1 << 17; // spin attack
pub const STATE2_30: u32 = 1 << 30; // stab lunge
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
    pub const TARGET_EXIT_R: usize = 27;
    pub const TARGET_EXIT_L: usize = 28;
    pub const PARALLEL_BACKWALK: usize = 31;
    pub const TARGET_WAIT_L: usize = 5;
    pub const TARGET_WAIT_R: usize = 6;
    pub const TARGET_ENTER: usize = 7;
    pub const SHORT_LANDING_: usize = 15;
    pub const PARALLEL_SIDEWALK: usize = 23;
    pub const SIDESTEP_R: usize = 24;
    pub const SIDESTEP_L: usize = 25;
    /// Climb up from a hang (`unk_84F` > 0).
    pub const HANG_CLIMB_UP: usize = 38;
    /// Mid-air ledge grab.
    pub const HANG_GRAB: usize = 39;
    /// Hanging loop after a mid-air grab.
    pub const HANG_WAIT: usize = 40;
    /// Climb up after a mid-air grab (`unk_84F` < 0).
    pub const HANG_GRAB_CLIMB_UP: usize = 41;
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
    /// `func_8084BBE4`: hanging from a ledge.
    Hang,
    /// `func_8084BDFC`: climbing up from a hang.
    ClimbUp,
    /// `func_80845668`: stepping or jumping up onto a ledge from the ground.
    ClimbLedge,
    /// `func_80840450`: standing locked on to a target.
    TargetIdle,
    /// `func_808407CC`: standing in parallel mode (Z with nothing to target).
    ParallelIdle,
    /// `func_8084227C`: running forward while targeting.
    TargetRun,
    /// `func_8084193C`: sidestepping around a target.
    Sidestep,
    /// `func_808423EC`: stepping back from a locked target.
    TargetBackwalk,
    /// `func_8084251C`: braking after the step back.
    TargetBackBrake,
    /// `func_80840DE4`: walking sideways in parallel mode.
    ParallelWalk,
    /// `func_808414F8`: walking/running backwards in parallel mode.
    ParallelBackwalk,
    /// `func_8084170C`: braking out of a parallel back run.
    ParallelBackBrake,
    /// `func_808417FC`: end of that brake.
    ParallelBackBrakeEnd,
    /// `func_808502D0`: a melee attack (sword slash).
    Attack,
    /// `func_8084D610`: treading water.
    Swim,
    /// `func_8084D84C`: swimming forward.
    SwimMove,
    /// `func_8084DAB4`: swimming while targeting.
    SwimTarget,
    /// `func_8084DC48`: diving.
    Dive,
    /// `func_8084E1EC`: surfacing after a dive.
    Surface,
}

/// Player's upper-body action (`this->func_82C`), run from `func_80836670`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpperAction {
    /// `func_8083485C`: nothing in hand (only the shield check).
    Default,
    /// `func_808349DC`: a sword in hand (shield check, pending item change).
    Sword,
    /// `func_80834A2C`: an item change playing on `skelAnime2`.
    Change,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::StandingStill => "func_80840BC8",
            Action::Run => "func_80842180",
            Action::Turn => "func_80841BA8",
            Action::Roll => "func_80844708",
            Action::Midair => "func_8084411C",
            Action::Hang => "func_8084BBE4",
            Action::ClimbUp => "func_8084BDFC",
            Action::ClimbLedge => "func_80845668",
            Action::TargetIdle => "func_80840450",
            Action::ParallelIdle => "func_808407CC",
            Action::TargetRun => "func_8084227C",
            Action::Sidestep => "func_8084193C",
            Action::TargetBackwalk => "func_808423EC",
            Action::TargetBackBrake => "func_8084251C",
            Action::ParallelWalk => "func_80840DE4",
            Action::ParallelBackwalk => "func_808414F8",
            Action::ParallelBackBrake => "func_8084170C",
            Action::ParallelBackBrakeEnd => "func_808417FC",
            Action::Attack => "func_808502D0",
            Action::Swim => "func_8084D610",
            Action::SwimMove => "func_8084D84C",
            Action::SwimTarget => "func_8084DAB4",
            Action::Dive => "func_8084DC48",
            Action::Surface => "func_8084E1EC",
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
    /// `D_80853614` / `D_80853618`: B pressed / held with the weapon already in hand this frame.
    pub d_80853614: bool,
    pub d_80853618: bool,
}

/// Per-frame environment the update needs.
pub struct Env<'a> {
    pub data: &'a GameData,
    pub col: &'a StaticCollision,
    /// `Camera_GetInputDirYaw(GET_ACTIVE_CAM(play))`.
    pub cam_input_yaw: i16,
    /// `play->gameplayFrames` (face alternation).
    pub gameplay_frames: u32,
    /// Targetable actors (index = what `unk_664` holds) and last frame's target context.
    pub targets: &'a [TargetActor],
    pub target: TargetView,
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
    /// `unk_664`: the targeted actor (index into `Env::targets`).
    pub unk_664: Option<usize>,
    /// `unk_66C`: Z-target timer.
    pub unk_66C: i16,
    /// `bodyPartsPos[PLAYER_BODYPART_HEAD]` from the last draw (also `actor.focus.pos`).
    pub head_pos: Vec3,
    /// `skelAnime2`: the upper-body animation (item changes), merged into `skel` by `func_80836670`.
    pub skel2: SkelAnime,
    /// `func_82C`.
    pub upper: UpperAction,
    /// `heldItemActionParam`, `itemActionParam` (PLAYER_AP_*), `heldItemId` (the item being
    /// changed to, as its action param).
    pub held_item_ap: i32,
    pub item_ap: i32,
    pub held_item: Option<i32>,
    /// `modelGroup`, `nextModelGroup` (PLAYER_MODELGROUP_*).
    pub model_group: usize,
    pub next_model_group: usize,
    /// `currentShield` (PLAYER_SHIELD_*: 1 Deku, 2 Hylian).
    pub current_shield: u8,
    /// The item on the B button, as its action param (the equipped sword).
    pub b_item: i32,
    pub unk_15A: usize,
    pub unk_830: f32,
    pub unk_844: i8,
    pub unk_845: i8,
    /// `meleeWeaponAnimation` (PLAYER_MWA_*).
    pub melee_weapon_animation: usize,
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
            unk_664: None,
            unk_66C: 0,
            head_pos: pos + Vec3::Y * 50.0,
            skel2: SkelAnime::new_link(data, data.player_anim(group::WAIT, 0), BASE_TRANSL),
            upper: UpperAction::Default,
            held_item_ap: 0,
            item_ap: 0,
            held_item: None,
            model_group: data.items.model_group("DEFAULT"),
            next_model_group: data.items.model_group("DEFAULT"),
            current_shield: if adult { 2 } else { 1 },
            b_item: data.items.ap(if adult { "SWORD_MASTER" } else { "SWORD_KOKIRI" }),
            unk_15A: 0,
            unk_830: 0.0,
            unk_844: 0,
            unk_845: 0,
            melee_weapon_animation: 0,
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
        // func_808473D4 (do-action HUD): not modelled.
        self.func_80836BEC(env);

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

        // Damage (func_808382DC): not modelled.
        self.func_8083D53C(env);
        self.func_8083AA10(env);

        self.func_8083D6EC();

        self.state1 &= !(STATE1_12 | STATE1_22 | (1 << 1) | (1 << 9));
        self.state2 &= !(STATE2_0 | STATE2_2 | STATE2_3 | STATE2_5 | STATE2_6 | STATE2_8 | (1 << 9) | STATE2_12 | STATE2_14 | STATE2_16 | STATE2_22 | STATE2_26);
        self.state3 &= !STATE3_4;

        self.func_80847298();
        self.func_8083315C(env);

        // D_808535E8: 0.5 while swimming.
        self.s.speed_scale = if self.state1 & STATE1_27 != 0 { 0.5 } else { 1.0 };

        self.s.d_80853614 = false;
        self.s.d_80853618 = false;
        if self.state3 & (1 << 2) == 0 {
            self.run_action(env);
        }

        // Player_UpdateCamAndSeqModes: camera is not ported.
        if self.skel.move_flags & 8 != 0 {
            let s = if self.skel.move_flags & 4 != 0 { 1.0 } else { self.age.translation_scale };
            self.skel.request_move_actor(s);
        }
        self.func_808368EC(env);
        let _ = data;
        self.actor.home_pos = self.actor.world_pos;
    }

    /// `AnimationContext_Update`, run after all actors (only Player here) have updated.
    pub fn finish_frame(&mut self) {
        self.skel.run_queue(&mut self.actor);
    }

    /// The draw-time part of Player that changes its joint table: `func_8008F87C` (foot IK) for
    /// both legs, as `Player_OverrideLimbDrawGameplayCommon` runs it for `PLAYER_LIMB_L_THIGH`
    /// and `PLAYER_LIMB_R_THIGH`. Call after `finish_frame` (Player_Draw follows every update).
    pub fn apply_foot_ik(&mut self, data: &GameData, col: &StaticCollision) -> [crate::footik::LegResult; 2] {
        // Player_OverrideLimbDrawGameplayCommon, PLAYER_LIMB_ROOT: child root scaling, minus unk_6C4.
        let j0 = self.skel.joint[0];
        let mut root = glam::Vec3::new(j0[0] as f32, j0[1] as f32, j0[2] as f32);
        if !self.adult {
            let mf = self.skel.move_flags;
            if mf & 4 == 0 || mf & 1 != 0 {
                root.x *= 0.64;
                root.z *= 0.64;
            }
            if mf & 4 == 0 || mf & 2 != 0 {
                root.y *= 0.64;
            }
        }
        root.y -= self.unk_6C4;
        let r = self.actor.shape_rot;
        let actor = crate::footik::actor_matrix(self.actor.world_pos, self.actor.shape_y_offset, [r.x, r.y, r.z]);
        let rig = &data.rigs[if self.adult { 0 } else { 1 }];
        let legs = [("L_THIGH", "L_SHIN", "L_FOOT"), ("R_THIGH", "R_SHIN", "R_FOOT")];
        let legs = legs.map(|(t, s, f)| {
            crate::footik::solve_leg(&data.foot_ik, rig, col, self.adult, actor, root, &mut self.skel.joint, self.unk_6C4, data.limb(t), data.limb(s), data.limb(f))
        });
        // Player_PostLimbDraw's bodyPartsPos[PLAYER_BODYPART_HEAD]: the head limb's origin (the
        // look rotations of the upper body are left out here).
        let head = crate::footik::limb_matrix(rig, actor, root, &self.skel.joint, data.limb("HEAD"));
        self.head_pos = head.transform_point3(Vec3::ZERO);
        legs
    }

    fn run_action(&mut self, env: &Env) {
        match self.action {
            Action::StandingStill => self.func_80840BC8(env),
            Action::Run => self.func_80842180(env),
            Action::Turn => self.func_80841BA8(env),
            Action::Roll => self.func_80844708(env),
            Action::Midair => self.func_8084411C(env),
            Action::Hang => self.func_8084BBE4(env),
            Action::ClimbUp => self.func_8084BDFC(env),
            Action::ClimbLedge => self.func_80845668(env),
            Action::TargetIdle => self.func_80840450(env),
            Action::ParallelIdle => self.func_808407CC(env),
            Action::TargetRun => self.func_8084227C(env),
            Action::Sidestep => self.func_8084193C(env),
            Action::TargetBackwalk => self.func_808423EC(env),
            Action::TargetBackBrake => self.func_8084251C(env),
            Action::ParallelWalk => self.func_80840DE4(env),
            Action::ParallelBackwalk => self.func_808414F8(env),
            Action::ParallelBackBrake => self.func_8084170C(env),
            Action::ParallelBackBrakeEnd => self.func_808417FC(env),
            Action::Attack => self.func_808502D0(env),
            Action::Swim => self.func_8084D610(env),
            Action::SwimMove => self.func_8084D84C(env),
            Action::SwimTarget => self.func_8084DAB4(env),
            Action::Dive => self.func_8084DC48(env),
            Action::Surface => self.func_8084E1EC(env),
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
    fn func_80839768(&self, env: &Env, off: Vec3) -> Option<(Vec3, PolyId)> {
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
        // func_800432A0: a rotating platform turns Player's currentYaw too.
        self.current_yaw = self.current_yaw.wrapping_add(self.actor.carried_yaw);
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
    fn wall_height_class(&mut self, env: &Env, wp: PolyId, probe: Vec3) -> u8 {
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

    /// `func_8083A6AC`: grab the ledge Player just walked off: a line back towards the previous
    /// position finds the cliff face, and Player hangs from its top (`func_8083A5C4`).
    fn func_8083A6AC(&mut self, env: &Env) -> bool {
        let data = env.data;
        if self.actor.y_dist_to_water < -80.0 && abs16(self.unk_898) < 2730 && abs16(self.unk_89A) < 2730 {
            let d = Vec3::new(self.actor.prev_pos.x - self.actor.world_pos.x, 0.0, self.actor.prev_pos.z - self.actor.world_pos.z);
            let len = (d.x * d.x + d.z * d.z).sqrt();
            let k = if len != 0.0 { 5.0 / len } else { 0.0 };
            let b = Vec3::new(self.actor.prev_pos.x + d.x * k, self.actor.world_pos.y, self.actor.prev_pos.z + d.z * k);
            if let Some((_, poly)) = env.col.entity_line_test(self.actor.world_pos, b, true, false, false, true)
                && abs16(env.col.poly(poly).normal[1]) < 600
            {
                let n = env.col.poly_normal(poly);
                let sp54 = udist_plane_to_pos(n, env.col.poly(poly).dist as f32, self.actor.world_pos);
                let climbable = self.s.floor_property == FLOOR_PROPERTY_6 || env.col.wall_flags(poly) & WALL_FLAG_3 != 0;
                if climbable {
                    // func_8083A5C4 with link_normal_Fclimb_startB, then func_8084BF1C (climbing
                    // down a ladder or vine wall): not ported.
                    self.note("climbing down onto a climbable wall (func_8084BF1C) not ported; falling instead");
                    return false;
                }
                self.func_8083A5C4(data, env, poly, sp54, data.anim("link_normal_fall"));
                self.state1 |= STATE1_13;
                self.state1 &= !STATE1_17;
                return true;
            }
        }
        false
    }

    /// `func_8083A5C4`: start hanging from the top edge of `wall`, `dist` away from its plane.
    fn func_8083A5C4(&mut self, data: &GameData, env: &Env, wall: PolyId, dist: f32, anim: AnimId) {
        let n = env.col.poly_normal(wall);
        self.setup_action(data, Action::Hang, 0);
        // func_80832564: func_80832440, and drop a held actor (none).
        self.func_80832440();
        self.skel.play_once(data, anim);
        self.actor.world_pos.x -= (dist + 1.0) * n.x;
        self.actor.world_pos.z -= (dist + 1.0) * n.z;
        self.current_yaw = atan2_s(n.z, n.x);
        self.actor.shape_rot.y = self.current_yaw;
        self.func_80832224();
        self.func_80832CFC();
    }

    /// `func_80832224`.
    fn func_80832224(&mut self) {
        self.func_80832210();
        self.unk_6AD = 0;
    }

    /// `func_80832E48`: apply the root motion accumulated so far to the actor (`flags` 1: x/z,
    /// 2: y, 4: y unscaled), then fold the root yaw into the facing.
    fn func_80832E48(&mut self, flags: u16) {
        self.skel.move_flags = flags;
        self.skel.prev_transl = self.skel.base_transl;
        let mut pos = self.skel.update_translation(self.actor.shape_rot.y);
        if flags & 1 != 0 {
            if !self.adult {
                pos.x *= 0.64;
                pos.z *= 0.64;
            }
            self.actor.world_pos.x += pos.x * self.actor.scale.x;
            self.actor.world_pos.z += pos.z * self.actor.scale.z;
        }
        if flags & 2 != 0 {
            if flags & 4 == 0 {
                pos.y *= self.age.translation_scale;
            }
            self.actor.world_pos.y += pos.y * self.actor.scale.y;
        }
        self.func_808322FC();
    }

    /// `func_80837B60`: bake the whole root offset (the hanging body) into the position.
    fn func_80837B60(&mut self) {
        self.skel.prev_transl = self.skel.joint[0];
        self.func_80832E48(3);
    }

    /// `func_80837B9C`: let go and fall.
    fn func_80837B9C(&mut self, data: &GameData) {
        self.setup_action(data, Action::Midair, 0);
        let a = data.anim("link_normal_landing_wait");
        self.skel.play_loop(data, a);
        self.unk_850 = 1;
        if self.unk_6AD != 3 {
            self.unk_6AD = 0;
        }
    }

    /// `func_8084BBE4`: hanging from a ledge. Any stick direction climbs up; A lets go.
    fn func_8084BBE4(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_6;
        let fall = data.anim("link_normal_fall");
        if self.skel.update(data) {
            let anim = if self.unk_84F > 0 { data.anim("link_normal_fall_wait") } else { self.anim(data, group::HANG_WAIT) };
            self.skel.play_loop(data, anim);
        } else if self.unk_84F == 0 {
            let f = if self.skel.animation == fall { 11.0 } else { 1.0 };
            if self.skel.on_frame(f) {
                // func_80832770: grab sound.
                self.unk_84F = if self.skel.animation == fall { 1 } else { -1 };
            }
        }
        scaled_step_to_s(&mut self.actor.shape_rot.y, self.current_yaw, 0x800);
        if self.unk_84F != 0 {
            if self.unk_847[self.unk_846 as usize] >= 0 {
                let anim = if self.unk_84F > 0 { self.anim(data, group::HANG_CLIMB_UP) } else { self.anim(data, group::HANG_GRAB_CLIMB_UP) };
                self.func_8083A9B8(data, anim);
                return;
            }
            // `actor.shape.feetFloorFlag` (feet touching a floor, set at draw time) also lets go;
            // not modelled.
            if self.input.cur.held(BTN_A) {
                self.func_80837B60();
                self.linear_velocity = if self.unk_84F < 0 { -0.8 } else { 0.8 };
                self.func_80837B9C(data);
                self.state1 &= !(STATE1_13 | STATE1_14);
            }
        }
    }

    /// `func_8083A9B8`: start climbing up from the hang.
    fn func_8083A9B8(&mut self, data: &GameData, anim: AnimId) {
        self.setup_action(data, Action::ClimbUp, 0);
        self.skel.play_once_set_speed(data, anim, 1.3);
    }

    /// `func_8084BDFC`: climbing up; at the end the animation's x/z root motion is applied
    /// and Player stands on the ledge.
    fn func_8084BDFC(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_6;
        if self.skel.update(data) {
            self.func_80832E48(1);
            self.func_8083C0E8(data);
            return;
        }
        if self.skel.on_frame(self.skel.end_frame - 6.0) {
            // func_808328A0: landing sound.
        } else if self.skel.on_frame(self.skel.end_frame - 34.0) {
            self.state1 &= !(STATE1_13 | STATE1_14);
        }
    }

    /// `func_80838A14` (interrupt 12): climb onto the wall in front when it's ledge height
    /// (`unk_88C` ≥ 2) after pushing into it for 6 frames or on A; hop up a low step
    /// (`unk_88C` = 1) after 3 frames.
    fn func_80838A14(&mut self, env: &Env) -> bool {
        let data = env.data;
        let swimming = self.state1 & STATE1_27 != 0;
        if self.state1 & STATE1_11 == 0 && self.unk_88C >= 2 && (!swimming || self.age.unk_14 > self.wall_height) {
            if swimming {
                // func_808332B8: swimming with Kokiri boots.
                if self.actor.y_dist_to_water < 50.0 {
                    if self.unk_88C < 2 || self.wall_height > self.age.unk_10 {
                        return false;
                    }
                } else {
                    return false;
                }
            } else if !self.grounded() || (self.age.unk_14 <= self.wall_height && swimming) {
                return false;
            }
            // The WALL_FLAG_6 prompt case applies to dynamic collision only (wallBgId != BGCHECK_SCENE).
            if self.unk_88D >= 6 || self.input.press.held(BTN_A) {
                self.setup_action(data, Action::ClimbLedge, 0);
                self.state1 |= STATE1_18;
                let mut sp34 = self.wall_height;
                let anim;
                if self.age.unk_14 <= sp34 {
                    anim = data.anim("link_normal_250jump_start");
                    self.linear_velocity = 1.0;
                } else {
                    let n = self.actor.wall_poly.map(|w| env.col.poly_normal(w)).unwrap_or(Vec3::ZERO);
                    let sp24 = self.wall_distance + 0.5;
                    self.state1 |= STATE1_14;
                    let k = self.age.translation_scale;
                    if swimming {
                        anim = data.anim("link_swimer_swim_15step_up");
                        sp34 -= 60.0 * k;
                        self.state1 &= !STATE1_27;
                    } else if self.age.unk_18 <= sp34 {
                        anim = data.anim("link_normal_150step_up");
                        sp34 -= 59.0 * k;
                    } else {
                        anim = data.anim("link_normal_100step_up");
                        sp34 -= 41.0 * k;
                    }
                    self.actor.shape_y_offset -= sp34 * 100.0;
                    self.actor.world_pos.x -= sp24 * n.x;
                    self.actor.world_pos.y += self.wall_height;
                    self.actor.world_pos.z -= sp24 * n.z;
                    self.func_80832224();
                }
                self.actor.bg_check_flags |= BGCHECKFLAG_GROUND;
                self.skel.play_once_set_speed(data, anim, 1.3);
                self.skel.disable_queue();
                self.current_yaw = self.actor.wall_yaw.wrapping_add(i16::MIN);
                self.actor.shape_rot.y = self.current_yaw;
                return true;
            }
        } else if self.grounded() && self.unk_88C == 1 && self.unk_88D >= 3 {
            // func_808389E8: hop up a low step.
            let vy = self.wall_height * 0.08 + 5.5;
            let a = data.anim("link_normal_jump");
            self.func_80838940(data, Some(a), vy);
            self.linear_velocity = 2.5;
            return true;
        }
        false
    }

    /// `func_80845668`: stepping up onto a ledge (100/150 step-up animations, the model
    /// offset `shape.yOffset` easing back to 0), or the tall-ledge jump that ends in a mid-air grab.
    fn func_80845668(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5;
        let sp3c = self.skel.update(data);
        if self.skel.animation == data.anim("link_normal_250jump_start") {
            self.linear_velocity = 1.0;
            if self.skel.on_frame(8.0) {
                let mut t = self.wall_height.min(self.age.wall_probe_height);
                t *= if self.state1 & STATE1_27 != 0 { 0.085 } else { 0.072 };
                if !self.adult {
                    t += 1.0;
                }
                self.func_80838940(data, None, t);
                self.unk_850 = -1;
            }
        } else {
            let temp2 = self.func_808374A0(env, 4.0);
            if temp2 == 0 {
                self.state1 &= !(STATE1_14 | STATE1_18);
                return;
            }
            if sp3c || temp2 > 0 {
                self.func_8083C0E8(data);
                self.state1 &= !(STATE1_14 | STATE1_18);
                return;
            }
            let a = self.skel.animation;
            let temp3 = if a == data.anim("link_swimer_swim_15step_up") {
                // Frame 30: func_8083D0A8 (water surfacing) — no water yet.
                50.0
            } else if a == data.anim("link_normal_150step_up") {
                30.0
            } else if a == data.anim("link_normal_100step_up") {
                16.0
            } else {
                0.0
            };
            let _landing_sound = self.skel.on_frame(temp3);
            if a == data.anim("link_normal_100step_up") || self.skel.cur_frame > 5.0 {
                if self.unk_850 == 0 {
                    // func_80832854: jump sound.
                    self.unk_850 = 1;
                }
                step_to_f(&mut self.actor.shape_y_offset, 0.0, 150.0);
            }
        }
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
            let mut yaw = self.actor.shape_rot.y;
            if let Some(t) = self.unk_664 {
                if env.target.reticle_locked && self.state2 & STATE2_6 == 0 {
                    yaw = crate::target::yaw_to(self.actor.world_pos, env.targets[t].focus);
                }
            } else if self.func_80833B2C() {
                yaw = self.target_yaw;
            }
            (false, speed, yaw)
        } else {
            (true, speed, yaw.wrapping_add(env.cam_input_yaw))
        }
    }

    /// `func_80837348`: the per-action list of interrupts (`D_80854448`). Ported: 6
    /// (`func_8083C1DC`, roll / put the sword away on A), 7 (`func_80850224`, B attacks), 10
    /// (`func_8083BDBC`, hops while targeting) and 12 (`func_80838A14`, climbing onto ledges);
    /// the others need other items, doors or NPCs and report "not taken".
    fn func_80837348(&mut self, env: &Env, list: &[i8], arg3: bool) -> bool {
        if self.state1 & (1 | (1 << 7) | STATE1_29) != 0 {
            return false;
        }
        if arg3 {
            // D_808535E0 = func_80836670(): the upper-body item action.
            self.s.item_action = self.func_80836670(env) as i32;
        }
        // No interrupts while an item change is pending or playing.
        if self.state1 & STATE1_8 != 0 || self.upper == UpperAction::Change {
            return false;
        }
        let mut i = 0;
        loop {
            let e = list[i];
            let idx = e.unsigned_abs() as usize;
            if idx == 6 && self.func_8083C1DC(env) {
                return true;
            }
            if idx == 12 && self.func_80838A14(env) {
                return true;
            }
            if idx == 10 && self.func_8083BDBC(env) {
                return true;
            }
            if idx == 7 && self.func_80850224(env) {
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
    const D_808543E0: &'static [i8] = &[13, 2, 4, 9, 10, 11, 8, -7];
    const D_808543E8: &'static [i8] = &[13, 1, 2, 5, 3, 4, 9, 10, 11, 7, 8, -6];
    const D_808543F4: &'static [i8] = &[13, 1, 2, 3, 4, 9, 10, 11, 8, 7, -6];
    const D_80854400: &'static [i8] = &[13, 2, 4, 9, 10, 11, 8, -7];
    const D_80854408: &'static [i8] = &[13, 2, 4, 9, 10, 11, 12, 8, -7];
    const D_80854430: &'static [i8] = &[13, 1, 2, 3, 12, 5, 4, 9, 10, 11, 8, 7, -6];
    const D_80854440: &'static [i8] = &[10, 8, -7];
    const D_80854444: &'static [i8] = &[0, 12, 5, -4];

    /// `func_8083C1DC`: A pressed → roll if the stick points forward.
    fn func_8083C1DC(&mut self, env: &Env) -> bool {
        if !self.func_80833B54_env(env) && self.s.item_action == 0 && self.state1 & (1 << 23) == 0 && self.input.press.held(BTN_A) {
            if self.func_8083BC7C(env) {
                return true;
            }
            // unk_837 (a charge timer) is 0 here.
            if self.held_item_ap >= env.data.items.ap("SWORD_MASTER") {
                self.func_80835F44(env.data, 0);
            } else {
                self.state2 ^= 1 << 20;
            }
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

    /// `func_80833B54`: locked on to a hostile target (sets `PLAYER_STATE1_4`).
    fn func_80833B54_env(&mut self, env: &Env) -> bool {
        if let Some(t) = self.unk_664
            && env.targets.get(t).is_some_and(|a| a.is_hostile())
        {
            self.state1 |= STATE1_4;
            return true;
        }
        self.func_80833B54()
    }

    /// The "not locked on" half of `func_80833B54` (clears `PLAYER_STATE1_4`).
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

    /// `func_80839FFC`: switch to the standing action (locked-on, parallel or plain) without
    /// touching the animation.
    fn func_80839FFC(&mut self, data: &GameData) {
        let action = if self.state1 & STATE1_4 != 0 {
            Action::TargetIdle
        } else if self.func_80833B2C() {
            Action::ParallelIdle
        } else {
            Action::StandingStill
        };
        self.setup_action(data, action, 1);
    }

    /// `func_8083A060`.
    fn func_8083A060(&mut self, data: &GameData) {
        self.func_80839FFC(data);
        if self.state1 & STATE1_4 != 0 {
            self.unk_850 = 1;
        }
    }

    /// `func_8083A098`: stand, playing `anim` once (landing animations).
    fn func_8083A098(&mut self, data: &GameData, anim: AnimId) {
        self.func_8083A060(data);
        self.skel.play_once_set_speed(data, anim, self.s.speed_scale);
    }

    /// `func_8083C858`: start walking/running (the targeting run while targeting).
    fn func_8083C858(&mut self, data: &GameData) {
        let action = if self.func_80833BCC() { Action::TargetRun } else { Action::Run };
        self.setup_action(data, action, 1);
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

    /// `func_80836AB8(this, arg1)`: spread the focus rotation over head and torso (`arg1`:
    /// aiming, the whole upper body follows the focus).
    fn func_80836AB8_arg(&mut self, arg1: bool) -> i16 {
        if arg1 {
            self.unk_6BC = self.actor.focus_rot.x;
            self.unk_6AE |= 0x41;
            return self.actor.focus_rot.y;
        }
        self.func_80836AB8()
    }

    /// `func_80836AB8(this, 0)`.
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

    /// `func_8083DC54`: look at the target, or at the ground ahead.
    fn func_8083DC54(&mut self, env: &Env) {
        if self.unk_664.is_some() {
            self.func_8083DB98(env, false);
            return;
        }
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

    /// `func_808368EC`: the facing. Standing, it turns towards a locked target (once the reticle
    /// has locked) or to `targetYaw` in parallel mode; running/rolling it follows `currentYaw`.
    fn func_808368EC(&mut self, env: &Env) {
        let prev = self.actor.shape_rot.y;
        if self.state2 & (STATE2_5 | STATE2_6) == 0 {
            if let Some(t) = self.unk_664.and_then(|t| env.targets.get(t))
                && env.target.reticle_locked
            {
                let y = crate::target::yaw_to(self.actor.world_pos, t.focus);
                scaled_step_to_s(&mut self.actor.shape_rot.y, y, 4000);
            } else if self.state1 & STATE1_17 != 0 {
                let y = self.target_yaw;
                scaled_step_to_s(&mut self.actor.shape_rot.y, y, 4000);
            }
        } else if self.state2 & STATE2_6 == 0 {
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
            if self.func_80833B54_env(env) {
                self.func_8083CEAC(data);
                return;
            }
            if self.func_80833B2C() {
                self.func_80839F30(data);
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
            if self.func_80833C04(env) {
                self.func_8083C858(data);
                return;
            }
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
        // respawn[RESPAWN_MODE_TOP].data > 40 (void-out float): not modelled.
        if self.state1 & STATE1_4 != 0 {
            self.actor.gravity = -1.2;
        }
        let (_, speed, yaw) = self.func_80837268(env, 0.0);
        if !self.grounded() {
            self.skel.update(data);
            if self.state2 & STATE2_19 == 0 {
                self.func_8083DFE0(speed, yaw);
            }
            self.func_80836670(env);
            // func_8083BBA0 (the jump slash in the air) is not ported.
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
                    {
                        if self.wall_height >= 150.0 && self.stick_dir() == 0 {
                            // func_8083EC18: grab a climbable wall from the air — not ported.
                            self.note("mid-air wall climb (func_8083EC18) not ported");
                        } else if self.unk_88C >= 2
                            && self.wall_height < 150.0
                            && ((self.actor.world_pos.y - self.actor.floor_height) + self.wall_height) > 70.0 * self.age.translation_scale
                            && let Some(wp) = self.actor.wall_poly
                        {
                            self.skel.disable_queue();
                            self.actor.world_pos.y += self.wall_height;
                            let a = self.anim(data, group::HANG_GRAB);
                            let dist = self.wall_distance;
                            self.func_8083A5C4(data, env, wp, dist, a);
                            self.current_yaw = self.current_yaw.wrapping_add(i16::MIN);
                            self.actor.shape_rot.y = self.current_yaw;
                            self.state1 |= STATE1_13;
                            return;
                        }
                    }
                }
            }
        } else {
            let mut anim = self.anim(data, group::LANDING);
            if self.state2 & STATE2_19 != 0 {
                let row = data.side_hop_anims[self.unk_84F.clamp(0, 3) as usize];
                anim = if self.state1 & STATE1_4 != 0 { row[2] } else { row[1] };
            } else if self.skel.animation == data.anim("link_normal_run_jump") {
                anim = data.anim("link_normal_run_jump_end");
            } else if self.state1 & STATE1_4 != 0 {
                anim = data.anim("link_anchor_landingR");
                self.func_80833C3C();
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
    // Z-targeting (func_80836BEC) and the targeting actions

    /// `func_80833B2C`: parallel mode or a non-hostile target.
    fn func_80833B2C(&self) -> bool {
        self.state1 & (STATE1_16 | STATE1_17 | STATE1_30) != 0
    }

    /// `func_80833BCC`: locked on or in parallel mode.
    fn func_80833BCC(&self) -> bool {
        self.state1 & STATE1_4 != 0 || self.func_80833B2C()
    }

    /// `func_80833C04`.
    fn func_80833C04(&mut self, env: &Env) -> bool {
        self.func_80833B54_env(env) || self.func_80833B2C()
    }

    /// `func_8008EDF0`: drop the target.
    fn func_8008EDF0(&mut self) {
        self.unk_664 = None;
        self.state2 &= !STATE2_13;
    }

    /// `func_8008EE08`: leave targeting (or, in the air, remember to after landing).
    fn func_8008EE08(&mut self) {
        if self.grounded()
            || self.state1 & ((1 << 21) | STATE1_23 | STATE1_27) != 0
            || (self.state1 & (STATE1_18 | STATE1_19) == 0 && (self.actor.world_pos.y - self.actor.floor_height) < 100.0)
        {
            self.state1 &= !(STATE1_15 | STATE1_16 | STATE1_17 | STATE1_18 | STATE1_19 | STATE1_30);
        } else if self.state1 & (STATE1_18 | STATE1_19 | (1 << 21)) == 0 {
            self.state1 |= STATE1_19;
        }
        self.func_8008EDF0();
    }

    /// `func_808355DC`: enter parallel mode, squaring up to a wall in front.
    fn func_808355DC(&mut self) {
        self.state1 |= STATE1_17;
        if self.skel.move_flags & 0x80 == 0 && self.actor.bg_check_flags & BGCHECKFLAG_PLAYER_WALL_INTERACT != 0 && self.s.wall_facing_diff < 0x2000 {
            self.current_yaw = self.actor.wall_yaw.wrapping_add(i16::MIN);
            self.actor.shape_rot.y = self.current_yaw;
        }
        self.target_yaw = self.actor.shape_rot.y;
    }

    /// `func_80836BEC`: the Z button. With the default "Switch" setting (`zTargetSetting` 0) a
    /// press locks on to `arrowPointedActor` (or the next candidate if it's already locked) and
    /// keeps it (`PLAYER_STATE2_13`) until pressed again; with nothing to target, parallel mode.
    fn func_80836BEC(&mut self, env: &Env) {
        let z = self.input.cur.held(BTN_Z);
        if !z {
            self.state1 &= !STATE1_30;
        }
        if self.state1 & (STATE1_7 | STATE1_29) != 0 || self.state3 & STATE3_7 != 0 {
            self.unk_66C = 0;
        } else if z || self.state2 & STATE2_13 != 0 {
            if self.unk_66C <= 5 {
                self.unk_66C = 5;
            } else {
                self.unk_66C -= 1;
            }
        } else if self.state1 & STATE1_17 != 0 {
            self.unk_66C = 0;
        } else if self.unk_66C != 0 {
            self.unk_66C -= 1;
        }
        let sp1c = self.unk_66C >= 6;
        // func_8083224C (a talk request, ACTOR_FLAG_8) is never set here.
        if self.unk_66C != 0 || self.state1 & (STATE1_12 | STATE1_25) != 0 {
            if self.state1 & STATE1_25 == 0 && self.input.press.held(BTN_Z) {
                let hold = false;
                self.state1 |= STATE1_15;
                match env.target.arrow_pointed.filter(|&t| env.targets.get(t).is_some_and(|a| a.flags & ACTOR_FLAG_27 == 0)) {
                    Some(t) => {
                        let mut to = Some(t);
                        if to == self.unk_664 {
                            to = env.target.unk_94;
                        }
                        if to != self.unk_664 {
                            if !hold {
                                self.state2 |= STATE2_13;
                            }
                            self.unk_664 = to;
                            self.unk_66C = 15;
                            self.state2 &= !(STATE2_1 | STATE2_21);
                        } else if !hold {
                            self.func_8008EDF0();
                        }
                        self.state1 &= !STATE1_30;
                    }
                    None => {
                        if self.state1 & (STATE1_17 | STATE1_30) == 0 {
                            self.func_808355DC();
                        }
                    }
                }
            }
            if let Some(t) = self.unk_664 {
                if env.targets.get(t).is_none_or(|a| crate::target::lost(&env.data.target_ranges, a, true, self.actor.shape_rot.y, sp1c)) {
                    self.func_8008EDF0();
                    self.state1 |= STATE1_30;
                }
            }
            if let Some(t) = self.unk_664 {
                self.state1 &= !(STATE1_16 | STATE1_17);
                if self.state1 & STATE1_11 != 0 || !env.targets[t].is_hostile() {
                    self.state1 |= STATE1_16;
                }
            } else if self.state1 & STATE1_17 != 0 {
                self.state2 &= !STATE2_13;
            } else {
                self.func_8008EE08();
            }
        } else {
            self.func_8008EE08();
        }
    }

    /// `func_8083DB98`: turn the head (and torso) towards the target's focus.
    fn func_8083DB98(&mut self, env: &Env, arg1: bool) -> i16 {
        let Some(t) = self.unk_664.and_then(|t| env.targets.get(t)) else { return self.actor.shape_rot.y };
        let from = Vec3::new(self.actor.world_pos.x, self.head_pos.y + 3.0, self.actor.world_pos.z);
        let pitch = crate::target::pitch_to(from, t.focus);
        let yaw = crate::target::yaw_to(from, t.focus);
        smooth_step_to_s(&mut self.actor.focus_rot.y, yaw, 4, 10000, 0);
        smooth_step_to_s(&mut self.actor.focus_rot.x, pitch, 4, 10000, 0);
        self.unk_6AE |= 2;
        self.func_80836AB8_arg(arg1)
    }

    /// `func_808334E4` / `func_80833528`: the locked-on wait animations (right / left foot forward).
    fn func_808334E4(&self, data: &GameData) -> AnimId {
        self.anim(data, group::TARGET_WAIT_R)
    }
    fn func_80833528(&self, data: &GameData) -> AnimId {
        self.anim(data, group::TARGET_WAIT_L)
    }

    /// `func_808401B0`: blend the two target waits by `unk_870`.
    fn func_808401B0(&mut self, data: &GameData) {
        let (a, b) = (self.func_808334E4(data), self.func_80833528(data));
        self.skel.blend_to_joint(data, a, self.unk_868, b, self.unk_868, self.unk_870);
    }

    /// `func_80840138`: which foot leads (`unk_874`), eased into `unk_870`.
    fn func_80840138(&mut self, speed: f32, yaw: i16) {
        let d = yaw.wrapping_sub(self.actor.shape_rot.y);
        if speed > 0.0 {
            self.unk_874 = if d < 0 { 0.0 } else { 1.0 };
        }
        step_to_f(&mut self.unk_870, self.unk_874, 0.3);
    }

    /// `func_8083FC68`: locked on, classify the stick: 1 run forward, -1 step back, 0 sidestep/stay.
    fn func_8083FC68(&mut self, env: &Env, speed: f32, yaw: i16) -> i32 {
        let sp1c = yaw.wrapping_sub(self.actor.shape_rot.y) as f32;
        if self.unk_664.is_some() {
            self.func_8083DB98(env, false);
        }
        let t = sp1c.abs() / 32768.0;
        if speed > (t * t) * 50.0 + 6.0 {
            1
        } else if speed > (1.0 - t) * 10.0 + 6.8 {
            -1
        } else {
            0
        }
    }

    /// `func_8083FD78`: the same for parallel mode (relative to `targetYaw`).
    fn func_8083FD78(&mut self, env: &Env, speed: &mut f32, yaw: &mut i16) -> i32 {
        let sp2e = yaw.wrapping_sub(self.target_yaw);
        let sp2c = (sp2e as i32).unsigned_abs() as u16;
        // Bow / boomerang aiming: not held.
        if self.unk_664.is_some() {
            return self.func_8083FC68(env, *speed, *yaw);
        }
        self.func_8083DC54(env);
        if *speed != 0.0 && sp2c < 6000 {
            return 1;
        } else if *speed > sin_s((0x4000u16.wrapping_sub(sp2c >> 1)) as i16) * 200.0 {
            return -1;
        }
        0
    }

    /// `func_8083CEAC`: lock on (standing).
    fn func_8083CEAC(&mut self, data: &GameData) {
        self.setup_action(data, Action::TargetIdle, 1);
        let a = self.anim(data, group::TARGET_ENTER);
        self.func_80832B0C(data, a);
        self.unk_850 = 1;
    }

    /// `func_80839F30`: parallel mode (standing).
    fn func_80839F30(&mut self, data: &GameData) {
        self.setup_action(data, Action::ParallelIdle, 1);
        let a = self.anim(data, group::WAIT);
        self.func_80832B0C(data, a);
        self.current_yaw = self.actor.shape_rot.y;
    }

    /// `func_80839E88`: back to the locked-on wait, keeping the leading foot.
    fn func_80839E88(&mut self, data: &GameData) {
        self.setup_action(data, Action::TargetIdle, 1);
        let anim = if self.unk_870 < 0.5 {
            self.unk_870 = 0.0;
            self.func_808334E4(data)
        } else {
            self.unk_870 = 1.0;
            self.func_80833528(data)
        };
        self.unk_874 = self.unk_870;
        self.skel.play_loop(data, anim);
        self.current_yaw = self.actor.shape_rot.y;
    }

    /// `func_80839F90`.
    fn func_80839F90(&mut self, data: &GameData) {
        if self.state1 & STATE1_4 != 0 {
            self.func_80839E88(data);
        } else if self.func_80833B2C() {
            self.func_80839F30(data);
        } else {
            self.func_80853080(data);
        }
    }

    /// `func_8083CF10`: target lost while standing.
    fn func_8083CF10(&mut self, data: &GameData) {
        if self.linear_velocity != 0.0 {
            self.func_8083C858(data);
        } else {
            self.func_8083CE0C(data);
        }
    }

    /// `func_8083CE0C`: end the lock-on stance.
    fn func_8083CE0C(&mut self, data: &GameData) {
        self.setup_action(data, Action::StandingStill, 1);
        let a = if self.unk_870 < 0.5 { self.anim(data, group::TARGET_EXIT_L) } else { self.anim(data, group::TARGET_EXIT_R) };
        self.skel.play_once(data, a);
        self.current_yaw = self.actor.shape_rot.y;
    }

    /// `func_8083CBF0`: step back from the target.
    fn func_8083CBF0(&mut self, data: &GameData, yaw: i16) {
        self.setup_action(data, Action::TargetBackwalk, 1);
        let a = data.anim("link_anchor_back_walk");
        let last = data.anims[a].last_frame();
        self.skel.change(data, a, 2.2, 0.0, last, ANIMMODE_ONCE, -6.0);
        self.linear_velocity = 8.0;
        self.current_yaw = yaw;
    }

    /// `func_8083CC9C`: start sidestepping.
    fn func_8083CC9C(&mut self, data: &GameData) {
        self.setup_action(data, Action::Sidestep, 1);
        let a = self.anim(data, group::SIDESTEP_L);
        self.skel.change(data, a, 1.0, 0.0, 0.0, ANIMMODE_LOOP, -6.0);
        self.unk_868 = 0.0;
    }

    /// `func_8083CB2C`: start walking backwards in parallel mode.
    fn func_8083CB2C(&mut self, data: &GameData, yaw: i16) {
        self.setup_action(data, Action::ParallelBackwalk, 1);
        self.skel.copy_joint_to_morph();
        self.unk_864 = 0.0;
        self.unk_868 = 0.0;
        self.current_yaw = yaw;
    }

    /// `func_8083CB94`: start walking sideways in parallel mode.
    fn func_8083CB94(&mut self, data: &GameData) {
        self.setup_action(data, Action::ParallelWalk, 1);
        let a = self.anim(data, group::WALK);
        self.skel.change(data, a, 1.0, 0.0, 0.0, ANIMMODE_LOOP, -6.0);
    }

    /// `func_80835DAC`: switch action keeping the animation-driven movement flags.
    fn func_80835DAC(&mut self, data: &GameData, action: Action, flags: i32) {
        let t = self.skel.move_flags;
        self.skel.move_flags = 0;
        self.setup_action(data, action, flags);
        self.skel.move_flags = t;
    }

    /// `func_80840450`: locked on, standing (and turning to face the target).
    fn func_80840450(&mut self, env: &Env) {
        let data = env.data;
        // PLAYER_STATE3_3 (sword swing recovery): no melee weapon in hand yet.
        self.state3 &= !STATE3_3;
        if self.unk_850 != 0 {
            if self.skel.update(data) {
                self.func_80832DBC();
                let a = self.func_808334E4(data);
                self.skel.play_loop(data, a);
                self.unk_850 = 0;
                self.state3 &= !STATE3_3;
            }
            self.func_80833C3C();
        } else {
            self.func_808401B0(data);
        }
        self.func_8083721C();
        if !self.func_80837348(env, Self::D_808543E0, true) {
            // func_80834B5C (shield up) is never the upper-body action here.
            if !self.func_80833B54_env(env) && !self.func_80833B2C() {
                self.func_8083CF10(data);
                return;
            }
            let (_, sp44, sp42) = self.func_80837268(env, 0.0);
            let t1 = self.func_8083FC68(env, sp44, sp42);
            if t1 > 0 {
                self.func_8083C8DC(data, sp42);
                return;
            }
            if t1 < 0 {
                self.func_8083CBF0(data, sp42);
                return;
            }
            if sp44 > 4.0 {
                self.func_8083CC9C(data);
                return;
            }
            self.func_8084029C(self.linear_velocity * 0.3 + 1.0);
            self.func_80840138(sp44, sp42);
            let t2 = self.unk_868 as u32;
            if t2 < 6 || t2.wrapping_sub(0xE) < 6 {
                step_to_f(&mut self.linear_velocity, 0.0, 1.5);
                return;
            }
            let t3 = sp42.wrapping_sub(self.current_yaw);
            let t4 = abs16(t3);
            if t4 > 0x4000 {
                if step_to_f(&mut self.linear_velocity, 0.0, 1.5) {
                    self.current_yaw = sp42;
                }
                return;
            }
            asym_step_to_f(&mut self.linear_velocity, sp44 * 0.3, 2.0, 1.5);
            if self.state3 & STATE3_3 == 0 {
                scaled_step_to_s(&mut self.current_yaw, sp42, (t4 as f32 * 0.1) as i16);
            }
        }
    }

    /// `func_808407CC`: parallel mode, standing.
    fn func_808407CC(&mut self, env: &Env) {
        let data = env.data;
        if self.skel.update(data) {
            self.func_80832DBC();
            let a = self.anim(data, group::WAIT);
            self.skel.play_once(data, a);
        }
        self.func_8083721C();
        if !self.func_80837348(env, Self::D_808543E8, true) {
            if self.func_80833B54_env(env) {
                self.func_8083CEAC(data);
                return;
            }
            if !self.func_80833B2C() {
                self.func_80835DAC(data, Action::StandingStill, 1);
                self.current_yaw = self.actor.shape_rot.y;
                return;
            }
            let (_, mut sp3c, mut sp3a) = self.func_80837268(env, 0.0);
            let t1 = self.func_8083FD78(env, &mut sp3c, &mut sp3a);
            if t1 > 0 {
                self.func_8083C8DC(data, sp3a);
                return;
            }
            if t1 < 0 {
                self.func_8083CB2C(data, sp3a);
                return;
            }
            if sp3c > 4.9 {
                self.func_8083CC9C(data);
                self.func_80833C3C();
                return;
            }
            if sp3c != 0.0 {
                self.func_8083CB94(data);
                return;
            }
            let t2 = sp3a.wrapping_sub(self.actor.shape_rot.y);
            if abs16(t2) > 800 {
                self.func_8083CD54(data, sp3a);
            }
        }
    }

    /// `func_8084227C`: running forward while targeting.
    fn func_8084227C(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5;
        self.func_80841EE4(data);
        if !self.func_80837348(env, Self::D_80854430, true) {
            if !self.func_80833C04(env) {
                self.func_8083C858(data);
                return;
            }
            let (_, mut sp2c, mut sp2a) = self.func_80837268(env, 0.0);
            if !self.func_8083C484(&mut sp2c, &mut sp2a) {
                let stop = if self.func_80833B2C() {
                    sp2c != 0.0 && self.func_8083FD78(env, &mut sp2c, &mut sp2a) <= 0
                } else {
                    self.func_8083FC68(env, sp2c, sp2a) <= 0
                };
                if stop {
                    self.func_80839F90(data);
                    return;
                }
                self.func_8083DF68(sp2c, sp2a);
                self.func_8083DDC8(env);
                if self.linear_velocity == 0.0 && sp2c == 0.0 {
                    self.func_80839F90(data);
                }
            }
        }
    }

    /// `func_80841860`: sidestep animation, the two sidestep cycles blended by `unk_870`.
    fn func_80841860(&mut self, data: &GameData) {
        let a = self.anim(data, group::SIDESTEP_R);
        let b = self.anim(data, group::SIDESTEP_L);
        self.skel.animation = a;
        let r = |n| self.regs.reg(n) as f32 / 1000.0;
        self.func_8084029C(r(30) + r(32) * self.linear_velocity);
        let frame = self.unk_868 * (16.0 / 29.0);
        self.skel.blend_to_joint(data, b, frame, a, frame, self.unk_870);
    }

    /// `func_8084193C`: sidestepping.
    fn func_8084193C(&mut self, env: &Env) {
        let data = env.data;
        self.func_80841860(data);
        if !self.func_80837348(env, Self::D_80854408, true) {
            if !self.func_80833C04(env) {
                self.func_8083C858(data);
                return;
            }
            let (_, mut sp3c, mut sp3a) = self.func_80837268(env, 0.0);
            let t1 = if self.func_80833B2C() { self.func_8083FD78(env, &mut sp3c, &mut sp3a) } else { self.func_8083FC68(env, sp3c, sp3a) };
            if t1 > 0 {
                self.func_8083C858(data);
                return;
            }
            if t1 < 0 {
                if self.func_80833B2C() {
                    self.func_8083CB2C(data, sp3a);
                } else {
                    self.func_8083CBF0(data, sp3a);
                }
                return;
            }
            if self.linear_velocity < 3.6 && sp3c < 4.0 {
                if self.state1 & STATE1_4 == 0 && self.func_80833B2C() {
                    self.func_8083CB94(data);
                } else {
                    self.func_80839F90(data);
                }
                return;
            }
            self.func_80840138(sp3c, sp3a);
            let t2 = sp3a.wrapping_sub(self.current_yaw);
            let t3 = abs16(t2);
            if t3 > 0x4000 {
                if step_to_f(&mut self.linear_velocity, 0.0, 3.0) {
                    self.current_yaw = sp3a;
                }
                return;
            }
            asym_step_to_f(&mut self.linear_velocity, sp3c * 0.9, 2.0, 3.0);
            scaled_step_to_s(&mut self.current_yaw, sp3a, (t3 as f32 * 0.1) as i16);
        }
    }

    /// `func_808423EC`: the step back from a locked target.
    fn func_808423EC(&mut self, env: &Env) {
        let data = env.data;
        let sp34 = self.skel.update(data);
        if !self.func_80837348(env, Self::D_80854408, true) {
            if !self.func_80833C04(env) {
                self.func_8083C858(data);
                return;
            }
            let (_, sp30, sp2e) = self.func_80837268(env, 0.0);
            if self.skel.morph_weight == 0.0 && self.skel.cur_frame > 5.0 {
                self.func_8083721C();
                if self.skel.cur_frame > 10.0 && self.func_8083FC68(env, sp30, sp2e) < 0 {
                    self.func_8083CBF0(data, sp2e);
                    return;
                }
                if sp34 {
                    // func_8083CD00.
                    self.setup_action(data, Action::TargetBackBrake, 1);
                    let a = data.anim("link_anchor_back_brake");
                    self.skel.play_once_set_speed(data, a, 2.0);
                }
            }
        }
    }

    /// `func_8084251C`: braking after the step back.
    fn func_8084251C(&mut self, env: &Env) {
        let data = env.data;
        let sp34 = self.skel.update(data);
        self.func_8083721C();
        if !self.func_80837348(env, Self::D_80854440, true) {
            let (_, sp30, sp2e) = self.func_80837268(env, 0.0);
            if self.linear_velocity == 0.0 {
                self.current_yaw = self.actor.shape_rot.y;
                if self.func_8083FC68(env, sp30, sp2e) > 0 {
                    self.func_8083C858(data);
                    return;
                }
                if sp30 != 0.0 || sp34 {
                    self.func_80839F90(data);
                }
            }
        }
    }

    /// `func_80840DE4`: walking sideways in parallel mode. The side-walk cycle plays at a speed set
    /// by `linearVelocity × MREG(95) / 100`, signed by which way Player moves.
    fn func_80840DE4(&mut self, env: &Env) {
        let data = env.data;
        self.skel.mode = ANIMMODE_LOOP;
        self.skel.animation = self.anim(data, group::PARALLEL_SIDEWALK);
        let (frames, coeff) = (29.0, self.regs.mreg(95) as f32 / 100.0);
        self.skel.anim_length = frames;
        self.skel.end_frame = frames - 1.0;
        let dir = if self.current_yaw.wrapping_sub(self.actor.shape_rot.y) >= 0 { 1.0 } else { -1.0 };
        self.skel.play_speed = dir * (self.linear_velocity * coeff);
        self.skel.update(data);
        if !self.func_80837348(env, Self::D_808543F4, true) {
            if self.func_80833B54_env(env) {
                self.func_8083CEAC(data);
                return;
            }
            if !self.func_80833B2C() {
                self.func_80853080(data);
                return;
            }
            let (_, mut sp44, mut sp42) = self.func_80837268(env, 0.0);
            let t1 = self.func_8083FD78(env, &mut sp44, &mut sp42);
            if t1 > 0 {
                self.func_8083C8DC(data, sp42);
                return;
            }
            if t1 < 0 {
                self.func_8083CB2C(data, sp42);
                return;
            }
            if sp44 > 4.9 {
                self.func_8083CC9C(data);
                self.func_80833C3C();
                return;
            }
            if sp44 == 0.0 && self.linear_velocity == 0.0 {
                self.func_80839F30(data);
                return;
            }
            let t2 = sp42.wrapping_sub(self.current_yaw);
            let t3 = abs16(t2);
            if t3 > 0x4000 {
                if step_to_f(&mut self.linear_velocity, 0.0, 1.5) {
                    self.current_yaw = sp42;
                }
                return;
            }
            asym_step_to_f(&mut self.linear_velocity, sp44 * 0.4, 1.5, 1.5);
            scaled_step_to_s(&mut self.current_yaw, sp42, (t3 as f32 * 0.1) as i16);
        }
    }

    /// `func_80841138`: parallel back walk/run animation (like `func_80841EE4` with the back cycles).
    fn func_80841138(&mut self, data: &GameData) {
        let r = |n| self.regs.reg(n) as f32 / 1000.0;
        let back = self.anim(data, group::PARALLEL_BACKWALK);
        let temp1;
        if self.unk_864 < 1.0 {
            self.func_8084029C(r(35));
            self.skel.load_to_joint(data, back, self.unk_868);
            self.unk_864 = (self.unk_864 + UPDATE_SCALE).min(1.0);
            temp1 = self.unk_864;
        } else {
            let temp2 = self.linear_velocity - (self.regs.reg(48) as f32 / 100.0);
            if temp2 < 0.0 {
                temp1 = 1.0;
                self.func_8084029C(r(35) + r(36) * self.linear_velocity);
                self.skel.load_to_joint(data, back, self.unk_868);
            } else {
                let mut t = r(37) * temp2;
                if t < 1.0 {
                    self.func_8084029C(r(35) + r(36) * self.linear_velocity);
                } else {
                    t = 1.0;
                    self.func_8084029C(1.2 + r(38) * temp2);
                }
                temp1 = t;
                self.skel.load_to_morph(data, back, self.unk_868);
                let run = data.anim("link_normal_back_run");
                self.skel.load_to_joint(data, run, self.unk_868 * (16.0 / 29.0));
            }
        }
        if temp1 < 1.0 {
            self.skel.interp_joint_morph(1.0 - temp1);
        }
    }

    /// `func_808414F8`: walking/running backwards in parallel mode.
    fn func_808414F8(&mut self, env: &Env) {
        let data = env.data;
        self.func_80841138(data);
        if !self.func_80837348(env, Self::D_80854400, true) {
            if !self.func_80833C04(env) {
                let y = self.current_yaw;
                self.func_8083C8DC(data, y);
                return;
            }
            let (_, mut sp34, mut sp32) = self.func_80837268(env, 0.0);
            let sp2c = self.func_8083FD78(env, &mut sp34, &mut sp32);
            if sp2c >= 0 {
                // func_80841458.
                let handled = if self.linear_velocity > 6.0 {
                    // func_8084140C.
                    self.setup_action(data, Action::ParallelBackBrake, 1);
                    let a = data.anim("link_normal_back_brake");
                    self.func_80832B0C(data, a);
                    true
                } else if sp34 != 0.0 {
                    if self.func_8083721C() {
                        // func_80841458 also resets *arg2 to currentYaw; it isn't read again.
                        sp34 = 0.0;
                        false
                    } else {
                        true
                    }
                } else {
                    false
                };
                if !handled {
                    if sp2c != 0 {
                        self.func_8083C858(data);
                    } else if sp34 > 4.9 {
                        self.func_8083CC9C(data);
                    } else {
                        self.func_8083CB94(data);
                    }
                }
            } else {
                let sp2a = sp32.wrapping_sub(self.current_yaw);
                asym_step_to_f(&mut self.linear_velocity, sp34 * 1.5, 1.5, 2.0);
                scaled_step_to_s(&mut self.current_yaw, sp32, (sp2a as f32 * 0.1) as i16);
                if sp34 == 0.0 && self.linear_velocity == 0.0 {
                    self.func_80839F30(data);
                }
            }
        }
    }

    /// `func_8084170C`: braking out of a parallel back run.
    fn func_8084170C(&mut self, env: &Env) {
        let data = env.data;
        let sp34 = self.skel.update(data);
        self.func_8083721C();
        if !self.func_80837348(env, Self::D_80854400, true) {
            let (_, mut sp30, mut sp2e) = self.func_80837268(env, 0.0);
            if self.linear_velocity == 0.0 {
                self.current_yaw = self.actor.shape_rot.y;
                if self.func_8083FD78(env, &mut sp30, &mut sp2e) > 0 {
                    self.func_8083C858(data);
                } else if sp30 != 0.0 || sp34 {
                    // func_808416C0.
                    self.setup_action(data, Action::ParallelBackBrakeEnd, 1);
                    let a = data.anim("link_normal_back_brake_end");
                    self.skel.play_once(data, a);
                }
            }
        }
    }

    /// `func_808417FC`: end of the parallel back brake.
    fn func_808417FC(&mut self, env: &Env) {
        let data = env.data;
        let sp1c = self.skel.update(data);
        if !self.func_80837348(env, Self::D_80854400, true) && sp1c {
            self.func_80839F30(data);
        }
    }

    /// `func_8083BDBC` (interrupt 10): A while targeting: a side hop or backflip away from the
    /// stick's forward, or a roll forward (a jump slash with a sword: not ported).
    fn func_8083BDBC(&mut self, env: &Env) -> bool {
        let floor_effect = self.actor.floor_poly.map(|p| env.col.floor_effect(p)).unwrap_or(0);
        // Room behaviour type 1 is 0 here (not ROOM_BEHAVIOR_TYPE1_2).
        if self.input.press.held(BTN_A) && self.s.floor_type != FLOOR_TYPE_7 && floor_effect != 1 {
            let sp2c = self.stick_dir();
            if sp2c <= 0 {
                if self.func_80833BCC() {
                    self.func_8083BC04(env.data);
                    return true;
                }
            } else {
                self.func_8083BCD0(env.data, sp2c);
                return true;
            }
        }
        false
    }

    /// `func_8083BCD0`: hop in direction `arg2` (1 left, 2 back = backflip, 3 right).
    fn func_8083BCD0(&mut self, data: &GameData, arg2: i8) {
        let side = arg2 & 1 != 0;
        let a = data.side_hop_anims[arg2 as usize][0];
        self.func_80838940(data, Some(a), if !side { 5.8 } else { 3.5 });
        self.unk_850 = 1;
        self.unk_84F = arg2;
        self.current_yaw = self.actor.shape_rot.y.wrapping_add(((arg2 as i32) << 14) as i16);
        self.linear_velocity = if !side { 6.0 } else { 8.5 };
        self.state2 |= STATE2_19;
    }

    // ================================================================================
    // Items and the sword (func_80836670, func_80835F44, func_808502D0)

    /// `func_80833350`: -1 if playing the wait, 1 + index if an idle fidget (`D_80853D7C`), else 0.
    fn func_80833350(&self, data: &GameData) -> i32 {
        if self.anim(data, group::WAIT) == self.skel.animation {
            return -1;
        }
        for (i, a) in data.idle_variants.iter().flat_map(|p| p.iter()).enumerate() {
            if *a == self.skel.animation {
                return i as i32 + 1;
            }
        }
        0
    }

    /// `func_80836670`: run the upper-body action; while it's active, its animation replaces the
    /// upper body (`D_80853410`), or the whole body when standing on the wait/fidget.
    fn func_80836670(&mut self, env: &Env) -> bool {
        let data = env.data;
        // Hookshot flight: not held.
        if self.func_808365C8() {
            self.func_80834298(env);
        }
        if !self.run_upper(env) {
            return false;
        }
        // unk_830 (blend back out of skelAnime2) is only ever set to 0 in this decomp.
        if self.func_80833350(data) == 0 || self.linear_velocity != 0.0 {
            let mask = data.items.upper_body;
            let src = self.skel2.joint;
            self.skel.request_copy_external(&src, Some(mask));
        } else {
            let src = self.skel2.joint;
            self.skel.request_copy_external(&src, None);
        }
        true
    }

    /// `func_808365C8`: the item system may run.
    fn func_808365C8(&self) -> bool {
        self.upper != UpperAction::Change || self.held_item == Some(self.held_item_ap)
    }

    fn run_upper(&mut self, env: &Env) -> bool {
        match self.upper {
            // func_8083485C / func_808349DC: func_80834758 (shield up on R) is not ported.
            UpperAction::Default => false,
            UpperAction::Sword => {
                // func_8083499C.
                if self.state1 & STATE1_8 != 0 {
                    self.func_808340DC(env.data);
                    true
                } else {
                    false
                }
            }
            UpperAction::Change => self.func_80834A2C(env),
        }
    }

    /// `D_80853EDC[actionParam]`: the upper-body action for a held item.
    fn upper_for(&self, data: &GameData, ap: i32) -> UpperAction {
        let it = &data.items;
        if (it.ap("SWORD_MASTER")..=it.ap("SWORD_BGS")).contains(&ap) { UpperAction::Sword } else { UpperAction::Default }
    }

    /// `func_80833638`.
    fn func_80833638(&mut self, f: UpperAction) {
        self.upper = f;
        self.unk_830 = 0.0;
    }

    /// `func_80834298`: B / C buttons, then a pending change.
    fn func_80834298(&mut self, env: &Env) {
        if self.state1 & STATE1_8 == 0 && (self.held_item_ap == self.item_ap || self.state1 & STATE1_22 != 0) {
            self.func_80833DF8(env);
        }
        if self.state1 & STATE1_8 != 0 {
            self.func_808340DC(env.data);
        }
    }

    /// `func_80833DF8`: the item buttons (only B carries an item here).
    fn func_80833DF8(&mut self, env: &Env) {
        if self.state1 & (STATE1_11 | STATE1_29) != 0 {
            return;
        }
        if self.input.press.held(crate::input::BTN_B) {
            let b = self.b_item;
            self.func_80835F44(env.data, b);
        } else if self.input.cur.held(crate::input::BTN_B) && self.b_item == self.held_item_ap {
            self.s.d_80853618 = true;
        }
    }

    /// `Player_ActionToMeleeWeapon`.
    fn melee_weapon(ap: i32) -> i32 {
        let m = ap - 2;
        if m > 0 && m < 6 { m } else { 0 }
    }

    /// `func_80835F44`: use / change to the item `ap` (as its action param; 0 = put away). For
    /// the sword this starts the change animation, or, with the sword already in hand, flags the
    /// press (`D_80853614`) for an attack.
    fn func_80835F44(&mut self, data: &GameData, ap: i32) {
        let ok = (self.held_item_ap == self.item_ap && (self.state1 & STATE1_22 == 0 || Self::melee_weapon(ap) != 0 || ap == 0))
            || (self.item_ap < 0 && (Self::melee_weapon(ap) != 0 || ap == 0));
        if !ok || !(ap == 0 || self.state1 & STATE1_27 == 0) {
            return;
        }
        // Sticks, nuts, lens, magic, masks, ocarina and bottles aren't on the buttons here.
        if ap != self.held_item_ap {
            let it = &data.items;
            self.next_model_group = it.action_model_group[ap as usize];
            let next_type = it.model_group_anim_type[self.next_model_group];
            let cur_type = it.model_group_anim_type[self.model_group];
            if self.held_item_ap >= 0 && self.held_item != Some(ap) && it.change_matrix[cur_type][next_type] != 0 {
                self.held_item = Some(ap);
                self.state1 |= STATE1_8;
            } else {
                self.func_80833664(data, ap);
            }
        } else {
            self.s.d_80853614 = true;
            self.s.d_80853618 = true;
        }
    }

    /// `func_80833664`: swap the item now, keeping the animation group.
    fn func_80833664(&mut self, data: &GameData, ap: i32) {
        let cur = self.skel.animation;
        let groups = data.anim_group_names.len();
        let found = (0..groups).find(|&g| data.player_anim(g, self.model_anim_type) == cur);
        self.state1 &= !(STATE1_3 | STATE1_24);
        self.func_8083399C(data, ap);
        if let Some(g) = found {
            self.skel.animation = data.player_anim(g, self.model_anim_type);
        }
    }

    /// `func_8083399C`: the item is in hand.
    fn func_8083399C(&mut self, data: &GameData, ap: i32) {
        self.held_item_ap = ap;
        self.item_ap = ap;
        self.model_group = self.next_model_group;
        self.state1 &= !(STATE1_3 | STATE1_24);
        // D_80853FE8[ap] is func_80833770 (empty) for no item and the swords.
        self.player_set_model_group(data, self.model_group);
    }

    /// `Player_SetModelGroup`: the model group decides the animation type.
    fn player_set_model_group(&mut self, data: &GameData, mg: usize) {
        self.model_group = mg;
        let it = &data.items;
        let mut t = if mg == it.model_group("CHILD_HYLIAN_SHIELD") { 0 } else { it.model_group_anim_type[mg] };
        if t < 3 && self.current_shield == 0 {
            t = 0;
        }
        self.model_anim_type = t;
    }

    /// `func_808340DC`: start the change animation on `skelAnime2`.
    fn func_808340DC(&mut self, data: &GameData) {
        let sp37 = self.held_item.unwrap_or(0);
        self.func_80833638(UpperAction::Change);
        let it = &data.items;
        let next_type = it.model_group_anim_type[self.next_model_group];
        let sp38 = it.change_matrix[it.model_group_anim_type[self.model_group]][next_type];
        // Bottle / boomerang use D_808540F4[13]: not on the buttons.
        self.unk_15A = sp38.unsigned_abs() as usize;
        let mut anim = it.change_anims[self.unk_15A].0;
        if anim == data.anim("link_normal_fighter2free") && self.current_shield == 0 {
            anim = data.anim("link_normal_free2fighter_free");
        }
        let last = data.anims[anim].last_frame();
        let (mut speed, start, end) = if sp38 >= 0 { (1.2, 0.0, last) } else { (-1.2, last, 0.0) };
        if sp37 != 0 {
            speed *= 2.0;
        }
        self.skel2.change(data, anim, speed, start, end, ANIMMODE_ONCE, 0.0);
        self.state1 &= !STATE1_8;
    }

    /// `func_80834A2C`: the change playing. On its swap frame the item goes into the hand; once
    /// it's in hand a B press (or any press for a non-two-handed item) ends the change.
    fn func_80834A2C(&mut self, env: &Env) -> bool {
        let data = env.data;
        let done = self.skel2.update(data);
        let in_hand = self.held_item == Some(self.held_item_ap) && {
            self.s.d_80853614 = self.s.d_80853614 || self.model_anim_type != 3;
            self.s.d_80853614
        };
        if done || in_hand {
            let f = self.upper_for(data, self.held_item_ap);
            self.func_80833638(f);
            self.unk_6AC = 0;
            self.s.d_80853618 = self.s.d_80853614;
            return self.run_upper(env);
        }
        if self.func_80833350(data) != 0 {
            self.func_808348EC(env);
            let a = self.anim(data, group::WAIT);
            self.skel.play_once(data, a);
            self.unk_6AC = 0;
        } else {
            self.func_808348EC(env);
        }
        true
    }

    /// `func_808348EC`: swap the item on the change animation's swap frame.
    fn func_808348EC(&mut self, env: &Env) {
        let mut t = env.data.items.change_anims[self.unk_15A].1;
        if self.skel2.play_speed < 0.0 {
            t -= 1.0;
        }
        if self.skel2.on_frame(t) {
            // func_80834594: the sword sounds, then func_80835F44(heldItemId).
            let ap = self.held_item.unwrap_or(0);
            self.func_80835F44(env.data, ap);
        }
        self.func_80833B54_env(env);
    }

    /// `func_80850224` (interrupt 7): B with a melee weapon in hand attacks.
    fn func_80850224(&mut self, env: &Env) -> bool {
        // func_8083C6B8 (bottles, the fishing rod): not held.
        if self.func_8083BB20() {
            let sp24 = self.func_80837818(env);
            self.func_80837948(env.data, sp24);
            if sp24 >= env.data.items.mwa("SPIN_ATTACK_1H") {
                self.state2 |= STATE2_17;
                // func_80837530 (spin attack effects): not ported.
            }
            return true;
        }
        false
    }

    /// `func_8083BB20`.
    fn func_8083BB20(&self) -> bool {
        self.state1 & STATE1_22 == 0 && Self::melee_weapon(self.held_item_ap) != 0 && self.s.d_80853614
    }

    /// `func_808375D8`: the stick spun a quarter-turn each of the last 3 frames (a quick spin).
    fn func_808375D8(&self) -> bool {
        let mut sp = [0i32; 4];
        for (i, v) in self.unk_847.iter().enumerate() {
            if *v < 0 {
                return false;
            }
            sp[i] = (*v as i32 * 2) as i8 as i32;
        }
        let t1 = (sp[0] - sp[1]) as i8 as i32;
        if t1.abs() < 10 {
            return false;
        }
        for i in 1..3 {
            let t2 = (sp[i] - sp[i + 1]) as i8 as i32;
            if t2.abs() < 10 || t2 * t1 < 0 {
                return false;
            }
        }
        true
    }

    /// `func_80837818`: which attack, by stick direction (`D_80854480`).
    fn func_80837818(&mut self, env: &Env) -> usize {
        let it = &env.data.items;
        let sp1c = self.stick_dir();
        let sp18 = if self.func_808375D8() {
            it.mwa("SPIN_ATTACK_1H")
        } else if sp1c < 0 {
            if self.func_80833BCC() { it.mwa("FORWARD_SLASH_1H") } else { it.mwa("RIGHT_SLASH_1H") }
        } else {
            let mut a = it.attack_by_dir[sp1c as usize];
            if a == it.mwa("STAB_1H") {
                self.state2 |= STATE2_30;
                if !self.func_80833BCC() {
                    a = it.mwa("FORWARD_SLASH_1H");
                }
            }
            a
        };
        // Deku sticks and the two-handed Biggoron's Sword: not held.
        sp18
    }

    /// `func_80837948`: start attack `arg2`; the third of the same attack in a row is its combo.
    fn func_80837948(&mut self, data: &GameData, mut arg2: usize) {
        let it = &data.items;
        let (flip0, jump1) = (it.mwa("FLIPSLASH_START"), it.mwa("JUMPSLASH_FINISH"));
        self.setup_action(data, Action::Attack, 0);
        self.unk_844 = 8;
        if !(arg2 >= it.mwa("FLIPSLASH_FINISH") && arg2 <= jump1) {
            self.func_80832318();
        }
        if arg2 != self.melee_weapon_animation || self.unk_845 >= 3 {
            self.unk_845 = 0;
        }
        self.unk_845 += 1;
        if self.unk_845 >= 3 {
            arg2 += 2;
        }
        self.melee_weapon_animation = arg2;
        let a = it.attacks[arg2].anim;
        self.skel.play_once_set_speed(data, a, 2.0 / 3.0);
        if arg2 != flip0 && arg2 != it.mwa("JUMPSLASH_START") {
            self.func_80832F54(0x209);
        }
        self.current_yaw = self.actor.shape_rot.y;
        // func_80837918: the weapon colliders' damage flags (no actor collision here).
    }

    /// `func_80832318`: weapon inactive.
    fn func_80832318(&mut self) {
        self.state2 &= !STATE2_17;
        self.melee_weapon_state = 0;
    }

    /// `func_80832F54`: animation-driven movement with `flags` (`ANIM_FLAG_*`, 0x200 scales
    /// the start translation by `ageProperties->unk_08`).
    fn func_80832F54(&mut self, flags: u16) {
        if flags & 0x200 != 0 {
            // func_80832D20.
            self.func_80832CFC();
            let k = self.age.translation_scale;
            self.skel.prev_transl = self.skel.prev_transl.map(|v| (v as f32 * k) as i16);
        } else if flags & 0x100 != 0 || self.skel.move_flags != 0 {
            self.func_80832CFC();
        } else {
            self.skel.prev_transl = self.skel.joint[0];
            self.skel.prev_rot = self.actor.shape_rot.y;
        }
        self.skel.move_flags = flags;
        self.func_80832210();
        self.skel.disable_queue();
    }

    /// `func_8084285C`: the weapon is active from `arg1` to `arg3` (`func_80833A20`).
    fn func_8084285C(&mut self, arg1: f32, arg2: f32, arg3: f32) -> bool {
        let f = self.skel.cur_frame;
        if arg1 <= f && f <= arg3 {
            self.melee_weapon_state = if arg2 <= f { 1 } else { -1 };
            return true;
        }
        self.func_80832318();
        false
    }

    /// `func_8083C50C`: B released during the attack.
    fn func_8083C50C(&mut self) {
        if self.unk_844 > 0 && !self.input.cur.held(crate::input::BTN_B) {
            self.unk_844 = -self.unk_844;
        }
    }

    /// `func_808502D0`: attacking. At the end another B press chains (`func_80850224`);
    /// otherwise the end animation plays into the (targeting) stance.
    fn func_808502D0(&mut self, env: &Env) {
        let data = env.data;
        let a = data.items.attacks[self.melee_weapon_animation];
        self.state2 |= STATE2_5;
        // func_80842DF4 (weapon hits and recoil off walls): no actor/weapon collision.
        self.func_8084285C(0.0, a.active_start, a.active_end);
        if self.state2 & STATE2_30 != 0 && self.skel.on_frame(0.0) {
            self.linear_velocity = 15.0;
            self.state2 &= !STATE2_30;
        }
        // linearVelocity > 12: dust (func_8084269C).
        step_to_f(&mut self.linear_velocity, 0.0, 5.0);
        self.func_8083C50C();
        if self.skel.update(data) && !self.func_80850224(env) {
            let mf = self.skel.move_flags;
            let end = if self.state1 & STATE1_4 != 0 { a.end_locked } else { a.end };
            self.func_80832318();
            self.skel.move_flags = 0;
            self.func_8083A098(data, end);
            self.skel.move_flags = mf;
            self.state3 |= STATE3_3;
        }
    }

    // ================================================================================
    // Water (func_8083D53C) and swimming

    /// `func_8083D53C`: entering and leaving water.
    fn func_8083D53C(&mut self, env: &Env) {
        let a = self.action;
        if a == Action::ClimbLedge || a == Action::ClimbUp {
            return;
        }
        let swim_actions = matches!(a, Action::Swim | Action::SwimMove | Action::SwimTarget | Action::Dive | Action::Surface);
        if self.age.unk_2C < self.actor.y_dist_to_water {
            // Kokiri boots; func_8084E30C / func_8084E368 / func_8084D7C4 are not ported.
            if self.state1 & STATE1_27 == 0 || !swim_actions {
                self.func_8083D36C(env);
            }
        } else if self.state1 & STATE1_27 != 0 && self.actor.y_dist_to_water < self.age.unk_24 {
            if self.skel.move_flags == 0 {
                let y = self.actor.shape_rot.y;
                self.func_8083CD54(env.data, y);
            }
            let vy = self.actor.velocity.y;
            self.func_8083D0A8(vy);
        }
    }

    /// `func_8083D36C`: start swimming.
    fn func_8083D36C(&mut self, env: &Env) {
        let data = env.data;
        // func_80832564.
        self.func_80832440();
        if self.state2 & STATE2_10 != 0 {
            self.state2 &= !STATE2_10;
            self.func_8083D12C(env, false);
            self.unk_84F = 1;
        } else {
            // (From func_80844A44, the hookshot flight, it would dive: not ported.)
            self.setup_action(data, Action::Swim, 1);
            let a = if self.grounded() { data.anim("link_swimer_wait2swim_wait") } else { data.anim("link_swimer_land2swim_wait") };
            self.func_80832B0C(data, a);
        }
        // func_8083CFA8: the splash (effects not modelled).
        self.state1 |= STATE1_27;
        self.state2 |= STATE2_10;
        self.state1 &= !(STATE1_18 | STATE1_19);
        // Player_SetBootData: Kokiri boots keep their data in water.
    }

    /// `func_8083D0A8`: out of the water.
    fn func_8083D0A8(&mut self, _vy: f32) {
        self.state1 |= STATE1_18;
        self.state1 &= !STATE1_27;
        self.func_80832340();
    }

    /// `func_80832340`.
    fn func_80832340(&mut self) {
        self.state2 &= !(STATE2_10 | STATE2_11);
    }

    /// `func_8083D12C`: dive on A from the surface (`input`), or surface when rising close to it.
    fn func_8083D12C(&mut self, env: &Env, input: bool) -> bool {
        let data = env.data;
        if self.state1 & STATE1_10 == 0 && self.state2 & STATE2_10 == 0 && (!input || (self.input.press.held(BTN_A) && abs16(self.unk_6C2) < 12000)) {
            self.setup_action(data, Action::Dive, 0);
            let a = data.anim("link_swimer_swim_deep_start");
            self.skel.play_once(data, a);
            self.unk_6C2 = 0;
            self.state2 |= STATE2_10;
            self.actor.velocity.y = 0.0;
            if input {
                self.state2 |= STATE2_11;
            }
            return true;
        }
        if (self.state1 & STATE1_10 != 0 || self.state2 & STATE2_10 != 0) && self.actor.velocity.y > 0.0 && self.actor.y_dist_to_water < self.age.unk_30 {
            self.state2 &= !STATE2_10;
            if input {
                self.setup_action(data, Action::Surface, 1);
                self.unk_850 = 2;
            }
            self.func_80832340();
            let a = data.anim("link_swimer_swim_deep_end");
            self.func_80832B0C(data, a);
            return true;
        }
        false
    }

    /// `func_8084B000`: buoyancy. Floats up to `unk_28` below the surface; sinks slowly when
    /// deeper than 100 (and flags `PLAYER_STATE2_10`).
    fn func_8084B000(&mut self) {
        let mut phi_f14 = -5.0f32;
        let mut phi_f16 = self.age.unk_28;
        if self.actor.velocity.y < 0.0 {
            phi_f16 += 1.0;
        }
        let phi_f18;
        if self.actor.y_dist_to_water < phi_f16 {
            phi_f16 = if self.actor.velocity.y <= 0.0 { 0.0 } else { self.actor.velocity.y * 0.5 };
            phi_f18 = -0.1 - phi_f16;
        } else {
            phi_f14 = 2.0;
            phi_f16 = if self.actor.velocity.y >= 0.0 { 0.0 } else { self.actor.velocity.y * -0.3 };
            phi_f18 = phi_f16 + 0.1;
            if self.actor.y_dist_to_water > 100.0 {
                self.state2 |= STATE2_10;
            }
        }
        self.actor.velocity.y += phi_f18;
        if (self.actor.velocity.y - phi_f14) * phi_f18 > 0.0 {
            self.actor.velocity.y = phi_f14;
        }
        self.actor.gravity = 0.0;
    }

    /// `func_8084AEEC`: swim acceleration, only in frames 10..20 of the stroke; turn at 1600.
    fn func_8084AEEC(&mut self, v: f32, mut arg2: f32, arg3: i16) -> f32 {
        let mut v = v;
        let mut t = self.skel.cur_frame - 10.0;
        let limit = self.regs.run_speed_limit() * 0.8;
        if v > limit {
            v = limit;
        }
        if 0.0 < t && t < 10.0 {
            t *= 6.0;
        } else {
            t = 0.0;
            arg2 = 0.0;
        }
        let dec = v.abs() * 0.02 + 0.05;
        asym_step_to_f(&mut v, arg2 * 0.8, t, dec);
        scaled_step_to_s(&mut self.current_yaw, arg3, 1600);
        v
    }

    /// `func_8084B158`: stroke animation speed from the swim speed (doubled on A/B).
    fn func_8084B158(&mut self, data: &GameData, input: bool, arg3: f32) {
        let k = if input && (self.input.press.held(BTN_A) || self.input.press.held(crate::input::BTN_B)) { 1.0 } else { 0.5 };
        self.skel.play_speed = (k * arg3).max(1.0);
        self.skel.update(data);
    }

    /// `func_80832C6C`: loop `anim`, blending over 16 frames.
    fn func_80832C6C(&mut self, data: &GameData, anim: AnimId) {
        self.skel.change(data, anim, 1.0, 0.0, 0.0, ANIMMODE_LOOP, -16.0);
    }

    /// `func_8084D574` / `func_8084D5CC` / `func_80838F18`: swim forward, swim targeting, tread.
    fn func_8084D574(&mut self, data: &GameData, yaw: i16) {
        self.setup_action(data, Action::SwimMove, 0);
        self.current_yaw = yaw;
        self.actor.shape_rot.y = yaw;
        let a = data.anim("link_swimer_swim");
        self.func_80832C6C(data, a);
    }
    fn func_8084D5CC(&mut self, data: &GameData) {
        self.setup_action(data, Action::SwimTarget, 0);
        let a = data.anim("link_swimer_swim");
        self.func_80832C6C(data, a);
    }
    fn func_80838F18(&mut self, data: &GameData) {
        self.setup_action(data, Action::Swim, 0);
        let a = data.anim("link_swimer_swim_wait");
        self.func_80832C6C(data, a);
    }

    /// `func_8084D610`: treading water.
    fn func_8084D610(&mut self, env: &Env) {
        let data = env.data;
        // func_80832CB0.
        if self.skel.update(data) {
            let a = data.anim("link_swimer_swim_wait");
            self.skel.play_loop(data, a);
        }
        self.func_8084B000();
        if !self.func_80837348(env, Self::D_80854444, true) && !self.func_8083D12C(env, true) {
            if self.unk_6AD != 1 {
                self.unk_6AD = 0;
            }
            let (_, sp34, sp32) = self.func_80837268(env, 0.0);
            if sp34 != 0.0 {
                let t = self.actor.shape_rot.y.wrapping_sub(sp32);
                if abs16(t) > 0x6000 && !step_to_f(&mut self.linear_velocity, 0.0, 1.0) {
                    return;
                }
                if self.func_80833C04(env) {
                    self.func_8084D5CC(data);
                } else {
                    self.func_8084D574(data, sp32);
                }
            }
            let v = self.linear_velocity;
            self.linear_velocity = self.func_8084AEEC(v, sp34, sp32);
        }
    }

    /// `func_8084D84C`: swimming forward.
    fn func_8084D84C(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5;
        let v = self.linear_velocity;
        self.func_8084B158(data, true, v);
        self.func_8084B000();
        if !self.func_80837348(env, Self::D_80854444, true) && !self.func_8083D12C(env, true) {
            let (_, sp34, sp32) = self.func_80837268(env, 0.0);
            let t = self.actor.shape_rot.y.wrapping_sub(sp32);
            if sp34 == 0.0 || abs16(t) > 0x6000 {
                self.func_80838F18(data);
            } else if self.func_80833C04(env) {
                self.func_8084D5CC(data);
            }
            // func_8084D530: func_8084AEEC plus the stroke sounds (D_808549D0).
            let v = self.linear_velocity;
            self.linear_velocity = self.func_8084AEEC(v, sp34, sp32);
        }
    }

    /// `func_8084DAB4`: swimming while targeting (forward, back or sideways strokes).
    fn func_8084DAB4(&mut self, env: &Env) {
        let data = env.data;
        let v = self.linear_velocity;
        self.func_8084B158(data, true, v);
        self.func_8084B000();
        if !self.func_80837348(env, Self::D_80854444, true) && !self.func_8083D12C(env, true) {
            let (_, mut sp2c, mut sp2a) = self.func_80837268(env, 0.0);
            if sp2c == 0.0 {
                self.func_80838F18(data);
            } else if !self.func_80833C04(env) {
                self.func_8084D574(data, sp2a);
            } else {
                self.func_8084D980(env, &mut sp2c, &mut sp2a);
            }
            let v = self.linear_velocity;
            self.linear_velocity = self.func_8084AEEC(v, sp2c, sp2a);
        }
    }

    /// `func_8084D980`: the stroke for the direction while targeting.
    fn func_8084D980(&mut self, env: &Env, speed: &mut f32, yaw: &mut i16) -> bool {
        let data = env.data;
        let t1 = self.current_yaw.wrapping_sub(*yaw);
        let anim = if abs16(t1) > 0x6000 {
            if step_to_f(&mut self.linear_velocity, 0.0, 1.0) {
                self.current_yaw = *yaw;
            } else {
                *speed = 0.0;
                *yaw = self.current_yaw;
            }
            data.anim("link_swimer_swim_wait")
        } else {
            let t2 = self.func_8083FD78(env, speed, yaw);
            if t2 > 0 {
                data.anim("link_swimer_swim")
            } else if t2 < 0 {
                data.anim("link_swimer_back_swim")
            } else if self.actor.shape_rot.y.wrapping_sub(*yaw) > 0 {
                data.anim("link_swimer_Rside_swim")
            } else {
                data.anim("link_swimer_Lside_swim")
            }
        };
        if anim != self.skel.animation {
            self.func_80832C6C(data, anim);
            return true;
        }
        false
    }

    /// `func_8083D330`.
    fn func_8083D330(&mut self, data: &GameData) {
        let a = data.anim("link_swimer_swim");
        self.skel.play_loop(data, a);
        self.unk_6C2 = 16000;
        self.unk_850 = 1;
    }

    /// `func_8084DBC4`: move under water (horizontal and vertical swim speeds).
    fn func_8084DBC4(&mut self, env: &Env, arg2: f32) {
        let (_, sp2c, sp2a) = self.func_80837268(env, 0.0);
        let v = self.linear_velocity;
        self.linear_velocity = self.func_8084AEEC(v, sp2c * 0.5, sp2a);
        let vy = self.actor.velocity.y;
        let cy = self.current_yaw;
        self.actor.velocity.y = self.func_8084AEEC(vy, arg2, cy);
    }

    /// `func_8084DC48`: diving: the dive start, holding A to go deeper (to D_80854784[no scale]
    /// = 120 below the surface), then pitching back up and rising.
    fn func_8084DC48(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5;
        self.actor.gravity = 0.0;
        self.func_80836670(env);
        // func_8083B040 (C-button items): none.
        if self.unk_84F == 0 {
            if self.unk_850 == 0 {
                if self.skel.update(data) || (self.skel.cur_frame >= 22.0 && !self.input.cur.held(BTN_A)) {
                    self.func_8083D330(data);
                } else if self.skel.on_frame(20.0) {
                    self.actor.velocity.y = -2.0;
                }
                self.func_8083721C();
                return;
            }
            let vy = self.actor.velocity.y;
            self.func_8084B158(data, true, vy);
            self.unk_6C2 = 16000;
            // func_8083E5A8 (picking up an item) is never taken; no scale upgrade.
            if self.input.cur.held(BTN_A) && !self.grounded() && self.actor.y_dist_to_water < 120.0 {
                self.func_8084DBC4(env, -2.0);
            } else {
                self.unk_84F += 1;
                let a = data.anim("link_swimer_swim_wait");
                self.func_80832C6C(data, a);
            }
        } else if self.unk_84F == 1 {
            self.skel.update(data);
            self.func_8084B000();
            if self.unk_6C2 < 10000 {
                self.unk_84F += 1;
                self.unk_850 = self.actor.y_dist_to_water as i16;
                let a = data.anim("link_swimer_swim");
                self.func_80832C6C(data, a);
            }
        } else if !self.func_8083D12C(env, true) {
            let sp2c = (self.unk_850 as f32 * 0.018 + 4.0).min(8.0);
            let vy = self.actor.velocity.y.abs();
            self.func_8084B158(data, true, vy);
            scaled_step_to_s(&mut self.unk_6C2, -10000, 800);
            self.func_8084DBC4(env, sp2c);
        }
    }

    /// `func_8084E1EC`: surfacing.
    fn func_8084E1EC(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5;
        if self.skel.update(data) {
            // Not holding an item from the bottom (PLAYER_STATE1_10).
            self.func_80838F18(data);
            self.func_80832340();
        }
        self.func_8084B000();
        let y = self.actor.shape_rot.y;
        let v = self.linear_velocity;
        self.linear_velocity = self.func_8084AEEC(v, 0.0, y);
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
            root_pitch: self.unk_6C2,
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
    /// `unk_6C2`: the root limb's pitch while diving (`Player_OverrideLimbDrawGameplayCommon`).
    pub root_pitch: i16,
}
