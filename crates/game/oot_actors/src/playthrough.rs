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
//! **A Mad Scrub** (GAME-05 milestone 3b, `Route::Scrub`, the `scrub` test and script) runs
//! inside the Deku Tree on the `deku-tree-inside` preset, from a debug start in room 4
//! (`SCRUB_START_ROOM`: `Room_RequestNewRoom` and `Room_FinishRoomChange`, then Link placed at
//! `SCRUB_START`), 250 in front of the `En_Dekunuts` at (-74, -880, 1046) and facing it. Link holds
//! the guard (R) until the scrub's nut, bounced back off the Deku Shield, knocks it out of its
//! flower (`Step::NutBounced`: `EnDekunuts_SetupBeginRun`); then he locks on (Z) and runs it down,
//! slashing (B) once it's within reach (`Step::ScrubCaught`: `EnDekunuts_BeDamaged`), until it
//! dies (`Step::ScrubKilled`: gone, its drop from table 3, if any, picked up).
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
    /// A Mad Scrub's nut bounced back off the shield and knocked it out (`EnDekunuts_BeginRun`).
    NutBounced,
    /// The Mad Scrub caught and slashed (`EnDekunuts_BeDamaged`).
    ScrubCaught,
    /// The Mad Scrub dead and gone (`EnDekunuts_Die` over), its drop, if any, picked up.
    ScrubKilled,
    /// Room 0's top-floor switch pressed (`Obj_Switch` 0x2700: flag 0x27 set).
    SwitchPressed,
    /// The web over room 10's door burnt away (`Bg_Ydan_Sp` 0x19CA gone), Link free again
    /// after the attention cameras.
    WebBurnt,
    /// Room 10's sliding door opened (`Door_Shutter`: Player walking through it,
    /// `Player_Action_80845CA4`).
    DoorOpened,
    /// Through it, the door shut and barred behind Link (room 10's enemies alive:
    /// `DoorShutter_WaitClear`), and Link free after his pause (`PLAYER_CSACTION_7`).
    DoorBarred,
    /// A Deku Stick in hand from C-Left (`heldItemAction` `PLAYER_IA_DEKU_STICK`, the change over).
    StickOut,
    /// The stick lit at a torch's flame (Player's `unk_860` 210, `Obj_Syokudai`).
    StickLit,
    /// Through a plain sliding door into the next room, the door shut behind Link and he free.
    ThroughDoor,
    /// Holding on to a push block (`Player_Action_8084B78C`, A at its wall).
    BlockGrabbed,
    /// Room 3's block pushed off the upper floor into the pit: `Obj_Makeoshihiki`'s flag 0x10 set
    /// (its draw, with `NA_SE_SY_TRE_BOX_APPEAR`).
    BlockInPit,
    /// Down in the pit beside the block, climbed onto it: standing on its top.
    OnBlock,
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
            Step::NutBounced => "nut_bounced",
            Step::ScrubCaught => "scrub_caught",
            Step::ScrubKilled => "scrub_killed",
            Step::SwitchPressed => "switch_pressed",
            Step::WebBurnt => "web_burnt",
            Step::DoorOpened => "door_opened",
            Step::DoorBarred => "door_barred",
            Step::StickOut => "stick_out",
            Step::StickLit => "stick_lit",
            Step::ThroughDoor => "through_door",
            Step::BlockGrabbed => "block_grabbed",
            Step::BlockInPit => "block_in_pit",
            Step::OnBlock => "on_block",
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
    /// Inside the Deku Tree, a Mad Scrub's nut bounced back off the shield, the scrub caught and
    /// killed (GAME-05 milestone 3b), from a debug start in room 4 (`SCRUB_START`).
    Scrub,
    /// Inside the Deku Tree, room 0's top-floor switch pressed, the web over room 10's door
    /// burnt, and through the door, which bars behind Link (GAME-05 milestone 4a), from a debug
    /// start on the top floor (`SHUTTER_START`).
    Shutter,
    /// Inside the Deku Tree, a Deku Stick out from C-Left, lit at the middle floor's golden
    /// torch, round the floor and across its gap, the web over room 1's door burnt with it, and
    /// through the door into room 1 (GAME-05 milestone 4b), from a debug start by the torch
    /// with ten sticks on C-Left (`deku-tree-sticks`) and the torches lit by flag 0x27
    /// (`STICK_START`).
    Stick,
    /// Inside the Deku Tree, room 3's push block (`Obj_Oshihiki`, spawned by `Obj_Makeoshihiki`)
    /// pushed along the upper floor's channel and off its end into the pit (flag 0x10, the
    /// chime), then down into the pit beside it and up onto it (GAME-05 milestone 4c), from a
    /// debug start on the upper floor behind the block (`PUSH_START`).
    Push,
}

/// The `Push` route's block: room 3's `Obj_Oshihiki` (params 0xFFC0, small) on the upper floor,
/// where `Obj_Makeoshihiki` (0xFF10, `home.rot.z` 1) puts it with flag 0x10 clear; where it ends
/// up, down in the pit (`sBlocks[1]`); and the flag.
pub const PUSH_BLOCK_HOME: Vec3 = Vec3::new(-605.0, -820.0, -290.0);
pub const PUSH_BLOCK_PIT: Vec3 = Vec3::new(-365.0, -905.0, -290.0);
pub const PUSH_BLOCK_FLAG: i32 = 0x10;
/// Where the route starts Link: on the upper floor 95 behind the block, facing it (+x). Navi's
/// hint there (`Elf_Msg` 0x3208: text 0x108, within 80 of (-600, -291); gone with flag 0x10)
/// calls once he's closer.
pub const PUSH_START: (Vec3, i16) = (Vec3::new(-700.0, -810.0, -290.0), 0x4000);
/// The debug starts of GAME-05 milestone 4c (`game-push.bat`, `deku-tree-inside`): `(room,
/// position, yaw, name)`. `oot_actors --test debug_starts` checks each: Link stands, and the room
/// doesn't change.
/// - `room3`: on the upper floor behind room 3's push block, facing it: `Route::Push`'s start;
/// - `room7`: room 7 by its gravestones (`Bg_Haka`) and the Song of Time's hidden stair
///   (`Obj_Timeblock` 0x39FF), facing -z towards them;
/// - `room2`: room 2's 480 floor, west of its four hidden Song of Time blocks and under the ledge
///   (656) with the three rocks (`Obj_Bombiwa`), facing +x.
pub const PUSH_STARTS: [(i8, Vec3, i16, &str); 3] =
    [(3, PUSH_START.0, PUSH_START.1, "room3"), (7, Vec3::new(-1925.0, -760.0, 360.0), i16::MIN, "room7"), (2, Vec3::new(-1290.0, 480.0, 1440.0), 0x4000, "room2")];
/// From the channel's end, out of it onto the upper floor's -z side, then east off the floor's
/// edge (x -395) down into the pit (y -905) beside the block, and round to its south, in line
/// with its middle (the block's south face is at z -260, x -395 to -335).
pub const PUSH_PIT_PATH: [Vec3; 3] = [Vec3::new(-430.0, -810.0, -230.0), Vec3::new(-380.0, -905.0, -200.0), Vec3::new(-365.0, -905.0, -175.0)];

/// The `Stick` route's torch: room 0's middle-floor golden torch (`Obj_Syokudai` 0x03E7, lit by
/// flag 0x27), and the flag, set by the debug start as if the top floor's switch were pressed.
pub const STICK_TORCH_HOME: Vec3 = Vec3::new(400.0, 360.0, 121.0);
pub const STICK_TORCH_FLAG: i32 = 0x27;
/// Where the route starts Link: on the middle floor 70 across from the torch, facing it.
pub const STICK_START: (Vec3, i16) = (Vec3::new(330.0, 360.0, 100.0), 0x4000);
/// The web over room 1's door: `Bg_Ydan_Sp` 0x1FD6, facing the room's middle.
pub const STICK_WEB_HOME: Vec3 = Vec3::new(-388.0, 400.0, 389.0);
/// Room 1's door (transition 0, at (-455, 400, 455)), and where Link lines up in front of it on
/// room 0's side.
pub const STICK_DOOR: usize = 0;
pub const STICK_DOOR_FRONT: Vec3 = Vec3::new(-425.0, 400.0, 425.0);
/// The middle floor's walkway (y 360 by the torch, rising to 400 at its north end) round to the
/// gap before the door's ledge, and over it (from 352 to 338 degrees about the room's middle,
/// 107 across at the same height: Link jumps it at a run), then into the door's alcove.
pub const STICK_PATH: [Vec3; 6] = [
    Vec3::new(320.0, 360.0, 185.0),
    Vec3::new(185.0, 360.0, 320.0),
    Vec3::new(64.0, 387.0, 364.0),
    Vec3::new(-14.0, 400.0, 400.0),
    Vec3::new(-175.0, 400.0, 360.0),
    Vec3::new(-300.0, 400.0, 300.0),
];

/// The `Shutter` route's switch: room 0's top-floor `Obj_Switch` (params 0x2700), and its flag.
pub const SHUTTER_SWITCH_HOME: Vec3 = Vec3::new(-311.0, 800.0, -311.0);
pub const SHUTTER_SWITCH_FLAG: i32 = 0x27;
/// The web the flag burns: `Bg_Ydan_Sp` 0x19CA, over room 10's door.
pub const SHUTTER_WEB_HOME: Vec3 = Vec3::new(-491.0, 800.0, -1.0);
/// Room 10's door (transition 6, at (-560, 800, 0) facing -x), and where Link lines up in
/// front of it on room 0's side.
pub const SHUTTER_DOOR: usize = 6;
pub const SHUTTER_DOOR_FRONT: Vec3 = Vec3::new(-525.0, 800.0, 0.0);
/// Where the route starts Link: on the top floor 100 from the switch, facing it.
pub const SHUTTER_START: (Vec3, i16) = (Vec3::new(-382.0, 800.0, -241.0), -0x6000);

/// The `Scrub` route's Mad Scrub: room 4's `En_Dekunuts` (params 0xFF00), facing -z.
pub const SCRUB_HOME: Vec3 = Vec3::new(-74.0, -880.0, 1046.0);
/// Its room, and where the route starts Link: 250 in front of the scrub, facing it (yaw 0: +z).
pub const SCRUB_START_ROOM: i8 = 4;
pub const SCRUB_START: (Vec3, i16) = (Vec3::new(-74.0, -880.0, 796.0), 0);

/// A debug start in each room of the Deku Tree (MQ), `(room, position, yaw, name)`: on the
/// room's floor, clear of its holes (GAME-05 milestone 4a). The game's and the sandbox's
/// `--room`/`--at` take the same values (`game-dungeon.bat`), and
/// `oot_actors --test debug_starts` checks each: Link stands, and the room doesn't change.
/// Room 11 has none: past room 9's door, its whole floor is exit 2, the drop into Gohma's room.
pub const DEKU_TREE_ROOM_STARTS: [(i8, Vec3, i16, &str); 12] = [
    (0, Vec3::new(0.0, 0.0, 480.0), -0x8000, "lobby"),
    (0, Vec3::new(-420.0, 800.0, 60.0), -0x4000, "lobby-top"),
    (1, Vec3::new(-700.0, 400.0, 760.0), 0, "room1"),
    (2, Vec3::new(-1100.0, 280.0, 1150.0), 0, "room2"),
    (3, Vec3::new(-718.0, -820.0, 177.0), 8202, "room3"),
    (4, Vec3::new(-74.0, -880.0, 796.0), 0, "room4"),
    (5, Vec3::new(-1197.0, -880.0, 1079.0), -0x4000, "room5"),
    (6, Vec3::new(-1860.0, -760.0, 900.0), 0, "room6"),
    (7, Vec3::new(-1900.0, -760.0, 500.0), 0, "room7"),
    (8, Vec3::new(-2550.0, -760.0, -480.0), 0, "room8"),
    (9, Vec3::new(-660.0, -1880.0, -620.0), -0x8000, "room9"),
    (10, Vec3::new(-700.0, 800.0, 100.0), 0, "room10"),
];

/// The debug starts with Deku Sticks (GAME-05 milestone 4b, `game-sticks.bat`, the
/// `deku-tree-sticks` preset): `(room, position, yaw, name, switch flags set after Play_Init)`.
/// `oot_actors --test debug_starts` checks each: Link stands, and the room doesn't change.
/// - `torch`: room 0's middle floor by its golden torch, lit (0x27, as if the top floor's switch
///   were pressed): `Route::Stick`'s start;
/// - `room3`: by room 3's golden torch, lit (0x02, its floor switch's), the door to room 4 open
///   (0x15, its eye switch's): fire to carry to room 4's timed torches;
/// - `room10`: by room 10's wooden torch (always lit), its timed torch and the floor switch of the
///   rising platforms below;
/// - `room5`: room 5's own start, by the spiked log and the floating block;
/// - `room2`: on room 2's lift (`Obj_Lift` 0x0080, its top 18 above its home: it starts shaking at
///   once).
pub const STICK_STARTS: [(i8, Vec3, i16, &str, &[i32]); 5] = [
    (0, STICK_START.0, STICK_START.1, "torch", &[STICK_TORCH_FLAG]),
    (3, Vec3::new(-102.0, -880.0, 330.0), -0x8000, "room3", &[0x02, 0x15]),
    (10, Vec3::new(-700.0, 800.0, -60.0), -0x4000, "room10", &[]),
    (5, Vec3::new(-1197.0, -880.0, 1079.0), -0x4000, "room5", &[]),
    (2, Vec3::new(-1214.0, 408.0, 1208.0), 0, "room2", &[]),
];

/// A debug start in `room` of the scene `w` was just entered in: the room requested
/// (`Room_RequestNewRoom`), a frame for it to load, the change finished
/// (`Room_FinishRoomChange`), then Link placed at `pos` facing `yaw` (what `Route::debug_start`
/// and the game's `--room`/`--at` do).
pub fn deku_tree_room_start(w: &mut PlayState, room: i8, pos: Vec3, yaw: i16) {
    if room != w.room_ctx.cur.num && w.room_request(room) {
        w.tick_with(oot_game::play::scripted_input(PadState::default(), PadState::default()));
        w.room_change_done();
    }
    w.place_player(pos, yaw);
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
            Route::DekuBaba | Route::Combat | Route::Scrub | Route::Shutter | Route::Stick | Route::Push => "ENTR_DEKU_TREE_0",
            _ => "ENTR_LINKS_HOUSE_0",
        }
    }

    /// The save preset it needs, if any.
    pub fn preset(self) -> Option<&'static str> {
        match self {
            Route::DekuTree => Some("deku-tree-open"),
            Route::DekuBaba | Route::Combat | Route::Scrub | Route::Shutter | Route::Push => Some("deku-tree-inside"),
            Route::Stick => Some("deku-tree-sticks"),
            Route::SwordChest | Route::MidoShop | Route::NewSaveDekuTree | Route::NewFileDekuTree => None,
        }
    }

    /// Where Link starts instead of the entrance's spawn (a debug start), if anywhere.
    pub fn start(self) -> Option<(Vec3, i16)> {
        match self {
            Route::DekuBaba => Some(DEKU_BABA_START),
            Route::Combat => Some(COMBAT_START),
            Route::Scrub => Some(SCRUB_START),
            Route::Shutter => Some(SHUTTER_START),
            Route::Stick => Some(STICK_START),
            Route::Push => Some(PUSH_START),
            _ => None,
        }
    }

    /// The switch flags a debug start sets after `Play_Init` (the game's `--switch`).
    pub fn start_switches(self) -> &'static [i32] {
        match self {
            Route::Stick => &[STICK_TORCH_FLAG],
            _ => &[],
        }
    }

    /// The room a debug start changes to after `Play_Init`, if any.
    pub fn start_room(self) -> Option<i8> {
        match self {
            Route::Scrub => Some(SCRUB_START_ROOM),
            Route::Push => Some(3),
            _ => None,
        }
    }

    /// The debug start on a play state just entered at `entrance()`: the start's room
    /// (`Room_RequestNewRoom`, a frame for it to load, `Room_FinishRoomChange`), then Link placed.
    pub fn debug_start(self, w: &mut PlayState) {
        if let Some(room) = self.start_room()
            && w.room_request(room)
        {
            w.tick_with(oot_game::play::scripted_input(PadState::default(), PadState::default()));
            w.room_change_done();
        }
        for &flag in self.start_switches() {
            w.flags.set_switch(flag);
        }
        if let Some((p, y)) = self.start() {
            w.place_player(p, y);
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
            Route::Scrub => "scrub",
            Route::Shutter => "shutter",
            Route::Stick => "stick",
            Route::Push => "push",
        }
    }

    /// A cap on the route's length.
    pub fn max_frames(self) -> usize {
        match self {
            Route::MidoShop => 12000,
            Route::NewSaveDekuTree => 16000,
            Route::NewFileDekuTree => 24000,
            Route::Combat => 9000,
            Route::Scrub => 3000,
            Route::Shutter => 3000,
            Route::Stick => 3000,
            Route::Push => 3000,
            _ => Playthrough::MAX_FRAMES,
        }
    }

    /// The route a sandbox script names.
    pub fn from_script(name: &str) -> Option<Route> {
        [Route::DekuTree, Route::SwordChest, Route::MidoShop, Route::NewSaveDekuTree, Route::NewFileDekuTree, Route::DekuBaba, Route::Combat, Route::Scrub, Route::Shutter, Route::Stick, Route::Push].into_iter().find(|r| r.script() == name)
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
    /// The guard up until the Mad Scrub whose home is here is knocked out by its own nut
    /// (`bounce_nut`).
    BounceNut(Vec3),
    /// Catch that Mad Scrub and kill it, and pick up its drop (`catch_scrub`).
    CatchScrub(Vec3),
    /// Onto the floor switch whose home is here until its flag is set.
    PressSwitch(Vec3, i32, Step),
    /// Idle until the web whose home is here has burnt away and Link is free.
    WaitWebBurnt(Vec3, Step),
    /// Open the sliding door from this transition entry: walk to the point in front of it, at it
    /// until it offers (Player's `doorType` `PLAYER_DOORTYPE_SLIDING`), A, until Link walks
    /// through.
    OpenSlidingDoor(usize, Vec3, Step),
    /// Idle until the door from this transition entry is shut behind Link and he's free.
    WaitDoorShut(usize, Step),
    /// Idle until the main camera is the active one again (the attention cameras over).
    WaitCameras,
    /// C-Left until a Deku Stick is in hand and the change is over.
    TakeStick(Step),
    /// At the torch whose home is here, slowly, until the stick in hand catches fire.
    LightStick(Vec3, Step),
    /// At the web whose home is here, slowly, the stick burning, until it's burnt away (its
    /// one-point cutscene over) and Link is free.
    BurnWeb(Vec3, Step),
    /// Hold on to the push block whose home is here: at it until "Grab" (`PLAYER_STATE2_0`), then
    /// A held, standing, until Link holds on (`Player_Action_8084B78C`).
    GrabBlock(Vec3, Step),
    /// A held and the stick forward until this switch flag is set, holding on again if he lets
    /// go (a text that opens is read first).
    PushBlock(Vec3, i32, Step),
    /// At the push block (from beside it, lower down) until Link stands on its top.
    ClimbBlock(Step),
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
            Route::Scrub => vec![Task::BounceNut(SCRUB_HOME), Task::CatchScrub(SCRUB_HOME)],
            Route::Shutter => vec![
                Task::PressSwitch(SHUTTER_SWITCH_HOME, SHUTTER_SWITCH_FLAG, Step::SwitchPressed),
                Task::WaitWebBurnt(SHUTTER_WEB_HOME, Step::WebBurnt),
                Task::OpenSlidingDoor(SHUTTER_DOOR, SHUTTER_DOOR_FRONT, Step::DoorOpened),
                Task::WaitDoorShut(SHUTTER_DOOR, Step::DoorBarred),
            ],
            Route::Stick => vec![
                // The golden torches light with their attention cameras as the flag goes on.
                Task::WaitCameras,
                Task::TakeStick(Step::StickOut),
                Task::LightStick(STICK_TORCH_HOME, Step::StickLit),
                Task::Hurry(STICK_PATH.to_vec()),
                Task::BurnWeb(STICK_WEB_HOME, Step::WebBurnt),
                Task::OpenSlidingDoor(STICK_DOOR, STICK_DOOR_FRONT, Step::DoorOpened),
                Task::WaitDoorShut(STICK_DOOR, Step::ThroughDoor),
            ],
            Route::Push => vec![
                Task::GrabBlock(PUSH_BLOCK_HOME, Step::BlockGrabbed),
                Task::PushBlock(PUSH_BLOCK_HOME, PUSH_BLOCK_FLAG, Step::BlockInPit),
                // Link lets go once the block has dropped away from his hands.
                Task::Settle(None),
                Task::Walk(PUSH_PIT_PATH.to_vec()),
                Task::ClimbBlock(Step::OnBlock),
            ],
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
            Task::BounceNut(home) => self.bounce_nut(w, home),
            Task::CatchScrub(home) => self.catch_scrub(w, home),
            Task::PressSwitch(home, flag, step) => self.press_switch(w, home, flag, step),
            Task::WaitWebBurnt(home, step) => self.wait_web_burnt(w, home, step),
            Task::OpenSlidingDoor(index, front, step) => self.open_sliding_door(w, index, front, step),
            Task::WaitDoorShut(index, step) => self.wait_door_shut(w, index, step),
            Task::WaitCameras => self.wait_cameras(w),
            Task::TakeStick(step) => self.take_stick(w, step),
            Task::LightStick(home, step) => self.light_stick(w, home, step),
            Task::BurnWeb(home, step) => self.burn_web(w, home, step),
            Task::GrabBlock(home, step) => self.grab_block(w, home, step),
            Task::PushBlock(home, flag, step) => self.push_block(w, home, flag, step),
            Task::ClimbBlock(step) => self.climb_block(w, step),
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
                        // (Sidestep too: the stick towards a Keese off to the side starts a
                        // sidestep, which the next frame's stick has to keep going.)
                        if k.action == KA::Stay && matches!(p.action, PA::TargetIdle | PA::TargetRun | PA::Sidestep | PA::GuardHit | PA::StandingStill) {
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

    /// The `En_Dekunuts` (not its flower) whose home is `home`, if it's still there.
    fn mad_scrub_at(w: &PlayState, home: Vec3) -> Option<(ActorHandle, &crate::en_dekunuts::EnDekunuts)> {
        w.actors.all().into_iter().find_map(|h| w.actors.downcast::<crate::en_dekunuts::EnDekunuts>(h).filter(|n| n.actor.home_pos.distance(home) < 1.0 && n.actor.params != crate::en_dekunuts::DEKUNUTS_FLOWER && !n.actor.killed).map(|n| (h, n)))
    }

    /// `Task::BounceNut`: the guard (R, nothing locked on) until the scrub's nut has bounced off
    /// the Deku Shield and knocked it out of its flower (`EnDekunuts_BeginRun`, `Step::NutBounced`).
    /// `Task::PressSwitch`: towards the switch's middle until its flag is set.
    fn press_switch(&mut self, w: &PlayState, home: Vec3, flag: i32, step: Step) -> Option<PadState> {
        if w.flags.get_switch(flag) {
            self.finish(Some(step));
            return None;
        }
        self.wait += 1;
        if self.wait > 300 {
            self.failure = Some(format!("the switch at {home} never pressed"));
            return None;
        }
        Some(stick_towards(w, home, SLOW))
    }

    /// `Task::WaitWebBurnt`: idle until no web is at `home` and Link stands free (the
    /// attention cameras over).
    fn wait_web_burnt(&mut self, w: &PlayState, home: Vec3, step: Step) -> Option<PadState> {
        let web = w.actors.all().into_iter().any(|h| w.actors.downcast::<crate::bg_ydan_sp::BgYdanSp>(h).is_some_and(|s| s.actor.world_pos.distance(home) < 1.0));
        let p = w.player();
        if !web && p.cs_mode == 0 && Self::settled(w) {
            self.finish(Some(step));
            return None;
        }
        self.wait += 1;
        if self.wait > 600 {
            self.failure = Some(format!("the web at {home} never burnt (Link {:?}, cs {})", p.action, p.cs_mode));
            return None;
        }
        Some(PadState::default())
    }

    /// `Task::OpenSlidingDoor`'s phases in `sub`: 0 to the point in front, 1 at the door until it
    /// offers, then A until Link walks through.
    fn open_sliding_door(&mut self, w: &PlayState, index: usize, front: Vec3, step: Step) -> Option<PadState> {
        use crate::door_shutter::DoorShutter;
        let idle = PadState::default();
        self.wait += 1;
        if self.wait > 900 {
            self.failure = Some(format!("the door from transition {index} never opened (sub {})", self.sub));
            return None;
        }
        let Some(door) = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<DoorShutter>(h).filter(|d| d.transition_index() == index)) else {
            self.failure = Some(format!("no Door_Shutter from transition {index}"));
            return None;
        };
        let p = w.player();
        // A text that opens by itself (an Elf_Msg by the door calling Navi) holds Link until
        // it's read.
        if w.message_state() != TEXT_STATE_NONE {
            return Some(if Self::text_waits(w) { self.press(BTN_A) } else { idle });
        }
        if p.action == PA::ExitWalk {
            self.finish(Some(step));
            return None;
        }
        match self.sub {
            0 => {
                if Self::xz_dist(p.actor.world_pos, front) < 8.0 {
                    self.sub = 1;
                    return Some(idle);
                }
                Some(stick_towards(w, front, SLOW))
            }
            _ => {
                if p.door_type == crate::player::PLAYER_DOORTYPE_SLIDING {
                    return Some(self.press(BTN_A));
                }
                // Knocked off the line (a Keese): back in front of it.
                if Self::xz_dist(p.actor.world_pos, front) > 20.0 {
                    self.sub = 0;
                    return Some(idle);
                }
                // At the door, slowly: facing it, Link is offered to open it.
                Some(stick_towards(w, door.actor.world_pos, 30.0))
            }
        }
    }

    /// `Task::WaitCameras`: idle until `activeCamId` is `CAM_ID_MAIN` and Link stands.
    fn wait_cameras(&mut self, w: &PlayState) -> Option<PadState> {
        if w.active_cam_id == oot_game::camera::CAM_ID_MAIN && Self::settled(w) {
            self.finish(None);
            return None;
        }
        self.wait += 1;
        if self.wait > 600 {
            self.failure = Some(format!("the cameras never came back to the main one ({})", w.active_cam_id));
            return None;
        }
        Some(PadState::default())
    }

    /// `Task::TakeStick`: C-Left (pressed every other frame) until the stick is in hand
    /// (`heldItemAction`) and the change animation is over.
    fn take_stick(&mut self, w: &PlayState, step: Step) -> Option<PadState> {
        let p = w.player();
        if p.held_item_ap == crate::player::PLAYER_IA_DEKU_STICK && p.upper != crate::player::UpperAction::Change {
            self.finish(Some(step));
            return None;
        }
        self.wait += 1;
        if self.wait > 120 {
            self.failure = Some(format!("no Deku Stick from C-Left (held {}, C-Left {:#x})", p.held_item_ap, w.save.equips.button_items[1]));
            return None;
        }
        if p.held_item_id == oot_game::item::ITEM_DEKU_STICK { Some(PadState::default()) } else { Some(self.press(eng_input::pad::BTN_CLEFT)) }
    }

    /// `Task::LightStick`: until the stick's `unk_860` is set (it caught fire), Link steered so the
    /// stick's tip (held out to his right) comes to the flame, 67 above the torch's home: towards
    /// the flame less the tip's offset from Link, slowly, then still. A text that opens (Navi's
    /// hint by the torch) is read.
    fn light_stick(&mut self, w: &PlayState, home: Vec3, step: Step) -> Option<PadState> {
        let idle = PadState::default();
        let p = w.player();
        if p.held_item_ap == crate::player::PLAYER_IA_DEKU_STICK && p.unk_860 != 0 {
            self.finish(Some(step));
            return None;
        }
        self.wait += 1;
        if self.wait > 400 {
            self.failure = Some(format!("the stick never caught fire at {home} (tip {:?}, Link {:?})", p.melee_weapon_info[0].tip, p.actor.world_pos));
            return None;
        }
        if w.message_state() != TEXT_STATE_NONE {
            return Some(if Self::text_waits(w) { self.press(BTN_A) } else { idle });
        }
        let link = p.actor.world_pos;
        let to = home - (p.melee_weapon_info[0].tip - link);
        if Self::xz_dist(link, to) < 3.0 {
            return Some(idle);
        }
        Some(stick_towards(w, to, if Self::xz_dist(link, to) < 15.0 { 30.0 } else { SLOW }))
    }

    /// `Task::BurnWeb`: at the web slowly until it's gone (`Bg_Ydan_Sp` burnt away), any text
    /// read, the cameras back and Link free.
    fn burn_web(&mut self, w: &PlayState, home: Vec3, step: Step) -> Option<PadState> {
        let idle = PadState::default();
        let web = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<crate::bg_ydan_sp::BgYdanSp>(h).filter(|s| s.actor.home_pos.distance(home) < 1.0 || s.actor.world_pos.distance(home) < 1.0));
        self.wait += 1;
        if self.wait > 600 {
            let p = w.player();
            self.failure = Some(format!("the web at {home} never burnt (stick {} {}, tip {:?})", p.held_item_ap, p.unk_860, p.melee_weapon_info[0].tip));
            return None;
        }
        if w.message_state() != TEXT_STATE_NONE {
            return Some(if Self::text_waits(w) { self.press(BTN_A) } else { idle });
        }
        match web {
            Some(_) if w.active_cam_id == oot_game::camera::CAM_ID_MAIN => Some(stick_towards(w, home, SLOW)),
            Some(_) => Some(idle),
            None => {
                if w.active_cam_id == oot_game::camera::CAM_ID_MAIN && Self::settled(w) {
                    self.finish(Some(step));
                    return None;
                }
                Some(idle)
            }
        }
    }

    /// The push block (`Obj_Oshihiki`) whose start (its spawn place) is `home`, or the only one.
    fn push_block_actor(w: &PlayState, home: Vec3) -> Option<&crate::obj_oshihiki::ObjOshihiki> {
        let blocks: Vec<&crate::obj_oshihiki::ObjOshihiki> = w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<crate::obj_oshihiki::ObjOshihiki>(h)).collect();
        let n = blocks.len();
        blocks.into_iter().find(|b| n == 1 || b.actor.world_pos.distance(home) < 1.0)
    }

    /// Whether Link holds on to a wall to push or pull.
    fn holding_on(w: &PlayState) -> bool {
        matches!(w.player().action, PA::PushWait | PA::Push | PA::Pull)
    }

    /// The pad that gets Link to hold on to the block: at it, slowly, until "Grab"
    /// (`PLAYER_STATE2_0`: at its wall, facing it), then still with A held (A held when he's
    /// already still: a press while moving would roll).
    fn grab_pad(&mut self, w: &PlayState, block: Vec3) -> PadState {
        let p = w.player();
        if p.state2 & crate::player::STATE2_0 != 0 && p.linear_velocity == 0.0 {
            return PadState { button: BTN_A, ..Default::default() };
        }
        if p.state2 & crate::player::STATE2_0 != 0 {
            return PadState::default();
        }
        stick_towards(w, block, SLOW)
    }

    /// `Task::GrabBlock`.
    fn grab_block(&mut self, w: &PlayState, home: Vec3, step: Step) -> Option<PadState> {
        if Self::holding_on(w) {
            self.finish(Some(step));
            return None;
        }
        self.wait += 1;
        // Navi's hint by the block (text 0x108) is read on the way: several pages.
        if self.wait > 1200 {
            let p = w.player();
            self.failure = Some(format!("never held on to the block at {home} (Link {:?} at {:?}, state2 {:#x})", p.action, p.actor.world_pos, p.state2));
            return None;
        }
        if w.message_state() != TEXT_STATE_NONE {
            return Some(if Self::text_waits(w) { self.press(BTN_A) } else { PadState::default() });
        }
        let Some(b) = Self::push_block_actor(w, home) else {
            self.failure = Some(format!("no push block at {home}"));
            return None;
        };
        let at = b.actor.world_pos;
        Some(self.grab_pad(w, at))
    }

    /// `Task::PushBlock`: holding on, A and the stick towards the block (along Link's facing:
    /// `func_8083FFB8` takes the stick's part along it), until the flag is set; let go (a text,
    /// the block stopped by a wall), he holds on again.
    fn push_block(&mut self, w: &PlayState, home: Vec3, flag: i32, step: Step) -> Option<PadState> {
        if w.flags.get_switch(flag) {
            self.finish(Some(step));
            return None;
        }
        self.wait += 1;
        if self.wait > 900 {
            let p = w.player();
            self.failure = Some(format!("flag {flag:#x} never set (Link {:?} at {:?})", p.action, p.actor.world_pos));
            return None;
        }
        if w.message_state() != TEXT_STATE_NONE {
            return Some(if Self::text_waits(w) { self.press(BTN_A) } else { PadState::default() });
        }
        let Some(b) = Self::push_block_actor(w, home) else {
            self.failure = Some(format!("no push block at {home}"));
            return None;
        };
        let at = b.actor.world_pos;
        if Self::holding_on(w) {
            let p = w.player();
            let ahead = p.actor.world_pos + Vec3::new(eng_math::sin_s(p.actor.shape_rot.y), 0.0, eng_math::cos_s(p.actor.shape_rot.y)) * 100.0;
            let mut pad = stick_towards(w, ahead, RUN);
            pad.button = BTN_A;
            return Some(pad);
        }
        Some(self.grab_pad(w, at))
    }

    /// `Task::ClimbBlock`: at the block (its top's middle) until Link stands on it, settled.
    fn climb_block(&mut self, w: &PlayState, step: Step) -> Option<PadState> {
        let Some(b) = Self::push_block_actor(w, PUSH_BLOCK_PIT) else {
            self.failure = Some("no push block".into());
            return None;
        };
        let p = w.player();
        if p.actor.floor_bg_id == b.bg && Self::settled(w) {
            self.finish(Some(step));
            return None;
        }
        self.wait += 1;
        if self.wait > 300 {
            self.failure = Some(format!("never onto the block (Link {:?} at {:?}, floor bg {})", p.action, p.actor.world_pos, p.actor.floor_bg_id));
            return None;
        }
        if w.message_state() != TEXT_STATE_NONE {
            return Some(if Self::text_waits(w) { self.press(BTN_A) } else { PadState::default() });
        }
        if p.actor.floor_bg_id == b.bg {
            return Some(PadState::default());
        }
        // Straight at its south face. From the pit's floor it's 60 up, a child's `unk_14` or more:
        // after 6 frames pressed on it `Player_ActionHandler_12` jumps and grabs its edge
        // (`gPlayerAnim_link_normal_250jump_start`), and the hang climbs up with the stick past 55
        // (`controlStickSpinAngles`, past the dead zone): at full tilt.
        Some(stick_towards(w, b.actor.world_pos, FULL))
    }

    /// `Task::WaitDoorShut`: idle until the door is down and Link stands free.
    fn wait_door_shut(&mut self, w: &PlayState, index: usize, step: Step) -> Option<PadState> {
        use crate::door_shutter::{Action as DA, DoorShutter};
        let Some(door) = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<DoorShutter>(h).filter(|d| d.transition_index() == index)) else {
            self.failure = Some(format!("no Door_Shutter from transition {index}"));
            return None;
        };
        let shut = !matches!(door.action, DA::Open | DA::Close | DA::JabuDoorClose | DA::WaitPlayerSurprised);
        let p = w.player();
        if shut && p.cs_mode == 0 && Self::settled(w) {
            self.finish(Some(step));
            return None;
        }
        self.wait += 1;
        if self.wait > 600 {
            self.failure = Some(format!("the door from transition {index} never shut ({:?}; Link {:?}, cs {})", door.action, p.action, p.cs_mode));
            return None;
        }
        Some(PadState::default())
    }

    fn bounce_nut(&mut self, w: &PlayState, home: Vec3) -> Option<PadState> {
        use crate::en_dekunuts::Action as NA;
        use eng_input::pad::BTN_R;
        let idle = PadState::default();
        self.wait += 1;
        let Some((_, n)) = Self::mad_scrub_at(w, home) else {
            // Its room's actors spawn in the first frames.
            if self.wait > 60 {
                self.failure = Some(format!("no Mad Scrub at {home}"));
                return None;
            }
            return Some(idle);
        };
        if n.action == NA::BeginRun {
            self.finish(Some(Step::NutBounced));
            return None;
        }
        if self.wait > 600 {
            self.failure = Some(format!("the Mad Scrub's nut didn't come back: {:?}", n.action));
            return None;
        }
        Some(PadState { button: BTN_R, ..idle })
    }

    /// `Task::CatchScrub`'s phases in `sub`:
    /// - 0: Z on every other frame until Link is locked on to the scrub (`focusActor`);
    /// - 1: locked on (Z), after it at a run; within 45 across, B; once it's hit
    ///   (`EnDekunuts_BeDamaged`), `Step::ScrubCaught`; a slash that misses, after it again;
    /// - 2: until it's gone (`EnDekunuts_Die` over), noting its drop (`Item_DropCollectibleRandom`);
    /// - 3: onto the drop until it's collected, then `Step::ScrubKilled`.
    fn catch_scrub(&mut self, w: &PlayState, home: Vec3) -> Option<PadState> {
        use crate::en_dekunuts::Action as NA;
        use eng_input::pad::BTN_Z;
        let idle = PadState::default();
        self.wait += 1;
        if self.wait > 2400 {
            self.failure = Some(format!("the Mad Scrub's chase stalled in phase {}", self.sub));
            return None;
        }
        let z = |mut p: PadState| {
            p.button |= BTN_Z;
            p
        };
        match self.sub {
            0 | 1 => {
                let Some((h, n)) = Self::mad_scrub_at(w, home) else {
                    self.failure = Some("the Mad Scrub went before it was caught".into());
                    return None;
                };
                if matches!(n.action, NA::BeDamaged | NA::Die) {
                    self.steps.push((Step::ScrubCaught, self.frame));
                    self.done = Some(Step::ScrubCaught);
                    self.sub = 2;
                    self.items = w.actors.all().into_iter().filter(|&x| w.actors.downcast::<EnItem00>(x).is_some()).collect();
                    return Some(z(idle));
                }
                let p = w.player();
                if self.sub == 0 {
                    if p.focus_actor == Some(h) {
                        self.sub = 1;
                        return Some(z(idle));
                    }
                    return Some(if self.prev.button & BTN_Z == 0 { z(idle) } else { idle });
                }
                if p.focus_actor != Some(h) {
                    self.sub = 0;
                    return Some(idle);
                }
                if Self::xz_dist(p.actor.world_pos, n.actor.world_pos) < 45.0 && p.action != PA::Attack {
                    return Some(z(self.press(BTN_B)));
                }
                Some(z(stick_towards(w, n.actor.world_pos, RUN)))
            }
            2 => {
                if Self::mad_scrub_at(w, home).is_some() {
                    return Some(idle);
                }
                self.dropped = w.actors.all().into_iter().find(|h| !self.items.contains(h) && w.actors.downcast::<EnItem00>(*h).is_some());
                self.drop = self.dropped.and_then(|h| w.actors.actor(h)).map(|a| a.params);
                if self.dropped.is_none() {
                    self.finish(Some(Step::ScrubKilled));
                    return None;
                }
                self.sub = 3;
                Some(idle)
            }
            _ => {
                let Some(it) = self.dropped.and_then(|h| w.actors.downcast::<EnItem00>(h)).filter(|i| i.action != ItemAction::Collected) else {
                    self.finish(Some(Step::ScrubKilled));
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
