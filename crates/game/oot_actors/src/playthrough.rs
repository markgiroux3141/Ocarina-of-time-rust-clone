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
//! floor it falls down the middle.) The Keese first (since GAME-07 milestone 1's culling volume:
//! fighting the Deku Baba under its perch sets it diving, and it pulls up over Link out of the
//! camera's view, where it holds still). Link walks at it until it dives (within 120 of its
//! perch), locks on to it and holds the shield up (R on the lock-on: `func_80834758`) until it
//! blocks a dive (`Step::Blocked`); the Keese hovers after (`EnFirefly_Stay`) and he slashes it
//! (B) until it dies; it shrinks away and drops from table 14 (`Step::KeeseKilled`, the drop in
//! `drop` if any, picked up). Then the Deku Baba, up by then: Z let go, Link turns to it, locks on
//! (Z: the battle camera, `Camera_Battle1`), slashes it while it's upright
//! (`Step::KarebabaKilled`), and picks up the Deku Stick it leaves (`Step::StickTaken`).
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
    /// The Kokiri Sword worn (the pause menu opened and closed: the equipment page's stand-in).
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
    /// The Kokiri Sword and the Deku Shield worn (the pause menu opened and closed: the equipment
    /// page's stand-in).
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
    /// The Fairy Slingshot drawn with a seed in first person (`func_808351D4`'s wait, `heldActor`
    /// set, `Player_Action_8084B1D8`).
    SlingshotDrawn,
    /// A seed shot into an eye switch: its flag set (`Obj_Switch`'s eye, closed).
    EyeShot,
    /// The pause menu open (`PAUSE_STATE_MAIN`, idle).
    MenuOpened,
    /// The item page's cursor on the item to equip.
    CursorOnItem,
    /// The item on its C button (`KaleidoScope_UpdateItemEquip` over).
    ItemEquipped,
    /// The menu turned to the next page (`KaleidoScope_UpdatePageSwitch` over).
    PageTurned,
    /// The menu closed: the game resumed (`PAUSE_STATE_OFF`).
    MenuClosed,
    /// The dungeon map page's floor changed with the stick (`dungeonMapSlot`, its room maps
    /// loaded: `KaleidoScope_UpdateDungeonMap`).
    FloorChanged,
    /// Link dead: the game over started (`gameOverCtx.state` past `GAMEOVER_INACTIVE`).
    Died,
    /// "Would you like to save?": the game over's (`PAUSE_STATE_GAME_OVER_SAVE_PROMPT`), or the
    /// pause menu's turned in (`PAUSE_SAVE_PROMPT_STATE_WAIT_CHOICE`).
    SavePrompt,
    /// Its "Continue playing?" (`PAUSE_STATE_GAME_OVER_CONTINUE_PROMPT`), "No" taken at the save
    /// prompt.
    ContinuePrompt,
    /// "Yes" taken: the fade, the respawn at the dungeon's entrance, Link standing.
    Respawned,
    /// The pause menu's save prompt's "Yes" taken: the save written (`Sram_WriteSave`,
    /// `PAUSE_SAVE_PROMPT_STATE_SAVED`).
    Saved,
    /// After the console's reset, the file loaded (`FileSelect_LoadGame`, `Play_Init`) and Link
    /// standing.
    Loaded,
    /// At the boss room's entrance: Queen Gohma's intro started (`BossGoma_Encounter` state 1).
    GohmaEntered,
    /// The intro's first part over: Link free to look for her (state 3).
    GohmaWaiting,
    /// Looked at for 16 frames: her eye roll's cutscene (state 4).
    GohmaLookedAt,
    /// Her intro over: the fight (`BossGoma_FloorMain`, the camera back).
    GohmaBattle,
    /// A seed into her red eye: stunned (`BossGoma_FloorStunned`), the first time.
    GohmaStunned,
    /// A sword's hit while she's stunned (her health down), the first time.
    GohmaHit,
    /// A seed into her eye on the ceiling: knocked down (`BossGoma_FallStruckDown`), the first time.
    GohmaKnockedDown,
    /// Her health gone: `BossGoma_Defeated`.
    GohmaDefeated,
    /// Her death's cutscene over: she's gone, the camera back.
    GohmaGone,
    /// The heart container taken: a heart more (`Item_B_Heart`, `GI_HEART_CONTAINER_2`).
    HeartTaken,
    /// In the blue warp: its one-point cutscene, Link floating up (`DoorWarp1_ChildWarpOut`).
    WarpEntered,
    /// Out of the boss room: the scene changed, the emerald and its flags given.
    WarpedOut,
    /// Kokiri Forest by blue warp: Link landed (out of `Player_Action_BlueWarpArrive`).
    Arrived,
    /// The Deku Tree's emerald, part 1, over: its terminator's transition started.
    EmeraldPart1Over,
    /// In Queen Gohma's room (`ENTR_DEKU_TREE_BOSS_0`) by room 11's floor (exit 2), standing.
    BossRoom,
    /// The Kokiri Emerald drawn over Link (`Demo_Effect`'s jewel on its cue 2).
    EmeraldShown,
    /// The Deku Tree's death started (`CS_MISC_DEKU_TREE_DEATH`, the script's frame 782).
    TreeDeath,
    /// The emerald's last part over: its terminator's transition started.
    EmeraldOver,
    /// Kokiri Forest at `ENTR_KOKIRI_FOREST_11`: the scene changed, the fade over, Link standing.
    Forest11,
    /// The chain's parts, each on its scene's `Play_Init` in its cutscene layer: the castle town's
    /// cutscene map, layer 5 (Ganondorf on his horse); layer 4 (the goddesses); Gerudo Valley,
    /// layer 5 (Din) and 4 (the valley); Death Mountain Trail, layer 4 (Nayru); Kokiri Forest,
    /// layer 4 (Farore); the cutscene map, layer 6 (the Triforce); Kokiri Forest, layer 6 (the
    /// emerald).
    Ganondorf,
    Goddesses,
    Din,
    Valley,
    Nayru,
    Farore,
    Triforce,
    EmeraldPart9,
    /// Kokiri Forest's exit to the Lost Woods' bridge taken (the transition started).
    ForestLeft,
    /// The Lost Woods' layer 4 (`gLostWoodsFairyOcarinaCs`), entered: Saria's goodbye.
    Bridge,
    /// The Fairy Ocarina owned (`Cutscene_HandleConditionalTriggers`'s `Item_Give`) and its text
    /// (0x004A) shown.
    FairyOcarina,
    /// Hyrule Field entered at `ENTR_HYRULE_FIELD_3` (the bridge's terminator).
    Field,
    /// The field's intro (`gHyruleFieldIntroCs`) over: Link standing in Hyrule Field.
    FieldIntroOver,
    /// The owl's talk started (`En_Owl`, text 0x2064).
    OwlTalk,
    /// The owl flown off (gone, Link free).
    OwlFlown,
    /// The day's music stopped (`Environment_PlayTimeBasedSequence` past 17:10).
    MusicFaded,
    /// Night (`nightFlag`: past 18:00).
    Night,
    /// The drawbridge raised for the night (`Bg_Spot00_Hanebasi` at -0x4000).
    BridgeRaised,
    /// The night's critters (`TIMESEQ_NIGHT_CRITTERS` done: past 19:00).
    NightCritters,
    /// The Peahat killed (`EnPeehat_Adult_StateDie` at no health, then its explosion: gone).
    PeahatKilled,
    /// Castle Town's entrance entered over the drawbridge.
    CastleTown,
    /// The Stalchildren killed (`EnSkb_SetupDeath`), as many as the task asks.
    StalchildrenKilled,
    /// Kakariko Village entered up its stairs.
    Kakariko,
    /// The Stalchildren that followed Link killed or sunk: none left near him.
    StalchildrenGone,
    /// The market (`ENTR_MARKET_DAY_0`) entered from Castle Town's entrance, settled.
    Market,
    /// A townsman's text (`En_Hy`) read to the end and closed.
    Townsman,
    /// Malon's text (`En_Ma1`) read to the end and closed.
    Malon,
    /// The castle's road (`ENTR_HYRULE_CASTLE_0`) entered, its intro (`gHyruleCastleIntroCs`)
    /// over, settled.
    CastleRoad,
    /// Up a climbable wall (a vine) onto the ground at its top, settled.
    Vine,
    /// Talon shown the Cucco: awake (`EVENTCHKINF_TALON_WOKEN_IN_CASTLE`), the text closed.
    TalonWoken,
    /// Talon talked to awake (0x702C), its event read: his run home starts.
    TalonTalk,
    /// Talon's run home started (`EVENTCHKINF_TALON_RETURNED_FROM_CASTLE`) and over: gone.
    TalonGone,
    /// A crate (`Bg_Spot15_Rrbox`) pushed off the bank and landed in the moat.
    CrateInMoat,
    /// Out of the moat up onto its east bank (1571).
    MoatBank,
    /// Up onto the crates stacked in the moat.
    OnCrates,
    /// Through the drain's crawlspace, out at its far end.
    Drain,
    /// The guards' courtyard (`ENTR_CASTLE_COURTYARD_GUARDS_DAY_0`) entered, settled.
    Courtyard,
    /// Seen by a castle sentry (`En_Heishi3`): "Hey you!" (0x702D).
    Caught,
    /// Thrown out by the gate (`ENTR_HYRULE_CASTLE_4`), the circle opened, settled.
    ThrownOut,
    /// The ocarina out (`Player_Action_8084E3C4`).
    OcarinaOut,
    /// Saria's Song played: the staff knew it (`unk_E3F2`).
    SariasSong,
    /// Saria's talk through Navi (0xE0) read and closed.
    SariaTalk,
    /// The Sun's Song played.
    SunsSong,
    /// The Sun's Song's fast time over, by day.
    Dawn,
    /// The Song of Storms played.
    SongOfStorms,
    /// The big fairy out of the Song of Storms' spot (`Shot_Sun`).
    StormFairy,
    /// In Zelda's courtyard.
    ZeldaCourtyard,
    /// Zelda's Letter received.
    ZeldasLetter,
    /// Its get-item text read.
    LetterRead,
    /// Impa's lullaby script started.
    LullabyCs,
    /// Zelda's Lullaby played back.
    LullabyPlayed,
    /// Impa's goodbye in Hyrule Field over: Link at `ENTR_HYRULE_FIELD_17`.
    ImpaGone,
    /// A cucco lifted and carried (Player's `heldActor`).
    CuccoLifted,
    /// A cucco thrown into Anju's pen.
    CuccoPenned,
    /// Landed from a cucco's glide.
    Glided,
    /// Zelda's Letter shown to Kakariko's gate guard, his texts read.
    LetterShown,
    /// Kakariko's gate to Death Mountain open.
    GateOpen,
    /// The royal tomb's opening over (`gGraveyardTombOpeningChildCs`: its gravestone blown up,
    /// `EVENTCHKINF_1D`), Link free.
    TombOpened,
    /// Into the royal tomb (`ENTR_ROYAL_FAMILYS_TOMB_0`), settled.
    TombEntered,
    /// Up the steps to the royal tomb's door to room 1.
    DoorSteps,
    /// Dampé paid (10 rupees: `EnTk_UpdateTalkState`) and his dig on a spot over (`EnTk_Dig`'s
    /// last frame), its text closed.
    Dug,
    /// What a dig turned up, picked up.
    DugUp,
    /// The Sun's Song played by the ReDeads: all three stunned (`EnRd_SetupStunned`).
    ReDeadsStunned,
    /// The royal tomb's wall talked to: the Sun's Song's lesson (`gSunSongGraveSunSongTeachCs`).
    SunsSongLesson,
    /// The Sun's Song played back to the wall.
    SunsSongPlayed,
    /// The lesson's chain over (the graveyard's part 2, the tomb's part 3 at
    /// `ENTR_ROYAL_FAMILYS_TOMB_1`), Link free by the wall.
    SunsSongTaught,
    /// Kaepora Gaebora's talk read to the end (no, not again).
    Owl,
    /// The meadow's Wolfos killed: its switch flag (0x1F) set, the gate sinking.
    WolfosKilled,
    /// Past z -2220 on Saria's plateau: her lesson's cutscene started.
    SariasSongLesson,
    /// Saria's Song played back in the lesson.
    SariasSongPlayed,
    /// The lesson over, the song given (`QUEST_SONG_SARIA`), Link free.
    SariasSongLearned,
    /// Out of the Lost Woods into the Sacred Forest Meadow (`ENTR_SACRED_FOREST_MEADOW_0`), settled.
    MeadowEntered,
    /// On the small stump in the Lost Woods' room 9 (`En_Skj` type 5), Saria's Song played to the
    /// Skull Kid on his stump (the staff knows it).
    SkullKidSong,
    /// His piece of heart taken and its text read (`ITEMGETINF_16`), Link free.
    SkullKidHeart,
    /// A slingshot seed in the target's bullseye (room 1, `En_Dnt_Nomal`): one per hit in a row.
    Bullseye,
    /// The bigger seed bag taken and its text read (`ITEMGETINF_1D`), Link free.
    BulletBag,
    /// On the memory game's stump (`En_Skj` type 6) with the ocarina out: the Skull Kids' first
    /// notes (`MSGMODE_MEMORY_GAME_START`).
    MemoryGame,
    /// A turn of the memory game played back (`MSGMODE_MEMORY_GAME_ROUND_SUCCESS`): one a turn.
    /// Each round starts at three notes and adds one a turn to 5, 6 and 8: three turns, four, six.
    MemoryRound,
    /// The third round's prize taken and its text read (`ITEMGETINF_17`), Link free.
    MemoryGameWon,
    /// Epona's Song played back in Malon's lesson (`En_Ma1`, `OCARINA_ACTION_PLAYBACK_EPONA`).
    EponasSongPlayed,
    /// Lon Lon Ranch's layer 5 (`gLonLonRanchEponasSongCs`) entered: the playback's reload
    /// (`CS_INDEX_1`).
    EponasSongCs,
    /// Back at the ranch by `ENTR_LON_LON_RANCH_1` (the cutscene's destination 94), settled.
    RanchFromSong,
    /// The foal (`En_Horse_Link_Child`) come to Link, following him.
    FoalCame,
    /// The foal back home after her 300 frames, facing Link (`func_80A6A5A4`).
    FoalHome,
    /// Epona's Song played (the staff knows it).
    EponasSong,
    /// The foal, called by the song from her home, come to Link.
    FoalCalled,
    /// Into Talon's house (`souko`) by its door, settled.
    TalonsHouse,
    /// The Super Cucco game begun: Talon has thrown his three cuccos and the timer runs.
    CuccoGame,
    /// A super cucco held long enough for Talon (`superCuccoTimers`), his text read: one a cucco.
    SuperCucco,
    /// The game's end: back in the house at `ENTR_LON_LON_BUILDINGS_2`.
    CuccoGameOver,
    /// The first win's bottle of milk taken and its text read (`ITEMGETINF_TALON_BOTTLE`), Link
    /// free.
    MilkBottle,
    /// A Bomb Flower lifted: Link stands holding the bomb it handed him (`func_8002F5C4`).
    FlowerLifted,
    /// What Link carried put down (`Player_Action_808464B0`) and let go.
    PutDown,
    /// Dodongo's Cavern's opening over: its entrance wall broken (`EVENTCHKINF_B0`), the cutscene's
    /// end, Link free.
    DcOpening,
    /// A bomb out of the bag in Link's hand (`Player_InitExplosiveIA`).
    BombOut,
    /// A Bomb Flower set off by a blast (its bomb spawned, the flower regrowing).
    FlowerSetOff,
    /// Darunia's door (`Bg_Spot18_Shutter`) risen open (switch 0x1B: the lullaby at its spot).
    DaruniaDoor,
    /// The Goron's Bracelet taken from Darunia and its text read (the strength upgrade), Link free.
    Bracelet,
    /// What Link carried thrown (`Player_Action_808464B0`'s throw: at a run), his hands free.
    Thrown,
    /// The boulder at Dodongo's Cavern's mouth (`Bg_Spot16_Bombstone`) blown up: switch 4 and
    /// `EVENTCHKINF_23` set, its one-point cutscenes over, Link free.
    BoulderBlown,
    /// Dodongo's Cavern's bomb bag chest opened, the bag's text read, Link free.
    BombBag,
    /// A gravestone (`Bg_Haka`) pulled its 60 (`BgHaka_IdleOpened`), Link let go.
    StonePulled,
    /// Dodongo's Cavern's switch 0x20 pressed (room 7's `Obj_Switch` 0x2000): door 20 unbarred.
    BossPathOpen,
    /// Into King Dodongo's room (`ENTR_DODONGOS_CAVERN_BOSS_0`, room 16's pit), settled.
    BossFall,
    /// Dodongo's Cavern's pit boulder (`Obj_Bombiwa` 0x000E) blown: its flag 0x0E.
    PitBoulderBlown,
    /// The floor switch under it (`Obj_Switch` 0x0A00) pressed: flag 0x0A, the shortcut pillar's.
    PillarSwitch,
    /// Up a ladder onto the floor at its top, standing.
    LadderUp,
    /// The shortcut pillar (`Bg_Ddan_Jd` 0x000A) idle at the height a task waits for.
    PillarReady,
    /// On the shortcut pillar (`Bg_Ddan_Jd` 0x000A), standing.
    OnPillar,
    /// Up on Dodongo's Cavern's upper level (y 531), standing.
    UpperLevel,
    /// One of the skull's eyes (`Bg_Dodoago`) lit to 255, its one-point over, Link free.
    EyeLit,
    /// The skull's jaw open (`Bg_Dodoago` done: its flag 0x1A), one-point 3380 over, Link free.
    JawOpen,
    /// King Dodongo's room's floor (`Bg_Breakwall` 0x4001) bombed open: its switch flag 1.
    FloorBombed,
    /// King Dodongo's intro over (`EVENTCHKINF_BEGAN_KING_DODONGO_BATTLE`, the camera back, Link
    /// free): the fight begins.
    KingDodongoBattle,
    /// The Goron's Ruby's cutscene (`gDMTGoronRubyCs`) over: its terminator's transition started.
    RubyOver,
    /// Death Mountain Trail's spawn 5 (`ENTR_DEATH_MOUNTAIN_TRAIL_5`), standing.
    Trail5,
    /// The Great Fairy's wall on Death Mountain's summit (`Bg_Bombwall` 0x8003) broken: its switch
    /// flag 3, Link free.
    FairyWall,
    /// In the Great Fairy's fountain (`ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_0`), standing.
    Fountain,
    /// The Great Fairy's cutscene (`gGreatFairyMagicCs`) over: magic acquired, Link free.
    Magic,
    /// Death Mountain Trail's spawn 4 (`ENTR_DEATH_MOUNTAIN_TRAIL_4`, the fountain's return
    /// entrance), standing.
    Trail4,
    /// In Kakariko at the owl's landing (`ENTR_KAKARIKO_VILLAGE_14`, `gDMTOwlCs`'s terminator 66),
    /// standing.
    Kakariko14,
    /// Dodongo's Cavern's falling stairs (`Bg_Ddan_Kd` 0x000B) down: switch 0x0B, one-point 3050
    /// over, Link free.
    StairsFallen,
    /// The crate (`Obj_Kibako2`) a task waited on broken.
    CrateBroken,
    /// Dodongo's Cavern's silver rupees all taken: the tracker's flag 0x25, door 17's camera over,
    /// Link free.
    SilverRupees,
    /// Dodongo's Cavern's room 14's chest opened, its text read, Link free.
    ShieldChest,
    /// The Song of Time played (the staff knows it).
    TimeSongPlayed,
    /// A Song of Time block (`Obj_Timeblock`) gone, its camera over, Link free.
    TimeBlockGone,
    /// A bottle drunk (`Player_Action_8084EAC0` over), Link free.
    MilkDrunk,
    /// A fish caught in a bottle, its text read, Link free.
    FishCaught,
    /// A fish let out of the bottle (`Player_Action_8084EFC0` over), Link free.
    FishLetOut,
    /// Zora's River's falls open (`Bg_Spot03_Taki`: switch 0x38), one-point 4100 over.
    WaterfallOpened,
    /// In Zora's Domain (through the falls, `ENTR_ZORAS_DOMAIN_0`), settled.
    ZorasDomain,
    /// Zora's Domain's intro seen (`EVENTCHKINF_A4`), Link free.
    DomainIntro,
    /// Ruto's letter shown to King Zora, his texts read.
    KingZoraLetter,
    /// King Zora moved aside (`EVENTCHKINF_GAVE_LETTER_TO_KING_ZORA`), Link free.
    KingMoved,
    /// Inside Lord Jabu-Jabu (`SCENE_JABU_JABU`).
    JabuSwallowed,
    /// Jabu-Jabu's room 0: the west cow's target shot (`En_Wonder_Item` 0x1915: switch flag 0x15).
    CowShot,
    /// Room 1: the elevator level with door 1's corridor on its way down.
    ElevatorLevel,
    /// Room 1: on the elevator (`Bg_Bdan_Objects` type 1).
    OnElevator,
    /// Room 1: the elevator at the bottom.
    ElevatorDown,
    /// Under a wall in the water and up on its far side.
    Dived,
    /// Room 1: the yellow switch 0x19 down (door 2).
    YellowSwitch,
    /// Room 14: the boomerang's chest opened (`En_Box` 0x10C6, treasure flag 6).
    BoomerangChest,
    /// Jabu-Jabu's room 2: Ruto's first meeting over (`gRutoFirstMeetingCs`, she dropped through a
    /// hole: `INFTABLE_RUTO_MET_FIRST_TIME`), Link free.
    RutoMet,
    /// A held bomb's fuse short (`En_Bom`'s `timer`).
    FuseShort,
    /// A bomb thrown (Link's hands free again).
    BombThrown,
    /// Room 2: the cow's three boulders blown up (`Obj_Bombiwa` 0x0006, 0x0011, 0x001A).
    BouldersBroken,
    /// Room 11: its chest opened (`En_Box` 0x70E9, treasure flag 9: a Deku Stick).
    StickChest,
    /// Room 5: the crystal switch hit (`Obj_Switch` 0x1403: switch flag 0x14, door 0).
    CrystalShot,
    /// Barinade's room (`SCENE_JABU_JABU_BOSS`) entered (his intro takes Link at once).
    BarinadesRoom,
    /// Barinade's intro over (`EVENTCHKINF_BEGAN_BARINADE_BATTLE`, the camera back, Link free):
    /// the fight begins.
    BarinadeBattle,
    /// Link in Ruto's warp (`DoorWarp1_RutoWarpIdle` done: `WARP_BLUE_RUTO_STATE_ENTERED`).
    RutoWarpEntered,
    /// Ruto's words in her warp read (`WARP_BLUE_RUTO_STATE_WARPING`): the warp takes them.
    RutoWarpTalked,
    /// The Zora's Sapphire's cutscene (`gZorasFountainSapphireCs`) over: its terminator's
    /// transition started.
    SapphireOver,
    /// Zora's Fountain (`ENTR_ZORAS_FOUNTAIN_0`: in the water by Jabu-Jabu), Link free.
    ZorasFountain,
    /// Zelda's escape started: Hyrule Field's layer 5 (`gHyruleFieldZeldaEscapeCs`) running.
    EscapeStarted,
    /// Zelda's escape over: its terminator (33) back to Hyrule Field's layer 1, Link free.
    Escaped,
    /// The Ocarina of Time taken from the moat's bottom (`EVENTCHKINF_43`) and Link surfaced.
    OcarinaTaken,
    /// The ocarina's text closed: `gHyruleFieldZeldaSongOfTimeCs` running.
    SongOfTimeCs,
    /// The Temple of Time's layer 11 (`gTempleOfTimeSongOfTimeCs`) running.
    TempleOfTime,
    /// The Song of Time played back (`MSGMODE_SONG_PLAYBACK_SUCCESS`).
    SongOfTimePlayed,
    /// Hyrule Field at `ENTR_HYRULE_FIELD_16` after `gHyruleFieldGetOoTCs`, Link free.
    FieldAfterOcarina,
    /// Jabu-Jabu's room 3: Ruto's first talk closed (0x404C: `INFTABLE_RUTO_EXPLAINED_WHY_IN_JABU`).
    RutoExplained,
    /// Ruto's second talk (0x404D): she sat down and asks to be carried
    /// (`INFTABLE_RUTO_LET_LINK_CARRY`), the box closed.
    RutoSeated,
    /// Ruto lifted: Link stands carrying her.
    RutoLifted,
    /// Ruto put down (Link's hands free).
    RutoPutDown,
    /// A spout (`En_Siofuki`) risen with Link on it.
    SpoutUp,
    /// Room 3's blue switch held down by Ruto (switch flag 0x3C: door 12).
    RutoOnSwitch,
    /// Room 6 entered carrying Ruto: `gRutoFoundSapphireCs` over
    /// (`INFTABLE_RUTO_BROUGHT_TO_SAPPHIRE_ROOM`), Link free.
    RutoFoundSapphire,
    /// Ruto thrown (Link's hands free).
    RutoThrown,
    /// `gRutoObtainingSapphireCs` over: the Sapphire Ruto's (`INFTABLE_RUTO_HAS_SAPPHIRE`).
    RutoHasSapphire,
    /// Big Octo's platform rising with Ruto (one-point 3070: Link held).
    PlatformRising,
    /// Big Octo spawned on the risen platform and Ruto taken (`INFTABLE_RUTO_ABDUCTED`).
    BigOctoArrived,
    /// The platform down with Big Octo and his battle on (`Bg_Bdan_Objects`'
    /// `OctoPlatformBattleInProgress`), the cameras back, Link free.
    BigOctoBattle,
    /// The text by the frogs' log (`En_Wonder_Talk2` 0x4BBB: 0x22E) read (its switch flag 0x3B).
    FrogsTip,
    /// A song's reward from the frogs taken (its `EVENTCHKINF_SONGS_FOR_FROGS_*`), Link free.
    FrogsZl,
    FrogsEpona,
    FrogsSaria,
    FrogsSuns,
    FrogsTime,
    FrogsStorms,
    /// The frogs' song game's fourteen notes played.
    FrogsGame,
    /// Its piece of heart taken (`EVENTCHKINF_SONGS_FOR_FROGS_CHOIR`), Link free.
    FrogsChoir,
    /// Eight notes played to Bonooru (`MSGMODE_SCARECROW_SPAWN_RECORDING_ONGOING`).
    BonooruSong,
    /// His texts read: the spawn song set (`scarecrowSpawnSongSet`), Link free.
    BonooruRemembers,
    /// Talked to, he plays the spawn song back (`OCARINA_MODE_0F`); the box closed.
    BonooruPlayback,
    /// The Master Sword taken from its pedestal: `gPullMasterSwordCs` started.
    SwordPulled,
    /// The Master Sword put back in its pedestal: `gPlaceMasterSwordCs` started.
    SwordPutBack,
    /// Arrived by time travel (`ENTR_TEMPLE_OF_TIME_2`, start mode 1): off the pedestal
    /// (`Player_Action_TimeTravelEnd` over), Link standing.
    TimeTravelled,
    /// The Temple of Time's intro (`gTempleOfTimeIntroCs`, `EVENTCHKINF_A7`) over, Link free.
    TempleIntro,
    /// `gDoorOfTimeOpeningCs` over: the Door of Time open (`EVENTCHKINF_OPENED_DOOR_OF_TIME`),
    /// Link free.
    DoorOfTimeOpen,
    /// `gRevealMasterSwordCs` started (`EVENTCHKINF_REVEALED_MASTER_SWORD`).
    SwordReveal,
    /// Its script over, Link free.
    SwordRevealed,
    /// The cutscene map's layer 7 (`gCutsceneMapGanondorfFromMasterSwordCs`) playing.
    GanondorfFromSword,
    /// The Chamber of the Sages' layer 4 (`gChamberOfSagesLightMedallionCs`) playing, Link an
    /// adult.
    Chamber,
    /// The Light Medallion given (`QUEST_MEDALLION_LIGHT`: Rauru's `Item_Give`).
    LightMedallion,
    /// Terminator 18's transition to `ENTR_TEMPLE_OF_TIME_4` started.
    ChamberLeft,
    /// Back in the Temple at `ENTR_TEMPLE_OF_TIME_4`: Sheik's `gTempleOfTimeFirstAdultCs`
    /// started (`EVENTCHKINF_C5`).
    SheikAwakening,
    /// Its script over and the return's time warp gone (`EVENTCHKINF_C9`), Link free.
    AdultAwake,
    /// Out of the Temple of Time (`ENTR_TEMPLE_OF_TIME_EXTERIOR_DAY_1`: the ruins' `shrine_r` for
    /// an adult), settled.
    TempleExterior,
    /// Into the ruined market (`ENTR_MARKET_DAY_2`: `market_ruins` for an adult), settled.
    MarketRuins,
    /// Past its ReDeads into the ruined market entrance (`ENTR_MARKET_ENTRANCE_DAY_0`: `enrui`),
    /// settled.
    MarketEntranceRuins,
    /// Out to the field by the broken drawbridge (`ENTR_HYRULE_FIELD_7`), settled.
    FieldByTheDrawbridge,
    /// Sheik's `gTempleOfTimePreludeCs` started (`EVENTCHKINF_55`, the Prelude of Light given).
    PreludeStarted,
    /// The Prelude played back to Sheik (`MSGMODE_SONG_PLAYBACK_SUCCESS`).
    PreludePlayed,
    /// Its script over, Link free.
    PreludeLearned,
    /// The Prelude of Light played on the ocarina (the warp's "warp?" choice follows).
    PreludeWarpPlayed,
    /// Its choice answered yes: the warp-out (`Demo_Kankyo`'s `DEMOKANKYO_WARP_OUT`) started.
    PreludeWarpOut,
    /// Arrived on the Temple of Time's light pad (`ENTR_TEMPLE_OF_TIME_7`): the warp-in's script
    /// over, Link free.
    PreludeWarped,
    /// Epona's Song played on the ocarina (`R_EPONAS_SONG_PLAYED`): Epona comes.
    EponaSongPlayed,
    /// On Epona, the mount over (`Player_Action_8084CC98`).
    Mounted,
    /// Ridden through the points, and halted.
    RodeOver,
    /// Off the horse (`Player_Action_8084D3E4` over), standing.
    Dismounted,
    /// Ridden into an exit: the transition started, Link on the horse.
    RodeOut,
    /// In the next scene on the horse (`R_EXITED_SCENE_RIDING_HORSE`, `HORSE_PTYPE_PLAYER_SPAWNED_RIDING`),
    /// the fade over and the horse standing.
    RiddenIn,
    /// Ingo's rental paid (0x205C): the transition to the corral (`ENTR_LON_LON_RANCH_6`) started.
    RentalPaid,
    /// In the corral for the rental, Link free (the timer at 60).
    InCorral,
    /// The race taken from horseback (0x2038, "Do it"): the transition to the race's intro
    /// (`ENTR_LON_LON_RANCH_0`, `CS_INDEX_0`) started.
    RaceTaken,
    /// The race's start: Ingo's horse racing (`EnHorseGameCheck_UpdateIngoRace`'s start, its
    /// `inRace`).
    RaceStarted,
    /// Ingo's gloat over (0x203D, -50 Rupees): his rematch offered (`INGO_RACE_STATE_REMATCH`).
    IngoGloated,
    /// Ingo's loss over (0x203A): the second race's intro (the ranch's layer 4) reached.
    SecondRaceCalled,
    /// Ingo's gate scene over (0x203B, 0x203C, the gates shut): he guards the way out
    /// (`EnIn_GuardingGate`), Link free on Epona.
    GateShut,
    /// Past Ingo over the gate: the transition to the field (`ENTR_HYRULE_FIELD_15`) started.
    JumpedOut,
    /// In the field after the jump's cutscene (`gHyruleFieldGateEponaJumpCs`), Epona his
    /// (`EVENTCHKINF_EPONA_OBTAINED`), on her.
    EponaWon,
    /// Adult Malon's thanks over (0x2000: `INFTABLE_B8`).
    MalonThanked,
    /// Epona come to her song, within 1,200 of Malon (`EnMa3_GetTextId`'s `Actor_FindNearby`).
    EponaCame,
    /// Malon's obstacle course taken (0x2002 into 0x2003, "Let's go", 0x2092): the transition to
    /// its start (`ENTR_LON_LON_RANCH_0`, `CS_INDEX_0`) started.
    CourseTaken,
    /// The course's start: its referee (`En_Horse_Game_Check` type 4) letting Link go
    /// (`MALONRACE_PLAYER_MOVE`).
    CourseStarted,
    /// Malon's record over (0x208F): `EVENTCHKINF_HORSE_RACE_COW_UNLOCK` set, the high score kept.
    CourseRecord,
    /// Malon's talk after Epona's Song over (0x2051 into 0x2052:
    /// `INFTABLE_TALKED_TO_ADULT_MALON_AFTER_SONG`).
    MalonSang,
    /// A Big Poe out (`En_Po_Field` params `EN_PO_FIELD_BIG`, `EnPoField_Appear`): Link riding
    /// within 150 of one of its places.
    BigPoeAppeared,
    /// The Big Poe fleeing (`EnPoField_Flee`).
    BigPoeFleeing,
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
            Step::SlingshotDrawn => "slingshot_drawn",
            Step::EyeShot => "eye_shot",
            Step::MenuOpened => "menu_opened",
            Step::CursorOnItem => "cursor_on_item",
            Step::ItemEquipped => "item_equipped",
            Step::PageTurned => "page_turned",
            Step::MenuClosed => "menu_closed",
            Step::FloorChanged => "floor_changed",
            Step::Died => "died",
            Step::SavePrompt => "save_prompt",
            Step::ContinuePrompt => "continue_prompt",
            Step::Respawned => "respawned",
            Step::Saved => "saved",
            Step::Loaded => "loaded",
            Step::GohmaEntered => "gohma_entered",
            Step::GohmaWaiting => "gohma_waiting",
            Step::GohmaLookedAt => "gohma_looked_at",
            Step::GohmaBattle => "gohma_battle",
            Step::GohmaStunned => "gohma_stunned",
            Step::GohmaHit => "gohma_hit",
            Step::GohmaKnockedDown => "gohma_knocked_down",
            Step::GohmaDefeated => "gohma_defeated",
            Step::GohmaGone => "gohma_gone",
            Step::HeartTaken => "heart_taken",
            Step::WarpEntered => "warp_entered",
            Step::WarpedOut => "warped_out",
            Step::Arrived => "arrived",
            Step::EmeraldPart1Over => "emerald_part1_over",
            Step::BossRoom => "boss_room",
            Step::EmeraldShown => "emerald_shown",
            Step::TreeDeath => "tree_death",
            Step::EmeraldOver => "emerald_over",
            Step::Forest11 => "forest_11",
            Step::Ganondorf => "ganondorf",
            Step::Goddesses => "goddesses",
            Step::Din => "din",
            Step::Valley => "valley",
            Step::Nayru => "nayru",
            Step::Farore => "farore",
            Step::Triforce => "triforce",
            Step::EmeraldPart9 => "emerald_part9",
            Step::ForestLeft => "forest_left",
            Step::Bridge => "bridge",
            Step::FairyOcarina => "fairy_ocarina",
            Step::Field => "field",
            Step::FieldIntroOver => "field_intro_over",
            Step::OwlTalk => "owl_talk",
            Step::OwlFlown => "owl_flown",
            Step::MusicFaded => "music_faded",
            Step::Night => "night",
            Step::BridgeRaised => "bridge_raised",
            Step::NightCritters => "night_critters",
            Step::PeahatKilled => "peahat_killed",
            Step::CastleTown => "castle_town",
            Step::StalchildrenKilled => "stalchildren_killed",
            Step::Kakariko => "kakariko",
            Step::StalchildrenGone => "stalchildren_gone",
            Step::Market => "market",
            Step::Townsman => "townsman",
            Step::Malon => "malon",
            Step::CastleRoad => "castle_road",
            Step::Vine => "vine",
            Step::TalonWoken => "talon_woken",
            Step::TalonTalk => "talon_talk",
            Step::TalonGone => "talon_gone",
            Step::CrateInMoat => "crate_in_moat",
            Step::MoatBank => "moat_bank",
            Step::OnCrates => "on_crates",
            Step::Drain => "drain",
            Step::Courtyard => "courtyard",
            Step::Caught => "caught",
            Step::ThrownOut => "thrown_out",
            Step::OcarinaOut => "ocarina_out",
            Step::SariasSong => "sarias_song",
            Step::SariaTalk => "saria_talk",
            Step::SunsSong => "suns_song",
            Step::Dawn => "dawn",
            Step::SongOfStorms => "song_of_storms",
            Step::StormFairy => "storm_fairy",
            Step::ZeldaCourtyard => "zelda_courtyard",
            Step::ZeldasLetter => "zeldas_letter",
            Step::LetterRead => "letter_read",
            Step::LullabyCs => "lullaby_cs",
            Step::LullabyPlayed => "lullaby_played",
            Step::ImpaGone => "impa_gone",
            Step::CuccoLifted => "cucco_lifted",
            Step::CuccoPenned => "cucco_penned",
            Step::Glided => "glided",
            Step::LetterShown => "letter_shown",
            Step::GateOpen => "gate_open",
            Step::TombOpened => "tomb_opened",
            Step::TombEntered => "tomb_entered",
            Step::DoorSteps => "door_steps",
            Step::Dug => "dug",
            Step::DugUp => "dug_up",
            Step::ReDeadsStunned => "redeads_stunned",
            Step::SunsSongLesson => "suns_song_lesson",
            Step::SunsSongPlayed => "suns_song_played",
            Step::SunsSongTaught => "suns_song_taught",
            Step::Owl => "owl",
            Step::WolfosKilled => "wolfos_killed",
            Step::SariasSongLesson => "sarias_song_lesson",
            Step::SariasSongPlayed => "sarias_song_played",
            Step::SariasSongLearned => "sarias_song_learned",
            Step::MeadowEntered => "meadow_entered",
            Step::SkullKidSong => "skull_kid_song",
            Step::SkullKidHeart => "skull_kid_heart",
            Step::Bullseye => "bullseye",
            Step::BulletBag => "bullet_bag",
            Step::MemoryGame => "memory_game",
            Step::MemoryRound => "memory_round",
            Step::MemoryGameWon => "memory_game_won",
            Step::EponasSongPlayed => "eponas_song_played",
            Step::EponasSongCs => "eponas_song_cs",
            Step::RanchFromSong => "ranch_from_song",
            Step::FoalCame => "foal_came",
            Step::FoalHome => "foal_home",
            Step::EponasSong => "eponas_song",
            Step::FoalCalled => "foal_called",
            Step::TalonsHouse => "talons_house",
            Step::CuccoGame => "cucco_game",
            Step::SuperCucco => "super_cucco",
            Step::CuccoGameOver => "cucco_game_over",
            Step::MilkBottle => "milk_bottle",
            Step::FlowerLifted => "flower_lifted",
            Step::PutDown => "put_down",
            Step::DcOpening => "dc_opening",
            Step::BombOut => "bomb_out",
            Step::FlowerSetOff => "flower_set_off",
            Step::DaruniaDoor => "darunia_door",
            Step::Bracelet => "bracelet",
            Step::Thrown => "thrown",
            Step::BoulderBlown => "boulder_blown",
            Step::BombBag => "bomb_bag",
            Step::StonePulled => "stone_pulled",
            Step::BossPathOpen => "boss_path_open",
            Step::BossFall => "boss_fall",
            Step::PitBoulderBlown => "pit_boulder_blown",
            Step::PillarSwitch => "pillar_switch",
            Step::LadderUp => "ladder_up",
            Step::PillarReady => "pillar_ready",
            Step::OnPillar => "on_pillar",
            Step::UpperLevel => "upper_level",
            Step::EyeLit => "eye_lit",
            Step::JawOpen => "jaw_open",
            Step::FloorBombed => "floor_bombed",
            Step::KingDodongoBattle => "king_dodongo_battle",
            Step::RubyOver => "ruby_over",
            Step::Trail5 => "trail_5",
            Step::FairyWall => "fairy_wall",
            Step::Fountain => "fountain",
            Step::Magic => "magic",
            Step::Trail4 => "trail_4",
            Step::Kakariko14 => "kakariko_14",
            Step::StairsFallen => "stairs_fallen",
            Step::CrateBroken => "crate_broken",
            Step::SilverRupees => "silver_rupees",
            Step::ShieldChest => "shield_chest",
            Step::TimeSongPlayed => "time_song_played",
            Step::TimeBlockGone => "time_block_gone",
            Step::MilkDrunk => "milk_drunk",
            Step::FishCaught => "fish_caught",
            Step::FishLetOut => "fish_let_out",
            Step::WaterfallOpened => "waterfall_opened",
            Step::ZorasDomain => "zoras_domain",
            Step::DomainIntro => "domain_intro",
            Step::KingZoraLetter => "king_zora_letter",
            Step::KingMoved => "king_moved",
            Step::JabuSwallowed => "jabu_swallowed",
            Step::CowShot => "cow_shot",
            Step::ElevatorLevel => "elevator_level",
            Step::OnElevator => "on_elevator",
            Step::ElevatorDown => "elevator_down",
            Step::Dived => "dived",
            Step::YellowSwitch => "yellow_switch",
            Step::BoomerangChest => "boomerang_chest",
            Step::RutoMet => "ruto_met",
            Step::FuseShort => "fuse_short",
            Step::BombThrown => "bomb_thrown",
            Step::BouldersBroken => "boulders_broken",
            Step::StickChest => "stick_chest",
            Step::CrystalShot => "crystal_shot",
            Step::BarinadesRoom => "barinades_room",
            Step::BarinadeBattle => "barinade_battle",
            Step::RutoWarpEntered => "ruto_warp_entered",
            Step::RutoWarpTalked => "ruto_warp_talked",
            Step::SapphireOver => "sapphire_over",
            Step::ZorasFountain => "zoras_fountain",
            Step::EscapeStarted => "escape_started",
            Step::Escaped => "escaped",
            Step::OcarinaTaken => "ocarina_taken",
            Step::SongOfTimeCs => "song_of_time_cs",
            Step::TempleOfTime => "temple_of_time",
            Step::SongOfTimePlayed => "song_of_time_played",
            Step::FieldAfterOcarina => "field_after_ocarina",
            Step::RutoExplained => "ruto_explained",
            Step::RutoSeated => "ruto_seated",
            Step::RutoLifted => "ruto_lifted",
            Step::RutoPutDown => "ruto_put_down",
            Step::SpoutUp => "spout_up",
            Step::RutoOnSwitch => "ruto_on_switch",
            Step::RutoFoundSapphire => "ruto_found_sapphire",
            Step::RutoThrown => "ruto_thrown",
            Step::RutoHasSapphire => "ruto_has_sapphire",
            Step::PlatformRising => "platform_rising",
            Step::BigOctoArrived => "big_octo_arrived",
            Step::BigOctoBattle => "big_octo_battle",
            Step::FrogsTip => "frogs_tip",
            Step::FrogsZl => "frogs_zl",
            Step::FrogsEpona => "frogs_epona",
            Step::FrogsSaria => "frogs_saria",
            Step::FrogsSuns => "frogs_suns",
            Step::FrogsTime => "frogs_time",
            Step::FrogsStorms => "frogs_storms",
            Step::FrogsGame => "frogs_game",
            Step::FrogsChoir => "frogs_choir",
            Step::BonooruSong => "bonooru_song",
            Step::BonooruRemembers => "bonooru_remembers",
            Step::BonooruPlayback => "bonooru_playback",
            Step::SwordPulled => "sword_pulled",
            Step::SwordPutBack => "sword_put_back",
            Step::TimeTravelled => "time_travelled",
            Step::TempleIntro => "temple_intro",
            Step::DoorOfTimeOpen => "door_of_time_open",
            Step::SwordReveal => "sword_reveal",
            Step::SwordRevealed => "sword_revealed",
            Step::GanondorfFromSword => "ganondorf_from_sword",
            Step::Chamber => "chamber",
            Step::LightMedallion => "light_medallion",
            Step::ChamberLeft => "chamber_left",
            Step::SheikAwakening => "sheik_awakening",
            Step::AdultAwake => "adult_awake",
            Step::TempleExterior => "temple_exterior",
            Step::MarketRuins => "market_ruins",
            Step::MarketEntranceRuins => "market_entrance_ruins",
            Step::FieldByTheDrawbridge => "field_by_the_drawbridge",
            Step::PreludeStarted => "prelude_started",
            Step::PreludePlayed => "prelude_played",
            Step::PreludeLearned => "prelude_learned",
            Step::PreludeWarpPlayed => "prelude_warp_played",
            Step::PreludeWarpOut => "prelude_warp_out",
            Step::PreludeWarped => "prelude_warped",
            Step::EponaSongPlayed => "epona_song_played",
            Step::Mounted => "mounted",
            Step::RodeOver => "rode_over",
            Step::Dismounted => "dismounted",
            Step::RodeOut => "rode_out",
            Step::RiddenIn => "ridden_in",
            Step::RentalPaid => "rental_paid",
            Step::InCorral => "in_corral",
            Step::RaceTaken => "race_taken",
            Step::RaceStarted => "race_started",
            Step::IngoGloated => "ingo_gloated",
            Step::SecondRaceCalled => "second_race_called",
            Step::GateShut => "gate_shut",
            Step::JumpedOut => "jumped_out",
            Step::EponaWon => "epona_won",
            Step::MalonThanked => "malon_thanked",
            Step::EponaCame => "epona_came",
            Step::CourseTaken => "course_taken",
            Step::CourseStarted => "course_started",
            Step::CourseRecord => "course_record",
            Step::MalonSang => "malon_sang",
            Step::BigPoeAppeared => "big_poe_appeared",
            Step::BigPoeFleeing => "big_poe_fleeing",
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
    /// Inside the Deku Tree, the Fairy Slingshot out from C-Right and aimed in first person at
    /// room 1's eye switch, a seed shot into it (flag 0x0C, the door to room 2 unbarred with its
    /// camera), and through that door into room 2 (GAME-05 milestone 5a), from a debug start 250
    /// in front of the eye (`SLINGSHOT_START`, the `deku-tree-slingshot` preset).
    Slingshot,
    /// Inside the Deku Tree, the pause menu opened, the item page's cursor to the Fairy
    /// Slingshot, C-Right equipping it (the icon flying to the button), the page turned to the map,
    /// and the menu closed: the slingshot on C-Right (GAME-05 milestone 5b-1), from the scene's
    /// spawn with the slingshot owned and C-Right empty (`deku-tree-slingshot-owned`).
    Pause,
    /// Inside the Deku Tree, the pause menu opened and turned to the dungeon map page (1F, Link's
    /// floor: its rooms, the current one pulsing, chest 3's mark), the stick up to 2F (its maps
    /// loaded, chest 1's mark), and the menu closed (GAME-05 milestone 5b-2), from the scene's
    /// spawn with the compass and 3F to 1F visited (`deku-tree-compass`).
    DungeonMap,
    /// Inside the Deku Tree with a quarter heart, the top floor's Deku Baba's bite: Link dies,
    /// the game over's "GAME OVER" and its window, "No" at "Would you like to save?", "Yes" at
    /// "Continue playing?", and the respawn at the entrance (GAME-05 milestone 5b-2), from
    /// `DekuBaba`'s debug start (`DEKU_BABA_START`, `deku-tree-quarter-heart`).
    GameOver,
    /// Inside the Deku Tree with two hearts and the Fairy Slingshot on no button, the pause menu
    /// opened, the slingshot onto C-Left, B's save prompt, "Yes": the save written and the menu
    /// closed; then the console's reset loads file 2 back: Link at the Deku Tree's entrance with
    /// three hearts and the slingshot on C-Left (GAME-05 milestone 5c), from the scene's spawn
    /// (`deku-tree-save`, a debug start: file 2 of an SRAM in memory).
    Save,
    /// Queen Gohma's room (`ENTR_DEKU_TREE_BOSS_0`): to the room's entrance, her intro, the
    /// slingshot aimed up at her on the ceiling until she notices, the fight (seeds into her red
    /// eye, on the floor and on the ceiling; jump slashes while she's stunned), her death and the
    /// heart container taken (GAME-05 milestone 6a), from the entrance's spawn with the sword, the
    /// shield, sticks, nuts and the slingshot on C (`deku-tree-gohma`).
    Gohma,
    /// Queen Gohma's room after her defeat (`deku-tree-gohma-cleared`: her init leaves the blue
    /// warp): into the warp, floating up, the white fade out with the Kokiri Emerald; Kokiri
    /// Forest by blue warp (falling in, landing) and the Deku Tree's emerald cutscene, part 1,
    /// read through to its terminator (GAME-05 milestone 6b), from a debug start in front of the
    /// warp (`BLUE_WARP_START`).
    BlueWarp,
    /// Inside the Deku Tree, room 9 with its hint scrubs' puzzle solved (the room cleared, its door
    /// to room 11 unbarred): through that door, onto room 11's floor (exit 2, `floor_effect` 2: the
    /// respawn point set, a void out), and into Queen Gohma's room, standing (GAME-05 milestone 6c),
    /// from room 9's debug start.
    BossRoom,
    /// Kokiri Forest's cutscene layer 6 (`gKokiriForestKokiriEmeraldPart9Cs`), the Kokiri
    /// Emerald's last part, from its start (cutscene index 0xFFF2 on the `deku-tree-dead` preset):
    /// the emerald shown and floating over Link, the Deku Tree's texts and his death, A through
    /// the texts, to the terminator's `ENTR_KOKIRI_FOREST_11` (GAME-06 milestone 1a).
    Emerald,
    /// The emerald's cutscene chain from its part 2 (the castle town's cutscene map, layer 5:
    /// cutscene index 0xFFF1 on the `deku-tree-dead` preset) through the world's creation, the
    /// Triforce and the emerald to `ENTR_KOKIRI_FOREST_11`, A through every text (GAME-06
    /// milestone 1b).
    Creation,
    /// Out of the forest (GAME-06 milestone 2): from `ENTR_KOKIRI_FOREST_11` on the `deku-tree-dead`
    /// preset (where the chain leaves Link) to the Lost Woods' exit; Saria's goodbye on the bridge
    /// and the Fairy Ocarina; Hyrule Field's intro; the owl's talk and his flight.
    Farewell,
    /// Hyrule Field at dusk (GAME-06 milestone 3): from 17:00 in front of the drawbridge
    /// (`ENTR_HYRULE_FIELD_0`, `deku-tree-dead`), facing the sunset while the day's music stops,
    /// then round to the castle as night falls and the drawbridge rises, until the night's
    /// critters (past 19:00).
    Dusk,
    /// Hyrule Field by day (GAME-06 milestone 4): a grounded Peahat (`En_Peehat` -1, west of the
    /// castle) fought from a debug start 600 east of it, then east to the drawbridge and into
    /// Castle Town's entrance.
    Field,
    /// Hyrule Field at 20:00 (GAME-06 milestone 4): two Stalchildren (`En_Encount1`'s `En_Skb`)
    /// fought on the open field north of Lon Lon Ranch, then east to Kakariko's stairs and up
    /// them into the village.
    FieldNight,
    /// Castle Town by day (GAME-07 milestone 1): from its entrance (`ENTR_MARKET_ENTRANCE_DAY_0`,
    /// `deku-tree-dead`) into the market, a townsman (`En_Hy` type 9) and Malon (`En_Ma1`)
    /// talked to, and out north to the castle's road, where its intro plays.
    Market,
    /// Hyrule Castle's grounds (GAME-07 milestone 2): from the castle's road with the Cucco
    /// (`castle-cucco`) past the sentries to Talon by the crates; the Cucco shown, Talon woken and
    /// run home; two crates pushed into the moat, up them and through the drain into the guards'
    /// courtyard.
    Castle,
    /// Hyrule Castle's grounds, caught (GAME-07 milestone 2): from the castle's road (`castle-egg`)
    /// up the vine and into a sentry's sight; "Hey you!", the starburst circle and the throw-out by
    /// the gate (`ENTR_HYRULE_CASTLE_4`).
    CastleCaught,
    /// The ocarina (GAME-07 milestone 3): in Hyrule Field at 18:30 with the songs
    /// (`ocarina-songs`), from a debug start 312 north of the Song of Storms' fairy spot
    /// (`OCARINA_START`): Saria's Song (Navi's talk: Saria's, then no more), the Sun's Song (the
    /// night run through at 400 a frame to 06:30), then at the spot (`Shot_Sun` 0xFF41) the Song
    /// of Storms: the storm, and a big fairy.
    Ocarina,
    /// Zelda (GAME-07 milestone 4): from the drain into the guards' courtyard (`castle-courtyard`),
    /// past the patrols unseen to Zelda's courtyard, and on as `ZeldaTalk`.
    Zelda,
    /// Zelda (GAME-07 milestone 4): from Zelda's courtyard's entrance (`castle-courtyard`), her talk
    /// through the legend and the window to her letter.
    ZeldaTalk,
    /// Kakariko (GAME-07 milestone 5): from its entrance by day with Zelda's Letter
    /// (`kakariko-letter`), its intro, three cuccos carried into Anju's pen (the third glided off a
    /// roof), and the letter shown to the gate guard: the gate to Death Mountain opens.
    Kakariko,
    /// The graveyard (GAME-07 milestone 6): from its entrance at 18:30 with Zelda's Lullaby
    /// (`graveyard-lullaby`), its intro, the lullaby at the royal tomb (its gravestone blown up),
    /// and into the tomb.
    Graveyard,
    /// The Sacred Forest Meadow (GAME-08 milestone 1): from the Lost Woods' entrance from Kokiri
    /// Forest with Zelda's Letter and the Sun's Song (`graveyard-sun`), through the woods' right
    /// turns to the meadow, the Wolfos killed and its gate down, the maze, and Saria's lesson:
    /// Saria's Song learned.
    Meadow,
    /// The Lost Woods' Skull Kids and target scrub (GAME-08 milestone 2): from the woods' entrance
    /// with Saria's Song and the slingshot (`meadow-saria`), Saria's Song to the Skull Kid on his
    /// stump in room 9 (a piece of heart), the target's three bullseyes in room 1 (the bigger seed
    /// bag), and the memory game on the stump in room 1's hollow, three rounds won (a piece of
    /// heart).
    LostWoods,
    /// Lon Lon Ranch (GAME-08 milestone 3): from its gate with the Lost Woods' games won
    /// (`lost-woods-games`), its intro, Malon's three talks and her lesson: Epona's Song played
    /// back, `gLonLonRanchEponasSongCs` and back to the ranch (destination 94); the foal comes to
    /// Link, and from her home, called with the song.
    Ranch,
    /// Dodongo's Cavern's entrance (GAME-09 milestone 1), from its spawn with the Goron's Bracelet
    /// and the 20 bomb bag (`cavern-bombs`): the west Bomb Flower lifted and put down at the
    /// entrance wall, which its blast breaks (the cavern's opening, `gDcOpeningCs`); a bomb out of
    /// the bag put down by the east flower, which its blast sets off.
    Bombs,
    /// Darunia (GAME-09 milestone 2), from Goron City's bottom floor (`ranch-epona`): Zelda's
    /// Lullaby at his door, Saria's Song to him (`gGoronCityDaruniaCorrectSongCs`), the Goron's
    /// Bracelet.
    Darunia,
    /// The boulder at Dodongo's Cavern's mouth (GAME-09 milestone 2), from a debug start on the
    /// cliff above it with the bracelet (`goron-bracelet`): the cliff's Bomb Flower lifted and
    /// thrown down onto it (one-point 4180), the boulder blown up.
    Boulder,
    /// Dodongo's Cavern's bomb bag (GAME-09 milestone 3), from a debug start in the lobby's east
    /// corridor (`cavern-loop`: the loop through rooms 9, 10, 3, 12, 1 and 4 done): the big chest
    /// `En_Box` 0x0644 opened, `GI_BOMB_BAG_20`.
    BombBag,
    /// Dodongo's Cavern's boss path (GAME-09 milestone 3), from a debug start in room 7's alcove
    /// corridor north of the gravestone (`cavern-jaw`: the skull's jaw open): the gravestone
    /// (`Bg_Haka`) pulled off switch 0x20 (`Obj_Switch` 0x2000), the switch pressed, door 20, and
    /// the fall into room 16's pit to King Dodongo's room.
    Gravestone,
    /// Dodongo's Cavern's shortcut pillar (GAME-09 milestone 3), from a debug start on the lobby's
    /// island by its Bomb Flower (`cavern-lobby`: the entrance wall broken): the flower carried off the island's south-east corner onto the shelf by the pit's
    /// boulder (`Obj_Bombiwa` 0x000E), its blast, the floor switch under it (0x0A), one-point 3060
    /// and the pillar (`Bg_Ddan_Jd` 0x000A) shuttling to the upper level.
    Pillar,
    /// Dodongo's Cavern's skull (GAME-09 milestone 3), from a debug start on the bridge over it
    /// (`cavern-bomb-bag`: the loop and the bag): bombs onto C-Right in the pause menu, a bomb put
    /// down through each of the bridge's holes, the bomb catcher (`Bg_Dodoago`: fuse 50), each
    /// eye lit (one-point 3065), the jaw open (3380).
    Skull,
    /// King Dodongo's room (GAME-09 milestone 4), from its spawn (`cavern-boss`): a bomb (C-Right)
    /// put down on the cracked floor (`Bg_Breakwall` 0x4001), its blast, the drop through the hole
    /// into the arena and King Dodongo's intro (`BossDodongo_IntroCutscene`) to the fight's start.
    /// No fight: that's the hand test's.
    KingDodongo,
    /// King Dodongo's room after his defeat (GAME-09 milestone 4), from a debug start on the
    /// arena's east rim (`cavern-boss-beaten`: his init leaves the warp, the heart container and
    /// the lava's cover): the heart container, the blue warp, the Goron's Ruby
    /// (`DoorWarp1_ChildWarpOut`'s `EVENTCHKINF_25`, `Item_Give`), `gDMTGoronRubyCs` on Death
    /// Mountain Trail's layer 5 to its terminator, and the trail's spawn 5: Phase 10's exit.
    Ruby,
    /// The Great Fairy's wall on Death Mountain's summit (GAME-09 milestone 6), from a debug start
    /// in front of it with bombs (`goron-ruby`): a bomb (C-Right) put down by the wall
    /// (`Bg_Bombwall` 0x8003), its blast, and in through the hole (exit 5,
    /// `ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_0`). The falling rocks and the Skullwalltulas on the way up
    /// are the hand test's.
    FairyWall,
    /// The Great Fairy's fountain (GAME-09 milestone 6), from its spawn (`fairy-fountain`: the wall
    /// broken): Zelda's Lullaby on the Triforce (`En_Okarina_Tag` 0x1CB8, switch 0x38), her
    /// cutscene (`gGreatFairyMagicCs`: the orange warp at Link, the magic), and out by the return
    /// entrance (exit 0x7FFE, spawn 0: `ENTR_DEATH_MOUNTAIN_TRAIL_4`).
    GreatFairy,
    /// The summit's owl (GAME-09 milestone 6), from the fountain's return (`fairy-magic`: magic
    /// acquired): his talk (0x3063), his flight (`gDMTOwlCs`) to Kakariko
    /// (`ENTR_KAKARIKO_VILLAGE_14`).
    SummitOwl,
    /// Dodongo's Cavern's falling stairs (GAME-09 milestone 5), from a debug start in room 2 south
    /// of its Bomb Flowers (`fairy-magic`): a bomb (C-Right) put down by the rows' junction, both
    /// rows burning down to the stairs (`Bg_Ddan_Kd` 0x000B: two blasts 80 apart), one-point 3050
    /// and the fall: switch 0x0B.
    FallingStairs,
    /// Dodongo's Cavern's silver rupees (GAME-09 milestone 5), from a debug start on room 2's
    /// balcony with four of the five taken and the room cleared (`fairy-magic`; the rest are in
    /// crates, under a Beamos and in the air, and a Skulltula hangs over the balcony: the hand
    /// test's): the last one's crate bombed, the rupee taken, the tracker's flag 0x25 and door 17
    /// to room 5 unbarred with its camera.
    SilverRupees,
    /// Dodongo's Cavern's room 14 (GAME-09 milestone 5), from a debug start by its gravestone with
    /// the room cleared (its Poe the hand test's) on `cavern-stairs`: the gravestone (`Bg_Haka`)
    /// pulled and the chest behind it (`En_Box` 0x5541, `GI_SHIELD_HYLIAN`) opened.
    ShieldChest,
    /// Dodongo's Cavern's room 15 (GAME-09 milestone 5), from a debug start by its door with the
    /// room cleared (its Skulltulas the hand test's) on `cavern-song-of-time` (a child learns the
    /// song only after the three stones): the Song of Time by the nearest block (`Obj_Timeblock`
    /// 0x88FF, within 100): it goes.
    TimeBlocks,
    /// GAME-10 milestone 1: in Zora's Domain's shallows by the fish (a debug start, `zora-bottle`:
    /// the milk on C-Left), the milk drunk twice (half, then the bottle empty), a fish caught in the
    /// empty bottle with a swing, and let out again.
    Bottle,
    /// GAME-10 milestone 2: on Zora's River's platform by the falls (`ENTR_ZORAS_RIVER_2`'s spawn,
    /// `fairy-magic`), Zelda's Lullaby at the song spot (`En_Okarina_Tag` 0x07F8: switch 0x38),
    /// the falls open (`Bg_Spot03_Taki`, one-point 4100), through them into Zora's Domain and its
    /// intro.
    ZorasRiver,
    /// GAME-10 milestone 3: in King Zora's room (`ENTR_ZORAS_DOMAIN_1`, `zora-letter`: Ruto's
    /// letter on C-Left), the letter shown to him (0x401A, 0x401B, 0x401C), his mweep aside
    /// (`EVENTCHKINF_GAVE_LETTER_TO_KING_ZORA`) and the bottle empty.
    KingZora,
    /// GAME-10 milestone 3: in Zora's Fountain (`ENTR_ZORAS_FOUNTAIN_2`, `jabu-fish`: a fish on
    /// C-Left), the fish let out by Lord Jabu-Jabu (`En_Jj`), `gJabuInhalingCs`, and inside.
    JabuFish,
    /// GAME-10 milestone 4: Jabu-Jabu's room 0 (`ENTR_JABU_JABU_0`, `jabu-jabu`, the room marked
    /// cleared): the west cow's target shot with the slingshot (switch 0x15), door 1 into room 1.
    JabuCow,
    /// GAME-10 milestone 4: room 1 from the end of door 1's corridor (`jabu-room-1`, the room marked
    /// cleared): onto the elevator, down, across the pit's water, the yellow switch 0x19 (door 2).
    JabuElevator,
    /// GAME-10 milestone 4: room 14 marked cleared (`jabu-room-14`), in front of the boomerang's
    /// chest on its ledge: the chest opened (`GI_BOOMERANG`).
    JabuBoomerang,
    /// GAME-10 milestone 4: room 1's bottom east of the pit after the boomerang's chest
    /// (`jabu-boomerang`: the spout in front of door 14 down for good), through door 14 into room
    /// 14. Before the chest the spout pushes Link back short of the door.
    JabuDoor14,
    /// GAME-10 milestone 5: Jabu-Jabu's room 2 inside door 2 (`jabu-boomerang`, the room marked
    /// cleared): Ruto's first meeting (`gRutoFirstMeetingCs`, her drop through a hole), the bombs
    /// put on C-Down, the cow's boulders blown up from below, its target shot (switch 0x13: door
    /// 4) and through door 4 into room 7.
    JabuRuto,
    /// GAME-10 milestone 5: room 11 marked cleared (`jabu-room-7`: its chest out, its torch lit by
    /// the room's timer), inside door 7: the chest's Deku Stick put on C-Left, lit at the golden
    /// torch, carried through door 7 to room 7's web (`Bg_Ydan_Sp` 0x1FD6), which burns.
    JabuWeb,
    /// GAME-10 milestone 5: room 1 in front of door 9 with the tentacles' flags set
    /// (`jabu-crystal`: the red barrier gone): through door 9 into room 5, its crystal switch shot
    /// from the pool (switch 0x14), door 0 into room 15, whose floor is exit 2: the fall into
    /// Barinade's room (`ENTR_JABU_JABU_BOSS_0`).
    JabuCrystal,
    /// GAME-10 milestone 6: Barinade's room as Link falls in (`jabu-barinade`): his intro
    /// (`BossVa_BodyIntro`: the door shut, the Bari called, the title card) to the fight's start.
    /// No fight: that's the hand test's.
    Barinade,
    /// GAME-10 milestone 6: Barinade's room after his defeat (`barinade-beaten`, a debug start: his
    /// cleared room's init leaves Ruto, who rises through her blue warp, and the heart container):
    /// the heart container, into Ruto's warp, her words, the warp out (`EVENTCHKINF_37`, the Zora's
    /// Sapphire), `gZorasFountainSapphireCs` on Zora's Fountain's layer 4 to its terminator, and the
    /// fountain: Phase 11's exit, part 1.
    Sapphire,
    /// GAME-10 milestone 7: Hyrule Field with the three stones (`zora-sapphire`, a debug start short
    /// of the drawbridge): onto `Bg_Spot00_Hanebasi`'s trigger, Zelda's escape
    /// (`gHyruleFieldZeldaEscapeCs`, layer 5: Impa throws the Ocarina of Time into the moat) to its
    /// terminator, and the field's layer 1 again.
    Escape,
    /// GAME-10 milestone 7: from the moat's edge after the escape (`zelda-fled`): into the water, the
    /// dive to the Ocarina of Time (`Item_Ocarina` 0x0003), its text, `gHyruleFieldZeldaSongOfTimeCs`,
    /// the Temple of Time's `gTempleOfTimeSongOfTimeCs` (the Song of Time taught and played back),
    /// and Hyrule Field after `gHyruleFieldGetOoTCs`: Phase 11's exit, part 2 (Phase 12's start).
    OcarinaOfTime,
    /// GAME-10 milestone 8: Jabu-Jabu's room 3 by Ruto (`jabu-room-7`, the room marked cleared):
    /// her two talks (0x404C; 0x404D, she sits and asks to be carried), lifted, carried into the
    /// pool onto the yellow switch 0x3A, up its spout, the hop to the west ledge, down its south
    /// slope and onto the blue switch 0x3C, put down there: door 12 opens.
    JabuSwitch,
    /// GAME-10 milestone 8: room 3 by Ruto with the green tentacle's flag 0x0C (a debug start's:
    /// room 3's green barrier gone): her talks, lifted, carried up the spout to the west ledge and through
    /// door 21 into room 6 (`gRutoFoundSapphireCs`), thrown onto Big Octo's platform
    /// (`gRutoObtainingSapphireCs`: the Sapphire hers), the platform's rise (one-point 3070), Big
    /// Octo's arrival (Ruto taken: `INFTABLE_RUTO_ABDUCTED`) and the descent with him (3080) to
    /// the fight's start. No fight: that's the hand test's.
    JabuOcto,
    /// Zora's River's frogs (GAME-10 milestone 9, `ocarina-c` on their log): the six songs played
    /// to them in turn (each one's purple rupee, the Song of Storms' piece of heart), then their
    /// song game (0x40AB: the fourteen notes of `gFrogsSongPtr`, its piece of heart).
    Frogs,
    /// Lake Hylia's Bonooru (`En_Kakasi3`, GAME-10 milestone 9, `ocarina-c` in front of him): the
    /// ocarina out to him (0x40A4), eight new notes recorded (the spawn song set: 0x40A5), then
    /// talked to: he plays it back (0x40A0).
    Bonooru,
    /// The Master Sword pulled (GAME-11 milestone 1, `master-sword`: the child on the pedestal's
    /// step in room 0, entered by the hall): `gPullMasterSwordCs`, terminator 8 (the age changes: the
    /// equips swapped), the adult's arrival on the pedestal at `ENTR_TEMPLE_OF_TIME_2`.
    MasterSwordPull,
    /// The Master Sword put back (GAME-11 milestone 1, `master-sword-adult`, `MasterSwordPull`'s end:
    /// the adult arriving on the pedestal at `ENTR_TEMPLE_OF_TIME_2`): `gPlaceMasterSwordCs`,
    /// terminator 8, the child's arrival.
    MasterSwordBack,
    /// The Door of Time (GAME-11 milestone 2, `ocarina-of-time` at the Temple's entrance,
    /// `ENTR_TEMPLE_OF_TIME_0`): its intro (`gTempleOfTimeIntroCs`), the Ocarina of Time put on
    /// C-Down from the item page, the Song of Time at the altar (`En_Okarina_Tag` 4) and
    /// `gDoorOfTimeOpeningCs`. Its end is `door-of-time`.
    DoorOfTime,
    /// The first pull to the Chamber (`door-of-time`, from the Temple's entrance): round the altar,
    /// through the doorway into room 0, the reveal (`gRevealMasterSwordCs`), the pull
    /// (`gPullMasterSwordCs`, terminator 8's first time: the cutscene map), Ganondorf (terminator
    /// 25: the age), the Chamber of the Sages (Rauru, the Light Medallion) to terminator 18's
    /// transition. Its end is `light-medallion`.
    FirstPull,
    /// The awakening (`light-medallion`, `ENTR_TEMPLE_OF_TIME_4`): `En_Xc` 9 starts
    /// `gTempleOfTimeFirstAdultCs` (Navi, Sheik), the return's time warp shrinks
    /// (`EVENTCHKINF_C9`). Its end is `adult-awake`, Phase 13's start.
    Awakening,
    /// The way out (`adult-awake`, a debug start in room 0 where the awakening leaves Link): back
    /// through the doorway and the hall, out by exit 1 (`ENTR_TEMPLE_OF_TIME_EXTERIOR_DAY_1`: the
    /// ruins' `shrine_r`). Phase 12's exit.
    TempleExit,
    /// The ruins (`adult-awake`, outside the Temple where `TempleExit` ends, `shrine_r`): into the
    /// ruined market, round its eight ReDeads (none within 150, so none hears him), through the
    /// ruined market entrance (`enrui`) to the field by the broken drawbridge
    /// (`ENTR_HYRULE_FIELD_7`, `Bg_Spot00_Break`).
    Ruins,
    /// The Prelude of Light (GAME-11 milestone 3, `prelude`: a debug start, the adult with the
    /// Forest Medallion's `EVENTCHKINF_48`, at the Temple's entrance): down the hall into room 0,
    /// `En_Xc` 9 gives the Prelude and plays `gTempleOfTimePreludeCs` (Sheik's harp, the song
    /// played back). Its end is `prelude-learned`.
    Prelude,
    /// The sword put back in the story (`prelude-learned`, a debug start where the Prelude leaves
    /// Link in room 0): `gPlaceMasterSwordCs`, terminator 8, a child at `ENTR_TEMPLE_OF_TIME_2`.
    /// Its end is `child-again`.
    SwordBack,
    /// The sword pulled again (`child-again`, the child arriving on the pedestal):
    /// `gPullMasterSwordCs`, terminator 8, an adult at `ENTR_TEMPLE_OF_TIME_2`. Its end is
    /// `prelude-learned` again.
    SwordAgain,
    /// The Prelude's warp (`prelude-learned`, in Hyrule Field by the broken drawbridge): the song on
    /// the ocarina, "warp?" yes, the warp-out (Link hidden in its white-out), the arrival on the
    /// Temple of Time's light pad (`ENTR_TEMPLE_OF_TIME_7`, start mode 5: hidden until the warp-in's
    /// script). Its end is `prelude-learned`.
    PreludeWarp,
    /// Riding Epona in Lon Lon Ranch (`adult-awake` by day, in the corral): Epona fleeing
    /// (`EnHorse_FleePlayer`) until Epona's Song, then coming to Link; mounted from her left; a walk,
    /// a trot and a gallop with a spur, over one of the corral's fences (`Bg_Umajump`: a high jump),
    /// halted, got off. Its end is `adult-awake` (no flags).
    RideEpona,
    /// Riding Epona in Hyrule Field (the debug `epona`, Epona had; `ENTR_HYRULE_FIELD_6`, a debug
    /// start at the foot of the ranch's ramp): Epona's Song (she comes from where she was left),
    /// mounted, ridden up the ramp and into the ranch through its gate on her (`R_EXITED_SCENE_RIDING_HORSE`: the ranch's start
    /// spawns her under Link), halted, got off. Its end is `epona`.
    FieldRide,
    /// Ingo's rental (`adult-awake`, a debug start by Ingo at the corral's gate): 0x2030, "Ride",
    /// "Don't listen", 10 Rupees (0x205C); the corral (`ENTR_LON_LON_RANCH_6`, the minute's timer);
    /// Epona's Song, on her (`INFTABLE_AB`). Ends mounted, the timer running (the user's call: the
    /// rental's minute isn't waited out).
    IngoRental,
    /// The race offered and taken (`ingo-rental`): 0x2031, the rental paid; the corral, Epona's
    /// Song, on her, ridden up to Ingo and 0x2038 from horseback, "Do it"; the race's intro
    /// (`gLonLonRanchRaceIntroCs`) to the race's start. The race itself is the hand test's.
    IngoRace,
    /// The race lost (`ingo-race-lost`, at `ENTR_LON_LON_RANCH_8`): Ingo gloats at once (0x203D),
    /// -50 Rupees, the rematch offered. Its end: race state `INGO_RACE_STATE_REMATCH`, 0 Rupees.
    IngoLost,
    /// The first race won (`ingo-first-win`, at `ENTR_LON_LON_RANCH_7`): Ingo at once (0x203A),
    /// the second race called (the transition to its intro).
    IngoWon,
    /// Epona won and out (`ingo-second-win`, at `ENTR_LON_LON_RANCH_7` on Epona): Ingo's gate
    /// scene (0x203B, 0x203C, the gates swinging shut), then a gallop over the shut gate, past
    /// Ingo (`EnIn_GuardingGate`: `ENTR_HYRULE_FIELD_15`), the field's jump cutscene. Its end is
    /// `epona-won` (Phase 14's start).
    EponaWon,
    /// Malon's obstacle course offered and started (`epona-won`, a debug start by Malon at the
    /// corral's gate): 0x2000 (her thanks), Epona's Song (Epona near: `Actor_FindNearby` within
    /// 1,200), 0x2002 into 0x2003, "Let's go", 0x2092; the course's intro (layer 4) to its start.
    /// The laps are the hand test's (the user's call 6).
    MalonCourse,
    /// The course just won in 45 seconds, the first time (`malon-record`, at
    /// `ENTR_LON_LON_RANCH_7`): Malon at once (0x208F), the cow's unlock, the high score 45.
    MalonRecord,
    /// Adult Malon singing in the ranch by night before Epona (`adult-awake` at 22:00, a debug
    /// start before her): Epona's Song played to her, then 0x2051 into 0x2052.
    MalonSong,
    /// Hyrule Field's Big Poe (`epona-won`, `FieldRide`'s debug start at the foot of the ranch's
    /// ramp): Epona's Song, on her, ridden to the place by the ramp (`En_Po_Field` 0x17); the Big
    /// Poe appears ahead and flees. The fight and the soul's bottle are the hand test's.
    BigPoe,
}

/// `Route::Meadow`'s tasks. The Lost Woods' rooms are clearings about 400 across on a grid of
/// 800, joined by corridors under hollow logs, with each room's `En_Holl` plane half way along
/// its corridor; a wrong turn's corridor ends in exit 1 (back to Kokiri Forest). The way to the
/// meadow (from the scene's planes): room 0 at (0, 0), east to room 1 (800, 0), north to room 2
/// (800, -800), east to room 3 (1600, -800), north to room 4 (1600, -1600) and room 7 (1600,
/// -2400), west to room 8 (800, -2400), and north out of it (exit 2: `ENTR_SACRED_FOREST_MEADOW_0`).
/// The meadow's Mad Scrubs (`En_Dekunuts`) along the way, in the order Link meets them (homes).
const MEADOW_SCRUBS: [Vec3; 6] = [
    Vec3::new(289.0, 0.0, 1183.0),
    Vec3::new(-402.0, 0.0, 1177.0),
    Vec3::new(374.0, 0.0, 578.0),
    Vec3::new(-303.0, 0.0, 316.0),
    Vec3::new(24.0, 160.0, -947.0),
    Vec3::new(-38.0, 160.0, -1255.0),
];

fn meadow_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::Walk(vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(400.0, 0.0, 0.0),
            Vec3::new(800.0, 0.0, 0.0),
            Vec3::new(800.0, 0.0, -400.0),
            Vec3::new(830.0, 0.0, -700.0),
        ]),
        // Room 2's owl (En_Owl at (955, 210, -798)) calls Link as he comes in: 0x10C0, then 0x10C1
        // ("hear it again?"): no.
        Task::TalkThrough(vec![1], Step::Owl),
        Task::Walk(vec![
            Vec3::new(800.0, 0.0, -800.0),
            Vec3::new(1200.0, 0.0, -800.0),
            Vec3::new(1600.0, 0.0, -800.0),
            Vec3::new(1600.0, 0.0, -1200.0),
            Vec3::new(1600.0, 0.0, -1600.0),
            Vec3::new(1600.0, 0.0, -2000.0),
            Vec3::new(1600.0, 0.0, -2400.0),
            Vec3::new(1200.0, 0.0, -2400.0),
            Vec3::new(800.0, 0.0, -2400.0),
        ]),
        Task::Exit("ENTR_SACRED_FOREST_MEADOW_0", Some(Step::MeadowEntered)),
        // The Wolfos (En_Wf 0x1F00) by the entrance, buried until Link is within 240.
        Task::FightWolfos(Vec3::new(-345.0, 0.0, 1658.0), 0x1F, Step::WolfosKilled),
        // The gate (Bg_Spot05_Soko 1 at (-200, 0, 1613)) sinks with its attention camera; then the
        // maze's way (its hedges stand 120 high on a floor at 0): north through the gate, the
        // zigzag to the foot of the ramp at x 0, and up it (to 470 at z -2200).
        Task::Settle(None),
        // The maze's six Mad Scrubs (En_Dekunuts) spit at Link 160 to 480 away: at each Link stops
        // facing it, the shield up until its nut bounces back off it (EnDekunuts_BeginRun); it
        // runs off, and he runs after it and kills it.
        Task::Walk(vec![Vec3::new(-200.0, 0.0, 1475.0), Vec3::new(300.0, 0.0, 1475.0), Vec3::new(300.0, 0.0, 1420.0)]),
        Task::BounceNut(MEADOW_SCRUBS[0]),
        Task::ChaseScrub(MEADOW_SCRUBS[0]),
        Task::Walk(vec![Vec3::new(300.0, 0.0, 1175.0), Vec3::new(-60.0, 0.0, 1175.0)]),
        Task::BounceNut(MEADOW_SCRUBS[1]),
        Task::ChaseScrub(MEADOW_SCRUBS[1]),
        Task::Walk(vec![Vec3::new(-400.0, 0.0, 1175.0), Vec3::new(-400.0, 0.0, 875.0), Vec3::new(100.0, 0.0, 875.0), Vec3::new(100.0, 0.0, 578.0), Vec3::new(170.0, 0.0, 578.0)]),
        Task::BounceNut(MEADOW_SCRUBS[2]),
        Task::ChaseScrub(MEADOW_SCRUBS[2]),
        // (The chase ends south of the corridor at z 575, in the one along x 400.)
        // The fourth stands at (-303, 0, 316) down the corridor along x -300, whose north end is a
        // dip 23 deep (a Bg_Spot05_Soko block's place): its nut breaks on the dip's edge and never
        // comes back, so Link runs past it (within 120 it burrows, and only rises 100 to 150 frames
        // later).
        Task::Hurry(vec![Vec3::new(400.0, 0.0, 575.0), Vec3::new(-300.0, 0.0, 575.0), Vec3::new(-300.0, 0.0, -125.0)]),
        // East to the foot of the ramp along x 0, and up it at a run past the last two scrubs (on its
        // slope their nuts don't come back off the shield; they hit three times).
        Task::Hurry(vec![Vec3::new(0.0, 0.0, -125.0), Vec3::new(0.0, 100.0, -620.0), Vec3::new(0.0, 160.0, -1300.0), Vec3::new(0.0, 420.0, -2100.0)]),
        // On, north of z -2220, Saria (En_Sa, waiting with the letter and without her song) starts
        // her lesson (gMeadowSariasSongCs: EnSa_AwaitTeachingSariasSong).
        Task::Hurry(vec![Vec3::new(0.0, 480.0, -2260.0)]),
        Task::Texts(|w| w.cs_ctx.state != oot_game::cutscene::CS_STATE_IDLE, Step::SariasSongLesson),
        // Her texts, the song shown (TEACH), then played back (PLAYBACK).
        Task::Playback(oot_game::audio::ocarina::OCARINA_SONG_SARIAS, false, Step::SariasSongPlayed),
        // The script's end (its MISC 0x0C): EnSa_BecomeIdle gives the song (Item_Give(ITEM_SONG_SARIA)).
        Task::Texts(
            |w| w.save.check_quest_item(oot_game::item::QUEST_SONG_SARIA) && w.cs_ctx.state == oot_game::cutscene::CS_STATE_IDLE && w.player().cs_mode == 0,
            Step::SariasSongLearned,
        ),
    ]
}

/// Where `Route::LostWoods` aims the slingshot: the target's centre.
const TARGET_AIM: Vec3 = Vec3::new(1340.0, 50.0, -30.0);

/// The target scrub's (`En_Dnt_Nomal`) bullseyes in a row, if it's there.
fn target_hits(w: &PlayState) -> i16 {
    w.actors.all().into_iter().find_map(|h| w.actors.downcast::<crate::en_dnt_nomal::EnDntNomal>(h)).map(|d| d.hit_counter).unwrap_or(0)
}

/// `Route::LostWoods`'s tasks. Room 9 is west of room 0 (its plane at (-400, 0, 0)): a flat
/// clearing with the Skull Kid (`En_Skj` type 0) on a stump 80 high at (-1132, 80, -128), and the
/// small stump Link stands on (type 5) 20 high at (-951, 20, 52), 255 from him (before
/// `ITEMGETINF_16` he flees from Link within 200). Room 1, east of room 0: a clearing at 0 to
/// x 1000, then a hollow 180 below it holding the memory game's stumps (type 6, Link's, at
/// (1123, -140, 340), with the two Skull Kids' beside it); the target scrub's ledge (80 high) and
/// its target past the hollow's north-east.
fn lost_woods_tasks() -> Vec<Task> {
    use oot_game::audio::ocarina::OCARINA_SONG_SARIAS;
    vec![
        Task::Settle(None),
        // West into room 9 and onto the small stump, keeping 200 from the Skull Kid.
        Task::Walk(vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(-400.0, 0.0, 0.0), Vec3::new(-800.0, 0.0, 30.0), Vec3::new(-920.0, 0.0, 50.0), Vec3::new(-951.0, 20.0, 52.0)]),
        // On it (EnSkj_RangeCheck: within 26, not below), the ocarina out: Link centred on the
        // stump and the check (OCARINA_ACTION_CHECK_SARIA); Saria's Song: 0x10BB and the heart.
        Task::Ocarina(1, OCARINA_SONG_SARIAS, Step::SkullKidSong),
        Task::Texts(|w| w.save.get_item_get_inf(crate::en_skj::ITEMGETINF_16), Step::SkullKidHeart),
        // Back east through room 0 into room 1, to its clearing's edge over the hollow.
        Task::Walk(vec![Vec3::new(-800.0, 0.0, 0.0), Vec3::new(-400.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(400.0, 0.0, 0.0), Vec3::new(800.0, 0.0, 0.0), Vec3::new(960.0, 0.0, -30.0)]),
        // The target (EnDntNomal_TargetWait's quad at (1340, 50, -30), ±24) across the hollow:
        // three seeds within 8 of its centre in a row, from the clearing's edge (a shot almost
        // level: from the hollow's floor, aimed up, a seed's edge crosses it about 12 low). The
        // third starts the scrub's
        // camera (4140) with Link held; up comes the scrub, says 0x10AF and spits the bag
        // (En_Ex_Item), which flies at the camera and is offered: the bigger seed bag.
        Task::DrawSlingshot(Step::SlingshotDrawn),
        Task::ShootUntil(TARGET_AIM, |w| target_hits(w) >= 1, Step::Bullseye),
        Task::ShootUntil(TARGET_AIM, |w| target_hits(w) >= 2, Step::Bullseye),
        Task::ShootUntil(TARGET_AIM, |w| target_hits(w) >= 3, Step::Bullseye),
        Task::Texts(|w| w.save.get_item_get_inf(crate::en_dnt_nomal::ITEMGETINF_1D), Step::BulletBag),
        Task::Walk(vec![Vec3::new(960.0, 0.0, -175.0)]),
        // Down into the hollow: the clearing's east edge at x 1000 has a wall 25 high from z -150
        // to 200, but none at its north end; across the hollow's floor (-180) to the memory
        // game's stump (40 high).
        Task::Hurry(vec![Vec3::new(1050.0, -180.0, -175.0)]),
        Task::Walk(vec![Vec3::new(1060.0, -180.0, 250.0), Vec3::new(1100.0, -180.0, 340.0)]),
        Task::ClimbTo(Vec3::new(1123.0, -140.0, 340.0), Step::MemoryGame),
        // Its middle: EnSkj_RangeCheck wants Link within 26 of the stump's centre, not below it.
        Task::Walk(vec![Vec3::new(1123.0, -140.0, 340.0)]),
        Task::MemoryGame(Step::MemoryGameWon),
    ]
}

/// `Route::Ranch`'s tasks. The ranch (from the collision): its gate's lane runs south from the
/// spawn (1107, 0, -3740) between Talon's house and the stable (x 950 to 1050), out onto the
/// field at z -2400; the corral's fence is an oval round the middle (path 0, the galloping
/// horse's, runs inside it), its gate on the north side at x -300 to -150. Malon (`En_Ma1`
/// rot.z 3) stands at (64, 0, -567) by the foal (-4, 0, -572).
fn ranch_tasks() -> Vec<Task> {
    use oot_game::audio::ocarina::OCARINA_SONG_EPONAS;
    vec![
        // The ranch's intro (gLonLonRanchIntroCs, EVENTCHKINF_B4).
        Task::Settle(None),
        Task::Walk(vec![Vec3::new(975.0, 0.0, -3625.0), Vec3::new(975.0, 0.0, -2550.0), Vec3::new(800.0, 0.0, -2375.0), Vec3::new(700.0, 0.0, -2375.0), Vec3::new(-150.0, 0.0, -1525.0), Vec3::new(-150.0, 0.0, -875.0)]),
        // Malon's three texts (EnMa1_GetTextId at the ranch after Talon's return): 0x2047 (sets
        // EVENTCHKINF_TALKED_TO_CHILD_MALON_AT_RANCH), 0x2048 (INFTABLE_TOLD_EPONA_IS_SCARED), 0x2049
        // (EVENTCHKINF_CAN_LEARN_EPONAS_SONG).
        Task::Talk(Who::Malon, Vec3::new(75.0, 0.0, -650.0), Step::Malon),
        Task::Talk(Who::Malon, Vec3::new(75.0, 0.0, -650.0), Step::Malon),
        Task::Talk(Who::Malon, Vec3::new(75.0, 0.0, -650.0), Step::Malon),
        // The ocarina out by her (EnMa1_IdleTeachSong: within 30 + her collider's radius she sets
        // PLAYER_STATE2_23, and with it out PLAYER_STATE2_24): 0x2061, then she plays the song
        // (OCARINA_ACTION_TEACH_EPONA) and Link plays it back (_PLAYBACK_EPONA).
        Task::OcarinaOut(1, Step::OcarinaOut),
        Task::Playback(OCARINA_SONG_EPONAS, false, Step::EponasSongPlayed),
        // EnMa1_WaitForPlayback: OCARINA_MODE_03 reloads the ranch with CS_INDEX_1 (layer 5).
        Task::Texts(|w| w.scene_id == SCENE_LON_LON_RANCH && w.save.scene_layer == 5, Step::EponasSongCs),
        // Its terminator, destination 94: ENTR_LON_LON_RANCH_1, a white fade.
        Task::Texts(|w| w.scene_id == SCENE_LON_LON_RANCH && w.save.scene_layer != 5 && w.transition.trigger == TRANS_TRIGGER_OFF && w.transition.mode == TRANS_MODE_OFF, Step::RanchFromSong),
        Task::Settle(None),
        // EnHorseLinkChild entered by ENTR_LON_LON_RANCH_1: a neigh, and she comes to Link
        // (func_80A6A724, action 4) for 300 frames.
        Task::Texts(|w| foal_near(w, 4), Step::FoalCame),
        // Away from her home (more than 250: she doesn't run from him), until she has gone home
        // and faces him there (func_80A6A4DC, action 5).
        Task::Walk(vec![Vec3::new(-150.0, 0.0, -875.0), Vec3::new(-150.0, 0.0, -1100.0)]),
        Task::Texts(|w| foal(w).is_some_and(|f| f.action == 5), Step::FoalHome),
        // Epona's Song (R_EPONAS_SONG_PLAYED): she neighs and comes again.
        Task::Ocarina(1, OCARINA_SONG_EPONAS, Step::EponasSong),
        Task::Texts(|w| foal_near(w, 4), Step::FoalCalled),
        // Back out of the corral by its gate, north to the gate's lane, and up it to Talon's
        // house's door (En_Door at (1020, 0, -3100), facing -x: DOOR_SCENEEXIT, the floor
        // behind it exit 5, ENTR_LON_LON_BUILDINGS_0).
        Task::Walk(vec![Vec3::new(-150.0, 0.0, -1525.0), Vec3::new(700.0, 0.0, -2375.0), Vec3::new(800.0, 0.0, -2375.0), Vec3::new(975.0, 0.0, -2550.0), Vec3::new(975.0, 0.0, -3000.0)]),
        Task::OpenDoor(Vec3::new(985.0, 0.0, -3100.0), Vec3::new(1020.0, 0.0, -3100.0), None),
        Task::NewScene(Step::TalonsHouse),
        // Talon (En_Ta at (1054, 0, -29), sitting asleep at his table): 0x207E the first time
        // (EnTa_IdleSittingInLonLonHouse), its choice yes (EnTa_WaitForPlayCuccoGameResponse:
        // 10 rupees), 0x2080 (EnTa_StartCuccoGame): he throws his three super cuccos, and the
        // timer starts at 30 (EnTa_StartingCuccoGame3: Interface_SetTimer(30)).
        Task::Converse(Who::Talon, Vec3::new(1050.0, 0.0, 60.0), vec![0], |w| w.save.timer_state != oot_game::save::TIMER_STATE_OFF && w.player().cs_mode == 0, Step::CuccoGame),
        // Each held 7 frames: 0x2082, 0x2083, and the last 0x2084 (EnTa_RunCuccoGame:
        // EVENTINF_CUCCO_GAME_WON), whose event sends Link to ENTR_LON_LON_BUILDINGS_2.
        Task::CatchSuperCucco(Step::SuperCucco),
        Task::CatchSuperCucco(Step::SuperCucco),
        Task::CatchSuperCucco(Step::SuperCucco),
        // Back at ENTR_LON_LON_BUILDINGS_2 (by Talon), his room loaded: he speaks while the fade
        // is still on.
        Task::Texts(|w| w.entrance_by_name("ENTR_LON_LON_BUILDINGS_2") == Some(w.save.entrance_index) && w.actors.all().into_iter().any(|h| w.actors.downcast::<crate::en_ta::EnTa>(h).is_some()), Step::CuccoGameOver),
        // EnTa_IdleAfterCuccoGameFinished: he talks to Link at once (0x2086), the choice (either:
        // EnTa_WaitForMarryMalonResponse), 0x2087, and the bottle of milk (GI_BOTTLE_MILK_FULL,
        // ITEMGETINF_TALON_BOTTLE).
        Task::Converse(Who::Talon, Vec3::new(1050.0, 0.0, 12.0), vec![0], |w| w.save.get_item_get_inf(ITEMGETINF_TALON_BOTTLE) && w.player().action == PA::StandingStill, Step::MilkBottle),
    ]
}

/// `ITEMGETINF_TALON_BOTTLE` (`save.h`: 0x02).
const ITEMGETINF_TALON_BOTTLE: u16 = 0x02;

/// Dodongo's Cavern's lobby's two Bomb Flowers by its spawn (`ddan` room 0, on the strips 20 up
/// either side of the entrance).
const DC_WEST_FLOWER: Vec3 = Vec3::new(-293.0, 20.0, 307.0);
const DC_EAST_FLOWER: Vec3 = Vec3::new(290.0, 20.0, 307.0);

fn bombs_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        // The west flower lifted (the bracelet: Player_GetStrength); its bomb, 140 on the fuse,
        // in Link's hands.
        Task::LiftBombFlower(DC_WEST_FLOWER, Step::FlowerLifted),
        // Carried north up the middle to the entrance wall (Bg_Breakwall 0x0007 at (0, 0, 20)) and
        // put down 55 from it, then back south, beyond the blast (80 at its tenth frame).
        Task::PutDown(vec![Vec3::new(-150.0, 0.0, 250.0), Vec3::new(0.0, 0.0, 160.0), Vec3::new(0.0, 0.0, 75.0)], Vec3::new(0.0, 0.0, 20.0), Step::PutDown),
        Task::Walk(vec![Vec3::new(0.0, 0.0, 330.0)]),
        // BgBreakwall_Wait: the explosion's hit, the first time: EVENTCHKINF_B0, gDcOpeningCs (its
        // title card), to its stop.
        Task::Texts(|w| w.save.get_event_chk_inf(crate::bg_breakwall::EVENTCHKINF_B0) && w.cs_ctx.state == oot_game::cutscene::CS_STATE_IDLE && !w.player_in_cs_mode(), Step::DcOpening),
        Task::Settle(None),
        // To the east flower's strip, a bomb out (C-Right), put down short of the flower, and away
        // west: its blast sets the flower off (EnBombf_GrowBomb: a hit, a bomb at once).
        Task::Walk(vec![Vec3::new(150.0, 0.0, 380.0), Vec3::new(260.0, 20.0, 380.0)]),
        Task::TakeBomb(2, Step::BombOut),
        Task::PutDown(vec![], DC_EAST_FLOWER, Step::PutDown),
        Task::Walk(vec![Vec3::new(150.0, 0.0, 380.0), Vec3::new(-100.0, 0.0, 380.0)]),
        Task::Texts(|w| flower_near(w, DC_EAST_FLOWER).is_some_and(|f| f.flower_bomb_scale < 1.0), Step::FlowerSetOff),
    ]
}

/// `Route::Darunia`'s tasks: from Goron City's bottom floor (`ENTR_GORON_CITY_2`, no intro) to the
/// lullaby's spot by Darunia's door, the door, Darunia and the bracelet. The ways are a search over
/// the collision (a 25 grid, floors by `BgCheck_EntityRaycastDown`, walls tested 20 and 45 up).
fn darunia_tasks() -> Vec<Task> {
    use oot_game::audio::ocarina::{OCARINA_SONG_LULLABY, OCARINA_SONG_SARIAS};
    vec![
        Task::Settle(None),
        // South to the lullaby's spot (En_Okarina_Tag 0x1C9B at (-71, -2, -333), range 40: the
        // ocarina within 90 across and 40 up or down turns to it).
        Task::Walk(vec![Vec3::new(-94.0, -3.0, -200.0), Vec3::new(-71.0, -3.0, -300.0)]),
        Task::Ocarina(1, OCARINA_SONG_LULLABY, Step::LullabyPlayed),
        Task::WaitFor(Cond::DaruniaDoorOpen, Step::DaruniaDoor),
        // Through the door (En_Holl 0x003F at (-69, 31, -589), rooms 3 and 1) to Darunia (En_Du at
        // (91, 41, -1462)): within 136 he wants the ocarina.
        Task::Walk(vec![Vec3::new(-94.0, -3.0, -496.0), Vec3::new(-94.0, -3.0, -696.0), Vec3::new(-94.0, 40.0, -1096.0), Vec3::new(-94.0, 40.0, -1296.0), Vec3::new(40.0, 40.0, -1380.0)]),
        // Saria's Song: gGoronCityDaruniaCorrectSongCs, then 0x301C and the bracelet
        // (Actor_OfferGetItem(GI_GORONS_BRACELET)), A through the texts.
        Task::Ocarina(1, OCARINA_SONG_SARIAS, Step::SariasSong),
        Task::Texts(|w| oot_game::player_lib::player_get_strength(&w.save) > oot_game::player_lib::PLAYER_STR_NONE && !w.player_in_cs_mode(), Step::Bracelet),
        Task::Settle(None),
    ]
}

/// `Route::Boulder`'s tasks, from its debug start on the cliff above Dodongo's Cavern's mouth
/// (`goron-bracelet`): the cliff's Bomb Flower lifted with the bracelet (`Player_GetStrength`) and
/// thrown at a run off the ledge; its blast breaks the boulder.
fn boulder_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::LiftBombFlower(DMT_CLIFF_FLOWER, Step::FlowerLifted),
        // Back east, then at a run west off the ledge's edge (x about -1440): thrown from x -1413 it
        // falls below y 1400 inside the boulder's watch (Bg_Spot16_Bombstone func_808B57E0: within
        // 400 of (-1579, -790)), one-point 4180 for its fuse and 20, and lands by the boulder.
        Task::ThrowAtRun(vec![Vec3::new(-1300.0, 1409.0, -646.0), Vec3::new(-1600.0, 1408.0, -646.0)], Vec3::new(-1413.0, 1408.0, -646.0), Step::Thrown),
        // Its blast breaks the boulder (switch 4, EVENTCHKINF_23, a second 4180).
        Task::Texts(|w| w.flags.get_switch(4) && w.save.get_event_chk_inf(crate::bg_spot16_bombstone::EVENTCHKINF_23) && !w.player_in_cs_mode() && w.active_cam_id == oot_game::camera::CAM_ID_MAIN, Step::BoulderBlown),
        Task::Settle(None),
    ]
}

/// The Bomb Flower on the cliff above Dodongo's Cavern's mouth (spot16 room 0).
const DMT_CLIFF_FLOWER: Vec3 = Vec3::new(-1397.0, 1408.0, -646.0);

/// The bomb bag's chest (`ddan` room 0's `En_Box` 0x0644: a big chest, `GI_BOMB_BAG_20`, treasure
/// flag 4), in the lobby's east corridor (y 80).
pub const DC_BOMB_BAG_CHEST: Vec3 = Vec3::new(869.0, 80.0, -920.0);

/// Room 7's gravestone (`ddan` room 7's `Bg_Haka` 0x0000), at the mouth of the alcove's corridor
/// (y 126), on switch 0x20 (`Obj_Switch` 0x2000 at (-1, 126, -3220)).
pub const DC_ROOM7_GRAVESTONE: Vec3 = Vec3::new(0.0, 126.0, -3212.0);
/// Door 20 (the boss path's `Door_Shutter` 0x00A0, `SHUTTER_FRONT_SWITCH` on 0x20 from room 7):
/// its transition index and its front on room 7's side (it faces +x).
const DC_DOOR_20: usize = 20;
const DC_DOOR_20_FRONT: Vec3 = Vec3::new(-325.0, 16.0, -2784.0);

/// `Route::Gravestone`'s tasks: the stone pulled north (`BgHaka_Pull` slides it away from its
/// face, `shape.rot.y + 0x8000`: Link holds on at its north face and backs north), past it along
/// the corridor's east side (the stone 69 across in a corridor 100 wide) onto the switch, off the
/// corridor's mouth to room 7's floor, door 20, room 16's pit.
fn gravestone_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::PullStone(DC_ROOM7_GRAVESTONE, Step::StonePulled),
        Task::Walk(vec![Vec3::new(41.0, 136.0, -3305.0), Vec3::new(41.0, 136.0, -3218.0), Vec3::new(0.0, 126.0, -3219.0)]),
        Task::WaitFor(Cond::Switch(0x20), Step::BossPathOpen),
        Task::Walk(vec![Vec3::new(0.0, 126.0, -3150.0), Vec3::new(0.0, 16.0, -3050.0), Vec3::new(-250.0, 16.0, -2784.0)]),
        Task::OpenSlidingDoor(DC_DOOR_20, DC_DOOR_20_FRONT, Step::DoorOpened),
        Task::Exit("ENTR_DODONGOS_CAVERN_BOSS_0", Some(Step::BossFall)),
    ]
}

/// King Dodongo's room's cracked floor (`ddan_boss` room 1's `Bg_Breakwall` 0x4001,
/// `BWALL_KD_FLOOR`, switch flag 1), over the arena's north side.
pub const KD_FLOOR: Vec3 = Vec3::new(-890.0, -744.0, -2784.0);

/// `Route::KingDodongo`'s tasks, from the room's spawn (-440, -744, -2781) on the ledge over the
/// arena: west to the cracked floor, a bomb out (C-Right) put down on it, back east out of the
/// blast; its explosion breaks the floor (switch flag 1); onto the hole: the drop below y
/// -1223.76 starts King Dodongo's intro (the camera on his own sub camera, his walk in, the roar,
/// the title card), until it hands the camera back and the fight begins.
fn king_dodongo_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::Walk(vec![Vec3::new(-700.0, -744.0, -2784.0)]),
        Task::TakeBomb(2, Step::BombOut),
        // On the cracked floor (its blast's sphere reaches 72 at its tenth frame: the floor's quad is
        // 2 across at its middle).
        Task::PutDown(vec![Vec3::new(-855.0, -744.0, -2784.0)], KD_FLOOR, Step::PutDown),
        Task::Walk(vec![Vec3::new(-520.0, -744.0, -2784.0)]),
        Task::WaitFor(Cond::Switch(1), Step::FloorBombed),
        Task::Walk(vec![KD_FLOOR]),
        Task::Texts(
            |w| w.save.get_event_chk_inf(crate::boss_dodongo::EVENTCHKINF_BEGAN_KING_DODONGO_BATTLE) && !w.player_in_cs_mode() && w.active_cam_id == oot_game::camera::CAM_ID_MAIN,
            Step::KingDodongoBattle,
        ),
    ]
}

/// `Route::Ruby`'s tasks, from the arena's east rim after King Dodongo's defeat: the heart
/// container (its get-item text), the blue warp, the warp out to Death Mountain Trail
/// (`ENTR_DEATH_MOUNTAIN_TRAIL_0`, `CS_INDEX_1`: layer 5), `gDMTGoronRubyCs` with A through its
/// texts until its terminator (12) starts the transition, and the trail's spawn 5.
fn ruby_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::TakeHeart(Step::HeartTaken),
        Task::IntoWarp(Step::WarpEntered),
        Task::WarpOut(Step::WarpedOut),
        Task::WarpCutscene(4000, Step::RubyOver),
        Task::NewScene(Step::Trail5),
    ]
}

/// The Great Fairy's wall on Death Mountain's summit (spot16 room 0's `Bg_Bombwall` 0x8003, switch
/// flag 3); the trail's spawn 4 (-249, 3314, -4497), facing out along 5097, is where the
/// fountain's return entrance puts Link, in its hole.
pub const SUMMIT_WALL: Vec3 = Vec3::new(-245.0, 3305.0, -4480.0);
/// 150 out from spawn 4 along its facing (5097: (0.469, 0.883)).
pub const SUMMIT_WALL_FRONT: Vec3 = Vec3::new(-179.0, 3314.0, -4365.0);
/// The summit's owl (`En_Owl` 0x0213, `OWL_DEATH_MOUNTAIN`).
pub const SUMMIT_OWL: Vec3 = Vec3::new(-42.0, 3347.0, -4409.0);
/// The fountain's Triforce (`En_Okarina_Tag` 0x1CB8: Zelda's Lullaby, switch 0x38).
pub const FOUNTAIN_TRIFORCE: Vec3 = Vec3::new(-22.0, 10.0, -798.0);

/// `Route::FairyWall`'s tasks: a bomb out (C-Right) and put down 40 in front of the wall, back
/// off out of its blast; the wall breaks (switch flag 3); in through the hole (exit 5).
fn fairy_wall_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::TakeBomb(2, Step::BombOut),
        Task::PutDown(vec![Vec3::new(-226.0, 3314.0, -4445.0)], SUMMIT_WALL, Step::PutDown),
        Task::Walk(vec![Vec3::new(-150.0, 3314.0, -4310.0)]),
        Task::WaitFor(Cond::Switch(3), Step::FairyWall),
        Task::Exit("ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_0", Some(Step::Fountain)),
    ]
}

/// `Route::GreatFairy`'s tasks: north from the spawn onto the Triforce, Zelda's Lullaby (the ocarina
/// on C-Down; within 50 of the spot, `func_80ABEF2C`, it's the one the ocarina turns to); `Bg_Dy_Yoseizo` sees switch 0x38
/// and starts `gGreatFairyMagicCs`; A through its texts until magic is Link's and he's free;
/// then south out of the fountain by its exit (0x7FFE: spawn 0's return entrance).
fn great_fairy_tasks() -> Vec<Task> {
    use oot_game::audio::ocarina::OCARINA_SONG_LULLABY;
    vec![
        Task::Settle(None),
        Task::Walk(vec![Vec3::new(-24.0, 10.0, -400.0), Vec3::new(-22.0, 10.0, -780.0)]),
        Task::Ocarina(1, OCARINA_SONG_LULLABY, Step::LullabyPlayed),
        Task::Texts(|w| w.save.is_magic_acquired && w.cs_ctx.state == oot_game::cutscene::CS_STATE_IDLE && !w.player_in_cs_mode(), Step::Magic),
        Task::Exit("ENTR_DEATH_MOUNTAIN_TRAIL_4", Some(Step::Trail4)),
    ]
}

/// Dodongo's Cavern's room 2: the Bomb Flower rows' junction (between `En_Bombf` (-2015, 20,
/// -1305) and (-1903, 20, -1305)), on the strip 20 up.
pub const DC_ROOM2_JUNCTION: Vec3 = Vec3::new(-1959.0, 20.0, -1305.0);
/// The last silver rupee (`En_G_Switch` 0x1165 at (-1512, 531, -1083)) and its crate
/// (`Obj_Kibako2` at (-1511, 531, -1084)) on the balcony's east end.
pub const DC_ROOM2_LAST_RUPEE: Vec3 = Vec3::new(-1512.0, 531.0, -1083.0);
pub const DC_ROOM2_RUPEE_CRATE: Vec3 = Vec3::new(-1511.0, 531.0, -1084.0);
/// Room 14's gravestone (`Bg_Haka` 0x0000) and its chest (`En_Box` 0x5541, facing -z).
pub const DC_ROOM14_GRAVESTONE: Vec3 = Vec3::new(1474.0, 76.0, -4558.0);
pub const DC_ROOM14_CHEST: Vec3 = Vec3::new(1473.0, 76.0, -4575.0);
/// Room 15's block nearest its door (`Obj_Timeblock` 0x88FF; the song's range 100: `sRanges[1]`).
pub const DC_ROOM15_BLOCK: Vec3 = Vec3::new(-1958.0, 0.0, -470.0);

/// `Route::FallingStairs`'s tasks: a bomb out, put down by the rows' junction (its blast reaches
/// both junction flowers: `func_80033684`'s 80), back south out of the chain's way, and the stairs
/// down (`Bg_Ddan_Kd`'s flag 0x0B, set as they land; one-point 3050 holds Link meanwhile).
fn falling_stairs_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::TakeBomb(2, Step::BombOut),
        Task::PutDown(vec![Vec3::new(-1959.0, 0.0, -1262.0)], DC_ROOM2_JUNCTION, Step::PutDown),
        Task::Walk(vec![Vec3::new(-1959.0, 0.0, -1100.0)]),
        Task::WaitFor(Cond::Switch(0x0B), Step::StairsFallen),
        Task::WaitCameras,
    ]
}

/// `Route::SilverRupees`'s tasks: a bomb out, put down by the last rupee's crate, back west out of
/// its blast; the crate gone, onto the rupee: the tracker's flag 0x25 and door 17's camera.
fn silver_rupees_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::TakeBomb(2, Step::BombOut),
        Task::PutDown(vec![Vec3::new(-1565.0, 531.0, -1110.0)], DC_ROOM2_RUPEE_CRATE, Step::PutDown),
        Task::Walk(vec![Vec3::new(-1800.0, 531.0, -1125.0)]),
        Task::WaitFor(Cond::CrateGone(DC_ROOM2_RUPEE_CRATE), Step::CrateBroken),
        Task::Walk(vec![Vec3::new(-1600.0, 531.0, -1110.0), DC_ROOM2_LAST_RUPEE]),
        Task::WaitFor(Cond::Switch(0x25), Step::SilverRupees),
        Task::WaitCameras,
    ]
}

/// `Route::ShieldChest`'s tasks: the gravestone (its collision 60 deep, z -4590 to -4530, over
/// the chest and its front) pulled north from its north face; round it to the chest's front
/// (south: `En_Box`'s 0x8000 faces it north) and the chest opened.
fn shield_chest_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::PullStone(DC_ROOM14_GRAVESTONE, Step::StonePulled),
        Task::Walk(vec![Vec3::new(1540.0, 76.0, -4660.0), Vec3::new(1540.0, 76.0, -4515.0), Vec3::new(1473.0, 76.0, -4515.0)]),
        Task::OpenChest(DC_ROOM14_CHEST, Some(Step::ShieldChest)),
        Task::Settle(None),
    ]
}

/// `Route::TimeBlocks`'s tasks: within 100 of the nearest block, the Song of Time (the ocarina on
/// C-Down): the block's free play hears it, and 110 frames later it goes with its camera.
fn time_blocks_tasks() -> Vec<Task> {
    use oot_game::audio::ocarina::OCARINA_SONG_TIME;
    vec![
        Task::Settle(None),
        Task::Walk(vec![Vec3::new(-1958.0, 0.0, -540.0)]),
        Task::Ocarina(1, OCARINA_SONG_TIME, Step::TimeSongPlayed),
        Task::WaitFor(Cond::TimeBlockHidden(DC_ROOM15_BLOCK), Step::TimeBlockGone),
        Task::WaitCameras,
    ]
}

/// `Route::Bottle`'s tasks: the milk (C-Left) drunk twice, the empty bottle out and a fish caught
/// in it, then let out.
fn bottle_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::UseItem(0, Step::MilkDrunk),
        Task::UseItem(0, Step::MilkDrunk),
        Task::CatchInBottle(0, Step::FishCaught),
        Task::UseItem(0, Step::FishLetOut),
        Task::Settle(None),
    ]
}

/// Zora's River's song spot (`En_Okarina_Tag` 0x07F8 at (4096, 921, -1400)), on the platform.
const RIVER_SONG_SPOT: Vec3 = Vec3::new(4110.0, 920.0, -1400.0);

/// `Route::ZorasRiver`'s tasks: on the song spot, Zelda's Lullaby (the ocarina on C-Down); the
/// falls open (switch 0x38, one-point 4100 holding Link); east at a run off the ledge (the run-off
/// jump over the falls' gap) into the tunnel and its exit 3 (`ENTR_ZORAS_DOMAIN_0`), and the
/// Domain's intro.
fn zoras_river_tasks() -> Vec<Task> {
    use oot_game::audio::ocarina::OCARINA_SONG_LULLABY;
    vec![
        Task::Settle(None),
        Task::Ocarina(1, OCARINA_SONG_LULLABY, Step::LullabyPlayed),
        Task::WaitFor(Cond::Switch(0x38), Step::WaterfallOpened),
        // One-point 4100 (it starts a few frames after the flag) over, Link let go.
        Task::WaitCameras,
        Task::Hurry(vec![Vec3::new(4300.0, 920.0, -1400.0)]),
        Task::Exit("ENTR_ZORAS_DOMAIN_0", Some(Step::ZorasDomain)),
        Task::Texts(|w| w.save.get_event_chk_inf(0xA4) && w.cs_ctx.state == oot_game::cutscene::CS_STATE_IDLE && !w.player_in_cs_mode(), Step::DomainIntro),
    ]
}

/// 300 in front of King Zora's throne (`En_Kz` 0x0100 at (628, 996, -1780), facing +z), dropped
/// to the floor below it.
const KING_ZORA_FRONT: Vec3 = Vec3::new(628.0, 996.0, -1480.0);

/// `Route::KingZora`'s tasks: from 300 in front of the throne, at the king until he
/// offers his talk, Ruto's letter shown from C-Left (`EXCH_ITEM_BOTTLE_RUTOS_LETTER`), his texts;
/// then his mweep along path 1 until the flag is set and Link let go.
fn king_zora_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::ShowItem(Who::KingZora, KING_ZORA_FRONT, 0, Step::KingZoraLetter),
        Task::WaitFor(Cond::EventChkInf(crate::en_kz::EVENTCHKINF_GAVE_LETTER_TO_KING_ZORA), Step::KingMoved),
        Task::Settle(None),
    ]
}

/// Jabu-Jabu's room 0: the west cow's target (`En_Wonder_Item` 0x1915 at (-201, -188, -538), on
/// the cow at (-224, -211, -581)), a little up its middle; its flag (door 1's, transition 1).
const JABU_COW_TARGET: Vec3 = Vec3::new(-201.0, -178.0, -538.0);
const JABU_COW_FLAG: i32 = 0x15;
const JABU_DOOR_1: usize = 1;
/// Door 1 (at (0, -240, -1127): Jabu-Jabu's doors stand 80 over their floor), room 0's side.
const JABU_DOOR_1_FRONT: Vec3 = Vec3::new(0.0, -320.0, -1090.0);

/// `Route::JabuCow`'s tasks: the slingshot put on C-Right; from the spawn down into the bowl (its floor -340), the slingshot
/// drawn there and the cow's target shot; the attention camera on door 1 (it unbars), down the
/// corridor (its floor -320) and through.
fn jabu_cow_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        // The story's save has the slingshot on no button (the bottle, the ocarina, the bombs):
        // onto C-Right from the item page, as Route::Pause.
        Task::OpenMenu(Step::MenuOpened),
        Task::MenuCursor(PAUSE_SLOT, Step::CursorOnItem),
        Task::EquipOnC(PAUSE_C_BUTTON, Step::ItemEquipped),
        Task::CloseMenu(Step::MenuClosed),
        Task::Walk(vec![Vec3::new(-18.0, -300.0, 150.0), Vec3::new(-40.0, -340.0, -150.0)]),
        Task::DrawSlingshot(Step::SlingshotDrawn),
        Task::Shoot(JABU_COW_TARGET, JABU_COW_FLAG, Step::CowShot),
        Task::LeaveFirstPerson,
        Task::WaitCameras,
        Task::Walk(vec![Vec3::new(0.0, -340.0, -400.0), Vec3::new(0.0, -320.0, -800.0)]),
        Task::OpenSlidingDoor(JABU_DOOR_1, JABU_DOOR_1_FRONT, Step::DoorOpened),
        Task::WaitDoorShut(JABU_DOOR_1, Step::ThroughDoor),
    ]
}

/// Room 1's elevator's `timer` (512 to 0: its height `home - sin(timer × π/256) × 471.24`, at
/// the top at 384, at the bottom at 128), if it's there.
fn jabu_elevator_timer(w: &PlayState) -> Option<i16> {
    use crate::bg_bdan_objects::{Action, BgBdanObjects};
    w.actors.all().into_iter().find_map(|h| w.actors.downcast::<BgBdanObjects>(h).filter(|o| o.action == Action::ElevatorOscillate).map(|o| o.timer))
}

/// The C button the slingshot is on (C-Right, where the Deku Tree's runs put it, if it's on none).
fn slingshot_button(w: &PlayState) -> u16 {
    use eng_input::pad::{BTN_CDOWN, BTN_CLEFT, BTN_CRIGHT};
    match w.save.equips.button_items[1..4].iter().position(|&i| i == oot_game::item::ITEM_SLINGSHOT) {
        Some(0) => BTN_CLEFT,
        Some(1) => BTN_CDOWN,
        _ => BTN_CRIGHT,
    }
}

/// Link's height.
fn link_y(w: &PlayState) -> f32 {
    w.player().actor.world_pos.y
}

/// `Route::JabuElevator`'s tasks. The corridor from door 1 (floor -320) ends 95 short of the
/// elevator (x -60 to 60, z -1650 to -1760; its top -299 to -1242): Link runs off its end as the
/// elevator comes level with it on the way down, and lands on it. Ridden to the bottom; off it to
/// the west into the pit's water (`waterBoxes[1]`, -1248), across to the west ledge (-1233: 160
/// over the pit's ramp, only joined to it by water), out, and onto the yellow switch 0x19.
fn jabu_elevator_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        // timer 360: the elevator's top at -320 and sinking (Link's run and jump take about 30
        // frames: it's about 60 lower when he lands).
        Task::Texts(|w| jabu_elevator_timer(w) == Some(360), Step::ElevatorLevel),
        Task::Hurry(vec![Vec3::new(0.0, -320.0, -1705.0)]),
        Task::Texts(|w| (link_y(w) - (-771.0 - (jabu_elevator_timer(w).unwrap_or(0) as f32 * (std::f32::consts::PI / 256.0)).sin() * 471.24)).abs() < 2.0, Step::OnElevator),
        Task::Settle(None),
        Task::Texts(|w| jabu_elevator_timer(w).is_some_and(|t| t <= 132) && link_y(w) < -1200.0, Step::ElevatorDown),
        // The shaft is walled round (x -300 here) from 62 under the water's surface up: west to
        // the wall, under it (the dive, 120 without a scale), up in the west pool, out onto the
        // west ledge at its middle, south to the switch.
        Task::Walk(vec![Vec3::new(-180.0, -1248.0, -1700.0)]),
        Task::DiveUnder(Vec3::new(-420.0, -1300.0, -1700.0), -1248.0, Step::Dived),
        Task::Walk(vec![Vec3::new(-620.0, -1233.0, -1700.0), Vec3::new(-612.0, -1233.0, -1836.0)]),
        Task::WaitFor(Cond::Switch(0x19), Step::YellowSwitch),
        Task::WaitCameras,
        Task::Settle(None),
    ]
}

/// Door 14 (transition 14, at (313, -1153, -1703) facing -x), room 1's side.
const JABU_DOOR_14: usize = 14;
const JABU_DOOR_14_FRONT: Vec3 = Vec3::new(270.0, -1233.0, -1703.0);

/// `Route::JabuDoor14`'s tasks: east to door 14 (room 1's spout 0x15C6 at (341, -1233, -1704), 200
/// wide, stands across it: down at -45 since the chest) and through.
fn jabu_door_14_tasks() -> Vec<Task> {
    vec![Task::Settle(None), Task::OpenSlidingDoor(JABU_DOOR_14, JABU_DOOR_14_FRONT, Step::DoorOpened), Task::WaitDoorShut(JABU_DOOR_14, Step::ThroughDoor), Task::Settle(None)]
}

/// The hand test's starts in Jabu-Jabu's Belly (GAME-10 milestones 4 and 5, `game-jabu.bat`):
/// `(name, preset, room, position, yaw, rooms cleared, switch flags set)`. `oot_actors --test
/// jabu_runs` checks each: Link stands, and the room doesn't change.
/// - `cow`: room 0's spawn (the Shaboms, the Octorok; the west cow to shoot);
/// - `elevator`: room 1 at the end of door 1's corridor, facing the elevator;
/// - `flobbery`: room 1's west walkway by its three Flobberies (the Biris float over the shaft);
/// - `pit`: room 1's bottom east of the pit, facing it (the Stinger in the water, the tall switch
///   on the bottom, the wall to dive under to the yellow switch);
/// - `spout`: the same place facing the spout across door 14 (it lifts Link, pushes him back);
/// - `platform`: room 1's walkway (y 80) west of the falling platform, facing it;
/// - `room2`: room 2 inside door 2 (the Biris, the Flobbery, the floor drifting to its holes);
/// - `room14`: room 14's floor south of the Stingers (the Like Like, the Lizalfos, the yellow
///   switch 0x39 raising the water);
/// - `boomerang`: room 14 cleared, in front of the boomerang's chest on its ledge;
/// - `door14`: room 1's bottom after the chest (the spout down), facing door 14.
///
/// Milestone 5's:
/// - `ruto`: room 2 inside door 2, room 2 cleared: walk towards Ruto for her first meeting;
/// - `carry`: room 3 by Ruto after the meeting (`jabu-room-7`): talk, lift her (A), carry her,
///   throw her, drop her in the water; onto the blue switch 0x3C (door 12);
/// - `stick`: room 11 cleared (its chest's Deku Stick, the torch lit by its timer's flag 0x01),
///   inside door 7: the stick to room 7's web;
/// - `barriers`: room 7 by its two gray barriers (`En_Bx` 0x0A82), its crates (`Obj_Kibako`)
///   north of them;
/// - `gray`, `red`, `green`: inside rooms 9, 8 and 10, facing their tentacle (`En_Ba`), the
///   boomerang owned (`jabu-web`: none killed);
/// - `crates`: room 7 by its crates with the blue switch's boulders blown (`jabu-crystal`): a crate
///   on the blue switch 0x0D holds door 6 open;
/// - `crystal`: room 1 in door 9's corridor with the tentacles' flags (`jabu-crystal`);
/// - `door0`: room 5 in front of door 0 after the crystal (`jabu-barinade`): through, room 15's
///   floor is the exit to Barinade.
/// - `octo`: room 3 by Ruto with the green tentacle's flag 0x0C (`jabu-room-7`): carried onto the
///   spout's yellow switch, the hop to the west ledge, door 21 into room 6, thrown onto the
///   platform;
/// - `platform`: room 6 inside door 21 with the Sapphire Ruto's (`jabu-sapphire`): she waits on
///   the platform; near it, it rises (3070), Big Octo comes and takes her, it comes down (3080);
/// - `bigocto`: room 6 where the platform's descent puts Link, facing the platform, Big Octo met
///   (`jabu-big-octo`): the platform spawns him at once, the battle on;
/// - `bari`: room 6's upper walkway (y 80) south of the Bari (`En_Vali` at (-1367, 411, -2490));
/// - `tailpasaran`: room 12 inside door 5 (its three `En_Tp`);
/// - `room4`, `room13`: inside door 8 (room 4: Flobberies, a Lizalfos, crates, hookshot posts) and
///   door 13 (room 13: Stingers, the lens-only Keese, a timer).
pub const JABU_STARTS: [(&str, &str, i8, Vec3, i16, &[i8], &[i32]); 28] = [
    ("cow", "jabu-jabu", 0, Vec3::new(-18.0, -240.0, 302.0), i16::MIN, &[], &[]),
    ("elevator", "jabu-room-1", 1, Vec3::new(0.0, -320.0, -1500.0), i16::MIN, &[], &[]),
    ("flobbery", "jabu-room-1", 1, Vec3::new(-200.0, -325.0, -1600.0), i16::MIN, &[], &[]),
    ("pit", "jabu-room-1", 1, Vec3::new(160.0, -1233.0, -1703.0), -0x4000, &[], &[]),
    ("spout", "jabu-room-1", 1, Vec3::new(160.0, -1233.0, -1703.0), 0x4000, &[], &[]),
    ("platform", "jabu-room-1", 1, Vec3::new(0.0, 80.0, -1700.0), 0x4000, &[], &[]),
    ("room2", "jabu-room-14", 2, Vec3::new(0.0, -340.0, -2560.0), i16::MIN, &[], &[]),
    ("room14", "jabu-room-14", 14, Vec3::new(1000.0, -1293.0, -2450.0), 0, &[], &[]),
    ("boomerang", "jabu-room-14", 14, Vec3::new(780.0, -1073.0, -2376.0), -0x4000, &[14], &[]),
    ("door14", "jabu-boomerang", 1, Vec3::new(160.0, -1233.0, -1703.0), 0x4000, &[], &[]),
    ("ruto", "jabu-boomerang", 2, JABU_ROOM_2_START, i16::MIN, &[2], &[]),
    ("carry", "jabu-room-7", 3, Vec3::new(140.0, -1113.0, -3270.0), 0x4000, &[], &[]),
    ("stick", "jabu-room-7", 11, JABU_ROOM_11_START, i16::MIN, &[11], &[0x01]),
    ("barriers", "jabu-web", 7, Vec3::new(-640.0, -340.0, -4800.0), -0x4000, &[], &[]),
    ("gray", "jabu-web", 9, Vec3::new(1120.0, -340.0, -4683.0), 0x4000, &[], &[]),
    ("red", "jabu-web", 8, Vec3::new(0.0, -340.0, -5370.0), i16::MIN, &[], &[]),
    ("green", "jabu-web", 10, Vec3::new(-1120.0, -340.0, -4683.0), -0x4000, &[], &[]),
    ("crates", "jabu-crystal", 7, Vec3::new(-560.0, -340.0, -4660.0), -0x4000, &[], &[]),
    ("crystal", "jabu-crystal", 1, JABU_DOOR_9_START, 0x4000, &[1], &[]),
    ("door0", "jabu-barinade", 5, Vec3::new(1360.0, -320.0, -2040.0), i16::MIN, &[5], &[]),
    ("room5", "jabu-crystal", 5, Vec3::new(700.0, -320.0, -1703.0), 0x4000, &[], &[]),
    ("octo", "jabu-room-7", 3, JABU_ROOM_3_START, 0x4000, &[], &[0x0C]),
    ("platform", "jabu-sapphire", 6, Vec3::new(-1080.0, -1025.0, -3343.0), -0x4000, &[], &[]),
    ("bigocto", "jabu-big-octo", 6, Vec3::new(-1130.0, -1025.0, -3500.0), -10136, &[], &[]),
    ("bari", "jabu-web", 6, Vec3::new(-1360.0, 80.0, -2700.0), 0, &[], &[]),
    ("tailpasaran", "jabu-web", 12, Vec3::new(-660.0, -340.0, -5300.0), i16::MIN, &[], &[]),
    ("room4", "jabu-web", 4, Vec3::new(-800.0, 80.0, -1703.0), -0x4000, &[], &[]),
    ("room13", "jabu-web", 13, Vec3::new(-680.0, -1113.0, -2640.0), -0x22D8, &[], &[]),
];

/// Room 14's boomerang chest (`En_Box` 0x10C6 at (699, -1073, -2376), facing -x: its front the
/// side away from that, +x).
const JABU_BOOMERANG_CHEST: Vec3 = Vec3::new(699.0, -1073.0, -2376.0);

/// `Route::JabuBoomerang`'s tasks: the chest (there with the room cleared, `ROOM_CLEAR_BIG`)
/// opened; the boomerang's get-item.
fn jabu_boomerang_tasks() -> Vec<Task> {
    vec![Task::Settle(None), Task::OpenChest(JABU_BOOMERANG_CHEST, Some(Step::BoomerangChest)), Task::Settle(None)]
}

/// Room 2 inside door 2 (door 2 at (0, -260, -2443): its floor -340).
const JABU_ROOM_2_START: Vec3 = Vec3::new(0.0, -340.0, -2560.0);
/// Room 2's cow's target (`En_Wonder_Item` 0x1953 at (248, -165, -2771): switch flag 0x13, door 4),
/// in the wall's niche 175 over the floor behind three boulders (`Obj_Bombiwa` 0x0006, 0x0011,
/// 0x001A at y -179 to -184); aimed a little up its middle (at its foot the seed meets the niche's
/// wall).
const JABU_ROOM_2_COW_TARGET: Vec3 = Vec3::new(248.0, -152.0, -2771.0);
const JABU_ROOM_2_COW_FLAG: i32 = 0x13;
/// Door 4 (transition 4, at (0, -260, -3983)), room 2's side.
const JABU_DOOR_4: usize = 4;
const JABU_DOOR_4_FRONT: Vec3 = Vec3::new(0.0, -340.0, -3940.0);
/// `INFTABLE_RUTO_MET_FIRST_TIME` (`save.h`: 0x141).
const INFTABLE_RUTO_MET_FIRST_TIME: u16 = 0x141;

/// Whether Link holds a bomb whose fuse (`En_Bom`'s `timer`) is down to `t` or under.
fn bomb_fuse_under(w: &PlayState, t: i16) -> bool {
    w.player().held_actor.and_then(|h| w.actors.downcast::<crate::en_bom::EnBom>(h)).is_some_and(|b| b.timer <= t)
}

/// `Route::JabuRuto`'s tasks. Ruto (`En_Ru1` 0x0202 at (216, -340, -3072)) waits for Link to come
/// within 265 of her along z (and no lower): her first meeting's cutscene, her texts (0x404B,
/// 0x401E), her walk off and drop through a hole. The bombs onto C-Down from the item page; below
/// the cow's niche a bomb held until its fuse is short and thrown up, to burst by the boulders
/// (`Actor_FindNearbyExplosive`: within 80); the cow's target shot; door 4.
fn jabu_ruto_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::Walk(vec![Vec3::new(120.0, -340.0, -2830.0)]),
        Task::Texts(|w| w.save.get_inf_table(INFTABLE_RUTO_MET_FIRST_TIME) && w.player().state1 & oot_game::actor_ctx::PLAYER_STATE1_29 == 0, Step::RutoMet),
        Task::WaitCameras,
        Task::Settle(None),
        // The bombs (SLOT_BOMB) onto C-Down from the item page.
        Task::OpenMenu(Step::MenuOpened),
        Task::MenuCursor(PAUSE_SLOT_BOMB, Step::CursorOnItem),
        Task::EquipOnC(1, Step::ItemEquipped),
        Task::CloseMenu(Step::MenuClosed),
        Task::Walk(vec![Vec3::new(247.0, -340.0, -3010.0)]),
        Task::TakeBomb(1, Step::BombOut),
        Task::Texts(|w| bomb_fuse_under(w, JABU_BOMB_FUSE), Step::FuseShort),
        Task::ThrowAtRun(vec![Vec3::new(247.0, -340.0, -2960.0), Vec3::new(247.0, -340.0, -2830.0)], JABU_BOMB_RELEASE, Step::BombThrown),
        Task::WaitFor(Cond::Switch(0x06), Step::BouldersBroken),
        Task::Settle(None),
        // Far enough back that the seed's line clears the niche's lip (its floor about -184).
        Task::Walk(vec![Vec3::new(247.0, -340.0, -3080.0)]),
        Task::DrawSlingshot(Step::SlingshotDrawn),
        Task::Shoot(JABU_ROOM_2_COW_TARGET, JABU_ROOM_2_COW_FLAG, Step::CowShot),
        Task::LeaveFirstPerson,
        Task::WaitCameras,
        // Round the holes down to room 3 (x -80 to 0 at z -3040 to -3120, 180 to 220 at -3200 to
        // -3240, 0 to 80 at -3280 to -3360, and two more south-west): down x 300, then west.
        // Then under the arch at the corridor's mouth (its top -280, z -3680 to -3760).
        Task::Walk(vec![Vec3::new(300.0, -340.0, -3100.0), Vec3::new(300.0, -340.0, -3620.0), Vec3::new(0.0, -340.0, -3630.0), Vec3::new(0.0, -340.0, -3880.0)]),
        Task::OpenSlidingDoor(JABU_DOOR_4, JABU_DOOR_4_FRONT, Step::DoorOpened),
        Task::WaitDoorShut(JABU_DOOR_4, Step::ThroughDoor),
        Task::Settle(None),
    ]
}

/// The item page's 6 by 4 grid (`ITEM_GRID_COLS`, `ITEM_GRID_ROWS`).
const ITEM_GRID_COLS: i16 = 6;
const ITEM_GRID_ROWS: i16 = 4;

/// Where one push of the stick takes the item page's cursor from `(x, y)`, by
/// `KaleidoScope_DrawItemSelect`'s rules: left or right it passes empty slots, past the row's end
/// going on from its starting column a row down (`None` back on its own row: the page's arrow);
/// up or down it passes empty slots, and stays (`None`) with none that way. `filled` says which
/// slots (`cursorPoint`) hold an item.
fn item_cursor_move(filled: &impl Fn(i16) -> bool, (x0, y0): (i16, i16), (dx, dy): (i16, i16)) -> Option<(i16, i16)> {
    let (mut x, mut y) = (x0, y0);
    loop {
        if dx != 0 {
            if (dx < 0 && x != 0) || (dx > 0 && x < ITEM_GRID_COLS - 1) {
                x += dx;
            } else {
                x = x0;
                y = (y + 1) % ITEM_GRID_ROWS;
                if y == y0 {
                    return None;
                }
                continue;
            }
        } else if (dy < 0 && y != 0) || (dy > 0 && y < ITEM_GRID_ROWS - 1) {
            y += dy;
        } else {
            return None;
        }
        if filled(x + y * ITEM_GRID_COLS) {
            return Some((x, y));
        }
    }
}

/// The stick's first push (`stick_x`, `stick_y`: up the screen is the stick up) on the shortest
/// way of `item_cursor_move`s from `from` to the slot `to`, if there's one.
fn item_cursor_first_move(filled: impl Fn(i16) -> bool, from: (i16, i16), to: i16) -> Option<(i8, i8)> {
    // Grid moves (x, y) and their sticks: right, left, up the screen (cursorY less one), down.
    const MOVES: [((i16, i16), (i8, i8)); 4] = [((1, 0), (80, 0)), ((-1, 0), (-80, 0)), ((0, -1), (0, 80)), ((0, 1), (0, -80))];
    let target = (to % ITEM_GRID_COLS, to / ITEM_GRID_COLS);
    let mut first: std::collections::HashMap<(i16, i16), Option<(i8, i8)>> = std::collections::HashMap::from([(from, None)]);
    let mut queue = std::collections::VecDeque::from([from]);
    while let Some(at) = queue.pop_front() {
        if at == target {
            return first[&at];
        }
        for (d, stick) in MOVES {
            if let Some(next) = item_cursor_move(&filled, at, d)
                && !first.contains_key(&next)
            {
                first.insert(next, Some(first[&at].unwrap_or(stick)));
                queue.push_back(next);
            }
        }
    }
    None
}

/// The item page's slots (`SLOT_BOMB` 2, `SLOT_DEKU_STICK` 0: `inventory.h`).
const PAUSE_SLOT_BOMB: u16 = 2;
const PAUSE_SLOT_STICK: u16 = 0;

/// The bomb's fuse when Link sets off at a run from 170 south of the niche: the run, the throw
/// and its flight to the niche's wall.
const JABU_BOMB_FUSE: i16 = 30;
/// Where Link throws it, running north (thrown at his speed plus 8, 12 up).
const JABU_BOMB_RELEASE: Vec3 = Vec3::new(247.0, -340.0, -2920.0);

/// Room 11 inside door 7 (door 7 at (660, -260, -5183), facing +z: room 11 behind it).
const JABU_ROOM_11_START: Vec3 = Vec3::new(660.0, -340.0, -5260.0);
/// Room 11's chest (`En_Box` 0x70E9 at (663, -340, -5933): `ROOM_CLEAR_SMALL`, a Deku Stick).
const JABU_STICK_CHEST: Vec3 = Vec3::new(663.0, -340.0, -5933.0);
/// Room 11's golden torch (`Obj_Syokudai` 0x03C1 on its wall: switch flag 0x01, the room's timer's).
const JABU_TORCH: Vec3 = Vec3::new(790.0, -340.0, -5372.0);
/// Door 7, room 11's side.
const JABU_DOOR_7: usize = 7;
const JABU_DOOR_7_FRONT: Vec3 = Vec3::new(660.0, -340.0, -5215.0);
/// Room 7's web in front of door 11 (`Bg_Ydan_Sp` 0x1FD6 at (827, -340, -4687): burnt, switch
/// flag 0x16).
const JABU_WEB: Vec3 = Vec3::new(827.0, -340.0, -4687.0);

/// `Route::JabuWeb`'s tasks: room 11's chest (its Deku Stick), the stick onto C-Left from the item
/// page (the bottle off it), out (C-Left), lit at the torch, through door 7 into room 7 and at the
/// web, burning, until it's gone.
fn jabu_web_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::OpenChest(JABU_STICK_CHEST, Some(Step::StickChest)),
        Task::OpenMenu(Step::MenuOpened),
        Task::MenuCursor(PAUSE_SLOT_STICK, Step::CursorOnItem),
        Task::EquipOnC(0, Step::ItemEquipped),
        Task::CloseMenu(Step::MenuClosed),
        Task::Walk(vec![Vec3::new(700.0, -340.0, -5400.0)]),
        Task::TakeStick(Step::StickOut),
        Task::LightStick(JABU_TORCH, Step::StickLit),
        Task::Hurry(vec![Vec3::new(670.0, -340.0, -5300.0), Vec3::new(660.0, -340.0, -5225.0)]),
        Task::OpenSlidingDoor(JABU_DOOR_7, JABU_DOOR_7_FRONT, Step::DoorOpened),
        Task::WaitDoorShut(JABU_DOOR_7, Step::ThroughDoor),
        // Room 7's east side is walled at x 740 north to z -4780; the web stands in the alcove
        // beyond (z -4740 to -4660), facing west. The floor's conveyors (speed 2) drift Link back
        // to the room's middle faster than a slow walk: at a run up to the web, whose plane the
        // stick's tip crosses (BgYdanSp_WallWebIdle: its z under 1 in the web's frame).
        Task::Hurry(vec![Vec3::new(700.0, -340.0, -4900.0), Vec3::new(700.0, -340.0, -4710.0), Vec3::new(812.0, -340.0, -4693.0)]),
        Task::BurnWeb(JABU_WEB, Step::WebBurnt),
        Task::Settle(None),
    ]
}

/// Room 1 in door 9's corridor (door 9 at (620, -240, -1703), facing -x: room 5 behind it).
const JABU_DOOR_9_START: Vec3 = Vec3::new(420.0, -320.0, -1703.0);
const JABU_DOOR_9: usize = 9;
const JABU_DOOR_9_FRONT: Vec3 = Vec3::new(580.0, -320.0, -1703.0);
/// Room 5's crystal switch (`Obj_Switch` 0x1403 at (1456, -60, -1188): switch flag 0x14, door 0),
/// 380 over the pool's floor (-440, under water to -420). Its sphere (`sCrystalJntSphElementsInit`:
/// 20 across, 300 up at the actor's scale 0.1) is at y -30: aimed 8 over it for the seed's fall
/// (`En_Arrow`'s gravity -0.4) over the 600 or so to it.
const JABU_CRYSTAL: Vec3 = Vec3::new(1456.0, -22.0, -1188.0);
const JABU_CRYSTAL_FLAG: i32 = 0x14;
/// Door 0 (transition 0, at (1360, -240, -2119)), room 5's side.
const JABU_DOOR_0: usize = 0;
const JABU_DOOR_0_FRONT: Vec3 = Vec3::new(1360.0, -320.0, -2080.0);
/// `SCENE_JABU_JABU_BOSS` (`scene_table.h`: 0x13).
const SCENE_JABU_JABU_BOSS: u16 = 0x13;

/// `Route::JabuCrystal`'s tasks: through door 9 (the red barrier gone with its flag); room 5's
/// floor (-320) drops 70 to its pool's slope; from the pool (-440, 20 deep) the crystal shot with
/// the slingshot (C-Right); back up the 62 step to the south floor, door 0 (barred until the
/// crystal's flag) into room 15, whose whole floor (x 1220 to 1500, z -2119 to -2399) is exit 2's:
/// Barinade's room, entered.
fn jabu_crystal_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::OpenSlidingDoor(JABU_DOOR_9, JABU_DOOR_9_FRONT, Step::DoorOpened),
        Task::WaitDoorShut(JABU_DOOR_9, Step::ThroughDoor),
        Task::Walk(vec![Vec3::new(1100.0, -320.0, -1720.0), Vec3::new(1300.0, -400.0, -1700.0), Vec3::new(1456.0, -440.0, -1650.0)]),
        Task::DrawSlingshot(Step::SlingshotDrawn),
        Task::Shoot(JABU_CRYSTAL, JABU_CRYSTAL_FLAG, Step::CrystalShot),
        Task::LeaveFirstPerson,
        Task::WaitCameras,
        // The pool's south rim: a ramp from its east end (x 1700, -391) up to the south floor
        // (x 1460, -320) along z -1900.
        Task::Walk(vec![Vec3::new(1620.0, -440.0, -1800.0), Vec3::new(1700.0, -400.0, -1860.0), Vec3::new(1690.0, -390.0, -1910.0), Vec3::new(1460.0, -320.0, -1920.0), Vec3::new(1360.0, -320.0, -2000.0)]),
        Task::OpenSlidingDoor(JABU_DOOR_0, JABU_DOOR_0_FRONT, Step::DoorOpened),
        // Barinade's intro takes Link at once (his init's INTRO_START): the run ends as the room
        // is entered, before it sets EVENTCHKINF_BEGAN_BARINADE_BATTLE (Route::Barinade's).
        Task::Texts(|w| w.scene_id == SCENE_JABU_JABU_BOSS, Step::BarinadesRoom),
    ]
}

/// `EVENTCHKINF_BEGAN_BARINADE_BATTLE` (`save.h`: 0x76).
const EVENTCHKINF_BEGAN_BARINADE_BATTLE: u16 = 0x76;

/// `Route::Barinade`'s tasks: Link falls into the room and Barinade's intro takes him at once
/// (`INTRO_START`: `PLAYER_CSACTION_8`, turned to face him); the door shut, the Bari called down
/// and attached, the title card; until it hands the camera back and the fight begins.
fn barinade_tasks() -> Vec<Task> {
    vec![Task::Texts(
        |w| w.save.get_event_chk_inf(EVENTCHKINF_BEGAN_BARINADE_BATTLE) && !w.player_in_cs_mode() && w.active_cam_id == oot_game::camera::CAM_ID_MAIN,
        Step::BarinadeBattle,
    )]
}

/// `SCENE_ZORAS_FOUNTAIN` (`scene_table.h`: 0x59).
const SCENE_ZORAS_FOUNTAIN: u16 = 0x59;

/// `Route::Sapphire`'s tasks, from the room's spawn after Barinade's defeat: the heart container
/// (its get-item text; 160 east of where he hung), Ruto's warp once she has risen (into it: she
/// marks Link's spot and turns him to her), A through her words (0x4022) until the warp warps,
/// the warp out to Zora's Fountain (`ENTR_ZORAS_FOUNTAIN_0`, `CS_INDEX_0`: layer 4),
/// `gZorasFountainSapphireCs` with A through its texts (0x4025's choice: the first answer) until
/// its terminator (13) starts the transition, and the fountain (layer 0), Link treading water.
fn sapphire_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::TakeHeart(Step::HeartTaken),
        Task::IntoRutoWarp(Step::RutoWarpEntered),
        Task::Texts(|w| ruto_warp(w).is_some_and(|d| d.ruto_warp_state == crate::door_warp1::WARP_BLUE_RUTO_STATE_WARPING), Step::RutoWarpTalked),
        Task::WarpOut(Step::WarpedOut),
        Task::WarpCutscene(4000, Step::SapphireOver),
        // ENTR_ZORAS_FOUNTAIN_0's spawn (layer 0's spawn 0, -1480, -10, 1755) is in the water by
        // Jabu-Jabu: Link treads water there, so no standing settle.
        Task::Texts(
            |w| w.scene_id == SCENE_ZORAS_FOUNTAIN && w.save.scene_layer == 0 && w.transition.trigger == TRANS_TRIGGER_OFF && w.transition.mode == TRANS_MODE_OFF && !w.player_in_cs_mode(),
            Step::ZorasFountain,
        ),
    ]
}

/// `SCENE_HYRULE_FIELD` (`scene_table.h`: 0x51) and `SCENE_TEMPLE_OF_TIME` (0x43).
const SCENE_FIELD: u16 = 0x51;
const SCENE_TEMPLE_OF_TIME: u16 = 0x43;
/// `ENTR_HYRULE_FIELD_16` (`entrance_table.h`: 0x50F): where `gTempleOfTimeSongOfTimeCs`'s
/// terminator (53) leaves Link, `gHyruleFieldGetOoTCs`'s entrance.
const ENTR_HYRULE_FIELD_16: u16 = 0x50F;

/// Where the `Escape` route starts Link: on the field 450 south of spawn 0 (160, 0, 1415), short of
/// `Bg_Spot00_Hanebasi`'s trigger (x within 450, z 1080 to 1700), facing the drawbridge (north).
pub const ESCAPE_START: (Vec3, i16) = (Vec3::new(160.0, 0.0, 1865.0), -0x8000);
/// Where the `OcarinaOfTime` route starts Link: on the moat's south bank (floor 0 to z 1090; the
/// water's surface -60, its floor -140) south of the ocarina (299, -136, 884), facing it (north).
pub const MOAT_START: (Vec3, i16) = (Vec3::new(299.0, 0.0, 1160.0), -0x8000);
/// `Item_Ocarina` 0x0003's placement (`spot00` layer 1) and the moat's water surface over it.
const MOAT_OCARINA: Vec3 = Vec3::new(299.0, -136.0, 884.0);
const MOAT_SURFACE: f32 = -60.0;

/// `Route::Escape`'s tasks: north onto the drawbridge's trigger (`EVENTCHKINF_ZELDA_FLED_CASTLE`
/// and `_82`, `ENTR_HYRULE_FIELD_0` with 0xFFF1: layer 5), Zelda's escape to its terminator (33,
/// `ENTR_HYRULE_FIELD_0`: layer 1), Link free at spawn 0.
fn escape_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::WalkUntil(vec![Vec3::new(160.0, 0.0, 1500.0)], |w| w.scene_id == SCENE_FIELD && w.save.scene_layer == 5 && w.cs_ctx.state != oot_game::cutscene::CS_STATE_IDLE, Step::EscapeStarted),
        Task::Texts(
            |w| w.scene_id == SCENE_FIELD && w.save.scene_layer == 1 && w.cs_ctx.state == oot_game::cutscene::CS_STATE_IDLE && w.transition.trigger == TRANS_TRIGGER_OFF && w.transition.mode == TRANS_MODE_OFF && !w.player_in_cs_mode(),
            Step::Escaped,
        ),
        Task::Settle(None),
    ]
}

/// `Route::OcarinaOfTime`'s tasks: off the bank into the moat, the dive to the ocarina (the floor
/// at -140 is within any scale's dive; its offer within 30 across and 50 up or down: Player takes it
/// from the bottom and surfaces holding it up), A through its text (0x3A): its close starts
/// `gHyruleFieldZeldaSongOfTimeCs`; its terminator (52) to the Temple of Time (`CS_INDEX_7`: layer
/// 11, `gTempleOfTimeSongOfTimeCs`): Zelda's Song of Time taught and played back; its terminator
/// (53) to `ENTR_HYRULE_FIELD_16`, `gHyruleFieldGetOoTCs`, and Link free in the field.
fn ocarina_of_time_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::Walk(vec![Vec3::new(MOAT_OCARINA.x, 0.0, 1040.0), Vec3::new(MOAT_OCARINA.x, MOAT_SURFACE, 960.0)]),
        Task::DiveFor(MOAT_OCARINA, |w| w.save.get_event_chk_inf(crate::item_ocarina::EVENTCHKINF_43), Step::OcarinaTaken),
        Task::Texts(|w| w.cs_ctx.state != oot_game::cutscene::CS_STATE_IDLE, Step::SongOfTimeCs),
        Task::Texts(|w| w.scene_id == SCENE_TEMPLE_OF_TIME && w.cs_ctx.state != oot_game::cutscene::CS_STATE_IDLE, Step::TempleOfTime),
        Task::Playback(oot_game::audio::ocarina::OCARINA_SONG_TIME, false, Step::SongOfTimePlayed),
        Task::Texts(
            |w| w.scene_id == SCENE_FIELD && w.save.entrance_index == ENTR_HYRULE_FIELD_16 && w.cs_ctx.state == oot_game::cutscene::CS_STATE_IDLE && w.transition.trigger == TRANS_TRIGGER_OFF && w.transition.mode == TRANS_MODE_OFF && !w.player_in_cs_mode(),
            Step::FieldAfterOcarina,
        ),
        Task::Settle(None),
    ]
}

/// `SCENE_CHAMBER_OF_THE_SAGES` (0x44) and `SCENE_CUTSCENE_MAP` (0x47) (`scene_table.h`).
const SCENE_CHAMBER_OF_THE_SAGES: u16 = 0x44;
const SCENE_CUTSCENE_MAP: u16 = 0x47;
/// `EVENTCHKINF_A7` (the Temple's intro seen), `EVENTCHKINF_OPENED_DOOR_OF_TIME` (0x4B),
/// `EVENTCHKINF_REVEALED_MASTER_SWORD` (0x4F), `EVENTCHKINF_C5` (Sheik's awakening),
/// `EVENTCHKINF_C9` (the return's time warp gone) (`save.h`).
const EVENTCHKINF_A7: u16 = 0xA7;
const EVENTCHKINF_OPENED_DOOR_OF_TIME: u16 = 0x4B;
const EVENTCHKINF_REVEALED_MASTER_SWORD: u16 = 0x4F;
const EVENTCHKINF_C5: u16 = 0xC5;
const EVENTCHKINF_C9: u16 = 0xC9;
/// The item page's `SLOT_OCARINA` (`inventory.h`: 7).
const PAUSE_SLOT_OCARINA: u16 = 7;
/// Where `gTempleOfTimeFirstAdultCs` leaves Link (its last player cues, from frame 525): in room
/// 0 between the pedestal and the doorway.
pub const AWAKENING_END: Vec3 = Vec3::new(0.0, 40.0, 214.0);
/// The hall's way from its entrance (spawn 0, (0, -40, 2453)) to the doorway: the altar (17 high
/// at x -50 to 50, z 1300 to 1350; a 41 rise from the hall's floor in front of it) stands in the
/// middle, the floor rising to the door's level (0) on slopes either side of it: round its east
/// side, then down the middle through the doorway (z 1080) and the `En_Holl` (z 920) into room 0.
fn temple_hall_to_room_0() -> Vec<Vec3> {
    vec![Vec3::new(150.0, -40.0, 1450.0), Vec3::new(150.0, -20.0, 1300.0), Vec3::new(100.0, 0.0, 1200.0), Vec3::new(0.0, 0.0, 1100.0), Vec3::new(0.0, 0.0, 950.0)]
}

/// No cutscene running and Link free (not in a cutscene mode).
fn cs_over(w: &PlayState) -> bool {
    w.cs_ctx.state == oot_game::cutscene::CS_STATE_IDLE && !w.player_in_cs_mode()
}

/// `Route::DoorOfTime`'s tasks: the intro (`sEntranceCutsceneTable`'s `ENTR_TEMPLE_OF_TIME_0` row:
/// `EVENTCHKINF_A7`), the Ocarina of Time (`ocarina-of-time` has it on no button) onto C-Down
/// from the item page, to the altar (`En_Okarina_Tag` 0x13FF at (-1, -40, 1384): within 90 across
/// it listens for the Song of Time), the song: `gDoorOfTimeOpeningCs` (the stones, the door
/// sliding; `Demo_Kankyo` sets `EVENTCHKINF_OPENED_DOOR_OF_TIME` as it finishes) until Link is
/// free.
fn door_of_time_tasks() -> Vec<Task> {
    vec![
        // (The intro starts at Play_Init: EVENTCHKINF_A7 is set by then.)
        Task::Texts(|w| w.save.get_event_chk_inf(EVENTCHKINF_A7) && cs_over(w), Step::TempleIntro),
        Task::Settle(None),
        Task::OpenMenu(Step::MenuOpened),
        Task::MenuCursor(PAUSE_SLOT_OCARINA, Step::CursorOnItem),
        Task::EquipOnC(1, Step::ItemEquipped),
        Task::CloseMenu(Step::MenuClosed),
        // In front of the altar, 31 from the song spot: within 50 (its interactRange 0) it marks
        // Link as one to play to it (PLAYER_STATE2_23), and Player's ocarina waits for its check
        // (OCARINA_ACTION_CHECK_TIME) instead of playing freely. Spawn 3's place is 66 away.
        Task::Walk(vec![Vec3::new(0.0, -40.0, 1450.0), Vec3::new(0.0, -40.0, 1415.0)]),
        Task::Settle(None),
        Task::Ocarina(1, oot_game::audio::ocarina::OCARINA_SONG_TIME, Step::TimeSongPlayed),
        Task::Texts(|w| w.save.get_event_chk_inf(EVENTCHKINF_OPENED_DOOR_OF_TIME) && cs_over(w), Step::DoorOfTimeOpen),
        Task::Settle(None),
    ]
}

/// `Route::FirstPull`'s tasks: through the open doorway into room 0, facing the sword within 800
/// (`BgTokiSwd_Idle`: the reveal, `gRevealMasterSwordCs`, Navi's 0x70E6 to 0x70E8); to the
/// pedestal and A (the pull, `gPullMasterSwordCs`; terminator 8's first time sets
/// `EVENTCHKINF_OBTAINED_MASTER_SWORD`: `ENTR_CUTSCENE_MAP_0` with `CS_INDEX_3`), Ganondorf's
/// 0x700A (terminator 25: `linkAgeOnLoad` adult, `ENTR_CHAMBER_OF_THE_SAGES_0` with `CS_INDEX_0`),
/// the Chamber's layer 4: Rauru's texts with the choice (its first answer, 0x7053, goes on), the
/// Light Medallion (0x0040), 0x70E9, until terminator 18's transition starts.
fn first_pull_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::Walk(temple_hall_to_room_0()),
        Task::WalkUntil(vec![Vec3::new(0.0, 0.0, 600.0)], |w| w.save.get_event_chk_inf(EVENTCHKINF_REVEALED_MASTER_SWORD) && w.cs_ctx.state != oot_game::cutscene::CS_STATE_IDLE, Step::SwordReveal),
        Task::Texts(cs_over, Step::SwordRevealed),
        Task::Walk(vec![Vec3::new(0.0, 0.0, 300.0)]),
        Task::Pedestal(Step::SwordPulled),
        Task::Texts(|w| w.scene_id == SCENE_CUTSCENE_MAP && w.cs_ctx.state != oot_game::cutscene::CS_STATE_IDLE, Step::GanondorfFromSword),
        Task::Texts(|w| w.scene_id == SCENE_CHAMBER_OF_THE_SAGES && w.cs_ctx.state != oot_game::cutscene::CS_STATE_IDLE, Step::Chamber),
        Task::Texts(|w| w.save.check_quest_item(oot_game::item::QUEST_MEDALLION_FOREST + 5), Step::LightMedallion),
        Task::Texts(|w| w.scene_id == SCENE_CHAMBER_OF_THE_SAGES && w.transition.trigger != TRANS_TRIGGER_OFF, Step::ChamberLeft),
    ]
}

/// `Route::Awakening`'s tasks: arrived an adult at `ENTR_TEMPLE_OF_TIME_4`, `En_Xc` 9 sets
/// `EVENTCHKINF_C5` and starts `gTempleOfTimeFirstAdultCs`; A through Navi's and Sheik's texts
/// until the script is over (`CS_MISC_STOP_CUTSCENE`, no terminator) with the return's time warp
/// gone (`DemoEffect_UpdateTimeWarpReturnFromChamberOfSages`' 250 frames: `EVENTCHKINF_C9`).
fn awakening_tasks() -> Vec<Task> {
    vec![
        Task::Texts(|w| w.save.get_event_chk_inf(EVENTCHKINF_C5) && w.cs_ctx.state != oot_game::cutscene::CS_STATE_IDLE, Step::SheikAwakening),
        Task::Texts(|w| w.save.get_event_chk_inf(EVENTCHKINF_C9) && cs_over(w), Step::AdultAwake),
        Task::Settle(None),
    ]
}

/// `Route::TempleExit`'s tasks: from where the awakening leaves Link, back through the doorway
/// (the `En_Holl` into room 1), round the altar and down the hall to its entrance's exit (exit 1,
/// `ENTR_TEMPLE_OF_TIME_EXTERIOR_DAY_1`: `shrine_r` for an adult), settled outside.
fn temple_exit_tasks() -> Vec<Task> {
    let mut way = temple_hall_to_room_0();
    way.reverse();
    way.push(Vec3::new(0.0, -40.0, 2300.0));
    vec![Task::Settle(None), Task::Walk(way), Task::Exit("ENTR_TEMPLE_OF_TIME_EXTERIOR_DAY_1", Some(Step::TempleExterior))]
}

/// The ruined market's way from the Temple's road (its north-east, spawn 2 (477, 0, -367)) to
/// the market entrance's (exit 1, the south, between the walls at x -100 and 100): north of the
/// ReDead at (247, 0, -460), round the square's round fountain (its wall 107 from (0, 0)) on its
/// west, between it and the ReDeads at (-283, 0, -154) and (-199, 0, 268), then down the middle:
/// every ReDead stays over 160 away (`EnRd_Idle` hears Link, `PLAYER_STATE2_3`, only within 150).
fn market_ruins_way() -> Vec<Vec3> {
    vec![
        Vec3::new(420.0, 0.0, -240.0),
        Vec3::new(0.0, 0.0, -150.0),
        Vec3::new(-120.0, 0.0, -120.0),
        Vec3::new(-150.0, 0.0, 0.0),
        Vec3::new(-110.0, 0.0, 110.0),
        Vec3::new(-40.0, 0.0, 200.0),
        Vec3::new(0.0, 0.0, 300.0),
        Vec3::new(0.0, 0.0, 600.0),
    ]
}

/// `Route::Ruins`' tasks: from the Temple's steps (`shrine_r`) into the ruined market, round its
/// ReDeads, into the ruined market entrance and out to the field (`ENTR_HYRULE_FIELD_7`).
fn ruins_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::Exit("ENTR_MARKET_DAY_2", Some(Step::MarketRuins)),
        Task::Walk(market_ruins_way()),
        Task::Exit("ENTR_MARKET_ENTRANCE_DAY_0", Some(Step::MarketEntranceRuins)),
        Task::Exit("ENTR_HYRULE_FIELD_7", Some(Step::FieldByTheDrawbridge)),
    ]
}

/// `Route::Prelude`'s tasks: down the hall and through the doorway into room 0, where `En_Xc` 9
/// (with `EVENTCHKINF_48` and no `EVENTCHKINF_55`) sets 0x55, gives the Prelude of Light and
/// starts `gTempleOfTimePreludeCs`; A through Sheik's texts, the song played back after her harp
/// (`MSGMODE_SONG_PLAYBACK`), on until the script is over and Link is free.
fn prelude_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::Walk(temple_hall_to_room_0()),
        Task::WalkUntil(vec![Vec3::new(0.0, 0.0, 600.0)], |w| w.save.get_event_chk_inf(0x55) && w.cs_ctx.state != oot_game::cutscene::CS_STATE_IDLE, Step::PreludeStarted),
        Task::Playback(oot_game::audio::ocarina::OCARINA_SONG_PRELUDE, false, Step::PreludePlayed),
        Task::Texts(cs_over, Step::PreludeLearned),
        Task::Settle(None),
    ]
}

/// `Route::PreludeWarp`'s tasks: the ocarina out (C-Right, the adult's), the Prelude of Light,
/// "warp?" answered yes (the first answer): Player's warp song action, `Demo_Kankyo`'s warp-out
/// and `Environment_WarpSongLeave`; the arrival at `ENTR_TEMPLE_OF_TIME_7` (start mode 5, the
/// warp-in's sparkles and script) until Link is free.
fn prelude_warp_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::OcarinaHeard(2, oot_game::audio::ocarina::OCARINA_SONG_PRELUDE, Step::PreludeWarpPlayed),
        Task::TalkThrough(vec![0], Step::PreludeWarpOut),
        Task::Texts(|w| w.scene_id == SCENE_TEMPLE_OF_TIME && w.save.entrance_index == ENTR_TEMPLE_OF_TIME_7 && cs_over(w) && w.transition.trigger == TRANS_TRIGGER_OFF && w.transition.mode == TRANS_MODE_OFF, Step::PreludeWarped),
        Task::Settle(None),
    ]
}

/// `Route::RideEpona`'s start: Lon Lon Ranch's corral, south-east of Epona's place ((0, 0, -500),
/// `Horse_SetupInGameplay`'s spawn without her had), facing her.
const CORRAL_START: Vec3 = Vec3::new(400.0, 0.0, -800.0);
const CORRAL_START_YAW: i16 = -0x25C8;
/// `Route::RideEpona`'s ride: east of the corral's fence `Bg_Umajump` 0 at (295, -27, -989) (rot y
/// 0x4000: it runs north to south), back to its line, then west over it on a gallop with spurs
/// (fast enough to jump: `EnHorse_UpdateBgCheckInfo` from 8).
const CORRAL_RIDE: [Vec3; 3] = [Vec3::new(750.0, 0.0, -990.0), Vec3::new(800.0, 0.0, -990.0), Vec3::new(-300.0, 0.0, -990.0)];

/// `Route::RideEpona`'s tasks: Epona's Song (C-Right: the adult's ocarina), on her, the ride over
/// the fence, off her.
fn ride_epona_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::Ocarina(2, oot_game::audio::ocarina::OCARINA_SONG_EPONAS, Step::EponaSongPlayed),
        Task::Mount(Step::Mounted),
        Task::Ride(CORRAL_RIDE.to_vec(), true, Step::RodeOver),
        Task::Dismount(Step::Dismounted),
        Task::Settle(None),
    ]
}

/// `Route::FieldRide`'s start: the open field at the foot of the ramp up to Lon Lon Ranch's gate
/// (the gate's spawn 6 is on the ramp, walled from the field below: Epona called there stops at
/// the wall), facing up it.
const RAMP_FOOT: Vec3 = Vec3::new(-1500.0, 61.0, 5600.0);
const RAMP_FOOT_YAW: i16 = -0x1A00;
/// The ramp's turns up to the gate (the floor's heights: about 90 at its foot, 313 at the gate;
/// the exit's polys start partway up).
const GATE_RAMP: [Vec3; 2] = [Vec3::new(-1650.0, 95.0, 5780.0), Vec3::new(-1800.0, 190.0, 5930.0)];

/// `Route::FieldRide`'s tasks: Epona's Song, on her, up the ramp and into the ranch's gate (the
/// field's exit 7, `ENTR_LON_LON_RANCH_0`), in on her, off her.
fn field_ride_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::Ocarina(2, oot_game::audio::ocarina::OCARINA_SONG_EPONAS, Step::EponaSongPlayed),
        Task::Mount(Step::Mounted),
        Task::RideExit(GATE_RAMP.to_vec(), "ENTR_LON_LON_RANCH_0", Step::RodeOut),
        Task::RideIn(Step::RiddenIn),
        Task::Dismount(Step::Dismounted),
        Task::Settle(None),
    ]
}

/// The Ingo runs' debug start: before Ingo (`En_In` params 2 at (-158, 0, -1596), rot y 0x5DDE)
/// by the corral's gate, facing him.
const BY_INGO: Vec3 = Vec3::new(-40.0, 0.0, -1700.0);
const BY_INGO_YAW: i16 = -0x2290;
/// Where Link talks to him from: before him.
const INGO_FRONT: Vec3 = Vec3::new(-60.0, 0.0, -1640.0);
/// Where Link rides to in the corral to talk to the rental's Ingo (params 3 at (-154, 0, -1439),
/// rot y 0x1C72): before him.
const RENTAL_INGO_FRONT: Vec3 = Vec3::new(-60.0, 0.0, -1300.0);

/// The rental's start, as Ingo's offer takes it: 0x2030 (or 0x2031), "Ride" (0); 0x2034, "Don't
/// listen" (1): 10 Rupees (0x205C), its event ends the talk; the corral.
fn ingo_rental_start() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::Converse(Who::Ingo, INGO_FRONT, vec![0, 1], |w| w.transition.trigger != TRANS_TRIGGER_OFF, Step::RentalPaid),
        Task::Texts(|w| w.scene_id == SCENE_LON_LON_RANCH && w.save.entrance_index == crate::playthrough::ENTR_LON_LON_RANCH_6 && w.transition.trigger == TRANS_TRIGGER_OFF && w.transition.mode == TRANS_MODE_OFF && w.player().cs_mode == 0, Step::InCorral),
        Task::Ocarina(2, oot_game::audio::ocarina::OCARINA_SONG_EPONAS, Step::EponaSongPlayed),
        Task::Mount(Step::Mounted),
    ]
}

/// `Route::IngoRental`'s tasks.
fn ingo_rental_tasks() -> Vec<Task> {
    ingo_rental_start()
}

/// `Route::IngoRace`'s tasks: the rental, ridden up to Ingo, 0x2038 "Do it" (0), the race's intro
/// to its start.
fn ingo_race_tasks() -> Vec<Task> {
    let mut t = ingo_rental_start();
    t.extend([
        Task::RideTalk(Who::Ingo, RENTAL_INGO_FRONT, vec![0], |w| w.transition.trigger != TRANS_TRIGGER_OFF, Step::RaceTaken),
        Task::Texts(race_started, Step::RaceStarted),
    ]);
    t
}

/// `Step::RaceStarted`'s condition: Ingo's horse (`En_Horse` type 1, `ENHORSE_ACT_INGO_RACE`)
/// racing.
fn race_started(w: &PlayState) -> bool {
    w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<crate::en_horse::EnHorse>(h)).any(|h| h.ty == crate::en_horse::HORSE_HNI && h.action == crate::en_horse::ENHORSE_ACT_INGO_RACE && h.in_race)
}

/// `Route::EponaWon`'s gallop: north from where `Horse_SetupInGameplay` puts Link on Epona (894,
/// 0, -2084) for a run-up, then south over the shut gates at z -2420 (`Bg_Ingate` params 1 and 3,
/// 80 high: `EnHorse_UpdateBgCheckInfo` jumps that only past 13.8, spurred), past Ingo (at (899, 0,
/// -2536): behind the point 90 before him, `EnIn_GuardingGate`).
const OVER_THE_GATE: [Vec3; 2] = [Vec3::new(900.0, 0.0, -1700.0), Vec3::new(900.0, 0.0, -3000.0)];

/// `Route::EponaWon`'s tasks.
fn epona_won_tasks() -> Vec<Task> {
    vec![
        Task::Texts(|w| w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<crate::en_in::EnIn>(h)).any(|i| i.action == crate::en_in::Action::GuardingGate) && w.player().cs_mode == 0, Step::GateShut),
        Task::RideUntil(OVER_THE_GATE.to_vec(), |w| w.transition.trigger == oot_game::transition::TRANS_TRIGGER_START, Step::JumpedOut),
        Task::Texts(
            |w| w.scene_id == SCENE_FIELD && w.save.get_event_chk_inf(0x18) && cs_over(w) && w.transition.trigger == TRANS_TRIGGER_OFF && w.player().state1 & crate::player::STATE1_23 != 0,
            Step::EponaWon,
        ),
    ]
}

/// `Step::CourseStarted`'s condition: the course's referee (`En_Horse_Game_Check` made type 4 by
/// Epona had) letting Link go.
fn course_started(w: &PlayState) -> bool {
    use crate::en_horse_game_check::{EnHorseGameCheck, HORSEGAME_MALON_RACE, MALONRACE_PLAYER_MOVE};
    w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<EnHorseGameCheck>(h)).any(|g| g.ty == HORSEGAME_MALON_RACE && g.malon.race_flags & MALONRACE_PLAYER_MOVE != 0)
}

/// `Step::EponaCame`'s condition: Epona within 1,200 of Malon (`En_Ma3`) and standing
/// (`ENHORSE_ACT_IDLE`), so that she's in no one's way when Link talks.
fn epona_near_malon(w: &PlayState) -> bool {
    use crate::en_horse::{ENHORSE_ACT_IDLE, EnHorse, HORSE_EPONA};
    let Some(m) = w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<crate::en_ma3::EnMa3>(h)).find(|m| !m.actor.killed).map(|m| m.actor.world_pos) else { return false };
    w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<EnHorse>(h)).any(|h| h.ty == HORSE_EPONA && !h.actor.killed && h.action == ENHORSE_ACT_IDLE && (h.actor.world_pos.x - m.x).hypot(h.actor.world_pos.z - m.z) < 1200.0)
}

/// `Route::MalonCourse`'s tasks: Malon (`En_Ma3`, on Ingo 2's spot: the Ingo runs' start and talk
/// point serve), her thanks; Epona's Song (Epona from the ranch's song point, 1,283 from Malon:
/// too far for her course until she comes); the course taken; its start.
fn malon_course_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::Talk(Who::AdultMalon, INGO_FRONT, Step::MalonThanked),
        Task::Ocarina(2, oot_game::audio::ocarina::OCARINA_SONG_EPONAS, Step::EponaSongPlayed),
        Task::Texts(epona_near_malon, Step::EponaCame),
        Task::Converse(Who::AdultMalon, INGO_FRONT, vec![0], |w| w.transition.trigger != TRANS_TRIGGER_OFF, Step::CourseTaken),
        Task::Texts(course_started, Step::CourseStarted),
    ]
}

/// `Route::MalonSong`'s debug start: 40 before adult Malon by night (`En_Ma2` rot.z 6 at (66, 0,
/// -402)), facing her: within her 48 (`EnMa2_WaitForOcarina`: `PLAYER_STATE2_23`).
const BEFORE_NIGHT_MALON: Vec3 = Vec3::new(66.0, 0.0, -362.0);

/// `Route::MalonSong`'s tasks: the ocarina out to her (`OCARINA_ACTION_CHECK_EPONA`), Epona's
/// Song; 30 frames on she talks (0x2051 into 0x2052).
fn malon_song_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::Ocarina(2, oot_game::audio::ocarina::OCARINA_SONG_EPONAS, Step::EponaSongPlayed),
        Task::Texts(|w| w.save.get_inf_table(crate::en_ma2::INFTABLE_TALKED_TO_ADULT_MALON_AFTER_SONG), Step::MalonSang),
        Task::Settle(None),
    ]
}

/// `Route::BigPoe`'s goal: the Big Poe's place by the ranch's ramp (`En_Po_Field` params 0x17).
const POE_PLACE_17: Vec3 = Vec3::new(-1933.0, 104.0, 5455.0);

/// The live `En_Po_Field` (one serves the ten places) a Big Poe in the action.
fn big_poe_in(w: &PlayState, action: crate::en_po_field::Action) -> bool {
    use crate::en_po_field::{EN_PO_FIELD_BIG, EnPoField};
    w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<EnPoField>(h)).any(|p| !p.actor.killed && p.actor.params == EN_PO_FIELD_BIG && p.action == action)
}

/// `Route::BigPoe`'s tasks: on Epona (`FieldRide`'s start), at the place until the Big Poe is out
/// (`EnPoField_WaitForSpawn`: riding, its switch flag clear: no roll), then until it flees.
fn big_poe_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::Ocarina(2, oot_game::audio::ocarina::OCARINA_SONG_EPONAS, Step::EponaSongPlayed),
        Task::Mount(Step::Mounted),
        Task::RideUntil(vec![POE_PLACE_17], |w| big_poe_in(w, crate::en_po_field::Action::Appear), Step::BigPoeAppeared),
        Task::Texts(|w| big_poe_in(w, crate::en_po_field::Action::Flee), Step::BigPoeFleeing),
    ]
}

/// `ENTR_LON_LON_RANCH_6` (`entrance_table.h`: 0x4CA): the rental's corral.
pub const ENTR_LON_LON_RANCH_6: u16 = 0x4CA;

/// `ENTR_TEMPLE_OF_TIME_7` (`entrance_table.h`: 0x5F4): spawn 7, room 1, on `Bg_Mjin`'s pad (the
/// Prelude's warp).
const ENTR_TEMPLE_OF_TIME_7: u16 = 0x5F4;

/// Room 3's east floor west of Ruto (`En_Ru1` 0x0203 at (222, -1113, -3270)).
const JABU_ROOM_3_START: Vec3 = Vec3::new(140.0, -1113.0, -3270.0);
/// Room 3's blue switch (`Bg_Bdan_Switch` 0x3C00 at (44, -1114, -3596): switch flag 0x3C, door 12).
const JABU_BLUE_SWITCH_3C: Vec3 = Vec3::new(44.0, -1114.0, -3596.0);
const JABU_BLUE_SWITCH_3C_FLAG: i32 = 0x3C;

/// Ruto's two talks in room 3 (0x404C; then 0x404D, she sits and asks to be carried) and the
/// lift.
fn jabu_ruto_lift_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::Talk(Who::Ruto, JABU_ROOM_3_START, Step::RutoExplained),
        Task::Talk(Who::Ruto, JABU_ROOM_3_START, Step::RutoSeated),
        Task::LiftWho(Who::Ruto, Step::RutoLifted),
    ]
}

/// Room 3's yellow switch (`Bg_Bdan_Switch` 0x3A02 at (-15, -1204, -3330), in the pool over the
/// spout `En_Siofuki` 0x0EBF: switch flag 0x3A).
const JABU_YELLOW_SWITCH_3A: Vec3 = Vec3::new(-15.0, -1204.0, -3330.0);

/// Carrying Ruto from room 3's east floor up to its west ledge (-1015, door 21's). The east floor
/// (-1113, Ruto's) is cut off from the rest by the pool (its floor -1209, 3 deep): off its edge
/// into it, onto the yellow switch 0x3A over the spout, which rises 400 for 300 frames and lifts
/// Link to about -1000; from its top at a run west, its push carries him over the 150 or so to the
/// ledge (Wikibooks' Master Quest walkthrough: "step on the yellow switch ... jump over to the
/// highest platform you can reach"). The ledge's sides are slopes Link slides down, not up.
fn jabu_spout_to_ledge_tasks() -> Vec<Task> {
    vec![
        Task::Walk(vec![Vec3::new(100.0, -1206.0, -3280.0), JABU_YELLOW_SWITCH_3A]),
        Task::WaitFor(Cond::Switch(0x3A), Step::YellowSwitch),
        Task::Texts(|w| w.player().actor.world_pos.y > -1010.0, Step::SpoutUp),
        Task::Hurry(vec![Vec3::new(-300.0, -1015.0, -3330.0)]),
    ]
}

/// `Route::JabuSwitch`'s tasks: up to the ledge, down its south slope to the switch's floor (the
/// south floor, -1113, joined to nothing else), Ruto put down on the blue switch.
fn jabu_switch_tasks() -> Vec<Task> {
    let mut t = jabu_ruto_lift_tasks();
    t.extend(jabu_spout_to_ledge_tasks());
    t.extend([
        Task::Walk(vec![Vec3::new(-300.0, -1015.0, -3440.0), Vec3::new(-230.0, -1113.0, -3570.0), Vec3::new(-60.0, -1113.0, -3600.0)]),
        // A carried actor put down lands about 6 in front of Link: he puts her down standing on
        // the switch himself (its camera takes the first A), then steps off.
        Task::PutDown(vec![JABU_BLUE_SWITCH_3C + Vec3::new(12.0, 1.0, 0.0)], Vec3::new(140.0, -1113.0, -3596.0), Step::RutoPutDown),
        Task::Walk(vec![Vec3::new(-60.0, -1113.0, -3590.0)]),
        Task::WaitFor(Cond::Switch(JABU_BLUE_SWITCH_3C_FLAG), Step::RutoOnSwitch),
        Task::Settle(None),
    ]);
    t
}

/// Door 21 (transition 21, at (-1001, -935, -3343)): room 3's west ledge into room 6.
const JABU_DOOR_21: usize = 21;
const JABU_DOOR_21_FRONT: Vec3 = Vec3::new(-960.0, -1015.0, -3343.0);
/// Big Octo's platform (`Bg_Bdan_Objects` 0x1F00 at (-1360, -1025, -3343)).
const JABU_OCTO_PLATFORM: Vec3 = Vec3::new(-1360.0, -1025.0, -3343.0);

/// Whether Big Octo's platform has its battle on.
fn octo_battle(w: &PlayState) -> bool {
    use crate::bg_bdan_objects::{Action, BgBdanObjects};
    w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<BgBdanObjects>(h)).any(|o| o.action == Action::OctoPlatformBattleInProgress)
}

/// En_Fr 0x0000's spot on Zora's River's log (room 0): the ocarina out within 30 of it (above it)
/// sets it as Player's listener.
pub const FROGS_SONG_SPOT: Vec3 = Vec3::new(990.0, 205.0, -1220.0);

/// `Route::Frogs`' tasks: the log's forced text read; each of the six songs on the log (C-Down: `ocarina-c`'s), its reward
/// taken (its flag set, Link free); then the ocarina out once more: 0x40AB and the song game.
fn frogs_tasks() -> Vec<Task> {
    use oot_game::audio::ocarina::{OCARINA_SONG_EPONAS, OCARINA_SONG_LULLABY, OCARINA_SONG_SARIAS, OCARINA_SONG_STORMS, OCARINA_SONG_SUNS, OCARINA_SONG_TIME};
    // The log's forced text (En_Wonder_Talk2 0x4BBB at (1000, 205, -1202): 0x22E, its switch flag
    // 0x3B) opens as Link arrives.
    let mut tasks = vec![Task::TextsFor(Cond::Switch(0x3B), Step::FrogsTip)];
    for (song, played, flag, taken) in [
        (OCARINA_SONG_LULLABY, Step::LullabyPlayed, 0xD1, Step::FrogsZl),
        (OCARINA_SONG_EPONAS, Step::EponasSong, 0xD2, Step::FrogsEpona),
        (OCARINA_SONG_SARIAS, Step::SariasSong, 0xD4, Step::FrogsSaria),
        (OCARINA_SONG_SUNS, Step::SunsSong, 0xD3, Step::FrogsSuns),
        (OCARINA_SONG_TIME, Step::TimeSongPlayed, 0xD5, Step::FrogsTime),
        (OCARINA_SONG_STORMS, Step::SongOfStorms, 0xD6, Step::FrogsStorms),
    ] {
        tasks.push(Task::OcarinaHeard(1, song, played));
        tasks.push(Task::TextsFor(Cond::EventChkInf(flag), taken));
    }
    tasks.push(Task::OcarinaNotes(1, |w| w.msg_ctx.msg_mode == oot_game::message::MSGMODE_FROGS_PLAYING, Tune::Frogs, Step::FrogsGame));
    tasks.push(Task::TextsFor(Cond::EventChkInf(0xD0), Step::FrogsChoir));
    tasks
}

/// 70 in front of Bonooru (`En_Kakasi3` at (-154, -1293, 2888), facing +z).
pub const BONOORU_FRONT: Vec3 = Vec3::new(-154.0, -1293.0, 2958.0);

/// Room 0's spawn 4 (`ENTR_TEMPLE_OF_TIME_4`'s), on the pedestal's step: the pull's debug start.
pub const MASTER_SWORD_STEP: Vec3 = Vec3::new(0.0, 70.0, 46.0);

/// `Route::Bonooru`'s tasks: the ocarina out in front of him, his 0x40A4 read, eight notes (A,
/// C-Down, A, C-Up twice over: none of the songs in them); his 0x40A5 read, the song set; then a
/// talk: 0x40A0 and his playback (one-point 2280) to `OCARINA_MODE_0F`.
fn bonooru_tasks() -> Vec<Task> {
    use oot_game::audio::ocarina::{OCARINA_BTN_A, OCARINA_BTN_C_DOWN, OCARINA_BTN_C_UP};
    let tune = vec![OCARINA_BTN_A, OCARINA_BTN_C_DOWN, OCARINA_BTN_A, OCARINA_BTN_C_UP, OCARINA_BTN_A, OCARINA_BTN_C_DOWN, OCARINA_BTN_A, OCARINA_BTN_C_UP];
    vec![
        Task::Settle(None),
        Task::OcarinaNotes(1, |w| w.msg_ctx.msg_mode == oot_game::message::MSGMODE_SCARECROW_SPAWN_RECORDING_ONGOING, Tune::Notes(tune), Step::BonooruSong),
        Task::Texts(|w| w.save.scarecrow_spawn_song_set != 0 && !w.player_in_cs_mode() && w.player().action != PA::Ocarina, Step::BonooruRemembers),
        Task::Talk(Who::Bonooru, BONOORU_FRONT, Step::BonooruPlayback),
    ]
}

/// `Route::JabuOcto`'s tasks.
fn jabu_octo_tasks() -> Vec<Task> {
    use crate::en_ru1::{INFTABLE_RUTO_BROUGHT_TO_SAPPHIRE_ROOM, INFTABLE_RUTO_HAS_SAPPHIRE};
    let mut t = jabu_ruto_lift_tasks();
    t.extend(jabu_spout_to_ledge_tasks());
    t.extend([
        // West along the ledge past the green barrier's place (En_Bx 0x0C81, gone with 0x0C).
        Task::Walk(vec![Vec3::new(-600.0, -1015.0, -3343.0), Vec3::new(-900.0, -1015.0, -3343.0)]),
        Task::OpenSlidingDoor(JABU_DOOR_21, JABU_DOOR_21_FRONT, Step::DoorOpened),
        Task::WaitDoorShut(JABU_DOOR_21, Step::ThroughDoor),
        Task::Texts(|w| w.save.get_inf_table(INFTABLE_RUTO_BROUGHT_TO_SAPPHIRE_ROOM) && w.cs_ctx.state == oot_game::cutscene::CS_STATE_IDLE && !w.player_in_cs_mode(), Step::RutoFoundSapphire),
        Task::Walk(vec![Vec3::new(-1150.0, -1025.0, -3343.0)]),
        Task::Throw(JABU_OCTO_PLATFORM, Step::RutoThrown),
        Task::Texts(|w| w.save.get_inf_table(INFTABLE_RUTO_HAS_SAPPHIRE), Step::RutoHasSapphire),
        // The platform rises once Link is within 250 of it (BgBdanObjects_OctoPlatform_WaitForRutoToStartCutscene).
        Task::WalkUntil(vec![Vec3::new(-1170.0, -1025.0, -3343.0)], |w| w.player_in_cs_mode(), Step::PlatformRising),
        Task::Texts(|w| w.save.get_inf_table(crate::bg_bdan_objects::INFTABLE_RUTO_ABDUCTED), Step::BigOctoArrived),
        Task::Texts(|w| octo_battle(w) && !w.player_in_cs_mode(), Step::BigOctoBattle),
        Task::WaitCameras,
        Task::Settle(None),
    ]);
    t
}

/// Ruto's blue warp (`Door_Warp1` `WARP_BLUE_RUTO`), if it's there.
fn ruto_warp(w: &PlayState) -> Option<&crate::door_warp1::DoorWarp1> {
    use crate::door_warp1::{DoorWarp1, WARP_BLUE_RUTO};
    w.actors.all().into_iter().find_map(|h| w.actors.downcast::<DoorWarp1>(h).filter(|d| d.actor.params == WARP_BLUE_RUTO && !d.actor.killed))
}

/// `Route::JabuFish`'s tasks: from spawn 2 to 100 west of the feeding spot (`En_Jj`'s
/// (-1589, 53, -43): within 300 across), the fish let out from C-Left (`Player_IsDroppingFish`);
/// 100 frames later `gJabuInhalingCs`, and its end's transition into Jabu-Jabu.
fn jabu_fish_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::Walk(vec![Vec3::new(-1689.0, 20.0, -43.0)]),
        Task::UseItem(0, Step::FishLetOut),
        // SCENE_JABU_JABU (scene_table.h: 0x02).
        Task::Texts(|w| w.scene_id == 0x02, Step::JabuSwallowed),
        Task::Settle(None),
    ]
}

/// `Route::SummitOwl`'s tasks: from spawn 4 to the owl (within 120 he offers his talk: 0x3063 with
/// magic), his texts, and his flight to Kakariko (`gDMTOwlCs`, terminator 66).
fn summit_owl_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::Converse(Who::Owl, Vec3::new(-110.0, 3314.0, -4420.0), vec![], |w| w.scene_id == 0x52, Step::OwlTalk),
        Task::Settle(Some(Step::Kakariko14)),
    ]
}

/// The lobby island's Bomb Flower (`ddan` room 0's `En_Bombf` 0xFFFF at (-2, 0, -1170)).
const DC_ISLAND_FLOWER: Vec3 = Vec3::new(-2.0, 0.0, -1170.0);
/// The pit's boulder over the shortcut pillar's switch (`Obj_Bombiwa` 0x000E; `Obj_Switch` 0x0A00
/// at (429, -160, -1717)).
const DC_PIT_BOULDER: Vec3 = Vec3::new(432.0, -160.0, -1719.0);

/// `Route::Pillar`'s tasks: the island's flower lifted; off the island's south-east corner at a
/// run (Player's run-off jump, carrying: short of the -160 shelf, into the lava, y -180
/// `FLOOR_TYPE_2`), thrown from there at a run towards the boulder (`Task::ThrowAtRun`), Link
/// back out of its blast; the blast (`func_80033684`'s reach) breaks the boulder (flag 0x0E);
/// onto the switch (flag 0x0A); the pillar to the upper level.
fn pillar_tasks() -> Vec<Task> {
    vec![
        Task::Settle(None),
        Task::LiftBombFlower(DC_ISLAND_FLOWER, Step::FlowerLifted),
        Task::ThrowAtRun(vec![Vec3::new(80.0, 0.0, -1300.0), Vec3::new(330.0, -160.0, -1590.0), DC_PIT_BOULDER], Vec3::new(330.0, -160.0, -1590.0), Step::Thrown),
        Task::Walk(vec![Vec3::new(300.0, -160.0, -1600.0), Vec3::new(260.0, -160.0, -1650.0)]),
        Task::WaitFor(Cond::Switch(0x0E), Step::PitBoulderBlown),
        Task::Walk(vec![Vec3::new(429.0, -160.0, -1717.0)]),
        Task::WaitFor(Cond::Switch(0x0A), Step::PillarSwitch),
        // One-point 3060 over, the pillar shuttling (Bg_Ddan_Jd: +140 and +700 at 5, 100 frames
        // idle at each). East along the shelf to the pit's south-east ladder (x 636, z -1662: wall
        // type 2), up it onto the lobby's east floor (y 0).
        Task::Walk(vec![Vec3::new(560.0, -160.0, -1690.0), Vec3::new(610.0, -160.0, -1665.0)]),
        Task::ClimbTo(Vec3::new(670.0, 0.0, -1640.0), Step::LadderUp),
        // North along the floor's edge to the pillar (x 360 to 480, z -1120 to -1240); at its
        // middle (its top level with the floor), a run west off the edge onto it (120 across).
        Task::Walk(vec![Vec3::new(640.0, 0.0, -1400.0), Vec3::new(625.0, 0.0, -1180.0)]),
        Task::WaitFor(Cond::Pillar(DC_SHORTCUT_PILLAR, 140.0), Step::PillarReady),
        Task::RunOntoPillar(DC_SHORTCUT_PILLAR, Step::OnPillar),
        // Ridden to its top (y 520, its top 560), then east at a run off its edge: short of the
        // upper level (170 across, 29 below), Player's mid-air ledge grab and the climb.
        Task::WaitFor(Cond::Pillar(DC_SHORTCUT_PILLAR, 700.0), Step::PillarReady),
        Task::ClimbTo(Vec3::new(720.0, 531.0, -1150.0), Step::UpperLevel),
    ]
}

/// The shortcut pillar's home (`Bg_Ddan_Jd` 0x000A in room 0's pit).
const DC_SHORTCUT_PILLAR: Vec3 = Vec3::new(420.0, -180.0, -1180.0);

/// The middles of the holes in the bridge over the skull (y 591, z -1490 to -1585; the holes x
/// 63 to 137 either side), over its bomb catcher's cylinders (`Bg_Dodoago`: 90 either side, 215
/// in front of it at (0, 196, -1750)).
const DC_HOLE_RIGHT: Vec3 = Vec3::new(100.0, 591.0, -1535.0);
const DC_HOLE_LEFT: Vec3 = Vec3::new(-100.0, 591.0, -1535.0);

/// The skull's eye's glow (`roomCtx.drawParams`, `SDC_DODONGOS_CAVERN`'s env alpha) full, its
/// one-point over and Link free.
fn eye_lit(w: &PlayState, eye: usize) -> bool {
    w.scene.as_ref().is_some_and(|sc| sc.draw.room_draw_params[eye] == 255) && !w.player_in_cs_mode() && w.active_cam_id == oot_game::camera::CAM_ID_MAIN
}

/// `Route::Skull`'s tasks: bombs on C-Right (the pause menu's item page, `SLOT_BOMB`); east to the
/// right hole's west edge, a bomb out and put down at the edge facing the hole: it falls through
/// the catcher (fuse 50) onto the skull's head by its right eye (`BGDODOAGO_EYE_RIGHT`: the
/// blast's yaw from the skull past its facing); west to the left hole for the other eye, the jaw.
fn skull_tasks() -> Vec<Task> {
    use crate::bg_dodoago::{BGDODOAGO_EYE_LEFT, BGDODOAGO_EYE_RIGHT};
    vec![
        Task::Settle(None),
        Task::OpenMenu(Step::MenuOpened),
        Task::MenuCursor(oot_game::item::SLOT_BOMB as u16, Step::CursorOnItem),
        Task::EquipOnC(2, Step::ItemEquipped),
        Task::CloseMenu(Step::MenuClosed),
        Task::Settle(None),
        Task::Walk(vec![Vec3::new(62.0, 591.0, -1535.0)]),
        Task::TakeBomb(2, Step::BombOut),
        Task::PutDown(vec![], DC_HOLE_RIGHT, Step::PutDown),
        Task::Texts(|w| eye_lit(w, BGDODOAGO_EYE_RIGHT), Step::EyeLit),
        Task::Walk(vec![Vec3::new(-62.0, 591.0, -1535.0)]),
        Task::TakeBomb(2, Step::BombOut),
        Task::PutDown(vec![], DC_HOLE_LEFT, Step::PutDown),
        Task::Texts(
            |w| {
                eye_lit(w, BGDODOAGO_EYE_LEFT)
                    && w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<crate::bg_dodoago::BgDodoago>(h)).any(|s| s.action == crate::bg_dodoago::Action::DoNothing)
            },
            Step::JawOpen,
        ),
        Task::Settle(None),
    ]
}

/// `Route::BombBag`'s tasks: to the chest's front and A, its get-item cutscene and text.
fn bomb_bag_tasks() -> Vec<Task> {
    vec![Task::Settle(None), Task::OpenChest(DC_BOMB_BAG_CHEST, Some(Step::BombBag)), Task::Settle(None)]
}

/// The Bomb Flower placed at `home`.
fn flower_near(w: &PlayState, home: Vec3) -> Option<&crate::en_bombf::EnBombf> {
    w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<crate::en_bombf::EnBombf>(h)).find(|f| f.actor.params < 0 && f.actor.home_pos.distance(home) < 1.0)
}

/// The foal (`En_Horse_Link_Child`), if she's there.
fn foal(w: &PlayState) -> Option<&crate::en_horse_link_child::EnHorseLinkChild> {
    w.actors.all().into_iter().find_map(|h| w.actors.downcast::<crate::en_horse_link_child::EnHorseLinkChild>(h))
}

/// The foal in `action`, within 150 of Link (her walk's pace: `func_80A6A7D0`).
fn foal_near(w: &PlayState, action: i32) -> bool {
    let link = w.player().actor.world_pos;
    foal(w).is_some_and(|f| f.action == action && Vec3::new(f.actor.world_pos.x - link.x, 0.0, f.actor.world_pos.z - link.z).length() < 150.0)
}

/// `SCENE_LON_LON_RANCH` (`scene_table.h`: 0x63).
const SCENE_LON_LON_RANCH: u16 = 0x63;

/// Anju's pen (`func_80ABA244`: |x - 330| < 90, |z - 1610| < 190): fenced all round (its north
/// side at z 1400, its east at x 441), so a cucco is thrown over the north fence from just
/// outside it.
const PEN_FRONT: Vec3 = Vec3::new(330.0, 80.0, 1350.0);
const PEN_THROW_AT: Vec3 = Vec3::new(330.0, 80.0, 1600.0);

/// `Route::Graveyard`'s tasks. The graveyard's floors (from the collision): the way in from the
/// west at 0, a ramp up at x -700 to -550 to the first rows of graves at 60 (a wall at x -562
/// south of z 150), another at x -150 to 0 to the second at 120, walled at x -42 from z -180 to
/// 520 (60 high), and a third at x 300 to 400 to the royal tomb's terrace at 180 (to x 850). The
/// rows' gravestones stand 120 apart along z; the way keeps between them.
fn graveyard_tasks() -> Vec<Task> {
    use oot_game::audio::ocarina::OCARINA_SONG_LULLABY;
    vec![
        // The graveyard's intro (gGraveyardIntroCs, EVENTCHKINF_B6).
        Task::Settle(None),
        // Dampé's round (path 0) starts south of Link's way: on it, north of the dig spot at
        // (-473, 60, 202), Link waits for him; he stops about 100 short of Link (EnTk_Rest), on
        // the spot. Talked to: 0x5018, 0x5019's choice yes (10 rupees), and he digs there.
        Task::Walk(vec![Vec3::new(-700.0, 3.0, 119.0), Vec3::new(-520.0, 61.0, 119.0)]),
        // The paid text stays open (MSGMODE_PAUSED) through the dig and closes at its end: a dig
        // on a spot has drawn a prize (EnTk_ChooseReward's counts).
        Task::Converse(Who::Dampe, Vec3::new(-474.0, 60.0, 122.0), vec![0], |w| dampe(w).is_some_and(|d| d.reward_count.iter().any(|&c| c != 0)), Step::Dug),
        Task::PickNearest(Step::DugUp),
        // East between the first row's gravestones at z 59 and 179 (x -348), south round the
        // wall's end, up between the second row's at z 579 and 699 (x 155), up the ramp to the
        // tomb's terrace, to the lullaby's spot (En_Okarina_Tag type 6 at (602, 181, 80)).
        Task::Walk(vec![
            Vec3::new(-348.0, 60.0, 119.0),
            Vec3::new(-200.0, 60.0, 119.0),
            Vec3::new(-200.0, 60.0, 639.0),
            Vec3::new(0.0, 120.0, 639.0),
            Vec3::new(155.0, 120.0, 639.0),
            Vec3::new(300.0, 120.0, 500.0),
            Vec3::new(420.0, 180.0, 200.0),
            Vec3::new(500.0, 181.0, 80.0),
            Vec3::new(580.0, 181.0, 80.0),
        ]),
        // Zelda's Lullaby (C-Down), within 50 of the spot (where it calls Link's ocarina to it,
        // PLAYER_STATE2_23): the tomb's opening (gGraveyardTombOpeningChildCs).
        Task::Ocarina(1, OCARINA_SONG_LULLABY, Step::LullabyPlayed),
        Task::Texts(|w| w.save.get_event_chk_inf(0x1D) && w.cs_ctx.state == oot_game::cutscene::CS_STATE_IDLE, Step::TombOpened),
        Task::Settle(None),
        // Into the hole where the gravestone stood (exit 7: ENTR_ROYAL_FAMILYS_TOMB_0).
        Task::Exit("ENTR_ROYAL_FAMILYS_TOMB_0", Some(Step::TombEntered)),
        // The tomb's room 0: down its slope, and its four perched Keese (En_Firefly 3) each
        // approached until it dives, blocked, and killed: the room cleared, its door to room 1
        // (Door_Shutter SHUTTER_FRONT_CLEAR, Link behind it) unbars.
        Task::BlockKeese(TOMB_KEESE[0]),
        Task::KillKeese(TOMB_KEESE[0]),
        Task::BlockKeese(TOMB_KEESE[1]),
        Task::KillKeese(TOMB_KEESE[1]),
        Task::BlockKeese(TOMB_KEESE[2]),
        Task::KillKeese(TOMB_KEESE[2]),
        Task::BlockKeese(TOMB_KEESE[3]),
        Task::KillKeese(TOMB_KEESE[3]),
        // The door stands on a block 120 high, reached up a notch in its middle: a ledge of 60
        // (climbed), then steps to 90 and 120.
        Task::ClimbTo(Vec3::new(5.0, 60.0, 1222.0), Step::DoorSteps),
        Task::ClimbTo(Vec3::new(5.0, 90.0, 1192.0), Step::DoorSteps),
        Task::ClimbTo(Vec3::new(5.0, 120.0, 1160.0), Step::DoorSteps),
        Task::OpenSlidingDoor(1, Vec3::new(0.0, 120.0, 1150.0), Step::DoorOpened),
        // Down the corridor (x 0, from 120 at z 1100 to 0 at z 750) into room 1, and through it
        // clear of the ReDeads' screams (within 150 in front of one): (130, 177) faces west across
        // the way, (-196, 428) east, (215, 446) south.
        Task::Walk(vec![
            Vec3::new(0.0, 60.0, 900.0),
            Vec3::new(0.0, 5.0, 720.0),
            Vec3::new(40.0, 0.0, 560.0),
            Vec3::new(40.0, 0.0, 400.0),
            Vec3::new(-60.0, 0.0, 250.0),
            Vec3::new(-60.0, 0.0, 60.0),
            Vec3::new(0.0, 0.0, -150.0),
            // The corridor to room 2 (its En_Holl at z -420) and up its step (10 to 40 at z -1025).
            Vec3::new(0.0, 0.0, -500.0),
            Vec3::new(0.0, -10.0, -900.0),
        ]),
        Task::ClimbTo(Vec3::new(0.0, 40.0, -1080.0), Step::DoorSteps),
        // The wall's talk (En_Okarina_Tag type 5, within 120 facing it): at its event the lesson.
        Task::Converse(Who::SongWall, Vec3::new(0.0, 40.0, -1090.0), Vec::new(), |w| w.cs_ctx.state != oot_game::cutscene::CS_STATE_IDLE, Step::SunsSongLesson),
        // The lesson's text and the song shown (TEACH 0x0B), then played back (PLAYBACK 0x18).
        Task::Playback(oot_game::audio::ocarina::OCARINA_SONG_SUNS, false, Step::SunsSongPlayed),
        // Its terminator to the graveyard's layer 5 (gGraveyardSunsSongPart2Cs: CS_DEST 105), and
        // that one's to ENTR_ROYAL_FAMILYS_TOMB_1, where the entrance table plays part 3
        // (gSunSongGraveSunSongTeachPart2Cs, EVENTCHKINF_5A).
        Task::Texts(
            |w| w.save.entrance_index == ENTR_ROYAL_FAMILYS_TOMB_1 && w.save.get_event_chk_inf(0x5A) && w.cs_ctx.state == oot_game::cutscene::CS_STATE_IDLE && w.player().cs_mode == 0 && w.transition.trigger == TRANS_TRIGGER_OFF,
            Step::SunsSongTaught,
        ),
        Task::Settle(None),
        // Back down to room 1, and the Sun's Song there: every ReDead updating is stunned
        // (EnRd_UpdateDamage: no distance check), grey for 600 frames.
        Task::Walk(vec![Vec3::new(0.0, 10.0, -1000.0), Vec3::new(0.0, 0.0, -500.0), Vec3::new(0.0, 0.0, -200.0)]),
        Task::Ocarina(1, oot_game::audio::ocarina::OCARINA_SONG_SUNS, Step::SunsSong),
        Task::Texts(
            |w| {
                let rd: Vec<_> = w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<crate::en_rd::EnRd>(h)).collect();
                rd.len() == 3 && rd.iter().all(|r| r.stunned_by_suns_song)
            },
            Step::ReDeadsStunned,
        ),
    ]
}

/// `ENTR_ROYAL_FAMILYS_TOMB_1` (`entrance_table.h`: 0x574): by the tomb's wall, where the Sun's
/// Song's lesson's part 3 plays.
const ENTR_ROYAL_FAMILYS_TOMB_1: u16 = 0x574;

/// Dampé (`En_Tk`), if he's out.
fn dampe(w: &PlayState) -> Option<&crate::en_tk::EnTk> {
    w.actors.all().into_iter().find_map(|h| w.actors.downcast::<crate::en_tk::EnTk>(h).filter(|d| !d.actor.killed))
}

/// The royal tomb's room 0's Keese (`En_Firefly` 3), by their homes, in the run's order.
const TOMB_KEESE: [Vec3; 4] = [Vec3::new(22.0, 146.0, 1757.0), Vec3::new(281.0, 81.0, 1496.0), Vec3::new(54.0, 73.0, 1253.0), Vec3::new(-63.0, 27.0, 1256.0)];

/// `Route::Kakariko`'s tasks. Kakariko's floors (from the collision): the way in from the west
/// falls from 138 to the village's 0; the pen and the south are at 80; a terrace at 200 north of
/// the middle, reached up a ramp along z -25 from x -300 west to x -700; a landing at 320 north
/// of it (up the ramps at x -150 to -50), and the gate's stairs to 400. The low roof south of the
/// watchtower (x 50 to 300, z -50 to 320, ridge 288) joins the terrace at its north eave (213).
fn kakariko_tasks() -> Vec<Task> {
    vec![
        // Kakariko's intro (gKakarikoVillageIntroCs, EVENTCHKINF_A3): Link walks in.
        Task::Settle(None),
        // Down the way in to its first cucco (-1697, 80, 870: sKakarikoPosList[0]).
        Task::Walk(vec![Vec3::new(-2000.0, 125.0, 1000.0)]),
        Task::Lift(Vec3::new(-1697.0, 80.0, 870.0), Step::CuccoLifted),
        // Carried down into the village, north of the house at x -700 to -400 (z 650 to 950), up
        // the ramp at x 0 (z 1000 to 1100) to the south's 80, and east to the pen's north fence.
        Task::Walk(vec![
            Vec3::new(-1300.0, 30.0, 700.0),
            Vec3::new(-800.0, 0.0, 500.0),
            Vec3::new(0.0, 0.0, 900.0),
            Vec3::new(0.0, 80.0, 1150.0),
            Vec3::new(100.0, 80.0, 1260.0),
            PEN_FRONT,
        ]),
        Task::Throw(PEN_THROW_AT, Step::CuccoPenned),
        // The cucco east of the pen (796, 80, 1639: sKakarikoPosList[2]), back round to the fence.
        Task::Lift(Vec3::new(796.0, 80.0, 1639.0), Step::CuccoLifted),
        Task::Walk(vec![Vec3::new(560.0, 80.0, 1380.0), PEN_FRONT]),
        Task::Throw(PEN_THROW_AT, Step::CuccoPenned),
        // North across the village to the ramp's foot, up it west, onto the terrace, up to the
        // landing below the gate's stairs, to the cucco there (57, 320, -673).
        Task::Walk(vec![
            Vec3::new(100.0, 80.0, 1250.0),
            Vec3::new(-150.0, 0.0, 600.0),
            Vec3::new(-300.0, 0.0, -25.0),
            Vec3::new(-700.0, 200.0, -25.0),
            Vec3::new(-600.0, 200.0, -200.0),
            Vec3::new(-100.0, 200.0, -300.0),
            Vec3::new(-100.0, 300.0, -560.0),
        ]),
        Task::Lift(Vec3::new(57.0, 320.0, -673.0), Step::CuccoLifted),
        // Down onto the terrace, east of the watchtower onto the low roof, over its ridge and off
        // its south edge: the glide down to the village, then south to the pen.
        Task::Walk(vec![Vec3::new(-100.0, 300.0, -520.0), Vec3::new(-50.0, 200.0, -300.0), Vec3::new(170.0, 200.0, -170.0), Vec3::new(170.0, 213.0, -40.0), Vec3::new(170.0, 288.0, 120.0)]),
        Task::Glide(vec![Vec3::new(170.0, 212.0, 300.0), Vec3::new(170.0, 0.0, 460.0)], Step::Glided),
        Task::Walk(vec![Vec3::new(0.0, 0.0, 900.0), Vec3::new(0.0, 80.0, 1150.0), Vec3::new(100.0, 80.0, 1260.0), PEN_FRONT]),
        Task::Throw(PEN_THROW_AT, Step::CuccoPenned),
        // Back north, up the ramp and the terrace, the landing, and east up the gate's stairs (320
        // to 400 between x -200 and 0) to the guard.
        Task::Walk(vec![
            Vec3::new(100.0, 80.0, 1250.0),
            Vec3::new(-150.0, 0.0, 600.0),
            Vec3::new(-300.0, 0.0, -25.0),
            Vec3::new(-700.0, 200.0, -25.0),
            Vec3::new(-600.0, 200.0, -200.0),
            Vec3::new(-100.0, 200.0, -300.0),
            Vec3::new(-100.0, 300.0, -560.0),
            Vec3::new(-200.0, 330.0, -800.0),
            Vec3::new(-180.0, 345.0, -900.0),
            Vec3::new(0.0, 390.0, -960.0),
            Vec3::new(100.0, 400.0, -1050.0),
        ]),
        // The guard (-30, 400, -1298) facing south-east: Zelda's Letter on C-Left, his texts, the
        // gate's sub camera.
        Task::ShowItem(Who::GateGuard, Vec3::new(10.0, 400.0, -1200.0), 0, Step::LetterShown),
        Task::WaitGate(Step::GateOpen),
    ]
}

/// The emerald's cutscene chain after part 1 (`Cutscene_Command_Destination`'s terminators): each
/// part's scene (`scene_table.h`), its cutscene layer, its step.
pub const CHAIN: [(u16, usize, Step); 8] = [
    (0x47, 5, Step::Ganondorf),
    (0x47, 4, Step::Goddesses),
    (0x5A, 5, Step::Din),
    (0x5A, 4, Step::Valley),
    (0x60, 4, Step::Nayru),
    (0x55, 4, Step::Farore),
    (0x47, 6, Step::Triforce),
    (0x55, 6, Step::EmeraldPart9),
];

/// The `BossRoom` route's door: room 9's to room 11 (transition 4), and in front of it on room
/// 9's side (it faces room 9, rot y 0x105B).
pub const ROOM9_DOOR: usize = 4;
pub const ROOM9_DOOR_FRONT: Vec3 = Vec3::new(-882.0, -1880.0, -932.0);

/// Where the `BlueWarp` route starts Link: 150 in front of the warp her init leaves at (0, -640,
/// 0), facing it.
pub const BLUE_WARP_START: (Vec3, i16) = (Vec3::new(0.0, -640.0, 150.0), -0x8000);

/// Where `BossGoma_Encounter` waits for Link: the boss room's entrance (150, 350), within 60.
pub const GOHMA_ENTRANCE: Vec3 = Vec3::new(150.0, -640.0, 350.0);
/// Where the `Gohma` route looks up at her from in her intro: halfway from the entrance to the
/// ceiling's centre.
pub const GOHMA_LOOK_FROM: Vec3 = Vec3::new(0.0, -640.0, 0.0);
/// Where the `Gohma` route shoots up at her on the ceiling from: 310 from the ceiling's centre
/// (-150, -350, where she settles), towards the entrance.
pub const GOHMA_SHOOT_UP_FROM: Vec3 = Vec3::new(-150.0, -640.0, -40.0);

/// The `Save` route's button (C-Left), and the file the debug start saves to and the reset
/// loads (file 2: `fileNum` 1).
pub const SAVE_C_BUTTON: usize = 0;
pub const SAVE_FILE: usize = 2;

/// The `Pause` route's item: the Fairy Slingshot's slot (`SLOT_SLINGSHOT`, the grid's second row,
/// first column), and its button (C-Right).
pub const PAUSE_SLOT: u16 = 6;
pub const PAUSE_C_BUTTON: usize = 2;

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

/// Room 1's eye switch (`Obj_Switch` 0x0C02, facing 0x6000, over the door to room 2: its flag
/// 0x0C opens it), and room 3's (0x1502, facing -z, over the door to room 4: flag 0x15).
pub const ROOM1_EYE_HOME: Vec3 = Vec3::new(-920.0, 542.0, 918.0);
pub const ROOM1_EYE_FLAG: i32 = 0x0C;
pub const ROOM3_EYE_HOME: Vec3 = Vec3::new(-76.0, -727.0, 551.0);
pub const ROOM3_EYE_FLAG: i32 = 0x15;
/// Room 2's ladder (`Bg_Ydan_Maruta` 0x0121, facing -0x2000, 280 above the floor until a seed hits
/// it: flag 0x21).
pub const ROOM2_LADDER_HOME: Vec3 = Vec3::new(-1066.0, 560.0, 1066.0);
pub const ROOM2_LADDER_FLAG: i32 = 0x21;
/// Room 10's chest with the Fairy Slingshot (`En_Box` 0x10A6: `GI_SLINGSHOT`, flag 6, appearing
/// when the room is cleared).
pub const ROOM10_SLINGSHOT_CHEST_HOME: Vec3 = Vec3::new(-1082.0, 820.0, 387.0);
/// The eye's front: its triangles are 8.5 in front of its home (`sEyeTrisElementsInit`), along
/// its facing (0x6000).
pub const ROOM1_EYE_FRONT: Vec3 = Vec3::new(6.01, 0.0, -6.01);
/// Room 1's door to room 2 (transition 5, at (-936, 400, 936) facing 0x6000), and where Link lines
/// up in front of it on room 1's side.
pub const ROOM1_DOOR: usize = 5;
pub const ROOM1_DOOR_FRONT: Vec3 = Vec3::new(-915.0, 400.0, 915.0);
/// Where `Route::Slingshot` starts Link: room 1, 250 in front of the eye, facing it.
pub const SLINGSHOT_START: (Vec3, i16) = (Vec3::new(-743.0, 400.0, 741.0), -0x2000);
/// The debug starts with the slingshot (GAME-05 milestone 5a, `game-slingshot.bat`, the
/// `deku-tree-slingshot` preset): `(room, position, yaw, name)`. `oot_actors --test
/// debug_starts` checks each: Link stands, and the room doesn't change.
/// - `room1`: room 1, 250 in front of its eye switch, facing it: `Route::Slingshot`'s start;
/// - `room3`: room 3's floor under its eye switch, facing it (+z);
/// - `room2`: room 2's floor south-west of the ladder, facing it, where the lift (`Obj_Lift`, its
///   top 408) doesn't block the shot (the travel test's spot);
/// - `room10`: in front of room 10's slingshot chest (its front is the side away from its facing,
///   +z: Link stands south of it facing +z); the chest appears once the room's enemies are gone.
pub const SLINGSHOT_STARTS: [(i8, Vec3, i16, &str); 4] = [
    (1, SLINGSHOT_START.0, SLINGSHOT_START.1, "room1"),
    (3, Vec3::new(-76.0, -880.0, 351.0), 0, "room3"),
    (2, Vec3::new(-1130.0, 280.0, 1360.0), 30533, "room2"),
    (10, Vec3::new(-1082.0, 820.0, 300.0), 0, "room10"),
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

/// The `Field` route's Peahat: the grounded one (params -1) west of the castle.
pub const FIELD_PEAHAT_HOME: Vec3 = Vec3::new(-4858.0, -300.0, -216.0);
/// Where the `Field` route starts Link: 600 east of it, facing it (yaw -0x4000: -x).
pub const FIELD_START: (Vec3, i16) = (Vec3::new(-4258.0, -300.0, -216.0), -0x4000);
/// Where the `FieldNight` route starts Link: on the grass north of Lon Lon Ranch, away from the
/// river's water box (`EnEncount1_SpawnStalchildOrWolfos` spawns nothing where the floor is under
/// a water box's surface), facing east.
pub const FIELD_NIGHT_START: (Vec3, i16) = (Vec3::new(1500.0, 0.0, 2000.0), 0x4000);

/// The Song of Storms' fairy spot in Hyrule Field: `Shot_Sun` 0xFF41's home (`ootx scene-info
/// --scene spot00`), and where the `Ocarina` route starts Link: 312 north of it (outside its 150),
/// facing it (+z); and where he plays the Song of Storms, 80 from it.
pub const SHOT_SUN_HOME: Vec3 = Vec3::new(5143.0, -55.0, 5012.0);
pub const OCARINA_START: (Vec3, i16) = (Vec3::new(5143.0, -30.0, 4700.0), 0);
pub const OCARINA_STORMS_AT: Vec3 = Vec3::new(5143.0, -60.0, 4932.0);

/// The `Zelda` and `ZeldaTalk` routes from Zelda's courtyard's entrance: up the courtyard to
/// Zelda at the window; her talk (yes to each question), the legend on the cutscene map, back, the
/// window, Ganondorf, and her letter.
fn zelda_tail() -> Vec<Task> {
    vec![
        Task::Converse(Who::Zelda, Vec3::new(-425.0, 84.0, 0.0), Vec::new(), |w| w.save.get_event_chk_inf(crate::en_zl4::EVENTCHKINF_OBTAINED_ZELDAS_LETTER), Step::ZeldasLetter),
        // The letter's get-item text read and Link free.
        Task::Texts(|w| w.player().cs_mode == 0 && !matches!(w.player().action, PA::Talk | PA::GetItem), Step::LetterRead),
        // East back down the courtyard to Impa by its entrance: within 130 west of her
        // (`Demo_Im` type 5 at x 336), her lullaby's script.
        Task::Walk(vec![Vec3::new(250.0, 44.0, 0.0)]),
        Task::Texts(|w| w.cs_ctx.state != oot_game::cutscene::CS_STATE_IDLE, Step::LullabyCs),
        // Impa whistles it; Link plays it back: one wrong note, then right.
        Task::Playback(oot_game::audio::ocarina::OCARINA_SONG_LULLABY, true, Step::LullabyPlayed),
        // The script's terminator to Hyrule Field's layer 12, Impa's goodbye there, and its terminator
        // to ENTR_HYRULE_FIELD_17.
        Task::Texts(|w| w.save.entrance_index == ENTR_HYRULE_FIELD_17 && w.cs_ctx.state == oot_game::cutscene::CS_STATE_IDLE && w.player().cs_mode == 0 && w.transition.trigger == TRANS_TRIGGER_OFF, Step::ImpaGone),
    ]
}

/// `ENTR_HYRULE_FIELD_17` (`entrance_table.h`): where Impa's goodbye leaves Link.
const ENTR_HYRULE_FIELD_17: u16 = 0x594;

/// The `Zelda` route's conditions on the guards' courtyard's patrols (`Task::WaitGuards`), by the
/// screen Link waits to cross: each guard by his path, where he's headed (`waypoint`: the corners
/// 0 to 3 and, for types 2 and 3, the middles 4 to 7), where he is, whether he waits.
mod courtyard {
    use oot_game::play::PlayState;

    use crate::en_heishi1::{Action, EnHeishi1};

    fn guard(w: &PlayState, path: i16) -> Option<&EnHeishi1> {
        w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<EnHeishi1>(h)).find(|g| !g.actor.killed && g.path == path)
    }

    fn at(w: &PlayState, path: i16, test: impl Fn(&EnHeishi1) -> bool) -> bool {
        guard(w, path).is_some_and(test)
    }

    /// Path 0 headed up his west side (3), round to his north-east corner and his wait there.
    pub fn screen7(w: &PlayState) -> bool {
        at(w, 0, |g| g.waypoint == 3)
    }

    /// Path 1 headed up his west side; path 2 well up his east side (1) or along his north side (2).
    pub fn screen2(w: &PlayState) -> bool {
        at(w, 1, |g| g.waypoint == 3) && at(w, 2, |g| matches!(g.waypoint, 1 | 2) && g.actor.world_pos.z < 60.0)
    }

    /// Path 3 in the north half of his round (from the west middle to the east middle).
    pub fn screen3(w: &PlayState) -> bool {
        at(w, 3, |g| matches!(g.waypoint, 5 | 2 | 6 | 3) && g.actor.world_pos.z < 60.0)
    }

    /// Path 4 waiting at his south-west corner (2), looking west; path 6 just come to his north-east
    /// corner (2), its wait (`waitTimer`) not half over.
    pub fn screen4(w: &PlayState) -> bool {
        at(w, 4, |g| g.waypoint == 2 && g.action == Action::Wait) && at(w, 6, |g| g.waypoint == 2 && g.action == Action::Wait && g.timers[1] >= 40)
    }

    /// Path 4 up his west side (6, 3), north of the box's middle.
    pub fn screen4_west(w: &PlayState) -> bool {
        at(w, 4, |g| matches!(g.waypoint, 6 | 3) && g.actor.world_pos.z < 60.0)
    }

    /// The plaza's two (paths 5 and 7, round the planter the same way: south down the east side,
    /// west along the south): both in its north-east quarter, past the north side's middle and
    /// north of the east side's.
    pub fn plaza(w: &PlayState) -> bool {
        [5, 7].into_iter().all(|path| at(w, path, |g| g.actor.world_pos.x > -1260.0 && g.actor.world_pos.z < -400.0))
    }
}

/// The Deku Baba the `DekuBaba` route fights: room 0's `En_Dekubaba` (params 0) on the top floor.
pub const DEKU_BABA_HOME: Vec3 = Vec3::new(-195.0, 800.0, -195.0);
/// Where the `DekuBaba` route starts Link: 85 from the Deku Baba, on the floor outwards of it,
/// facing it (yaw 0x2000).
pub const DEKU_BABA_START: (Vec3, i16) = (Vec3::new(-255.104, 800.0, -255.104), 0x2000);

impl Route {
    /// The entrance a route starts at.
    pub fn entrance(self) -> &'static str {
        match self {
            Route::DekuBaba | Route::Combat | Route::Scrub | Route::Shutter | Route::Stick | Route::Push | Route::Slingshot | Route::Pause | Route::DungeonMap | Route::GameOver | Route::Save => "ENTR_DEKU_TREE_0",
            Route::Gohma | Route::BlueWarp => "ENTR_DEKU_TREE_BOSS_0",
            Route::BossRoom => "ENTR_DEKU_TREE_0",
            Route::Emerald => "ENTR_KOKIRI_FOREST_0",
            Route::Creation => "ENTR_CUTSCENE_MAP_0",
            Route::Farewell => "ENTR_KOKIRI_FOREST_11",
            Route::Dusk | Route::Field | Route::FieldNight => "ENTR_HYRULE_FIELD_0",
            Route::Market => "ENTR_MARKET_ENTRANCE_DAY_0",
            Route::Castle | Route::CastleCaught => "ENTR_HYRULE_CASTLE_0",
            Route::Ocarina => "ENTR_HYRULE_FIELD_0",
            Route::Zelda => "ENTR_CASTLE_COURTYARD_GUARDS_DAY_0",
            Route::ZeldaTalk => "ENTR_CASTLE_COURTYARD_ZELDA_0",
            Route::Kakariko => "ENTR_KAKARIKO_VILLAGE_0",
            Route::Graveyard => "ENTR_GRAVEYARD_0",
            Route::Meadow | Route::LostWoods => "ENTR_LOST_WOODS_0",
            Route::Ranch => "ENTR_LON_LON_RANCH_0",
            Route::Bombs => "ENTR_DODONGOS_CAVERN_0",
            // Goron City's spawn 2, the bottom floor (sEntranceCutsceneTable's intro is for _0).
            Route::Darunia => "ENTR_GORON_CITY_2",
            // The trail's spawn 1, by Goron City's door (the intro is for _0).
            Route::Boulder => "ENTR_DEATH_MOUNTAIN_TRAIL_1",
            Route::BombBag => "ENTR_DODONGOS_CAVERN_0",
            // Spawn 1: room 7, by door 20 (the return from King Dodongo's room).
            Route::Gravestone => "ENTR_DODONGOS_CAVERN_1",
            Route::Pillar => "ENTR_DODONGOS_CAVERN_0",
            Route::Skull => "ENTR_DODONGOS_CAVERN_0",
            Route::KingDodongo | Route::Ruby => "ENTR_DODONGOS_CAVERN_BOSS_0",
            // The summit's spawn 4 (the fountain's return entrance, at the wall).
            Route::FairyWall | Route::SummitOwl => "ENTR_DEATH_MOUNTAIN_TRAIL_4",
            Route::GreatFairy => "ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_0",
            // The lobby, then the run's room (the debug start's --room).
            Route::FallingStairs | Route::SilverRupees | Route::ShieldChest | Route::TimeBlocks => "ENTR_DODONGOS_CAVERN_0",
            // Spawn 0, room 1 (the intro is EVENTCHKINF_A4's: the preset has it seen).
            Route::Bottle => "ENTR_ZORAS_DOMAIN_0",
            Route::ZorasRiver => "ENTR_ZORAS_RIVER_2",
            Route::KingZora => "ENTR_ZORAS_DOMAIN_1",
            Route::JabuFish => "ENTR_ZORAS_FOUNTAIN_2",
            // Spawn 0, room 0; the debug starts change room.
            Route::JabuCow | Route::JabuElevator | Route::JabuBoomerang | Route::JabuDoor14 | Route::JabuRuto | Route::JabuWeb | Route::JabuCrystal | Route::JabuSwitch | Route::JabuOcto => "ENTR_JABU_JABU_0",
            Route::Frogs => "ENTR_ZORAS_RIVER_0",
            Route::Bonooru => "ENTR_LAKE_HYLIA_0",
            // The hall's entrance (room 1; the intro seen), then room 0 (the debug start). Not
            // ENTR_TEMPLE_OF_TIME_4, the adult's from the Chamber: with it the pull's time warp
            // (Demo_Effect 0xFF0F) is drawn before the pull (DemoEffect_DrawTimeWarp), and a child
            // never enters by it.
            Route::MasterSwordPull => "ENTR_TEMPLE_OF_TIME_0",
            // Spawn 2, room 0: start mode 1 (Player_StartMode_TimeTravel), on the pedestal.
            Route::MasterSwordBack => "ENTR_TEMPLE_OF_TIME_2",
            // Spawn 0, the hall's entrance (room 1). The exit's debug start then moves to room 0,
            // as the pull's does (not ENTR_TEMPLE_OF_TIME_4: see MasterSwordPull).
            Route::DoorOfTime | Route::FirstPull | Route::TempleExit => "ENTR_TEMPLE_OF_TIME_0",
            // Spawn 4, room 0: the return from the Chamber (terminator 18's).
            Route::Awakening => "ENTR_TEMPLE_OF_TIME_4",
            // The Temple's steps (shrine_r's spawn 1 for an adult): TempleExit's end.
            Route::Ruins => "ENTR_TEMPLE_OF_TIME_EXTERIOR_DAY_1",
            // The hall's entrance; SwordBack's debug start then moves to room 0.
            Route::Prelude | Route::SwordBack => "ENTR_TEMPLE_OF_TIME_0",
            // Spawn 2, room 0: the time travel's arrival (start mode 1).
            Route::SwordAgain => "ENTR_TEMPLE_OF_TIME_2",
            // The field's spawn 7, by the broken drawbridge (the Ruins run's end).
            Route::PreludeWarp => "ENTR_HYRULE_FIELD_7",
            // The ranch's gateway from the field (spawn 0); the debug start moves Link into the
            // corral.
            Route::RideEpona => "ENTR_LON_LON_RANCH_0",
            // The field outside the ranch's gate (spawn 6, coming out of the ranch).
            Route::FieldRide => "ENTR_HYRULE_FIELD_6",
            // The ranch's gateway (spawn 0); the debug start moves Link to Ingo by the corral's gate.
            Route::IngoRental | Route::IngoRace => "ENTR_LON_LON_RANCH_0",
            Route::IngoLost => "ENTR_LON_LON_RANCH_8",
            Route::IngoWon | Route::EponaWon => "ENTR_LON_LON_RANCH_7",
            // The ranch's gateway (spawn 0: Ingo's init clears the race word); the debug starts
            // move Link to Malon.
            Route::MalonCourse | Route::MalonSong => "ENTR_LON_LON_RANCH_0",
            // Where the course's referee sends Link back (Malon beside spawn 7).
            Route::MalonRecord => "ENTR_LON_LON_RANCH_7",
            // The field outside the ranch's gate (spawn 6), as FieldRide.
            Route::BigPoe => "ENTR_HYRULE_FIELD_6",
            // Spawn 0, room 1: Link falls in from room 15.
            Route::Barinade | Route::Sapphire => "ENTR_JABU_JABU_BOSS_0",
            // Spawn 0, by the drawbridge (the market's way out); the debug starts move Link.
            Route::Escape | Route::OcarinaOfTime => "ENTR_HYRULE_FIELD_0",
            _ => "ENTR_LINKS_HOUSE_0",
        }
    }

    /// The save preset it needs, if any.
    pub fn preset(self) -> Option<&'static str> {
        match self {
            Route::DekuTree => Some("deku-tree-open"),
            Route::DekuBaba | Route::Combat | Route::Scrub | Route::Shutter | Route::Push => Some("deku-tree-inside"),
            Route::Stick => Some("deku-tree-sticks"),
            Route::Slingshot => Some("deku-tree-slingshot"),
            Route::Pause => Some("deku-tree-slingshot-owned"),
            Route::DungeonMap => Some("deku-tree-compass"),
            Route::GameOver => Some("deku-tree-quarter-heart"),
            Route::Save => Some("deku-tree-save"),
            Route::Gohma => Some("deku-tree-gohma"),
            Route::BlueWarp => Some("deku-tree-gohma-cleared"),
            Route::BossRoom => Some("deku-tree-slingshot"),
            Route::Emerald | Route::Creation | Route::Farewell | Route::Dusk | Route::Field | Route::FieldNight | Route::Market => Some("deku-tree-dead"),
            Route::Castle => Some("castle-cucco"),
            Route::CastleCaught => Some("castle-egg"),
            Route::Ocarina => Some("ocarina-songs"),
            Route::Zelda | Route::ZeldaTalk => Some("castle-courtyard"),
            Route::Kakariko => Some("kakariko-letter"),
            Route::Graveyard => Some("graveyard-lullaby"),
            Route::Meadow => Some("graveyard-sun"),
            Route::LostWoods => Some("meadow-saria"),
            Route::Ranch => Some("lost-woods-games"),
            Route::Bombs => Some("cavern-bombs"),
            Route::Darunia => Some("ranch-epona"),
            Route::Boulder => Some("goron-bracelet"),
            Route::BombBag => Some("cavern-loop"),
            Route::Gravestone => Some("cavern-jaw"),
            Route::Pillar => Some("cavern-lobby"),
            Route::Skull => Some("cavern-bomb-bag"),
            Route::KingDodongo => Some("cavern-boss"),
            Route::Ruby => Some("cavern-boss-beaten"),
            Route::FairyWall => Some("goron-ruby"),
            Route::GreatFairy => Some("fairy-fountain"),
            Route::SummitOwl => Some("fairy-magic"),
            Route::FallingStairs | Route::SilverRupees => Some("fairy-magic"),
            Route::TimeBlocks => Some("cavern-song-of-time"),
            Route::Bottle => Some("zora-bottle"),
            Route::ZorasRiver => Some("fairy-magic"),
            Route::KingZora => Some("zora-letter"),
            Route::JabuFish => Some("jabu-fish"),
            Route::JabuCow => Some("jabu-jabu"),
            Route::JabuElevator => Some("jabu-room-1"),
            Route::JabuBoomerang => Some("jabu-room-14"),
            Route::JabuDoor14 => Some("jabu-boomerang"),
            Route::JabuRuto => Some("jabu-boomerang"),
            Route::JabuWeb => Some("jabu-room-7"),
            Route::JabuCrystal => Some("jabu-crystal"),
            Route::JabuSwitch | Route::JabuOcto => Some("jabu-room-7"),
            Route::Frogs | Route::Bonooru => Some("ocarina-c"),
            Route::MasterSwordPull => Some("master-sword"),
            Route::MasterSwordBack => Some("master-sword-adult"),
            Route::DoorOfTime => Some("ocarina-of-time"),
            Route::FirstPull => Some("door-of-time"),
            Route::Awakening => Some("light-medallion"),
            Route::TempleExit | Route::Ruins => Some("adult-awake"),
            Route::Prelude => Some("prelude"),
            Route::SwordBack | Route::PreludeWarp => Some("prelude-learned"),
            Route::RideEpona => Some("adult-awake"),
            Route::FieldRide => Some("epona"),
            Route::IngoRental => Some("adult-awake"),
            Route::IngoRace => Some("ingo-rental"),
            Route::IngoLost => Some("ingo-race-lost"),
            Route::IngoWon => Some("ingo-first-win"),
            Route::EponaWon => Some("ingo-second-win"),
            Route::MalonCourse => Some("epona-won"),
            Route::MalonRecord => Some("malon-record"),
            Route::MalonSong => Some("adult-awake"),
            Route::BigPoe => Some("epona-won"),
            Route::SwordAgain => Some("child-again"),
            Route::Barinade => Some("jabu-barinade"),
            Route::Sapphire => Some("barinade-beaten"),
            Route::Escape => Some("zora-sapphire"),
            Route::OcarinaOfTime => Some("zelda-fled"),
            Route::ShieldChest => Some("cavern-stairs"),
            Route::SwordChest | Route::MidoShop | Route::NewSaveDekuTree | Route::NewFileDekuTree => None,
        }
    }

    /// Where Link starts instead of the entrance's spawn (a debug start), if anywhere.
    pub fn start(self) -> Option<(Vec3, i16)> {
        match self {
            Route::DekuBaba | Route::GameOver => Some(DEKU_BABA_START),
            Route::Combat => Some(COMBAT_START),
            Route::Scrub => Some(SCRUB_START),
            Route::Shutter => Some(SHUTTER_START),
            Route::Stick => Some(STICK_START),
            Route::Push => Some(PUSH_START),
            Route::Slingshot => Some(SLINGSHOT_START),
            Route::BlueWarp => Some(BLUE_WARP_START),
            Route::BossRoom => Some((Vec3::new(-660.0, -1880.0, -620.0), -0x8000)),
            Route::Field => Some(FIELD_START),
            Route::Escape => Some(ESCAPE_START),
            Route::OcarinaOfTime => Some(MOAT_START),
            Route::FieldNight => Some(FIELD_NIGHT_START),
            Route::Ocarina => Some(OCARINA_START),
            // On the cliff above the cavern, east of its Bomb Flower, facing west.
            Route::Boulder => Some((Vec3::new(-1307.0, 1409.0, -660.0), -0x4000)),
            // In the lobby's east corridor, east of the chest, facing west.
            Route::BombBag => Some((Vec3::new(1000.0, 80.0, -920.0), -0x4000)),
            // In room 7's alcove corridor north of the gravestone, facing it (south).
            Route::Gravestone => Some((Vec3::new(0.0, 126.0, -3300.0), 0)),
            // On the lobby's island, south of its Bomb Flower, facing it (north).
            Route::Pillar => Some((Vec3::new(-2.0, 0.0, -1240.0), 0)),
            // On the bridge over the skull, between its holes, facing east.
            Route::Skull => Some((Vec3::new(0.0, 591.0, -1535.0), 0x4000)),
            // On the arena's east rim (the lava's square ends at x -530), facing the heart container
            // (-690, -1523.76, -3304) and the warp beyond it (west).
            Route::Ruby => Some((Vec3::new(-470.0, -1504.0, -3304.0), -0x4000)),
            // On the summit 150 out from the Great Fairy's wall (spawn 4 faces out along 5097),
            // facing it.
            Route::FairyWall => Some((SUMMIT_WALL_FRONT, -27671)),
            // Room 2's floor south of the Bomb Flowers' strip, facing the junction (north).
            Route::FallingStairs => Some((Vec3::new(-1959.0, 0.0, -1180.0), -0x8000)),
            // Room 2's balcony, west of the last rupee's crate, facing it (east).
            Route::SilverRupees => Some((Vec3::new(-1650.0, 531.0, -1110.0), 0x4000)),
            // Room 14, north of the gravestone and its chest, facing them (south).
            Route::ShieldChest => Some((Vec3::new(1474.0, 76.0, -4680.0), 0)),
            // Room 15 inside its door, facing the blocks (north).
            Route::TimeBlocks => Some((Vec3::new(-1958.0, 0.0, -650.0), -0x8000)),
            // Room 1's shallows (20 deep, the floor at -20), west of the fish (Obj_Mure 0x5322 at
            // (653, -10, -485)), facing them (east).
            Route::Bottle => Some((Vec3::new(560.0, -20.0, -420.0), 0x4000)),
            // On the song ledge in front of the falls (x 4060 to 4140; the spawn is in the tunnel
            // behind them, across the falls' gap), facing them.
            Route::ZorasRiver => Some((RIVER_SONG_SPOT, 0x4000)),
            // 300 in front of the throne (it faces +z; the spawn is behind it), facing the king.
            Route::KingZora => Some((KING_ZORA_FRONT, i16::MIN)),
            // Room 1 at the end of door 1's corridor, facing the elevator (-z).
            Route::JabuElevator => Some((Vec3::new(0.0, -320.0, -1500.0), i16::MIN)),
            // Room 14's ledge east of the boomerang's chest, facing it (-x).
            Route::JabuBoomerang => Some((Vec3::new(780.0, -1073.0, -2376.0), -0x4000)),
            // Room 1's bottom east of the pit, facing door 14 (+x).
            Route::JabuDoor14 => Some((Vec3::new(160.0, -1233.0, -1703.0), 0x4000)),
            // Room 2 inside door 2, facing in (-z).
            Route::JabuRuto => Some((JABU_ROOM_2_START, i16::MIN)),
            // Room 11 inside door 7, facing in (-z).
            Route::JabuWeb => Some((JABU_ROOM_11_START, i16::MIN)),
            // Room 1 in door 9's corridor (the red barrier's place), facing the door (+x).
            Route::JabuCrystal => Some((JABU_DOOR_9_START, 0x4000)),
            // Room 3's east floor west of Ruto, facing her (+x).
            Route::JabuSwitch | Route::JabuOcto => Some((JABU_ROOM_3_START, 0x4000)),
            // On the frogs' log at its song spot (En_Fr 0x0000), facing as it does.
            Route::Frogs => Some((FROGS_SONG_SPOT, 0x1555)),
            // 70 in front of Bonooru (-154, -1293, 2888), facing him (within 80 he marks Link as
            // one to play to him: PLAYER_STATE2_23).
            Route::Bonooru => Some((BONOORU_FRONT, -0x8000)),
            // Spawn 4's place on the pedestal's step, facing as it does.
            Route::MasterSwordPull => Some((MASTER_SWORD_STEP, -3641)),
            // Where gTempleOfTimeFirstAdultCs leaves Link (its last player cue: (0, 40, 214)),
            // facing the doorway (+z).
            Route::TempleExit => Some((AWAKENING_END, 0)),
            // Where gTempleOfTimePreludeCs leaves Link (its player cues from frame 107: (0, 40,
            // 214)), facing the pedestal (-z).
            Route::SwordBack => Some((AWAKENING_END, i16::MIN)),
            Route::RideEpona => Some((CORRAL_START, CORRAL_START_YAW)),
            Route::FieldRide => Some((RAMP_FOOT, RAMP_FOOT_YAW)),
            Route::IngoRental | Route::IngoRace => Some((BY_INGO, BY_INGO_YAW)),
            // Malon by day stands on Ingo 2's spot.
            Route::MalonCourse => Some((BY_INGO, BY_INGO_YAW)),
            Route::MalonSong => Some((BEFORE_NIGHT_MALON, i16::MIN)),
            Route::BigPoe => Some((RAMP_FOOT, RAMP_FOOT_YAW)),
            _ => None,
        }
    }

    /// The switch flags a debug start sets after `Play_Init` (the game's `--switch`).
    pub fn start_switches(self) -> &'static [i32] {
        match self {
            Route::Stick => &[STICK_TORCH_FLAG],
            // Room 11's clear comes with its timer's flag 0x01 (the golden torch), as on a save
            // that beat it: Obj_Roomtimer 0x07FF is an enemy until its first update, so a room
            // marked cleared never spawns it to set the flag.
            Route::JabuWeb => &[0x01],
            // The green tentacle's flag (room 10's `En_Ba` 0x0C01, the hand test's fight): room
            // 3's green barrier (`En_Bx` 0x0C81) in front of the ledge's way to door 21 gone.
            Route::JabuOcto => &[0x0C],
            _ => &[],
        }
    }

    /// The rooms a debug start marks cleared (`Flags_SetClear`) before it changes room: their
    /// enemies don't spawn (`Actor_Spawn`), as on a save that beat them. Room 1's big Deku Baba
    /// wakes within 500 of Link anywhere on its floor and fights him: the slingshot's run isn't
    /// about it.
    pub fn start_clears(self) -> &'static [i8] {
        match self {
            Route::Slingshot => &[1],
            Route::BossRoom => &[9],
            // Room 2's Skulltulas (one hangs over the balcony), room 14's Poe and room 15's
            // Skulltulas: the hand test's.
            Route::SilverRupees => &[2],
            Route::ShieldChest => &[14],
            Route::TimeBlocks => &[15],
            // Jabu-Jabu: room 0's Shaboms and Octorok, room 1's Biris, Flobberies and Stinger,
            // room 14's Stingers, Like Like and Lizalfos (the hand test's); room 14's clear also
            // brings the boomerang's chest (ROOM_CLEAR_BIG).
            Route::JabuCow => &[0],
            Route::JabuElevator | Route::JabuDoor14 => &[1],
            Route::JabuBoomerang => &[14],
            // Room 2's Biris and Flobbery; room 11's two caged Like Likes (its clear brings the
            // chest, ROOM_CLEAR_SMALL, and the timer's flag 0x01 lights the torch); rooms 1 and
            // 5's (room 5's Like Likes and Skullwalltula).
            Route::JabuRuto => &[2],
            Route::JabuWeb => &[11],
            Route::JabuCrystal => &[1, 5],
            // Room 3's Biri and Shabom (room 6 isn't: its clear sinks Big Octo's platform).
            Route::JabuSwitch | Route::JabuOcto => &[3],
            _ => &[],
        }
    }

    /// The room a debug start changes to after `Play_Init`, if any.
    pub fn start_room(self) -> Option<i8> {
        match self {
            Route::Scrub => Some(SCRUB_START_ROOM),
            Route::Push => Some(3),
            Route::Slingshot => Some(1),
            Route::BossRoom => Some(9),
            Route::FallingStairs | Route::SilverRupees => Some(2),
            Route::ShieldChest => Some(14),
            Route::TimeBlocks => Some(15),
            Route::JabuElevator | Route::JabuDoor14 => Some(1),
            Route::JabuBoomerang => Some(14),
            Route::JabuRuto => Some(2),
            Route::JabuWeb => Some(11),
            Route::JabuCrystal => Some(1),
            Route::JabuSwitch | Route::JabuOcto => Some(3),
            Route::MasterSwordPull | Route::TempleExit | Route::SwordBack => Some(0),
            _ => None,
        }
    }

    /// The debug start on a play state just entered at `entrance()`: the rooms it marks cleared,
    /// the start's room (`Room_RequestNewRoom`, a frame for it to load, `Room_FinishRoomChange`),
    /// then Link placed.
    pub fn debug_start(self, w: &mut PlayState) {
        for &room in self.start_clears() {
            w.flags.set_clear(room);
        }
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
        if self == Route::SilverRupees {
            // The room's actors initialised (the tracker zeroes the count), then four of the five
            // rupees taken: gone, and the overlay's count at 4.
            w.tick_with(oot_game::play::scripted_input(PadState::default(), PadState::default()));
            silver_rupees_four_taken(w);
        }
    }

    /// The cutscene index its debug start enters with (`cutsceneIndex`: a cutscene layer), if any.
    pub fn cutscene(self) -> Option<u16> {
        match self {
            // CS_INDEX_2: Kokiri Forest's layer 6.
            Route::Emerald => Some(0xFFF2),
            // CS_INDEX_1: the cutscene map's layer 5 (CS_DEST_CUTSCENE_MAP_GANON_HORSE's).
            Route::Creation => Some(0xFFF1),
            _ => None,
        }
    }

    /// Whether it starts on the file select's new file (`SaveContext::file_select_new`, the
    /// opening) instead of `SaveContext::new` at the entrance.
    pub fn new_file(self) -> bool {
        self == Route::NewFileDekuTree
    }

    /// The time of day it starts at (hour, minute: the sandbox's `--time`) when it isn't a new
    /// save's 10:00 (`Sram_InitNewSave`).
    pub fn time(self) -> Option<(i32, i32)> {
        match self {
            Route::Dusk => Some((17, 0)),
            Route::FieldNight => Some((20, 0)),
            Route::Ocarina => Some((18, 30)),
            // Dampé is out only after 18:00 (EnTk_Init: dayTime <= CLOCK_TIME(18, 0) kills him).
            Route::Graveyard => Some((18, 30)),
            // Malon sings in the ranch by night (EnMa2_GetSpawnIndex: IS_NIGHT).
            Route::MalonSong => Some((22, 0)),
            _ => None,
        }
    }

    /// `time()` as `dayTime`.
    pub fn day_time(self) -> u16 {
        let (h, m) = self.time().unwrap_or((10, 0));
        oot_game::env::clock_time(h, m) as u16
    }

    /// The save it starts with, entering by `entrance_index` (its entrance's).
    pub fn save(self, entrance_index: u16) -> oot_game::save::SaveContext {
        use oot_game::save::SaveContext;
        if self.new_file() {
            return SaveContext::file_select_new();
        }
        let mut s = SaveContext::new(entrance_index, false, self.day_time());
        if let Some(p) = self.preset() {
            s.apply_preset(p).expect("the route's preset");
        }
        // A debug start is file 2, as the file select made it ("ZELDAZ": Sram_InitSave)
        // (docs/adr/0049-saving.md).
        s.file_num = (SAVE_FILE - 1) as i32;
        s.newf = *b"ZELDAZ";
        if let Some(c) = self.cutscene() {
            s.cutscene_index = c;
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
            Route::Slingshot => "slingshot",
            Route::Pause => "pause",
            Route::DungeonMap => "dungeon-map",
            Route::GameOver => "game-over",
            Route::Save => "save",
            Route::Gohma => "gohma",
            Route::BlueWarp => "blue-warp",
            Route::BossRoom => "boss-room",
            Route::Emerald => "emerald",
            Route::Creation => "creation",
            Route::Farewell => "farewell",
            Route::Dusk => "dusk",
            Route::Field => "field",
            Route::FieldNight => "field-night",
            Route::Market => "market",
            Route::Castle => "castle",
            Route::CastleCaught => "castle-caught",
            Route::Ocarina => "ocarina",
            Route::Zelda => "zelda",
            Route::ZeldaTalk => "zelda-talk",
            Route::Kakariko => "kakariko",
            Route::Graveyard => "graveyard",
            Route::Meadow => "meadow",
            Route::LostWoods => "lost-woods",
            Route::Ranch => "ranch",
            Route::Bombs => "bombs",
            Route::Darunia => "darunia",
            Route::Boulder => "boulder",
            Route::BombBag => "bomb-bag",
            Route::Gravestone => "gravestone",
            Route::Pillar => "pillar",
            Route::Skull => "skull",
            Route::KingDodongo => "kingdodongo",
            Route::Ruby => "ruby",
            Route::FairyWall => "fairy-wall",
            Route::GreatFairy => "great-fairy",
            Route::SummitOwl => "summit-owl",
            Route::FallingStairs => "falling-stairs",
            Route::SilverRupees => "silver-rupees",
            Route::ShieldChest => "shield-chest",
            Route::TimeBlocks => "time-blocks",
            Route::Bottle => "bottle",
            Route::ZorasRiver => "zoras-river",
            Route::KingZora => "king-zora",
            Route::JabuFish => "jabu-fish",
            Route::JabuCow => "jabu-cow",
            Route::JabuElevator => "jabu-elevator",
            Route::JabuBoomerang => "jabu-boomerang",
            Route::JabuDoor14 => "jabu-door-14",
            Route::JabuRuto => "jabu-ruto",
            Route::JabuWeb => "jabu-web",
            Route::JabuCrystal => "jabu-crystal",
            Route::Barinade => "barinade",
            Route::Sapphire => "sapphire",
            Route::Escape => "zelda-escape",
            Route::OcarinaOfTime => "ocarina-of-time",
            Route::JabuSwitch => "jabu-switch",
            Route::JabuOcto => "jabu-octo",
            Route::Frogs => "frogs",
            Route::Bonooru => "bonooru",
            Route::MasterSwordPull => "master-sword-pull",
            Route::MasterSwordBack => "master-sword-back",
            Route::DoorOfTime => "door-of-time",
            Route::FirstPull => "first-pull",
            Route::Awakening => "awakening",
            Route::TempleExit => "temple-exit",
            Route::Ruins => "ruins",
            Route::Prelude => "prelude",
            Route::SwordBack => "sword-back",
            Route::SwordAgain => "sword-again",
            Route::PreludeWarp => "prelude-warp",
            Route::RideEpona => "ride-epona",
            Route::FieldRide => "field-ride",
            Route::IngoRental => "ingo-rental",
            Route::IngoRace => "ingo-race",
            Route::IngoLost => "ingo-lost",
            Route::IngoWon => "ingo-won",
            Route::EponaWon => "epona-won",
            Route::MalonCourse => "malon-course",
            Route::MalonRecord => "malon-record",
            Route::MalonSong => "malon-song",
            Route::BigPoe => "big-poe",
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
            Route::Slingshot => 3000,
            Route::Pause => 1000,
            Route::DungeonMap => 1000,
            Route::GameOver => 1500,
            Route::Save => 1000,
            Route::Gohma => 6000,
            Route::BlueWarp => 3000,
            Route::BossRoom => 1000,
            Route::Emerald => 2500,
            Route::Creation => 9000,
            Route::Farewell => 6000,
            Route::Dusk => 1500,
            Route::Field => 4000,
            Route::FieldNight => 4000,
            Route::Market => 4000,
            Route::Castle => 8000,
            Route::CastleCaught => 2000,
            Route::Ocarina => 3000,
            Route::Zelda => 20000,
            Route::ZeldaTalk => 12000,
            Route::Kakariko => 8000,
            Route::Graveyard => 30000,
            Route::Meadow => 30000,
            Route::LostWoods => 30000,
            Route::Ranch => 20000,
            Route::Bombs => 3000,
            Route::Darunia => 4000,
            Route::Boulder => 1500,
            Route::BombBag => 1500,
            Route::Gravestone => 3000,
            Route::Pillar => 2000,
            Route::Skull => 2000,
            Route::KingDodongo => 2000,
            Route::Ruby => 5000,
            Route::FairyWall => 1500,
            Route::GreatFairy => 5000,
            Route::SummitOwl => 4000,
            Route::FallingStairs => 2000,
            Route::SilverRupees => 1500,
            Route::ShieldChest => 1500,
            Route::TimeBlocks => 1500,
            Route::Bottle => 3000,
            Route::ZorasRiver => 3000,
            Route::KingZora => 3000,
            Route::JabuFish => 3000,
            Route::JabuCow | Route::JabuElevator | Route::JabuBoomerang | Route::JabuDoor14 | Route::JabuRuto | Route::JabuWeb | Route::JabuCrystal => 3000,
            Route::Barinade => 3000,
            Route::Sapphire => 5000,
            Route::Escape => 4000,
            Route::OcarinaOfTime => 8000,
            Route::JabuSwitch => 3000,
            Route::JabuOcto => 6000,
            Route::Frogs => 8000,
            Route::Bonooru => 3000,
            Route::MasterSwordPull | Route::MasterSwordBack => 2000,
            Route::DoorOfTime => 6000,
            Route::FirstPull => 12000,
            Route::Awakening => 4000,
            Route::TempleExit => 3000,
            Route::Ruins => 4000,
            Route::Prelude => 6000,
            Route::SwordBack | Route::SwordAgain => 2000,
            Route::PreludeWarp => 3000,
            Route::RideEpona | Route::FieldRide | Route::IngoRental | Route::IngoLost | Route::IngoWon | Route::EponaWon => 4000,
            Route::MalonCourse | Route::MalonRecord | Route::MalonSong | Route::BigPoe => 4000,
            Route::IngoRace => 8000,
            _ => Playthrough::MAX_FRAMES,
        }
    }

    /// Whether it needs the audio side running (the ocarina's routes: `AudioOcarina_Update` runs in
    /// `Audio_Update`, which a scene's audio spec holds until the audio side has loaded it).
    pub fn needs_audio(self) -> bool {
        matches!(self, Route::Ocarina | Route::Zelda | Route::ZeldaTalk | Route::Graveyard | Route::Meadow | Route::LostWoods | Route::Ranch | Route::Darunia | Route::GreatFairy | Route::TimeBlocks | Route::ZorasRiver | Route::OcarinaOfTime | Route::Frogs | Route::Bonooru | Route::DoorOfTime | Route::Prelude | Route::PreludeWarp | Route::RideEpona | Route::FieldRide | Route::IngoRental | Route::IngoRace | Route::MalonCourse | Route::MalonSong | Route::BigPoe)
    }

    /// The route a sandbox script names.
    pub fn from_script(name: &str) -> Option<Route> {
        [Route::DekuTree, Route::SwordChest, Route::MidoShop, Route::NewSaveDekuTree, Route::NewFileDekuTree, Route::DekuBaba, Route::Combat, Route::Scrub, Route::Shutter, Route::Stick, Route::Push, Route::Slingshot, Route::Pause, Route::DungeonMap, Route::GameOver, Route::Save, Route::Gohma, Route::BlueWarp, Route::BossRoom, Route::Emerald, Route::Creation, Route::Farewell, Route::Dusk, Route::Field, Route::FieldNight, Route::Market, Route::Castle, Route::CastleCaught, Route::Ocarina, Route::Zelda, Route::ZeldaTalk, Route::Kakariko, Route::Graveyard, Route::Meadow, Route::LostWoods, Route::Ranch, Route::Bombs, Route::Darunia, Route::Boulder, Route::BombBag, Route::Gravestone, Route::Pillar, Route::Skull, Route::KingDodongo, Route::Ruby, Route::FairyWall, Route::GreatFairy, Route::SummitOwl, Route::FallingStairs, Route::SilverRupees, Route::ShieldChest, Route::TimeBlocks, Route::Bottle, Route::ZorasRiver, Route::KingZora, Route::JabuFish, Route::JabuCow, Route::JabuElevator, Route::JabuBoomerang, Route::JabuDoor14, Route::JabuRuto, Route::JabuWeb, Route::JabuCrystal, Route::Barinade, Route::Sapphire, Route::Escape, Route::OcarinaOfTime, Route::JabuSwitch, Route::JabuOcto, Route::Frogs, Route::Bonooru, Route::MasterSwordPull, Route::MasterSwordBack, Route::DoorOfTime, Route::FirstPull, Route::Awakening, Route::TempleExit, Route::Ruins, Route::Prelude, Route::SwordBack, Route::SwordAgain, Route::PreludeWarp, Route::RideEpona, Route::FieldRide, Route::IngoRental, Route::IngoRace, Route::IngoLost, Route::IngoWon, Route::EponaWon, Route::MalonCourse, Route::MalonRecord, Route::MalonSong, Route::BigPoe].into_iter().find(|r| r.script() == name)
    }
}

/// `Route::SilverRupees`'s debug start: room 2's silver rupees but the last (on the balcony) gone and
/// `sCollectedCount` 4, as if Link had taken them.
pub fn silver_rupees_four_taken(w: &mut PlayState) {
    use crate::en_g_switch::{ENGSWITCH_SILVER_RUPEE, EnGSwitch};
    let taken: Vec<_> = w
        .actors
        .all()
        .into_iter()
        .filter(|&h| w.actors.downcast::<EnGSwitch>(h).is_some_and(|s| s.type_ == ENGSWITCH_SILVER_RUPEE && s.actor.home_pos.distance(DC_ROOM2_LAST_RUPEE) > 1.0))
        .collect();
    for h in &taken {
        if let Some(a) = w.actors.actor_mut(*h) {
            a.kill();
        }
    }
    crate::en_g_switch::statics(w).collected_count = taken.len() as i16;
}

/// `Task::OcarinaNotes`' notes (`OCARINA_BTN_*`).
#[derive(Debug, Clone)]
enum Tune {
    Notes(Vec<u8>),
    /// The frogs' song game's first (`gFrogsSongPtr`, the pack's: `EnFr_GetNextNoteFrogSong`
    /// before `EVENTCHKINF_SONGS_FOR_FROGS_CHOIR`).
    Frogs,
}

/// What the run does, one task at a time.
#[derive(Debug, Clone)]
enum Task {
    /// Lift the cucco (`En_Niw`) nearest the point when the task starts: towards it until it
    /// offers itself to Link's lift (Player's `interactRangeActor`), A, until he stands carrying it.
    Lift(Vec3, Step),
    /// `Task::Lift` for the actor: towards it until it offers itself to Link's lift, A, until he
    /// stands carrying it.
    LiftWho(Who, Step),
    /// The Master Sword's pedestal (`Bg_Toki_Swd`): towards it until it offers itself (Player's
    /// `interactRangeActor`), A, until its cutscene starts (the pull or the putting back).
    Pedestal(Step),
    /// Epona standing (`ENHORSE_ACT_IDLE`, idle or whinnying): to a point 60 out on her left, then
    /// at her slowly until she offers herself (Player's `rideActor`: "Climb" on A), A, until the
    /// mount is over (`Player_Action_8084CC98` with `av2.actionVar2` set).
    Mount(Step),
    /// On the horse, through the points: the stick at full tilt turned by the angle from her
    /// heading to the point (`EnHorse_StickDirection` reads the stick from her; farther off than
    /// `RIDE_TURN` she's stopped and turned in place), each passed within `RIDE_RADIUS`; A (a spur:
    /// "Faster") whenever she gallops lined up with all six carrots, if asked; past the last, the
    /// stick let go until she stands, then the step.
    Ride(Vec<Vec3>, bool, Step),
    /// `Ride` through the points, not stopping, then at the exit to the entrance (the middle of its
    /// polys), until the transition starts.
    RideExit(Vec<Vec3>, &'static str, Step),
    /// On the horse, idle until the scene has changed and its fade is over, then until the horse
    /// stands (`EN_HORSE_CHECK_4`).
    RideIn(Step),
    /// On a standing horse ("Down": `PLAYER_STATE2_22`), A until Link is off it and standing.
    Dismount(Step),
    /// `Ride` through the points, not stopping, spurring whenever lined up and moving with a carrot
    /// left, toward the last one until the condition holds.
    RideUntil(Vec<Vec3>, fn(&PlayState) -> bool, Step),
    /// On the horse: ridden to the point, halted, turned to `Who`, A until the talk starts, then
    /// as `Converse` (the choices in order, until the condition once the box is gone).
    RideTalk(Who, Vec3, Vec<u8>, fn(&PlayState) -> bool, Step),
    /// Towards the point until Link faces it, then A: what he carries is thrown (a cucco always is:
    /// `ACTOR_FLAG_THROW_ONLY`), until his hands are free and he stands.
    Throw(Vec3, Step),
    /// Lift the Bomb Flower placed nearest the point: towards it until it offers its bomb (Player's
    /// `interactRangeActor`), A, until Link stands holding the bomb it hands him.
    LiftBombFlower(Vec3, Step),
    /// Carrying, through the points (`Task::Walk`'s), then turned towards the second point (within
    /// 0x1000) and still (slower than 1.1: `Player_CanThrowCarriedActor` puts it down), A, until
    /// his hands are free and he stands.
    PutDown(Vec<Vec3>, Vec3, Step),
    /// This C button (0 C-Left) once, until Link stands holding a bomb (`En_Bom`).
    TakeBomb(usize, Step),
    /// This C button (0 C-Left) once: an item used in place (a bottle drunk or let out), A through
    /// any text; done when Link stands again with no box open.
    UseItem(usize, Step),
    /// With an empty bottle on this C button: out (the button), at the nearest fish (`En_Fish`)
    /// slowly until it offers itself (Player's `interactRangeActor`), the button again (the swing),
    /// A through the catch's text; done when Link stands with a fish in the bottle. A miss tries
    /// again.
    CatchInBottle(usize, Step),
    /// Carrying, through the points at a run (`Task::Walk`'s), then on towards the last; within 20
    /// of the release point (the second `Vec3`), running (faster than 1.1: Player throws rather
    /// than puts down), A, until his hands are free.
    ThrowAtRun(Vec<Vec3>, Vec3, Step),
    /// Through the points carrying a cucco: off an edge Link glides (Player's `minVelocityY` -2,
    /// `Player_UpperAction_CarryActor`); the step on the frame he lands, then on through the rest.
    Glide(Vec<Vec3>, Step),
    /// Still until Kakariko's gate (`Bg_Gate_Shutter`) has slid open and the guard's texts are read.
    WaitGate(Step),
    /// Idle until the transition is over and Link stands, then `SETTLE_FRAMES` more.
    Settle(Option<Step>),
    /// Steer into the floor of the exit to the entrance, until the transition starts; then
    /// (unless the step is the exit itself) wait for the scene change and settle.
    Exit(&'static str, Option<Step>),
    /// Steer through the points, each passed within `WAYPOINT_RADIUS`.
    Walk(Vec<Vec3>),
    /// `Walk` on until the condition holds (a trigger on the way that takes Link before he gets
    /// there: the drawbridge's), then the step; A through any text on the way.
    WalkUntil(Vec<Vec3>, fn(&PlayState) -> bool, Step),
    /// Steer at the point (up a climbable wall, a ledge) until Link stands at its height, settled.
    ClimbTo(Vec3, Step),
    /// From the point, at the actor until it offers a talk, then its C button (0 C-Left): the item
    /// shown (Player's exchange), its texts read to the end.
    ShowItem(Who, Vec3, usize, Step),
    /// Still until the actor is gone (killed), then settled.
    WaitGone(Who, Step),
    /// Towards the point until a castle sentry sees Link ("Hey you!", 0x702D: `Step::Caught`), A
    /// through his text; the throw-out's scene entered and settled (the step).
    Caught(Vec3, Step),
    /// The milk crate (`Bg_Spot15_Rrbox`) nearest the point, grabbed (A) from the spot at the
    /// offset from it and pushed along the axis until its position along it reaches the value, or
    /// it falls into the moat and lands (then the step).
    PushCrate(Vec3, Vec3, Vec3, f32, Option<Step>),
    /// From the porch onto the ladder's top, then down it.
    LadderDown,
    /// Talk to the actor: walk to the point, steer at the actor until it offers, press A, and
    /// press A through its text until the box closes.
    Talk(Who, Vec3, Step),
    /// The ocarina out with its C button (0 C-Left) and the song played (`gOcarinaSongButtons`,
    /// the pack's: each note held 3 frames, let go 2), the step when the staff knows it
    /// (`unk_E3F2`); done when the ocarina is away.
    Ocarina(usize, u8, Step),
    /// `Ocarina`, done when the song is heard (the listener's texts follow with the ocarina out:
    /// the frogs').
    OcarinaHeard(usize, u8, Step),
    /// The ocarina out with its C button (0 C-Left) to a listener that starts its own box (a
    /// scarecrow's recording, the frogs' song game), A through its texts until the condition holds
    /// (the box ready for notes); then the tune, a note at a time, each begun only while the
    /// condition holds (held 3 frames, let go 2); the step after the last.
    OcarinaNotes(usize, fn(&PlayState) -> bool, Tune, Step),
    /// Through the text that opens (within 120 frames): each choice answered from the list in
    /// turn (the stick down for the second), each text on with A; done when it closes.
    TalkThrough(Vec<u8>, Step),
    /// Until the Sun's Song's fast time is over (`sunsSongState` inactive), by day.
    WaitDawn(Step),
    /// Onto the item (`En_Item00`) nearest Link when the task starts, until it's collected; then
    /// the step. Nothing within 200: the step at once.
    PickNearest(Step),
    /// Until a big fairy (`En_Elf` `FAIRY_HEAL_BIG`) is out within 200 of the point.
    WaitFairy(Vec3, Step),
    /// Through the points (`Task::Walk`'s), unseen by the courtyard's guards (`En_Heishi1`): still
    /// while the next steps would be in a guard's sight (`sneak`).
    Sneak(Vec<Vec3>),
    /// Still until the courtyard's guards (`En_Heishi1`) are where the condition wants them: the
    /// phases of their rounds where they look away from the way on (`courtyard`'s conditions).
    WaitGuards(fn(&PlayState) -> bool, &'static str),
    /// A talk that runs on through scripts and scenes: from the point, at the actor until it offers,
    /// A (`Task::Talk`'s start); then A at every box that waits (or an event's), each choice
    /// answered from the list in turn (the first answer past its end), until the condition holds
    /// with no box open (`converse`).
    Converse(Who, Vec3, Vec<u8>, fn(&PlayState) -> bool, Step),
    /// A through the boxes that wait (or an event's) until the condition holds with no box open.
    Texts(fn(&PlayState) -> bool, Step),
    /// `Texts` until the `Cond` holds.
    TextsFor(Cond, Step),
    /// A cutscene's ocarina playback (`MSGMODE_SONG_PLAYBACK`): one wrong note first (and A through
    /// the failure's text) if asked, then the song's notes; A through the texts around it; done (the
    /// step) once the playback's success is seen.
    Playback(u8, bool, Step),
    /// Cut the first of these bushes (home positions) that drops something, and collect it.
    CutBushes(Vec<Vec3>),
    /// Steer through the points at full tilt.
    Hurry(Vec<Vec3>),
    /// Swimming: A held (the dive) steering at the point until within reach of it across, then let
    /// go until Link is back at the water's surface (this height, within 40): under a wall that
    /// stops short of the bottom.
    DiveUnder(Vec3, f32, Step),
    /// Swimming: the dive (A pressed at the surface, held under it) steering at the point until the
    /// condition holds (an item taken from the bottom: the Ocarina of Time's), then the step.
    DiveFor(Vec3, fn(&PlayState) -> bool, Step),
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
    /// The pause menu opened (Start) and closed again (Start), whose resume runs the equipment
    /// page's stand-in, until everything owned is worn (`SaveContext::equip_owned_unworn` has
    /// nothing left to do).
    Equip(Step),
    /// Start until the pause menu is open and idle.
    OpenMenu(Step),
    /// The stick (a push, then a release, each move) until the item page's cursor is on this slot.
    MenuCursor(u16, Step),
    /// This C button (0 C-Left, 1 C-Down, 2 C-Right) until the item under the cursor is on it.
    EquipOnC(usize, Step),
    /// R until the menu has turned to the page on the right.
    TurnPageRight(Step),
    /// The stick up (a push, then a release) until the dungeon map page shows another floor.
    MapFloorUp(Step),
    /// Idle until Link is dead and the game over has started.
    WaitDeath(Step),
    /// Idle until the pause menu is in this state.
    WaitPauseState(u16, Step),
    /// At a prompt, the stick to this choice (`promptChoice`: 0 yes, 4 no), then A, until the
    /// state moves on.
    PromptChoice(i16, Step),
    /// Start until the menu has closed.
    CloseMenu(Step),
    /// B until the pause menu's save prompt has turned in.
    OpenSavePrompt(Step),
    /// At the save prompt, the stick to "Yes", then A, until the save is written.
    SaveYes(Step),
    /// The console's reset (`Playthrough::take_reset`: the runner loads this file), then idle
    /// until Link stands.
    Reset(usize, Step),
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
    /// Kill the Wolfos (`En_Wf`) whose home is here: at it while it's buried (it rises within 240);
    /// turned to it and locked on (Z); in at it from beyond 75, the shield up, and the sword (B,
    /// every other frame) when it's open (its slash's spin from frame 20, bounced off, hurt or
    /// stunned) within 90; done when it's gone and its switch flag is set (the step).
    FightWolfos(Vec3, i32, Step),
    /// Kill the withered Deku Baba whose home is here and pick up its stick (`fight_karebaba`).
    FightKarebaba(Vec3),
    /// The guard up until the Mad Scrub whose home is here is knocked out by its own nut
    /// (`bounce_nut`).
    BounceNut(Vec3),
    /// Catch that Mad Scrub and kill it, and pick up its drop (`catch_scrub`).
    CatchScrub(Vec3),
    /// After that Mad Scrub at a run (it freezes where the camera doesn't see it) until it's within
    /// 150, then `CatchScrub`.
    ChaseScrub(Vec3),
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
    /// The slingshot's C button (C-Right if it's on none) held until the slingshot is drawn with a seed in hand in first person.
    DrawSlingshot(Step),
    /// Its C button still held, the stick steering the aim until the seed in hand points at this
    /// position, then let go, until this switch flag is set (a miss draws again).
    Shoot(Vec3, i32, Step),
    /// `Task::Shoot` until the condition holds instead of a switch flag.
    ShootUntil(Vec3, fn(&PlayState) -> bool, Step),
    /// A until first person is over.
    LeaveFirstPerson,
    /// To the boss room's entrance until Queen Gohma's intro starts (`Step::GohmaEntered`), then
    /// idle until its first part is over (`Step::GohmaWaiting`).
    GohmaIntro,
    /// In to `GOHMA_LOOK_FROM`, then the slingshot drawn and aimed at her eye on the ceiling
    /// (C-Right held: no seed shot) until she's noticed and her eye roll starts.
    GohmaLook(Step),
    /// Idle (C-Right let go once out of first person) until her intro is over: the fight, the main
    /// camera back.
    GohmaWaitBattle(Step),
    /// The fight (`gohma_fight`), until she's defeated.
    GohmaFight(Step),
    /// Idle until her death's cutscene is over: she's gone, Link free.
    GohmaWaitGone(Step),
    /// To the heart container and A through its text until it's taken and Link stands.
    TakeHeart(Step),
    /// Until the blue warp (`Door_Warp1` 0) has grown in, then into it until it takes Link
    /// (`DoorWarp1_ChildWarpOut`).
    IntoWarp(Step),
    /// Until Ruto's blue warp (`Door_Warp1` `WARP_BLUE_RUTO`) is ready (she has risen through it),
    /// then into it until it holds Link (`DoorWarp1_RutoWarpIdle` done).
    IntoRutoWarp(Step),
    /// Idle until the scene changes.
    WarpOut(Step),
    /// Kokiri Forest's arrival: idle as Link falls (`Step::Arrived` once he's landed), A through
    /// the cutscene's texts (the first choice), until its terminator starts the transition (once
    /// the script has run).
    EmeraldPart1(Step),
    /// A blue warp's arrival cutscene (the Goron's Ruby's): `EmeraldPart1`'s, for up to this many
    /// frames (its texts hold the script).
    WarpCutscene(usize, Step),
    /// Kokiri Forest's layer 6: `Step::EmeraldShown` once the emerald is drawn,
    /// `Step::TreeDeath` at the death's frame, A through the texts, until the terminator starts
    /// the transition.
    Emerald(Step),
    /// Idle until the scene changes (a transition already started), then until Link stands in
    /// the new one with the fade over (`settle`).
    NewScene(Step),
    /// The emerald's cutscene chain: each part's step as its scene's cutscene layer is entered
    /// (`CHAIN`), A through the texts, until Link stands at `ENTR_KOKIRI_FOREST_11` (`step`).
    Chain(Step),
    /// Steer into the floor of the exit to the entrance until the transition starts (`step`); the
    /// next task waits for the scene.
    ExitInto(&'static str, Step),
    /// The bridge and the field's intro: `Step::Bridge` in the Lost Woods' layer 4,
    /// `Step::FairyOcarina` at its text, `Step::Field` in Hyrule Field, A through the texts,
    /// until Link stands in the field with no cutscene running (`step`).
    Farewell(Step),
    /// The owl (`En_Owl`): towards him until his talk takes Link (`Step::OwlTalk`), A through
    /// his texts, his question answered "OK" (choice 1: 0 hears it again), until he has flown
    /// off and Link stands (`step`).
    Owl(Step),
    /// At the push block (from beside it, lower down) until Link stands on its top.
    ClimbBlock(Step),
    /// Idle until the condition holds.
    WaitFor(Cond, Step),
    /// On the memory game's stump: the ocarina out (C-Down) whenever nothing runs, A through the
    /// texts (yes to another round), each round's notes played back as the Skull Kids played them
    /// (`OCARINA_SONG_MEMORY_GAME`'s notes so far, each held 3 frames, let go 2), until
    /// `ITEMGETINF_17` is set and the box is closed (the step); `Step::MemoryGame` and
    /// `Step::MemoryRound` on the way.
    MemoryGame(Step),
    /// Talon's Super Cucco game: after the nearest super cucco (`En_Niw` 0xD) Talon still counts
    /// (gravity at or under 0: a found one floats off with 0.1), lift it (A once Player is offered
    /// it), hold it until Talon's text opens, A through it; done when the box is closed and Link's
    /// hands are free, or the game's end has started its transition.
    CatchSuperCucco(Step),
    /// Open the door (`En_Door`) by this point: to the point, at the door slowly until Player is
    /// offered it (`PLAYER_DOORTYPE_HANDLE`), A; done when a scene exit's transition starts (the
    /// next task waits for the scene).
    OpenDoor(Vec3, Vec3, Option<Step>),
    /// The gravestone (`Bg_Haka`) whose home is here pulled: at it slowly until Link holds on
    /// (A at its wall), then A and the stick away from it until it's pulled its 60
    /// (`BgHaka_IdleOpened`) and Link has let go.
    PullStone(Vec3, Step),
    /// At a run onto the `Bg_Ddan_Jd` whose home is here (its middle), off whatever edge is in
    /// the way: done when Link stands on its collision.
    RunOntoPillar(Vec3, Step),
    /// The ocarina out with its C button (0 C-Left), no song: until a text opens (an actor
    /// waiting for it, Malon's lesson: `PLAYER_STATE2_24`), then the step.
    OcarinaOut(usize, Step),
    /// The grounded Peahat whose home is here: locked on (Z), guarding (R) its blades, the
    /// sword (B) at its root whenever it's within 70, seeking or recoiling, until it's gone.
    FightPeahat(Vec3, Step),
    /// Stalchildren as they rise: locked on, at the nearest (in to 50), the sword within 55,
    /// guarding otherwise, until this many have died (0: until none is left but sinking ones).
    FightStalchildren(usize, Step),
}

/// What `Task::WaitFor` and `Task::TextsFor` wait for.
#[derive(Debug, Clone, Copy)]
enum Cond {
    /// The time-based music past `TIMESEQ_FADE_DAY_BGM` (its stop sent).
    MusicFaded,
    /// `nightFlag`.
    Night,
    /// The drawbridge (`Bg_Spot00_Hanebasi`, `DT_DRAWBRIDGE`) at -0x4000.
    BridgeRaised,
    /// The time-based music at `TIMESEQ_DAY_BEGIN_SFX` (the night's critters set).
    NightCritters,
    /// Darunia's door (`Bg_Spot18_Shutter`) risen its 180 (`func_808B9698` done), Link free.
    DaruniaDoorOpen,
    /// The switch flag set, Link free (no cutscene or text holding him).
    Switch(i32),
    /// The `Bg_Ddan_Jd` whose home is here idle this far above it, Link free.
    Pillar(Vec3, f32),
    /// The crate (`Obj_Kibako2`) whose home is here gone.
    CrateGone(Vec3),
    /// The Song of Time block (`Obj_Timeblock`) whose home is here hidden (its collision off).
    TimeBlockHidden(Vec3),
    /// The `eventChkInf` flag set, Link free (no cutscene or text holding him).
    EventChkInf(u16),
}

impl Cond {
    fn holds(self, w: &PlayState) -> bool {
        use oot_game::audio::scene::{TIMESEQ_DAY_BEGIN_SFX, TIMESEQ_NIGHT_BEGIN_SFX};
        match self {
            Cond::MusicFaded => w.time_seq_state >= TIMESEQ_NIGHT_BEGIN_SFX && w.time_seq_state != oot_game::audio::scene::TIMESEQ_DISABLED,
            Cond::Night => w.save.night_flag,
            Cond::BridgeRaised => w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<crate::bg_spot00_hanebasi::BgSpot00Hanebasi>(h)).any(|b| b.actor.params == crate::bg_spot00_hanebasi::DT_DRAWBRIDGE && b.actor.shape_rot.x == -0x4000),
            Cond::NightCritters => w.time_seq_state == TIMESEQ_DAY_BEGIN_SFX,
            Cond::DaruniaDoorOpen => {
                w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<crate::bg_spot18_shutter::BgSpot18Shutter>(h)).any(|d| d.actor.world_pos.y >= d.actor.home_pos.y + 180.0) && !w.player_in_cs_mode()
            }
            Cond::Switch(flag) => w.flags.get_switch(flag) && !w.player_in_cs_mode() && w.message_state() == oot_game::message::TEXT_STATE_NONE,
            Cond::EventChkInf(flag) => w.save.get_event_chk_inf(flag) && !w.player_in_cs_mode() && w.message_state() == oot_game::message::TEXT_STATE_NONE,
            Cond::CrateGone(home) => !w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<crate::obj_kibako2::ObjKibako2>(h)).any(|c| !c.actor.killed && c.actor.home_pos.distance(home) < 1.0),
            Cond::TimeBlockHidden(home) => w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<crate::obj_timeblock::ObjTimeblock>(h)).any(|t| t.actor.home_pos.distance(home) < 1.0 && !t.is_visible),
            Cond::Pillar(home, rise) => {
                !w.player_in_cs_mode()
                    && w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<crate::bg_ddan_jd::BgDdanJd>(h)).any(|p| p.actor.home_pos.distance(home) < 1.0 && p.action == crate::bg_ddan_jd::Action::Idle && p.actor.world_pos.y == home.y + rise)
            }
        }
    }
}

/// The pause menu open and idle (`PAUSE_STATE_MAIN`, `PAUSE_MAIN_STATE_IDLE`).
fn menu_idle(w: &PlayState) -> bool {
    w.pause_ctx.state == oot_game::kaleido::PAUSE_STATE_MAIN && w.pause_ctx.main_state == oot_game::kaleido::PAUSE_MAIN_STATE_IDLE
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
    /// The `En_Hy` of this type (`params & 0x7F`).
    Hylian(i16),
    /// Malon (`En_Ma1`).
    Malon,
    /// Talon (`En_Ta`).
    Talon,
    /// Zelda (`En_Zl4`).
    Zelda,
    /// Kakariko's gate guard (`En_Heishi2` type 5).
    GateGuard,
    /// The royal tomb's wall (`En_Okarina_Tag` type 5).
    SongWall,
    /// Dampé (`En_Tk`).
    Dampe,
    /// An owl (`En_Owl`).
    Owl,
    /// King Zora (`En_Kz`).
    KingZora,
    /// Ruto (`En_Ru1`).
    Ruto,
    /// Bonooru (`En_Kakasi3`).
    Bonooru,
    /// The ranch's adult Ingo on foot (`En_In` params 2 to 4).
    Ingo,
    /// Adult Malon by the corral after Epona (`En_Ma3`).
    AdultMalon,
}

const SETTLE_FRAMES: usize = 10;
const WAYPOINT_RADIUS: f32 = 30.0;
/// `Task::Sneak`: how far ahead of Link a step is checked, the margin on a guard's 70, the turn
/// past his head's widest look, and the frames ahead a flying ball is followed.
const SNEAK_AHEAD: f32 = 40.0;
const SNEAK_MARGIN: f32 = 20.0;
const SNEAK_FAN: i32 = 0x800;
const SNEAK_BALL_FRAMES: usize = 12;
/// The stick's tilt for a run (of the pad's ±80), and for the slow approaches: a walk.
/// (`Player_CalcSpeedAndYawFromControlStick` takes 20 off the dead-zoned magnitude, so much under 30 only turns Link.)
const RUN: f32 = 60.0;
const SLOW: f32 = 40.0;
/// Full tilt (the stick's relative position clamps at 60 past the dead zone).
const FULL: f32 = 80.0;
/// `Task::Ride`: how near a point counts as passed (a gallop covers 8 to 14 a frame), and the most
/// the stick turns from her heading (`cos` above `EnHorse_MountedIdle`'s 0.7071: a walk, not a
/// turn in place).
const RIDE_RADIUS: f32 = 120.0;
const RIDE_TURN: i32 = 0x1F00;
/// How much more the stick turns than she's off: a gallop turns slowly.
const RIDE_GAIN: i32 = 4;
/// `Task::RideTalk`'s approach: the stick at this share of full tilt, a walk.
const RIDE_WALK: f32 = 0.45;

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
    /// The answers `Task::Converse` gives in place of its own (`with_choices`).
    choices: Option<Vec<u8>>,
    /// The frames spent waiting for the boulder, each wait.
    pub boulder_waits: Vec<usize>,
    /// The rupees Link has at each step (`gSaveContext.rupees`).
    pub rupees: Vec<(Step, i16)>,
    /// The tries a switch took (`Task::SlashSwitch`).
    tries: usize,
    /// The rupees on the last frame seen.
    last_rupees: i16,
    /// The menu's page when `Task::TurnPageRight` pressed R.
    items_page: u16,
    /// The file `Task::Reset` asks the runner to load.
    reset: Option<usize>,
    /// The owl `Task::Owl` talks to (the nearest when it started).
    owl: Option<ActorHandle>,
    /// `Task::PushCrate`'s crate, kept once chosen.
    crate_handle: Option<ActorHandle>,
    /// The item page's cursor positions `Task::MenuCursor` has been at.
    cursor_seen: Vec<(i16, i16)>,
    /// The cucco `Task::Lift` took, until `Task::Throw` throws it.
    carried: Option<ActorHandle>,
    /// `Task::Ocarina`'s note frames, `Task::TalkThrough`'s choices answered.
    notes: usize,
    /// A cap on the run's length: its routes' caps added up.
    max_frames: usize,
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
            Route::Combat => vec![Task::BlockKeese(COMBAT_KEESE_HOME), Task::KillKeese(COMBAT_KEESE_HOME), Task::FightKarebaba(COMBAT_KAREBABA_HOME)],
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
            Route::Slingshot => vec![
                Task::DrawSlingshot(Step::SlingshotDrawn),
                Task::Shoot(ROOM1_EYE_HOME + ROOM1_EYE_FRONT, ROOM1_EYE_FLAG, Step::EyeShot),
                Task::LeaveFirstPerson,
                // The door to room 2 unbars with its attention camera.
                Task::WaitCameras,
                Task::OpenSlidingDoor(ROOM1_DOOR, ROOM1_DOOR_FRONT, Step::DoorOpened),
                Task::WaitDoorShut(ROOM1_DOOR, Step::ThroughDoor),
            ],
            Route::Pause => vec![
                Task::Settle(None),
                Task::OpenMenu(Step::MenuOpened),
                Task::MenuCursor(PAUSE_SLOT, Step::CursorOnItem),
                Task::EquipOnC(PAUSE_C_BUTTON, Step::ItemEquipped),
                Task::TurnPageRight(Step::PageTurned),
                Task::CloseMenu(Step::MenuClosed),
            ],
            Route::DungeonMap => vec![
                Task::Settle(None),
                Task::OpenMenu(Step::MenuOpened),
                Task::TurnPageRight(Step::PageTurned),
                Task::MapFloorUp(Step::FloorChanged),
                Task::CloseMenu(Step::MenuClosed),
            ],
            Route::GameOver => vec![
                Task::WaitDeath(Step::Died),
                Task::WaitPauseState(oot_game::kaleido::PAUSE_STATE_GAME_OVER_SAVE_PROMPT, Step::SavePrompt),
                Task::PromptChoice(4, Step::ContinuePrompt),
                Task::PromptChoice(0, Step::Respawned),
            ],
            Route::Save => vec![
                Task::Settle(None),
                Task::OpenMenu(Step::MenuOpened),
                Task::MenuCursor(PAUSE_SLOT, Step::CursorOnItem),
                Task::EquipOnC(SAVE_C_BUTTON, Step::ItemEquipped),
                Task::OpenSavePrompt(Step::SavePrompt),
                Task::SaveYes(Step::Saved),
                Task::WaitPauseState(oot_game::kaleido::PAUSE_STATE_OFF, Step::MenuClosed),
                Task::Reset(SAVE_FILE, Step::Loaded),
            ],
            Route::Gohma => vec![
                Task::GohmaIntro,
                Task::GohmaLook(Step::GohmaLookedAt),
                Task::GohmaWaitBattle(Step::GohmaBattle),
                Task::GohmaFight(Step::GohmaDefeated),
                Task::GohmaWaitGone(Step::GohmaGone),
                Task::TakeHeart(Step::HeartTaken),
            ],
            Route::BlueWarp => vec![Task::IntoWarp(Step::WarpEntered), Task::WarpOut(Step::WarpedOut), Task::EmeraldPart1(Step::EmeraldPart1Over)],
            // Room 11 has no actors: its floor past the door is the exit.
            Route::BossRoom => vec![Task::OpenSlidingDoor(ROOM9_DOOR, ROOM9_DOOR_FRONT, Step::DoorOpened), Task::Exit("ENTR_DEKU_TREE_BOSS_0", Some(Step::BossRoom))],
            Route::Emerald => vec![Task::Emerald(Step::EmeraldOver), Task::NewScene(Step::Forest11)],
            Route::Creation => vec![Task::Chain(Step::Forest11)],
            Route::Dusk => vec![
                Task::Settle(None),
                // West a little: the camera behind Link looks into the sunset.
                Task::Walk(vec![Vec3::new(40.0, 0.0, 1430.0)]),
                Task::WaitFor(Cond::MusicFaded, Step::MusicFaded),
                Task::WaitFor(Cond::Night, Step::Night),
                // Round to the castle: the drawbridge rising.
                Task::Walk(vec![Vec3::new(40.0, 0.0, 1330.0)]),
                Task::WaitFor(Cond::BridgeRaised, Step::BridgeRaised),
                Task::WaitFor(Cond::NightCritters, Step::NightCritters),
            ],
            Route::Field => vec![
                Task::FightPeahat(FIELD_PEAHAT_HOME, Step::PeahatKilled),
                // South over the stream through the west lowland, east up its slope onto the
                // castle's side (south of z 2450 the only way: a cliff north of it), and north to
                // the drawbridge and over it.
                Task::Walk(vec![Vec3::new(-3200.0, -300.0, 1600.0), Vec3::new(-2900.0, -300.0, 2650.0), Vec3::new(-1300.0, 0.0, 2650.0), Vec3::new(-500.0, 0.0, 1800.0), Vec3::new(0.0, 0.0, 1300.0)]),
                Task::Exit("ENTR_MARKET_ENTRANCE_DAY_1", Some(Step::CastleTown)),
            ],
            Route::FieldNight => vec![
                Task::FightStalchildren(2, Step::StalchildrenKilled),
                // East over the dry stream bed where its banks are gentle (z 1680), north along its
                // east bank to the owl's tree at the stairs' foot (he takes Link within 480): his
                // talk (OWL_KAKARIKO, text 0x206C) and his flight; then up the stairs.
                Task::Walk(vec![Vec3::new(2600.0, -50.0, 1680.0), Vec3::new(3350.0, -50.0, 1680.0)]),
                // (More rise round him there: frozen through the talk, PLAYER_STATE1_TALKING; the
                // owl's task fights them while he flies off.) Then straight up the stairs past
                // them (GAME-10 milestone 1: crossing the stream, its current, sWaterConveyorSpeeds,
                // carries Link, and a scripted fight with the ones round the tree was a matter of
                // luck; fights are the hand test's).
                Task::Owl(Step::OwlFlown),
                Task::Exit("ENTR_KAKARIKO_VILLAGE_0", Some(Step::Kakariko)),
            ],
            Route::Market => vec![
                Task::Settle(None),
                Task::Exit("ENTR_MARKET_DAY_0", Some(Step::Market)),
                // North-west from the entrance's spawn (-4, 718): the townsman in the red shirt
                // (type 9, at (-72, 245), in front of the blue shirt's (-85, 204)) from south of him.
                Task::Walk(vec![Vec3::new(-120.0, 0.0, 450.0)]),
                Task::Talk(Who::Hylian(crate::en_hy::ENHY_TYPE_MAN_2_MUSTACHE_RED_SHIRT), Vec3::new(-100.0, 0.0, 320.0), Step::Townsman),
                // West of the fountain to Malon (-3, -181), from her west.
                Task::Walk(vec![Vec3::new(-220.0, 0.0, 150.0), Vec3::new(-220.0, 0.0, -150.0)]),
                Task::Talk(Who::Malon, Vec3::new(-90.0, 0.0, -190.0), Step::Malon),
                // North out of the market up the road to the castle.
                Task::Walk(vec![Vec3::new(-60.0, 0.0, -400.0)]),
                Task::Exit("ENTR_HYRULE_CASTLE_0", Some(Step::CastleRoad)),
            ],
            Route::Castle => vec![
                Task::Settle(None),
                // The castle's owl (OWL_HYRULE_CASTLE, on the road's west wall) calls Link within
                // 540: his talk (0x2068, 0x2069, the question 0x206A: OK) and his flight.
                Task::Owl(Step::OwlFlown),
                // East along the road to the vine by Malon (WALL_TYPE_4: x 90 to 92, z 3385 to
                // 3408, 1091 up to 1367), up it.
                Task::Walk(vec![Vec3::new(40.0, 1091.0, 3396.0)]),
                Task::ClimbTo(Vec3::new(160.0, 1367.0, 3396.0), Step::Vine),
                // East along the wall's top onto the bridge over the gate (1443), off its east side
                // (330 down: under 400, no fall damage) onto the road past Lon Lon's fence, up that
                // road north, then west over the field outside the sentries' sight (250 in front of
                // each, 150 behind) to the second vine (from (-652, 1558) to (-613, 1532), 1450 up
                // to 1606).
                Task::Walk(vec![
                    Vec3::new(940.0, 1429.0, 3390.0),
                    Vec3::new(1120.0, 1443.0, 3210.0),
                    Vec3::new(1220.0, 1443.0, 3150.0),
                    Vec3::new(1650.0, 1113.0, 2720.0),
                    Vec3::new(1670.0, 1224.0, 2580.0),
                    Vec3::new(1850.0, 1302.0, 2360.0),
                    Vec3::new(1850.0, 1330.0, 2280.0),
                    Vec3::new(1610.0, 1375.0, 2070.0),
                    Vec3::new(-60.0, 1393.0, 2070.0),
                    Vec3::new(-590.0, 1450.0, 1600.0),
                    Vec3::new(-615.0, 1450.0, 1572.0),
                ]),
                // The vine faces south-east (the plateau is north of it).
                Task::ClimbTo(Vec3::new(-655.0, 1606.0, 1510.0), Step::Vine),
                // North over the west plateau, off its edge onto the castle's lawn (1571) west of
                // the door's two sentries (their box x -380 to -190, z 700 to 1020), into the moat
                // and east along it under them (z 610) to its corner, where a ramp under the water
                // rises to 1522 (the channel north is barred at z 360); up onto the east bank.
                Task::Walk(vec![
                    Vec3::new(-680.0, 1713.0, 1080.0),
                    Vec3::new(-790.0, 1765.0, 970.0),
                    Vec3::new(-760.0, 1571.0, 900.0),
                    Vec3::new(-480.0, 1571.0, 720.0),
                    Vec3::new(-450.0, 1510.0, 610.0),
                    Vec3::new(500.0, 1510.0, 610.0),
                    Vec3::new(770.0, 1520.0, 640.0),
                ]),
                Task::ClimbTo(Vec3::new(850.0, 1571.0, 640.0), Step::MoatBank),
                // North along the bank to Talon, asleep by the crates.
                Task::Walk(vec![Vec3::new(900.0, 1571.0, -170.0)]),
                Task::ShowItem(Who::Talon, Vec3::new(950.0, 1561.0, -299.0), 0, Step::TalonWoken),
                Task::Talk(Who::Talon, Vec3::new(950.0, 1561.0, -299.0), Step::TalonTalk),
                Task::WaitGone(Who::Talon, Step::TalonGone),
                // The crates, now free to go west or south by Talon's place (func_808B3AAC): the
                // one at (950, -459) 8 pushes south to z -299, then 9 west off the bank (its edge
                // at x 800) into the moat at x 770; the one at (910, -519) 11 south, 7 west, onto
                // it (the chime: squarely on it).
                // (Round the crates east to their spots: walked into, a crate's 60 is climbed.)
                Task::Walk(vec![Vec3::new(1020.0, 1561.0, -330.0), Vec3::new(1020.0, 1561.0, -505.0)]),
                Task::PushCrate(Vec3::new(950.0, 1561.0, -459.0), Vec3::new(15.0, 0.0, -46.0), Vec3::Z, -299.0, None),
                Task::Walk(vec![Vec3::new(1020.0, 1561.0, -360.0), Vec3::new(1020.0, 1561.0, -300.0)]),
                Task::PushCrate(Vec3::new(950.0, 1561.0, -299.0), Vec3::new(50.0, 0.0, 0.0), Vec3::NEG_X, -770.0, Some(Step::CrateInMoat)),
                Task::Walk(vec![Vec3::new(1000.0, 1561.0, -420.0), Vec3::new(1000.0, 1561.0, -570.0)]),
                Task::PushCrate(Vec3::new(910.0, 1561.0, -519.0), Vec3::new(0.0, 0.0, -46.0), Vec3::Z, -299.0, None),
                Task::Walk(vec![Vec3::new(990.0, 1561.0, -360.0), Vec3::new(990.0, 1561.0, -300.0)]),
                Task::PushCrate(Vec3::new(910.0, 1561.0, -299.0), Vec3::new(50.0, 0.0, 0.0), Vec3::NEG_X, -770.0, Some(Step::CrateInMoat)),
                // Up onto the stack (its top 60 above the bank), and west off it over the moat
                // onto the castle wall's ledge (1617) by the drain.
                Task::ClimbTo(Vec3::new(790.0, 1621.0, -299.0), Step::OnCrates),
                Task::Hurry(vec![Vec3::new(600.0, 1617.0, -305.0)]),
                // Into the drain (WALL_TYPE_5 at x 560, z -293 to -325), crawling west to its end:
                // exit 2, the guards' courtyard.
                Task::Crawl(Vec3::new(590.0, 1617.0, -309.0), Vec3::new(540.0, 1617.0, -309.0), Step::Drain),
                Task::Exit("ENTR_CASTLE_COURTYARD_GUARDS_DAY_0", Some(Step::Courtyard)),
            ],
            Route::CastleCaught => vec![
                Task::Settle(None),
                Task::Owl(Step::OwlFlown),
                Task::Walk(vec![Vec3::new(40.0, 1091.0, 3396.0)]),
                Task::ClimbTo(Vec3::new(160.0, 1367.0, 3396.0), Step::Vine),
                // East along the wall's top onto the bridge, and north along it towards the field:
                // the sentry at its foot (1196, 1422, 2481) faces it, 250 in front.
                Task::Walk(vec![Vec3::new(940.0, 1429.0, 3390.0), Vec3::new(1120.0, 1443.0, 3210.0), Vec3::new(1200.0, 1443.0, 3000.0)]),
                Task::Caught(Vec3::new(1200.0, 1443.0, 2600.0), Step::ThrownOut),
            ],
            Route::Zelda => vec![
                Task::Settle(None),
                // Out of the drain's channel onto the grass and along the south wall (the patrols'
                // paths end 100 short of it) to the first screen's edge, where no guard looks.
                Task::Sneak(vec![Vec3::new(2205.0, 0.0, 240.0), Vec3::new(2100.0, 0.0, 250.0), Vec3::new(1830.0, 0.0, 255.0)]),
                // The first patrol (path 0) from when he turns north up his west side, round his
                // north side: on along the wall into the next screen.
                Task::WaitGuards(courtyard::screen7, "path 0 up his west side"),
                // East of the pillar at x 960: on the second screen, out of its patrols' looks.
                Task::Sneak(vec![Vec3::new(1100.0, 0.0, 250.0), Vec3::new(1030.0, 0.0, 140.0)]),
                Task::WaitGuards(courtyard::screen2, "path 1 up his west side, path 2 up his east side"),
                // On to the third screen, behind its east fence (60 high, to z 220: the middle
                // patrol's looks under the bridge stop at it).
                Task::Sneak(vec![Vec3::new(1030.0, 0.0, 255.0), Vec3::new(380.0, 0.0, 255.0), Vec3::new(290.0, 0.0, 215.0)]),
                Task::WaitGuards(courtyard::screen3, "path 3 in his north half"),
                // Under the bridge's ends (the gaps south of its fences) to behind its west fence.
                Task::Sneak(vec![Vec3::new(230.0, 0.0, 250.0), Vec3::new(-230.0, 0.0, 250.0), Vec3::new(-290.0, 0.0, 200.0)]),
                Task::WaitGuards(courtyard::screen4, "path 4 at his south-west corner, path 6 at his north-east"),
                // Along the wall to behind path 4's corner, until he's gone up his west side.
                Task::Sneak(vec![Vec3::new(-860.0, 0.0, 262.0)]),
                Task::WaitGuards(courtyard::screen4_west, "path 4 up his west side"),
                Task::Sneak(vec![Vec3::new(-1100.0, 0.0, 260.0)]),
                Task::WaitGuards(courtyard::plaza, "the plaza's patrols"),
                // East of the bench, west along the corridor between it and the corner blocks, through
                // the gap by the south-west block, up the west side behind the patrols, across their
                // north side to the passage north, and west along it.
                Task::Sneak(vec![
                    Vec3::new(-1130.0, 0.0, 50.0),
                    Vec3::new(-1420.0, 0.0, 40.0),
                    Vec3::new(-1420.0, 0.0, -90.0),
                    Vec3::new(-1470.0, 0.0, -130.0),
                    Vec3::new(-1470.0, 0.0, -580.0),
                    Vec3::new(-1310.0, 0.0, -640.0),
                    Vec3::new(-1310.0, 0.0, -740.0),
                ]),
                Task::Sneak(vec![Vec3::new(-1600.0, 0.0, -780.0)]),
                Task::Exit("ENTR_CASTLE_COURTYARD_ZELDA_0", Some(Step::ZeldaCourtyard)),
            ]
            .into_iter()
            .chain(zelda_tail())
            .collect(),
            Route::ZeldaTalk => [Task::Settle(None)].into_iter().chain(zelda_tail()).collect(),
            Route::Kakariko => kakariko_tasks(),
            Route::Graveyard => graveyard_tasks(),
            Route::Meadow => meadow_tasks(),
            Route::LostWoods => lost_woods_tasks(),
            Route::Ranch => ranch_tasks(),
            Route::Bombs => bombs_tasks(),
            Route::Darunia => darunia_tasks(),
            Route::Boulder => boulder_tasks(),
            Route::BombBag => bomb_bag_tasks(),
            Route::Gravestone => gravestone_tasks(),
            Route::Pillar => pillar_tasks(),
            Route::Skull => skull_tasks(),
            Route::KingDodongo => king_dodongo_tasks(),
            Route::Ruby => ruby_tasks(),
            Route::FairyWall => fairy_wall_tasks(),
            Route::GreatFairy => great_fairy_tasks(),
            Route::SummitOwl => summit_owl_tasks(),
            Route::FallingStairs => falling_stairs_tasks(),
            Route::SilverRupees => silver_rupees_tasks(),
            Route::ShieldChest => shield_chest_tasks(),
            Route::TimeBlocks => time_blocks_tasks(),
            Route::Bottle => bottle_tasks(),
            Route::ZorasRiver => zoras_river_tasks(),
            Route::KingZora => king_zora_tasks(),
            Route::JabuFish => jabu_fish_tasks(),
            Route::JabuCow => jabu_cow_tasks(),
            Route::JabuElevator => jabu_elevator_tasks(),
            Route::JabuBoomerang => jabu_boomerang_tasks(),
            Route::JabuDoor14 => jabu_door_14_tasks(),
            Route::JabuRuto => jabu_ruto_tasks(),
            Route::JabuWeb => jabu_web_tasks(),
            Route::JabuCrystal => jabu_crystal_tasks(),
            Route::JabuSwitch => jabu_switch_tasks(),
            Route::JabuOcto => jabu_octo_tasks(),
            Route::Frogs => frogs_tasks(),
            Route::Bonooru => bonooru_tasks(),
            Route::MasterSwordPull => vec![Task::Pedestal(Step::SwordPulled), Task::NewScene(Step::TimeTravelled)],
            Route::MasterSwordBack => vec![Task::Settle(None), Task::Pedestal(Step::SwordPutBack), Task::NewScene(Step::TimeTravelled)],
            Route::DoorOfTime => door_of_time_tasks(),
            Route::FirstPull => first_pull_tasks(),
            Route::Awakening => awakening_tasks(),
            Route::TempleExit => temple_exit_tasks(),
            Route::Ruins => ruins_tasks(),
            Route::Prelude => prelude_tasks(),
            Route::SwordBack => vec![Task::Settle(None), Task::Pedestal(Step::SwordPutBack), Task::NewScene(Step::TimeTravelled)],
            Route::SwordAgain => vec![Task::Settle(None), Task::Pedestal(Step::SwordPulled), Task::NewScene(Step::TimeTravelled)],
            Route::PreludeWarp => prelude_warp_tasks(),
            Route::RideEpona => ride_epona_tasks(),
            Route::FieldRide => field_ride_tasks(),
            Route::IngoRental => ingo_rental_tasks(),
            Route::IngoRace => ingo_race_tasks(),
            Route::IngoLost => vec![
                Task::Texts(|w| crate::en_in::race_state(w) == crate::en_in::INGO_RACE_STATE_REMATCH && w.transition.trigger == TRANS_TRIGGER_OFF, Step::IngoGloated),
                Task::Settle(None),
            ],
            Route::EponaWon => epona_won_tasks(),
            Route::IngoWon => vec![Task::Texts(|w| w.scene_id == SCENE_LON_LON_RANCH && w.save.scene_layer == 4, Step::SecondRaceCalled)],
            Route::MalonCourse => malon_course_tasks(),
            Route::MalonRecord => vec![Task::Texts(|w| w.save.get_event_chk_inf(crate::en_ma3::EVENTCHKINF_HORSE_RACE_COW_UNLOCK), Step::CourseRecord), Task::Settle(None)],
            Route::MalonSong => malon_song_tasks(),
            Route::BigPoe => big_poe_tasks(),
            Route::Barinade => barinade_tasks(),
            Route::Sapphire => sapphire_tasks(),
            Route::Escape => escape_tasks(),
            Route::OcarinaOfTime => ocarina_of_time_tasks(),
            Route::Ocarina => vec![
                Task::Settle(None),
                // Saria's Song: Navi talks (0xE0, naviTextId -0xE0): yes, Saria's text, then no
                // at "talk again?" (0xE3).
                Task::Ocarina(0, oot_game::audio::ocarina::OCARINA_SONG_SARIAS, Step::SariasSong),
                Task::TalkThrough(vec![0, 1], Step::SariaTalk),
                // The Sun's Song by night: the clock at 400 a frame to 06:30.
                Task::Ocarina(0, oot_game::audio::ocarina::OCARINA_SONG_SUNS, Step::SunsSong),
                Task::WaitDawn(Step::Dawn),
                // South to the fairy spot (Shot_Sun hears within 150) and the Song of Storms.
                Task::Walk(vec![OCARINA_STORMS_AT]),
                Task::Settle(None),
                Task::Ocarina(0, oot_game::audio::ocarina::OCARINA_SONG_STORMS, Step::SongOfStorms),
                Task::WaitFairy(SHOT_SUN_HOME, Step::StormFairy),
            ],
            Route::Farewell => vec![
                Task::Settle(None),
                // Mido blocks the path out of the meadow (EnMd_BlockPath: with the emerald, until he's
                // talked to); he walks off along his path, the box held, and is gone at its end.
                Task::Talk(Who::Mido, Vec3::new(1680.0, 0.0, 120.0), Step::Mido),
                // West out of the path, over the ford, past the shop and Mido's house.
                Task::Walk(vec![Vec3::new(1520.0, 0.0, 120.0), Vec3::new(1420.0, 0.0, 230.0), Vec3::new(1375.0, 0.0, 380.0), Vec3::new(1250.0, 0.0, 410.0), Vec3::new(900.0, 0.0, 430.0), Vec3::new(850.0, 0.0, 200.0), Vec3::new(548.0, 3.0, -158.0), Vec3::new(188.0, 3.0, -198.0), Vec3::new(-150.0, 0.0, -350.0), Vec3::new(-445.0, 0.0, -400.0)]),
                Task::ExitInto("ENTR_LOST_WOODS_9", Step::ForestLeft),
                Task::Farewell(Step::FieldIntroOver),
                // West to the mouth of the hollow under the owl's perch; the owl's task goes on up
                // it (he takes Link within 360).
                Task::Walk(vec![Vec3::new(4440.0, -160.0, 8450.0)]),
                Task::Owl(Step::OwlFlown),
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
            choices: None,
            boulder_waits: Vec::new(),
            rupees: Vec::new(),
            tries: 0,
            last_rupees: 0,
            items_page: 0,
            reset: None,
            owl: None,
            crate_handle: None,
            cursor_seen: Vec::new(),
            carried: None,
            notes: 0,
            max_frames: route.max_frames(),
        }
    }

    /// `routes`' tasks one after another, as one run from where the first starts (the first
    /// route's name in messages).
    pub fn for_routes(routes: &[Route]) -> Playthrough {
        let mut run = Self::for_route(routes[0]);
        for &r in &routes[1..] {
            run.tasks.extend(Self::for_route(r).tasks);
            run.max_frames += r.max_frames();
        }
        run
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
            // The shop's door, round the sign in front of it (En_A_Obj 0x1E0A at (728, 0, -195)).
            Task::Walk(vec![Vec3::new(690.0, 0.0, -260.0), Vec3::new(854.0, 0.0, -250.0)]),
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

    /// The file `Task::Reset` wants loaded: the runner does the console's reset
    /// (`PlayState::console_reset`) before it runs this frame.
    pub fn take_reset(&mut self) -> Option<usize> {
        self.reset.take()
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
        self.crate_handle = None;
        self.cursor_seen.clear();
    }

    /// The pad for the next frame, or `None` when the run is over: done, or stuck (then
    /// `failure` says why).
    /// The run, with these answers in place of its talks' own (`Task::Converse`).
    pub fn with_choices(mut self, choices: Vec<u8>) -> Playthrough {
        self.choices = Some(choices);
        self
    }

    pub fn next(&mut self, w: &PlayState) -> Option<PadState> {
        if self.failure.is_some() || self.finished() {
            return None;
        }
        if self.frame >= self.max_frames {
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

    /// Start, when `KaleidoSetup_Update` would read it: no message box, no transition, no
    /// cutscene.
    fn press_start_to_open(&self, w: &PlayState) -> PadState {
        let ready = w.msg_ctx.msg_mode == oot_game::message::MSGMODE_NONE && w.transition.trigger == TRANS_TRIGGER_OFF && w.transition.mode == TRANS_MODE_OFF && !w.play_in_cs_mode();
        if ready { self.press(BTN_START) } else { PadState::default() }
    }

    /// `Task::MenuCursor`: the item page's cursor (`cursorX`, `cursorY` on the 6x4 grid) to
    /// `slot`, along its row first, then its column; each move a push of the stick and a release
    /// (`KaleidoScope_DrawPages`' repeat passes a first push at once).
    fn menu_cursor(&mut self, w: &PlayState, slot: u16, step: Step) -> Option<PadState> {
        use oot_game::kaleido::PAUSE_ITEM;
        let p = &w.pause_ctx;
        let i = PAUSE_ITEM as usize;
        let idle = PadState::default();
        if p.cursor_special_pos == 0 && p.cursor_point[i] as u16 == slot && menu_idle(w) {
            self.finish(Some(step));
            return None;
        }
        self.wait += 1;
        if self.wait > 200 {
            self.failure = Some(format!("the cursor never got to slot {slot} (at {}, special {})", p.cursor_point[i], p.cursor_special_pos));
            return None;
        }
        // A release between pushes.
        if self.prev.stick_x != 0 || self.prev.stick_y != 0 || !menu_idle(w) {
            return Some(idle);
        }
        let (sx, sy) = if p.cursor_special_pos == oot_game::kaleido::PAUSE_CURSOR_PAGE_LEFT {
            (80, 0)
        } else if p.cursor_special_pos == oot_game::kaleido::PAUSE_CURSOR_PAGE_RIGHT {
            (-80, 0)
        } else {
            // Along the row first, then the column; but where that would come back to a slot
            // already passed (empty slots are skipped, rows wrap), the shortest way of the
            // cursor's moves instead.
            let filled = |point: i16| w.save.inventory.items[point as usize] != oot_game::item::ITEM_NONE;
            let (tx, ty) = ((slot % 6) as i16, (slot / 6) as i16);
            let (x, y) = (p.cursor_x[i], p.cursor_y[i]);
            self.cursor_seen.push((x, y));
            let (d, stick) = if x != tx { ((if tx < x { -1 } else { 1 }, 0), (if tx < x { -80 } else { 80 }, 0)) } else { ((0, if ty < y { -1 } else { 1 }), (0, if ty < y { 80 } else { -80 })) };
            match item_cursor_move(&filled, (x, y), d) {
                // (None: onto the page's arrow, as before.)
                next if next.is_none_or(|n| !self.cursor_seen.contains(&n)) => stick,
                _ => match item_cursor_first_move(filled, (x, y), slot as i16) {
                    Some(m) => m,
                    None => {
                        self.failure = Some(format!("no way for the cursor from slot {} to slot {slot}", p.cursor_point[i]));
                        return None;
                    }
                },
            }
        };
        Some(PadState { button: 0, stick_x: sx, stick_y: sy })
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
                // A through a text that waits meanwhile: on a new save, Saria's greeting
                // (gKokiriForestSariaGreetingCs) plays on the porch as Kokiri Forest's room 0 loads.
                _ if Self::text_waits(w) => Some(self.press(BTN_A)),
                _ => self.settle(w, step),
            },
            Task::Walk(points) => self.walk(w, &points, RUN),
            Task::WalkUntil(points, until, step) => {
                self.wait += 1;
                if self.wait > 3000 {
                    self.failure = Some(format!("walked {points:?} without the condition holding"));
                    return None;
                }
                if w.message_state() == TEXT_STATE_NONE && until(w) {
                    self.finish(Some(step));
                    return None;
                }
                if w.message_state() != TEXT_STATE_NONE {
                    return Some(if Self::text_waits(w) { self.press(BTN_A) } else { idle });
                }
                // The points in turn (`walk`'s), the last one held once passed.
                let link = w.player().actor.world_pos;
                while self.sub + 1 < points.len() && Self::xz_dist(link, points[self.sub]) < WAYPOINT_RADIUS {
                    self.sub += 1;
                }
                Some(stick_towards(w, points[self.sub], RUN))
            }
            Task::Lift(near, step) => self.lift(w, near, step),
            Task::Pedestal(step) => self.pedestal(w, step),
            Task::Mount(step) => self.mount(w, step),
            Task::Ride(points, spur, step) => self.ride(w, &points, spur, step),
            Task::RideExit(points, name, step) => {
                self.wait += 1;
                if self.wait > 3000 {
                    self.failure = Some(format!("never rode into the exit to {name}"));
                    return None;
                }
                if w.transition.trigger != TRANS_TRIGGER_OFF {
                    self.finish(Some(step));
                    return None;
                }
                let Some((_, c)) = exit_to(w, name) else {
                    self.failure = Some(format!("no exit to {name} in scene {}", w.scene_id));
                    return None;
                };
                let Some(horse) = Self::ridden(w) else {
                    self.failure = Some("not on a horse".into());
                    return None;
                };
                while self.sub < points.len() && Self::xz_dist(horse.actor.world_pos, points[self.sub]) < RIDE_RADIUS {
                    self.sub += 1;
                }
                Some(Self::ride_stick(horse, points.get(self.sub).copied().unwrap_or(c)))
            }
            Task::RideIn(step) => match self.sub {
                0 => {
                    self.scene_changes = w.scene_changes;
                    self.sub = 1;
                    Some(idle)
                }
                1 => {
                    if w.scene_changes > self.scene_changes {
                        self.sub = 2;
                        self.wait = 0;
                    }
                    Some(idle)
                }
                _ => {
                    self.wait += 1;
                    if self.wait > 900 {
                        self.failure = Some(format!("never in the scene on the horse (Link {:?})", w.player().action));
                        return None;
                    }
                    let fade_over = w.transition.trigger == TRANS_TRIGGER_OFF && w.transition.mode == TRANS_MODE_OFF;
                    if fade_over && Self::ridden(w).is_some_and(|h| h.actor.speed_xz == 0.0 && h.check_4()) {
                        self.finish(Some(step));
                        return None;
                    }
                    Some(idle)
                }
            },
            Task::RideUntil(points, until, step) => {
                self.wait += 1;
                if self.wait > 3000 {
                    self.failure = Some("the ride's condition never held".into());
                    return None;
                }
                if until(w) {
                    self.finish(Some(step));
                    return None;
                }
                let Some(horse) = Self::ridden(w) else {
                    self.failure = Some(format!("not on a horse (Link {:?})", w.player().action));
                    return None;
                };
                while self.sub + 1 < points.len() && Self::xz_dist(horse.actor.world_pos, points[self.sub]) < RIDE_RADIUS {
                    self.sub += 1;
                }
                let to = points[self.sub.min(points.len() - 1)];
                let mut pad = Self::ride_stick(horse, to);
                let lined_up = (Self::ride_angle(horse, to) as i32).abs() < 0x800;
                // EnHorse_CheckBoost spurs a walk, a trot or a gallop.
                let gait = matches!(horse.action, crate::en_horse::ENHORSE_ACT_MOUNTED_WALK | crate::en_horse::ENHORSE_ACT_MOUNTED_TROT | crate::en_horse::ENHORSE_ACT_MOUNTED_GALLOP);
                if lined_up && gait && horse.num_boosts > 0 && horse.state_flags & crate::en_horse::ENHORSE_BOOST == 0 && self.prev.button & BTN_A == 0 {
                    pad.button |= BTN_A;
                }
                Some(pad)
            }
            Task::RideTalk(who, to, choices, until, step) => {
                if self.sub >= 3 {
                    return self.converse(w, who, to, &choices, until, step);
                }
                self.wait += 1;
                if self.wait > 3000 {
                    self.failure = Some(format!("never talked to {who:?} from horseback (sub {})", self.sub));
                    return None;
                }
                let Some(horse) = Self::ridden(w) else {
                    self.failure = Some(format!("not on a horse (Link {:?})", w.player().action));
                    return None;
                };
                if w.message_state() != TEXT_STATE_NONE || w.player().action == PA::Talk {
                    self.sub = 3;
                    self.notes = 0;
                    return Some(idle);
                }
                let Some(h) = Self::find(w, who) else {
                    self.failure = Some(format!("no {who:?} to talk to"));
                    return None;
                };
                let at = w.actors.actor(h).unwrap().world_pos;
                let walk = |mut pad: PadState| {
                    // At a walk (`EnHorse_MountedWalk` under the trot's stick): she stops at once.
                    pad.stick_x = (pad.stick_x as f32 * RIDE_WALK) as i8;
                    pad.stick_y = (pad.stick_y as f32 * RIDE_WALK) as i8;
                    pad
                };
                let offered = w.player().target_actor == Some(h);
                match self.sub {
                    0 => {
                        if Self::xz_dist(horse.actor.world_pos, to) < 40.0 || offered {
                            self.sub = 1;
                            return Some(idle);
                        }
                        Some(walk(Self::ride_stick(horse, to)))
                    }
                    1 => {
                        if horse.actor.speed_xz == 0.0 && horse.check_4() {
                            self.sub = 2;
                        }
                        Some(idle)
                    }
                    _ => {
                        // Turned to him (in place: the stick to the side), then A.
                        if (Self::ride_angle(horse, at) as i32).abs() > 0x800 && horse.actor.speed_xz == 0.0 {
                            let mut pad = Self::ride_stick(horse, at);
                            if (Self::ride_angle(horse, at) as i32).abs() <= RIDE_TURN {
                                pad.stick_y = 0;
                                pad.stick_x = if Self::ride_angle(horse, at) > 0 { -FULL as i8 } else { FULL as i8 };
                            }
                            return Some(pad);
                        }
                        if horse.actor.speed_xz != 0.0 {
                            return Some(idle);
                        }
                        // A only on his offer (Player's `targetActor`): A standing is "Down".
                        if !offered {
                            return Some(walk(Self::ride_stick(horse, at)));
                        }
                        Some(self.press(BTN_A))
                    }
                }
            }
            Task::Dismount(step) => {
                self.wait += 1;
                if self.wait > 600 {
                    self.failure = Some(format!("never off the horse (Link {:?})", w.player().action));
                    return None;
                }
                let p = w.player();
                if p.state1 & crate::player::STATE1_23 == 0 && p.action == PA::StandingStill {
                    self.finish(Some(step));
                    return None;
                }
                if p.action == PA::Ride && p.state2 & crate::player::STATE2_22 != 0 {
                    return Some(self.press(BTN_A));
                }
                Some(idle)
            }
            Task::LiftWho(who, step) => {
                if self.carried.is_none() {
                    self.carried = Self::find(w, who);
                }
                let at = self.carried.and_then(|h| w.actors.actor(h)).map(|a| a.world_pos).unwrap_or_default();
                self.lift(w, at, step)
            }
            Task::LiftBombFlower(near, step) => self.lift_bomb_flower(w, near, step),
            Task::PutDown(points, face, step) => self.put_down(w, &points, face, step),
            Task::TakeBomb(c, step) => self.take_bomb(w, c, step),
            Task::UseItem(c, step) => self.use_item(w, c, step),
            Task::CatchInBottle(c, step) => self.catch_in_bottle(w, c, step),
            Task::ThrowAtRun(points, release, step) => self.throw_at_run(w, &points, release, step),
            Task::Throw(at, step) => self.throw(w, at, step),
            Task::Glide(points, step) => self.glide(w, &points, step),
            Task::WaitGate(step) => {
                self.wait += 1;
                if self.wait > 600 {
                    self.failure = Some("Kakariko's gate never opened".into());
                    return None;
                }
                // Slid open to x -89 (`func_80878300`), the guard's INFTABLE_76 set.
                let open = w.save.get_inf_table(crate::bg_gate_shutter::INFTABLE_76)
                    && w.actors.all().into_iter().any(|h| w.actors.downcast::<crate::bg_gate_shutter::BgGateShutter>(h).is_some_and(|g| g.actor.world_pos.x <= -89.0));
                if open && w.message_state() == TEXT_STATE_NONE {
                    self.finish(Some(step));
                    return None;
                }
                Some(if Self::text_waits(w) { self.press(BTN_A) } else { PadState::default() })
            }
            Task::ClimbTo(top, step) => self.climb_to(w, top, step),
            Task::ShowItem(who, from, button, step) => self.show_item(w, who, from, button, step),
            Task::Ocarina(button, song, step) => self.play_song(w, button, song, step, true),
            Task::OcarinaHeard(button, song, step) => self.play_song(w, button, song, step, false),
            Task::OcarinaNotes(button, ready, tune, step) => self.ocarina_notes(w, button, ready, &tune, step),
            Task::TalkThrough(choices, step) => self.talk_through(w, &choices, step),
            Task::WaitDawn(step) => {
                self.wait += 1;
                if self.wait > 1500 {
                    self.failure = Some("the Sun's Song's time never ended".into());
                    return None;
                }
                if w.save.suns_song_state == 0 && w.save.is_day() {
                    self.finish(Some(step));
                    return None;
                }
                Some(idle)
            }
            Task::WaitFairy(near, step) => {
                self.wait += 1;
                if self.wait > 600 {
                    self.failure = Some("no fairy came".into());
                    return None;
                }
                let fairy = w.actors.all().into_iter().any(|h| {
                    w.actors.downcast::<crate::en_elf::EnElf>(h).is_some_and(|e| !e.actor.killed && e.actor.params == crate::en_elf::FAIRY_HEAL_BIG && Self::xz_dist(e.actor.world_pos, near) < 200.0)
                });
                if fairy {
                    self.finish(Some(step));
                    return None;
                }
                Some(idle)
            }
            Task::PushCrate(near, offset, dir, until, step) => self.push_crate(w, near, offset, dir, until, step),
            Task::Caught(to, step) => {
                self.wait += 1;
                if self.wait > 1500 {
                    self.failure = Some("never caught".into());
                    return None;
                }
                match self.sub {
                    0 => {
                        if w.msg_ctx.text_id == 0x702D && w.message_state() != TEXT_STATE_NONE {
                            self.mark(Step::Caught);
                            self.sub = 1;
                            return Some(idle);
                        }
                        Some(stick_towards(w, to, RUN))
                    }
                    1 => {
                        if w.transition.trigger != TRANS_TRIGGER_OFF {
                            self.scene_changes = w.scene_changes;
                            self.sub = 2;
                            return Some(idle);
                        }
                        let event = w.message_state() == oot_game::message::TEXT_STATE_EVENT;
                        Some(if Self::text_waits(w) || event { self.press(BTN_A) } else { idle })
                    }
                    2 => {
                        if w.scene_changes > self.scene_changes {
                            self.sub = 3;
                        }
                        Some(idle)
                    }
                    _ => self.settle(w, Some(step)),
                }
            }
            Task::WaitGone(who, step) => {
                self.wait += 1;
                if self.wait > 1500 {
                    self.failure = Some(format!("{who:?} never gone"));
                    return None;
                }
                if Self::find(w, who).is_some() || w.message_state() != TEXT_STATE_NONE {
                    return Some(PadState::default());
                }
                self.sub = 0;
                self.settle(w, Some(step))
            }
            Task::Hurry(points) => self.walk(w, &points, FULL),
            Task::DiveUnder(to, surface, step) => self.dive_under(w, to, surface, step),
            Task::DiveFor(to, until, step) => {
                self.wait += 1;
                if self.wait > 600 {
                    self.failure = Some(format!("the dive for {to} never got it (at {})", w.player().actor.world_pos));
                    return None;
                }
                if until(w) {
                    self.finish(Some(step));
                    return None;
                }
                let mut pad = stick_towards(w, to, FULL);
                // Pressed until the dive starts (a press at the surface), then held under it (the hold
                // keeps it going down): held once Link is in the dive or near the point's depth.
                let link = w.player().actor.world_pos;
                pad.button = if w.player().action == crate::player::Action::Dive || link.y < to.y + 40.0 { BTN_A } else { self.press(BTN_A).button };
                Some(pad)
            }
            Task::Sneak(points) => self.sneak(w, &points),
            Task::WaitGuards(ready, what) => {
                self.wait += 1;
                if self.wait > 3000 {
                    self.failure = Some(format!("the guards never came round: {what}"));
                    return None;
                }
                if w.msg_ctx.text_id == 0x702D && w.message_state() != TEXT_STATE_NONE {
                    let at = w.player().actor.world_pos;
                    self.failure = Some(format!("caught by a courtyard guard waiting at ({:.0}, {:.0}, {:.0})", at.x, at.y, at.z));
                    return None;
                }
                if ready(w) {
                    self.finish(None);
                    return None;
                }
                Some(PadState::default())
            }
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
            Task::PickNearest(step) => self.pick_nearest(w, step),
            Task::SlashSwitch(from, at) => self.slash_switch(w, from, at),
            Task::BuyShield => self.buy_shield(w),
            Task::Equip(step) => {
                // Open the menu, then close it: its resume equips (the equipment page's stand-in).
                if self.sub == 0 && !w.save.clone().equip_owned_unworn() {
                    self.finish(Some(step));
                    return None;
                }
                self.wait += 1;
                if self.wait > 300 {
                    self.failure = Some(format!("the pause menu didn't equip what's owned (equipment {:#06x}, worn {:#06x}, pause state {})", w.save.inventory.equipment, w.save.equips.equipment, w.pause_ctx.state));
                    return None;
                }
                match self.sub {
                    0 => {
                        if w.pause_ctx.state != oot_game::kaleido::PAUSE_STATE_OFF {
                            self.sub = 1;
                            return Some(idle);
                        }
                        Some(self.press_start_to_open(w))
                    }
                    1 => {
                        if menu_idle(w) {
                            self.sub = 2;
                        }
                        Some(idle)
                    }
                    2 => {
                        if w.pause_ctx.state == oot_game::kaleido::PAUSE_STATE_CLOSING {
                            self.sub = 3;
                            return Some(idle);
                        }
                        Some(self.press(BTN_START))
                    }
                    _ => {
                        if w.pause_ctx.state == oot_game::kaleido::PAUSE_STATE_OFF {
                            self.sub = 0;
                        }
                        Some(idle)
                    }
                }
            }
            Task::OpenMenu(step) => {
                if menu_idle(w) {
                    self.finish(Some(step));
                    return None;
                }
                self.wait += 1;
                if self.wait > 200 {
                    self.failure = Some(format!("the pause menu didn't open (pause state {})", w.pause_ctx.state));
                    return None;
                }
                Some(if w.pause_ctx.state == oot_game::kaleido::PAUSE_STATE_OFF { self.press_start_to_open(w) } else { idle })
            }
            Task::MenuCursor(slot, step) => self.menu_cursor(w, slot, step),
            Task::EquipOnC(button, step) => {
                use oot_game::kaleido::{PAUSE_ITEM, PAUSE_MAIN_STATE_3};
                let p = &w.pause_ctx;
                let item = p.cursor_item[PAUSE_ITEM as usize];
                // sub 0: pressed; 1: the icon in flight; done when it's on the button.
                if self.sub == 1 && menu_idle(w) {
                    if w.save.equips.button_items[button + 1] as u16 == item {
                        self.finish(Some(step));
                        return None;
                    }
                    self.failure = Some(format!("C button {button} has {:#x}, not {item:#x}", w.save.equips.button_items[button + 1]));
                    return None;
                }
                self.wait += 1;
                if self.wait > 120 {
                    self.failure = Some(format!("the equip never finished (main state {})", p.main_state));
                    return None;
                }
                if p.main_state == PAUSE_MAIN_STATE_3 {
                    self.sub = 1;
                }
                let b = [eng_input::pad::BTN_CLEFT, eng_input::pad::BTN_CDOWN, eng_input::pad::BTN_CRIGHT][button];
                Some(if self.sub == 0 { self.press(b) } else { idle })
            }
            Task::TurnPageRight(step) => {
                let p = &w.pause_ctx;
                if self.sub == 0 {
                    self.sub = 1;
                    self.items_page = p.page_index;
                    return Some(self.press(eng_input::pad::BTN_R));
                }
                if menu_idle(w) && p.page_index != self.items_page {
                    self.finish(Some(step));
                    return None;
                }
                self.wait += 1;
                if self.wait > 60 {
                    self.failure = Some(format!("the page never turned (page {}, main state {})", p.page_index, p.main_state));
                    return None;
                }
                Some(idle)
            }
            Task::MapFloorUp(step) => {
                let p = &w.pause_ctx;
                // sub 0: off the page's left arrow (where R's turn leaves the cursor) onto the
                // floors' column, the stick right; then the floor viewed noted, the stick up;
                // 1: released, until the floor changes.
                if self.sub == 0 && p.cursor_special_pos != 0 {
                    self.wait += 1;
                    if self.wait > 60 {
                        self.failure = Some(format!("the cursor never left the arrow ({})", p.cursor_special_pos));
                        return None;
                    }
                    return Some(if self.prev.stick_x != 0 || !menu_idle(w) { idle } else { PadState { button: 0, stick_x: 80, stick_y: 0 } });
                }
                if self.sub == 0 {
                    if self.prev.stick_x != 0 {
                        return Some(idle);
                    }
                    self.sub = 1;
                    self.wait = 0;
                    self.items_page = p.dungeon_map_slot as u16;
                    return Some(PadState { button: 0, stick_x: 0, stick_y: 80 });
                }
                if menu_idle(w) && p.dungeon_map_slot as u16 != self.items_page {
                    self.finish(Some(step));
                    return None;
                }
                self.wait += 1;
                if self.wait > 60 {
                    self.failure = Some(format!("the map's floor never changed (slot {}, cursor {})", p.dungeon_map_slot, p.cursor_point[oot_game::kaleido::PAUSE_MAP as usize]));
                    return None;
                }
                Some(idle)
            }
            Task::WaitDeath(step) => {
                if w.game_over_ctx.state != oot_game::game_over::GAMEOVER_INACTIVE {
                    self.finish(Some(step));
                    return None;
                }
                self.wait += 1;
                if self.wait > 300 {
                    self.failure = Some(format!("Link never died (health {})", w.save.health));
                    return None;
                }
                Some(idle)
            }
            Task::WaitPauseState(state, step) => {
                if w.pause_ctx.state == state {
                    self.finish(Some(step));
                    return None;
                }
                self.wait += 1;
                if self.wait > 600 {
                    self.failure = Some(format!("the pause menu never got to state {state} (state {})", w.pause_ctx.state));
                    return None;
                }
                Some(idle)
            }
            Task::PromptChoice(choice, step) => {
                use oot_game::kaleido::{PAUSE_STATE_GAME_OVER_CONTINUE_PROMPT, PAUSE_STATE_GAME_OVER_SAVE_PROMPT};
                let p = &w.pause_ctx;
                // sub 0: at the prompt (the state noted); 1: A pressed, until the respawn's done
                // or the state has moved on.
                if self.sub == 0 {
                    self.items_page = p.state;
                    self.sub = 1;
                }
                let at_prompt = p.state == self.items_page && (p.state == PAUSE_STATE_GAME_OVER_SAVE_PROMPT || p.state == PAUSE_STATE_GAME_OVER_CONTINUE_PROMPT);
                self.wait += 1;
                if self.wait > 400 {
                    self.failure = Some(format!("the prompt never moved on (state {}, choice {})", p.state, p.prompt_choice));
                    return None;
                }
                if self.items_page == PAUSE_STATE_GAME_OVER_CONTINUE_PROMPT && !at_prompt {
                    // Continued: the fade, the respawn, then Link standing.
                    return self.settle(w, Some(step));
                }
                if !at_prompt {
                    self.finish(Some(step));
                    return None;
                }
                // The stick to the choice (a push, then a release), then A.
                if p.prompt_choice != choice {
                    if self.prev.stick_x != 0 {
                        return Some(idle);
                    }
                    return Some(PadState { button: 0, stick_x: if choice == 0 { -80 } else { 80 }, stick_y: 0 });
                }
                Some(self.press(eng_input::pad::BTN_A))
            }
            Task::CloseMenu(step) => {
                use oot_game::kaleido::PAUSE_STATE_OFF;
                if self.sub == 1 && w.pause_ctx.state == PAUSE_STATE_OFF {
                    self.finish(Some(step));
                    return None;
                }
                self.wait += 1;
                if self.wait > 120 {
                    self.failure = Some(format!("the pause menu never closed (pause state {})", w.pause_ctx.state));
                    return None;
                }
                if self.sub == 0 {
                    self.sub = 1;
                    return Some(self.press(BTN_START));
                }
                Some(idle)
            }
            Task::OpenSavePrompt(step) => {
                use oot_game::kaleido::{PAUSE_SAVE_PROMPT_STATE_WAIT_CHOICE, PAUSE_STATE_SAVE_PROMPT};
                let p = &w.pause_ctx;
                if p.state == PAUSE_STATE_SAVE_PROMPT && p.save_prompt_state == PAUSE_SAVE_PROMPT_STATE_WAIT_CHOICE {
                    self.finish(Some(step));
                    return None;
                }
                self.wait += 1;
                if self.wait > 60 {
                    self.failure = Some(format!("the save prompt never turned in (pause state {}, prompt state {})", p.state, p.save_prompt_state));
                    return None;
                }
                Some(if menu_idle(w) { self.press(eng_input::pad::BTN_B) } else { idle })
            }
            Task::SaveYes(step) => {
                use oot_game::kaleido::{PAUSE_SAVE_PROMPT_STATE_SAVED, PAUSE_SAVE_PROMPT_STATE_WAIT_CHOICE};
                let p = &w.pause_ctx;
                if p.save_prompt_state == PAUSE_SAVE_PROMPT_STATE_SAVED {
                    self.finish(Some(step));
                    return None;
                }
                self.wait += 1;
                if self.wait > 60 || p.save_prompt_state != PAUSE_SAVE_PROMPT_STATE_WAIT_CHOICE {
                    self.failure = Some(format!("the save prompt never saved (prompt state {}, choice {})", p.save_prompt_state, p.prompt_choice));
                    return None;
                }
                // The stick to Yes (a push, then a release), then A.
                if p.prompt_choice != 0 {
                    return Some(if self.prev.stick_x != 0 { idle } else { PadState { button: 0, stick_x: -80, stick_y: 0 } });
                }
                Some(self.press(eng_input::pad::BTN_A))
            }
            Task::Reset(file, step) => {
                if self.sub == 0 {
                    self.sub = 1;
                    self.reset = Some(file);
                    return Some(idle);
                }
                self.settle(w, Some(step))
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
            Task::Converse(who, from, choices, until, step) => {
                let choices = self.choices.clone().unwrap_or(choices);
                self.converse(w, who, from, &choices, until, step)
            }
            Task::Texts(..) | Task::TextsFor(..) => {
                self.wait += 1;
                if self.wait > 20000 {
                    self.failure = Some(format!("the texts never ended (text {:#x})", w.msg_ctx.text_id));
                    return None;
                }
                let (done, step) = match &task {
                    Task::Texts(until, step) => (until(w), *step),
                    Task::TextsFor(cond, step) => (cond.holds(w), *step),
                    _ => unreachable!(),
                };
                if w.message_state() == TEXT_STATE_NONE && done {
                    self.finish(Some(step));
                    return None;
                }
                let event = w.message_state() == oot_game::message::TEXT_STATE_EVENT;
                Some(if Self::text_waits(w) || event { self.press(BTN_A) } else { idle })
            }
            Task::Playback(song, wrong_first, step) => self.playback(w, song, wrong_first, step),
            Task::Talk(who, from, step) => {
                let Some(h) = Self::find(w, who) else {
                    // Gone as the talk ended (Mido with the emerald: EnMd_Walk's Actor_Kill as he
                    // closes the box).
                    if self.sub >= 3 && w.message_state() == TEXT_STATE_NONE {
                        self.finish(Some(step));
                        return None;
                    }
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
                    // A whenever a box waits (an event text too: the actor's `Message_ShouldAdvance`),
                    // until it's closed and Link stands.
                    _ => {
                        if w.message_state() == TEXT_STATE_NONE && !matches!(p.action, PA::Talk | PA::ItemPutAway) {
                            self.finish(Some(step));
                            return None;
                        }
                        let event = w.message_state() == oot_game::message::TEXT_STATE_EVENT;
                        Some(if Self::text_waits(w) || event { self.press(BTN_A) } else { idle })
                    }
                }
            }
            Task::CutBushes(bushes) => self.cut_bushes(w, &bushes),
            Task::FightBaba(home) => self.fight_baba(w, home),
            Task::BlockKeese(home) => self.block_keese(w, home),
            Task::KillKeese(home) => self.kill_keese(w, home),
            Task::ChaseScrub(home) => {
                if self.sub == 0
                    && let Some((_, n)) = Self::mad_scrub_at(w, home)
                    && n.actor.world_pos.distance(w.player().actor.world_pos) > 150.0
                {
                    self.wait += 1;
                    if self.wait > 2400 {
                        self.failure = Some("the Mad Scrub's chase never closed in".into());
                        return None;
                    }
                    return Some(stick_towards(w, n.actor.world_pos, RUN));
                }
                self.catch_scrub(w, home)
            }
            Task::FightWolfos(home, flag, step) => self.fight_wolfos(w, home, flag, step),
            Task::FightKarebaba(home) => self.fight_karebaba(w, home),
            Task::BounceNut(home) => self.bounce_nut(w, home),
            Task::CatchScrub(home) => self.catch_scrub(w, home),
            Task::PressSwitch(home, flag, step) => self.press_switch(w, home, flag, step),
            Task::WaitWebBurnt(home, step) => self.wait_web_burnt(w, home, step),
            Task::OpenSlidingDoor(index, front, step) => self.open_sliding_door(w, index, front, step),
            Task::CatchSuperCucco(step) => {
                self.wait += 1;
                if self.wait > 1200 {
                    self.failure = Some(format!("no super cucco caught (Link {:?}, sub {})", w.player().action, self.sub));
                    return None;
                }
                let p = w.player();
                let event = w.message_state() == oot_game::message::TEXT_STATE_EVENT;
                match self.sub {
                    0 => {
                        let link = p.actor.world_pos;
                        let target = w
                            .actors
                            .all()
                            .into_iter()
                            .filter(|&h| w.actors.downcast::<crate::en_niw::EnNiw>(h).is_some_and(|c| !c.actor.killed && crate::en_niw::TYPE.get(c.actor.params) == 0xD && c.actor.gravity <= 0.0))
                            .min_by(|&a, &b| {
                                let d = |h| w.actors.actor(h).map_or(f32::MAX, |c| c.world_pos.distance(link));
                                d(a).total_cmp(&d(b))
                            });
                        let Some(h) = target else {
                            self.failure = Some("no super cucco left".into());
                            return None;
                        };
                        if p.interact_range_actor == Some(h) && p.state1 & crate::player::STATE1_11 == 0 {
                            self.sub = 1;
                            self.carried = Some(h);
                            return Some(self.press(BTN_A));
                        }
                        let c = w.actors.actor(h).unwrap().world_pos;
                        let mag = if Self::xz_dist(link, c) > 80.0 { RUN } else { SLOW };
                        Some(stick_towards(w, c, mag))
                    }
                    1 => {
                        if w.message_state() != TEXT_STATE_NONE {
                            self.sub = 2;
                            return Some(idle);
                        }
                        // The A went elsewhere, or the cucco got away: after another.
                        if p.state1 & crate::player::STATE1_11 == 0 && p.held_actor.is_none() {
                            self.sub = 0;
                            self.carried = None;
                        }
                        Some(idle)
                    }
                    _ => {
                        if w.transition.trigger != TRANS_TRIGGER_OFF || (w.message_state() == TEXT_STATE_NONE && p.held_actor.is_none()) {
                            self.carried = None;
                            self.finish(Some(step));
                            return None;
                        }
                        Some(if Self::text_waits(w) || event { self.press(BTN_A) } else { idle })
                    }
                }
            }
            Task::OpenDoor(front, door, step) => {
                self.wait += 1;
                if self.wait > 900 {
                    self.failure = Some(format!("the door at {door:?} never opened (sub {})", self.sub));
                    return None;
                }
                let p = w.player();
                if w.transition.trigger != TRANS_TRIGGER_OFF {
                    self.finish(step);
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
                        if p.door_type == crate::player::PLAYER_DOORTYPE_HANDLE {
                            return Some(self.press(BTN_A));
                        }
                        Some(stick_towards(w, door, 30.0))
                    }
                }
            }
            Task::WaitDoorShut(index, step) => self.wait_door_shut(w, index, step),
            Task::WaitCameras => self.wait_cameras(w),
            Task::TakeStick(step) => self.take_stick(w, step),
            Task::LightStick(home, step) => self.light_stick(w, home, step),
            Task::BurnWeb(home, step) => self.burn_web(w, home, step),
            Task::GrabBlock(home, step) => self.grab_block(w, home, step),
            Task::PullStone(home, step) => self.pull_stone(w, home, step),
            Task::RunOntoPillar(home, step) => {
                self.wait += 1;
                if self.wait > 300 {
                    self.failure = Some(format!("never onto the pillar at {home} (Link {:?} at {:?})", w.player().action, w.player().actor.world_pos));
                    return None;
                }
                let Some(pillar) = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<crate::bg_ddan_jd::BgDdanJd>(h).filter(|p| p.actor.home_pos.distance(home) < 1.0)) else {
                    self.failure = Some(format!("no pillar at {home}"));
                    return None;
                };
                let p = w.player();
                if p.grounded() && p.actor.floor_bg_id == pillar.bg {
                    return self.settle(w, Some(step));
                }
                Some(stick_towards(w, pillar.actor.world_pos, FULL))
            }
            Task::PushBlock(home, flag, step) => self.push_block(w, home, flag, step),
            Task::ClimbBlock(step) => self.climb_block(w, step),
            Task::DrawSlingshot(step) => self.draw_slingshot(w, step),
            Task::Shoot(target, flag, step) => self.shoot(w, target, |w| w.flags.get_switch(flag), step),
            Task::ShootUntil(target, until, step) => self.shoot(w, target, until, step),
            Task::MemoryGame(step) => self.memory_game(w, step),
            Task::OcarinaOut(button, step) => {
                use eng_input::pad::{BTN_CDOWN, BTN_CLEFT, BTN_CRIGHT};
                self.wait += 1;
                if self.wait > 600 {
                    self.failure = Some(format!("no text with the ocarina out (Link {:?})", w.player().action));
                    return None;
                }
                if w.message_state() != TEXT_STATE_NONE {
                    self.finish(Some(step));
                    return None;
                }
                Some(if w.player().action == PA::Ocarina { idle } else { self.press([BTN_CLEFT, BTN_CDOWN, BTN_CRIGHT][button]) })
            }
            Task::LeaveFirstPerson => {
                if w.player().unk_6AD == 0 {
                    self.finish(None);
                    return None;
                }
                Some(self.press(BTN_A))
            }
            Task::GohmaIntro => self.gohma_intro(w),
            Task::GohmaLook(step) => self.gohma_look(w, step),
            Task::GohmaWaitBattle(step) => self.gohma_wait_battle(w, step),
            Task::GohmaFight(step) => self.gohma_fight(w, step),
            Task::GohmaWaitGone(step) => self.gohma_wait_gone(w, step),
            Task::TakeHeart(step) => self.take_heart(w, step),
            Task::IntoWarp(step) => self.into_warp(w, step),
            Task::IntoRutoWarp(step) => self.into_ruto_warp(w, step),
            Task::WarpOut(step) => {
                if self.sub == 0 {
                    self.scene_changes = w.scene_changes;
                    self.sub = 1;
                }
                if w.scene_changes > self.scene_changes {
                    self.finish(Some(step));
                    return None;
                }
                self.wait += 1;
                if self.wait > 600 {
                    self.failure = Some("the scene never changed (the blue warp's or a cutscene's way out)".into());
                    return None;
                }
                Some(idle)
            }
            Task::EmeraldPart1(step) => self.warp_cutscene(w, 2400, step),
            Task::WarpCutscene(limit, step) => self.warp_cutscene(w, limit, step),
            Task::Emerald(step) => self.emerald(w, step),
            Task::Chain(step) => self.chain(w, step),
            Task::ExitInto(name, step) => {
                if w.transition.trigger != TRANS_TRIGGER_OFF {
                    self.finish(Some(step));
                    return None;
                }
                let Some((_, c)) = exit_to(w, name) else {
                    self.failure = Some(format!("no exit to {name} in scene {}", w.scene_id));
                    return None;
                };
                Some(stick_towards(w, c, RUN))
            }
            Task::Farewell(step) => self.farewell(w, step),
            Task::Owl(step) => self.owl(w, step),
            Task::FightPeahat(home, step) => self.fight_peahat(w, home, step),
            Task::FightStalchildren(count, step) => self.fight_stalchildren(w, count, step),
            Task::WaitFor(cond, step) => {
                if cond.holds(w) {
                    self.finish(Some(step));
                    return None;
                }
                Some(idle)
            }
            Task::NewScene(step) => match self.sub {
                0 => {
                    self.scene_changes = w.scene_changes;
                    self.sub = 1;
                    Some(idle)
                }
                1 => {
                    if w.scene_changes > self.scene_changes {
                        self.sub = 2;
                        self.wait = 0;
                    } else {
                        self.wait += 1;
                        if self.wait > 600 {
                            self.failure = Some("the scene never changed".into());
                            return None;
                        }
                    }
                    Some(idle)
                }
                _ => self.settle(w, Some(step)),
            },
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
    fn fight_wolfos(&mut self, w: &PlayState, home: Vec3, flag: i32, step: Step) -> Option<PadState> {
        use crate::en_wf::{EnWf, Func};
        use eng_input::pad::{BTN_B, BTN_R, BTN_Z};
        self.wait += 1;
        if self.wait > 6000 {
            self.failure = Some("the Wolfos fight stalled".into());
            return None;
        }
        let wolfos = w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<EnWf>(h).map(|r| (h, r))).find(|(_, r)| r.actor.home_pos.distance(home) < 50.0);
        let Some((h, r)) = wolfos.filter(|(_, r)| !r.actor.killed) else {
            if w.flags.get_switch(flag) {
                self.finish(Some(step));
                return None;
            }
            return Some(PadState::default());
        };
        let pos = r.actor.world_pos;
        let dist = r.actor.xz_dist_to_player;
        let open = match r.func {
            Func::AttackBouncedOff | Func::Damaged | Func::Stunned => true,
            Func::Attack => r.skel.cur_frame >= 20.0,
            _ => false,
        };
        let p = w.player();
        let locked = p.focus_actor == Some(h);
        let mut pad = PadState::default();
        if r.func == Func::WaitAppear {
            pad = stick_towards(w, pos, 80.0);
        } else if r.func == Func::Die {
            // Its howl, then its flag and gone.
        } else if !locked {
            // Turned to it (and at it), then Z (a new press) once it's in front.
            let off = eng_math::vec3f_yaw(p.actor.world_pos, pos).wrapping_sub(p.actor.shape_rot.y);
            if (off as i32).abs() < 0x2000 && self.prev.button & BTN_Z == 0 {
                pad.button |= BTN_Z;
            } else {
                pad = stick_towards(w, pos, if dist > 75.0 { 60.0 } else { 20.0 });
            }
        } else {
            pad.button |= BTN_Z;
            if dist > 75.0 {
                let s = stick_towards(w, pos, 60.0);
                (pad.stick_x, pad.stick_y) = (s.stick_x, s.stick_y);
            }
            if open && dist < 90.0 {
                if self.prev.button & BTN_B == 0 {
                    pad.button |= BTN_B;
                }
            } else {
                pad.button |= BTN_R;
            }
        }
        Some(pad)
    }

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

    /// `Task::DrawSlingshot`: its C button held until the slingshot is drawn (`unk_836` set, the wait)
    /// with a seed in hand, in first person.
    fn draw_slingshot(&mut self, w: &PlayState, step: Step) -> Option<PadState> {
        let p = w.player();
        if p.held_actor.is_some() && p.unk_836 > 0 && p.action == crate::player::Action::FirstPerson {
            self.finish(Some(step));
            return None;
        }
        self.wait += 1;
        if self.wait > 120 {
            self.failure = Some(format!("the slingshot was never drawn (held {}, C buttons {:x?}, seeds {})", p.held_item_ap, &w.save.equips.button_items[1..4], w.save.ammo(oot_game::item::ITEM_SLINGSHOT)));
            return None;
        }
        Some(PadState { button: slingshot_button(w), ..Default::default() })
    }

    /// `Task::Shoot`: aiming in first person, C-Right held: the seed in hand's direction (its
    /// `world.rot`, from the left hand's matrix: what `Actor_SetProjectileSpeed` shoots it along)
    /// steered at `target` with the stick (`func_8084ABD8`: each frame `(1 - cos(stick * 200)) *
    /// 1500`; the stick's magnitude the largest whose step is within the error), then C-Right let
    /// go within 64 of it, and idle until the flag is set. A seed that misses: drawn again.
    fn shoot(&mut self, w: &PlayState, target: Vec3, done: impl Fn(&PlayState) -> bool, step: Step) -> Option<PadState> {
        let idle = PadState::default();
        if done(w) {
            self.finish(Some(step));
            return None;
        }
        self.wait += 1;
        if self.wait > 600 {
            self.failure = Some(format!("no seed hit the target at {target} ({step:?})"));
            return None;
        }
        let p = w.player();
        let held = PadState { button: slingshot_button(w), ..Default::default() };
        match self.sub {
            0 => {
                if p.held_actor.is_some() && p.unk_836 > 0 && p.unk_6AD == 2 {
                    self.sub = 1;
                }
                Some(held)
            }
            1 => {
                let Some(seed) = p.held_actor.and_then(|h| w.actors.actor(h)) else {
                    self.sub = 0;
                    return Some(held);
                };
                // Math_Vec3f_Yaw and Math_Vec3f_Pitch from the seed (rot.x is the pitch down).
                let d = target - seed.world_pos;
                let want_yaw = eng_math::vec3f_yaw(seed.world_pos, target);
                let want_pitch = eng_math::atan2_s((d.x * d.x + d.z * d.z).sqrt(), -d.y);
                let (ey, ep) = (want_yaw.wrapping_sub(seed.world_rot.y) as i32, want_pitch.wrapping_sub(seed.world_rot.x) as i32);
                if ey.abs() < 64 && ep.abs() < 64 {
                    self.sub = 2;
                    self.tries = 0;
                    return Some(idle);
                }
                // The largest stick (past the dead zone of 7) whose step is within the error.
                let stick = |err: i32| -> i8 {
                    let s = (1..=60).rev().find(|&s: &i32| ((1.0 - eng_math::cos_s((s * 200) as i16)) * 1500.0) as i32 <= err.abs() * 3 / 4).unwrap_or(0);
                    if s == 0 { 0 } else { ((s + 7) * err.signum()) as i8 }
                };
                // A negative stick x turns the focus's yaw up; a positive stick y pitches it down.
                Some(PadState { button: slingshot_button(w), stick_x: -stick(ey), stick_y: stick(ep) })
            }
            _ => {
                self.tries += 1;
                if self.tries > 40 {
                    self.sub = 0;
                }
                Some(idle)
            }
        }
    }

    /// `Task::MemoryGame`: `sub` 0 outside the Link's turn, 1 in it (`notes` counting his
    /// frames), 2 after a round's success until the next turn.
    fn memory_game(&mut self, w: &PlayState, step: Step) -> Option<PadState> {
        use eng_input::pad::{BTN_CDOWN, BTN_CLEFT, BTN_CRIGHT, BTN_CUP};
        use oot_game::audio::ocarina::{OCARINA_BTN_A, OCARINA_BTN_C_DOWN, OCARINA_BTN_C_LEFT, OCARINA_BTN_C_RIGHT, OCARINA_BTN_C_UP, OCARINA_SONG_MEMORY_GAME};
        use oot_game::message::{MSGMODE_MEMORY_GAME_PLAYER_PLAYING, MSGMODE_MEMORY_GAME_ROUND_SUCCESS, MSGMODE_MEMORY_GAME_START, MSGMODE_NONE};
        let idle = PadState::default();
        self.wait += 1;
        if self.wait > 12000 {
            self.failure = Some(format!("the memory game never ended (msgMode {:#x}, text {:#x}, round {})", w.msg_ctx.msg_mode, w.msg_ctx.text_id, w.save.ocarina_game_round_num));
            return None;
        }
        let mode = w.msg_ctx.msg_mode;
        let st = w.message_state();
        // ITEMGETINF_17 (save.h): the third round's prize.
        if w.save.get_item_get_inf(0x17) && st == TEXT_STATE_NONE && w.player().cs_mode == 0 && w.player().action != PA::Talk {
            self.finish(Some(step));
            return None;
        }
        if mode == MSGMODE_MEMORY_GAME_START {
            self.mark(Step::MemoryGame);
        }
        if mode == MSGMODE_MEMORY_GAME_ROUND_SUCCESS && self.sub != 2 {
            self.sub = 2;
            self.steps.push((Step::MemoryRound, self.frame));
        }
        if mode == MSGMODE_MEMORY_GAME_PLAYER_PLAYING {
            if self.sub != 1 {
                self.sub = 1;
                self.notes = 0;
            }
            let o = &w.audio.ocarina;
            let (note, phase) = (self.notes / 5, self.notes % 5);
            if note >= o.memory_game_append_pos as usize {
                return Some(idle);
            }
            self.notes += 1;
            let pitch = o.song_note(OCARINA_SONG_MEMORY_GAME, note as i32).pitch;
            let pad = match o.pitch_to_button_map.get(pitch as usize).copied() {
                Some(OCARINA_BTN_A) => BTN_A,
                Some(OCARINA_BTN_C_DOWN) => BTN_CDOWN,
                Some(OCARINA_BTN_C_RIGHT) => BTN_CRIGHT,
                Some(OCARINA_BTN_C_LEFT) => BTN_CLEFT,
                Some(OCARINA_BTN_C_UP) => BTN_CUP,
                _ => 0,
            };
            return Some(if phase < 3 { PadState { button: pad, ..Default::default() } } else { idle });
        }
        // The ocarina out somewhere else (free play, not the game's): B puts it away.
        if mode == oot_game::message::MSGMODE_OCARINA_PLAYING && w.msg_ctx.ocarina_action == oot_game::message::OCARINA_ACTION_FREE_PLAY {
            self.tries = 0;
            return Some(self.press(BTN_B));
        }
        if st == TEXT_STATE_CHOICE || Self::text_waits(w) || st == oot_game::message::TEXT_STATE_EVENT {
            // (A on a choice: its first answer, yes.)
            self.tries = 0;
            return Some(self.press(BTN_A));
        }
        // Nothing running for 20 frames (a prize is offered the frame after its text): the
        // ocarina out (C-Down), again every 30 frames.
        let p = w.player();
        if mode == MSGMODE_NONE && st == TEXT_STATE_NONE && p.cs_mode == 0 && matches!(p.action, PA::StandingStill) {
            self.tries += 1;
            if self.tries % 30 == 20 {
                return Some(PadState { button: BTN_CDOWN, ..Default::default() });
            }
        } else {
            self.tries = 0;
        }
        Some(idle)
    }

    /// Queen Gohma, if she's in the room.
    fn gohma(w: &PlayState) -> Option<&crate::boss_goma::BossGoma> {
        w.actors.all().into_iter().find_map(|h| w.actors.downcast::<crate::boss_goma::BossGoma>(h).filter(|g| !g.actor.killed))
    }

    /// `step` noted as done on this frame, once, without ending the task.
    fn mark(&mut self, step: Step) {
        if !self.steps.iter().any(|(s, _)| *s == step) {
            self.steps.push((step, self.frame));
            self.done = Some(step);
        }
    }

    /// The slingshot drawn (C-Right held) and aimed in first person at `target`, as `Task::Shoot`
    /// aims: the pad, and whether the seed in hand points within 64 of it.
    fn aim_slingshot(w: &PlayState, target: Vec3) -> (PadState, bool) {
        let p = w.player();
        let held = PadState { button: eng_input::pad::BTN_CRIGHT, ..Default::default() };
        let Some(seed) = p.held_actor.and_then(|h| w.actors.actor(h)) else { return (held, false) };
        if p.unk_836 <= 0 || p.unk_6AD != 2 {
            return (held, false);
        }
        let d = target - seed.world_pos;
        let want_yaw = eng_math::vec3f_yaw(seed.world_pos, target);
        let want_pitch = eng_math::atan2_s((d.x * d.x + d.z * d.z).sqrt(), -d.y);
        let (ey, ep) = (want_yaw.wrapping_sub(seed.world_rot.y) as i32, want_pitch.wrapping_sub(seed.world_rot.x) as i32);
        let stick = |err: i32| -> i8 {
            let s = (1..=60).rev().find(|&s: &i32| ((1.0 - eng_math::cos_s((s * 200) as i16)) * 1500.0) as i32 <= err.abs() * 3 / 4).unwrap_or(0);
            if s == 0 { 0 } else { ((s + 7) * err.signum()) as i8 }
        };
        (PadState { button: eng_input::pad::BTN_CRIGHT, stick_x: -stick(ey), stick_y: stick(ep) }, ey.abs() < 64 && ep.abs() < 64)
    }

    /// `Task::GohmaIntro`'s phases in `sub`: 0 to the room's entrance (her trigger) until she
    /// holds Link and her intro starts; 1 idle until its first part is over (state 3: Link free).
    fn gohma_intro(&mut self, w: &PlayState) -> Option<PadState> {
        let idle = PadState::default();
        let Some(g) = Self::gohma(w) else {
            self.wait += 1;
            if self.wait > 60 {
                self.failure = Some("no Boss_Goma in the room".into());
                return None;
            }
            return Some(idle);
        };
        self.wait += 1;
        if self.wait > 900 {
            self.failure = Some(format!("Gohma's intro stalled in phase {} (state {}, Link at {})", self.sub, g.action_state, w.player().actor.world_pos));
            return None;
        }
        match self.sub {
            0 => {
                if g.action_state >= 1 {
                    self.mark(Step::GohmaEntered);
                    self.sub = 1;
                    return Some(idle);
                }
                // At full tilt: the corridor's mouth rises 16, which a slow walk stops at.
                Some(stick_towards(w, GOHMA_ENTRANCE, FULL))
            }
            _ => {
                if g.action_state == 3 {
                    self.finish(Some(Step::GohmaWaiting));
                    return None;
                }
                Some(idle)
            }
        }
    }

    /// `Task::GohmaLook`.
    fn gohma_look(&mut self, w: &PlayState, step: Step) -> Option<PadState> {
        let Some(g) = Self::gohma(w) else {
            self.failure = Some("Gohma gone before she was looked at".into());
            return None;
        };
        if g.action_state != 3 {
            self.finish(Some(step));
            return None;
        }
        self.wait += 1;
        if self.wait > 900 {
            self.failure = Some(format!("Gohma never noticed (state 3: projected {}, Link at {})", g.actor.projected_pos, w.player().actor.world_pos));
            return None;
        }
        // In from the entrance first: from there she's at the edge of the 800 her check allows.
        if self.sub == 0 {
            if Self::xz_dist(w.player().actor.world_pos, GOHMA_LOOK_FROM) > 30.0 {
                return Some(stick_towards(w, GOHMA_LOOK_FROM, FULL));
            }
            self.sub = 1;
        }
        Some(Self::aim_slingshot(w, g.actor.focus_pos).0)
    }

    /// `Task::GohmaWaitBattle`.
    fn gohma_wait_battle(&mut self, w: &PlayState, step: Step) -> Option<PadState> {
        use crate::boss_goma::Action as GA;
        let Some(g) = Self::gohma(w) else {
            self.failure = Some("Gohma gone in her intro".into());
            return None;
        };
        if g.action == GA::FloorMain && w.active_cam_id == oot_game::camera::CAM_ID_MAIN {
            self.finish(Some(step));
            return None;
        }
        self.wait += 1;
        if self.wait > 900 {
            self.failure = Some(format!("Gohma's intro never ended (state {})", g.action_state));
            return None;
        }
        // C-Right kept until first person is over, so letting it go shoots nothing.
        Some(if w.player().unk_6AD == 2 { PadState { button: eng_input::pad::BTN_CRIGHT, ..Default::default() } } else { PadState::default() })
    }

    /// `Task::GohmaFight`, keyed on her action each frame:
    /// - on the floor and patient (`patienceTimer`), or rearing up to attack: locked on (Z) and the
    ///   slingshot drawn (C-Right held: aimed at her by the lock-on), a seed let go only while her
    ///   eye is red and open (her posture, prepare and attack: `EYESTATE_IRIS_FOLLOW_NO_IFRAMES`, so
    ///   the shot doesn't close it), then 25 frames for it to fly;
    /// - stunned (`BossGoma_FloorStunned`, `_FloorDamaged`): locked on, the sword out (B), at her
    ///   eye, a jump slash (A) within 60 of it while she can be hurt, 30 frames apart;
    /// - on the ceiling or on her way to it: locked on, the slingshot drawn as on the floor; with
    ///   the lock lost (out of its range), from under the ceiling's centre on the entrance's side,
    ///   the slingshot aimed up at her in first person; either way a seed let go only while she
    ///   prepares her eggs (her eye red and open);
    /// - falling or landing: idle.
    ///
    /// Z is pressed only when Link isn't locked on (it toggles: "Switch" targeting).
    /// `Step::GohmaStunned`, `GohmaHit` and `GohmaKnockedDown` are noted the first time.
    fn gohma_fight(&mut self, w: &PlayState, step: Step) -> Option<PadState> {
        use crate::boss_goma::Action as GA;
        use eng_input::pad::{BTN_CRIGHT, BTN_Z};
        let idle = PadState::default();
        let Some(gh) = w.actors.all().into_iter().find(|&h| w.actors.downcast::<crate::boss_goma::BossGoma>(h).is_some_and(|g| !g.actor.killed)) else {
            self.failure = Some("Gohma gone before her defeat".into());
            return None;
        };
        let g = w.actors.downcast::<crate::boss_goma::BossGoma>(gh).expect("Gohma");
        if g.action == GA::Defeated {
            self.finish(Some(step));
            return None;
        }
        self.wait += 1;
        if self.wait > 4000 {
            self.failure = Some(format!("the fight with Gohma stalled ({:?}, health {})", g.action, g.actor.col_chk_info.health));
            return None;
        }
        if g.children_gohma_state.iter().any(|&c| c > 0) {
            self.failure = Some("Gohma laid her eggs: the run doesn't fight her larvae".into());
            return None;
        }
        if g.action == GA::FloorStunned {
            self.mark(Step::GohmaStunned);
        }
        if g.action == GA::FallStruckDown {
            self.mark(Step::GohmaKnockedDown);
        }
        if (g.actor.col_chk_info.health as i8) < 10 {
            self.mark(Step::GohmaHit);
        }
        let p = w.player();
        let eye = g.actor.focus_pos;
        let locked = p.focus_actor == Some(gh);
        // Z's edge, when not locked on yet.
        let lock = |pad: &mut PadState, prev: PadState| {
            if !locked && prev.button & BTN_Z == 0 {
                pad.button |= BTN_Z;
            }
        };
        // `tries`: the frames since the last seed or slash; `sub`: the frames C-Right has been held.
        self.tries += 1;
        let eye_open = g.eye_closed_timer == 0 && g.eye_lid_bottom_rot_x > -0x400;
        let red = matches!(g.action, GA::FloorAttackPosture | GA::FloorPrepareAttack | GA::FloorAttack);
        let ceiling = matches!(g.action, GA::CeilingMoveToCenter | GA::CeilingIdle | GA::CeilingPrepareSpawnGohmas | GA::WallClimb);
        match g.action {
            GA::FloorStunned | GA::FloorDamaged => {
                self.sub = 0;
                if p.unk_6AD != 0 {
                    return Some(self.press(BTN_A));
                }
                let near = Self::xz_dist(p.actor.world_pos, eye) < 60.0;
                let mut pad = if near { idle } else { stick_towards(w, eye, FULL) };
                lock(&mut pad, self.prev);
                if !locked {
                    return Some(pad);
                }
                if p.held_item_id != oot_game::item::ITEM_SWORD_KOKIRI {
                    // The sword out (a slash).
                    return Some(self.press(BTN_B));
                }
                if near && g.action == GA::FloorStunned && g.invincibility_frames == 0 && self.tries > 30 {
                    self.tries = 0;
                    pad.button |= BTN_A;
                }
                Some(pad)
            }
            _ if red || ceiling || (g.patience_timer != 0 && matches!(g.action, GA::FloorMain | GA::FloorIdle)) => {
                if self.tries < 25 {
                    self.sub = 0;
                    return Some(idle);
                }
                let fire_ok = if ceiling { g.action == GA::CeilingPrepareSpawnGohmas && eye_open } else { red && eye_open };
                if p.unk_6AD == 2 {
                    // In first person (the lock lost on the ceiling).
                    let (pad, on) = Self::aim_slingshot(w, eye);
                    if on && fire_ok {
                        self.tries = 0;
                        return Some(idle);
                    }
                    return Some(pad);
                }
                if !locked {
                    if ceiling {
                        let stand = GOHMA_SHOOT_UP_FROM;
                        if Self::xz_dist(p.actor.world_pos, stand) > 20.0 {
                            let mut pad = stick_towards(w, stand, FULL);
                            lock(&mut pad, self.prev);
                            return Some(pad);
                        }
                        // Out of the lock-on's range: first person.
                        return Some(Self::aim_slingshot(w, eye).0);
                    }
                    let mut pad = idle;
                    lock(&mut pad, self.prev);
                    return Some(pad);
                }
                // Locked on: C-Right held draws the slingshot aimed at her.
                self.sub += 1;
                if p.held_actor.is_some() && self.sub > 10 && fire_ok {
                    self.tries = 0;
                    self.sub = 0;
                    return Some(idle);
                }
                Some(PadState { button: BTN_CRIGHT, ..idle })
            }
            _ => {
                self.sub = 0;
                if p.unk_6AD != 0 {
                    return Some(self.press(BTN_A));
                }
                Some(idle)
            }
        }
    }

    /// `Task::IntoWarp`.
    fn into_warp(&mut self, w: &PlayState, step: Step) -> Option<PadState> {
        use crate::door_warp1::{Action as WA, DoorWarp1, WARP_DUNGEON_CHILD};
        let idle = PadState::default();
        self.wait += 1;
        if self.wait > 600 {
            self.failure = Some("the blue warp never took Link".into());
            return None;
        }
        let warp = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<DoorWarp1>(h).filter(|d| d.actor.params == WARP_DUNGEON_CHILD && !d.actor.killed));
        match warp {
            Some(d) if d.action == WA::ChildWarpOut => {
                self.finish(Some(step));
                None
            }
            Some(d) if d.action == WA::ChildWarpIdle => Some(stick_towards(w, d.actor.world_pos, SLOW)),
            _ => Some(idle),
        }
    }

    /// `Task::IntoRutoWarp`.
    fn into_ruto_warp(&mut self, w: &PlayState, step: Step) -> Option<PadState> {
        use crate::door_warp1::{Action as WA, WARP_BLUE_RUTO_STATE_INITIAL};
        let idle = PadState::default();
        self.wait += 1;
        if self.wait > 900 {
            self.failure = Some("Ruto's warp never took Link".into());
            return None;
        }
        match ruto_warp(w) {
            Some(d) if d.action == WA::RutoWarpIdle && d.ruto_warp_state != WARP_BLUE_RUTO_STATE_INITIAL => Some(stick_towards(w, d.actor.world_pos, SLOW)),
            Some(d) if d.action != WA::RutoWarpIdle && d.action != WA::WarpAppear => {
                self.finish(Some(step));
                None
            }
            _ => Some(idle),
        }
    }

    /// `Task::EmeraldPart1`, `Task::WarpCutscene`.
    fn warp_cutscene(&mut self, w: &PlayState, limit: usize, step: Step) -> Option<PadState> {
        let idle = PadState::default();
        self.wait += 1;
        if self.wait > limit {
            self.failure = Some(format!("the warp's arrival cutscene never ended (cutscene frame {})", w.cs_ctx.frames));
            return None;
        }
        if w.player().action != PA::BlueWarpArrive {
            self.mark(Step::Arrived);
        }
        // `sub`: the script seen running (the fade in from the warp is a transition too).
        if w.cs_ctx.state != oot_game::cutscene::CS_STATE_IDLE {
            self.sub = 1;
        }
        if self.sub == 1 && w.transition.trigger == oot_game::transition::TRANS_TRIGGER_START {
            self.finish(Some(step));
            return None;
        }
        if w.message_state() != TEXT_STATE_NONE {
            return Some(if Self::text_waits(w) { self.press(BTN_A) } else { idle });
        }
        Some(idle)
    }

    /// `Task::Emerald`.
    fn emerald(&mut self, w: &PlayState, step: Step) -> Option<PadState> {
        use crate::demo_effect::{DEMO_EFFECT_JEWEL_KOKIRI, DemoEffect};
        let idle = PadState::default();
        self.wait += 1;
        if self.wait > 2400 {
            self.failure = Some(format!("the emerald's last part never ended (cutscene frame {})", w.cs_ctx.frames));
            return None;
        }
        let running = w.cs_ctx.state != oot_game::cutscene::CS_STATE_IDLE;
        let jewel_drawn = w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<DemoEffect>(h)).any(|d| d.effect_type() == DEMO_EFFECT_JEWEL_KOKIRI && d.draw.is_some() && !d.actor.killed && w.cs_ctx.npc_actions[1].is_some_and(|c| c.action != 1));
        if running && jewel_drawn {
            self.mark(Step::EmeraldShown);
        }
        if running && w.cs_ctx.frames >= 782 {
            self.mark(Step::TreeDeath);
        }
        if running {
            self.sub = 1;
        }
        if self.sub == 1 && w.transition.trigger == oot_game::transition::TRANS_TRIGGER_START {
            self.finish(Some(step));
            return None;
        }
        if w.message_state() != TEXT_STATE_NONE {
            return Some(if Self::text_waits(w) { self.press(BTN_A) } else { idle });
        }
        Some(idle)
    }

    /// `Task::Chain`.
    fn chain(&mut self, w: &PlayState, step: Step) -> Option<PadState> {
        let idle = PadState::default();
        self.wait += 1;
        if self.wait > 8500 {
            self.failure = Some(format!("the chain didn't end: scene {:#x} layer {}, cutscene frame {}", w.scene_id, w.save.scene_layer, w.cs_ctx.frames));
            return None;
        }
        if let Some(&(_, _, part)) = CHAIN.iter().find(|&&(scene, layer, _)| w.scene_id == scene && w.save.scene_layer == layer) {
            self.mark(part);
        }
        if w.entrance_by_name("ENTR_KOKIRI_FOREST_11") == Some(w.save.entrance_index) && w.save.scene_layer < 4 {
            return self.settle(w, Some(step));
        }
        if w.message_state() != TEXT_STATE_NONE {
            return Some(if Self::text_waits(w) && (w.message_state() != TEXT_STATE_CHOICE || w.msg_ctx.choice_index == 0) { self.press(BTN_A) } else { idle });
        }
        Some(idle)
    }

    /// `Task::Farewell`: the scene and the texts mark the steps; A through every text (the first
    /// choice), until Link stands in Hyrule Field with no cutscene running or waiting.
    fn farewell(&mut self, w: &PlayState, step: Step) -> Option<PadState> {
        use oot_game::cutscene::CS_STATE_IDLE;
        /// `SCENE_LOST_WOODS`, `SCENE_HYRULE_FIELD` (`scene_table.h`).
        const SCENE_LOST_WOODS: u16 = 0x5B;
        const SCENE_HYRULE_FIELD: u16 = 0x51;
        let idle = PadState::default();
        self.wait += 1;
        if self.wait > 4000 {
            self.failure = Some(format!("the farewell didn't end: scene {:#x} layer {}, cutscene frame {}", w.scene_id, w.save.scene_layer, w.cs_ctx.frames));
            return None;
        }
        if w.scene_id == SCENE_LOST_WOODS && w.save.scene_layer == 4 {
            self.mark(Step::Bridge);
        }
        if w.message_state() != TEXT_STATE_NONE && w.msg_ctx.text_id == 0x004A {
            self.mark(Step::FairyOcarina);
        }
        if w.scene_id == SCENE_HYRULE_FIELD {
            self.mark(Step::Field);
            if w.cs_ctx.state == CS_STATE_IDLE && w.save.cutscene_trigger == 0 && w.message_state() == TEXT_STATE_NONE {
                return self.settle(w, Some(step));
            }
        }
        if w.message_state() != TEXT_STATE_NONE {
            return Some(if Self::text_waits(w) && (w.message_state() != TEXT_STATE_CHOICE || w.msg_ctx.choice_index == 0) { self.press(BTN_A) } else { idle });
        }
        Some(idle)
    }

    /// `Task::Owl`.
    fn owl(&mut self, w: &PlayState, step: Step) -> Option<PadState> {
        use oot_game::message::TEXT_STATE_EVENT;
        let idle = PadState::default();
        self.wait += 1;
        if self.wait > 3000 {
            self.failure = Some("the owl's talk and flight didn't end".into());
            return None;
        }
        // The nearest owl (the field has four), kept once chosen.
        if self.owl.is_none() {
            let link = w.player().actor.world_pos;
            self.owl = w
                .actors
                .all()
                .into_iter()
                .filter(|&h| w.actors.downcast::<crate::en_owl::EnOwl>(h).is_some_and(|o| !o.actor.killed))
                .min_by(|&a, &b| w.actors.actor(a).unwrap().world_pos.distance(link).total_cmp(&w.actors.actor(b).unwrap().world_pos.distance(link)));
        }
        let owl = self.owl.and_then(|h| w.actors.downcast::<crate::en_owl::EnOwl>(h)).filter(|o| !o.actor.killed);
        let st = w.message_state();
        if st != TEXT_STATE_NONE {
            // His first text (0x2064 outside the forest, 0x2068 on the castle's road, 0x206C at
            // Kakariko's stairs).
            if matches!(w.msg_ctx.text_id, 0x2064 | 0x2068 | 0x206C) {
                self.mark(Step::OwlTalk);
            }
            if st == TEXT_STATE_CHOICE {
                // OWL_OK is the second choice: the stick down (a push, then a release), then A.
                if w.msg_ctx.choice_index != 1 {
                    return Some(if self.prev.stick_y != 0 { idle } else { PadState { stick_y: -60, ..idle } });
                }
                return Some(self.press(BTN_A));
            }
            return Some(if Self::text_waits(w) || st == TEXT_STATE_EVENT { self.press(BTN_A) } else { idle });
        }
        match owl {
            // Before his talk: towards him (he takes Link within 360).
            Some(o) if !self.steps.iter().any(|(s, _)| *s == Step::OwlTalk) => Some(stick_towards(w, o.actor.world_pos, RUN)),
            // Flying off: Link's free (the Stalchildren too, at night).
            Some(_) => Some(self.stalchild_fight_pad(w).unwrap_or(idle)),
            None => {
                if !self.steps.iter().any(|(s, _)| *s == Step::OwlTalk) {
                    self.failure = Some("no owl to talk to".into());
                    return None;
                }
                self.settle(w, Some(step))
            }
        }
    }

    /// `Task::GohmaWaitGone`.
    fn gohma_wait_gone(&mut self, w: &PlayState, step: Step) -> Option<PadState> {
        self.wait += 1;
        if self.wait > 2000 {
            self.failure = Some("Gohma's death never ended".into());
            return None;
        }
        if Self::gohma(w).is_none() && w.active_cam_id == oot_game::camera::CAM_ID_MAIN && Self::settled(w) {
            self.finish(Some(step));
            return None;
        }
        Some(PadState::default())
    }

    /// `Task::TakeHeart`: at the heart container (`Item_B_Heart`) until it's taken, A through its
    /// text, then until Link stands.
    fn take_heart(&mut self, w: &PlayState, step: Step) -> Option<PadState> {
        let idle = PadState::default();
        self.wait += 1;
        if self.wait > 1200 {
            self.failure = Some("the heart container was never taken".into());
            return None;
        }
        if w.message_state() != TEXT_STATE_NONE {
            return Some(if Self::text_waits(w) { self.press(BTN_A) } else { idle });
        }
        let heart = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<crate::item_b_heart::ItemBHeart>(h).filter(|b| !b.actor.killed));
        match heart {
            Some(b) => Some(stick_towards(w, b.actor.world_pos, SLOW)),
            None => {
                if Self::settled(w) {
                    self.finish(Some(step));
                    return None;
                }
                Some(idle)
            }
        }
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

    /// `Task::PullStone`.
    fn pull_stone(&mut self, w: &PlayState, home: Vec3, step: Step) -> Option<PadState> {
        use crate::bg_haka::{Action as HakaAction, BgHaka};
        self.wait += 1;
        if self.wait > 900 {
            let p = w.player();
            self.failure = Some(format!("the gravestone at {home} never pulled (Link {:?} at {:?}, state2 {:#x})", p.action, p.actor.world_pos, p.state2));
            return None;
        }
        let Some(stone) = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<BgHaka>(h).filter(|b| b.actor.home_pos.distance(home) < 1.0)) else {
            self.failure = Some(format!("no gravestone at {home}"));
            return None;
        };
        let held = Self::holding_on(w);
        if stone.action == HakaAction::IdleOpened {
            if held {
                return Some(PadState::default());
            }
            self.finish(Some(step));
            return None;
        }
        if held {
            let p = w.player();
            let behind = p.actor.world_pos - Vec3::new(eng_math::sin_s(p.actor.shape_rot.y), 0.0, eng_math::cos_s(p.actor.shape_rot.y)) * 100.0;
            let mut pad = stick_towards(w, behind, RUN);
            pad.button = BTN_A;
            return Some(pad);
        }
        let at = stone.actor.world_pos;
        Some(self.grab_pad(w, at))
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
    /// `Task::FightPeahat`.
    fn fight_peahat(&mut self, w: &PlayState, home: Vec3, step: Step) -> Option<PadState> {
        use crate::en_peehat::{EnPeehat, PEAHAT_STATE_ATTACK_RECOIL, PEAHAT_STATE_SEEK_PLAYER};
        use eng_input::pad::{BTN_R, BTN_Z};
        let idle = PadState::default();
        self.wait += 1;
        let Some(p) = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<EnPeehat>(h).filter(|p| p.actor.home_pos.distance(home) < 1.0)) else {
            // Its room's actors spawn in the first frames; once seen, gone is killed.
            if self.sub > 0 {
                self.finish(Some(step));
                return None;
            }
            if self.wait > 60 {
                self.failure = Some(format!("no Peahat at {home}"));
                return None;
            }
            return Some(idle);
        };
        if self.wait > 1500 {
            self.failure = Some(format!("the Peahat fight stalled (state {}, health {})", p.state, p.actor.col_chk_info.health));
            return None;
        }
        // Z pressed once, then held (a release on the next two frames: the lock's edge).
        self.sub += 1;
        let mut pad = PadState { button: if (2..4).contains(&self.sub) { 0 } else { BTN_Z }, ..idle };
        if self.tries > 0 {
            self.tries -= 1;
        } else if p.actor.xz_dist_to_player < 70.0 && (p.state == PEAHAT_STATE_SEEK_PLAYER || p.state == PEAHAT_STATE_ATTACK_RECOIL) {
            pad.button |= BTN_B;
            self.tries = 12;
        } else {
            pad.button |= BTN_R;
        }
        Some(pad)
    }

    /// `Task::FightStalchildren`.
    fn fight_stalchildren(&mut self, w: &PlayState, count: usize, step: Step) -> Option<PadState> {
        use crate::en_skb::{Action as SA, EnSkb};
        let idle = PadState::default();
        self.wait += 1;
        if self.wait > 2500 {
            self.failure = Some(format!("the Stalchildren fight stalled ({} of {count} killed)", self.items.len()));
            return None;
        }
        let skbs: Vec<(ActorHandle, &EnSkb)> = w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<EnSkb>(h).map(|s| (h, s))).collect();
        for (h, s) in &skbs {
            if s.action == SA::Death && !self.items.contains(h) {
                self.items.push(*h);
            }
        }
        let left = skbs.iter().filter(|(_, s)| !s.actor.killed && !matches!(s.action, SA::Death | SA::Despawn)).count();
        if (count > 0 && self.items.len() >= count) || (count == 0 && left == 0) {
            self.items.clear();
            self.finish(Some(step));
            return None;
        }
        Some(self.stalchild_fight_pad(w).unwrap_or(idle))
    }

    /// `Task::FightStalchildren`'s input against the nearest risen Stalchild, if there's one.
    fn stalchild_fight_pad(&mut self, w: &PlayState) -> Option<PadState> {
        use crate::en_skb::{EnSkb, SKB_BEHAVIOR_ATTACKING};
        use eng_input::pad::{BTN_R, BTN_Z};
        let idle = PadState::default();
        let link = w.player().actor.world_pos;
        let target = w
            .actors
            .all()
            .into_iter()
            .filter_map(|h| w.actors.downcast::<EnSkb>(h))
            .filter(|s| !s.actor.killed && s.action_state >= SKB_BEHAVIOR_ATTACKING)
            .min_by(|a, b| Self::xz_dist(a.actor.world_pos, link).total_cmp(&Self::xz_dist(b.actor.world_pos, link)));
        let Some(s) = target else {
            self.sub = 0;
            return None;
        };
        self.sub += 1;
        let z = if (2..4).contains(&self.sub) { 0 } else { BTN_Z };
        let d = Self::xz_dist(s.actor.world_pos, link);
        if self.tries > 0 {
            self.tries -= 1;
            return Some(PadState { button: z, ..idle });
        }
        if d > 50.0 {
            let mut p = stick_towards(w, s.actor.world_pos, RUN);
            p.button |= z;
            return Some(p);
        }
        if d < 55.0 {
            self.tries = 12;
            return Some(PadState { button: z | BTN_B, ..idle });
        }
        Some(PadState { button: z | BTN_R, ..idle })
    }

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
                // Z let go (held from the Keese's fight), so the next press locks on.
                if self.prev.button & BTN_Z != 0 {
                    return Some(idle);
                }
                // Woken by Link coming near (or already up: the Keese's fight is beside it).
                if matches!(b.action, KA::Awaken | KA::Upright | KA::Spin) {
                    self.sub = 1;
                    return Some(z(idle));
                }
                Some(stick_towards(w, home, RUN))
            }
            1 => {
                // Not locked on and it's off to the side (Link faces where the Keese died): turned
                // to it in place first, no Z (a light tilt, as for the Keese).
                let off = eng_math::vec3f_yaw(link, home).wrapping_sub(w.player().actor.shape_rot.y);
                if w.player().focus_actor != Some(h) && (off as i32).abs() > 0x1000 {
                    return Some(stick_towards(w, home, 25.0));
                }
                // Not locked on yet (Z went down out of range): Z again every other frame.
                let z = |mut p: PadState| {
                    if w.player().focus_actor == Some(h) || self.prev.button & BTN_Z == 0 {
                        p.button |= BTN_Z;
                    }
                    p
                };
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

    /// `Task::ClimbTo`: the stick at `top`, full (a hang climbs up only on a full tilt:
    /// `controlStickSpinAngles`, a magnitude of 55), until Link stands within 10 of its height,
    /// then settled.
    fn climb_to(&mut self, w: &PlayState, top: Vec3, step: Step) -> Option<PadState> {
        let p = w.player();
        if self.sub == 1 {
            return self.settle(w, Some(step));
        }
        if p.grounded() && (p.actor.world_pos.y - top.y).abs() < 10.0 && matches!(p.action, PA::StandingStill | PA::Run) {
            self.sub = 1;
            return Some(PadState::default());
        }
        self.wait += 1;
        if self.wait > 900 {
            self.failure = Some(format!("never up to {top} (Link {:?} at {:?})", p.action, p.actor.world_pos));
            return None;
        }
        // Hanging (Player_Action_8084BBE4), the stick along Link's facing climbs up
        // (controlStickSpinAngles >= 0): the grab turned him to face the wall's normal.
        if p.action == PA::Hang {
            let ahead = p.actor.world_pos + Vec3::new(eng_math::sin_s(p.actor.shape_rot.y), 0.0, eng_math::cos_s(p.actor.shape_rot.y)) * 100.0;
            return Some(stick_towards(w, ahead, FULL));
        }
        Some(stick_towards(w, top, FULL))
    }

    /// `Task::PushCrate`'s phases: 0 to the spot at the offset from the crate, 1 at it until Link
    /// holds on (A at its wall), 2 A and the stick ahead until it's far enough or has fallen; then
    /// Link lets go and settles.
    fn push_crate(&mut self, w: &PlayState, near: Vec3, offset: Vec3, dir: Vec3, until: f32, step: Option<Step>) -> Option<PadState> {
        use crate::bg_spot15_rrbox::{Action as CrateAction, BgSpot15Rrbox};
        let idle = PadState::default();
        if self.crate_handle.is_none() {
            self.crate_handle = w
                .actors
                .all()
                .into_iter()
                .filter(|&h| w.actors.downcast::<BgSpot15Rrbox>(h).is_some())
                .min_by(|&a, &b| Self::xz_dist(w.actors.actor(a).unwrap().world_pos, near).total_cmp(&Self::xz_dist(w.actors.actor(b).unwrap().world_pos, near)));
        }
        let Some(c) = self.crate_handle.and_then(|h| w.actors.downcast::<BgSpot15Rrbox>(h)) else {
            self.failure = Some(format!("no crate near {near}"));
            return None;
        };
        let at = c.actor.world_pos;
        let p = w.player();
        self.wait += 1;
        if self.wait > 2400 {
            self.failure = Some(format!("the crate at {at} never pushed to {until} along {dir} (Link {:?} at {:?})", p.action, p.actor.world_pos));
            return None;
        }
        // Done: in the moat (fallen and landed), or far enough along and at rest.
        let fallen = at.y < near.y - 20.0;
        if self.sub == 3 || (c.action == CrateAction::Rest && (fallen || at.dot(dir) >= until - 0.5)) {
            self.sub = 3;
            if Self::holding_on(w) {
                return Some(idle);
            }
            return self.settle(w, step);
        }
        match self.sub {
            0 => {
                let behind = at + offset;
                if Self::xz_dist(p.actor.world_pos, behind) < 8.0 {
                    self.sub = 1;
                    return Some(idle);
                }
                Some(stick_towards(w, behind, SLOW))
            }
            1 => {
                if Self::holding_on(w) {
                    self.sub = 2;
                    return Some(idle);
                }
                Some(self.grab_pad(w, at))
            }
            _ => {
                if !Self::holding_on(w) {
                    if c.action == CrateAction::Fall {
                        return Some(idle);
                    }
                    self.sub = 0;
                    return Some(idle);
                }
                let ahead = p.actor.world_pos + dir * 100.0;
                let mut pad = stick_towards(w, ahead, RUN);
                pad.button = BTN_A;
                Some(pad)
            }
        }
    }

    /// `Task::ShowItem`'s phases: 0 to the point, 1 at the actor until it offers a talk (Player's
    /// `talkActor`), 2 its C button, 3 the texts (A whenever one waits) until closed and Link
    /// stands.
    /// `Task::Ocarina`.
    fn play_song(&mut self, w: &PlayState, button: usize, song: u8, step: Step, away: bool) -> Option<PadState> {
        use eng_input::pad::{BTN_A, BTN_CDOWN, BTN_CLEFT, BTN_CRIGHT, BTN_CUP};
        use oot_game::audio::ocarina::{OCARINA_BTN_A, OCARINA_BTN_C_DOWN, OCARINA_BTN_C_LEFT, OCARINA_BTN_C_RIGHT, OCARINA_BTN_C_UP};
        let idle = PadState::default();
        let p = w.player();
        self.wait += 1;
        if self.wait > 1200 {
            self.failure = Some(format!("song {song} never played (Link {:?}, msgMode {:#x})", p.action, w.msg_ctx.msg_mode));
            return None;
        }
        match self.sub {
            // The ocarina out.
            0 => {
                if p.action == PA::Ocarina {
                    self.mark(Step::OcarinaOut);
                    self.sub = 1;
                    return Some(idle);
                }
                if w.message_state() != TEXT_STATE_NONE {
                    return Some(idle);
                }
                Some(self.press([BTN_CLEFT, BTN_CDOWN, BTN_CRIGHT][button]))
            }
            // Its staff up (MSGMODE_OCARINA_PLAYING).
            1 => {
                if w.msg_ctx.msg_mode == oot_game::message::MSGMODE_OCARINA_PLAYING {
                    self.sub = 2;
                    self.notes = 0;
                }
                Some(idle)
            }
            // The notes.
            2 => {
                let b = w.audio.ocarina.song_buttons(song);
                let (note, phase) = (self.notes / 5, self.notes % 5);
                self.notes += 1;
                if note >= b.num_buttons as usize {
                    self.sub = 3;
                    return Some(idle);
                }
                let pad = match b.buttons_index[note] {
                    OCARINA_BTN_A => BTN_A,
                    OCARINA_BTN_C_DOWN => BTN_CDOWN,
                    OCARINA_BTN_C_RIGHT => BTN_CRIGHT,
                    OCARINA_BTN_C_LEFT => BTN_CLEFT,
                    OCARINA_BTN_C_UP => BTN_CUP,
                    _ => 0,
                };
                Some(if phase < 3 { PadState { button: pad, ..Default::default() } } else { idle })
            }
            // Heard.
            3 => {
                if w.msg_ctx.unk_e3f2 == song as u16 && w.msg_ctx.msg_mode != oot_game::message::MSGMODE_OCARINA_PLAYING {
                    if !away {
                        self.finish(Some(step));
                        return None;
                    }
                    self.mark(step);
                    self.sub = 4;
                }
                Some(idle)
            }
            // The ocarina away (a talk may follow at once: Saria's Song's).
            _ => {
                if p.action != PA::Ocarina {
                    self.finish(None);
                    return None;
                }
                Some(idle)
            }
        }
    }

    /// `Task::OcarinaNotes`.
    fn ocarina_notes(&mut self, w: &PlayState, button: usize, ready: fn(&PlayState) -> bool, tune: &Tune, step: Step) -> Option<PadState> {
        use eng_input::pad::{BTN_A, BTN_CDOWN, BTN_CLEFT, BTN_CRIGHT, BTN_CUP};
        let idle = PadState::default();
        let p = w.player();
        self.wait += 1;
        if self.wait > 3000 {
            self.failure = Some(format!("the tune never played (Link {:?}, msgMode {:#x}, note {})", p.action, w.msg_ctx.msg_mode, self.notes / 5));
            return None;
        }
        match self.sub {
            // The ocarina out.
            0 => {
                if p.action == PA::Ocarina {
                    self.mark(Step::OcarinaOut);
                    self.sub = 1;
                    return Some(idle);
                }
                if w.message_state() != TEXT_STATE_NONE {
                    return Some(idle);
                }
                Some(self.press([BTN_CLEFT, BTN_CDOWN, BTN_CRIGHT][button]))
            }
            // The listener's texts until its box is ready.
            1 => {
                if ready(w) {
                    self.sub = 2;
                    self.notes = 0;
                    return Some(idle);
                }
                let event = w.message_state() == oot_game::message::TEXT_STATE_EVENT;
                Some(if Self::text_waits(w) || event { self.press(BTN_A) } else { idle })
            }
            // The notes.
            _ => {
                let notes = match tune {
                    Tune::Notes(n) => n.clone(),
                    Tune::Frogs => w.audio.ocarina.frogs_song_notes.to_vec(),
                };
                let (note, phase) = (self.notes / 5, self.notes % 5);
                let Some(&b) = notes.get(note) else {
                    self.finish(Some(step));
                    return None;
                };
                if phase == 0 && !ready(w) {
                    return Some(idle);
                }
                self.notes += 1;
                let pad = [BTN_A, BTN_CDOWN, BTN_CRIGHT, BTN_CLEFT, BTN_CUP].get(b as usize).copied().unwrap_or(0);
                Some(if phase < 3 { PadState { button: pad, ..Default::default() } } else { idle })
            }
        }
    }

    /// `Task::TalkThrough`.
    fn talk_through(&mut self, w: &PlayState, choices: &[u8], step: Step) -> Option<PadState> {
        let idle = PadState::default();
        self.wait += 1;
        if self.wait > 3000 {
            self.failure = Some(format!("the talk never ended (text {:#x})", w.msg_ctx.text_id));
            return None;
        }
        let st = w.message_state();
        if self.sub == 0 {
            if st != TEXT_STATE_NONE {
                self.sub = 1;
                self.notes = 0;
            } else if self.wait > 120 {
                self.failure = Some("no text opened".into());
                return None;
            }
            return Some(idle);
        }
        if st == TEXT_STATE_NONE {
            self.finish(Some(step));
            return None;
        }
        if st == TEXT_STATE_CHOICE {
            let c = choices.get(self.notes).copied().unwrap_or(0);
            if c == 1 && w.msg_ctx.choice_index == 0 {
                // The stick down to the second answer.
                return Some(PadState { stick_y: -60, ..Default::default() });
            }
            if self.prev.stick_y != 0 {
                return Some(idle);
            }
            let pad = self.press(BTN_A);
            if pad.button != 0 {
                self.notes += 1;
            }
            return Some(pad);
        }
        // (Saria's texts end in an event: A goes on from it too.)
        let event = st == oot_game::message::TEXT_STATE_EVENT;
        Some(if Self::text_waits(w) || event { self.press(BTN_A) } else { idle })
    }

    /// `Task::Playback`: `sub` 0 until the playback's staff is up (A through texts before it), 1 the
    /// wrong note, 2 until it's back up after the failure (A through its text), 3 the song's notes
    /// (each held 3 frames, let go 2), 4 until the success.
    fn playback(&mut self, w: &PlayState, song: u8, wrong_first: bool, step: Step) -> Option<PadState> {
        use eng_input::pad::{BTN_CDOWN, BTN_CLEFT, BTN_CRIGHT, BTN_CUP};
        use oot_game::audio::ocarina::{OCARINA_BTN_A, OCARINA_BTN_C_DOWN, OCARINA_BTN_C_LEFT, OCARINA_BTN_C_RIGHT, OCARINA_BTN_C_UP};
        use oot_game::message::{MSGMODE_SONG_PLAYBACK, MSGMODE_SONG_PLAYBACK_SUCCESS};
        let idle = PadState::default();
        self.wait += 1;
        if self.wait > 4000 {
            self.failure = Some(format!("song {song} never played back (msgMode {:#x}, text {:#x})", w.msg_ctx.msg_mode, w.msg_ctx.text_id));
            return None;
        }
        let pad_of = |b: u8| match b {
            OCARINA_BTN_A => BTN_A,
            OCARINA_BTN_C_DOWN => BTN_CDOWN,
            OCARINA_BTN_C_RIGHT => BTN_CRIGHT,
            OCARINA_BTN_C_LEFT => BTN_CLEFT,
            OCARINA_BTN_C_UP => BTN_CUP,
            _ => 0,
        };
        let buttons = w.audio.ocarina.song_buttons(song);
        let text_a = |s: &mut Self| {
            let event = w.message_state() == oot_game::message::TEXT_STATE_EVENT;
            if Self::text_waits(w) || event { s.press(BTN_A) } else { idle }
        };
        match self.sub {
            0 | 2 => {
                if w.msg_ctx.msg_mode == MSGMODE_SONG_PLAYBACK {
                    self.sub = if self.sub == 0 && wrong_first { 1 } else { 3 };
                    self.notes = 0;
                    return Some(idle);
                }
                Some(text_a(self))
            }
            1 => {
                // A note the song doesn't start with.
                let first = buttons.buttons_index[0];
                let wrong = if first == OCARINA_BTN_A { BTN_CUP } else { BTN_A };
                let phase = self.notes;
                self.notes += 1;
                if phase >= 5 {
                    self.sub = 2;
                    return Some(idle);
                }
                Some(if phase < 3 { PadState { button: wrong, ..Default::default() } } else { idle })
            }
            3 => {
                let (note, phase) = (self.notes / 5, self.notes % 5);
                self.notes += 1;
                if note >= buttons.num_buttons as usize {
                    self.sub = 4;
                    return Some(idle);
                }
                let pad = pad_of(buttons.buttons_index[note]);
                Some(if phase < 3 { PadState { button: pad, ..Default::default() } } else { idle })
            }
            _ => {
                if w.msg_ctx.msg_mode == MSGMODE_SONG_PLAYBACK_SUCCESS {
                    self.finish(Some(step));
                    return None;
                }
                Some(idle)
            }
        }
    }

    /// `Task::Converse`: `sub` 0 to the point, 1 at the actor until it offers, 2 A until the talk
    /// takes, 3 through the boxes.
    fn converse(&mut self, w: &PlayState, who: Who, from: Vec3, choices: &[u8], until: fn(&PlayState) -> bool, step: Step) -> Option<PadState> {
        let idle = PadState::default();
        self.wait += 1;
        if self.wait > 20000 {
            self.failure = Some(format!("the talk with {who:?} never ended (text {:#x})", w.msg_ctx.text_id));
            return None;
        }
        let link = w.player().actor.world_pos;
        if self.sub == 1 || self.sub == 2 {
            let p = w.player();
            if p.cs_mode != 0 || p.action == PA::Talk || w.message_state() != TEXT_STATE_NONE {
                self.sub = 3;
                self.notes = 0;
                return Some(idle);
            }
        }
        if self.sub < 3 {
            let Some(h) = Self::find(w, who) else {
                self.failure = Some(format!("no {who:?} to talk to"));
                return None;
            };
            let at = w.actors.actor(h).unwrap().world_pos;
            let p = w.player();
            match self.sub {
                0 => {
                    if Self::xz_dist(link, from) < 10.0 {
                        self.sub = 1;
                    }
                    return Some(stick_towards(w, from, SLOW));
                }
                1 => {
                    if p.target_actor == Some(h) {
                        self.sub = 2;
                        return Some(idle);
                    }
                    return Some(stick_towards(w, at, SLOW));
                }
                _ => {
                    if p.action == PA::Talk || p.action == PA::ItemPutAway || p.cs_mode != 0 || w.message_state() != TEXT_STATE_NONE {
                        self.sub = 3;
                        self.notes = 0;
                        return Some(idle);
                    }
                    if p.target_actor != Some(h) {
                        self.sub = 1;
                        return Some(idle);
                    }
                    return Some(self.press(BTN_A));
                }
            }
        }
        let st = w.message_state();
        if st == TEXT_STATE_NONE && until(w) {
            self.finish(Some(step));
            return None;
        }
        if st == TEXT_STATE_CHOICE {
            let c = choices.get(self.notes).copied().unwrap_or(0);
            if c == 1 && w.msg_ctx.choice_index == 0 {
                return Some(PadState { stick_y: -60, ..Default::default() });
            }
            if self.prev.stick_y != 0 {
                return Some(idle);
            }
            let pad = self.press(BTN_A);
            if pad.button != 0 {
                self.notes += 1;
            }
            return Some(pad);
        }
        let event = st == oot_game::message::TEXT_STATE_EVENT;
        Some(if Self::text_waits(w) || event { self.press(BTN_A) } else { idle })
    }

    /// `Task::Lift`.
    /// The horse Link is on (Player's `rideActor`, riding).
    fn ridden(w: &PlayState) -> Option<&crate::en_horse::EnHorse> {
        let p = w.player();
        if p.state1 & crate::player::STATE1_23 == 0 {
            return None;
        }
        p.ride_actor.and_then(|h| crate::en_horse::ridden(&w.actors, h))
    }

    /// Epona nearest Link.
    fn epona(w: &PlayState) -> Option<&crate::en_horse::EnHorse> {
        let link = w.player().actor.world_pos;
        w.actors
            .all()
            .into_iter()
            .filter_map(|h| w.actors.downcast::<crate::en_horse::EnHorse>(h))
            .filter(|e| !e.actor.killed && e.ty == crate::en_horse::HORSE_EPONA)
            .min_by(|a, b| a.actor.world_pos.distance(link).total_cmp(&b.actor.world_pos.distance(link)))
    }

    /// The yaw from the horse's heading to `to`.
    fn ride_angle(horse: &crate::en_horse::EnHorse, to: Vec3) -> i16 {
        oot_game::target::yaw_to(horse.actor.world_pos, to).wrapping_sub(horse.actor.shape_rot.y)
    }

    /// The stick steering the horse at `to`: `EnHorse_StickDirection`'s angle (`atan2(-x, y)`, from
    /// straight up) the yaw from her heading to the point times `RIDE_GAIN`, at most `RIDE_TURN`, at full tilt. Far
    /// off her heading (more than `RIDE_TURN`), let go until she stands, then straight to the side
    /// (`EnHorse_MountedTurn`: she turns in place).
    fn ride_stick(horse: &crate::en_horse::EnHorse, to: Vec3) -> PadState {
        let want = Self::ride_angle(horse, to);
        if (want as i32).abs() > RIDE_TURN && horse.actor.speed_xz != 0.0 {
            return PadState::default();
        }
        let a = if (want as i32).abs() > RIDE_TURN { if want > 0 { 0x4000 } else { -0x4000 } } else { (want as i32 * RIDE_GAIN).clamp(-RIDE_TURN, RIDE_TURN) as i16 };
        let (sn, cs) = (eng_math::sin_s(a), eng_math::cos_s(a));
        PadState { button: 0, stick_x: (-sn * FULL) as i8, stick_y: (cs * FULL) as i8 }
    }

    /// `Task::Mount`.
    fn mount(&mut self, w: &PlayState, step: Step) -> Option<PadState> {
        use crate::en_horse::*;
        let idle = PadState::default();
        self.wait += 1;
        if self.wait > 2000 {
            let horse = Self::epona(w).map(|h| (h.action, h.animation_idx, h.actor.world_pos));
            self.failure = Some(format!("never on the horse (Link {:?} at {}; Epona's action, animation and place {horse:?})", w.player().action, w.player().actor.world_pos));
            return None;
        }
        let p = w.player();
        if p.state1 & crate::player::STATE1_23 != 0 {
            if p.action == PA::Ride && p.action_var2 != 0 {
                self.finish(Some(step));
                return None;
            }
            // Mounting.
            return Some(idle);
        }
        let Some(horse) = Self::epona(w) else { return Some(idle) };
        // Offered (Player's `rideActor`, cleared at his update's end: the "Climb" it gave).
        if w.interface_ctx.unk_1f0 == oot_game::interface::DO_ACTION_CLIMB {
            return Some(self.press(BTN_A));
        }
        let standing = horse.action == ENHORSE_ACT_IDLE && (horse.animation_idx == ENHORSE_ANIM_IDLE || horse.animation_idx == ENHORSE_ANIM_WHINNEY);
        if !standing {
            self.sub = 0;
            return Some(idle);
        }
        let yaw = horse.actor.shape_rot.y.wrapping_add(0x4000);
        let left = horse.actor.world_pos + Vec3::new(eng_math::sin_s(yaw), 0.0, eng_math::cos_s(yaw)) * 60.0;
        let link = p.actor.world_pos;
        if self.sub == 0 {
            if Self::xz_dist(link, left) < 15.0 {
                self.sub = 1;
            } else {
                return Some(stick_towards(w, left, RUN));
            }
        }
        Some(stick_towards(w, horse.actor.world_pos, SLOW))
    }

    /// `Task::Ride`.
    fn ride(&mut self, w: &PlayState, points: &[Vec3], spur: bool, step: Step) -> Option<PadState> {
        use crate::en_horse::*;
        self.wait += 1;
        if self.wait > 3000 {
            self.failure = Some(format!("never rode through {points:?}"));
            return None;
        }
        let Some(horse) = Self::ridden(w) else {
            self.failure = Some(format!("not on a horse (Link {:?})", w.player().action));
            return None;
        };
        while self.sub < points.len() && Self::xz_dist(horse.actor.world_pos, points[self.sub]) < RIDE_RADIUS {
            self.sub += 1;
        }
        if self.sub >= points.len() {
            if horse.actor.speed_xz == 0.0 && horse.check_4() {
                self.finish(Some(step));
                return None;
            }
            return Some(PadState::default());
        }
        let mut pad = Self::ride_stick(horse, points[self.sub]);
        let lined_up = (Self::ride_angle(horse, points[self.sub]) as i32).abs() < 0x800;
        if spur && lined_up && horse.action == ENHORSE_ACT_MOUNTED_GALLOP && horse.num_boosts == 6 && self.prev.button & BTN_A == 0 {
            pad.button |= BTN_A;
        }
        Some(pad)
    }

    /// `Task::Pedestal`.
    fn pedestal(&mut self, w: &PlayState, step: Step) -> Option<PadState> {
        self.wait += 1;
        if self.wait > 600 {
            self.failure = Some(format!("the pedestal never taken (Link {:?} at {})", w.player().action, w.player().actor.world_pos));
            return None;
        }
        // (Its object loads a few frames after Play_Init.)
        let Some(h) = w.actors.all().into_iter().find(|&h| w.actors.downcast::<crate::bg_toki_swd::BgTokiSwd>(h).is_some()) else {
            return Some(PadState::default());
        };
        let sword = w.actors.downcast::<crate::bg_toki_swd::BgTokiSwd>(h).unwrap();
        if sword.action != crate::bg_toki_swd::Action::Func808BAF40 && w.cs_ctx.state != oot_game::cutscene::CS_STATE_IDLE {
            self.finish(Some(step));
            return None;
        }
        let p = w.player();
        if self.sub == 0 {
            if p.interact_range_actor == Some(h) {
                self.sub = 1;
                return Some(self.press(BTN_A));
            }
            return Some(stick_towards(w, sword.actor.world_pos, SLOW));
        }
        Some(PadState::default())
    }

    fn lift(&mut self, w: &PlayState, near: Vec3, step: Step) -> Option<PadState> {
        let idle = PadState::default();
        self.wait += 1;
        if self.wait > 900 {
            self.failure = Some(format!("the cucco near {near} never lifted (Link {:?})", w.player().action));
            return None;
        }
        if self.carried.is_none() {
            self.carried = w
                .actors
                .all()
                .into_iter()
                .filter(|&h| w.actors.downcast::<crate::en_niw::EnNiw>(h).is_some_and(|c| !c.actor.killed))
                .min_by(|&a, &b| {
                    let d = |h| w.actors.actor(h).map_or(f32::MAX, |c| c.world_pos.distance(near));
                    d(a).total_cmp(&d(b))
                });
        }
        let Some(h) = self.carried else {
            self.failure = Some(format!("no cucco near {near}"));
            return None;
        };
        let Some(c) = w.actors.actor(h) else {
            self.failure = Some("the cucco went".into());
            return None;
        };
        let p = w.player();
        match self.sub {
            0 => {
                if p.interact_range_actor == Some(h) && p.state1 & crate::player::STATE1_11 == 0 {
                    self.sub = 1;
                    return Some(self.press(BTN_A));
                }
                let mag = if Self::xz_dist(p.actor.world_pos, c.world_pos) > 80.0 { RUN } else { SLOW };
                Some(stick_towards(w, c.world_pos, mag))
            }
            _ => {
                if p.held_actor == Some(h) && p.action == PA::StandingStill {
                    self.finish(Some(step));
                    return None;
                }
                if p.state1 & crate::player::STATE1_11 == 0 && p.held_actor.is_none() {
                    // The A went to something else (a talk): try again.
                    self.sub = 0;
                }
                Some(idle)
            }
        }
    }

    /// `Task::Throw`.
    fn throw(&mut self, w: &PlayState, at: Vec3, step: Step) -> Option<PadState> {
        let idle = PadState::default();
        self.wait += 1;
        if self.wait > 300 {
            self.failure = Some(format!("never threw (Link {:?})", w.player().action));
            return None;
        }
        let p = w.player();
        match self.sub {
            0 => {
                let link = p.actor.world_pos;
                let yaw = eng_math::atan2_s(at.z - link.z, at.x - link.x);
                if (yaw.wrapping_sub(p.actor.shape_rot.y) as i32).abs() < 0x400 {
                    self.sub = 1;
                    return Some(self.press(BTN_A));
                }
                Some(stick_towards(w, at, SLOW))
            }
            _ => {
                if p.held_actor.is_none() && p.action != PA::Throw && p.action != PA::PutDown {
                    self.carried = None;
                    self.finish(Some(step));
                    return None;
                }
                if p.held_actor.is_some() && !matches!(p.action, PA::Throw | PA::PutDown) {
                    return Some(self.press(BTN_A));
                }
                Some(idle)
            }
        }
    }

    /// `Task::ThrowAtRun`: `sub` counts the points passed, then 1000 the A pressed.
    fn throw_at_run(&mut self, w: &PlayState, points: &[Vec3], release: Vec3, step: Step) -> Option<PadState> {
        self.wait += 1;
        if self.wait > 600 {
            self.failure = Some(format!("never threw (Link {:?})", w.player().action));
            return None;
        }
        let p = w.player();
        if self.sub >= 1000 {
            if p.held_actor.is_none() && !matches!(p.action, PA::Throw | PA::PutDown) {
                self.finish(Some(step));
                return None;
            }
            return Some(PadState::default());
        }
        let link = p.actor.world_pos;
        while self.sub < points.len() && Self::xz_dist(link, points[self.sub]) < WAYPOINT_RADIUS {
            self.sub += 1;
        }
        let target = points[self.sub.min(points.len() - 1)];
        let mut pad = stick_towards(w, target, RUN);
        if self.sub + 1 >= points.len() && Self::xz_dist(link, release) < 20.0 && p.actor.speed_xz > 1.1 {
            pad.button |= BTN_A;
            self.sub = 1000;
        }
        Some(pad)
    }

    /// `Task::LiftBombFlower`.
    fn lift_bomb_flower(&mut self, w: &PlayState, near: Vec3, step: Step) -> Option<PadState> {
        self.wait += 1;
        if self.wait > 600 {
            self.failure = Some(format!("the Bomb Flower near {near} never lifted (Link {:?})", w.player().action));
            return None;
        }
        let flowers = || w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<crate::en_bombf::EnBombf>(h).map(|f| (h, f)));
        let Some((h, f)) = flowers().filter(|(_, f)| f.actor.params < 0).min_by(|a, b| a.1.actor.home_pos.distance(near).total_cmp(&b.1.actor.home_pos.distance(near))) else {
            self.failure = Some(format!("no Bomb Flower near {near}"));
            return None;
        };
        let p = w.player();
        let holds_bomb = p.held_actor.is_some_and(|b| w.actors.downcast::<crate::en_bombf::EnBombf>(b).is_some_and(|b| b.actor.params == 0));
        if holds_bomb && p.action == PA::StandingStill {
            self.finish(Some(step));
            return None;
        }
        if self.sub == 0 {
            if p.interact_range_actor == Some(h) && p.state1 & crate::player::STATE1_11 == 0 {
                self.sub = 1;
                return Some(self.press(BTN_A));
            }
            // At a run until on the flower's level and close: up a slope a slower stick leaves
            // Link standing (his speed less the slope's).
            let near_level = (p.actor.world_pos.y - f.actor.world_pos.y).abs() < 1.0;
            let mag = if Self::xz_dist(p.actor.world_pos, f.actor.world_pos) > 80.0 || !near_level { RUN } else { SLOW };
            return Some(stick_towards(w, f.actor.world_pos, mag));
        }
        if p.state1 & crate::player::STATE1_11 == 0 && p.held_actor.is_none() && p.action == PA::StandingStill {
            // The A went to something else: try again.
            self.sub = 0;
        }
        Some(PadState::default())
    }

    /// `Task::PutDown`: `sub` counts the points passed, then 1000 turning, 1001 stopping, 1002 the
    /// A pressed.
    fn put_down(&mut self, w: &PlayState, points: &[Vec3], face: Vec3, step: Step) -> Option<PadState> {
        self.wait += 1;
        if self.wait > 900 {
            self.failure = Some(format!("never put it down (Link {:?})", w.player().action));
            return None;
        }
        let p = w.player();
        if self.sub < 1000 {
            let link = p.actor.world_pos;
            while self.sub < points.len() && Self::xz_dist(link, points[self.sub]) < WAYPOINT_RADIUS {
                self.sub += 1;
            }
            if self.sub < points.len() {
                return Some(stick_towards(w, points[self.sub], RUN));
            }
            self.sub = 1000;
        }
        let link = p.actor.world_pos;
        let yaw = eng_math::atan2_s(face.z - link.z, face.x - link.x);
        match self.sub {
            1000 => {
                if (yaw.wrapping_sub(p.actor.shape_rot.y) as i32).abs() < 0x1000 {
                    self.sub = 1001;
                    return Some(PadState::default());
                }
                Some(stick_towards(w, face, SLOW))
            }
            1001 => {
                if p.linear_velocity == 0.0 && p.action == PA::StandingStill {
                    self.sub = 1002;
                    return Some(self.press(BTN_A));
                }
                Some(PadState::default())
            }
            _ => {
                if p.held_actor.is_none() && p.action == PA::StandingStill {
                    self.finish(Some(step));
                    return None;
                }
                if p.held_actor.is_some() && p.action == PA::StandingStill && !w.player_in_cs_mode() {
                    // The A went nowhere (a switch's cutscene took it): again.
                    self.sub = 1001;
                }
                Some(PadState::default())
            }
        }
    }

    /// `Task::TakeBomb`.
    fn take_bomb(&mut self, w: &PlayState, c: usize, step: Step) -> Option<PadState> {
        self.wait += 1;
        let p = w.player();
        if p.held_actor.is_some_and(|b| w.actors.downcast::<crate::en_bom::EnBom>(b).is_some()) && p.action == PA::StandingStill && p.upper != crate::player::UpperAction::Change {
            self.finish(Some(step));
            return None;
        }
        if self.wait > 120 {
            self.failure = Some(format!("no bomb out (held {}, bombs {})", p.held_item_ap, w.save.ammo(oot_game::item::ITEM_BOMB)));
            return None;
        }
        if self.sub == 0 {
            self.sub = 1;
            const C_BUTTONS: [u16; 3] = [eng_input::pad::BTN_CLEFT, eng_input::pad::BTN_CDOWN, eng_input::pad::BTN_CRIGHT];
            return Some(self.press(C_BUTTONS[c]));
        }
        Some(PadState::default())
    }

    /// `Task::UseItem`: `sub` 0 the press, 1 until Link has left standing, 2 back.
    fn use_item(&mut self, w: &PlayState, c: usize, step: Step) -> Option<PadState> {
        const C_BUTTONS: [u16; 3] = [eng_input::pad::BTN_CLEFT, eng_input::pad::BTN_CDOWN, eng_input::pad::BTN_CRIGHT];
        self.wait += 1;
        if self.wait > 900 {
            self.failure = Some(format!("the item on C button {c} never ended (Link {:?})", w.player().action));
            return None;
        }
        let p = w.player();
        let standing = p.action == PA::StandingStill || p.action == PA::Swim;
        match self.sub {
            0 => {
                self.sub = 1;
                Some(self.press(C_BUTTONS[c]))
            }
            1 => {
                if !standing {
                    self.sub = 2;
                }
                Some(PadState::default())
            }
            _ => {
                if w.message_state() != oot_game::message::TEXT_STATE_NONE {
                    return Some(self.press(BTN_A));
                }
                if standing && !w.player_in_cs_mode() {
                    self.finish(Some(step));
                    return None;
                }
                Some(PadState::default())
            }
        }
    }

    /// `Task::CatchInBottle`: `sub` 0 the bottle out, 1 at a fish, 2 the swing.
    fn catch_in_bottle(&mut self, w: &PlayState, c: usize, step: Step) -> Option<PadState> {
        const C_BUTTONS: [u16; 3] = [eng_input::pad::BTN_CLEFT, eng_input::pad::BTN_CDOWN, eng_input::pad::BTN_CRIGHT];
        self.wait += 1;
        if self.wait > 1500 {
            self.failure = Some(format!("no fish caught (Link {:?})", w.player().action));
            return None;
        }
        let p = w.player();
        let on_button = w.save.equips.button_items[c + 1];
        let standing = p.action == PA::StandingStill || p.action == PA::Swim;
        if on_button == oot_game::item::ITEM_BOTTLE_FISH && standing && w.message_state() == oot_game::message::TEXT_STATE_NONE && !w.player_in_cs_mode() {
            self.finish(Some(step));
            return None;
        }
        match self.sub {
            0 => {
                if p.held_item_ap == crate::player::PLAYER_IA_BOTTLE && p.upper != crate::player::UpperAction::Change && p.state1 & crate::player::STATE1_8 == 0 {
                    self.sub = 1;
                    return Some(PadState::default());
                }
                Some(self.press(C_BUTTONS[c]))
            }
            1 => {
                let fish = p.interact_range_actor.and_then(|h| w.actors.actor(h)).is_some_and(|a| a.id == crate::en_fish::ACTOR_EN_FISH);
                if fish {
                    self.sub = 2;
                    return Some(self.press(C_BUTTONS[c]));
                }
                let link = p.actor.world_pos;
                let nearest = w
                    .actors
                    .all()
                    .into_iter()
                    .filter_map(|h| w.actors.actor(h))
                    .filter(|a| a.id == crate::en_fish::ACTOR_EN_FISH && !a.killed)
                    .map(|a| a.world_pos)
                    .min_by(|a, b| Self::xz_dist(*a, link).total_cmp(&Self::xz_dist(*b, link)));
                match nearest {
                    Some(f) => Some(stick_towards(w, f, SLOW)),
                    None => Some(PadState::default()),
                }
            }
            _ => {
                if w.message_state() != oot_game::message::TEXT_STATE_NONE {
                    return Some(self.press(BTN_A));
                }
                if p.action != PA::SwingBottle && standing {
                    // A miss: at a fish again.
                    self.sub = 1;
                }
                Some(PadState::default())
            }
        }
    }

    /// `Task::Glide`: `sub` counts the points passed; `tries` is 1 once Link is in the air.
    fn glide(&mut self, w: &PlayState, points: &[Vec3], step: Step) -> Option<PadState> {
        let p = w.player();
        self.wait += 1;
        if self.wait > 1200 {
            self.failure = Some("the glide never came down".into());
            return None;
        }
        if !p.grounded() {
            if self.tries == 0 && p.held_actor.is_none() {
                self.failure = Some("off the edge without the cucco".into());
                return None;
            }
            self.tries = 1;
        } else if self.tries == 1 {
            self.tries = 2;
            self.mark(step);
        }
        let link = p.actor.world_pos;
        while self.sub < points.len() && Self::xz_dist(link, points[self.sub]) < WAYPOINT_RADIUS {
            self.sub += 1;
        }
        if self.sub >= points.len() {
            if self.tries == 1 {
                // Still in the air: on towards the last point until he lands.
                return Some(stick_towards(w, points[points.len() - 1], RUN));
            }
            if self.tries != 2 {
                self.failure = Some("the glide's way ended without a glide".into());
                return None;
            }
            self.tries = 0;
            self.finish(None);
            return None;
        }
        Some(stick_towards(w, points[self.sub], RUN))
    }

    fn show_item(&mut self, w: &PlayState, who: Who, from: Vec3, button: usize, step: Step) -> Option<PadState> {
        let idle = PadState::default();
        let Some(h) = Self::find(w, who) else {
            self.failure = Some(format!("no {who:?} to show the item to"));
            return None;
        };
        let at = w.actors.actor(h).unwrap().world_pos;
        let p = w.player();
        let link = p.actor.world_pos;
        self.wait += 1;
        // (Kakariko's gate guard's texts on the letter run some 1,400 frames.)
        if self.wait > 3000 {
            self.failure = Some(format!("the item never shown to {who:?} (Link {:?} at {link})", p.action));
            return None;
        }
        match self.sub {
            0 => {
                if Self::xz_dist(link, from) < WAYPOINT_RADIUS {
                    self.sub = 1;
                }
                Some(stick_towards(w, from, RUN))
            }
            1 => {
                if p.target_actor == Some(h) && p.linear_velocity == 0.0 {
                    self.sub = 2;
                    return Some(idle);
                }
                if p.target_actor == Some(h) {
                    return Some(idle);
                }
                Some(stick_towards(w, at, SLOW))
            }
            2 => {
                if p.action == PA::ExchangeItem || w.message_state() != TEXT_STATE_NONE {
                    self.sub = 3;
                    return Some(idle);
                }
                if p.target_actor != Some(h) {
                    self.sub = 1;
                    return Some(idle);
                }
                Some(self.press([eng_input::pad::BTN_CLEFT, eng_input::pad::BTN_CDOWN, eng_input::pad::BTN_CRIGHT][button]))
            }
            _ => {
                if w.message_state() == TEXT_STATE_NONE && !matches!(p.action, PA::ExchangeItem | PA::Talk) {
                    self.finish(Some(step));
                    return None;
                }
                // A at a box that waits, or at an event's (the gate guard's letter, his
                // `func_80A53C0C`).
                let event = w.message_state() == oot_game::message::TEXT_STATE_EVENT;
                Some(if Self::text_waits(w) || event { self.press(BTN_A) } else { idle })
            }
        }
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

    /// `Task::DiveUnder`: A pressed at the surface and held under it, steering at `to` (sub 0); within `WAYPOINT_RADIUS` of it across,
    /// let go, still steering, until Link is within 40 of the surface (sub 1).
    fn dive_under(&mut self, w: &PlayState, to: Vec3, surface: f32, step: Step) -> Option<PadState> {
        let link = w.player().actor.world_pos;
        self.wait += 1;
        if self.wait > 600 {
            self.failure = Some(format!("the dive never reached {to} (at {link}, sub {})", self.sub));
            return None;
        }
        if self.sub == 0 && Self::xz_dist(link, to) < WAYPOINT_RADIUS {
            self.sub = 1;
        }
        if self.sub == 1 && link.y > surface - 40.0 {
            self.finish(Some(step));
            return None;
        }
        let mut pad = stick_towards(w, to, FULL);
        if self.sub == 0 {
            // The dive starts on a press at the surface; under it, A held keeps it going.
            pad.button = if w.player().action == crate::player::Action::Dive || link.y < surface - 45.0 { BTN_A } else { self.press(BTN_A).button };
        }
        Some(pad)
    }

    /// `Task::Sneak`: `walk`, but still while the next steps (`SNEAK_AHEAD` on along the way)
    /// would be seen and where Link stands isn't (`seen_at`); a catch fails the run.
    fn sneak(&mut self, w: &PlayState, points: &[Vec3]) -> Option<PadState> {
        if w.msg_ctx.text_id == 0x702D && w.message_state() != TEXT_STATE_NONE {
            let at = w.player().actor.world_pos;
            self.failure = Some(format!("caught by a courtyard guard at ({:.0}, {:.0}, {:.0})", at.x, at.y, at.z));
            return None;
        }
        let link = w.player().actor.world_pos;
        let mut i = self.sub;
        while i < points.len() && Self::xz_dist(link, points[i]) < WAYPOINT_RADIUS {
            i += 1;
        }
        if i < points.len() {
            let d = points[i] - link;
            let d = Vec3::new(d.x, 0.0, d.z).normalize_or_zero();
            let ahead = link + d * SNEAK_AHEAD;
            if Self::seen_at(w, ahead) && !Self::seen_at(w, link) {
                self.wait += 1;
                return Some(PadState::default());
            }
        }
        self.walk(w, points, FULL)
    }

    /// Whether a courtyard guard (`En_Heishi1`) looking on the active camera's screen, or the
    /// floor's at `p`, would see Link at `p`: within `70 + SNEAK_MARGIN` of the guard; or his
    /// look's ball (`Effect_Ss_Solder_Srch_Ball`: from 60 up, 30 a frame, stopped by the first poly
    /// a sphere of 30 touches) able to reach within `70 + SNEAK_MARGIN` of `p` in any direction his
    /// head turns to (the body's yaw ± 0x2500, and `SNEAK_FAN` more); or a ball in flight that
    /// will.
    fn seen_at(w: &PlayState, p: Vec3) -> bool {
        use oot_game::surface::SurfaceType;
        let active = w.active_camera().bg_cam_index;
        let floor = w.col.entity_raycast_down(p + Vec3::Y * 30.0).1.map(|id| w.col.bg_cam_index(id) as i16);
        let reach = 70.0 + SNEAK_MARGIN;
        let ball_reaches = |mut pos: Vec3, vel: Vec3, steps: usize| {
            for _ in 0..steps {
                pos += vel;
                if w.col.sph_vs_first_poly(pos, 30.0) {
                    return false;
                }
                if pos.distance(p) < reach {
                    return true;
                }
            }
            false
        };
        for h in w.actors.all() {
            let Some(g) = w.actors.downcast::<crate::en_heishi1::EnHeishi1>(h) else { continue };
            if g.actor.killed || g.ty == 5 {
                continue;
            }
            let cams = crate::en_heishi1::bg_cam_indices(g.path);
            if !cams.iter().any(|&c| c == active || Some(c) == floor) {
                continue;
            }
            if Self::xz_dist(g.actor.world_pos, p) < reach {
                return true;
            }
            let yaw_to = eng_math::vec3f_yaw(g.actor.world_pos, p);
            let off = yaw_to.wrapping_sub(g.actor.shape_rot.y) as i32;
            if off.abs() <= 0x2500 + SNEAK_FAN {
                let head = g.actor.world_pos + Vec3::Y * 60.0;
                let dir = Vec3::new(p.x - head.x, 0.0, p.z - head.z).normalize_or_zero() * 30.0;
                if ball_reaches(head, dir, 100) {
                    return true;
                }
            }
        }
        // The balls already flying.
        w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == oot_game::effect::EFFECT_SS_SOLDER_SRCH_BALL).any(|e| ball_reaches(e.pos, e.velocity, SNEAK_BALL_FRAMES))
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
                // A crawlspace that ends in an exit (Hyrule Castle's drain): its transition ends it.
                if w.transition.trigger != TRANS_TRIGGER_OFF {
                    self.finish(Some(step));
                    return None;
                }
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
    fn pick_nearest(&mut self, w: &PlayState, step: Step) -> Option<PadState> {
        let link = w.player().actor.world_pos;
        if self.sub == 0 {
            self.sub = 1;
            self.dropped = Self::drops(w)
                .into_iter()
                .filter(|&h| w.actors.downcast::<EnItem00>(h).is_some_and(|e| e.action != ItemAction::Collected && Self::xz_dist(e.actor.world_pos, link) < 200.0))
                .min_by(|&a, &b| Self::xz_dist(w.actors.actor(a).unwrap().world_pos, link).total_cmp(&Self::xz_dist(w.actors.actor(b).unwrap().world_pos, link)));
            self.drop = self.dropped.and_then(|h| w.actors.actor(h)).map(|a| a.params);
        }
        let Some(e) = self.dropped.and_then(|h| w.actors.downcast::<EnItem00>(h)).filter(|e| e.action != ItemAction::Collected) else {
            self.finish(Some(step));
            return None;
        };
        self.wait += 1;
        if self.wait > 400 {
            self.failure = Some(format!("the item at {} wasn't picked up", e.actor.world_pos));
            return None;
        }
        let at = e.actor.world_pos;
        Some(stick_towards(w, at, if Self::xz_dist(link, at) < 40.0 { SLOW } else { RUN }))
    }

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
            Who::Hylian(ty) => w.actors.downcast::<crate::en_hy::EnHy>(h).is_some_and(|a| a.actor.params & 0x7F == ty),
            Who::Malon => w.actors.downcast::<crate::en_ma1::EnMa1>(h).is_some_and(|a| !a.actor.killed),
            Who::Talon => w.actors.downcast::<crate::en_ta::EnTa>(h).is_some_and(|a| !a.actor.killed),
            Who::Zelda => w.actors.downcast::<crate::en_zl4::EnZl4>(h).is_some_and(|a| !a.actor.killed),
            Who::GateGuard => w.actors.downcast::<crate::en_heishi2::EnHeishi2>(h).is_some_and(|a| !a.actor.killed && a.ty == 5),
            Who::Dampe => w.actors.downcast::<crate::en_tk::EnTk>(h).is_some_and(|a| !a.actor.killed),
            Who::Owl => w.actors.downcast::<crate::en_owl::EnOwl>(h).is_some_and(|a| !a.actor.killed),
            Who::KingZora => w.actors.downcast::<crate::en_kz::EnKz>(h).is_some_and(|a| !a.actor.killed),
            Who::Ruto => w.actors.downcast::<crate::en_ru1::EnRu1>(h).is_some_and(|a| !a.actor.killed),
            Who::Bonooru => w.actors.downcast::<crate::en_kakasi3::EnKakasi3>(h).is_some_and(|a| !a.actor.killed),
            Who::Ingo => w.actors.downcast::<crate::en_in::EnIn>(h).is_some_and(|a| !a.actor.killed && (2..=4).contains(&crate::en_in::TYPE.get(a.actor.params))),
            Who::AdultMalon => w.actors.downcast::<crate::en_ma3::EnMa3>(h).is_some_and(|a| !a.actor.killed),
            Who::SongWall => w.actors.downcast::<crate::en_okarina_tag::EnOkarinaTag>(h).is_some_and(|a| !a.actor.killed && a.type_ == 5),
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

impl Playthrough {
    /// A slingshot shot from wherever Link stands (the travel test's shots, GAME-05 milestone 5a):
    /// `Task::DrawSlingshot`, `Task::Shoot` at `target` until switch flag `flag` is set, then
    /// `Task::LeaveFirstPerson`; steps `SlingshotDrawn` and `EyeShot` (the flag set, whatever it
    /// opens). `Route::Slingshot`'s frame cap.
    pub fn slingshot_shot(target: Vec3, flag: i32) -> Playthrough {
        let mut p = Self::for_route(Route::Slingshot);
        p.tasks = vec![Task::DrawSlingshot(Step::SlingshotDrawn), Task::Shoot(target, flag, Step::EyeShot), Task::LeaveFirstPerson];
        p
    }
}
