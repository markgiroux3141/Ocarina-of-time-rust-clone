# GAME-12: Epona, Ingo's races and the adult Hyrule Field

**Goal:** Phase 13 of the [roadmap](ROADMAP.md): adult Link at Lon Lon Ranch rents a horse from
Ingo, calls Epona with Epona's Song and rides her, races Ingo twice with the bet, wins Epona, is
shut in, jumps the fence out, and rides in Hyrule Field. Then the ranch after Epona (Malon's
obstacle course, Malon at night) and the field's Poes. Exit (ROADMAP's outline): Epona won.

**Status:** surveyed, proposed and decided (2026-10-24; the user took every recommendation: the
calls' section). The phase starts from the `adult-awake` preset (GAME-11's exit review).
Milestone 1 is done (hand-tested 2026-10-25, committed and pushed). Milestone 2 is done
(hand-tested 2026-10-25, committed and pushed). Milestone 3 is done (hand-tested 2026-10-26, committed and
pushed): **the phase's exit is met**, `epona-won` is Phase 14's start.

| # | Milestone | Status |
|---|---|---|
| 1 | Riding Epona | done (2026-10-25) |
| 2 | Ingo, the rental and the races | done (2026-10-25) |
| 3 | Epona won and out (the exit) | done (2026-10-26) |
| 4 | The ranch after and the field's Poes | |

## The survey

Checked against the decomp (52a510f) and the pack (`out/data27`), not against earlier notes. Line
numbers are the decomp's. `ootx scene-info` was run on every layer of `spot20` (0 to 10),
`souko`, `malon_stable` and `spot00` (0 to 13). Two read-only helpers read `En_Horse`, Player's
riding code, `z_horse.c` and the camera, and the ranch's actors. Their key claims were checked
against the C: Epona fleeing until the song, Ingo's texts and rental, and the horse's `Rand` calls.

### The start: `adult-awake`

The preset was read in GAME-11's exit review. Link is an adult:
- **Buttons:** the Master Sword on B, C-Left empty, C-Down bombs (15), C-Right the Ocarina of
  Time.
- **Items and equipment:** sticks, the slingshot, the boomerang, an empty bottle, Zelda's Letter;
  the Hylian Shield and the Kokiri Tunic.
- **Quest and stats:** Epona's Song among his five songs; six hearts; **16 rupees**.

The ranch's flags from Phase 9 are set, `EVENTCHKINF_CAN_LEARN_EPONAS_SONG` among them.
`EVENTCHKINF_EPONA_OBTAINED` (0x18), `INFTABLE_INGO_TALKED_TO_ADULT_LINK` (0x9A) and the race's
`eventInf[0]` are clear. The preset holds no time of day; Ingo works by day only (`IS_NIGHT`:
0x204E, "Z Z Z…").

### The scenes and their layers

The entrance table's rows for the ranch (`ENTR_LON_LON_RANCH_0` and the rest) are all spawn 0's
scene with the usual four rows, so an adult loads layer 2 by day and 3 by night. `souko` (Talon's
house and the silo) and `malon_stable` have header 0 on every layer: one set for both ages.

| Scene | Layer | Who's there | Phase |
|---|---|---|---|
| `spot20` (the ranch) | 2 (adult, day) | Ingo `En_In` 0xFFFF (rot.z 7: after Epona), 2, 3, 4 (rot.z 5: before); adult Malon `En_Ma3` (after Epona only); `En_Horse` 0xFFFF (Ingo's rental horse: bit 15, `HORSE_HNI`); `Bg_Ingate` 0 ×2 (the corral's gate), 1 and 3 (the ranch's gate Ingo shuts); `Bg_Umajump` 0 ×2 (the corral's fences); 6 `En_Horse_Normal` (rot.z 5 before Epona, 7 after); 5 `En_Niw`; `En_Weather_Tag` 0x0201; 2 `En_Wonder_Item` | **13** |
| | 3 (adult, night) | `Bg_Ingate` 0 ×4, `En_Ta` 2 (Talon back from Kakariko), `En_Ma2` rot.z 6 (singing, before Epona) and 8 (after), `Bg_Umajump` 0 ×2 | **13** |
| | 4 (`CS_INDEX_0`: `gLonLonRanchRaceIntroCs`) | The races' layer: `En_Horse_Game_Check` 1 (Ingo's race, or Malon's course once Epona is had), `Bg_Ingate` 0 ×4, `Bg_Umajump` 1 ×8 (the course's fences: killed without Epona) | **13** |
| | 5 (`gLonLonRanchEponasSongCs`) | The child's Epona's Song (Phase 9, done) | 9 |
| | 6 to 10 | The credits (`Demo_Ec`, `En_Light`, `Demo_6K`) | 19 |
| `malon_stable` | all | `En_In` 0xFFFF rot.z 2, 3, 4; `En_Ma2` rot.z 5 (by day, before Epona); 2 `En_Horse_Normal`; 2 `En_Cow`; `En_Ta` 2 | 9 and **13** |
| `souko` | all | `En_Ta` (the house), cows, crates, cuccos, pots: all ported (Phase 9) | 9 |
| `spot00` (the field) | 2, 3 | `Bg_Spot00_Break` ×4 (ported in Phase 12), **10 `En_Po_Field`** (params 0x16 to 0x1F: each a switch flag), `Obj_Hamishi`, the grottos, `Shot_Sun`. No `Bg_Umajump`: the horse jumps the field's low walls by its own collision tests | **13** |
| | 10 (`gHyruleFieldGateEponaJump2Cs`) | Unused (`z_horse.c:176`); 2 `Bg_Umajump` 0xFFFF | 13 (as coverage has it) |

**Coverage:** COVERAGE's Phase 13 row is right: `Bg_Ingate`, `Bg_Umajump`, `En_Horse`,
`En_Horse_Game_Check`, `En_Ma3` and `En_Po_Field`, none ported. `En_In`'s and `En_Ma2`'s adult
parts belong here too, but they count as ported (their child's parts are). No corrections
proposed.

### The route

1. **To the ranch.** From the Temple, `Route::Ruins`' way to `ENTR_HYRULE_FIELD_7`, then across
   the field to its exit 7 (`ENTR_LON_LON_RANCH_0`, spawn 0, the ranch's gateway). The field
   crossing is long: a debug start enters the ranch at `ENTR_LON_LON_RANCH_0` by day.
2. **Epona** (`Horse_SetupInGameplay`, `z_horse.c:136-142`). In the ranch without Epona,
   `Play_Init`'s `Actor_InitPlayerHorse` spawns her at (0, 0, -500), `HORSE_PTYPE_1`.
   - Her init gives her `EnHorse_InitFleePlayer` (`z_en_horse.c:913-915`): she runs from Link
     within 300 and goes home, unrideable.
   - `R_EPONAS_SONG_PLAYED` (the message box's, ported: `msg_ctx.r_eponas_song_played`) makes her
     rideable (`:2615`). The placed rental horse (`HORSE_HNI`) is rideable at once.
3. **Ingo and the rental** (`EnIn_GetTextIdAdult`, `z_en_in.c:160-214`).
   - By day before Epona, Ingo 2 stands by the corral's gate. Texts: 0x2030 ("…rumors…", the first
     time: sets 0x9A), then 0x2031 ("Pay me 10 Rupees and you can ride").
   - Paying starts the rental: `EnIn_StartRentalPeriod` reloads at `ENTR_LON_LON_RANCH_6` (spawn
     6, inside the corral). It sets the race state to 1 and starts a 60-second timer
     (`Interface_SetTimer(60)`, the HUD timers ported in Phase 9). Ingo 3 runs the rental
     (`EnIn_RentalPeriod`, `:691`).
   - Riding during the first rental sets `INFTABLE_AB`. When the timer runs out (0x2025, "Time's
     up"), `EnIn_EndRentalPeriod` goes to `ENTR_LON_LON_RANCH_8` and sets **`EVENTCHKINF_1B`**.
4. **The race offer.** On a later rental, on horseback with `EVENTCHKINF_1B`, Ingo says 0x2038
   ("You're getting better! How about a little race…"). Yes needs **50 rupees** (`:703`; under 50
   the text closes). That writes the horse's type and sets the state to racing. It goes to
   `ENTR_LON_LON_RANCH_0` with `CS_INDEX_0`: layer 4 and `gLonLonRanchRaceIntroCs`.
   - `Horse_SetupInCutscene` puts Link on his horse (`HORSE_PTYPE_6`) at (-250, 1, -1580).
   - The bet is taken only on a loss (`EnIn_PlayerLostRace`: -50).
5. **The race** (`En_Horse_Game_Check` 1, `:79-241`).
   - It spawns Ingo on his horse (`0x8003`: `HORSE_PTYPE_INGO_SPAWNED_RIDING`, which spawns
     `En_In` params 1, rot.z 1, the rider) at (-250, 1, -1650).
   - The timer starts on frame 50; Link may go on frame 80, Ingo on 81.
   - Ingo's horse follows eight waypoints (`sIngoRaceWaypoints`) at 12.07 the first time, 12.625
     the second. Link's horse can spur six boosts to 14 (Epona) or 12 (the rental).
   - Three checkpoints within 400, then the finish. At 180 seconds, or on the ranch's exit
     square, Ingo wins. One lap is about 9,000 units: some 750 to 1,000 frames.
   - Ending: a win goes to `ENTR_LON_LON_RANCH_7` (state 4, or 6 with `SECOND_RACE`), a loss to
     `_8` (state 3, then 7 for a rematch: 0x205B).
6. **The first win:** 0x203A ("Sh-shoooot!"): a second race for the horse (`SECOND_RACE`).
   - A rental horse in the second race rears at the start (`ENHORSE_FLAG_21 | _20`, `:828`,
     `:3504-3512`), so the second race is won on Epona: Epona's Song is needed.
7. **Epona won, the gate shut** (`EnIn_PlayerWonSecondRace` and on, `:796-918`).
   - At `_7` with state 6, `Horse_SetupInCutscene` puts Link on Epona (`HORSE_PTYPE_5`) at
     (894, 0, -2084).
   - Texts 0x203B ("What's up with that horse?! Is that Epona?") and 0x203C ("…I'll give the horse
     to you…").
   - `EnIn_ClosingGate` drives `csCtx.curFrame` 0 to 50 (the door's sound on 44). `Bg_Ingate` 1
     and 3 swing shut over frames 10 to 50 (`func_80892890`).
   - Ingo then guards the gate: riding past him goes to `ENTR_HYRULE_FIELD_15`.
8. **The jump out.** The ranch's fence exits 2, 3 and 4 lead to `ENTR_HYRULE_FIELD_11`, `_12`
   and `_13`; the gate to `_15`.
   - `Cutscene_HandleEntranceTriggers` (`z_demo.c:2412-2436`, ported, with its
     `EVENTCHKINF_EPONA_OBTAINED` exception at `cutscene.rs:1651`) sets **0x18** and plays the
     South, West, East or Gate `EponaJumpCs`.
   - `Horse_SetupInCutscene` mounts Link on Epona for it (`HORSE_PTYPE_7`, `EnHorse_Cs*`). **The
     exit.**
9. **Riding in the field.** With 0x18, every field-like scene spawns Epona: where she was left
   (`horseData`), or inactive at the scene's song point (`HORSE_PTYPE_INACTIVE`: the field's
   (-460, 100, 6640)). Epona's Song calls her (`EnHorse_Spawn`'s points). Scene exits on
   horseback keep him mounted (`R_EXITED_SCENE_RIDING_HORSE`, `HORSE_PTYPE_PLAYER_SPAWNED_RIDING`).
10. **The ranch after.**
    - By day: Ingo 0xFFFF (obedient: 0x205F); adult Malon (`En_Ma3`: 0x2000 to 0x2003, then her
      obstacle course, `En_Horse_Game_Check` 4: two laps over the eight `Bg_Umajump` 1 within 180
      seconds; under 50 seconds `EVENTCHKINF_HORSE_RACE_COW_UNLOCK`; `HIGH_SCORE(HS_HORSE_RACE)`).
    - By night: Malon (`En_Ma2` rot.z 8) and Ingo asleep in the stable. Before Epona, Malon (rot.z
      6) sings and waits for Epona's Song (`INFTABLE_PLAYED_SONG_FOR_ADULT_MALON`).
    - Talon (`En_Ta` 2) is back only with `EVENTCHKINF_TALON_RETURNED_FROM_KAKARIKO` (0x6B), which
      Kakariko's adult Talon sets once Epona is had: Phase 17's village.
11. **The field's Poes** (`En_Po_Field`). One instance serves all ten places. Near one (within 150
    across), a **Big Poe** appears if Link rides (or on a 40% roll on foot) and its switch flag is
    clear; otherwise a small Poe on foot, nothing on horseback.
    - Health 4; the sword and arrows do 2. A Big Poe flees when hit.
    - The soul goes in a bottle: 0x5008 (`ITEM_BOTTLE_POE`) or 0x508F (`ITEM_BOTTLE_BIG_POE`, the
      switch flag set).
    - A fight: the hand test's.

**Flags in order** (the story's): 0x9A (Ingo's first words) → race state 1, `INFTABLE_A2` cleared
→ `INFTABLE_AB` (riding) → `EVENTCHKINF_1B` (the first rental over) → the horse's type, state 2
→ state 4 (the first win) → `SECOND_RACE`, state 2 → state 6 → `EVENTCHKINF_EPONA_OBTAINED`
(the field's entrance cutscene).

### What's already ported and reused

- **Horses:** `En_Horse_Normal` and `En_Horse_Link_Child` (the plain horses with their adult
  checks, the foal and Epona's Song for the child), the skin system's `Skin_DrawImpl` path (GAME-04b),
  `Horse_RotateToPoint` (`oot_game::horse`, which notes the rest as BACKLOG #51), the save's
  `horseData`.
- **Ingo, Malon, Talon:** `En_In` (the child's whole; the adult's texts and choice replies are
  ported but never shown; its four adult roles are placeholders chosen at init, `en_in.rs:512-534`),
  `En_Ma2` (`EnMa2_GetSpawnIndex` and the kill only, `en_ma2.rs`), `En_Ma1` (killed for an adult,
  as the C), `En_Ta` (whole, the stable's type 2 included).
- **Systems:** the HUD timers (the rental and the race), the entrance cutscene table with the four
  Epona jump scripts in the pack, `R_EPONAS_SONG_PLAYED`, the camera's `Camera_KeepOn1`,
  `Camera_KeepOn3` and `Camera_Subj3` (three of `CAM_SET_HORSE`'s five modes), Player's 15 or so
  `PLAYER_STATE1_23` guards already in place, `ARROW_NORMAL_HORSE`, the bottles (the Poes' souls).

### The systems needed

| System | C | The port has | Missing |
|---|---|---|---|
| **Player on a horse** | `z_player.c`: `Player_ActionHandler_3` and `D_80854578` (the mount, 7189-7243), `func_8083A360`, `Player_Action_8084CC98` (riding: Link's animation from the horse's frame, the lash, the camera's request, the bow aimed, talk; 13226-13398), the dismount (`func_8084C89C`, `_8084C9BC`, `Player_Action_8084D3E4`), 27 `PLAYER_STATE1_23` branches elsewhere (the do-actions, exits, re-mounting on spawn); 18 `gPlayerAnim_link_uma_*` in `gameplay_keep` | the guards, the arrow's kind | about 390 lines dedicated, about 80 in branches. No riding draw in `z_player_lib.c` |
| **`En_Horse`** | 3,868 + 207. Types `HORSE_EPONA` (`object_horse`, `gEponaSkel`) and `HORSE_HNI` (`object_hni`: Ingo's and the rental); params `HORSE_PTYPE_*` 1 to 9 | nothing | about 3,450 this phase: init and types, the mounted movement (509), the jumps (144), inactive and called (40, 226), Ingo's horse and race info (203), fleeing (160), floors and collision with the auto-jump (430), boosts, the draw with `riderPos` from skin limb 30 (183), the cutscene actions (about 200 of 296). Later: horseback archery (169, Phase 18), Gerudo Valley's bridge jump (161, Phase 18), the credits' cutscenes |
| **`z_horse.c` and `z_actor.c`** | `Horse_CanSpawn`, `_ResetHorseData`, `_FixLakeHyliaPosition`, `_SetupInGameplay`, `_SetupInCutscene`, `_InitPlayerHorse` (about 300); `Actor_MountHorse`, `_RequestHorseCameraSetting`, `_InitPlayerHorse`, `_IsMounted`, `_SetRideActor`, `_NotMounted` (about 40) | `Horse_RotateToPoint` | all the rest (about 340) |
| **The horse camera** | `CAM_SET_HORSE` (`sCamSetHorseModes`): normal and Z-parallel `Camera_Normal3` (144) with `Camera_CalcAtForHorse` (39); `KEEP1`, `KEEP3`, `SUBJ3` | the last three | `Camera_Normal3`, `Camera_CalcAtForHorse`, the setting and its modes (183) |
| **The ranch's actors** | `En_In`'s adult (about 530 of 1,059), `En_Horse_Game_Check` (487: 1 Ingo's race, 4 Malon's course; 2 Gerudo archery, 29 lines, Phase 18), `Bg_Ingate` (124), `Bg_Umajump` (70), `En_Ma3` (444), `En_Ma2`'s adult (about 300) | the child's parts | all |
| **The field's Poes** | `En_Po_Field` (1,018: the shared Poe about 600, its soul 180, flame 65, draw 125) | nothing | all |

**What an adult on Epona needs that the spikes' Player lacks:** the whole riding side above. That
covers `rideActor`, the mount and dismount with their animations, the riding action and its upper
body (the sword's swing and the bow from horseback), the scene exits on horseback and the
re-mounting at the next scene's start, and the horse camera.

### Sizes

| Part | Lines of C (about) |
|---|---|
| Riding: Player's riding, `En_Horse`'s Epona and rental horse (mounted, jumps, called, fleeing, floors, draw), `z_horse.c`'s gameplay setup, `z_actor.c`'s helpers, the horse camera, `Bg_Umajump` | 470 + 2,900 + 200 + 40 + 183 + 70 = 3,860 |
| Ingo's races: `En_In`'s adult (rental, races, rematch), `En_Horse`'s Ingo horse and race info, `Horse_SetupInCutscene`'s race part, `En_Horse_Game_Check` 1, `Bg_Ingate` | 400 + 200 + 40 + 220 + 124 = 980 |
| Epona won and out: `En_In`'s second win, gate and guard, `En_Horse`'s cutscene actions, `Horse_SetupInCutscene`'s jumps and the trapped Epona | 130 + 200 + 60 = 390 |
| The ranch after and the field's Poes: `En_Ma3`, `En_Ma2`'s adult, `En_Horse_Game_Check` 4, `En_Po_Field` | 444 + 300 + 140 + 1,018 = 1,900 |
| Not this phase: horseback archery, the bridge jump, `En_Horse_Game_Check` 2 (Phase 18); the credits' horses (19) | (360 + 29 + about 100) |

**About 7,100 lines**: the largest phase yet. `En_Horse` (3,868) is the largest overlay ported
so far after Player. It and Player's riding are one system: the horse places the rider from its
skin's limb at draw time, and Player reads the horse's animation frame.

### Things to check (for the user, or at a milestone's start)

- **Rupees.** `adult-awake` has 16. The rental takes 10, and the race wants 50 at the offer (the
  bet is paid only on a loss). The story's player gathers them (the ranch's pots and grass, the
  field). Coverage over completability: the race's presets add rupees (the user's call 4).
- **The first rental waits out its timer.** The race is offered only after a ridden rental has
  ended (`EVENTCHKINF_1B`), and nothing ends the first one but its 60 seconds (1,200 frames).
- **The race is deterministic but long.** The horses' `Rand` calls are their sounds, blinks and
  dust only; movement comes from the inputs alone. Winning takes a lap of steering and spurring
  (about 9,000 units, 750 to 1,000 frames) against Ingo's waypoint rider: a long scripted chase
  (the user's call 3).
- **`riderPos` comes from the draw** (`EnHorse_PostDraw`, `Skin_GetLimbPos` on limb 30, `:3703`).
  Headless runs must run the horse's post-draw, as collider spheres updated in draw do
  (`draw_update`).
- **Into the corral without the rental:** Ingo stands by its gate (`Bg_Ingate` 0 ×2). Can an
  adult walk in, call Epona and ride before paying? To check in the port once the gates are in.
- **The horse's bake:** `object_horse` and `object_hni` skins import generically
  (`oot_import/src/pack.rs:409-415`), but `En_Horse` needs its bake (as `En_Horse_Normal/skin`)
  and `ootx` characters: a pack format bump.
- **Goldens:** `En_Po_Field` sits on the field's adult layers 2 and 3, where `Route::Ruins` ends
  and `PreludeWarp` starts. Its places are all over 900 from spawn 7, but if its update calls
  `Rand` while waiting (`func_80AD6330`'s soul colour), those cases' sequences move. `En_Horse`
  doesn't spawn in the field without 0x18, and the child's scenes don't place it.
- **Talon at the stable** (`En_Ta` 2) needs 0x6B from Kakariko's adult Talon (Phase 17): until
  then the C kills him, and the port does too.

## The proposal

Four milestones in the story's order. Riding first, since everything after stands on it; the exit
at 3.

| # | Milestone | What | Run (short debug starts; no fights) |
|---|---|---|---|
| 1 | **Riding Epona** | Player's riding whole (mount, ride, dismount, the 27 branches, exits on horseback and re-mounting); `En_Horse`'s Epona and rental horse (all but Ingo's race, the cutscenes, archery and the bridge); `z_horse.c`'s gameplay setup and `Horse_InitPlayerHorse`; `z_actor.c`'s helpers; `Camera_Normal3`, `Camera_CalcAtForHorse`, `CAM_SET_HORSE`; `Bg_Umajump` whole; the bake. BACKLOG #51 (Epona's Song calls her) and #68 (the horses' sound positions) | (a) The ranch by day from `adult-awake`, moved inside the corral (`--at`): Epona flees, Epona's Song, she comes, A to mount, walk, gallop, spur, jump a corral fence, dismount. (b) A debug preset `epona` (0x18): the field at `ENTR_HYRULE_FIELD_7`, Epona's Song, she comes from her song point, ride, jump a low wall, ride into the ranch on horseback (still mounted) |
| 2 | **Ingo, the rental and the races** | `En_In`'s adult (rental, races, rematch, the losses); `En_Horse`'s Ingo horse, race info, `HORSE_PTYPE_3` and `6`; `Horse_SetupInCutscene`'s race part; `En_Horse_Game_Check` 1; `Bg_Ingate` whole; layer 4's race intro | (a) The rental: Ingo's 0x2030, 0x2031, pay, the corral (`_6`), the song, Epona mounted (`INFTABLE_AB`). (b) From a preset past the first rental (`EVENTCHKINF_1B`, 60 rupees): pay, mount, 0x2038, yes, the race's intro and start. (c) Each outcome from a preset at `_7` or `_8` (a win's 0x203A, a loss's 0x203D and the -50) |
| 3 | **Epona won and out (the exit)** | `En_In`'s second win, the gate shut and Ingo guarding it; `En_Horse`'s cutscene actions; `Horse_SetupInCutscene`'s trapped Epona and the four jumps | From a preset after the second win (state 6 at `_7`): 0x203B, 0x203C, the gate shutting, ride to a fence, the jump, the field's Epona jump cutscene, `EVENTCHKINF_EPONA_OBTAINED`: **the exit** (`epona-won`, Phase 14's start). Then riding in the field from it |
| 4 | **The ranch after and the field's Poes** | `En_Ma3` whole (Malon's course, `En_Horse_Game_Check` 4, the high score, the cow's unlock); `En_Ma2`'s adult whole; Ingo's obedient and sleeping roles; `En_Po_Field` whole | (a) Malon by day on `epona-won`: her texts and the course's start. (b) Malon by night before Epona, Epona's Song played to her. (c) The field on Epona near a Big Poe's place: it appears (riding: no roll). The fight, the soul in a bottle: the hand test's |

The scope rule is as before: every actor whole, except the parts another phase's init chooses
(their init's checks, then a placeholder). That leaves out horseback archery,
`En_Horse_Game_Check` 2 and the Gerudo Valley bridge jump (Phase 18).

**The helpers' share** (at most two at once):
- `En_Horse` and Player's riding are one system and stay in the main session, with the camera.
- Milestone 1: one helper on `Bg_Umajump` and the `En_Horse` bake and characters.
- Milestone 2: one on `En_Horse_Game_Check` and `Bg_Ingate`.
- Milestone 4: two, on `En_Po_Field`, and on `En_Ma3` with `En_Ma2`'s adult. `En_Po_Field` can
  start alongside milestone 2 or 3: it needs only Player's riding flag.

**Presets**, chained:
- `epona`: a debug start, `adult-awake` with 0x18, for milestone 1's field run.
- `ingo-rental`: after the first rental, with `EVENTCHKINF_1B` and rupees.
- `ingo-first-win`: state 4.
- `ingo-second-win`: state 6.
- `epona-won`: 0x18 after the jump; **Phase 14's start**.

## The user's calls

2026-10-24: the user took every recommendation below:
- **The split:** the four milestones in this order, the exit at 3.
- **`En_Po_Field`:** in this phase, whole; its fight the hand test's.
- **The race:** no scripted race; its intro and start in the runs, presets past each outcome, the
  race the hand test's.
- **Rupees:** `ingo-rental` sets 60, noted as a debug start's.
- **The first rental:** its run ends once Epona is mounted; `ingo-rental` stands past the timer.
- **Malon's course:** its start in a run, the laps the hand test's.
- **The `epona` debug preset** for milestone 1's field run.

The calls as proposed:

1. **The split and its order** as proposed: four milestones, riding first, the exit at 3.
   Recommended. Or: fold 3 into 2 (the exit at 2, a larger milestone).
2. **`En_Po_Field` in this phase** (COVERAGE puts it here), whole with its fight the hand test's.
   Recommended. Or: leave it for Phase 17 or 20 (the Poe collector's trade), as the field's
   breadth.
3. **The race in the runs.** Recommended: no scripted race. The runs play its intro and start
   (a few hundred frames), and debug presets stand past each outcome; the race itself is the hand
   test's. Or: one scripted race (deterministic, about 1,000 frames of steering and spurs to a
   win), for a golden of the whole race.
4. **Rupees for the race's presets** (the story's 16 aren't enough for 10 + 50). Recommended:
   `ingo-rental` sets 60 rupees, noted as a debug start's. Or: a run that gathers them (the
   ranch's pots and grass), against the long-route rule.
5. **The first rental's 60 seconds.** Recommended: the rental run ends once Epona is mounted, and
   `ingo-rental` stands past the timer. Or: a run that waits it out (1,200 frames).
6. **Malon's obstacle course** (`En_Ma3`, `En_Horse_Game_Check` 4: two laps over eight fences).
   Recommended: its start in a run, the laps the hand test's, as the races.
7. **The `epona` debug preset** for milestone 1's field run, ahead of the story's `epona-won`.
   Recommended.

## Milestone 1: Riding Epona

**Status:** done: built 2026-10-24, hand-tested 2026-10-25 ("it works"), committed and pushed on
the user's ask.

**The answer:** adult Link rides Epona:
- in Lon Lon Ranch before she's his, she runs from him (`EnHorse_FleePlayer`) until Epona's Song,
  then comes;
- A at her side ("Climb") mounts her from that side; the stick walks, trots and gallops her; A
  spurs her on ("Faster": a carrot gone, back over time); at a gallop she jumps what's in her way
  (the corral's fences: a high jump); let go, she stops; A ("Down") gets him off;
- the camera is the horse's (`CAM_SET_HORSE`, `Camera_Normal3`) while he's on her;
- riding through an exit takes him into the next scene still on her, galloping in
  (`R_EXITED_SCENE_RIDING_HORSE`); off her, Epona stays where he left her (`horseData`).

BACKLOG #51 closes (Epona's Song calls her). ADR 0091.

Scripts: `scripts\run\game-epona.bat` (menu 178: empty, `field`, `ranch`, `night`),
`test-epona.bat` (179), `sandbox-epona.bat` (180).

### What was built

- **`En_Horse`** (`en_horse.rs`, about 3,300 lines) whole but for Phase 18's parts (horseback
  archery, the Gerudo Valley bridge jump's start, the Gerudo saddle; each logged, the horse stands)
  and rumble: Epona (`object_horse`) and the rental horse (`object_hni`); every action, Ingo's race
  and the cutscene actions included (milestones 2 and 3 run them); the jumps from the post-draw;
  `riderPos` from limb 30, the head's sound point from limb 13; the dust; Epona's eyes. Its schema
  (`type`, the param types, the ranch's `rot.z`), with the bakes `En_Horse/epona` and `En_Horse/hni`.
- **`Bg_Umajump`** whole: the jumpable fence, type 1 (the obstacle course's) only with Epona had.
- **`z_horse.c`** (`oot_game::horse`) whole: `Horse_InitPlayerHorse` from `Play_Init`, the gameplay
  setup (Epona where she was left, her song points, riding in), the cutscene setup, Lake Hylia's
  fix; `z_actor.c`'s horse helpers.
- **Player's riding** whole: action handler 3, the mount from either side, `Player_Action_8084CC98`
  (riding: the stick to the horse, talking and checking from horseback), `Player_Action_8084D3E4` (getting off), re-mounting at a scene's start, the
  exits on horseback; `PlayerIface`'s `ride_actor`, `set_ride_actor`, `mount_horse`.
- **The horse camera:** `Camera_Normal3`, `Camera_CalcAtForHorse`; `Player_GetHeight` 32 higher
  riding; `func_8002DCE4`'s 8 riding.
- **The carrots** (`gCarrotIconTex`, pack format 35) by the "Faster" do-action.
- **The runs** (`playthrough.rs`; tests in `riding_runs.rs`): `Route::RideEpona` and
  `Route::FieldRide`, the tasks `Mount`, `Ride`, `RideExit`, `RideIn`, `Dismount`; the debug preset
  `epona` (`adult-awake` with `EVENTCHKINF_EPONA_OBTAINED`, the user's call 7).

### Results

- The two runs (frames):
  - the corral (`ride-epona`): 796; the song heard 77, on her 378, through the points and halted
    741, off her 786;
  - the field (`field-ride`): 640; the song heard 77, on her 428, into the gate 515, in the ranch on
    her and halted 595, off her 629.
- The texts are the song's: 0x086E, 0x086F, 0x089A.
- Epona's actions in the corral: fleeing, idle (rideable) on the song, following Link, idle, frozen
  by the mount, mounted idle, a walk, a trot, a gallop, stopping; stopped and turned in place; a
  gallop again, the high jump over the fence at (295, -27, -989) (frames 588 to 605), stopping,
  idle. Her top speed 14 (a spur's), three spurs taken.
- The field: she comes to the song from her place south of the ramp, the ramp at a trot and a
  gallop on the horse camera; in the ranch Link is on her from its first frame and she gallops in
  (`HORSE_PTYPE_PLAYER_SPAWNED_RIDING`), on the horse camera again; `R_EXITED_SCENE_RIDING_HORSE` is
  cleared by her init.
- Tests: `riding_runs` (2), `en_horse` (3: she flees until her song, then comes; the rental horse by
  day before Epona only, no horses at night before her, Epona inactive once had; the obstacle
  course's fence needs her). 1,820 tests pass, 1 ignored; `cargo test -p layering` passes; no warnings.
- Goldens: all 369 of `out/golden_base43`'s hashes the same (no earlier case rides or enters the ranch as an
  adult); 4 new cases (each run's trace and end, the fence jumped, the ramp at a gallop): 375 hashes,
  276 cases, the same over two runs (`out/golden_base44`).

### Corrections and findings

- **Player clears `rideActor` at the end of each of his updates** unless he's on it (`parent`):
  the horse offers itself again each frame before Player's update (`ACTORCAT_BG` before
  `ACTORCAT_PLAYER`). A script sees the offer only by the "Climb" do-action it gives.
- **A route's start room** reloads the room's actors; Epona, spawned into room 0 by
  `Horse_SetupInGameplay`, went with them. The ranch is one room: its route has no start room.
- **The field run starts at the ramp's foot**, not at `ENTR_HYRULE_FIELD_7` with a low wall
  jumped, as proposed: the gate's spawn (6) is on a ramp walled from the field, where Epona called
  stops at the wall below; and the gate's exit polys start partway up the ramp, so `RideExit` rides
  its turns itself and ends as the transition starts. Hyrule Field's adult layer has no
  `Bg_Umajump` (its two are on the cutscene layers 8 and 10, params 0xFFFF, one leaning: the
  schema's -1 and `rot.z`); its low walls are the hand test's.
- **A scripted gallop turns slowly** (the stick's angle is all `EnHorse_StickDirection` reads): the
  runs steer at four times the angle off, and stop her and turn in place when a point is far off
  her heading, as a player does.
- **No helper** this milestone: `Bg_Umajump` and the bake were small enough for the main session.

### Known gaps

- Her voice plays at the horse, not at her head (`unk_21C`): BACKLOG #68, which now covers
  `En_Horse` too.
- The camera's speed ratio swimming (`func_8002DCE4`'s 0.6) isn't modelled: BACKLOG #111.
- Phase 18's parts: horseback archery, the bridge jump, the Gerudo saddle. No rumble.

### How to check

```
scripts\run\test-epona.bat
cargo test --release --workspace
cargo test -p layering
python scripts\golden.py check --bin-dir target\game26\release
scripts\run\sandbox-epona.bat
```

### By hand

`scripts\run\game-epona.bat` (menu 178). WASD the stick (Shift walks), Space A, E B, J/K/L the C
buttons, I C-Up, Enter Start.

1. **Empty** (the ranch's corral by day, an adult before Epona): Epona runs from you. L for the
   ocarina, Epona's Song (I, J, L, I, J, L): she whinnies and comes. Walk to her side: "Climb" on
   A; A: Link climbs on from that side. Push the stick: a walk, a trot, a gallop; A: "Faster", a
   carrot goes and she surges; the carrots come back over time. Gallop straight at one of the
   corral's two low fences: she jumps it. Turn with the stick; let go: she slows and stops. A
   ("Down"): Link gets off on her side. The camera: behind and above her while riding, back to
   normal after.
2. **`field`** (Hyrule Field below the ranch's gate, Epona had): play her song: she comes. Get on,
   ride up the ramp to the north-west and through the gate: the ranch, Link still on her,
   galloping in. Ride about the field and try the low walls.
3. **`ranch`** (the ranch with Epona had): no rental horse in the corral; her song calls her.
4. **`night`** (the ranch at 22:00 before Epona): no horses in the corral.

## Milestone 2: Ingo, the rental and the races

**Status:** done: built and hand-tested 2026-10-25 ("this works"), committed and pushed on the
user's ask.

**The answer:** adult Link rents a horse from Ingo and races him:
- Ingo by the corral's gate offers the rental (0x2030, then 0x2031): 10 Rupees, the riding
  lesson if wanted. Link is taken into the corral with a minute on the timer and the horse's music.
  He can ride the rental horse or call Epona. When the minute runs out, Ingo stops him (0x2025) and
  takes him back to the gate.
- After a first rental Link rode in (`EVENTCHKINF_1B`), Ingo offers the race from horseback
  (0x2038: 50 Rupees). The race's intro is a 50-frame cutscene with Link and Ingo at the start line
  by the gate, then the timer and the start shot. One lap round the corral, by three checkpoints.
- The race lost: Ingo gloats (0x203D, or 0x203E after a loss before), takes the 50 Rupees and
  offers a rematch (0x205B).
- The first race won: Ingo, shaken, calls the second race at once (0x203A), for the horse.
- The second race won: Ingo gives the horse but shuts the gate (0x203B, 0x203C, the gate's
  sound, the gates swinging shut) and guards it. That run is milestone 3's: it ends in the jump out.

ADR 0092. Scripts: `scripts\run\game-ingo.bat` (menu 181: empty, `race`, `lost`, `won`),
`test-ingo.bat` (182), `sandbox-ingo.bat` (183).

### What was built

- **`En_In`'s adult whole:** the rental (`EnIn_StartRentalPeriod`, `EnIn_RentalPeriod`,
  `EnIn_RentalTimeExpired`, `EnIn_EndRentalPeriod`), the race's rider (`EnIn_Racing`), the outcomes
  (`EnIn_PlayerLostRace`, `EnIn_OfferingRematch`, `EnIn_PlayerWonFirstRace`), the second win and
  the gate (`EnIn_PlayerWonSecondRace`, `EnIn_ClosingGate`, `EnIn_FinishClosingGate`,
  `EnIn_GuardingGate`), `EnIn_StartTransition`, and the tracking's rental cases. His init no longer
  stands in placeholders for the adult. His obedient and sleeping roles (milestone 4's in the
  proposal) are whole with it. The adult's 22 lines are in his schema's texts.
- **`Bg_Ingate`** (the helper's) whole: the ranch's four gates, and the trapped gates' swing by
  the cutscene's frame.
- **`En_Horse_Game_Check`** (the helper's) whole: Ingo's race (spawning Ingo's horse, the timer,
  the start, the checkpoints, the finish and its transitions), Gerudo archery's start, type 3,
  Malon's obstacle course.
- **The runs** (`playthrough.rs`; tests in `ingo_runs.rs`): `Route::IngoRental`, `IngoRace`,
  `IngoLost`, `IngoWon`; the task `RideTalk` (ridden to a point at a walk, halted, turned to him, A
  on his offer, then as `Converse`); `Who::Ingo`. The presets `ingo-rental` (`adult-awake` past a
  first ridden rental, with 60 Rupees: the user's call 4), `ingo-race-lost` and `ingo-first-win`
  (the race word as the referee leaves it).

### Results

- The four runs (frames):
  - the rental (`ingo-rental`): 1,244. Paid 776, in the corral 798, the song heard 843, on Epona
    1,244, the timer still running;
  - the race (`ingo-race`): 1,054. Paid 273, in the corral 295, on Epona 741, the race taken from
    horseback 971, the intro, both horses racing 1,054 (Link's from the referee's 81st frame,
    Ingo's from its 82nd, the timer from its 51st);
  - the loss (`ingo-lost`): 92 (0x203D, -50 Rupees, the rematch's state);
  - the first win (`ingo-won`): 211 (0x203A, the second race's intro: layer 4).
- The texts are the C's. The rental: 0x2030 running on into 0x2031, 0x2034, 0x205C. The race:
  0x2031, 0x2034, 0x205C, Epona's Song's three, 0x2038.
- The rental's minute (a test): 0x2025 by a sub camera, Link put 100 before Ingo and held, then
  `ENTR_LON_LON_RANCH_8`, the race word back to the offer, `EVENTCHKINF_1B` for a ridden first
  rental.
- Tests: `ingo_runs` (4); `en_in` (9, two of them new or rewritten for the adult: which Ingo stays
  by the race's state, the minute running out); `bg_ingate` (2) and `en_horse_game_check` (4),
  the helper's. 1,831 tests pass, 1 ignored; `cargo test -p layering` passes; no warnings.
- Goldens: 4 of `out/golden_base44`'s 375 hashes changed, each proven in `target/game26proof`.
  With the adult Ingo's placeholder put back and the gates and referee unregistered, all four are
  as before. With only Ingo's placeholder back, `ride_epona`'s trace and end shot are (Ingo by the
  corral now), and `ride_epona_jump/shot.png` and `field_ride/trace.json` still differ (the
  corral's gates drawn, the ranch's gates in the trace). 5 new cases (9 hashes): 384 hashes, 281
  cases, the same over two runs (`out/golden_base45`).

### Corrections and findings

- **Talking from horseback needs no Z:** the talk offer comes within his range facing him. A with
  no offer is "Down" (getting off), so `RideTalk` presses A only on the offer.
- **The race starts from the scene's start:** the referee counts from its first frame. The intro
  cutscene is 50 frames, the timer starts at 51, the horses at 81 and 82.
- **Spawn 0 resets the race:** Ingo's init clears the race's word when Link's respawn point is the
  ranch's entrance (`sIsObedient`). So the presets past the race enter at `_7` and `_8`, as the
  referee sends Link, and tests of the rental enter at `_6`.
- **Malon's obstacle course, two game bugs** (the helper's, marked `@bug (game)`): it reads Link's
  horse without checking he has one, and the first fence's foul check reads `fenceCheck[-1]`
  (`raceFlags`).
- **The helper's share went as proposed:** `Bg_Ingate` and `En_Horse_Game_Check`, in parallel with
  Ingo.

### Known gaps

- The race itself, and the rental's minute run out, are the hand test's (the user's calls 3 and 5).
- The referee's `DREG(25) = 0` (a debug register nothing reads) is left out.

### How to check

```
scripts\run\test-ingo.bat
cargo test --release --workspace
cargo test -p layering
python scripts\golden.py check --bin-dir target\game26\release
scripts\run\sandbox-ingo.bat
```

### By hand

`scripts\run\game-ingo.bat` (menu 181). WASD the stick (Shift walks), Space A, E B, Q Z, J/K/L the
C buttons, I C-Up, Enter Start.

1. **Empty** (the ranch by day before Epona, by Ingo at the corral's gate): talk to him: his rant,
   then "Pay me 10 Rupees": "Ride". Listen to the lesson or not; 10 Rupees go. The corral, the
   horse's music, a 1:00 timer counting down. Ride the rental horse, or play Epona's Song (L,
   then I J L I J L) and ride her. Ride up to Ingo and talk from horseback: "going home?" Let the
   timer run out: Ingo stops you ("Time's up!"), the screen goes, back at the gate.
2. **`race`** (past a first ridden rental, 60 Rupees): rent again, call Epona, ride to Ingo and
   talk to him: the race for 50 Rupees, "Do it". The start line by the gate, the timer, the shot.
   Race him one lap round the corral (A spurs; keep inside the course). Win: Ingo, shaken, calls
   the second race (win that too: the gate scene, milestone 3's end). Lose: he gloats and takes 50
   Rupees.
3. **`lost`** (the race just lost): Ingo laughs at once, the Rupees count down by 50, then "another
   race?" (0x205B): with 50 Rupees "Race" races again; with none left (this start's case) he
   scoffs (0x2039).
4. **`won`** (the first race just won): Ingo at once: another race, for the horse. The second
   race's start line follows.

## Milestone 3: Epona won and out (the exit)

**Status:** done: built and hand-tested 2026-10-26 ("everything works"), committed and pushed
on the user's ask.

**The answer:** the phase's exit is met. After the second race is won, these play in the C's
order:
- Ingo gives Epona up and shuts the ranch's way out: 0x203B and 0x203C by a sub camera, then the
  gates swing shut with their sound.
- He stands guard. Link gallops at the shut gate with spurs and clears it: an 80-high gate needs
  her top speed.
- Past Ingo, Hyrule Field's leap cutscene plays (`gHyruleFieldGateEponaJumpCs`). Its entrance row
  makes Epona Link's (`EVENTCHKINF_EPONA_OBTAINED`), and Link is left on her in the field.

The other three leaps (over the ranch's fence south, east and west) play their cutscenes the
same way. The run's end is the preset **`epona-won`, Phase 14's start**.

ADR 0093. Scripts: `scripts\run\game-epona-won.bat` (menu 184: empty, `south`, `east`, `west`,
`gate`, `field`), `test-epona-won.bat` (185), `sandbox-epona-won.bat` (186).

### What was built

Almost everything this needed came with milestones 1 and 2:
- `Horse_SetupInGameplay`'s trapped Epona: on her at (894, 0, -2084) with the race's state 6;
- the four leaps' spawns (`HORSE_PTYPE_7` under Link, mounted);
- `En_Horse`'s cutscene actions;
- the entrance cutscene table's four rows;
- `En_In`'s gate scene and guard;
- `Bg_Ingate`'s swing.

New here:
- **`Play_LoadToLastEntrance`'s fence-jump case** (`OOT_VERSION >= PAL_1_1`): a fall after a leap
  respawns at `ENTR_HYRULE_FIELD_6` instead of replaying the leap. Ganon's collapsing tower's case
  stays Phase 19's.
- **The run** (`playthrough.rs`; tests in `epona_won_run.rs`): `Route::EponaWon` and the task
  `RideUntil` (ridden through points, not stopping, spurring from any gait when lined up, until
  a condition).
- **The presets** `ingo-second-win` (the race's state 6, `EVENTINF_INGO_RACE_SECOND_RACE`, `_0F`)
  and `epona-won` (with `EVENTCHKINF_EPONA_OBTAINED`).

### Results

- The run (`epona-won`): 1,026 frames.
  - The gate scene over at 410 (0x203B, 0x203C).
  - A run-up north, then south over the gate: the high jump at about 680. The transition to the
    field is at 687.
  - The leap's cutscene, then on Epona in the field with her his at 1,026.
- Epona's actions: idle under Link (`HORSE_PTYPE_5`), the gaits, a stop and reverse at a wall in
  the run-up, the high jump; in the field `ENHORSE_ACT_CS_UPDATE`, then mounted idle.
- The four leaps (a test): from `ENTR_HYRULE_FIELD_11`, `_12`, `_13` and `_15` without Epona,
  each cutscene starts and ends. Epona is then his, Link on her, horseData's scene the field. A
  respawn goes to `ENTR_HYRULE_FIELD_6`.
- Tests: `epona_won_run` (2). 1,839 tests pass, 1 ignored; `cargo test -p layering` passes; no
  warnings.
- Goldens: all 384 of `out/golden_base45`'s hashes the same. 5 new cases (6 hashes): 390 hashes,
  286 cases, the same over two runs (`out/golden_base46`).

### Corrections and findings

- **The gate needs her top speed.** It's 80 high, and `EnHorse_UpdateBgCheckInfo` jumps more than
  72 only above 13.8. A gallop under that brakes from 150 off. The run takes a run-up and spurs
  from a walk on, which `EnHorse_CheckBoost` allows.
- **The ranch side was ported ahead:** the trapped Epona, the cutscene horse and the leaps came
  whole with milestone 1's `En_Horse` and `z_horse.c`. No helper was needed.

### Known gaps

- Riding in the field from `epona-won` is the hand test's (the `field-ride` run covers riding into
  the ranch from the field).
- The phase's last milestone (4: the ranch after Epona and the field's Poes) is open.

### How to check

```
scripts\run\test-epona-won.bat
cargo test --release --workspace
cargo test -p layering
python scripts\golden.py check --bin-dir target\game26\release
scripts\run\sandbox-epona-won.bat
```

### By hand

`scripts\run\game-epona-won.bat` (menu 184). WASD the stick (Shift walks), Space A, E B, Q Z, J/K/L
the C buttons, I C-Up, Enter Start.

1. **Empty** (the second race just won, on Epona by the way out):
   - Ingo: "Is that Epona?", then "I'll never let you leave this ranch!".
   - The camera pulls back as the gates swing shut with a clang.
   - Get out: gallop at the gate with spurs (A) and jump it, or jump the ranch's outer fence. The
     field's leap cutscene plays, and you ride on in the field.
2. **`south`, `east`, `west`, `gate`:** straight into each leap's cutscene in the field.
3. **`field`** (`epona-won`): Hyrule Field outside the ranch's gate with Epona yours: her song
   calls her; ride, and try the field's low walls.
