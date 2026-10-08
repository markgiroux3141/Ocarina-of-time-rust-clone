//! Shared setup for the scripted-input tests: loads Player data from the asset pack
//! (`oot import` builds it; `OOT_PACK` picks another), or skips when there is none.

#![allow(dead_code)]

use std::sync::{Arc, OnceLock};

use eng_collision::bgcheck::CollisionContext;
use eng_input::pad::{Input, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_game::play::{PlayState, scripted_input};
use oot_game::player_lib::PlayerRules;
use oot_game::course;
use oot_game::data::GameData;
use oot_game::pack::GamePack;

/// The asset pack, opened once.
pub fn pack() -> Option<&'static GamePack> {
    static PACK: OnceLock<Option<GamePack>> = OnceLock::new();
    PACK.get_or_init(|| match GamePack::open_default() {
        Ok(p) => Some(p),
        Err(e) => {
            eprintln!("skipping: {e:#}");
            None
        }
    })
    .as_ref()
}

/// Player's draw rules from the pack.
pub fn rules() -> Option<Arc<PlayerRules>> {
    static RULES: OnceLock<Option<Arc<PlayerRules>>> = OnceLock::new();
    RULES.get_or_init(|| Some(Arc::new(pack()?.player_rules().expect("PlayerRules from the pack")))).clone()
}

/// A play state with Player at `pos` facing `yaw` on `col` (what the spikes' `World::new` was).
pub fn new_world(data: Arc<GameData>, col: CollisionContext, adult: bool, pos: Vec3, yaw: i16) -> PlayState {
    oot_actors::new_play(data, rules().expect("the pack"), col, adult, pos, yaw)
}

pub fn data() -> Option<Arc<GameData>> {
    static DATA: OnceLock<Option<Arc<GameData>>> = OnceLock::new();
    DATA.get_or_init(|| Some(Arc::new(pack()?.game_data().expect("GameData from the pack")))).clone()
}

pub fn world_at(pos: Vec3, yaw: i16) -> Option<PlayState> {
    let d = data()?;
    let c = course::build();
    Some(new_world(d, CollisionContext::new(c.collision), true, pos, yaw))
}

pub fn world() -> Option<PlayState> {
    let c = course::build();
    world_at(c.spawn, c.spawn_yaw)
}

/// One frame's record of what the tests check.
#[derive(Debug, Clone)]
pub struct Frame {
    pub n: u32,
    pub action: String,
    pub pos: Vec3,
    pub speed: f32,
    pub vy: f32,
    pub yaw: i16,
    pub facing: i16,
    pub grounded: bool,
    pub anim: String,
    pub anim_frame: f32,
    /// `shape.yOffset`.
    pub y_offset: f32,
}

/// A stick in N64 units (the pad reports ±80 at full tilt on an original controller).
pub fn stick(x: i8, y: i8) -> PadState {
    PadState { button: 0, stick_x: x, stick_y: y }
}

pub fn with(mut p: PadState, buttons: u16) -> PadState {
    p.button |= buttons;
    p
}

/// Runs `script` (one pad state per frame) and records every frame.
pub fn run(w: &mut PlayState, script: &[PadState]) -> Vec<Frame> {
    let mut prev = PadState::default();
    let mut out = Vec::new();
    for &cur in script {
        let input: Input = scripted_input(prev, cur);
        w.tick_with(input);
        prev = cur;
        let p = w.player();
        out.push(Frame {
            n: w.gameplay_frames,
            action: format!("{:?}", p.action),
            pos: p.actor.world_pos,
            speed: p.linear_velocity,
            vy: p.actor.velocity.y,
            yaw: p.current_yaw,
            facing: p.actor.shape_rot.y,
            grounded: p.grounded(),
            anim: w.data.anim_name(p.skel.animation).to_string(),
            anim_frame: p.skel.cur_frame,
            y_offset: p.actor.shape_y_offset,
        });
    }
    out
}

pub fn repeat(p: PadState, n: usize) -> Vec<PadState> {
    vec![p; n]
}

pub fn dump(frames: &[Frame]) {
    for f in frames {
        println!(
            "{:4} {:14} pos=({:8.2},{:7.2},{:8.2}) v={:6.3} vy={:6.2} yaw={:6} face={:6} g={} {} @{:.1}",
            f.n, f.action, f.pos.x, f.pos.y, f.pos.z, f.speed, f.vy, f.yaw, f.facing, f.grounded as u8, f.anim, f.anim_frame
        );
    }
}

/// The pause menu as a player uses it (GAME-05 milestone 5b): Start until the menu is open and
/// idle, the item page's cursor to `slot` (along its row, then its column; each move a push of
/// the stick and a release), C button `button` (0 C-Left, 1 C-Down, 2 C-Right) until the equip is
/// over, then Start until the game resumes.
pub fn pause_equip(w: &mut PlayState, slot: u16, button: usize) {
    use eng_input::pad::{BTN_CDOWN, BTN_CLEFT, BTN_CRIGHT, BTN_START};
    use oot_game::kaleido::*;
    let tick = |w: &mut PlayState, cur: PadState| w.tick_with(scripted_input(PadState::default(), cur));
    let idle_menu = |w: &PlayState| w.pause_ctx.state == PAUSE_STATE_MAIN && w.pause_ctx.main_state == PAUSE_MAIN_STATE_IDLE;
    // Start, again each other frame until KaleidoSetup_Update takes it (not during a
    // transition or a cutscene).
    for f in 0..300 {
        if idle_menu(w) {
            break;
        }
        let start = w.pause_ctx.state == PAUSE_STATE_OFF && f % 2 == 0;
        tick(w, PadState { button: if start { BTN_START } else { 0 }, ..Default::default() });
    }
    assert!(idle_menu(w), "the pause menu didn't open (state {})", w.pause_ctx.state);
    assert_eq!(w.pause_ctx.page_index, PAUSE_ITEM, "the menu opened on another page");
    for _ in 0..40 {
        let p = &w.pause_ctx;
        let (x, y) = (p.cursor_x[0], p.cursor_y[0]);
        let (tx, ty) = ((slot % 6) as i16, (slot / 6) as i16);
        let (sx, sy) = if p.cursor_special_pos == PAUSE_CURSOR_PAGE_LEFT {
            (80, 0)
        } else if p.cursor_special_pos == PAUSE_CURSOR_PAGE_RIGHT {
            (-80, 0)
        } else if x != tx {
            (if tx < x { -80 } else { 80 }, 0)
        } else if y != ty {
            (0, if ty < y { 80 } else { -80 })
        } else {
            break;
        };
        tick(w, PadState { button: 0, stick_x: sx, stick_y: sy });
        tick(w, PadState::default());
    }
    assert_eq!((w.pause_ctx.cursor_point[0], w.pause_ctx.cursor_special_pos), (slot as i16, 0), "the cursor didn't get to slot {slot}");
    tick(w, PadState { button: [BTN_CLEFT, BTN_CDOWN, BTN_CRIGHT][button], ..Default::default() });
    for _ in 0..40 {
        if idle_menu(w) {
            break;
        }
        tick(w, PadState::default());
    }
    tick(w, PadState { button: BTN_START, ..Default::default() });
    for _ in 0..40 {
        if !w.pause_ctx.is_paused() {
            break;
        }
        tick(w, PadState::default());
    }
    assert!(!w.pause_ctx.is_paused(), "the pause menu didn't close");
}
