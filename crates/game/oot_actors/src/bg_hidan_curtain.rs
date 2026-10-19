//! `Bg_Hidan_Curtain` (`ovl_Bg_Hidan_Curtain/z_bg_hidan_curtain.c`): a fire wall, a ring of flames
//! (`gEffFireCircleDL`) that burns and knocks Link back, put out by its switch flag (GAME-09
//! milestone 3). Dodongo's Cavern (Master Quest) places three in room 8 (0x2FFC, flag 0x3C) and two
//! in room 12 (0x2FF9 and 0x2FFA, flags 0x39 and 0x3A); the Fire Temple, the Spirit Temple, Ganon's
//! Castle and Kakariko's cutscene layer 4 more.
//!
//! - **Its types** (`type`, 0 to 6; another is killed at init): 0 burns until the room is clear,
//!   then goes out for good (and isn't there once it's clear); 6 the same, but put out by its
//!   switch flag; 1 is gone once its chest flag is set, and its flag starts one-point cutscene 3350
//!   (50 frames) before it goes out for 300; 2 (small) goes out for 400; 3 starts one-point 3360
//!   (30 frames) and goes out for 160; 4 (small) and 5 are the other way up: out until the flag is
//!   set, then 4 burns for 200, 5 until the flag is cleared.
//! - **Going out and back** (`BgHidanCurtain_TurnOff`, `_TurnOn`): it sinks (5 a frame, 3 the small
//!   one) by its height (144, 88) and rises again; risen back it clears its switch flag
//!   (`Flags_UnsetSwitch`) and waits for it again. The timed types count down only once it's out
//!   (`BgHidanCurtain_WaitForTimer`; 1 and 3 with the timer's tick, `Actor_PlaySfx_FlaggedTimer`).
//! - **The fire**: its alpha is how far it's risen (255 at the top); over 50 its cylinder (81
//!   across and 144 high, 46 and 88 the small one, scaled with the rise) hurts (4,
//!   `HIT_SPECIAL_EFFECT_FIRE`: Link burns) and pushes Link (immovable), and a hit knocks him down
//!   (`Actor_SetPlayerKnockbackLargeNoDamage`, 5 away, 1 up); `NA_SE_EV_FIRE_PILLAR_S` while it
//!   burns (not in a cutscene layer). The cylinder stays where the wall was placed. Nothing of
//!   this (nor the flames' scroll) while the main camera opens a big chest or turns about
//!   (`CAM_SET_SLOW_CHEST_CS`, `CAM_SET_TURN_AROUND`).
//!
//! The whole overlay is ported. The one-point cutscenes 3350 and 3360 (the Fire Temple's) aren't:
//! `OnePointCutscene_Init` logs them (`oot_game::onepoint`), and the wall waits its frames all the
//! same. The draw (`BgHidanCurtain_Draw`) is `gEffFireCircleDL` after `Gfx_SetupDL_25Xlu`, yellow
//! (255, 220, 0) at its alpha over red, its texture scrolling by `texScroll`.
//!
//! Params: `type` bits 12 to 15 (0 to 6 as above; 7 and up killed); `treasureFlag` bits 6 to 11
//! (type 1: the chest flag that takes it away); `switchFlag` bits 0 to 5 (types 1 to 6; the params
//! are masked to it at init).
//! Flags: reads the room's clear flag (types 0 and 6); reads the chest flag `treasureFlag` (type
//! 1); reads and clears the switch flag `switchFlag`.

use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::step_to_f;
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile, actor_set_player_knockback_large_no_damage};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::sfx::SFX_FLAG;
use oot_game::camera::{CAM_ID_MAIN, CAM_SET_SLOW_CHEST_CS, CAM_SET_TURN_AROUND};
use oot_game::collision_check::*;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::scene_table::gfx_two_tex_scroll;

/// `ACTOR_BG_HIDAN_CURTAIN` (`actor_table.h`: 0x0049).
pub const ACTOR_BG_HIDAN_CURTAIN: i16 = 0x0049;
/// `OBJECT_GAMEPLAY_KEEP`.
pub const OBJECT: &str = "gameplay_keep";

/// `Bg_Hidan_Curtain_Profile`: `ACTORCAT_PROP`, always updating.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_BG_HIDAN_CURTAIN, name: "Bg_Hidan_Curtain", category: ACTORCAT_PROP, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `PARAMS_GET_U(params, 12, 4)`: the type.
pub const TYPE: Field = Field::params(
    "type",
    12,
    4,
    Kind::Named(&[
        (0, "out once the room is clear, then gone"),
        (1, "the switch flag: cutscene 3350, out 300; gone with the chest flag"),
        (2, "small; the switch flag puts it out for 400"),
        (3, "the switch flag: cutscene 3360, out 160"),
        (4, "small, the other way up: the switch flag lights it for 200"),
        (5, "the other way up: lit while the switch flag is set"),
        (6, "the switch flag puts it out for good; gone once the room is clear"),
    ]),
    "what puts it out (or lights it), and for how long",
);
/// `PARAMS_GET_U(params, 6, 6)`: `treasureFlag`.
pub const TREASURE_FLAG: Field = Field::params("treasureFlag", 6, 6, Kind::ChestFlag, "type 1: set, it's gone (0x3F in the other types' placements)");
/// `params & 0x3F`: the switch flag.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 0, 6, Kind::SwitchFlag, "set: out (types 1, 2, 3, 6) or lit (4, 5); cleared when it's back");

/// Bg_Hidan_Curtain's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, TREASURE_FLAG, SWITCH_FLAG],
    flags: &[
        FlagUse::new(Flag::Clear).reads().note("types 0 and 6: gone at init once the room is clear; type 0 goes out on it"),
        FlagUse::new(Flag::Chest("treasureFlag")).reads().note("type 1: gone at init, or once out, with it set"),
        FlagUse::new(Flag::Switch("switchFlag")).reads().clears().note("set: out (or lit, types 4 and 5); cleared once it's back"),
    ],
    todo: false,
    objects: &[],
    texts: &[],
};

/// `NA_SE_EV_FIRE_PILLAR_S` (`environmentbank_table.h`: 0x28A3).
pub const NA_SE_EV_FIRE_PILLAR_S: u16 = 0x28A3;

/// `BgHidanCurtainParams`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BgHidanCurtainParams {
    pub radius: i16,
    pub height: i16,
    pub scale: f32,
    pub rise_dist: f32,
    pub rise_speed: f32,
}

/// `sHCParams`: the big one's and the small one's (`size`).
pub const HC_PARAMS: [BgHidanCurtainParams; 2] = [
    BgHidanCurtainParams { radius: 81, height: 144, scale: 0.090, rise_dist: 144.0, rise_speed: 5.0 },
    BgHidanCurtainParams { radius: 46, height: 88, scale: 0.055, rise_dist: 88.0, rise_speed: 3.0 },
];

/// `sCylinderInit`: the flames, hurting 4 with fire, pushing Link.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_PLAYER, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x2000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_FIRE, damage: 0x04 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 81, height: 144, y_shift: 0, pos: [0, 0, 0] },
};

/// One-point cutscenes 3350 and 3360 (`z_onepointdemo.c`: the Fire Temple's walls).
const ONEPOINT_3350: i16 = 3350;
const ONEPOINT_3360: i16 = 3360;

/// The draw's segments: `gDPSetPrimColor`/`gDPSetEnvColor` (a free one, run first), and
/// `Gfx_TwoTexScroll` on 8.
const SEG_COLOR: u8 = 0x0E;
const SEG_TEX: u8 = 0x08;
const BAKE: &str = "Bg_Hidan_Curtain/flames";

/// `Gfx_TwoTexScroll(G_TX_RENDERTILE, texScroll & 0x7F, 0, 32, 64, 1, 0, (texScroll × -0xF) &
/// 0xFF, 32, 64)` (`texScroll` an s16).
fn scroll(tex_scroll: i16) -> Vec<(u32, u32)> {
    let t = tex_scroll as i32;
    gfx_two_tex_scroll(0, (t & 0x7F) as u32, 0, 0x20, 0x40, 1, 0, (t.wrapping_mul(-0xF) & 0xFF) as u32, 0x20, 0x40)
}

/// `BgHidanCurtain_Draw`'s mesh: `gEffFireCircleDL` (`Gfx_SetupDL_25Xlu` before it: the list sets
/// its own render mode), its colours (prim with LOD fraction 0x80 and minimum LOD 0x80) and the
/// scroll on segment 8 dynamic.
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: BAKE.into(),
        object: OBJECT.into(),
        segments: vec![(SEG_COLOR, BakeSegment::Dynamic(vec![(0xFA00_8080, 0xFFDC_00FF), (0xFB00_0000, 0xFF00_0000), (0xDF00_0000, 0)])), (SEG_TEX, BakeSegment::Dynamic(scroll(0)))],
        prelude: vec![SEG_COLOR],
        body: BakeBody::DLists(vec![(OBJECT.into(), "gEffFireCircleDL".into())]),
    }]
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    WaitForSwitchOn,
    WaitForCutscene,
    WaitForClear,
    WaitForSwitchOff,
    TurnOn,
    TurnOff,
    WaitForTimer,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::WaitForSwitchOn => "BgHidanCurtain_WaitForSwitchOn",
            Action::WaitForCutscene => "BgHidanCurtain_WaitForCutscene",
            Action::WaitForClear => "BgHidanCurtain_WaitForClear",
            Action::WaitForSwitchOff => "BgHidanCurtain_WaitForSwitchOff",
            Action::TurnOn => "BgHidanCurtain_TurnOn",
            Action::TurnOff => "BgHidanCurtain_TurnOff",
            Action::WaitForTimer => "BgHidanCurtain_WaitForTimer",
        }
    }
}

pub struct BgHidanCurtain {
    pub actor: Actor,
    pub action: Action,
    /// `type`.
    pub ty: u8,
    pub treasure_flag: u8,
    /// `size`: 0 big, 1 small (types 2 and 4).
    pub size: u8,
    pub alpha: u8,
    pub timer: i16,
    pub tex_scroll: i16,
    pub collider: ColliderCylinder,
}

impl BgHidanCurtain {
    fn hc_params(&self) -> BgHidanCurtainParams {
        HC_PARAMS[self.size as usize]
    }

    fn switch_flag(&self) -> i32 {
        SWITCH_FLAG.get(self.actor.params) as i32
    }

    /// `BgHidanCurtain_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // schema: the whole params printed.
        log::debug!("Curtain (arg_data 0x{:04x})", actor.params);
        let mut this = BgHidanCurtain {
            actor,
            action: Action::WaitForSwitchOn,
            ty: 0,
            treasure_flag: 0,
            size: 0,
            alpha: 0,
            timer: 0,
            tex_scroll: 0,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
        };
        this.actor.set_focus(20.0);
        this.ty = TYPE.get(this.actor.params) as u8;
        if this.ty > 6 {
            // schema: the whole params printed.
            log::error!("Error : object type is not set (../z_bg_hidan_curtain.c 352)(arg_data 0x{:04x})", this.actor.params);
            this.actor.kill();
            return Box::new(this);
        }
        this.size = if this.ty == 2 || this.ty == 4 { 1 } else { 0 };
        let hc = this.hc_params();
        this.treasure_flag = TREASURE_FLAG.get(this.actor.params) as u8;
        this.actor.params &= 0x3F; // schema: the C's write: the params are the switch flag from here
        // DEBUG_FEATURES: never true once masked. schema: the whole params checked.
        if this.actor.params < 0 || this.actor.params > 0x3F {
            // schema: the whole params printed.
            log::warn!("Warning : object save bit is not set (../z_bg_hidan_curtain.c 373)(arg_data 0x{:04x})", this.actor.params);
        }
        this.actor.scale = Vec3::splat(hc.scale);
        let w = this.actor.world_pos;
        this.collider.dim.pos = [w.x as i16, w.y as i16, w.z as i16];
        this.collider.dim.radius = hc.radius;
        this.collider.dim.height = hc.height;
        this.collider.update(&this.actor);
        // CollisionCheck_SetInfo(&colChkInfo, NULL, &sCcInfoInit): health 1, 80 across, 100 high,
        // immovable.
        this.actor.col_chk_info.health = 1;
        this.actor.col_chk_info.cyl_radius = 80;
        this.actor.col_chk_info.cyl_height = 100;
        this.actor.col_chk_info.mass = MASS_IMMOVABLE;
        if this.ty == 0 {
            this.action = Action::WaitForClear;
        } else {
            this.action = Action::WaitForSwitchOn;
            if this.ty == 4 || this.ty == 5 {
                this.actor.world_pos.y = this.actor.home_pos.y - hc.rise_dist;
            }
        }
        if (this.ty == 1 && play.flags.get_treasure(this.treasure_flag as i32)) || ((this.ty == 0 || this.ty == 6) && play.flags.get_clear(this.actor.room)) {
            this.actor.kill();
        }
        this.tex_scroll = (play.rand.zero_one() * 15.0) as i16;
        Box::new(this)
    }

    /// `OnePointCutscene_Init(play, cs_id, timer, &this->actor, CAM_ID_MAIN)`.
    fn onepoint(&self, play: &mut PlayState, cs_id: i16, timer: i16) {
        let me = play.cur_actor.map(|h| play.cam_actor_of(h, &self.actor));
        play.onepoint_cutscene_init(cs_id, timer, me, CAM_ID_MAIN);
    }

    /// `BgHidanCurtain_WaitForSwitchOn`: its flag set: type 1 one-point 3350 and 50 frames, type 3
    /// one-point 3360 and 30, the others out at once.
    fn wait_for_switch_on(&mut self, play: &mut PlayState) {
        if play.flags.get_switch(self.switch_flag()) {
            if self.ty == 1 {
                self.action = Action::WaitForCutscene;
                self.onepoint(play, ONEPOINT_3350, -99);
                self.timer = 50;
            } else if self.ty == 3 {
                self.action = Action::WaitForCutscene;
                self.onepoint(play, ONEPOINT_3360, 60);
                self.timer = 30;
            } else {
                self.action = Action::TurnOff;
            }
        }
    }

    /// `BgHidanCurtain_WaitForCutscene`.
    fn wait_for_cutscene(&mut self) {
        let t = self.timer;
        self.timer -= 1;
        if t == 0 {
            self.action = Action::TurnOff;
        }
    }

    /// `BgHidanCurtain_WaitForClear`.
    fn wait_for_clear(&mut self, play: &PlayState) {
        if play.flags.get_clear(self.actor.room) {
            self.action = Action::TurnOff;
        }
    }

    /// `BgHidanCurtain_WaitForSwitchOff`.
    fn wait_for_switch_off(&mut self, play: &PlayState) {
        if !play.flags.get_switch(self.switch_flag()) {
            self.action = Action::TurnOn;
        }
    }

    /// `BgHidanCurtain_TurnOn`: rising back to its home; there, its flag cleared, waiting again.
    fn turn_on(&mut self, play: &mut PlayState) {
        let rise_speed = HC_PARAMS[self.size as usize].rise_speed;
        if step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y, rise_speed) {
            let flag = self.switch_flag();
            play.flags.unset_switch(flag);
            self.action = Action::WaitForSwitchOn;
        }
    }

    /// `BgHidanCurtain_TurnOff`: sinking by its height; down, gone (0 and 6), waiting for the flag
    /// to clear (5), or its timer: 400 (2), 200 (4), 160 (3), 300 (1).
    fn turn_off(&mut self) {
        let hc = self.hc_params();
        if step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y - hc.rise_dist, hc.rise_speed) {
            if self.ty == 0 || self.ty == 6 {
                self.actor.kill();
            } else if self.ty == 5 {
                self.action = Action::WaitForSwitchOff;
            } else {
                self.timer = match self.ty {
                    2 => 400,
                    4 => 200,
                    3 => 160,
                    // type 1
                    _ => 300,
                };
                self.action = Action::WaitForTimer;
            }
        }
    }

    /// `BgHidanCurtain_WaitForTimer`: then back on; 1 and 3 with the timer's tick.
    fn wait_for_timer(&mut self) {
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.timer == 0 {
            self.action = Action::TurnOn;
        }
        if self.ty == 1 || self.ty == 3 {
            self.actor.play_sfx_flagged_timer(self.timer as i32);
        }
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            Action::WaitForSwitchOn => self.wait_for_switch_on(play),
            Action::WaitForCutscene => self.wait_for_cutscene(),
            Action::WaitForClear => self.wait_for_clear(play),
            Action::WaitForSwitchOff => self.wait_for_switch_off(play),
            Action::TurnOn => self.turn_on(play),
            Action::TurnOff => self.turn_off(),
            Action::WaitForTimer => self.wait_for_timer(),
        }
    }

    /// Types 4 and 5 keep their height mirrored about half their rise (`2 × home − riseDist − y`):
    /// the actions move them as the others, the other way up.
    fn mirror(&mut self) {
        if self.ty == 4 || self.ty == 5 {
            let hc = self.hc_params();
            self.actor.world_pos.y = (2.0 * self.actor.home_pos.y) - hc.rise_dist - self.actor.world_pos.y;
        }
    }
}

impl ActorImpl for BgHidanCurtain {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BgHidanCurtain_Update`.
    fn update(&mut self, play: &mut PlayState) {
        let hc = self.hc_params();
        let setting = play.game_camera.setting;
        if setting == CAM_SET_SLOW_CHEST_CS || setting == CAM_SET_TURN_AROUND {
            self.collider.base.at_flags &= !AT_HIT;
            return;
        }
        if self.collider.base.at_flags & AT_HIT != 0 {
            self.collider.base.at_flags &= !AT_HIT;
            actor_set_player_knockback_large_no_damage(play, 5.0, self.actor.yaw_towards_player, 1.0);
        }
        self.mirror();
        self.run_action(play);
        self.mirror();
        let rise_progress = (hc.rise_dist - (self.actor.home_pos.y - self.actor.world_pos.y)) / hc.rise_dist;
        self.alpha = (255.0 * rise_progress) as u8;
        if self.alpha > 50 {
            self.collider.dim.height = (hc.height as f32 * rise_progress) as i16;
            play.collision_check_set_at(&self.actor, 0, &mut self.collider);
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
            if !play.is_cutscene_layer() {
                self.actor.play_sfx_flagged(NA_SE_EV_FIRE_PILLAR_S - SFX_FLAG);
            }
        } else if self.ty == 1 && play.flags.get_treasure(self.treasure_flag as i32) {
            self.actor.kill();
        }
        self.tex_scroll = self.tex_scroll.wrapping_add(1);
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.switches = vec![self.alpha as u32, self.tex_scroll as u16 as u32];
        rs
    }

    /// `BgHidanCurtain_Draw`: `gEffFireCircleDL` with prim `(255, 220, 0, alpha)`, env `(255, 0,
    /// 0, 0)` and its scroll.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let [alpha, tex_scroll] = rs.switches.as_slice() else { return };
        let mut sv = SegmentValues::default();
        sv.read(SEG_TEX, &scroll(*tex_scroll as u16 as i16));
        sv.prim[SEG_COLOR as usize] = Some([255, 220, 0, *alpha as u8]);
        sv.env[SEG_COLOR as usize] = Some([255, 0, 0, 0]);
        out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE)), transform: actor_draw_matrix(rs), bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
    }

    /// `BgHidanCurtain_Destroy`: `Collider_DestroyCylinder`.
    fn destroy(&mut self, _play: &mut PlayState) {}

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
