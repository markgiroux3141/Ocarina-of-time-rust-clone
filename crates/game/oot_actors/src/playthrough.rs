//! The Kokiri Forest playthrough (GAME-02 milestone 4): a scripted run from Link's bed to the
//! Deku Tree's scene, steering each frame as a player would. The `playthrough` test runs it
//! with checks at each step, and the sandbox's `playthrough` script writes its trace.
//!
//! It needs the Deku Tree's mouth open, so it runs on a save with the `deku-tree-open` preset
//! (`oot_game::save::SAVE_PRESETS`): the talk that opens it in the game is a cutscene, which
//! isn't ported yet.
//!
//! The route follows Kokiri Forest's collision: out of Link's house, down the ladder, to the
//! sign, to the Kokiri child by the bushes (child 4), the bushes, then east through the
//! stream, past Mido, along the path through the `En_Holl` into room 1, across the meadow
//! and into the mouth. Mido and Saria are placeholders, so neither stands in the way.
//!
//! **The drop depends on `Rand`.** A cut Kokiri bush draws from drop table 2, which gives
//! something for 5 of its 16 entries at full health (`func_8001F404` turns the hearts into
//! green rupees). The run cuts the four bushes by child 4 in turn until one drops. A change to
//! who calls `Rand` and when changes the draws; if all four then come up empty (about one
//! time in five), the run stops with "none of the 4 bushes dropped anything", and the list or
//! its order needs changing.

use eng_input::pad::{BTN_A, BTN_B, PadState};
use glam::Vec3;
use oot_game::actor_ctx::ActorHandle;
use oot_game::message::{TEXT_STATE_AWAITING_NEXT, TEXT_STATE_CHOICE, TEXT_STATE_DONE, TEXT_STATE_DONE_HAS_NEXT, TEXT_STATE_NONE};
use oot_game::play::PlayState;
use oot_game::transition::{TRANS_MODE_OFF, TRANS_TRIGGER_OFF};

use crate::PlayExt;
use crate::en_item00::{Action as ItemAction, EnItem00};
use crate::en_kanban::EnKanban;
use crate::en_ko::EnKo;
use crate::en_kusa::{Action as BushAction, EnKusa};
use crate::player::Action as PA;
use crate::script::{exit_to, stick_towards};

/// The steps, in order. Each is reported by `Playthrough::take_done` after the `next` call
/// that finished it, so a caller can check the play state it finished on before running the
/// next frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Link's house (`ENTR_LINK_HOME_0`), faded in and standing.
    House,
    /// Out through the door onto the porch (`ENTR_SPOT04_3`), settled.
    OutDoor,
    /// Down the ladder to the ground.
    Ladder,
    /// The sign by the house read to the end and closed.
    Sign,
    /// A Kokiri child's text read to the end and closed.
    Kokiri,
    /// A bush cut and its drop collected.
    Bush,
    /// In front of the Deku Tree's mouth, standing.
    Tree,
    /// Into the mouth: its floor's exit started the transition.
    Mouth,
    /// The Deku Tree's scene, faded in and settled.
    DekuTree,
}

impl Step {
    pub fn name(self) -> &'static str {
        match self {
            Step::House => "house",
            Step::OutDoor => "out_door",
            Step::Ladder => "ladder",
            Step::Sign => "sign",
            Step::Kokiri => "kokiri",
            Step::Bush => "bush",
            Step::Tree => "tree",
            Step::Mouth => "mouth",
            Step::DekuTree => "deku_tree",
        }
    }
}

/// What the run does, one task at a time.
#[derive(Debug, Clone)]
enum Task {
    /// Idle until the transition is over and Link stands, then `SETTLE_FRAMES` more.
    Settle(Option<Step>),
    /// Steer into the floor of the exit to the entrance, until the transition starts; then
    /// (unless the step is the exit itself) wait for the scene change and settle.
    Exit(&'static str, Step),
    /// Steer through the points, each passed within `WAYPOINT_RADIUS`.
    Walk(Vec<Vec3>),
    /// From the porch onto the ladder's top, then down it.
    LadderDown,
    /// Talk to the actor: walk to the point, steer at the actor until it offers, press A, and
    /// press A through its text until the box closes.
    Talk(Who, Vec3, Step),
    /// Cut the first of these bushes (home positions) that drops something, and collect it.
    CutBushes(Vec<Vec3>),
}

/// An actor the run talks to.
#[derive(Debug, Clone, Copy)]
enum Who {
    /// The `En_Kanban` whose home is here.
    Sign(Vec3),
    /// The `En_Ko` of this child type (`params & 0xFF`).
    Kokiri(i16),
}

const SETTLE_FRAMES: usize = 10;
const WAYPOINT_RADIUS: f32 = 30.0;
/// The stick's tilt for a run (of the pad's ±80), and for the slow approaches: a walk.
/// (`func_80836FAC` takes 20 off the dead-zoned magnitude, so much under 30 only turns Link.)
const RUN: f32 = 60.0;
const SLOW: f32 = 40.0;

pub struct Playthrough {
    tasks: Vec<Task>,
    task: usize,
    /// The current task's progress, and a frame counter it uses.
    sub: usize,
    wait: usize,
    frame: usize,
    prev: PadState,
    done: Option<Step>,
    /// Each step and the frame (1-based) it was done on.
    pub steps: Vec<(Step, usize)>,
    /// The text ids opened on the way, in order.
    pub texts: Vec<u16>,
    /// The bushes cut (their home positions), and the drop collected (`En_Item00` params).
    pub bushes_cut: Vec<Vec3>,
    pub drop: Option<i16>,
    /// The scene changes seen when the current exit started.
    scene_changes: u32,
    /// The drops there were before a slash, and the one it dropped.
    items: Vec<ActorHandle>,
    dropped: Option<ActorHandle>,
    /// Why the run stopped short, if it did.
    pub failure: Option<String>,
}

impl Default for Playthrough {
    fn default() -> Self {
        Self::new()
    }
}

impl Playthrough {
    /// The entrance the run starts at, and the save preset it needs.
    pub const ENTRANCE: &'static str = "ENTR_LINK_HOME_0";
    pub const PRESET: &'static str = "deku-tree-open";
    /// A cap on the run's length.
    pub const MAX_FRAMES: usize = 6000;

    pub fn new() -> Playthrough {
        let tasks = vec![
            Task::Settle(Some(Step::House)),
            Task::Exit("ENTR_SPOT04_3", Step::OutDoor),
            Task::LadderDown,
            // The sign by Link's house (params 0x031F) faces -z: read from in front of it.
            Task::Talk(Who::Sign(Vec3::new(49.0, -80.0, 967.0)), Vec3::new(49.0, -80.0, 900.0), Step::Sign),
            // Up the slope out of the house's hollow into the village.
            Task::Walk(vec![Vec3::new(0.0, -80.0, 800.0), Vec3::new(0.0, 0.0, 480.0), Vec3::new(400.0, 0.0, 470.0)]),
            // Child 4 at (669, 0, 521) faces about -z.
            Task::Talk(Who::Kokiri(4), Vec3::new(660.0, 0.0, 465.0), Step::Kokiri),
            Task::CutBushes(vec![Vec3::new(594.0, 0.0, 542.0), Vec3::new(572.0, 0.0, 603.0), Vec3::new(678.0, 0.0, 596.0), Vec3::new(385.0, 0.0, 643.0)]),
            // Through the stream's ford and along its east side, past Mido, along the path to
            // the En_Holl and down into the meadow, to the open jaw.
            Task::Walk(vec![
                Vec3::new(900.0, 0.0, 430.0),
                Vec3::new(1250.0, 0.0, 410.0),
                Vec3::new(1375.0, 0.0, 380.0),
                Vec3::new(1400.0, 0.0, 250.0),
                Vec3::new(1450.0, 0.0, 150.0),
                Vec3::new(1900.0, 0.0, 140.0),
                Vec3::new(2175.0, 0.0, 100.0),
                Vec3::new(2175.0, 0.0, -148.0),
                Vec3::new(2175.0, 0.0, -480.0),
                Vec3::new(2600.0, 0.0, -520.0),
                Vec3::new(3000.0, 0.0, -600.0),
                Vec3::new(3350.0, 0.0, -950.0),
                Vec3::new(3650.0, 0.0, -1180.0),
                // Over the gap in the tree's floor that only the open jaw covers.
                Vec3::new(3950.0, 0.0, -1200.0),
            ]),
            Task::Settle(Some(Step::Tree)),
            Task::Exit("ENTR_YDAN_0", Step::Mouth),
            Task::Settle(Some(Step::DekuTree)),
        ];
        Playthrough {
            tasks,
            task: 0,
            sub: 0,
            wait: 0,
            frame: 0,
            prev: PadState::default(),
            done: None,
            steps: Vec::new(),
            texts: Vec::new(),
            bushes_cut: Vec::new(),
            drop: None,
            scene_changes: 0,
            items: Vec::new(),
            dropped: None,
            failure: None,
        }
    }

    /// The step done on the last frame, once.
    pub fn take_done(&mut self) -> Option<Step> {
        self.done.take()
    }

    /// Whether every task is done.
    pub fn finished(&self) -> bool {
        self.task >= self.tasks.len()
    }

    /// Which task the run is in, for messages.
    pub fn at(&self) -> String {
        match self.tasks.get(self.task) {
            Some(t) => format!("task {} ({t:?}), sub {}", self.task, self.sub),
            None => "done".into(),
        }
    }

    fn finish(&mut self, step: Option<Step>) {
        if let Some(s) = step {
            self.steps.push((s, self.frame));
            self.done = Some(s);
        }
        self.task += 1;
        self.sub = 0;
        self.wait = 0;
    }

    /// The pad for the next frame, or `None` when the run is over: done, or stuck (then
    /// `failure` says why).
    pub fn next(&mut self, w: &PlayState) -> Option<PadState> {
        if self.failure.is_some() || self.finished() {
            return None;
        }
        if self.frame >= Self::MAX_FRAMES {
            self.failure = Some(format!("out of frames in {}", self.at()));
            return None;
        }
        if w.message_state() != TEXT_STATE_NONE && self.texts.last() != Some(&w.msg_ctx.text_id) {
            self.texts.push(w.msg_ctx.text_id);
        }
        let pad = loop {
            let before = self.task;
            let p = self.step(w);
            if self.failure.is_some() {
                return None;
            }
            match p {
                Some(p) => break p,
                None if self.finished() => return None,
                // The task finished without needing a frame: go on to the next one.
                None if self.task != before => continue,
                None => break PadState::default(),
            }
        };
        self.frame += 1;
        self.prev = pad;
        Some(pad)
    }

    /// No transition, and Link standing on the ground.
    fn settled(w: &PlayState) -> bool {
        let p = w.player();
        w.transition.trigger == TRANS_TRIGGER_OFF && w.transition.mode == TRANS_MODE_OFF && p.action == PA::StandingStill && p.grounded()
    }

    /// Waits for `settled`, then `SETTLE_FRAMES` more, then finishes with `step`.
    fn settle(&mut self, w: &PlayState, step: Option<Step>) -> Option<PadState> {
        if Self::settled(w) {
            self.wait += 1;
            if self.wait > SETTLE_FRAMES {
                self.finish(step);
                return None;
            }
        } else {
            self.wait = 0;
        }
        Some(PadState::default())
    }

    /// A button pressed this frame, released on the next (so each press is an edge).
    fn press(&self, button: u16) -> PadState {
        if self.prev.button & button != 0 { PadState::default() } else { PadState { button, ..Default::default() } }
    }

    fn xz_dist(a: Vec3, b: Vec3) -> f32 {
        Vec3::new(a.x - b.x, 0.0, a.z - b.z).length()
    }

    /// The current task's input for the next frame; `None` when it finished this frame.
    fn step(&mut self, w: &PlayState) -> Option<PadState> {
        let idle = PadState::default();
        let task = self.tasks[self.task].clone();
        let link = w.player().actor.world_pos;
        match task {
            Task::Settle(step) => self.settle(w, step),
            Task::Exit(name, step) => match self.sub {
                0 => {
                    if w.transition.trigger != TRANS_TRIGGER_OFF {
                        self.scene_changes = w.scene_changes;
                        if step == Step::Mouth {
                            // The step is the exit itself; the next task waits for the scene.
                            self.finish(Some(step));
                            return None;
                        }
                        self.sub = 1;
                        return Some(idle);
                    }
                    let Some((_, c)) = exit_to(w, name) else {
                        self.failure = Some(format!("no exit to {name} in scene {}", w.scene_id));
                        return None;
                    };
                    Some(stick_towards(w, c, RUN))
                }
                // Play_Init in the next scene, then settle.
                1 => {
                    if w.scene_changes > self.scene_changes {
                        self.sub = 2;
                    }
                    Some(idle)
                }
                _ => self.settle(w, Some(step)),
            },
            Task::Walk(points) => {
                while self.sub < points.len() && Self::xz_dist(link, points[self.sub]) < WAYPOINT_RADIUS {
                    self.sub += 1;
                }
                if self.sub >= points.len() {
                    self.finish(None);
                    return None;
                }
                Some(stick_towards(w, points[self.sub], RUN))
            }
            Task::LadderDown => {
                let a = w.player().action;
                match self.sub {
                    // Off the porch towards the ladder (as climb.rs's
                    // child_link_climbs_down_from_his_porch) until Link takes hold of it.
                    0 => {
                        if matches!(a, PA::Climb | PA::ItemPutAway) {
                            self.sub = 1;
                            return Some(idle);
                        }
                        Some(stick_towards(w, Vec3::new(-29.0, 100.0, 990.0), RUN))
                    }
                    // Down the ladder (the stick pulled back), and off it at the bottom.
                    1 => {
                        if a == PA::ClimbEnd {
                            self.sub = 2;
                            return Some(idle);
                        }
                        Some(if a == PA::Climb { PadState { stick_y: -60, ..Default::default() } } else { idle })
                    }
                    _ => {
                        if Self::settled(w) {
                            self.finish(Some(Step::Ladder));
                            return None;
                        }
                        Some(idle)
                    }
                }
            }
            Task::Talk(who, from, step) => {
                let Some(h) = Self::find(w, who) else {
                    self.failure = Some(format!("no {who:?} to talk to"));
                    return None;
                };
                let at = w.actors.actor(h).unwrap().world_pos;
                let p = w.player();
                match self.sub {
                    0 => {
                        if Self::xz_dist(link, from) < WAYPOINT_RADIUS {
                            self.sub = 1;
                        }
                        Some(stick_towards(w, from, RUN))
                    }
                    // At it until it offers (Player's targetActor), then stop.
                    1 => {
                        if p.target_actor == Some(h) {
                            self.sub = 2;
                            return Some(idle);
                        }
                        Some(stick_towards(w, at, SLOW))
                    }
                    // A, while it still offers.
                    2 => {
                        if p.action == PA::Talk || p.action == PA::ItemPutAway || w.message_state() != TEXT_STATE_NONE {
                            self.sub = 3;
                            return Some(idle);
                        }
                        if p.target_actor != Some(h) {
                            self.sub = 1;
                            return Some(idle);
                        }
                        Some(self.press(BTN_A))
                    }
                    // A whenever a box waits (the end of one with a next text, a choice's
                    // first answer), until it's closed and Link stands.
                    _ => {
                        if w.message_state() == TEXT_STATE_NONE && !matches!(p.action, PA::Talk | PA::ItemPutAway) {
                            self.finish(Some(step));
                            return None;
                        }
                        let waiting = matches!(w.message_state(), TEXT_STATE_AWAITING_NEXT | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_CHOICE);
                        Some(if waiting { self.press(BTN_A) } else { idle })
                    }
                }
            }
            Task::CutBushes(bushes) => self.cut_bushes(w, &bushes),
        }
    }

    fn find(w: &PlayState, who: Who) -> Option<ActorHandle> {
        w.actors.all().into_iter().find(|&h| match who {
            Who::Sign(home) => w.actors.downcast::<EnKanban>(h).is_some_and(|k| Self::xz_dist(k.actor.home_pos, home) < 1.0),
            Who::Kokiri(ty) => w.actors.downcast::<EnKo>(h).is_some_and(|k| k.actor.params & 0xFF == ty),
        })
    }

    fn drops(w: &PlayState) -> Vec<ActorHandle> {
        w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnItem00>(h).is_some()).collect()
    }

    /// `sub` is 4 × the bush's index plus its phase: the approach, the slash, the drop, the
    /// pickup. A drop can land at Link's feet and be picked up before the slash is over, so
    /// it's looked for from the slash on.
    fn cut_bushes(&mut self, w: &PlayState, bushes: &[Vec3]) -> Option<PadState> {
        let idle = PadState::default();
        let (i, phase) = (self.sub / 4, self.sub % 4);
        let Some(&home) = bushes.get(i) else {
            self.failure = Some(format!("none of the {} bushes dropped anything", bushes.len()));
            return None;
        };
        let link = w.player().actor.world_pos;
        let bush = w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnKusa>(h).is_some_and(|k| Self::xz_dist(k.actor.home_pos, home) < 1.0));
        match phase {
            // To 45 from the bush on Link's side of it, then slowly at it: B once 32 away.
            0 => {
                if bush.is_none() {
                    self.failure = Some(format!("no bush at {home}"));
                    return None;
                }
                if Self::xz_dist(link, home) < 32.0 {
                    self.sub += 1;
                    self.wait = 0;
                    self.items = Self::drops(w);
                    self.dropped = None;
                    return Some(self.press(BTN_B));
                }
                let d = Vec3::new(link.x - home.x, 0.0, link.z - home.z).normalize_or_zero();
                if self.wait > 0 || Self::xz_dist(link, home + d * 45.0) < 10.0 {
                    self.wait += 1;
                    return Some(stick_towards(w, home, SLOW));
                }
                Some(stick_towards(w, home + d * 45.0, RUN))
            }
            // The slash plays out; a miss tries again. (A cut bush of type 0, Kokiri's, is
            // gone: EnKusa_Main kills it.)
            1 => {
                if self.dropped.is_none() {
                    self.dropped = Self::drops(w).into_iter().find(|h| !self.items.contains(h));
                    if let Some(h) = self.dropped {
                        self.drop = w.actors.downcast::<EnItem00>(h).map(|e| e.actor.params);
                    }
                }
                self.wait += 1;
                if self.wait > 30 {
                    self.wait = 0;
                    if bush.is_none_or(|h| w.actors.downcast::<EnKusa>(h).is_some_and(|k| k.action == BushAction::CutWaitRegrow)) {
                        self.bushes_cut.push(home);
                        self.sub += 1;
                    } else {
                        self.sub -= 1;
                    }
                }
                Some(idle)
            }
            // A drop? Wait for it to land (or be picked up). None: the next bush.
            2 => match self.dropped {
                None => {
                    self.sub += 2;
                    Some(idle)
                }
                Some(h) => {
                    if w.actors.downcast::<EnItem00>(h).is_none_or(|e| matches!(e.action, ItemAction::Rest | ItemAction::Collected)) {
                        self.sub += 1;
                    }
                    Some(idle)
                }
            },
            // Onto it until it's collected (or already gone: collected).
            _ => match self.dropped.and_then(|h| w.actors.downcast::<EnItem00>(h)) {
                Some(e) if e.action != ItemAction::Collected => Some(stick_towards(w, e.actor.world_pos, SLOW)),
                _ => {
                    self.finish(Some(Step::Bush));
                    None
                }
            },
        }
    }
}
