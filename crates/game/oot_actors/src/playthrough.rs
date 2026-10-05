//! The Kokiri Forest playthroughs: scripted runs from Link's bed, steering each frame as a
//! player would (`Route`). The tests run them with checks at each step, and the sandbox's
//! scripts write their traces.
//!
//! **The Deku Tree** (GAME-02 milestone 4, `Route::DekuTree`, the `playthrough` test and
//! script) goes into the Deku Tree's scene. It needs the Deku Tree's mouth open, so it runs on
//! a save with the `deku-tree-open` preset (`oot_game::save::SAVE_PRESETS`), which skips the
//! tree's talk (`Route::NewSaveDekuTree` plays it). The route follows Kokiri
//! Forest's collision: out of Link's house, down the ladder, to the sign, to the Kokiri child
//! by the bushes (child 4), the bushes, then east through the stream, round Mido where he
//! stands aside (the preset sets `EVENTCHKINF_04`), along the path through the `En_Holl` into
//! room 1, across the meadow and into the mouth. Saria is a placeholder.
//!
//! **The Kokiri Sword** (GAME-03 milestone 2, `Route::SwordChest`, the `sword_chest` test and
//! the `sword-chest` script) runs on a new save: out of the house and down the ladder, west
//! through the village and up the ramp onto the plateau, into the crawlspace by its sign, out
//! into the training area (room 2), round the boulder's corridors while it rolls elsewhere
//! (waiting where its path doesn't reach), up to the chest, and the chest opened and its text
//! read.
//!
//! **Mido and the shop** (GAME-03 milestone 3, `Route::MidoShop`, the `mido_shop` test and the
//! `mido-shop` script) runs on a new save: the Kokiri Sword's run, picking up room 2's two blue
//! rupees on the way (the bottom corridor's west end, and the alcove where it waits for the
//! boulder); back past the boulder and through the crawlspace; the hidden switch by the
//! plateau's sign slashed and its rupee; down the ramp, two green rupees, Mido's house and its
//! four chests, two more green rupees and the free multitag's tag points; into the shop, its own
//! rupee, and the Deku Shield bought; Start (the pause menu's stand-in) to wear the sword and the
//! shield; out, east over the ford to Mido, his talk, and past him once he has stepped aside.
//! Every rupee on the way is one the C places (no `Rand` drops): 42, for the shield's 40. The
//! forced text by the shop (`En_Wonder_Talk2`, 0x218) holds Link (Player's cutscene mode 8)
//! until the walk reads it.
//!
//! **The new save into the Deku Tree** (GAME-03 milestone 4, `Route::NewSaveDekuTree`, the
//! `new_save_deku_tree` test and the `new-save-deku-tree` script) is the Mido and shop run, then
//! east along the path through the `En_Holl` into room 1 and into the meadow. There the Deku
//! Tree's first talk (`Bg_Treemouth`'s `gDekuTreeMeetingCs`) starts by itself and walks Link in; the run
//! reads it and answers yes, which plays `gDekuTreeMouthOpeningCs` and opens the mouth (`EVENTCHKINF_05`).
//! Then over the open jaw into the mouth, to `ENTR_DEKU_TREE_0`, whose intro (`gDekuTreeIntroCs`)
//! plays the first time in. No preset.
//!
//! **A new file into the Deku Tree** (GAME-03 milestone 5, `Route::NewFileDekuTree`, the
//! `new_file_deku_tree` test and the `new-file-deku-tree` script) starts as the file select
//! starts a new file (`SaveContext::file_select_new`): the opening plays, A read at each box that
//! waits for it as a player would (Link's house's layer 5, the nightmare on Hyrule Field's layer
//! 4, Kokiri Forest's layer 7 where the Deku Tree sends Navi, and her waking Link on the house's
//! layer 4); then the new save's run from where the wake-up leaves Link, with C-Up to Navi on
//! the plateau once she has her text (`naviTimer` 600 on: 0x140, "the Great Deku Tree wants to
//! talk to you").
//!
//! **A Deku Baba** (GAME-05 milestone 2, `Route::DekuBaba`, the `deku_baba` test and the
//! `deku-baba` script) runs inside the Deku Tree on the `deku-tree-inside` preset (the intro
//! seen), from a debug start on the top floor of room 0, 85 from the Deku Baba at (-195, 800,
//! -195) and facing it (`DEKU_BABA_START`: the sandbox's `--at`, the test's `place_player`). Link
//! stands until it bites him (half a heart: the exit's hit), then waits out its next bite, which
//! misses from where the stagger left him; slashes it while it's stuck to the ground (weakened, it
//! lies stretched out), runs in and cuts its stem, and ends once its head is a Deku Stick.
//!
//! **The shield and the first fights** (GAME-05 milestone 3a, `Route::Combat`, the `combat` test
//! and script) run inside the Deku Tree on the `deku-tree-inside` preset, from a debug start on
//! the ground floor of room 0 (`COMBAT_START`), by the withered Deku Baba at (-88, 0, -363) and
//! the Keese perched on the wall 48 from it at (-54, 262, -397). (The ground floor, not the top
//! one: a withered Deku Baba's head flies back from where it faced as it dies, and on the top
//! floor it falls down the middle.) Link walks up to the Deku Baba until it springs, locks on
//! (Z: the battle camera, `Camera_Battle1`), slashes it while it's upright
//! (`Step::KarebabaKilled`), and picks up the Deku Stick it leaves (`Step::StickTaken`); the
//! Keese dives at him meanwhile (within 120 of its perch). The Deku Baba first: while it's up, Z
//! would lock on to it rather than the Keese (the nearer, in front), and nowhere within reach of
//! the Keese has it outside the 60 degrees `Attention_WeightedDistToPlayerSq` looks in. Then Link
//! locks on to the Keese and holds the shield up (R on the lock-on: `func_80834758`) until it
//! blocks a dive (`Step::Blocked`); the Keese hovers after (`EnFirefly_Stay`) and he slashes it
//! (B) until it dies; it shrinks away and drops from table 14 (`Step::KeeseKilled`, the drop in
//! `drop` if any, picked up). Between the two, Link walks back to the start: the Deku Baba grows
//! back where he picked up its stick, and bites within reach of its home.
//!
//! **The drop depends on `Rand`.** A cut Kokiri bush draws from drop table 2, which gives
//! something for 5 of its 16 entries at full health (`func_8001F404` turns the hearts into
//! green rupees). The run cuts the four bushes by child 4 in turn until one drops (in the order
//! that gives a drop with `Object_Kankyo`'s draws in the forest and, since GAME-05 milestone 3a,
//! the effects' draws and the slash's recoil off the walls by the bushes). A change to
//! who calls `Rand` and when changes the draws; if all four then come up empty (about one
//! time in five), the run stops with "none of the 4 bushes dropped anything", and the list or
//! its order needs changing.

use eng_input::pad::{BTN_A, BTN_B, BTN_START, PadState};
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
use crate::en_md::{Action as MdAction, EnMd};
use crate::en_girla::{EnGirlA, SI_DEKU_SHIELD};
use crate::en_wonder_item::EnWonderItem;
use crate::player::{Action as PA, STATE2_16};
use crate::script::{exit_to, stick_towards};

/// The steps, in order. Each is reported by `Playthrough::take_done` after the `next` call
/// that finished it, so a caller can check the play state it finished on before running the
/// next frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Link's house (`ENTR_LINKS_HOUSE_0`), faded in and standing.
    House,
    /// Out through the door onto the porch (`ENTR_KOKIRI_FOREST_3`), settled.
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
    /// Into the crawlspace: the crawl began (`Player_Action_8084C760`).
    Crawlspace,
    /// Out of the crawlspace into the training area (room 2), standing.
    TrainingArea,
    /// Past the boulder's corridors, at the foot of the slope up to the chest.
    Boulder,
    /// The chest opened, its item's text read and closed, Link standing.
    Chest,
    /// The Kokiri Sword worn (Start: the pause menu's stand-in).
    SwordOn,
    /// Back through the crawlspace onto the plateau (room 0), standing.
    Plateau,
    /// The plateau sign's hidden switch (`En_Wonder_Item` mode 3) slashed and its rupee got.
    Switch,
    /// Into Mido's house (`ENTR_MIDOS_HOUSE_0`), settled.
    MidoHouse,
    /// Mido's house's four chests opened, and back out in Kokiri Forest (`ENTR_KOKIRI_FOREST_9`).
    MidoChests,
    /// Into the Kokiri shop (`ENTR_KOKIRI_SHOP_0`), settled.
    Shop,
    /// The Deku Shield bought: its text read, the shopping over, Link standing.
    Shield,
    /// The Kokiri Sword and the Deku Shield worn (Start: the pause menu's stand-in).
    Equipped,
    /// Out of the shop (`ENTR_KOKIRI_FOREST_4`), settled.
    ShopOut,
    /// Mido's text read to the end.
    Mido,
    /// Mido at his path's end (`EnMd_Watch`), out of the way.
    MidoAside,
    /// Past where Mido stood, on the path to the Deku Tree.
    PastMido,
    /// The Deku Tree's talk (`gDekuTreeMeetingCs`) answered yes and its script (`gDekuTreeMouthOpeningCs`) over:
    /// `EVENTCHKINF_05`, Link free.
    TreeTalk,
    /// The opening: Hyrule Field's cutscene layer 4 (the nightmare) entered.
    Nightmare,
    /// Kokiri Forest's cutscene layer 7 (the Deku Tree sends Navi) entered.
    NaviSent,
    /// Link's house's cutscene layer 4 over: Navi has woken Link, and he stands.
    WakeUp,
    /// C-Up to Navi, and her text read to the end.
    Navi,
    /// A Deku Baba's bite has hit Link.
    Bitten,
    /// Slashed while stuck after a missed bite, it lies stretched out (`EnDekubaba_Vulnerable`).
    BabaWeakened,
    /// Its stem cut, its head is a Deku Stick (`EnDekubaba_DekuStick`).
    BabaCut,
    /// A Keese's dive blocked by the shield (Player's `Player_Action_808435C4`).
    Blocked,
    /// The Keese slashed to death, gone (`EnFirefly_Disappear` over) and its drop, if any, picked up.
    KeeseKilled,
    /// The withered Deku Baba slashed while upright (`EnKarebaba_Dying`).
    KarebabaKilled,
    /// Its Deku Stick picked up (`GI_DEKU_STICKS_1`).
    StickTaken,
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
            Step::SwordOn => "sword_on",
            Step::Plateau => "plateau",
            Step::Switch => "switch",
            Step::MidoHouse => "mido_house",
            Step::MidoChests => "mido_chests",
            Step::Shop => "shop",
            Step::Shield => "shield",
            Step::Equipped => "equipped",
            Step::ShopOut => "shop_out",
            Step::Mido => "mido",
            Step::MidoAside => "mido_aside",
            Step::PastMido => "past_mido",
            Step::TreeTalk => "tree_talk",
            Step::Nightmare => "nightmare",
            Step::NaviSent => "navi_sent",
            Step::WakeUp => "wake_up",
            Step::Navi => "navi",
            Step::Bitten => "bitten",
            Step::BabaWeakened => "baba_weakened",
            Step::BabaCut => "baba_cut",
            Step::Blocked => "blocked",
            Step::KeeseKilled => "keese_killed",
            Step::KarebabaKilled => "karebaba_killed",
            Step::StickTaken => "stick_taken",
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
    /// Link's bed to the sword, 40 rupees, the Deku Shield from the shop, both worn, and past
    /// Mido, on a new save (GAME-03 milestone 3).
    MidoShop,
    /// The Mido and shop run, then into room 1, the Deku Tree's talk answered yes, and into
    /// his mouth (GAME-03 milestone 4): no preset.
    NewSaveDekuTree,
    /// The file select's new file: the opening, then the new save's run from where the wake-up
    /// leaves Link, with C-Up to Navi on the way (GAME-03 milestone 5).
    NewFileDekuTree,
    /// Inside the Deku Tree, a Deku Baba's bite, then its stem cut (GAME-05 milestone 2), from a
    /// debug start (`DEKU_BABA_START`).
    DekuBaba,
    /// Inside the Deku Tree, a Keese's dive blocked with the shield, the Keese and a withered
    /// Deku Baba killed with the battle camera on, their drops picked up (GAME-05 milestone 3a),
    /// from a debug start (`COMBAT_START`).
    Combat,
}

/// The `Combat` route's Keese: room 0's `En_Firefly` (params 3, perched) on the ground floor's
/// wall, 262 up.
pub const COMBAT_KEESE_HOME: Vec3 = Vec3::new(-54.0, 262.0, -397.0);
/// Its withered Deku Baba: room 0's `En_Karebaba` on the ground floor.
pub const COMBAT_KAREBABA_HOME: Vec3 = Vec3::new(-88.0, 0.0, -363.0);
/// Where the `Combat` route starts Link: on the ground floor, 150 across from the Keese towards
/// the room's middle, facing it (yaw 0x8000: -z).
pub const COMBAT_START: (Vec3, i16) = (Vec3::new(-54.0, 0.0, -247.0), -0x8000);

/// The Deku Baba the `DekuBaba` route fights: room 0's `En_Dekubaba` (params 0) on the top floor.
pub const DEKU_BABA_HOME: Vec3 = Vec3::new(-195.0, 800.0, -195.0);
/// Where the `DekuBaba` route starts Link: 85 from the Deku Baba, on the floor outwards of it,
/// facing it (yaw 0x2000).
pub const DEKU_BABA_START: (Vec3, i16) = (Vec3::new(-255.104, 800.0, -255.104), 0x2000);

impl Route {
    /// The entrance a route starts at.
    pub fn entrance(self) -> &'static str {
        match self {
            Route::DekuBaba | Route::Combat => "ENTR_DEKU_TREE_0",
            _ => "ENTR_LINKS_HOUSE_0",
        }
    }

    /// The save preset it needs, if any.
    pub fn preset(self) -> Option<&'static str> {
        match self {
            Route::DekuTree => Some("deku-tree-open"),
            Route::DekuBaba | Route::Combat => Some("deku-tree-inside"),
            Route::SwordChest | Route::MidoShop | Route::NewSaveDekuTree | Route::NewFileDekuTree => None,
        }
    }

    /// Where Link starts instead of the entrance's spawn (a debug start), if anywhere.
    pub fn start(self) -> Option<(Vec3, i16)> {
        match self {
            Route::DekuBaba => Some(DEKU_BABA_START),
            Route::Combat => Some(COMBAT_START),
            _ => None,
        }
    }

    /// Whether it starts on the file select's new file (`SaveContext::file_select_new`, the
    /// opening) instead of `SaveContext::new` at the entrance.
    pub fn new_file(self) -> bool {
        self == Route::NewFileDekuTree
    }

    /// The save it starts with, entering by `entrance_index` (its entrance's).
    pub fn save(self, entrance_index: u16) -> oot_game::save::SaveContext {
        use oot_game::save::SaveContext;
        if self.new_file() {
            return SaveContext::file_select_new();
        }
        let mut s = SaveContext::new(entrance_index, false, oot_game::env::clock_time(10, 0) as u16);
        if let Some(p) = self.preset() {
            s.apply_preset(p).expect("the route's preset");
        }
        s
    }

    /// The sandbox script that runs it.
    pub fn script(self) -> &'static str {
        match self {
            Route::DekuTree => "playthrough",
            Route::SwordChest => "sword-chest",
            Route::MidoShop => "mido-shop",
            Route::NewSaveDekuTree => "new-save-deku-tree",
            Route::NewFileDekuTree => "new-file-deku-tree",
            Route::DekuBaba => "deku-baba",
            Route::Combat => "combat",
        }
    }

    /// A cap on the route's length.
    pub fn max_frames(self) -> usize {
        match self {
            Route::MidoShop => 12000,
            Route::NewSaveDekuTree => 16000,
            Route::NewFileDekuTree => 24000,
            Route::Combat => 9000,
            _ => Playthrough::MAX_FRAMES,
        }
    }

    /// The route a sandbox script names.
    pub fn from_script(name: &str) -> Option<Route> {
        [Route::DekuTree, Route::SwordChest, Route::MidoShop, Route::NewSaveDekuTree, Route::NewFileDekuTree, Route::DekuBaba, Route::Combat].into_iter().find(|r| r.script() == name)
    }
}

/// What the run does, one task at a time.
#[derive(Debug, Clone)]
enum Task {
    /// Idle until the transition is over and Link stands, then `SETTLE_FRAMES` more.
    Settle(Option<Step>),
    /// Steer into the floor of the exit to the entrance, until the transition starts; then
    /// (unless the step is the exit itself) wait for the scene change and settle.
    Exit(&'static str, Option<Step>),
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
    /// towards the mouth (the second point) until A says "Enter" (`PLAYER_STATE2_DO_ACTION_ENTER`); A; the
    /// stick forward until Link is out and standing.
    Crawl(Vec3, Vec3, Step),
    /// Idle until the boulder rolls on its path from point `.0` to point `.1` and has covered
    /// at least `.2` of that segment.
    WaitBoulder(i16, i16, f32),
    /// Open the chest whose home is here: walk in front of it, at it until it offers its
    /// item, A, then A through the item's text until the box closes and Link stands.
    OpenChest(Vec3, Option<Step>),
    /// Pick up the `En_Item00` placed here (its home): onto it until it's collected.
    Pick(Vec3),
    /// Slash the interact switch (`En_Wonder_Item` mode 3) whose home is `.1`: from `.0`, at it,
    /// B, until it's gone; then pick up the rupee it dropped.
    SlashSwitch(Vec3, Vec3),
    /// The Kokiri shop: to the counter, talk to the shopkeeper, the stick right to the right
    /// shelf, A on its first item (the Deku Shield), "Buy", A through the item's text, B at
    /// "anything else?", until Link stands.
    BuyShield,
    /// Start (`KaleidoSetup_Update`'s pause menu, the equipping stand-in) until everything
    /// owned is worn (`SaveContext::equip_owned_unworn` has nothing left to do).
    Equip(Step),
    /// Idle until Mido has walked to his path's end.
    WaitMido,
    /// Towards the point until the Deku Tree's talk starts (`Bg_Treemouth`'s trigger), then A
    /// through its texts, yes at its question, until the scripts are over and Link is free.
    TreeTalk(Vec3),
    /// The opening's scripts (`sub`: 0 Link's house's layer 5, 1 the nightmare, 2 Kokiri Forest's
    /// layer 7, 3 the wake-up): A at each box that waits, until the last one is over and Link
    /// stands in his house.
    Opening,
    /// Idle until Navi has her text (Player's `naviTextId`, set by her update), then C-Up, and
    /// A through her text until the box closes and Link stands.
    TalkNavi,
    /// Fight the Deku Baba whose home is here (`fight_baba`).
    FightBaba(Vec3),
    /// Block the dive of the perched Keese whose home is here (`block_keese`).
    BlockKeese(Vec3),
    /// Kill that Keese and pick up its drop (`kill_keese`).
    KillKeese(Vec3),
    /// Kill the withered Deku Baba whose home is here and pick up its stick (`fight_karebaba`).
    FightKarebaba(Vec3),
}

/// An actor the run talks to.
#[derive(Debug, Clone, Copy)]
enum Who {
    /// The `En_Kanban` whose home is here.
    Sign(Vec3),
    /// The `En_Ko` of this child type (`params & 0xFF`).
    Kokiri(i16),
    /// Mido (`En_Md`).
    Mido,
}

const SETTLE_FRAMES: usize = 10;
const WAYPOINT_RADIUS: f32 = 30.0;
/// The stick's tilt for a run (of the pad's ±80), and for the slow approaches: a walk.
/// (`Player_CalcSpeedAndYawFromControlStick` takes 20 off the dead-zoned magnitude, so much under 30 only turns Link.)
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
    /// The rupees Link has at each step (`gSaveContext.rupees`).
    pub rupees: Vec<(Step, i16)>,
    /// The tries a switch took (`Task::SlashSwitch`).
    tries: usize,
    /// The rupees on the last frame seen.
    last_rupees: i16,
}

impl Default for Playthrough {
    fn default() -> Self {
        Self::new()
    }
}

impl Playthrough {
    /// The entrance the Deku Tree run starts at, and the save preset it needs.
    pub const ENTRANCE: &'static str = "ENTR_LINKS_HOUSE_0";
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
            Route::SwordChest => Self::sword_chest(false),
            Route::MidoShop => Self::mido_shop(),
            Route::NewSaveDekuTree => Self::new_save_deku_tree(),
            Route::NewFileDekuTree => Self::new_file_deku_tree(),
            Route::DekuBaba => vec![Task::FightBaba(DEKU_BABA_HOME)],
            // Back to the start between the fights: the Deku Baba grows back where Link picked up
            // its stick, and its head bites within reach of its home.
            Route::Combat => vec![Task::FightKarebaba(COMBAT_KAREBABA_HOME), Task::Walk(vec![COMBAT_START.0]), Task::BlockKeese(COMBAT_KEESE_HOME), Task::KillKeese(COMBAT_KEESE_HOME)],
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
            rupees: Vec::new(),
            tries: 0,
            last_rupees: 0,
        }
    }

    /// The Kokiri Sword run's tasks; with `rupees`, room 2's two blue rupees picked up on the way
    /// (the bottom corridor's west end, and the alcove it waits in).
    fn sword_chest(rupees: bool) -> Vec<Task> {
        let west_rupee = if rupees {
            vec![
                Task::Walk(vec![Vec3::new(-760.0, 120.0, 1470.0), Vec3::new(-800.0, 120.0, 1545.0), Vec3::new(-940.0, 120.0, 1565.0)]),
                // Room 2's En_Item00 0x0F01: a blue rupee.
                Task::Pick(Vec3::new(-1009.0, 120.0, 1556.0)),
                Task::Walk(vec![Vec3::new(-940.0, 120.0, 1565.0), Vec3::new(-800.0, 120.0, 1545.0), Vec3::new(-700.0, 120.0, 1515.0)]),
            ]
        } else {
            // The bottom corridor west of the boulder's path (its corner at (-575, 1538)).
            vec![Task::Walk(vec![Vec3::new(-760.0, 120.0, 1470.0), Vec3::new(-700.0, 120.0, 1515.0)])]
        };
        // Room 2's En_Item00 0x0E01, in the alcove: a blue rupee.
        let alcove_rupee = if rupees { vec![Task::Pick(Vec3::new(-712.0, 120.0, 1857.0))] } else { Vec::new() };
        let mut v = vec![
            Task::Settle(Some(Step::House)),
            Task::Exit("ENTR_KOKIRI_FOREST_3", Some(Step::OutDoor)),
            Task::LadderDown,
            // Out of the house's hollow, west through the village to the foot of the ramp, and
            // up it onto the plateau (y 120).
            Task::Walk(vec![Vec3::new(0.0, -80.0, 800.0), Vec3::new(0.0, 0.0, 480.0), Vec3::new(-300.0, 0.0, 300.0), Vec3::new(-450.0, 0.0, -150.0), Vec3::new(-650.0, 0.0, -170.0), Vec3::new(-650.0, 120.0, 250.0)]),
            // North across the plateau to the crawlspace by its sign (0x0337): its mouth is the
            // wall at z 1059, x -801..-769 (WALL_FLAG_CRAWLSPACE_1).
            Task::Walk(vec![Vec3::new(-650.0, 120.0, 500.0), Vec3::new(-785.0, 120.0, 850.0)]),
            Task::Crawl(Vec3::new(-785.0, 120.0, 1000.0), Vec3::new(-785.0, 120.0, 1059.0), Step::TrainingArea),
        ];
        v.extend(west_rupee);
        v.extend([
            // Once it has turned north up the middle corridor (path 2's points 2 to 3), after it
            // to the corridors' top-left corner and into the alcove west of it.
            Task::WaitBoulder(2, 3, 0.35),
            Task::Hurry(vec![Vec3::new(-555.0, 120.0, 1545.0), Vec3::new(-555.0, 120.0, 1850.0), Vec3::new(-690.0, 120.0, 1880.0)]),
        ]);
        v.extend(alcove_rupee);
        v.extend([
            // Once it has passed along the top corridor (points 3 to 4), after it east to the
            // top-right corner, and north out of its way.
            Task::WaitBoulder(3, 4, 0.35),
            Task::Hurry(vec![Vec3::new(-560.0, 120.0, 1885.0), Vec3::new(-250.0, 120.0, 1895.0), Vec3::new(-250.0, 120.0, 1960.0)]),
            Task::Settle(Some(Step::Boulder)),
            // Up the slope to the chest's dais.
            Task::Walk(vec![Vec3::new(-250.0, 140.0, 2080.0), Vec3::new(-232.0, 160.0, 2170.0)]),
            // room 2's En_Box (params 0x04E0): the Kokiri Sword.
            Task::OpenChest(Vec3::new(-232.0, 178.0, 2245.0), Some(Step::Chest)),
        ]);
        v
    }

    /// The Mido and shop run's tasks: the sword with room 2's rupees, then back to the village,
    /// the rest of the rupees, the shop, the equipping, and Mido.
    fn mido_shop() -> Vec<Task> {
        let mut v = Self::sword_chest(true);
        v.extend([
            // Text 0xA4 says to equip it: Start, the pause menu's stand-in, puts it on B.
            Task::Equip(Step::SwordOn),
            // Back down from the chest's dais, north of the corridors' top-right corner (path 2's
            // point 0, where the loop starts again).
            Task::Walk(vec![Vec3::new(-250.0, 140.0, 2080.0), Vec3::new(-250.0, 120.0, 1960.0)]),
            // Once it has turned south down the east corridor (points 0 to 1), west along the top
            // corridor ahead of it into the alcove (it comes up the middle one, points 2 to 3,
            // after about 100 frames).
            Task::WaitBoulder(0, 1, 0.1),
            Task::Hurry(vec![Vec3::new(-260.0, 120.0, 1895.0), Vec3::new(-560.0, 120.0, 1885.0), Vec3::new(-690.0, 120.0, 1880.0)]),
            // Once it's past the top-left corner going east (points 3 to 4), south down the middle
            // corridor and west along the bottom one ahead of it, to the crawlspace.
            Task::WaitBoulder(3, 4, 0.25),
            Task::Hurry(vec![Vec3::new(-555.0, 120.0, 1850.0), Vec3::new(-555.0, 120.0, 1545.0), Vec3::new(-700.0, 120.0, 1515.0), Vec3::new(-760.0, 120.0, 1470.0)]),
            // Into the crawlspace from room 2 (its mouth there is the wall Link climbed out of),
            // out into room 0 by the sign.
            Task::Crawl(Vec3::new(-785.0, 120.0, 1430.0), Vec3::new(-785.0, 120.0, 1340.0), Step::Plateau),
            // South over the plateau to the sign (0x0336) with the hidden switch beside it
            // (En_Wonder_Item 0x1A53: mode 3, a blue rupee, switch 0x13; rot.z 0, a sword or a
            // stick).
            Task::Walk(vec![Vec3::new(-785.0, 120.0, 850.0), Vec3::new(-650.0, 120.0, 700.0)]),
            Task::SlashSwitch(Vec3::new(-455.0, 120.0, 625.0), Vec3::new(-488.0, 140.0, 600.0)),
            // Down the ramp to the village, two green rupees (En_Item00 0x2700, 0x2400) at its
            // foot's east side.
            Task::Walk(vec![Vec3::new(-650.0, 120.0, 500.0), Vec3::new(-650.0, 120.0, 250.0), Vec3::new(-650.0, 0.0, -170.0), Vec3::new(-450.0, 0.0, -150.0)]),
            Task::Pick(Vec3::new(-459.0, 1.0, 181.0)),
            Task::Pick(Vec3::new(-537.0, 1.0, 194.0)),
            // South to Mido's house.
            Task::Walk(vec![Vec3::new(-450.0, 0.0, -150.0), Vec3::new(-445.0, 0.0, -486.0)]),
            Task::Exit("ENTR_MIDOS_HOUSE_0", Some(Step::MidoHouse)),
            // His four chests (En_Box 0x59A0, 0x59A1: blue rupees; 0x5982: a green one; 0x5903: a
            // recovery heart).
            Task::OpenChest(Vec3::new(58.0, 0.0, -55.0), None),
            Task::OpenChest(Vec3::new(58.0, 0.0, 35.0), None),
            Task::OpenChest(Vec3::new(-60.0, 0.0, 35.0), None),
            Task::OpenChest(Vec3::new(-60.0, 0.0, -55.0), None),
            Task::Exit("ENTR_KOKIRI_FOREST_9", Some(Step::MidoChests)),
            // East to two green rupees (En_Item00 0x2500, 0x2600).
            Task::Walk(vec![Vec3::new(-445.0, 0.0, -400.0), Vec3::new(-150.0, 0.0, -350.0)]),
            Task::Pick(Vec3::new(35.0, 1.0, -418.0)),
            Task::Pick(Vec3::new(107.0, 1.0, -418.0)),
            // The free multitag's two tag points (En_Wonder_Item 0x0FE0 at (188, 3, -198) and
            // (548, 3, -158)), within 80 frames of each other: a blue rupee, collected at once.
            Task::Hurry(vec![Vec3::new(188.0, 3.0, -198.0), Vec3::new(548.0, 3.0, -158.0)]),
            // The shop's door.
            Task::Walk(vec![Vec3::new(854.0, 0.0, -250.0)]),
            Task::Exit("ENTR_KOKIRI_SHOP_0", Some(Step::Shop)),
            // The shop's own rupee (En_Wonder_Item 0x1250 at (146, 0, -97): a proximity drop, a
            // blue rupee), within 50 of it past the counter's right end (the walk stops 30 short
            // of its last point).
            Task::Walk(vec![Vec3::new(150.0, 0.0, 40.0), Vec3::new(158.0, 0.0, -85.0)]),
            Task::BuyShield,
            Task::Equip(Step::Equipped),
            Task::Exit("ENTR_KOKIRI_FOREST_4", Some(Step::ShopOut)),
            // North to the ford and over it, to Mido by the path to the Deku Tree.
            Task::Walk(vec![Vec3::new(850.0, 0.0, 200.0), Vec3::new(900.0, 0.0, 430.0), Vec3::new(1250.0, 0.0, 410.0), Vec3::new(1375.0, 0.0, 380.0), Vec3::new(1400.0, 0.0, 250.0)]),
            Task::Talk(Who::Mido, Vec3::new(1420.0, 0.0, 150.0), Step::Mido),
            Task::WaitMido,
            // Past where he stood, east along the path.
            Task::Walk(vec![Vec3::new(1470.0, 0.0, 120.0), Vec3::new(1650.0, 0.0, 140.0)]),
            Task::Settle(Some(Step::PastMido)),
        ]);
        v
    }

    /// The new save's run into the Deku Tree: the Mido and shop run, then along the path to the
    /// `En_Holl` into room 1 and into the meadow, where the tree's first talk starts; yes, and
    /// on to his open mouth.
    fn new_save_deku_tree() -> Vec<Task> {
        let mut v = Self::mido_shop();
        v.extend([
            Task::Walk(vec![Vec3::new(1900.0, 0.0, 140.0), Vec3::new(2175.0, 0.0, 100.0), Vec3::new(2175.0, 0.0, -148.0), Vec3::new(2175.0, 0.0, -480.0)]),
            // func_808BC8B8's trigger: within 1658 of the mouth and facing it within 0x4E20.
            Task::TreeTalk(Vec3::new(2600.0, 0.0, -520.0)),
            // From where the talk's cues left Link (short of (2857, 0, -594)), over the gap the
            // open jaw covers.
            Task::Walk(vec![Vec3::new(3000.0, 0.0, -600.0), Vec3::new(3350.0, 0.0, -950.0), Vec3::new(3650.0, 0.0, -1180.0), Vec3::new(3950.0, 0.0, -1200.0)]),
            Task::Settle(Some(Step::Tree)),
            Task::Exit("ENTR_DEKU_TREE_0", Some(Step::Mouth)),
            // The Deku Tree's intro (gDekuTreeIntroCs) holds Link until it ends.
            Task::Settle(Some(Step::DekuTree)),
        ]);
        v
    }

    /// The new file's run: the opening, then the new save's run into the Deku Tree from where it
    /// leaves Link, with C-Up to Navi on the plateau before the crawlspace (by then her timer
    /// is past 600).
    fn new_file_deku_tree() -> Vec<Task> {
        let mut v = vec![Task::Opening];
        let rest = Self::new_save_deku_tree();
        let crawl = rest.iter().position(|t| matches!(t, Task::Crawl(..))).expect("the crawlspace");
        for (i, t) in rest.into_iter().enumerate() {
            if i == crawl {
                v.push(Task::TalkNavi);
            }
            v.push(t);
        }
        v
    }

    /// The Deku Tree run's tasks.
    fn deku_tree() -> Vec<Task> {
        vec![
            Task::Settle(Some(Step::House)),
            Task::Exit("ENTR_KOKIRI_FOREST_3", Some(Step::OutDoor)),
            Task::LadderDown,
            // The sign by Link's house (params 0x031F) faces -z: read from in front of it.
            Task::Talk(Who::Sign(Vec3::new(49.0, -80.0, 967.0)), Vec3::new(49.0, -80.0, 900.0), Step::Sign),
            // Up the slope out of the house's hollow into the village.
            Task::Walk(vec![Vec3::new(0.0, -80.0, 800.0), Vec3::new(0.0, 0.0, 480.0), Vec3::new(400.0, 0.0, 470.0)]),
            // Child 4 at (669, 0, 521) faces about -z.
            Task::Talk(Who::Kokiri(4), Vec3::new(660.0, 0.0, 465.0), Step::Kokiri),
            Task::CutBushes(vec![Vec3::new(594.0, 0.0, 542.0), Vec3::new(385.0, 0.0, 643.0), Vec3::new(678.0, 0.0, 596.0), Vec3::new(572.0, 0.0, 603.0)]),
            // Through the stream's ford and along its east side, round Mido where he stands
            // aside (path 1's end, (1412, 0, 211): the preset's EVENTCHKINF_04) by the narrow
            // bank west of him, along the path to the En_Holl and down into the meadow, to the
            // open jaw.
            Task::Walk(vec![
                Vec3::new(900.0, 0.0, 430.0),
                Vec3::new(1250.0, 0.0, 410.0),
                Vec3::new(1375.0, 0.0, 380.0),
                Vec3::new(1388.0, 0.0, 268.0),
                Vec3::new(1345.0, 0.0, 230.0),
                Vec3::new(1350.0, 0.0, 165.0),
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
            Task::Exit("ENTR_DEKU_TREE_0", Some(Step::Mouth)),
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
            self.rupees.push((s, self.last_rupees));
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
        if self.frame >= self.route.max_frames() {
            self.failure = Some(format!("out of frames in {}", self.at()));
            return None;
        }
        self.last_rupees = w.save.rupees;
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
                        if step == Some(Step::Mouth) {
                            // The step is the exit itself; the next task waits for the scene.
                            self.finish(step);
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
                _ => self.settle(w, step),
            },
            Task::Walk(points) => self.walk(w, &points, RUN),
            Task::Hurry(points) => self.walk(w, &points, FULL),
            Task::Crawl(approach, mouth, step) => self.crawl(w, approach, mouth, step),
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
            Task::OpenChest(home, step) => self.open_chest(w, home, step),
            Task::Pick(home) => self.pick(w, home),
            Task::SlashSwitch(from, at) => self.slash_switch(w, from, at),
            Task::BuyShield => self.buy_shield(w),
            Task::Equip(step) => {
                // Done once the stand-in would equip nothing more.
                if !w.save.clone().equip_owned_unworn() {
                    self.finish(Some(step));
                    return None;
                }
                self.wait += 1;
                if self.wait > 30 {
                    self.failure = Some(format!("Start didn't equip what's owned (equipment {:#06x}, worn {:#06x})", w.save.inventory.equipment, w.save.equips.equipment));
                    return None;
                }
                // KaleidoSetup_Update reads Start only with no message box.
                Some(if w.msg_ctx.msg_mode == oot_game::message::MSGMODE_NONE { self.press(BTN_START) } else { idle })
            }
            Task::WaitMido => {
                let Some(m) = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<EnMd>(h)) else {
                    self.failure = Some("no Mido to wait for".into());
                    return None;
                };
                if m.action == MdAction::Arrived {
                    self.finish(Some(Step::MidoAside));
                    return None;
                }
                self.wait += 1;
                if self.wait > 600 {
                    self.failure = Some(format!("Mido didn't reach his path's end: {:?} at {}", m.action, m.actor.world_pos));
                    return None;
                }
                Some(idle)
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
            Task::FightBaba(home) => self.fight_baba(w, home),
            Task::BlockKeese(home) => self.block_keese(w, home),
            Task::KillKeese(home) => self.kill_keese(w, home),
            Task::FightKarebaba(home) => self.fight_karebaba(w, home),
            Task::Opening => self.opening(w),
            Task::TalkNavi => self.talk_navi(w),
            Task::TreeTalk(towards) => {
                use oot_game::cutscene::CS_STATE_IDLE;
                match self.sub {
                    0 => {
                        if w.cs_ctx.state != CS_STATE_IDLE {
                            self.sub = 1;
                            return Some(idle);
                        }
                        if Self::xz_dist(link, towards) < WAYPOINT_RADIUS {
                            self.failure = Some(format!("the Deku Tree's talk didn't start on the way to {towards}"));
                            return None;
                        }
                        Some(stick_towards(w, towards, RUN))
                    }
                    // A through the texts; A at the question takes its first answer, yes.
                    _ => {
                        self.wait += 1;
                        if self.wait > 3000 {
                            self.failure = Some(format!("the Deku Tree's talk didn't end: cs state {}, frames {}, text {:#x}", w.cs_ctx.state, w.cs_ctx.frames, w.msg_ctx.text_id));
                            return None;
                        }
                        let over = w.cs_ctx.state == CS_STATE_IDLE && w.player().cs_mode == 0 && w.message_state() == TEXT_STATE_NONE;
                        if over && w.save.get_event_chk_inf(oot_game::save::EVENTCHKINF_05) {
                            self.finish(Some(Step::TreeTalk));
                            return None;
                        }
                        if over {
                            self.failure = Some("the Deku Tree's talk ended without EVENTCHKINF_05".into());
                            return None;
                        }
                        Some(if Self::text_waits(w) && (w.message_state() != TEXT_STATE_CHOICE || w.msg_ctx.choice_index == 0) { self.press(BTN_A) } else { idle })
                    }
                }
            }
        }
    }

    /// `Task::FightBaba`'s phases in `sub`, keyed on the Deku Baba's action
    /// (`oot_actors::en_dekubaba::Action`) and Link's health:
    /// - 0: standing, until the bite hits (`Step::Bitten`);
    /// - 1: Z held (locked on), until it's stuck after a missed bite (`EnDekubaba_RecoverFromAttackMiss`);
    /// - 2: at its head, B within 50;
    /// - 3: until it lies stretched out (`EnDekubaba_Vulnerable`, `Step::BabaWeakened`), else
    ///   back to 2;
    /// - 4: at its home at full tilt, B within 40;
    /// - 5: until its head is a Deku Stick (`Step::BabaCut`), else back to 4.
    fn fight_baba(&mut self, w: &PlayState, home: Vec3) -> Option<PadState> {
        use crate::en_dekubaba::{Action as BA, EnDekubaba};
        use eng_input::pad::BTN_Z;
        let idle = PadState::default();
        let Some(b) = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<EnDekubaba>(h).filter(|b| b.actor.home_pos.distance(home) < 1.0)) else {
            // Its room's actors spawn (and wait for their object) in the first frames.
            self.wait += 1;
            if self.wait > 60 {
                self.failure = Some(format!("no Deku Baba at {home}"));
                return None;
            }
            return Some(idle);
        };
        let link = w.player().actor.world_pos;
        let z = |mut p: PadState| {
            p.button |= BTN_Z;
            p
        };
        self.wait += 1;
        if self.wait > 400 {
            self.failure = Some(format!("the Deku Baba fight stalled in phase {} ({:?})", self.sub, b.action));
            return None;
        }
        match self.sub {
            0 => {
                if w.save.health < w.save.health_capacity {
                    self.steps.push((Step::Bitten, self.frame));
                    self.done = Some(Step::Bitten);
                    self.sub = 1;
                    self.wait = 0;
                }
                Some(idle)
            }
            1 => {
                if b.action == BA::RecoverFromAttackMiss {
                    self.sub = 2;
                    self.wait = 0;
                }
                Some(z(idle))
            }
            2 => {
                let mut p = z(stick_towards(w, b.actor.world_pos, SLOW));
                if Self::xz_dist(link, b.actor.world_pos) < 50.0 {
                    p.button |= BTN_B;
                    self.sub = 3;
                    self.wait = 0;
                }
                Some(p)
            }
            3 => {
                if b.action == BA::Vulnerable {
                    self.steps.push((Step::BabaWeakened, self.frame));
                    self.done = Some(Step::BabaWeakened);
                    self.sub = 4;
                    self.wait = 0;
                } else if self.wait > 30 && b.action != BA::Attacked {
                    self.sub = 2;
                    self.wait = 0;
                }
                Some(z(idle))
            }
            4 => {
                let mut p = z(stick_towards(w, home, FULL));
                if Self::xz_dist(link, home) < 40.0 {
                    p.button |= BTN_B;
                    self.sub = 5;
                    self.wait = 0;
                }
                Some(p)
            }
            _ => {
                if b.action == BA::DekuStick {
                    self.finish(Some(Step::BabaCut));
                    return None;
                }
                if self.wait > 30 && b.action == BA::Vulnerable {
                    self.sub = 4;
                    self.wait = 0;
                }
                Some(idle)
            }
        }
    }

    /// The `En_Firefly` whose home is `home`, if it's still there.
    fn keese_at(w: &PlayState, home: Vec3) -> Option<(ActorHandle, &crate::en_firefly::EnFirefly)> {
        w.actors.all().into_iter().find_map(|h| w.actors.downcast::<crate::en_firefly::EnFirefly>(h).filter(|k| k.actor.home_pos.distance(home) < 1.0).map(|k| (h, k)))
    }

    /// `Task::BlockKeese`'s phases in `sub`:
    /// - 0: at the Keese across the floor while it's perched, until it isn't (it dives within
    ///   120: `EnFirefly_AttackFromPerched`);
    /// - 1: turned to it in place, and Z pressed while it's ahead, until Link is locked on to it
    ///   (`focusActor`);
    /// - 2: Z (locked on) and R (the shield up on the upper body, `func_80834758`), until the
    ///   shield takes a dive (Player's `Player_Action_808435C4`, `Step::Blocked`).
    fn block_keese(&mut self, w: &PlayState, home: Vec3) -> Option<PadState> {
        use crate::en_firefly::Action as KA;
        use eng_input::pad::{BTN_R, BTN_Z};
        let idle = PadState::default();
        let Some((_, k)) = Self::keese_at(w, home) else {
            self.wait += 1;
            if self.wait > 60 {
                self.failure = Some(format!("no Keese at {home}"));
                return None;
            }
            return Some(idle);
        };
        self.wait += 1;
        if self.wait > 2400 {
            self.failure = Some(format!("the Keese's block stalled in phase {} ({:?})", self.sub, k.action));
            return None;
        }
        let (kh, k_pos) = (Self::keese_at(w, home).map(|(h, _)| h), k.actor.world_pos);
        match self.sub {
            0 => {
                if k.action != KA::Perched {
                    self.sub = 1;
                    self.wait = 0;
                    return Some(idle);
                }
                Some(stick_towards(w, home, SLOW))
            }
            1 => {
                if w.player().focus_actor.is_some() && w.player().focus_actor == kh {
                    self.sub = 2;
                    return Some(PadState { button: BTN_Z, ..idle });
                }
                // Turned to it in place while it's off to the side (a light tilt: past the dead
                // zone, too little to walk; no Z, which would strafe), else Z on every other frame
                // until it's in range.
                let off = eng_math::vec3f_yaw(w.player().actor.world_pos, k_pos).wrapping_sub(w.player().actor.shape_rot.y);
                if (off as i32).abs() > 0x1000 {
                    return Some(stick_towards(w, k_pos, 25.0));
                }
                Some(if self.prev.button & BTN_Z == 0 { PadState { button: BTN_Z, ..idle } } else { idle })
            }
            _ => {
                if w.player().action == PA::GuardHit {
                    self.finish(Some(Step::Blocked));
                    return None;
                }
                if w.player().focus_actor != kh {
                    self.sub = 1;
                    return Some(idle);
                }
                Some(PadState { button: BTN_Z | BTN_R, ..idle })
            }
        }
    }

    /// `Task::KillKeese`'s phases in `sub`:
    /// - 0: B once, drawing the sword (its first slash at nothing), locked on (Z);
    /// - 1: locked on with the shield up (Z and R) until a dive is blocked and the Keese hovers
    ///   (`EnFirefly_Stay`, its `AT_HIT`), then R let go, in at a run to within 25 across, and B;
    /// - 4: Z held through the slash; killed (`EnFirefly_Die`): 2; else back to 1;
    /// - 2: until it's gone (`EnFirefly_Disappear` over), noting the drop (`Item_DropCollectibleRandom`);
    /// - 3: onto the drop until it's collected, then `Step::KeeseKilled`.
    fn kill_keese(&mut self, w: &PlayState, home: Vec3) -> Option<PadState> {
        use crate::en_firefly::Action as KA;
        use eng_input::pad::{BTN_R, BTN_Z};
        let idle = PadState::default();
        self.wait += 1;
        if self.wait > 5000 {
            self.failure = Some(format!("the Keese fight stalled in phase {}", self.sub));
            return None;
        }
        match self.sub {
            0 | 1 | 4 => {
                let Some((_, k)) = Self::keese_at(w, home) else {
                    self.failure = Some("the Keese went before it was killed".into());
                    return None;
                };
                if matches!(k.action, KA::Die | KA::Disappear) {
                    self.sub = 2;
                    self.items = w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnItem00>(h).is_some()).collect();
                    return Some(idle);
                }
                let p = w.player();
                match self.sub {
                    0 => {
                        if p.held_item_ap == w.data.items.ap("SWORD_KOKIRI") && p.action != PA::Attack {
                            self.sub = 1;
                        }
                        Some(PadState { button: BTN_Z | if self.prev.button & BTN_B == 0 && p.action != PA::Attack { BTN_B } else { 0 }, ..idle })
                    }
                    1 => {
                        if k.action == KA::Stay && matches!(p.action, PA::TargetIdle | PA::TargetRun | PA::GuardHit | PA::StandingStill) {
                            // In reach (25 across: it hovers some 55 up): B; else in at a run, still
                            // locked on.
                            if Self::xz_dist(p.actor.world_pos, k.actor.world_pos) > 25.0 {
                                let mut s = stick_towards(w, k.actor.world_pos, RUN);
                                s.button |= BTN_Z;
                                return Some(s);
                            }
                            self.sub = 4;
                            self.tries = 0;
                            return Some(PadState { button: BTN_Z | BTN_B, ..idle });
                        }
                        Some(PadState { button: BTN_Z | BTN_R, ..idle })
                    }
                    _ => {
                        self.tries += 1;
                        if self.tries > 6 && p.action != PA::Attack {
                            self.sub = 1;
                        }
                        Some(PadState { button: BTN_Z, ..idle })
                    }
                }
            }
            2 => {
                if Self::keese_at(w, home).is_some() {
                    return Some(idle);
                }
                self.dropped = w.actors.all().into_iter().find(|h| !self.items.contains(h) && w.actors.downcast::<EnItem00>(*h).is_some());
                self.drop = self.dropped.and_then(|h| w.actors.actor(h)).map(|a| a.params);
                if self.dropped.is_none() {
                    self.finish(Some(Step::KeeseKilled));
                    return None;
                }
                self.sub = 3;
                Some(idle)
            }
            _ => {
                let Some(it) = self.dropped.and_then(|h| w.actors.downcast::<EnItem00>(h)).filter(|i| i.action != ItemAction::Collected) else {
                    self.finish(Some(Step::KeeseKilled));
                    return None;
                };
                Some(stick_towards(w, it.actor.world_pos, SLOW))
            }
        }
    }

    /// `Task::FightKarebaba`'s phases in `sub`, keyed on its action (`en_karebaba::Action`):
    /// - 0: towards it until it springs (`EnKarebaba_Awaken`);
    /// - 1: locked on (Z); once it's upright (`EnKarebaba_Upright`, `_Spin`), within 40 of its
    ///   home, B;
    /// - 2: until it's dying (`Step::KarebabaKilled`), else back to 1;
    /// - 3: until its head is the stick (`EnKarebaba_DeadItemDrop`), then onto it, A while it
    ///   offers (Player's `getItemId`), until Link has it;
    /// - 4: A through the item's text until the box closes and Link stands (`Step::StickTaken`).
    fn fight_karebaba(&mut self, w: &PlayState, home: Vec3) -> Option<PadState> {
        use crate::en_karebaba::{Action as KA, EnKarebaba};
        use eng_input::pad::BTN_Z;
        let idle = PadState::default();
        let Some((h, b)) = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<EnKarebaba>(h).filter(|b| b.actor.home_pos.distance(home) < 1.0).map(|b| (h, b))) else {
            // Its room's actors spawn (and wait for their object) in the first frames.
            self.wait += 1;
            if self.wait > 60 {
                self.failure = Some(format!("no withered Deku Baba at {home}"));
                return None;
            }
            return Some(idle);
        };
        let link = w.player().actor.world_pos;
        self.wait += 1;
        if self.wait > 900 {
            self.failure = Some(format!("the withered Deku Baba fight stalled in phase {} ({:?})", self.sub, b.action));
            return None;
        }
        let z = |mut p: PadState| {
            p.button |= BTN_Z;
            p
        };
        match self.sub {
            0 => {
                if b.action == KA::Awaken {
                    self.sub = 1;
                    return Some(z(idle));
                }
                Some(stick_towards(w, home, RUN))
            }
            1 => {
                if matches!(b.action, KA::Upright | KA::Spin) && Self::xz_dist(link, home) < 40.0 {
                    self.sub = 2;
                    self.tries = 0;
                    let mut p = z(idle);
                    p.button |= BTN_B;
                    return Some(p);
                }
                if Self::xz_dist(link, home) < 32.0 {
                    return Some(z(idle));
                }
                Some(z(stick_towards(w, home, SLOW)))
            }
            2 => {
                if b.action == KA::Dying {
                    self.steps.push((Step::KarebabaKilled, self.frame));
                    self.done = Some(Step::KarebabaKilled);
                    self.sub = 3;
                    self.tries = w.save.ammo(oot_game::item::ITEM_DEKU_STICK) as usize;
                    return Some(idle);
                }
                self.tries += 1;
                if self.tries > 20 {
                    self.sub = 1;
                }
                Some(z(idle))
            }
            4 => {
                // A through the item's text until the box closes and Link stands.
                if w.message_state() == TEXT_STATE_NONE && !matches!(w.player().action, PA::GetItem | PA::ItemPutAway) {
                    self.finish(Some(Step::StickTaken));
                    return None;
                }
                Some(if Self::text_waits(w) { self.press(BTN_A) } else { idle })
            }
            _ => {
                if w.save.ammo(oot_game::item::ITEM_DEKU_STICK) as usize > self.tries {
                    self.sub = 4;
                    return Some(idle);
                }
                if b.action != KA::DeadItemDrop {
                    return Some(idle);
                }
                let p = w.player();
                if p.interact_range_actor == Some(h) && p.get_item_id != 0 {
                    return Some(self.press(BTN_A));
                }
                Some(stick_towards(w, b.actor.world_pos, SLOW))
            }
        }
    }

    /// `Task::Opening`: A at each box that waits (every other frame), marking each scene the
    /// chain enters, until the wake-up's script is over and Link stands in his house.
    fn opening(&mut self, w: &PlayState) -> Option<PadState> {
        use oot_game::cutscene::CS_STATE_IDLE;
        const SCENE_HYRULE_FIELD: u16 = 0x51;
        const SCENE_KOKIRI_FOREST: u16 = 0x55;
        const SCENE_LINKS_HOUSE: u16 = 0x34;
        let at = (w.scene_id, w.save.scene_layer);
        let next = match self.sub {
            0 => (at == (SCENE_HYRULE_FIELD, 4)).then_some(Step::Nightmare),
            1 => (at == (SCENE_KOKIRI_FOREST, 7)).then_some(Step::NaviSent),
            2 => (at == (SCENE_LINKS_HOUSE, 4)).then_some(Step::WakeUp),
            _ => None,
        };
        if let Some(s) = next {
            self.sub += 1;
            if s != Step::WakeUp {
                self.steps.push((s, self.frame));
                self.done = Some(s);
                self.rupees.push((s, self.last_rupees));
            }
        }
        if self.sub == 3 {
            let over = w.cs_ctx.state == CS_STATE_IDLE && w.save.cutscene_index == 0 && w.player().cs_mode == 0 && w.message_state() == TEXT_STATE_NONE;
            if over && Self::settled(w) {
                self.wait += 1;
                if self.wait > SETTLE_FRAMES {
                    self.finish(Some(Step::WakeUp));
                    return None;
                }
            }
        }
        Some(if w.message_state() != TEXT_STATE_NONE && Self::text_waits(w) { self.press(BTN_A) } else { PadState::default() })
    }

    /// `Task::TalkNavi`'s phases in `sub`: 0 idle until Navi has her text; 1 C-Up until the talk
    /// starts; 2 A through her text until the box closes and Link stands.
    fn talk_navi(&mut self, w: &PlayState) -> Option<PadState> {
        let idle = PadState::default();
        let p = w.player();
        self.wait += 1;
        if self.wait > 3000 {
            self.failure = Some(format!("the talk to Navi stalled: sub {}, naviTimer {}, naviTextId {:#x}", self.sub, w.save.navi_timer, p.navi_text_id));
            return None;
        }
        match self.sub {
            0 => {
                if p.navi_text_id != 0 {
                    self.sub = 1;
                }
                Some(idle)
            }
            1 => {
                if w.message_state() != TEXT_STATE_NONE {
                    self.sub = 2;
                    return Some(idle);
                }
                Some(self.press(eng_input::pad::BTN_CUP))
            }
            _ => {
                if w.message_state() == TEXT_STATE_NONE && Self::settled(w) {
                    self.finish(Some(Step::Navi));
                    return None;
                }
                Some(if Self::text_waits(w) { self.press(BTN_A) } else { idle })
            }
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
        let mut pad = stick_towards(w, points[self.sub], mag);
        // A text that opens by itself (a forced En_Wonder_Talk2, like the one by the shop) holds
        // Link (Player's cutscene mode 8) until it's read: A through it, as a player would.
        if w.message_state() != TEXT_STATE_NONE && Self::text_waits(w) {
            pad.button = self.press(BTN_A).button;
        }
        Some(pad)
    }

    /// `Task::Crawl`'s phases in `sub`: 0 to the approach, 1 at the mouth until "Enter", 2 A
    /// (with the stick still at the mouth, so Link stays in `Player_Action_80842180`, whose interrupts
    /// include the wall's, `Player_ActionHandler_5`), 3 crawling, 4 out and settling.
    fn crawl(&mut self, w: &PlayState, approach: Vec3, mouth: Vec3, step: Step) -> Option<PadState> {
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
            // Player_Action_8084C760 reads the stick's tilt straight (rel.stick_y), not the camera's way.
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
            _ => self.settle(w, Some(step)),
        }
    }

    /// `Task::OpenChest`'s phases: 0 in front (35 before it, on its axis: its collision holds
    /// Link at 34), running (a slow walk stalls at the foot of the Kokiri Sword chest's
    /// mound), 1 at it until it offers (a chest's negative get-item id), 2 A, 3 the opening and
    /// the text.
    fn open_chest(&mut self, w: &PlayState, home: Vec3, step: Option<Step>) -> Option<PadState> {
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
                    self.finish(step);
                    return None;
                }
                if w.message_state() != TEXT_STATE_NONE {
                    self.wait += 1;
                }
                Some(if Self::text_waits(w) { self.press(BTN_A) } else { idle })
            }
        }
    }

    /// The `En_Item00` placed at `home` (not yet collected).
    fn item_at(w: &PlayState, home: Vec3) -> Option<(ActorHandle, &EnItem00)> {
        w.actors.all().into_iter().find_map(|h| w.actors.downcast::<EnItem00>(h).filter(|e| e.actor.home_pos.distance(home) < 1.0 && e.action != ItemAction::Collected).map(|e| (h, e)))
    }

    /// `Task::Pick`: onto the item until it's collected (gone, or `EnItem00_Collected`), at a
    /// walk for the last 40.
    fn pick(&mut self, w: &PlayState, home: Vec3) -> Option<PadState> {
        let link = w.player().actor.world_pos;
        let item = match self.dropped {
            Some(h) if self.sub == 1 => w.actors.downcast::<EnItem00>(h).filter(|e| e.action != ItemAction::Collected).map(|e| (h, e)),
            _ => Self::item_at(w, home),
        };
        match item {
            None if self.sub == 0 => {
                self.failure = Some(format!("no item to pick up at {home}"));
                None
            }
            None => {
                self.dropped = None;
                self.finish(None);
                None
            }
            Some((h, e)) => {
                self.sub = 1;
                self.dropped = Some(h);
                self.wait += 1;
                if self.wait > 400 {
                    self.failure = Some(format!("the item at {home} wasn't picked up (at {})", e.actor.world_pos));
                    return None;
                }
                let at = e.actor.world_pos;
                Some(stick_towards(w, at, if Self::xz_dist(link, at) < 40.0 { SLOW } else { RUN }))
            }
        }
    }

    /// `Task::SlashSwitch`'s phases in `sub`: 0 to `from`, 1 at the switch until 35 from it,
    /// then B; 2 the slash (30 frames), then the switch gone or another try; 3 the rupee it
    /// dropped picked up.
    fn slash_switch(&mut self, w: &PlayState, from: Vec3, at: Vec3) -> Option<PadState> {
        let idle = PadState::default();
        let link = w.player().actor.world_pos;
        let switch = w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnWonderItem>(h).is_some_and(|s| s.actor.home_pos.distance(at) < 1.0));
        match self.sub {
            0 => {
                if switch.is_none() {
                    self.failure = Some(format!("no switch at {at}"));
                    return None;
                }
                if Self::xz_dist(link, from) < 10.0 {
                    self.sub = 1;
                }
                Some(stick_towards(w, from, SLOW))
            }
            1 => {
                if Self::xz_dist(link, at) < 35.0 {
                    self.sub = 2;
                    self.wait = 0;
                    self.items = Self::drops(w);
                    return Some(self.press(BTN_B));
                }
                Some(stick_towards(w, at, SLOW))
            }
            2 => {
                self.wait += 1;
                if switch.is_none() {
                    self.dropped = Self::drops(w).into_iter().find(|h| !self.items.contains(h));
                    if self.dropped.is_none() {
                        self.failure = Some("the switch went without a drop".into());
                        return None;
                    }
                    self.sub = 3;
                    self.wait = 0;
                    return Some(idle);
                }
                if self.wait > 30 {
                    self.tries += 1;
                    if self.tries > 5 {
                        self.failure = Some(format!("the switch at {at} wasn't hit in 5 slashes"));
                        return None;
                    }
                    self.sub = 0;
                }
                Some(idle)
            }
            _ => match self.dropped.and_then(|h| w.actors.downcast::<EnItem00>(h)) {
                Some(e) if e.action != ItemAction::Collected => {
                    self.wait += 1;
                    if self.wait > 300 {
                        self.failure = Some("the switch's rupee wasn't picked up".into());
                        return None;
                    }
                    // Wait for it to land, then onto it.
                    if !matches!(e.action, ItemAction::Rest) && self.wait < 60 {
                        return Some(idle);
                    }
                    Some(stick_towards(w, e.actor.world_pos, SLOW))
                }
                _ => {
                    self.dropped = None;
                    self.finish(Some(Step::Switch));
                    None
                }
            },
        }
    }

    /// `Task::BuyShield`'s phases in `sub`, each keyed on the shopkeeper's state
    /// (`EnOssan.stateFlag`) and the message box: 0 to the counter; 1 at the shopkeeper until
    /// he offers; 2 A; 3 A at "Welcome!" (its event end); 4 the stick right at 0x83's choice; 5 A
    /// on the right shelf's first item's description; 6 A on "Buy"; 7 A through the item's
    /// text; 8 B at "anything else?"; 9 the box closed and Link standing.
    fn buy_shield(&mut self, w: &PlayState) -> Option<PadState> {
        use crate::en_ossan::*;
        use oot_game::message::TEXT_STATE_EVENT;
        let idle = PadState::default();
        let Some((h, o)) = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<EnOssan>(h).map(|o| (h, o))) else {
            self.failure = Some("no shopkeeper".into());
            return None;
        };
        let p = w.player();
        let link = p.actor.world_pos;
        let st = w.message_state();
        self.wait += 1;
        if self.wait > 1500 {
            self.failure = Some(format!("the shopping stalled: sub {}, shopkeeper state {}, text {:#x}", self.sub, o.state_flag, w.msg_ctx.text_id));
            return None;
        }
        match self.sub {
            0 => {
                let counter = Vec3::new(20.0, 0.0, 40.0);
                if Self::xz_dist(link, counter) < 10.0 {
                    self.sub = 1;
                }
                Some(stick_towards(w, counter, SLOW))
            }
            1 => {
                if p.target_actor == Some(h) {
                    self.sub = 2;
                    return Some(idle);
                }
                Some(stick_towards(w, o.actor.world_pos, SLOW))
            }
            2 => {
                if o.state_flag == OSSAN_STATE_START_CONVERSATION {
                    self.sub = 3;
                    return Some(idle);
                }
                if p.target_actor != Some(h) && p.action != PA::Talk {
                    self.sub = 1;
                    return Some(idle);
                }
                Some(self.press(BTN_A))
            }
            3 => {
                if o.state_flag == OSSAN_STATE_FACING_SHOPKEEPER {
                    self.sub = 4;
                    return Some(idle);
                }
                Some(if st == TEXT_STATE_EVENT { self.press(BTN_A) } else { idle })
            }
            4 => {
                if o.state_flag == OSSAN_STATE_LOOK_SHELF_RIGHT || o.state_flag == OSSAN_STATE_BROWSE_RIGHT_SHELF {
                    self.sub = 5;
                    return Some(idle);
                }
                // A fresh tilt right (EnOssan_UpdateJoystickInputState), once the choice is up.
                Some(if st == TEXT_STATE_CHOICE && self.prev.stick_x == 0 { PadState { stick_x: RUN as i8, ..Default::default() } } else { idle })
            }
            5 => {
                if o.state_flag == OSSAN_STATE_SELECT_ITEM {
                    self.sub = 6;
                    return Some(idle);
                }
                let on_shield = o.shelf_slots[o.cursor_index as usize].and_then(|s| w.actors.downcast::<EnGirlA>(s)).is_some_and(|g| g.actor.params == SI_DEKU_SHIELD);
                if o.state_flag == OSSAN_STATE_BROWSE_RIGHT_SHELF && o.draw_cursor != 0 && st == TEXT_STATE_EVENT {
                    if !on_shield {
                        self.failure = Some(format!("the cursor isn't on the Deku Shield (slot {})", o.cursor_index));
                        return None;
                    }
                    return Some(self.press(BTN_A));
                }
                Some(idle)
            }
            6 => {
                if o.state_flag == OSSAN_STATE_GIVE_ITEM_FANFARE {
                    self.sub = 7;
                    return Some(idle);
                }
                if o.state_flag != OSSAN_STATE_SELECT_ITEM {
                    self.failure = Some(format!("the shield couldn't be bought: shopkeeper state {}, text {:#x}, {} rupees", o.state_flag, w.msg_ctx.text_id, w.save.rupees));
                    return None;
                }
                Some(if st == TEXT_STATE_CHOICE && w.msg_ctx.choice_index == 0 { self.press(BTN_A) } else { idle })
            }
            7 => {
                if o.state_flag == OSSAN_STATE_CONTINUE_SHOPPING_PROMPT {
                    self.sub = 8;
                    return Some(idle);
                }
                Some(if Self::text_waits(w) { self.press(BTN_A) } else { idle })
            }
            8 => {
                if o.state_flag == OSSAN_STATE_IDLE {
                    self.sub = 9;
                    return Some(idle);
                }
                Some(if st == TEXT_STATE_CHOICE { self.press(BTN_B) } else { idle })
            }
            _ => {
                if st == TEXT_STATE_NONE && Self::settled(w) {
                    self.finish(Some(Step::Shield));
                    return None;
                }
                Some(idle)
            }
        }
    }

    fn find(w: &PlayState, who: Who) -> Option<ActorHandle> {
        w.actors.all().into_iter().find(|&h| match who {
            Who::Sign(home) => w.actors.downcast::<EnKanban>(h).is_some_and(|k| Self::xz_dist(k.actor.home_pos, home) < 1.0),
            Who::Kokiri(ty) => w.actors.downcast::<EnKo>(h).is_some_and(|k| k.actor.params & 0xFF == ty),
            Who::Mido => w.actors.downcast::<EnMd>(h).is_some(),
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
