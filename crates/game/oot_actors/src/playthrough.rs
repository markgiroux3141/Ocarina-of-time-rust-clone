//! The Kokiri Forest playthroughs: scripted runs from Link's bed, steering each frame as a
//! player would (`Route`). The tests run them with checks at each step, and the sandbox's
//! scripts write their traces.
//!
//! **The Deku Tree** (GAME-02 milestone 4, `Route::DekuTree`, the `playthrough` test and
//! script) goes into the Deku Tree's scene. It needs the Deku Tree's mouth open, so it runs on
//! a save with the `deku-tree-open` preset (`oot_game::save::SAVE_PRESETS`): the talk that
//! opens it in the game is a cutscene, which isn't ported yet. The route follows Kokiri
//! Forest's collision: out of Link's house, down the ladder, to the sign, to the Kokiri child
//! by the bushes (child 4), the bushes, then east through the stream, past Mido, along the
//! path through the `En_Holl` into room 1, across the meadow and into the mouth. Mido and
//! Saria are placeholders, so neither stands in the way.
//!
//! **The Kokiri Sword** (GAME-03 milestone 2, `Route::SwordChest`, the `sword_chest` test and
//! the `sword-chest` script) runs on a new save: out of the house and down the ladder, west
//! through the village and up the ramp onto the plateau, into the crawlspace by its sign, out
//! into the training area (room 2), round the boulder's corridors while it rolls elsewhere
//! (waiting where its path doesn't reach), up to the chest, and the chest opened and its text
//! read.
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
use oot_game::message::{MSGMODE_TEXT_AWAIT_INPUT, TEXT_STATE_AWAITING_NEXT, TEXT_STATE_CHOICE, TEXT_STATE_DONE, TEXT_STATE_DONE_HAS_NEXT, TEXT_STATE_NONE};
use oot_game::play::PlayState;
use oot_game::transition::{TRANS_MODE_OFF, TRANS_TRIGGER_OFF};

use crate::PlayExt;
use crate::en_box::EnBox;
use crate::en_goroiwa::EnGoroiwa;
use crate::en_item00::{Action as ItemAction, EnItem00};
use crate::en_kanban::EnKanban;
use crate::en_ko::EnKo;
use crate::en_kusa::{Action as BushAction, EnKusa};
use crate::player::{Action as PA, STATE2_16};
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
    /// Into the crawlspace: the crawl began (`func_8084C760`).
    Crawlspace,
    /// Out of the crawlspace into the training area (room 2), standing.
    TrainingArea,
    /// Past the boulder's corridors, at the foot of the slope up to the chest.
    Boulder,
    /// The chest opened, its item's text read and closed, Link standing.
    Chest,
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
            Step::Crawlspace => "crawlspace",
            Step::TrainingArea => "training_area",
            Step::Boulder => "boulder",
            Step::Chest => "chest",
        }
    }
}

/// The scripted runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    /// Link's bed into the Deku Tree, on the `deku-tree-open` preset (GAME-02 milestone 4).
    DekuTree,
    /// Link's bed to the Kokiri Sword's chest, opened, on a new save (GAME-03 milestone 2).
    SwordChest,
}

impl Route {
    /// The entrance a route starts at.
    pub fn entrance(self) -> &'static str {
        "ENTR_LINK_HOME_0"
    }

    /// The save preset it needs, if any.
    pub fn preset(self) -> Option<&'static str> {
        match self {
            Route::DekuTree => Some("deku-tree-open"),
            Route::SwordChest => None,
        }
    }

    /// The sandbox script that runs it.
    pub fn script(self) -> &'static str {
        match self {
            Route::DekuTree => "playthrough",
            Route::SwordChest => "sword-chest",
        }
    }

    /// The route a sandbox script names.
    pub fn from_script(name: &str) -> Option<Route> {
        [Route::DekuTree, Route::SwordChest].into_iter().find(|r| r.script() == name)
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
    /// Steer through the points at full tilt.
    Hurry(Vec<Vec3>),
    /// Into a crawlspace and through it: walk to the first point, lined up with the mouth;
    /// towards the mouth (the second point) until A says "Enter" (`PLAYER_STATE2_16`); A; the
    /// stick forward until Link is out and standing.
    Crawl(Vec3, Vec3),
    /// Idle until the boulder rolls on its path from point `.0` to point `.1` and has covered
    /// at least `.2` of that segment.
    WaitBoulder(i16, i16, f32),
    /// Open the chest whose home is here: walk in front of it, at it until it offers its
    /// item, A, then A through the item's text until the box closes and Link stands.
    OpenChest(Vec3),
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
/// Full tilt (the stick's relative position clamps at 60 past the dead zone).
const FULL: f32 = 80.0;

pub struct Playthrough {
    pub route: Route,
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
    /// The frames spent waiting for the boulder, each wait.
    pub boulder_waits: Vec<usize>,
}

impl Default for Playthrough {
    fn default() -> Self {
        Self::new()
    }
}

impl Playthrough {
    /// The entrance the Deku Tree run starts at, and the save preset it needs.
    pub const ENTRANCE: &'static str = "ENTR_LINK_HOME_0";
    pub const PRESET: &'static str = "deku-tree-open";
    /// A cap on a run's length.
    pub const MAX_FRAMES: usize = 6000;

    /// The Deku Tree run.
    pub fn new() -> Playthrough {
        Self::for_route(Route::DekuTree)
    }

    pub fn for_route(route: Route) -> Playthrough {
        let tasks = match route {
            Route::DekuTree => Self::deku_tree(),
            Route::SwordChest => Self::sword_chest(),
        };
        Playthrough {
            route,
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
            boulder_waits: Vec::new(),
        }
    }

    /// The Kokiri Sword run's tasks.
    fn sword_chest() -> Vec<Task> {
        vec![
            Task::Settle(Some(Step::House)),
            Task::Exit("ENTR_SPOT04_3", Step::OutDoor),
            Task::LadderDown,
            // Out of the house's hollow, west through the village to the foot of the ramp, and
            // up it onto the plateau (y 120).
            Task::Walk(vec![Vec3::new(0.0, -80.0, 800.0), Vec3::new(0.0, 0.0, 480.0), Vec3::new(-300.0, 0.0, 300.0), Vec3::new(-450.0, 0.0, -150.0), Vec3::new(-650.0, 0.0, -170.0), Vec3::new(-650.0, 120.0, 250.0)]),
            // North across the plateau to the crawlspace by its sign (0x0337): its mouth is the
            // wall at z 1059, x -801..-769 (WALL_FLAG_4).
            Task::Walk(vec![Vec3::new(-650.0, 120.0, 500.0), Vec3::new(-785.0, 120.0, 850.0)]),
            Task::Crawl(Vec3::new(-785.0, 120.0, 1000.0), Vec3::new(-785.0, 120.0, 1059.0)),
            // The bottom corridor west of the boulder's path (its corner at (-575, 1538)).
            Task::Walk(vec![Vec3::new(-760.0, 120.0, 1470.0), Vec3::new(-700.0, 120.0, 1515.0)]),
            // Once it has turned north up the middle corridor (path 2's points 2 to 3), after it
            // to the corridors' top-left corner and into the alcove west of it.
            Task::WaitBoulder(2, 3, 0.35),
            Task::Hurry(vec![Vec3::new(-555.0, 120.0, 1545.0), Vec3::new(-555.0, 120.0, 1850.0), Vec3::new(-690.0, 120.0, 1880.0)]),
            // Once it has passed along the top corridor (points 3 to 4), after it east to the
            // top-right corner, and north out of its way.
            Task::WaitBoulder(3, 4, 0.35),
            Task::Hurry(vec![Vec3::new(-560.0, 120.0, 1885.0), Vec3::new(-250.0, 120.0, 1895.0), Vec3::new(-250.0, 120.0, 1960.0)]),
            Task::Settle(Some(Step::Boulder)),
            // Up the slope to the chest's dais.
            Task::Walk(vec![Vec3::new(-250.0, 140.0, 2080.0), Vec3::new(-232.0, 160.0, 2170.0)]),
            // room 2's En_Box (params 0x04E0): the Kokiri Sword.
            Task::OpenChest(Vec3::new(-232.0, 178.0, 2245.0)),
        ]
    }

    /// The Deku Tree run's tasks.
    fn deku_tree() -> Vec<Task> {
        vec![
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
        ]
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
            Task::Walk(points) => self.walk(w, &points, RUN),
            Task::Hurry(points) => self.walk(w, &points, FULL),
            Task::Crawl(approach, mouth) => self.crawl(w, approach, mouth),
            Task::WaitBoulder(from, to, frac) => {
                let Some(b) = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<EnGoroiwa>(h)) else {
                    self.failure = Some("no boulder to wait for".into());
                    return None;
                };
                let path = &w.setup_path_list()[(b.actor.params & 0xFF) as usize];
                let (p0, p1) = (path.point(from as usize), path.point(to as usize));
                let seg = p1 - p0;
                let progress = (b.actor.world_pos - p0).dot(seg) / seg.length_squared();
                if b.current_waypoint == from && b.next_waypoint == to && progress >= frac {
                    self.boulder_waits.push(self.wait);
                    self.finish(None);
                    return None;
                }
                self.wait += 1;
                Some(idle)
            }
            Task::OpenChest(home) => self.open_chest(w, home),
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
                    // A whenever a box waits, until it's closed and Link stands.
                    _ => {
                        if w.message_state() == TEXT_STATE_NONE && !matches!(p.action, PA::Talk | PA::ItemPutAway) {
                            self.finish(Some(step));
                            return None;
                        }
                        Some(if Self::text_waits(w) { self.press(BTN_A) } else { idle })
                    }
                }
            }
            Task::CutBushes(bushes) => self.cut_bushes(w, &bushes),
        }
    }

    /// Whether the message box waits for A: the end of a box with a next text, a choice's first
    /// answer, the end, or a box break (`MSGMODE_TEXT_AWAIT_INPUT`, which `Message_GetState`
    /// reports as `TEXT_STATE_DONE_FADING`, its fallback).
    fn text_waits(w: &PlayState) -> bool {
        matches!(w.message_state(), TEXT_STATE_AWAITING_NEXT | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_CHOICE) || w.msg_ctx.msg_mode == MSGMODE_TEXT_AWAIT_INPUT
    }

    /// Steers through `points`, each passed within `WAYPOINT_RADIUS`, at stick magnitude `mag`.
    fn walk(&mut self, w: &PlayState, points: &[Vec3], mag: f32) -> Option<PadState> {
        let link = w.player().actor.world_pos;
        while self.sub < points.len() && Self::xz_dist(link, points[self.sub]) < WAYPOINT_RADIUS {
            self.sub += 1;
        }
        if self.sub >= points.len() {
            self.finish(None);
            return None;
        }
        Some(stick_towards(w, points[self.sub], mag))
    }

    /// `Task::Crawl`'s phases in `sub`: 0 to the approach, 1 at the mouth until "Enter", 2 A
    /// (with the stick still at the mouth, so Link stays in `func_80842180`, whose interrupts
    /// include the wall's, `func_8083F7BC`), 3 crawling, 4 out and settling.
    fn crawl(&mut self, w: &PlayState, approach: Vec3, mouth: Vec3) -> Option<PadState> {
        let idle = PadState::default();
        let p = w.player();
        let link = p.actor.world_pos;
        match self.sub {
            0 => {
                if Self::xz_dist(link, approach) < 10.0 {
                    self.sub = 1;
                }
                Some(stick_towards(w, approach, SLOW))
            }
            1 => {
                if p.state2 & STATE2_16 != 0 {
                    self.sub = 2;
                }
                Some(stick_towards(w, mouth, SLOW))
            }
            2 => {
                if matches!(p.action, PA::Crawl | PA::ItemPutAway) {
                    self.sub = 3;
                    return Some(idle);
                }
                if p.state2 & STATE2_16 == 0 {
                    self.sub = 1;
                    return Some(stick_towards(w, mouth, SLOW));
                }
                let mut pad = stick_towards(w, mouth, SLOW);
                pad.button = self.press(BTN_A).button;
                Some(pad)
            }
            // func_8084C760 reads the stick's tilt straight (rel.stick_y), not the camera's way.
            3 => {
                if p.action == PA::Crawl && !self.steps.iter().any(|s| s.0 == Step::Crawlspace) {
                    self.steps.push((Step::Crawlspace, self.frame));
                    self.done = Some(Step::Crawlspace);
                }
                if p.action == PA::CrawlExit {
                    self.sub = 4;
                    return Some(idle);
                }
                Some(PadState { stick_y: RUN as i8, ..Default::default() })
            }
            _ => self.settle(w, Some(Step::TrainingArea)),
        }
    }

    /// `Task::OpenChest`'s phases: 0 in front (35 before it, on its axis: its collision holds
    /// Link at 34), running (a slow walk stalls at the foot of the Kokiri Sword chest's
    /// mound), 1 at it until it offers (a chest's negative get-item id), 2 A, 3 the opening and
    /// the text.
    fn open_chest(&mut self, w: &PlayState, home: Vec3) -> Option<PadState> {
        let idle = PadState::default();
        let Some((h, chest)) = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<EnBox>(h).filter(|b| b.actor.home_pos.distance(home) < 1.0).map(|b| (h, b))) else {
            self.failure = Some(format!("no chest at {home}"));
            return None;
        };
        let yaw = chest.actor.shape_rot.y;
        let front = home - Vec3::new(eng_math::sin_s(yaw), 0.0, eng_math::cos_s(yaw)) * 35.0;
        let p = w.player();
        let link = p.actor.world_pos;
        match self.sub {
            0 => {
                if Self::xz_dist(link, front) < 10.0 {
                    self.sub = 1;
                }
                Some(stick_towards(w, front, RUN))
            }
            1 => {
                if p.interact_range_actor == Some(h) && p.get_item_id < 0 {
                    self.sub = 2;
                    return Some(idle);
                }
                Some(stick_towards(w, home, SLOW))
            }
            2 => {
                if p.action == PA::GetItem {
                    self.sub = 3;
                    return Some(idle);
                }
                if p.interact_range_actor != Some(h) {
                    self.sub = 1;
                    return Some(idle);
                }
                Some(self.press(BTN_A))
            }
            _ => {
                if w.message_state() == TEXT_STATE_NONE && p.action == PA::StandingStill && self.wait > 0 {
                    self.finish(Some(Step::Chest));
                    return None;
                }
                if w.message_state() != TEXT_STATE_NONE {
                    self.wait += 1;
                }
                Some(if Self::text_waits(w) { self.press(BTN_A) } else { idle })
            }
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
